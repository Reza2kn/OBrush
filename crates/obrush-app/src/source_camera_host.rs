//! Optional source framing controller; ordinary camera math remains separate.
//! Initial loose bounds are source-derived. General octree history/gizmo bounds
//! and source navigation gains/animations remain explicit unsupported coverage.
use crate::source_camera_math::{
    ClipInput, ClipRange, ProjectionKind, ProjectionZoom, SceneBounds, SourceCameraFrame,
    SourcePose, ViewportDimensions, WorldToView, source_grid_clip_bounds,
};
use glam::{DQuat, DVec2, DVec3};
use obrush_core::{MAX_SUBTOOLS, MAX_VERTICES, Mesh, SubtoolRenderSnapshot};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

const MAX_POSITION_CACHE_BYTES: usize = 64 * 1024 * 1024;
// Candidate preparation shares accepted rows; at most one new position vector
// is allocated while the accepted cache is retained. No Vec-to-Arc slice copy.
const MAX_POSITION_CACHE_TRANSIENT_BYTES: usize = 2 * MAX_POSITION_CACHE_BYTES;
#[derive(Clone, Debug)]
struct BoundsMemo {
    mesh: Weak<Mesh>,
    positions: Arc<Vec<glam::Vec3>>,
    local: SceneBounds,
}
#[derive(Clone, Debug, Default)]
struct BoundsCache {
    rows: HashMap<u32, BoundsMemo>,
    bytes: usize,
    hits: u64,
    position_reuses: u64,
    scans: u64,
}
impl BoundsCache {
    fn local(&mut self, id: u32, mesh: &Arc<Mesh>) -> Result<SceneBounds, String> {
        if let Some(old) = self.rows.get_mut(&id) {
            if old.mesh.upgrade().is_some_and(|m| Arc::ptr_eq(&m, mesh)) {
                self.hits = self.hits.saturating_add(1);
                return Ok(old.local);
            }
            if old.positions.as_slice() == mesh.positions.as_slice() {
                old.mesh = Arc::downgrade(mesh);
                self.position_reuses = self.position_reuses.saturating_add(1);
                return Ok(old.local);
            }
        }
        let local = initial_loose_bounds(mesh)?;
        self.scans = self.scans.saturating_add(1);
        let needed = mesh
            .positions
            .len()
            .checked_mul(std::mem::size_of::<glam::Vec3>())
            .ok_or("Camera bounds cache size overflow")?;
        if let Some(old) = self.rows.remove(&id) {
            self.bytes = self
                .bytes
                .saturating_sub(old.positions.len() * std::mem::size_of::<glam::Vec3>());
        }
        if self
            .bytes
            .checked_add(needed)
            .is_some_and(|v| v <= MAX_POSITION_CACHE_BYTES)
        {
            let mut words = Vec::new();
            words
                .try_reserve_exact(mesh.positions.len())
                .map_err(|_| "Camera position cache allocation failed")?;
            words.extend_from_slice(&mesh.positions);
            self.rows.insert(
                id,
                BoundsMemo {
                    mesh: Arc::downgrade(mesh),
                    positions: Arc::new(words),
                    local,
                },
            );
            self.bytes += needed;
        }
        // Exceeding the cache budget never grows unbounded storage; a future
        // frame can compute its validated box again. No performance claim.
        Ok(local)
    }
    // Mesh-only bounds for Frame all. The hidden grid belongs to clip-range
    // optimization, but must never enlarge the requested model fit.
    fn meshes(&mut self, objects: &[SubtoolRenderSnapshot]) -> Result<Option<SceneBounds>, String> {
        if objects.len() > MAX_SUBTOOLS {
            return Err("Camera scene exceeds object budget".into());
        }
        self.rows
            .retain(|id, _| objects.iter().any(|o| o.id == *id));
        self.bytes = self
            .rows
            .values()
            .map(|r| r.positions.len() * std::mem::size_of::<glam::Vec3>())
            .sum();
        let mut bounds: Option<SceneBounds> = None;
        for object in objects {
            let local = self.local(object.id, &object.mesh)?;
            let world = if let Some(meta) = &object.metadata {
                transform_bounds(local, meta.affine.local_to_world)?
            } else {
                local
            };
            bounds = Some(bounds.map_or(world, |b| b.union(world)));
        }
        Ok(bounds)
    }
    fn scene(&mut self, objects: &[SubtoolRenderSnapshot]) -> Result<SceneBounds, String> {
        if objects.len() > MAX_SUBTOOLS {
            return Err("Camera scene exceeds object budget".into());
        }
        self.rows
            .retain(|id, _| objects.iter().any(|o| o.id == *id));
        self.bytes = self
            .rows
            .values()
            .map(|r| r.positions.len() * std::mem::size_of::<glam::Vec3>())
            .sum();
        let mut bounds = source_grid_clip_bounds();
        for object in objects {
            let local = self.local(object.id, &object.mesh)?;
            let world = if let Some(meta) = &object.metadata {
                transform_bounds(local, meta.affine.local_to_world)?
            } else {
                local
            };
            bounds = bounds.union(world);
        }
        Ok(bounds)
    }
}

/// Source initial root loose AABB, including collapsed-axis thickness. This is
/// not a reconstruction of its history-dependent expanding/rebuilding octree.
pub(crate) fn initial_loose_bounds(mesh: &Mesh) -> Result<SceneBounds, String> {
    if mesh.positions.len() > MAX_VERTICES {
        return Err("Camera object exceeds core vertex budget".into());
    }
    let Some(first) = mesh.positions.first() else {
        return Err("Source camera object has no positions".into());
    };
    let (mut low, mut high) = (first.as_dvec3(), first.as_dvec3());
    for p in &mesh.positions {
        if !p.is_finite() {
            return Err("Camera object position is nonfinite".into());
        }
        let p = p.as_dvec3();
        low = low.min(p);
        high = high.max(p);
    }
    let extent = high - low;
    let pad = extent.length() * 0.2;
    for axis in 0..3 {
        if extent[axis] < 1e-16 {
            low[axis] -= pad;
            high[axis] += pad;
        }
    }
    SceneBounds::checked(low, high)
}
fn transform_bounds(local: SceneBounds, matrix: [f32; 16]) -> Result<SceneBounds, String> {
    if matrix.iter().any(|v| !v.is_finite())
        || matrix[3] != 0.0
        || matrix[7] != 0.0
        || matrix[11] != 0.0
        || matrix[15] != 1.0
    {
        return Err("Camera object matrix is not a finite affine".into());
    }
    let mut low = DVec3::new(
        f64::from(matrix[12]),
        f64::from(matrix[13]),
        f64::from(matrix[14]),
    );
    let mut high = low;
    // Same per-component bounding operation as source computeWorldBound,
    // independent of transforming and re-rounding all eight corners.
    for axis in 0..3 {
        for component in 0..3 {
            let m = f64::from(matrix[axis * 4 + component]);
            let a = m * local.minimum[axis];
            let b = m * local.maximum[axis];
            low[component] += a.min(b);
            high[component] += a.max(b);
        }
    }
    SceneBounds::checked(low, high)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FramePose {
    pub target: DVec3,
    pub camera_to_world: DQuat,
    pub orthographic: bool,
    pub fov_degrees: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct SourceCameraController {
    zoom: ProjectionZoom,
    /// Source `_offset` in camera-local coordinates. A picked orbit pivot
    /// changes this together with the center so the visible view stays fixed.
    pivot_offset: DVec3,
    viewport: ViewportDimensions,
    clip: ClipRange,
    cache: BoundsCache,
    pub(crate) frame: Option<Arc<SourceCameraFrame>>,
    pub(crate) error: Option<String>,
    pub(crate) serial: u64,
    pub(crate) bounds: Option<SceneBounds>,
}
impl SourceCameraController {
    pub(crate) fn new(zoom: f64, viewport: ViewportDimensions) -> Result<Self, String> {
        Ok(Self {
            zoom: ProjectionZoom::checked(zoom)?,
            pivot_offset: DVec3::ZERO,
            viewport,
            clip: ClipRange::initial(),
            cache: BoundsCache::default(),
            frame: None,
            error: None,
            serial: 0,
            bounds: None,
        })
    }
    pub(crate) fn discard_bounds_cache(&mut self) {
        self.cache = BoundsCache::default();
    }
    pub(crate) fn zoom(&self) -> f64 {
        self.zoom.value()
    }
    pub(crate) fn pivot_offset(&self) -> DVec3 {
        self.pivot_offset
    }
    pub(crate) fn set_pivot_offset(&mut self, value: DVec3) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Source camera pivot offset is nonfinite".into());
        }
        self.pivot_offset = value;
        Ok(())
    }
    pub(crate) fn reset_pivot_offset(&mut self) {
        self.pivot_offset = DVec3::ZERO;
    }
    /// Retarget the orbit without changing the current perspective view.
    /// SculptGL updates `_center`, `_offset`, and perspective `_trans[2]` in
    /// `Camera.setPivot`; no document or GPU state is touched here.
    pub(crate) fn retarget_pivot(
        &mut self,
        center: DVec3,
        pivot: DVec3,
        camera_to_world: DQuat,
        fov_degrees: f64,
        orthographic: bool,
    ) -> Result<(), String> {
        if !center.is_finite()
            || !pivot.is_finite()
            || !camera_to_world.is_finite()
            || camera_to_world.length_squared() < 1e-20
            || !fov_degrees.is_finite()
            || !(0.0..180.0).contains(&fov_degrees)
            || fov_degrees == 0.0
        {
            return Err("Source camera pivot pose is invalid".into());
        }
        let rotation = camera_to_world.normalize();
        let old_depth = self.zoom.value() * 45.0 / fov_degrees;
        let old_local_eye = DVec3::new(
            -self.pivot_offset.x,
            -self.pivot_offset.y,
            if orthographic { 1000.0 } else { old_depth } - self.pivot_offset.z,
        );
        let old_eye = center + rotation * old_local_eye;
        let local_delta = rotation.conjugate() * (pivot - center);
        let mut offset = self.pivot_offset + local_delta;
        let mut zoom = self.zoom;
        if orthographic {
            // Source explicitly clears the depth offset for orthographic pivots.
            offset.z = 0.0;
        } else {
            let depth = old_eye.distance(pivot);
            zoom = ProjectionZoom::checked(depth * fov_degrees / 45.0)?;
            offset.z += depth - old_depth;
        }
        if !offset.is_finite() {
            return Err("Source camera pivot offset exceeds finite range".into());
        }
        self.zoom = zoom;
        self.pivot_offset = offset;
        Ok(())
    }
    pub(crate) fn viewport(&self) -> ViewportDimensions {
        self.viewport
    }
    pub(crate) fn set_navigation_viewport(&mut self, value: ViewportDimensions) {
        self.viewport = value;
    }
    pub(crate) fn set_zoom(&mut self, value: f64) -> Result<(), String> {
        self.zoom = ProjectionZoom::checked(value)?;
        Ok(())
    }
    pub(crate) fn view_height(&self) -> f64 {
        2.0 * f64::from(self.viewport.render_pixels[1]) * self.zoom.value() * 0.00055
    }
    pub(crate) fn set_view_height(&mut self, value: f64) -> Result<(), String> {
        self.set_zoom(value / (f64::from(self.viewport.render_pixels[1]) * 0.0011))
    }
    pub(crate) fn fit_bounds(
        &mut self,
        pose: FramePose,
        bounds: SceneBounds,
    ) -> Result<DVec3, String> {
        let bounds = SceneBounds::checked(bounds.minimum, bounds.maximum)?;
        let current = SourceCameraFrame::checked(
            SourcePose {
                rotation: WorldToView::from_camera_to_world(pose.camera_to_world)?,
                center: pose.target,
                plane_translation: DVec2::ZERO,
                offset_z: 0.0,
                zoom: self.zoom,
            },
            if pose.orthographic {
                ProjectionKind::Orthographic
            } else {
                ProjectionKind::Perspective
            },
            pose.fov_degrees,
            self.viewport,
            ClipInput::Retained(self.clip),
        )?;
        let near = self.clip.near_distance;
        let x = if pose.orthographic {
            f64::from(self.viewport.render_pixels[0].min(self.viewport.render_pixels[1])) / near
                * 0.5
        } else {
            // Source computes this from its Float32 projection, not an ideal
            // trigonometric frustum. Keep the original operation order.
            let p = current.gpu_projection().to_cols_array().map(f64::from);
            let left = near * (p[8] - 1.0) / p[0];
            let right = near * (1.0 + p[8]) / p[0];
            let top = near * (1.0 + p[9]) / p[5];
            let bottom = near * (p[9] - 1.0) / p[5];
            (right - left).abs().min((top - bottom).abs()) / near * 0.5
        };
        let [dx, dy, dz] = (bounds.maximum - bounds.minimum).to_array();
        let radius = 0.5 * (dx * dx + dy * dy + dz * dz).sqrt();
        let fit = (1.0 + x * x).sqrt() / x;
        let zoom = (0.8 * radius) * fit;
        let center = (bounds.minimum + bounds.maximum) * 0.5;
        if !center.is_finite() || !x.is_finite() || x <= 0.0 {
            return Err("Source camera frame has undefined bounds or frustum".into());
        }
        let zoom = ProjectionZoom::checked(zoom)?;
        self.zoom = zoom;
        self.error = None;
        Ok(center)
    }
    pub(crate) fn fit_scene(
        &mut self,
        pose: FramePose,
        objects: &[SubtoolRenderSnapshot],
    ) -> Result<Option<DVec3>, String> {
        let mut candidate = self.clone();
        let Some(bounds) = candidate.cache.meshes(objects)? else {
            return Ok(None);
        };
        let center = candidate.fit_bounds(pose, bounds)?;
        *self = candidate;
        Ok(Some(center))
    }
    pub(crate) fn prepare(
        &mut self,
        pose: FramePose,
        viewport: ViewportDimensions,
        objects: &[SubtoolRenderSnapshot],
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate.viewport = viewport;
        let clip = if objects.is_empty() {
            ClipInput::Retained(candidate.clip)
        } else {
            let bounds = candidate.cache.scene(objects)?;
            candidate.bounds = Some(bounds);
            ClipInput::Optimize(bounds)
        };
        let frame = SourceCameraFrame::checked(
            SourcePose {
                rotation: WorldToView::from_camera_to_world(pose.camera_to_world)?,
                center: pose.target,
                plane_translation: -candidate.pivot_offset.truncate(),
                offset_z: candidate.pivot_offset.z,
                zoom: candidate.zoom,
            },
            if pose.orthographic {
                ProjectionKind::Orthographic
            } else {
                ProjectionKind::Perspective
            },
            pose.fov_degrees,
            viewport,
            clip,
        )?;
        candidate.clip = frame.clip;
        candidate.frame = Some(Arc::new(frame));
        candidate.error = None;
        candidate.serial = candidate.serial.saturating_add(1);
        *self = candidate;
        Ok(())
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"policy":"sculptgl_static_v1","serial":self.serial,"projection_zoom":self.zoom.value(),"pivot_offset":self.pivot_offset.to_array(),"render_pixels":self.viewport.render_pixels,"user_points":self.viewport.user_points.to_array(),"cache_position_bytes":self.cache.bytes,"cache_limit_bytes":MAX_POSITION_CACHE_BYTES,"cache_transient_position_bytes_bound":MAX_POSITION_CACHE_TRANSIENT_BYTES,"cache_rows":self.cache.rows.len(),"cache_pointer_hits":self.cache.hits,"cache_unchanged_position_reuses":self.cache.position_reuses,"cache_scans":self.cache.scans,"clip_bounds":self.bounds.map(|b|[b.minimum.to_array(),b.maximum.to_array()]),"near":self.clip.near_distance,"far":self.clip.far_distance,"error":self.error,"frame":self.frame.as_ref().map(|f|serde_json::json!({"render_pixels":f.viewport.render_pixels,"user_points":f.viewport.user_points.to_array(),"eye64":f.eye64.to_array(),"view64":f.view64.to_cols_array(),"projection64":f.projection64.to_cols_array(),"projection_gpu":f.gpu_projection().to_cols_array(),"curvature_fov":f.curvature_fov()})),"picked_pivot_candidate":true,"scope":"Exact initial loose/static box provider plus hidden reference grid; picked-pivot orbit candidate; general source octree history/gizmos/animation/navigation gains unverified"})
    }
}

pub(crate) fn measured_viewport(
    ctx: &eframe::egui::Context,
    rect: eframe::egui::Rect,
) -> Result<ViewportDimensions, String> {
    let ppp = ctx.pixels_per_point();
    let screen = ctx.viewport_rect().size() * ppp;
    if !ppp.is_finite() || ppp <= 0.0 || !screen.is_finite() || screen.x <= 0.0 || screen.y <= 0.0 {
        return Err("Camera viewport measurement is invalid".into());
    }
    let info = eframe::egui::PaintCallbackInfo {
        viewport: rect,
        clip_rect: rect,
        pixels_per_point: ppp,
        screen_size_px: [screen.x.round() as u32, screen.y.round() as u32],
    };
    let px = info.viewport_in_pixels();
    if px.width_px <= 0 || px.height_px <= 0 {
        return Err("Camera viewport is empty".into());
    }
    ViewportDimensions::checked(
        [px.width_px as u32, px.height_px as u32],
        DVec2::new(f64::from(rect.width()), f64::from(rect.height())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mesh() -> Arc<Mesh> {
        Arc::new(
            Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3\nf 1 3 4\n").unwrap(),
        )
    }
    fn snapshot(id: u32, mesh: Arc<Mesh>) -> SubtoolRenderSnapshot {
        SubtoolRenderSnapshot {
            id,
            active: true,
            mesh,
            metadata: None,
        }
    }
    fn viewport() -> ViewportDimensions {
        ViewportDimensions::checked([1000, 500], DVec2::new(500.0, 250.0)).unwrap()
    }
    fn pose() -> FramePose {
        FramePose {
            target: DVec3::ZERO,
            camera_to_world: DQuat::IDENTITY,
            orthographic: true,
            fov_degrees: 45.0,
        }
    }

    #[test]
    fn collapsed_axis_uses_initial_loose_thickness() {
        let m = mesh();
        let bounds = initial_loose_bounds(&m).unwrap();
        assert_eq!(bounds.minimum.x, -1.0);
        assert_eq!(bounds.maximum.y, 1.0);
        let expected = 8.0f64.sqrt() * 0.2;
        assert_eq!(bounds.minimum.z, -expected);
        assert_eq!(bounds.maximum.z, expected);
    }
    #[test]
    fn affine_bounds_enclose_independent_corner_products() {
        let local =
            SceneBounds::checked(DVec3::new(-2.0, -1.0, -3.0), DVec3::new(4.0, 2.0, 1.0)).unwrap();
        let matrix = [
            -2.0, 0.5, 0.0, 0.0, 0.25, 3.0, 0.0, 0.0, 0.0, -0.5, 0.75, 0.0, 9.0, -4.0, 2.0, 1.0,
        ];
        let actual = transform_bounds(local, matrix).unwrap();
        let mut low = DVec3::splat(f64::INFINITY);
        let mut high = DVec3::splat(f64::NEG_INFINITY);
        for x in [-2.0, 4.0] {
            for y in [-1.0, 2.0] {
                for z in [-3.0, 1.0] {
                    let point = DVec3::new(
                        -2.0 * x + 0.25 * y + 9.0,
                        0.5 * x + 3.0 * y - 0.5 * z - 4.0,
                        0.75 * z + 2.0,
                    );
                    low = low.min(point);
                    high = high.max(point);
                }
            }
        }
        assert_eq!(actual.minimum, low);
        assert_eq!(actual.maximum, high);
    }
    #[test]
    fn nonfinite_and_projective_bounds_reject() {
        let b = SceneBounds::checked(DVec3::ZERO, DVec3::ONE).unwrap();
        let mut m = glam::Mat4::IDENTITY.to_cols_array();
        m[3] = 0.1;
        assert!(transform_bounds(b, m).is_err());
        m[3] = 0.0;
        m[12] = f32::INFINITY;
        assert!(transform_bounds(b, m).is_err());
        let mut bad = (*mesh()).clone();
        bad.positions[0].x = f32::NAN;
        assert!(initial_loose_bounds(&bad).is_err());
    }
    #[test]
    fn repeated_pointer_reuses_one_cache_row() {
        let m = mesh();
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        let objects = [snapshot(7, m.clone())];
        c.prepare(pose(), viewport(), &objects).unwrap();
        c.prepare(pose(), viewport(), &objects).unwrap();
        assert_eq!(c.cache.rows.len(), 1);
        assert_eq!(c.cache.scans, 1);
        assert_eq!(c.cache.hits, 1);
        assert_eq!(Arc::strong_count(&m), 2);
        assert_eq!(c.cache.bytes, 4 * std::mem::size_of::<glam::Vec3>());
    }
    #[test]
    fn color_only_new_snapshot_reuses_bounds_without_retaining_mesh() {
        let m = mesh();
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, m.clone())])
            .unwrap();
        let mut changed = (*m).clone();
        changed.colors[0] = [0.25, 0.5, 0.75];
        let changed = Arc::new(changed);
        c.prepare(pose(), viewport(), &[snapshot(7, changed.clone())])
            .unwrap();
        assert_eq!(c.cache.scans, 1);
        assert_eq!(c.cache.position_reuses, 1);
        assert_eq!(Arc::strong_count(&m), 1);
        assert_eq!(Arc::strong_count(&changed), 1);
    }
    #[test]
    fn geometry_change_rescans_but_old_published_frame_is_immutable() {
        let m = mesh();
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, m.clone())])
            .unwrap();
        let accepted = c.frame.clone().unwrap();
        let accepted_matrix = accepted.projection64.to_cols_array();
        let mut changed = (*m).clone();
        changed.positions[1].x = 300.0;
        c.prepare(pose(), viewport(), &[snapshot(7, Arc::new(changed))])
            .unwrap();
        assert_eq!(c.cache.scans, 2);
        assert_eq!(accepted.projection64.to_cols_array(), accepted_matrix);
        assert_ne!(
            c.frame.as_ref().unwrap().clip.far_distance,
            accepted.clip.far_distance
        );
    }
    #[test]
    fn old_ids_are_pruned_and_callback_cache_can_be_dropped() {
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, mesh())])
            .unwrap();
        c.prepare(pose(), viewport(), &[snapshot(8, mesh())])
            .unwrap();
        assert!(!c.cache.rows.contains_key(&7));
        assert_eq!(c.cache.rows.len(), 1);
        let mut callback = c.clone();
        callback.discard_bounds_cache();
        assert_eq!(callback.cache.bytes, 0);
        assert!(callback.cache.rows.is_empty());
        assert!(Arc::ptr_eq(
            callback.frame.as_ref().unwrap(),
            c.frame.as_ref().unwrap()
        ));
        assert_eq!(c.cache.rows.len(), 1);
    }
    #[test]
    fn budget_exhaustion_computes_without_adding_a_row() {
        // Admission is checked before an allocation. The private byte counter
        // simulates a full cache without allocating a 64 MiB test fixture.
        let mut c = BoundsCache {
            bytes: MAX_POSITION_CACHE_BYTES,
            ..Default::default()
        };
        let bounds = c.local(7, &mesh()).unwrap();
        assert!(bounds.minimum.is_finite());
        assert!(c.rows.is_empty());
        assert_eq!(c.bytes, MAX_POSITION_CACHE_BYTES);
        assert_eq!(MAX_POSITION_CACHE_TRANSIENT_BYTES, 128 * 1024 * 1024);
    }
    #[test]
    fn excessive_object_count_rejects_atomically() {
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, mesh())])
            .unwrap();
        let before = c.diagnostics();
        let m = mesh();
        let objects: Vec<_> = (0..=MAX_SUBTOOLS)
            .map(|i| snapshot(i as u32, m.clone()))
            .collect();
        assert!(c.prepare(pose(), viewport(), &objects).is_err());
        assert_eq!(c.diagnostics(), before);
    }
    #[test]
    fn empty_scene_retains_clip_and_does_not_add_grid_only_reoptimization() {
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, mesh())])
            .unwrap();
        let clip = c.clip;
        let mut moved = pose();
        moved.target = DVec3::new(100_000.0, 0.0, 0.0);
        c.prepare(moved, viewport(), &[]).unwrap();
        assert_eq!(c.clip.near_distance, clip.near_distance);
        assert_eq!(c.clip.far_distance, clip.far_distance);
    }
    #[test]
    fn invalid_scale_and_pose_preserve_prior_accepted_state() {
        let mut c = SourceCameraController::new(4.0, viewport()).unwrap();
        c.prepare(pose(), viewport(), &[snapshot(7, mesh())])
            .unwrap();
        let before = c.diagnostics();
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(c.set_view_height(value).is_err());
            assert_eq!(c.diagnostics(), before);
        }
        let mut bad = pose();
        bad.fov_degrees = 180.0;
        assert!(c.prepare(bad, viewport(), &[]).is_err());
        assert_eq!(c.diagnostics(), before);
    }
    #[test]
    fn actual_pixel_rounding_is_distinct_from_logical_extent() {
        let ctx = eframe::egui::Context::default();
        ctx.set_pixels_per_point(2.0);
        let rect = eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(123.25, 51.25),
            eframe::egui::pos2(443.375, 251.375),
        );
        let raw = eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::vec2(1024.0, 768.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            let measured = measured_viewport(ctx, rect).unwrap();
            assert_eq!(measured.render_pixels, [640, 400]);
            assert_eq!(measured.user_points, DVec2::new(320.125, 200.125));
            assert_ne!(measured.pixels_per_user_point().x, 2.0);
        });
    }
}
