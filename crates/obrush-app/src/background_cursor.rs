//! Optional empty-view cursor presentation; never a geometry or input setter.
//!
//! Source receives JS numbers; this host receives genuine egui f32 positions.
//! Promotion for matrix arithmetic does not recover discarded input precision.
use eframe::egui;

const MAX_COORDINATE: f32 = 16_777_216.0;
const MAX_VIEWPORT_SIDE: u32 = 32_768;
pub(crate) const COLOR: [f32; 3] = [0.8, 0.4, 0.0];
pub(crate) const VERTEX_BYTES: usize = (50 + 12) * 3 * 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CursorProfile {
    #[default]
    Studio,
    SourceBackground,
}
impl CursorProfile {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Studio => "Studio",
            Self::SourceBackground => "SculptGL background",
        }
    }
    fn caption(self) -> crate::ui_text::CaptionKey {
        match self {
            Self::Studio => crate::ui_text::CaptionKey::CursorStudio,
            Self::SourceBackground => crate::ui_text::CaptionKey::CursorSourceBackground,
        }
    }
}

pub(crate) fn render_profile(
    ui: &mut egui::Ui,
    profile: &mut CursorProfile,
) -> [(&'static str, egui::Response); 2] {
    ui.label(crate::ui_text::caption(
        crate::ui_text::CaptionKey::CursorStyle,
    ));
    let studio = ui.radio_value(
        profile,
        CursorProfile::Studio,
        crate::ui_text::caption(CursorProfile::Studio.caption()),
    );
    let source = ui
        .radio_value(
            profile,
            CursorProfile::SourceBackground,
            crate::ui_text::caption(CursorProfile::SourceBackground.caption()),
        )
        .on_hover_text(crate::ui_text::caption(
            crate::ui_text::CaptionKey::CursorEmptyScope,
        ));
    [
        ("viewport.cursor.studio", studio),
        ("viewport.cursor.source_background", source),
    ]
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerProvenance {
    pub logical_words: [u32; 2],
    pub viewport_words: [u32; 4],
    pub pixels_per_point_word: u32,
    pub serial: u64,
    pub kind: &'static str,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CursorPointer {
    pixels: [f32; 2],
    provenance: Option<PointerProvenance>,
    serial: u64,
    nonempty_input_seen: bool,
    rejected: Option<&'static str>,
    rejected_words: Option<[u32; 2]>,
}
impl CursorPointer {
    pub(crate) fn observe_egui(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        nonempty_document: bool,
        modal: bool,
    ) {
        let mut points = [None; 128];
        let mut count = 0;
        let mut overflow = false;
        ctx.input(|input| {
            for event in &input.events {
                let point = match *event {
                    egui::Event::PointerMoved(pos) => Some((pos, "egui PointerMoved f32")),
                    egui::Event::PointerButton {
                        pos, pressed: true, ..
                    } => Some((pos, "egui PointerButton down f32")),
                    _ => None,
                };
                if let Some(point) = point {
                    if let Some(slot) = points.get_mut(count) {
                        *slot = Some(point);
                        count += 1;
                    } else {
                        overflow = true;
                    }
                }
            }
        });
        if overflow {
            self.rejected = Some("Cursor frame pointer budget exceeded");
            return;
        }
        for (position, kind) in points.into_iter().take(count).flatten() {
            let owned = position.is_finite()
                && response.interact_rect.contains(position)
                && ctx.layer_id_at(position) == Some(response.layer_id)
                && !modal;
            self.observe(
                position,
                response.rect,
                ctx.pixels_per_point(),
                owned,
                nonempty_document,
                kind,
            );
        }
    }
    /// Only genuine moved/down events owned by the viewport reach conversion.
    /// Up, PointerGone, GUI input and idle publication retain the last position.
    pub(crate) fn observe(
        &mut self,
        position: egui::Pos2,
        rect: egui::Rect,
        pixels_per_point: f32,
        viewport_owned: bool,
        nonempty_document: bool,
        kind: &'static str,
    ) {
        if !position.is_finite() {
            self.rejected = Some("Nonfinite egui pointer input");
            self.rejected_words = Some([position.x.to_bits(), position.y.to_bits()]);
            return;
        }
        if !viewport_owned {
            return;
        }
        let local = (position - rect.min) * pixels_per_point;
        if !rect.is_finite()
            || !rect.is_positive()
            || !pixels_per_point.is_finite()
            || pixels_per_point <= 0.0
            || !local.is_finite()
            || local.abs().max_elem() > MAX_COORDINATE
        {
            self.rejected = Some("Invalid viewport pointer conversion");
            self.rejected_words = Some([position.x.to_bits(), position.y.to_bits()]);
            return;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.rejected = Some("Viewport pointer serial exhausted");
            return;
        };
        self.serial = serial;
        self.pixels = [local.x, local.y];
        self.provenance = Some(PointerProvenance {
            logical_words: [position.x.to_bits(), position.y.to_bits()],
            viewport_words: [
                rect.min.x.to_bits(),
                rect.min.y.to_bits(),
                rect.max.x.to_bits(),
                rect.max.y.to_bits(),
            ],
            pixels_per_point_word: pixels_per_point.to_bits(),
            serial,
            kind,
        });
        self.nonempty_input_seen |= nonempty_document;
        self.rejected = None;
        self.rejected_words = None;
    }

    pub(crate) fn capture(&self, scope: CursorScope) -> Result<CursorFrame, &'static str> {
        if let Some(error) = self.rejected {
            return Err(error);
        }
        if !scope.empty_document || !scope.empty_scene || !scope.no_pick {
            return Err("Surface cursor presentation is not implemented");
        }
        if self.nonempty_input_seen {
            return Err("Nonempty viewport input history is outside this subset");
        }
        if !scope.idle {
            return Err("Active cursor presentation is not implemented");
        }
        if !scope.screen_space {
            return Err("Enable screen-space radius for this cursor");
        }
        if scope.pixels_per_point != 1.0 || scope.zoom_factor != 1.0 {
            return Err("Cursor HiDPI and UI zoom are not verified");
        }
        if !scope.radius.is_finite() || !(5.0..=500.0).contains(&scope.radius) {
            return Err("Cursor radius must be finite and within 5..500");
        }
        if scope.viewport.contains(&0) || scope.viewport.iter().any(|n| *n > MAX_VIEWPORT_SIDE) {
            return Err("Cursor viewport exceeds bounded dimensions");
        }
        Ok(CursorFrame {
            viewport: scope.viewport,
            pointer_pixels: [f64::from(self.pixels[0]), f64::from(self.pixels[1])],
            radius_pixels: f64::from(scope.radius),
            symmetry: scope.symmetry,
            provenance: self.provenance,
            scope,
        })
    }

    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"transport":"genuine egui f32; f64 promotion is not recovered precision",
            "initial_state":self.provenance.is_none(),"pixels":self.pixels,
            "pixel_words":self.pixels.map(f32::to_bits),"nonempty_input_seen":self.nonempty_input_seen,
            "rejected":self.rejected,"rejected_words":self.rejected_words,
            "last_input":self.provenance.map(|p|serde_json::json!({"logical_words":p.logical_words,
                "viewport_words":p.viewport_words,"pixels_per_point_word":p.pixels_per_point_word,
                "serial":p.serial,"kind":p.kind,"viewport_owned":true}))})
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorScope {
    pub viewport: [u32; 2],
    pub radius: f32,
    pub pixels_per_point: f32,
    pub zoom_factor: f32,
    pub empty_document: bool,
    pub empty_scene: bool,
    pub no_pick: bool,
    pub idle: bool,
    pub screen_space: bool,
    pub symmetry: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorFrame {
    pub viewport: [u32; 2],
    pub pointer_pixels: [f64; 2],
    pub radius_pixels: f64,
    pub symmetry: bool,
    pub provenance: Option<PointerProvenance>,
    pub scope: CursorScope,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct CursorStyle {
    pub profile: CursorProfile,
    pub frame: Option<CursorFrame>,
    pub unavailable: Option<&'static str>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorMatrices {
    pub circle: [f32; 16],
    pub dot: [f32; 16],
    pub symmetry: [f32; 16],
}

/// The public source formula stores ortho/translate/scale intermediates as f32.
/// Matrix multiplication evaluates retained operands as scalar f64, then f32.
pub(crate) fn matrices(frame: CursorFrame) -> Result<CursorMatrices, &'static str> {
    if frame.viewport.contains(&0)
        || frame.viewport.iter().any(|n| *n > MAX_VIEWPORT_SIDE)
        || frame
            .pointer_pixels
            .iter()
            .any(|p| !p.is_finite() || p.abs() > f64::from(MAX_COORDINATE))
        || !frame.radius_pixels.is_finite()
        || !(5.0..=500.0).contains(&frame.radius_pixels)
    {
        return Err("Cursor matrix inputs are invalid");
    }
    let w = f64::from(frame.viewport[0]) * 0.5;
    let h = f64::from(frame.viewport[1]) * 0.5;
    let projection = [(1.0 / w) as f32, (1.0 / h) as f32, -0.1_f32];
    let translation = [
        (-w + frame.pointer_pixels[0]) as f32,
        (h - frame.pointer_pixels[1]) as f32,
    ];
    let scaled = |scale: f32| {
        let mut m = [0.0; 16];
        m[0] = (f64::from(projection[0]) * f64::from(scale)) as f32;
        m[5] = (f64::from(projection[1]) * f64::from(scale)) as f32;
        m[10] = (f64::from(projection[2]) * f64::from(scale)) as f32;
        m[12] = (f64::from(projection[0]) * f64::from(translation[0])) as f32;
        m[13] = (f64::from(projection[1]) * f64::from(translation[1])) as f32;
        m[15] = 1.0;
        m
    };
    let mut symmetry = [0.0; 16];
    symmetry[15] = 1.0;
    Ok(CursorMatrices {
        circle: scaled(frame.radius_pixels as f32),
        dot: scaled(50.0),
        symmetry,
    })
}

pub(crate) fn circle_vertices() -> [f32; 150] {
    let mut out = [0.0; 150];
    for i in 0..50 {
        let angle = (std::f64::consts::TAU * i as f64) / 50.0;
        out[i * 3] = angle.cos() as f32;
        out[i * 3 + 1] = angle.sin() as f32;
    }
    out
}
pub(crate) fn dot_vertices() -> [f32; 36] {
    let mut out = [0.0; 36];
    // Source's full fan: center0, ring indices1..11, closing at396 degrees.
    for i in 1..12 {
        let angle = (std::f64::consts::TAU * i as f64) / 10.0;
        out[i * 3] = (angle.cos() * 0.05) as f32;
        out[i * 3 + 1] = (angle.sin() * 0.05) as f32;
    }
    out
}
