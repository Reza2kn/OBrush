//! Staged, independent host input state. No window, renderer, timer or file IO.
//! Integration must discard invalidated host queues and apply each timer permit
//! synchronously; this module never creates a timer callback backlog.
use obrush_core::{BrushKind, BrushSettings, LockedStamp, RasterAlpha, Vec3};

pub const MAX_ALPHA_SIDE: u32 = 1024;
pub const MAX_ALPHA_PIXELS: usize = 1024 * 1024;
pub const MAX_IMPORTED_ALPHAS: usize = 32;
pub const MAX_IMPORTED_ALPHA_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PENDING_SAMPLES: usize = 32;
pub const MAX_ALPHA_NAME_CHARS: usize = 80;
pub const TOOL_COUNT: usize = BrushKind::ALL.len();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaSelection {
    None,
    Square,
    Skin,
    Imported(u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaConversion {
    LegacyWeightedOpacity,
    SculptGlMeanRgb,
}

#[derive(Clone, Debug)]
pub struct AlphaEntry {
    pub id: u64,
    pub name: String,
    pub conversion: AlphaConversion,
    bitmap: RasterAlpha,
}
impl AlphaEntry {
    pub fn bitmap(&self) -> &RasterAlpha {
        &self.bitmap
    }
}

/// Global immutable raster library; all selectors refer to stable identifiers.
#[derive(Clone)]
pub struct AlphaLibrary {
    square: RasterAlpha,
    skin: RasterAlpha,
    imports: Vec<AlphaEntry>,
    imported_bytes: usize,
    next_id: u64,
}
impl AlphaLibrary {
    pub fn new() -> Result<Self, String> {
        let mut square = Vec::with_capacity(64 * 64);
        let mut skin = Vec::with_capacity(64 * 64);
        for y in 0..64_u32 {
            for x in 0..64_u32 {
                let u = (f64::from(x) + 0.5) / 32.0 - 1.0;
                let v = (f64::from(y) + 0.5) / 32.0 - 1.0;
                // A soft square interior, authored independently of bundled JPEGs.
                let s = ((0.82 - u.abs().max(v.abs())) / 0.12).clamp(0.0, 1.0);
                square.push((255.0 * s * s * (3.0 - 2.0 * s)).round() as u8);
                // An independently authored deterministic pore field with a fade.
                let coarse = hash_pixel(x / 3, y / 3) & 255;
                let fine = hash_pixel(x, y) & 255;
                let pore = 0.15 + 0.70 * f64::from(coarse) / 255.0 + 0.15 * f64::from(fine) / 255.0;
                let fade = (1.0 - u * u - v * v).clamp(0.0, 1.0);
                skin.push((255.0 * pore * fade * fade).round() as u8);
            }
        }
        Ok(Self {
            square: RasterAlpha::new(64, 64, square)?,
            skin: RasterAlpha::new(64, 64, skin)?,
            imports: Vec::new(),
            imported_bytes: 0,
            next_id: 1,
        })
    }
    pub fn entries(&self) -> &[AlphaEntry] {
        &self.imports
    }
    pub fn imported_bytes(&self) -> usize {
        self.imported_bytes
    }
    pub fn resolve(&self, id: AlphaSelection) -> Result<Option<&RasterAlpha>, String> {
        match id {
            AlphaSelection::None => Ok(None),
            AlphaSelection::Square => Ok(Some(&self.square)),
            AlphaSelection::Skin => Ok(Some(&self.skin)),
            AlphaSelection::Imported(id) => self
                .imports
                .iter()
                .find(|entry| entry.id == id)
                .map(|e| Some(&e.bitmap))
                .ok_or_else(|| "Unknown imported alpha identifier".into()),
        }
    }
    fn import(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
        conversion: AlphaConversion,
    ) -> Result<AlphaSelection, String> {
        let count = decoded_pixels(width, height, rgba)?;
        if self.imports.len() >= MAX_IMPORTED_ALPHAS
            || count > MAX_IMPORTED_ALPHA_BYTES.saturating_sub(self.imported_bytes)
        {
            return Err("Imported alpha library is full".into());
        }
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Imported alpha identifiers exhausted")?;
        let name: String = name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_ALPHA_NAME_CHARS)
            .collect();
        if name.trim().is_empty() {
            return Err("Imported alpha requires a display name".into());
        }
        let bitmap = match conversion {
            AlphaConversion::LegacyWeightedOpacity => RasterAlpha::from_rgba(width, height, rgba)?,
            AlphaConversion::SculptGlMeanRgb => {
                let mut pixels = Vec::new();
                pixels
                    .try_reserve_exact(count)
                    .map_err(|_| "Cannot allocate imported alpha")?;
                pixels.extend(rgba.chunks_exact(4).map(|p| {
                    // The pinned SculptGL browser draws into Canvas2D and
                    // reads the image back before averaging RGB. Its 8-bit
                    // premultiply/unpremultiply round-trip changes low-alpha
                    // channels, including hidden RGB at alpha zero.
                    let alpha = u32::from(p[3]);
                    let readback = |channel: u8| {
                        if alpha == 0 {
                            0
                        } else {
                            let premultiplied = (u32::from(channel) * alpha + 127) / 255;
                            ((premultiplied * 255 + alpha / 2) / alpha).min(255)
                        }
                    };
                    ((readback(p[0]) + readback(p[1]) + readback(p[2]) + 1) / 3) as u8
                }));
                RasterAlpha::new(width, height, pixels)?
            }
        };
        self.imports
            .try_reserve(1)
            .map_err(|_| "Cannot allocate alpha library entry")?;
        let id = self.next_id;
        self.imports.push(AlphaEntry {
            id,
            name,
            conversion,
            bitmap,
        });
        self.next_id = next_id;
        self.imported_bytes += count;
        Ok(AlphaSelection::Imported(id))
    }
    fn remove(&mut self, id: u64) -> Result<(), String> {
        let index = self
            .imports
            .iter()
            .position(|e| e.id == id)
            .ok_or("Unknown imported alpha identifier")?;
        self.imported_bytes -= self.imports[index].bitmap.pixels().len();
        self.imports.remove(index);
        Ok(())
    }
}
fn hash_pixel(x: u32, y: u32) -> u32 {
    let mut z = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b) ^ 0x62b8_7d31;
    z ^= z >> 16;
    z = z.wrapping_mul(0x7feb_352d);
    z ^= z >> 15;
    z
}
fn decoded_pixels(width: u32, height: u32, rgba: &[u8]) -> Result<usize, String> {
    if width == 0 || height == 0 || width > MAX_ALPHA_SIDE || height > MAX_ALPHA_SIDE {
        return Err("Alpha dimensions exceed the bounded raster size".into());
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .filter(|&n| n <= MAX_ALPHA_PIXELS)
        .ok_or("Alpha pixel limit exceeded")?;
    if rgba.len() != count * 4 {
        return Err("Decoded RGBA length differs from dimensions".into());
    }
    Ok(count)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolPreferences {
    pub radius: f32,
    pub screen_space_radius: bool,
    pub screen_radius_pixels: f32,
    pub strength: f32,
    pub invert: bool,
    pub color: [f32; 3],
    pub paint_rgb: bool,
    pub paint_roughness: bool,
    pub paint_metallic: bool,
    pub roughness: f32,
    pub metallic: f32,
    pub hardness: f32,
    pub backface_culling: bool,
    pub adaptive_detail: bool,
    pub smooth_tangent: bool,
    pub draw_clay: bool,
    pub draw_accumulate: bool,
    pub move_topology_check: bool,
    pub move_along_normal: bool,
    pub alpha: AlphaSelection,
    pub alpha_rotation: f32,
    pub lock_position: bool,
}
impl ToolPreferences {
    pub fn from_settings(s: &BrushSettings) -> Self {
        Self {
            radius: s.radius,
            screen_space_radius: false,
            screen_radius_pixels: 75.0,
            strength: s.strength,
            invert: s.invert,
            color: s.color,
            paint_rgb: s.paint_rgb,
            paint_roughness: s.paint_roughness,
            paint_metallic: s.paint_metallic,
            roughness: s.roughness,
            metallic: s.metallic,
            hardness: s.hardness,
            backface_culling: s.backface_culling,
            adaptive_detail: s.adaptive_detail,
            smooth_tangent: s.smooth_tangent,
            draw_clay: s.draw_clay,
            draw_accumulate: s.draw_accumulate,
            move_topology_check: s.move_topology_check,
            move_along_normal: s.move_along_normal,
            alpha: AlphaSelection::None,
            alpha_rotation: s.alpha_rotation,
            lock_position: false,
        }
    }
    fn validate(&self, kind: BrushKind) -> Result<(), String> {
        if !self.radius.is_finite()
            || self.radius <= 1e-6
            || self.radius > 1_000_000.0
            || !self.screen_radius_pixels.is_finite()
            || !(5.0..=500.0).contains(&self.screen_radius_pixels)
            || !self.strength.is_finite()
            || !(0.0..=1.0).contains(&self.strength)
            || !self.hardness.is_finite()
            || !(0.0..=1.0).contains(&self.hardness)
            || self
                .color
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !self.roughness.is_finite()
            || !(0.0..=1.0).contains(&self.roughness)
            || !self.metallic.is_finite()
            || !(0.0..=1.0).contains(&self.metallic)
            || !self.alpha_rotation.is_finite()
        {
            return Err("Tool preferences must contain bounded finite values".into());
        }
        if self.lock_position && !LockedStamp::supports_kind(kind) {
            return Err("This tool does not support locked stamps".into());
        }
        Ok(())
    }
    fn settings(
        &self,
        kind: BrushKind,
        global: &GlobalPreferences,
        alpha: Option<RasterAlpha>,
    ) -> BrushSettings {
        BrushSettings {
            kind,
            radius: self.radius,
            strength: self.strength,
            invert: self.invert,
            color: self.color,
            paint_rgb: self.paint_rgb,
            paint_roughness: self.paint_roughness,
            paint_metallic: self.paint_metallic,
            roughness: self.roughness,
            metallic: self.metallic,
            hardness: self.hardness,
            backface_culling: self.backface_culling,
            adaptive_detail: self.adaptive_detail,
            smooth_tangent: self.smooth_tangent,
            draw_clay: self.draw_clay,
            draw_accumulate: self.draw_accumulate,
            move_topology_check: self.move_topology_check,
            move_along_normal: self.move_along_normal,
            alpha,
            alpha_rotation: self.alpha_rotation,
            symmetry_x: global.symmetry_x,
            symmetry_offset: global.symmetry_offset,
            ..Default::default()
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PressureProfile {
    LegacyBinary { strength: bool, radius: bool },
    SculptGlFactors { intensity: f32, radius: f32 },
}
impl PressureProfile {
    fn validate(self) -> Result<(), String> {
        if let Self::SculptGlFactors { intensity, radius } = self
            && (!intensity.is_finite()
                || !radius.is_finite()
                || !(0.0..=1.0).contains(&intensity)
                || !(0.0..=1.0).contains(&radius))
        {
            return Err("Pressure factors must be finite and in0..1".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierPolicy {
    OBrushLegacy,
    SculptGlPinned,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}
/// A request is metadata until the corresponding core response is implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseProfileRequest {
    OBrushLegacy015,
    SculptGlPinned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponseProfileStatus {
    pub requested: ResponseProfileRequest,
    pub applied: ResponseProfileRequest,
    pub requested_implemented: bool,
    pub reference_verified: bool,
}
impl ResponseProfileRequest {
    pub fn status(self) -> ResponseProfileStatus {
        ResponseProfileStatus {
            requested: self,
            applied: Self::OBrushLegacy015,
            requested_implemented: self == Self::OBrushLegacy015,
            reference_verified: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalPreferences {
    pub symmetry_x: bool,
    pub symmetry_offset: f32,
    pub continuous: bool,
    pub pressure: PressureProfile,
    pub modifiers: ModifierPolicy,
    pub response_profile: ResponseProfileRequest,
}
impl GlobalPreferences {
    pub fn from_settings(s: &BrushSettings) -> Self {
        Self {
            symmetry_x: s.symmetry_x,
            symmetry_offset: s.symmetry_offset,
            continuous: false,
            pressure: PressureProfile::LegacyBinary {
                strength: s.pressure_strength,
                radius: s.pressure_radius,
            },
            modifiers: ModifierPolicy::OBrushLegacy,
            response_profile: ResponseProfileRequest::OBrushLegacy015,
        }
    }
    fn validate(self) -> Result<(), String> {
        self.pressure.validate()?;
        if !self.symmetry_offset.is_finite() || self.symmetry_offset.abs() > 1_000_000.0 {
            return Err("Symmetry offset is invalid".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContactInput {
    Mouse,
    Pen(f32),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureResponse {
    pub normalized_contact: f32,
    pub intensity_multiplier: f32,
    pub radius_multiplier: f32,
    pub core_strength_clipped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureToken {
    id: u64,
    revision: u64,
}
impl GestureToken {
    pub fn id(self) -> u64 {
        self.id
    }
    pub fn revision(self) -> u64 {
        self.revision
    }
}
/// Stored logical-pixel request; pressure scales its effective size at sampling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRadiusPreferences {
    pub enabled: bool,
    pub pixels: f32,
}
/// Captured base and effective radius. Host camera conversion must apply the
/// supplied pressure multiplier only once; this descriptor performs no projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRadiusSample {
    pub enabled: bool,
    pub base_pixels: f32,
    pub effective_pixels: f32,
}
#[derive(Clone, Debug)]
pub struct PreviewSample {
    pub settings: BrushSettings,
    pub pressure: PressureResponse,
    pub response_profile: ResponseProfileStatus,
    pub screen_radius: ScreenRadiusSample,
}
#[derive(Clone, Debug)]
pub struct FrozenSample {
    pub token: GestureToken,
    pub screen_radius: ScreenRadiusSample,
    pub settings: BrushSettings,
    pub pressure: PressureResponse,
    pub response_profile: ResponseProfileStatus,
}
#[derive(Clone, Debug)]
pub struct FrozenGesture {
    token: GestureToken,
    base: BrushSettings,
    screen_radius: ScreenRadiusPreferences,
    pressure: PressureProfile,
    lock_position: bool,
    continuous: bool,
    response_profile: ResponseProfileRequest,
}
impl FrozenGesture {
    pub fn token(&self) -> GestureToken {
        self.token
    }
    pub fn kind(&self) -> BrushKind {
        self.base.kind
    }
    pub fn lock_position(&self) -> bool {
        self.lock_position
    }
    pub fn wants_continuous(&self) -> bool {
        self.continuous
    }
    pub fn screen_radius(&self) -> ScreenRadiusPreferences {
        self.screen_radius
    }
    fn sample(
        &self,
        input: ContactInput,
        culling_axis: Option<Vec3>,
        alpha_tangent: Option<Vec3>,
    ) -> Result<FrozenSample, String> {
        let sample = resolve_sample(
            &self.base,
            self.pressure,
            self.response_profile,
            self.screen_radius,
            input,
            culling_axis,
            alpha_tangent,
        )?;
        Ok(FrozenSample {
            token: self.token,
            screen_radius: sample.screen_radius,
            settings: sample.settings,
            pressure: sample.pressure,
            response_profile: sample.response_profile,
        })
    }
}
fn resolve_sample(
    base: &BrushSettings,
    profile: PressureProfile,
    response_profile: ResponseProfileRequest,
    screen: ScreenRadiusPreferences,
    input: ContactInput,
    culling_axis: Option<Vec3>,
    alpha_tangent: Option<Vec3>,
) -> Result<PreviewSample, String> {
    if culling_axis.is_some_and(|v| !v.is_finite() || v == Vec3::ZERO)
        || alpha_tangent.is_some_and(|v| !v.is_finite())
    {
        return Err("Invalid captured sample axes".into());
    }
    let (contact, curve_pressure) = match input {
        ContactInput::Mouse => (1.0, 0.5),
        ContactInput::Pen(p) => {
            if !p.is_finite() {
                return Err("Pen pressure must be finite".into());
            }
            let p = p.clamp(0.0, 1.0);
            (p, p)
        }
    };
    let mut settings = base.clone();
    settings.culling_axis = culling_axis;
    settings.alpha_tangent = alpha_tangent;
    let (intensity_multiplier, radius_multiplier) = match profile {
        PressureProfile::LegacyBinary { strength, radius } => {
            settings.pressure = contact;
            settings.pressure_strength = strength;
            settings.pressure_radius = radius;
            (
                if strength { contact } else { 1.0 },
                if radius { contact } else { 1.0 },
            )
        }
        PressureProfile::SculptGlFactors { intensity, radius } => {
            let i = 1.0 + intensity * (curve_pressure * 2.0 - 1.0);
            let r = 1.0 + radius * (curve_pressure * 2.0 - 1.0);
            settings.pressure = if contact > 0.0 { 1.0 } else { 0.0 };
            settings.pressure_strength = false;
            settings.pressure_radius = false;
            settings.strength *= i;
            settings.radius *= r;
            (i, r)
        }
    };
    if !settings.radius.is_finite() || settings.radius > 1_000_000.0 {
        return Err("Pressure-scaled radius exceeds the sculpting bound".into());
    }
    let pressure = PressureResponse {
        normalized_contact: contact,
        intensity_multiplier,
        radius_multiplier,
        core_strength_clipped: settings.strength > 1.0,
    };
    let effective_pixels = screen.pixels * radius_multiplier;
    if !effective_pixels.is_finite() || !(0.0..=1000.0).contains(&effective_pixels) {
        return Err("Pressure-scaled screen radius exceeds its bound".into());
    }
    Ok(PreviewSample {
        settings,
        pressure,
        response_profile: response_profile.status(),
        screen_radius: ScreenRadiusSample {
            enabled: screen.enabled,
            base_pixels: screen.pixels,
            effective_pixels,
        },
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interruption {
    Release,
    Cancel,
    NoContact,
    Navigation,
    FocusLost,
    TargetChanged,
    ClockReversed,
    ClockOverflow,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueInvalidation {
    pub gesture: Option<GestureToken>,
    pub discarded_samples: usize,
    pub reason: Interruption,
}

#[derive(Clone)]
pub struct BrushPreferenceStore {
    tools: [ToolPreferences; TOOL_COUNT],
    global: GlobalPreferences,
    selected: BrushKind,
    library: AlphaLibrary,
    revision: u64,
    next_gesture: u64,
    active: Option<FrozenGesture>,
    pending: usize,
}
impl BrushPreferenceStore {
    /// Compatibility initialization. Supplied bitmap is imported once globally
    /// and selected only for the starting tool; it is not silently lost.
    pub fn from_legacy(
        s: &BrushSettings,
        alpha_name: Option<&str>,
        lock_position: bool,
    ) -> Result<Self, String> {
        let mut seed = ToolPreferences::from_settings(s);
        seed.lock_position = lock_position;
        seed.validate(s.kind)?;
        let global = GlobalPreferences::from_settings(s);
        global.validate()?;
        let mut library = AlphaLibrary::new()?;
        if let Some(alpha) = &s.alpha {
            let name = alpha_name
                .unwrap_or("Existing alpha")
                .chars()
                .filter(|c| !c.is_control())
                .take(MAX_ALPHA_NAME_CHARS)
                .collect::<String>();
            if name.trim().is_empty() {
                return Err("Existing alpha requires a display name".into());
            }
            library.imported_bytes = alpha.pixels().len();
            library.next_id = 2;
            library.imports.push(AlphaEntry {
                id: 1,
                name,
                conversion: AlphaConversion::LegacyWeightedOpacity,
                bitmap: alpha.clone(),
            });
            seed.alpha = AlphaSelection::Imported(1);
        }
        let mut tools = std::array::from_fn(|_| {
            let mut p = seed.clone();
            p.alpha = AlphaSelection::None;
            p.lock_position = false;
            p
        });
        tools[tool_index(s.kind)] = seed;
        Ok(Self {
            tools,
            global,
            selected: s.kind,
            library,
            revision: 0,
            next_gesture: 1,
            active: None,
            pending: 0,
        })
    }
    pub fn selected(&self) -> BrushKind {
        self.selected
    }
    pub fn tool(&self, kind: BrushKind) -> &ToolPreferences {
        &self.tools[tool_index(kind)]
    }
    pub fn global(&self) -> GlobalPreferences {
        self.global
    }
    pub fn library(&self) -> &AlphaLibrary {
        &self.library
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn pending_samples(&self) -> usize {
        self.pending
    }
    pub fn active(&self) -> Option<&FrozenGesture> {
        self.active.as_ref()
    }
    fn editable_revision(&self) -> Result<u64, String> {
        if self.active.is_some() || self.pending > 0 {
            return Err("Finish or interrupt the gesture and discard captured inputs before changing preferences".into());
        }
        self.revision
            .checked_add(1)
            .ok_or_else(|| "Preference revision exhausted".into())
    }
    pub fn set_selected(&mut self, kind: BrushKind) -> Result<(), String> {
        let next = self.editable_revision()?;
        self.selected = kind;
        self.revision = next;
        Ok(())
    }
    pub fn set_tool(&mut self, kind: BrushKind, prefs: ToolPreferences) -> Result<(), String> {
        let next = self.editable_revision()?;
        prefs.validate(kind)?;
        self.library.resolve(prefs.alpha)?;
        self.tools[tool_index(kind)] = prefs;
        self.revision = next;
        Ok(())
    }
    pub fn set_global(&mut self, prefs: GlobalPreferences) -> Result<(), String> {
        let next = self.editable_revision()?;
        prefs.validate()?;
        self.global = prefs;
        self.revision = next;
        Ok(())
    }
    /// Destination is the tool which initiated asynchronous file selection,
    /// rather than whichever tool happens to be selected when decoding ends.
    pub fn import_rgba(
        &mut self,
        destination: BrushKind,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
        conversion: AlphaConversion,
    ) -> Result<AlphaSelection, String> {
        let next = self.editable_revision()?;
        let id = self.library.import(name, width, height, rgba, conversion)?;
        self.tools[tool_index(destination)].alpha = id;
        self.revision = next;
        Ok(id)
    }
    pub fn remove_alpha(&mut self, id: u64) -> Result<(), String> {
        let next = self.editable_revision()?;
        self.library.remove(id)?;
        for prefs in &mut self.tools {
            if prefs.alpha == AlphaSelection::Imported(id) {
                prefs.alpha = AlphaSelection::None;
            }
        }
        self.revision = next;
        Ok(())
    }
    fn resolve_tool_settings(
        &self,
        modifiers: Modifiers,
    ) -> Result<(BrushSettings, ScreenRadiusPreferences), String> {
        let kind = effective_tool(self.selected, modifiers, self.global.modifiers);
        let prefs = self.tool(kind);
        let mut base = prefs.settings(
            kind,
            &self.global,
            self.library.resolve(prefs.alpha)?.cloned(),
        );
        if self.global.modifiers == ModifierPolicy::OBrushLegacy {
            base.invert ^= modifiers.alt;
        } else if modifiers.alt && !modifiers.shift {
            if kind == BrushKind::Move {
                base.move_along_normal = !base.move_along_normal;
            } else if reference_negative_control(kind) {
                base.invert = !base.invert;
            }
        }
        Ok((
            base,
            ScreenRadiusPreferences {
                enabled: prefs.screen_space_radius,
                pixels: prefs.screen_radius_pixels,
            },
        ))
    }
    /// Read-only hover resolution. It neither creates nor changes a token,
    /// revision, pending input count, frozen gesture, history or timer permit.
    pub fn preview(
        &self,
        modifiers: Modifiers,
        input: ContactInput,
        culling_axis: Option<Vec3>,
        alpha_tangent: Option<Vec3>,
    ) -> Result<PreviewSample, String> {
        let (base, screen) = self.resolve_tool_settings(modifiers)?;
        resolve_sample(
            &base,
            self.global.pressure,
            self.global.response_profile,
            screen,
            input,
            culling_axis,
            alpha_tangent,
        )
    }
    pub fn begin_gesture(&mut self, modifiers: Modifiers) -> Result<FrozenGesture, String> {
        if self.active.is_some() || self.pending > 0 {
            return Err("A gesture or captured inputs are already active".into());
        }
        let next = self
            .next_gesture
            .checked_add(1)
            .ok_or("Gesture identifiers exhausted")?;
        let (base, screen_radius) = self.resolve_tool_settings(modifiers)?;
        let prefs = self.tool(base.kind);
        let gesture = FrozenGesture {
            token: GestureToken {
                id: self.next_gesture,
                revision: self.revision,
            },
            base,
            screen_radius,
            pressure: self.global.pressure,
            lock_position: prefs.lock_position,
            continuous: self.global.continuous,
            response_profile: self.global.response_profile,
        };
        self.next_gesture = next;
        self.active = Some(gesture.clone());
        Ok(gesture)
    }
    pub fn sample(
        &self,
        token: GestureToken,
        input: ContactInput,
        axis: Option<Vec3>,
        tangent: Option<Vec3>,
    ) -> Result<FrozenSample, String> {
        self.require_token(token)?.sample(input, axis, tangent)
    }
    pub fn queue_sample(
        &mut self,
        token: GestureToken,
        input: ContactInput,
        axis: Option<Vec3>,
        tangent: Option<Vec3>,
    ) -> Result<FrozenSample, String> {
        if self.pending >= MAX_PENDING_SAMPLES {
            return Err("Captured input queue is full".into());
        }
        let sample = self.sample(token, input, axis, tangent)?;
        self.pending += 1;
        Ok(sample)
    }
    pub fn consume_queued(&mut self, token: GestureToken) -> Result<(), String> {
        self.require_token(token)?;
        if self.pending == 0 {
            return Err("No captured input remains".into());
        }
        self.pending -= 1;
        Ok(())
    }
    pub fn end_gesture(&mut self, token: GestureToken) -> Result<(), String> {
        self.require_token(token)?;
        if self.pending > 0 {
            return Err("Captured inputs must be consumed or discarded before finishing".into());
        }
        self.active = None;
        Ok(())
    }
    pub fn interrupt(&mut self, reason: Interruption) -> QueueInvalidation {
        let receipt = QueueInvalidation {
            gesture: self.active.as_ref().map(FrozenGesture::token),
            discarded_samples: self.pending,
            reason,
        };
        self.active = None;
        self.pending = 0;
        receipt
    }
    pub fn require_token(&self, token: GestureToken) -> Result<&FrozenGesture, String> {
        self.active
            .as_ref()
            .filter(|g| g.token == token && token.revision == self.revision)
            .ok_or_else(|| "Captured gesture token is stale".into())
    }
}
fn tool_index(kind: BrushKind) -> usize {
    BrushKind::ALL
        .iter()
        .position(|&k| k == kind)
        .expect("Complete fixed tool enumeration")
}
fn effective_tool(selected: BrushKind, m: Modifiers, policy: ModifierPolicy) -> BrushKind {
    match policy {
        ModifierPolicy::OBrushLegacy => {
            if m.control {
                BrushKind::Mask
            } else if m.shift {
                BrushKind::Smooth
            } else {
                selected
            }
        }
        ModifierPolicy::SculptGlPinned => {
            if m.shift && !m.control && !m.alt {
                BrushKind::Smooth
            } else if m.control && !m.shift && !m.alt {
                BrushKind::Mask
            } else {
                selected
            }
        }
    }
}
fn reference_negative_control(kind: BrushKind) -> bool {
    matches!(
        kind,
        BrushKind::Draw
            | BrushKind::Clay
            | BrushKind::Inflate
            | BrushKind::Flatten
            | BrushKind::Crease
            | BrushKind::Pinch
            | BrushKind::Mask
    )
}

/// Observed raw source defaults, not a world-unit or inverse-semantics mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceDefaults {
    pub radius_pixels: f32,
    pub intensity: Option<f32>,
    pub negative: Option<bool>,
    pub hardness: Option<f32>,
    pub clay: Option<bool>,
    pub accumulate: Option<bool>,
    pub topology_check: Option<bool>,
    pub culling: Option<bool>,
    pub tangent: Option<bool>,
}
pub fn observed_reference_defaults(kind: BrushKind) -> Option<ReferenceDefaults> {
    let mut d = ReferenceDefaults {
        radius_pixels: 50.0,
        intensity: Some(0.75),
        negative: Some(false),
        hardness: None,
        clay: None,
        accumulate: None,
        topology_check: None,
        culling: Some(false),
        tangent: None,
    };
    match kind {
        BrushKind::Draw | BrushKind::Clay => {
            d.intensity = Some(0.5);
            d.clay = Some(true);
            d.accumulate = Some(true);
        }
        BrushKind::Inflate => d.intensity = Some(0.3),
        BrushKind::Smooth => {
            d.negative = None;
            d.tangent = Some(false);
        }
        BrushKind::Flatten => d.negative = Some(true),
        BrushKind::Crease => {
            d.radius_pixels = 25.0;
            d.negative = Some(true);
        }
        BrushKind::Move => {
            d.radius_pixels = 150.0;
            d.intensity = Some(1.0);
            d.topology_check = Some(true);
            d.culling = None;
        }
        BrushKind::Mask => {
            d.hardness = Some(0.25);
            d.intensity = Some(1.0);
            d.negative = Some(true);
        }
        BrushKind::Paint => {
            d.hardness = Some(0.75);
            d.negative = None;
        }
        BrushKind::Pinch => {}
        BrushKind::Twist => {
            d.radius_pixels = 75.0;
            d.intensity = None;
            d.negative = None;
        }
        BrushKind::LocalScale => {
            d.intensity = None;
            d.negative = None;
        }
        BrushKind::Drag => {
            d.radius_pixels = 150.0;
            d.intensity = None;
            d.negative = None;
            d.culling = None;
        }
        BrushKind::Morph => return None,
    }
    Some(d)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousCapability {
    Ordinary,
    LockedAccumulationRequired,
    Unsupported,
}
pub fn continuous_capability(kind: BrushKind, locked: bool) -> ContinuousCapability {
    if !LockedStamp::supports_kind(kind) {
        ContinuousCapability::Unsupported
    } else if locked {
        ContinuousCapability::LockedAccumulationRequired
    } else {
        ContinuousCapability::Ordinary
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContinuousPolicy {
    pub period_micros: u64,
    pub pause_gap_micros: u64,
}
impl Default for ContinuousPolicy {
    fn default() -> Self {
        Self {
            period_micros: 16_600,
            pause_gap_micros: 100_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinuousReadiness {
    pub token: Option<GestureToken>,
    pub primary_down: bool,
    pub pressure: f32,
    pub focused: bool,
    pub navigating: bool,
    pub target_visible: bool,
    pub pointer_in_viewport: bool,
    pub queue_has_room: bool,
    pub operation_busy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContinuousTick {
    pub token: GestureToken,
    pub at_micros: u64,
}
#[derive(Clone, Copy, Debug)]
struct TimerState {
    token: GestureToken,
    last_poll: u64,
    next_due: u64,
}
pub struct ContinuousScheduler {
    policy: ContinuousPolicy,
    state: Option<TimerState>,
    emitted: u64,
    skipped: u64,
    last_stop: Option<Interruption>,
}
impl ContinuousScheduler {
    pub fn new(policy: ContinuousPolicy) -> Result<Self, String> {
        if !(1_000..=100_000).contains(&policy.period_micros)
            || policy.pause_gap_micros < policy.period_micros
            || policy.pause_gap_micros > 1_000_000
        {
            return Err("Continuous clock policy is outside bounded intervals".into());
        }
        Ok(Self {
            policy,
            state: None,
            emitted: 0,
            skipped: 0,
            last_stop: None,
        })
    }
    pub fn start(&mut self, gesture: &FrozenGesture, now: u64) -> Result<(), String> {
        if self.state.is_some() {
            return Err("Continuous gesture is already active".into());
        }
        if !gesture.continuous
            || continuous_capability(gesture.kind(), gesture.lock_position)
                != ContinuousCapability::Ordinary
        {
            return Err("Gesture does not support ordinary continuous dabs".into());
        }
        let next_due = now
            .checked_add(self.policy.period_micros)
            .ok_or("Continuous timestamp overflow")?;
        self.state = Some(TimerState {
            token: gesture.token,
            last_poll: now,
            next_due,
        });
        self.last_stop = None;
        Ok(())
    }
    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }
    pub fn emitted(&self) -> u64 {
        self.emitted
    }
    /// Number of paused/overloaded polls, not the number of elapsed timer ticks.
    pub fn skipped(&self) -> u64 {
        self.skipped
    }
    pub fn last_stop(&self) -> Option<Interruption> {
        self.last_stop
    }
    pub fn stop(&mut self, reason: Interruption) {
        self.state = None;
        self.last_stop = Some(reason);
    }
    /// Current readiness must be read every host frame. Never enqueue returned
    /// permits for later: capture current pressure/settings and execute once.
    pub fn poll(&mut self, now: u64, r: ContinuousReadiness) -> Option<ContinuousTick> {
        let mut state = self.state?;
        let stop = if r.token != Some(state.token) {
            Some(Interruption::TargetChanged)
        } else if !r.primary_down {
            Some(Interruption::Release)
        } else if !r.pressure.is_finite() || r.pressure <= 0.0 {
            Some(Interruption::NoContact)
        } else if !r.focused {
            Some(Interruption::FocusLost)
        } else if r.navigating {
            Some(Interruption::Navigation)
        } else if !r.target_visible || !r.pointer_in_viewport {
            Some(Interruption::TargetChanged)
        } else if now < state.last_poll {
            Some(Interruption::ClockReversed)
        } else {
            None
        };
        if let Some(reason) = stop {
            self.stop(reason);
            return None;
        }
        let next = match now.checked_add(self.policy.period_micros) {
            Some(v) => v,
            None => {
                self.stop(Interruption::ClockOverflow);
                return None;
            }
        };
        let paused = now - state.last_poll > self.policy.pause_gap_micros;
        state.last_poll = now;
        if paused || !r.queue_has_room || r.operation_busy {
            state.next_due = next;
            self.skipped = self.skipped.saturating_add(1);
            self.state = Some(state);
            return None;
        }
        if now < state.next_due {
            self.state = Some(state);
            return None;
        }
        state.next_due = next;
        self.state = Some(state);
        self.emitted = self.emitted.saturating_add(1);
        Some(ContinuousTick {
            token: state.token,
            at_micros: now,
        })
    }
}
