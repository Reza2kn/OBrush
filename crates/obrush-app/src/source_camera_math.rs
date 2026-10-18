//! Optional display-free source camera frame; host integration is staged only.
//! Primary provenance and consumer gates are retained by the source stage.
//! Legacy Camera methods must remain untouched when this policy is absent.
use glam::{DMat3, DMat4, DQuat, DVec2, DVec3, DVec4, Mat4};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionKind {
    Perspective,
    Orthographic,
}

/// Actual render-buffer pixels and actual user-space points are distinct units.
#[derive(Clone, Copy, Debug)]
pub struct ViewportDimensions {
    pub render_pixels: [u32; 2],
    pub user_points: DVec2,
}
impl ViewportDimensions {
    pub fn checked(render_pixels: [u32; 2], user_points: DVec2) -> Result<Self, String> {
        if render_pixels.contains(&0)
            || !user_points.is_finite()
            || user_points.min_element() <= 0.0
        {
            return Err("Camera viewport has invalid render/user dimensions".into());
        }
        Ok(Self {
            render_pixels,
            user_points,
        })
    }
    pub fn aspect(self) -> f64 {
        f64::from(self.render_pixels[0]) / f64::from(self.render_pixels[1])
    }
    pub fn pixels_per_user_point(self) -> DVec2 {
        DVec2::new(
            f64::from(self.render_pixels[0]),
            f64::from(self.render_pixels[1]),
        ) / self.user_points
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SceneBounds {
    pub minimum: DVec3,
    pub maximum: DVec3,
}
impl SceneBounds {
    pub fn checked(minimum: DVec3, maximum: DVec3) -> Result<Self, String> {
        if !minimum.is_finite() || !maximum.is_finite() || minimum.cmpgt(maximum).any() {
            return Err("Camera clip bounds are nonfinite or reversed".into());
        }
        Ok(Self { minimum, maximum })
    }
    pub fn union(self, other: Self) -> Self {
        Self {
            minimum: self.minimum.min(other.minimum),
            maximum: self.maximum.max(other.maximum),
        }
    }
    pub fn center(self) -> DVec3 {
        self.minimum * 0.5 + self.maximum * 0.5
    }
    pub fn radius(self) -> f64 {
        (self.maximum - self.minimum).length() * 0.5
    }
}

/// Positive source `_trans[2]`, not the orthographic eye distance.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionZoom(f64);
impl ProjectionZoom {
    pub fn checked(value: f64) -> Result<Self, String> {
        if !value.is_finite() || value <= 0.0 {
            return Err("Source camera zoom must be finite and positive".into());
        }
        Ok(Self(value))
    }
    pub fn value(self) -> f64 {
        self.0
    }
}

/// The source quaternion rotates WORLD into VIEW, unlike OBrush's camera-to-world orientation.
#[derive(Clone, Copy, Debug)]
pub struct WorldToView(DQuat);
impl WorldToView {
    pub fn checked(value: DQuat) -> Result<Self, String> {
        if !value.is_finite() || value.length_squared() <= 1e-20 {
            return Err("Source camera rotation is invalid".into());
        }
        Ok(Self(value.normalize()))
    }
    pub fn from_camera_to_world(value: DQuat) -> Result<Self, String> {
        Self::checked(value.conjugate())
    }
    pub fn rotation(self) -> DQuat {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SourcePose {
    pub rotation: WorldToView,
    pub center: DVec3,
    /// Exact source `trans.xy-offset.xy`, not a world-space target translation.
    pub plane_translation: DVec2,
    pub offset_z: f64,
    pub zoom: ProjectionZoom,
}
#[derive(Clone, Copy, Debug)]
pub struct ClipRange {
    pub near_distance: f64,
    pub far_distance: f64,
}
impl ClipRange {
    pub fn initial() -> Self {
        Self {
            near_distance: 0.05,
            far_distance: 5000.0,
        }
    }
    pub fn from_bounds(eye: DVec3, bounds: SceneBounds) -> Result<Self, String> {
        let bounds = SceneBounds::checked(bounds.minimum, bounds.maximum)?;
        let distance = eye.distance(bounds.center());
        let radius = bounds.radius();
        let value = Self {
            near_distance: (distance - radius).max(0.01),
            far_distance: distance + radius,
        };
        if !value.near_distance.is_finite()
            || !value.far_distance.is_finite()
            || value.far_distance <= 0.0
        {
            return Err("Source camera clip range exceeds finite coordinates".into());
        }
        Ok(value)
    }
}

/// Empty scenes retain the previous clip state; only a fresh source camera
/// starts at.05/5000. Never infer these defaults from absent SGLv1 fields.
#[derive(Clone, Copy, Debug)]
pub enum ClipInput {
    Initial,
    Retained(ClipRange),
    Optimize(SceneBounds),
}

#[derive(Clone, Debug)]
pub struct SourceCameraFrame {
    pub projection_kind: ProjectionKind,
    pub viewport: ViewportDimensions,
    pub view64: DMat4,
    pub projection64: DMat4,
    pub eye64: DVec3,
    pub clip: ClipRange,
    pub vertical_fov_degrees: f64,
    pub projection_zoom: ProjectionZoom,
}
impl SourceCameraFrame {
    pub fn checked(
        pose: SourcePose,
        projection_kind: ProjectionKind,
        vertical_fov_degrees: f64,
        viewport: ViewportDimensions,
        clip_input: ClipInput,
    ) -> Result<Self, String> {
        let viewport = ViewportDimensions::checked(viewport.render_pixels, viewport.user_points)?;
        if !pose.center.is_finite()
            || !pose.plane_translation.is_finite()
            || !pose.offset_z.is_finite()
            || !vertical_fov_degrees.is_finite()
            || vertical_fov_degrees <= 0.0
            || vertical_fov_degrees >= 180.0
        {
            return Err("Source camera pose/FOV is invalid".into());
        }
        let z = match projection_kind {
            ProjectionKind::Perspective => pose.zoom.value() * 45.0 / vertical_fov_degrees,
            ProjectionKind::Orthographic => 1000.0,
        };
        let local_eye = DVec3::new(
            pose.plane_translation.x,
            pose.plane_translation.y,
            z - pose.offset_z,
        );
        let eye64 = pose.center + pose.rotation.rotation().conjugate() * local_eye;
        // Source lookAt for its axis-aligned local eye/target is this translation
        // followed by world-to-view rotation and center translation.
        let view64 = DMat4::from_translation(-local_eye)
            * DMat4::from_quat(pose.rotation.rotation())
            * DMat4::from_translation(-pose.center);
        let clip = match clip_input {
            ClipInput::Initial => ClipRange::initial(),
            ClipInput::Optimize(bounds) => ClipRange::from_bounds(eye64, bounds)?,
            ClipInput::Retained(value) => value,
        };
        if !clip.near_distance.is_finite()
            || !clip.far_distance.is_finite()
            || clip.near_distance <= 0.0
            || clip.far_distance <= 0.0
        {
            return Err("Retained source clipping is invalid".into());
        }
        let projection64 = match projection_kind {
            ProjectionKind::Perspective => {
                // Direct source infinite-far matrix. Do not construct a singular
                // temporary finite matrix when far==near, then patch its entries.
                let fy = 1.0 / (vertical_fov_degrees.to_radians() * 0.5).tan();
                DMat4::from_cols(
                    DVec4::new(fy / viewport.aspect(), 0.0, 0.0, 0.0),
                    DVec4::new(0.0, fy, 0.0, 0.0),
                    DVec4::new(0.0, 0.0, -1.0, -1.0),
                    DVec4::new(0.0, 0.0, -2.0 * clip.near_distance, 0.0),
                )
            }
            ProjectionKind::Orthographic => {
                let scale = pose.zoom.value() * 0.00055;
                let w = f64::from(viewport.render_pixels[0]) * scale;
                let h = f64::from(viewport.render_pixels[1]) * scale;
                // Source passes NEGATIVE optimized near to gl-matrix. This is
                // intentional: near/far must surround eye1000, not legacy distance.
                DMat4::orthographic_rh_gl(-w, w, -h, h, -clip.near_distance, clip.far_distance)
            }
        };
        if !eye64.is_finite()
            || !view64.is_finite()
            || !projection64.is_finite()
            || !view64.as_mat4().is_finite()
            || !projection64.as_mat4().is_finite()
            || projection64.as_mat4().determinant() == 0.0
        {
            return Err("Source camera frame is not representable by the GPU projection".into());
        }
        Ok(Self {
            projection_kind,
            viewport,
            view64,
            projection64,
            eye64,
            clip,
            vertical_fov_degrees,
            projection_zoom: pose.zoom,
        })
    }
    pub fn gpu_projection(&self) -> Mat4 {
        self.projection64.as_mat4()
    }
    pub fn curvature_fov(&self) -> f64 {
        match self.projection_kind {
            ProjectionKind::Perspective => self.vertical_fov_degrees,
            ProjectionKind::Orthographic => -self.projection_zoom.value().abs() * 25.0,
        }
    }
    /// Source radius changes use horizontal screen displacement at hit depth.
    pub fn horizontal_world_units_per_user_point(&self, view_depth: f64) -> Result<f64, String> {
        let scale = self.viewport.pixels_per_user_point().x;
        let per_device_pixel = match self.projection_kind {
            ProjectionKind::Orthographic => self.projection_zoom.value() * 0.0011,
            ProjectionKind::Perspective => {
                2.0 * view_depth * (self.vertical_fov_degrees.to_radians() * 0.5).tan()
                    / f64::from(self.viewport.render_pixels[1])
            }
        };
        let result = per_device_pixel * scale;
        if !result.is_finite() || result <= 0.0 {
            return Err("Invalid source screen footprint depth".into());
        }
        Ok(result)
    }
    pub fn project_device(&self, world: DVec3) -> Result<DVec3, String> {
        let clip = self.projection64 * self.view64 * world.extend(1.0);
        if !clip.is_finite() || clip.w.abs() <= 1e-20 {
            return Err("Source projection has undefined homogeneous position".into());
        }
        let ndc = clip.truncate() / clip.w;
        Ok(DVec3::new(
            (ndc.x + 1.0) * 0.5 * f64::from(self.viewport.render_pixels[0]),
            (1.0 - ndc.y) * 0.5 * f64::from(self.viewport.render_pixels[1]),
            (ndc.z + 1.0) * 0.5,
        ))
    }
    pub fn unproject_device(&self, screen: DVec3) -> Result<DVec3, String> {
        let ndc = DVec3::new(
            2.0 * screen.x / f64::from(self.viewport.render_pixels[0]) - 1.0,
            1.0 - 2.0 * screen.y / f64::from(self.viewport.render_pixels[1]),
            2.0 * screen.z - 1.0,
        );
        let inverse = (self.projection64 * self.view64).inverse();
        let point = inverse * ndc.extend(1.0);
        if !point.is_finite() || point.w.abs() <= 1e-20 {
            return Err("Source unprojection is undefined".into());
        }
        Ok(point.truncate() / point.w)
    }
    /// Mathematically the source depth0/.1 ray, constructed analytically to
    /// avoid subtracting nearly coincident world points at large affine offsets.
    /// Its near origin still honors the source SIGNED orthographic clip plane.
    pub fn ray_device(&self, screen: DVec2) -> Result<(DVec3, DVec3), String> {
        if !screen.is_finite() {
            return Err("Camera screen coordinates are nonfinite".into());
        }
        let ndc = DVec2::new(
            2.0 * screen.x / f64::from(self.viewport.render_pixels[0]) - 1.0,
            1.0 - 2.0 * screen.y / f64::from(self.viewport.render_pixels[1]),
        );
        let camera_to_world = DMat3::from_mat4(self.view64).transpose();
        let (local_origin, local_direction) = match self.projection_kind {
            ProjectionKind::Perspective => {
                let tangent = (self.vertical_fov_degrees.to_radians() * 0.5).tan();
                let direction = DVec3::new(
                    ndc.x * tangent * self.viewport.aspect(),
                    ndc.y * tangent,
                    -1.0,
                );
                (direction * self.clip.near_distance, direction.normalize())
            }
            ProjectionKind::Orthographic => {
                let scale = self.projection_zoom.value() * 0.00055;
                (
                    DVec3::new(
                        ndc.x * f64::from(self.viewport.render_pixels[0]) * scale,
                        ndc.y * f64::from(self.viewport.render_pixels[1]) * scale,
                        self.clip.near_distance,
                    ),
                    -DVec3::Z,
                )
            }
        };
        let origin = self.eye64 + camera_to_world * local_origin;
        let direction = (camera_to_world * local_direction).normalize();
        if !origin.is_finite() || !direction.is_finite() {
            return Err("Source camera ray is singular".into());
        }
        Ok((origin, direction))
    }
}

/// This source grid affects optimizeNearFar even when draw-grid is off. The
/// three casts independently model source Float32 matrix publication steps.
/// Gizmo/preview bounds must be supplied separately by the host clip provider.
pub fn source_grid_clip_bounds() -> SceneBounds {
    let loose_y_half = 2.0f64.sqrt() * 0.2;
    let loose_diagonal = (2.0 + (2.0 * loose_y_half).powi(2)).sqrt();
    let normalized = (100.0 / loose_diagonal) as f32;
    let translated_y = (f64::from(normalized) * -0.45) as f32;
    let scaled = (f64::from(normalized) * 2.5) as f32;
    let half = f64::from(scaled) * 0.5;
    let y_half = f64::from(scaled) * loose_y_half;
    SceneBounds {
        minimum: DVec3::new(-half, f64::from(translated_y) - y_half, -half),
        maximum: DVec3::new(half, f64::from(translated_y) + y_half, half),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(kind: ProjectionKind, zoom: f64) -> SourceCameraFrame {
        SourceCameraFrame::checked(
            SourcePose {
                rotation: WorldToView::checked(DQuat::IDENTITY).unwrap(),
                center: DVec3::ZERO,
                plane_translation: DVec2::ZERO,
                offset_z: 0.0,
                zoom: ProjectionZoom::checked(zoom).unwrap(),
            },
            kind,
            45.0,
            ViewportDimensions::checked([1000, 500], DVec2::new(1000.0, 500.0)).unwrap(),
            ClipInput::Optimize(
                SceneBounds::checked(DVec3::splat(-1.0), DVec3::splat(1.0)).unwrap(),
            ),
        )
        .unwrap()
    }
    #[test]
    fn ortho_zoom_is_separate_from_eye() {
        let a = frame(ProjectionKind::Orthographic, 2.0);
        let b = frame(ProjectionKind::Orthographic, 4.0);
        assert_eq!(a.eye64, DVec3::new(0.0, 0.0, 1000.0));
        assert_eq!(a.view64, b.view64);
        assert!((a.projection64.x_axis.x - 1.0 / 1.1).abs() < 1e-12);
        assert!((b.projection64.x_axis.x - a.projection64.x_axis.x * 0.5).abs() < 1e-12);
    }
    #[test]
    fn optimized_signed_ortho_clip_contains_surface() {
        let a = frame(ProjectionKind::Orthographic, 2.0);
        let p = a.project_device(DVec3::ZERO).unwrap();
        assert!(p.z > 0.0 && p.z < 1.0);
        assert!((a.clip.far_distance - (1000.0 + 3.0f64.sqrt())).abs() < 1e-12);
    }
    #[test]
    fn perspective_source_matrix_is_infinite_far() {
        let a = frame(ProjectionKind::Perspective, 4.0);
        assert_eq!(a.eye64.z, 4.0);
        assert_eq!(a.projection64.z_axis.z, -1.0);
        assert_eq!(a.projection64.z_axis.w, -1.0);
        assert_eq!(a.projection64.w_axis.z, -2.0 * a.clip.near_distance);
    }
    #[test]
    fn source_ray_has_finite_near_and_point1_endpoints() {
        for k in [ProjectionKind::Perspective, ProjectionKind::Orthographic] {
            let a = frame(k, 4.0);
            let (_, d) = a.ray_device(DVec2::new(500.0, 250.0)).unwrap();
            assert!((d + DVec3::Z).length() < 1e-12);
        }
    }
    #[test]
    fn project_unproject_round_trip() {
        for k in [ProjectionKind::Perspective, ProjectionKind::Orthographic] {
            let a = frame(k, 4.0);
            let p = DVec3::new(0.125, -0.25, 0.5);
            let q = a.unproject_device(a.project_device(p).unwrap()).unwrap();
            assert!((p - q).length() < 1e-9);
        }
    }
    #[test]
    fn explicit_user_units_are_not_assumed_device_pixels() {
        let a = frame(ProjectionKind::Orthographic, 4.0);
        assert_eq!(
            a.horizontal_world_units_per_user_point(1000.0).unwrap(),
            0.0044
        );
        let mut b = a.clone();
        b.viewport = ViewportDimensions::checked([1000, 500], DVec2::new(500.0, 250.0)).unwrap();
        assert_eq!(
            b.horizontal_world_units_per_user_point(1000.0).unwrap(),
            0.0088
        );
    }
    #[test]
    fn curvature_uses_zoom_not_fixed_eye() {
        let a = frame(ProjectionKind::Orthographic, 4.0);
        assert_eq!(a.curvature_fov(), -100.0);
        assert_eq!(
            frame(ProjectionKind::Perspective, 4.0).curvature_fov(),
            45.0
        );
    }
    #[test]
    fn invalid_pose_inputs_reject() {
        assert!(ProjectionZoom::checked(0.0).is_err());
        assert!(ProjectionZoom::checked(f64::NAN).is_err());
        assert!(ViewportDimensions::checked([0, 500], DVec2::ONE).is_err());
        assert!(WorldToView::checked(DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0)).is_err());
        let bad = SourcePose {
            rotation: WorldToView::checked(DQuat::IDENTITY).unwrap(),
            center: DVec3::splat(1e300),
            plane_translation: DVec2::ZERO,
            offset_z: 0.0,
            zoom: ProjectionZoom::checked(4.0).unwrap(),
        };
        assert!(
            SourceCameraFrame::checked(
                bad,
                ProjectionKind::Orthographic,
                45.0,
                ViewportDimensions::checked([1000, 500], DVec2::new(1000.0, 500.0)).unwrap(),
                ClipInput::Initial
            )
            .is_err()
        );
    }
    #[test]
    fn world_to_view_direction_is_explicit() {
        let q = WorldToView::checked(DQuat::from_rotation_y(-std::f64::consts::FRAC_PI_2)).unwrap();
        let pose = SourcePose {
            rotation: q,
            center: DVec3::ZERO,
            plane_translation: DVec2::ZERO,
            offset_z: 0.0,
            zoom: ProjectionZoom::checked(4.0).unwrap(),
        };
        let a = SourceCameraFrame::checked(
            pose,
            ProjectionKind::Orthographic,
            45.0,
            ViewportDimensions::checked([1000, 500], DVec2::new(1000.0, 500.0)).unwrap(),
            ClipInput::Initial,
        )
        .unwrap();
        assert!((a.eye64 - DVec3::new(1000.0, 0.0, 0.0)).length() < 1e-9);
    }
    #[test]
    fn source_grid_is_clip_input_even_when_not_drawn() {
        let grid = source_grid_clip_bounds();
        assert!(
            grid.minimum.x < -82.0
                && grid.maximum.x > 82.0
                && grid.minimum.y < -75.0
                && grid.maximum.y > 16.0
        );
        let mesh = SceneBounds::checked(DVec3::splat(-1.0), DVec3::splat(1.0)).unwrap();
        let bounds = grid.union(mesh);
        let clip = ClipRange::from_bounds(DVec3::new(0.0, 0.0, 1000.0), bounds).unwrap();
        assert!(clip.near_distance < 900.0 && clip.far_distance > 1100.0);
    }
    #[test]
    fn analytical_ray_matches_finite_unproject_direction() {
        for k in [ProjectionKind::Perspective, ProjectionKind::Orthographic] {
            let a = frame(k, 4.0);
            let s = DVec2::new(610.0, 171.0);
            let (near, d) = a.ray_device(s).unwrap();
            let n = a.unproject_device(s.extend(0.0)).unwrap();
            let f = a.unproject_device(s.extend(0.1)).unwrap();
            assert!((near - n).length() < 1e-9);
            assert!((d - (f - n).normalize()).length() < 1e-9);
        }
    }
    #[test]
    fn empty_scene_retains_prior_clip_context() {
        let old = ClipRange {
            near_distance: 123.0,
            far_distance: 2456.0,
        };
        let pose = SourcePose {
            rotation: WorldToView::checked(DQuat::IDENTITY).unwrap(),
            center: DVec3::ZERO,
            plane_translation: DVec2::ZERO,
            offset_z: 0.0,
            zoom: ProjectionZoom::checked(4.0).unwrap(),
        };
        let a = SourceCameraFrame::checked(
            pose,
            ProjectionKind::Orthographic,
            45.0,
            ViewportDimensions::checked([1000, 500], DVec2::new(1000.0, 500.0)).unwrap(),
            ClipInput::Retained(old),
        )
        .unwrap();
        assert_eq!(a.clip.near_distance, 123.0);
        assert_eq!(a.clip.far_distance, 2456.0);
    }
}
