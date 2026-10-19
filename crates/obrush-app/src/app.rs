use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use glam::Vec3;
use obrush_core::{
    AttributeExportPolicy, AttributeExportReport, BrushKind, BrushResult, BrushSettings,
    CylinderSettings, DecimateSettings, Document, Hit, LocalTransformStroke, MaskExtractReport,
    MaskExtractSelection, MaskExtractSettings, MaskFilterOperation, MaskFilterReport,
    MaskFilterSettings, MaskRegionReport, MaskRegionSettings, Mesh, MeshImportReport, MoveStroke,
    ObjectTransform, PaintFillReport, PaintPickInterpolation, PaintSample, PlyEncoding, Ray,
    RemeshSettings, StlEncoding, SubtoolMergePolicy, SubtoolMergeSettings, SubtoolRenderSnapshot,
    TopologyOperation, TorusSettings,
};

use crate::background_mask::{BackgroundMaskAction, BackgroundMaskGesture};
use crate::brush_preferences::{
    AlphaConversion, AlphaSelection, BrushPreferenceStore, ContactInput,
    Modifiers as PreferenceModifiers, PressureProfile, ScreenRadiusPreferences, ToolPreferences,
};
use crate::continuous_dab::{ContinuousDab, apply_continuous_dab};
use crate::continuous_host::{
    ClockPolicy, HostContinuousClock, Readiness, StopReason, StrokeKey, supports_continuous,
};
use crate::dab_queue::{DabQueue, DabTiming};
use crate::environment_assets::PRESETS as ENVIRONMENT_PRESETS;
use crate::environment_gl::BackgroundMode;
use crate::footprint::ScreenFootprint;
use crate::move_drag::MoveDrag;
use crate::navigation::{NavigationButton, NavigationCapture};
use crate::renderer::{Camera, CameraMode, Renderer, ShadingMode, ViewportStyle};
use crate::stroke::StrokeSampler;
use crate::uv_texture_request::UvTextureRequests;
use crate::uv_view::ViewTexture;
#[path = "inspector_layout.rs"]
mod inspector_layout;
#[path = "preferences_host.rs"]
mod preferences_host;
#[path = "sgl_export_host.rs"]
mod sgl_export_host;
#[path = "source_geometry_app.rs"]
mod source_geometry_app;
#[path = "source_geometry_ui.rs"]
mod source_geometry_ui;
#[path = "source_modal_host.rs"]
mod source_modal_host;
#[path = "source_sgl_add_host.rs"]
mod source_sgl_add_host;
#[path = "ui_hit_rect.rs"]
mod ui_hit_rect;
#[path = "uv_bake_host.rs"]
mod uv_bake_host;
use obrush_core::{LockedStamp, LockedStampReport, LockedStampSample};

#[path = "transform_drag.rs"]
mod transform_drag;
use transform_drag::{RotationDrag, ScaleDrag, TranslationDrag, TranslationHandle, project_world};

#[path = "mask_region_gesture.rs"]
mod mask_region_gesture;
use mask_region_gesture::{BrowserNavigationButtons, MaskRegionGesture, MaskSelectionMode};

struct HostMaskRegionReport {
    subtool_id: u32,
    revision: u64,
    captured_revision: u64,
    settings: MaskRegionSettings,
    report: MaskRegionReport,
    skipped_tiny_click: bool,
}

#[derive(PartialEq)]
struct RegionSettingsFingerprint {
    numbers: [f32; 26],
    switches: [bool; 21],
    integers: [u32; 3],
    layer: Option<usize>,
    layer_preview: Option<(usize, f32)>,
    drafts: [String; 2],
    pivot: TransformPivot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TransformPivot {
    #[default]
    Center,
    Origin,
}

#[derive(Clone, Debug)]
struct TransformValues {
    translation: Vec3,
    rotation_degrees: Vec3,
    scale: f32,
    axis_scale: Vec3,
    pivot: TransformPivot,
}

impl Default for TransformValues {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_degrees: Vec3::ZERO,
            scale: 1.0,
            axis_scale: Vec3::ONE,
            pivot: TransformPivot::Center,
        }
    }
}

struct HostMaskFilterReport {
    subtool_id: Option<u32>,
    revision: u64,
    report: MaskFilterReport,
}

struct HostPaintFillReport {
    subtool_id: Option<u32>,
    revision: u64,
    report: PaintFillReport,
}

struct HostPaintPickReport {
    subtool_id: Option<u32>,
    revision: u64,
    hit: Hit,
    sample: PaintSample,
}

/// Filters edit masks only, but still respect host gesture and job ownership.
fn mask_filter_unavailable_reason(
    document: &Document,
    topology_busy: bool,
    live_transform: bool,
) -> Option<&'static str> {
    if !document.active_subtool_visible() {
        Some("Show the active part to edit its mask.")
    } else if topology_busy {
        Some("Finish or cancel the surface operation before editing the mask.")
    } else if live_transform {
        Some("Release or cancel the transform drag before editing the mask.")
    } else {
        None
    }
}

fn mask_filter_after_barrier(
    document: &mut Document,
    settings: &MaskFilterSettings,
    stroke_rejected: bool,
    sculptgl_response: bool,
) -> Result<MaskFilterReport, String> {
    if stroke_rejected {
        return Err("The preceding shape edit was rejected; retry on the restored part.".into());
    }
    if sculptgl_response
        && matches!(
            settings.operation,
            MaskFilterOperation::Blur | MaskFilterOperation::Sharpen
        )
    {
        document.filter_mask_sculptgl(settings.operation)
    } else {
        document.filter_mask(settings)
    }
}

/// The native and browser command share the same complete document transaction.
/// A rolled-back material/layer edit must never be reported as an accepted Fill.
fn paint_fill_after_barrier(
    document: &mut Document,
    settings: &BrushSettings,
    stroke_rejected: bool,
) -> Result<PaintFillReport, String> {
    if stroke_rejected {
        return Err("The preceding shape edit was rejected; retry on the restored part.".into());
    }
    if !document.active_subtool_visible() {
        return Err("Show the active part before filling paint.".into());
    }
    if settings.paint_roughness || settings.paint_metallic {
        document.begin_material_paint_stroke();
    } else {
        document.begin_stroke();
    }
    let result = document
        .require_active_mesh_mut()
        .and_then(|mesh| mesh.fill_paint_channels(settings));
    document.end_stroke();
    if let Some(error) = document.take_layer_error() {
        return Err(error);
    }
    result
}

struct HostMaskExtractReport {
    source_subtool_id: Option<u32>,
    new_subtool_id: u32,
    revision: u64,
    report: MaskExtractReport,
}

/// Host command gates use cheap metadata only; extraction/validation stays core.
fn mask_extract_unavailable_reason(
    document: &Document,
    topology_busy: bool,
    live_transform: bool,
) -> Option<&'static str> {
    if !document.active_subtool_visible() {
        Some("Show the active part before extracting its mask.")
    } else if topology_busy {
        Some("Finish or cancel the surface operation before extracting.")
    } else if live_transform {
        Some("Release or cancel the transform drag before extracting.")
    } else if document.subtool_count() >= 32 {
        Some("Scenes support at most 32 parts. Delete a part before creating another.")
    } else if document
        .active_mesh()
        .is_some_and(|m| m.positions.len() > 250_000 || m.triangles.len() > 500_000)
    {
        Some(
            "Extraction supports up to 250K points and 500K faces. Choose a lower subdivision level or reduce the surface first.",
        )
    } else {
        None
    }
}

/// Only a successfully finished source stroke may reach the extraction core.
fn mask_extract_after_barrier_with_selection(
    document: &mut Document,
    settings: &MaskExtractSettings,
    selection: MaskExtractSelection,
    keep_source_active: bool,
    sculptgl_surface_offset: bool,
    stroke_rejected: bool,
) -> Result<(u32, MaskExtractReport), String> {
    if stroke_rejected {
        return Err(
            "The preceding shape edit was rejected; retry extraction on the restored part.".into(),
        );
    }
    document.extract_mask_as_subtool_with_surface_options(
        settings,
        selection,
        keep_source_active,
        sculptgl_surface_offset,
    )
}

#[derive(Clone)]
struct ActiveTransformDrag {
    gesture: TranslationDrag,
    scale_drag: Option<ScaleDrag>,
    rotation_drag: Option<RotationDrag>,
    kind: GizmoKind,
    pivot: Vec3,
    subtool_id: u32,
    selected_ids: Vec<u32>,
    mask_weighted: bool,
    revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GizmoKind {
    Translate,
    Plane,
    Scale,
    UniformScale,
    Rotate,
}

impl ActiveTransformDrag {
    fn rotation_degrees(&self) -> Vec3 {
        self.rotation_drag.as_ref().map_or(Vec3::ZERO, |drag| {
            self.gesture.handle.axis().unwrap_or(Vec3::ZERO) * drag.degrees
        })
    }

    fn axis_scale(&self) -> Vec3 {
        let Some(scale_drag) = self.scale_drag.as_ref() else {
            return Vec3::ONE;
        };
        if self.kind == GizmoKind::UniformScale {
            return Vec3::splat(scale_drag.factor);
        }
        self.gesture.handle.axis().map_or(Vec3::ONE, |axis| {
            Vec3::ONE + axis * (scale_drag.factor - 1.0)
        })
    }
}

/// Pure cancellation path shared by focus, Escape, mode toggle and reset.
/// It deliberately has no Document argument and cannot alter geometry/history.
fn clear_transform_preview(
    drag: &mut Option<ActiveTransformDrag>,
    release: &mut Option<Pos2>,
    preview: &mut Vec3,
    preview_scale: &mut Vec3,
    preview_rotation: &mut Vec3,
    preview_pivot: &mut Vec3,
) -> bool {
    let changed = drag.is_some()
        || *preview != Vec3::ZERO
        || *preview_scale != Vec3::ONE
        || *preview_rotation != Vec3::ZERO;
    *drag = None;
    *release = None;
    *preview = Vec3::ZERO;
    *preview_scale = Vec3::ONE;
    *preview_rotation = Vec3::ZERO;
    *preview_pivot = Vec3::ZERO;
    changed
}

#[derive(Clone)]
struct GizmoHandle {
    handle: TranslationHandle,
    kind: GizmoKind,
    rect: egui::Rect,
    start: Pos2,
    end: Pos2,
    enabled: bool,
    pivot: Vec3,
    length: f32,
    quad: Option<[Pos2; 4]>,
    arc: Option<Vec<Pos2>>,
}

impl GizmoHandle {
    fn key(&self) -> &'static str {
        match (self.kind, self.handle) {
            (GizmoKind::Scale, TranslationHandle::X) => "gizmo.scale_x",
            (GizmoKind::Scale, TranslationHandle::Y) => "gizmo.scale_y",
            (GizmoKind::Scale, TranslationHandle::Z) => "gizmo.scale_z",
            (GizmoKind::UniformScale, TranslationHandle::Free) => "gizmo.scale_uniform",
            (GizmoKind::Rotate, TranslationHandle::X) => "gizmo.rotate_x",
            (GizmoKind::Rotate, TranslationHandle::Y) => "gizmo.rotate_y",
            (GizmoKind::Rotate, TranslationHandle::Z) => "gizmo.rotate_z",
            _ => self.handle.key(),
        }
    }

    fn contains(&self, pointer: Pos2) -> bool {
        if let Some(arc) = &self.arc {
            arc.windows(2).any(|pair| {
                let segment = pair[1] - pair[0];
                let factor =
                    ((pointer - pair[0]).dot(segment) / segment.length_sq()).clamp(0.0, 1.0);
                pointer.distance(pair[0] + segment * factor) <= 7.0
            })
        } else if let Some(quad) = self.quad {
            let mut sign = 0.0_f32;
            for i in 0..4 {
                let edge = quad[(i + 1) % 4] - quad[i];
                let to_pointer = pointer - quad[i];
                let cross = edge.x * to_pointer.y - edge.y * to_pointer.x;
                if cross.abs() > 1.0e-4 {
                    if sign != 0.0 && cross.signum() != sign {
                        return false;
                    }
                    sign = cross.signum();
                }
            }
            true
        } else if self.kind == GizmoKind::UniformScale {
            let radius = self.start.distance(self.end);
            (self.start.distance(pointer) - radius).abs() <= 8.0
        } else {
            self.rect.contains(pointer)
        }
    }

    fn rotation_tangent(&self, pointer: Pos2) -> Option<egui::Vec2> {
        let arc = self.arc.as_ref()?;
        let segment = arc.windows(2).min_by(|a, b| {
            let distance = |pair: &[Pos2]| {
                let edge = pair[1] - pair[0];
                let t = ((pointer - pair[0]).dot(edge) / edge.length_sq()).clamp(0.0, 1.0);
                pointer.distance_sq(pair[0] + edge * t)
            };
            distance(a).total_cmp(&distance(b))
        })?;
        Some(segment[1] - segment[0])
    }
}

const ACCENT: Color32 = Color32::from_rgb(243, 137, 85);
const MUTED: Color32 = Color32::from_rgb(141, 147, 156);
const PANEL: Color32 = Color32::from_rgb(27, 29, 33);
const BRUSHES: [(BrushKind, &str, &str); 14] = [
    (BrushKind::Draw, "Draw", "1"),
    (BrushKind::Clay, "Clay", "2"),
    (BrushKind::Inflate, "Inflate", "3"),
    (BrushKind::Smooth, "Smooth", "4"),
    (BrushKind::Flatten, "Flatten", "5"),
    (BrushKind::Crease, "Crease", "6"),
    (BrushKind::Move, "Move", "7"),
    (BrushKind::Mask, "Mask", "8"),
    (BrushKind::Paint, "Paint", "9"),
    (BrushKind::Morph, "Morph", "0"),
    (BrushKind::Pinch, "Pinch", "P"),
    (BrushKind::Twist, "Twist", "J"),
    (BrushKind::LocalScale, "Local Scale", "L"),
    (BrushKind::Drag, "Drag", "G"),
];

struct ActiveStroke {
    settings: BrushSettings,
    screen_radius: ScreenRadiusPreferences,
    pressure_factors: Option<(f32, f32, f32, f32)>,
    last_screen: Option<Pos2>,
    sampler: StrokeSampler,
    movement: Option<ActiveMove>,
    local_transform: Option<ActiveLocalTransform>,
    locked_stamp: Option<ActiveLockedStamp>,
    kind: BrushKind,
    invert: bool,
    queued: bool,
    continuous: Option<StrokeKey>,
    source_paint: Option<crate::source_paint_host::SharedSourcePaint>,
    source_geometry: Option<crate::source_geometry_host::SharedSourceGeometry>,
    source_geometry_dirty_before: Option<bool>,
    source_paint_display_before: Option<(bool, bool)>,
    continuous_locked: Option<ActiveContinuousLocked>,
}

fn pressure_factors_for_stroke(
    preferences: &BrushPreferenceStore,
    kind: BrushKind,
) -> Option<(f32, f32, f32, f32)> {
    let PressureProfile::SculptGlFactors { intensity, radius } = preferences.global().pressure
    else {
        return None;
    };
    let tool = preferences.tool(kind);
    Some((tool.strength, tool.radius, intensity, radius))
}

struct ActiveContinuousLocked {
    gesture: crate::continuous_locked_host::ContinuousLockedGesture,
    dirty_before: bool,
    vertex_color_before: bool,
}

struct CapturedDab {
    ray: Ray,
    settings: BrushSettings,
    footprint: Option<ScreenFootprint>,
}

#[derive(Clone, Copy, Debug)]
struct BrowserPenMove {
    pos: Pos2,
    pressure: f32,
}

const MAX_BROWSER_PEN_MOVES: usize = 256;

#[derive(Default)]
struct BrowserPenSamples {
    queued: std::collections::VecDeque<BrowserPenMove>,
    dropped: u64,
    overflowed: bool,
}

impl BrowserPenSamples {
    fn push(&mut self, sample: BrowserPenMove) {
        if self.queued.len() == MAX_BROWSER_PEN_MOVES {
            self.queued.pop_front();
            self.dropped = self.dropped.saturating_add(1);
            self.overflowed = true;
        }
        self.queued.push_back(sample);
    }

    fn drain(&mut self) -> (std::collections::VecDeque<BrowserPenMove>, bool) {
        let overflowed = std::mem::take(&mut self.overflowed);
        (std::mem::take(&mut self.queued), overflowed)
    }

    fn clear(&mut self) {
        self.queued.clear();
        self.overflowed = false;
    }
}

fn browser_pen_ordinary_stroke(stroke: &ActiveStroke) -> bool {
    stroke.queued
        && stroke.continuous.is_none()
        && stroke.locked_stamp.is_none()
        && stroke.local_transform.is_none()
        && stroke.source_geometry.is_none()
        && stroke.source_paint.is_none()
        && !matches!(stroke.kind, BrushKind::Drag)
}

fn camera_ray_at(camera: &Camera, screen: Pos2, rect: egui::Rect) -> Ray {
    let ndc = glam::Vec2::new(
        2.0 * (screen.x - rect.left()) / rect.width() - 1.0,
        1.0 - 2.0 * (screen.y - rect.top()) / rect.height(),
    );
    camera.ray(ndc, rect.aspect_ratio())
}

fn captured_stroke_dab(
    stroke: &ActiveStroke,
    camera: &Camera,
    pos: Pos2,
    rect: egui::Rect,
    kind: BrushKind,
    invert: bool,
    input: ContactInput,
) -> Option<CapturedDab> {
    if !rect.contains(pos) {
        return None;
    }
    let screen_radius = stroke.screen_radius;
    let mut settings = stroke.settings.clone();
    let (contact, curve) = match input {
        ContactInput::Mouse => (1.0, 0.5),
        ContactInput::Pen(pressure) if pressure.is_finite() => {
            (pressure.clamp(0.0, 1.0), pressure.clamp(0.0, 1.0))
        }
        ContactInput::Pen(_) => return None,
    };
    let mut pixels = screen_radius.pixels;
    if let Some((base_strength, base_radius, intensity, radius)) = stroke.pressure_factors {
        let slope = curve * 2.0 - 1.0;
        settings.strength = base_strength * (1.0 + intensity * slope);
        settings.radius = base_radius * (1.0 + radius * slope);
        settings.pressure = if contact > 0.0 { 1.0 } else { 0.0 };
        settings.pressure_strength = false;
        settings.pressure_radius = false;
        pixels *= 1.0 + radius * slope;
    } else {
        settings.pressure = contact;
    }
    settings.alpha_tangent = Some(camera.axes().0);
    settings.kind = kind;
    settings.invert ^= invert;
    let ray = camera_ray_at(camera, pos, rect);
    settings.culling_axis = Some(-ray.direction);
    let footprint = screen_radius
        .enabled
        .then(|| ScreenFootprint::capture(camera, pixels, rect.height(), rect.aspect_ratio()))
        .flatten();
    if screen_radius.enabled && footprint.is_none() {
        return None;
    }
    Some(CapturedDab {
        ray,
        settings,
        footprint,
    })
}

fn local_settings(document: &Document, mut settings: BrushSettings) -> BrushSettings {
    if let Some(axis) = settings.alpha_tangent {
        settings.alpha_tangent = crate::scene_space::local_vector(document, axis).ok();
    }
    if let Some(axis) = settings.culling_axis {
        settings.culling_axis = crate::scene_space::local_vector(document, axis).ok();
    }
    settings
}

struct ActiveLockedStamp {
    stamp: LockedStamp,
    start: Pos2,
    viewport: egui::Rect,
    units_per_pixel: f32,
    tangent: Vec3,
    base_rotation: f32,
    dirty_before: bool,
    vertex_color_before: bool,
    paints_rgb: bool,
}

fn sample_locked_stamp(
    locked: &mut ActiveLockedStamp,
    mesh: &mut Mesh,
    point: Pos2,
    pressure: f32,
) -> Result<LockedStampReport, String> {
    let travel = point - locked.start;
    let radius = travel.length() * locked.units_per_pixel;
    let rotation = if travel == Vec2::ZERO {
        locked.base_rotation
    } else {
        locked.base_rotation - travel.y.atan2(travel.x)
    };
    locked.stamp.apply(
        mesh,
        LockedStampSample {
            radius,
            alpha_rotation: rotation,
            alpha_tangent: Some(locked.tangent),
            pressure,
        },
    )
}

struct HostLockedStampReport {
    subtool_id: Option<u32>,
    revision: u64,
    report: LockedStampReport,
}

struct ActiveLocalTransform {
    stroke: Option<LocalTransformStroke>,
    hit: Hit,
    axis: Vec3,
    selection: BrushSettings,
    start: Pos2,
    pivot_screen: Pos2,
    total_parameter: f32,
    last_pressure: f32,
    last_vector: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ExportFormat {
    #[default]
    Obj,
    StlAscii,
    StlBinary,
    PlyAscii,
    PlyBinary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SubdivisionChoice {
    #[default]
    Loop,
    StaticSmooth,
    StaticLinear,
}

impl SubdivisionChoice {
    fn name(self) -> &'static str {
        match self {
            Self::Loop => "Loop smooth",
            Self::StaticSmooth => "SculptGL static smooth",
            Self::StaticLinear => "SculptGL linear edges",
        }
    }
    fn static_settings(self) -> Option<obrush_core::StaticSubdivisionSettings> {
        match self {
            Self::Loop => None,
            Self::StaticSmooth => Some(obrush_core::StaticSubdivisionSettings {
                linear_edges: false,
            }),
            Self::StaticLinear => {
                Some(obrush_core::StaticSubdivisionSettings { linear_edges: true })
            }
        }
    }
}

#[derive(Clone, Copy)]
enum PrimitiveKind {
    Sphere,
    Cube,
    Cylinder,
    Torus,
}

impl PrimitiveKind {
    fn name(self) -> &'static str {
        match self {
            Self::Sphere => "Sphere",
            Self::Cube => "Cube",
            Self::Cylinder => "Cylinder",
            Self::Torus => "Torus",
        }
    }

    fn create(self, sphere_detail: u32) -> Result<Mesh, String> {
        let detail = sphere_detail.min(7);
        let mut mesh = match self {
            Self::Sphere => Ok(Mesh::sphere(detail)),
            Self::Cube => Ok(Mesh::cube()),
            Self::Cylinder => Mesh::cylinder(&CylinderSettings::default()),
            Self::Torus => Mesh::torus(&TorusSettings::default()),
        }?;
        // Use the requested editable density for every generated solid. The
        // existing cube starts with four refinement steps, substantially fewer
        // samples than the high-detail sphere. Linear refinement keeps the
        // dimensions and the chosen primitive shape.
        let target_triangles = 12 * 4_usize.pow(detail);
        while mesh.triangles.len() < target_triangles {
            mesh.subdivide()?;
        }
        Ok(mesh)
    }

    fn create_for_scene(self, detail: u32, source_profile: bool) -> Result<Mesh, String> {
        if source_profile && detail == 6 {
            return match self {
                Self::Sphere => Mesh::sphere_quad_smooth(7),
                Self::Cube => Mesh::cube_quad_grid(7),
                Self::Cylinder => Mesh::source_default_cylinder(),
                Self::Torus => Mesh::source_default_torus(),
            };
        }
        self.create(detail)
    }
}

impl ExportFormat {
    fn scene_format(
        self,
        source_obj: Option<obrush_core::ObjColorOptions>,
    ) -> obrush_core::SceneGeometryFormat {
        match self {
            Self::Obj => source_obj.map_or(
                obrush_core::SceneGeometryFormat::Obj,
                obrush_core::SceneGeometryFormat::ObjSculptGl,
            ),
            Self::StlAscii => obrush_core::SceneGeometryFormat::Stl(StlEncoding::Ascii),
            Self::StlBinary => source_obj.map_or(
                obrush_core::SceneGeometryFormat::Stl(StlEncoding::Binary),
                |_| obrush_core::SceneGeometryFormat::StlSculptGl,
            ),
            Self::PlyAscii => obrush_core::SceneGeometryFormat::Ply(PlyEncoding::Ascii),
            Self::PlyBinary => source_obj.map_or(
                obrush_core::SceneGeometryFormat::Ply(PlyEncoding::BinaryLittleEndian),
                |_| obrush_core::SceneGeometryFormat::PlySculptGl,
            ),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Obj => "OBJ",
            Self::StlAscii => "STL text",
            Self::StlBinary => "STL binary + color",
            Self::PlyAscii => "PLY text",
            Self::PlyBinary => "PLY binary",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Obj => "obj",
            Self::StlAscii | Self::StlBinary => "stl",
            Self::PlyAscii | Self::PlyBinary => "ply",
        }
    }
}

struct ActiveMove {
    stroke: Option<MoveStroke>,
    screen: MoveDrag,
    hit: Hit,
    selection: BrushSettings,
    last_pressure: f32,
    start: Pos2,
    units_per_pixel: f32,
}

struct TopologyJob {
    id: u32,
    revision: u64,
    started: web_time::Instant,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: std::sync::mpsc::Receiver<Result<TopologyWorkerResult, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

enum TopologyWorkerResult {
    #[cfg(not(target_arch = "wasm32"))]
    Mesh(Box<Mesh>, Box<obrush_core::TopologyReport>),
    Project(String),
}

fn decode_topology_project(
    contents: &str,
) -> Result<(Document, obrush_core::TopologyReport), String> {
    if contents.len() > 256 * 1024 * 1024 {
        return Err("Topology result exceeds the size limit".into());
    }
    let value: serde_json::Value = serde_json::from_str(contents).map_err(|e| e.to_string())?;
    let project = value
        .get("project")
        .and_then(|v| v.as_str())
        .ok_or("Missing topology project")?;
    let document = Document::from_json(project)?;
    let report = value.get("report").ok_or("Missing topology report")?;
    let report = match report.get("kind").and_then(|v| v.as_str()) {
        Some("remesh") => obrush_core::TopologyReport::Remesh(
            serde_json::from_value(report["report"].clone()).map_err(|e| e.to_string())?,
        ),
        Some("remesh_selected") => obrush_core::TopologyReport::RemeshSelected(
            serde_json::from_value(report["report"].clone()).map_err(|e| e.to_string())?,
        ),
        Some("decimate") => obrush_core::TopologyReport::Decimate(
            serde_json::from_value(report["report"].clone()).map_err(|e| e.to_string())?,
        ),
        _ => return Err("Unknown topology report".into()),
    };
    Ok((document, report))
}

#[derive(Clone, Copy)]
struct AlphaImportRequest {
    id: u32,
    destination: BrushKind,
    conversion: AlphaConversion,
}

fn keyboard_input_observation(
    ctx: &egui::Context,
    phase: &'static str,
) -> Option<serde_json::Value> {
    let frame = ctx.cumulative_frame_nr();
    ctx.input(|input| {
        let interesting = input.events.iter().any(|event| matches!(event,
            egui::Event::Key {..} | egui::Event::PointerButton {..}));
        if !interesting { return None; }
        let keys = input.events.iter().filter_map(|event| match event {
            egui::Event::Key {key,pressed,repeat,modifiers,..} => Some(serde_json::json!({"key":format!("{key:?}"),"pressed":pressed,"repeat":repeat,"shift":modifiers.shift})),
            _ => None,
        }).take(16).collect::<Vec<_>>();
        let buttons = input.events.iter().filter_map(|event| match event {
            egui::Event::PointerButton {pos,button,pressed,..} => Some(serde_json::json!({"button":format!("{button:?}"),"pressed":pressed,"position":[pos.x,pos.y]})),
            _ => None,
        }).take(16).collect::<Vec<_>>();
        Some(serde_json::json!({"phase":phase,"frame":frame,"input_time":input.time,"input_focused":input.focused,
            "keys":keys,"buttons":buttons,"text_event_count":input.events.iter().filter(|event|matches!(event,egui::Event::Text(_))).count(),
            "bracket_pressed":input.key_pressed(egui::Key::OpenBracket),"command":input.modifiers.command,"shift":input.modifiers.shift,
            "primary_down":input.pointer.primary_down(),"primary_released":input.pointer.primary_released()}))
    })
}

fn initial_brush_settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        invert: false,
        symmetry_x: true,
        color: [0.72, 0.34, 0.22],
        ..Default::default()
    }
}
fn temporary_modifiers(selected: BrushKind, effective: BrushKind) -> PreferenceModifiers {
    PreferenceModifiers {
        shift: effective == BrushKind::Smooth && selected != effective,
        control: effective == BrushKind::Mask && selected != effective,
        alt: false,
    }
}
fn preference_view_candidate(
    store: &BrushPreferenceStore,
    brush: &BrushSettings,
    screen: bool,
    pixels: f32,
    locked: bool,
) -> Result<Option<BrushPreferenceStore>, String> {
    let kind = store.selected();
    if brush.kind != kind {
        return Err("Select brushes through their saved preference record".into());
    }
    let old = store.tool(kind);
    let mut prefs = ToolPreferences::from_settings(brush);
    prefs.alpha = old.alpha;
    prefs.adaptive_detail = old.adaptive_detail;
    prefs.screen_space_radius = screen;
    prefs.screen_radius_pixels = pixels;
    prefs.lock_position = locked;
    let mut global = store.global();
    global.symmetry_x = brush.symmetry_x;
    global.symmetry_offset = brush.symmetry_offset;
    if matches!(global.pressure, PressureProfile::LegacyBinary { .. }) {
        global.pressure = PressureProfile::LegacyBinary {
            strength: brush.pressure_strength,
            radius: brush.pressure_radius,
        };
    }
    if &prefs == old && global == store.global() {
        return Ok(None);
    }
    let mut candidate = store.clone();
    candidate.set_tool(kind, prefs)?;
    candidate.set_global(global)?;
    Ok(Some(candidate))
}
fn alpha_import_candidate(
    store: &BrushPreferenceStore,
    destination: BrushKind,
    conversion: AlphaConversion,
    name: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<BrushPreferenceStore, String> {
    let mut candidate = store.clone();
    candidate.import_rgba(destination, name, width, height, rgba, conversion)?;
    Ok(candidate)
}

fn alpha_image_import_candidate(
    store: &BrushPreferenceStore,
    request: Option<AlphaImportRequest>,
    id: u32,
    name: &str,
    bytes: &[u8],
) -> Result<BrushPreferenceStore, String> {
    let request = request
        .filter(|request| request.id == id)
        .ok_or("Alpha import request is stale")?;
    let decoded = crate::alpha_decode::decode_alpha_image(bytes)?;
    alpha_import_candidate(
        store,
        request.destination,
        request.conversion,
        name,
        decoded.width,
        decoded.height,
        &decoded.rgba,
    )
}

fn alpha_selection_name(store: &BrushPreferenceStore, id: AlphaSelection) -> Option<String> {
    match id {
        AlphaSelection::None => None,
        AlphaSelection::Square => Some("Square".into()),
        AlphaSelection::Skin => Some("Skin".into()),
        AlphaSelection::Imported(id) => store
            .library()
            .entries()
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.name.clone()),
    }
}

pub struct OBrushApp {
    egui_ctx: egui::Context,
    app_preferences: crate::app_preferences::AppPreferences,
    ui_fonts: crate::ui_fonts::UiFonts,
    shortcut_resolver: crate::shortcut_profile::ShortcutResolver,
    show_app_preferences: bool,
    app_preference_controls: BTreeMap<String, [f32; 4]>,
    locale_response_observations: BTreeMap<String, serde_json::Value>,
    preferences_window_observation: Option<[f32; 4]>,
    last_locale_barrier_observation: Option<serde_json::Value>,
    startup_source: Option<crate::startup_options::SourceStartupSettings>,
    startup_gaps: Vec<String>,
    source_boot_framing_pending: bool,
    startup_model_pending: Option<String>,
    startup_models: crate::startup_model::ModelRequests,
    #[cfg(not(target_arch = "wasm32"))]
    native_startup_model: Option<crate::startup_model::NativeModelJob>,
    document: Document,
    resident_studio: crate::resident_studio::ResidentStudioBridge,
    brush: BrushSettings,
    preferences: BrushPreferenceStore,
    alpha_conversion: AlphaConversion,
    alpha_import_request: Option<AlphaImportRequest>,
    next_alpha_import_id: u32,
    brush_screen_space: bool,
    brush_screen_radius: f32,
    screen_radius_editor: crate::screen_radius_ui::ScreenRadiusDraft,
    screen_radius_hit_rect: ui_hit_rect::HitRectDiagnostics,
    brush_lock_position: bool,
    last_stamp_report: Option<HostLockedStampReport>,
    camera: Camera,
    viewport_style: ViewportStyle,
    background_cursor_pointer: crate::background_cursor::CursorPointer,
    uv_texture_requests: UvTextureRequests,
    background_requests: UvTextureRequests,
    renderer: Arc<Mutex<Renderer>>,
    rendered_scene: Vec<SubtoolRenderSnapshot>,
    revision: u64,
    snapshot_revision: u64,
    render_snapshot_timing: DabTiming,
    render_certified_handoffs: u64,
    app_update_timing: DabTiming,
    app_panel_timing: DabTiming,
    app_viewport_timing: DabTiming,
    stroke: Option<ActiveStroke>,
    background_mask_gesture: Option<BackgroundMaskGesture>,
    pending_dabs: DabQueue,
    browser_pen_samples: BrowserPenSamples,
    keyboard_diagnostics: std::collections::VecDeque<serde_json::Value>,
    source_keyboard_camera: preferences_host::SourceKeyboardCamera,
    continuous_clock: HostContinuousClock,
    continuous_origin: web_time::Instant,
    next_continuous_serial: u64,
    last_continuous_tick: Option<u64>,
    unprocessed_release: Option<(Pos2, egui::Modifiers, f32)>,
    message: String,
    dirty: bool,
    stroke_rejected: bool,
    show_help: bool,
    inspector_open: bool,
    brush_radius_px: f32,
    last_viewport: egui::Rect,
    primitive_detail: u32,
    subdivision_choice: SubdivisionChoice,
    topology_job: Option<TopologyJob>,
    uv_bake_job: Option<uv_bake_host::UvBakeHostJob>,
    next_uv_bake_id: u32,
    uv_bake_size: obrush_core::UvBakeSize,
    last_uv_bake_report: Option<obrush_core::UvBakeReport>,
    next_job_id: u32,
    remesh_resolution: u32,
    remesh_block_mode: bool,
    remesh_surface_nets_mode: bool,
    remesh_mc_relax: bool,
    remesh_discard_uvs: bool,
    remesh_flatten_selected: bool,
    remesh_bake_selected_poses: bool,
    decimate_ratio: f32,
    alpha_name: Option<String>,
    paint_pick_armed: bool,
    paint_pick_dragging: bool,
    last_paint_sample: Option<HostPaintPickReport>,
    last_paint_fill: Option<HostPaintFillReport>,
    source_paint_preferences: crate::source_paint_host::SourcePaintPreferences,
    source_geometry_preferences: source_geometry_app::GeometryPreferenceStore,
    source_geometry_alpha_tracks: crate::source_geometry_host::SharedSourceGeometryAlphaTracks,
    source_geometry_editor_drafts: [source_geometry_ui::GeometryEditorDrafts; 7],
    source_geometry_controls_hit_rect: ui_hit_rect::HitRectDiagnostics,
    last_source_geometry_report: Option<serde_json::Value>,
    source_paint_editor_drafts: crate::source_paint_ui::SourcePaintEditorDrafts,
    source_paint_controls_hit_rect: ui_hit_rect::HitRectDiagnostics,
    source_paint_raw: crate::source_paint_host::RawPaintInput,
    source_modal: source_modal_host::HostModal,
    last_source_paint_report: Option<serde_json::Value>,
    last_continuous_locked_report: Option<serde_json::Value>,
    source_mask_preview: Option<(
        u64,
        Option<u32>,
        usize,
        obrush_core::MaskHierarchyConversionReport,
    )>,
    source_allow_mask_loss: bool,
    contact_pressure: crate::pressure::ContactPressure,
    last_input_pen: bool,
    selected_layer: Option<usize>,
    layer_name_index: Option<usize>,
    layer_name_draft: String,
    layer_name_committed: String,
    layer_intensity_preview: Option<(usize, f32)>,
    layer_controls: BTreeMap<String, [f32; 4]>,
    layer_controls_enabled: BTreeMap<String, bool>,
    layer_controls_hit_rect: ui_hit_rect::HitRectDiagnostics,
    morph_controls: BTreeMap<String, [f32; 4]>,
    morph_controls_enabled: BTreeMap<String, bool>,
    scene_controls: BTreeMap<String, [f32; 4]>,
    scene_controls_enabled: BTreeMap<String, bool>,
    scene_clip_rect: [f32; 4],
    subtool_name_id: Option<u32>,
    subtool_name_draft: String,
    subtool_name_committed: String,
    ui_subtool_id: Option<u32>,
    pending_append: bool,
    source_sgl_import: crate::source_sgl_import::SourceSglImportState,
    transform_mode: bool,
    transform_values: TransformValues,
    transform_drag: Option<ActiveTransformDrag>,
    transform_respect_masks: bool,
    unprocessed_transform_release: Option<Pos2>,
    transform_controls: BTreeMap<String, [f32; 4]>,
    transform_controls_enabled: BTreeMap<String, bool>,
    transform_clip_rect: [f32; 4],
    gizmo_handles: Vec<GizmoHandle>,
    gizmo_clip_rect: [f32; 4],
    extract_settings: MaskExtractSettings,
    extract_whole_faces: bool,
    extract_keep_source_active: bool,
    extract_sculptgl_surface_offset: bool,
    last_extract_report: Option<HostMaskExtractReport>,
    extract_controls: BTreeMap<String, [f32; 4]>,
    extract_controls_enabled: BTreeMap<String, bool>,
    extract_clip_rect: [f32; 4],
    mask_filter_settings: MaskFilterSettings,
    mask_filter_sculptgl_response: bool,
    last_mask_filter_report: Option<HostMaskFilterReport>,
    mask_filter_controls: BTreeMap<String, [f32; 4]>,
    mask_filter_controls_enabled: BTreeMap<String, bool>,
    mask_filter_clip_rect: [f32; 4],
    mask_selection_mode: MaskSelectionMode,
    mask_region_gesture: Option<MaskRegionGesture>,
    region_suppressed_frame: bool,
    last_mask_region_report: Option<HostMaskRegionReport>,
    last_import_report: Option<MeshImportReport>,
    export_format: ExportFormat,
    obj_export_zbrush_color: bool,
    obj_export_append_rgb: bool,
    export_allow_attribute_loss: bool,
    last_export_report: Option<AttributeExportReport>,
    last_scene_geometry_report: Option<obrush_core::SceneGeometryReport>,
    camera_navigation_active: bool,
    merge_selection: std::collections::BTreeSet<u32>,
    merge_flatten: bool,
    mask_region_controls: BTreeMap<String, [f32; 4]>,
    mask_region_controls_enabled: BTreeMap<String, bool>,
    mask_region_clip_rect: [f32; 4],
    mask_region_cancel_reason: Option<String>,
    pending_sgl_export: Option<sgl_export_host::PendingSglExport>,
    sgl_export_selected_only: bool,
    allow_sgl_conversions: bool,
    last_sgl_export_report: Option<obrush_core::SglExportReport>,
    browser_navigation: BrowserNavigationButtons,
    navigation_capture: NavigationCapture,
    /// Source navigation is resolved before publishing the one input/render frame.
    source_navigation_prepared: bool,
    source_frame_revision: Option<u64>,
    /// Normal numeric commands wait for this frame's measured viewport.
    source_scale_command: Option<(bool, f64)>,
    browser_navigation_coordinates: bool,
    native_navigation_buttons: u16,
}

impl OBrushApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let startup = crate::startup_options::StartupOptions::capture(
            crate::app_preferences::PreferenceProfile::OBrush,
            "",
            None,
        )
        .map_err(std::io::Error::other)?;
        Self::new_with_startup(cc, startup)
    }
    pub fn new_with_startup(
        cc: &eframe::CreationContext<'_>,
        startup: crate::startup_options::StartupOptions,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut app = Self::new_legacy(cc)?;
        app.apply_startup(startup).map_err(std::io::Error::other)?;
        Ok(app)
    }
    fn new_legacy(
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        configure_theme(&cc.egui_ctx);
        let gl = cc
            .gl
            .as_ref()
            .ok_or("OBrush requires the OpenGL/WebGL renderer")?;
        let renderer = Renderer::new(gl).map_err(std::io::Error::other)?;
        let mesh = Mesh::sphere(6);
        let document = Document::new(mesh);
        let brush = initial_brush_settings();
        let preferences = BrushPreferenceStore::from_legacy(&brush, None, false)
            .map_err(std::io::Error::other)?;
        Ok(Self {
            egui_ctx: cc.egui_ctx.clone(),
            app_preferences: Default::default(),
            ui_fonts: Default::default(),
            shortcut_resolver: crate::shortcut_profile::ShortcutResolver::new(
                crate::app_preferences::PreferenceProfile::OBrush,
            ),
            show_app_preferences: false,
            app_preference_controls: BTreeMap::new(),
            locale_response_observations: BTreeMap::new(),
            preferences_window_observation: None,
            last_locale_barrier_observation: None,
            startup_source: None,
            startup_gaps: Vec::new(),
            source_boot_framing_pending: false,
            startup_model_pending: None,
            startup_models: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            native_startup_model: None,
            rendered_scene: document.subtool_render_snapshots(),
            ui_subtool_id: document.active_subtool_id(),
            document,
            resident_studio: Default::default(),
            brush,
            preferences,
            alpha_conversion: AlphaConversion::LegacyWeightedOpacity,
            alpha_import_request: None,
            next_alpha_import_id: 1,
            brush_screen_space: false,
            brush_screen_radius: 75.0,
            screen_radius_editor: Default::default(),
            screen_radius_hit_rect: Default::default(),
            brush_lock_position: false,
            background_cursor_pointer: Default::default(),
            last_stamp_report: None,
            camera: Camera::default(),
            viewport_style: ViewportStyle {
                color: [0.61, 0.38, 0.28],
                wireframe: false,
                show_mask: true,
                use_vertex_color: false,
                antialias: true,
                active_preview_translation: Vec3::ZERO,
                preview_subtool_ids: Vec::new(),
                ..Default::default()
            },
            uv_texture_requests: UvTextureRequests::default(),
            background_requests: UvTextureRequests::default(),
            renderer: Arc::new(Mutex::new(renderer)),
            revision: 1,
            snapshot_revision: 1,
            render_snapshot_timing: DabTiming::default(),
            render_certified_handoffs: 0,
            app_update_timing: DabTiming::default(),
            app_panel_timing: DabTiming::default(),
            app_viewport_timing: DabTiming::default(),
            stroke: None,
            background_mask_gesture: None,
            pending_dabs: DabQueue::default(),
            browser_pen_samples: BrowserPenSamples::default(),
            keyboard_diagnostics: std::collections::VecDeque::with_capacity(32),
            source_keyboard_camera: Default::default(),
            continuous_clock: HostContinuousClock::new(ClockPolicy::default())
                .map_err(std::io::Error::other)?,
            continuous_origin: web_time::Instant::now(),
            next_continuous_serial: 1,
            last_continuous_tick: None,
            unprocessed_release: None,
            message: "Ready to sculpt. Drag on the surface to begin.".into(),
            dirty: false,
            stroke_rejected: false,
            show_help: false,
            inspector_open: false,
            brush_radius_px: 48.0,
            last_viewport: egui::Rect::NOTHING,
            primitive_detail: 6,
            subdivision_choice: SubdivisionChoice::Loop,
            topology_job: None,
            uv_bake_job: None,
            next_uv_bake_id: 0,
            uv_bake_size: obrush_core::UvBakeSize::default(),
            last_uv_bake_report: None,
            next_job_id: 0,
            remesh_resolution: 64,
            remesh_block_mode: false,
            remesh_surface_nets_mode: false,
            remesh_mc_relax: true,
            remesh_discard_uvs: false,
            remesh_flatten_selected: false,
            remesh_bake_selected_poses: false,
            decimate_ratio: 0.5,
            alpha_name: None,
            paint_pick_armed: false,
            paint_pick_dragging: false,
            last_paint_sample: None,
            last_paint_fill: None,
            source_paint_preferences: Default::default(),
            source_geometry_preferences: Default::default(),
            source_geometry_alpha_tracks: Arc::new(Mutex::new(Default::default())),
            source_geometry_editor_drafts: std::array::from_fn(|_| Default::default()),
            source_geometry_controls_hit_rect: Default::default(),
            last_source_geometry_report: None,
            source_paint_editor_drafts: Default::default(),
            source_paint_controls_hit_rect: Default::default(),
            source_paint_raw: Default::default(),
            source_modal: Default::default(),
            last_source_paint_report: None,
            last_continuous_locked_report: None,
            source_mask_preview: None,
            source_allow_mask_loss: false,
            contact_pressure: Default::default(),
            last_input_pen: false,
            selected_layer: None,
            layer_name_index: None,
            layer_name_draft: String::new(),
            layer_name_committed: String::new(),
            layer_intensity_preview: None,
            layer_controls: BTreeMap::new(),
            layer_controls_enabled: BTreeMap::new(),
            layer_controls_hit_rect: Default::default(),
            morph_controls: BTreeMap::new(),
            morph_controls_enabled: BTreeMap::new(),
            scene_controls: BTreeMap::new(),
            scene_controls_enabled: BTreeMap::new(),
            scene_clip_rect: [0.0; 4],
            subtool_name_id: None,
            subtool_name_draft: String::new(),
            subtool_name_committed: String::new(),
            pending_append: false,
            source_sgl_import: crate::source_sgl_import::SourceSglImportState::default(),
            transform_mode: false,
            transform_values: TransformValues::default(),
            transform_drag: None,
            transform_respect_masks: false,
            unprocessed_transform_release: None,
            transform_controls: BTreeMap::new(),
            transform_controls_enabled: BTreeMap::new(),
            transform_clip_rect: [0.0; 4],
            gizmo_handles: Vec::new(),
            gizmo_clip_rect: [0.0; 4],
            extract_settings: MaskExtractSettings::default(),
            extract_whole_faces: false,
            extract_keep_source_active: false,
            extract_sculptgl_surface_offset: false,
            last_extract_report: None,
            extract_controls: BTreeMap::new(),
            extract_controls_enabled: BTreeMap::new(),
            extract_clip_rect: [0.0; 4],
            mask_filter_settings: MaskFilterSettings::default(),
            mask_filter_sculptgl_response: false,
            last_mask_filter_report: None,
            mask_filter_controls: BTreeMap::new(),
            mask_filter_controls_enabled: BTreeMap::new(),
            mask_filter_clip_rect: [0.0; 4],
            mask_selection_mode: MaskSelectionMode::default(),
            mask_region_gesture: None,
            region_suppressed_frame: false,
            last_mask_region_report: None,
            last_import_report: None,
            export_format: ExportFormat::default(),
            obj_export_zbrush_color: true,
            obj_export_append_rgb: false,
            export_allow_attribute_loss: false,
            last_export_report: None,
            last_scene_geometry_report: None,
            camera_navigation_active: false,
            merge_selection: std::collections::BTreeSet::new(),
            merge_flatten: false,
            mask_region_controls: BTreeMap::new(),
            mask_region_controls_enabled: BTreeMap::new(),
            mask_region_clip_rect: [0.0; 4],
            mask_region_cancel_reason: None,
            pending_sgl_export: None,
            sgl_export_selected_only: false,
            allow_sgl_conversions: false,
            last_sgl_export_report: None,
            browser_navigation: BrowserNavigationButtons::default(),
            navigation_capture: NavigationCapture::default(),
            source_navigation_prepared: false,
            source_frame_revision: None,
            source_scale_command: None,
            browser_navigation_coordinates: false,
            native_navigation_buttons: 0,
        })
    }

    pub fn sculpt_metrics(&self) -> String {
        let mut metrics = self.pending_dabs.metrics();
        metrics["browser_pen_moves_queued"] = self.browser_pen_samples.queued.len().into();
        metrics["browser_pen_moves_dropped"] = self.browser_pen_samples.dropped.into();
        metrics["browser_pen_moves_overflowed"] = self.browser_pen_samples.overflowed.into();
        metrics["render_snapshot_timing"] = self.render_snapshot_timing.metrics();
        metrics["render_certified_handoffs"] = self.render_certified_handoffs.into();
        metrics["app_update_timing"] = serde_json::json!({
            "total": self.app_update_timing.metrics(),
            "before_viewport": self.app_panel_timing.metrics(),
            "viewport": self.app_viewport_timing.metrics(),
            "scope": "CPU wall time inside eframe App::update; excludes host paint and browser RAF overhead",
        });
        metrics["paint_timing"] = self
            .renderer
            .lock()
            .ok()
            .map_or(serde_json::Value::Null, |renderer| {
                renderer.sculpt_paint_timing()
            });
        metrics.to_string()
    }

    fn refresh_rendered_scene(&mut self) {
        if self.snapshot_revision != self.revision {
            let started = web_time::Instant::now();
            self.rendered_scene = self.document.subtool_render_snapshots();
            self.render_snapshot_timing.record(started.elapsed());
            self.snapshot_revision = self.revision;
        }
    }

    pub fn brush_metrics(&self) -> String {
        let layers: Vec<_> = self.document.layer_summaries().into_iter().map(|layer| {
            serde_json::json!({"index":layer.index,"name":layer.name,"intensity":layer.intensity,
                "visible":layer.visible,"recording":layer.recording,"has_geometry":layer.has_geometry})
        }).collect();
        let morph_target = self.document.morph_target_summary().map(|target| {
            serde_json::json!({"level":target.creation_level,"creation_level":target.creation_level,
                "vertices":target.vertices,"triangles":target.triangles,"available":target.available})
        });
        let subtools: Vec<_> = self
            .document
            .subtool_summaries()
            .into_iter()
            .map(|object| {
                serde_json::json!({"id":object.id,"name":object.name,"visible":object.visible,
                "effective_visible":object.effective_visible,"active":object.active,
                "vertices":object.vertices,"triangles":object.triangles})
            })
            .collect();
        let uploads = self.renderer.lock().ok().map(|renderer| {
            renderer.subtool_upload_stats().into_iter().map(|(id,vertices,triangles,uploads)|
                serde_json::json!({"id":id,"vertices":vertices,"triangles":triangles,"uploads":uploads})
            ).collect::<Vec<_>>()
        }).unwrap_or_default();
        let scene_error = self
            .renderer
            .lock()
            .ok()
            .and_then(|renderer| renderer.scene_error().map(str::to_owned));
        let continuous = self.continuous_clock.report();
        let source_paint = serde_json::json!({"mode":format!("{:?}",self.source_paint_preferences.mode),
            "scope":"active-part ordinary/timer; U; UIzoom1; alphaNone; staticLock preview/timer; no mirror/inverse/rotation; camera/DPR/pressure parity unverified",
            "camera_policy":"OBrush precise view64 with captured projection32",
            "raw_pressure":self.source_paint_raw.sample().ok().map(|raw|serde_json::json!({"value":raw.pressure,"pen":raw.pen,"browser_number":raw.browser_number,"absent":raw.absent})),
            "active":self.stroke.as_ref().and_then(|s|s.source_paint.as_ref()).and_then(|s|s.lock().ok()).map(|s|s.diagnostics()),
            "last_completed":self.last_source_paint_report,
            "retained_normal_state":self.document.active_subtool_id().and_then(|id|self.document.source_geometry_state(id)).map(|state|obrush_core::SourcePaintNormalView::RetainedGeometry(&state).report())});

        let pressure_profile = match self.preferences.global().pressure {
            PressureProfile::LegacyBinary { strength, radius } => {
                serde_json::json!({"kind":"legacy","strength":strength,"radius":radius})
            }
            PressureProfile::SculptGlFactors { intensity, radius } => {
                serde_json::json!({"kind":"sculptgl_factors","intensity":intensity,"radius":radius})
            }
        };
        let mut metrics = serde_json::json!({
            "continuous":{"requested":self.preferences.global().continuous,"eligible":supports_continuous(self.brush.kind,false),
                "locked_conflict":self.preferences.global().continuous && self.brush_lock_position,
                "active":continuous.active_key.is_some(),"active_serial":continuous.active_key.map(|k|k.serial),
                "subtool_id":continuous.active_key.map(|k|k.subtool_id),"consumed_tick_permissions":continuous.consumed_ticks,
                "skipped_polls":continuous.skipped_polls,"expired_permits":continuous.expired_permits,
                "outstanding_permit":continuous.outstanding_permit,"last_stop":continuous.last_stop.map(|s|format!("{s:?}")),
                "last_tick_micros":self.last_continuous_tick,"period_micros":16_600,"pause_gap_micros":100_000,
                "timer_queue_tail":0,"continuous_locked_supported":true},
            "pressure":self.brush.pressure,"pressure_strength":self.brush.pressure_strength,
            "pressure_radius":self.brush.pressure_radius,"adaptive_detail":self.brush.adaptive_detail,
            "kind":self.brush.kind.name(),"radius":self.brush.radius,"strength":self.brush.strength,
            "symmetry_x":self.brush.symmetry_x,"alpha":self.alpha_name,
            "alpha_width":self.brush.alpha.as_ref().map(|alpha|alpha.width()),
            "alpha_height":self.brush.alpha.as_ref().map(|alpha|alpha.height()),
            "alpha_rotation":self.brush.alpha_rotation,"topology_busy":self.topology_job.is_some(),"uv_bake":self.uv_bake_metrics(),
            "topology_job_id":self.topology_job.as_ref().map(|job|job.id),
            "vertices":self.document.active_mesh().map_or(0,|m|m.positions.len()),"triangles":self.document.active_mesh().map_or(0,|m|m.triangles.len()),
            "subdivision_level":self.document.current_level(),"subdivision_levels":self.document.level_count(),
            "layers":layers,"layer_count":self.document.layer_count(),
            "recording_layer":self.document.recording_layer(),
            "layers_locked_topology":self.document.layer_topology_locked(),
            "selected_layer":self.selected_layer,"layer_controls":self.layer_controls,
            "layer_controls_enabled":self.layer_controls_enabled,
            "morph_target":morph_target,"morph_controls":self.morph_controls,
            "morph_controls_enabled":self.morph_controls_enabled,"message":self.message,
            "subtools":subtools,"active_subtool_id":self.document.active_subtool_id(),
            "sgl_export":{"pending_report":self.pending_sgl_export.as_ref().map(|pending|&pending.report),"pending_requires_allow":self.pending_sgl_export.as_ref().map(|pending|pending.report.requires_allow()),"selected_only":self.sgl_export_selected_only,"allow_requested":self.allow_sgl_conversions,"last_report":self.last_sgl_export_report},
            "solo":self.document.solo(),"active_subtool_visible":self.document.active_subtool_visible(),
            "scene_controls":self.scene_controls,"scene_controls_enabled":self.scene_controls_enabled,
            "scene_clip_rect":self.scene_clip_rect,
            "subtool_uploads":uploads,"scene_error":scene_error
        });
        if let Some(metrics) = metrics.as_object_mut() {
            metrics.insert("pressure_profile".into(), pressure_profile);
            metrics.insert("input_is_pen".into(), self.last_input_pen.into());
            metrics.insert(
                "faces".into(),
                self.document
                    .active_mesh()
                    .map_or(0, Mesh::face_count)
                    .into(),
            );
        }
        // Keep diagnostic values unchanged without pushing the combined json!
        // invocation above its compiler macro-recursion bound.
        metrics["keyboard_diagnostics"] = serde_json::json!(self.keyboard_diagnostics);
        metrics["selected_subtool_ids"] = serde_json::json!(self.document.selected_subtool_ids());
        metrics["obj_export_options"] = serde_json::json!({"zbrush":self.obj_export_zbrush_color,"append_rgb":self.obj_export_append_rgb});
        metrics["last_scene_geometry_export"] = self.last_scene_geometry_report.as_ref().map_or(serde_json::Value::Null, |report| serde_json::json!({
            "parts":report.parts,"vertices":report.vertices,"triangles":report.triangles,
            "baked_pose_parts":report.baked_pose_parts,"omitted_source_origin_parts":report.omitted_source_origin_parts,
            "omitted_color_vertices":report.omitted_color_vertices,
            "quantized_color_vertices":report.quantized_color_vertices,
            "quantized_material_vertices":report.quantized_material_vertices,
            "quantized_mask_vertices":report.quantized_mask_vertices,
            "maximum_color_error":report.maximum_color_error,
            "maximum_material_error":report.maximum_material_error,
            "maximum_mask_error":report.maximum_mask_error
        }));
        metrics["layer_controls_hit_rect"] =
            serde_json::json!(self.layer_controls_hit_rect.entries());
        metrics["layer_controls_hit_rect_truncated"] =
            serde_json::json!(self.layer_controls_hit_rect.truncated());
        metrics["screen_radius_numeric"] = self.screen_radius_editor.diagnostics();
        metrics["screen_radius_controls_hit_rect"] =
            serde_json::json!(self.screen_radius_hit_rect.entries());
        metrics["source_paint"] = source_paint;
        metrics["source_modals"] = self.source_modal.observation.clone();
        metrics["source_geometry"] = self.source_geometry_diagnostics();
        metrics["source_geometry_controls_hit_rect"] =
            serde_json::to_value(self.source_geometry_controls_hit_rect.entries())
                .unwrap_or(serde_json::Value::Null);
        metrics["source_geometry_controls_focus"] =
            serde_json::json!(self.source_geometry_controls_hit_rect.focus_entries());
        metrics["source_geometry_controls_hit_rect_truncated"] =
            self.source_geometry_controls_hit_rect.truncated().into();
        metrics["source_paint_controls_hit_rect"] =
            serde_json::to_value(self.source_paint_controls_hit_rect.entries())
                .unwrap_or(serde_json::Value::Null);
        metrics["source_paint_controls_hit_rect_truncated"] =
            serde_json::Value::Bool(self.source_paint_controls_hit_rect.truncated());
        metrics["source_paint_cursor"] = self.source_modal.cursor_observation.clone();
        metrics["background_cursor"] = serde_json::json!({
            "profile":self.viewport_style.background_cursor.profile.name(),
            "pointer":self.background_cursor_pointer.diagnostics(),
            "host_frame":self.viewport_style.background_cursor.frame.map(|f|serde_json::json!({
                "viewport":f.viewport,"pointer_pixels":f.pointer_pixels,"radius_pixels":f.radius_pixels,
                "symmetry":f.symmetry,"input_serial":f.provenance.map(|p|p.serial),
                "scope":{"empty_document":f.scope.empty_document,"empty_scene":f.scope.empty_scene,
                    "no_pick":f.scope.no_pick,"idle":f.scope.idle,"screen_space":f.scope.screen_space,
                    "pixels_per_point":f.scope.pixels_per_point,"zoom_factor":f.scope.zoom_factor,
                    "radius_word":f.scope.radius.to_bits()}})),
            "unavailable":self.viewport_style.background_cursor.unavailable,
            "gpu":self.renderer.lock().ok().map(|r|r.background_cursor_diagnostics())});
        metrics["continuous"]["active_mode"] =
            serde_json::json!(continuous.active_mode.map(|m| format!("{m:?}")));
        metrics["continuous"]["continuous_locked_supported"] = serde_json::json!(true);
        metrics["continuous"]["locked_conflict"] = serde_json::json!(
            self.preferences.global().continuous
                && self.brush_lock_position
                && self.continuous_locked_blocker(self.brush.kind).is_err()
        );
        metrics["continuous_locked"] = serde_json::json!({"scope":"9 ordinary legacy tools; fixed topology; precise f64 anchor/current camera and declared f32 core conversions; UIzoom1",
            "active":self.stroke.as_ref().and_then(|s|s.continuous_locked.as_ref()).map(|s|s.gesture.diagnostics()),"last_completed":self.last_continuous_locked_report});
        let extra = serde_json::json!({
            "dirty":self.dirty,"document_revision":self.revision,"snapshot_revision":self.snapshot_revision,
            "transform_controls":self.transform_controls,"transform_controls_enabled":self.transform_controls_enabled,
            "transform_clip_rect":self.transform_clip_rect,
            "transform_settings":{
                "translation":self.transform_values.translation.to_array(),
                "rotation_degrees":self.transform_values.rotation_degrees.to_array(),
                "scale":self.transform_values.scale,
                "axis_scale":self.transform_values.axis_scale.to_array(),
                "pivot_kind":if self.transform_values.pivot==TransformPivot::Center {"center"} else {"origin"},
                "pivot":self.transform_pivot().to_array(),"mode":self.transform_mode,
                "preview_translation":self.viewport_style.active_preview_translation.to_array(),
                "preview_scale":self.viewport_style.active_preview_scale.to_array(),
                "preview_rotation_degrees":self.viewport_style.active_preview_rotation.to_array(),
                "drag_rotation_tangent":self.transform_drag.as_ref().and_then(|drag|drag.rotation_drag.as_ref()).map(|drag|drag.tangent()),
                "preview_pivot":self.viewport_style.active_preview_pivot.to_array(),
                "preview_subtool_ids":self.transform_drag.as_ref().map_or_else(Vec::new,|drag|drag.selected_ids.clone()),
                "respect_masks":self.transform_respect_masks,
                "dragging":self.transform_drag.is_some(),
                "drag_handle":self.transform_drag.as_ref().map(|drag|match (drag.kind,drag.gesture.handle) {
                    (GizmoKind::Scale, TranslationHandle::X)=>"gizmo.scale_x",
                    (GizmoKind::Scale, TranslationHandle::Y)=>"gizmo.scale_y",
                    (GizmoKind::Scale, TranslationHandle::Z)=>"gizmo.scale_z",
                    (GizmoKind::UniformScale, TranslationHandle::Free)=>"gizmo.scale_uniform",
                    (GizmoKind::Rotate, TranslationHandle::X)=>"gizmo.rotate_x",
                    (GizmoKind::Rotate, TranslationHandle::Y)=>"gizmo.rotate_y",
                    (GizmoKind::Rotate, TranslationHandle::Z)=>"gizmo.rotate_z",
                    _=>drag.gesture.handle.key()})
            },
            "gizmo_controls":self.gizmo_handles.iter().map(|handle|(handle.key(),[handle.rect.min.x,handle.rect.min.y,handle.rect.max.x,handle.rect.max.y])).collect::<BTreeMap<_,_>>(),
            "gizmo_controls_enabled":self.gizmo_handles.iter().map(|handle|(handle.key(),handle.enabled)).collect::<BTreeMap<_,_>>(),
            "gizmo_clip_rect":self.gizmo_clip_rect,
            "gizmo_data":self.gizmo_handles.iter().map(|handle|serde_json::json!({"key":handle.key(),
                "pivot":handle.pivot.to_array(),"axis":handle.handle.axis().map(|axis|axis.to_array()),"world_length":handle.length,
                "screen_start":[handle.start.x,handle.start.y],"screen_end":[handle.end.x,handle.end.y],
                "screen_tangent":handle.rotation_tangent(handle.end).map(|t|[t.x,t.y]),"enabled":handle.enabled})).collect::<Vec<_>>(),
            "source_sgl_import":self.source_sgl_import.metrics(),"import_report":self.last_import_report,"export_format":self.export_format.name(),
            "subdivision_mode":self.subdivision_choice.name(),"remesh_block_mode":self.remesh_block_mode,"remesh_resolution":self.remesh_resolution,
            "merge_selection":self.merge_selection,"merge_flatten":self.merge_flatten,
            "camera":{"source_framing":self.camera.source_diagnostics(),"source_frame_revision":self.source_frame_revision,"source_scale_pending":self.source_scale_command,"precise_target":self.camera.precise_target.map(|p|p.to_array()),"position64":self.camera.position64().to_array(),"view64":self.camera.view64().to_cols_array(),"projection":self.camera.projection(self.last_viewport.aspect_ratio().max(0.001)).to_cols_array(),"source_vertical_fov":self.camera.source_vertical_fov,"yaw":self.camera.yaw,"pitch":self.camera.pitch,"distance":self.camera.distance,"target":self.camera.target.to_array(),
                "orthographic":self.camera.orthographic,"fov_degrees":self.camera.fov_degrees,"mode":self.camera.mode.name(),"speed":self.camera.speed,"use_pivot":self.camera.use_pivot,
                "position":self.camera.position().to_array(),"viewport":[self.last_viewport.min.x,self.last_viewport.min.y,self.last_viewport.max.x,self.last_viewport.max.y],
                "view_projection":self.camera.view_projection(self.last_viewport.aspect_ratio().max(0.001)).to_cols_array()}
        });
        if let (Some(metrics), serde_json::Value::Object(extra)) = (metrics.as_object_mut(), extra)
        {
            metrics.extend(extra);
        }
        metrics["remesh_surface_nets_mode"] = serde_json::json!(self.remesh_surface_nets_mode);
        metrics["remesh_mc_relax"] = serde_json::json!(self.remesh_mc_relax);
        metrics["remesh_discard_uvs"] = serde_json::json!(self.remesh_discard_uvs);
        metrics["remesh_flatten_selected"] = serde_json::json!(self.remesh_flatten_selected);
        metrics["remesh_bake_selected_poses"] = serde_json::json!(self.remesh_bake_selected_poses);
        let brush_options = serde_json::json!({
            "lock_position":self.brush_lock_position,
            "locked_stamp_report":self.last_stamp_report.as_ref().map(|record|serde_json::json!({
                "subtool_id":record.subtool_id,"document_revision":record.revision,"report":record.report,
            })),
            "screen_space_radius":self.brush_screen_space,"screen_radius_pixels":self.brush_screen_radius,
            "hardness":self.brush.hardness,"backface_culling":self.brush.backface_culling,
            "smooth_tangent":self.brush.smooth_tangent,"draw_clay":self.brush.draw_clay,
            "draw_accumulate":self.brush.draw_accumulate,"move_topology_check":self.brush.move_topology_check,
            "move_along_normal":self.brush.move_along_normal,"symmetry_offset":self.brush.symmetry_offset,
            "paint_color":self.brush.color,"invert":self.brush.invert,
            "paint_fill":self.last_paint_fill.as_ref().map(|r|serde_json::json!({"subtool_id":r.subtool_id,
                "revision":r.revision,"changed_vertices":r.report.changed_vertices,
                "changed_rgb_vertices":r.report.changed_rgb_vertices,"changed_material_vertices":r.report.changed_material_vertices,
                "materialized":r.report.materialized})),
            "paint_picker":{"armed":self.paint_pick_armed,"dragging":self.paint_pick_dragging,
                "sample":self.last_paint_sample.as_ref().map(|r| {let p=r.sample; serde_json::json!({
                    "subtool_id":r.subtool_id,"revision":r.revision,"rgb":p.rgb,"roughness":p.roughness,
                    "metallic":p.metallic,"triangle_index":p.triangle_index,"original_face_index":p.original_face_index,
                    "hit_position":r.hit.position.to_array(),"triangle":r.hit.triangle,"barycentric":r.hit.barycentric,
                    "interpolation":"original_face_inverse_distance"})})},
            "paint_rgb":self.brush.paint_rgb,"paint_roughness":self.brush.paint_roughness,
            "paint_metallic":self.brush.paint_metallic,"roughness":self.brush.roughness,"metallic":self.brush.metallic,
            "local_transform_parameter":self.stroke.as_ref().and_then(|stroke|stroke.local_transform.as_ref())
                .and_then(|local|local.stroke.as_ref()).map(LocalTransformStroke::accepted_parameter),
        });
        if let (Some(metrics), serde_json::Value::Object(options)) =
            (metrics.as_object_mut(), brush_options)
        {
            metrics.extend(options);
        }
        let extract_report=self.last_extract_report.as_ref().map(|record| {
            let report=&record.report;
            serde_json::json!({
                "source_subtool_id":record.source_subtool_id,"new_subtool_id":record.new_subtool_id,"document_revision":record.revision,
                "input_vertices":report.input_vertices,"input_triangles":report.input_triangles,
                "selected_source_triangles":report.selected_source_triangles,"patch_vertices":report.patch_vertices,"patch_triangles":report.patch_triangles,
                "patch_components":report.patch_components,"boundary_edges":report.boundary_edges,"boundary_loops":report.boundary_loops,
                "output_vertices":report.output_vertices,"output_triangles":report.output_triangles,
                "threshold":report.threshold,"thickness":report.thickness
            })
        });
        let extraction = serde_json::json!({
            "extract_controls":self.extract_controls,"extract_controls_enabled":self.extract_controls_enabled,"extract_clip_rect":self.extract_clip_rect,
            "extract_settings":{"threshold":self.extract_settings.threshold,"thickness":self.extract_settings.thickness,"selection":if self.extract_whole_faces {"whole_faces"} else {"clipped"},"keep_source_active":self.extract_keep_source_active,"sculptgl_surface_offset":self.extract_sculptgl_surface_offset},
            "extract_report":extract_report
        });
        if let (Some(metrics), serde_json::Value::Object(extraction)) =
            (metrics.as_object_mut(), extraction)
        {
            metrics.extend(extraction);
        }
        let filter_report=self.last_mask_filter_report.as_ref().map(|record| {
            let report=&record.report;
            serde_json::json!({
                "subtool_id":record.subtool_id,"document_revision":record.revision,
                "operation":report.operation.name(),"iterations":report.iterations,"strength":report.strength,
                "vertices":report.vertices,"changed_vertices":report.changed_vertices,"max_abs_change":report.max_abs_change
            })
        });
        let environment_metrics = serde_json::json!({
            "display_flags":{"show_grid":self.viewport_style.source_display.show_grid,"show_contour":self.viewport_style.source_display.show_contour,"contour_color":self.viewport_style.source_display.contour_color,"show_symmetry_line":self.viewport_style.source_display.show_symmetry_line,"guide_profile":self.viewport_style.source_display.guide_profile.name(),"brush_symmetry_enabled":self.brush.symmetry_x,"brush_offset":self.brush.symmetry_offset,"selected_ids":self.document.selected_subtool_ids(),"guide_scope":"View only; active Source-object plane uses normalized per-object offset and retained loose tree bounds where available; camera and brush parity remain separate"},
            "environment":{"preset":self.viewport_style.environment.preset,"filmic":self.viewport_style.environment.filmic,"darken_unselected":self.viewport_style.environment.darken_unselected,"custom_background":self.viewport_style.environment.custom_background,"background_mode":self.viewport_style.environment.background_mode.name(),"background_fill":self.viewport_style.environment.background_fill,"background_blur":self.viewport_style.environment.background_blur,"image":self.viewport_style.environment.background_image.as_ref().map(|t|serde_json::json!({"name":t.name,"width":t.width,"height":t.height,"bytes":t.pixels_bottom_up.len()})),"pending_request":self.background_requests.pending(),"gpu":self.renderer.lock().ok().map(|r|r.environment_diagnostics())},
        });
        if let (Some(target), serde_json::Value::Object(environment)) =
            (metrics.as_object_mut(), environment_metrics)
        {
            target.extend(environment);
        }
        let filters = serde_json::json!({
            "ui_zoom_factor":self.egui_ctx.zoom_factor(),
            "viewport_resolution_scale":self.viewport_style.resolution_scale,
            "viewport_resolution":self.renderer.lock().ok().map(|r| { let (pixels,error)=r.resolution_status(); serde_json::json!({"pixels":pixels,"error":error}) }),
            "history":{"limit":self.document.history_limit(),"undo":self.document.history_depths().0,"redo":self.document.history_depths().1},
            "viewport_shading":self.viewport_style.shading.name(),"flat_shading":self.viewport_style.flat_shading,"exposure":self.viewport_style.exposure,
            "curvature":self.viewport_style.curvature,"opacity":self.viewport_style.opacity,
            "uv_texture_pending_request":self.uv_texture_requests.pending(),
            "uv_texture":self.viewport_style.uv_texture.as_ref().map(|t|serde_json::json!({"name":t.name,"width":t.width,"height":t.height,"rgba_bytes":t.pixels_bottom_up.len(),"source_rows":"bottom-up upload; PNG input decoded top-down"})),
            "uv_render":self.renderer.lock().ok().map(|r|r.uv_render_metrics(false)),
            "surface_attributes":{"material_vertices":self.document.active_mesh().and_then(|m|m.materials.as_ref()).map_or(0,Vec::len),
                "original_faces":self.document.active_mesh().and_then(|m|m.surface.as_ref()).map_or(0,|s|s.faces.len()),
                "uv_coordinates":self.document.active_mesh().and_then(|m|m.surface.as_ref()).and_then(|s| s.uv.as_ref()).map_or(0, |uv| uv.coordinates.len()),"mask_convention":self.document.active_mesh().map(|m|m.mask_convention),"adaptive_detail_supported":self.document.supports_adaptive_detail()},
            "export_allow_attribute_loss":self.export_allow_attribute_loss,
            "export_report":self.last_export_report.as_ref().map(|r|serde_json::json!({"omitted_material_vertices":r.omitted_material_vertices,"omitted_source_mask_vertices":r.omitted_source_mask_vertices,
                "omitted_original_faces":r.omitted_original_faces,"omitted_uv_coordinates":r.omitted_uv_coordinates,"omitted_uv_faces":r.omitted_uv_faces})),
            "navigation_skipped_segments":self.navigation_capture.skipped_segments(),
            "mask_filter_controls":self.mask_filter_controls,"mask_filter_controls_enabled":self.mask_filter_controls_enabled,"mask_filter_clip_rect":self.mask_filter_clip_rect,
            "mask_filter_settings":{"iterations":self.mask_filter_settings.iterations,"strength":self.mask_filter_settings.strength,"sculptgl_response":self.mask_filter_sculptgl_response},
            "mask_filter_report":filter_report
        });
        if let (Some(metrics), serde_json::Value::Object(filters)) =
            (metrics.as_object_mut(), filters)
        {
            metrics.extend(filters);
        }
        let region_gesture = self.mask_region_gesture.as_ref().map(|gesture| {
            serde_json::json!({
                "mode":gesture.mode.name(),"erase":gesture.erase,"subtool_id":gesture.subtool_id,
                "captured_document_revision":gesture.revision,
                "view_projection":gesture.view_projection,"viewport":gesture.settings().viewport,
                "start":[gesture.start.x,gesture.start.y],"end":[gesture.end.x,gesture.end.y],
                "points":gesture.points.iter().map(|point|[point.x,point.y]).collect::<Vec<_>>(),
                "point_count":gesture.points.len(),"max_distance_from_press":gesture.max_travel_sq.sqrt()
            })
        });
        let region_report = self.last_mask_region_report.as_ref().map(|record| {
            let report = &record.report;
            serde_json::json!({
                "subtool_id":record.subtool_id,"document_revision":record.revision,
                "captured_document_revision":record.captured_revision,
                "view_projection":record.settings.view_projection,"viewport":record.settings.viewport,
                "region":record.settings.region,"kind":report.kind.name(),"erase":report.erase,
                "vertices":report.vertices,"projected_vertices":report.projected_vertices,
                "selected_vertices":report.selected_vertices,"changed_vertices":report.changed_vertices,
                "polygon_points":report.polygon_points,"point_edge_work":report.point_edge_work,
                "max_abs_change":report.max_abs_change,"skipped_tiny_click":record.skipped_tiny_click
            })
        });
        let regions = serde_json::json!({
            "mask_region_controls":self.mask_region_controls,"mask_region_controls_enabled":self.mask_region_controls_enabled,
            "mask_region_clip_rect":self.mask_region_clip_rect,
            "mask_region_settings":{"mode":self.mask_selection_mode.name(),"through_depth":true,
                "point_limit":obrush_core::MAX_MASK_REGION_POINTS,"point_edge_work_limit":obrush_core::MAX_MASK_REGION_WORK,
                "tiny_click_max_distance":1.0},
            "mask_region_gesture":region_gesture,
            "mask_region_state":{"active":self.mask_region_gesture.is_some(),
                "suppressed_this_frame":self.region_suppressed_frame,"cancel_reason":self.mask_region_cancel_reason},
            "mask_region_report":region_report
        });
        if let (Some(metrics), serde_json::Value::Object(regions)) =
            (metrics.as_object_mut(), regions)
        {
            metrics.extend(regions);
        }
        let modifiers = self.egui_ctx.input(|i| i.modifiers);
        let hover=self.preferences.preview(PreferenceModifiers {shift:modifiers.shift,control:modifiers.ctrl,alt:modifiers.alt},self.current_contact_input(),None,None)
            .ok().map(|p|serde_json::json!({"kind":p.settings.kind.name(),"radius":p.settings.radius,"effective_world_radius":p.settings.effective_radius(),"pressure":p.settings.pressure,"screen_space_radius":p.screen_radius.enabled,"screen_radius_pixels":p.screen_radius.base_pixels,"effective_screen_pixels":p.screen_radius.effective_pixels,"alpha_width":p.settings.alpha.as_ref().map(|a|a.width()),"alpha_height":p.settings.alpha.as_ref().map(|a|a.height())}));
        metrics["preferences"] = serde_json::json!({
            "hover":hover,
            "revision":self.preferences.revision(),"selected":self.preferences.selected().name(),
            "continuous_integrated":true,"continuous_locked_supported":true,
            "alpha_conversion":match self.alpha_conversion {AlphaConversion::LegacyWeightedOpacity=>"weighted_opacity",AlphaConversion::SculptGlMeanRgb=>"mean_rgb"},
            "alpha_import_request":self.alpha_import_request.map(|r|serde_json::json!({"id":r.id,"destination":r.destination.name(),"conversion":match r.conversion {AlphaConversion::LegacyWeightedOpacity=>"weighted_opacity",AlphaConversion::SculptGlMeanRgb=>"mean_rgb"}})),
            "alpha_bytes":self.preferences.library().imported_bytes(),
            "alpha_entries":self.preferences.library().entries().iter().map(|e|serde_json::json!({"id":e.id,"name":e.name,"conversion":match e.conversion {AlphaConversion::LegacyWeightedOpacity=>"weighted_opacity",AlphaConversion::SculptGlMeanRgb=>"mean_rgb"}})).collect::<Vec<_>>(),
            "tools":BrushKind::ALL.into_iter().map(|kind| {let p=self.preferences.tool(kind);serde_json::json!({"kind":kind.name(),"color":p.color,"invert":p.invert,"radius":p.radius,"strength":p.strength,"screen_space_radius":p.screen_space_radius,"screen_radius_pixels":p.screen_radius_pixels,"lock_position":p.lock_position,"alpha":format!("{:?}",p.alpha),"paint_rgb":p.paint_rgb,"paint_roughness":p.paint_roughness,"paint_metallic":p.paint_metallic,"roughness":p.roughness,"metallic":p.metallic})}).collect::<Vec<_>>()
        });
        metrics["preferences"]["continuous_locked_supported"] = serde_json::json!(true);
        metrics["app_preferences"] = self.app_preferences_metrics();
        metrics.to_string()
    }

    pub fn set_source_paint_input(
        &mut self,
        pressure: f64,
        pen: bool,
        released: bool,
        x: f64,
        y: f64,
        cancelled: bool,
    ) {
        self.source_paint_raw
            .receive(pressure, pen, released, true, Some([x, y]));
        if cancelled
            && self
                .stroke
                .as_ref()
                .is_some_and(|s| s.continuous_locked.is_some())
        {
            self.unprocessed_release = None;
            self.cancel_continuous_locked();
        }
        if cancelled
            && self
                .stroke
                .as_ref()
                .is_some_and(|s| s.source_paint.is_some() || s.source_geometry.is_some())
        {
            self.interrupt_source_paint_tail("Source Paint pointer was cancelled");
            self.stop_continuous(StopReason::NoContact);
            self.unprocessed_release = None;
            self.finish_stroke_deferred();
        }
    }
    pub fn set_input_pressure(&mut self, pressure: f32, pen: bool, released: bool) {
        self.last_input_pen = pen;
        self.source_paint_raw
            .receive(f64::from(pressure), pen, released, false, None);
        if pen {
            if released {
                self.contact_pressure.release();
            } else {
                self.contact_pressure.contact(pressure);
            }
        } else {
            self.contact_pressure.mouse();
        }
        self.brush.pressure = self.contact_pressure.current();
        self.egui_ctx.request_repaint();
    }

    /// Retain ordered held pen positions with their own pressure until the
    /// next frame. Egui's pointer state exposes only the final position of a
    /// same-frame burst; the bounded queue keeps intermediate brush dabs from
    /// inheriting the final event's pressure.
    pub fn queue_browser_pen_move(&mut self, pressure: f32, x: f32, y: f32) {
        if !self
            .stroke
            .as_ref()
            .is_some_and(browser_pen_ordinary_stroke)
        {
            return;
        }
        if !pressure.is_finite()
            || !(0.0..=1.0).contains(&pressure)
            || !x.is_finite()
            || !y.is_finite()
            || x.abs() > 100_000.0
            || y.abs() > 100_000.0
        {
            return;
        }
        self.browser_pen_samples.push(BrowserPenMove {
            pos: Pos2::new(x, y),
            pressure,
        });
        self.egui_ctx.request_repaint();
    }

    fn current_contact_input(&self) -> ContactInput {
        if self.last_input_pen {
            ContactInput::Pen(self.brush.pressure)
        } else {
            ContactInput::Mouse
        }
    }

    /// Production browser input bridge for navigation chords omitted by the
    /// web runner's button-down mapping. It never changes document contents.
    pub fn set_navigation_buttons(&mut self, buttons: u16) {
        self.browser_navigation.update(buttons);
        self.egui_ctx.request_repaint();
    }

    fn load_brush_view(&mut self) {
        let pressure = self.brush.pressure;
        let kind = self.preferences.selected();
        let prefs = self.preferences.tool(kind);
        self.brush_screen_space = prefs.screen_space_radius;
        self.brush_screen_radius = prefs.screen_radius_pixels;
        self.brush_lock_position = prefs.lock_position;
        if let Ok(sample) = self.preferences.preview(
            PreferenceModifiers::default(),
            if self.last_input_pen {
                ContactInput::Pen(pressure)
            } else {
                ContactInput::Mouse
            },
            None,
            None,
        ) {
            self.brush = sample.settings;
            // SculptGL factors belong to the captured input sample. Keep the
            // inspector's editable values at their unscaled tool defaults so
            // a later unrelated setting change cannot bake pen pressure into
            // the saved radius or strength.
            if matches!(
                self.preferences.global().pressure,
                PressureProfile::SculptGlFactors { .. }
            ) {
                self.brush.radius = prefs.radius;
                self.brush.strength = prefs.strength;
            }
            self.brush.adaptive_detail &= self.adaptive_detail_available();
        }
        self.alpha_name = alpha_selection_name(&self.preferences, prefs.alpha);
        self.screen_radius_editor.cancel(
            &self.egui_ctx,
            self.brush.kind,
            self.brush_screen_radius,
            self.brush_screen_space,
        );
    }

    fn adaptive_detail_available(&self) -> bool {
        self.document.supports_adaptive_detail()
    }

    fn commit_brush_view(&mut self) -> bool {
        let candidate = match preference_view_candidate(
            &self.preferences,
            &self.brush,
            self.brush_screen_space,
            self.brush_screen_radius,
            self.brush_lock_position,
        ) {
            Ok(candidate) => candidate,
            Err(error) => {
                self.load_brush_view();
                self.message = format!("Brush settings rejected: {error}");
                return false;
            }
        };
        let Some(candidate) = candidate else {
            return true;
        };
        // Actual stroke settings remain captured while the mutable UI is a candidate.
        // The old release endpoint/queue/history finish before replacement.
        self.finish_stroke();
        if self.stroke_rejected {
            self.load_brush_view();
            return false;
        }
        self.preferences = candidate;
        self.load_brush_view();
        true
    }

    fn select_brush(&mut self, kind: BrushKind) -> bool {
        if kind != self.brush.kind {
            self.modal_close_picker("UI tool changed");
        }
        if !self.commit_brush_view() {
            return false;
        }
        let mut candidate = self.preferences.clone();
        if candidate.selected() == kind {
            return true;
        }
        if let Err(error) = candidate.set_selected(kind) {
            self.message = error;
            return false;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return false;
        }
        self.paint_pick_armed = false;
        self.paint_pick_dragging = false;
        self.preferences = candidate;
        self.load_brush_view();
        true
    }

    fn select_alpha(&mut self, selection: AlphaSelection) {
        if !self.commit_brush_view() {
            return;
        }
        let mut candidate = self.preferences.clone();
        let kind = candidate.selected();
        let mut prefs = candidate.tool(kind).clone();
        if prefs.alpha == selection {
            return;
        }
        prefs.alpha = selection;
        if let Err(error) = candidate.set_tool(kind, prefs) {
            self.message = error;
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.preferences = candidate;
        self.load_brush_view();
    }

    fn remove_selected_alpha(&mut self) {
        if !self.commit_brush_view() {
            return;
        }
        let AlphaSelection::Imported(id) = self.preferences.tool(self.preferences.selected()).alpha
        else {
            return;
        };
        let mut candidate = self.preferences.clone();
        if let Err(error) = candidate.remove_alpha(id) {
            self.message = error;
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.preferences = candidate;
        self.load_brush_view();
    }

    fn saved_brush_at(
        &self,
        point: Vec3,
        rect: egui::Rect,
        kind: BrushKind,
    ) -> Option<BrushSettings> {
        let sample = self
            .preferences
            .preview(
                temporary_modifiers(self.preferences.selected(), kind),
                self.current_contact_input(),
                None,
                None,
            )
            .ok()?;
        let mut settings = sample.settings;
        if sample.screen_radius.enabled {
            let pixels = if matches!(
                self.preferences.global().pressure,
                PressureProfile::SculptGlFactors { .. }
            ) {
                sample.screen_radius.effective_pixels
            } else {
                sample.screen_radius.base_pixels
            };
            settings.radius =
                ScreenFootprint::capture(&self.camera, pixels, rect.height(), rect.aspect_ratio())?
                    .radius_at_world(&self.document, point)?;
        }
        settings.adaptive_detail &= self.adaptive_detail_available();
        Some(settings)
    }

    pub fn set_navigation_pointer(&mut self, buttons: u16, x: f32, y: f32, shift: bool) {
        self.browser_navigation_coordinates = true;
        let zoom = self.egui_ctx.zoom_factor();
        self.navigation_capture
            .pointer(buttons, [x / zoom, y / zoom], shift);
        self.set_navigation_buttons(buttons);
    }

    fn navigation_down(&self) -> bool {
        self.browser_navigation.navigation_down()
            || self.navigation_capture.held()
            || self.navigation_capture.pending()
    }

    /// Explicit browser contact loss differs from a normal button release.
    /// The interruption stays latched until the next application update.
    pub fn cancel_pointer_contact(&mut self) {
        self.browser_pen_samples.clear();
        if self
            .stroke
            .as_ref()
            .is_some_and(|s| s.continuous_locked.is_some())
        {
            self.unprocessed_release = None;
            self.stop_continuous(StopReason::FocusLost);
            self.finish_stroke_deferred();
        }
        if self.interrupt_source_paint_tail("Source Paint lost pointer focus") {
            self.unprocessed_release = None;
            self.stop_continuous(StopReason::FocusLost);
            self.finish_stroke_deferred();
        }
        self.navigation_capture.cancel();
        self.native_navigation_buttons = 0;
        self.browser_navigation.cancel_contact();
        self.egui_ctx.request_repaint();
    }

    /// Direct decoded RGBA imports remain supported; async picker requests
    /// additionally bind the initiating tool and conversion by immutable ID.
    pub fn load_alpha(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.install_alpha(
            self.preferences.selected(),
            self.alpha_conversion,
            name,
            width,
            height,
            rgba,
        )
    }

    pub fn load_alpha_for_request(
        &mut self,
        id: u32,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        let request = self
            .alpha_import_request
            .filter(|r| r.id == id)
            .ok_or("Alpha import request is stale")?;
        self.install_alpha(
            request.destination,
            request.conversion,
            name,
            width,
            height,
            rgba,
        )?;
        self.alpha_import_request = None;
        Ok(())
    }

    /// Decode raw PNG bytes without Canvas premultiplication or color conversion.
    /// A stale request is rejected before decoding, and failures keep the active
    /// stroke, library and saved options untouched.
    pub fn load_alpha_png_for_request(
        &mut self,
        id: u32,
        name: &str,
        png: &[u8],
    ) -> Result<(), String> {
        self.load_alpha_image_for_request(id, name, png)
    }

    pub fn load_alpha_image_for_request(
        &mut self,
        id: u32,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let candidate = alpha_image_import_candidate(
            &self.preferences,
            self.alpha_import_request,
            id,
            name,
            bytes,
        )?;
        self.install_validated_alpha(candidate, name)?;
        self.alpha_import_request = None;
        Ok(())
    }

    fn install_alpha(
        &mut self,
        destination: BrushKind,
        conversion: AlphaConversion,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        // Validate/decode/library-budget a cloned candidate before releasing any
        // actual stroke. Invalid imports leave document, queue and prefs intact.
        let candidate = alpha_import_candidate(
            &self.preferences,
            destination,
            conversion,
            name,
            width,
            height,
            rgba,
        )?;
        self.install_validated_alpha(candidate, name)
    }

    fn install_validated_alpha(
        &mut self,
        candidate: BrushPreferenceStore,
        name: &str,
    ) -> Result<(), String> {
        self.finish_stroke();
        if self.stroke_rejected {
            return Err("The preceding edit was rejected; retry alpha import".into());
        }
        self.preferences = candidate;
        self.load_brush_view();
        self.message = format!("Alpha loaded: {name}.");
        self.egui_ctx.request_repaint();
        Ok(())
    }

    pub fn begin_uv_texture_request(&mut self) -> Result<u32, String> {
        self.uv_texture_requests.invalidate();
        self.finish_stroke();
        if self.stroke_rejected {
            return Err(
                "Texture file command rejected after a failed stroke; previous asset kept".into(),
            );
        }
        self.uv_texture_requests.begin()
    }
    pub fn uv_texture_request_is_current(&self, id: u32) -> bool {
        self.uv_texture_requests.is_current(id)
    }
    pub fn cancel_uv_texture_request(&mut self, id: u32) {
        self.uv_texture_requests.cancel(id);
    }
    pub fn load_uv_texture_png_for_request(
        &mut self,
        id: u32,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        if !self.uv_texture_requests.is_current(id) {
            return Err("Texture response is stale or cancelled; previous asset kept".into());
        }
        let texture = match crate::uv_view::decode_png(name, bytes) {
            Ok(texture) => texture,
            Err(error) => {
                self.uv_texture_requests.cancel(id);
                return Err(error);
            }
        };
        // Current owner is checked again after decode/barrier, so all host
        // routes obey the same ownership even if completion becomes async later.
        self.finish_stroke();
        if self.stroke_rejected {
            self.uv_texture_requests.cancel(id);
            return Err(
                "Texture import rejected after a failed stroke; previous asset kept".into(),
            );
        }
        if !self.uv_texture_requests.cancel(id) {
            return Err("Texture request no longer owns the asset".into());
        }
        self.install_view_texture(texture)
    }
    pub fn load_uv_texture(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        let texture = ViewTexture::from_rgba(name, width, height, rgba)?;
        self.uv_texture_requests.invalidate();
        self.install_view_texture(texture)
    }
    pub fn load_uv_texture_png(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let texture = crate::uv_view::decode_png(name, bytes)?;
        self.uv_texture_requests.invalidate();
        self.install_view_texture(texture)
    }
    fn install_view_texture(&mut self, texture: ViewTexture) -> Result<(), String> {
        self.finish_stroke();
        if self.stroke_rejected {
            return Err(
                "Viewport texture change rejected after a failed stroke; previous asset kept"
                    .into(),
            );
        }
        self.cancel_mask_region("Texture command cancelled the pending selection.", true);
        self.message = format!("UV viewport texture loaded: {}", texture.name);
        self.viewport_style.uv_texture = Some(Arc::new(texture));
        self.egui_ctx.request_repaint();
        Ok(())
    }
    fn reset_uv_texture(&mut self) -> Result<(), String> {
        self.uv_texture_requests.invalidate();
        self.finish_stroke();
        if self.stroke_rejected {
            return Err("Texture reset rejected after a failed stroke; previous asset kept".into());
        }
        self.viewport_style.uv_texture = None;
        self.egui_ctx.request_repaint();
        Ok(())
    }
    pub fn uv_render_metrics(&self) -> String {
        self.renderer
            .lock()
            .map(|r| r.uv_render_metrics(true).to_string())
            .unwrap_or_else(|_| "{}".into())
    }
    pub fn background_request_is_current(&self, id: u32) -> bool {
        self.background_requests.is_current(id)
    }
    pub fn cancel_background_request(&mut self, id: u32) {
        self.background_requests.cancel(id);
    }
    pub fn load_background_image_for_request(
        &mut self,
        id: u32,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        if !self.background_requests.is_current(id) {
            return Err("Background response is stale; previous image kept".into());
        }
        let candidate = if bytes.starts_with(&[255, 216]) {
            crate::uv_view::decode_jpeg(name, bytes)
        } else {
            crate::uv_view::decode_png(name, bytes)
        };
        let candidate = match candidate {
            Ok(image) => image,
            Err(error) => {
                self.background_requests.cancel(id);
                return Err(error);
            }
        };
        // Validate bytes before any geometry/history barrier. Decode/rejection is atomic.
        self.finish_stroke();
        if self.stroke_rejected {
            self.background_requests.cancel(id);
            return Err(
                "Background command rejected after the preceding edit; previous image kept".into(),
            );
        }
        if !self.background_requests.cancel(id) {
            return Err("Background request no longer owns the image".into());
        }
        self.viewport_style.environment.background_image = Some(Arc::new(candidate));
        // Image replacement/reset retains the selected environment/background mode.
        self.viewport_style.environment.custom_background = true;
        self.message = format!("Background loaded: {name}");
        self.egui_ctx.request_repaint();
        Ok(())
    }
    fn import_background_image(&mut self) {
        let id = match self.background_requests.begin() {
            Ok(id) => id,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        pick_background_image(id);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Background image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            let result = (|| {
                if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
                    return Err("Background image exceeds16MiB input limit".into());
                }
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                self.load_background_image_for_request(
                    id,
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    &bytes,
                )
            })();
            if let Err(error) = result {
                self.background_requests.cancel(id);
                self.message = format!("Background image rejected: {error}");
            }
        } else {
            self.background_requests.cancel(id);
        }
    }
    fn reset_background_image(&mut self) {
        self.background_requests.invalidate();
        self.viewport_style.environment.background_image = None;
        // Image replacement/reset retains the selected environment/background mode.
        self.egui_ctx.request_repaint();
    }
    fn environment_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(9.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CC38AF64CE9B3,
            ))
            .size(11.0)
            .color(preferences_host::muted(ui)),
        );
        let old = self.viewport_style.environment.preset;
        let name = ENVIRONMENT_PRESETS
            .get(old as usize)
            .map_or("Unknown environment", |p| p.name);
        let selected = egui::ComboBox::from_id_salt("environment_preset")
            .width(ui.available_width())
            .truncate()
            .selected_text(name)
            .show_ui(ui, |ui| {
                for (index, preset) in ENVIRONMENT_PRESETS.iter().enumerate() {
                    let row = ui.selectable_value(
                        &mut self.viewport_style.environment.preset,
                        index as u32,
                        preset.name,
                    );
                    self.track_layer_control(&format!("environment.preset.{index}"), &row);
                }
            })
            .response;
        self.track_layer_control("environment.preset", &selected);
        if old != self.viewport_style.environment.preset {
            self.viewport_style.exposure = ENVIRONMENT_PRESETS
                [self.viewport_style.environment.preset as usize]
                .default_exposure;
        }
        let filmic = ui.checkbox(
            &mut self.viewport_style.environment.filmic,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CADBA4F3F1C64),
        );
        self.track_layer_control("environment.filmic", &filmic);
        let darken = ui.checkbox(
            &mut self.viewport_style.environment.darken_unselected,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C0903E803CA38),
        );
        self.track_layer_control("environment.darken_unselected", &darken);
        let resolution_label = crate::i18n::source_text(
            self.app_preferences.locale,
            crate::source_text_keys::SourceTextKey::Resolution,
            crate::i18n::TextArguments::None,
        )
        .map(|text| text.text.into_owned())
        .unwrap_or_else(|_| "Resolution".into());
        ui.label(format!(
            "{resolution_label} · {:.2}×",
            self.viewport_style.resolution_scale
        ));
        let resolution = ui.add(
            egui::Slider::new(&mut self.viewport_style.resolution_scale, 0.5..=2.0)
                .step_by(0.02)
                .show_value(false),
        );
        self.track_layer_control("viewport.resolution", &resolution);
        if let Some((pixels, Some(error))) = self.renderer.lock().ok().map(|renderer| {
            let (pixels, error) = renderer.resolution_status();
            (pixels, error.map(str::to_owned))
        }) {
            ui.label(format!("{}×{} px · {error}", pixels[0], pixels[1]));
        }
        let grid_label = crate::i18n::source_text(
            self.app_preferences.locale,
            crate::source_text_keys::SourceTextKey::RenderingGrid,
            crate::i18n::TextArguments::None,
        )
        .map(|text| text.text.into_owned())
        .unwrap_or_else(|_| "Show grid".into());
        let grid = ui.checkbox(
            &mut self.viewport_style.source_display.show_grid,
            grid_label,
        );
        self.track_layer_control("display.grid", &grid);
        let contour_label = crate::i18n::source_text(
            self.app_preferences.locale,
            crate::source_text_keys::SourceTextKey::ContourShow,
            crate::i18n::TextArguments::None,
        )
        .map(|text| text.text.into_owned())
        .unwrap_or_else(|_| "Show contour".into());
        let contour = ui.checkbox(
            &mut self.viewport_style.source_display.show_contour,
            contour_label,
        );
        self.track_layer_control("display.contour", &contour);
        if self.viewport_style.source_display.show_contour {
            let color_label = crate::i18n::source_text(
                self.app_preferences.locale,
                crate::source_text_keys::SourceTextKey::ContourColor,
                crate::i18n::TextArguments::None,
            )
            .map(|text| text.text.into_owned())
            .unwrap_or_else(|_| "Contour color".into());
            ui.horizontal(|ui| {
                ui.label(color_label);
                let mut chosen = self.viewport_style.source_display.contour_color;
                let color = ui.color_edit_button_rgba_unmultiplied(&mut chosen);
                if color.changed() {
                    self.viewport_style.source_display.contour_color = chosen;
                }
                self.track_layer_control("display.contour_color", &color);
            });
        }
        let guide = ui
            .checkbox(
                &mut self.viewport_style.source_display.show_symmetry_line,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CB117A63DA026),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C10C069826A6D,
            ));
        self.track_layer_control("display.symmetry_line", &guide);
        if self.viewport_style.source_display.show_symmetry_line {
            let profile = egui::ComboBox::from_id_salt("symmetry_guide_profile")
                .width(140.0)
                .truncate()
                .selected_text(self.viewport_style.source_display.guide_profile.name())
                .show_ui(ui, |ui| {
                    for (value, id) in [
                        (
                            crate::source_display::GuidePlaneProfile::ActiveSourceObject,
                            "active_object",
                        ),
                        (
                            crate::source_display::GuidePlaneProfile::CurrentBrush,
                            "brush",
                        ),
                        (
                            crate::source_display::GuidePlaneProfile::ReferenceCenterPreview,
                            "reference_center",
                        ),
                    ] {
                        let row = ui.selectable_value(
                            &mut self.viewport_style.source_display.guide_profile,
                            value,
                            value.name(),
                        );
                        self.track_layer_control(&format!("display.guide_profile.{id}"), &row);
                    }
                })
                .response;
            self.track_layer_control("display.guide_profile", &profile);
            if self.viewport_style.source_display.guide_profile
                == crate::source_display::GuidePlaneProfile::ReferenceCenterPreview
            {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CCC4BF142C03D,
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                );
            }
        }
        let custom = ui.checkbox(
            &mut self.viewport_style.environment.custom_background,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CF048A0D08F99),
        );
        self.track_layer_control("background.enabled", &custom);
        if self.viewport_style.environment.custom_background {
            let combo = egui::ComboBox::from_id_salt("background_mode")
                .width(ui.available_width())
                .selected_text(self.viewport_style.environment.background_mode.name())
                .show_ui(ui, |ui| {
                    for (index, mode) in [
                        BackgroundMode::Image,
                        BackgroundMode::Environment,
                        BackgroundMode::Ambient,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let row = ui.selectable_value(
                            &mut self.viewport_style.environment.background_mode,
                            mode,
                            mode.name(),
                        );
                        self.track_layer_control(&format!("background.mode.{index}"), &row);
                    }
                })
                .response;
            self.track_layer_control("background.mode", &combo);
            let fill = ui.checkbox(
                &mut self.viewport_style.environment.background_fill,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C630045C3B717),
            );
            self.track_layer_control("background.fill", &fill);
            {
                // File replacement/reset are available under every background mode.
                let import = ui.button(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C3381DC4E7656,
                ));
                self.track_layer_control("background.import", &import);
                if import.clicked() {
                    self.import_background_image();
                }
                let reset = ui.add_enabled(
                    self.viewport_style.environment.background_image.is_some(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C9BD78F90D72D,
                    )),
                );
                self.track_layer_control("background.reset", &reset);
                if reset.clicked() {
                    self.reset_background_image();
                }
                if let Some(image) = self.viewport_style.environment.background_image.as_ref() {
                    ui.add(egui::Label::new(&image.name).truncate())
                        .on_hover_text(&image.name);
                }
            }
            if self.viewport_style.environment.background_mode == BackgroundMode::Environment {
                label_value(
                    ui,
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::CD282024F3CDC),
                    &format!("{:.2}", self.viewport_style.environment.background_blur),
                );
                let blur = ui.add(
                    egui::Slider::new(
                        &mut self.viewport_style.environment.background_blur,
                        0.0..=1.0,
                    )
                    .show_value(false),
                );
                self.track_layer_control("background.blur", &blur);
            }
        }
    }
    fn import_uv_texture(&mut self) {
        let id = match self.begin_uv_texture_request() {
            Ok(id) => id,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        pick_uv_texture(id);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("UV viewport PNG", &["png"])
            .pick_file()
        {
            let result = (|| {
                if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
                    return Err("Texture PNG exceeds the 16 MiB limit".to_owned());
                }
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                self.load_uv_texture_png_for_request(
                    id,
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    &bytes,
                )
            })();
            if let Err(error) = result {
                self.uv_texture_requests.cancel(id);
                self.message = format!("Texture import failed; previous asset kept: {error}");
            }
        } else {
            self.uv_texture_requests.cancel(id);
        }
    }

    fn begin_alpha_import(&mut self) -> Result<AlphaImportRequest, String> {
        let id = self.next_alpha_import_id;
        let next = id
            .checked_add(1)
            .ok_or("Alpha import request IDs exhausted")?;
        let request = AlphaImportRequest {
            id,
            destination: self.preferences.selected(),
            conversion: self.alpha_conversion,
        };
        self.alpha_import_request = Some(request);
        self.next_alpha_import_id = next;
        Ok(request)
    }

    fn import_alpha(&mut self) {
        if !self.commit_brush_view() {
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let request = match self.begin_alpha_import() {
            Ok(request) => request,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        pick_alpha(request.id);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Brush alpha image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            let result = (|| {
                use std::io::Read;
                let file = std::fs::File::open(&path).map_err(|error| error.to_string())?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve(64 * 1024)
                    .map_err(|_| "Cannot allocate alpha input")?;
                file.take(crate::alpha_decode::MAX_ALPHA_IMAGE_INPUT_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| error.to_string())?;
                self.load_alpha_image_for_request(
                    request.id,
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    &bytes,
                )
            })();
            if let Err(error) = result {
                self.message = format!("Alpha import failed: {error}");
            }
        }
    }

    fn cancel_topology(&mut self) {
        if let Some(job) = self.topology_job.take() {
            #[cfg(target_arch = "wasm32")]
            cancel_topology_worker(job.id);
            #[cfg(not(target_arch = "wasm32"))]
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.message = "Operation cancelled. Its result will not replace your sculpt.".into();
        }
    }

    fn start_topology(&mut self, operation: TopologyOperation) {
        self.modal_cancel("topology command");
        if self.topology_job.is_some() || self.uv_bake_job.is_some() {
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        if self.document.layer_topology_locked()
            && !matches!(
                &operation,
                TopologyOperation::RemeshSelected {
                    policy: SubtoolMergePolicy::FlattenDisplayed,
                    ..
                }
            )
        {
            self.message = "Bake sculpt layers before rebuilding or reducing the surface.".into();
            return;
        }
        self.next_job_id = self.next_job_id.wrapping_add(1);
        let id = self.next_job_id;
        let revision = self.revision;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let selected_project = if matches!(&operation, TopologyOperation::RemeshSelected { .. })
            {
                match self.document.to_json() {
                    Ok(project) => Some(project),
                    Err(error) => {
                        self.message = error;
                        return;
                    }
                }
            } else {
                None
            };
            let mesh = if selected_project.is_none() {
                match self.document.require_active_mesh() {
                    Ok(mesh) => Some(mesh.snapshot()),
                    Err(error) => {
                        self.message = error;
                        return;
                    }
                }
            } else {
                None
            };
            let ctx = self.egui_ctx.clone();
            let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let worker_cancel = cancel.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            match std::thread::Builder::new()
                .name("obrush-topology".into())
                .spawn(move || {
                    let result = if let Some(project) = selected_project {
                        serde_json::to_string(&operation)
                            .map_err(|error| error.to_string())
                            .and_then(|operation| {
                                obrush_core::execute_topology_project_with_cancel(
                                    &project,
                                    &operation,
                                    || worker_cancel.load(std::sync::atomic::Ordering::Relaxed),
                                )
                            })
                            .map(TopologyWorkerResult::Project)
                    } else {
                        obrush_core::execute_topology_with_cancel(
                            mesh.as_ref().expect("ordinary topology has an active mesh"),
                            &operation,
                            || worker_cancel.load(std::sync::atomic::Ordering::Relaxed),
                        )
                        .map(|(mesh, report)| {
                            TopologyWorkerResult::Mesh(Box::new(mesh), Box::new(report))
                        })
                    };
                    let _ = sender.send(result);
                    ctx.request_repaint();
                }) {
                Ok(_) => {
                    self.topology_job = Some(TopologyJob {
                        id,
                        revision,
                        started: web_time::Instant::now(),
                        receiver,
                        cancel,
                    })
                }
                Err(error) => {
                    self.message = format!("Cannot start operation: {error}");
                    return;
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let request = self.document.to_json().and_then(|project| {
                serde_json::to_string(&operation)
                    .map(|operation| (project, operation))
                    .map_err(|e| e.to_string())
            });
            let (project, operation) = match request {
                Ok(value) => value,
                Err(error) => {
                    self.message = error;
                    return;
                }
            };
            self.topology_job = Some(TopologyJob {
                id,
                revision,
                started: web_time::Instant::now(),
            });
            if let Err(error) = start_topology_worker(id, &project, &operation) {
                self.topology_job = None;
                self.message = format!("Cannot start operation: {error:?}");
                return;
            }
        }
        self.message = "Working on a copy of your sculpt… You can orbit or cancel.".into();
        self.egui_ctx.request_repaint();
    }

    fn accept_topology(&mut self, job: TopologyJob, result: Result<TopologyWorkerResult, String>) {
        if self.revision != job.revision {
            self.message = "Operation result discarded because the document changed.".into();
            return;
        }
        let result = match result {
            #[cfg(not(target_arch = "wasm32"))]
            Ok(TopologyWorkerResult::Mesh(mesh, report)) => Ok((*mesh, *report)),
            Ok(TopologyWorkerResult::Project(contents)) => {
                match decode_topology_project(&contents) {
                    Ok((document, obrush_core::TopologyReport::RemeshSelected(report))) => {
                        let triangles = document
                            .active_mesh()
                            .map_or(0, |mesh| mesh.triangles.len());
                        match self.document.replace_document(document) {
                            Ok(()) => {
                                self.changed();
                                self.message = format!(
                                    "Fused {} selected parts into {} triangles. Undo restores every source part.",
                                    number(report.consumed_ids.len()),
                                    number(triangles)
                                );
                                if !report.flattened_ids.is_empty() {
                                    self.message.push_str(" Selected levels, layers or targets were flattened; Undo restores them.");
                                }
                            }
                            Err(error) => {
                                self.message = format!("Operation result rejected: {error}")
                            }
                        }
                        self.egui_ctx.request_repaint();
                        return;
                    }
                    Ok((document, report)) => document
                        .mesh
                        .map(|mesh| (mesh, report))
                        .ok_or_else(|| "Topology result has no active mesh".into()),
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        match result {
            Ok((mesh, report)) => {
                if matches!(&report, obrush_core::TopologyReport::Decimate(report) if report.collapses==0)
                {
                    self.message =
                        "No safe reduction was available. Sculpt and history preserved.".into();
                    self.egui_ctx.request_repaint();
                    return;
                }
                let triangles = mesh.triangles.len();
                match self.document.replace_mesh(mesh) {
                    Ok(()) => {
                        self.changed();
                        self.message = format!(
                            "Surface updated: {} triangles. Undo restores the previous sculpt.",
                            number(triangles)
                        );
                        if let obrush_core::TopologyReport::Decimate(report) = &report
                            && !report.reached_target
                        {
                            self.message = format!(
                                "Reduced to {} triangles; {}. Undo restores the previous sculpt.",
                                number(triangles),
                                report.termination
                            );
                        }
                        if let obrush_core::TopologyReport::Remesh(report) = &report
                            && report.discarded_uv_faces > 0
                        {
                            self.message = format!(
                                "Surface updated: {} triangles. {} UV faces discarded; Undo restores the original UVs.",
                                number(triangles),
                                number(report.discarded_uv_faces)
                            );
                        }
                    }
                    Err(error) => self.message = format!("Operation result rejected: {error}"),
                }
            }
            Err(error) => self.message = format!("Operation failed; sculpt preserved: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    #[cfg(target_arch = "wasm32")]
    pub fn complete_topology(&mut self, id: u32, success: bool, contents: &str) {
        if self.topology_job.as_ref().is_none_or(|job| job.id != id) {
            return;
        }
        let job = self.topology_job.take().unwrap();
        let result = if success {
            Ok(TopologyWorkerResult::Project(contents.to_owned()))
        } else {
            Err(contents.chars().take(1000).collect::<String>())
        };
        self.accept_topology(job, result);
    }

    fn poll_topology(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = self.topology_job.as_ref() {
            let result = match job.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Background operation ended unexpectedly".into()))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let job = self.topology_job.take().unwrap();
                self.accept_topology(job, result);
            }
        }
        if self.topology_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn changed(&mut self) {
        self.resident_studio.invalidate();
        self.modal_invalidate_publication("document publication changed");
        self.cancel_startup_model();
        self.cancel_uv_bake(false);
        self.paint_pick_armed = false;
        self.paint_pick_dragging = false;
        self.last_paint_sample = None;
        let had_region = self.mask_region_gesture.is_some();
        self.cancel_mask_region(
            "The part changed; pending mask selection cancelled.",
            had_region,
        );
        if self.ui_subtool_id != self.document.active_subtool_id() {
            self.ui_subtool_id = self.document.active_subtool_id();
            self.selected_layer = None;
            self.layer_name_index = None;
            self.layer_intensity_preview = None;
            self.subtool_name_id = None;
            self.transform_values = TransformValues::default();
        }
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
        self.brush.adaptive_detail = self.document.supports_adaptive_detail();
    }

    fn finish_stroke(&mut self) {
        self.cancel_mask_region("A command cancelled the pending mask selection.", true);
        self.finish_stroke_inner();
    }

    fn finish_stroke_inner(&mut self) {
        self.background_mask_gesture = None;
        self.stop_continuous(StopReason::Command);
        let transform_ok = self.finish_transform_drag();
        self.stroke_rejected = !transform_ok;
        // Commands precede viewport layout. A release delivered in this same
        // frame still belongs to the active stroke and must be sampled first.
        if let Some((position, _modifiers, pressure)) = self.unprocessed_release.take()
            && (self.last_viewport.contains(position)
                || self.stroke.as_ref().is_some_and(|stroke| {
                    stroke.locked_stamp.is_some()
                        || stroke.source_paint.as_ref().is_some_and(|s| {
                            s.lock().is_ok_and(|s| {
                                s.locked_mode()
                                    == Some(obrush_core::SourcePaintLockedMode::AbsolutePreview)
                            })
                        })
                }))
            && let Some(stroke) = self.stroke.as_ref()
        {
            // A command chord on the release frame belongs to the command,
            // not to a new Mask/Smooth gesture on the old stroke's endpoint.
            let (kind, invert) = (stroke.kind, stroke.invert);
            let previous_pressure = self.brush.pressure;
            self.brush.pressure = pressure;
            self.sample_stroke_to(position, self.last_viewport, kind, invert);
            self.brush.pressure = previous_pressure;
        }
        self.finish_stroke_deferred();
        self.process_dabs(usize::MAX, None);
    }

    fn finish_stroke_deferred(&mut self) {
        self.browser_pen_samples.clear();
        self.stop_continuous(StopReason::Command);
        if let Some(stroke) = self.stroke.take() {
            if let Some(session) = stroke.source_geometry.as_ref() {
                self.finish_source_geometry_session(&stroke, session.clone());
            } else if stroke.queued {
                let source_locked = stroke
                    .source_paint
                    .as_ref()
                    .is_some_and(|s| s.lock().is_ok_and(|s| s.is_locked()));
                let progress = if source_locked {
                    self.pending_dabs.release_source_paint_locked(
                        &mut self.document,
                        stroke.source_paint.as_ref().expect("captured Source Lock"),
                    )
                } else {
                    self.pending_dabs.release(&mut self.document)
                };
                if source_locked && progress.error.is_some() {
                    self.stroke_rejected = true;
                }
                if progress.changed {
                    self.changed();
                }
                if progress.painted {
                    self.viewport_style.use_vertex_color = true;
                }
                if source_locked {
                    crate::source_paint_host::restore_locked_display(
                        &mut self.dirty,
                        &mut self.viewport_style.use_vertex_color,
                        stroke.source_paint_display_before,
                        stroke
                            .source_paint
                            .as_ref()
                            .is_some_and(|s| s.lock().is_ok_and(|s| s.is_locked_at_original())),
                    );
                }
                if let Some(error) = progress.error {
                    self.message = format!("Brush input rejected: {error}");
                }
                let mut source_commit_error = None;
                if stroke.source_paint.is_some()
                    && let Some(error) = self.document.take_layer_error()
                {
                    source_commit_error = Some(error.clone());
                    if let Some((dirty, painted)) = stroke.source_paint_display_before {
                        self.dirty = dirty;
                        self.viewport_style.use_vertex_color = painted;
                    }
                    self.revision = self.revision.wrapping_add(1);
                    self.stroke_rejected = true;
                    self.message = format!(
                        "Source Paint commit rejected; restored original document: {error}"
                    );
                }
                if let Some(session) = stroke.source_paint.as_ref() {
                    self.last_source_paint_report = session.lock().ok().map(|mut s| {
                        s.finish_alpha(source_commit_error.is_some() || self.stroke_rejected);
                        serde_json::json!({"revision":self.revision,"commit_error":source_commit_error,
                            "dirty":self.dirty,"vertex_color":self.viewport_style.use_vertex_color,"capture":s.diagnostics()})
                    });
                }
            } else {
                self.document.end_stroke();
                if let Some(locked) = stroke.continuous_locked.as_ref() {
                    let commit_error = self.document.take_layer_error();
                    if commit_error.is_some() || locked.gesture.at_original() {
                        self.dirty = locked.dirty_before;
                        self.viewport_style.use_vertex_color = locked.vertex_color_before;
                    }
                    if let Some(error) = &commit_error {
                        self.revision = self.revision.wrapping_add(1);
                        self.stroke_rejected = true;
                        self.message =
                            format!("Continuous Lock commit rejected; restored original: {error}");
                    }
                    self.last_continuous_locked_report = Some(serde_json::json!({
                        "revision":self.revision,"commit_error":commit_error,"capture":locked.gesture.diagnostics()}));
                }
                if let Some(stamp) = stroke.locked_stamp.as_ref()
                    && stamp.stamp.at_origin()
                {
                    self.dirty = stamp.dirty_before;
                    self.viewport_style.use_vertex_color = stamp.vertex_color_before;
                }
            }
        }
        self.source_paint_raw.finish();
        self.contact_pressure.finish_release();
        self.brush.pressure = self.contact_pressure.current();
        self.show_layer_error();
    }

    fn cancel_absolute_locked_stamp(&mut self) -> bool {
        if self
            .stroke
            .as_ref()
            .is_some_and(|s| s.continuous_locked.is_some())
        {
            return false;
        }
        self.cancel_locked_stamp()
    }

    fn cancel_locked_stamp(&mut self) -> bool {
        if self
            .stroke
            .as_ref()
            .is_some_and(|s| s.continuous_locked.is_some())
        {
            return self.cancel_continuous_locked();
        }
        if self
            .stroke
            .as_ref()
            .is_none_or(|stroke| stroke.locked_stamp.is_none())
        {
            return false;
        }
        let mut stroke = self.stroke.take().unwrap();
        let locked = stroke.locked_stamp.as_mut().unwrap();
        match self
            .document
            .require_active_mesh_mut()
            .and_then(|mesh| locked.stamp.cancel(mesh))
        {
            Ok(report) => {
                if report.changed() {
                    self.changed();
                }
                self.last_stamp_report = Some(HostLockedStampReport {
                    subtool_id: self.document.active_subtool_id(),
                    revision: self.revision,
                    report,
                });
                self.document.end_stroke();
                self.dirty = locked.dirty_before;
                self.viewport_style.use_vertex_color = locked.vertex_color_before;
                self.message = "Stamp cancelled. The original surface is restored.".into();
            }
            Err(error) => {
                self.document.end_stroke();
                self.message = format!("Stamp cancellation rejected: {error}");
            }
        }
        self.unprocessed_release = None;
        self.source_paint_raw.finish();
        self.contact_pressure.finish_release();
        self.brush.pressure = self.contact_pressure.current();
        true
    }

    fn process_dabs(&mut self, maximum: usize, budget: Option<std::time::Duration>) {
        let result = self
            .pending_dabs
            .process(&mut self.document, maximum, budget);
        self.handle_dab_progress(result);
    }

    fn handle_dab_progress(&mut self, result: crate::dab_queue::Progress) {
        let prior_revision = self.revision;
        let prior_snapshot_current = self.snapshot_revision == prior_revision;
        if result.derived_cache_changed {
            self.egui_ctx.request_repaint();
        }
        if result.changed {
            self.changed();
        }
        if result.changed
            && result.error.is_none()
            && prior_snapshot_current
            && let Some(proof) = result.render_snapshot.as_ref()
            && let Some(active) = self
                .rendered_scene
                .iter_mut()
                .find(|snapshot| snapshot.active && snapshot.id == proof.snapshot().id)
        {
            // The core captured this immutable pair inside the successful
            // controlled dab. Other scene objects were not edited by it.
            *active = proof.snapshot().clone();
            self.snapshot_revision = self.revision;
            self.render_certified_handoffs = self.render_certified_handoffs.saturating_add(1);
            if let Ok(mut renderer) = self.renderer.lock() {
                renderer.adopt_checked_source_snapshot(proof);
            }
        }
        if result.painted {
            self.viewport_style.use_vertex_color = true;
        }
        if let Some(error) = result.error {
            if self
                .stroke
                .as_ref()
                .is_some_and(|s| s.source_paint.is_some() || s.source_geometry.is_some())
            {
                self.stop_continuous(StopReason::Command);
                self.unprocessed_release = None;
                self.finish_stroke_deferred();
                self.stroke_rejected = true;
            }
            if self.stroke.as_ref().is_some_and(|s| s.continuous.is_some()) {
                self.continuous_clock.stop(StopReason::Command);
                self.finish_stroke_deferred();
            }
            self.message = format!("Brush input rejected: {error}");
        }
        self.show_layer_error();
    }

    fn continuous_now(&self) -> Option<u64> {
        self.continuous_origin.elapsed().as_micros().try_into().ok()
    }

    fn stop_continuous(&mut self, reason: StopReason) {
        let report = self.continuous_clock.report();
        if report.active_key.is_some() || report.outstanding_permit {
            self.continuous_clock.stop(reason);
        }
    }

    fn set_continuous_mode(&mut self, enabled: bool) -> bool {
        if !self.commit_brush_view() {
            return false;
        }
        let mut candidate = self.preferences.clone();
        let mut global = candidate.global();
        if global.continuous == enabled {
            return true;
        }
        global.continuous = enabled;
        if let Err(error) = candidate.set_global(global) {
            self.message = format!("Continuous setting rejected: {error}");
            return false;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return false;
        }
        self.preferences = candidate;
        self.load_brush_view();
        true
    }

    fn set_pressure_profile(&mut self, pressure: PressureProfile) -> bool {
        if !self.commit_brush_view() {
            return false;
        }
        let mut candidate = self.preferences.clone();
        let mut global = candidate.global();
        if global.pressure == pressure {
            return true;
        }
        global.pressure = pressure;
        if let Err(error) = candidate.set_global(global) {
            self.message = format!("Pressure setting rejected: {error}");
            return false;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return false;
        }
        self.preferences = candidate;
        self.load_brush_view();
        true
    }

    fn continuous_frame(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let Some(stroke) = self.stroke.as_ref() else {
            self.stop_continuous(StopReason::StrokeChanged);
            return;
        };
        let Some(key) = stroke.continuous else {
            self.stop_continuous(StopReason::StrokeChanged);
            return;
        };
        if stroke.continuous_locked.is_some() {
            self.continuous_locked_frame(ctx, rect, key);
            return;
        }
        if let Some(session) = stroke.source_geometry.clone() {
            self.source_geometry_continuous_frame(ctx, rect, key, session);
            return;
        }
        if let Some(session) = stroke.source_paint.clone() {
            self.source_paint_continuous_frame(ctx, rect, key, session);
            return;
        }
        let (position, primary_down, focused) = ctx.input(|input| {
            (
                input.pointer.interact_pos(),
                input.pointer.primary_down(),
                input.focused,
            )
        });
        let position = position.filter(|p| p.is_finite() && rect.contains(*p));
        let target_matches =
            Some(key.subtool_id) == self.document.active_subtool_id() && key.kind == stroke.kind;
        let surface_hit = position.is_some_and(|p| self.pick(p, rect).is_some());
        let ready = Readiness {
            key: target_matches.then_some(key),
            mode_enabled: self.preferences.global().continuous,
            primary_down,
            pressure: self.brush.pressure,
            focused,
            navigating: ctx
                .input(|input| viewport_navigation_requested(input, self.navigation_down())),
            target_visible: self.document.active_subtool_visible(),
            pointer_in_viewport: position.is_some(),
            surface_hit,
            queue_empty: self.pending_dabs.is_empty(),
            operation_busy: self.topology_job.is_some(),
        };
        let Some(now) = self.continuous_now() else {
            self.continuous_clock.stop(StopReason::ClockOverflow);
            self.finish_stroke_deferred();
            return;
        };
        let report = self.continuous_clock.report();
        if report.active_key.is_none() {
            let restartable = matches!(
                report.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            );
            let valid = target_matches
                && ready.mode_enabled
                && primary_down
                && focused
                && !ready.navigating
                && ready.target_visible
                && position.is_some()
                && surface_hit
                && ready.queue_empty
                && !ready.operation_busy
                && ready.pressure.is_finite()
                && ready.pressure > 0.0
                && ready.pressure <= 1.0;
            if !restartable || !valid {
                return;
            }
            if let Err(error) = self.continuous_clock.start(key, true, false, now) {
                self.message = error;
                self.finish_stroke_deferred();
                return;
            }
        }
        // Interruption readiness must reach the clock even if there is no usable cursor.
        let fallback = Pos2::new(rect.left(), rect.top());
        let capture = captured_stroke_dab(
            self.stroke.as_ref().unwrap(),
            &self.camera,
            position.unwrap_or(fallback),
            rect,
            key.kind,
            self.stroke.as_ref().unwrap().invert,
            self.current_contact_input(),
        );
        let Some(capture) = capture else {
            self.continuous_clock.stop(StopReason::Command);
            self.message = "Continuous brush projection rejected.".into();
            self.finish_stroke_deferred();
            return;
        };
        let scene_ray = if !self.camera.source_enabled()
            && self.rendered_scene.iter().all(|o| o.metadata.is_none())
        {
            obrush_core::SceneRay64 {
                origin: capture.ray.origin.as_dvec3(),
                direction: capture.ray.direction.as_dvec3(),
            }
        } else {
            self.screen_scene_ray(position.unwrap_or(fallback), rect)
        };
        let captured_settings = local_settings(&self.document, capture.settings);
        let result = apply_continuous_dab(
            &mut self.continuous_clock,
            &mut self.pending_dabs,
            &mut self.document,
            ContinuousDab {
                frame: ctx.cumulative_frame_nr(),
                now_micros: now,
                ready,
                ray: scene_ray,
                settings: captured_settings,
                footprint: capture.footprint,
            },
        );
        if result.attempted {
            self.last_continuous_tick = result.at_micros;
        }
        let rejected = result.rejected;
        self.handle_dab_progress(result.progress);
        let after = self.continuous_clock.report();
        if rejected
            || (after.active_key.is_none()
                && !matches!(
                    after.last_stop,
                    Some(StopReason::NoContact | StopReason::TargetLost)
                ))
        {
            self.finish_stroke_deferred();
            return;
        }
        if let Some(now) = self.continuous_now()
            && let Some(delay) = self.continuous_clock.micros_until_due(now)
        {
            ctx.request_repaint_after(std::time::Duration::from_micros(delay.max(1)));
        }
    }

    fn history(&mut self, redo: bool) {
        self.modal_invalidate_publication("history command");
        self.layer_intensity_preview = None;
        self.layer_name_index = None;
        self.cancel_topology();
        self.finish_stroke();
        let changed = if redo {
            self.document.redo()
        } else {
            self.document.undo()
        };
        if changed {
            self.changed();
            self.message = if redo {
                "Edit restored."
            } else {
                "Edit undone."
            }
            .into();
        }
    }

    fn show_layer_error(&mut self) {
        if let Some(error) = self.document.take_layer_error() {
            // Layer recording can reject an invalid edit at the stroke barrier.
            // Refresh the GPU snapshot after restoring that complete state.
            self.changed();
            self.stroke_rejected = true;
            self.message = format!("Shape edit rejected; sculpt restored: {error}");
        }
    }

    fn layer_action(
        &mut self,
        action: impl FnOnce(&mut Document) -> Result<(), String>,
        message: &str,
    ) {
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        self.layer_intensity_preview = None;
        match action(&mut self.document) {
            Ok(()) => {
                self.changed();
                self.message = message.into();
            }
            Err(error) => self.message = format!("Layer change failed: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    fn track_layer_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.layer_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.layer_controls_enabled
            .insert(key.into(), response.enabled());
        self.layer_controls_hit_rect.record(key, Some(response));
    }

    fn track_morph_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.morph_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.morph_controls_enabled
            .insert(key.into(), response.enabled());
        if key.starts_with("source_paint.") {
            self.source_paint_controls_hit_rect
                .record(key, Some(response));
        }
    }

    fn morph_action(
        &mut self,
        action: impl FnOnce(&mut Document) -> Result<(), String>,
        message: &str,
    ) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.layer_intensity_preview = None;
        match action(&mut self.document) {
            Ok(()) => {
                self.changed();
                self.message = message.into();
            }
            Err(error) => self.message = format!("Morph target change failed: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    fn morph_panel(&mut self, ui: &mut egui::Ui) {
        section(
            ui,
            "07",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C6E0E4B166896),
        );
        ui.add_space(10.0);
        let busy = self.topology_job.is_some() || !self.document.active_subtool_visible();
        let target = self.document.morph_target_summary();
        let available = target.as_ref().is_some_and(|target| target.available);
        if let Some(target) = &target {
            ui.label(
                RichText::new(format!(
                    "Stored on level {} · {} points",
                    target.creation_level + 1,
                    target.vertices
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            if !available {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C41CF01C0046A,
                    ))
                    .size(11.0)
                    .color(preferences_host::muted(ui)),
                );
            }
        } else {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CC7FBCF6BF3A2,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let store = ui
                .add_enabled(
                    !busy,
                    egui::Button::new(if target.is_some() {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C0FFE44A69286)
                    } else {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C5D4BF2363491)
                    }),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7CD6B496C269,
                ));
            self.track_morph_control("morph.store", &store);
            if store.clicked() {
                self.morph_action(
                    Document::store_morph_target,
                    "Shape stored. Morph (0) brushes toward it; Restore recalls the whole shape.",
                );
            }
            let delete = ui
                .add_enabled(
                    !busy && target.is_some(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CDEB97020792A,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CB52A0B8FA0D0,
                ));
            self.track_morph_control("morph.delete", &delete);
            if delete.clicked() {
                self.morph_action(
                    Document::delete_morph_target,
                    "Morph target deleted. Undo restores it.",
                );
            }
        });
        ui.horizontal(|ui| {
            let restore = ui
                .add_enabled(
                    !busy && available,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C7B9FFB1D1AD0,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C412142F1C5A4,
                ));
            self.track_morph_control("morph.restore", &restore);
            if restore.clicked() {
                self.morph_action(
                    Document::restore_morph_target,
                    "Stored shape restored. Undo returns to the sculpt.",
                );
            }
            let swap = ui
                .add_enabled(
                    !busy && available,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C463DA7399217,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C4544F5E90240,
                ));
            self.track_morph_control("morph.swap", &swap);
            if swap.clicked() {
                self.morph_action(
                    Document::swap_morph_target,
                    "Current and stored shapes swapped.",
                );
            }
        });
        if target.is_some() {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C57A4942BA374,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
    }

    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        section(
            ui,
            "06",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CFEF18E1049D0),
        );
        ui.add_space(10.0);
        let busy = self.topology_job.is_some() || !self.document.active_subtool_visible();
        ui.horizontal(|ui| {
            let new = ui.add_enabled(!busy && self.document.layer_count() < 16, egui::Button::new(crate::ui_text::caption(crate::ui_text::CaptionKey::C40616030C681)));
            self.track_layer_control("layers.new", &new);
            if new.clicked() {
                self.finish_stroke();
                if self.stroke_rejected { return; }
                let name = format!("Layer {}", self.document.layer_count() + 1);
                match self.document.add_layer(&name) {
                    Ok(index) => {
                        self.selected_layer = Some(index);
                        self.layer_name_index = None;
                        self.layer_intensity_preview = None;
                        self.changed();
                        self.message = format!("Recording {name}. Sculpt changes stay on this layer.");
                    }
                    Err(error) => self.message = format!("Cannot add layer: {error}"),
                }
            }
            let bake = ui.add_enabled(!busy && self.document.layer_count() > 0, egui::Button::new(crate::ui_text::caption(crate::ui_text::CaptionKey::C8B4642F04BE2)))
                .on_hover_text(crate::ui_text::caption(crate::ui_text::CaptionKey::C1F392D24F45E));
            self.track_layer_control("layers.bake", &bake);
            if bake.clicked() {
                self.layer_action(Document::bake_layers, "Visible shape baked. Subdivision and rebuilding are available; Undo restores layers.");
                self.selected_layer = None;
                self.layer_name_index = None;
            }
        });
        let summaries = self.document.layer_summaries();
        if summaries.is_empty() {
            self.selected_layer = None;
            self.layer_name_index = None;
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CD1E3F59240E2,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            return;
        }
        if self
            .selected_layer
            .is_none_or(|index| index >= summaries.len())
        {
            self.selected_layer = self.document.recording_layer().or(Some(0));
            self.layer_name_index = None;
        }
        ui.add_space(6.0);
        if let Some(index) = self.document.recording_layer() {
            ui.add(
                egui::Label::new(
                    RichText::new(format!("Recording: {}", summaries[index].name))
                        .size(11.0)
                        .color(ACCENT),
                )
                .truncate(),
            )
            .on_hover_text(&summaries[index].name);
            let stop = ui.add_enabled(
                !busy,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C60D9EBD387D1,
                )),
            );
            self.track_layer_control("layers.stop", &stop);
            if stop.clicked() {
                self.layer_action(
                    |document| document.set_recording_layer(None),
                    "Recording stopped. New shape edits change the base; layers stay separate.",
                );
            }
        } else {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C88D8927CDFC5,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
        }
        for layer in &summaries {
            ui.push_id(layer.index, |ui| {
                ui.horizontal(|ui| {
                    let mut visible = layer.visible;
                    let visibility = ui
                        .add_enabled(!busy, egui::Checkbox::without_text(&mut visible))
                        .on_hover_text(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::C92BBDEC9929D,
                        ));
                    self.track_layer_control(
                        &format!("layers.visibility.{}", layer.index),
                        &visibility,
                    );
                    if visibility.changed() {
                        self.layer_action(
                            |document| document.set_layer_visibility(layer.index, visible),
                            "Layer visibility updated. Recording stops when its layer is hidden.",
                        );
                    }
                    let label = format!(
                        "{}{}",
                        layer.name,
                        if layer.recording { " · REC" } else { "" }
                    );
                    let select = ui
                        .add_sized(
                            [ui.available_width(), 24.0],
                            egui::Button::new(egui::RichText::new(label))
                                .selected(self.selected_layer == Some(layer.index))
                                .truncate(),
                        )
                        .on_hover_text(&layer.name);
                    self.track_layer_control(&format!("layers.select.{}", layer.index), &select);
                    if select.clicked() {
                        self.finish_stroke();
                        self.selected_layer = Some(layer.index);
                        self.layer_name_index = None;
                        self.layer_intensity_preview = None;
                    }
                });
            });
        }
        let index = self.selected_layer.unwrap_or(0);
        let Some(layer) = self
            .document
            .layer_summaries()
            .into_iter()
            .find(|layer| layer.index == index)
        else {
            return;
        };
        if self.layer_name_index != Some(index) || self.layer_name_committed != layer.name {
            self.layer_name_index = Some(index);
            self.layer_name_draft = layer.name.clone();
            self.layer_name_committed = layer.name.clone();
        }
        ui.add_space(8.0);
        let mut intensity = self
            .layer_intensity_preview
            .filter(|(i, _)| *i == index)
            .map_or(layer.intensity, |(_, value)| value)
            * 100.0;
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CCE56B4B3A72D),
            &format!("{intensity:.0}%"),
        );
        let slider = ui
            .add_enabled(
                !busy,
                egui::Slider::new(&mut intensity, 0.0..=100.0)
                    .step_by(1.0)
                    .show_value(false),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C2DB8E912D1CD,
            ));
        self.track_layer_control("layers.intensity", &slider);
        if slider.changed() {
            self.layer_intensity_preview = Some((index, intensity / 100.0));
        }
        if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
            self.layer_action(
                |document| document.set_layer_intensity(index, intensity / 100.0),
                "Layer intensity updated. Recording is available at 100% on a visible layer.",
            );
        }
        ui.horizontal(|ui| {
            let name = ui.add_enabled(
                !busy,
                egui::TextEdit::singleline(&mut self.layer_name_draft).desired_width(116.0),
            );
            self.track_layer_control("layers.name", &name);
            let rename = ui.add_enabled(
                !busy && self.layer_name_draft != layer.name,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CAF0C53FFF14B,
                )),
            );
            self.track_layer_control("layers.rename", &rename);
            if rename.clicked()
                || (name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                let draft = self.layer_name_draft.clone();
                self.layer_action(
                    |document| document.set_layer_name(index, &draft),
                    "Layer renamed.",
                );
                self.layer_name_index = None;
            }
        });
        ui.horizontal(|ui| {
            let recording = self.document.recording_layer() == Some(index);
            let record = ui
                .add_enabled(
                    !busy && (recording || (layer.visible && layer.intensity == 1.0)),
                    egui::Button::new(if recording {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C60D9EBD387D1)
                    } else {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C35C9431076A9)
                    }),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C59558E3EEF38,
                ));
            self.track_layer_control("layers.record", &record);
            if record.clicked() {
                let target = if recording { None } else { Some(index) };
                self.layer_action(
                    |document| document.set_recording_layer(target),
                    if recording {
                        "Recording stopped. New shape edits change the base."
                    } else {
                        "Recording selected layer."
                    },
                );
            }
            let delete = ui
                .add_enabled(
                    !busy,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C6D9383ACE6DE,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CEF675F65EAAF,
                ));
            self.track_layer_control("layers.delete", &delete);
            if delete.clicked() {
                self.layer_action(
                    |document| document.delete_layer(index),
                    "Layer deleted. Undo restores it.",
                );
                self.selected_layer = None;
                self.layer_name_index = None;
            }
        });
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C1764AFB17B44,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
    }

    fn region_settings_fingerprint(&self) -> RegionSettingsFingerprint {
        RegionSettingsFingerprint {
            numbers: [
                self.brush.radius,
                self.brush.strength,
                self.brush.alpha_rotation,
                self.brush.color[0],
                self.brush.color[1],
                self.brush.color[2],
                self.viewport_style.color[0],
                self.viewport_style.color[1],
                self.viewport_style.color[2],
                self.extract_settings.threshold,
                self.extract_settings.thickness,
                self.mask_filter_settings.strength,
                self.transform_values.translation.x,
                self.transform_values.translation.y,
                self.transform_values.translation.z,
                self.transform_values.rotation_degrees.x,
                self.transform_values.rotation_degrees.y,
                self.transform_values.rotation_degrees.z,
                self.transform_values.scale,
                self.transform_values.axis_scale.x,
                self.transform_values.axis_scale.y,
                self.transform_values.axis_scale.z,
                self.decimate_ratio,
                self.brush.hardness,
                self.brush.symmetry_offset,
                self.brush_screen_radius,
            ],
            switches: [
                self.brush.invert,
                self.brush.symmetry_x,
                self.brush.pressure_strength,
                self.brush.pressure_radius,
                self.viewport_style.wireframe,
                self.viewport_style.show_mask,
                self.viewport_style.use_vertex_color,
                self.viewport_style.antialias,
                self.brush.backface_culling,
                self.brush.smooth_tangent,
                self.brush.draw_clay,
                self.brush.draw_accumulate,
                self.brush.move_topology_check,
                self.brush.move_along_normal,
                self.brush_screen_space,
                self.brush_lock_position,
                self.extract_whole_faces,
                self.extract_keep_source_active,
                self.extract_sculptgl_surface_offset,
                self.remesh_block_mode,
                self.remesh_surface_nets_mode,
            ],
            integers: [
                self.primitive_detail,
                self.remesh_resolution,
                self.mask_filter_settings.iterations,
            ],
            layer: self.selected_layer,
            layer_preview: self.layer_intensity_preview,
            drafts: [
                self.layer_name_draft.clone(),
                self.subtool_name_draft.clone(),
            ],
            pivot: self.transform_values.pivot,
        }
    }

    fn cancel_mask_region(&mut self, reason: &str, suppress_frame: bool) -> bool {
        let had = clear_mask_region(
            &mut self.mask_region_gesture,
            &mut self.region_suppressed_frame,
            suppress_frame,
        );
        if had {
            self.mask_region_cancel_reason = Some(reason.into());
            self.egui_ctx.request_repaint();
        }
        had
    }

    fn set_mask_selection_mode(&mut self, mode: MaskSelectionMode) {
        self.cancel_mask_region("Mask mode changed; pending selection cancelled.", true);
        // Switching to regions cancels the shader preview before the ordinary
        // command barrier, so it cannot accidentally bake a transform.
        self.cancel_transform_drag();
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        if mode != MaskSelectionMode::Brush && !self.select_brush(BrushKind::Mask) {
            return;
        }
        self.mask_selection_mode = mode;
        if mode != MaskSelectionMode::Brush {
            self.transform_mode = false;
            self.message = format!(
                "{} mask: drag an area on the active part, through depth. Alt at press erases.",
                mode.name()
            );
        } else {
            self.message = "Brush masking selected.".into();
        }
    }

    fn mask_region_panel(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        self.mask_region_clip_rect = [clip.min.x, clip.min.y, clip.max.x, clip.max.y];
        for (mode, key) in [
            (MaskSelectionMode::Brush, "mask.region.brush"),
            (MaskSelectionMode::Rectangle, "mask.region.rectangle"),
            (MaskSelectionMode::Lasso, "mask.region.lasso"),
        ] {
            let button = ui
                .add_enabled_ui(
                    self.document.active_subtool_visible() && self.topology_job.is_none(),
                    |ui| {
                        ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::Button::selectable(self.mask_selection_mode == mode, mode.name()),
                        )
                    },
                )
                .inner;
            let rect = button.rect;
            self.mask_region_controls
                .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
            self.mask_region_controls_enabled
                .insert(key.into(), button.enabled());
            if button.clicked() {
                self.set_mask_selection_mode(mode);
            }
        }
        if self.mask_selection_mode != MaskSelectionMode::Brush {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C87FD6BDFEF42,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        ui.add_space(7.0);
    }

    fn region_masking_context(&self, ctx: &egui::Context) -> bool {
        if self.transform_mode || self.transform_drag.is_some() {
            return false;
        }
        self.mask_region_gesture.is_some()
            || (self.mask_selection_mode != MaskSelectionMode::Brush
                && ctx.input(|input| {
                    effective_brush_kind(self.brush.kind, input.modifiers) == BrushKind::Mask
                }))
    }

    fn finish_mask_region(&mut self, viewport: egui::Rect) {
        let Some(gesture) = self.mask_region_gesture.take() else {
            return;
        };
        if gesture.viewport != viewport
            || Some(gesture.subtool_id) != self.document.active_subtool_id()
            || gesture.revision != self.revision
            || !self.document.active_subtool_visible()
        {
            self.mask_region_cancel_reason =
                Some("The viewport or part changed; pending selection cancelled.".into());
            return;
        }
        let settings = gesture.settings();
        let tiny = gesture.tiny_click();
        let result = if tiny {
            Ok(MaskRegionReport {
                kind: settings.region.kind(),
                erase: gesture.erase,
                vertices: self.document.active_mesh().map_or(0, |m| m.positions.len()),
                projected_vertices: 0,
                selected_vertices: 0,
                changed_vertices: 0,
                polygon_points: if gesture.mode == MaskSelectionMode::Rectangle {
                    4
                } else {
                    gesture.points.len()
                },
                point_edge_work: 0,
                max_abs_change: 0.0,
            })
        } else {
            self.document.mask_region(&settings)
        };
        match result {
            Ok(report) => {
                if report.changed_vertices > 0 {
                    self.changed();
                }
                self.message = if tiny {
                    "Mask selection click made no change. Drag an area to select points.".into()
                } else if report.changed_vertices == 0 {
                    format!(
                        "{} mask selected {} points; no mask change. History kept.",
                        report.kind.name(),
                        number(report.selected_vertices)
                    )
                } else {
                    format!(
                        "{} mask {} {} points on the active part. Undo restores the mask.",
                        report.kind.name(),
                        if report.erase { "erased" } else { "set on" },
                        number(report.changed_vertices)
                    )
                };
                self.last_mask_region_report = Some(HostMaskRegionReport {
                    subtool_id: gesture.subtool_id,
                    revision: self.revision,
                    captured_revision: gesture.revision,
                    settings,
                    report,
                    skipped_tiny_click: tiny,
                });
                self.mask_region_cancel_reason = None;
            }
            Err(error) => {
                self.mask_region_cancel_reason = Some(error.clone());
                self.message = if error.contains("work estimate") {
                    "Selection is too complex for this mesh. Try a shorter lasso or a lower subdivision level.".into()
                } else {
                    format!("Region mask rejected; part kept: {error}")
                };
            }
        }
        self.source_paint_raw.finish();
        self.contact_pressure.finish_release();
        self.brush.pressure = self.contact_pressure.current();
        self.egui_ctx.request_repaint();
    }

    /// Ordered raw events preserve press modifiers/coordinates even when Ctrl
    /// and the mouse are released before this frame's final pointer state.
    fn mask_region_input(&mut self, ctx: &egui::Context, response: &egui::Response) -> bool {
        let rect = response.rect;
        let (events, down, focused, delta, scroll) = ctx.input(|input| {
            (
                input.events.clone(),
                input.pointer.primary_down(),
                input.focused,
                input.pointer.delta(),
                input.raw_scroll_delta.y,
            )
        });
        let requested_press = events.iter().any(|event| {
            region_press_requested(event, rect, self.mask_selection_mode, self.brush.kind)
        });
        let context = self.region_masking_context(ctx);
        // A command can also change the selected mode or brush. Suppression
        // precedes routing so that its old press/release cannot become a new
        // ordinary sculpt gesture in this same frame.
        if self.region_suppressed_frame {
            return true;
        }
        if self.mask_region_gesture.is_none()
            && (!requested_press || self.mask_selection_mode == MaskSelectionMode::Brush)
            && !context
        {
            return false;
        }
        if !focused {
            self.cancel_mask_region("Focus lost; pending selection cancelled.", true);
            return true;
        }
        if self.mask_region_gesture.as_ref().is_some_and(|gesture| {
            gesture.viewport != rect
                || Some(gesture.subtool_id) != self.document.active_subtool_id()
                || gesture.revision != self.revision
        }) {
            self.cancel_mask_region(
                "Viewport or part changed; pending selection cancelled.",
                true,
            );
            return true;
        }
        let navigation =
            ctx.input(|input| viewport_navigation_requested(input, self.navigation_down()));
        if self.mask_region_gesture.is_some() && navigation {
            self.cancel_mask_region("Camera navigation cancelled the pending selection.", true);
            return true;
        }
        if self.mask_region_gesture.is_none() {
            self.navigate_camera(ctx, response, delta, scroll);
        }
        for event in events {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } => {
                    if self.mask_region_gesture.is_some() {
                        self.cancel_mask_region(
                            "Another press cancelled the pending selection.",
                            true,
                        );
                        break;
                    }
                    if self.mask_selection_mode == MaskSelectionMode::Brush
                        || effective_brush_kind(self.brush.kind, modifiers) != BrushKind::Mask
                        || !rect.contains(pos)
                        || navigation
                    {
                        continue;
                    }
                    if ctx
                        .layer_id_at(pos)
                        .is_some_and(|layer| layer != response.layer_id)
                    {
                        continue;
                    }
                    if self.topology_job.is_some() || !self.document.active_subtool_visible() {
                        self.message="Show the active part and finish any surface operation before region masking.".into();
                        continue;
                    }
                    self.finish_stroke_inner();
                    if self.stroke_rejected {
                        return true;
                    }
                    match MaskRegionGesture::new(
                        self.mask_selection_mode,
                        rect,
                        match self.active_local_projection(rect) {
                            Ok(matrix) => matrix,
                            Err(error) => {
                                self.message = error;
                                return true;
                            }
                        },
                        match self.document.active_subtool_id() {
                            Some(id) => id,
                            None => return true,
                        },
                        self.revision,
                        pos,
                        modifiers.alt,
                    ) {
                        Ok(gesture) => {
                            self.mask_region_gesture = Some(gesture);
                            self.mask_region_cancel_reason = None;
                        }
                        Err(error) => {
                            self.mask_region_cancel_reason = Some(error.clone());
                            self.message = error;
                            self.region_suppressed_frame = true;
                            return true;
                        }
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(gesture) = self.mask_region_gesture.as_mut()
                        && let Err(error) = gesture.move_to(pos)
                    {
                        self.cancel_mask_region(&error, true);
                        self.message = error;
                        return true;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(gesture) = self.mask_region_gesture.as_mut() {
                        if let Err(error) = gesture.release_at(pos) {
                            self.cancel_mask_region(&error, true);
                            self.message = error;
                            return true;
                        }
                        self.finish_mask_region(rect);
                    }
                }
                egui::Event::PointerGone if self.mask_region_gesture.is_some() => {
                    self.cancel_mask_region(
                        "Pointer contact lost; pending selection cancelled.",
                        true,
                    );
                    break;
                }
                _ => {}
            }
        }
        if self.mask_region_gesture.is_some() && !down {
            self.cancel_mask_region(
                "Contact lost without release; pending selection cancelled.",
                true,
            );
        }
        true
    }

    fn draw_mask_region_overlay(&self, painter: &egui::Painter) {
        let Some(gesture) = &self.mask_region_gesture else {
            return;
        };
        let color = if gesture.erase {
            Color32::from_rgb(234, 98, 98)
        } else {
            ACCENT
        };
        if gesture.mode == MaskSelectionMode::Rectangle {
            let rect = egui::Rect::from_two_pos(gesture.start, gesture.end);
            painter.rect_filled(rect, 0.0, color.gamma_multiply(0.12));
            painter.rect_stroke(rect, 0.0, Stroke::new(1.5, color), egui::StrokeKind::Inside);
        } else {
            let mut points = gesture.points.clone();
            if points.len() > 1 {
                points.push(gesture.start);
                painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
            }
        }
    }

    fn track_mask_filter_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.mask_filter_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.mask_filter_controls_enabled
            .insert(key.into(), response.enabled());
    }

    fn apply_mask_filter(&mut self, operation: MaskFilterOperation) {
        if let Some(reason) = mask_filter_unavailable_reason(
            &self.document,
            self.topology_job.is_some(),
            self.transform_drag.is_some(),
        ) {
            self.message = reason.into();
            return;
        }
        self.finish_stroke();
        let settings = MaskFilterSettings {
            operation,
            iterations: self.mask_filter_settings.iterations,
            strength: self.mask_filter_settings.strength,
        };
        let subtool_id = self.document.active_subtool_id();
        match mask_filter_after_barrier(
            &mut self.document,
            &settings,
            self.stroke_rejected,
            self.mask_filter_sculptgl_response,
        ) {
            Ok(report) => {
                if report.changed_vertices > 0 {
                    self.changed();
                    self.message = format!(
                        "{} changed the mask on {} points. Shape and paint kept; Undo restores the mask.",
                        operation.name(),
                        number(report.changed_vertices)
                    );
                } else {
                    self.message = format!(
                        "{} made no mask change. Shape and history kept.",
                        operation.name()
                    );
                }
                self.last_mask_filter_report = Some(HostMaskFilterReport {
                    subtool_id,
                    revision: self.revision,
                    report,
                });
            }
            Err(error) => self.message = format!("Mask filter rejected; part preserved: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    fn mask_filter_panel(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        self.mask_filter_clip_rect = [clip.min.x, clip.min.y, clip.max.x, clip.max.y];
        let unavailable = mask_filter_unavailable_reason(
            &self.document,
            self.topology_job.is_some(),
            self.transform_drag.is_some(),
        );
        let enabled = unavailable.is_none();
        ui.add_space(8.0);
        let response=ui.add_enabled(enabled,egui::Checkbox::new(&mut self.mask_filter_sculptgl_response,"SculptGL Blur / Sharpen response"))
            .on_hover_text("Use SculptGL's one-pass mask Blur and Sharpen rules. Grow and Shrink keep OBrush's graph policy.");
        self.track_mask_filter_control("mask.sculptgl_response", &response);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C9315CD299160,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let width = ui.available_width();
            let passes = ui
                .add_enabled_ui(enabled && !self.mask_filter_sculptgl_response, |ui| {
                    ui.add_sized(
                        [width, 22.0],
                        egui::DragValue::new(&mut self.mask_filter_settings.iterations)
                            .range(1..=16)
                            .speed(1.0),
                    )
                })
                .inner
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CAF37755AFA12,
                ));
            self.track_mask_filter_control("mask.passes", &passes);
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7150E23974A0,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let width = ui.available_width();
            let amount = ui
                .add_enabled_ui(enabled && !self.mask_filter_sculptgl_response, |ui| {
                    ui.add_sized(
                        [width, 22.0],
                        egui::DragValue::new(&mut self.mask_filter_settings.strength)
                            .range(0.0..=1.0)
                            .speed(0.01)
                            .max_decimals(6),
                    )
                })
                .inner
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C971BB02CB270,
                ));
            self.track_mask_filter_control("mask.amount", &amount);
        });
        for operations in [
            [
                (
                    MaskFilterOperation::Grow,
                    "mask.grow",
                    "Expand the masked area.",
                ),
                (
                    MaskFilterOperation::Shrink,
                    "mask.shrink",
                    "Contract the masked area.",
                ),
            ],
            [
                (MaskFilterOperation::Blur, "mask.blur", "Soften the mask."),
                (
                    MaskFilterOperation::Sharpen,
                    "mask.sharpen",
                    "Increase contrast in the mask.",
                ),
            ],
        ] {
            ui.horizontal(|ui| {
                let width = ((ui.available_width() - ui.spacing().item_spacing.x) * 0.5).max(1.0);
                for (operation, key, hint) in operations {
                    let button = ui
                        .add_enabled_ui(enabled, |ui| {
                            ui.add_sized(
                                [width, 25.0],
                                egui::Button::new(crate::ui_text::mask_filter_name(operation)),
                            )
                        })
                        .inner
                        .on_hover_text(hint);
                    self.track_mask_filter_control(key, &button);
                    if button.clicked() {
                        self.apply_mask_filter(operation);
                    }
                }
            });
        }
        if let Some(reason) = unavailable {
            ui.label(
                RichText::new(reason)
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
            );
        }
        if let Some(record) = &self.last_mask_filter_report {
            ui.label(
                RichText::new(format!(
                    "Last {}: {} points changed",
                    record.report.operation.name(),
                    number(record.report.changed_vertices)
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
    }

    fn track_extract_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.extract_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.extract_controls_enabled
            .insert(key.into(), response.enabled());
    }

    fn create_mask_extract(&mut self) {
        // A live transform is an unfinished separate gesture, and this button
        // is disabled until it is released/cancelled. Never bake it implicitly.
        if let Some(reason) = mask_extract_unavailable_reason(
            &self.document,
            self.topology_job.is_some(),
            self.transform_drag.is_some(),
        ) {
            self.message = reason.into();
            return;
        }
        self.finish_stroke();
        self.cancel_topology();
        let source_subtool_id = self.document.active_subtool_id();
        let selection = if self.extract_whole_faces {
            MaskExtractSelection::WholeIncidentFaces
        } else {
            MaskExtractSelection::Clipped
        };
        match mask_extract_after_barrier_with_selection(
            &mut self.document,
            &self.extract_settings,
            selection,
            self.extract_keep_source_active,
            self.extract_sculptgl_surface_offset,
            self.stroke_rejected,
        ) {
            Ok((new_subtool_id, report)) => {
                self.cancel_transform_drag();
                self.layer_intensity_preview = None;
                self.changed();
                self.message = format!(
                    "Created a separate part: {} points, {} faces. Original and mask kept; Undo restores the scene.",
                    number(report.output_vertices),
                    number(report.output_triangles)
                );
                self.last_extract_report = Some(HostMaskExtractReport {
                    source_subtool_id,
                    new_subtool_id,
                    revision: self.revision,
                    report,
                });
            }
            Err(error) => self.message = format!("Extraction rejected; original kept: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    fn mask_extract_panel(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        self.extract_clip_rect = [clip.min.x, clip.min.y, clip.max.x, clip.max.y];
        section(
            ui,
            "",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C9447B1539BDA),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C4BAB4B66A689,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        let unavailable = mask_extract_unavailable_reason(
            &self.document,
            self.topology_job.is_some(),
            self.transform_drag.is_some(),
        );
        let enabled = unavailable.is_none();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CCCFE760A4456,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let width = ui.available_width();
            let threshold = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [width, 22.0],
                        egui::DragValue::new(&mut self.extract_settings.threshold)
                            .range(0.01..=0.99)
                            .speed(0.01)
                            .max_decimals(6),
                    )
                })
                .inner
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C721F6BEF7054,
                ));
            self.track_extract_control("extract.threshold", &threshold);
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C88AD2ABFC83B,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let width = ui.available_width();
            let thickness = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [width, 22.0],
                        egui::DragValue::new(&mut self.extract_settings.thickness)
                            .range(-5.0..=5.0)
                            .speed(0.001)
                            .max_decimals(6),
                    )
                })
                .inner
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C792725753B7A,
                ));
            self.track_extract_control("extract.thickness", &thickness);
        });
        let whole_faces = ui
            .add_enabled_ui(enabled, |ui| {
                ui.checkbox(&mut self.extract_whole_faces, "Whole incident faces")
            })
            .inner
            .on_hover_text("Include every face touched by a mask-qualified point. This changes selection scope; reference offset and border smoothing remain separate work.");
        self.track_extract_control("extract.whole_faces", &whole_faces);
        if !self.extract_whole_faces {
            self.extract_sculptgl_surface_offset = false;
        }
        let keep_source = ui
            .add_enabled_ui(enabled, |ui| {
                ui.checkbox(&mut self.extract_keep_source_active, "Keep source selected")
            })
            .inner
            .on_hover_text("Create the extracted part without changing the active source. The source mask remains available for another operation.");
        self.track_extract_control("extract.keep_source", &keep_source);
        let surface_offset = ui
            .add_enabled_ui(enabled && self.extract_whole_faces, |ui| {
                ui.checkbox(&mut self.extract_sculptgl_surface_offset, "SculptGL surface offset")
            })
            .inner
            .on_hover_text("Offset the selected surface 0.01 units along face-averaged source normals and start the new part with white color and default material. Closed triangle sources can receive three border-smoothing passes; open or quad sources keep a checked closed shell.");
        self.track_extract_control("extract.surface_offset", &surface_offset);
        let create = ui
            .add_enabled_ui(enabled, |ui| {
                ui.add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C5C53594D8A50,
                    )),
                )
            })
            .inner
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C8F9C38536FE0,
            ));
        self.track_extract_control("extract.create", &create);
        if create.clicked() {
            self.create_mask_extract();
        }
        if let Some(reason) = unavailable {
            ui.label(RichText::new(reason).size(10.0).color(ACCENT));
        } else {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C170F7F889B31,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        if let Some(record) = &self.last_extract_report {
            ui.label(
                RichText::new(format!(
                    "Last extraction: {} points, {} faces",
                    number(record.report.output_vertices),
                    number(record.report.output_triangles)
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        ui.add_space(20.0);
    }

    fn transform_pivot(&self) -> Vec3 {
        if self.transform_values.pivot == TransformPivot::Origin {
            Vec3::ZERO
        } else {
            self.document
                .selected_subtools_center()
                .unwrap_or(Vec3::ZERO)
        }
    }

    fn cancel_transform_drag(&mut self) {
        if clear_transform_preview(
            &mut self.transform_drag,
            &mut self.unprocessed_transform_release,
            &mut self.viewport_style.active_preview_translation,
            &mut self.viewport_style.active_preview_scale,
            &mut self.viewport_style.active_preview_rotation,
            &mut self.viewport_style.active_preview_pivot,
        ) {
            self.egui_ctx.request_repaint();
        }
    }

    /// Command barrier: finish the last displayed absolute offset exactly once,
    /// including a pointer-up delivered before this frame's viewport layout.
    fn finish_transform_drag(&mut self) -> bool {
        let Some(mut drag) = self.transform_drag.take() else {
            return true;
        };
        if let Some(position) = self
            .unprocessed_transform_release
            .take()
            .or_else(|| self.egui_ctx.input(|input| input.pointer.interact_pos()))
        {
            drag.gesture.update(position);
            if let Some(scale_drag) = drag.scale_drag.as_mut() {
                scale_drag.update(position);
            }
            if let Some(rotation_drag) = drag.rotation_drag.as_mut() {
                rotation_drag.update(position);
            }
        }
        self.viewport_style.active_preview_translation = Vec3::ZERO;
        self.viewport_style.active_preview_scale = Vec3::ONE;
        self.viewport_style.active_preview_rotation = Vec3::ZERO;
        self.viewport_style.active_preview_pivot = Vec3::ZERO;
        if Some(drag.subtool_id) != self.document.active_subtool_id()
            || drag.selected_ids != self.document.selected_subtool_ids()
            || drag.revision != self.revision
        {
            self.message = "Transform cancelled because the selected shapes changed.".into();
            return false;
        }
        let edit = match drag.kind {
            GizmoKind::Translate | GizmoKind::Plane => {
                let transform = ObjectTransform {
                    translation: drag.gesture.translation,
                    ..Default::default()
                };
                if drag.mask_weighted {
                    self.document
                        .translate_selected_subtools_masked(transform.translation)
                } else {
                    self.document.transform_selected_subtools(&transform)
                }
            }
            GizmoKind::Scale | GizmoKind::UniformScale => self
                .document
                .scale_selected_subtools_axis(drag.axis_scale(), drag.pivot, drag.mask_weighted),
            GizmoKind::Rotate => {
                let transform = ObjectTransform {
                    rotation_degrees: drag.rotation_degrees(),
                    pivot: drag.pivot,
                    ..Default::default()
                };
                if drag.mask_weighted {
                    self.document.transform_selected_subtools_masked(&transform)
                } else {
                    self.document.transform_selected_subtools(&transform)
                }
            }
        };
        match edit {
            Ok(changed) => {
                if changed {
                    self.changed();
                    self.message = format!(
                        "{} {} selected part(s). One Undo restores their complete shapes.",
                        if matches!(drag.kind, GizmoKind::Scale | GizmoKind::UniformScale) {
                            "Scaled"
                        } else if drag.kind == GizmoKind::Rotate {
                            "Rotated"
                        } else {
                            "Moved"
                        },
                        drag.selected_ids.len()
                    );
                }
                true
            }
            Err(error) => {
                self.message = format!("Transform rejected; part preserved: {error}");
                false
            }
        }
    }

    fn toggle_transform_mode(&mut self) {
        self.modal_cancel("transform command");
        // Mode toggles cancel a live preview, unlike document commands which
        // commit it. No transform history is created by entering/leaving mode.
        self.cancel_transform_drag();
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.transform_mode = !self.transform_mode;
        self.message = if self.transform_mode {
            "Transform mode: arrows move one axis, colored pads move in a plane, arcs rotate, squares scale one axis, orange ring scales uniformly, center moves freely. T returns to sculpting; Esc cancels.".into()
        } else {
            "Sculpt mode.".into()
        };
    }

    fn apply_numeric_transform(&mut self) {
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        self.cancel_topology();
        let transform = ObjectTransform {
            translation: self.transform_values.translation,
            rotation_degrees: self.transform_values.rotation_degrees,
            scale: self.transform_values.scale,
            pivot: self.transform_pivot(),
        };
        let selected_count = self.document.selected_subtool_ids().len();
        let axis_scale = self.transform_values.axis_scale;
        let edit = if axis_scale != Vec3::ONE {
            if transform.translation != Vec3::ZERO
                || transform.rotation_degrees != Vec3::ZERO
                || transform.scale != 1.0
            {
                Err(
                    "Apply axis scale separately from translation, rotation and uniform scale"
                        .into(),
                )
            } else {
                self.document.scale_selected_subtools_axis(
                    axis_scale,
                    transform.pivot,
                    self.transform_respect_masks,
                )
            }
        } else if self.transform_respect_masks {
            self.document.transform_selected_subtools_masked(&transform)
        } else {
            self.document.transform_selected_subtools(&transform)
        };
        match edit {
            Ok(changed) => {
                if changed {
                    self.changed();
                    self.message = if self.transform_respect_masks {
                        format!(
                            "Mask-weighted transform changed {selected_count} selected part(s). One Undo restores their complete shapes."
                        )
                    } else {
                        format!(
                            "Transformed {selected_count} selected part(s). Levels, layers, target, paint and masks stay attached; one Undo restores them."
                        )
                    };
                } else {
                    self.message = "Transform made no shape change.".into();
                }
                let pivot = self.transform_values.pivot;
                self.transform_values = TransformValues {
                    pivot,
                    ..Default::default()
                };
            }
            Err(error) => self.message = format!("Transform rejected; scene preserved: {error}"),
        }
        self.egui_ctx.request_repaint();
    }

    fn track_transform_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.transform_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.transform_controls_enabled
            .insert(key.into(), response.enabled());
    }

    fn transform_panel(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        self.transform_clip_rect = [clip.min.x, clip.min.y, clip.max.x, clip.max.y];
        section(
            ui,
            "",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C4E05C6BB6EB2),
        );
        ui.add_space(8.0);
        let mut mode = self.transform_mode;
        let control = ui.checkbox(
            &mut mode,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C92C0B43B9C6D),
        );
        let command = if self.source_shortcut_profile() {
            crate::shortcut_profile::Command::SelectTransform
        } else {
            crate::shortcut_profile::Command::ToggleTransform
        };
        let control = control.on_hover_text(
            crate::ui_text::command_hint(command)
                .unwrap_or_else(|| "No assigned shortcut in this profile".to_owned()),
        );
        self.track_transform_control("transform.mode", &control);
        if control.changed() {
            self.toggle_transform_mode();
        }
        let mut respect_masks = self.transform_respect_masks;
        let control = ui.add_enabled(
            self.transform_drag.is_none(),
            egui::Checkbox::new(&mut respect_masks, "Respect vertex masks"),
        ).on_hover_text("Mask-weighted translation, rotation, uniform and world-axis scale commit the unprotected fraction of each vertex. The live gizmo previews a rigid edit matrix; release applies the weights. Retained levels, layers and Morph Targets need a matched transport policy.");
        self.track_transform_control("transform.respect_masks", &control);
        if control.changed() {
            self.transform_respect_masks = respect_masks;
        }
        let enabled = self.document.active_subtool_visible()
            && self.topology_job.is_none()
            && self.transform_drag.is_none();
        for (label, rotation) in [
            ("Translate", false),
            ("Rotate · world X, then Y, then Z", true),
        ] {
            ui.label(
                RichText::new(label)
                    .size(11.0)
                    .color(preferences_host::muted(ui)),
            );
            for (index, axis) in ["X", "Y", "Z"].iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(*axis)
                            .monospace()
                            .color(preferences_host::muted(ui)),
                    );
                    let available = ui.available_width();
                    let value = if rotation {
                        &mut self.transform_values.rotation_degrees[index]
                    } else {
                        &mut self.transform_values.translation[index]
                    };
                    let range = if rotation {
                        -36000.0..=36000.0
                    } else {
                        -1_000_000.0..=1_000_000.0
                    };
                    let field = ui
                        .add_enabled_ui(enabled, |ui| {
                            ui.add_sized(
                                [available, 22.0],
                                egui::DragValue::new(value)
                                    .speed(if rotation { 0.5 } else { 0.01 })
                                    .range(range)
                                    .max_decimals(6),
                            )
                        })
                        .inner
                        .on_hover_text(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::C9A75588A825E,
                        ));
                    self.track_transform_control(
                        &format!(
                            "transform.{}{}",
                            if rotation { "r" } else { "t" },
                            axis.to_ascii_lowercase()
                        ),
                        &field,
                    );
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CA903EAB2C738,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let width = ui.available_width();
            let scale = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [width, 22.0],
                        egui::DragValue::new(&mut self.transform_values.scale)
                            .speed(0.01)
                            .range(0.001..=1000.0)
                            .max_decimals(6),
                    )
                })
                .inner
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C79723D0D02E6,
                ));
            self.track_transform_control("transform.scale", &scale);
        });
        ui.label(
            RichText::new("Scale axes · world X / Y / Z")
                .size(11.0)
                .color(preferences_host::muted(ui)),
        );
        for (index, axis) in ["X", "Y", "Z"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(*axis)
                        .monospace()
                        .color(preferences_host::muted(ui)),
                );
                let width = ui.available_width();
                let field = ui
                    .add_enabled_ui(enabled, |ui| {
                        ui.add_sized(
                            [width, 22.0],
                            egui::DragValue::new(&mut self.transform_values.axis_scale[index])
                                .speed(0.01)
                                .range(0.001..=1000.0)
                                .max_decimals(6),
                        )
                    })
                    .inner
                    .on_hover_text("Positive scale around the selected world pivot. Use one axis to match an axis-scale handle.");
                self.track_transform_control(&format!("transform.s{}", axis.to_ascii_lowercase()), &field);
            });
        }
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C23398E36577D,
            ))
            .size(11.0)
            .color(preferences_host::muted(ui)),
        );
        for (kind, key, label) in [
            (
                TransformPivot::Center,
                "transform.pivot_center",
                "Selection center",
            ),
            (
                TransformPivot::Origin,
                "transform.pivot_origin",
                "World origin",
            ),
        ] {
            let control = ui.add_enabled(
                enabled,
                egui::Button::selectable(self.transform_values.pivot == kind, label),
            );
            self.track_transform_control(key, &control);
            if control.clicked() {
                self.transform_values.pivot = kind;
            }
        }
        ui.horizontal(|ui| {
            let apply = ui.add_enabled(
                enabled,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CC3F4196857EC,
                )),
            );
            self.track_transform_control("transform.apply", &apply);
            if apply.clicked() {
                self.apply_numeric_transform();
            }
            let reset = ui.button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CCFB20D20D92C,
            ));
            self.track_transform_control("transform.reset", &reset);
            if reset.clicked() {
                self.cancel_transform_drag();
                self.transform_values = TransformValues::default();
            }
        });
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C6C6414320FCC,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        ui.add_space(20.0);
    }

    fn make_gizmo_handles(&self, rect: egui::Rect) -> Vec<GizmoHandle> {
        if !self.transform_mode || !self.document.active_subtool_visible() {
            return Vec::new();
        }
        let pivot = self.transform_pivot() + self.viewport_style.active_preview_translation;
        let Some(start) = project_world(&self.camera, rect, pivot) else {
            return Vec::new();
        };
        let forward = if self.camera.source_enabled() {
            self.camera.axes().2
        } else {
            (self.camera.target - self.camera.position()).normalize_or_zero()
        };
        let depth = (pivot - self.camera.position()).dot(forward);
        if depth <= 0.0 || !depth.is_finite() {
            return Vec::new();
        }
        let units_per_pixel =
            self.camera
                .world_units_per_pixel(depth, rect.height(), rect.aspect_ratio());
        if !units_per_pixel.is_finite() || units_per_pixel <= 0.0 {
            return Vec::new();
        }
        let length = units_per_pixel * 85.0;
        let mut handles = Vec::with_capacity(14);
        for (kind, handle) in [
            (GizmoKind::Scale, TranslationHandle::X),
            (GizmoKind::Scale, TranslationHandle::Y),
            (GizmoKind::Scale, TranslationHandle::Z),
            (GizmoKind::Translate, TranslationHandle::X),
            (GizmoKind::Translate, TranslationHandle::Y),
            (GizmoKind::Translate, TranslationHandle::Z),
            (GizmoKind::Translate, TranslationHandle::Free),
            (GizmoKind::UniformScale, TranslationHandle::Free),
        ] {
            let handle_length = match kind {
                GizmoKind::Scale => length * 1.48,
                GizmoKind::UniformScale => length * 1.48 * 1.3,
                GizmoKind::Translate | GizmoKind::Plane | GizmoKind::Rotate => length,
            };
            let end = if kind == GizmoKind::UniformScale {
                start
                    + egui::vec2(
                        handle_length / units_per_pixel / 2.0_f32.sqrt(),
                        -handle_length / units_per_pixel / 2.0_f32.sqrt(),
                    )
            } else {
                handle
                    .axis()
                    .and_then(|axis| {
                        project_world(&self.camera, rect, pivot + axis * handle_length)
                    })
                    .unwrap_or(start)
            };
            let handle_rect = if kind == GizmoKind::UniformScale {
                let diameter = 2.0 * handle_length / units_per_pixel + 18.0;
                egui::Rect::from_center_size(start, egui::vec2(diameter, diameter))
            } else {
                egui::Rect::from_center_size(end, egui::vec2(18.0, 18.0))
            };
            let enabled = TranslationDrag::new(&self.camera, rect, end, pivot, handle).is_some()
                && (handle == TranslationHandle::Free || start.distance(end) >= 16.0)
                && rect.contains_rect(handle_rect)
                && (!matches!(kind, GizmoKind::Scale | GizmoKind::UniformScale)
                    || ScaleDrag::new(start, end, handle.axis().map(|_| end)).is_some());
            handles.push(GizmoHandle {
                handle,
                kind,
                rect: handle_rect,
                start,
                end,
                enabled,
                pivot,
                length: handle_length,
                quad: None,
                arc: None,
            });
        }
        for (handle, first, second) in [
            (TranslationHandle::PlaneX, Vec3::Y, Vec3::Z),
            (TranslationHandle::PlaneY, Vec3::X, Vec3::Z),
            (TranslationHandle::PlaneZ, Vec3::X, Vec3::Y),
        ] {
            let Some(normal) = handle.plane_normal() else {
                continue;
            };
            if forward.dot(normal).abs() < 0.15 {
                continue;
            }
            let inner = length * 0.23;
            let outer = length * 0.46;
            let corners = [
                pivot + first * inner + second * inner,
                pivot + first * outer + second * inner,
                pivot + first * outer + second * outer,
                pivot + first * inner + second * outer,
            ];
            let Some(quad) = corners
                .map(|point| project_world(&self.camera, rect, point))
                .into_iter()
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let quad = [quad[0], quad[1], quad[2], quad[3]];
            let end = Pos2::new(
                quad.iter().map(|p| p.x).sum::<f32>() * 0.25,
                quad.iter().map(|p| p.y).sum::<f32>() * 0.25,
            );
            let bounds = egui::Rect::from_min_max(
                Pos2::new(
                    quad.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                    quad.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
                ),
                Pos2::new(
                    quad.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                    quad.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
                ),
            );
            let enabled = rect.contains_rect(bounds)
                && TranslationDrag::new(&self.camera, rect, end, pivot, handle).is_some();
            handles.push(GizmoHandle {
                handle,
                kind: GizmoKind::Plane,
                rect: bounds,
                start,
                end,
                enabled,
                pivot,
                length,
                quad: Some(quad),
                arc: None,
            });
        }
        for (handle, first, second) in [
            (TranslationHandle::X, Vec3::Y, Vec3::Z),
            (TranslationHandle::Y, Vec3::X, Vec3::Z),
            (TranslationHandle::Z, Vec3::X, Vec3::Y),
        ] {
            let radius = length * 0.76;
            let Some(arc) = (0..=64)
                .map(|step| {
                    let angle = std::f32::consts::TAU * step as f32 / 64.0;
                    project_world(
                        &self.camera,
                        rect,
                        pivot + radius * (first * angle.cos() + second * angle.sin()),
                    )
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let end = arc[8];
            let bounds = egui::Rect::from_min_max(
                Pos2::new(
                    arc.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                    arc.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
                ),
                Pos2::new(
                    arc.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                    arc.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
                ),
            );
            let enabled = rect.contains_rect(bounds)
                && arc.windows(2).any(|pair| pair[0].distance(pair[1]) > 1.0);
            handles.push(GizmoHandle {
                handle,
                kind: GizmoKind::Rotate,
                rect: bounds,
                start,
                end,
                enabled,
                pivot,
                length: radius,
                quad: None,
                arc: Some(arc),
            });
        }
        handles
    }

    fn transform_input(&mut self, ctx: &egui::Context, response: &egui::Response) {
        let rect = response.rect;
        let (pos, pressed, down, released, focused, delta, scroll) = ctx.input(|input| {
            (
                input.pointer.interact_pos(),
                input.pointer.primary_pressed(),
                input.pointer.primary_down(),
                input.pointer.primary_released(),
                input.focused,
                input.pointer.delta(),
                input.raw_scroll_delta.y,
            )
        });
        if !focused {
            self.cancel_transform_drag();
            return;
        }
        if self.transform_drag.is_some() {
            let point = self.unprocessed_transform_release.or(pos);
            if let Some(point) = point
                && let Some(drag) = self.transform_drag.as_mut()
            {
                drag.gesture.update(point);
                if let Some(scale_drag) = drag.scale_drag.as_mut() {
                    scale_drag.update(point);
                }
                if let Some(rotation_drag) = drag.rotation_drag.as_mut() {
                    rotation_drag.update(point);
                }
                if matches!(drag.kind, GizmoKind::Scale | GizmoKind::UniformScale) {
                    self.viewport_style.active_preview_scale = drag.axis_scale();
                } else if drag.kind == GizmoKind::Rotate {
                    self.viewport_style.active_preview_rotation = drag.rotation_degrees();
                } else {
                    self.viewport_style.active_preview_translation = drag.gesture.translation;
                }
            }
            if released {
                self.finish_transform_drag();
            } else if !down {
                // Losing contact without an actual release event cancels.
                self.cancel_transform_drag();
            }
            return;
        }
        self.navigate_camera(ctx, response, delta, scroll);
        if !pressed
            || !response.hovered()
            || ctx.input(|input| viewport_navigation_requested(input, self.navigation_down()))
        {
            return;
        }
        let Some(pos) = pos else {
            return;
        };
        let handles = self.make_gizmo_handles(rect);
        let selected = handles
            .iter()
            .rev()
            .find(|handle| {
                handle.kind == GizmoKind::Plane && handle.enabled && handle.contains(pos)
            })
            .or_else(|| {
                // The free handle retains priority over axis handles; thin
                // rotation arcs yield to the filled plane pads above.
                handles
                    .iter()
                    .rev()
                    .find(|handle| handle.enabled && handle.contains(pos))
            })
            .cloned();
        if let Some(handle) = selected {
            self.finish_stroke();
            if self.stroke_rejected {
                return;
            }
            self.cancel_topology();
            if let Some(mut gesture) = TranslationDrag::new(
                &self.camera,
                rect,
                pos,
                handle.pivot,
                if handle.kind == GizmoKind::Rotate {
                    TranslationHandle::Free
                } else {
                    handle.handle
                },
            ) {
                if handle.kind == GizmoKind::Rotate {
                    gesture.handle = handle.handle;
                }
                let rotation_drag = if handle.kind == GizmoKind::Rotate {
                    handle
                        .rotation_tangent(pos)
                        .and_then(|tangent| RotationDrag::new(rect, pos, tangent))
                } else {
                    None
                };
                if handle.kind == GizmoKind::Rotate && rotation_drag.is_none() {
                    return;
                }
                self.transform_drag = Some(ActiveTransformDrag {
                    gesture,
                    scale_drag: if matches!(
                        handle.kind,
                        GizmoKind::Translate | GizmoKind::Plane | GizmoKind::Rotate
                    ) {
                        None
                    } else {
                        ScaleDrag::new(handle.start, pos, handle.handle.axis().map(|_| handle.end))
                    },
                    rotation_drag,
                    kind: handle.kind,
                    pivot: handle.pivot,
                    subtool_id: match self.document.active_subtool_id() {
                        Some(id) => id,
                        None => return,
                    },
                    selected_ids: self.document.selected_subtool_ids(),
                    mask_weighted: self.transform_respect_masks,
                    revision: self.revision,
                });
                self.viewport_style.active_preview_translation = Vec3::ZERO;
                self.viewport_style.active_preview_scale = Vec3::ONE;
                self.viewport_style.active_preview_rotation = Vec3::ZERO;
                self.viewport_style.active_preview_pivot = handle.pivot;
            }
        } else if let Some(hit) = self.scene_pick(pos, rect)
            && Some(hit.id) != self.document.active_subtool_id()
        {
            self.scene_action(
                |document| document.select_subtool(hit.id),
                "Active part selected for transforms.",
            );
        }
    }

    fn draw_transform_gizmo(&mut self, painter: &egui::Painter, rect: egui::Rect) {
        self.gizmo_handles = self.make_gizmo_handles(rect);
        if self.transform_drag.is_some() {
            painter.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if painter
            .ctx()
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|pos| {
                self.gizmo_handles
                    .iter()
                    .any(|handle| handle.enabled && handle.contains(pos))
            })
        {
            painter.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        self.gizmo_clip_rect = [rect.min.x, rect.min.y, rect.max.x, rect.max.y];
        for handle in &self.gizmo_handles {
            let color = if !handle.enabled {
                MUTED
            } else if handle.kind == GizmoKind::UniformScale {
                Color32::from_rgb(238, 153, 94)
            } else {
                match handle.handle {
                    TranslationHandle::X | TranslationHandle::PlaneX => {
                        Color32::from_rgb(234, 98, 98)
                    }
                    TranslationHandle::Y | TranslationHandle::PlaneY => {
                        Color32::from_rgb(105, 208, 128)
                    }
                    TranslationHandle::Z | TranslationHandle::PlaneZ => {
                        Color32::from_rgb(100, 153, 238)
                    }
                    TranslationHandle::Free => ACCENT,
                }
            };
            if let Some(arc) = &handle.arc {
                for segment in arc.windows(2) {
                    painter.line_segment([segment[0], segment[1]], Stroke::new(2.5, color));
                }
            } else if let Some(quad) = handle.quad {
                painter.add(egui::Shape::convex_polygon(
                    quad.to_vec(),
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 75),
                    Stroke::new(1.5, color),
                ));
            } else if handle.kind == GizmoKind::UniformScale {
                painter.circle_stroke(
                    handle.start,
                    handle.start.distance(handle.end),
                    Stroke::new(3.0, color),
                );
            } else if handle.handle == TranslationHandle::Free {
                painter.rect_filled(
                    egui::Rect::from_center_size(handle.end, egui::vec2(12.0, 12.0)),
                    2.0,
                    color,
                );
                painter.rect_stroke(
                    egui::Rect::from_center_size(handle.end, egui::vec2(16.0, 16.0)),
                    2.0,
                    Stroke::new(1.0, Color32::BLACK),
                    egui::StrokeKind::Outside,
                );
            } else {
                if handle.kind == GizmoKind::Scale {
                    painter.line_segment([handle.start, handle.end], Stroke::new(2.0, color));
                    painter.rect_filled(
                        egui::Rect::from_center_size(handle.end, egui::vec2(11.0, 11.0)),
                        1.0,
                        color,
                    );
                } else {
                    painter.arrow(
                        handle.start,
                        handle.end - handle.start,
                        Stroke::new(3.0, color),
                    );
                }
                painter.text(
                    handle.end + egui::vec2(9.0, -9.0),
                    egui::Align2::CENTER_CENTER,
                    match handle.handle {
                        TranslationHandle::X => "X",
                        TranslationHandle::Y => "Y",
                        TranslationHandle::Z => "Z",
                        TranslationHandle::Free => "",
                        TranslationHandle::PlaneX
                        | TranslationHandle::PlaneY
                        | TranslationHandle::PlaneZ => "",
                    },
                    egui::FontId::monospace(12.0),
                    color,
                );
            }
        }
    }

    fn track_scene_control(&mut self, key: &str, response: &egui::Response) {
        let rect = response.rect;
        self.scene_controls
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
        self.scene_controls_enabled
            .insert(key.into(), response.enabled());
    }

    fn frame_scene(&mut self) {
        self.modal_cancel("camera framing");
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.camera
            .frame_scene(&self.document.subtool_render_snapshots());
    }

    fn scene_action(
        &mut self,
        action: impl FnOnce(&mut Document) -> Result<(), String>,
        message: &str,
    ) {
        self.modal_cancel("scene command");
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.cancel_topology();
        self.layer_intensity_preview = None;
        match action(&mut self.document) {
            Ok(()) => {
                self.changed();
                self.message = message.into();
            }
            Err(error) => self.message = format!("SubTool change rejected: {error}"),
        }
    }

    fn add_primitive_subtool(&mut self, primitive: PrimitiveKind) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let mut mesh = match primitive
            .create_for_scene(self.primitive_detail, self.source_shortcut_profile())
        {
            Ok(mesh) => mesh,
            Err(error) => {
                self.message = format!("Cannot create {}: {error}", primitive.name());
                return;
            }
        };
        let snapshots = self.document.subtool_render_snapshots();
        let max_x = snapshots
            .iter()
            .flat_map(|s| {
                s.mesh.positions.iter().map(move |p| {
                    s.metadata.as_ref().map_or(f64::from(p.x), |m| {
                        m.affine.world_point(*p).map_or(f64::from(p.x), |v| v.x)
                    })
                })
            })
            .map(|x| x as f32)
            .reduce(f32::max)
            .unwrap_or(-1.25);
        for point in &mut mesh.positions {
            point.x += max_x + 1.25;
        }
        let name = primitive.name();
        self.scene_action(
            |document| document.add_subtool(mesh, name).map(|_| ()),
            "Part added. Sculpt each part separately; Undo restores the scene.",
        );
        self.frame_scene();
    }

    fn merge_parts(&mut self, selected_group: bool) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.cancel_topology();
        let settings = SubtoolMergeSettings {
            ids: if selected_group {
                self.document.selected_subtool_ids()
            } else {
                self.merge_selection.iter().copied().collect()
            },
            name: "Merged parts".into(),
            policy: if selected_group || self.merge_flatten {
                SubtoolMergePolicy::FlattenDisplayed
            } else {
                SubtoolMergePolicy::RequirePlain
            },
            allow_source_pose_loss: selected_group || self.merge_flatten,
            allow_mask_precision_loss: selected_group || self.merge_flatten,
            allow_uv_loss: selected_group || self.merge_flatten,
        };
        match self.document.merge_subtools(&settings) {
            Ok(report) => {
                self.merge_selection.clear();
                self.layer_intensity_preview = None;
                self.changed();
                self.message = format!(
                    "Merged {} parts into {} points; baked {} poses, detached UVs from {} parts, and rounded {} mask values. Undo restores the originals.",
                    report.consumed_ids.len(),
                    report.vertices,
                    report.baked_pose_ids.len(),
                    report.discarded_uv_ids.len(),
                    report.mask_roundtrip_changed_words,
                );
                if !selected_group {
                    self.frame_scene();
                }
            }
            Err(error) => self.message = format!("Merge rejected: {error}"),
        }
    }

    fn subtools_panel(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        self.scene_clip_rect = [clip.min.x, clip.min.y, clip.max.x, clip.max.y];
        section(
            ui,
            "04",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C337856576635),
        );
        ui.add_space(10.0);
        let summaries = self.document.subtool_summaries();
        self.merge_selection
            .retain(|id| summaries.iter().any(|part| part.id == *id));
        let id = self.document.active_subtool_id();
        let selected_ids = self.document.selected_subtool_ids();
        let active = summaries
            .iter()
            .position(|object| Some(object.id) == id)
            .unwrap_or(0);
        ui.horizontal(|ui| {
            let sphere = ui.add_enabled(
                summaries.len() < 32,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C85E2A74A55A9,
                )),
            );
            self.track_scene_control("scene.add_sphere", &sphere);
            if sphere.clicked() {
                self.add_primitive_subtool(PrimitiveKind::Sphere);
            }
            let cube = ui.add_enabled(
                summaries.len() < 32,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CA6D087342259,
                )),
            );
            self.track_scene_control("scene.add_cube", &cube);
            if cube.clicked() {
                self.add_primitive_subtool(PrimitiveKind::Cube);
            }
        });
        ui.horizontal(|ui| {
            let cylinder = ui.add_enabled(
                summaries.len() < 32,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C3E08F0D470B5,
                )),
            );
            self.track_scene_control("scene.add_cylinder", &cylinder);
            if cylinder.clicked() {
                self.add_primitive_subtool(PrimitiveKind::Cylinder);
            }
            let torus = ui.add_enabled(
                summaries.len() < 32,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C4EBAFA696187,
                )),
            );
            self.track_scene_control("scene.add_torus", &torus);
            if torus.clicked() {
                self.add_primitive_subtool(PrimitiveKind::Torus);
            }
        });
        ui.horizontal(|ui| {
            let append = ui
                .add_enabled(
                    summaries.len() + self.document.selected_subtool_ids().len() <= 32,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C8B0C87C10042,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C5FB9611FCB10,
                ));
            self.track_scene_control("scene.append_obj", &append);
            if append.clicked() {
                self.append_obj_file();
            }
            let frame = ui.button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C675745B23F75,
            ));
            self.track_scene_control("scene.frame", &frame);
            if frame.clicked() {
                self.frame_scene();
            }
        });
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C3ADB5023D4BA,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        for object in &summaries {
            ui.horizontal(|ui| {
                let mut included = self.merge_selection.contains(&object.id);
                let merge = ui
                    .add_enabled(object.visible, egui::Checkbox::without_text(&mut included))
                    .on_hover_text(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C4DC88EFDEFB6,
                    ));
                self.track_scene_control(&format!("scene.merge_part.{}", object.id), &merge);
                if merge.changed() {
                    if included {
                        self.merge_selection.insert(object.id);
                    } else {
                        self.merge_selection.remove(&object.id);
                    }
                }
                let mut visible = object.visible;
                let eye = ui
                    .checkbox(&mut visible, "")
                    .on_hover_text(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C805DC198FAA1,
                    ));
                self.track_scene_control(&format!("scene.visibility.{}", object.id), &eye);
                if eye.changed() {
                    self.scene_action(
                        |document| document.set_subtool_visible(object.id, visible),
                        "Part visibility updated.",
                    );
                }
                let selected = ui
                    .add(
                        egui::Button::selectable(selected_ids.contains(&object.id), &object.name)
                            .truncate(),
                    )
                    .on_hover_text(format!(
                        "{} · {} vertices · {} triangles",
                        object.name, object.vertices, object.triangles
                    ));
                self.track_scene_control(&format!("scene.select.{}", object.id), &selected);
                if selected.clicked() {
                    let toggle = ui.input(|input| input.modifiers.ctrl || input.modifiers.command);
                    let (ids, active) = crate::source_display::panel_selection(
                        &self.document.selected_subtool_ids(),
                        self.document.active_subtool_id(),
                        object.id,
                        toggle,
                    );
                    self.scene_action(
                        |document| document.select_subtools(&ids, active),
                        "Part selection updated.",
                    );
                }
            });
        }
        let flatten = ui
            .checkbox(
                &mut self.merge_flatten,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CC92989E6A92F),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C49A6E85B293B,
            ));
        self.track_scene_control("scene.merge_flatten", &flatten);
        let merge = ui.add_enabled(
            self.merge_selection.len() >= 2,
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C5B8A2F217037,
            )),
        );
        self.track_scene_control("scene.merge", &merge);
        if merge.clicked() {
            self.merge_parts(false);
        }
        let merge_selected = ui
            .add_enabled(
                selected_ids.len() >= 2,
                egui::Button::new("Merge selected"),
            )
            .on_hover_text("Concatenate selected displayed parts in selection order. Poses and retained detail are baked, UVs detached, and differing mask conventions may round. Undo restores the originals.");
        self.track_scene_control("scene.merge_selected", &merge_selected);
        if merge_selected.clicked() {
            self.merge_parts(true);
        }
        let mut solo = self.document.solo();
        let solo_control = ui
            .checkbox(
                &mut solo,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CC703AB13B28E),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CFB3E700C4706,
            ));
        self.track_scene_control("scene.solo", &solo_control);
        if solo_control.changed() {
            self.scene_action(|document| document.set_solo(solo), "Solo view updated.");
        }
        ui.horizontal(|ui| {
            let isolate = ui.add_enabled(
                summaries.len() > selected_ids.len() && !selected_ids.is_empty(),
                egui::Button::new("Isolate selected"),
            );
            self.track_scene_control("scene.isolate_selected", &isolate);
            if isolate.clicked() {
                self.scene_action(
                    |document| document.isolate_selected_subtools(),
                    "Unselected parts hidden.",
                );
            }
            let show_all = ui.add_enabled(
                solo || summaries.iter().any(|object| !object.visible),
                egui::Button::new("Show all"),
            );
            self.track_scene_control("scene.show_all", &show_all);
            if show_all.clicked() {
                self.scene_action(
                    |document| document.show_all_subtools(),
                    "All parts visible.",
                );
            }
        });
        let clear = ui.add_enabled(
            !self.document.is_empty(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C2757CDD813C9,
            )),
        );
        self.track_scene_control("scene.clear", &clear);
        if clear.clicked() {
            self.scene_action(
                |d| d.clear_scene().map(|_| ()),
                "Scene cleared. Undo restores every part.",
            );
        }
        let Some(id) = id else {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C474BF3DF97F6,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            ui.add_space(20.0);
            return;
        };
        ui.horizontal(|ui| {
            let duplicate = ui
                .add_enabled(
                    summaries.len() < 32,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CFDD07C4B76A6,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CF42EFB55A3A5,
                ));
            self.track_scene_control("scene.duplicate", &duplicate);
            if duplicate.clicked() {
                self.scene_action(
                    |document| document.duplicate_selected_subtools().map(|_| ()),
                    "Selected parts copied at the same coordinates.",
                );
            }
            let delete = ui.add_enabled(
                !summaries.is_empty(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CCE571CAD918B,
                )),
            );
            self.track_scene_control("scene.delete", &delete);
            if delete.clicked() {
                self.scene_action(
                    |document| document.delete_selected_subtools(),
                    "Selected parts deleted. Undo restores them.",
                );
            }
        });
        ui.horizontal(|ui| {
            let up = ui.add_enabled(
                active > 0,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C9BC4A75BCFFF,
                )),
            );
            self.track_scene_control("scene.up", &up);
            if up.clicked() {
                self.scene_action(
                    |document| document.reorder_subtool(id, active - 1),
                    "Part moved up in the list.",
                );
            }
            let down = ui.add_enabled(
                active + 1 < summaries.len(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CE781304E11F4,
                )),
            );
            self.track_scene_control("scene.down", &down);
            if down.clicked() {
                self.scene_action(
                    |document| document.reorder_subtool(id, active + 1),
                    "Part moved down in the list.",
                );
            }
        });
        let committed = &summaries[active].name;
        if self.subtool_name_id != Some(id) || self.subtool_name_committed != *committed {
            self.subtool_name_id = Some(id);
            self.subtool_name_committed = committed.clone();
            self.subtool_name_draft = committed.clone();
        }
        ui.horizontal(|ui| {
            let name = ui
                .add(egui::TextEdit::singleline(&mut self.subtool_name_draft).desired_width(105.0));
            self.track_scene_control("scene.name", &name);
            let rename = ui.add_enabled(
                self.subtool_name_draft != *committed,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7BC3EED2F005,
                )),
            );
            self.track_scene_control("scene.rename", &rename);
            if rename.clicked()
                || (name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                let draft = self.subtool_name_draft.clone();
                self.scene_action(
                    |document| document.set_subtool_name(id, &draft),
                    "Part renamed.",
                );
            }
        });
        if !self.document.active_subtool_visible() {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CD64CE52F6362,
                ))
                .size(11.0)
                .color(ACCENT),
            );
        }
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C5C593338A00C,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        ui.add_space(20.0);
    }

    fn resident_studio_idle(&self) -> Result<(), String> {
        if self.stroke.is_some()
            || !self.pending_dabs.is_empty()
            || self.transform_drag.is_some()
            || self.mask_region_gesture.is_some()
            || self.topology_job.is_some()
            || self.uv_bake_job.is_some()
            || self.pending_sgl_export.is_some()
        {
            return Err("Resident studio handoff requires an idle document".into());
        }
        Ok(())
    }

    /// Capture the actual studio document for a separate WebGPU viewport.
    /// The token ties a future sparse result to this exact core snapshot.
    pub fn resident_studio_begin(&mut self) -> Result<(u64, String), String> {
        self.resident_studio_idle()?;
        self.resident_studio.begin(&self.document)
    }

    pub fn resident_studio_begin_upload(&mut self) -> Result<(u64, Vec<u8>), String> {
        self.resident_studio_idle()?;
        self.resident_studio.begin_upload(&self.document)
    }

    /// Actual studio camera ray for the separately presented GPU viewport.
    /// This reads no mesh and is valid only while its captured camera/viewport
    /// remain fixed; the browser controller cancels on any view change.
    pub fn resident_studio_ray(&self, x: f32, y: f32) -> Result<[f64; 7], String> {
        self.resident_studio_idle()?;
        let screen = Pos2::new(x, y);
        if !self.last_viewport.is_positive() || !self.last_viewport.contains(screen) {
            return Err("Resident pointer is outside the current sculpt viewport".into());
        }
        let ray = self.screen_scene_ray(screen, self.last_viewport);
        ray.validate()?;
        // The ordinary camera ray may originate in f32 screen math. Normalize
        // in f64 for the resident picker's stricter unit-direction contract.
        let direction = ray.direction.normalize();
        let maximum = self.camera.resident_pick_distance(ray)?;
        Ok([
            ray.origin.x,
            ray.origin.y,
            ray.origin.z,
            direction.x,
            direction.y,
            direction.z,
            maximum,
        ])
    }

    pub fn resident_studio_commit(&mut self, id: u64, sparse_bytes: &[u8]) -> Result<bool, String> {
        self.resident_studio_idle()?;
        let changed = self
            .resident_studio
            .commit(&mut self.document, id, sparse_bytes)?;
        if changed {
            self.changed();
            self.egui_ctx.request_repaint();
        }
        Ok(changed)
    }

    pub fn resident_studio_cancel(&mut self, id: u64) -> Result<(), String> {
        self.resident_studio.cancel(id)
    }

    pub fn resident_studio_commit_timing(
        &self,
    ) -> Option<obrush_core::resident_point_program::ResidentCommitTiming> {
        self.resident_studio.last_commit_timing()
    }

    pub fn load_document(&mut self, name: &str, contents: &str) -> Result<(), String> {
        self.load_document_bytes(name, contents.as_bytes())
    }

    pub fn load_document_bytes(&mut self, name: &str, contents: &[u8]) -> Result<(), String> {
        let append = std::mem::take(&mut self.pending_append);
        let prepared = crate::scene_import::prepare(name, contents, append, &self.viewport_style)?;
        self.cancel_mask_region("File command cancelled the pending selection.", true);
        self.finish_stroke();
        if self.stroke_rejected {
            return Err("Pending shape edit was rejected; retry after the restored stroke".into());
        }
        self.modal_cancel("file installation");
        let report = crate::scene_import::install(&mut self.document, prepared)?;
        if !append {
            self.source_sgl_import.reset_after_whole_open();
            self.source_boot_framing_pending = false;
        }
        self.cancel_topology();
        if !append {
            self.viewport_style.source_display.import_header(
                self.document
                    .scene_origin()
                    .and_then(|o| o.display.as_ref()),
            );
        }
        if let Some(camera) = self.document.scene_origin().and_then(|o| o.camera.as_ref()) {
            self.camera.orthographic = camera.projection == 1;
            self.camera.mode = match camera.mode {
                0 => CameraMode::Orbit,
                1 => CameraMode::Spherical,
                _ => CameraMode::Plane,
            };
            if camera.mode == 0 {
                self.camera.rotation = Some(glam::Quat::IDENTITY);
                self.camera.yaw = 0.0;
                self.camera.pitch = 0.0;
            }
            self.camera.fov_degrees = camera.fov_degrees;
            self.camera.boot_source_fov = None;
            self.camera.source_vertical_fov = true;
            self.camera.use_pivot = camera.use_pivot;
            // A whole-scene SGL Open carries source camera settings. Its
            // viewer must fit the imported scene with source projection math;
            // the measured viewport becomes available in the next UI frame.
            if !append
                && name
                    .rsplit_once('.')
                    .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("sgl"))
            {
                self.source_boot_framing_pending = true;
            }
        }
        self.viewport_style.use_vertex_color = report.as_ref().is_some_and(|r| r.imported_colors);
        self.last_import_report = report;
        self.last_extract_report = None;
        self.last_mask_filter_report = None;
        self.last_mask_region_report = None;
        self.frame_scene();
        self.changed();
        self.message = format!(
            "{} {name}. Previous scene is available in Undo.",
            if append { "Appended" } else { "Loaded" }
        );
        self.egui_ctx.request_repaint();
        Ok(())
    }

    fn import_file(&mut self) {
        self.cancel_startup_model();
        self.pending_append = false;
        self.pick_document_file(false);
    }

    fn append_obj_file(&mut self) {
        self.pending_append = true;
        self.pick_document_file(true);
    }

    fn pick_document_file(&mut self, append: bool) {
        self.modal_cancel("file picker command");
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = if append {
            rfd::FileDialog::new().add_filter("Meshes", &["obj", "ply", "stl"])
        } else {
            rfd::FileDialog::new()
                .add_filter("OBrush or meshes", &["obrush", "sgl", "obj", "ply", "stl"])
        }
        .pick_file()
        {
            let result = (|| {
                let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let limit = obrush_core::resource_limits::document_input_limit(&name);
                if size > limit as u64 {
                    return Err(format!(
                        "File exceeds the {} MiB import limit.",
                        limit / obrush_core::resource_limits::MIB
                    ));
                }
                let contents = std::fs::read(&path).map_err(|e| e.to_string())?;
                self.load_document_bytes(&name, &contents)
            })();
            if let Err(error) = result {
                self.message = format!("Import failed: {error}");
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = append;
            pick_file();
        }
    }

    fn save_file(&mut self, obj: bool) {
        if obj {
            self.export_file_as(ExportFormat::Obj);
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let contents = self.document.to_json();
        let contents = match contents {
            Ok(text) => text,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        self.write_file_contents(contents.into_bytes(), "obrush", true);
    }

    fn export_mesh(&mut self) {
        self.export_file_as(self.export_format);
    }

    fn export_file_as(&mut self, format: ExportFormat) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let policy = if self.export_allow_attribute_loss {
            AttributeExportPolicy::AllowAndReport
        } else {
            AttributeExportPolicy::RejectLoss
        };
        let scope = if self.sgl_export_selected_only {
            obrush_core::SglExportScope::SelectedParts
        } else {
            obrush_core::SglExportScope::AllScene
        };
        let source_obj = (self.app_preferences.shortcut_profile
            == crate::app_preferences::PreferenceProfile::SculptGl)
            .then_some(obrush_core::ObjColorOptions {
                zbrush: self.obj_export_zbrush_color,
                append_rgb: self.obj_export_append_rgb,
            });
        let result =
            self.document
                .export_scene_geometry(scope, format.scene_format(source_obj), policy);
        match result {
            Ok(export) => {
                let report = export.report;
                self.last_export_report = Some(report.attributes.clone());
                self.last_scene_geometry_report = Some(report.clone());
                self.write_file_contents(export.bytes, format.extension(), false);
                if cfg!(target_arch = "wasm32") {
                    self.message = format!(
                        "Exported {} parts and {} triangles; baked {} source poses ({} source origins omitted).",
                        report.parts,
                        report.triangles,
                        report.baked_pose_parts,
                        report.omitted_source_origin_parts
                    );
                }
            }
            Err(error) => self.message = format!("Export failed: {error}"),
        }
    }

    fn write_file_contents(&mut self, contents: Vec<u8>, extension: &str, project: bool) {
        let safe_name: String = self
            .document
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .take(80)
            .collect();
        let filename = format!(
            "{}.{}",
            if safe_name.is_empty() {
                "Untitled"
            } else {
                &safe_name
            },
            extension
        );
        self.write_file_contents_named(contents, extension, project, filename);
    }

    fn write_file_contents_named(
        &mut self,
        contents: Vec<u8>,
        extension: &str,
        project: bool,
        filename: String,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(&filename)
                .add_filter(extension, &[extension])
                .save_file()
            {
                match std::fs::write(&path, contents) {
                    Ok(()) => {
                        if project {
                            self.dirty = false;
                        }
                        self.message = format!("Saved {}", path.display());
                    }
                    Err(error) => self.message = format!("Save failed: {error}"),
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Keep the established text host bridge for public JSON/OBJ;
            // binary PLY/STL use the byte bridge without UTF-8 conversion.
            if project || extension == "obj" {
                match String::from_utf8(contents) {
                    Ok(text) => download_text(&filename, &text),
                    Err(_) => {
                        self.message = "Export failed: invalid UTF-8 text output".into();
                        return;
                    }
                }
            } else {
                download_bytes(&filename, &contents);
            }
            if project {
                self.dirty = false;
            }
            self.message = format!("Download requested: {filename}");
        }
    }

    fn primitive(&mut self, cube: bool) {
        self.cancel_topology();
        self.finish_stroke();
        if self.stroke_rejected
            || (!self.document.is_empty() && !self.document.active_subtool_visible())
        {
            return;
        }
        let primitive = if cube {
            PrimitiveKind::Cube
        } else {
            PrimitiveKind::Sphere
        };
        let mesh = match primitive.create(self.primitive_detail) {
            Ok(mesh) => mesh,
            Err(error) => {
                self.message = format!("Cannot create {}: {error}", primitive.name());
                return;
            }
        };
        match self.document.replace_mesh(mesh) {
            Ok(()) => {
                self.changed();
                self.frame_scene();
                self.message = "Active object replaced. Undo restores its previous mesh.".into();
            }
            Err(error) => self.message = error,
        }
    }

    fn subdivide(&mut self, smooth: bool) {
        self.cancel_topology();
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        let result = if smooth {
            self.document.subdivide_smooth()
        } else {
            self.document.subdivide()
        };
        match result {
            Ok(()) => {
                self.changed();
                self.message = if smooth {
                    "Surface subdivided and smoothed. Undo restores the previous form."
                } else {
                    "Mesh refined without smoothing. Undo restores the previous topology."
                }
                .into();
            }
            Err(error) => self.message = error,
        }
    }

    fn add_subdivision_level(&mut self) {
        self.modal_cancel("subdivision command");
        self.cancel_topology();
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        let settings = self
            .document
            .retained_static_subdivision_settings()
            .or_else(|| self.subdivision_choice.static_settings());
        let result = match settings {
            Some(settings) => self.document.add_source_static_subdivision_level(&settings),
            None => self.document.add_subdivision_level(),
        };
        match result {
            Ok(()) => {
                self.changed();
                self.message = format!(
                    "Subdivision level {} added. Lower levels remain editable; Undo restores the previous stack.",
                    if self.document.is_empty() {
                        0
                    } else {
                        self.document.current_level() + 1
                    }
                );
            }
            Err(error) => self.message = format!("Cannot add subdivision level: {error}"),
        }
    }

    fn select_subdivision_level(&mut self, level: usize) {
        self.modal_cancel("subdivision level command");
        self.cancel_topology();
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        match self.document.select_level(level) {
            Ok(()) => {
                self.changed();
                self.message = format!(
                    "Editing subdivision level {} of {}.",
                    if self.document.is_empty() {
                        0
                    } else {
                        self.document.current_level() + 1
                    },
                    self.document.level_count()
                );
            }
            Err(error) => self.message = format!("Cannot select subdivision level: {error}"),
        }
    }

    fn reverse_subdivision_level(&mut self) {
        self.modal_cancel("reverse subdivision command");
        self.cancel_topology();
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        match self.document.reverse_source_static_subdivision_level(
            &obrush_core::ReverseSubdivisionSettings::default(),
        ) {
            Ok(report) => {
                self.changed();
                self.message = format!(
                    "Recovered a lower level: {} faces from {} fine faces. Undo restores the original mesh.",
                    report.coarse_faces, report.fine_faces
                );
            }
            Err(error) => self.message = format!("Cannot reverse subdivision: {error}"),
        }
    }

    fn delete_subdivision_levels(&mut self, lower: bool) {
        self.modal_cancel("subdivision delete command");
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        self.cancel_topology();
        let result = if lower {
            self.document.delete_lower_subdivision_levels()
        } else {
            self.document.delete_higher_subdivision_levels()
        };
        match result {
            Ok(report) => {
                if report.retained_levels == report.previous_levels {
                    self.message = "No subdivision levels to delete. History kept.".into();
                    return;
                }
                self.changed();
                let morph = match report.morph_target {
                    obrush_core::MorphTargetDeletionDisposition::Dropped { .. } => {
                        " The Morph Target belonged to a deleted level and was removed; Undo restores it."
                    }
                    obrush_core::MorphTargetDeletionDisposition::Rebased { .. } => {
                        " The Morph Target now refers to its retained level."
                    }
                    _ => "",
                };
                self.message = format!(
                    "Deleted {} {} levels; active shape kept. Undo restores them.{morph}",
                    report.previous_levels - report.retained_levels,
                    if lower { "lower" } else { "higher" }
                );
            }
            Err(error) => self.message = format!("Cannot delete subdivision levels: {error}"),
        }
    }

    fn trace_keyboard_route(&mut self, ctx: &egui::Context, phase: &'static str) {
        let data = keyboard_input_observation(ctx, phase);
        if let Some(mut data) = data {
            data["focused_id"] = ctx
                .memory(|memory| memory.focused())
                .map(|id| serde_json::Value::String(format!("{id:?}")))
                .unwrap_or(serde_json::Value::Null);
            data["focused_id64_hex"] = ctx
                .memory(|memory| memory.focused())
                .map(|id| serde_json::Value::String(format!("{:016x}", id.value())))
                .unwrap_or(serde_json::Value::Null);
            data["wants_keyboard"] = serde_json::json!(ctx.wants_keyboard_input());
            data["stroke_active"] = serde_json::json!(self.stroke.is_some());
            data["radius"] = serde_json::json!(self.brush.radius);
            if self.keyboard_diagnostics.len() == 32 {
                self.keyboard_diagnostics.pop_front();
            }
            self.keyboard_diagnostics.push_back(data);
        }
    }

    pub(crate) fn toggle_wireframe(&mut self) {
        if let Some(metadata) = self.document.active_object_metadata() {
            let current = metadata
                .render
                .as_ref()
                .is_some_and(|render| render.show_wireframe);
            match self.document.set_selected_source_wireframe(!current) {
                Ok(true) => self.changed(),
                Ok(false) => {}
                Err(error) => self.message = format!("Wireframe rejected: {error}"),
            }
        } else {
            self.viewport_style.wireframe = !self.viewport_style.wireframe;
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        self.trace_keyboard_route(ctx, "before_shortcuts");
        if self.app_preferences.shortcut_profile
            == crate::app_preferences::PreferenceProfile::SculptGl
        {
            self.source_shortcuts(ctx);
            return;
        }
        // Match the modifiers carried by each key event. A fast chord may be
        // fully released before the next frame, leaving i.modifiers empty.
        // Toolbar button focus must not block project/history commands.
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.interrupt_source_paint_tail("Escape ended Source Paint contact") {
                self.unprocessed_release = None;
                self.finish_stroke_deferred();
            }
            self.paint_pick_armed = false;
            self.paint_pick_dragging = false;
            self.cancel_locked_stamp();
            self.cancel_mask_region("Escape cancelled the pending mask selection.", true);
        }
        let (redo, undo, save, open) = file_history_commands(ctx);
        if redo {
            self.history(true);
        }
        if undo {
            self.history(false);
        }
        if save {
            self.save_file(false);
        }
        if open {
            self.import_file();
        }
        if redo || undo || save || open {
            return;
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        let input = ctx.input(|i| {
            (
                i.modifiers,
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::X),
                i.key_pressed(egui::Key::W),
                i.key_pressed(egui::Key::H),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if input.0.command {
            if input.1 {
                self.history(input.0.shift);
            }
            if input.2 {
                self.history(true);
            }
            if input.3 {
                self.save_file(false);
            }
            if input.4 {
                self.import_file();
            }
            return;
        }
        if input.5 {
            self.finish_stroke();
            self.frame_scene();
        }
        if input.6 {
            self.finish_stroke();
            self.brush.symmetry_x = !self.brush.symmetry_x;
        }
        if input.7 {
            self.cancel_mask_region(
                "Viewport setting changed; pending selection cancelled.",
                true,
            );
            self.toggle_wireframe();
        }
        if input.8 {
            self.cancel_mask_region("Help command cancelled the pending selection.", true);
            self.show_help = !self.show_help;
        }
        if input.9 {
            self.cancel_transform_drag();
            self.transform_mode = false;
            self.finish_stroke();
        }
        if ctx.input(|input| input.key_pressed(egui::Key::T)) {
            self.toggle_transform_mode();
        }
        for (index, key) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
            egui::Key::Num0,
            egui::Key::P,
            egui::Key::J,
            egui::Key::L,
            egui::Key::G,
        ]
        .into_iter()
        .enumerate()
        {
            if ctx.input(|i| i.key_pressed(key)) {
                self.select_brush(BRUSHES[index].0);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::OpenBracket)) {
            if self.brush.kind == BrushKind::Paint
                && self.source_paint_preferences.mode
                    == crate::source_paint_host::PaintMode::SourceOrdinary
            {
                let mut candidate = self.source_paint_preferences.clone();
                candidate.radius = (candidate.radius * 0.85).clamp(5.0, 500.0);
                if candidate.validate().is_ok() {
                    self.finish_stroke();
                    if !self.stroke_rejected {
                        self.source_paint_preferences = candidate;
                    }
                }
            } else {
                self.cancel_mask_region(
                    "Brush setting changed; pending selection cancelled.",
                    true,
                );
                if self.brush_screen_space {
                    self.brush_screen_radius = (self.brush_screen_radius * 0.85).max(5.0);
                } else {
                    self.brush.radius = (self.brush.radius * 0.85).max(0.015);
                }
                self.commit_brush_view();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::CloseBracket)) {
            if self.brush.kind == BrushKind::Paint
                && self.source_paint_preferences.mode
                    == crate::source_paint_host::PaintMode::SourceOrdinary
            {
                let mut candidate = self.source_paint_preferences.clone();
                candidate.radius = (candidate.radius / 0.85).clamp(5.0, 500.0);
                if candidate.validate().is_ok() {
                    self.finish_stroke();
                    if !self.stroke_rejected {
                        self.source_paint_preferences = candidate;
                    }
                }
            } else {
                self.cancel_mask_region(
                    "Brush setting changed; pending selection cancelled.",
                    true,
                );
                if self.brush_screen_space {
                    self.brush_screen_radius = (self.brush_screen_radius / 0.85).min(500.0);
                } else {
                    self.brush.radius = (self.brush.radius / 0.85).min(2.0);
                }
                self.commit_brush_view();
            }
        }
    }

    fn header(&mut self, ctx: &egui::Context) {
        let compact = ctx.content_rect().width() < 900.0;
        egui::TopBottomPanel::top("header")
            .min_height(57.0)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::top_panel(ctx))
                    .inner_margin(egui::Margin::symmetric(if compact { 12 } else { 20 }, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if compact {
                        ui.label(RichText::new("OBrush").size(22.0).strong().color(ACCENT));
                    } else {
                        ui.label(RichText::new("O").size(27.0).strong().color(ACCENT));
                        ui.label(RichText::new("BRUSH").size(20.0).strong());
                        ui.add_space(13.0);
                        ui.label(
                            RichText::new(crate::ui_text::caption(
                                crate::ui_text::CaptionKey::C215ED96071EE,
                            ))
                            .size(10.0)
                            .color(preferences_host::muted(ui)),
                        );
                        ui.add_space(20.0);
                    }
                    if ui
                        .button(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::CED5BF4431B03,
                        ))
                        .on_hover_text(
                            if self.app_preferences.shortcut_profile
                                == crate::app_preferences::PreferenceProfile::SculptGl
                            {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::C19E565CBC989)
                            } else {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::CDA6DF21C4B10)
                            },
                        )
                        .clicked()
                    {
                        self.import_file();
                    }
                    if ui
                        .button(if compact {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C010BAA4FC183)
                        } else {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C566352BE2B27)
                        })
                        .on_hover_text(
                            if self.app_preferences.shortcut_profile
                                == crate::app_preferences::PreferenceProfile::SculptGl
                            {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::CA2DDC766055F)
                            } else {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::C57AD8D0F9C5B)
                            },
                        )
                        .clicked()
                    {
                        self.save_file(false);
                    }
                    if ui
                        .button(if compact {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C2886C2226F0F)
                        } else {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C996E0079E48E)
                        })
                        .on_hover_text(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::CAE33D3DD58A6,
                        ))
                        .clicked()
                    {
                        self.save_file(true);
                    }
                    if compact
                        && ui
                            .button(crate::ui_text::caption(
                                crate::ui_text::CaptionKey::C2DD2245CB1A5,
                            ))
                            .on_hover_text(crate::ui_text::caption(
                                crate::ui_text::CaptionKey::C5C7E88E89B9B,
                            ))
                            .clicked()
                    {
                        self.finish_stroke();
                        self.inspector_open = !self.inspector_open;
                    }
                    let help = ui.button(
                        if compact
                            || self.app_preferences.shortcut_profile
                                == crate::app_preferences::PreferenceProfile::SculptGl
                        {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C7FC0C24E4A7D)
                        } else {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C9FAF36F3443D)
                        },
                    );
                    let rect = help.rect;
                    self.app_preference_controls.insert(
                        "preferences.help".into(),
                        [rect.min.x, rect.min.y, rect.width(), rect.height()],
                    );
                    if help.clicked() {
                        self.finish_stroke();
                        self.show_help = !self.show_help;
                    }
                    let mesh = self.document.active_mesh();
                    ui.label(
                        RichText::new(format!(
                            "{}: {}  ·  Faces: {}",
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C4E87CE4CDFEB),
                            number(mesh.map_or(0, |mesh| mesh.positions.len())),
                            number(mesh.map_or(0, Mesh::face_count)),
                        ))
                        .size(10.0)
                        .color(preferences_host::muted(ui)),
                    );
                    if ctx.content_rect().width() >= 1100.0 {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(concat!(env!("CARGO_PKG_VERSION"), "  /  FOUNDATION"))
                                .size(10.0)
                                .color(preferences_host::muted(ui)),
                        );
                    }
                });
            });
    }

    fn tools(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tools")
            .exact_width(212.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::panel(ctx))
                    .inner_margin(egui::Margin::symmetric(15, 18)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("tools_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.tools_body(ui));
            });
    }

    fn tools_body(&mut self, ui: &mut egui::Ui) {
        self.modal_status(ui);
        ui.spacing_mut().slider_width = ui.available_width();
        section(
            ui,
            "01",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CED1CBE18A173),
        );
        ui.add_space(14.0);
        let tile_height = BRUSHES
            .iter()
            .map(|(kind, _, _)| {
                brush_label_galley(ui, &crate::ui_text::brush_name(*kind))
                    .size()
                    .y
            })
            .fold(67.0_f32, |height, label_height| {
                height.max(38.0 + label_height + 14.0)
            });
        egui::Grid::new("brushes")
            .num_columns(3)
            .spacing([5.0, 5.0])
            .show(ui, |ui| {
                for (i, (kind, _name, key)) in BRUSHES.iter().enumerate() {
                    let source_key = self.shortcut_resolver.brush_labels(*kind);
                    let key = if self.app_preferences.shortcut_profile
                        == crate::app_preferences::PreferenceProfile::SculptGl
                    {
                        source_key.as_str()
                    } else {
                        key
                    };
                    let display_name = crate::ui_text::brush_name(*kind);
                    let tile = brush_tile(
                        ui,
                        &display_name,
                        key,
                        self.brush.kind == *kind,
                        i,
                        tile_height,
                    );
                    self.track_morph_control(
                        &format!(
                            "brush.{}",
                            kind.name().to_ascii_lowercase().replace(' ', "_")
                        ),
                        &tile,
                    );
                    if *kind == BrushKind::Morph {
                        self.track_morph_control("brush.morph", &tile);
                    }
                    if tile.clicked() {
                        self.select_brush(*kind);
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(24.0);
        self.source_paint_panel(ui);
        self.source_geometry_panel(ui);
        if !(self.source_geometry_requested(self.brush.kind)
            || self.brush.kind == BrushKind::Paint
                && self.source_paint_preferences.mode
                    == crate::source_paint_host::PaintMode::SourceOrdinary)
        {
            section(
                ui,
                "02",
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C0A9341892DD6),
            );
            ui.add_space(13.0);
            label_value(
                ui,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::CE064C92D289E),
                &format!("{:.3}", self.brush.radius),
            );
            let radius = ui.add(
                egui::Slider::new(&mut self.brush.radius, 0.015..=1.0)
                    .show_value(false)
                    .logarithmic(true),
            );
            self.track_morph_control("settings.radius", &radius);
            ui.add_space(11.0);
            label_value(
                ui,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C4CDC2FA5E56B),
                &format!("{}%", (self.brush.strength * 100.0) as u32),
            );
            let strength =
                ui.add(egui::Slider::new(&mut self.brush.strength, 0.0..=1.0).show_value(false));
            self.track_morph_control("settings.strength", &strength);
            if self.brush.kind != BrushKind::Draw
                && self.brush.radius
                    < self
                        .document
                        .active_mesh()
                        .map_or(f32::INFINITY, estimated_edge_length)
                        * 2.5
            {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C9B45876EA4DC,
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CD3CEEFE9068F,
                ));
            }
            ui.add_space(13.0);
            let invert = ui
                .checkbox(
                    &mut self.brush.invert,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C678AF70390D2),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C31809CF81EFE,
                ));
            self.track_morph_control("settings.invert", &invert);
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(13.0);
            let symmetry = ui
                .checkbox(
                    &mut self.brush.symmetry_x,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C97FCF490FDDC),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CA65108620F16,
                ));
            self.track_morph_control("settings.symmetry", &symmetry);
            ui.add_space(10.0);
            let mut sculptgl_pressure = matches!(
                self.preferences.global().pressure,
                PressureProfile::SculptGlFactors { .. }
            );
            if ui
                .checkbox(&mut sculptgl_pressure, "SculptGL pressure curve")
                .changed()
            {
                let pressure = if sculptgl_pressure {
                    PressureProfile::SculptGlFactors {
                        intensity: 0.0,
                        radius: 0.75,
                    }
                } else {
                    PressureProfile::LegacyBinary {
                        strength: true,
                        radius: false,
                    }
                };
                self.set_pressure_profile(pressure);
            }
            match self.preferences.global().pressure {
                PressureProfile::SculptGlFactors {
                    mut intensity,
                    mut radius,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("Intensity factor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{intensity:.2}"));
                        });
                    });
                    let changed = ui
                        .add(
                            egui::Slider::new(&mut intensity, 0.0..=1.0)
                                .step_by(0.01)
                                .show_value(false),
                        )
                        .changed();
                    ui.horizontal(|ui| {
                        ui.label("Radius factor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{radius:.2}"));
                        });
                    });
                    let changed = changed
                        | ui.add(
                            egui::Slider::new(&mut radius, 0.0..=1.0)
                                .step_by(0.01)
                                .show_value(false),
                        )
                        .changed();
                    if changed {
                        self.set_pressure_profile(PressureProfile::SculptGlFactors {
                            intensity,
                            radius,
                        });
                    }
                }
                PressureProfile::LegacyBinary { .. } => {
                    let pressure_strength = ui.checkbox(
                        &mut self.brush.pressure_strength,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C417A00C008A1),
                    );
                    self.track_morph_control("settings.pressure_strength", &pressure_strength);
                    let pressure_radius = ui
                        .checkbox(
                            &mut self.brush.pressure_radius,
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C4C39AE12DFA7),
                        )
                        .on_hover_text(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::C9E1DEE7A566A,
                        ));
                    self.track_morph_control("settings.pressure_radius", &pressure_radius);
                }
            }
            self.brush_options_panel(ui);
            if self.brush.kind == BrushKind::Morph {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C28B3E06D0F8F,
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                );
            }
        }
        let source_paint = self.brush.kind == BrushKind::Paint
            && self.source_paint_preferences.mode
                == crate::source_paint_host::PaintMode::SourceOrdinary;
        if !self.source_geometry_requested(self.brush.kind)
            && (!source_paint || !self.preferences.tool(BrushKind::Paint).lock_position)
        {
            ui.add_space(10.0);
            let mut selection = self.preferences.tool(self.preferences.selected()).alpha;
            let label = alpha_selection_name(&self.preferences, selection).unwrap_or("None".into());
            let label = crate::ui_text::alpha_selected_name(selection, &label);
            let entries: Vec<_> = self
                .preferences
                .library()
                .entries()
                .iter()
                .map(|e| (e.id, e.name.clone()))
                .collect();
            let selector = egui::ComboBox::from_id_salt("alpha_library")
                .selected_text(label.clone())
                .width(ui.available_width())
                .truncate()
                .show_ui(ui, |ui| {
                    let none = ui.selectable_value(
                        &mut selection,
                        AlphaSelection::None,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C98589F40BA48),
                    );
                    self.track_morph_control("alpha.option.none", &none);
                    let square = ui.selectable_value(
                        &mut selection,
                        AlphaSelection::Square,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CA0EA42396A95),
                    );
                    self.track_morph_control("alpha.option.square", &square);
                    let skin = ui.selectable_value(
                        &mut selection,
                        AlphaSelection::Skin,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CA7F63B73C877),
                    );
                    self.track_morph_control("alpha.option.skin", &skin);
                    for (id, name) in entries {
                        let option =
                            ui.selectable_value(&mut selection, AlphaSelection::Imported(id), name);
                        self.track_morph_control(&format!("alpha.option.imported.{id}"), &option);
                    }
                });
            let selector_response = selector.response.on_hover_text(label);
            self.track_morph_control("alpha.library", &selector_response);
            if selection != self.preferences.tool(self.preferences.selected()).alpha {
                self.select_alpha(selection);
            }
            let conversion = egui::ComboBox::from_id_salt("alpha_conversion")
                .selected_text(match self.alpha_conversion {
                    AlphaConversion::LegacyWeightedOpacity => {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CD3053E29DD1F)
                    }
                    AlphaConversion::SculptGlMeanRgb => {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CDC59BA10F835)
                    }
                })
                .show_ui(ui, |ui| {
                    let weighted = ui.selectable_value(
                        &mut self.alpha_conversion,
                        AlphaConversion::LegacyWeightedOpacity,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CD3053E29DD1F),
                    );
                    self.track_morph_control("alpha.convert.weighted_opacity", &weighted);
                    let mean = ui.selectable_value(
                        &mut self.alpha_conversion,
                        AlphaConversion::SculptGlMeanRgb,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CDC59BA10F835),
                    );
                    self.track_morph_control("alpha.convert.mean_rgb", &mean);
                });
            self.track_morph_control("alpha.conversion", &conversion.response);
            ui.horizontal(|ui| {
                let import = ui
                    .button(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C1BE6D2F01842,
                    ))
                    .on_hover_text(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CF4FB2B02B8FF,
                    ));
                self.track_morph_control("alpha.import", &import);
                if import.clicked() {
                    self.import_alpha();
                }
                let clear = ui.add_enabled(
                    self.brush.alpha.is_some(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CFF7BC7F66552,
                    )),
                );
                self.track_morph_control("alpha.clear", &clear);
                if clear.clicked() {
                    self.select_alpha(AlphaSelection::None);
                }
            });
            let remove = ui.add_enabled(
                matches!(
                    self.preferences.tool(self.preferences.selected()).alpha,
                    AlphaSelection::Imported(_)
                ),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C56AB4FE19F89,
                )),
            );
            self.track_morph_control("alpha.remove", &remove);
            if remove.clicked() {
                self.remove_selected_alpha();
            }
            if self.brush.alpha.is_some() && !source_paint {
                let mut angle = self.brush.alpha_rotation.to_degrees();
                label_value(
                    ui,
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::C8B737D16D7B3),
                    &format!("{angle:.0}°"),
                );
                let rotation = ui.add(egui::Slider::new(&mut angle, 0.0..=360.0).show_value(false));
                self.track_morph_control("alpha.rotation", &rotation);
                if rotation.changed() {
                    self.brush.alpha_rotation = angle.to_radians();
                    self.commit_brush_view();
                }
            }
            ui.add_space(20.0);
        } else if self.brush.kind == BrushKind::Paint {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CECAA211C172F,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        section(
            ui,
            "03",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CE7F3367AA3C3),
        );
        ui.add_space(12.0);
        self.mask_region_panel(ui);
        ui.horizontal(|ui| {
            let clear = ui.add_enabled(
                self.document.active_subtool_visible(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CFF7BC7F66552,
                )),
            );
            self.track_morph_control("mask.clear", &clear);
            self.track_mask_filter_control("mask.clear", &clear);
            if clear.clicked() {
                self.finish_stroke();
                if self.stroke_rejected {
                    return;
                }
                self.document.clear_mask();
                self.changed();
            }
            let invert = ui.add_enabled(
                self.document.active_subtool_visible(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C28A8888DE6F5,
                )),
            );
            self.track_morph_control("mask.invert", &invert);
            self.track_mask_filter_control("mask.invert", &invert);
            if invert.clicked() {
                self.finish_stroke();
                if self.stroke_rejected {
                    return;
                }
                self.document.invert_mask();
                self.changed();
            }
        });
        self.mask_filter_panel(ui);
        ui.add_space(24.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CC6A4DB3EE261,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        ui.add_space(8.0);
        if self.source_shortcut_profile() {
            ui.label(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C8E657D951526,
            ));
        } else {
            for (key, action) in [
                ("Shift", "Smooth"),
                ("Ctrl", "Mask"),
                ("Alt", "Invert"),
                ("[  ]", "Brush radius"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{key:7}"))
                            .monospace()
                            .size(11.0)
                            .color(ACCENT),
                    );
                    ui.label(
                        RichText::new(action)
                            .size(11.0)
                            .color(preferences_host::muted(ui)),
                    );
                });
            }
        }
    }

    fn brush_options_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        let screen = ui.checkbox(
            &mut self.brush_screen_space,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C48AC9C2722B2),
        );
        self.track_morph_control("settings.screen_space", &screen);
        if supports_continuous(self.brush.kind, false) {
            let mut enabled = self.preferences.global().continuous;
            let continuous = ui
                .add_enabled(
                    true,
                    egui::Checkbox::new(
                        &mut enabled,
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CAC6F86452736),
                    ),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CCF7B351FA82F,
                ));
            self.track_morph_control("settings.continuous", &continuous);
            if continuous.changed() {
                self.set_continuous_mode(enabled);
            }
            if enabled && self.brush_lock_position {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C9077D4A91FEF,
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                );
            }
        }
        let locked = ui
            .add_enabled(
                LockedStamp::supports_kind(self.brush.kind),
                egui::Checkbox::new(
                    &mut self.brush_lock_position,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C189B82C8A250),
                ),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CC6AF1CED3192,
            ));
        self.track_morph_control("settings.lock_position", &locked);
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CE4D9EC144301),
            &format!("{:.0} px", self.brush_screen_radius),
        );
        let pixels = ui.add_enabled(
            self.brush_screen_space,
            egui::Slider::new(&mut self.brush_screen_radius, 5.0..=500.0).show_value(false),
        );
        self.track_morph_control("settings.screen_radius", &pixels);
        let numeric_radius = crate::screen_radius_ui::render(
            ui,
            self.brush.kind,
            self.brush_screen_radius,
            self.brush_screen_space,
            &mut self.screen_radius_editor,
        );
        self.track_morph_control(crate::screen_radius_ui::KEY, &numeric_radius.response);
        self.screen_radius_hit_rect
            .record(crate::screen_radius_ui::KEY, Some(&numeric_radius.response));
        if let Some(value) = numeric_radius.candidate {
            self.brush_screen_radius = value;
        }
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CF25E5396EC0E),
            &format!("{:.0}%", self.brush.hardness * 100.0),
        );
        let hardness =
            ui.add(egui::Slider::new(&mut self.brush.hardness, 0.0..=1.0).show_value(false));
        self.track_morph_control("settings.hardness", &hardness);
        let culling = ui.checkbox(
            &mut self.brush.backface_culling,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CDCA99E6F20A8),
        );
        self.track_morph_control("settings.culling", &culling);
        let offset = ui.add(
            egui::DragValue::new(&mut self.brush.symmetry_offset)
                .speed(0.005)
                .range(-1_000_000.0..=1_000_000.0)
                .prefix(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C058247451000,
                )),
        );
        self.track_morph_control("settings.symmetry_offset", &offset);
        let mut changed = locked.changed()
            || screen.changed()
            || pixels.changed()
            || numeric_radius.candidate.is_some()
            || hardness.changed()
            || culling.changed()
            || offset.changed();
        match self.brush.kind {
            BrushKind::Draw => {
                let clay = ui.checkbox(
                    &mut self.brush.draw_clay,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CB872669E1C20),
                );
                self.track_morph_control("settings.draw_clay", &clay);
                let accumulate = ui.checkbox(
                    &mut self.brush.draw_accumulate,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C4D4A7F97846F),
                );
                self.track_morph_control("settings.draw_accumulate", &accumulate);
                changed |= clay.changed() || accumulate.changed();
            }
            BrushKind::Smooth => {
                let tangent = ui.checkbox(
                    &mut self.brush.smooth_tangent,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CA0143501678A),
                );
                self.track_morph_control("settings.smooth_tangent", &tangent);
                changed |= tangent.changed();
            }
            BrushKind::Move => {
                let topology = ui.checkbox(
                    &mut self.brush.move_topology_check,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CF3975F3FCECB),
                );
                self.track_morph_control("settings.move_topology", &topology);
                let normal = ui.checkbox(
                    &mut self.brush.move_along_normal,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CCCF95AFDBE90),
                );
                self.track_morph_control("settings.move_normal", &normal);
                changed |= topology.changed() || normal.changed();
            }
            BrushKind::Paint => self.paint_settings_panel(ui),
            _ => {}
        }
        if changed {
            self.commit_brush_view();
        }
    }

    fn paint_settings_panel(&mut self, ui: &mut egui::Ui) {
        // Release queued endpoints with their old captured settings before
        // committing a new channel selection or target, including LockedStamp.
        let mut rgb = self.brush.paint_rgb;
        let mut rough = self.brush.paint_roughness;
        let mut metal = self.brush.paint_metallic;
        let mut roughness = self.brush.roughness;
        let mut metallic = self.brush.metallic;
        let color = ui.checkbox(
            &mut rgb,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CADF15C2BCBAA),
        );
        self.track_morph_control("settings.paint_rgb", &color);
        let rough_channel = ui.checkbox(
            &mut rough,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CA67E1E58E923),
        );
        self.track_morph_control("settings.paint_roughness", &rough_channel);
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CA17BCDB89EFD),
            &format!("{:.0}%", roughness * 100.0),
        );
        let rough_target = ui.add(egui::Slider::new(&mut roughness, 0.0..=1.0).show_value(false));
        self.track_morph_control("settings.roughness", &rough_target);
        let metal_channel = ui.checkbox(
            &mut metal,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C8C3745F29067),
        );
        self.track_morph_control("settings.paint_metallic", &metal_channel);
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CB0C5D68D33CB),
            &format!("{:.0}%", metallic * 100.0),
        );
        let metal_target = ui.add(egui::Slider::new(&mut metallic, 0.0..=1.0).show_value(false));
        self.track_morph_control("settings.metallic", &metal_target);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C70F1D6BE7E9A,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        if !rgb && !rough && !metal {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C1A8DC4DCA79A,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        if color.changed()
            || rough_channel.changed()
            || metal_channel.changed()
            || rough_target.changed()
            || metal_target.changed()
        {
            self.brush.paint_rgb = rgb;
            self.brush.paint_roughness = rough;
            self.brush.paint_metallic = metal;
            self.brush.roughness = roughness;
            self.brush.metallic = metallic;
            self.commit_brush_view();
        }
        ui.horizontal(|ui| {
            let fill = ui.add_enabled(
                self.document.active_subtool_visible() && self.topology_job.is_none(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C729B2CEA87BC,
                )),
            );
            self.track_morph_control("paint.fill_all", &fill);
            if fill.clicked() {
                self.fill_paint();
            }
            let pick = ui.add_enabled(
                self.document.active_subtool_visible() && self.topology_job.is_none(),
                egui::Button::new(if self.paint_pick_armed {
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C91A20036F554)
                } else {
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CDB351844E20B)
                }),
            );
            self.track_morph_control("paint.pick", &pick);
            if pick.clicked() {
                self.arm_paint_picker();
            }
        });
        if self.paint_pick_armed {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CB728B3D8AF82,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
    }

    fn fill_paint(&mut self) {
        if !self.commit_brush_view() {
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.paint_pick_armed = false;
        self.paint_pick_dragging = false;
        self.cancel_topology();
        let mut settings = self.brush.clone();
        settings.kind = BrushKind::Paint;
        match paint_fill_after_barrier(&mut self.document, &settings, false) {
            Ok(report) if report.changed_vertices > 0 => {
                self.changed();
                self.last_paint_fill = Some(HostPaintFillReport {
                    subtool_id: self.document.active_subtool_id(),
                    revision: self.revision,
                    report,
                });
                if report.changed_rgb_vertices > 0 {
                    self.viewport_style.use_vertex_color = true;
                }
                self.message = format!(
                    "Filled {} points on the active part. Undo restores their paint.",
                    report.changed_vertices
                );
            }
            Ok(report) => {
                self.last_paint_fill = Some(HostPaintFillReport {
                    subtool_id: self.document.active_subtool_id(),
                    revision: self.revision,
                    report,
                });
                self.message = "Paint fill made no changes. History kept.".into();
            }
            Err(error) => self.message = format!("Paint fill rejected: {error}"),
        }
    }

    fn arm_paint_picker(&mut self) {
        self.modal_cancel("legacy picker command");
        if !self.commit_brush_view() {
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        self.paint_pick_armed = !self.paint_pick_armed;
        self.paint_pick_dragging = false;
        self.last_paint_sample = None;
        self.message = if self.paint_pick_armed {
            "Click or drag on the active surface to sample its paint.".into()
        } else {
            "Paint picker cancelled.".into()
        };
    }

    fn sample_paint(&mut self, position: Pos2, rect: egui::Rect) {
        let Some(selected) = self
            .scene_pick(position, rect)
            .filter(|h| Some(h.id) == self.document.active_subtool_id())
        else {
            return;
        };
        let (hit, _rounding) = match selected.local_brush_hit() {
            Ok(v) => v,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        let sample = match self.document.require_active_mesh().and_then(|mesh| {
            mesh.pick_paint_channels_at_triangle(
                &hit,
                selected.triangle_index,
                PaintPickInterpolation::OriginalFaceInverseDistance,
            )
        }) {
            Ok(sample) => sample,
            Err(error) => {
                self.message = format!("Paint sample rejected: {error}");
                return;
            }
        };
        self.brush.color = sample.rgb;
        self.brush.roughness = sample.roughness;
        self.brush.metallic = sample.metallic;
        if self.commit_brush_view() {
            self.last_paint_sample = Some(HostPaintPickReport {
                subtool_id: self.document.active_subtool_id(),
                revision: self.revision,
                hit,
                sample,
            });
            self.message = "Paint sampled; shape and history kept.".into();
        }
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        self.layer_controls.clear();
        self.layer_controls_enabled.clear();
        self.layer_controls_hit_rect.clear();
        if ctx.content_rect().width() < 900.0 {
            let mut open = self.inspector_open;
            egui::Window::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CB3E8D2C9DE2A,
            ))
            .id(egui::Id::new("Object & surface"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(250.0)
            .default_pos(egui::pos2(
                (ctx.content_rect().width() - 285.0).max(10.0),
                70.0,
            ))
            .max_height((ctx.content_rect().height() - 120.0).max(180.0))
            .vscroll(true)
            .show(ctx, |ui| self.inspector_body(ui));
            self.inspector_open = open;
        } else {
            egui::SidePanel::right("inspector")
                .exact_width(235.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(preferences_host::panel(ctx))
                        .inner_margin(egui::Margin::symmetric(18, 18)),
                )
                .show(ctx, |ui| {
                    inspector_layout::scroll(ui, |ui| self.inspector_body(ui));
                });
        }
    }

    fn source_appearance_panel(&mut self, ui: &mut egui::Ui) {
        let Some(metadata) = self.document.active_object_metadata() else {
            return;
        };
        let mut render = metadata
            .render
            .clone()
            .unwrap_or_else(obrush_core::baseline_source_render_defaults);
        let before = render.clone();
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C45F6A7866079,
            ))
            .size(11.0)
            .color(preferences_host::muted(ui)),
        );
        let source_shader = egui::ComboBox::from_id_salt("source_object_shader")
            .selected_text(match render.shader {
                0 => crate::ui_text::caption(crate::ui_text::CaptionKey::C577F3AA38DE7),
                1 => crate::ui_text::caption(crate::ui_text::CaptionKey::C87B4E2BF9896),
                2 => crate::ui_text::caption(crate::ui_text::CaptionKey::CD72F20ED5AB8),
                3 => crate::ui_text::caption(crate::ui_text::CaptionKey::C3E0A4B565275),
                4 => crate::ui_text::caption(crate::ui_text::CaptionKey::C0B17F582BE32),
                5 => crate::ui_text::caption(crate::ui_text::CaptionKey::C5347433F28DE),
                _ => crate::ui_text::caption(crate::ui_text::CaptionKey::CFD76CC07D179),
            })
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (code, _name) in [
                    (0, "PBR"),
                    (1, "Flat"),
                    (2, "Normal"),
                    (3, "Wireframe"),
                    (4, "UV"),
                    (5, "Matcap"),
                ] {
                    let available = code != 4
                        || self
                            .document
                            .active_mesh()
                            .and_then(|m| m.surface.as_ref())
                            .and_then(|s| s.uv.as_ref())
                            .is_some();
                    let row = ui
                        .add_enabled_ui(available, |ui| {
                            ui.selectable_value(
                                &mut render.shader,
                                code,
                                crate::ui_text::source_shader_name(code),
                            )
                        })
                        .inner;
                    self.track_layer_control(&format!("source.shader.{code}"), &row);
                }
            })
            .response;
        self.track_layer_control("source.shader", &source_shader);
        if render.shader == 5 {
            let source_matcap = egui::ComboBox::from_id_salt("source_object_matcap")
                .selected_text(
                    if (render.matcap as usize) < crate::matcap::MATCAP_NAMES.len() {
                        crate::ui_text::matcap_name(render.matcap as usize)
                    } else {
                        "Unresolved Matcap".to_owned()
                    },
                )
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (index, _name) in crate::matcap::MATCAP_NAMES.iter().enumerate() {
                        let row = ui.selectable_value(
                            &mut render.matcap,
                            index as u32,
                            crate::ui_text::matcap_name(index),
                        );
                        self.track_layer_control(&format!("source.matcap.{index}"), &row);
                    }
                })
                .response;
            self.track_layer_control("source.matcap", &source_matcap);
        }
        let flat = ui.checkbox(
            &mut render.flat_shading,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C3A95F39447FA),
        );
        self.track_layer_control("source.flat", &flat);
        let wire = ui.checkbox(
            &mut render.show_wireframe,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C856D2A1F51BF),
        );
        self.track_layer_control("source.wire", &wire);
        let mut transparency = ((1.0 - render.opacity) * 100.0).round() as u32;
        let alpha = ui.add(egui::Slider::new(&mut transparency, 0..=100).text(
            crate::ui_text::caption(crate::ui_text::CaptionKey::C404BB0A38ADC),
        ));
        self.track_layer_control("source.opacity", &alpha);
        if alpha.changed() {
            render.opacity = 1.0 - transparency as f32 / 100.0;
        }
        if render.shader != before.shader
            || render.matcap != before.matcap
            || render.show_wireframe != before.show_wireframe
            || render.flat_shading != before.flat_shading
            || render.opacity.to_bits() != before.opacity.to_bits()
        {
            self.finish_stroke();
            if self.stroke_rejected {
                return;
            }
            let only_flat_changed = render.flat_shading != before.flat_shading
                && render.shader == before.shader
                && render.matcap == before.matcap
                && render.show_wireframe == before.show_wireframe
                && render.opacity.to_bits() == before.opacity.to_bits();
            let only_wireframe_changed = render.show_wireframe != before.show_wireframe
                && render.shader == before.shader
                && render.matcap == before.matcap
                && render.flat_shading == before.flat_shading
                && render.opacity.to_bits() == before.opacity.to_bits();
            let only_transparency_changed = render.opacity.to_bits() != before.opacity.to_bits()
                && render.shader == before.shader
                && render.matcap == before.matcap
                && render.flat_shading == before.flat_shading
                && render.show_wireframe == before.show_wireframe;
            let result = if only_flat_changed {
                self.document
                    .set_selected_source_flat_shading(render.flat_shading)
            } else if only_wireframe_changed {
                self.document
                    .set_selected_source_wireframe(render.show_wireframe)
            } else if only_transparency_changed {
                self.document.set_selected_source_transparency(transparency)
            } else {
                self.document.set_active_source_render(render)
            };
            match result {
                Ok(true) => self.changed(),
                Ok(false) => {}
                Err(error) => self.message = format!("Source appearance rejected: {error}"),
            }
        }
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C7C2D9064FA2A,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
    }

    fn inspector_body(&mut self, ui: &mut egui::Ui) {
        inspector_layout::constrain_content(ui);
        self.subtools_panel(ui);
        self.mask_extract_panel(ui);
        self.transform_panel(ui);
        section(
            ui,
            "05",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CC0859E17D49E),
        );
        ui.add_space(14.0);
        ui.add(egui::Label::new(RichText::new(&self.document.name).strong().size(16.0)).truncate())
            .on_hover_text(&self.document.name);
        ui.add_space(9.0);
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C4E87CE4CDFEB),
            &number(self.document.active_mesh().map_or(0, |m| m.positions.len())),
        );
        label_value(
            ui,
            "Faces",
            &number(self.document.active_mesh().map_or(0, Mesh::face_count)),
        );
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CF6C5A19FCE27),
            &number(self.document.active_mesh().map_or(0, |m| m.triangles.len())),
        );
        if self
            .document
            .active_mesh()
            .is_some_and(Mesh::has_attributes)
        {
            label_value(
                ui,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C44D8C149FF26),
                &number(
                    self.document
                        .active_mesh()
                        .and_then(|m| m.materials.as_ref())
                        .map_or(0, Vec::len),
                ),
            );
            label_value(
                ui,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C2E9F2849C611),
                &number(
                    self.document
                        .active_mesh()
                        .and_then(|m| m.surface.as_ref())
                        .and_then(|s| s.uv.as_ref())
                        .map_or(0, |uv| uv.coordinates.len()),
                ),
            );
            if self
                .document
                .active_mesh()
                .is_some_and(|m| m.surface.is_some())
            {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C9322339DD2B2,
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                );
                let detach = ui.add_enabled(
                    !self.document.has_retained_subdivision_hierarchy()
                        && !self.document.layer_topology_locked(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C3A77138D3BE5,
                    )),
                );
                self.track_layer_control("attributes.detach_surface", &detach);
                if detach.clicked() {
                    self.scene_action(Document::detach_surface_topology,
                        "UVs and original face records removed; geometry and materials kept. Undo restores them.");
                }
            }
        }
        let topology_locked =
            self.document.layer_topology_locked() || !self.document.active_subtool_visible();
        let primitive_locked = self.document.layer_topology_locked()
            || (!self.document.is_empty() && !self.document.active_subtool_visible());
        ui.add_space(13.0);
        egui::ComboBox::from_id_salt("new_sphere_detail")
            .selected_text(match self.primitive_detail {
                5 => crate::ui_text::caption(crate::ui_text::CaptionKey::C5C435BA8362A),
                7 => crate::ui_text::caption(crate::ui_text::CaptionKey::CB13460C496E5),
                _ => crate::ui_text::caption(crate::ui_text::CaptionKey::C689DA08EC6C7),
            })
            .width(190.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.primitive_detail,
                    5,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C5C435BA8362A),
                );
                ui.selectable_value(
                    &mut self.primitive_detail,
                    6,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C689DA08EC6C7),
                );
                ui.selectable_value(
                    &mut self.primitive_detail,
                    7,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CB13460C496E5),
                );
            })
            .response
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CB8EA20FB897B,
            ));
        ui.horizontal(|ui| {
            let sphere = ui.add_enabled(
                !primitive_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7984A45F9F2C,
                )),
            );
            self.track_layer_control("primitive.sphere", &sphere);
            if sphere.clicked() {
                self.primitive(false);
            }
            let cube = ui.add_enabled(
                !primitive_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C219702B49D4B,
                )),
            );
            self.track_layer_control("primitive.cube", &cube);
            if cube.clicked() {
                self.primitive(true);
            }
        });
        if self.document.has_retained_subdivision_hierarchy() {
            self.subdivision_choice = match self.document.retained_static_subdivision_settings() {
                Some(settings) if settings.linear_edges => SubdivisionChoice::StaticLinear,
                Some(_) => SubdivisionChoice::StaticSmooth,
                None => SubdivisionChoice::Loop,
            };
        }
        let mode = ui
            .add_enabled_ui(
                !self.document.has_retained_subdivision_hierarchy() && !topology_locked,
                |ui| {
                    egui::ComboBox::from_id_salt("subdivision_mode")
                        .selected_text(self.subdivision_choice.name())
                        .width(190.0)
                        .show_ui(ui, |ui| {
                            for choice in [
                                SubdivisionChoice::Loop,
                                SubdivisionChoice::StaticSmooth,
                                SubdivisionChoice::StaticLinear,
                            ] {
                                let response = ui.selectable_value(
                                    &mut self.subdivision_choice,
                                    choice,
                                    choice.name(),
                                );
                                self.track_layer_control(
                                    match choice {
                                        SubdivisionChoice::Loop => "object.subdivision.loop",
                                        SubdivisionChoice::StaticSmooth => {
                                            "object.subdivision.static_smooth"
                                        }
                                        SubdivisionChoice::StaticLinear => {
                                            "object.subdivision.static_linear"
                                        }
                                    },
                                    &response,
                                );
                            }
                        })
                        .response
                },
            )
            .inner;
        self.track_layer_control("object.subdivision.mode", &mode);
        mode.on_hover_text(crate::ui_text::caption(
            crate::ui_text::CaptionKey::C8EDFE3E5A112,
        ));
        let add_level = ui
            .add_enabled_ui(!topology_locked, |ui| {
                ui.add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C548AB144F3B7,
                    )),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C5DC265351538,
                ))
            })
            .inner;
        self.track_layer_control("object.add_level", &add_level);
        if add_level.clicked() {
            self.add_subdivision_level();
        }
        let reverse_label = crate::i18n::source_text(
            self.app_preferences.locale,
            crate::source_text_keys::SourceTextKey::MultiresReverse,
            crate::i18n::TextArguments::None,
        )
        .map(|text| text.text.into_owned())
        .unwrap_or_else(|_| "Reverse subdivision".into());
        let reverse = ui.add_enabled(
            !topology_locked && self.document.can_reverse_source_static_subdivision_level(),
            egui::Button::new(reverse_label),
        );
        self.track_layer_control("object.reverse", &reverse);
        let reverse = reverse.on_hover_text(
            "Recover a lower smooth level from a structurally subdivided mesh. Select the lowest level first; existing UV IDs must remain representable.",
        );
        if reverse.clicked() {
            self.reverse_subdivision_level();
        }
        let level = self.document.current_level();
        let levels = self.document.level_count();
        ui.horizontal(|ui| {
            let lower = ui.add_enabled(
                level > 0 && !topology_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CF05D2176850F,
                )),
            );
            self.track_layer_control("object.lower", &lower);
            if lower.clicked() {
                self.select_subdivision_level(level - 1);
            }
            ui.label(format!(
                "{} / {}",
                if levels == 0 { 0 } else { level + 1 },
                levels
            ));
            let higher = ui.add_enabled(
                level + 1 < levels && !topology_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CE802A8747436,
                )),
            );
            self.track_layer_control("object.higher", &higher);
            if higher.clicked() {
                self.select_subdivision_level(level + 1);
            }
        });
        ui.horizontal(|ui| {
            let lower = ui.add_enabled(
                level > 0 && !topology_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C3F2A7C23CAD7,
                )),
            );
            self.track_layer_control("object.delete_lower", &lower);
            if lower.clicked() {
                self.delete_subdivision_levels(true);
            }
            let higher = ui.add_enabled(
                level + 1 < levels && !topology_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7589718A5F7D,
                )),
            );
            self.track_layer_control("object.delete_higher", &higher);
            if higher.clicked() {
                self.delete_subdivision_levels(false);
            }
        });
        if self.document.has_retained_subdivision_hierarchy() {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C6B7975DA72CA,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
        }
        let refine = ui
            .add_enabled(
                !topology_locked,
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C9D2300FF648F,
                ))
                .small(),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C3C189345B7E1,
            ));
        self.track_layer_control("object.refine", &refine);
        if refine.clicked() {
            self.subdivide(false);
        }
        if self.document.layer_topology_locked() {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CB81316F6B3A4,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
        }
        ui.add_space(12.0);
        let rebuild = ui.collapsing(
            crate::ui_text::caption(crate::ui_text::CaptionKey::CF214501F9C03),
            |ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C7F0D9399CB2C,
                    ))
                    .size(11.0)
                    .color(preferences_host::muted(ui)),
                );
                label_value(
                    ui,
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::C75A14B79611F),
                    &self.remesh_resolution.to_string(),
                );
                let resolution = ui.add(
                    egui::Slider::new(&mut self.remesh_resolution, 16..=128).show_value(false),
                );
                self.track_layer_control("object.remesh.resolution", &resolution);
                let smooth_nets =
                    ui.checkbox(&mut self.remesh_surface_nets_mode, "Smooth Surface Nets");
                self.track_layer_control("object.remesh.smooth_nets", &smooth_nets);
                if smooth_nets.clicked() && self.remesh_surface_nets_mode {
                    self.remesh_block_mode = false;
                }
                let block = ui.checkbox(&mut self.remesh_block_mode, "Block remesh");
                self.track_layer_control("object.remesh.block", &block);
                if block.clicked() && self.remesh_block_mode {
                    self.remesh_surface_nets_mode = false;
                }
                let discard_uvs = ui.checkbox(
                    &mut self.remesh_discard_uvs,
                    "Discard UVs during remesh (Undo restores)",
                );
                self.track_layer_control("object.remesh.discard_uvs", &discard_uvs);
                let busy = self.topology_job.is_some() || self.uv_bake_job.is_some();
                let remesh = ui.add_enabled(
                    !busy && !topology_locked,
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CA4277F551BC1,
                    )),
                );
                self.track_layer_control("object.remesh", &remesh);
                if remesh.clicked() {
                    self.start_topology(TopologyOperation::Remesh(RemeshSettings {
                        resolution: self.remesh_resolution,
                        block_mode: self.remesh_block_mode,
                        surface_nets_mode: self.remesh_surface_nets_mode,
                        allow_uv_loss: self.remesh_discard_uvs,
                        ..Default::default()
                    }));
                }
                ui.separator();
                ui.label("Voxel remeshing (manifold triangles)");
                let relax = ui.checkbox(&mut self.remesh_mc_relax, "Relax topology");
                self.track_layer_control("object.remesh.marching_cubes.relax", &relax);
                let mc_remesh = ui.add_enabled(
                    !busy && !topology_locked,
                    egui::Button::new("Remesh triangles"),
                );
                self.track_layer_control("object.remesh.marching_cubes", &mc_remesh);
                if mc_remesh.clicked() {
                    self.start_topology(TopologyOperation::Remesh(RemeshSettings {
                        resolution: self.remesh_resolution,
                        marching_cubes_mode: true,
                        smoothing_steps: if self.remesh_mc_relax { 1 } else { 0 },
                        allow_uv_loss: self.remesh_discard_uvs,
                        ..Default::default()
                    }));
                }
                let flatten = ui.checkbox(
                    &mut self.remesh_flatten_selected,
                    "Flatten selected levels/layers/targets",
                );
                self.track_layer_control("object.remesh.selected.flatten", &flatten);
                let bake_poses = ui.checkbox(
                    &mut self.remesh_bake_selected_poses,
                    "Bake selected poses; discard Source provenance",
                );
                self.track_layer_control("object.remesh.selected.bake_poses", &bake_poses);
                let selected_count = self.document.selected_subtool_ids().len();
                let fuse = ui.add_enabled(
                    !busy
                        && selected_count >= 2
                        && (!topology_locked || self.remesh_flatten_selected),
                    egui::Button::new("Fuse selected meshes"),
                );
                self.track_layer_control("object.remesh.selected", &fuse);
                if fuse.clicked() {
                    self.start_topology(TopologyOperation::RemeshSelected {
                        remesh: RemeshSettings {
                            resolution: self.remesh_resolution,
                            marching_cubes_mode: true,
                            smoothing_steps: if self.remesh_mc_relax { 1 } else { 0 },
                            allow_uv_loss: self.remesh_discard_uvs,
                            allow_source_pose_loss: self.remesh_bake_selected_poses,
                            ..Default::default()
                        },
                        policy: if self.remesh_flatten_selected {
                            SubtoolMergePolicy::FlattenDisplayed
                        } else {
                            SubtoolMergePolicy::RequirePlain
                        },
                    });
                }
                let nets_or_block = self.remesh_surface_nets_mode || self.remesh_block_mode;
                let fuse_voxel = ui.add_enabled(
                    !busy
                        && selected_count >= 2
                        && nets_or_block
                        && (!topology_locked || self.remesh_flatten_selected),
                    egui::Button::new(if self.remesh_block_mode {
                        "Fuse selected with Block remesh"
                    } else if self.remesh_surface_nets_mode {
                        "Fuse selected with Smooth Surface Nets"
                    } else {
                        "Choose a Surface Nets mode to fuse"
                    }),
                );
                self.track_layer_control("object.remesh.selected.nets_block", &fuse_voxel);
                if fuse_voxel.clicked() {
                    self.start_topology(TopologyOperation::RemeshSelected {
                        remesh: RemeshSettings {
                            resolution: self.remesh_resolution,
                            block_mode: self.remesh_block_mode,
                            surface_nets_mode: self.remesh_surface_nets_mode,
                            allow_uv_loss: self.remesh_discard_uvs,
                            allow_source_pose_loss: self.remesh_bake_selected_poses,
                            ..Default::default()
                        },
                        policy: if self.remesh_flatten_selected {
                            SubtoolMergePolicy::FlattenDisplayed
                        } else {
                            SubtoolMergePolicy::RequirePlain
                        },
                    });
                }
                let mut keep = self.decimate_ratio * 100.0;
                label_value(
                    ui,
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::CE7C8AEB2B7AB),
                    &format!("{keep:.0}%"),
                );
                if ui
                    .add(egui::Slider::new(&mut keep, 10.0..=95.0).show_value(false))
                    .changed()
                {
                    self.decimate_ratio = keep / 100.0;
                }
                let decimate = ui
                    .add_enabled(
                        !busy && !topology_locked,
                        egui::Button::new(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::C8368312596B0,
                        )),
                    )
                    .on_hover_text(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C3DF38CCF596B,
                    ));
                self.track_layer_control("object.decimate", &decimate);
                if decimate.clicked() {
                    let (minimum, maximum) = self
                        .document
                        .active_mesh()
                        .into_iter()
                        .flat_map(|m| m.positions.iter())
                        .fold(
                            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                            |(lo, hi), &p| (lo.min(p), hi.max(p)),
                        );
                    self.start_topology(TopologyOperation::Decimate(DecimateSettings {
                        target_ratio: self.decimate_ratio,
                        max_error: minimum.distance(maximum) * 0.02,
                        ..Default::default()
                    }));
                }
            },
        );
        self.track_layer_control("object.rebuild", &rebuild.header_response);
        if let Some(job) = self.topology_job.as_ref() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "Working · {:.1}s",
                    job.started.elapsed().as_secs_f64()
                ));
            });
            if ui
                .button(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C02B587E03098,
                ))
                .clicked()
            {
                self.cancel_topology();
            }
        }
        ui.add_space(24.0);
        self.layers_panel(ui);
        ui.add_space(24.0);
        self.morph_panel(ui);
        ui.add_space(24.0);
        section(
            ui,
            "08",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C53040DD3EFE4),
        );
        self.source_appearance_panel(ui);
        let ordinary_selected = self.document.active_mesh().is_some()
            && self.document.active_object_metadata().is_none();
        let active_shading = self.document.active_mesh().and_then(|mesh| {
            crate::renderer::consumer_style_for_mesh(
                self.document.active_object_metadata(),
                mesh,
                &self.viewport_style,
            )
            .ok()
            .map(|style| style.shading)
        });
        let visible_pbr = self.rendered_scene.iter().any(|object| {
            crate::renderer::consumer_style_for_mesh(
                object.metadata.as_ref(),
                &object.mesh,
                &self.viewport_style,
            )
            .is_ok_and(|style| style.shading == ShadingMode::Pbr)
        });
        ui.add_enabled_ui(ordinary_selected, |ui| {
            let shading = egui::ComboBox::from_id_salt("viewport_shading")
                .selected_text(crate::ui_text::shading_name(self.viewport_style.shading))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for mode in [
                        ShadingMode::Studio,
                        ShadingMode::Pbr,
                        ShadingMode::Normal,
                        ShadingMode::Uv,
                        ShadingMode::Matcap,
                        ShadingMode::Flat,
                    ] {
                        let available = mode != ShadingMode::Uv
                            || self
                                .document
                                .active_mesh()
                                .and_then(|m| m.surface.as_ref())
                                .and_then(|s| s.uv.as_ref())
                                .is_some();
                        let row = ui
                            .add_enabled_ui(available, |ui| {
                                ui.selectable_value(
                                    &mut self.viewport_style.shading,
                                    mode,
                                    crate::ui_text::shading_name(mode),
                                )
                            })
                            .inner;
                        self.track_layer_control(
                            &format!(
                                "viewport.shading.{}",
                                mode.name().to_ascii_lowercase().replace(' ', "_")
                            ),
                            &row,
                        );
                    }
                })
                .response;
            self.track_layer_control("viewport.shading", &shading);
            if self.viewport_style.shading == ShadingMode::Matcap {
                let selected = self.viewport_style.matcap_index as usize;
                egui::ComboBox::from_id_salt("viewport_matcap")
                    .selected_text(crate::ui_text::matcap_name(selected))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for (index, _name) in crate::matcap::MATCAP_NAMES.iter().enumerate() {
                            let row = ui.selectable_value(
                                &mut self.viewport_style.matcap_index,
                                index as u32,
                                crate::ui_text::matcap_name(index),
                            );
                            self.track_layer_control(&format!("viewport.matcap.{index}"), &row);
                        }
                    });
            }
            let flat = ui.checkbox(
                &mut self.viewport_style.flat_shading,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C93D818E52343),
            );
            self.track_layer_control("viewport.flat_shading", &flat);
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CDB684170E645,
                ))
                .size(11.0)
                .color(preferences_host::muted(ui)),
            );
            let value = ui
                .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.viewport_style.curvature)
                            .range(0.0..=5.0)
                            .speed(0.05)
                            .fixed_decimals(2),
                    )
                })
                .inner;
            self.track_layer_control("viewport.curvature_value", &value);
        });
        let curvature = ui.add(
            egui::Slider::new(&mut self.viewport_style.curvature, 0.0..=5.0).show_value(false),
        );
        self.track_layer_control("viewport.curvature", &curvature);
        ui.add_enabled_ui(ordinary_selected, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::CA09A9445B430,
                    ))
                    .size(11.0)
                    .color(preferences_host::muted(ui)),
                );
                let value = ui
                    .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.viewport_style.opacity)
                                .range(0.0..=1.0)
                                .speed(0.01)
                                .fixed_decimals(2),
                        )
                    })
                    .inner;
                self.track_layer_control("viewport.opacity_value", &value);
            });
            let opacity = ui.add(
                egui::Slider::new(&mut self.viewport_style.opacity, 0.0..=1.0).show_value(false),
            );
            self.track_layer_control("viewport.opacity", &opacity);
        });
        if active_shading == Some(ShadingMode::Uv) {
            ui.label(
                self.viewport_style
                    .uv_texture
                    .as_ref()
                    .map_or("UV checker", |t| t.name.as_str()),
            );
            let import = ui.button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CF3F850B891AC,
            ));
            self.track_layer_control("viewport.uv_import", &import);
            if import.clicked() {
                self.import_uv_texture();
            }
            let reset = ui.add_enabled(
                self.viewport_style.uv_texture.is_some(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CAC4F31BFC3D4,
                )),
            );
            self.track_layer_control("viewport.uv_reset", &reset);
            if reset.clicked()
                && let Err(error) = self.reset_uv_texture()
            {
                self.message = error;
            }
        }
        label_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CF5602B18C2D9),
            &format!("{:.2}", self.viewport_style.exposure),
        );
        let exposure = ui.add_enabled(
            visible_pbr,
            egui::Slider::new(&mut self.viewport_style.exposure, 0.0..=5.0).show_value(false),
        );
        self.track_layer_control("viewport.exposure", &exposure);
        self.environment_panel(ui);
        ui.add_space(13.0);
        ui.add_enabled_ui(ordinary_selected, |ui| {
            ui.horizontal(|ui| {
                for (label, color) in [
                    ("Clay", [0.61, 0.38, 0.28]),
                    ("Stone", [0.62, 0.64, 0.66]),
                    ("Jade", [0.25, 0.48, 0.39]),
                ] {
                    let selected =
                        self.viewport_style.color == color && !self.viewport_style.use_vertex_color;
                    if ui.selectable_label(selected, label).clicked() {
                        self.viewport_style.color = color;
                        self.viewport_style.use_vertex_color = false;
                    }
                }
            });
            ui.add_space(13.0);
            let wire = ui
                .checkbox(
                    &mut self.viewport_style.wireframe,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C9ACCC7445A77),
                )
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C22B57379AED8,
                ));
            let wire = wire.on_hover_text(
                crate::ui_text::command_hint(crate::shortcut_profile::Command::Wireframe)
                    .unwrap_or_else(|| "No assigned shortcut in this profile".to_owned()),
            );
            self.track_layer_control("viewport.wireframe", &wire);
        });
        let antialias = ui
            .checkbox(
                &mut self.viewport_style.antialias,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CEF491282C4FE),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CF91846123F5B,
            ));
        self.track_layer_control("viewport.antialias", &antialias);
        for (key, response) in crate::background_cursor::render_profile(
            ui,
            &mut self.viewport_style.background_cursor.profile,
        ) {
            self.track_layer_control(key, &response);
        }
        if self.viewport_style.antialias {
            let status = self.renderer.lock().ok().map(|renderer| {
                (
                    renderer.antialias_samples(),
                    renderer.antialias_algorithm(),
                    renderer.antialias_error().map(str::to_owned),
                )
            });
            if let Some((samples, algorithm, error)) = status {
                if algorithm == "fxaa" {
                    ui.label(
                        RichText::new(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::CD342B0124270,
                        ))
                        .size(10.0)
                        .color(preferences_host::muted(ui)),
                    );
                } else if samples > 1 {
                    ui.label(
                        RichText::new(format!("{samples}× anti-aliasing active"))
                            .size(10.0)
                            .color(preferences_host::muted(ui)),
                    );
                } else if let Some(error) = error {
                    ui.label(
                        RichText::new(crate::ui_text::caption(
                            crate::ui_text::CaptionKey::C48CD47AF01EA,
                        ))
                        .size(10.0)
                        .color(preferences_host::muted(ui)),
                    )
                    .on_hover_text(error);
                } else {
                    ui.ctx().request_repaint();
                }
            }
        }
        let show_mask = ui.checkbox(
            &mut self.viewport_style.show_mask,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C803979D8917F),
        );
        self.track_layer_control("viewport.show_mask", &show_mask);
        let show_paint = ui
            .add_enabled_ui(ordinary_selected, |ui| {
                ui.checkbox(
                    &mut self.viewport_style.use_vertex_color,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C8309D228EC76),
                )
            })
            .inner;
        self.track_layer_control("viewport.show_vertex_color", &show_paint);
        ui.add_space(16.0);
        if !(self.brush.kind == BrushKind::Paint
            && self.source_paint_preferences.mode
                == crate::source_paint_host::PaintMode::SourceOrdinary)
        {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C218C4BF11852,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
            ui.add_space(8.0);
            let mut color_draft = self.brush.color;
            let color_picker = ui.color_edit_button_rgb(&mut color_draft);
            self.track_morph_control("settings.color", &color_picker);
            if color_picker.changed() {
                self.brush.color = color_draft;
                self.commit_brush_view();
            }
            ui.horizontal(|ui| {
                for (index, color) in [
                    [0.72, 0.34, 0.22],
                    [0.22, 0.45, 0.66],
                    [0.3, 0.58, 0.36],
                    [0.85, 0.71, 0.35],
                    [0.78, 0.75, 0.71],
                ]
                .into_iter()
                .enumerate()
                {
                    let fill = Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    );
                    let swatch = ui.add(
                        egui::Button::new(" ")
                            .min_size(Vec2::splat(24.0))
                            .fill(fill),
                    );
                    self.track_morph_control(&format!("settings.color_swatch.{index}"), &swatch);
                    if swatch.clicked() && self.select_brush(BrushKind::Paint) {
                        self.brush.color = color;
                        if self.commit_brush_view() {
                            self.viewport_style.use_vertex_color = true;
                        }
                    }
                }
            });
            ui.add_space(24.0);
        } else {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C5A56A2F6F982,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }
        self.camera_panel(ui);
        self.mesh_files_panel(ui);
        section(
            ui,
            "11",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C0DB451BE86C9),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.document.can_undo(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C752CC88F4CAA,
                    )),
                )
                .clicked()
            {
                self.history(false);
            }
            if ui
                .add_enabled(
                    self.document.can_redo(),
                    egui::Button::new(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C707DD6DE65D6,
                    )),
                )
                .clicked()
            {
                self.history(true);
            }
        });
        let history_label = crate::i18n::source_text(
            self.app_preferences.locale,
            crate::source_text_keys::SourceTextKey::StateMaxStack,
            crate::i18n::TextArguments::None,
        )
        .map(|text| text.text.into_owned())
        .unwrap_or_else(|_| "History length".into());
        let mut history_limit = self.document.history_limit();
        ui.label(history_label);
        let prior_slider_width = ui.spacing().slider_width;
        ui.spacing_mut().slider_width = 140.0;
        let limit_control = ui
            .add(egui::Slider::new(&mut history_limit, 3..=50))
            .on_hover_text("Large scenes may retain fewer steps under the memory limit.");
        ui.spacing_mut().slider_width = prior_slider_width;
        self.track_layer_control("history.limit", &limit_control);
        if limit_control.changed() {
            self.finish_stroke();
            if let Err(error) = self.document.set_history_limit(history_limit) {
                self.message = format!("History length rejected: {error}");
            }
        }
        ui.add_space(21.0);
        ui.separator();
        ui.add_space(14.0);
        if ui
            .button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C9ED18C72A105,
            ))
            .clicked()
        {
            self.frame_scene();
        }
        ui.add_space(9.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C5C2AEB9985AF,
            ))
            .size(11.0)
            .color(preferences_host::muted(ui)),
        );
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C09B529E2E0EF,
            ))
            .size(11.0)
            .color(preferences_host::muted(ui)),
        );
        ui.add_space(18.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C0FCAE54E4CE8,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
    }

    fn change_camera_framing(&mut self, enabled: bool) -> Result<(), String> {
        self.finish_stroke();
        if self.stroke_rejected {
            return Err("Finish the restored stroke before changing camera framing".into());
        }
        self.cancel_transform_drag();
        self.cancel_mask_region("Camera framing changed; selection cancelled.", true);
        let viewport =
            crate::source_camera_host::measured_viewport(&self.egui_ctx, self.last_viewport)?;
        let mut candidate = self.camera.clone();
        candidate.set_source_framing(enabled, viewport)?;
        candidate.prepare_source_frame(viewport, &self.document.subtool_render_snapshots())?;
        self.camera = candidate;
        self.source_scale_command = None;
        self.camera_navigation_active = false;
        self.egui_ctx.request_repaint();
        Ok(())
    }

    /// Only the optional source policy changes viewport ordering. Legacy retains
    /// its existing input/process/render order and arithmetic. Static source pivot motion remains disabled. The resulting frame is
    /// immutable for this frame's geometry sampling and painting.
    fn prepare_source_viewport(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
    ) -> Result<(), String> {
        self.source_navigation_prepared = false;
        if self.source_boot_framing_pending {
            let viewport = crate::source_camera_host::measured_viewport(ctx, response.rect)?;
            let mut candidate = self.camera.clone();
            // Whole-scene SGL Open starts from SculptGL's reset view, whose
            // center is the origin when the pre-fit clip range is optimized.
            candidate.source_camera = None;
            candidate.target = Vec3::ZERO;
            candidate.precise_target = Some(glam::DVec3::ZERO);
            candidate.set_source_framing(true, viewport)?;
            let objects = self.document.subtool_render_snapshots();
            // SculptGL has already optimized the clip range for the loaded
            // scene when resetCameraMeshes computes its frustum-fit factor.
            // Seed that retained near plane before the source fit.
            candidate.prepare_source_frame(viewport, &objects)?;
            candidate.frame_scene(&objects);
            candidate.prepare_source_frame(viewport, &objects)?;
            self.camera = candidate;
            self.source_boot_framing_pending = false;
        }
        if !self.camera.source_enabled() {
            self.source_frame_revision = None;
            self.source_scale_command = None;
            return Ok(());
        }
        let viewport = crate::source_camera_host::measured_viewport(ctx, response.rect)?;
        self.camera.measure_source_navigation(viewport);
        if let Some((orthographic, value)) = self.source_scale_command.take() {
            let result = if orthographic {
                self.camera.set_source_view_height(value)
            } else {
                self.camera.set_source_projection_zoom(value)
            };
            if let Err(error) = result {
                self.message = format!("Camera scale rejected: {error}");
            }
        }
        let (focused, delta, scroll, navigation) = ctx.input(|i| {
            (
                i.focused,
                i.pointer.delta(),
                i.raw_scroll_delta.y,
                viewport_navigation_requested(i, self.navigation_down()),
            )
        });
        if focused
            && self.pending_sgl_export.is_none()
            && self.transform_drag.is_none()
            && self.mask_region_gesture.is_none()
            && !self.region_suppressed_frame
        {
            if navigation && self.stroke.is_some() {
                self.interrupt_source_paint_tail("Source Paint navigation interrupted the contact");
                self.cancel_locked_stamp();
                self.finish_stroke();
            }
            if self.stroke.is_none() {
                self.navigate_camera(ctx, response, delta, scroll);
                self.source_navigation_prepared = true;
            }
        }
        self.refresh_rendered_scene();
        self.camera
            .prepare_source_frame(viewport, &self.rendered_scene)?;
        self.source_frame_revision = Some(self.revision);
        Ok(())
    }

    fn camera_panel(&mut self, ui: &mut egui::Ui) {
        section(
            ui,
            "09",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CCD829A253D85),
        );
        ui.add_space(12.0);
        let previous_framing = self.camera.source_enabled();
        let mut source_framing = previous_framing;
        let framing_label = if source_framing {
            crate::ui_text::caption(crate::ui_text::CaptionKey::C499EEDC4E01E)
        } else {
            "OBrush".to_owned()
        };
        let framing = inspector_layout::combo(ui, "camera_framing", &framing_label, |ui| {
            for (enabled, label, key) in [
                (false, "OBrush", "camera.framing.legacy"),
                (true, "SculptGL static", "camera.framing.source"),
            ] {
                let row = ui.selectable_value(&mut source_framing, enabled, label);
                self.track_layer_control(key, &row);
            }
        })
        .response;
        self.track_layer_control("camera.framing", &framing);
        if source_framing != previous_framing
            && let Err(error) = self.change_camera_framing(source_framing)
        {
            self.message = format!("Camera framing rejected: {error}");
        }
        let previous = (
            self.camera.orthographic,
            self.camera.fov_degrees,
            self.camera.mode,
            self.camera.speed,
            self.camera.use_pivot,
        );
        let projection = ui.checkbox(
            &mut self.camera.orthographic,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C6BFC8C503C21),
        );
        self.track_layer_control("camera.orthographic", &projection);
        let fov = if self.camera.source_enabled() {
            let mut draft = self.camera.fov_degrees;
            let response = ui.add_enabled(
                !self.camera.orthographic,
                egui::DragValue::new(&mut draft)
                    .prefix(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C5E0B2C13B602,
                    ))
                    .range(0.001..=179.999)
                    .clamp_existing_to_range(false)
                    .speed(1.0),
            );
            if response.changed() {
                if draft.is_finite() && draft > 0.0 && draft < 180.0 {
                    self.camera.fov_degrees = draft;
                } else {
                    self.message = "FOV must be finite and between 0 and 180 degrees.".into();
                }
            }
            response
        } else {
            ui.add_enabled(
                !self.camera.orthographic,
                egui::DragValue::new(&mut self.camera.fov_degrees)
                    .prefix(crate::ui_text::caption(
                        crate::ui_text::CaptionKey::C5E0B2C13B602,
                    ))
                    .range(10.0..=90.0)
                    .speed(1.0),
            )
        };
        if fov.changed() {
            self.camera.boot_source_fov = None;
        }
        self.track_layer_control("camera.fov", &fov);
        if self.camera.source_enabled() {
            let orthographic = self.camera.orthographic;
            let mut value = if orthographic {
                self.camera.source_view_height().unwrap_or(1.0)
            } else {
                self.camera.source_projection_zoom().unwrap_or(1.0)
            };
            let response = ui.add(
                egui::DragValue::new(&mut value)
                    .prefix(if orthographic {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C2F4AFEB04806)
                    } else {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CC13311FF9FF6)
                    })
                    .range(0.00001..=100_000_000.0)
                    .clamp_existing_to_range(false)
                    .speed(0.01)
                    .max_decimals(8),
            );
            self.track_layer_control(
                if orthographic {
                    "camera.view_height"
                } else {
                    "camera.projection_zoom"
                },
                &response,
            );
            if response.changed() {
                self.finish_stroke();
                self.cancel_transform_drag();
                if !self.stroke_rejected {
                    self.source_scale_command = Some((orthographic, value));
                }
                self.egui_ctx.request_repaint();
            }
            inspector_layout::prose(
                ui,
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CF5D41785D514,
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
        }

        let mode = inspector_layout::combo(
            ui,
            "camera_mode",
            &crate::ui_text::camera_mode_name(self.camera.mode),
            |ui| {
                for mode in [CameraMode::Orbit, CameraMode::Spherical, CameraMode::Plane] {
                    let row = ui.selectable_value(
                        &mut self.camera.mode,
                        mode,
                        crate::ui_text::camera_mode_name(mode),
                    );
                    self.track_layer_control(
                        &format!("camera.mode.{}", mode.name().to_ascii_lowercase()),
                        &row,
                    );
                }
            },
        )
        .response;
        self.track_layer_control("camera.mode", &mode);
        inspector_layout::navigation_value(
            ui,
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CB480949ED50E),
            &format!("{:.0}%", self.camera.speed * 100.0),
        );
        let speed = ui.add(egui::Slider::new(&mut self.camera.speed, 0.05..=1.0).show_value(false));
        self.track_layer_control("camera.speed", &speed);
        let source_framing = self.camera.source_enabled();
        let pivot = ui
            .add_enabled(
                !source_framing,
                egui::Checkbox::new(
                    &mut self.camera.use_pivot,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CB591FA407121),
                ),
            )
            .on_disabled_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C9370B67E7131,
            ));
        self.track_layer_control("camera.pivot", &pivot);
        if previous
            != (
                self.camera.orthographic,
                self.camera.fov_degrees,
                self.camera.mode,
                self.camera.speed,
                self.camera.use_pivot,
            )
        {
            self.cancel_transform_drag();
            self.finish_stroke();
            self.camera_navigation_active = false;
        }
        let [fit, front, left, top] = inspector_layout::camera_actions(ui);
        self.track_layer_control("camera.center", &fit);
        if fit.clicked() {
            self.finish_stroke();
            self.cancel_transform_drag();
            self.frame_scene();
        }
        for (axis, key, view) in [
            (0, "camera.front", front),
            (1, "camera.left", left),
            (2, "camera.top", top),
        ] {
            self.track_layer_control(key, &view);
            if view.clicked() {
                self.finish_stroke();
                self.cancel_transform_drag();
                self.camera.axis_view(axis);
            }
        }
        if self.camera.source_enabled() {
            let reset = ui.button("Reset view");
            self.track_layer_control("camera.reset", &reset);
            if reset.clicked() {
                self.modal_cancel("camera reset");
                self.finish_stroke();
                self.cancel_transform_drag();
                if !self.stroke_rejected {
                    self.camera_navigation_active = false;
                    self.camera.reset_source_view();
                }
            }
        }
        inspector_layout::prose(
            ui,
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C1F51DB0D93A3,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        ui.add_space(20.0);
    }

    fn mesh_files_panel(&mut self, ui: &mut egui::Ui) {
        section(
            ui,
            "10",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CC5875DE51DBD),
        );
        ui.add_space(12.0);
        let source_order = inspector_layout::combo(
            ui,
            "source_sgl_order",
            self.source_sgl_import.policy.name(),
            |ui| {
                for policy in [
                    crate::source_sgl_import::ByteOrderPolicy::ReferenceNative,
                    crate::source_sgl_import::ByteOrderPolicy::LittleEndian,
                    crate::source_sgl_import::ByteOrderPolicy::BigEndian,
                ] {
                    let row = ui.selectable_value(
                        &mut self.source_sgl_import.policy,
                        policy,
                        policy.name(),
                    );
                    self.track_layer_control(
                        &format!("file.source_sgl.order.{}", policy.key()),
                        &row,
                    );
                }
            },
        )
        .response;
        self.track_layer_control("file.source_sgl.order", &source_order);
        let scale_center = ui
            .checkbox(
                &mut self.source_sgl_import.options.scale_center,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CB74FE24C7254),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CD0625C68DA0D,
            ));
        self.track_layer_control("file.source_sgl.scale_center", &scale_center);
        let vertex_srgb = ui
            .checkbox(
                &mut self.source_sgl_import.options.vertex_srgb,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CC0AAB1B128DF),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CF3680B8703EA,
            ));
        self.track_layer_control("file.source_sgl.vertex_srgb", &vertex_srgb);
        let add = ui
            .button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C37416D9B13ED,
            ))
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C86096F6BA3BC,
            ));
        self.track_layer_control("file.source_sgl.add", &add);
        if add.clicked() {
            self.pick_source_sgl();
        }
        ui.add_space(10.0);
        let format =
            inspector_layout::combo(ui, "mesh_export_format", self.export_format.name(), |ui| {
                for format in [
                    ExportFormat::Obj,
                    ExportFormat::StlAscii,
                    ExportFormat::StlBinary,
                    ExportFormat::PlyAscii,
                    ExportFormat::PlyBinary,
                ] {
                    let row = ui.selectable_value(&mut self.export_format, format, format.name());
                    let key = match format {
                        ExportFormat::Obj => "obj",
                        ExportFormat::StlAscii => "stl_ascii",
                        ExportFormat::StlBinary => "stl_binary",
                        ExportFormat::PlyAscii => "ply_ascii",
                        ExportFormat::PlyBinary => "ply_binary",
                    };
                    self.track_layer_control(&format!("file.export_format.{key}"), &row);
                }
            })
            .response;
        self.track_layer_control("file.export_format", &format);
        if self.app_preferences.shortcut_profile
            == crate::app_preferences::PreferenceProfile::SculptGl
            && self.export_format == ExportFormat::Obj
        {
            let zbrush = ui.checkbox(
                &mut self.obj_export_zbrush_color,
                "OBJ color zbrush (#MRGB/#MAT)",
            );
            self.track_layer_control("file.export_obj.zbrush", &zbrush);
            let append = ui.checkbox(&mut self.obj_export_append_rgb, "OBJ color append (v RGB)");
            self.track_layer_control("file.export_obj.append_rgb", &append);
            if self.obj_export_zbrush_color {
                inspector_layout::prose(ui, RichText::new("OBJ RGB, material and mask channels are quantized to 8 bits; the project keeps their original values.").size(10.0).color(MUTED));
            }
        }
        let selected_scope = ui.add_enabled(
            !self.document.selected_subtool_ids().is_empty(),
            egui::Checkbox::new(
                &mut self.sgl_export_selected_only,
                "Selected parts only (SGL and mesh exports)",
            ),
        );
        self.track_layer_control("file.export_sgl.selected_only", &selected_scope);
        let exports = sgl_export_host::mesh_export_action_controls(
            ui,
            self.document.active_mesh(),
            &mut self.export_allow_attribute_loss,
        );
        if let Some(allow) = &exports.allow_attribute_loss {
            self.track_layer_control("file.allow_attribute_loss", allow);
        }
        self.track_layer_control("file.export_sgl", &exports.scene);
        if exports.scene.clicked() {
            self.begin_sgl_export();
        }
        self.track_layer_control("file.export_mesh", &exports.active);
        if exports.active.clicked() {
            self.export_mesh();
        }
        self.uv_bake_panel(ui);
        inspector_layout::prose(
            ui,
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C1F3E8A33E7AB,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        if let Some(report) = &self.last_export_report
            && report != &AttributeExportReport::default()
        {
            inspector_layout::prose(ui,RichText::new(format!("Last mesh export omitted {} material rows, {} source mask values, {} original faces and {} UV coordinates. Project data was kept.",
                    number(report.omitted_material_vertices),number(report.omitted_source_mask_vertices),number(report.omitted_original_faces),number(report.omitted_uv_coordinates)))
                    .size(10.0).color(ACCENT));
        }
        if let Some(report) = &self.last_scene_geometry_report {
            if report.omitted_color_vertices > 0 {
                inspector_layout::prose(ui, RichText::new(format!("Last mesh export omitted {} RGB rows because both OBJ color switches were off.", number(report.omitted_color_vertices))).size(10.0).color(ACCENT));
            }
            if report.quantized_color_vertices > 0
                || report.quantized_material_vertices > 0
                || report.quantized_mask_vertices > 0
            {
                inspector_layout::prose(ui, RichText::new(format!("Last OBJ export 8-bit maximum errors: RGB {:.6}, material {:.6}, mask {:.6}. Project values were kept.", report.maximum_color_error, report.maximum_material_error, report.maximum_mask_error)).size(10.0).color(ACCENT));
            }
        }
        if let Some(report) = self.last_import_report.as_ref() {
            inspector_layout::prose(
                ui,
                RichText::new(format!(
                    "Last import: {} points / {} triangles",
                    report
                        .source_vertices
                        .saturating_sub(report.welded_vertices),
                    report.triangulated_faces
                ))
                .size(10.0)
                .color(preferences_host::muted(ui)),
            );
            for warning in &report.warnings {
                inspector_layout::prose(
                    ui,
                    RichText::new(warning)
                        .size(10.0)
                        .color(preferences_host::muted(ui)),
                );
            }
            if !report.skipped_properties.is_empty() {
                inspector_layout::prose(
                    ui,
                    RichText::new(format!(
                        "Skipped fields: {}",
                        report.skipped_properties.join(", ")
                    ))
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
                );
            }
        }
        ui.add_space(20.0);
    }

    fn navigate_camera(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        delta: Vec2,
        scroll: f32,
    ) {
        if self.source_navigation_prepared {
            return;
        }
        let (secondary, middle, point, shift) = ctx.input(|input| {
            (
                input.pointer.secondary_down(),
                input.pointer.middle_down(),
                input.pointer.latest_pos(),
                input.modifiers.shift,
            )
        });
        let secondary = secondary || self.browser_navigation.secondary_down();
        let middle = middle || self.browser_navigation.middle_down();
        let navigation = secondary || middle;
        let rect = response.rect;
        let captured = self.browser_navigation_coordinates
            || self.navigation_capture.pending()
            || self.navigation_capture.held();
        for motion in self.navigation_capture.take() {
            let origin = Pos2::from(motion.origin);
            if !rect.contains(origin) {
                continue;
            }
            let start = Pos2::from(motion.start);
            let end = Pos2::from(motion.end);
            match motion.button {
                NavigationButton::Orbit => {
                    if motion.first
                        && !self.camera.source_enabled()
                        && self.camera.use_pivot
                        && let Some(hit) = self.pick(origin, rect)
                        && let Ok(point) =
                            crate::scene_space::world_point(&self.document, hit.position)
                    {
                        self.camera.set_pivot(point.as_vec3());
                    }
                    let ndc = |position: Pos2| {
                        glam::Vec2::new(
                            (position.x - rect.min.x) / rect.width() * 2.0 - 1.0,
                            1.0 - (position.y - rect.min.y) / rect.height() * 2.0,
                        )
                    };
                    self.camera.rotate(ndc(start), ndc(end), end - start);
                    if motion.shift {
                        self.camera.snap_axis();
                    }
                }
                NavigationButton::Pan => self.camera.pan(end - start),
            }
        }
        if response.hovered() && !captured {
            if secondary {
                let point = point.unwrap_or(rect.center());
                if !self.camera_navigation_active
                    && !self.camera.source_enabled()
                    && self.camera.use_pivot
                    && let Some(hit) = self.pick(point, rect)
                    && let Ok(point) = crate::scene_space::world_point(&self.document, hit.position)
                {
                    self.camera.set_pivot(point.as_vec3());
                }
                let ndc = |position: Pos2| {
                    glam::Vec2::new(
                        (position.x - rect.min.x) / rect.width() * 2.0 - 1.0,
                        1.0 - (position.y - rect.min.y) / rect.height() * 2.0,
                    )
                };
                self.camera.rotate(ndc(point - delta), ndc(point), delta);
                if shift {
                    self.camera.snap_axis();
                }
            }
            if middle {
                self.camera.pan(delta);
            }
        }
        if response.hovered() && scroll != 0.0 {
            self.camera.scroll_zoom(scroll);
        }
        self.camera_navigation_active = navigation;
    }

    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::top_panel(ctx))
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.dirty {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C4B91AA94895E)
                        } else {
                            crate::ui_text::caption(crate::ui_text::CaptionKey::C43E1A5DAC71D)
                        })
                        .color(ACCENT),
                    );
                    ui.label(
                        RichText::new(&self.message)
                            .size(11.0)
                            .color(preferences_host::muted(ui)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.brush.symmetry_x {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::C9E74CD3D8D6C)
                            } else {
                                crate::ui_text::caption(crate::ui_text::CaptionKey::CCE3D63DAB0CC)
                            })
                            .size(10.0)
                            .color(preferences_host::muted(ui)),
                        );
                    });
                });
            });
    }

    fn scene_pick(&self, screen: Pos2, rect: egui::Rect) -> Option<obrush_core::SceneRayHit64> {
        self.document
            .raycast_scene(
                self.screen_scene_ray(screen, rect),
                crate::scene_space::picking_policy(&self.document).ok()?,
            )
            .ok()
            .flatten()
    }
    fn pick(&self, screen: Pos2, rect: egui::Rect) -> Option<Hit> {
        self.scene_pick(screen, rect)
            .filter(|h| Some(h.id) == self.document.active_subtool_id())?
            .local_brush_hit()
            .ok()
            .map(|(h, _)| h)
    }
    fn screen_ray(&self, screen: Pos2, rect: egui::Rect) -> Ray {
        camera_ray_at(&self.camera, screen, rect)
    }
    fn screen_scene_ray(&self, screen: Pos2, rect: egui::Rect) -> obrush_core::SceneRay64 {
        if !self.camera.source_enabled() && self.rendered_scene.iter().all(|o| o.metadata.is_none())
        {
            return self.screen_ray(screen, rect).into();
        }
        let ndc = glam::Vec2::new(
            2.0 * (screen.x - rect.left()) / rect.width() - 1.0,
            1.0 - 2.0 * (screen.y - rect.top()) / rect.height(),
        );
        self.camera.ray64(ndc, rect.aspect_ratio())
    }
    fn local_depth(&self, point: Vec3) -> Result<f32, String> {
        if !self.camera.source_enabled() && self.document.active_object_metadata().is_none() {
            return Ok((point - self.camera.position())
                .dot(self.camera.axes().2)
                .max(0.0001));
        }
        let depth = (crate::scene_space::world_point(&self.document, point)?
            - self.camera.position64())
        .dot(self.camera.axes64().2);
        if !depth.is_finite() {
            return Err("Affine camera depth exceeds finite range".into());
        }
        Ok((depth as f32).max(0.0001))
    }
    fn active_local_projection(&self, rect: egui::Rect) -> Result<[f32; 16], String> {
        let Some(metadata) = self.document.active_object_metadata() else {
            return Ok(self
                .camera
                .view_projection(rect.aspect_ratio())
                .to_cols_array());
        };
        let pose = obrush_core::ObjectRenderTransform::for_object(Some(metadata))?;
        let (mv, _) = pose.model_view(self.camera.view64().to_cols_array())?;
        let projection =
            self.camera.projection(rect.aspect_ratio()) * glam::Mat4::from_cols_array(&mv);
        if !projection.is_finite() {
            return Err("Local region projection exceeds finite range".into());
        }
        Ok(projection.to_cols_array())
    }
    fn background_mask_input(&mut self, ctx: &egui::Context, response: &egui::Response) -> bool {
        let rect = response.rect;
        let (events, focused, down, navigation) = ctx.input(|input| {
            (
                input.events.clone(),
                input.focused,
                input.pointer.primary_down(),
                viewport_navigation_requested(input, self.navigation_down()),
            )
        });
        if let Some(gesture) = self.background_mask_gesture
            && (!focused
                || navigation
                || !gesture.matches(rect, self.document.active_subtool_id(), self.revision))
        {
            self.background_mask_gesture = None;
            return true;
        }
        let mut consumed = self.background_mask_gesture.is_some();
        let mut completed = None;
        for event in events {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } => {
                    if self.background_mask_gesture.is_some() {
                        self.background_mask_gesture = None;
                        return true;
                    }
                    if !focused
                        || navigation
                        || !modifiers.ctrl
                        || self.mask_selection_mode != MaskSelectionMode::Brush
                        || !rect.contains(pos)
                        || !self.document.active_subtool_visible()
                        || self.topology_job.is_some()
                        || ctx
                            .layer_id_at(pos)
                            .is_some_and(|layer| layer != response.layer_id)
                        || self.scene_pick(pos, rect).is_some()
                    {
                        continue;
                    }
                    self.finish_stroke_inner();
                    if self.stroke_rejected {
                        return true;
                    }
                    if let Some(id) = self.document.active_subtool_id() {
                        self.background_mask_gesture =
                            BackgroundMaskGesture::new(pos, rect, id, self.revision);
                        consumed = self.background_mask_gesture.is_some();
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(gesture) = self.background_mask_gesture.as_mut() {
                        gesture.move_to(pos);
                    }
                }
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(gesture) = self.background_mask_gesture.take() {
                        completed = Some(gesture.release());
                        consumed = true;
                    }
                }
                egui::Event::PointerGone if self.background_mask_gesture.take().is_some() => {
                    return true;
                }
                _ => {}
            }
        }
        if let Some(action) = completed {
            match action {
                BackgroundMaskAction::Invert => self.document.invert_mask(),
                BackgroundMaskAction::Clear => self.document.clear_mask(),
            }
            self.changed();
            self.message = match action {
                BackgroundMaskAction::Invert => "Active mask inverted from the background.",
                BackgroundMaskAction::Clear => "Active mask cleared from the background.",
            }
            .into();
        } else if !down && self.background_mask_gesture.take().is_some() {
            consumed = true;
        }
        consumed
    }
    fn sculpt_input(&mut self, ctx: &egui::Context, response: &egui::Response) {
        if self.pending_sgl_export.is_some() {
            return;
        }
        if self.transform_mode || self.transform_drag.is_some() {
            self.transform_input(ctx, response);
            return;
        }
        if self.mask_region_input(ctx, response) {
            return;
        }
        if self.background_mask_input(ctx, response) {
            return;
        }
        self.unprocessed_release = None;
        let rect = response.rect;
        let (pos, pressed, down, released, delta, scroll, modifiers, focused) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.delta(),
                i.raw_scroll_delta.y,
                i.modifiers,
                i.focused,
            )
        });
        if released {
            self.continuous_clock.stop(StopReason::Released);
            self.brush.pressure = self.contact_pressure.endpoint();
        }
        if !focused {
            self.paint_pick_armed = false;
            self.paint_pick_dragging = false;
            self.cancel_absolute_locked_stamp();
        }
        if self.brush.kind != BrushKind::Paint || (!down && !released && self.paint_pick_dragging) {
            self.paint_pick_armed = false;
            self.paint_pick_dragging = false;
        }
        if !focused || (!down && !released) {
            self.interrupt_source_paint_tail("Source Paint contact/focus was interrupted");
            self.finish_stroke_deferred();
        }
        if !focused {
            return;
        }
        if self
            .stroke
            .as_ref()
            .and_then(|stroke| stroke.locked_stamp.as_ref())
            .is_some_and(|stamp| stamp.viewport != rect)
        {
            self.cancel_locked_stamp();
            return;
        }
        let navigation =
            ctx.input(|input| viewport_navigation_requested(input, self.navigation_down()));
        if navigation && self.stroke.is_some() {
            self.interrupt_source_paint_tail("Source Paint navigation interrupted the contact");
            self.cancel_absolute_locked_stamp();
            self.finish_stroke();
        }
        if self.stroke.is_none() {
            self.navigate_camera(ctx, response, delta, scroll);
        }
        if self.topology_job.is_some() {
            return;
        }
        let locked_drag = self.stroke.as_ref().is_some_and(|stroke| {
            stroke.locked_stamp.is_some()
                || stroke.source_paint.as_ref().is_some_and(|s| {
                    s.lock().is_ok_and(|s| {
                        s.locked_mode() == Some(obrush_core::SourcePaintLockedMode::AbsolutePreview)
                    })
                })
        });
        let Some(pos) = pos.filter(|p| p.is_finite() && (rect.contains(*p) || locked_drag)) else {
            if self.interrupt_source_paint_tail("Source Paint left its viewport") {
                self.unprocessed_release = None;
                self.finish_stroke_deferred();
                return;
            }
            if released {
                self.paint_pick_armed = false;
                self.paint_pick_dragging = false;
                self.finish_stroke_deferred();
            } else if let Some(stroke) = self.stroke.as_mut() {
                stroke.last_screen = None;
                stroke.sampler.break_path();
            }
            return;
        };
        if ctx.input(|input| viewport_navigation_requested(input, self.navigation_down())) {
            if released {
                self.finish_stroke_deferred();
            }
            return;
        }
        if self.paint_pick_armed && (modifiers.ctrl || modifiers.shift) {
            self.paint_pick_armed = false;
            self.paint_pick_dragging = false;
        }
        if self.paint_pick_armed && self.brush.kind == BrushKind::Paint {
            let source_mode = self.source_paint_preferences.mode
                == crate::source_paint_host::PaintMode::SourceOrdinary;
            if pressed && response.hovered() && !source_mode {
                self.paint_pick_dragging = true;
            }
            if self.paint_pick_dragging && (pressed || down || released) {
                self.sample_paint(pos, rect);
            }
            if released || !focused {
                self.paint_pick_armed = false;
                self.paint_pick_dragging = false;
            }
            // Picking owns this contact and never creates a brush or queued dab.
            return;
        }
        let kind = if modifiers.ctrl {
            BrushKind::Mask
        } else if modifiers.shift {
            BrushKind::Smooth
        } else {
            self.brush.kind
        };
        let source_geometry_requested = self.source_geometry_requested(kind);
        let source_requested = kind == BrushKind::Paint
            && self.source_paint_preferences.mode
                == crate::source_paint_host::PaintMode::SourceOrdinary;
        let restarting = down
            && self.stroke.as_ref().is_some_and(|stroke| {
                stroke.kind != kind
                    || ((stroke.locked_stamp.is_some()
                        || stroke.continuous_locked.is_some()
                        || stroke.source_geometry.is_some()
                        || matches!(
                            kind,
                            BrushKind::Move
                                | BrushKind::Twist
                                | BrushKind::LocalScale
                                | BrushKind::Drag
                        ))
                        && stroke.invert != modifiers.alt)
            });
        if restarting || (pressed && kind == BrushKind::Move) {
            // A modifier starts a new gesture against the committed surface.
            // Otherwise returning from Smooth to Move could replace that edit
            // with the frozen surface from the earlier Move gesture.
            self.finish_stroke();
        }
        if pressed && response.hovered() && !source_requested {
            self.finish_stroke();
            if self.stroke_rejected {
                return;
            }
            if let Some(hit) = self.scene_pick(pos, rect)
                && Some(hit.id) != self.document.active_subtool_id()
            {
                self.scene_action(
                    |document| document.select_subtool(hit.id),
                    "Active part selected.",
                );
            }
        }
        if source_geometry_requested && (pressed || restarting) && response.hovered() {
            self.begin_source_geometry_stroke(ctx, pos, rect, kind, modifiers.alt);
            return;
        }
        if source_requested && (pressed || restarting) && response.hovered() {
            self.begin_source_paint_stroke(ctx, pos, rect, modifiers.alt);
            return;
        }
        if (pressed || restarting)
            && response.hovered()
            && self.preferences.global().continuous
            && self.preferences.tool(kind).lock_position
        {
            self.begin_continuous_locked(ctx, pos, rect, kind, modifiers.alt);
            return;
        }
        if (pressed || restarting)
            && response.hovered()
            && let Some(hit) = self.pick(pos, rect)
        {
            self.finish_stroke_deferred();
            let locked =
                self.preferences.tool(kind).lock_position && LockedStamp::supports_kind(kind);
            let continuous_mode =
                self.preferences.global().continuous && supports_continuous(kind, false);
            if continuous_mode && locked {
                self.message = "Turn off lock or continuous before starting this brush.".into();
                return;
            }
            let queued = !locked
                && !matches!(
                    kind,
                    BrushKind::Move | BrushKind::Twist | BrushKind::LocalScale
                );
            let forward = if self.camera.source_enabled() {
                self.camera.axes().2
            } else {
                (self.camera.target - self.camera.position()).normalize()
            };
            let depth = self.local_depth(hit.position).unwrap_or(0.0001);
            let locked_stamp = if locked {
                let Some(mut settings) = self.saved_brush_at(hit.position, rect, kind) else {
                    return;
                };
                settings.kind = kind;
                settings.invert ^= modifiers.alt;
                let tangent =
                    match crate::scene_space::local_vector(&self.document, self.camera.axes().0) {
                        Ok(v) => v,
                        Err(error) => {
                            self.message = error;
                            return;
                        }
                    };
                settings.alpha_tangent = Some(tangent);
                settings.culling_axis = crate::scene_space::local_vector(
                    &self.document,
                    -self.screen_scene_ray(pos, rect).direction.as_vec3(),
                )
                .ok();
                match self.document.require_active_mesh().and_then(|mesh| {
                    LockedStamp::prepare(mesh, hit.position, hit.normal, &settings)
                }) {
                    Ok(stamp) => Some(ActiveLockedStamp {
                        stamp,
                        start: pos,
                        viewport: rect,
                        units_per_pixel: self.camera.world_units_per_pixel(
                            depth,
                            rect.height(),
                            rect.aspect_ratio(),
                        ) / crate::scene_space::source_scale(&self.document)
                            .unwrap_or(1.0) as f32,
                        tangent,
                        base_rotation: settings.alpha_rotation,
                        dirty_before: self.dirty,
                        vertex_color_before: self.viewport_style.use_vertex_color,
                        paints_rgb: settings.paint_rgb,
                    }),
                    Err(error) => {
                        self.message = format!("Stamp could not start: {error}");
                        return;
                    }
                }
            } else {
                None
            };
            let movement = if kind == BrushKind::Move {
                let (right, up, _) = self.camera.axes();
                let Some(mut settings) = self.saved_brush_at(hit.position, rect, kind) else {
                    return;
                };
                settings.kind = kind;
                settings.alpha_tangent =
                    crate::scene_space::local_vector(&self.document, right).ok();
                settings.culling_axis = crate::scene_space::local_vector(
                    &self.document,
                    -self.screen_scene_ray(pos, rect).direction.as_vec3(),
                )
                .ok();
                settings.invert ^= modifiers.alt;
                // Pen-down may precede positive contact. Defer selection then,
                // so pressure-radius captures the first real contact radius.
                let prepared = if settings.pressure > 0.0 {
                    self.document
                        .require_active_mesh()
                        .and_then(|mesh| {
                            mesh.prepare_move_stroke_with_seed(
                                hit.position,
                                hit.normal,
                                &settings,
                                Some(hit.triangle),
                            )
                        })
                        .map(Some)
                } else {
                    Ok(None)
                };
                match prepared {
                    Ok(stroke) => {
                        let units_per_pixel = self.camera.world_units_per_pixel(
                            depth,
                            rect.height(),
                            rect.aspect_ratio(),
                        );
                        Some(ActiveMove {
                            stroke,
                            screen: MoveDrag::new(right, up, units_per_pixel),
                            hit,
                            selection: settings,
                            last_pressure: self.brush.pressure,
                            start: pos,
                            units_per_pixel,
                        })
                    }
                    Err(error) => {
                        self.message = format!("Move could not start: {error}");
                        return;
                    }
                }
            } else {
                None
            };
            let local_transform = if matches!(kind, BrushKind::Twist | BrushKind::LocalScale) {
                let Some(mut settings) = self.saved_brush_at(hit.position, rect, kind) else {
                    return;
                };
                settings.kind = kind;
                settings.invert ^= modifiers.alt;
                settings.alpha_tangent =
                    crate::scene_space::local_vector(&self.document, self.camera.axes().0).ok();
                settings.culling_axis = crate::scene_space::local_vector(
                    &self.document,
                    -self.screen_scene_ray(pos, rect).direction.as_vec3(),
                )
                .ok();
                let axis = match crate::scene_space::local_vector(&self.document, -forward) {
                    Ok(v) => v,
                    Err(error) => {
                        self.message = error;
                        return;
                    }
                };
                let prepared = if settings.pressure > 0.0 {
                    self.document
                        .require_active_mesh()
                        .and_then(|mesh| {
                            mesh.prepare_local_transform_stroke(
                                hit.position,
                                hit.normal,
                                axis,
                                &settings,
                            )
                        })
                        .map(Some)
                } else {
                    Ok(None)
                };
                match prepared {
                    Ok(stroke) => Some(ActiveLocalTransform {
                        stroke,
                        hit,
                        axis,
                        selection: settings,
                        start: pos,
                        pivot_screen: pos,
                        total_parameter: 0.0,
                        last_pressure: self.brush.pressure,
                        last_vector: None,
                    }),
                    Err(error) => {
                        self.message = format!("Cannot start {}: {error}", kind.name());
                        return;
                    }
                }
            } else {
                None
            };
            let Some(settings) = self.saved_brush_at(hit.position, rect, kind) else {
                return;
            };
            let continuous = if continuous_mode {
                let Some(next) = self.next_continuous_serial.checked_add(1) else {
                    self.message = "Continuous stroke identifiers are exhausted.".into();
                    return;
                };
                let key = StrokeKey {
                    serial: self.next_continuous_serial,
                    subtool_id: match self.document.active_subtool_id() {
                        Some(id) => id,
                        None => return,
                    },
                    kind,
                };
                let Some(now) = self.continuous_now() else {
                    self.message = "Continuous clock exceeded its supported duration.".into();
                    return;
                };
                if let Err(error) = self.continuous_clock.start(key, true, false, now) {
                    self.message = error;
                    return;
                }
                self.next_continuous_serial = next;
                Some(key)
            } else {
                None
            };
            if queued {
                if kind == BrushKind::Drag {
                    self.pending_dabs.begin_drag(hit.position, hit.normal);
                } else {
                    self.pending_dabs.begin();
                }
            } else {
                if kind == BrushKind::Paint && (settings.paint_roughness || settings.paint_metallic)
                {
                    self.document.begin_material_paint_stroke();
                } else {
                    self.document.begin_stroke();
                }
            }
            let radius_px = settings.effective_radius().unwrap_or(settings.radius)
                / self
                    .camera
                    .world_units_per_pixel(depth, rect.height(), rect.aspect_ratio());
            self.stroke = Some(ActiveStroke {
                settings,
                screen_radius: ScreenRadiusPreferences {
                    enabled: self.preferences.tool(kind).screen_space_radius,
                    pixels: self.preferences.tool(kind).screen_radius_pixels,
                },
                pressure_factors: pressure_factors_for_stroke(&self.preferences, kind),
                last_screen: Some(pos),
                sampler: StrokeSampler::new(pos, (radius_px * 0.12).max(1.0)),
                movement,
                local_transform,
                locked_stamp,
                kind,
                invert: modifiers.alt,
                queued,
                continuous,
                source_geometry: None,
                source_geometry_dirty_before: None,
                source_paint: None,
                source_paint_display_before: None,
                continuous_locked: None,
            });
            if queued && kind != BrushKind::Drag && continuous.is_none() {
                self.stamp(pos, rect, kind, modifiers.alt, Vec3::ZERO);
            }
        } else if (down || released) && self.stroke.is_some() {
            let (sample_kind, invert) = if released {
                let stroke = self.stroke.as_ref().unwrap();
                (stroke.kind, stroke.invert)
            } else {
                (kind, modifiers.alt)
            };
            let replay = self
                .stroke
                .as_ref()
                .is_some_and(browser_pen_ordinary_stroke)
                && self.last_input_pen
                && !modifiers.shift
                && !modifiers.ctrl
                && !modifiers.alt
                && ctx.zoom_factor() == 1.0;
            if replay {
                let (samples, overflowed) = self.browser_pen_samples.drain();
                if overflowed {
                    if let Some(stroke) = self.stroke.as_mut() {
                        stroke.last_screen = None;
                        stroke.sampler.break_path();
                    }
                    self.message =
                        "Pen input exceeded the per-frame sample limit; the path was split.".into();
                }
                for sample in samples {
                    if self.stroke.is_none() {
                        break;
                    }
                    if rect.contains(sample.pos) {
                        self.brush.pressure = sample.pressure;
                        self.sample_stroke_to(sample.pos, rect, sample_kind, invert);
                    } else if let Some(stroke) = self.stroke.as_mut() {
                        stroke.last_screen = None;
                        stroke.sampler.break_path();
                    }
                }
                self.brush.pressure = if released {
                    self.contact_pressure.endpoint()
                } else {
                    self.contact_pressure.current()
                };
            } else {
                self.browser_pen_samples.clear();
            }
            if self.stroke.is_some() {
                self.sample_stroke_to(pos, rect, sample_kind, invert);
            }
        }
        if released {
            self.browser_pen_samples.clear();
            self.finish_stroke_deferred();
        } else if !down {
            self.browser_pen_samples.clear();
        }
    }

    fn sample_stroke_to(&mut self, pos: Pos2, rect: egui::Rect, kind: BrushKind, invert: bool) {
        self.stroke.as_mut().unwrap().invert = invert;
        if let Some(stroke) = self.stroke.as_mut().filter(|s| s.continuous.is_some()) {
            stroke.last_screen = Some(pos);
            stroke.sampler.skip_to(pos);
            return;
        }
        if let Some(session) = self.stroke.as_ref().and_then(|s| s.source_geometry.clone()) {
            self.sample_source_geometry_to(session, pos, rect);
            return;
        }
        if let Some(session) = self.stroke.as_ref().and_then(|s| s.source_paint.clone()) {
            if invert {
                if let Ok(mut source) = session.lock()
                    && source.is_locked()
                {
                    let _ = source.cancel_locked(&mut self.document);
                }
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                self.stroke_rejected = true;
                self.message = "Source Paint has no inverse operation.".into();
                return;
            }
            let prepared = self
                .capture_source_paint_input(pos, rect)
                .and_then(|input| {
                    session
                        .lock()
                        .map_err(|_| "Source Paint session lock failed".to_string())?
                        .path_samples(input)
                });
            match prepared {
                Ok(samples) => {
                    for input in samples {
                        self.pending_dabs.source_paint_dab(session.clone(), input);
                    }
                }
                Err(error) => {
                    if let Ok(mut source) = session.lock() {
                        if source.is_locked() {
                            let _ = source.cancel_locked(&mut self.document);
                        }
                        source.failed = Some(error.clone());
                    }
                    self.stop_continuous(StopReason::Command);
                    self.finish_stroke_deferred();
                    self.stroke_rejected = true;
                    self.message = format!("Source Paint path rejected: {error}");
                }
            }
            return;
        }
        let (previous, stroke_kind) = {
            let stroke = self.stroke.as_ref().unwrap();
            (stroke.last_screen, stroke.kind)
        };
        if let Some(locked) = self.stroke.as_mut().unwrap().locked_stamp.as_mut() {
            let vertex_color_before = locked.vertex_color_before;
            let paints_rgb = locked.paints_rgb;
            match self
                .document
                .require_active_mesh_mut()
                .and_then(|mesh| sample_locked_stamp(locked, mesh, pos, self.brush.pressure))
            {
                Ok(report) => {
                    if report.changed() {
                        self.changed();
                    }
                    if report.kind == BrushKind::Paint && paints_rgb && report.changed() {
                        self.viewport_style.use_vertex_color = true;
                    }
                    if report.at_origin {
                        self.viewport_style.use_vertex_color = vertex_color_before;
                    }
                    self.last_stamp_report = Some(HostLockedStampReport {
                        subtool_id: self.document.active_subtool_id(),
                        revision: self.revision,
                        report,
                    });
                }
                Err(error) => self.message = format!("Stamp preview rejected: {error}"),
            }
            if let Some(stroke) = self.stroke.as_mut() {
                stroke.last_screen = Some(pos);
                stroke.sampler.skip_to(pos);
            }
            return;
        }
        if matches!(stroke_kind, BrushKind::Twist | BrushKind::LocalScale) && kind == stroke_kind {
            let mut settings = self.stroke.as_ref().unwrap().settings.clone();
            settings.pressure = self.brush.pressure;
            settings.kind = kind;
            settings.invert ^= invert;
            let Some(local) = self.stroke.as_mut().unwrap().local_transform.as_mut() else {
                return;
            };
            settings.culling_axis = local.selection.culling_axis;
            let changed_input =
                pos != previous.unwrap_or(pos) || local.last_pressure != settings.pressure;
            if kind == BrushKind::LocalScale {
                local.total_parameter = (pos.x - local.start.x) * 0.01;
            } else {
                let vector = pos - local.pivot_screen;
                if vector.length() >= 30.0 {
                    let vector = vector.normalized();
                    if let Some(old) = local.last_vector {
                        let cross = vector.x * old.y - vector.y * old.x;
                        local.total_parameter += cross.atan2(vector.dot(old));
                    }
                    local.last_vector = Some(vector);
                }
            }
            local.last_pressure = settings.pressure;
            let outcome = (|| {
                if local.stroke.is_none() && settings.pressure > 0.0 {
                    let mut selection = local.selection.clone();
                    selection.pressure = settings.pressure;
                    local.stroke = Some(
                        self.document
                            .require_active_mesh()?
                            .prepare_local_transform_stroke(
                                local.hit.position,
                                local.hit.normal,
                                local.axis,
                                &selection,
                            )?,
                    );
                }
                let mesh = self.document.require_active_mesh_mut()?;
                Ok::<_, String>(if changed_input {
                    local
                        .stroke
                        .as_mut()
                        .map_or_else(BrushResult::default, |stroke| {
                            stroke.apply(mesh, local.total_parameter, &settings)
                        })
                } else {
                    BrushResult::default()
                })
            })();
            match outcome {
                Ok(result) => {
                    if result.changed_vertices > 0 {
                        self.changed();
                    }
                    if result.changed_vertices > 0 && result.displacement_scale < 1.0 {
                        self.message = format!("{} limited to protect the surface.", kind.name());
                    }
                }
                Err(error) => self.message = format!("{} could not start: {error}", kind.name()),
            }
            if let Some(stroke) = self.stroke.as_mut() {
                stroke.last_screen = Some(pos);
                stroke.sampler.skip_to(pos);
            }
        } else if stroke_kind == BrushKind::Move && kind == BrushKind::Move {
            let screen_delta = pos - previous.unwrap_or(pos);
            let pressure_changed = self
                .stroke
                .as_ref()
                .unwrap()
                .movement
                .as_ref()
                .is_some_and(|movement| movement.last_pressure != self.brush.pressure);
            if screen_delta.length_sq() > 0.0 || pressure_changed {
                let mut settings = self.stroke.as_ref().unwrap().settings.clone();
                settings.pressure = self.brush.pressure;
                settings.kind = BrushKind::Move;
                settings.invert ^= invert;
                let Some(movement) = self.stroke.as_mut().unwrap().movement.as_mut() else {
                    return;
                };
                settings.culling_axis = movement.selection.culling_axis;
                let total_delta = movement.screen.advance(previous.unwrap_or(pos), pos);
                movement.last_pressure = settings.pressure;
                let outcome: Result<BrushResult, String> = (|| {
                    if movement.stroke.is_none() && settings.pressure > 0.0 {
                        let mut selection = movement.selection.clone();
                        selection.pressure = settings.pressure;
                        movement.stroke = Some(
                            self.document
                                .require_active_mesh()?
                                .prepare_move_stroke_with_seed(
                                    movement.hit.position,
                                    movement.hit.normal,
                                    &selection,
                                    Some(movement.hit.triangle),
                                )?,
                        );
                    }
                    let total_delta =
                        crate::scene_space::local_vector(&self.document, total_delta)?;
                    let source_scale = crate::scene_space::source_scale(&self.document)? as f32;
                    let mesh = self.document.require_active_mesh_mut()?;
                    Ok(movement
                        .stroke
                        .as_mut()
                        .map_or_else(BrushResult::default, |stroke| {
                            if settings.move_along_normal {
                                let travel = pos - movement.start;
                                let distance = travel.length()
                                    * if travel.x < 0.0 {
                                        -movement.units_per_pixel
                                    } else {
                                        movement.units_per_pixel
                                    };
                                stroke.apply_normal_distance(
                                    mesh,
                                    distance / source_scale,
                                    &settings,
                                )
                            } else {
                                stroke.apply(mesh, total_delta, &settings)
                            }
                        }))
                })();
                let result = match outcome {
                    Ok(result) => result,
                    Err(error) => {
                        self.message = format!("Move could not start: {error}");
                        BrushResult::default()
                    }
                };
                let changed = result.changed_vertices > 0;
                if changed {
                    self.changed();
                }
                if result.detail_limited {
                    self.message =
                        "Move detail reached a limit. Add a subdivision level for finer pulls."
                            .into();
                } else if changed && result.displacement_scale < 1.0 {
                    self.message = "Move was limited to protect the surface.".into();
                }
            }
            if let Some(stroke) = self.stroke.as_mut() {
                stroke.last_screen = Some(pos);
                stroke.sampler.skip_to(pos);
            }
        } else {
            let points = self.stroke.as_mut().unwrap().sampler.sample_to(pos);
            for point in points {
                self.stamp(point, rect, kind, invert, Vec3::ZERO);
            }
            if let Some(stroke) = self.stroke.as_mut() {
                stroke.last_screen = Some(pos);
            }
        }
    }

    fn stamp(&mut self, pos: Pos2, rect: egui::Rect, kind: BrushKind, invert: bool, delta: Vec3) {
        if let Some(session) = self.stroke.as_ref().and_then(|s| s.source_geometry.clone()) {
            self.stamp_source_geometry(session, pos, rect);
            return;
        }
        if let Some(session) = self.stroke.as_ref().and_then(|s| s.source_paint.clone()) {
            let outcome = self
                .capture_source_paint_input(pos, rect)
                .and_then(|input| {
                    crate::source_paint_host::apply_shared(&mut self.document, &session, input)
                });
            match outcome {
                Ok(report) => {
                    if report.changed_vertices > 0 {
                        self.changed();
                    }
                    if report.changed_rgb_vertices > 0 {
                        self.viewport_style.use_vertex_color = true;
                    }
                }
                Err(error) => {
                    if let Ok(mut source) = session.lock() {
                        source.failed = Some(error.clone());
                    }
                    self.unprocessed_release = None;
                    self.stop_continuous(StopReason::Command);
                    self.finish_stroke_deferred();
                    self.message = format!("Source Paint rejected: {error}");
                    self.stroke_rejected = true;
                }
            }
            return;
        }
        let Some(stroke) = self.stroke.as_ref() else {
            return;
        };
        let Some(CapturedDab {
            ray: captured_ray,
            settings,
            footprint,
        }) = captured_stroke_dab(
            stroke,
            &self.camera,
            pos,
            rect,
            kind,
            invert,
            self.current_contact_input(),
        )
        else {
            return;
        };
        let ray = if !self.camera.source_enabled()
            && self.rendered_scene.iter().all(|o| o.metadata.is_none())
        {
            obrush_core::SceneRay64 {
                origin: captured_ray.origin.as_dvec3(),
                direction: captured_ray.direction.as_dvec3(),
            }
        } else {
            self.screen_scene_ray(pos, rect)
        };
        let mut settings = local_settings(&self.document, settings);
        if self.stroke.as_ref().is_some_and(|stroke| stroke.queued) {
            // Capture the input camera and brush settings; queued picking still
            // follows the evolving mesh when this ordered dab is applied.
            if footprint.is_some() {
                self.pending_dabs
                    .dab_scene_with_footprint(ray, settings, delta, footprint);
            } else {
                self.pending_dabs
                    .dab_scene_with_footprint(ray, settings, delta, None);
            }
            return;
        }
        if let Some(hit) = self.pick(pos, rect) {
            if let Some(footprint) = footprint {
                let Some(radius) = footprint.radius_at_world(&self.document, hit.position) else {
                    return;
                };
                settings.radius = radius;
            }
            if kind == BrushKind::Paint && (settings.paint_roughness || settings.paint_metallic) {
                self.document.begin_material_paint_stroke();
            }
            let result = if kind == BrushKind::Morph {
                self.document
                    .apply_morph_brush_with_result(hit.position, hit.normal, &settings)
                    .map(|result| result.changed_vertices)
            } else {
                self.document
                    .require_active_mesh_mut()
                    .map(|mesh| mesh.apply_brush(hit.position, hit.normal, delta, &settings))
            };
            match result {
                Ok(changed) if changed > 0 => {
                    self.changed();
                    if kind == BrushKind::Paint && settings.paint_rgb {
                        self.viewport_style.use_vertex_color = true;
                    }
                }
                Err(error) => self.message = format!("Morph brush unavailable: {error}"),
                _ => {}
            }
        }
    }

    fn viewport(&mut self, ctx: &egui::Context) {
        self.sgl_export_review(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(20, 22, 26)))
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                self.last_viewport = rect;
                self.trace_keyboard_route(ctx, "viewport_before_focus");
                crate::viewport_focus::handoff_viewport_focus(ctx, rect);
                self.trace_keyboard_route(ctx, "viewport_after_focus");
                // Observe actual bounded viewport input, never DOM/f64 guesses.
                self.background_cursor_pointer.observe_egui(
                    ctx,
                    &response,
                    !self.document.is_empty(),
                    self.pending_sgl_export.is_some(),
                );
                let source_ready = self.prepare_source_viewport(ctx, &response);
                if source_ready.is_ok() {
                    self.modal_flush_picker(ctx, rect);
                    if !self.modal_suppresses_contact() {
                        self.sculpt_input(ctx, &response);
                    }
                    self.process_dabs(64, Some(std::time::Duration::from_millis(8)));
                    self.continuous_frame(ctx, rect);
                } else if let Err(error) = source_ready {
                    self.modal_cancel("prepared frame rejected");
                    // No input or GPU fallback may consume an invalid requested frame.
                    if let Some(source) = self.camera.source_camera.as_mut() {
                        source.error = Some(error.clone());
                    }
                    self.cancel_locked_stamp();
                    self.finish_stroke();
                    self.message = format!("Camera frame rejected: {error}");
                }
                self.source_navigation_prepared = false;
                if self.camera.source_enabled() && self.source_frame_revision != Some(self.revision)
                {
                    // Dabs can change clip bounds after publication. Publish the
                    // next accepted frame on the next GUI frame, never mutate
                    // this frame between picking and rendering.
                    ctx.request_repaint();
                }
                if !self.pending_dabs.is_empty() {
                    ctx.request_repaint();
                }
                self.refresh_rendered_scene();
                self.source_modal.cursor_observation = serde_json::Value::Null;
                self.modal_prepare_snapshot(ctx);
                self.modal_cursor(ctx, rect);
                self.viewport_style.background_cursor.frame = None;
                self.viewport_style.background_cursor.unavailable = None;
                if self.viewport_style.background_cursor.profile
                    == crate::background_cursor::CursorProfile::SourceBackground
                {
                    let idle = self.stroke.is_none()
                        && self.pending_dabs.is_empty()
                        && self.transform_drag.is_none()
                        && !self.transform_mode
                        && self.mask_region_gesture.is_none()
                        && !self.region_masking_context(ctx)
                        && !self.camera_navigation_active
                        && self.pending_sgl_export.is_none()
                        && ctx.input(|i| {
                            !i.pointer.any_down()
                                && !viewport_navigation_requested(i, self.navigation_down())
                        });
                    match crate::source_camera_host::measured_viewport(ctx, rect) {
                        Ok(viewport) => {
                            let scope = crate::background_cursor::CursorScope {
                                viewport: viewport.render_pixels,
                                radius: self.brush_screen_radius,
                                pixels_per_point: ctx.pixels_per_point(),
                                zoom_factor: ctx.zoom_factor(),
                                empty_document: self.document.is_empty(),
                                empty_scene: self.rendered_scene.is_empty(),
                                no_pick: self.document.is_empty()
                                    && self.document.active_subtool_id().is_none(),
                                idle,
                                screen_space: self.brush_screen_space,
                                symmetry: self.brush.symmetry_x,
                            };
                            match self.background_cursor_pointer.capture(scope) {
                                Ok(frame) => {
                                    self.viewport_style.background_cursor.frame = Some(frame)
                                }
                                Err(error) => {
                                    self.viewport_style.background_cursor.unavailable = Some(error)
                                }
                            }
                        }
                        Err(_) => {
                            self.viewport_style.background_cursor.unavailable =
                                Some("Cursor viewport measurement rejected")
                        }
                    }
                }
                let renderer = Arc::clone(&self.renderer);
                let scene = self.rendered_scene.clone();
                use crate::source_geometry_host::SourceNormalRenderOwner;
                let normal_owner = match self.stroke.as_ref() {
                    None => SourceNormalRenderOwner::Idle,
                    Some(stroke) if stroke.source_geometry.is_some() => {
                        SourceNormalRenderOwner::Geometry
                    }
                    Some(stroke) if stroke.source_paint.is_some() => {
                        let captured = stroke
                            .source_paint
                            .as_ref()
                            .and_then(|session| session.lock().ok())
                            .and_then(|session| {
                                session
                                    .retained_normal_state()
                                    .map(|state| (session.subtool, state))
                            });
                        SourceNormalRenderOwner::CapturedPaint(captured)
                    }
                    Some(_) => SourceNormalRenderOwner::Legacy,
                };
                let geometry_normals = crate::source_geometry_host::normal_render_states(
                    &self.document,
                    scene.iter().map(|object| object.id),
                    normal_owner,
                );
                let camera = self.camera.paint_snapshot();
                let mut style = self.viewport_style.clone();
                style.source_display.capture(&self.document, &self.brush);
                style.preview_subtool_ids = self
                    .transform_drag
                    .as_ref()
                    .map_or_else(Vec::new, |drag| drag.selected_ids.clone());
                ui.painter().add(egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                        if let Ok(mut renderer) = renderer.lock() {
                            renderer.paint_scene_with_geometry_normals(
                                painter.gl(),
                                painter.intermediate_fbo(),
                                info,
                                &scene,
                                &geometry_normals,
                                &camera,
                                &style,
                            );
                        }
                    })),
                });
                let painter = ui.painter();
                self.draw_transform_gizmo(painter, rect);
                self.draw_mask_region_overlay(painter);
                if self.region_masking_context(ctx) && response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if let Some(error) = self
                    .renderer
                    .lock()
                    .ok()
                    .and_then(|renderer| renderer.scene_error().map(str::to_owned))
                {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Viewport: {error}"),
                        egui::FontId::proportional(13.0),
                        ACCENT,
                    );
                }
                painter.text(
                    rect.left_top() + Vec2::new(22.0, 20.0),
                    egui::Align2::LEFT_TOP,
                    if self.camera.orthographic {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C08547D0298E8)
                    } else {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::CC7B2AF4A18D1)
                    },
                    egui::FontId::monospace(10.0),
                    MUTED,
                );
                painter.text(
                    rect.left_bottom() + Vec2::new(22.0, -22.0),
                    egui::Align2::LEFT_BOTTOM,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C02F933521F03),
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(82, 87, 97),
                );
                painter.text(
                    rect.right_top() + Vec2::new(-22.0, 20.0),
                    egui::Align2::RIGHT_TOP,
                    if self.transform_mode {
                        crate::ui_text::caption(crate::ui_text::CaptionKey::C8A58CA483875)
                    } else if self.region_masking_context(ctx) {
                        format!("{} MASK", self.mask_selection_mode.name().to_uppercase())
                    } else {
                        crate::ui_text::brush_name(self.brush.kind).to_uppercase()
                    },
                    egui::FontId::monospace(10.0),
                    ACCENT,
                );
                // Orientation triad follows the camera rather than implying a fixed front view.
                let origin = rect.right_bottom() + Vec2::new(-45.0, -45.0);
                let (right, up, _forward) = self.camera.axes();
                for (axis, label, color) in [
                    (Vec3::X, "X", Color32::from_rgb(219, 106, 100)),
                    (Vec3::Y, "Y", Color32::from_rgb(133, 186, 125)),
                    (Vec3::Z, "Z", Color32::from_rgb(114, 156, 218)),
                ] {
                    let end = origin + Vec2::new(axis.dot(right), -axis.dot(up)) * 23.0;
                    painter.line_segment([origin, end], Stroke::new(1.5, color));
                    painter.text(
                        end,
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::monospace(9.0),
                        color,
                    );
                }
                if let Some(stroke) = self.stroke.as_ref()
                    && let Some(locked) = stroke.locked_stamp.as_ref()
                    && let Some(pos) = stroke.last_screen
                {
                    let radius = (pos - locked.start).length();
                    painter.circle_stroke(locked.start, radius, Stroke::new(1.2, ACCENT));
                    painter.line_segment([locked.start, pos], Stroke::new(1.0, ACCENT));
                } else if !self.transform_mode
                    && !self.region_masking_context(ctx)
                    && self.paint_surface_cursor(ctx, painter, &response, rect)
                {
                    // Actual sealed Source contacts were drawn, or an explicit
                    // unavailable state was published; no legacy pick fallback.
                } else if !self.transform_mode
                    && !self.region_masking_context(ctx)
                    && let Some(pos) = response.hover_pos()
                    && let Some(hit) = self.pick(pos, rect)
                {
                    let depth = self.local_depth(hit.position).unwrap_or(0.0001);
                    let modifiers = ctx.input(|input| input.modifiers);
                    let preview = self.preferences.preview(
                        PreferenceModifiers {
                            shift: modifiers.shift,
                            control: modifiers.ctrl,
                            alt: modifiers.alt,
                        },
                        self.current_contact_input(),
                        None,
                        None,
                    );
                    let Ok(preview) = preview else {
                        return;
                    };
                    self.brush_radius_px = if let Some(p) = self
                        .source_geometry_preferences
                        .get(preview.settings.kind)
                        .filter(|p| {
                            p.mode
                                == crate::source_geometry_host::SourceGeometryMode::SourceOrdinary
                        }) {
                        self.source_paint_raw.sample().map_or(0., |raw| {
                            (p.radius * (1. + p.radius_factor * (raw.pressure * 2. - 1.))) as f32
                        })
                    } else if preview.settings.kind == BrushKind::Paint
                        && self.source_paint_preferences.mode
                            == crate::source_paint_host::PaintMode::SourceOrdinary
                    {
                        self.source_paint_raw.sample().map_or(0.0, |raw| {
                            (self.source_paint_preferences.radius
                                * (1.0
                                    + self.source_paint_preferences.radius_factor
                                        * (raw.pressure * 2.0 - 1.0)))
                                as f32
                        })
                    } else if preview.screen_radius.enabled {
                        preview.screen_radius.effective_pixels
                    } else {
                        preview.settings.effective_radius().unwrap_or(0.0)
                            * crate::scene_space::source_scale(&self.document).unwrap_or(1.0) as f32
                            / self.camera.world_units_per_pixel(
                                depth,
                                rect.height(),
                                rect.aspect_ratio(),
                            )
                    };
                    let color = if preview.settings.kind == BrushKind::Mask {
                        Color32::from_rgb(222, 210, 112)
                    } else {
                        ACCENT
                    };
                    painter.circle_stroke(pos, self.brush_radius_px, Stroke::new(1.2, color));
                    painter.circle_stroke(pos, 2.0, Stroke::new(1.0, color));
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
            });
    }

    fn help(&mut self, ctx: &egui::Context) {
        let source_profile = self.app_preferences.shortcut_profile
            == crate::app_preferences::PreferenceProfile::SculptGl;
        let mut preferences_clicked = false;
        let mut preferences_rect = None;
        self.app_preference_controls.remove("preferences.open");
        egui::Window::new(crate::ui_text::caption(
            crate::ui_text::CaptionKey::CB1768462B9D8,
        ))
        .id(egui::Id::new("Welcome to OBrush"))
        .open(&mut self.show_help)
        .collapsible(false)
        .resizable(false)
        .default_width(430.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C494F5F6A28A4,
                ))
                .size(19.0)
                .strong(),
            );
            ui.add_space(14.0);
            ui.label(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C18DCDB07BBF0,
            ));
            ui.add_space(14.0);
            if source_profile {
                ui.label(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C9687CB92031A,
                ));
                ui.label(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C045BEF52111D,
                ));
            } else {
                egui::Grid::new("help_keys")
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        for (key, action) in [
                            ("1–9 / 0", "Select brush / Morph"),
                            ("P / J / L / G", "Pinch / Twist / Local Scale / Drag"),
                            ("Shift + drag", "Smooth"),
                            ("Ctrl + drag", "Mask"),
                            ("Alt + drag", "Invert brush"),
                            ("Right drag", "Orbit camera"),
                            ("Middle drag", "Pan camera"),
                            ("Scroll", "Zoom"),
                            ("F / X / W", "Frame / symmetry / wireframe"),
                            ("T / Esc", "Transform mode / cancel preview"),
                            ("⌘/Ctrl Z", "Undo"),
                            ("⌘/Ctrl Shift Z", "Redo"),
                            ("⌘/Ctrl S / O", "Save / open"),
                            ("[ / ]", "Decrease / increase radius"),
                        ] {
                            ui.label(RichText::new(key).monospace().color(ACCENT));
                            ui.label(action);
                            ui.end_row();
                        }
                    });
            }
            let control = ui.button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C515ED995EE97,
            ));
            preferences_clicked = control.clicked();
            let rect = control.rect;
            preferences_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
            ui.add_space(16.0);
            ui.label(
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CE3B5ADDCE549,
                ))
                .color(preferences_host::muted(ui)),
            );
        });
        if let Some(rect) = preferences_rect {
            self.app_preference_controls
                .insert("preferences.open".into(), rect);
        }
        if preferences_clicked {
            self.show_app_preferences = true;
        }
    }
}

fn clear_mask_region(
    gesture: &mut Option<MaskRegionGesture>,
    suppressed_frame: &mut bool,
    suppress_frame: bool,
) -> bool {
    if suppress_frame {
        *suppressed_frame = true;
    }
    gesture.take().is_some()
}

fn region_press_requested(
    event: &egui::Event,
    viewport: egui::Rect,
    mode: MaskSelectionMode,
    brush: BrushKind,
) -> bool {
    mode != MaskSelectionMode::Brush
        && matches!(event, egui::Event::PointerButton {
            pos, button:egui::PointerButton::Primary, pressed:true, modifiers
        } if viewport.contains(*pos) && effective_brush_kind(brush,*modifiers)==BrushKind::Mask)
}

fn region_navigation_requested(input: &egui::InputState) -> bool {
    input.pointer.button_down(egui::PointerButton::Secondary)
        || input.pointer.button_down(egui::PointerButton::Middle)
        || input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::PointerButton {
                    button: egui::PointerButton::Secondary | egui::PointerButton::Middle,
                    pressed: true,
                    ..
                }
            )
        })
}

fn viewport_navigation_requested(input: &egui::InputState, browser_navigation_down: bool) -> bool {
    browser_navigation_down || region_navigation_requested(input)
}

#[cfg(test)]
mod attribute_host_tests {
    use super::*;

    #[test]
    fn registered_mesh_exports_keep_rich_project_and_redo_when_rejected_or_explicitly_lossy() {
        let mut document = Document::from_json(include_str!(
            "../../obrush-core/tests/fixtures/independent-material-uv-seam-v6.obrush"
        ))
        .unwrap();
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        for format in [
            ExportFormat::Obj,
            ExportFormat::StlAscii,
            ExportFormat::StlBinary,
            ExportFormat::PlyAscii,
            ExportFormat::PlyBinary,
        ] {
            assert!(
                document
                    .export_scene_geometry(
                        obrush_core::SglExportScope::AllScene,
                        format.scene_format(None),
                        AttributeExportPolicy::RejectLoss
                    )
                    .is_err()
            );
            assert_eq!(document.to_json().unwrap(), saved);
            assert!(document.can_redo());
            let export = document
                .export_scene_geometry(
                    obrush_core::SglExportScope::AllScene,
                    format.scene_format(None),
                    AttributeExportPolicy::AllowAndReport,
                )
                .unwrap();
            assert!(!export.bytes.is_empty());
            assert_eq!(export.report.attributes.omitted_material_vertices, 4);
            if format == ExportFormat::Obj {
                assert_eq!(export.report.attributes.omitted_uv_coordinates, 0);
                let text = String::from_utf8(export.bytes).unwrap();
                assert_eq!(text.lines().filter(|s| s.starts_with("vt ")).count(), 6);
                assert_eq!(text.lines().filter(|s| s.starts_with("f ")).count(), 2);
            } else {
                assert_eq!(export.report.attributes.omitted_uv_coordinates, 6);
                assert_eq!(export.report.attributes.omitted_original_faces, 2);
            }
            assert_eq!(document.to_json().unwrap(), saved);
            assert!(document.can_redo());
        }
        assert!(document.redo());
        assert_ne!(document.to_json().unwrap(), saved);
    }
}

#[cfg(test)]
mod primitive_readiness_tests {
    use super::*;

    #[test]
    fn source_default_cube_uses_reference_density_without_changing_manual_detail() {
        let source = PrimitiveKind::Cube.create_for_scene(6, true).unwrap();
        assert_eq!(source.positions.len(), 98_306);
        assert_eq!(source.surface.as_ref().unwrap().faces.len(), 98_304);
        assert_eq!(source.triangles.len(), 196_608);

        let ordinary = PrimitiveKind::Cube.create_for_scene(6, false).unwrap();
        assert_eq!(ordinary.positions.len(), 24_578);
        assert!(ordinary.surface.is_none());
        let manual = PrimitiveKind::Cube.create_for_scene(5, true).unwrap();
        assert_eq!(manual.triangles.len(), 12 * 4_usize.pow(5));
    }

    #[test]
    fn source_default_sphere_uses_quad_surface_without_changing_manual_detail() {
        let source = PrimitiveKind::Sphere.create_for_scene(6, true).unwrap();
        assert_eq!(source.positions.len(), 98_306);
        assert_eq!(source.surface.as_ref().unwrap().faces.len(), 98_304);
        assert_eq!(source.triangles.len(), 196_608);

        let ordinary = PrimitiveKind::Sphere.create_for_scene(6, false).unwrap();
        assert_eq!(ordinary.positions.len(), 40_962);
        assert!(ordinary.surface.is_none());
        let manual = PrimitiveKind::Sphere.create_for_scene(5, true).unwrap();
        assert_eq!(manual.triangles.len(), 20 * 4_usize.pow(5));
        assert!(manual.surface.is_none());
    }

    #[test]
    fn source_default_cylinder_and_torus_keep_original_faces() {
        for (kind, expected) in [
            (PrimitiveKind::Cylinder, (66_562, 67_584, 133_120)),
            (PrimitiveKind::Torus, (65_536, 65_536, 131_072)),
        ] {
            let source = kind.create_for_scene(6, true).unwrap();
            assert_eq!(
                (
                    source.positions.len(),
                    source.face_count(),
                    source.triangles.len()
                ),
                expected
            );
            assert!(source.surface.is_some());

            let ordinary = kind.create_for_scene(6, false).unwrap();
            assert!(ordinary.surface.is_none());
            let manual = kind.create_for_scene(5, true).unwrap();
            assert!(manual.surface.is_none());
        }
    }

    #[test]
    fn face_center_draw_on_new_cube_edits_immediately_and_undo_restores_every_channel() {
        let settings = BrushSettings {
            radius: 0.18,
            strength: 0.35,
            symmetry_x: false,
            ..Default::default()
        };
        let mut coarse = Mesh::cube();
        let ray = Ray {
            origin: Vec3::new(0.02, 0.03, 4.0),
            direction: -Vec3::Z,
        };
        let hit = coarse.raycast(ray).unwrap();
        assert_eq!(coarse.positions.len(), 1538);
        assert_eq!(coarse.triangles.len(), 3072);
        let coarse_changed = coarse.apply_brush(hit.position, hit.normal, Vec3::ZERO, &settings);
        assert!(coarse_changed > 0);

        let mesh = PrimitiveKind::Cube.create(6).unwrap();
        let mut document = Document::new(mesh);
        let before = document.to_json().unwrap();
        let hit = document
            .require_active_mesh()
            .unwrap()
            .raycast(ray)
            .unwrap();
        document.begin_stroke();
        let result = document
            .require_active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
        document.end_stroke();
        assert!(result.changed_vertices > 20);
        assert!(result.changed_vertices > coarse_changed * 4);
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .positions
                .iter()
                .any(|p| p.z > 1.001)
        );
        document.require_active_mesh().unwrap().validate().unwrap();
        let edited = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(!document.can_undo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), edited);
    }

    #[test]
    fn each_added_primitive_has_bounded_editable_density_and_unchanged_base_dimensions() {
        for kind in [
            PrimitiveKind::Sphere,
            PrimitiveKind::Cube,
            PrimitiveKind::Cylinder,
            PrimitiveKind::Torus,
        ] {
            let mesh = kind.create(6).unwrap();
            mesh.validate().unwrap();
            assert!(mesh.triangles.len() >= 49_152, "{}", kind.name());
            assert!(mesh.positions.len() <= obrush_core::MAX_VERTICES);
            assert!(mesh.triangles.len() <= obrush_core::MAX_TRIANGLES);
            let extent = mesh
                .positions
                .iter()
                .fold(Vec3::ZERO, |size, p| size.max(p.abs()));
            let expected = match kind {
                PrimitiveKind::Sphere | PrimitiveKind::Cube => Vec3::ONE,
                PrimitiveKind::Cylinder => Vec3::new(0.5, 1.0, 0.5),
                PrimitiveKind::Torus => Vec3::new(0.6, 0.1, 0.6),
            };
            assert!(
                extent.distance(expected) < 2e-6,
                "{}: {extent:?}",
                kind.name()
            );
        }
    }
}

fn effective_brush_kind(kind: BrushKind, modifiers: egui::Modifiers) -> BrushKind {
    if modifiers.ctrl {
        BrushKind::Mask
    } else if modifiers.shift {
        BrushKind::Smooth
    } else {
        kind
    }
}

fn file_history_commands(ctx: &egui::Context) -> (bool, bool, bool, bool) {
    ctx.input_mut(|input| {
        let redo = input.consume_key(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::Z,
        ) || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
        let undo = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
        let save = input.consume_key(egui::Modifiers::COMMAND, egui::Key::S);
        let open = input.consume_key(egui::Modifiers::COMMAND, egui::Key::O);
        (redo, undo, save, open)
    })
}

impl eframe::App for OBrushApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        crate::ui_fonts::bound_raw_input(input);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let update_started = web_time::Instant::now();
        self.ui_fonts.update(ctx, self.app_preferences.locale);
        crate::ui_text::install_admitted(self.ui_fonts.capability(), &self.shortcut_resolver);
        self.region_suppressed_frame = false;
        self.poll_startup_model(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        if self.source_shortcut_profile() {
            let dropped = ctx.input(|input| input.raw.dropped_files.clone());
            if !dropped.is_empty() {
                self.add_dropped_source_files(dropped);
            }
        }
        if !self.browser_navigation_coordinates {
            // Native raw events keep the final release endpoint even when the
            // entire gesture arrives before one redraw. Browser DOM capture is
            // authoritative when present, so these events are not duplicated.
            let (events, focused) = ctx.input(|input| (input.events.clone(), input.focused));
            if !focused {
                self.navigation_capture.cancel();
                self.native_navigation_buttons = 0;
            } else {
                for event in events {
                    match event {
                        egui::Event::PointerButton {
                            pos,
                            button,
                            pressed,
                            modifiers,
                        } => {
                            let bit = match button {
                                egui::PointerButton::Secondary => 2,
                                egui::PointerButton::Middle => 4,
                                _ => 0,
                            };
                            if pressed {
                                self.native_navigation_buttons |= bit;
                            } else {
                                self.native_navigation_buttons &= !bit;
                            }
                            self.navigation_capture.pointer(
                                self.native_navigation_buttons,
                                pos.into(),
                                modifiers.shift,
                            );
                        }
                        egui::Event::PointerMoved(pos) => {
                            let shift = ctx.input(|input| input.modifiers.shift);
                            self.navigation_capture.pointer(
                                self.native_navigation_buttons,
                                pos.into(),
                                shift,
                            );
                        }
                        egui::Event::PointerGone => {
                            self.navigation_capture.cancel();
                            self.native_navigation_buttons = 0;
                        }
                        _ => {}
                    }
                }
            }
        }
        let contact_interrupted = self.browser_navigation.take_cancel_request();
        if contact_interrupted {
            self.cancel_absolute_locked_stamp();
        }
        if contact_interrupted
            && (self.mask_region_gesture.is_some()
                || self.mask_selection_mode != MaskSelectionMode::Brush)
        {
            self.cancel_mask_region(
                "Pointer input interrupted; pending selection cancelled.",
                true,
            );
        }
        let region_settings_before = self
            .mask_region_gesture
            .as_ref()
            .map(|_| self.region_settings_fingerprint());
        self.mask_region_controls.clear();
        self.mask_region_controls_enabled.clear();
        self.morph_controls.clear();
        self.morph_controls_enabled.clear();
        self.source_paint_controls_hit_rect.clear();
        self.screen_radius_hit_rect.clear();
        self.screen_radius_editor.begin_frame();
        self.source_geometry_controls_hit_rect.clear();
        self.scene_controls.clear();
        self.scene_controls_enabled.clear();
        self.transform_controls.clear();
        self.transform_controls_enabled.clear();
        self.extract_controls.clear();
        self.extract_controls_enabled.clear();
        self.mask_filter_controls.clear();
        self.mask_filter_controls_enabled.clear();
        if !ctx.input(|input| input.focused) {
            self.cancel_absolute_locked_stamp();
            self.cancel_mask_region("Focus lost; pending selection cancelled.", true);
            self.cancel_transform_drag();
        }
        self.unprocessed_transform_release = if self.transform_drag.is_some() {
            ctx.input(|input| {
                input.events.iter().rev().find_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } => Some(*pos),
                    _ => None,
                })
            })
        } else {
            None
        };
        self.poll_topology(ctx);
        self.poll_uv_bake(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let forces = ctx.input(|input| {
                input
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::Touch { phase, force, .. } => Some((*phase, *force)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });
            for (phase, force) in forces {
                match phase {
                    egui::TouchPhase::Start | egui::TouchPhase::Move => {
                        self.set_input_pressure(force.unwrap_or(1.0), true, false);
                        self.source_paint_raw.receive(
                            f64::from(force.unwrap_or(0.5)),
                            true,
                            false,
                            false,
                            None,
                        );
                        self.source_paint_raw.current.absent = force.is_none();
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                        self.set_input_pressure(0.0, true, true);
                        if phase == egui::TouchPhase::Cancel
                            && self
                                .stroke
                                .as_ref()
                                .is_some_and(|s| s.continuous_locked.is_some())
                        {
                            self.unprocessed_release = None;
                            self.cancel_continuous_locked();
                        }
                        if phase == egui::TouchPhase::Cancel
                            && self.stroke.as_ref().is_some_and(|s| {
                                s.source_paint.is_some() || s.source_geometry.is_some()
                            })
                        {
                            self.interrupt_source_paint_tail(
                                "Native Source Paint contact was cancelled",
                            );
                            self.unprocessed_release = None;
                            self.finish_stroke_deferred();
                        }
                    }
                }
            }
        }
        self.unprocessed_release = if self.stroke.is_some() {
            ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                    } => Some((*pos, *modifiers, self.contact_pressure.endpoint())),
                    _ => None,
                })
            })
        } else {
            None
        };
        self.modal_begin_frame(ctx);
        self.shortcuts(ctx);
        self.source_keyboard_camera_frame(ctx);
        self.trace_keyboard_route(ctx, "after_shortcuts");
        self.header(ctx);
        self.footer(ctx);
        self.tools(ctx);
        self.inspector(ctx);
        self.screen_radius_editor.finish_frame(
            ctx,
            self.brush.kind,
            self.brush_screen_radius,
            self.brush_screen_space,
        );
        self.commit_brush_view();
        if region_settings_before.is_some_and(|before| before != self.region_settings_fingerprint())
        {
            self.cancel_mask_region("A setting changed; pending mask selection cancelled.", true);
        }
        self.app_panel_timing.record(update_started.elapsed());
        let viewport_started = web_time::Instant::now();
        self.viewport(ctx);
        self.app_viewport_timing.record(viewport_started.elapsed());
        self.help(ctx);
        self.preferences_window(ctx);
        self.ui_fonts.observe_end_update(ctx);
        self.modal_observe_end_update();
        self.app_update_timing.record(update_started.elapsed());
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.cancel_startup_model();
        self.cancel_uv_bake(false);
        self.cancel_topology();
        self.finish_stroke();
        if let Some(gl) = gl
            && let Ok(mut renderer) = self.renderer.lock()
        {
            renderer.destroy(gl);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace=window,js_name=obrushReadStartupModel)]
    fn start_model_read(id: u32, url: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace=window,js_name=obrushCancelStartupModel)]
    fn cancel_model_read(id: u32);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushPickFile)]
    fn pick_file();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace=window,js_name=obrushPickSourceSgl)]
    fn pick_source_sgl_file(request_id: u32);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace=window,js_name=obrushSourceSglNativeLittle)]
    fn source_sgl_native_little() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushDownloadBytes)]
    fn download_bytes(name: &str, contents: &[u8]);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushDownload)]
    fn download_text(name: &str, contents: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushPickAlpha)]
    fn pick_alpha(request_id: u32);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushPickUVTexture)]
    fn pick_uv_texture(id: u32);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace=window,js_name=obrushPickBackground)]
    fn pick_background_image(id: u32);
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_namespace = window, js_name = obrushStartTopology)]
    fn start_topology_worker(
        id: u32,
        project: &str,
        operation: &str,
    ) -> Result<(), wasm_bindgen::JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushCancelTopology)]
    fn cancel_topology_worker(id: u32);
}

fn configure_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.override_text_color = Some(Color32::from_rgb(218, 220, 225));
    style.visuals.selection.bg_fill = Color32::from_rgb(93, 55, 40);
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 43, 49);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(40, 43, 49);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(55, 59, 66));
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(65, 52, 48);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(65, 52, 48);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(96, 59, 44);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.slider_width = 178.0;
    // Scrollbars reserve a gutter so they cannot intercept the end of a
    // slider or a row button when a panel grows taller than the viewport.
    style.spacing.scroll.floating = false;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
    ctx.set_style(style);
}

fn section(ui: &mut egui::Ui, index: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(index).monospace().size(10.0).color(ACCENT));
        ui.add_space(4.0);
        ui.label(
            RichText::new(title)
                .size(10.0)
                .strong()
                .color(preferences_host::muted(ui)),
        );
    });
}

fn label_value(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .size(11.0)
                .color(preferences_host::muted(ui)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.0).monospace());
        });
    });
}

fn estimated_edge_length(mesh: &Mesh) -> f32 {
    let step = (mesh.triangles.len() / 512).max(1);
    let mut sum = 0.0;
    let mut count = 0;
    for triangle in mesh.triangles.iter().step_by(step) {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            if let (Some(a), Some(b)) = (
                mesh.positions.get(a as usize),
                mesh.positions.get(b as usize),
            ) {
                let length = a.distance(*b);
                if length.is_finite() {
                    sum += length;
                    count += 1;
                }
            }
        }
    }
    if count > 0 { sum / count as f32 } else { 0.0 }
}

fn number(value: usize) -> String {
    let s = value.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn brush_label_galley(ui: &egui::Ui, label: &str) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        egui::FontId::proportional(10.0),
        Color32::PLACEHOLDER,
        49.0,
    );
    job.halign = egui::Align::Center;
    ui.painter().layout_job(job)
}

fn brush_tile(
    ui: &mut egui::Ui,
    label: &str,
    key: &str,
    selected: bool,
    kind: usize,
    height: f32,
) -> egui::Response {
    let galley = brush_label_galley(ui, label);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(57.0, height), egui::Sense::click());
    let painter = ui.painter();
    let fill = if selected {
        Color32::from_rgb(69, 45, 36)
    } else if response.hovered() {
        Color32::from_rgb(48, 48, 52)
    } else {
        Color32::from_rgb(35, 38, 43)
    };
    painter.rect(
        rect,
        5.0,
        fill,
        Stroke::new(
            1.0,
            if selected {
                ACCENT
            } else {
                Color32::from_rgb(51, 55, 62)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let center = rect.center_top() + Vec2::new(0.0, 24.0);
    let ink = if selected {
        ACCENT
    } else {
        Color32::from_rgb(173, 179, 188)
    };
    painter.circle_stroke(center, 11.0, Stroke::new(1.0, ink));
    match kind {
        0..=2 => {
            painter.line_segment(
                [center - Vec2::new(6.0, 0.0), center + Vec2::new(6.0, 0.0)],
                Stroke::new(1.3, ink),
            );
            painter.line_segment(
                [center - Vec2::new(0.0, 6.0), center + Vec2::new(0.0, 6.0)],
                Stroke::new(1.3, ink),
            );
            if kind == 1 {
                painter.circle_stroke(center, 7.0, Stroke::new(0.8, ink));
            }
            if kind == 2 {
                painter.circle_stroke(center, 14.0, Stroke::new(0.7, ink));
            }
        }
        3 => {
            for y in [-4.0, 0.0, 4.0] {
                painter.line_segment(
                    [center + Vec2::new(-6.0, y), center + Vec2::new(6.0, y)],
                    Stroke::new(1.0, ink),
                );
            }
        }
        4 => {
            painter.line_segment(
                [center + Vec2::new(-7.0, 2.0), center + Vec2::new(7.0, 2.0)],
                Stroke::new(2.0, ink),
            );
        }
        5 => {
            painter.line_segment(
                [center + Vec2::new(-6.0, -4.0), center + Vec2::new(0.0, 5.0)],
                Stroke::new(1.0, ink),
            );
            painter.line_segment(
                [center + Vec2::new(0.0, 5.0), center + Vec2::new(6.0, -4.0)],
                Stroke::new(1.0, ink),
            );
        }
        6 => {
            painter.line_segment(
                [center - Vec2::new(7.0, 0.0), center + Vec2::new(7.0, 0.0)],
                Stroke::new(1.0, ink),
            );
            painter.line_segment(
                [center + Vec2::new(3.0, -4.0), center + Vec2::new(7.0, 0.0)],
                Stroke::new(1.0, ink),
            );
            painter.line_segment(
                [center + Vec2::new(3.0, 4.0), center + Vec2::new(7.0, 0.0)],
                Stroke::new(1.0, ink),
            );
        }
        7 => {
            painter.circle_filled(center, 7.0, ink.gamma_multiply(0.45));
        }
        9 => {
            for (from, to) in [
                ((-6.0, -2.0), (-1.0, -6.0)),
                ((-1.0, -6.0), (5.0, -3.0)),
                ((5.0, -3.0), (5.0, 4.0)),
                ((5.0, 4.0), (-3.0, 4.0)),
                ((-3.0, 4.0), (0.0, 1.0)),
                ((-3.0, 4.0), (0.0, 7.0)),
            ] {
                painter.line_segment(
                    [
                        center + Vec2::new(from.0, from.1),
                        center + Vec2::new(to.0, to.1),
                    ],
                    Stroke::new(1.2, ink),
                );
            }
        }
        _ => {
            painter.circle_filled(center, 6.0, ACCENT);
        }
    }
    let label_position = Pos2::new(rect.center().x, rect.bottom() - 14.0 - galley.size().y);
    painter.galley_with_override_text_color(label_position, galley, ink);
    painter.text(
        rect.right_top() + Vec2::new(-4.0, 4.0),
        egui::Align2::RIGHT_TOP,
        key,
        egui::FontId::monospace(8.0),
        MUTED,
    );
    response.on_hover_text(format!("{label} brush · {key}"))
}

#[cfg(test)]
mod locked_stamp_host_tests {
    use super::*;

    fn capture(mesh: &Mesh) -> ActiveLockedStamp {
        let settings = BrushSettings {
            radius: 0.18,
            strength: 0.35,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        };
        ActiveLockedStamp {
            stamp: LockedStamp::prepare(mesh, Vec3::Z, Vec3::Z, &settings).unwrap(),
            start: Pos2::new(100.0, 200.0),
            viewport: egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0)),
            units_per_pixel: 0.002,
            tangent: Vec3::X,
            base_rotation: 0.2,
            dirty_before: false,
            vertex_color_before: false,
            paints_rgb: settings.paint_rgb,
        }
    }

    #[test]
    fn absolute_pointer_travel_drives_radius_rotation_and_same_endpoint_shape() {
        let original = Mesh::sphere(4);
        let mut coarse = original.snapshot();
        let mut fine = original.snapshot();
        let mut a = capture(&coarse);
        let mut b = capture(&fine);
        let end = Pos2::new(130.0, 240.0);
        let report = sample_locked_stamp(&mut a, &mut coarse, end, 0.5).unwrap();
        assert!(report.changed());
        assert!((f64::from(report.radius) - 0.1).abs() < 1e-8);
        assert!((f64::from(report.alpha_rotation) - (0.2_f64 - 40.0_f64.atan2(30.0))).abs() < 1e-7);
        assert_eq!(report.alpha_tangent, Some(Vec3::X));
        assert_eq!(report.pressure, 0.5);
        for step in 1..20 {
            sample_locked_stamp(
                &mut b,
                &mut fine,
                Pos2::new(100.0 + step as f32 * 1.5, 200.0 + step as f32 * 2.0),
                0.5,
            )
            .unwrap();
        }
        sample_locked_stamp(&mut b, &mut fine, end, 0.5).unwrap();
        assert_eq!(coarse.positions, fine.positions);
        assert_eq!(coarse.normals, fine.normals);
        assert_eq!(coarse.triangles, fine.triangles);
        assert!(
            !sample_locked_stamp(&mut b, &mut fine, end, 0.5)
                .unwrap()
                .changed()
        );
    }

    #[test]
    fn returning_to_exact_press_restores_original_and_preserves_genuine_redo() {
        let mut document = Document::new(Mesh::sphere(4));
        document.begin_stroke();
        document.require_active_mesh_mut().unwrap().masks[0] = 0.5;
        document.end_stroke();
        assert!(document.undo());
        let original = document.to_json().unwrap();
        let mut locked = capture(document.require_active_mesh().unwrap());
        document.begin_stroke();
        assert!(
            sample_locked_stamp(
                &mut locked,
                document.require_active_mesh_mut().unwrap(),
                Pos2::new(150.0, 220.0),
                1.0
            )
            .unwrap()
            .changed()
        );
        let start = locked.start;
        let restored = sample_locked_stamp(
            &mut locked,
            document.require_active_mesh_mut().unwrap(),
            start,
            0.0,
        )
        .unwrap();
        assert!(restored.changed() && restored.at_origin);
        assert_eq!(restored.radius, 0.0);
        document.end_stroke();
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.can_redo());
        assert!(!locked.dirty_before);
    }
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn released_modifier_chord_still_selects_redo_only() {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            modifiers: egui::Modifiers::NONE,
            events: vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: Some(egui::Key::Z),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::MAC_CMD
                    | egui::Modifiers::COMMAND
                    | egui::Modifiers::SHIFT,
            }],
            ..Default::default()
        };
        let mut commands = (false, false, false, false);
        let _ = ctx.run(raw, |ctx| {
            commands = file_history_commands(ctx);
        });
        assert_eq!(commands, (true, false, false, false));
    }
}

#[cfg(test)]
mod transform_input_tests {
    use super::*;

    #[test]
    fn actual_preview_clear_keeps_saved_geometry_and_redo_untouched() {
        let mut document = Document::new(Mesh::sphere(1));
        let id = document
            .active_subtool_id()
            .expect("nonempty fixture active id");
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.25, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let mut gesture = TranslationDrag::new(
            &camera,
            rect,
            rect.center(),
            Vec3::ZERO,
            TranslationHandle::Free,
        )
        .unwrap();
        assert!(gesture.update(rect.center() + egui::vec2(80.0, -30.0)));
        let mut preview = gesture.translation;
        let mut preview_scale = Vec3::ONE;
        let mut preview_rotation = Vec3::new(0.0, 0.0, 12.0);
        let mut preview_pivot = Vec3::new(1.0, 2.0, 3.0);
        let mut drag = Some(ActiveTransformDrag {
            gesture,
            scale_drag: None,
            rotation_drag: None,
            kind: GizmoKind::Translate,
            pivot: Vec3::ZERO,
            subtool_id: id,
            selected_ids: vec![id],
            mask_weighted: false,
            revision: 4,
        });
        let mut release = Some(rect.center() + egui::vec2(80.0, -30.0));
        assert!(clear_transform_preview(
            &mut drag,
            &mut release,
            &mut preview,
            &mut preview_scale,
            &mut preview_rotation,
            &mut preview_pivot
        ));
        assert!(drag.is_none() && release.is_none());
        assert_eq!(preview, Vec3::ZERO);
        assert_eq!(preview_scale, Vec3::ONE);
        assert_eq!(preview_rotation, Vec3::ZERO);
        assert_eq!(preview_pivot, Vec3::ZERO);
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
        assert!(!clear_transform_preview(
            &mut drag,
            &mut release,
            &mut preview,
            &mut preview_scale,
            &mut preview_rotation,
            &mut preview_pivot
        ));
    }

    #[test]
    fn exact_return_gesture_commits_no_transform_and_keeps_redo() {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let mut gesture = TranslationDrag::new(
            &camera,
            rect,
            rect.center(),
            Vec3::ZERO,
            TranslationHandle::X,
        )
        .unwrap();
        gesture.update(rect.center() + egui::vec2(70.0, 10.0));
        gesture.update(rect.center());
        assert_eq!(gesture.translation, Vec3::ZERO);
        assert!(
            !document
                .transform_active_subtool(&ObjectTransform {
                    translation: gesture.translation,
                    ..Default::default()
                })
                .unwrap()
        );
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }

    #[test]
    fn scale_handle_uses_absolute_axis_factor_and_one_undoable_core_edit() {
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let pivot = Vec3::ZERO;
        let length = 0.4;
        let start = project_world(&camera, rect, pivot + Vec3::X * length).unwrap();
        let end = project_world(&camera, rect, pivot + Vec3::X * length * 1.5).unwrap();
        let screen_factor = end.distance(project_world(&camera, rect, pivot).unwrap())
            / start.distance(project_world(&camera, rect, pivot).unwrap());
        let gesture =
            TranslationDrag::new(&camera, rect, start, pivot, TranslationHandle::X).unwrap();
        let mut drag = ActiveTransformDrag {
            gesture,
            scale_drag: ScaleDrag::new(
                project_world(&camera, rect, pivot).unwrap(),
                start,
                Some(start),
            ),
            rotation_drag: None,
            kind: GizmoKind::Scale,
            pivot,
            subtool_id: 1,
            selected_ids: vec![1],
            mask_weighted: false,
            revision: 1,
        };
        let mut chunked = drag.clone();
        assert!(drag.scale_drag.as_mut().unwrap().update(end));
        for step in 1..=20 {
            assert!(
                chunked
                    .scale_drag
                    .as_mut()
                    .unwrap()
                    .update(start.lerp(end, step as f32 / 20.0))
            );
        }
        assert_eq!(drag.axis_scale(), chunked.axis_scale());
        assert!(
            drag.axis_scale()
                .abs_diff_eq(Vec3::new(screen_factor, 1.0, 1.0), 2.0e-6),
            "observed scale: {:?}",
            drag.axis_scale()
        );
        let mut document = Document::new(Mesh::sphere(1));
        let before = document.to_json().unwrap();
        assert!(
            document
                .scale_selected_subtools_axis(drag.axis_scale(), pivot, false)
                .unwrap()
        );
        let after = document.to_json().unwrap();
        assert_ne!(before, after);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
        drag.scale_drag.as_mut().unwrap().update(start);
        assert_eq!(drag.axis_scale(), Vec3::ONE);
        let uniform_start = project_world(&camera, rect, pivot + Vec3::X * length).unwrap();
        let mut uniform = ActiveTransformDrag {
            gesture: TranslationDrag::new(
                &camera,
                rect,
                uniform_start,
                pivot,
                TranslationHandle::Free,
            )
            .unwrap(),
            scale_drag: ScaleDrag::new(
                project_world(&camera, rect, pivot).unwrap(),
                uniform_start,
                None,
            ),
            rotation_drag: None,
            kind: GizmoKind::UniformScale,
            pivot,
            subtool_id: 1,
            selected_ids: vec![1],
            mask_weighted: false,
            revision: 1,
        };
        uniform.scale_drag.as_mut().unwrap().update(end);
        assert!(
            uniform
                .axis_scale()
                .abs_diff_eq(Vec3::splat(screen_factor), 2.0e-6)
        );
    }
}

#[cfg(test)]
mod mask_extract_host_tests {
    use super::*;

    #[test]
    fn mask_extract_gate_blocks_hidden_parts_jobs_and_live_transforms() {
        let mut document = Document::new(Mesh::sphere(1));
        assert!(mask_extract_unavailable_reason(&document, false, false).is_none());
        assert!(
            mask_extract_unavailable_reason(&document, true, false)
                .unwrap()
                .contains("surface operation")
        );
        assert!(
            mask_extract_unavailable_reason(&document, false, true)
                .unwrap()
                .contains("transform drag")
        );
        let id = document
            .active_subtool_id()
            .expect("nonempty fixture active id");
        document.set_subtool_visible(id, false).unwrap();
        assert!(
            mask_extract_unavailable_reason(&document, false, false)
                .unwrap()
                .contains("Show")
        );
        document.set_solo(true).unwrap();
        assert!(mask_extract_unavailable_reason(&document, false, false).is_some());
        document.set_subtool_visible(id, true).unwrap();
        assert!(mask_extract_unavailable_reason(&document, false, false).is_none());
    }

    #[test]
    fn rejected_source_stroke_barrier_preserves_complete_scene_and_redo() {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        let result = mask_extract_after_barrier_with_selection(
            &mut document,
            &MaskExtractSettings::default(),
            MaskExtractSelection::Clipped,
            false,
            false,
            true,
        );
        assert!(result.unwrap_err().contains("preceding shape edit"));
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }

    #[test]
    fn extraction_without_mask_creates_no_part_or_history_and_preserves_redo() {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        assert!(
            mask_extract_after_barrier_with_selection(
                &mut document,
                &MaskExtractSettings::default(),
                MaskExtractSelection::Clipped,
                false,
                false,
                false
            )
            .is_err()
        );
        assert_eq!(document.subtool_count(), 1);
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }
}

#[cfg(test)]
mod mask_filter_host_tests {
    use super::*;

    #[test]
    fn filter_host_gate_blocks_hidden_parts_jobs_and_live_transforms() {
        let mut document = Document::new(Mesh::sphere(1));
        assert!(mask_filter_unavailable_reason(&document, false, false).is_none());
        assert!(
            mask_filter_unavailable_reason(&document, true, false)
                .unwrap()
                .contains("surface operation")
        );
        assert!(
            mask_filter_unavailable_reason(&document, false, true)
                .unwrap()
                .contains("transform drag")
        );
        let id = document
            .active_subtool_id()
            .expect("nonempty fixture active id");
        document.set_subtool_visible(id, false).unwrap();
        assert!(
            mask_filter_unavailable_reason(&document, false, false)
                .unwrap()
                .contains("Show")
        );
        document.set_subtool_visible(id, true).unwrap();
        assert!(mask_filter_unavailable_reason(&document, false, false).is_none());
    }

    fn document_with_redo() -> Document {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        document
    }

    #[test]
    fn filter_rejected_stroke_barrier_preserves_project_and_redo() {
        let mut document = document_with_redo();
        let saved = document.to_json().unwrap();
        let result =
            mask_filter_after_barrier(&mut document, &MaskFilterSettings::default(), true, false);
        assert!(result.unwrap_err().contains("preceding shape edit"));
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }

    #[test]
    fn rounded_noop_report_and_invalid_settings_preserve_redo() {
        let mut document = document_with_redo();
        let saved = document.to_json().unwrap();
        let report =
            mask_filter_after_barrier(&mut document, &MaskFilterSettings::default(), false, false)
                .unwrap();
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(report.max_abs_change, 0.0);
        assert_eq!(document.to_json().unwrap(), saved);
        let bad = MaskFilterSettings {
            iterations: 0,
            ..Default::default()
        };
        assert!(mask_filter_after_barrier(&mut document, &bad, false, false).is_err());
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }
}

#[cfg(test)]
mod mask_region_host_tests {
    use super::*;

    fn same_frame_navigation_cancels(button: egui::PointerButton) {
        let ctx = egui::Context::default();
        let viewport = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut gesture = Some(
            MaskRegionGesture::new(
                MaskSelectionMode::Rectangle,
                viewport,
                glam::Mat4::IDENTITY.to_cols_array(),
                3,
                19,
                Pos2::new(100.0, 100.0),
                false,
            )
            .unwrap(),
        );
        gesture
            .as_mut()
            .unwrap()
            .move_to(Pos2::new(650.0, 500.0))
            .unwrap();
        let raw = egui::RawInput {
            events: vec![
                egui::Event::PointerButton {
                    pos: Pos2::new(350.0, 250.0),
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: Pos2::new(350.0, 250.0),
                    button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut suppressed = false;
        let _ = ctx.run(raw, |ctx| {
            ctx.input(|input| {
                assert!(!input.pointer.button_down(button));
                if region_navigation_requested(input) {
                    clear_mask_region(&mut gesture, &mut suppressed, true);
                }
            });
        });
        assert!(
            gesture.is_none(),
            "same-frame {button:?} navigation must cancel the preview"
        );
        assert!(suppressed, "the same-frame release must stay suppressed");
    }

    #[test]
    fn same_frame_secondary_press_release_cancels_pending_region() {
        same_frame_navigation_cancels(egui::PointerButton::Secondary);
    }

    #[test]
    fn same_frame_middle_press_release_cancels_pending_region() {
        same_frame_navigation_cancels(egui::PointerButton::Middle);
    }

    #[test]
    fn dom_chord_latch_cancels_before_release_and_next_fresh_capture_keeps_redo() {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        let viewport = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let create_capture = || {
            MaskRegionGesture::new(
                MaskSelectionMode::Rectangle,
                viewport,
                glam::Mat4::IDENTITY.to_cols_array(),
                document
                    .active_subtool_id()
                    .expect("nonempty fixture active id"),
                17,
                Pos2::new(100.0, 100.0),
                false,
            )
            .unwrap()
        };
        let mut gesture = Some(create_capture());
        gesture
            .as_mut()
            .unwrap()
            .move_to(Pos2::new(650.0, 500.0))
            .unwrap();
        let mut input = BrowserNavigationButtons::default();
        for buttons in [1, 3, 1, 0] {
            input.update(buttons);
        }
        assert!(!input.navigation_down());
        let mut suppressed = false;
        if input.take_cancel_request() {
            clear_mask_region(&mut gesture, &mut suppressed, true);
        }
        assert!(suppressed && gesture.is_none());
        assert_eq!(document.to_json().unwrap(), saved);
        // The next frame has no sticky latch/down state, and a genuine fresh
        // primary press can capture a new preview without altering the project.
        input.update(1);
        assert!(!input.take_cancel_request() && !input.navigation_down());
        let fresh_press = egui::Event::PointerButton {
            pos: Pos2::new(100.0, 100.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(region_press_requested(
            &fresh_press,
            viewport,
            MaskSelectionMode::Rectangle,
            BrushKind::Mask
        ));
        gesture = Some(create_capture());
        assert!(gesture.is_some());
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
    }

    #[test]
    fn held_dom_navigation_blocks_sculpt_after_ctrl_is_released_until_button_release() {
        let ctx = egui::Context::default();
        let mut browser = BrowserNavigationButtons::default();
        browser.update(3);
        assert!(browser.take_cancel_request());
        let raw = egui::RawInput {
            modifiers: egui::Modifiers::NONE,
            events: vec![egui::Event::PointerMoved(Pos2::new(350.0, 250.0))],
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            ctx.input(|input| {
                assert!(!region_navigation_requested(input));
                assert_eq!(
                    effective_brush_kind(BrushKind::Draw, input.modifiers),
                    BrushKind::Draw
                );
                assert!(viewport_navigation_requested(
                    input,
                    browser.navigation_down()
                ));
                browser.update(1);
                assert!(!viewport_navigation_requested(
                    input,
                    browser.navigation_down()
                ));
                assert!(!browser.take_cancel_request());
            });
        });
    }

    #[test]
    fn actual_ctrl_press_routes_to_regions_after_modifiers_and_pointer_are_released() {
        let ctx = egui::Context::default();
        let viewport = egui::Rect::from_min_max(Pos2::new(200.0, 50.0), Pos2::new(800.0, 650.0));
        let raw = egui::RawInput {
            modifiers: egui::Modifiers::NONE,
            events: vec![
                egui::Event::PointerButton {
                    pos: Pos2::new(210.0, 60.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                },
                egui::Event::PointerMoved(Pos2::new(850.0, 700.0)),
                egui::Event::PointerButton {
                    pos: Pos2::new(850.0, 700.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut requested = false;
        let _ = ctx.run(raw, |ctx| {
            ctx.input(|input| {
                assert!(!input.pointer.primary_down());
                assert_eq!(input.modifiers, egui::Modifiers::NONE);
                assert!(!viewport.contains(input.pointer.interact_pos().unwrap()));
                requested = input.events.iter().any(|event| {
                    region_press_requested(
                        event,
                        viewport,
                        MaskSelectionMode::Rectangle,
                        BrushKind::Draw,
                    )
                });
                assert!(!input.events.iter().any(|event| {
                    region_press_requested(
                        event,
                        viewport,
                        MaskSelectionMode::Brush,
                        BrushKind::Draw,
                    )
                }));
            });
        });
        assert!(requested);
    }

    #[test]
    fn command_cancellation_blocks_same_frame_release_and_preserves_complete_redo() {
        let mut document = Document::new(Mesh::sphere(1));
        document
            .transform_active_subtool(&ObjectTransform {
                translation: Vec3::new(0.125, 0.0, 0.0),
                ..Default::default()
            })
            .unwrap();
        assert!(document.undo());
        let saved = document.to_json().unwrap();
        let viewport = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut gesture = Some(
            MaskRegionGesture::new(
                MaskSelectionMode::Rectangle,
                viewport,
                glam::Mat4::IDENTITY.to_cols_array(),
                document
                    .active_subtool_id()
                    .expect("nonempty fixture active id"),
                17,
                Pos2::new(50.0, 50.0),
                false,
            )
            .unwrap(),
        );
        gesture
            .as_mut()
            .unwrap()
            .move_to(Pos2::new(700.0, 550.0))
            .unwrap();
        let mut suppressed = false;
        assert!(clear_mask_region(&mut gesture, &mut suppressed, true));
        assert!(suppressed);
        assert!(gesture.is_none());
        assert_eq!(document.to_json().unwrap(), saved);
        assert!(document.redo());
        // Suppression remains set if a second command has no preview to clear.
        assert!(!clear_mask_region(&mut gesture, &mut suppressed, false));
        assert!(suppressed);
    }

    #[test]
    fn temporary_ctrl_retains_priority_without_changing_the_selected_brush() {
        assert_eq!(
            effective_brush_kind(
                BrushKind::Draw,
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT
            ),
            BrushKind::Mask
        );
        assert_eq!(
            effective_brush_kind(BrushKind::Draw, egui::Modifiers::SHIFT),
            BrushKind::Smooth
        );
        assert_eq!(
            effective_brush_kind(BrushKind::Draw, egui::Modifiers::NONE),
            BrushKind::Draw
        );
    }
}

#[cfg(test)]
mod paint_fill_host_tests {
    use super::*;

    fn settings() -> BrushSettings {
        BrushSettings {
            kind: BrushKind::Paint,
            paint_rgb: false,
            paint_roughness: true,
            roughness: 0.75,
            ..Default::default()
        }
    }

    #[test]
    fn fill_uses_one_undo_step_and_preserves_implicit_coarse_defaults() {
        let mut d = Document::new(Mesh::sphere(0));
        d.add_subdivision_level().unwrap();
        let before = d.to_json().unwrap();
        let r = paint_fill_after_barrier(&mut d, &settings(), false).unwrap();
        assert!(r.materialized && r.changed_material_vertices > 0);
        assert_eq!(r.changed_rgb_vertices, 0);
        let after = d.to_json().unwrap();
        assert_eq!(
            Document::from_json(&after).unwrap().to_json().unwrap(),
            after
        );
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
        d.select_level(0).unwrap();
        assert!(
            d.require_active_mesh()
                .unwrap()
                .materials
                .as_ref()
                .unwrap()
                .iter()
                .all(|v| *v == obrush_core::DEFAULT_VERTEX_MATERIAL)
        );
    }

    #[test]
    fn no_op_fill_preserves_none_and_prior_redo() {
        let mut d = Document::new(Mesh::sphere(0));
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[0][0] = 0.1;
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut s = settings();
        s.paint_roughness = false;
        let r = paint_fill_after_barrier(&mut d, &s, false).unwrap();
        assert_eq!(r.changed_vertices, 0);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
    }

    #[test]
    fn rejected_prior_stroke_and_invalid_targets_cannot_create_fill_history() {
        let mut d = Document::new(Mesh::sphere(0));
        let before = d.to_json().unwrap();
        assert!(paint_fill_after_barrier(&mut d, &settings(), true).is_err());
        let mut invalid = settings();
        invalid.metallic = f32::NAN;
        assert!(paint_fill_after_barrier(&mut d, &invalid, false).is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
}

#[cfg(test)]
mod preference_host_tests {
    use super::*;
    fn store() -> BrushPreferenceStore {
        BrushPreferenceStore::from_legacy(&initial_brush_settings(), None, false).unwrap()
    }
    fn sample(s: &BrushPreferenceStore, kind: BrushKind, pressure: f32) -> BrushSettings {
        s.preview(
            temporary_modifiers(s.selected(), kind),
            ContactInput::Pen(pressure),
            None,
            None,
        )
        .unwrap()
        .settings
    }

    #[test]
    fn host_default_views_preserve_existing_geometry_pressure_and_material_flags() {
        let s = store();
        for kind in BrushKind::ALL {
            let p = s.tool(kind);
            assert_eq!(p.radius, 0.18);
            assert_eq!(p.strength, 0.35);
            assert!(p.paint_rgb);
            assert!(!p.paint_roughness && !p.paint_metallic);
            assert_eq!([p.roughness, p.metallic], [0.18, 0.08]);
            assert!(!p.screen_space_radius);
            assert_eq!(p.screen_radius_pixels, 75.0);
        }
        assert_eq!(
            s.global().pressure,
            PressureProfile::LegacyBinary {
                strength: true,
                radius: false
            }
        );
    }

    #[test]
    fn sculptgl_pressure_factors_survive_unrelated_brush_edits_without_compounding() {
        let mut s = store();
        let mut global = s.global();
        global.pressure = PressureProfile::SculptGlFactors {
            intensity: 0.5,
            radius: 0.75,
        };
        s.set_global(global).unwrap();
        let base = s.tool(BrushKind::Draw).clone();
        let mut view = sample(&s, BrushKind::Draw, 0.25);
        view.radius = base.radius;
        view.strength = base.strength;
        view.color = [0.25, 0.5, 0.75];
        s = preference_view_candidate(&s, &view, false, 75.0, false)
            .unwrap()
            .unwrap();
        assert_eq!(s.global().pressure, global.pressure);
        assert_eq!(s.tool(BrushKind::Draw).radius, base.radius);
        assert_eq!(s.tool(BrushKind::Draw).strength, base.strength);
        assert_eq!(s.tool(BrushKind::Draw).color, view.color);
        for (pressure, intensity, radius) in [
            (0.0, 0.5, 0.25),
            (0.25, 0.75, 0.625),
            (0.5, 1.0, 1.0),
            (1.0, 1.5, 1.75),
        ] {
            let response = s
                .preview(
                    PreferenceModifiers::default(),
                    ContactInput::Pen(pressure),
                    None,
                    None,
                )
                .unwrap();
            assert_eq!(response.pressure.intensity_multiplier, intensity);
            assert_eq!(response.pressure.radius_multiplier, radius);
            assert!((response.settings.strength - base.strength * intensity).abs() < 1e-7);
            assert!((response.settings.radius - base.radius * radius).abs() < 1e-7);
        }
    }

    #[test]
    fn candidate_view_retains_per_tool_screen_alpha_color_and_global_preferences() {
        let mut s = store();
        let mut p = s.tool(BrushKind::Draw).clone();
        p.alpha = AlphaSelection::Square;
        s.set_tool(BrushKind::Draw, p).unwrap();
        let mut b = sample(&s, BrushKind::Draw, 0.5);
        b.radius = 0.625;
        b.strength = 0.5;
        b.color = [0.25, 0.5, 0.75];
        b.symmetry_x = false;
        b.pressure_radius = true;
        let mut s = preference_view_candidate(&s, &b, true, 130.0, false)
            .unwrap()
            .unwrap();
        assert_eq!(s.tool(BrushKind::Draw).alpha, AlphaSelection::Square);
        assert!(s.tool(BrushKind::Draw).screen_space_radius);
        assert_eq!(s.tool(BrushKind::Draw).screen_radius_pixels, 130.0);
        assert!(!s.global().symmetry_x);
        s.set_selected(BrushKind::Paint).unwrap();
        assert_eq!(s.tool(BrushKind::Paint).radius, 0.18);
        assert_eq!(s.tool(BrushKind::Paint).color, [0.72, 0.34, 0.22]);
        s.set_selected(BrushKind::Draw).unwrap();
        assert_eq!(sample(&s, BrushKind::Draw, 1.0).radius, 0.625);
    }

    #[test]
    fn transient_pressure_axes_and_adaptive_capability_do_not_overwrite_saved_options() {
        let s = store();
        let mut b = sample(&s, BrushKind::Draw, 0.125);
        b.adaptive_detail = false;
        b.alpha_tangent = Some(Vec3::Y);
        b.culling_axis = Some(Vec3::Z);
        assert!(
            preference_view_candidate(&s, &b, false, 75.0, false)
                .unwrap()
                .is_none()
        );
        assert!(s.tool(BrushKind::Draw).adaptive_detail);
        assert_eq!(s.revision(), 0);
    }

    #[test]
    fn rejected_ui_candidate_preserves_preferences_and_the_real_pending_dab_queue() {
        let s = store();
        let mut b = sample(&s, BrushKind::Draw, 1.0);
        b.roughness = f32::NAN;
        let mut q = DabQueue::default();
        q.begin();
        q.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 4.0),
                direction: -Vec3::Z,
            },
            sample(&s, BrushKind::Draw, 1.0),
            Vec3::ZERO,
        );
        let before = q.metrics();
        assert!(preference_view_candidate(&s, &b, false, 75.0, false).is_err());
        assert_eq!(q.metrics()["pending_dabs"], before["pending_dabs"]);
        assert_eq!(q.metrics()["coalesced_dabs"], before["coalesced_dabs"]);
        assert_eq!(s.revision(), 0);
    }

    #[test]
    fn old_captured_release_dab_uses_original_settings_before_new_view_install() {
        let s = store();
        let mut old = sample(&s, BrushKind::Draw, 1.0);
        old.adaptive_detail = false;
        old.symmetry_x = false;
        old.radius = 0.6;
        let mut b = old.clone();
        b.radius = 0.05;
        b.strength = 0.95;
        let candidate = preference_view_candidate(&s, &b, true, 300.0, false)
            .unwrap()
            .unwrap();
        let mut q = DabQueue::default();
        let mut d = Document::new(Mesh::sphere(2));
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 4.0),
            direction: -Vec3::Z,
        };
        let before = d.to_json().unwrap();
        q.begin();
        q.dab(ray, old.clone(), Vec3::ZERO);
        let progress = q.release(&mut d);
        assert!(progress.changed);
        let mut oracle = Document::new(Mesh::sphere(2));
        let h = oracle.require_active_mesh().unwrap().raycast(ray).unwrap();
        oracle.begin_stroke();
        oracle.require_active_mesh_mut().unwrap().apply_brush(
            h.position,
            h.normal,
            Vec3::ZERO,
            &old,
        );
        oracle.end_stroke();
        assert_eq!(d.to_json().unwrap(), oracle.to_json().unwrap());
        assert_eq!(candidate.tool(BrushKind::Draw).radius, 0.05);
        assert_eq!(q.metrics()["pending_dabs"], 0);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }

    #[test]
    fn temporary_tools_get_their_own_saved_screen_and_alpha_without_changing_selection() {
        let mut s = store();
        let mut p = s.tool(BrushKind::Smooth).clone();
        p.radius = 0.2;
        p.screen_space_radius = true;
        p.screen_radius_pixels = 23.0;
        p.alpha = AlphaSelection::Skin;
        s.set_tool(BrushKind::Smooth, p).unwrap();
        let hover = s
            .preview(
                PreferenceModifiers {
                    shift: true,
                    ..Default::default()
                },
                ContactInput::Pen(0.5),
                None,
                None,
            )
            .unwrap();
        let press = s
            .preview(
                temporary_modifiers(s.selected(), BrushKind::Smooth),
                ContactInput::Pen(0.5),
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            format!("{:?}", hover.settings),
            format!("{:?}", press.settings)
        );
        assert_eq!(hover.screen_radius, press.screen_radius);
        assert_eq!(hover.screen_radius.base_pixels, 23.0);
        assert!(hover.settings.alpha.is_some());
        assert_eq!(s.selected(), BrushKind::Draw);
    }

    #[test]
    fn initiating_tool_and_conversion_survive_async_selection_changes() {
        let mut s = store();
        let request = AlphaImportRequest {
            id: 7,
            destination: BrushKind::Draw,
            conversion: AlphaConversion::SculptGlMeanRgb,
        };
        s.set_selected(BrushKind::Paint).unwrap();
        let candidate = alpha_import_candidate(
            &s,
            request.destination,
            request.conversion,
            "red",
            1,
            1,
            &[255, 0, 0, 0],
        )
        .unwrap();
        assert_eq!(candidate.selected(), BrushKind::Paint);
        assert_eq!(candidate.tool(BrushKind::Paint).alpha, AlphaSelection::None);
        let bitmap = candidate
            .library()
            .resolve(candidate.tool(BrushKind::Draw).alpha)
            .unwrap()
            .unwrap();
        assert_eq!(bitmap.pixels(), [0]);
        assert_eq!(
            candidate.library().entries()[0].conversion,
            AlphaConversion::SculptGlMeanRgb
        );
    }

    #[test]
    fn both_import_conversions_actually_change_rgba_to_the_documented_weights() {
        let s = store();
        let mean = alpha_import_candidate(
            &s,
            BrushKind::Draw,
            AlphaConversion::SculptGlMeanRgb,
            "x",
            2,
            1,
            &[255, 0, 0, 0, 255, 0, 0, 255],
        )
        .unwrap();
        let weighted = alpha_import_candidate(
            &s,
            BrushKind::Draw,
            AlphaConversion::LegacyWeightedOpacity,
            "x",
            2,
            1,
            &[255, 0, 0, 0, 255, 0, 0, 255],
        )
        .unwrap();
        assert_eq!(mean.library().entries()[0].bitmap().pixels(), [0, 85]);
        assert_eq!(weighted.library().entries()[0].bitmap().pixels(), [0, 77]);
    }

    #[test]
    fn invalid_async_import_does_not_mutate_original_library_preferences_or_queue() {
        let s = store();
        let old = sample(&s, BrushKind::Draw, 1.0);
        let mut q = DabQueue::default();
        q.begin();
        q.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 4.0),
                direction: -Vec3::Z,
            },
            old,
            Vec3::ZERO,
        );
        assert!(
            alpha_import_candidate(
                &s,
                BrushKind::Draw,
                AlphaConversion::SculptGlMeanRgb,
                "x",
                1025,
                1,
                &[]
            )
            .is_err()
        );
        assert_eq!(s.library().imported_bytes(), 0);
        assert!(s.library().entries().is_empty());
        assert_eq!(s.tool(BrushKind::Draw).alpha, AlphaSelection::None);
        assert_eq!(q.metrics()["pending_dabs"], 1);
    }

    #[test]
    fn actual_png_candidate_preserves_request_destination_and_zeroes_hidden_rgb_for_source() {
        let png = include_bytes!("../tests/fixtures/independent-hidden-rgb2x2.png");
        let request = AlphaImportRequest {
            id: 29,
            destination: BrushKind::Draw,
            conversion: AlphaConversion::SculptGlMeanRgb,
        };
        let mut s = store();
        s.set_selected(BrushKind::Paint).unwrap();
        let result =
            alpha_image_import_candidate(&s, Some(request), 29, "hidden RGB", png).unwrap();
        assert_eq!(result.selected(), BrushKind::Paint);
        assert_eq!(result.tool(BrushKind::Paint).alpha, AlphaSelection::None);
        let bitmap = result
            .library()
            .resolve(result.tool(BrushKind::Draw).alpha)
            .unwrap()
            .unwrap();
        assert_eq!(bitmap.pixels(), [0, 85, 85, 255]);
        assert_eq!(s.library().imported_bytes(), 0);
    }

    #[test]
    fn stale_async_png_and_invalid_decode_reject_before_actual_queue_release() {
        let png = include_bytes!("../tests/fixtures/independent-hidden-rgb2x2.png");
        let request = AlphaImportRequest {
            id: 30,
            destination: BrushKind::Draw,
            conversion: AlphaConversion::LegacyWeightedOpacity,
        };
        let s = store();
        let mut q = DabQueue::default();
        let mut old_settings = sample(&s, BrushKind::Draw, 1.0);
        old_settings.radius = 0.6;
        old_settings.adaptive_detail = false;
        old_settings.symmetry_x = false;
        q.begin();
        q.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 4.0),
                direction: -Vec3::Z,
            },
            old_settings.clone(),
            Vec3::ZERO,
        );
        let stable_metrics = |queue: &DabQueue| {
            let mut metrics = queue.metrics();
            // Wall-clock age is expected to advance during PNG decoding. Every
            // actual queue counter must stay exact until explicit release.
            metrics
                .as_object_mut()
                .unwrap()
                .remove("oldest_pending_age_ms");
            metrics
        };
        let before = stable_metrics(&q);
        // Request mismatch is checked even before malformed PNG validation.
        assert_eq!(
            alpha_image_import_candidate(&s, Some(request), 29, "x", &[])
                .err()
                .unwrap(),
            "Alpha import request is stale"
        );
        assert!(alpha_image_import_candidate(&s, None, 30, "x", png).is_err());
        assert!(alpha_image_import_candidate(&s, Some(request), 30, "x", &png[..33]).is_err());
        assert_eq!(stable_metrics(&q), before);
        assert_eq!(s.revision(), 0);
        assert_eq!(s.library().imported_bytes(), 0);
        // The same live request remains usable after a bad decode.
        let candidate = alpha_image_import_candidate(&s, Some(request), 30, "x", png).unwrap();
        assert_eq!(
            candidate.library().entries()[0].bitmap().pixels(),
            [0, 75, 29, 64]
        );
        assert_eq!(stable_metrics(&q), before);
        let mut document = Document::new(Mesh::sphere(2));
        let mut oracle = Document::new(Mesh::sphere(2));
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 4.0),
            direction: -Vec3::Z,
        };
        let hit = oracle.require_active_mesh().unwrap().raycast(ray).unwrap();
        oracle.begin_stroke();
        oracle.require_active_mesh_mut().unwrap().apply_brush(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &old_settings,
        );
        oracle.end_stroke();
        q.release(&mut document);
        assert_eq!(document.to_json().unwrap(), oracle.to_json().unwrap());
        assert_eq!(document.can_undo(), oracle.can_undo());
        assert_eq!(q.metrics()["pending_dabs"], 0);
    }

    #[test]
    fn library_clear_and_delete_do_not_alias_another_tools_selection() {
        let mut s = alpha_import_candidate(
            &store(),
            BrushKind::Draw,
            AlphaConversion::LegacyWeightedOpacity,
            "shared",
            1,
            1,
            &[255; 4],
        )
        .unwrap();
        let id = s.tool(BrushKind::Draw).alpha;
        let mut p = s.tool(BrushKind::Paint).clone();
        p.alpha = id;
        s.set_tool(BrushKind::Paint, p).unwrap();
        let mut p = s.tool(BrushKind::Draw).clone();
        p.alpha = AlphaSelection::None;
        s.set_tool(BrushKind::Draw, p).unwrap();
        assert_eq!(s.tool(BrushKind::Paint).alpha, id);
        let AlphaSelection::Imported(id) = id else {
            unreachable!()
        };
        s.remove_alpha(id).unwrap();
        assert_eq!(s.tool(BrushKind::Paint).alpha, AlphaSelection::None);
        assert!(s.library().entries().is_empty());
    }
    fn captured_continuous_fixture(settings: BrushSettings, screen: bool) -> ActiveStroke {
        let kind = settings.kind;
        ActiveStroke {
            settings,
            screen_radius: ScreenRadiusPreferences {
                enabled: screen,
                pixels: 75.0,
            },
            pressure_factors: None,
            last_screen: None,
            sampler: StrokeSampler::new(Pos2::ZERO, 1.0),
            movement: None,
            local_transform: None,
            locked_stamp: None,
            kind,
            invert: false,
            queued: true,
            continuous: Some(StrokeKey {
                serial: 7,
                subtool_id: 1,
                kind,
            }),
            source_geometry: None,
            source_geometry_dirty_before: None,
            source_paint: None,
            source_paint_display_before: None,
            continuous_locked: None,
        }
    }

    #[test]
    fn browser_pen_samples_are_bounded_fifo_and_clear_cancels_an_overflow() {
        let mut samples = BrowserPenSamples::default();
        for x in 0..=MAX_BROWSER_PEN_MOVES {
            samples.push(BrowserPenMove {
                pos: Pos2::new(x as f32, 12.0),
                pressure: x as f32 / MAX_BROWSER_PEN_MOVES as f32,
            });
        }
        assert_eq!(samples.queued.len(), MAX_BROWSER_PEN_MOVES);
        assert_eq!(samples.dropped, 1);
        let (drained, overflowed) = samples.drain();
        assert!(overflowed);
        assert_eq!(drained.front().unwrap().pos.x, 1.0);
        assert_eq!(drained.back().unwrap().pos.x, MAX_BROWSER_PEN_MOVES as f32);
        assert!(samples.queued.is_empty());
        samples.push(BrowserPenMove {
            pos: Pos2::new(10.0, 20.0),
            pressure: 0.25,
        });
        samples.clear();
        assert!(samples.queued.is_empty());
        assert!(!samples.overflowed);
        assert_eq!(samples.dropped, 1);
    }

    #[test]
    fn browser_pen_replay_only_accepts_active_ordinary_queued_strokes() {
        let mut stroke = captured_continuous_fixture(BrushSettings::default(), false);
        assert!(!browser_pen_ordinary_stroke(&stroke));
        stroke.continuous = None;
        assert!(browser_pen_ordinary_stroke(&stroke));
        stroke.kind = BrushKind::Drag;
        assert!(!browser_pen_ordinary_stroke(&stroke));
        stroke.kind = BrushKind::Draw;
        stroke.queued = false;
        assert!(!browser_pen_ordinary_stroke(&stroke));
    }

    #[test]
    fn continuous_capture_retains_press_color_alpha_radius_and_only_uses_current_contact_overlay() {
        let settings = BrushSettings {
            kind: BrushKind::Paint,
            radius: 0.42,
            strength: 0.23,
            color: [0.125, 0.75, 0.375],
            alpha: Some(obrush_core::RasterAlpha::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap()),
            alpha_rotation: 0.37,
            paint_rgb: false,
            paint_roughness: true,
            roughness: 0.8,
            ..Default::default()
        };
        let stroke = captured_continuous_fixture(settings, false);
        let mut later_ui = stroke.settings.clone();
        later_ui.color = [1.0; 3];
        later_ui.radius = 0.9;
        later_ui.alpha = None;
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let dab = captured_stroke_dab(
            &stroke,
            &Camera::default(),
            rect.center(),
            rect,
            BrushKind::Paint,
            true,
            ContactInput::Pen(0.25),
        )
        .unwrap();
        assert_eq!(dab.settings.color, [0.125, 0.75, 0.375]);
        assert_eq!(dab.settings.radius, 0.42);
        assert_eq!(dab.settings.strength, 0.23);
        assert_eq!(dab.settings.alpha_rotation, 0.37);
        assert_eq!(
            dab.settings.alpha.as_ref().unwrap().pixels(),
            [1, 2, 3, 4, 5, 6]
        );
        assert_eq!(dab.settings.pressure, 0.25);
        assert!(dab.settings.invert);
        assert!(!dab.settings.paint_rgb && dab.settings.paint_roughness);
        assert_eq!(dab.settings.roughness, 0.8);
        assert_ne!(dab.settings.color, later_ui.color);
        assert!(dab.footprint.is_none());
    }

    #[test]
    fn continuous_screen_capture_does_not_apply_contact_radius_twice() {
        let settings = BrushSettings {
            pressure_radius: true,
            radius: 0.5,
            ..Default::default()
        };
        let stroke = captured_continuous_fixture(settings, true);
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let dab = captured_stroke_dab(
            &stroke,
            &camera,
            rect.center(),
            rect,
            BrushKind::Draw,
            false,
            ContactInput::Pen(0.25),
        )
        .unwrap();
        let radius = dab.footprint.unwrap().radius_at(Vec3::ZERO).unwrap();
        let mut at_hit = dab.settings;
        at_hit.radius = radius;
        assert!((at_hit.effective_radius().unwrap() - radius * 0.25).abs() < 1e-7);
        assert_eq!(stroke.screen_radius.pixels, 75.0);
    }

    #[test]
    fn captured_pressure_curve_tracks_each_pen_dab_and_keeps_mouse_neutral() {
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            ..Default::default()
        };
        let mut stroke = captured_continuous_fixture(settings, true);
        stroke.pressure_factors = Some((0.35, 0.18, 0.5, 0.75));
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let dab = |input| {
            captured_stroke_dab(
                &stroke,
                &camera,
                rect.center(),
                rect,
                BrushKind::Draw,
                false,
                input,
            )
            .unwrap()
        };
        let low = dab(ContactInput::Pen(0.25));
        let high = dab(ContactInput::Pen(0.75));
        let mouse = dab(ContactInput::Mouse);
        assert!((low.settings.strength - 0.2625).abs() < 1e-7);
        assert!((high.settings.strength - 0.4375).abs() < 1e-7);
        assert!((mouse.settings.strength - 0.35).abs() < 1e-7);
        assert!((low.settings.radius - 0.1125).abs() < 1e-7);
        assert!((high.settings.radius - 0.2475).abs() < 1e-7);
        assert!((mouse.settings.radius - 0.18).abs() < 1e-7);
        let radius = |dab: CapturedDab| dab.footprint.unwrap().radius_at(Vec3::ZERO).unwrap();
        let mouse_radius = radius(mouse);
        assert!((radius(low) / mouse_radius - 0.625).abs() < 1e-6);
        assert!((radius(high) / mouse_radius - 1.375).abs() < 1e-6);
    }

    #[test]
    fn continuous_capture_rejects_invalid_projection_and_outside_cursor() {
        let stroke = captured_continuous_fixture(BrushSettings::default(), true);
        let camera = Camera::default();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        assert!(
            captured_stroke_dab(
                &stroke,
                &camera,
                Pos2::new(-1.0, 0.0),
                rect,
                BrushKind::Draw,
                false,
                ContactInput::Pen(1.0)
            )
            .is_none()
        );
        let empty = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 0.0));
        assert!(
            captured_stroke_dab(
                &stroke,
                &camera,
                Pos2::new(400.0, 0.0),
                empty,
                BrushKind::Draw,
                false,
                ContactInput::Pen(1.0)
            )
            .is_none()
        );
    }
}

impl OBrushApp {
    fn capture_source_paint_input(
        &self,
        pos: Pos2,
        rect: egui::Rect,
    ) -> Result<crate::source_paint_host::SourcePaintInput, String> {
        crate::source_paint_host::capture_at_zoom(
            &self.camera,
            pos,
            rect,
            &self.source_paint_raw,
            self.egui_ctx.zoom_factor(),
        )
    }
    fn interrupt_source_paint_tail(&mut self, reason: &str) -> bool {
        if self.interrupt_source_geometry_tail(reason) {
            return true;
        }
        let Some(session) = self.stroke.as_ref().and_then(|s| s.source_paint.clone()) else {
            return false;
        };
        if let Ok(mut source) = session.lock() {
            if source.is_locked()
                && let Err(error) = source.cancel_locked(&mut self.document)
            {
                source.failed.get_or_insert(error);
            }
            source.failed.get_or_insert_with(|| reason.to_owned());
        }
        true
    }
    fn source_paint_panel(&mut self, ui: &mut egui::Ui) {
        if self.brush.kind != BrushKind::Paint {
            return;
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CCB65CD31C1AC,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        let available = if self.egui_ctx.zoom_factor() != 1.0 {
            Err(
                "Source Paint requires UI scale 100%; precise zoom preparation is not implemented"
                    .to_string(),
            )
        } else {
            crate::source_paint_host::stroke_blocker(
                &self.document,
                &self.source_paint_preferences,
                &self.brush,
                self.preferences.tool(BrushKind::Paint).lock_position,
            )
        };
        let mut mode = self.source_paint_preferences.mode;
        let legacy = ui.selectable_value(
            &mut mode,
            crate::source_paint_host::PaintMode::Legacy,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CB7CBE4272AD3),
        );
        self.track_morph_control("source_paint.mode.legacy", &legacy);
        let source = ui.add_enabled(
            available.is_ok(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C4BF09CCF0C75,
            ))
            .selected(mode == crate::source_paint_host::PaintMode::SourceOrdinary),
        );
        self.track_morph_control("source_paint.mode.source", &source);
        if source.clicked() {
            mode = crate::source_paint_host::PaintMode::SourceOrdinary;
        }
        if mode != self.source_paint_preferences.mode {
            self.finish_stroke();
            if !self.stroke_rejected {
                self.paint_pick_armed = false;
                self.paint_pick_dragging = false;
                self.source_paint_preferences.mode = mode;
            }
        }
        if let Err(reason) = available {
            ui.label(
                RichText::new(reason)
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
            );
        }
        let reset = source_paint_normal_controls::render(
            ui,
            self.document
                .active_subtool_id()
                .and_then(|id| self.document.source_geometry_state(id))
                .is_some(),
        );
        self.track_morph_control(source_paint_normal_controls::RESET_KEY, &reset);
        if reset.clicked() {
            self.finish_stroke();
            match source_paint_normal_controls::after_release(
                &mut self.document,
                !self.stroke_rejected,
            ) {
                Ok(true) => {
                    self.egui_ctx.request_repaint();
                    self.message="Transient Source normals reset; ordinary Paint uses a cold fresh producer.".into();
                }
                Ok(false) => {}
                Err(error) => self.message = error,
            }
        }
        if let Some(state) = self
            .document
            .active_subtool_id()
            .and_then(|id| self.document.source_geometry_state(id))
        {
            ui.label(
                RichText::new(format!(
                    "Retained selected-refresh normals · generation {}",
                    state.generation()
                ))
                .size(10.)
                .color(MUTED),
            );
        } else {
            ui.label(
                RichText::new("Cold fresh original-face normals")
                    .size(10.)
                    .color(MUTED),
            );
        }
        let preview = ui.add_enabled(
            self.document.active_subtool_visible(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C8D63821A78EB,
            )),
        );
        self.track_morph_control("source_paint.mask.preview", &preview);
        if preview.clicked() {
            self.finish_stroke();
            if !self.stroke_rejected {
                match self.document.active_subtool_mask_conversion_report(
                    obrush_core::MaskConvention::UnprotectedF32V1,
                ) {
                    Ok(report) => {
                        self.source_mask_preview = Some((
                            self.revision,
                            self.document.active_subtool_id(),
                            self.document.current_level(),
                            report,
                        ))
                    }
                    Err(error) => self.message = error,
                }
            }
        }
        if let Some((revision, id, level, report)) = self.source_mask_preview.clone() {
            let current = revision == self.revision
                && id == self.document.active_subtool_id()
                && level == self.document.current_level();
            ui.label(format!(
                "{} levels · {} mask values · {} roundtrip words change",
                report.levels.len(),
                report.vertices,
                report.roundtrip_changed_words
            ));
            ui.label(format!(
                "Max absolute {:.8e}, relative {:.8e}; {} signed-zero-only words",
                report.max_abs_roundtrip_error,
                report.max_relative_roundtrip_error,
                report.signed_zero_only_changes
            ));
            for row in &report.levels {
                if let Some(w) = row.first_changed_word {
                    ui.label(format!(
                        "Level {} / vertex {}: {:08x} → {:08x} → {:08x}",
                        row.level + 1,
                        w.vertex,
                        w.source_bits,
                        w.converted_bits,
                        w.roundtrip_bits
                    ));
                }
            }
            let allow = ui.checkbox(
                &mut self.source_allow_mask_loss,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CDA6B23E1206A),
            );
            self.track_morph_control("source_paint.mask.allow_loss", &allow);
            let convert = ui.add_enabled(
                current
                    && report.changed
                    && (report.roundtrip_changed_words == 0 || self.source_allow_mask_loss),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CC94C5D60B21F,
                )),
            );
            self.track_morph_control("source_paint.mask.convert", &convert);
            if convert.clicked() {
                self.finish_stroke();
                if !self.stroke_rejected {
                    let fresh = self.document.active_subtool_mask_conversion_report(
                        obrush_core::MaskConvention::UnprotectedF32V1,
                    );
                    if fresh.as_ref().is_ok_and(|fresh| fresh == &report) {
                        let policy = if self.source_allow_mask_loss {
                            obrush_core::MaskConversionPolicy::AllowAndReport
                        } else {
                            obrush_core::MaskConversionPolicy::RequireRoundtripExact
                        };
                        match self.document.convert_active_subtool_masks(
                            obrush_core::MaskConvention::UnprotectedF32V1,
                            policy,
                        ) {
                            Ok(result) => {
                                if result.changed {
                                    self.changed();
                                }
                                self.message="Complete active mask representation converted; one Undo restores all levels.".into();
                                self.source_mask_preview = None;
                            }
                            Err(e) => self.message = e,
                        }
                    } else {
                        self.message = "Mask precision preview is stale; inspect it again.".into();
                        self.source_mask_preview = None;
                    }
                }
            }
        }
        if self.source_paint_preferences.mode != crate::source_paint_host::PaintMode::SourceOrdinary
        {
            return;
        }
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C749A7E33B5DB,
            ))
            .size(10.0)
            .color(preferences_host::muted(ui)),
        );
        let numeric = crate::source_paint_ui::render_numeric_preferences(
            ui,
            &self.source_paint_preferences,
            &mut self.source_paint_editor_drafts,
        );
        for (key, response) in &numeric.controls {
            self.track_morph_control(key, response);
        }
        if let Some(error) = numeric.rejected {
            self.message = error;
            return;
        }
        let mut changed = numeric.candidate.is_some();
        let mut candidate = numeric
            .candidate
            .unwrap_or_else(|| self.source_paint_preferences.clone());
        for (label, value, key) in [
            ("RGB", &mut candidate.options.write_rgb, "source_paint.rgb"),
            (
                "Roughness",
                &mut candidate.options.write_roughness,
                "source_paint.roughness.enabled",
            ),
            (
                "Metallic",
                &mut candidate.options.write_metallic,
                "source_paint.metallic.enabled",
            ),
            (
                "Backface culling",
                &mut candidate.options.culling,
                "source_paint.culling",
            ),
        ] {
            let response = ui.checkbox(value, label);
            self.track_morph_control(key, &response);
            changed |= response.changed();
        }
        let symmetry = ui.checkbox(
            &mut self.brush.symmetry_x,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C97FCF490FDDC),
        );
        self.track_morph_control("source_paint.symmetry", &symmetry);
        if symmetry.changed() {
            self.commit_brush_view();
        }
        let mut color = candidate.options.color;
        let response = ui.color_edit_button_rgb(&mut color);
        self.track_morph_control("source_paint.color", &response);
        if response.changed() {
            candidate.options.color = color;
            changed = true;
        }
        let locked =
            crate::source_paint_host::render_lock_control(ui, &mut self.brush_lock_position);
        self.track_morph_control(crate::source_paint_host::SOURCE_PAINT_LOCK_CONTROL, &locked);
        if locked.changed() {
            self.commit_brush_view();
        }
        let mut continuous = self.preferences.global().continuous;
        let response = ui.checkbox(
            &mut continuous,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C6595EA93D585),
        );
        self.track_morph_control("source_paint.continuous", &response);
        if response.changed() {
            self.set_continuous_mode(continuous);
        }
        if changed {
            match crate::source_paint_ui::commit_candidate(Some(candidate), || {
                self.finish_stroke();
                !self.stroke_rejected
            }) {
                crate::source_paint_ui::PreferenceCommit::Accepted(candidate) => {
                    self.source_paint_preferences = candidate;
                }
                crate::source_paint_ui::PreferenceCommit::ReleaseRejected => {
                    self.source_paint_editor_drafts
                        .reset(&self.source_paint_preferences);
                }
                crate::source_paint_ui::PreferenceCommit::Invalid(error) => {
                    self.source_paint_editor_drafts
                        .reset(&self.source_paint_preferences);
                    self.message = error;
                }
                crate::source_paint_ui::PreferenceCommit::Unchanged => {}
            }
        }
    }

    fn source_paint_continuous_frame(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        key: StrokeKey,
        session: crate::source_paint_host::SharedSourcePaint,
    ) {
        let captured = match session.lock() {
            Ok(s) => s.preferences.clone(),
            Err(_) => {
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                self.message = "Source Paint session lock failed.".into();
                return;
            }
        };
        let (position, down, focused) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
                i.focused,
            )
        });
        let position = position.filter(|p| p.is_finite() && rect.contains(*p));
        let prepared = position
            .map(|p| self.capture_source_paint_input(p, rect))
            .transpose();
        let input = match prepared {
            Ok(value) => value,
            Err(error) => {
                if let Ok(mut source) = session.lock() {
                    if source.is_locked() {
                        let _ = source.cancel_locked(&mut self.document);
                    }
                    source.failed = Some(error.clone());
                }
                self.unprocessed_release = None;
                self.stop_continuous(StopReason::InvalidContact);
                self.finish_stroke_deferred();
                self.stroke_rejected = true;
                self.message = error;
                return;
            }
        };
        let Some(now) = self.continuous_now() else {
            self.stop_continuous(StopReason::ClockOverflow);
            self.finish_stroke_deferred();
            return;
        };
        let source_locked = session.lock().is_ok_and(|s| {
            s.locked_mode() == Some(obrush_core::SourcePaintLockedMode::ContinuousAccumulation)
        });
        let surface_hit = if source_locked {
            let readiness = input
                .map(|input| {
                    session
                        .lock()
                        .map_err(|_| "Source Paint Lock mutex failed".to_owned())
                        .and_then(|mut s| s.locked_timer_ready(&self.document, input))
                })
                .transpose();
            match readiness {
                Ok(hit) => hit.unwrap_or(false),
                Err(error) => {
                    if let Ok(mut source) = session.lock() {
                        let _ = source.cancel_locked(&mut self.document);
                        source.failed = Some(error.clone());
                    }
                    self.stop_continuous(StopReason::InvalidContact);
                    self.finish_stroke_deferred();
                    self.stroke_rejected = true;
                    self.message = error;
                    return;
                }
            }
        } else {
            input.is_some_and(|input| {
                self.document
                    .require_active_mesh()
                    .and_then(|mesh| {
                        obrush_core::prepare_source_paint_pass(
                            mesh,
                            self.document.active_object_metadata().map(|m| &m.affine),
                            obrush_core::PaintPreparationInput {
                                view: input.view,
                                screen: input.screen,
                                screen_radius: captured.radius,
                                pressure: input.raw.pressure,
                                radius_factor: captured.radius_factor,
                            },
                        )
                    })
                    .is_ok_and(|p| p.is_some())
            })
        };
        let ready = Readiness {
            key: (Some(key.subtool_id) == self.document.active_subtool_id()).then_some(key),
            mode_enabled: self.preferences.global().continuous,
            primary_down: down,
            pressure: if input.is_some_and(|p| p.in_contact) {
                1.0
            } else {
                0.0
            },
            focused,
            navigating: ctx.input(|i| viewport_navigation_requested(i, self.navigation_down())),
            target_visible: self.document.active_subtool_visible(),
            pointer_in_viewport: position.is_some(),
            surface_hit,
            queue_empty: self.pending_dabs.is_empty(),
            operation_busy: self.topology_job.is_some(),
        };
        let report = self.continuous_clock.report();
        if report.active_key.is_none()
            && matches!(
                report.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
            && ready.primary_down
            && ready.focused
            && !ready.navigating
            && ready.target_visible
            && ready.surface_hit
            && ready.queue_empty
            && ready.pressure > 0.
            && let Err(error) = if source_locked {
                self.continuous_clock
                    .start_locked_accumulation(key, true, now)
            } else {
                self.continuous_clock.start(key, true, false, now)
            }
        {
            self.message = error;
            return;
        }
        if let Some(permit) = self
            .continuous_clock
            .poll(ctx.cumulative_frame_nr(), now, ready)
        {
            if let Err(error) =
                self.continuous_clock
                    .consume(permit, ctx.cumulative_frame_nr(), ready)
            {
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                self.message = error;
                return;
            }
            if source_locked
                && permit.mode() != crate::continuous_host::ContinuousMode::LockedAccumulation
            {
                self.interrupt_source_paint_tail("Source Paint Lock timer received a foreign mode");
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                return;
            }
            if let Some(input) = input {
                self.last_continuous_tick = Some(permit.at_micros());
                self.pending_dabs.source_paint_dab(session, input);
                let progress = self.pending_dabs.process(&mut self.document, 1, None);
                self.handle_dab_progress(progress);
                debug_assert!(self.pending_dabs.is_empty());
            }
        }
        let report = self.continuous_clock.report();
        if report.active_key.is_none()
            && !matches!(
                report.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
        {
            self.finish_stroke_deferred();
            return;
        }
        if let Some(delay) = self.continuous_clock.micros_until_due(now) {
            ctx.request_repaint_after(std::time::Duration::from_micros(delay.max(1)));
        }
    }
}

impl OBrushApp {
    fn begin_source_paint_stroke(
        &mut self,
        ctx: &egui::Context,
        pos: Pos2,
        rect: egui::Rect,
        invert: bool,
    ) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let mut settings = self.brush.clone();
        settings.kind = BrushKind::Paint;
        settings.invert ^= invert;
        let locked = self.preferences.tool(BrushKind::Paint).lock_position;
        let prepared = self
            .capture_source_paint_input(pos, rect)
            .and_then(|input| {
                crate::source_paint_host::SourcePaintSession::prepare_tracked_for_stroke(
                    &mut self.document,
                    &self.source_paint_preferences,
                    &settings,
                    locked,
                    self.preferences.global().continuous,
                    input,
                    self.source_geometry_alpha_tracks.clone(),
                )
            });
        let session = match prepared {
            Ok(value) => value,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        if let Some(query) = self.paint_cursor_capture()
            && let Ok(mut s) = session.lock()
        {
            s.capture_cursor(query);
        }
        let continuous = if self.preferences.global().continuous {
            let Some(next) = self.next_continuous_serial.checked_add(1) else {
                self.message = "Continuous identifiers exhausted.".into();
                if let Ok(mut s) = session.lock() {
                    let _ = s.cancel_locked(&mut self.document);
                }
                return;
            };
            let Some(id) = self.document.active_subtool_id() else {
                if let Ok(mut s) = session.lock() {
                    let _ = s.cancel_locked(&mut self.document);
                }
                return;
            };
            let key = StrokeKey {
                serial: self.next_continuous_serial,
                subtool_id: id,
                kind: BrushKind::Paint,
            };
            let Some(now) = self.continuous_now() else {
                if let Ok(mut s) = session.lock() {
                    let _ = s.cancel_locked(&mut self.document);
                }
                return;
            };
            let started = if locked {
                self.continuous_clock
                    .start_locked_accumulation(key, true, now)
            } else {
                self.continuous_clock.start(key, true, false, now)
            };
            if let Err(error) = started {
                self.message = error;
                if let Ok(mut s) = session.lock() {
                    let _ = s.cancel_locked(&mut self.document);
                }
                return;
            }
            self.next_continuous_serial = next;
            ctx.request_repaint_after(std::time::Duration::from_micros(16_600));
            Some(key)
        } else {
            None
        };
        self.pending_dabs.begin();
        self.stroke = Some(ActiveStroke {
            settings,
            screen_radius: ScreenRadiusPreferences {
                enabled: true,
                pixels: self.source_paint_preferences.radius as f32,
            },
            pressure_factors: None,
            last_screen: Some(pos),
            sampler: StrokeSampler::new(pos, 1.0),
            movement: None,
            local_transform: None,
            locked_stamp: None,
            kind: BrushKind::Paint,
            invert,
            queued: true,
            continuous,
            source_geometry: None,
            source_geometry_dirty_before: None,
            source_paint: Some(session),
            source_paint_display_before: Some((self.dirty, self.viewport_style.use_vertex_color)),
            continuous_locked: None,
        });
    }
}

impl OBrushApp {
    fn continuous_locked_blocker(&self, kind: BrushKind) -> Result<(), String> {
        if !crate::continuous_host::supports_continuous_locked(kind) {
            return Err("This tool has no Continuous Lock workflow".into());
        }
        if kind == BrushKind::Paint
            && self.source_paint_preferences.mode
                == crate::source_paint_host::PaintMode::SourceOrdinary
        {
            return crate::source_paint_host::stroke_blocker(
                &self.document,
                &self.source_paint_preferences,
                &self.brush,
                true,
            );
        }
        if self.egui_ctx.zoom_factor() != 1.0 {
            return Err("Continuous Lock requires UI scale100% until CSS/logical zoom preparation is supported".into());
        }
        if !self.document.active_subtool_visible() {
            return Err("Show an active part for Continuous Lock".into());
        }
        Ok(())
    }
    fn begin_continuous_locked(
        &mut self,
        ctx: &egui::Context,
        pos: Pos2,
        rect: egui::Rect,
        kind: BrushKind,
        invert: bool,
    ) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let capture = (|| {
            self.continuous_locked_blocker(kind)?;
            if !self.pending_dabs.is_empty() {
                return Err("Continuous Lock needs the preceding queue to be complete".to_string());
            }
            let mut settings = self
                .preferences
                .preview(
                    temporary_modifiers(self.preferences.selected(), kind),
                    self.current_contact_input(),
                    None,
                    None,
                )?
                .settings;
            settings.kind = kind;
            settings.invert ^= invert;
            let input = self.capture_source_paint_input(pos, rect)?;
            let gesture = crate::continuous_locked_host::ContinuousLockedGesture::prepare(
                &self.document,
                &settings,
                input,
                self.camera.axes64().0.to_array(),
            )?;
            Ok::<_, String>((settings, gesture))
        })();
        let (settings, gesture) = match capture {
            Ok(v) => v,
            Err(e) => {
                self.message = format!("Continuous Lock could not start: {e}");
                return;
            }
        };
        let Some(next) = self.next_continuous_serial.checked_add(1) else {
            self.message = "Continuous identifiers exhausted.".into();
            return;
        };
        let Some(id) = self.document.active_subtool_id() else {
            return;
        };
        let key = StrokeKey {
            serial: self.next_continuous_serial,
            subtool_id: id,
            kind,
        };
        let Some(now) = self.continuous_now() else {
            self.message = "Continuous clock overflow.".into();
            return;
        };
        if let Err(e) = self
            .continuous_clock
            .start_locked_accumulation(key, true, now)
        {
            self.message = e;
            return;
        }
        self.next_continuous_serial = next;
        if gesture.paints_material() {
            self.document.begin_material_paint_stroke();
        } else {
            self.document.begin_stroke();
        }
        self.stroke = Some(ActiveStroke {
            settings,
            screen_radius: ScreenRadiusPreferences {
                enabled: true,
                pixels: self.preferences.tool(kind).screen_radius_pixels,
            },
            pressure_factors: pressure_factors_for_stroke(&self.preferences, kind),
            last_screen: Some(pos),
            sampler: StrokeSampler::new(pos, 1.0),
            movement: None,
            local_transform: None,
            locked_stamp: None,
            kind,
            invert,
            queued: false,
            continuous: Some(key),
            source_geometry: None,
            source_geometry_dirty_before: None,
            source_paint: None,
            source_paint_display_before: None,
            continuous_locked: Some(ActiveContinuousLocked {
                gesture,
                dirty_before: self.dirty,
                vertex_color_before: self.viewport_style.use_vertex_color,
            }),
        });
        self.last_continuous_locked_report = None;
        ctx.request_repaint_after(std::time::Duration::from_micros(16_600));
    }
    fn cancel_continuous_locked(&mut self) -> bool {
        if self
            .stroke
            .as_ref()
            .is_none_or(|s| s.continuous_locked.is_none())
        {
            return false;
        }
        self.stop_continuous(StopReason::Command);
        self.unprocessed_release = None;
        let result = {
            let locked = self
                .stroke
                .as_mut()
                .unwrap()
                .continuous_locked
                .as_mut()
                .unwrap();
            locked.gesture.cancel(&mut self.document)
        };
        match result {
            Ok(report) => {
                if report.changed_vertices > 0 {
                    self.changed();
                }
                self.finish_stroke_deferred();
                self.message =
                    "Continuous Lock cancelled; original surface and no-op history restored."
                        .into();
            }
            Err(error) => {
                self.finish_stroke_deferred();
                self.stroke_rejected = true;
                self.message = format!("Continuous Lock cancellation rejected: {error}");
            }
        }
        true
    }
    fn continuous_locked_frame(&mut self, ctx: &egui::Context, rect: egui::Rect, key: StrokeKey) {
        use crate::continuous_host::ContinuousMode;
        let (position, down, focused) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
                i.focused,
            )
        });
        let position = position.filter(|p| p.is_finite() && rect.contains(*p));
        let capture = position
            .map(|p| self.capture_source_paint_input(p, rect))
            .transpose();
        let input = match capture {
            Ok(v) => v,
            Err(e) => {
                self.end_rejected_continuous_locked(e);
                return;
            }
        };
        let preparation = if let Some(input) = input {
            self.stroke
                .as_mut()
                .and_then(|s| s.continuous_locked.as_mut())
                .ok_or_else(|| "Continuous Lock gesture was lost".to_string())
                .and_then(|s| {
                    s.gesture
                        .prepare_tick(&self.document, input, self.brush.pressure)
                })
                .map(Some)
        } else {
            Ok(None)
        };
        let prepared = match preparation {
            Ok(v) => v,
            Err(e) => {
                self.end_rejected_continuous_locked(e);
                return;
            }
        };
        let anchored_hit = prepared.as_ref().is_some_and(|p| p.anchored_hit());
        let ready = Readiness {
            key: (Some(key.subtool_id) == self.document.active_subtool_id()).then_some(key),
            mode_enabled: self.preferences.global().continuous,
            primary_down: down,
            pressure: self.brush.pressure,
            focused,
            navigating: ctx.input(|i| viewport_navigation_requested(i, self.navigation_down())),
            target_visible: self.document.active_subtool_visible(),
            pointer_in_viewport: position.is_some(),
            surface_hit: anchored_hit,
            queue_empty: self.pending_dabs.is_empty(),
            operation_busy: self.topology_job.is_some(),
        };
        let Some(now) = self.continuous_now() else {
            self.end_rejected_continuous_locked("Continuous clock overflow".into());
            return;
        };
        let previous = self.continuous_clock.report();
        if previous.active_key.is_none()
            && matches!(
                previous.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
            && ready.primary_down
            && ready.mode_enabled
            && ready.focused
            && !ready.navigating
            && ready.target_visible
            && ready.pointer_in_viewport
            && ready.surface_hit
            && ready.queue_empty
            && !ready.operation_busy
            && ready.pressure > 0.0
            && ready.pressure <= 1.0
            && let Err(e) = self
                .continuous_clock
                .start_locked_accumulation(key, true, now)
        {
            self.end_rejected_continuous_locked(e);
            return;
        }
        if let Some(permit) = self
            .continuous_clock
            .poll(ctx.cumulative_frame_nr(), now, ready)
        {
            if permit.mode() != ContinuousMode::LockedAccumulation {
                self.end_rejected_continuous_locked("Wrong timer mode for Continuous Lock".into());
                return;
            }
            if let Err(e) = self
                .continuous_clock
                .consume(permit, ctx.cumulative_frame_nr(), ready)
            {
                self.end_rejected_continuous_locked(e);
                return;
            }
            debug_assert!(self.pending_dabs.is_empty());
            self.last_continuous_tick = Some(permit.at_micros());
            let result = if let Some(prepared) = prepared {
                self.stroke
                    .as_mut()
                    .and_then(|s| s.continuous_locked.as_mut())
                    .ok_or_else(|| "Continuous Lock gesture was lost".to_string())
                    .and_then(|s| s.gesture.apply_tick(&mut self.document, prepared))
            } else {
                Err("Continuous Lock permission had no anchored sample".into())
            };
            match result {
                Ok(Some(report)) => {
                    let paints_rgb = self
                        .stroke
                        .as_ref()
                        .and_then(|s| s.continuous_locked.as_ref())
                        .is_some_and(|s| s.gesture.paints_rgb());
                    if report.changed_vertices > 0 {
                        self.changed();
                        if paints_rgb {
                            self.viewport_style.use_vertex_color = true;
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    self.end_rejected_continuous_locked(e);
                    return;
                }
            }
            debug_assert!(self.pending_dabs.is_empty());
        }
        let after = self.continuous_clock.report();
        if after.active_key.is_none()
            && !matches!(
                after.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
        {
            self.finish_stroke_deferred();
            return;
        }
        if let Some(delay) = self.continuous_clock.micros_until_due(now) {
            ctx.request_repaint_after(std::time::Duration::from_micros(delay.max(1)));
        }
    }
    fn end_rejected_continuous_locked(&mut self, error: String) {
        self.stop_continuous(StopReason::Command);
        self.unprocessed_release = None;
        self.finish_stroke_deferred();
        self.stroke_rejected = true;
        self.message = format!("Continuous Lock input rejected: {error}");
    }
}

#[cfg(test)]
#[path = "source_paint_ui_contract_tests.rs"]
mod source_paint_ui_contract_tests;

#[cfg(test)]
#[path = "source_paint_ui_host_tests.rs"]
mod source_paint_ui_host_tests;

#[cfg(test)]
#[path = "source_geometry_consumer_tests.rs"]
mod source_geometry_consumer_tests;
#[cfg(test)]
#[path = "source_geometry_ui_contract_tests.rs"]
mod source_geometry_ui_contract_tests;

#[path = "source_paint_normal_controls.rs"]
mod source_paint_normal_controls;

#[cfg(test)]
#[path = "screen_radius_ui_tests.rs"]
mod screen_radius_ui_tests;

#[cfg(test)]
#[path = "keyboard_shift_observer_tests.rs"]
mod keyboard_shift_observer_tests;

#[cfg(test)]
#[path = "brush_label_layout_tests.rs"]
mod brush_label_layout_tests;
#[cfg(test)]
#[path = "inspector_layout_tests.rs"]
mod inspector_layout_tests;

#[cfg(test)]
#[path = "source_paint_lock_host_tests.rs"]
mod source_paint_lock_host_tests;
