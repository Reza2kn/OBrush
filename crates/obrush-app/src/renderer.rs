//! Shared desktop OpenGL / browser WebGL2 sculpt viewport.
//!
//! The viewport uses an egui glow paint callback. Callers must request a depth
//! buffer in both `eframe::NativeOptions` and `eframe::WebOptions`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use crate::environment_assets::{EnvironmentLibrary, default_uv_texture};
use crate::environment_gl::{EnvironmentStyle, EnvironmentUniforms, LinearPipeline};
use crate::environment_glsl;
use crate::source_mask_view::source_mask_words;
use crate::uv_view::{
    MAX_CORRESPONDENCE_DIAGNOSTIC_ROWS, PackedUvMesh, ViewTexture, correspondence_fingerprint,
    texture_gpu_storage_bytes, uv_storage_upper_bound,
};
use eframe::{egui, glow};
use glam::{DMat4, DVec3, Mat3, Mat4, Quat, Vec2, Vec3};
use glow::HasContext;
use obrush_core::{
    Mesh, ObjectRenderTransform, Ray, SceneObjectMetadata, SourceShaderRole, SubtoolRenderSnapshot,
    baseline_source_render_defaults, resolve_source_appearance,
};

#[path = "selected_normal_certificates.rs"]
mod selected_normal_certificates;

use selected_normal_certificates::{SelectedNormalCertificates, SelectedNormalSeal};

const FIELD_OF_VIEW: f32 = 40.0_f32.to_radians();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    Orbit,
    Spherical,
    Plane,
}

impl CameraMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Orbit => "Orbit",
            Self::Spherical => "Spherical",
            Self::Plane => "Plane",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    /// Imported world centers can lie between f32 coordinates. Only the affine
    /// camera path consumes this retained value; ordinary legacy math stays exact.
    pub(crate) precise_target: Option<DVec3>,
    pub orthographic: bool,
    pub fov_degrees: f32,
    pub(crate) source_vertical_fov: bool,
    /// Raw URL parseFloat value; None preserves all preexisting legacy math.
    pub(crate) boot_source_fov: Option<f64>,
    pub mode: CameraMode,
    pub speed: f32,
    pub use_pivot: bool,
    pub(crate) rotation: Option<Quat>,
    pub(crate) source_camera: Option<crate::source_camera_host::SourceCameraController>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.35,
            pitch: 0.18,
            distance: 4.0,
            target: Vec3::ZERO,
            precise_target: None,
            orthographic: false,
            fov_degrees: FIELD_OF_VIEW.to_degrees(),
            source_vertical_fov: false,
            boot_source_fov: None,
            mode: CameraMode::Orbit,
            speed: 1.0,
            use_pivot: false,
            rotation: None,
            source_camera: None,
        }
    }
}

impl Camera {
    pub fn position(&self) -> Vec3 {
        if let Some(frame) = self.source_frame() {
            return frame.eye64.as_vec3();
        }
        if let Some(rotation) = self.rotation {
            return self.target + rotation * Vec3::Z * self.distance.max(0.01);
        }
        let offset = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        self.target + offset * self.distance.max(0.01)
    }

    pub fn position64(&self) -> DVec3 {
        if let Some(frame) = self.source_frame() {
            return frame.eye64;
        }
        let target = self
            .precise_target
            .unwrap_or_else(|| self.target.as_dvec3());
        let offset = if let Some(rotation) = self.rotation {
            (rotation * Vec3::Z).as_dvec3()
        } else {
            Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            )
            .as_dvec3()
        };
        target + offset * f64::from(self.distance.max(0.01))
    }
    pub fn view64(&self) -> DMat4 {
        if let Some(frame) = self.source_frame() {
            return frame.view64;
        }
        DMat4::look_at_rh(
            self.position64(),
            self.precise_target
                .unwrap_or_else(|| self.target.as_dvec3()),
            self.up_axis().as_dvec3(),
        )
    }
    pub fn axes64(&self) -> (DVec3, DVec3, DVec3) {
        if let Some(frame) = self.source_frame() {
            let basis = glam::DMat3::from_mat4(frame.view64).transpose();
            return (basis.x_axis, basis.y_axis, -basis.z_axis);
        }
        let forward = (self
            .precise_target
            .unwrap_or_else(|| self.target.as_dvec3())
            - self.position64())
        .normalize();
        let right = forward.cross(self.up_axis().as_dvec3()).normalize();
        let up = right.cross(forward).normalize();
        (right, up, forward)
    }
    pub fn ray64(&self, ndc: Vec2, aspect: f32) -> obrush_core::SceneRay64 {
        if let Some(frame) = self.source_frame() {
            let screen = glam::DVec2::new(
                (f64::from(ndc.x) + 1.0) * 0.5 * f64::from(frame.viewport.render_pixels[0]),
                (1.0 - f64::from(ndc.y)) * 0.5 * f64::from(frame.viewport.render_pixels[1]),
            );
            return match frame.ray_device(screen) {
                Ok((origin, direction)) => obrush_core::SceneRay64 { origin, direction },
                Err(_) => obrush_core::SceneRay64 {
                    origin: DVec3::splat(f64::NAN),
                    direction: DVec3::ZERO,
                },
            };
        }
        let (right, up, forward) = self.axes64();
        let tangent = f64::from((self.vertical_fov(aspect) * 0.5).tan());
        let offset = right * f64::from(ndc.x) * tangent * f64::from(aspect.max(0.001))
            + up * f64::from(ndc.y) * tangent;
        if self.orthographic {
            obrush_core::SceneRay64 {
                origin: self.position64() + offset * f64::from(self.distance),
                direction: forward,
            }
        } else {
            obrush_core::SceneRay64 {
                origin: self.position64(),
                direction: (forward + offset).normalize(),
            }
        }
    }
    /// Bound a resident GPU pick to the published studio clip frame. Source
    /// orthographic cameras use a signed near plane, so a distance based only
    /// on orbit radius can stop the ray before it reaches the visible mesh.
    pub(crate) fn resident_pick_distance(
        &self,
        ray: obrush_core::SceneRay64,
    ) -> Result<f64, String> {
        ray.validate()?;
        let reach = if let Some(frame) = self.source_frame() {
            let origin_z = frame.view64.transform_point3(ray.origin).z;
            let direction_z = frame.view64.transform_vector3(ray.direction).z;
            if !origin_z.is_finite() || !direction_z.is_finite() || direction_z >= -1e-6 {
                return Err("Resident source pick ray is outside the camera clip frame".into());
            }
            (origin_z + frame.clip.far_distance) / -direction_z + 1.0
        } else {
            f64::from(self.distance) * 100.0
        };
        if !reach.is_finite() || reach <= 0.0 {
            return Err("Resident pick span is outside the bounded camera frame".into());
        }
        Ok(reach.clamp(100.0, 2_000_000.0))
    }
    pub fn projection(&self, aspect: f32) -> Mat4 {
        if let Some(frame) = self.source_frame() {
            return frame.gpu_projection();
        }
        let near = (self.distance * 0.001).max(0.0001);
        let far = (self.distance * 100.0).max(100.0);
        let aspect = aspect.max(0.001);
        if self.orthographic {
            let half_height = self.distance * (self.vertical_fov(aspect) * 0.5).tan();
            let half_width = half_height * aspect;
            Mat4::orthographic_rh_gl(
                -half_width,
                half_width,
                -half_height,
                half_height,
                near,
                far,
            )
        } else {
            Mat4::perspective_rh_gl(self.vertical_fov(aspect), aspect, near, far)
        }
    }

    pub fn view(&self) -> Mat4 {
        if let Some(frame) = self.source_frame() {
            return frame.view64.as_mat4();
        }
        Mat4::look_at_rh(self.position(), self.target, self.up_axis())
    }

    fn orientation(&self) -> Quat {
        self.rotation
            .unwrap_or_else(|| Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch))
    }

    fn up_axis(&self) -> Vec3 {
        self.rotation.map_or(Vec3::Y, |rotation| rotation * Vec3::Y)
    }

    pub fn axes(&self) -> (Vec3, Vec3, Vec3) {
        if self.source_frame().is_some() {
            let (right, up, forward) = self.axes64();
            return (right.as_vec3(), up.as_vec3(), forward.as_vec3());
        }
        let forward = (self.target - self.position()).normalize_or_zero();
        let right = forward.cross(self.up_axis()).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        (right, up, forward)
    }

    fn base_fov(&self) -> f32 {
        if (self.source_vertical_fov || self.source_camera.is_some())
            && self.fov_degrees.is_finite()
            && self.fov_degrees > 0.0
            && self.fov_degrees < 180.0
        {
            return self.fov_degrees.to_radians();
        }
        if self.fov_degrees.is_finite() {
            self.fov_degrees.clamp(10.0, 90.0).to_radians()
        } else {
            FIELD_OF_VIEW
        }
    }

    /// Keep the narrower viewport dimension framed as panels or windows resize.
    pub fn vertical_fov(&self, aspect: f32) -> f32 {
        if self.source_camera.is_some() {
            return self.fov_degrees.to_radians();
        }
        if self.source_vertical_fov {
            return self.base_fov();
        }
        let tangent = (self.base_fov() * 0.5).tan() / aspect.clamp(0.001, 1.0);
        2.0 * tangent.atan()
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        if let Some(frame) = self.source_frame() {
            return frame.gpu_projection() * frame.view64.as_mat4();
        }
        let near = (self.distance * 0.001).max(0.0001);
        let far = (self.distance * 100.0).max(100.0);
        let aspect = aspect.max(0.001);
        let projection = if self.orthographic {
            let half_height = self.distance * (self.vertical_fov(aspect) * 0.5).tan();
            let half_width = half_height * aspect;
            Mat4::orthographic_rh_gl(
                -half_width,
                half_width,
                -half_height,
                half_height,
                near,
                far,
            )
        } else {
            Mat4::perspective_rh_gl(self.vertical_fov(aspect), aspect, near, far)
        };
        projection * self.view()
    }

    /// Shared projection scale for brush cursors, material drags and gizmos.
    pub fn world_units_per_pixel(&self, depth: f32, height: f32, aspect: f32) -> f32 {
        if let Some(frame) = self.source_frame() {
            return frame
                .horizontal_world_units_per_user_point(f64::from(depth))
                .map_or(f32::NAN, |v| v as f32);
        }
        let extent_depth = if self.orthographic {
            self.distance
        } else {
            depth
        };
        2.0 * extent_depth * (self.vertical_fov(aspect) * 0.5).tan() / height.max(1.0)
    }

    /// Construct a world ray from OpenGL NDC, with positive Y pointing upward.
    pub fn ray(&self, ndc: Vec2, aspect: f32) -> Ray {
        if self.source_frame().is_some() {
            let ray = self.ray64(ndc, aspect);
            return Ray {
                origin: ray.origin.as_vec3(),
                direction: ray.direction.as_vec3(),
            };
        }
        // Construct the perspective ray directly rather than subtracting two
        // almost-coincident unprojected points at the near plane. This keeps
        // brush picking stable on small details and after distant pans.
        let (right, up, forward) = self.axes();
        let tangent = (self.vertical_fov(aspect) * 0.5).tan();
        if self.orthographic {
            return Ray {
                origin: self.position()
                    + right * ndc.x * tangent * self.distance * aspect.max(0.001)
                    + up * ndc.y * tangent * self.distance,
                direction: forward,
            };
        }
        Ray {
            origin: self.position(),
            direction: (forward
                + right * ndc.x * tangent * aspect.max(0.001)
                + up * ndc.y * tangent)
                .normalize_or_zero(),
        }
    }

    /// Orbit from a pointer delta measured in egui screen points.
    pub fn orbit(&mut self, delta: egui::Vec2) {
        if let Some(rotation) = self.rotation {
            let direction = rotation * Vec3::Z;
            self.yaw = direction.x.atan2(direction.z);
            self.pitch = direction.y.clamp(-1.0, 1.0).asin();
            self.rotation = None;
        }
        self.yaw -= delta.x * 0.008 * self.speed;
        self.pitch = (self.pitch + delta.y * 0.008 * self.speed).clamp(-1.52, 1.52);
    }

    /// Alternate rotation modes retain roll and avoid pole singularities.
    pub fn rotate(&mut self, previous: Vec2, current: Vec2, delta: egui::Vec2) {
        if self.mode == CameraMode::Orbit {
            self.orbit(delta);
            return;
        }
        let orientation = self.orientation();
        let drag = (current - previous) * self.speed;
        let local_rotation = match self.mode {
            CameraMode::Spherical => {
                let project = |point: Vec2| {
                    let length = point.length_squared();
                    if length <= 1.0 {
                        Vec3::new(point.x, point.y, (1.0 - length).sqrt())
                    } else {
                        Vec3::new(point.x, point.y, 0.0).normalize()
                    }
                };
                let from = project(previous);
                let to = project(previous + drag);
                Quat::from_rotation_arc(to, from)
            }
            CameraMode::Plane => {
                let axis = Vec3::new(drag.y, -drag.x, 0.0);
                if axis.length_squared() <= 1e-12 {
                    return;
                }
                Quat::from_axis_angle(axis.normalize(), drag.length() * 3.0)
            }
            CameraMode::Orbit => unreachable!(),
        };
        self.rotation = Some((orientation * local_rotation).normalize());
    }

    /// Preserve eye position while changing the point about which it rotates.
    pub fn set_pivot(&mut self, pivot: Vec3) {
        if !pivot.is_finite() {
            return;
        }
        if self.source_camera.is_some() {
            let center = self
                .precise_target
                .unwrap_or_else(|| self.target.as_dvec3());
            let rotation = self.orientation().to_array().map(f64::from);
            let fov = self
                .boot_source_fov
                .unwrap_or_else(|| f64::from(self.fov_degrees));
            let orthographic = self.orthographic;
            let Some(source) = self.source_camera.as_mut() else {
                return;
            };
            match source.retarget_pivot(
                center,
                pivot.as_dvec3(),
                glam::DQuat::from_array(rotation),
                fov,
                orthographic,
            ) {
                Ok(()) => {
                    self.target = pivot;
                    self.precise_target = Some(pivot.as_dvec3());
                }
                Err(error) => source.error = Some(error),
            }
            return;
        }
        let eye = self.position();
        let direction = eye - pivot;
        if direction.length_squared() <= 1e-10 {
            return;
        }
        self.distance = direction.length();
        self.target = pivot;
        self.precise_target = None;
        self.rotation = Some(Quat::from_rotation_arc(Vec3::Z, direction.normalize()));
    }

    pub fn axis_view(&mut self, axis: usize) {
        if self.source_camera.is_some() {
            let requested = match axis {
                0 => Quat::IDENTITY,
                1 => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                _ => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            };
            let same = self.orientation().dot(requested).abs() > 0.999;
            self.rotation = Some(if same {
                requested * Quat::from_rotation_y(std::f32::consts::PI)
            } else {
                requested
            });
            return;
        }
        let requested = match axis {
            0 => Quat::IDENTITY,
            1 => Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            _ => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        };
        let same = self.orientation().dot(requested).abs() > 0.999;
        self.rotation = Some(if same {
            requested * Quat::from_rotation_y(std::f32::consts::PI)
        } else {
            requested
        });
    }

    pub fn snap_axis(&mut self) {
        let orientation = self.orientation();
        // The pinned reference searches these 24 quaternion representatives
        // in this order and keeps the later candidate on an exact tie.
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let d = 0.5;
        let candidates = [
            Quat::from_xyzw(1., 0., 0., 0.),
            Quat::from_xyzw(0., 1., 0., 0.),
            Quat::from_xyzw(0., 0., 1., 0.),
            Quat::IDENTITY,
            Quat::from_xyzw(s, s, 0., 0.),
            Quat::from_xyzw(s, -s, 0., 0.),
            Quat::from_xyzw(s, 0., s, 0.),
            Quat::from_xyzw(s, 0., -s, 0.),
            Quat::from_xyzw(s, 0., 0., s),
            Quat::from_xyzw(s, 0., 0., -s),
            Quat::from_xyzw(0., s, s, 0.),
            Quat::from_xyzw(0., s, -s, 0.),
            Quat::from_xyzw(0., s, 0., s),
            Quat::from_xyzw(0., s, 0., -s),
            Quat::from_xyzw(0., 0., s, s),
            Quat::from_xyzw(0., 0., s, -s),
            Quat::from_xyzw(d, d, d, d),
            Quat::from_xyzw(d, d, d, -d),
            Quat::from_xyzw(d, d, -d, d),
            Quat::from_xyzw(d, d, -d, -d),
            Quat::from_xyzw(d, -d, d, d),
            Quat::from_xyzw(d, -d, d, -d),
            Quat::from_xyzw(d, -d, -d, d),
            Quat::from_xyzw(-d, d, d, d),
        ];
        let mut best = Quat::IDENTITY;
        let mut error = f32::INFINITY;
        for candidate in candidates {
            let dot = orientation.dot(candidate);
            let candidate_error = 1.0 - dot * dot;
            if candidate_error <= error {
                error = candidate_error;
                best = candidate;
            }
        }
        self.rotation = Some(best);
    }

    /// Pan in the camera plane from a pointer delta in screen points.
    pub fn pan(&mut self, delta: egui::Vec2) {
        if let Some(source) = &self.source_camera {
            if !delta.is_finite() {
                return;
            }
            let viewport = source.viewport();
            let ppu = viewport.pixels_per_user_point().x;
            let units = if self.orthographic {
                source.zoom() * 0.0011 * ppu
            } else {
                let depth = source.zoom() * 45.0 / f64::from(self.fov_degrees);
                2.0 * depth * (f64::from(self.fov_degrees).to_radians() * 0.5).tan()
                    / f64::from(viewport.render_pixels[1])
                    * ppu
            };
            let rotation = self.orientation();
            let right = rotation * Vec3::X;
            let up = rotation * Vec3::Y;
            let shift = (-right.as_dvec3() * f64::from(delta.x)
                + up.as_dvec3() * f64::from(delta.y))
                * units
                * f64::from(self.speed);
            let target = self
                .precise_target
                .unwrap_or_else(|| self.target.as_dvec3())
                + shift;
            if target.is_finite() && target.as_vec3().is_finite() {
                self.target = target.as_vec3();
                self.precise_target = Some(target);
            }
            return;
        }
        let (right, up, _) = self.axes();
        let displacement = (-right * delta.x + up * delta.y) * self.distance * 0.0015 * self.speed;
        self.target += displacement;
        if let Some(target) = self.precise_target.as_mut() {
            *target += displacement.as_dvec3();
        }
    }

    /// One held-arrow Source camera timer tick. The pinned camera uses a
    /// 150-unit motion scale and stores scene coordinates at 100 units per
    /// normalized OBrush unit. Left/right translate in the camera plane;
    /// up/down change camera depth without editing the document.
    pub fn source_keyboard_step(&mut self, move_x: i8, move_z: i8) {
        if move_x == 0 && move_z == 0 {
            return;
        }
        let depth = self
            .source_camera
            .as_ref()
            .map_or(f64::from(self.distance), |source| source.zoom());
        if !depth.is_finite() || depth <= 0.0 {
            return;
        }
        if move_x != 0 {
            let right = (self.orientation() * Vec3::X).as_dvec3();
            let target = self
                .precise_target
                .unwrap_or_else(|| self.target.as_dvec3())
                - right * (f64::from(move_x) * depth * 0.0075);
            if target.is_finite() && target.as_vec3().is_finite() {
                self.target = target.as_vec3();
                self.precise_target = Some(target);
            }
        }
        if move_z != 0 {
            let next = (depth + f64::from(move_z) * 0.00375).clamp(0.00001, 100_000_000.0);
            if let Some(source) = self.source_camera.as_mut() {
                if let Err(error) = source.set_zoom(next) {
                    source.error = Some(error);
                }
            } else {
                self.distance = next as f32;
            }
        }
    }

    /// Wheel delta in egui points; positive amounts move toward the target.
    pub fn zoom(&mut self, amount: f32) {
        if let Some(source) = self.source_camera.as_mut() {
            if amount.is_finite() {
                let value = (source.zoom()
                    * (-f64::from(amount) * 0.01 * f64::from(self.speed)).exp())
                .clamp(0.00001, 100_000_000.0);
                if let Err(error) = source.set_zoom(value) {
                    source.error = Some(error);
                }
            }
            return;
        }
        self.distance =
            (self.distance * (-amount * 0.01 * self.speed).exp()).clamp(0.02, 100_000_000.0);
    }

    /// Bound one frame's raw wheel travel. Applying egui's smoothed wheel tail
    /// amplified a large scroll into several rapid zoom steps through the mesh.
    /// Deliberate repeated scrolling can still approach or cross a surface.
    pub fn scroll_zoom(&mut self, amount: f32) {
        if amount.is_finite() {
            self.zoom(amount.clamp(-25.0, 25.0));
        }
    }

    pub fn frame(&mut self, mesh: &Mesh) {
        if let Some(mut source) = self.source_camera.clone() {
            let fitted = crate::source_camera_host::initial_loose_bounds(mesh)
                .and_then(|bounds| source.fit_bounds(self.source_fit_pose(), bounds));
            self.accept_source_fit(source, fitted);
            return;
        }
        let Some(&first) = mesh.positions.first() else {
            return;
        };
        let (mut minimum, mut maximum) = (first, first);
        for &position in &mesh.positions {
            minimum = minimum.min(position);
            maximum = maximum.max(position);
        }
        self.target = (minimum + maximum) * 0.5;
        self.precise_target = None;
        let radius = mesh
            .positions
            .iter()
            .map(|position| position.distance(self.target))
            .fold(0.0_f32, f32::max);
        self.distance = (radius / (self.base_fov() * 0.5).sin() * 1.12).max(0.1);
        self.refit_source_zoom();
    }

    /// Frame every effectively visible object. Hidden objects are omitted by
    /// the document. Empty Source-framed scenes reset the view; Legacy leaves
    /// the camera unchanged.
    pub fn frame_scene(&mut self, objects: &[SubtoolRenderSnapshot]) {
        if let Some(mut source) = self.source_camera.clone() {
            match source.fit_scene(self.source_fit_pose(), objects) {
                Ok(Some(center)) => self.accept_source_fit(source, Ok(center)),
                Ok(None) => {
                    self.reset_source_view();
                }
                Err(error) => {
                    self.accept_source_fit(source, Err(error));
                }
            }
            return;
        }
        if objects.is_empty() {
            return;
        }
        if objects.iter().all(|o| o.metadata.is_none()) {
            self.precise_target = None;
            self.frame_plain_scene(objects);
            self.refit_source_zoom();
            return;
        }
        let point = |o: &SubtoolRenderSnapshot, p: Vec3| -> Option<DVec3> {
            o.metadata
                .as_ref()
                .map_or_else(|| Some(p.as_dvec3()), |m| m.affine.world_point(p).ok())
        };
        let mut low = DVec3::splat(f64::INFINITY);
        let mut high = DVec3::splat(f64::NEG_INFINITY);
        let mut count = 0usize;
        for object in objects {
            for &p in &object.mesh.positions {
                let Some(p) = point(object, p) else {
                    return;
                };
                low = low.min(p);
                high = high.max(p);
                count += 1;
            }
        }
        if count == 0 {
            return;
        }
        let center = (low + high) * 0.5;
        let mut radius = 0.0f64;
        for object in objects {
            for &p in &object.mesh.positions {
                let Some(p) = point(object, p) else {
                    return;
                };
                radius = radius.max(p.distance(center));
            }
        }
        let distance = (radius / f64::from((self.base_fov() * 0.5).sin()) * 1.12).max(0.1);
        if !center.is_finite() || !(distance as f32).is_finite() {
            return;
        }
        self.target = center.as_vec3();
        self.precise_target = Some(center);
        self.distance = distance as f32;
        self.refit_source_zoom();
    }
    fn frame_plain_scene(&mut self, objects: &[SubtoolRenderSnapshot]) {
        let mut positions = objects.iter().flat_map(|object| &object.mesh.positions);
        let Some(&first) = positions.next() else {
            return;
        };
        let (mut minimum, mut maximum) = (first, first);
        for &position in positions {
            minimum = minimum.min(position);
            maximum = maximum.max(position);
        }
        self.target = (minimum + maximum) * 0.5;
        let radius = objects
            .iter()
            .flat_map(|object| &object.mesh.positions)
            .map(|position| position.distance(self.target))
            .fold(0.0_f32, f32::max);
        self.distance = (radius / (self.base_fov() * 0.5).sin() * 1.12).max(0.1);
    }
    /// Paint callbacks retain only the immutable accepted frame, not historic
    /// bounds buffers. Legacy snapshots remain an ordinary Camera clone.
    pub(crate) fn paint_snapshot(&self) -> Self {
        let mut snapshot = self.clone();
        if let Some(source) = snapshot.source_camera.as_mut() {
            source.discard_bounds_cache();
        }
        snapshot
    }
    pub(crate) fn source_enabled(&self) -> bool {
        self.source_camera.is_some()
    }
    pub(crate) fn source_frame(&self) -> Option<&crate::source_camera_math::SourceCameraFrame> {
        self.source_camera.as_ref()?.frame.as_deref()
    }
    pub(crate) fn source_diagnostics(&self) -> serde_json::Value {
        self.source_camera.as_ref().map_or(serde_json::json!({"policy":"legacy_obrush","scope":"Original camera arithmetic unchanged"}),|s|s.diagnostics())
    }
    pub(crate) fn set_source_framing(
        &mut self,
        enabled: bool,
        viewport: crate::source_camera_math::ViewportDimensions,
    ) -> Result<(), String> {
        if enabled == self.source_enabled() {
            return Ok(());
        }
        if enabled {
            let fov = f64::from(self.fov_degrees);
            let old_tangent = f64::from((self.vertical_fov(viewport.aspect() as f32) * 0.5).tan());
            let zoom = if self.orthographic {
                f64::from(self.distance) * old_tangent
                    / (f64::from(viewport.render_pixels[1]) * 0.00055)
            } else {
                let source_tangent = (fov.to_radians() * 0.5).tan();
                f64::from(self.distance) * old_tangent / source_tangent * fov / 45.0
            };
            let source = crate::source_camera_host::SourceCameraController::new(zoom, viewport)?;
            self.source_camera = Some(source);
            // The pinned SculptGL URL default for `pivot` is true. Imported
            // SGL camera settings may override this after framing is enabled.
            self.use_pivot = true;
        } else {
            let source = self.source_camera.as_ref().unwrap();
            let mut destination = self.clone();
            destination.source_camera = None;
            let tangent =
                f64::from((destination.vertical_fov(viewport.aspect() as f32) * 0.5).tan());
            let distance = if self.orthographic {
                source.view_height() * 0.5 / tangent
            } else {
                source.zoom() * 45.0 / f64::from(self.fov_degrees)
                    * (f64::from(self.fov_degrees).to_radians() * 0.5).tan()
                    / tangent
            };
            if !distance.is_finite() || distance <= 0.0 || !(distance as f32).is_finite() {
                return Err("Camera framing conversion exceeds finite distance".into());
            }
            self.distance = distance as f32;
            self.source_camera = None;
        }
        Ok(())
    }
    pub(crate) fn prepare_source_frame(
        &mut self,
        viewport: crate::source_camera_math::ViewportDimensions,
        objects: &[SubtoolRenderSnapshot],
    ) -> Result<(), String> {
        if self.source_camera.is_none() {
            return Ok(());
        }
        let rotation = self.orientation().to_array();
        let pose = crate::source_camera_host::FramePose {
            target: self
                .precise_target
                .unwrap_or_else(|| self.target.as_dvec3()),
            camera_to_world: glam::DQuat::from_array(rotation.map(f64::from)),
            orthographic: self.orthographic,
            fov_degrees: self
                .boot_source_fov
                .unwrap_or_else(|| f64::from(self.fov_degrees)),
        };
        let source = self.source_camera.as_mut().unwrap();
        let result = source.prepare(pose, viewport, objects);
        if let Err(error) = &result {
            source.error = Some(error.clone());
        }
        result
    }
    pub(crate) fn measure_source_navigation(
        &mut self,
        viewport: crate::source_camera_math::ViewportDimensions,
    ) {
        if let Some(source) = self.source_camera.as_mut() {
            source.set_navigation_viewport(viewport);
        }
    }
    pub(crate) fn source_projection_zoom(&self) -> Option<f64> {
        self.source_camera.as_ref().map(|s| s.zoom())
    }
    pub(crate) fn source_pivot_offset(&self) -> Option<DVec3> {
        self.source_camera.as_ref().map(|s| s.pivot_offset())
    }
    pub(crate) fn set_source_pivot_offset(&mut self, value: DVec3) -> Result<(), String> {
        self.source_camera
            .as_mut()
            .ok_or("Source framing is not enabled")?
            .set_pivot_offset(value)
    }
    pub(crate) fn set_source_projection_zoom(&mut self, value: f64) -> Result<(), String> {
        self.source_camera
            .as_mut()
            .ok_or("Source framing is not enabled")?
            .set_zoom(value)
    }
    pub(crate) fn source_view_height(&self) -> Option<f64> {
        self.source_camera.as_ref().map(|s| s.view_height())
    }
    pub(crate) fn set_source_view_height(&mut self, value: f64) -> Result<(), String> {
        self.source_camera
            .as_mut()
            .ok_or("Source framing is not enabled")?
            .set_view_height(value)
    }
    fn source_fit_pose(&self) -> crate::source_camera_host::FramePose {
        crate::source_camera_host::FramePose {
            target: self
                .precise_target
                .unwrap_or_else(|| self.target.as_dvec3()),
            camera_to_world: glam::DQuat::from_array(self.orientation().to_array().map(f64::from)),
            orthographic: self.orthographic,
            fov_degrees: self
                .boot_source_fov
                .unwrap_or_else(|| f64::from(self.fov_degrees)),
        }
    }
    fn accept_source_fit(
        &mut self,
        mut candidate: crate::source_camera_host::SourceCameraController,
        fitted: Result<DVec3, String>,
    ) {
        match fitted {
            Ok(center) if center.is_finite() && center.as_vec3().is_finite() => {
                candidate.reset_pivot_offset();
                self.target = center.as_vec3();
                self.precise_target = Some(center);
                self.source_camera = Some(candidate);
            }
            Ok(_) => {
                if let Some(source) = self.source_camera.as_mut() {
                    source.error =
                        Some("Source frame center exceeds finite display coordinates".into());
                }
            }
            Err(error) => {
                if let Some(source) = self.source_camera.as_mut() {
                    source.error = Some(error);
                }
            }
        }
    }
    /// Immediate endpoint of the reference Reset view action. Animation and
    /// navigation speed remain separate reference-comparison scopes.
    pub(crate) fn reset_source_view(&mut self) -> bool {
        let Some(source) = self.source_camera.as_mut() else {
            return false;
        };
        if let Err(error) = source.set_zoom(80.0) {
            source.error = Some(error);
            return false;
        }
        source.error = None;
        source.reset_pivot_offset();
        self.target = Vec3::ZERO;
        self.precise_target = Some(DVec3::ZERO);
        self.rotation = Some(Quat::IDENTITY);
        self.yaw = 0.0;
        self.pitch = 0.0;
        true
    }
    fn refit_source_zoom(&mut self) {
        let Some(source) = self.source_camera.as_mut() else {
            return;
        };
        let viewport = source.viewport();
        let fov = f64::from(self.fov_degrees).to_radians();
        let zoom = if self.orthographic {
            f64::from(self.distance) * (fov * 0.5).tan()
                / (f64::from(viewport.render_pixels[1]) * 0.00055)
        } else {
            f64::from(self.distance) * f64::from(self.fov_degrees) / 45.0
        };
        if let Err(error) = source.set_zoom(zoom) {
            source.error = Some(error);
        }
    }
    pub(crate) fn curvature_fov(&self) -> f32 {
        if let Some(frame) = self.source_frame() {
            return frame.curvature_fov() as f32;
        }
        if self.orthographic {
            -self.distance.abs().max(0.01) * 25.0
        } else {
            self.base_fov().to_degrees()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum ShadingMode {
    #[default]
    Studio = 0,
    Pbr = 1,
    Normal = 2,
    Uv = 3,
    Matcap = 4,
    Flat = 5,
}

impl ShadingMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Studio => "Studio clay",
            Self::Pbr => "PBR",
            Self::Normal => "Normal",
            Self::Uv => "UV texture",
            Self::Matcap => "Matcap",
            Self::Flat => "Flat color",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ViewportStyle {
    pub color: [f32; 3],
    pub wireframe: bool,
    pub show_mask: bool,
    pub use_vertex_color: bool,
    pub shading: ShadingMode,
    pub matcap_index: u32,
    pub(crate) custom_matcaps: crate::matcap::CustomMatcaps,
    /// Wire-only source shader is distinct from an overlay.
    pub wire_only: bool,
    pub flat_shading: bool,
    pub exposure: f32,
    pub(crate) environment: EnvironmentStyle,
    pub(crate) source_display: crate::source_display::SourceDisplayStyle,
    pub(crate) background_cursor: crate::background_cursor::CursorStyle,
    /// Global viewport-only fields; per-selected-mesh style persistence is separate.
    pub curvature: f32,
    pub opacity: f32,
    pub uv_texture: Option<Arc<ViewTexture>>,
    /// Multisample the real geometry; this never smooths or changes the mesh.
    pub antialias: bool,
    /// Source browser scene pixels per CSS viewport pixel; other hosts retain
    /// pixels per host pixel. Does not scale UI or edit geometry.
    pub resolution_scale: f32,
    /// A host gesture preview only; vertex buffers and saved geometry stay put.
    pub active_preview_translation: Vec3,
    /// World-axis scale shown during a drag; the document remains unchanged.
    pub active_preview_scale: Vec3,
    /// World-axis Euler rotation shown during a drag; the document remains unchanged.
    pub active_preview_rotation: Vec3,
    pub active_preview_pivot: Vec3,
    /// Captured selection for a whole-gesture preview, independent of later UI state.
    pub preview_subtool_ids: Vec<u32>,
}

impl Default for ViewportStyle {
    fn default() -> Self {
        Self {
            color: [0.66, 0.68, 0.72],
            wireframe: false,
            show_mask: true,
            use_vertex_color: false,
            shading: ShadingMode::Studio,
            matcap_index: 3,
            custom_matcaps: Default::default(),
            wire_only: false,
            flat_shading: false,
            exposure: 2.5,
            environment: EnvironmentStyle::default(),
            source_display: crate::source_display::SourceDisplayStyle::default(),
            background_cursor: crate::background_cursor::CursorStyle::default(),
            curvature: 0.0,
            opacity: 1.0,
            uv_texture: None,
            antialias: true,
            resolution_scale: 1.0,
            active_preview_translation: Vec3::ZERO,
            active_preview_scale: Vec3::ONE,
            active_preview_rotation: Vec3::ZERO,
            active_preview_pivot: Vec3::ZERO,
            preview_subtool_ids: Vec::new(),
        }
    }
}

pub struct Renderer {
    paint_timing: crate::dab_queue::DabTiming,
    scene_admission_timing: crate::dab_queue::DabTiming,
    mesh_upload_timing: crate::dab_queue::DabTiming,
    program: glow::Program,
    grid_program: glow::Program,
    grid_view_projection: Option<glow::UniformLocation>,
    contour: Option<crate::source_contour_gl::ContourPipeline>,
    contour_error: Option<String>,
    shader_header: &'static str,
    linear_pipeline: Option<LinearPipeline>,
    environment_library: EnvironmentLibrary,
    composite_program: glow::Program,
    composite_texture: Option<glow::UniformLocation>,
    antialias_target: Option<AntialiasTarget>,
    antialias_size: Option<[i32; 2]>,
    antialias_error: Option<String>,
    resolution_target: Option<ResolutionTarget>,
    resolution_size: Option<[i32; 2]>,
    resolution_error: Option<String>,
    last_frame_resolution: [i32; 2],
    last_frame_linear: bool,
    last_frame_antialias: FrameAntialias,
    max_samples: i32,
    max_renderbuffer_size: i32,
    // A VAO is required by the fullscreen composite on desktop core contexts,
    // including when the visible scene is empty.
    vertex_array: glow::VertexArray,
    meshes: HashMap<u32, GpuMesh>,
    source_worker_preview: Option<SourceWorkerGpuPreview>,
    source_guide_bounds: Option<SourceGuideBoundsCache>,
    selected_certificates: SelectedNormalCertificates,
    uv_texture: Option<GpuViewTexture>,
    matcaps: HashMap<u32, GpuViewTexture>,
    uniforms: Uniforms,
    scene_error: Option<String>,
    background_cursor: Option<crate::background_cursor_gl::BackgroundCursorGl>,
    background_cursor_creation_error: Option<String>,
    background_cursor_status: &'static str,
    background_cursor_matrices: Option<crate::background_cursor::CursorMatrices>,
    background_cursor_draws: u32,
    destroyed: bool,
}

#[derive(Clone, Copy, Default)]
enum FrameAntialias {
    #[default]
    Unpainted,
    Direct,
    Multisample(i32),
    Fxaa,
}

struct Uniforms {
    linear_pipeline: Option<glow::UniformLocation>,
    darken_unselected: Option<glow::UniformLocation>,
    auxiliary_flat: Option<glow::UniformLocation>,
    selected: Option<glow::UniformLocation>,
    symmetry_line: Option<glow::UniformLocation>,
    symmetry_origin: Option<glow::UniformLocation>,
    symmetry_normal: Option<glow::UniformLocation>,
    environment: EnvironmentUniforms,
    view_projection: Option<glow::UniformLocation>,
    view: Option<glow::UniformLocation>,
    normal_matrix: Option<glow::UniformLocation>,
    color: Option<glow::UniformLocation>,
    show_mask: Option<glow::UniformLocation>,
    use_vertex_color: Option<glow::UniformLocation>,
    wire: Option<glow::UniformLocation>,
    active: Option<glow::UniformLocation>,
    preview_translation: Option<glow::UniformLocation>,
    shading: Option<glow::UniformLocation>,
    flat_shading: Option<glow::UniformLocation>,
    exposure: Option<glow::UniformLocation>,
    has_material_data: Option<glow::UniformLocation>,
    default_material: Option<glow::UniformLocation>,
    curvature: Option<glow::UniformLocation>,
    curvature_fov: Option<glow::UniformLocation>,
    opacity: Option<glow::UniformLocation>,
    has_uv_data: Option<glow::UniformLocation>,
    has_uv_texture: Option<glow::UniformLocation>,
    uv_texture: Option<glow::UniformLocation>,
}

pub(crate) fn consumer_style(
    metadata: Option<&SceneObjectMetadata>,
    style: &ViewportStyle,
) -> Result<ViewportStyle, String> {
    let Some(appearance) = resolve_source_appearance(metadata, &baseline_source_render_defaults())?
    else {
        return Ok(style.clone());
    };
    let mut out = style.clone();
    out.shading = match appearance.role {
        SourceShaderRole::Pbr => ShadingMode::Pbr,
        SourceShaderRole::Flat => ShadingMode::Flat,
        SourceShaderRole::Normal => ShadingMode::Normal,
        SourceShaderRole::Wireframe => {
            out.wire_only = true;
            ShadingMode::Flat
        }
        SourceShaderRole::Uv => ShadingMode::Uv,
        SourceShaderRole::Matcap => ShadingMode::Matcap,
        SourceShaderRole::Auxiliary(code) => {
            return Err(format!(
                "Source auxiliary shader {code} has no editable viewport consumer"
            ));
        }
    };
    out.matcap_index = appearance.settings.matcap;
    out.flat_shading = appearance.settings.flat_shading;
    out.wireframe = appearance.settings.show_wireframe || out.wire_only;
    out.opacity = appearance.settings.opacity;
    out.use_vertex_color = true;
    if out.shading == ShadingMode::Matcap {
        crate::matcap::preset(out.matcap_index)?;
    }
    Ok(out)
}
pub(crate) fn consumer_style_for_mesh(
    metadata: Option<&SceneObjectMetadata>,
    mesh: &Mesh,
    style: &ViewportStyle,
) -> Result<ViewportStyle, String> {
    let mut resolved = consumer_style(metadata, style)?;
    if metadata.is_some()
        && resolved.shading == ShadingMode::Uv
        && mesh.surface.as_ref().and_then(|s| s.uv.as_ref()).is_none()
    {
        resolved.shading = ShadingMode::Matcap;
        crate::matcap::preset(resolved.matcap_index)?;
    }
    Ok(resolved)
}
type ConsumerMatrices = ([f32; 16], [f32; 16], [f32; 9], Vec3);
#[derive(Clone, Copy)]
pub(crate) struct PreviewAffine {
    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation_degrees: Vec3,
    pub pivot: Vec3,
}
pub(crate) fn consumer_matrices(
    camera: &Camera,
    aspect: f32,
    pose: &ObjectRenderTransform,
    posed: bool,
    translation: Vec3,
) -> Result<ConsumerMatrices, String> {
    if !posed {
        return Ok((
            camera.view_projection(aspect).to_cols_array(),
            camera.view().to_cols_array(),
            Mat3::from_mat4(camera.view()).to_cols_array(),
            translation,
        ));
    }
    let view = camera.view64() * DMat4::from_translation(translation.as_dvec3());
    let (mv, normal) = pose.model_view(view.to_cols_array())?;
    let mvp = camera.projection(aspect) * Mat4::from_cols_array(&mv);
    if !mvp.is_finite() {
        return Err("Affine projection exceeds finite GPU range".into());
    }
    Ok((mvp.to_cols_array(), mv, normal, Vec3::ZERO))
}

#[cfg(test)]
pub(crate) fn consumer_matrices_scaled(
    camera: &Camera,
    aspect: f32,
    pose: &ObjectRenderTransform,
    posed: bool,
    translation: Vec3,
    scale: Vec3,
    pivot: Vec3,
) -> Result<ConsumerMatrices, String> {
    consumer_matrices_previewed(
        camera,
        aspect,
        pose,
        posed,
        PreviewAffine {
            translation,
            scale,
            rotation_degrees: Vec3::ZERO,
            pivot,
        },
    )
}

pub(crate) fn consumer_matrices_previewed(
    camera: &Camera,
    aspect: f32,
    pose: &ObjectRenderTransform,
    posed: bool,
    preview: PreviewAffine,
) -> Result<ConsumerMatrices, String> {
    let PreviewAffine {
        translation,
        scale,
        rotation_degrees,
        pivot,
    } = preview;
    if scale == Vec3::ONE && rotation_degrees == Vec3::ZERO {
        return consumer_matrices(camera, aspect, pose, posed, translation);
    }
    if !scale.is_finite()
        || scale.min_element() < 0.001
        || scale.max_element() > 1000.0
        || !pivot.is_finite()
        || !translation.is_finite()
        || !rotation_degrees.is_finite()
        || rotation_degrees.abs().max_element() > 36_000.0
    {
        return Err("Invalid affine preview".into());
    }
    let angle = rotation_degrees.as_dvec3() * (std::f64::consts::PI / 180.0);
    let rotation = DMat4::from_rotation_z(angle.z)
        * DMat4::from_rotation_y(angle.y)
        * DMat4::from_rotation_x(angle.x);
    let around_pivot = DMat4::from_translation(pivot.as_dvec3())
        * rotation
        * DMat4::from_scale(scale.as_dvec3())
        * DMat4::from_translation(-pivot.as_dvec3());
    let view = camera.view64() * DMat4::from_translation(translation.as_dvec3()) * around_pivot;
    let (mv, normal) = pose.model_view(view.to_cols_array())?;
    let mvp = camera.projection(aspect) * Mat4::from_cols_array(&mv);
    if !mvp.is_finite() {
        return Err("Affine preview projection exceeds finite GPU range".into());
    }
    Ok((mvp.to_cols_array(), mv, normal, Vec3::ZERO))
}

enum UploadSource<'a> {
    Revision(u64),
    Shared(&'a Arc<Mesh>),
}

struct RenderObject<'a> {
    id: u32,
    active: bool,
    mesh: &'a Mesh,
    metadata: Option<&'a SceneObjectMetadata>,
    source: UploadSource<'a>,
    selected_normals: Option<&'a Arc<obrush_core::SourceGeometryNormalState>>,
}

struct SourceGuideBoundsCache {
    mesh: Weak<Mesh>,
    normals: Option<Weak<obrush_core::SourceGeometryNormalState>>,
    bounds: [f64; 6],
}

/// The Source octree's loose root can outlive the current vertex AABB after a
/// stroke, so the guide must use its retained producer when one is available.
/// Imported objects without a retained producer are seeded once per immutable
/// mesh snapshot instead of rebuilding a large tree on every paint callback.
fn source_guide_root_loose(
    object: &RenderObject<'_>,
    cache: &mut Option<SourceGuideBoundsCache>,
) -> Result<Option<[f64; 6]>, String> {
    if object.metadata.is_none() {
        return Ok(None);
    }
    if let UploadSource::Shared(mesh) = &object.source
        && let Some(cached) = cache
    {
        let same_mesh = cached
            .mesh
            .upgrade()
            .is_some_and(|previous| Arc::ptr_eq(&previous, mesh));
        let same_normals = match (&cached.normals, object.selected_normals) {
            (None, None) => true,
            (Some(weak), Some(current)) => weak
                .upgrade()
                .is_some_and(|previous| Arc::ptr_eq(&previous, current)),
            _ => false,
        };
        if same_mesh && same_normals {
            return Ok(Some(cached.bounds));
        }
    }
    let bounds = match object.selected_normals {
        Some(state) if state.matches_mesh(object.mesh) => state.root_loose(),
        _ => obrush_core::SourceGeometryNormalState::seed_for_guide(object.mesh)
            .map_err(|error| format!("Cannot prepare Source mirror guide: {error:?}"))?
            .root_loose(),
    };
    if let UploadSource::Shared(mesh) = &object.source {
        *cache = Some(SourceGuideBoundsCache {
            mesh: Arc::downgrade(mesh),
            normals: object.selected_normals.map(Arc::downgrade),
            bounds,
        });
    }
    Ok(Some(bounds))
}

#[derive(Default)]
struct UploadIdentity {
    revision: Option<u64>,
    mesh: Option<Weak<Mesh>>,
    vertex_count: usize,
    triangle_count: usize,
    selected_normals: Option<Weak<obrush_core::SourceGeometryNormalState>>,
}

impl UploadIdentity {
    fn matches(&self, object: &RenderObject<'_>) -> bool {
        let same_source = match object.source {
            UploadSource::Revision(revision) => self.revision == Some(revision),
            UploadSource::Shared(mesh) => self
                .mesh
                .as_ref()
                .is_some_and(|previous| previous.as_ptr() == Arc::as_ptr(mesh)),
        };
        let same_normals = match (&self.selected_normals, object.selected_normals) {
            (None, None) => true,
            (Some(previous), Some(current)) => previous.as_ptr() == Arc::as_ptr(current),
            _ => false,
        };
        same_source
            && same_normals
            && self.vertex_count == object.mesh.positions.len()
            && self.triangle_count == object.mesh.triangles.len()
    }

    fn update(&mut self, object: &RenderObject<'_>) {
        match object.source {
            UploadSource::Revision(revision) => {
                self.revision = Some(revision);
                self.mesh = None;
            }
            UploadSource::Shared(mesh) => {
                self.revision = None;
                self.mesh = Some(Arc::downgrade(mesh));
            }
        }
        self.selected_normals = object.selected_normals.map(Arc::downgrade);
        self.vertex_count = object.mesh.positions.len();
        self.triangle_count = object.mesh.triangles.len();
    }
}

#[derive(Clone, Copy)]
struct CachedUploadBounds {
    low: Vec3,
    high: Vec3,
    vertex_count: usize,
}

impl CachedUploadBounds {
    fn full(mesh: &Mesh) -> Self {
        let (low, high) = mesh.positions.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), &p| (lo.min(p), hi.max(p)),
        );
        Self {
            low,
            high,
            vertex_count: mesh.positions.len(),
        }
    }

    fn updated(
        mesh: &Mesh,
        previous: Option<&Self>,
        old_rows: Option<&[[f32; 10]]>,
        ranges: Option<&[std::ops::Range<usize>]>,
    ) -> (Self, bool) {
        let Some((mut bounds, old_rows, ranges)) = previous
            .copied()
            .zip(old_rows)
            .zip(ranges)
            .map(|((bounds, old_rows), ranges)| (bounds, old_rows, ranges))
            .filter(|(bounds, old_rows, _)| {
                bounds.vertex_count == mesh.positions.len()
                    && old_rows.len() == mesh.positions.len()
            })
        else {
            return (Self::full(mesh), false);
        };
        // The exact cached GL rows describe the old uploaded coordinates. If a
        // changed range contains any old bound value, that extreme may shrink;
        // fall back to the original full SIMD-friendly fold. Otherwise every
        // old bound still exists outside the changed ranges.
        if ranges.iter().any(|range| {
            range.end > old_rows.len()
                || range.clone().any(|index| {
                    (0..3).any(|axis| {
                        old_rows[index][axis] == bounds.low[axis]
                            || old_rows[index][axis] == bounds.high[axis]
                    })
                })
        }) {
            return (Self::full(mesh), false);
        }
        for range in ranges {
            for index in range.clone() {
                bounds.low = bounds.low.min(mesh.positions[index]);
                bounds.high = bounds.high.max(mesh.positions[index]);
            }
        }
        (bounds, true)
    }
}

#[cfg(test)]
mod cached_upload_bounds_tests {
    use super::*;

    fn rows(mesh: &Mesh) -> Vec<[f32; 10]> {
        mesh.positions
            .iter()
            .map(|p| [p.x, p.y, p.z, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
            .collect()
    }

    #[test]
    fn changed_chunks_keep_exact_full_scan_bounds_or_fall_back_at_extrema() {
        let mut mesh = Mesh::sphere(2);
        let original = CachedUploadBounds::full(&mesh);
        let before = rows(&mesh);
        let interior = (0..mesh.positions.len())
            .find(|index| {
                (0..3).all(|axis| {
                    mesh.positions[*index][axis] != original.low[axis]
                        && mesh.positions[*index][axis] != original.high[axis]
                })
            })
            .unwrap();
        mesh.positions[interior] = original.high + Vec3::splat(0.125);
        let interior_range = interior..interior + 1;
        let (expanded, incremental) = CachedUploadBounds::updated(
            &mesh,
            Some(&original),
            Some(&before),
            Some(std::slice::from_ref(&interior_range)),
        );
        assert!(incremental);
        let full = CachedUploadBounds::full(&mesh);
        assert_eq!((expanded.low, expanded.high), (full.low, full.high));

        let before = rows(&mesh);
        let old_extreme = mesh
            .positions
            .iter()
            .position(|p| p.x == expanded.low.x)
            .unwrap();
        mesh.positions[old_extreme].x += 0.25;
        let extreme_range = old_extreme..old_extreme + 1;
        let (shrunk, incremental) = CachedUploadBounds::updated(
            &mesh,
            Some(&expanded),
            Some(&before),
            Some(std::slice::from_ref(&extreme_range)),
        );
        assert!(!incremental);
        let full = CachedUploadBounds::full(&mesh);
        assert_eq!((shrunk.low, shrunk.high), (full.low, full.high));

        assert_eq!(
            CachedUploadBounds::updated(&mesh, Some(&shrunk), Some(&rows(&mesh)), Some(&[]))
                .0
                .low,
            shrunk.low
        );
        assert_eq!(
            CachedUploadBounds::updated(&mesh, Some(&shrunk), None, None)
                .0
                .high,
            full.high
        );
    }
}

// One active selected Source object may keep the exact canonical mask words
// that produced its separate GL mask buffer. Display protection rows are not
// sufficient: conversion can round distinct raw masks to the same f32 value.
struct SourceMaskSnapshot {
    raw: Vec<f32>,
    convention: obrush_core::MaskConvention,
}

impl SourceMaskSnapshot {
    fn capture(mesh: &Mesh) -> Result<Self, String> {
        if mesh.masks.len() != mesh.positions.len() || mesh.masks.len() > obrush_core::MAX_VERTICES
        {
            return Err("Source mask snapshot exceeds bounded mesh rows".into());
        }
        let mut raw = Vec::new();
        raw.try_reserve_exact(mesh.masks.len())
            .map_err(|_| "Cannot allocate bounded Source mask snapshot")?;
        raw.extend_from_slice(&mesh.masks);
        Ok(Self {
            raw,
            convention: mesh.mask_convention,
        })
    }

    fn matches(&self, mesh: &Mesh) -> bool {
        self.convention == mesh.mask_convention
            && bytemuck::cast_slice::<_, u8>(&self.raw)
                == bytemuck::cast_slice::<_, u8>(&mesh.masks)
    }
}

#[cfg(test)]
mod source_mask_snapshot_tests {
    use super::*;

    #[test]
    fn raw_mask_snapshot_detects_changes_hidden_by_display_rounding() {
        let mut mesh = Mesh::sphere(1);
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(0.0);
        let previous = SourceMaskSnapshot::capture(&mesh).unwrap();
        mesh.positions[0].x += 0.125;
        assert!(
            previous.matches(&mesh),
            "Geometry-only edits keep the mask buffer"
        );

        mesh.masks[0] = f32::from_bits(1);
        assert_eq!(mesh.mask_convention.protection_for_display(0.0), 1.0);
        assert_eq!(
            mesh.mask_convention.protection_for_display(mesh.masks[0]),
            1.0
        );
        assert!(
            !previous.matches(&mesh),
            "Subnormal raw masks must invalidate it"
        );
        mesh.masks[0] = -0.0;
        assert!(
            !previous.matches(&mesh),
            "Signed zero is part of the source mask word"
        );
        mesh.masks[0] = 0.0;
        mesh.mask_convention = obrush_core::MaskConvention::ProtectedF32V1;
        assert!(
            !previous.matches(&mesh),
            "Mask conversion policy must also match"
        );
    }
}

struct GpuMesh {
    source_normals: Option<obrush_core::SourceNormalField>,
    selected_normals: Option<Arc<obrush_core::SourceGeometryNormalState>>,
    source_upload_cache: Option<CachedSourceRows>,
    vertex_array: glow::VertexArray,
    vertices: glow::Buffer,
    materials: glow::Buffer,
    triangles: glow::Buffer,
    edges: glow::Buffer,
    identity: UploadIdentity,
    triangle_index_count: i32,
    edge_index_count: i32,
    edges_dirty: bool,
    source_wire_edges: Option<bool>,
    uploads: u64,
    selected_chunk_witness_uploads: u64,
    selected_full_scan_uploads: u64,
    incremental_bounds_updates: u64,
    full_bounds_scans: u64,
    center: Vec3,
    bounds: Option<CachedUploadBounds>,
    uv: Option<GpuUvMesh>,
    source_masks: Option<glow::Buffer>,
    source_mask_snapshot: Option<SourceMaskSnapshot>,
    source_mask_buffer_reuses: u64,
    source_mask_buffer_uploads: u64,
}

/// Private display overlay for one checked Worker gesture. The public scene
/// and cached upload identity continue to point at the immutable baseline.
struct SourceWorkerGpuPreview {
    epoch: u64,
    subtool_id: u32,
    baseline: Arc<Mesh>,
    source_display: bool,
    pending: HashMap<u32, [f32; 10]>,
    current: HashMap<u32, [f32; 10]>,
    restoring: bool,
}

struct GpuUvMesh {
    vertex_array: glow::VertexArray,
    vertices: glow::Buffer,
    materials: glow::Buffer,
    coordinates: glow::Buffer,
    triangles: glow::Buffer,
    index_count: i32,
    correspondence: Vec<[u32; 2]>,
    correspondence_fingerprint: String,
    source_masks: Option<glow::Buffer>,
}
struct GpuViewTexture {
    texture: glow::Texture,
    source: Weak<ViewTexture>,
    name: String,
    size: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceUploadNormalRole {
    Legacy,
    FreshStatic,
    SelectedRefresh,
}
fn source_upload_normal_role(
    object: &RenderObject<'_>,
    style: &ViewportStyle,
) -> SourceUploadNormalRole {
    if source_normal_policy(object.metadata, style).is_none() {
        SourceUploadNormalRole::Legacy
    } else if object.selected_normals.is_some() {
        SourceUploadNormalRole::SelectedRefresh
    } else {
        SourceUploadNormalRole::FreshStatic
    }
}

fn source_normal_policy(
    metadata: Option<&SceneObjectMetadata>,
    effective_style: &ViewportStyle,
) -> Option<obrush_core::SourceNormalPolicy> {
    // Plain Studio keeps its legacy recipe even inside a mixed linear FBO.
    (metadata.is_some() || effective_style.shading != ShadingMode::Studio)
        .then_some(obrush_core::SourceNormalPolicy::StaticOriginalFaces)
}

/// Narrow, explicit admission for the transient sparse GPU overlay. Imported
/// Source shader 0 uses retained normals, not public Mesh normals.
pub(crate) fn source_worker_preview_style_eligible(
    metadata: Option<&SceneObjectMetadata>,
    style: &ViewportStyle,
) -> bool {
    if style.wireframe || style.wire_only || style.flat_shading || style.uv_texture.is_some() {
        return false;
    }
    let Some(metadata) = metadata else {
        return style.shading == ShadingMode::Studio;
    };
    let identity = [
        1.0_f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    metadata.affine.local_to_world.map(f32::to_bits) == identity.map(f32::to_bits)
        && metadata.render.as_ref().is_some_and(|render| {
            render.shader == 0
                && render.matcap == 0
                && !render.show_wireframe
                && !render.flat_shading
                && render.opacity.to_bits() == 1.0_f32.to_bits()
        })
        && style.shading == ShadingMode::Pbr
        && style.opacity.to_bits() == 1.0_f32.to_bits()
}
fn normal_cache_matches_policy(
    field: Option<&obrush_core::SourceNormalField>,
    object: &RenderObject<'_>,
    effective_style: &ViewportStyle,
) -> bool {
    match source_normal_policy(object.metadata, effective_style) {
        None => field.is_none(),
        Some(_) => field.is_some_and(|f| f.matches_mesh(object.mesh)),
    }
}
fn scene_source_normal_storage<'a>(
    objects: &[RenderObject<'a>],
    cached: &HashMap<u32, GpuMesh>,
    style: &ViewportStyle,
    certificates: &mut SelectedNormalCertificates,
) -> Result<[Option<SelectedNormalSeal<'a>>; obrush_core::MAX_SUBTOOLS], String> {
    if objects.len() > obrush_core::MAX_SUBTOOLS {
        return Err("Selected-normal certificate object limit exceeded".into());
    }
    certificates.retain_visible(objects);
    let mut selected = [None; obrush_core::MAX_SUBTOOLS];
    // Simulate sequential replacement, retaining the old field until the new
    // one is fully validated. Include exact old caches, not only new mesh sizes.
    let mut retained = cached
        .values()
        .try_fold(SelectedNormalCertificates::storage_bytes(), |sum, gpu| {
            sum.checked_add(
                gpu.source_normals
                    .as_ref()
                    .map_or(0, |field| field.report().storage.retained_bytes)
                    .saturating_add(
                        gpu.selected_normals
                            .as_ref()
                            .map_or(0, |s| s.approximate_bytes()),
                    ),
            )
        })
        .ok_or("Scene source normal cache overflow")?;
    let mut peak = retained;
    for (index, object) in objects.iter().enumerate() {
        let old = cached
            .get(&object.id)
            .and_then(|gpu| gpu.source_normals.as_ref());
        let old_bytes = old
            .map_or(0, |field| field.report().storage.retained_bytes)
            .saturating_add(
                cached
                    .get(&object.id)
                    .and_then(|g| g.selected_normals.as_ref())
                    .map_or(0, |s| s.approximate_bytes()),
            );
        let effective_style = consumer_style_for_mesh(object.metadata, object.mesh, style)?;
        match source_normal_policy(object.metadata, &effective_style) {
            None => {
                certificates.remove(object.id);
                retained = retained
                    .checked_sub(old_bytes)
                    .ok_or("Scene source cache accounting underflow")?
            }
            Some(_) => {
                if let Some(warm) = object.selected_normals {
                    selected[index] = certificates.certify(object)?;
                    if cached
                        .get(&object.id)
                        .and_then(|g| g.selected_normals.as_ref())
                        .is_some_and(|old| Arc::ptr_eq(old, warm))
                    {
                        continue;
                    }
                    let bytes = warm.approximate_bytes();
                    peak = peak.max(
                        retained
                            .checked_add(bytes)
                            .ok_or("Source retained cache peak overflow")?,
                    );
                    retained = retained
                        .checked_sub(old_bytes)
                        .and_then(|n| n.checked_add(bytes))
                        .ok_or("Source retained cache accounting overflow")?;
                    continue;
                }
                certificates.remove(object.id);
                if old.is_some_and(|field| field.matches_mesh(object.mesh)) {
                    continue;
                }
                let required = obrush_core::SourceNormalField::storage_requirement(object.mesh)?;
                peak = peak.max(
                    retained
                        .checked_add(required.peak_bytes)
                        .ok_or("Scene source normal scratch overflow")?,
                );
                retained = retained
                    .checked_sub(old_bytes)
                    .and_then(|n| n.checked_add(required.retained_bytes))
                    .ok_or("Scene source cache accounting overflow")?;
            }
        }
    }
    check_scene_source_normal_peak(peak)?;
    Ok(selected)
}
fn check_scene_source_normal_peak(peak: usize) -> Result<(), String> {
    if peak > obrush_core::resource_limits::MAX_RENDER_SOURCE_NORMAL_BYTES {
        return Err(
            "Visible SourceMetadata aggregate normal cache/replacement exceeds renderer resource limit; no Legacy fallback".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod source_normal_resource_tests {
    #[test]
    fn aggregate_cache_budget_distinguishes_selected_state_from_fresh_field() {
        let cap = obrush_core::resource_limits::MAX_RENDER_SOURCE_NORMAL_BYTES;
        // A complete selected geometry state includes its tree/proxy/domain,
        // not just the fresh normal field. Both old/new owners remain charged.
        let actual_one_million_selected_bytes = 200_943_116usize;
        let peak = actual_one_million_selected_bytes.checked_mul(2).unwrap()
            + super::SelectedNormalCertificates::storage_bytes();
        assert!(peak > obrush_core::MAX_SOURCE_NORMAL_BYTES);
        assert!(super::check_scene_source_normal_peak(peak).is_ok());
        assert!(super::check_scene_source_normal_peak(cap).is_ok());
        assert!(super::check_scene_source_normal_peak(cap + 1).is_err());
        assert!(super::check_scene_source_normal_peak(usize::MAX).is_err());
        assert_eq!(obrush_core::MAX_SOURCE_NORMAL_BYTES, 128 * 1024 * 1024);
    }
}
fn packed_geometry_rows(
    mesh: &Mesh,
    source_normals: Option<&obrush_core::SourceNormalField>,
) -> Result<Vec<[f32; 10]>, String> {
    if mesh.positions.len() > obrush_core::MAX_VERTICES
        || mesh.triangles.len() > obrush_core::MAX_TRIANGLES
    {
        return Err("Geometry normal upload exceeds mesh limits".into());
    }
    if source_normals.is_some_and(|field| !field.matches_mesh(mesh)) {
        return Err("Geometry source normals do not match actual mesh".into());
    }
    let mut vertices = Vec::new();
    vertices
        .try_reserve_exact(mesh.positions.len())
        .map_err(|_| "Cannot allocate bounded normal upload rows")?;
    for (index, &position) in mesh.positions.iter().enumerate() {
        let normal = source_normals.map_or_else(
            || mesh.normals.get(index).copied().unwrap_or(Vec3::Y),
            |field| field.vertex_normals()[index],
        );
        let mask = mesh
            .masks
            .get(index)
            .map_or(0.0, |&raw| mesh.mask_convention.protection_for_display(raw));
        let color = mesh.colors.get(index).copied().unwrap_or([1.0; 3]);
        vertices.push([
            position.x, position.y, position.z, normal.x, normal.y, normal.z, mask, color[0],
            color[1], color[2],
        ]);
    }
    Ok(vertices)
}

fn selected_render_matches_mesh(
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
) -> bool {
    #[cfg(test)]
    selected_normal_certificates::record_validation();
    state.matches_mesh(mesh)
}
fn packed_geometry_selected_rows(
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
) -> Result<Vec<[f32; 10]>, String> {
    if !selected_render_matches_mesh(mesh, state) {
        return Err("Selected-refresh Source render state is stale".into());
    }
    packed_geometry_selected_rows_after_validation(mesh, state)
}
fn packed_geometry_selected_rows_after_validation(
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
) -> Result<Vec<[f32; 10]>, String> {
    if mesh.positions.len() > obrush_core::MAX_VERTICES
        || mesh.triangles.len() > obrush_core::MAX_TRIANGLES
    {
        return Err("Geometry normal upload exceeds mesh limits".into());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(mesh.positions.len())
        .map_err(|_| "Cannot allocate bounded normal upload rows")?;
    for index in 0..mesh.positions.len() {
        rows.push(packed_selected_row(mesh, state, index));
    }
    Ok(rows)
}

fn packed_selected_row(
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
    index: usize,
) -> [f32; 10] {
    let position = mesh.positions[index];
    let normal = state.vertex_normals()[index];
    let mask = mesh
        .masks
        .get(index)
        .map_or(0.0, |&raw| mesh.mask_convention.protection_for_display(raw));
    let color = mesh.colors.get(index).copied().unwrap_or([1.0; 3]);
    [
        position.x, position.y, position.z, normal.x, normal.y, normal.z, mask, color[0], color[1],
        color[2],
    ]
}

const SOURCE_UPLOAD_CHUNK_ROWS: usize = obrush_core::SOURCE_GEOMETRY_CHUNK_ROWS;
const MAX_SPARSE_SOURCE_CHUNKS: usize = 128;

#[inline]
fn selected_row_changed(
    old: &[f32; 10],
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
    index: usize,
) -> bool {
    let point = mesh.positions[index];
    let normal = state.vertex_normals()[index];
    if old[0].to_bits() != point.x.to_bits()
        || old[1].to_bits() != point.y.to_bits()
        || old[2].to_bits() != point.z.to_bits()
        || old[3].to_bits() != normal.x.to_bits()
        || old[4].to_bits() != normal.y.to_bits()
        || old[5].to_bits() != normal.z.to_bits()
    {
        return true;
    }
    let mask = mesh
        .masks
        .get(index)
        .map_or(0.0, |&raw| mesh.mask_convention.protection_for_display(raw));
    if old[6].to_bits() != mask.to_bits() {
        return true;
    }
    let color = mesh.colors.get(index).copied().unwrap_or([1.0; 3]);
    old[7].to_bits() != color[0].to_bits()
        || old[8].to_bits() != color[1].to_bits()
        || old[9].to_bits() != color[2].to_bits()
}

#[inline]
fn selected_attribute_changed(old: &[f32; 10], mesh: &Mesh, index: usize) -> bool {
    let mask = mesh
        .masks
        .get(index)
        .map_or(0.0, |&raw| mesh.mask_convention.protection_for_display(raw));
    if old[6].to_bits() != mask.to_bits() {
        return true;
    }
    let color = mesh.colors.get(index).copied().unwrap_or([1.0; 3]);
    old[7].to_bits() != color[0].to_bits()
        || old[8].to_bits() != color[1].to_bits()
        || old[9].to_bits() != color[2].to_bits()
}

/// Exact byte comparison of the old and new 40-byte GL rows. This is only
/// called after the old GPU identity and both immutable Source normal states
/// have been established. None requests the ordinary whole-buffer fallback.
struct SelectedRowScan {
    ranges: Option<Vec<std::ops::Range<usize>>>,
    raw_masks_match: Option<bool>,
    materials_match: Option<bool>,
}

fn selected_row_upload_scan(
    old_rows: &[[f32; 10]],
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
    selected_chunks: Option<&[u64; 8]>,
    raw_masks: Option<&SourceMaskSnapshot>,
    old_materials: Option<&Option<Vec<[f32; 2]>>>,
) -> Result<SelectedRowScan, String> {
    if old_rows.len() != mesh.positions.len() {
        return Ok(SelectedRowScan {
            ranges: None,
            raw_masks_match: None,
            materials_match: None,
        });
    }
    let mut eligible = selected_chunks.copied();
    let mut raw_masks_match = None;
    let mut materials_match = None;
    if let Some(bits) = eligible.as_mut() {
        // A controlled geometry dab proves its position/normal write scope,
        // but an independently edited mask or RGB channel can retain the same
        // normal-state Arc. Check the public channels across the whole mesh
        // before skipping any geometric chunks. The same walk also verifies
        // the raw mask and material caches, avoiding two separate million-row
        // reads on the controlled-dab path. UV keeps its own upload policy.
        raw_masks_match = raw_masks.map(|old| {
            old.convention == mesh.mask_convention
                && old.raw.len() == old_rows.len()
                && mesh.masks.len() == old_rows.len()
        });
        materials_match = old_materials.map(|old| match (old.as_ref(), mesh.materials.as_ref()) {
            (None, None) => true,
            (Some(old), Some(new)) => old.len() == old_rows.len() && new.len() == old_rows.len(),
            _ => false,
        });
        for (index, old) in old_rows.iter().enumerate() {
            if selected_attribute_changed(old, mesh, index) {
                let chunk = index / SOURCE_UPLOAD_CHUNK_ROWS;
                bits[chunk / 64] |= 1_u64 << (chunk % 64);
            }
            if raw_masks_match == Some(true)
                && raw_masks.unwrap().raw[index].to_bits() != mesh.masks[index].to_bits()
            {
                raw_masks_match = Some(false);
            }
            if materials_match == Some(true)
                && let (Some(old), Some(new)) =
                    (old_materials.unwrap().as_ref(), mesh.materials.as_ref())
                && (old[index][0].to_bits() != new[index][0].to_bits()
                    || old[index][1].to_bits() != new[index][1].to_bits())
            {
                materials_match = Some(false);
            }
        }
    }
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    ranges
        .try_reserve_exact(MAX_SPARSE_SOURCE_CHUNKS)
        .map_err(|_| "Cannot allocate bounded Source upload ranges")?;
    let mut changed_chunks = 0;
    for start in (0..mesh.positions.len()).step_by(SOURCE_UPLOAD_CHUNK_ROWS) {
        let chunk = start / SOURCE_UPLOAD_CHUNK_ROWS;
        if eligible.is_some_and(|bits| bits[chunk / 64] & (1_u64 << (chunk % 64)) == 0) {
            continue;
        }
        let end = (start + SOURCE_UPLOAD_CHUNK_ROWS).min(mesh.positions.len());
        let changed =
            (start..end).any(|index| selected_row_changed(&old_rows[index], mesh, state, index));
        if changed {
            changed_chunks += 1;
            if changed_chunks > MAX_SPARSE_SOURCE_CHUNKS {
                return Ok(SelectedRowScan {
                    ranges: None,
                    raw_masks_match,
                    materials_match,
                });
            }
            if let Some(last) = ranges.last_mut()
                && last.end == start
            {
                last.end = end;
            } else {
                ranges.push(start..end);
            }
        }
    }
    Ok(SelectedRowScan {
        ranges: Some(ranges),
        raw_masks_match,
        materials_match,
    })
}

#[cfg(test)]
fn selected_row_upload_ranges(
    old_rows: &[[f32; 10]],
    mesh: &Mesh,
    state: &obrush_core::SourceGeometryNormalState,
    selected_chunks: Option<&[u64; 8]>,
) -> Result<Option<Vec<std::ops::Range<usize>>>, String> {
    selected_row_upload_scan(old_rows, mesh, state, selected_chunks, None, None)
        .map(|scan| scan.ranges)
}

// One active Source object may retain the exact rows last published to GL.
// This is bounded CPU storage, not a second mesh or a document/history owner.
const MAX_SOURCE_UPLOAD_CACHE_BYTES: usize = 48 * 1024 * 1024;

struct CachedSourceRows {
    vertices: Vec<[f32; 10]>,
    materials: Option<Vec<[f32; 2]>>,
}

impl CachedSourceRows {
    fn storage_bytes(&self) -> Option<usize> {
        let material_bytes = self.materials.as_ref().map_or(Some(0), |rows| {
            rows.capacity().checked_mul(std::mem::size_of::<[f32; 2]>())
        })?;
        self.vertices
            .capacity()
            .checked_mul(std::mem::size_of::<[f32; 10]>())?
            .checked_add(material_bytes)
    }
}

fn source_upload_cache_admitted(mesh: &Mesh, active: bool, selected: bool) -> bool {
    active
        && selected
        && mesh
            .positions
            .len()
            .checked_mul(std::mem::size_of::<[f32; 10]>())
            .and_then(|bytes| {
                mesh.materials.as_ref().map_or(Some(bytes), |rows| {
                    rows.len()
                        .checked_mul(std::mem::size_of::<[f32; 2]>())
                        .and_then(|material_bytes| bytes.checked_add(material_bytes))
                })
            })
            .is_some_and(|bytes| bytes <= MAX_SOURCE_UPLOAD_CACHE_BYTES)
}

fn copied_material_rows(mesh: &Mesh) -> Result<Option<Vec<[f32; 2]>>, String> {
    let Some(source) = mesh.materials.as_ref() else {
        return Ok(None);
    };
    let mut rows = Vec::new();
    rows.try_reserve_exact(source.len())
        .map_err(|_| "Cannot allocate bounded Source material upload cache")?;
    rows.extend_from_slice(source);
    Ok(Some(rows))
}

/// Bound geometry buffers separately from the viewport's multisample target.
/// Wire indices budget every triangle edge, even when deduplication saves space.
const MAX_SCENE_GPU_BYTES: usize = 256 * 1024 * 1024;

fn geometry_storage_bytes(vertices: usize, triangles: usize, wireframe: bool) -> Option<usize> {
    if vertices > obrush_core::MAX_VERTICES
        || triangles > obrush_core::MAX_TRIANGLES
        || triangles.checked_mul(3)? > i32::MAX as usize
    {
        return None;
    }
    vertices
        .checked_mul(48)?
        .checked_add(triangles.checked_mul(if wireframe { 36 } else { 12 })?)
}

fn scene_source_mask_bytes(objects: &[RenderObject<'_>]) -> Option<usize> {
    objects.iter().try_fold(0_usize, |total, object| {
        let base = object.mesh.positions.len().checked_mul(4)?;
        let seams = if object
            .mesh
            .surface
            .as_ref()
            .and_then(|s| s.uv.as_ref())
            .is_some()
        {
            object.mesh.triangles.len().checked_mul(12)?
        } else {
            0
        };
        total.checked_add(base)?.checked_add(seams)
    })
}
fn scene_storage_bytes(objects: &[RenderObject<'_>], wireframe: bool) -> Option<usize> {
    if objects.len() > obrush_core::MAX_SUBTOOLS {
        return None;
    }
    objects.iter().try_fold(0_usize, |total, object| {
        let bytes = geometry_storage_bytes(
            object.mesh.positions.len(),
            object.mesh.triangles.len(),
            wireframe,
        )?;
        let next = total.checked_add(bytes)?;
        (next <= MAX_SCENE_GPU_BYTES).then_some(next)
    })
}

fn scene_render_storage_bytes(objects: &[RenderObject<'_>], uv_mode: bool) -> Option<usize> {
    let base = scene_storage_bytes(objects, true)?;
    if !uv_mode {
        return Some(base);
    }
    objects.iter().try_fold(base, |total, object| {
        if object
            .mesh
            .surface
            .as_ref()
            .and_then(|s| s.uv.as_ref())
            .is_none()
        {
            return Some(total);
        }
        let next = total.checked_add(uv_storage_upper_bound(object.mesh)?)?;
        (next <= MAX_SCENE_GPU_BYTES).then_some(next)
    })
}

fn solid_draw_order<'a>(
    objects: &'a [RenderObject<'a>],
) -> impl Iterator<Item = &'a RenderObject<'a>> {
    objects
        .iter()
        .filter(|object| !object.active)
        .chain(objects.iter().filter(|object| object.active))
}

fn finite_unit_range(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}

impl Renderer {
    pub(crate) fn adopt_checked_source_snapshot(
        &mut self,
        proof: &obrush_core::CertifiedSourceRenderSnapshot,
    ) {
        self.selected_certificates.adopt_checked_dab(proof);
    }
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        let version = gl.version();
        if version.major < 3 || (!version.is_embedded && version.major == 3 && version.minor == 0) {
            return Err(format!(
                "OBrush requires OpenGL 3.1 or WebGL2; available context is {version:?}"
            ));
        }
        let header = if version.is_embedded {
            "#version 300 es\nprecision highp float;\nprecision highp int;\n"
        } else {
            // GLSL 1.40 works on desktop OpenGL 3.1, including macOS core contexts.
            "#version 140\n"
        };
        // SAFETY: eframe provides the current GL context, and all resource names
        // below are created by that context. No references into GL-owned memory
        // are retained.
        unsafe {
            let fragment_source = format!(
                "{}{}{}",
                environment_glsl::COLOR,
                environment_glsl::IBL,
                FRAGMENT_SHADER
            );
            let program = compile_program(gl, header, VERTEX_SHADER, &fragment_source, "Sculpt")?;
            let composite_program = match compile_program(
                gl,
                header,
                COMPOSITE_VERTEX_SHADER,
                COMPOSITE_FRAGMENT_SHADER,
                "Antialias composite",
            ) {
                Ok(value) => value,
                Err(error) => {
                    gl.delete_program(program);
                    return Err(error);
                }
            };
            let grid_program = match compile_program(
                gl,
                header,
                GRID_VERTEX_SHADER,
                GRID_FRAGMENT_SHADER,
                "Scene grid",
            ) {
                Ok(value) => value,
                Err(error) => {
                    gl.delete_program(program);
                    gl.delete_program(composite_program);
                    return Err(error);
                }
            };
            let vertex_array = match gl.create_vertex_array() {
                Ok(value) => value,
                Err(error) => {
                    gl.delete_program(program);
                    gl.delete_program(composite_program);
                    gl.delete_program(grid_program);
                    return Err(format!("Cannot create sculpt vertex array: {error}"));
                }
            };
            Ok(Self {
                paint_timing: Default::default(),
                scene_admission_timing: Default::default(),
                mesh_upload_timing: Default::default(),
                uniforms: Uniforms {
                    linear_pipeline: gl.get_uniform_location(program, "u_linear_pipeline"),
                    darken_unselected: gl.get_uniform_location(program, "u_darken_unselected"),
                    auxiliary_flat: gl.get_uniform_location(program, "u_auxiliary_flat"),
                    selected: gl.get_uniform_location(program, "u_selected"),
                    symmetry_line: gl.get_uniform_location(program, "u_symmetry_line"),
                    symmetry_origin: gl.get_uniform_location(program, "u_symmetry_origin"),
                    symmetry_normal: gl.get_uniform_location(program, "u_symmetry_normal"),
                    environment: EnvironmentUniforms::new(gl, program),
                    view_projection: gl.get_uniform_location(program, "u_view_projection"),
                    view: gl.get_uniform_location(program, "u_view"),
                    normal_matrix: gl.get_uniform_location(program, "u_normal_matrix"),
                    color: gl.get_uniform_location(program, "u_color"),
                    show_mask: gl.get_uniform_location(program, "u_show_mask"),
                    use_vertex_color: gl.get_uniform_location(program, "u_use_vertex_color"),
                    wire: gl.get_uniform_location(program, "u_wire"),
                    active: gl.get_uniform_location(program, "u_active"),
                    preview_translation: gl.get_uniform_location(program, "u_preview_translation"),
                    shading: gl.get_uniform_location(program, "u_shading_mode"),
                    flat_shading: gl.get_uniform_location(program, "u_flat_shading"),
                    exposure: gl.get_uniform_location(program, "u_exposure"),
                    has_material_data: gl.get_uniform_location(program, "u_has_material_data"),
                    default_material: gl.get_uniform_location(program, "u_default_material"),
                    curvature: gl.get_uniform_location(program, "u_curvature"),
                    curvature_fov: gl.get_uniform_location(program, "u_curvature_fov"),
                    opacity: gl.get_uniform_location(program, "u_opacity"),
                    has_uv_data: gl.get_uniform_location(program, "u_has_uv_data"),
                    has_uv_texture: gl.get_uniform_location(program, "u_has_uv_texture"),
                    uv_texture: gl.get_uniform_location(program, "u_uv_texture"),
                },
                program,
                grid_view_projection: gl.get_uniform_location(grid_program, "u_view_projection"),
                grid_program,
                contour: None,
                contour_error: None,
                shader_header: header,
                linear_pipeline: None,
                environment_library: EnvironmentLibrary::default(),
                composite_texture: gl.get_uniform_location(composite_program, "u_texture"),
                composite_program,
                antialias_target: None,
                antialias_size: None,
                antialias_error: None,
                resolution_target: None,
                resolution_size: None,
                resolution_error: None,
                last_frame_resolution: [0, 0],
                last_frame_linear: false,
                last_frame_antialias: FrameAntialias::Unpainted,
                max_samples: gl.get_parameter_i32(glow::MAX_SAMPLES),
                max_renderbuffer_size: gl
                    .get_parameter_i32(glow::MAX_RENDERBUFFER_SIZE)
                    .min(gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE)),
                vertex_array,
                meshes: HashMap::new(),
                source_worker_preview: None,
                source_guide_bounds: None,
                selected_certificates: SelectedNormalCertificates::default(),
                uv_texture: None,
                matcaps: HashMap::new(),
                scene_error: None,
                background_cursor: None,
                background_cursor_creation_error: None,
                background_cursor_status: "Unpainted",
                background_cursor_matrices: None,
                background_cursor_draws: 0,
                destroyed: false,
            })
        }
    }

    pub fn begin_source_worker_preview(
        &mut self,
        epoch: u64,
        subtool_id: u32,
        baseline: Arc<Mesh>,
        source_display: bool,
    ) -> Result<(), String> {
        if self
            .source_worker_preview
            .as_ref()
            .is_some_and(|preview| preview.restoring)
        {
            self.reset_source_worker_preview_upload();
        }
        if epoch == 0 || subtool_id == 0 || self.source_worker_preview.is_some() {
            return Err("Source Worker GPU preview already has an owner".into());
        }
        self.source_worker_preview = Some(SourceWorkerGpuPreview {
            epoch,
            subtool_id,
            baseline,
            source_display,
            pending: HashMap::new(),
            current: HashMap::new(),
            restoring: false,
        });
        Ok(())
    }

    pub fn queue_source_worker_preview_rows(
        &mut self,
        epoch: u64,
        rows: &[obrush_core::SourceWorkerPreviewRow],
    ) -> Result<(), String> {
        let preview = self
            .source_worker_preview
            .as_mut()
            .filter(|preview| preview.epoch == epoch && !preview.restoring)
            .ok_or("Source Worker GPU preview has a stale owner")?;
        if rows.iter().any(|row| {
            row.index as usize >= preview.baseline.positions.len()
                || row.display_normals.is_some() != preview.source_display
        }) {
            return Err(
                "Source Worker GPU preview row identity or normal policy is invalid".into(),
            );
        }
        let new_pending = rows
            .iter()
            .filter(|row| !preview.pending.contains_key(&row.index))
            .count();
        let new_current = rows
            .iter()
            .filter(|row| !preview.current.contains_key(&row.index))
            .count();
        preview
            .pending
            .try_reserve(new_pending)
            .map_err(|_| "Cannot reserve Worker GPU preview rows")?;
        preview
            .current
            .try_reserve(new_current)
            .map_err(|_| "Cannot reserve Worker GPU preview identities")?;
        for row in rows {
            let index = row.index as usize;
            if index >= preview.baseline.positions.len() {
                return Err("Worker GPU preview row is out of range".into());
            }
            let base = &preview.baseline;
            let normal = if preview.source_display {
                row.display_normals
                    .ok_or("Source Worker display-normal row is missing")?
                    .1
            } else {
                if row.display_normals.is_some() {
                    return Err("Source Worker unexpected display-normal row".into());
                }
                row.after_normal
            };
            let mask = base
                .masks
                .get(index)
                .map_or(0.0, |&raw| base.mask_convention.protection_for_display(raw));
            let color = base.colors.get(index).copied().unwrap_or([1.0; 3]);
            let words = [
                row.after_position[0],
                row.after_position[1],
                row.after_position[2],
                normal[0],
                normal[1],
                normal[2],
                mask,
                color[0],
                color[1],
                color[2],
            ];
            preview.pending.insert(row.index, words);
            preview.current.insert(row.index, words);
        }
        Ok(())
    }

    /// Restore touched baseline rows if this mesh remains displayed. A changed
    /// public Document causes a normal full upload instead of this restoration.
    pub fn end_source_worker_preview(&mut self, epoch: u64) -> Result<(), String> {
        let result = (|| {
            let preview = self
                .source_worker_preview
                .as_mut()
                .filter(|preview| preview.epoch == epoch)
                .ok_or("Source Worker GPU preview has a stale ending owner")?;
            let new_pending = preview
                .current
                .iter()
                .filter(|(row, _)| !preview.pending.contains_key(row))
                .count();
            preview
                .pending
                .try_reserve(new_pending)
                .map_err(|_| "Cannot reserve Worker GPU restore rows")?;
            let base = &preview.baseline;
            let baseline_normals = self
                .meshes
                .get(&preview.subtool_id)
                .ok_or("Source Worker GPU baseline object disappeared")?
                .source_worker_baseline_normals(base, preview.source_display)?;
            for &row in preview.current.keys() {
                let index = row as usize;
                let position = base.positions[index];
                let normal = *baseline_normals
                    .get(index)
                    .ok_or("Worker baseline display normal missing")?;
                let mask = base
                    .masks
                    .get(index)
                    .map_or(0.0, |&raw| base.mask_convention.protection_for_display(raw));
                let color = base.colors.get(index).copied().unwrap_or([1.0; 3]);
                preview.pending.insert(
                    row,
                    [
                        position.x, position.y, position.z, normal.x, normal.y, normal.z, mask,
                        color[0], color[1], color[2],
                    ],
                );
            }
            preview.restoring = true;
            Ok(())
        })();
        if result.is_err() {
            self.reset_source_worker_preview_upload();
        }
        result
    }

    fn reset_source_worker_preview_upload(&mut self) {
        if let Some(preview) = self.source_worker_preview.take()
            && let Some(gpu) = self.meshes.get_mut(&preview.subtool_id)
        {
            gpu.identity = UploadIdentity::default();
        }
    }

    pub fn source_worker_preview_active(&self, epoch: u64) -> bool {
        self.source_worker_preview
            .as_ref()
            .is_some_and(|preview| preview.epoch == epoch && !preview.restoring)
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)] // Kept for single-mesh callers outside the scene host.
    pub fn paint(
        &mut self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        info: egui::PaintCallbackInfo,
        mesh: &Mesh,
        revision: u64,
        camera: &Camera,
        style: &ViewportStyle,
    ) {
        let object = RenderObject {
            id: 0,
            active: true,
            mesh,
            metadata: None,
            selected_normals: None,
            source: UploadSource::Revision(revision),
        };
        self.paint_objects(gl, destination, info, &[object], camera, style, None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_scene(
        &mut self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        info: egui::PaintCallbackInfo,
        objects: &[SubtoolRenderSnapshot],
        camera: &Camera,
        style: &ViewportStyle,
    ) {
        self.paint_scene_with_geometry_normals(gl, destination, info, objects, &[], camera, style);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn paint_scene_with_geometry_normals(
        &mut self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        info: egui::PaintCallbackInfo,
        objects: &[SubtoolRenderSnapshot],
        normal_states: &[(u32, Arc<obrush_core::SourceGeometryNormalState>)],
        camera: &Camera,
        style: &ViewportStyle,
    ) {
        let objects: Vec<_> = objects
            .iter()
            .map(|object| RenderObject {
                id: object.id,
                active: object.active,
                mesh: &object.mesh,
                metadata: object.metadata.as_ref(),
                selected_normals: normal_states
                    .iter()
                    .find(|(id, _)| *id == object.id)
                    .map(|(_, state)| state),
                source: UploadSource::Shared(&object.mesh),
            })
            .collect();
        self.paint_objects_at_resolution(gl, destination, info, &objects, camera, style);
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_objects_at_resolution(
        &mut self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        info: egui::PaintCallbackInfo,
        objects: &[RenderObject<'_>],
        camera: &Camera,
        style: &ViewportStyle,
    ) {
        let host = info.viewport_in_pixels();
        self.last_frame_resolution = [host.width_px, host.height_px];
        let requested = style.resolution_scale;
        if !requested.is_finite() || !(0.5..=2.0).contains(&requested) {
            self.resolution_error = Some("Viewport resolution must be from 0.5 to 2".into());
            self.paint_objects(gl, destination, info, objects, camera, style, None);
            return;
        }
        if host.width_px <= 0 || host.height_px <= 0 {
            return;
        }
        // SculptGL's Source ratio multiplies the CSS viewport dimensions,
        // independently of browser DPR. Only the browser Source scene target
        // follows that convention; the host framebuffer, camera/picking and
        // final composite continue to use physical host pixels.
        let source_css_pixels = cfg!(target_arch = "wasm32") && camera.source_enabled();
        let base = if source_css_pixels {
            [
                f64::from(info.viewport.width()),
                f64::from(info.viewport.height()),
            ]
        } else {
            [f64::from(host.width_px), f64::from(host.height_px)]
        };
        let scaled = [
            (base[0] * f64::from(requested)).round() as i32,
            (base[1] * f64::from(requested)).round() as i32,
        ];
        if scaled == [host.width_px, host.height_px] {
            self.resolution_error = None;
            if let Some(target) = self.resolution_target.take() {
                // SAFETY: the callback's GL context is current and the target
                // is owned by this renderer.
                unsafe { target.destroy(gl) };
            }
            self.resolution_size = None;
            self.paint_objects(gl, destination, info, objects, camera, style, None);
            return;
        }
        if scaled.contains(&0) {
            return;
        }
        // This allocation is independent of the document and ordinary scene
        // buffers. Reject large targets before asking the GPU to allocate.
        if !resolution_target_fits(scaled[0], scaled[1], self.max_renderbuffer_size) {
            self.resolution_error =
                Some("Viewport resolution exceeds GPU size or memory budget".into());
            self.paint_objects(gl, destination, info, objects, camera, style, None);
            return;
        }
        // SAFETY: egui has made this GL context current for the callback. The
        // wrapper restores all state and the painter's supplied framebuffer.
        unsafe {
            let state = GlState::capture(gl);
            if self.resolution_size != Some(scaled) {
                if let Some(old) = self.resolution_target.take() {
                    old.destroy(gl);
                }
                self.resolution_size = Some(scaled);
                match ResolutionTarget::new(gl, scaled[0], scaled[1]) {
                    Ok(target) => {
                        self.resolution_target = Some(target);
                        self.resolution_error = None;
                    }
                    Err(error) => self.resolution_error = Some(error),
                }
            }
            if let Some((framebuffer, texture)) = self
                .resolution_target
                .as_ref()
                .map(|target| (target.framebuffer, target.texture))
            {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.disable(glow::SCISSOR_TEST);
                gl.color_mask(true, true, true, true);
                gl.depth_mask(true);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear_depth_f32(1.0);
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                let local_info = egui::PaintCallbackInfo {
                    viewport: egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(scaled[0] as f32, scaled[1] as f32),
                    ),
                    clip_rect: egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(scaled[0] as f32, scaled[1] as f32),
                    ),
                    pixels_per_point: 1.0,
                    screen_size_px: [scaled[0] as u32, scaled[1] as u32],
                };
                self.paint_objects(
                    gl,
                    Some(framebuffer),
                    local_info,
                    objects,
                    camera,
                    style,
                    Some([host.width_px as u32, host.height_px as u32]),
                );
                let clip = info.clip_rect_in_pixels();
                let left = host.left_px.max(clip.left_px);
                let bottom = host.from_bottom_px.max(clip.from_bottom_px);
                let right = (host.left_px + host.width_px).min(clip.left_px + clip.width_px);
                let top = (host.from_bottom_px + host.height_px)
                    .min(clip.from_bottom_px + clip.height_px);
                if right > left && top > bottom {
                    gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
                    gl.viewport(
                        host.left_px,
                        host.from_bottom_px,
                        host.width_px,
                        host.height_px,
                    );
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(left, bottom, right - left, top - bottom);
                    gl.disable(glow::DEPTH_TEST);
                    gl.depth_mask(false);
                    gl.enable(glow::BLEND);
                    gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                    gl.blend_func_separate(
                        glow::ONE,
                        glow::ONE_MINUS_SRC_ALPHA,
                        glow::ONE_MINUS_DST_ALPHA,
                        glow::ONE,
                    );
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                    gl.use_program(Some(self.composite_program));
                    gl.bind_vertex_array(Some(self.vertex_array));
                    gl.uniform_1_i32(self.composite_texture.as_ref(), 0);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                }
                self.last_frame_resolution = scaled;
            }
            state.restore(gl);
            gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
        }
        if self.resolution_target.is_none() {
            self.paint_objects(gl, destination, info, objects, camera, style, None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_objects(
        &mut self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        info: egui::PaintCallbackInfo,
        objects: &[RenderObject<'_>],
        camera: &Camera,
        style: &ViewportStyle,
        source_host_viewport: Option<[u32; 2]>,
    ) {
        let paint_started = web_time::Instant::now();
        self.background_cursor_matrices = None;
        self.background_cursor_draws = 0;
        self.background_cursor_status = "Callback has not submitted cursor draws";
        let cursor_scene_empty = objects.is_empty();
        let viewport = info.viewport_in_pixels();
        if self.destroyed || viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }
        if let Some(error) = camera.source_camera.as_ref().and_then(|s| s.error.as_ref()) {
            self.scene_error = Some(format!("Source camera: {error}"));
            return;
        }
        if let Some(frame) = camera.source_frame() {
            if frame.viewport.render_pixels
                != source_host_viewport
                    .unwrap_or([viewport.width_px as u32, viewport.height_px as u32])
            {
                self.scene_error = Some(
                    "Source camera measured viewport differs from actual paint viewport".into(),
                );
                return;
            }
        } else if camera.source_enabled() {
            self.scene_error = Some("Source camera has no valid published frame".into());
            return;
        }
        let clip = info.clip_rect_in_pixels();
        let left = viewport.left_px.max(clip.left_px);
        let bottom = viewport.from_bottom_px.max(clip.from_bottom_px);
        let right = (viewport.left_px + viewport.width_px).min(clip.left_px + clip.width_px);
        let top = (viewport.from_bottom_px + viewport.height_px)
            .min(clip.from_bottom_px + clip.height_px);
        if right <= left || top <= bottom {
            return;
        }

        // SAFETY: the egui glow callback makes this context current and supplies
        // its destination via Painter::intermediate_fbo(). We never query GL
        // object bindings on WebGL, where null bindings panic in glow 0.16.
        unsafe {
            let state = GlState::capture(gl);
            self.scene_error = None;
            let linear_requested = style.shading != ShadingMode::Studio
                || objects.iter().any(|object| object.metadata.is_some())
                || style.environment.filmic
                || style.environment.custom_background
                || style.source_display.show_symmetry_line
                || !style.environment.darken_unselected;
            if linear_requested {
                if let Some(target) = self.antialias_target.take() {
                    target.destroy(gl);
                }
                self.antialias_size = None;
                self.antialias_error = None;
            }
            let admission_started = web_time::Instant::now();
            let visible_ids: HashSet<_> = objects.iter().map(|object| object.id).collect();
            self.meshes.retain(|id, mesh| {
                if visible_ids.contains(id) {
                    true
                } else {
                    mesh.destroy(gl);
                    false
                }
            });
            let mut selected_seals = [None; obrush_core::MAX_SUBTOOLS];
            let objects = if visible_ids.len() != objects.len() {
                self.scene_error = Some("Visible objects have duplicate render IDs".into());
                &[][..]
            } else if scene_render_storage_bytes(
                objects,
                objects.iter().any(|o| {
                    consumer_style_for_mesh(o.metadata, o.mesh, style)
                        .is_ok_and(|s| s.shading == ShadingMode::Uv)
                }),
            )
            .and_then(|base| {
                let extra = if linear_requested {
                    scene_source_mask_bytes(objects)?
                } else {
                    0
                };
                base.checked_add(extra)
                    .filter(|total| *total <= MAX_SCENE_GPU_BYTES)
            })
            .is_none()
            {
                // Always reserve wire storage, including cached wires while
                // wireframe is disabled. Toggling cannot exceed this budget.
                self.scene_error =
                    Some("Visible scene exceeds object or geometry GPU memory limits".into());
                &[][..]
            } else {
                match scene_source_normal_storage(
                    objects,
                    &self.meshes,
                    style,
                    &mut self.selected_certificates,
                ) {
                    Ok(seals) => {
                        selected_seals = seals;
                        objects
                    }
                    Err(error) => {
                        self.scene_error = Some(error);
                        &[][..]
                    }
                }
            };
            self.scene_admission_timing
                .record(admission_started.elapsed());
            let upload_started = web_time::Instant::now();
            for (object_index, object) in objects.iter().enumerate() {
                let object_style =
                    match consumer_style_for_mesh(object.metadata, object.mesh, style) {
                        Ok(v) => v,
                        Err(error) => {
                            self.scene_error = Some(format!("Object {}: {error}", object.id));
                            continue;
                        }
                    };
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    self.meshes.entry(object.id)
                {
                    match GpuMesh::new(gl) {
                        Ok(mesh) => {
                            entry.insert(mesh);
                        }
                        Err(error) => {
                            self.scene_error = Some(error);
                            continue;
                        }
                    }
                }
                let gpu = self.meshes.get_mut(&object.id).expect("mesh was inserted");
                if !object.active {
                    gpu.source_upload_cache = None;
                    gpu.source_mask_snapshot = None;
                }
                gl.bind_vertex_array(Some(gpu.vertex_array));
                let normal_match =
                    gpu.normal_cache_matches(object, &object_style, selected_seals[object_index]);
                let mut upload = Ok(());
                if !gpu.identity.matches(object) || !normal_match {
                    upload = gpu.upload(gl, object, &object_style, selected_seals[object_index]);
                }
                if upload.is_ok()
                    && object_style.shading == ShadingMode::Uv
                    && object
                        .mesh
                        .surface
                        .as_ref()
                        .and_then(|s| s.uv.as_ref())
                        .is_some()
                    && gpu.uv.is_none()
                {
                    upload = gpu.upload_uv(gl, object.mesh);
                } else if object_style.shading != ShadingMode::Uv
                    && let Some(uv) = gpu.uv.take()
                {
                    uv.destroy(gl);
                }
                if upload.is_ok() && linear_requested {
                    upload = gpu.ensure_source_masks(gl, object.mesh);
                }
                if upload.is_ok()
                    && object_style.wireframe
                    && (gpu.edges_dirty || gpu.source_wire_edges != Some(object.metadata.is_some()))
                {
                    upload = gpu.upload_edges(gl, object.mesh, object.metadata.is_some());
                }
                let mut clear_worker_preview = false;
                if upload.is_ok()
                    && let Some(preview) = self.source_worker_preview.as_mut()
                    && preview.subtool_id == object.id
                {
                    if !source_worker_preview_style_eligible(object.metadata, &object_style)
                        || preview.source_display != object.metadata.is_some()
                        || object_style.wireframe
                        || object_style.wire_only
                        || object_style.flat_shading
                        || object_style.uv_texture.is_some()
                        || (!preview.source_display
                            && (gpu.selected_normals.is_some() || gpu.source_normals.is_some()))
                        || (preview.source_display
                            && gpu.selected_normals.is_none()
                            && gpu.source_normals.is_none())
                    {
                        upload = Err("Source Worker GPU preview render style changed".into());
                        clear_worker_preview = true;
                    } else if !std::ptr::eq(object.mesh, preview.baseline.as_ref()) {
                        if preview.restoring {
                            clear_worker_preview = true;
                        } else {
                            upload = Err("Source Worker GPU preview baseline changed".into());
                            clear_worker_preview = true;
                        }
                    } else {
                        upload = gpu.apply_source_worker_preview_rows(
                            gl,
                            &mut preview.pending,
                            &preview.current,
                            &preview.baseline,
                            preview.source_display,
                        );
                        if upload.is_ok() && preview.restoring {
                            clear_worker_preview = true;
                        }
                    }
                }
                if clear_worker_preview {
                    self.reset_source_worker_preview_upload();
                }
                if let Err(error) = upload {
                    self.scene_error = Some(format!("Object {}: {error}", object.id));
                    // Never mark a failed upload as current or draw partial
                    // old/new buffers. The document remains untouched.
                    if let Some(gpu) = self.meshes.remove(&object.id) {
                        gpu.destroy(gl);
                    }
                    if self
                        .source_worker_preview
                        .as_ref()
                        .is_some_and(|preview| preview.subtool_id == object.id)
                    {
                        self.reset_source_worker_preview_upload();
                    }
                }
            }
            self.mesh_upload_timing.record(upload_started.elapsed());
            let atlas_for_frame = if linear_requested {
                match self.prepare_linear_pipeline(
                    gl,
                    [viewport.width_px, viewport.height_px],
                    style,
                ) {
                    Ok(atlas) => Some(atlas),
                    Err(error) => {
                        self.scene_error = Some(error);
                        None
                    }
                }
            } else {
                None
            };
            let use_linear = atlas_for_frame.is_some();
            let contour_color = style.source_display.contour_color;
            let contour_requested = style.source_display.show_contour
                && contour_color.iter().all(|x| x.is_finite())
                && contour_color[3] > 0.0;
            let mut contour_prepared = false;
            if contour_requested {
                let contour_meshes: Vec<_> = objects
                    .iter()
                    .filter(|object| style.source_display.selected(object.id, object.active))
                    .filter_map(|object| {
                        let gpu = self.meshes.get(&object.id)?;
                        let pose = ObjectRenderTransform::for_object(object.metadata).ok()?;
                        let translation = if style.preview_subtool_ids.contains(&object.id) {
                            style.active_preview_translation
                        } else {
                            Vec3::ZERO
                        };
                        let scale = if style.preview_subtool_ids.contains(&object.id) {
                            style.active_preview_scale
                        } else {
                            Vec3::ONE
                        };
                        let rotation = if style.preview_subtool_ids.contains(&object.id) {
                            style.active_preview_rotation
                        } else {
                            Vec3::ZERO
                        };
                        let (mvp, _, _, _) = consumer_matrices_previewed(
                            camera,
                            viewport.width_px as f32 / viewport.height_px as f32,
                            &pose,
                            object.metadata.is_some(),
                            PreviewAffine {
                                translation,
                                scale,
                                rotation_degrees: rotation,
                                pivot: style.active_preview_pivot,
                            },
                        )
                        .ok()?;
                        Some(crate::source_contour_gl::ContourMesh {
                            vao: gpu.vertex_array,
                            triangles: gpu.triangles,
                            count: gpu.triangle_index_count,
                            mvp,
                        })
                    })
                    .collect();
                if !contour_meshes.is_empty() {
                    if self.contour.is_none() {
                        match crate::source_contour_gl::ContourPipeline::new(gl, self.shader_header)
                        {
                            Ok(pipeline) => self.contour = Some(pipeline),
                            Err(error) => self.contour_error = Some(error),
                        }
                    }
                    if let Some(contour) = self.contour.as_mut() {
                        match contour.draw_mask(
                            gl,
                            [viewport.width_px, viewport.height_px],
                            &contour_meshes,
                        ) {
                            Ok(()) => {
                                self.contour_error = None;
                                contour_prepared = true;
                            }
                            Err(error) => self.contour_error = Some(error),
                        }
                    }
                    if let Some(error) = self.contour_error.as_ref() {
                        self.scene_error = Some(format!("Selection contour: {error}"));
                    }
                }
            }
            if !contour_prepared && let Some(contour) = self.contour.as_mut() {
                contour.discard_target(gl);
            }
            let use_antialias = !use_linear
                && style.antialias
                && self.ensure_antialias_target(gl, viewport.width_px, viewport.height_px);
            self.last_frame_linear = use_linear;
            self.last_frame_antialias = if use_linear && style.antialias {
                FrameAntialias::Fxaa
            } else if use_antialias {
                FrameAntialias::Multisample(self.antialias_target.as_ref().unwrap().samples)
            } else {
                FrameAntialias::Direct
            };
            gl.bind_vertex_array(Some(self.vertex_array));

            if use_linear {
                let pipeline = self
                    .linear_pipeline
                    .as_ref()
                    .expect("prepared linear pipeline");
                pipeline.begin_opaque(gl);
                gl.scissor(
                    left - viewport.left_px,
                    bottom - viewport.from_bottom_px,
                    right - left,
                    top - bottom,
                );
            } else if use_antialias {
                let target = self
                    .antialias_target
                    .as_ref()
                    .expect("antialias target exists");
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.multisample));
                gl.viewport(0, 0, viewport.width_px, viewport.height_px);
                // Clear the complete offscreen target, then apply the callback
                // clip in local framebuffer coordinates.
                gl.disable(glow::SCISSOR_TEST);
                gl.color_mask(true, true, true, true);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                gl.scissor(
                    left - viewport.left_px,
                    bottom - viewport.from_bottom_px,
                    right - left,
                    top - bottom,
                );
            } else {
                gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
                gl.viewport(
                    viewport.left_px,
                    viewport.from_bottom_px,
                    viewport.width_px,
                    viewport.height_px,
                );
                gl.scissor(left, bottom, right - left, top - bottom);
            }
            gl.enable(glow::SCISSOR_TEST);
            #[cfg(not(target_arch = "wasm32"))]
            if use_antialias {
                gl.enable(glow::MULTISAMPLE);
            } else {
                gl.disable(glow::MULTISAMPLE);
            }
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.depth_mask(true);
            gl.depth_range_f32(0.0, 1.0);
            gl.clear_depth_f32(1.0);
            // GL clears obey the scissor rectangle, preserving other viewports.
            if !use_linear {
                gl.clear(glow::DEPTH_BUFFER_BIT);
            }
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.color_mask(true, true, true, true);
            gl.use_program(Some(self.program));
            gl.uniform_1_i32(
                self.uniforms.linear_pipeline.as_ref(),
                i32::from(use_linear),
            );
            gl.uniform_1_i32(
                self.uniforms.darken_unselected.as_ref(),
                i32::from(style.environment.darken_unselected),
            );
            if let Some(atlas) = atlas_for_frame.as_ref() {
                self.uniforms.environment.upload(
                    gl,
                    atlas,
                    &glam::DMat3::from_mat4(camera.view64())
                        .transpose()
                        .as_mat3()
                        .to_cols_array(),
                );
                self.linear_pipeline.as_ref().unwrap().bind_environment(gl);
            }

            let aspect = viewport.width_px as f32 / viewport.height_px as f32;
            gl.uniform_matrix_4_f32_slice(
                self.uniforms.view_projection.as_ref(),
                false,
                &camera.view_projection(aspect).to_cols_array(),
            );
            gl.uniform_matrix_4_f32_slice(
                self.uniforms.view.as_ref(),
                false,
                &camera.view().to_cols_array(),
            );
            gl.uniform_matrix_3_f32_slice(
                self.uniforms.normal_matrix.as_ref(),
                false,
                &Mat3::from_mat4(camera.view()).to_cols_array(),
            );
            gl.uniform_3_f32(
                self.uniforms.color.as_ref(),
                style.color[0],
                style.color[1],
                style.color[2],
            );
            gl.uniform_1_i32(self.uniforms.show_mask.as_ref(), i32::from(style.show_mask));
            gl.uniform_1_i32(
                self.uniforms.use_vertex_color.as_ref(),
                i32::from(style.use_vertex_color),
            );
            gl.uniform_1_i32(self.uniforms.wire.as_ref(), 0);
            gl.uniform_1_i32(self.uniforms.shading.as_ref(), style.shading as i32);
            gl.uniform_1_i32(
                self.uniforms.flat_shading.as_ref(),
                i32::from(style.flat_shading),
            );
            let exposure = if style.exposure.is_finite() {
                if use_linear {
                    style.exposure.clamp(0.0, 5.0)
                } else {
                    style.exposure.clamp(0.1, 8.0)
                }
            } else {
                1.0
            };
            gl.uniform_1_f32(self.uniforms.exposure.as_ref(), exposure);
            let material = obrush_core::DEFAULT_VERTEX_MATERIAL;
            gl.uniform_2_f32(
                self.uniforms.default_material.as_ref(),
                material[0],
                material[1],
            );
            gl.uniform_1_f32(
                self.uniforms.curvature.as_ref(),
                finite_unit_range(style.curvature, 0.0, 5.0, 0.0),
            );
            gl.uniform_1_f32(self.uniforms.curvature_fov.as_ref(), camera.curvature_fov());
            let opacity = finite_unit_range(style.opacity, 0.0, 1.0, 1.0);
            gl.uniform_1_f32(self.uniforms.opacity.as_ref(), opacity);
            let uv_image = if use_linear
                && style.uv_texture.is_none()
                && objects.iter().any(|object| {
                    consumer_style_for_mesh(object.metadata, object.mesh, style)
                        .is_ok_and(|s| s.shading == ShadingMode::Uv)
                }) {
                match default_uv_texture() {
                    Ok(image) => Some(image),
                    Err(error) => {
                        self.scene_error = Some(error);
                        None
                    }
                }
            } else {
                style.uv_texture.clone()
            };
            if let Err(error) = self.ensure_view_texture(gl, uv_image.as_ref()) {
                self.scene_error = Some(error);
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(
                glow::TEXTURE_2D,
                self.uv_texture.as_ref().map(|t| t.texture),
            );
            gl.uniform_1_i32(self.uniforms.uv_texture.as_ref(), 0);
            gl.uniform_1_i32(
                self.uniforms.has_uv_texture.as_ref(),
                i32::from(self.uv_texture.is_some()),
            );
            if !use_linear
                && (style.wireframe
                    || objects.iter().any(|o| {
                        consumer_style_for_mesh(o.metadata, o.mesh, style)
                            .is_ok_and(|s| s.wireframe)
                    }))
            {
                gl.enable(glow::POLYGON_OFFSET_FILL);
                gl.polygon_offset(1.0, 1.0);
            } else {
                gl.disable(glow::POLYGON_OFFSET_FILL);
            }
            if style.source_display.show_grid {
                // Source's fixed XZ floor is depth tested as opaque geometry.
                // Later meshes hide it naturally; the far background only fills
                // untouched depth in the linear render path.
                gl.use_program(Some(self.grid_program));
                gl.bind_vertex_array(Some(self.vertex_array));
                gl.uniform_matrix_4_f32_slice(
                    self.grid_view_projection.as_ref(),
                    false,
                    &camera.view_projection(aspect).to_cols_array(),
                );
                gl.draw_arrays(glow::LINES, 0, 88);
                gl.use_program(Some(self.program));
            }
            // Opaque first; transparent shells use world transformed centers.
            let source_scene = objects.iter().all(|object| object.metadata.is_some());
            let mut draw_order: Vec<_> = if source_scene {
                objects.iter().collect()
            } else {
                solid_draw_order(objects).collect()
            };
            draw_order.sort_by(|a, b| {
                let alpha = |o: &RenderObject<'_>| {
                    consumer_style_for_mesh(o.metadata, o.mesh, style).map_or(1.0, |s| s.opacity)
                };
                let ta = alpha(a) < if a.metadata.is_some() { 0.99 } else { 1.0 };
                let tb = alpha(b) < if b.metadata.is_some() { 0.99 } else { 1.0 };
                ta.cmp(&tb).then_with(|| {
                    if (ta && tb) || source_scene {
                        let z = |o: &RenderObject<'_>| {
                            let center = o.metadata.map_or_else(
                                || self.meshes.get(&o.id).map_or(Vec3::ZERO, |g| g.center),
                                |m| Vec3::from_array(m.affine.interaction_center),
                            );
                            let world = o.metadata.map_or_else(
                                || center.as_dvec3(),
                                |m| m.affine.world_point(center).unwrap_or(DVec3::ZERO),
                            );
                            let preview_world = if style.preview_subtool_ids.contains(&o.id) {
                                let pivot = style.active_preview_pivot.as_dvec3();
                                let angle = style.active_preview_rotation.as_dvec3()
                                    * (std::f64::consts::PI / 180.0);
                                let rotated = (DMat4::from_rotation_z(angle.z)
                                    * DMat4::from_rotation_y(angle.y)
                                    * DMat4::from_rotation_x(angle.x))
                                .transform_vector3(
                                    (world - pivot) * style.active_preview_scale.as_dvec3(),
                                );
                                pivot + rotated + style.active_preview_translation.as_dvec3()
                            } else {
                                world
                            };
                            camera.view64().transform_point3(preview_world).z
                        };
                        if source_scene {
                            if ta && tb {
                                z(a).total_cmp(&z(b))
                            } else {
                                z(b).total_cmp(&z(a))
                            }
                        } else {
                            z(a).total_cmp(&z(b)).then_with(|| a.id.cmp(&b.id))
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
            });
            let mut transparent_started = false;
            for object in draw_order {
                let object_style =
                    match consumer_style_for_mesh(object.metadata, object.mesh, style) {
                        Ok(v) => v,
                        Err(error) => {
                            self.scene_error = Some(error);
                            continue;
                        }
                    };
                let pose = match ObjectRenderTransform::for_object(object.metadata) {
                    Ok(v) => v,
                    Err(error) => {
                        self.scene_error = Some(error);
                        continue;
                    }
                };
                let translation = if style.preview_subtool_ids.contains(&object.id) {
                    style.active_preview_translation
                } else {
                    Vec3::ZERO
                };
                let scale = if style.preview_subtool_ids.contains(&object.id) {
                    style.active_preview_scale
                } else {
                    Vec3::ONE
                };
                let rotation = if style.preview_subtool_ids.contains(&object.id) {
                    style.active_preview_rotation
                } else {
                    Vec3::ZERO
                };
                let (mvp, mv, normal, preview) = match consumer_matrices_previewed(
                    camera,
                    aspect,
                    &pose,
                    object.metadata.is_some(),
                    PreviewAffine {
                        translation,
                        scale,
                        rotation_degrees: rotation,
                        pivot: style.active_preview_pivot,
                    },
                ) {
                    Ok(v) => v,
                    Err(error) => {
                        self.scene_error = Some(error);
                        continue;
                    }
                };
                gl.uniform_matrix_4_f32_slice(self.uniforms.view_projection.as_ref(), false, &mvp);
                gl.uniform_matrix_4_f32_slice(self.uniforms.view.as_ref(), false, &mv);
                gl.uniform_matrix_3_f32_slice(self.uniforms.normal_matrix.as_ref(), false, &normal);
                gl.uniform_1_i32(
                    self.uniforms.selected.as_ref(),
                    i32::from(style.source_display.selected(object.id, object.active)),
                );
                let guide = if style.source_display.guide_enabled(object.active) {
                    let root_loose = if style.source_display.guide_profile
                        == crate::source_display::GuidePlaneProfile::ActiveSourceObject
                    {
                        source_guide_root_loose(object, &mut self.source_guide_bounds)
                    } else {
                        Ok(None)
                    };
                    match root_loose.and_then(|bounds| {
                        crate::source_display::guide_plane_uniforms(
                            &style.source_display,
                            object.mesh,
                            object.metadata,
                            bounds,
                            &mv,
                            &normal,
                        )
                    }) {
                        Ok(plane) => Some(plane),
                        Err(error) => {
                            self.scene_error = Some(error);
                            None
                        }
                    }
                } else {
                    None
                };
                gl.uniform_1_i32(
                    self.uniforms.symmetry_line.as_ref(),
                    i32::from(guide.is_some()),
                );
                if let Some(plane) = guide {
                    gl.uniform_3_f32(
                        self.uniforms.symmetry_origin.as_ref(),
                        plane.origin[0],
                        plane.origin[1],
                        plane.origin[2],
                    );
                    gl.uniform_3_f32(
                        self.uniforms.symmetry_normal.as_ref(),
                        plane.normal[0],
                        plane.normal[1],
                        plane.normal[2],
                    );
                }
                gl.uniform_1_i32(self.uniforms.shading.as_ref(), object_style.shading as i32);
                gl.uniform_1_i32(
                    self.uniforms.wire.as_ref(),
                    i32::from(object_style.wire_only),
                );
                gl.uniform_1_i32(
                    self.uniforms.auxiliary_flat.as_ref(),
                    i32::from(
                        object
                            .metadata
                            .and_then(|m| m.render.as_ref())
                            .is_some_and(|render| render.shader == 1),
                    ),
                );
                gl.uniform_1_i32(
                    self.uniforms.flat_shading.as_ref(),
                    i32::from(object_style.flat_shading),
                );
                gl.uniform_1_i32(
                    self.uniforms.use_vertex_color.as_ref(),
                    i32::from(object_style.use_vertex_color),
                );
                let opacity = finite_unit_range(object_style.opacity, 0.0, 1.0, 1.0);
                gl.uniform_1_f32(self.uniforms.opacity.as_ref(), opacity);
                let transparent = opacity < if object.metadata.is_some() { 0.99 } else { 1.0 };
                if use_linear && transparent && !transparent_started {
                    if let Err(error) = self.draw_linear_background(
                        gl,
                        camera,
                        style,
                        atlas_for_frame.as_ref().unwrap(),
                    ) {
                        self.scene_error = Some(error);
                    }
                    self.linear_pipeline.as_ref().unwrap().begin_transparent(gl);
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(
                        left - viewport.left_px,
                        bottom - viewport.from_bottom_px,
                        right - left,
                        top - bottom,
                    );
                    self.draw_scene_wires(gl, objects, camera, style, aspect, true);
                    gl.use_program(Some(self.program));
                    gl.uniform_1_i32(
                        self.uniforms.wire.as_ref(),
                        i32::from(object_style.wire_only),
                    );
                    transparent_started = true;
                    // Wires changed pose/style uniforms. Reinstall this object's actual pose.
                    gl.uniform_matrix_4_f32_slice(
                        self.uniforms.view_projection.as_ref(),
                        false,
                        &mvp,
                    );
                    gl.uniform_matrix_4_f32_slice(self.uniforms.view.as_ref(), false, &mv);
                    gl.uniform_matrix_3_f32_slice(
                        self.uniforms.normal_matrix.as_ref(),
                        false,
                        &normal,
                    );
                    gl.uniform_1_i32(self.uniforms.shading.as_ref(), object_style.shading as i32);
                    gl.uniform_1_i32(
                        self.uniforms.auxiliary_flat.as_ref(),
                        i32::from(
                            object
                                .metadata
                                .and_then(|m| m.render.as_ref())
                                .is_some_and(|render| render.shader == 1),
                        ),
                    );
                    gl.uniform_1_i32(
                        self.uniforms.flat_shading.as_ref(),
                        i32::from(object_style.flat_shading),
                    );
                    gl.uniform_1_i32(
                        self.uniforms.use_vertex_color.as_ref(),
                        i32::from(object_style.use_vertex_color),
                    );
                    gl.uniform_1_f32(self.uniforms.opacity.as_ref(), opacity);
                }
                if transparent {
                    gl.enable(glow::BLEND);
                    gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                    gl.blend_func_separate(
                        glow::ONE,
                        glow::ONE_MINUS_SRC_ALPHA,
                        glow::ONE,
                        glow::ONE_MINUS_SRC_ALPHA,
                    );
                    gl.depth_mask(false);
                    gl.enable(glow::CULL_FACE);
                } else {
                    gl.disable(glow::BLEND);
                    gl.depth_mask(true);
                    gl.disable(glow::CULL_FACE);
                }
                // Source keeps global CCW even under a reflected affine pose;
                // its front-facing normal flip and transparent shell order use it.
                gl.front_face(if object.metadata.is_none() && pose.reflected {
                    glow::CW
                } else {
                    glow::CCW
                });
                gl.uniform_1_i32(self.uniforms.active.as_ref(), i32::from(object.active));
                gl.uniform_1_i32(
                    self.uniforms.has_material_data.as_ref(),
                    i32::from(object.mesh.materials.is_some()),
                );
                gl.active_texture(glow::TEXTURE0);
                if object_style.shading == ShadingMode::Matcap {
                    match self.ensure_matcap_texture(
                        gl,
                        object_style.matcap_index,
                        &object_style.custom_matcaps,
                    ) {
                        Ok(texture) => gl.bind_texture(glow::TEXTURE_2D, Some(texture)),
                        Err(error) => {
                            self.scene_error = Some(error);
                            continue;
                        }
                    }
                } else {
                    gl.bind_texture(
                        glow::TEXTURE_2D,
                        self.uv_texture.as_ref().map(|t| t.texture),
                    );
                }
                let Some(gpu) = self.meshes.get(&object.id) else {
                    continue;
                };
                let translation = preview;
                gl.uniform_3_f32(
                    self.uniforms.preview_translation.as_ref(),
                    translation.x,
                    translation.y,
                    translation.z,
                );
                // Active objects win exact coplanar ties, as they do in core
                // picking. This does not reveal them through nearer surfaces.
                gl.depth_func(if object.active || object.metadata.is_some() {
                    glow::LEQUAL
                } else {
                    glow::LESS
                });
                let uv = if !object_style.wire_only && object_style.shading == ShadingMode::Uv {
                    gpu.uv.as_ref()
                } else {
                    None
                };
                gl.uniform_1_i32(self.uniforms.has_uv_data.as_ref(), i32::from(uv.is_some()));
                gl.bind_vertex_array(Some(uv.map_or(gpu.vertex_array, |u| u.vertex_array)));
                gl.bind_buffer(
                    glow::ELEMENT_ARRAY_BUFFER,
                    Some(if object_style.wire_only {
                        gpu.edges
                    } else {
                        uv.map_or(gpu.triangles, |u| u.triangles)
                    }),
                );
                let count = if object_style.wire_only {
                    gpu.edge_index_count
                } else {
                    uv.map_or(gpu.triangle_index_count, |u| u.index_count)
                };
                let primitive = if object_style.wire_only {
                    glow::LINES
                } else {
                    glow::TRIANGLES
                };
                if transparent {
                    // Draw one shell's back then front. Center-sorted intersecting
                    // objects remain an explicit transparency-quality limitation.
                    for face in [glow::FRONT, glow::BACK] {
                        gl.cull_face(face);
                        gl.draw_elements(primitive, count, glow::UNSIGNED_INT, 0);
                    }
                } else {
                    gl.draw_elements(primitive, count, glow::UNSIGNED_INT, 0);
                }
            }

            if use_linear {
                if !transparent_started {
                    if let Err(error) = self.draw_linear_background(
                        gl,
                        camera,
                        style,
                        atlas_for_frame.as_ref().unwrap(),
                    ) {
                        self.scene_error = Some(error);
                    }
                    self.linear_pipeline.as_ref().unwrap().begin_transparent(gl);
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(
                        left - viewport.left_px,
                        bottom - viewport.from_bottom_px,
                        right - left,
                        top - bottom,
                    );
                    self.draw_scene_wires(gl, objects, camera, style, aspect, true);
                }
            } else {
                self.draw_scene_wires(gl, objects, camera, style, aspect, false);
            }
            if contour_prepared
                && use_linear
                && let Some(transparent) = self
                    .linear_pipeline
                    .as_ref()
                    .and_then(crate::environment_gl::LinearPipeline::transparent_framebuffer)
            {
                self.contour.as_ref().unwrap().composite(
                    gl,
                    Some(transparent),
                    [0, 0, viewport.width_px, viewport.height_px],
                    [
                        left - viewport.left_px,
                        bottom - viewport.from_bottom_px,
                        right - left,
                        top - bottom,
                    ],
                    self.vertex_array,
                    contour_color,
                    true,
                );
            }
            if use_linear {
                gl.bind_vertex_array(Some(self.vertex_array));
                self.linear_pipeline
                    .as_ref()
                    .unwrap()
                    .merge(gl, style.environment.filmic);
                gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
                gl.viewport(
                    viewport.left_px,
                    viewport.from_bottom_px,
                    viewport.width_px,
                    viewport.height_px,
                );
                gl.enable(glow::SCISSOR_TEST);
                gl.scissor(left, bottom, right - left, top - bottom);
                gl.bind_vertex_array(Some(self.vertex_array));
                self.linear_pipeline
                    .as_ref()
                    .unwrap()
                    .present(gl, style.antialias);
            } else if use_antialias {
                let target = self
                    .antialias_target
                    .as_ref()
                    .expect("antialias target exists");
                gl.disable(glow::SCISSOR_TEST);
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(target.multisample));
                gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(target.resolved));
                gl.blit_framebuffer(
                    0,
                    0,
                    viewport.width_px,
                    viewport.height_px,
                    0,
                    0,
                    viewport.width_px,
                    viewport.height_px,
                    glow::COLOR_BUFFER_BIT,
                    glow::NEAREST,
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
                gl.viewport(
                    viewport.left_px,
                    viewport.from_bottom_px,
                    viewport.width_px,
                    viewport.height_px,
                );
                gl.enable(glow::SCISSOR_TEST);
                gl.scissor(left, bottom, right - left, top - bottom);
                gl.disable(glow::DEPTH_TEST);
                gl.depth_mask(false);
                gl.enable(glow::BLEND);
                gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                // Resolving transparent empty samples already premultiplies
                // edge RGB by coverage. Preserve egui's existing background.
                gl.blend_func_separate(
                    glow::ONE,
                    glow::ONE_MINUS_SRC_ALPHA,
                    glow::ONE_MINUS_DST_ALPHA,
                    glow::ONE,
                );
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(target.texture));
                gl.use_program(Some(self.composite_program));
                gl.bind_vertex_array(Some(self.vertex_array));
                gl.uniform_1_i32(self.composite_texture.as_ref(), 0);
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
            }
            if contour_prepared && !use_linear {
                self.contour.as_ref().unwrap().composite(
                    gl,
                    destination,
                    [
                        viewport.left_px,
                        viewport.from_bottom_px,
                        viewport.width_px,
                        viewport.height_px,
                    ],
                    [left, bottom, right - left, top - bottom],
                    self.vertex_array,
                    contour_color,
                    false,
                );
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
            // Genuine post-presentation guides. The original real state restore
            // remains the callback boundary; no extra render or hidden camera.
            self.background_cursor_status = "Studio cursor profile";
            if style.background_cursor.profile
                == crate::background_cursor::CursorProfile::SourceBackground
            {
                self.background_cursor_status = style
                    .background_cursor
                    .unavailable
                    .unwrap_or("Cursor frame unavailable");
                if let Some(frame) = style.background_cursor.frame {
                    if !cursor_scene_empty {
                        self.background_cursor_status = "Actual renderer scene is not empty";
                    } else if frame.viewport
                        != [viewport.width_px as u32, viewport.height_px as u32]
                    {
                        self.background_cursor_status = "Cursor host/paint viewport mismatch";
                    } else if let Ok(matrices) = crate::background_cursor::matrices(frame) {
                        if self.background_cursor.is_none()
                            && self.background_cursor_creation_error.is_none()
                        {
                            match crate::background_cursor_gl::BackgroundCursorGl::new(
                                gl,
                                self.shader_header,
                            ) {
                                Ok(cursor) => self.background_cursor = Some(cursor),
                                Err(error) => {
                                    self.background_cursor_creation_error =
                                        Some(error.chars().take(2048).collect())
                                }
                            }
                        }
                        if let Some(cursor) = self.background_cursor.as_ref() {
                            gl.viewport(
                                viewport.left_px,
                                viewport.from_bottom_px,
                                viewport.width_px,
                                viewport.height_px,
                            );
                            gl.enable(glow::SCISSOR_TEST);
                            gl.scissor(left, bottom, right - left, top - bottom);
                            match cursor.draw(gl, frame, matrices) {
                                Ok(draws) => {
                                    self.background_cursor_draws = draws;
                                    self.background_cursor_matrices = Some(matrices);
                                    self.background_cursor_status =
                                        "Post-presentation cursor draws submitted";
                                }
                                Err(error) => {
                                    self.background_cursor_creation_error =
                                        Some(error.chars().take(2048).collect());
                                    self.background_cursor_status = "Cursor context rejected";
                                }
                            }
                        } else {
                            self.background_cursor_status = "Cursor resources unavailable";
                        }
                    } else {
                        self.background_cursor_status = "Cursor matrix inputs rejected";
                    }
                }
            }
            state.restore(gl);
        }
        self.paint_timing.record(paint_started.elapsed());
    }

    pub fn sculpt_paint_timing(&self) -> serde_json::Value {
        serde_json::json!({
            "paint": self.paint_timing.metrics(),
            "scene_admission": self.scene_admission_timing.metrics(),
            "mesh_upload": self.mesh_upload_timing.metrics(),
            "selected_chunk_witness_uploads": self.meshes.values().map(|m|m.selected_chunk_witness_uploads).sum::<u64>(),
            "selected_full_scan_uploads": self.meshes.values().map(|m|m.selected_full_scan_uploads).sum::<u64>(),
            "incremental_bounds_updates": self.meshes.values().map(|m|m.incremental_bounds_updates).sum::<u64>(),
            "full_bounds_scans": self.meshes.values().map(|m|m.full_bounds_scans).sum::<u64>(),
            "source_mask_buffer_reuses": self.meshes.values().map(|m|m.source_mask_buffer_reuses).sum::<u64>(),
            "source_mask_buffer_uploads": self.meshes.values().map(|m|m.source_mask_buffer_uploads).sum::<u64>(),
            "scope": "CPU wall time in actual renderer callback; stages are nested in paint and do not include the egui host frame",
        })
    }

    unsafe fn draw_linear_background(
        &self,
        gl: &glow::Context,
        camera: &Camera,
        style: &ViewportStyle,
        atlas: &crate::environment_ibl::EnvironmentAtlas,
    ) -> Result<(), String> {
        unsafe {
            // Background uses gl_VertexID, independently of the last mesh VAO.
            gl.bind_vertex_array(Some(self.vertex_array));
            self.linear_pipeline
                .as_ref()
                .expect("prepared linear pipeline")
                .draw_background(
                    gl,
                    &style.environment,
                    atlas,
                    &glam::DMat3::from_mat4(camera.view64())
                        .transpose()
                        .as_mat3()
                        .to_cols_array(),
                )
        }
    }

    unsafe fn draw_scene_wires(
        &self,
        gl: &glow::Context,
        objects: &[RenderObject<'_>],
        camera: &Camera,
        style: &ViewportStyle,
        aspect: f32,
        source: bool,
    ) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.enable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            if source {
                gl.enable(glow::BLEND);
                gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            }
            if style.wireframe
                || objects.iter().any(|o| {
                    consumer_style_for_mesh(o.metadata, o.mesh, style).is_ok_and(|s| s.wireframe)
                })
            {
                gl.disable(glow::POLYGON_OFFSET_FILL);
                gl.depth_func(if source { glow::LESS } else { glow::LEQUAL });
                gl.depth_mask(source);
                gl.line_width(1.0);
                gl.uniform_1_i32(self.uniforms.wire.as_ref(), 1);
                gl.uniform_1_i32(self.uniforms.has_uv_data.as_ref(), 0);
                // Indexed edges work in WebGL2, where glPolygonMode is absent.
                for object in objects {
                    let object_style =
                        match consumer_style_for_mesh(object.metadata, object.mesh, style) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                    if !object_style.wireframe {
                        continue;
                    }
                    if source
                        && object_style.wire_only
                        && !object
                            .metadata
                            .and_then(|m| m.render.as_ref())
                            .is_some_and(|r| r.show_wireframe)
                    {
                        continue;
                    }
                    let pose = match ObjectRenderTransform::for_object(object.metadata) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let translation = if style.preview_subtool_ids.contains(&object.id) {
                        style.active_preview_translation
                    } else {
                        Vec3::ZERO
                    };
                    let scale = if style.preview_subtool_ids.contains(&object.id) {
                        style.active_preview_scale
                    } else {
                        Vec3::ONE
                    };
                    let rotation = if style.preview_subtool_ids.contains(&object.id) {
                        style.active_preview_rotation
                    } else {
                        Vec3::ZERO
                    };
                    let Ok((mvp, mv, normal, preview)) = consumer_matrices_previewed(
                        camera,
                        aspect,
                        &pose,
                        object.metadata.is_some(),
                        PreviewAffine {
                            translation,
                            scale,
                            rotation_degrees: rotation,
                            pivot: style.active_preview_pivot,
                        },
                    ) else {
                        continue;
                    };
                    gl.uniform_matrix_4_f32_slice(
                        self.uniforms.view_projection.as_ref(),
                        false,
                        &mvp,
                    );
                    gl.uniform_matrix_4_f32_slice(self.uniforms.view.as_ref(), false, &mv);
                    gl.uniform_matrix_3_f32_slice(
                        self.uniforms.normal_matrix.as_ref(),
                        false,
                        &normal,
                    );
                    gl.uniform_1_f32(
                        self.uniforms.opacity.as_ref(),
                        finite_unit_range(object_style.opacity, 0.0, 1.0, 1.0),
                    );
                    if source || object_style.opacity < 1.0 {
                        gl.enable(glow::BLEND);
                        gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                        gl.blend_func_separate(
                            glow::ONE,
                            glow::ONE_MINUS_SRC_ALPHA,
                            glow::ONE,
                            glow::ONE_MINUS_SRC_ALPHA,
                        );
                    } else {
                        gl.disable(glow::BLEND);
                    }
                    let Some(gpu) = self.meshes.get(&object.id) else {
                        continue;
                    };
                    gl.uniform_1_i32(self.uniforms.active.as_ref(), i32::from(object.active));
                    let translation = preview;
                    gl.uniform_3_f32(
                        self.uniforms.preview_translation.as_ref(),
                        translation.x,
                        translation.y,
                        translation.z,
                    );
                    gl.bind_vertex_array(Some(gpu.vertex_array));
                    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(gpu.edges));
                    gl.draw_elements(glow::LINES, gpu.edge_index_count, glow::UNSIGNED_INT, 0);
                }
            }
        }
    }

    /// Multisampling used in the last painted frame. A cached target does not
    /// imply it was used; FXAA is a separate, single-sample postprocess.
    pub fn antialias_samples(&self) -> i32 {
        match self.last_frame_antialias {
            FrameAntialias::Multisample(samples) => samples,
            _ => 0,
        }
    }

    pub fn antialias_algorithm(&self) -> &'static str {
        match self.last_frame_antialias {
            FrameAntialias::Unpainted => "unpainted",
            FrameAntialias::Direct => "direct",
            FrameAntialias::Multisample(_) => "multisample",
            FrameAntialias::Fxaa => "fxaa",
        }
    }

    pub fn antialias_error(&self) -> Option<&str> {
        if self.last_frame_linear {
            None
        } else {
            self.antialias_error.as_deref()
        }
    }

    pub fn resolution_status(&self) -> ([i32; 2], Option<&str>) {
        (self.last_frame_resolution, self.resolution_error.as_deref())
    }

    /// Render-only seam correspondence, bounded when explicitly requested.
    /// Counts/fingerprint cover all rows; a truncation flag never implies full
    /// row proof. Core geometry/UV IDs can be independently reconstructed.
    pub fn uv_render_metrics(&self, include_rows: bool) -> serde_json::Value {
        let mut objects:Vec<_>=self.meshes.iter().map(|(&id,g)| {
            let uv=g.uv.as_ref();
            serde_json::json!({"id":id,"core_vertices":g.identity.vertex_count,"core_triangles":g.identity.triangle_count,
                "source_normal_policy":g.source_normals.as_ref().map(|field| field.report()),
                "selected_refresh_normal_policy":g.selected_normals.as_ref().map(|s|serde_json::json!({"policy":format!("{:?}",SourceUploadNormalRole::SelectedRefresh),"generation":s.generation(),"bytes":s.approximate_bytes()})),
                "uv_render_vertices":uv.map_or(0,|u|u.correspondence.len()),
                "uv_render_index_count":uv.map_or(0,|u|u.index_count),
                "correspondence_fingerprint_fnv1a64_le":uv.map(|u|u.correspondence_fingerprint.as_str()),
                "correspondence":if include_rows { uv.map(|u| &u.correspondence[..u.correspondence.len().min(MAX_CORRESPONDENCE_DIAGNOSTIC_ROWS)]) } else {None},
                "correspondence_truncated":include_rows&&uv.is_some_and(|u|u.correspondence.len()>MAX_CORRESPONDENCE_DIAGNOSTIC_ROWS),
                "correspondence_row_limit":MAX_CORRESPONDENCE_DIAGNOSTIC_ROWS})
        }).collect();
        objects.sort_by_key(|o| o["id"].as_u64());
        serde_json::json!({"objects":objects,"texture":self.uv_texture.as_ref().map(|t|serde_json::json!({"name":t.name,"width":t.size[0],"height":t.size[1],"gpu_storage_bytes":texture_gpu_storage_bytes(t.size[0],t.size[1]),"mipmapped":t.size[0].is_power_of_two()&&t.size[1].is_power_of_two()})),"scene_error":self.scene_error,
            "scope":"UV VAO only; each row maps render -> original geometric/corner UV IDs; welded core and ordinary40+8 buffers stay separate"})
    }

    pub(crate) fn max_view_texture_side(&self) -> u32 {
        self.max_renderbuffer_size.max(0) as u32
    }

    pub(crate) fn matcap_diagnostics(&self) -> serde_json::Value {
        let mut loaded: Vec<_> = self
            .matcaps
            .iter()
            .map(|(&id, texture)| {
                serde_json::json!({"id":id,"name":texture.name,"width":texture.size[0],"height":texture.size[1]})
            })
            .collect();
        loaded.sort_by_key(|entry| entry["id"].as_u64());
        serde_json::json!({"loaded":loaded,"scene_error":self.scene_error})
    }

    unsafe fn ensure_view_texture(
        &mut self,
        gl: &glow::Context,
        image: Option<&Arc<ViewTexture>>,
    ) -> Result<(), String> {
        let Some(image) = image else {
            if let Some(old) = self.uv_texture.take() {
                unsafe {
                    gl.delete_texture(old.texture);
                }
            }
            return Ok(());
        };
        if self
            .uv_texture
            .as_ref()
            .is_some_and(|t| t.source.as_ptr() == Arc::as_ptr(image))
        {
            return Ok(());
        }
        unsafe {
            if image.width > gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE).max(0) as u32
                || image.height > gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE).max(0) as u32
            {
                return Err(
                    "Viewport texture exceeds this GPU's texture-size limit; previous asset kept"
                        .into(),
                );
            }
            texture_gpu_storage_bytes(image.width, image.height)
                .ok_or("Viewport texture GPU allocation limit exceeded")?;
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Cannot create viewport texture: {e}"))?;
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            let mipmapped = image.width.is_power_of_two() && image.height.is_power_of_two();
            let wrap = if mipmapped {
                glow::REPEAT
            } else {
                glow::CLAMP_TO_EDGE
            };
            for (key, value) in [
                (
                    glow::TEXTURE_MIN_FILTER,
                    if mipmapped {
                        glow::LINEAR_MIPMAP_LINEAR
                    } else {
                        glow::LINEAR
                    },
                ),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, wrap),
                (glow::TEXTURE_WRAP_T, wrap),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, key, value as i32);
            }
            let row_length = gl.get_parameter_i32(glow::UNPACK_ROW_LENGTH);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                image.width as i32,
                image.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&image.pixels_bottom_up)),
            );
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, row_length);
            if mipmapped {
                gl.generate_mipmap(glow::TEXTURE_2D);
            }
            if let Err(error) = geometry_gl_result(gl, "viewport UV texture upload") {
                gl.delete_texture(texture);
                return Err(error);
            }
            if let Some(old) = self.uv_texture.replace(GpuViewTexture {
                texture,
                source: Arc::downgrade(image),
                name: image.name.clone(),
                size: [image.width, image.height],
            }) {
                gl.delete_texture(old.texture);
            }
        }
        Ok(())
    }

    unsafe fn ensure_matcap_texture(
        &mut self,
        gl: &glow::Context,
        index: u32,
        custom: &crate::matcap::CustomMatcaps,
    ) -> Result<glow::Texture, String> {
        if let Some(texture) = self.matcaps.get(&index) {
            return Ok(texture.texture);
        }
        let image = if index < crate::matcap::MATCAP_NAMES.len() as u32 {
            crate::matcap::preset(index)?
        } else {
            custom
                .get(index)
                .ok_or("Session MatCap is no longer available")?
        };
        if self.matcaps.len()
            >= crate::matcap::MATCAP_NAMES.len() + crate::matcap::MAX_CUSTOM_MATCAPS
        {
            return Err("Matcap catalog GPU cache limit reached".into());
        }
        let previous = self.uv_texture.take();
        let upload = unsafe { self.ensure_view_texture(gl, Some(&image)) };
        let new = self.uv_texture.take();
        self.uv_texture = previous;
        upload?;
        let texture = new.ok_or("Matcap texture upload did not produce a resource")?;
        let handle = texture.texture;
        self.matcaps.insert(index, texture);
        Ok(handle)
    }

    /// Read-only per-object upload counters. IDs are sorted for stable output;
    /// counters reset when hidden/deleted objects release their GPU resources.
    pub fn subtool_upload_stats(&self) -> Vec<(u32, usize, usize, u64)> {
        let mut stats: Vec<_> = self
            .meshes
            .iter()
            .map(|(&id, mesh)| {
                (
                    id,
                    mesh.identity.vertex_count,
                    mesh.identity.triangle_count,
                    mesh.uploads,
                )
            })
            .collect();
        stats.sort_unstable_by_key(|entry| entry.0);
        stats
    }

    pub(crate) fn background_cursor_diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"status":self.background_cursor_status,"draws_submitted":self.background_cursor_draws,
            "resources_cached":self.background_cursor.is_some(),"fixed_vertex_bytes":crate::background_cursor::VERTEX_BYTES,
            "creation_error":self.background_cursor_creation_error,"rgba":[0.8_f32,0.4_f32,0.0_f32,1.0_f32],
            "matrices":self.background_cursor_matrices.map(|m|serde_json::json!({"circle":m.circle,"dot":m.dot,"symmetry":m.symmetry,
                "circle_words":m.circle.map(f32::to_bits),"dot_words":m.dot.map(f32::to_bits),"symmetry_words":m.symmetry.map(f32::to_bits)})),
            "scope":"Empty/no-pick/fresh-cache/idle; f32 host input; DPR1/UIzoom1; surface/active/native/pressure/HiDPI parity unverified"})
    }

    pub fn scene_error(&self) -> Option<&str> {
        self.scene_error.as_deref()
    }

    unsafe fn prepare_linear_pipeline(
        &mut self,
        gl: &glow::Context,
        size: [i32; 2],
        style: &ViewportStyle,
    ) -> Result<Arc<crate::environment_ibl::EnvironmentAtlas>, String> {
        let atlas = self.environment_library.get(style.environment.preset)?;
        unsafe {
            if self.linear_pipeline.is_none() {
                self.linear_pipeline = Some(LinearPipeline::new(gl, self.shader_header)?);
            }
            self.linear_pipeline.as_mut().unwrap().prepare(
                gl,
                size,
                &atlas,
                style.environment.background_image.as_ref(),
            )?;
        }
        Ok(atlas)
    }
    pub fn environment_diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"cached_presets":self.environment_library.cached_count(),"last_frame_linear":self.last_frame_linear,"last_frame_antialias":self.antialias_algorithm(),"last_frame_multisamples":self.antialias_samples(),"pipeline":self.linear_pipeline.as_ref().map(LinearPipeline::diagnostics)})
    }
    unsafe fn ensure_antialias_target(
        &mut self,
        gl: &glow::Context,
        width: i32,
        height: i32,
    ) -> bool {
        if self.antialias_size == Some([width, height]) {
            return self.antialias_target.is_some();
        }
        if let Some(target) = self.antialias_target.take() {
            unsafe { target.destroy(gl) };
        }
        self.antialias_size = Some([width, height]);
        self.antialias_error = None;
        for samples in [4, 2] {
            if !antialias_target_fits(
                width,
                height,
                samples,
                self.max_samples,
                self.max_renderbuffer_size,
            ) {
                continue;
            }
            match unsafe { AntialiasTarget::new(gl, width, height, samples) } {
                Ok(target) => {
                    self.antialias_target = Some(target);
                    self.antialias_error = None;
                    return true;
                }
                Err(error) => self.antialias_error = Some(error),
            }
        }
        if self.antialias_error.is_none() {
            self.antialias_error =
                Some("Viewport exceeds multisample format or memory limits".into());
        }
        false
    }

    /// Release resources while the originating context is current (on_exit).
    pub fn destroy(&mut self, gl: &glow::Context) {
        if self.destroyed {
            return;
        }
        unsafe {
            if let Some(cursor) = self.background_cursor.take()
                && let Err(error) = cursor.destroy(gl)
            {
                self.background_cursor_creation_error = Some(error);
            }
            if let Some(target) = self.antialias_target.take() {
                target.destroy(gl);
            }
            if let Some(target) = self.resolution_target.take() {
                target.destroy(gl);
            }
            if let Some(contour) = self.contour.take() {
                contour.destroy(gl);
            }
            for (_, texture) in self.matcaps.drain() {
                gl.delete_texture(texture.texture);
            }
            if let Some(texture) = self.uv_texture.take() {
                gl.delete_texture(texture.texture);
            }
            for (_, mesh) in self.meshes.drain() {
                mesh.destroy(gl);
            }
            gl.delete_vertex_array(self.vertex_array);
            if let Some(pipeline) = self.linear_pipeline.take() {
                pipeline.destroy(gl);
            }
            gl.delete_program(self.program);
            gl.delete_program(self.grid_program);
            gl.delete_program(self.composite_program);
        }
        self.selected_certificates.clear();
        self.destroyed = true;
    }
}

impl GpuMesh {
    unsafe fn apply_source_worker_preview_rows(
        &mut self,
        gl: &glow::Context,
        pending: &mut HashMap<u32, [f32; 10]>,
        current: &HashMap<u32, [f32; 10]>,
        baseline: &Mesh,
        source_display: bool,
    ) -> Result<(), String> {
        if pending.is_empty() {
            return Ok(());
        }
        let mut indices: Vec<_> = pending.keys().copied().collect();
        indices.sort_unstable();
        // A bounded dense span can absorb a few unchanged gaps using exact
        // sparse-shadow or baseline words. Choose the least overdraw that
        // stays below the WebGL call and byte budgets. Reject before GL if
        // fragmentation still exceeds them.
        const MAX_CALLS: usize = 16;
        const MAX_ROWS: usize = 262_144;
        const MAX_RUN_ROWS: u32 = 8_192;
        let mut selected = None;
        for gap in [0_u32, 8, 32, 128, 512, 2_048] {
            let mut groups = Vec::new();
            groups
                .try_reserve_exact(MAX_CALLS + 1)
                .map_err(|_| "Cannot reserve Worker GPU upload ranges")?;
            let mut first = indices[0];
            let mut last = first;
            let mut total = 0_usize;
            for &index in &indices[1..] {
                if index - last <= gap + 1 && index - first < MAX_RUN_ROWS {
                    last = index;
                } else {
                    total += (last - first + 1) as usize;
                    groups.push((first, last));
                    first = index;
                    last = index;
                }
                if groups.len() > MAX_CALLS || total > MAX_ROWS {
                    break;
                }
            }
            if groups.len() > MAX_CALLS || total > MAX_ROWS {
                continue;
            }
            total += (last - first + 1) as usize;
            groups.push((first, last));
            if groups.len() <= MAX_CALLS && total <= MAX_ROWS {
                selected = Some(groups);
                break;
            }
        }
        let groups =
            selected.ok_or("Source Worker sparse GPU packet exceeds call or row budget")?;
        let baseline_normals = self.source_worker_baseline_normals(baseline, source_display)?;
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
        }
        for (start, end) in groups {
            let mut words = Vec::new();
            words
                .try_reserve_exact((end - start + 1) as usize * 10)
                .map_err(|_| "Cannot reserve Source Worker GPU row upload")?;
            for index in start..=end {
                if let Some(row) = pending.get(&index).or_else(|| current.get(&index)) {
                    words.extend_from_slice(row);
                } else {
                    let position = baseline.positions[index as usize];
                    let normal = *baseline_normals
                        .get(index as usize)
                        .ok_or("Worker baseline display normal missing")?;
                    let mask = baseline.masks.get(index as usize).map_or(0.0, |&raw| {
                        baseline.mask_convention.protection_for_display(raw)
                    });
                    let color = baseline
                        .colors
                        .get(index as usize)
                        .copied()
                        .unwrap_or([1.0; 3]);
                    words.extend_from_slice(&[
                        position.x, position.y, position.z, normal.x, normal.y, normal.z, mask,
                        color[0], color[1], color[2],
                    ]);
                }
            }
            let offset = i32::try_from(u64::from(start) * 40)
                .map_err(|_| "Source Worker GPU row offset exceeds WebGL range")?;
            unsafe {
                gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    offset,
                    bytemuck::cast_slice(&words),
                );
            }
        }
        unsafe {
            geometry_gl_result(gl, "Source Worker sparse row upload")?;
        }
        pending.clear();
        Ok(())
    }
    fn source_worker_baseline_normals<'a>(
        &'a self,
        baseline: &'a Mesh,
        source_display: bool,
    ) -> Result<&'a [Vec3], String> {
        if !source_display {
            return Ok(&baseline.normals);
        }
        if let Some(selected) = &self.selected_normals
            && selected.matches_mesh(baseline)
        {
            return Ok(selected.vertex_normals());
        }
        if let Some(static_field) = &self.source_normals
            && static_field.matches_mesh(baseline)
        {
            return Ok(static_field.vertex_normals());
        }
        Err("Source Worker GPU baseline display normals are unavailable".into())
    }
    fn normal_cache_matches(
        &self,
        object: &RenderObject<'_>,
        effective_style: &ViewportStyle,
        selected_seal: Option<SelectedNormalSeal<'_>>,
    ) -> bool {
        if source_upload_normal_role(object, effective_style)
            == SourceUploadNormalRole::SelectedRefresh
            && let Some(warm) = object.selected_normals
        {
            return self.source_normals.is_none()
                && self
                    .selected_normals
                    .as_ref()
                    .is_some_and(|old| Arc::ptr_eq(old, warm))
                && (selected_seal.is_some_and(|seal| seal.matches(object.mesh, warm))
                    || selected_render_matches_mesh(object.mesh, warm));
        }
        self.selected_normals.is_none()
            && normal_cache_matches_policy(self.source_normals.as_ref(), object, effective_style)
    }
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let (mut vertex_array, mut vertices, mut materials, mut triangles, mut edges) =
            (None, None, None, None, None);
        let result = (|| unsafe {
            let array = gl.create_vertex_array()?;
            vertex_array = Some(array);
            let vertex_buffer = gl.create_buffer()?;
            vertices = Some(vertex_buffer);
            let material_buffer = gl.create_buffer()?;
            materials = Some(material_buffer);
            let triangle_buffer = gl.create_buffer()?;
            triangles = Some(triangle_buffer);
            let edge_buffer = gl.create_buffer()?;
            edges = Some(edge_buffer);
            gl.bind_vertex_array(Some(array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            // Position, smooth normal, mask, and RGB color are interleaved.
            for (location, size, offset) in [(0, 3, 0), (1, 3, 12), (2, 1, 24), (3, 3, 28)] {
                gl.enable_vertex_attrib_array(location);
                gl.vertex_attrib_pointer_f32(location, size, glow::FLOAT, false, 40, offset);
            }
            // Preserve the established 40-byte geometry/RGB stream. Surface
            // material scalars use a separately bounded 8-byte attribute stream.
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(material_buffer));
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 2, glow::FLOAT, false, 8, 0);
            geometry_gl_result(gl, "object buffer setup")?;
            Ok(Self {
                vertex_array: array,
                vertices: vertex_buffer,
                materials: material_buffer,
                triangles: triangle_buffer,
                edges: edge_buffer,
                identity: UploadIdentity::default(),
                triangle_index_count: 0,
                edge_index_count: 0,
                edges_dirty: true,
                source_wire_edges: None,
                uploads: 0,
                selected_chunk_witness_uploads: 0,
                selected_full_scan_uploads: 0,
                incremental_bounds_updates: 0,
                full_bounds_scans: 0,
                center: Vec3::ZERO,
                bounds: None,
                uv: None,
                source_masks: None,
                source_mask_snapshot: None,
                source_mask_buffer_reuses: 0,
                source_mask_buffer_uploads: 0,
                source_normals: None,
                selected_normals: None,
                source_upload_cache: None,
            })
        })();
        if result.is_err() {
            unsafe {
                if let Some(value) = edges {
                    gl.delete_buffer(value);
                }
                if let Some(value) = triangles {
                    gl.delete_buffer(value);
                }
                if let Some(value) = vertices {
                    gl.delete_buffer(value);
                }
                if let Some(value) = materials {
                    gl.delete_buffer(value);
                }
                if let Some(value) = vertex_array {
                    gl.delete_vertex_array(value);
                }
            }
        }
        result.map_err(|error: String| format!("Cannot create sculpt object buffers: {error}"))
    }

    unsafe fn upload(
        &mut self,
        gl: &glow::Context,
        object: &RenderObject<'_>,
        effective_style: &ViewportStyle,
        selected_seal: Option<SelectedNormalSeal<'_>>,
    ) -> Result<(), String> {
        let mesh = object.mesh;
        let previous_selected = self.selected_normals.clone();
        // The preceding upload used this retained producer's immutable face
        // allocation. A fresh producer, changed counts, or a non-Source path
        // must upload exact public indices again; no whole-stream comparison.
        let reuse_triangles = self.identity.vertex_count == mesh.positions.len()
            && self.identity.triangle_count == mesh.triangles.len()
            && self
                .selected_normals
                .as_ref()
                .zip(object.selected_normals)
                .is_some_and(|(old, current)| old.shares_topology_storage(current));
        // Derive/validate before touching GL. Failure never uploads Legacy as a
        // substitute for an explicitly source consumer normal policy.
        match source_normal_policy(object.metadata, effective_style) {
            Some(policy) => {
                if let Some(warm) = object.selected_normals {
                    if !selected_seal.is_some_and(|seal| seal.matches(mesh, warm))
                        && !selected_render_matches_mesh(mesh, warm)
                    {
                        return Err(
                            "Selected-refresh Source render state is stale; explicitly reset it"
                                .into(),
                        );
                    }
                    self.source_normals = None;
                    self.selected_normals = Some(warm.clone());
                } else {
                    self.selected_normals = None;
                    // A stale field cannot draw the new mesh. Release it before
                    // rebuilding so a million-vertex edit does not charge both
                    // old retained data and new scratch to the per-field cap.
                    // The caller removes this GPU object if rebuilding fails.
                    if self
                        .source_normals
                        .as_ref()
                        .is_some_and(|old| old.report().policy != policy || !old.matches_mesh(mesh))
                    {
                        self.source_normals = None;
                    }
                    obrush_core::SourceNormalField::refresh(
                        &mut self.source_normals,
                        mesh,
                        policy,
                    )?;
                }
            }
            None => {
                self.source_normals = None;
                self.selected_normals = None;
            }
        }
        let mut selected_chunk_witness = false;
        let mut selected_full_scan = false;
        let sparse_scan = if reuse_triangles {
            match (
                self.source_upload_cache.as_ref(),
                previous_selected.as_ref(),
                self.selected_normals.as_ref(),
            ) {
                (Some(cache), Some(old_normals), Some(normals))
                    if old_normals.shares_topology_storage(normals) =>
                {
                    let chunks = normals.render_selected_chunks_from(old_normals);
                    selected_chunk_witness = chunks.is_some();
                    selected_full_scan = chunks.is_none();
                    Some(selected_row_upload_scan(
                        &cache.vertices,
                        mesh,
                        normals,
                        chunks.as_ref(),
                        self.source_mask_snapshot.as_ref(),
                        Some(&cache.materials),
                    )?)
                }
                _ => None,
            }
        } else {
            None
        };
        let raw_masks_match = sparse_scan.as_ref().and_then(|scan| scan.raw_masks_match);
        let materials_match = sparse_scan.as_ref().and_then(|scan| scan.materials_match);
        let sparse_ranges = sparse_scan.and_then(|scan| scan.ranges);
        let mut vertices = if sparse_ranges.is_some() {
            None
        } else if let Some(warm) = &self.selected_normals {
            Some(
                if let Some(seal) = selected_seal.filter(|seal| seal.matches(mesh, warm)) {
                    seal.packed_rows()?
                } else {
                    packed_geometry_selected_rows(mesh, warm)?
                },
            )
        } else {
            Some(packed_geometry_rows(mesh, self.source_normals.as_ref())?)
        };
        let reuse_materials = materials_match.unwrap_or_else(|| {
            self.source_upload_cache.as_ref().is_some_and(|cache| {
                match (&cache.materials, &mesh.materials) {
                    (None, None) => true,
                    (Some(old), Some(new)) => {
                        bytemuck::cast_slice::<_, u8>(old) == bytemuck::cast_slice::<_, u8>(new)
                    }
                    _ => false,
                }
            })
        });
        let reuse_source_masks = self.source_masks.is_some()
            && self
                .source_mask_snapshot
                .as_ref()
                .is_some_and(|snapshot| raw_masks_match.unwrap_or_else(|| snapshot.matches(mesh)));
        if !reuse_source_masks {
            self.source_mask_snapshot = None;
            if let Some(buffer) = self.source_masks.take() {
                unsafe {
                    gl.bind_vertex_array(Some(self.vertex_array));
                    gl.disable_vertex_attrib_array(6);
                    gl.vertex_attrib_1_f32(6, 1.0);
                    gl.delete_buffer(buffer);
                }
            }
        }
        if let Some(uv) = self.uv.take() {
            unsafe {
                uv.destroy(gl);
            }
        }
        let (bounds, incremental_bounds) = CachedUploadBounds::updated(
            mesh,
            self.bounds.as_ref(),
            self.source_upload_cache
                .as_ref()
                .map(|cache| cache.vertices.as_slice()),
            sparse_ranges.as_deref(),
        );
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            if let Some(ranges) = sparse_ranges.as_ref() {
                let normals = self
                    .selected_normals
                    .as_ref()
                    .expect("Sparse Source upload requires selected normals");
                for range in ranges {
                    let mut rows = Vec::new();
                    rows.try_reserve_exact(range.len())
                        .map_err(|_| "Cannot allocate bounded sparse Source upload rows")?;
                    for index in range.clone() {
                        rows.push(packed_selected_row(mesh, normals, index));
                    }
                    gl.buffer_sub_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        i32::try_from(range.start * std::mem::size_of::<[f32; 10]>())
                            .map_err(|_| "Sparse Source upload offset exceeds GL range")?,
                        bytemuck::cast_slice(&rows),
                    );
                }
            } else {
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(vertices.as_ref().expect("Full upload rows exist")),
                    glow::DYNAMIC_DRAW,
                );
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.materials));
            // Plain legacy meshes use a uniform default, without allocating or
            // repeatedly uploading an unnecessary all-default material array.
            if mesh.materials.is_some() {
                gl.enable_vertex_attrib_array(4);
            } else {
                gl.disable_vertex_attrib_array(4);
            }
            let materials = mesh.materials.as_deref().unwrap_or(&[]);
            if !reuse_materials {
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(materials),
                    glow::DYNAMIC_DRAW,
                );
            }
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.triangles));
            if !reuse_triangles {
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    bytemuck::cast_slice(&mesh.triangles),
                    glow::DYNAMIC_DRAW,
                );
            }
        }
        self.triangle_index_count = (mesh.triangles.len() * 3).min(i32::MAX as usize) as i32;
        // Old wire storage may be larger than this revision's reservation.
        // Release it now rather than retaining it when wireframe is disabled.
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.edges));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &[], glow::DYNAMIC_DRAW);
            geometry_gl_result(gl, "geometry upload")?;
        }
        if source_upload_cache_admitted(mesh, object.active, self.selected_normals.is_some()) {
            if let Some(ranges) = sparse_ranges {
                let normals = self
                    .selected_normals
                    .as_ref()
                    .expect("Sparse Source upload requires selected normals");
                let cache = self
                    .source_upload_cache
                    .as_mut()
                    .expect("Sparse Source upload requires prior packed rows");
                for range in ranges {
                    for index in range {
                        cache.vertices[index] = packed_selected_row(mesh, normals, index);
                    }
                }
                if !reuse_materials {
                    cache.materials = copied_material_rows(mesh)?;
                }
                if cache
                    .storage_bytes()
                    .is_none_or(|bytes| bytes > MAX_SOURCE_UPLOAD_CACHE_BYTES)
                {
                    self.source_upload_cache = None;
                }
            } else {
                let candidate = CachedSourceRows {
                    vertices: vertices.take().expect("Full Source upload rows exist"),
                    materials: copied_material_rows(mesh)?,
                };
                self.source_upload_cache = candidate
                    .storage_bytes()
                    .filter(|&bytes| bytes <= MAX_SOURCE_UPLOAD_CACHE_BYTES)
                    .map(|_| candidate);
            }
        } else {
            self.source_upload_cache = None;
        }
        self.center = (bounds.low + bounds.high) * 0.5;
        self.bounds = Some(bounds);
        self.incremental_bounds_updates = self
            .incremental_bounds_updates
            .saturating_add(u64::from(incremental_bounds));
        self.full_bounds_scans = self
            .full_bounds_scans
            .saturating_add(u64::from(!incremental_bounds));
        self.edge_index_count = 0;
        self.identity.update(object);
        self.uploads = self.uploads.saturating_add(1);
        self.selected_chunk_witness_uploads = self
            .selected_chunk_witness_uploads
            .saturating_add(u64::from(selected_chunk_witness));
        self.selected_full_scan_uploads = self
            .selected_full_scan_uploads
            .saturating_add(u64::from(selected_full_scan));
        self.source_mask_buffer_reuses = self
            .source_mask_buffer_reuses
            .saturating_add(u64::from(reuse_source_masks));
        self.edges_dirty = true;
        Ok(())
    }

    unsafe fn upload_uv(&mut self, gl: &glow::Context, mesh: &Mesh) -> Result<(), String> {
        // Aggregate preflight was performed before GPU mutation. This per-object
        // guard independently bounds CPU seam scratch and individual GPU rows.
        let packed = if let Some(normals) = &self.selected_normals {
            PackedUvMesh::build_with_source_geometry_normals(mesh, MAX_SCENE_GPU_BYTES, normals)?
        } else if let Some(normals) = &self.source_normals {
            PackedUvMesh::build_with_source_normals(mesh, MAX_SCENE_GPU_BYTES, normals)?
        } else {
            PackedUvMesh::build(mesh, MAX_SCENE_GPU_BYTES)?
        };
        self.uv = Some(unsafe { GpuUvMesh::new(gl, packed)? });
        Ok(())
    }

    unsafe fn ensure_source_masks(
        &mut self,
        gl: &glow::Context,
        mesh: &Mesh,
    ) -> Result<(), String> {
        if self.source_masks.is_none() {
            let snapshot = self
                .source_upload_cache
                .as_ref()
                .map(|_| SourceMaskSnapshot::capture(mesh))
                .transpose()?;
            let words = source_mask_words(mesh, None)?;
            self.source_masks =
                Some(unsafe { upload_source_mask_buffer(gl, self.vertex_array, &words)? });
            self.source_mask_snapshot = snapshot;
            self.source_mask_buffer_uploads = self.source_mask_buffer_uploads.saturating_add(1);
        }
        if let Some(uv) = self.uv.as_mut() {
            unsafe {
                uv.ensure_source_masks(gl, mesh)?;
            }
        }
        Ok(())
    }

    unsafe fn upload_edges(
        &mut self,
        gl: &glow::Context,
        mesh: &Mesh,
        source_faces: bool,
    ) -> Result<(), String> {
        let indices = if source_faces {
            crate::source_wire_view::original_wire_edges(mesh)?
        } else {
            let mut seen = HashSet::with_capacity(mesh.triangles.len());
            let mut indices = Vec::<[u32; 2]>::with_capacity(mesh.triangles.len());
            for &[a, b, c] in &mesh.triangles {
                for (start, end) in [(a, b), (b, c), (c, a)] {
                    let edge = [start.min(end), start.max(end)];
                    if seen.insert(edge) {
                        indices.push(edge);
                    }
                }
            }
            indices
        };
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.edges));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                glow::DYNAMIC_DRAW,
            );
            geometry_gl_result(gl, "wire upload")?;
        }
        self.edge_index_count = (indices.len() * 2).min(i32::MAX as usize) as i32;
        self.edges_dirty = false;
        self.source_wire_edges = Some(source_faces);
        Ok(())
    }

    unsafe fn destroy(&self, gl: &glow::Context) {
        unsafe {
            if let Some(uv) = self.uv.as_ref() {
                uv.destroy(gl);
            }
            if let Some(buffer) = self.source_masks {
                gl.delete_buffer(buffer);
            }
            gl.delete_buffer(self.edges);
            gl.delete_buffer(self.triangles);
            gl.delete_buffer(self.vertices);
            gl.delete_buffer(self.materials);
            gl.delete_vertex_array(self.vertex_array);
        }
    }
}

impl GpuUvMesh {
    unsafe fn ensure_source_masks(
        &mut self,
        gl: &glow::Context,
        mesh: &Mesh,
    ) -> Result<(), String> {
        if self.source_masks.is_none() {
            let words = source_mask_words(mesh, Some(&self.correspondence))?;
            self.source_masks =
                Some(unsafe { upload_source_mask_buffer(gl, self.vertex_array, &words)? });
        }
        Ok(())
    }
    unsafe fn new(gl: &glow::Context, packed: PackedUvMesh) -> Result<Self, String> {
        let mut array = None;
        let mut buffers = [None; 4];
        let result = (|| unsafe {
            let vao = gl.create_vertex_array()?;
            array = Some(vao);
            gl.bind_vertex_array(Some(vao));
            for buffer in &mut buffers {
                *buffer = Some(gl.create_buffer()?);
            }
            let vertices = buffers[0].unwrap();
            let materials = buffers[1].unwrap();
            let coordinates = buffers[2].unwrap();
            let triangles = buffers[3].unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&packed.vertices),
                glow::DYNAMIC_DRAW,
            );
            for (location, size, offset) in [(0, 3, 0), (1, 3, 12), (2, 1, 24), (3, 3, 28)] {
                gl.enable_vertex_attrib_array(location);
                gl.vertex_attrib_pointer_f32(location, size, glow::FLOAT, false, 40, offset);
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(materials));
            gl.vertex_attrib_pointer_f32(4, 2, glow::FLOAT, false, 8, 0);
            if packed.materials.is_some() {
                gl.enable_vertex_attrib_array(4);
            } else {
                gl.disable_vertex_attrib_array(4);
            }
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(packed.materials.as_deref().unwrap_or(&[])),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(coordinates));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&packed.uv),
                glow::DYNAMIC_DRAW,
            );
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 2, glow::FLOAT, false, 8, 0);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(triangles));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&packed.triangles),
                glow::DYNAMIC_DRAW,
            );
            geometry_gl_result(gl, "UV seam upload")?;
            Ok(Self {
                vertex_array: vao,
                vertices,
                materials,
                coordinates,
                triangles,
                index_count: (packed.triangles.len() * 3) as i32,
                correspondence_fingerprint: correspondence_fingerprint(&packed.correspondence),
                correspondence: packed.correspondence,
                source_masks: None,
            })
        })();
        if result.is_err() {
            unsafe {
                for buffer in buffers.into_iter().flatten() {
                    gl.delete_buffer(buffer);
                }
                if let Some(vao) = array {
                    gl.delete_vertex_array(vao);
                }
            }
        }
        result.map_err(|e: String| format!("Cannot upload render-only UV seams: {e}"))
    }
    unsafe fn destroy(&self, gl: &glow::Context) {
        unsafe {
            if let Some(buffer) = self.source_masks {
                gl.delete_buffer(buffer);
            }
            for buffer in [
                self.vertices,
                self.materials,
                self.coordinates,
                self.triangles,
            ] {
                gl.delete_buffer(buffer);
            }
            gl.delete_vertex_array(self.vertex_array);
        }
    }
}

unsafe fn upload_source_mask_buffer(
    gl: &glow::Context,
    vao: glow::VertexArray,
    words: &[f32],
) -> Result<glow::Buffer, String> {
    unsafe {
        let buffer = gl.create_buffer()?;
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(words),
            glow::DYNAMIC_DRAW,
        );
        gl.enable_vertex_attrib_array(6);
        gl.vertex_attrib_pointer_f32(6, 1, glow::FLOAT, false, 4, 0);
        if let Err(error) = geometry_gl_result(gl, "canonical source mask upload") {
            gl.delete_buffer(buffer);
            return Err(error);
        }
        Ok(buffer)
    }
}

unsafe fn geometry_gl_result(gl: &glow::Context, operation: &str) -> Result<(), String> {
    let error = unsafe { gl.get_error() };
    if error == glow::NO_ERROR {
        return Ok(());
    }
    // Drain bounded follow-on errors before another object tries to upload.
    // Resource cleanup and the visible scene diagnostic preserve the failure.
    for _ in 0..8 {
        if unsafe { gl.get_error() } == glow::NO_ERROR {
            break;
        }
    }
    Err(format!("GPU {operation} failed (0x{error:x})"))
}

/// Bounds transient GPU storage independently of mesh size. RGBA8 and depth24
/// are budgeted as four bytes each, plus a four-byte single-sample resolve.
fn antialias_target_fits(
    width: i32,
    height: i32,
    samples: i32,
    max_samples: i32,
    max_side: i32,
) -> bool {
    const MAX_BYTES: u64 = 256 * 1024 * 1024;
    width > 0
        && height > 0
        && width <= max_side
        && height <= max_side
        && samples >= 2
        && samples <= max_samples
        && width as u64 * height as u64 <= MAX_BYTES / (8 * samples as u64 + 4)
}

/// One RGBA8 image and one depth24 attachment, capped independently of the
/// geometry and antialias budgets. Never allocate from an unbounded ratio.
fn resolution_target_fits(width: i32, height: i32, max_side: i32) -> bool {
    const MAX_BYTES: u64 = 128 * 1024 * 1024;
    width > 0
        && height > 0
        && width <= max_side
        && height <= max_side
        && (width as u64) * (height as u64) <= MAX_BYTES / 8
}

struct ResolutionTarget {
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    depth: glow::Renderbuffer,
}

impl ResolutionTarget {
    unsafe fn new(gl: &glow::Context, width: i32, height: i32) -> Result<Self, String> {
        let (mut framebuffer, mut texture, mut depth) = (None, None, None);
        let result = (|| unsafe {
            let fb = gl.create_framebuffer()?;
            framebuffer = Some(fb);
            let image = gl.create_texture()?;
            texture = Some(image);
            let depth_buffer = gl.create_renderbuffer()?;
            depth = Some(depth_buffer);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(image));
            for (name, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(image),
                0,
            );
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_buffer));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth_buffer),
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!(
                    "Viewport resolution framebuffer is incomplete (0x{status:x})"
                ));
            }
            Ok(Self {
                framebuffer: fb,
                texture: image,
                depth: depth_buffer,
            })
        })();
        if result.is_err() {
            unsafe {
                if let Some(value) = depth {
                    gl.delete_renderbuffer(value);
                }
                if let Some(value) = texture {
                    gl.delete_texture(value);
                }
                if let Some(value) = framebuffer {
                    gl.delete_framebuffer(value);
                }
                for _ in 0..8 {
                    if gl.get_error() == glow::NO_ERROR {
                        break;
                    }
                }
            }
        }
        result
    }

    unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            gl.delete_renderbuffer(self.depth);
            gl.delete_texture(self.texture);
            gl.delete_framebuffer(self.framebuffer);
        }
    }
}

struct AntialiasTarget {
    multisample: glow::Framebuffer,
    color: glow::Renderbuffer,
    depth: glow::Renderbuffer,
    resolved: glow::Framebuffer,
    texture: glow::Texture,
    samples: i32,
}

impl AntialiasTarget {
    unsafe fn new(
        gl: &glow::Context,
        width: i32,
        height: i32,
        samples: i32,
    ) -> Result<Self, String> {
        let (mut multisample, mut color, mut depth, mut resolved, mut texture) =
            (None, None, None, None, None);
        // Retain each successfully allocated handle so every partial failure
        // releases its resources, including unsupported multisample formats.
        let result = (|| unsafe {
            let ms = gl.create_framebuffer()?;
            multisample = Some(ms);
            let color_buffer = gl.create_renderbuffer()?;
            color = Some(color_buffer);
            let depth_buffer = gl.create_renderbuffer()?;
            depth = Some(depth_buffer);
            let single = gl.create_framebuffer()?;
            resolved = Some(single);
            let image = gl.create_texture()?;
            texture = Some(image);

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(ms));
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(color_buffer));
            gl.renderbuffer_storage_multisample(
                glow::RENDERBUFFER,
                samples,
                glow::RGBA8,
                width,
                height,
            );
            // Desktop drivers may round a requested sample count upward.
            // Report and budget the actual attachment count, not the request.
            let actual_samples =
                gl.get_renderbuffer_parameter_i32(glow::RENDERBUFFER, glow::RENDERBUFFER_SAMPLES);
            if !antialias_target_fits(width, height, actual_samples, i32::MAX, i32::MAX) {
                return Err("Actual multisample storage exceeds the viewport memory budget".into());
            }
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(color_buffer),
            );
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_buffer));
            gl.renderbuffer_storage_multisample(
                glow::RENDERBUFFER,
                actual_samples,
                glow::DEPTH_COMPONENT24,
                width,
                height,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth_buffer),
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!(
                    "{samples}x viewport framebuffer is incomplete (0x{status:x})"
                ));
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(image));
            for (name, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(single));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(image),
                0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!(
                    "Viewport resolve framebuffer is incomplete (0x{status:x})"
                ));
            }
            Ok(Self {
                multisample: ms,
                color: color_buffer,
                depth: depth_buffer,
                resolved: single,
                texture: image,
                samples: actual_samples,
            })
        })();
        if result.is_err() {
            unsafe {
                if let Some(value) = texture {
                    gl.delete_texture(value);
                }
                if let Some(value) = resolved {
                    gl.delete_framebuffer(value);
                }
                if let Some(value) = depth {
                    gl.delete_renderbuffer(value);
                }
                if let Some(value) = color {
                    gl.delete_renderbuffer(value);
                }
                if let Some(value) = multisample {
                    gl.delete_framebuffer(value);
                }
                // Failed allocation may report INVALID_OPERATION/OUT_OF_MEMORY.
                // Drain those errors here before retrying a smaller sample count.
                for _ in 0..8 {
                    if gl.get_error() == glow::NO_ERROR {
                        break;
                    }
                }
            }
        }
        result
    }

    unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
            gl.delete_framebuffer(self.resolved);
            gl.delete_renderbuffer(self.depth);
            gl.delete_renderbuffer(self.color);
            gl.delete_framebuffer(self.multisample);
        }
    }
}

/// State shared with egui. The framebuffer is restored using the painter's
/// supplied target. WebGL's null object getters panic in glow 0.16, so on that
/// target we clear our bindings and egui reinstates its objects after callbacks.
struct GlState {
    #[cfg(not(target_arch = "wasm32"))]
    program: Option<glow::Program>,
    #[cfg(not(target_arch = "wasm32"))]
    vertex_array: Option<glow::VertexArray>,
    #[cfg(not(target_arch = "wasm32"))]
    array_buffer: Option<glow::Buffer>,
    #[cfg(not(target_arch = "wasm32"))]
    renderbuffer: Option<glow::Renderbuffer>,
    #[cfg(not(target_arch = "wasm32"))]
    textures: [Option<glow::Texture>; 4],
    #[cfg(not(target_arch = "wasm32"))]
    multisample: bool,
    active_texture: u32,
    clear_color: [f32; 4],
    blend_equation: [u32; 2],
    blend_factors: [u32; 4],
    viewport: [i32; 4],
    scissor: [i32; 4],
    color_mask: [bool; 4],
    depth_range: [f32; 2],
    depth_clear: f32,
    depth_func: u32,
    depth_mask: bool,
    depth_test: bool,
    scissor_test: bool,
    blend: bool,
    cull_face: bool,
    cull_face_mode: i32,
    front_face: i32,
    polygon_offset_fill: bool,
    polygon_offset_factor: f32,
    polygon_offset_units: f32,
    line_width: f32,
}

impl GlState {
    unsafe fn capture(gl: &glow::Context) -> Self {
        let mut viewport = [0; 4];
        let mut scissor = [0; 4];
        let mut depth_range = [0.0; 2];
        let mut clear_color = [0.0; 4];
        unsafe {
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor);
            gl.get_parameter_f32_slice(glow::DEPTH_RANGE, &mut depth_range);
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut clear_color);
            let active_texture = gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
            #[cfg(not(target_arch = "wasm32"))]
            let textures = std::array::from_fn(|unit| {
                gl.active_texture(glow::TEXTURE0 + unit as u32);
                let binding = gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
                gl.active_texture(active_texture);
                binding
            });
            Self {
                #[cfg(not(target_arch = "wasm32"))]
                program: gl.get_parameter_program(glow::CURRENT_PROGRAM),
                #[cfg(not(target_arch = "wasm32"))]
                vertex_array: gl.get_parameter_vertex_array(glow::VERTEX_ARRAY_BINDING),
                #[cfg(not(target_arch = "wasm32"))]
                array_buffer: gl.get_parameter_buffer(glow::ARRAY_BUFFER_BINDING),
                #[cfg(not(target_arch = "wasm32"))]
                renderbuffer: gl.get_parameter_renderbuffer(glow::RENDERBUFFER_BINDING),
                #[cfg(not(target_arch = "wasm32"))]
                textures,
                #[cfg(not(target_arch = "wasm32"))]
                multisample: gl.is_enabled(glow::MULTISAMPLE),
                active_texture,
                clear_color,
                blend_equation: [
                    gl.get_parameter_i32(glow::BLEND_EQUATION_RGB) as u32,
                    gl.get_parameter_i32(glow::BLEND_EQUATION_ALPHA) as u32,
                ],
                blend_factors: [
                    gl.get_parameter_i32(glow::BLEND_SRC_RGB) as u32,
                    gl.get_parameter_i32(glow::BLEND_DST_RGB) as u32,
                    gl.get_parameter_i32(glow::BLEND_SRC_ALPHA) as u32,
                    gl.get_parameter_i32(glow::BLEND_DST_ALPHA) as u32,
                ],
                viewport,
                scissor,
                color_mask: gl.get_parameter_bool_array(glow::COLOR_WRITEMASK),
                depth_range,
                depth_clear: gl.get_parameter_f32(glow::DEPTH_CLEAR_VALUE),
                depth_func: gl.get_parameter_i32(glow::DEPTH_FUNC) as u32,
                depth_mask: gl.get_parameter_bool(glow::DEPTH_WRITEMASK),
                depth_test: gl.is_enabled(glow::DEPTH_TEST),
                scissor_test: gl.is_enabled(glow::SCISSOR_TEST),
                blend: gl.is_enabled(glow::BLEND),
                cull_face: gl.is_enabled(glow::CULL_FACE),
                cull_face_mode: gl.get_parameter_i32(glow::CULL_FACE_MODE),
                front_face: gl.get_parameter_i32(glow::FRONT_FACE),
                polygon_offset_fill: gl.is_enabled(glow::POLYGON_OFFSET_FILL),
                polygon_offset_factor: gl.get_parameter_f32(glow::POLYGON_OFFSET_FACTOR),
                polygon_offset_units: gl.get_parameter_f32(glow::POLYGON_OFFSET_UNITS),
                line_width: gl.get_parameter_f32(glow::LINE_WIDTH),
            }
        }
    }

    unsafe fn restore(self, gl: &glow::Context) {
        unsafe {
            #[cfg(not(target_arch = "wasm32"))]
            {
                gl.use_program(self.program);
                // The index-buffer binding belongs to the VAO. Only our VAO
                // was edited, so switching back restores the host's indices.
                // An explicit ELEMENT_ARRAY_BUFFER bind with VAO0 would be
                // invalid in a desktop core-profile context.
                gl.bind_vertex_array(self.vertex_array);
                gl.bind_buffer(glow::ARRAY_BUFFER, self.array_buffer);
                gl.bind_renderbuffer(glow::RENDERBUFFER, self.renderbuffer);
                for (unit, texture) in self.textures.into_iter().enumerate() {
                    gl.active_texture(glow::TEXTURE0 + unit as u32);
                    gl.bind_texture(glow::TEXTURE_2D, texture);
                }
                if self.multisample {
                    gl.enable(glow::MULTISAMPLE);
                } else {
                    gl.disable(glow::MULTISAMPLE);
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                // Leave our VAO untouched while removing our bindings. Egui
                // immediately reinstates its program, VAO, vertex/index
                // buffers in prepare_painting after every paint callback.
                gl.use_program(None);
                gl.bind_vertex_array(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
                gl.bind_renderbuffer(glow::RENDERBUFFER, None);
                for unit in 0..4 {
                    gl.active_texture(glow::TEXTURE0 + unit);
                    gl.bind_texture(glow::TEXTURE_2D, None);
                }
            }
            gl.active_texture(self.active_texture);
            gl.clear_color(
                self.clear_color[0],
                self.clear_color[1],
                self.clear_color[2],
                self.clear_color[3],
            );
            gl.blend_equation_separate(self.blend_equation[0], self.blend_equation[1]);
            gl.blend_func_separate(
                self.blend_factors[0],
                self.blend_factors[1],
                self.blend_factors[2],
                self.blend_factors[3],
            );
            gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            gl.scissor(
                self.scissor[0],
                self.scissor[1],
                self.scissor[2],
                self.scissor[3],
            );
            gl.cull_face(self.cull_face_mode as u32);
            gl.front_face(self.front_face as u32);
            gl.color_mask(
                self.color_mask[0],
                self.color_mask[1],
                self.color_mask[2],
                self.color_mask[3],
            );
            gl.depth_range_f32(self.depth_range[0], self.depth_range[1]);
            gl.clear_depth_f32(self.depth_clear);
            gl.depth_func(self.depth_func);
            gl.depth_mask(self.depth_mask);
            gl.polygon_offset(self.polygon_offset_factor, self.polygon_offset_units);
            gl.line_width(self.line_width);
            for (capability, enabled) in [
                (glow::DEPTH_TEST, self.depth_test),
                (glow::SCISSOR_TEST, self.scissor_test),
                (glow::BLEND, self.blend),
                (glow::CULL_FACE, self.cull_face),
                (glow::POLYGON_OFFSET_FILL, self.polygon_offset_fill),
            ] {
                if enabled {
                    gl.enable(capability);
                } else {
                    gl.disable(capability);
                }
            }
        }
    }
}

pub(crate) unsafe fn compile_program(
    gl: &glow::Context,
    header: &str,
    vertex_source: &str,
    fragment_source: &str,
    name: &str,
) -> Result<glow::Program, String> {
    unsafe {
        let program = gl
            .create_program()
            .map_err(|error| format!("Cannot create {name} shader: {error}"))?;
        let mut shaders = Vec::with_capacity(2);
        for (stage, source, label) in [
            (glow::VERTEX_SHADER, vertex_source, "vertex"),
            (glow::FRAGMENT_SHADER, fragment_source, "fragment"),
        ] {
            let shader = match gl.create_shader(stage) {
                Ok(shader) => shader,
                Err(error) => {
                    for shader in shaders {
                        gl.delete_shader(shader);
                    }
                    gl.delete_program(program);
                    return Err(format!("Cannot create {name} {label} shader: {error}"));
                }
            };
            gl.shader_source(shader, &format!("{header}{source}"));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                for shader in shaders {
                    gl.delete_shader(shader);
                }
                gl.delete_program(program);
                return Err(format!("{name} {label} shader failed to compile:\n{log}"));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        for (location, name) in [
            (0, "a_position"),
            (1, "a_normal"),
            (2, "a_mask"),
            (3, "a_color"),
            (4, "a_material"),
            (5, "a_uv"),
            (6, "a_source_unprotected"),
        ] {
            gl.bind_attrib_location(program, location, name);
        }
        gl.link_program(program);
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(format!("{name} shader failed to link:\n{log}"));
        }
        Ok(program)
    }
}

// SculptGL's default createGridArray makes 22 lines on each axis, from
// -0.5 to 0.5. Scene.initGrid translates Y by -0.45 and scales by 2.5.
// Generate the same fixed 88 vertices without an extra GPU vertex buffer.
const GRID_VERTEX_SHADER: &str = r#"
uniform mat4 u_view_projection;
void main() {
    int line = gl_VertexID / 2;
    float along = -1.25 + float(line % 22) * (2.5 / 21.0);
    float end_point = (gl_VertexID % 2 == 0) ? -1.25 : 1.25;
    vec3 world = line < 22
        ? vec3(along, -0.45, end_point)
        : vec3(end_point, -0.45, along);
    gl_Position = u_view_projection * vec4(world, 1.0);
}
"#;

const GRID_FRAGMENT_SHADER: &str = r#"
out vec4 out_color;
void main() { out_color = vec4(0.04, 0.04, 0.04, 1.0); }
"#;

const COMPOSITE_VERTEX_SHADER: &str = r#"
out vec2 v_uv;
void main() {
    // A single fullscreen triangle needs no vertex/index upload or extra VAO.
    v_uv = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    gl_Position = vec4(v_uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const COMPOSITE_FRAGMENT_SHADER: &str = r#"
in vec2 v_uv;
uniform sampler2D u_texture;
out vec4 out_color;
void main() {
    out_color = texture(u_texture, v_uv);
}
"#;

const VERTEX_SHADER: &str = r#"
in vec3 a_position;
in vec3 a_normal;
in float a_mask;
in float a_source_unprotected;
in vec3 a_color;
in vec2 a_material;
in vec2 a_uv;
uniform mat4 u_view_projection;
uniform mat4 u_view;
uniform mat3 u_normal_matrix;
uniform bool u_linear_pipeline;
uniform bool u_wire;
uniform vec3 u_preview_translation;
out vec3 v_normal;
out vec3 v_source_normal;
out float v_source_unprotected;
out vec3 v_position;
out vec3 v_preserved_eye;
out float v_mask;
out vec3 v_color;
out vec2 v_material;
out vec2 v_uv;
void main() {
    vec3 position = a_position + u_preview_translation;
    gl_Position = u_view_projection * vec4(position, 1.0);
    if(u_linear_pipeline&&u_wire) gl_Position.w+=0.0001;
    v_position = (u_view * vec4(position, 1.0)).xyz;
    v_normal = u_normal_matrix * a_normal;
    v_source_normal=normalize(v_normal);
    v_source_unprotected=a_source_unprotected;
    v_preserved_eye = v_position/max(1.0,abs(u_view[3][2]));
    v_mask = a_mask;
    v_color = a_color;
    v_material = a_material;
    v_uv = a_uv;
}
"#;

const FRAGMENT_SHADER: &str = r#"
in vec3 v_normal;
in vec3 v_source_normal;
in float v_source_unprotected;
uniform bool u_linear_pipeline;
uniform bool u_darken_unselected;
uniform bool u_auxiliary_flat;
uniform bool u_selected;
uniform bool u_symmetry_line;
uniform vec3 u_symmetry_origin;
uniform vec3 u_symmetry_normal;
in vec3 v_position;
in vec3 v_preserved_eye;
in float v_mask;
in vec3 v_color;
in vec2 v_material;
in vec2 v_uv;
uniform sampler2D u_uv_texture;
uniform bool u_has_uv_texture;
uniform bool u_has_uv_data;
uniform float u_curvature;
uniform float u_curvature_fov;
uniform float u_opacity;
uniform vec3 u_color;
uniform bool u_show_mask;
uniform bool u_use_vertex_color;
uniform bool u_wire;
uniform bool u_active;
uniform int u_shading_mode;
uniform bool u_flat_shading;
uniform float u_exposure;
uniform bool u_has_material_data;
uniform vec2 u_default_material;
out vec4 out_color;

// Independent direct-light GGX/Schlick evaluation. Environment-map lighting
// and a reference-matched filmic pipeline remain separate viewport work.
vec3 pbrLight(vec3 n, vec3 v, vec3 l, vec3 radiance, vec3 albedo, float roughness, float metallic) {
    vec3 h = normalize(v+l);
    float nv = max(dot(n,v),0.0001);
    float nl = max(dot(n,l),0.0);
    float nh = max(dot(n,h),0.0);
    float vh = max(dot(v,h),0.0);
    float alpha = roughness*roughness;
    float a2 = alpha*alpha;
    float denom = nh*nh*(a2-1.0)+1.0;
    float distribution = a2/max(3.14159265359*denom*denom,0.000000000001);
    float k = (roughness+1.0)*(roughness+1.0)*0.125;
    float geometry = nv/(nv*(1.0-k)+k) * nl/max(nl*(1.0-k)+k,0.0001);
    vec3 f0 = mix(vec3(0.04),albedo,metallic);
    vec3 fresnel = f0+(vec3(1.0)-f0)*pow(1.0-vh,5.0);
    vec3 specular = distribution*geometry*fresnel/max(4.0*nv*nl,0.0001);
    vec3 diffuse = (vec3(1.0)-fresnel)*(1.0-metallic)*albedo/3.14159265359;
    return (diffuse+specular)*radiance*nl;
}

vec3 safeNormal(vec3 n) { return dot(n,n)>1e-12?normalize(n):vec3(0.0,0.0,1.0); }
vec3 uvLinear(vec3 rgb) {
    return mix(pow((rgb+0.055)/1.055,vec3(2.4)),rgb/12.92,lessThanEqual(rgb,vec3(0.04045)));
}
vec3 checker(vec2 uv) {
    vec2 cell=floor(fract(uv)*8.0);
    float alternating=mod(cell.x+cell.y,2.0);
    return mix(mix(vec3(0.18,0.23,0.31),vec3(0.85,0.87,0.90),alternating),vec3(fract(uv),0.2),0.2);
}
vec4 finishColor(vec3 color) {
    if(u_curvature>=0.001) {
        vec3 n=safeNormal(v_normal),dx=dFdx(n),dy=dFdy(n);
        float depth=u_curvature_fov>0.0?length(v_position)*u_curvature_fov:-u_curvature_fov;
        float curvature=(cross(n-dx,n+dx).y-cross(n-dy,n+dy).x)*u_curvature*80.0/max(depth,0.0001);
        color=mix(mix(color,color*0.3,clamp(-curvature*15.0,0.0,1.0)),color*2.0,clamp(curvature*25.0,0.0,1.0));
    }
    return vec4(color*u_opacity,u_opacity);
}

vec3 brightenSymmetryGuide(vec3 color) {
    if(u_symmetry_line&&abs(dot(u_symmetry_normal,v_position-u_symmetry_origin))<0.15)
        color=min(color*1.5,vec3(1.0));
    return color;
}
vec4 finishSourceColor(vec3 color) {
    if(u_curvature>=0.001) {
        vec3 n=normalize(v_source_normal),dx=dFdx(n),dy=dFdy(n);
        float depth=u_curvature_fov>0.0?length(v_position)*u_curvature_fov:-u_curvature_fov;
        float curvature=(cross(n-dx,n+dx).y-cross(n-dy,n+dy).x)*u_curvature*80.0/depth;
        color=mix(mix(color,color*0.3,clamp(-curvature*15.0,0.0,1.0)),color*2.0,clamp(curvature*25.0,0.0,1.0));
    }
    if(u_darken_unselected&&!u_selected) color*=0.3;
    color*=u_show_mask?(0.3+0.7*v_source_unprotected):1.0;
    color=brightenSymmetryGuide(color);
    return u_opacity!=1.0?vec4(color*u_opacity,u_opacity):linearToRgbm(color);
}
vec4 preservedStudioInLinearTarget(vec3 normal,vec3 tint) {
    vec3 view=normalize(-v_position),key=normalize(vec3(-0.55,0.75,1.0)),fill=normalize(vec3(0.8,0.1,0.8)),rim=normalize(vec3(0.4,0.6,-0.8));
    vec3 color=tint;
    if(u_show_mask) color=mix(color,vec3(0.16,0.13,0.20),clamp(v_mask,0.0,1.0)*0.82);
    vec3 albedo=pow(max(color,vec3(0.0)),vec3(2.2));
    float diffuse=max(dot(normal,key),0.0),secondary=max(dot(normal,fill),0.0),edge=max(dot(normal,rim),0.0);
    float highlight=pow(max(dot(normal,normalize(key+view)),0.0),36.0),fresnel=pow(1.0-max(dot(normal,view),0.0),3.0);
    vec3 light=vec3(0.24,0.27,0.32)+diffuse*vec3(0.85,0.81,0.75)+secondary*vec3(0.15,0.18,0.24)+edge*vec3(0.18,0.20,0.25);
    vec3 shaded=albedo*light+vec3(highlight*0.11)+fresnel*vec3(0.025,0.035,0.055);
    if(!u_active&&u_darken_unselected) shaded*=0.90;
    // Studio is an OBrush display-space mode. Preserve its original response
    // when it shares a linear scene with imported source styles.
    vec4 display=finishColor(pow(max(shaded,vec3(0.0)),vec3(1.0/2.2)));
    vec3 linear=displayToLinear(display.a>0.0?display.rgb/display.a:vec3(0.0));
    // Only the explicit guide control adds this view effect to Studio.
    linear=brightenSymmetryGuide(linear);
    return display.a!=1.0?vec4(linear*display.a,display.a):linearToRgbm(linear);
}
vec4 sourceSurfaceColor() {
    if(u_wire) return vec4(0.0,0.0,0.0,0.4);
    if(u_auxiliary_flat) return vec4(1.0,0.0,0.0,1.0);
    vec3 normal=u_flat_shading?normalize(cross(dFdx(v_position),dFdy(v_position))):normalize(gl_FrontFacing?v_source_normal:-v_source_normal);
    vec3 tint=u_use_vertex_color?v_color:u_color;
    if(u_shading_mode==0) return preservedStudioInLinearTarget(normal,tint);
    vec3 linear;
    if(u_shading_mode==2) linear=displayToLinear(normal*0.5+0.5);
    else if(u_shading_mode==3&&u_has_uv_data) {
        // Imported UV images are display colors. The Source clay color must
        // not darken them unless vertex-color tint is explicitly enabled.
        linear=displayToLinear(texture(u_uv_texture,v_uv).rgb)
            *(u_use_vertex_color?displayToLinear(v_color):vec3(1.0));
    }
    else if(u_shading_mode==4) {
        vec3 z=normalize(v_preserved_eye),x=vec3(-z.z,0.0,z.x),y=cross(x,z);
        vec2 uv=vec2(0.5)+0.5*vec2(dot(normal,x),dot(normal,y));
        linear=displayToLinear(texture(u_uv_texture,uv).rgb)*displayToLinear(tint);
    } else if(u_shading_mode==5) linear=displayToLinear(tint);
    else {
        vec2 material=u_has_material_data?v_material:u_default_material;
        linear=environmentPbr(normal,-normalize(v_position),tint,material.x,material.y,u_exposure);
    }
    return finishSourceColor(linear);
}
void main() {
    if(u_linear_pipeline) {out_color=sourceSurfaceColor();return;}
    if (u_wire) {
        out_color = vec4(vec3(0.10, 0.12, 0.15)*u_opacity,u_opacity);
        return;
    }
    vec3 normal = u_flat_shading ? safeNormal(cross(dFdx(v_position),dFdy(v_position))) : safeNormal(gl_FrontFacing?v_normal:-v_normal);
    if (u_shading_mode == 2) {
        out_color=finishColor(normal*0.5+0.5);
        return;
    }
    if(u_shading_mode==3&&u_has_uv_data) {
        vec3 rgb=u_has_uv_texture?texture(u_uv_texture,v_uv).rgb:checker(v_uv);
        // Viewport image alpha is ignored; the explicit opacity control owns
        // compositing. Vertex tint is used only when its existing toggle is on.
        vec3 linear=uvLinear(rgb)*(u_use_vertex_color?uvLinear(v_color):vec3(1.0));
        if(u_show_mask)linear=mix(linear,uvLinear(vec3(0.16,0.13,0.20)),clamp(v_mask,0.0,1.0)*0.82);
        if(!u_active)linear*=0.90;
        out_color=finishColor(pow(max(linear,vec3(0.0)),vec3(1.0/2.2)));
        return;
    }
    if(u_shading_mode==4) {
        vec3 z=normalize(v_preserved_eye);
        vec3 x=vec3(-z.z,0.0,z.x);
        vec3 y=cross(x,z);
        vec2 uv=vec2(0.5)+0.5*vec2(dot(normal,x),dot(normal,y));
        vec3 tint=u_use_vertex_color?v_color:u_color;
        vec3 linear=uvLinear(texture(u_uv_texture,uv).rgb)*uvLinear(tint);
        if(u_show_mask)linear=mix(linear,uvLinear(vec3(0.16,0.13,0.20)),clamp(v_mask,0.0,1.0)*0.82);
        if(!u_active)linear*=0.90;
        out_color=finishColor(pow(max(linear,vec3(0.0)),vec3(1.0/2.2)));
        return;
    }
    if(u_shading_mode==5){out_color=finishColor(u_use_vertex_color?v_color:u_color);return;}
    vec3 view = normalize(-v_position);
    vec3 key = normalize(vec3(-0.55, 0.75, 1.0));
    vec3 fill = normalize(vec3(0.8, 0.1, 0.8));
    vec3 rim = normalize(vec3(0.4, 0.6, -0.8));
    vec3 color = u_use_vertex_color ? v_color : u_color;
    if (u_show_mask) color = mix(color, vec3(0.16, 0.13, 0.20), clamp(v_mask, 0.0, 1.0) * 0.82);
    vec3 albedo = pow(max(color, vec3(0.0)), vec3(2.2));
    if (u_shading_mode == 1) {
        vec2 material=u_has_material_data?v_material:u_default_material;
        float roughness=clamp(material.x,0.04,1.0);
        float metallic=clamp(material.y,0.0,1.0);
        vec3 shaded=albedo*vec3(0.10,0.12,0.16)*(1.0-metallic*0.85);
        shaded+=pbrLight(normal,view,key,vec3(3.2,3.0,2.8),albedo,roughness,metallic);
        shaded+=pbrLight(normal,view,fill,vec3(0.65,0.8,1.1),albedo,roughness,metallic);
        shaded+=pbrLight(normal,view,rim,vec3(0.8,0.9,1.1),albedo,roughness,metallic);
        if(!u_active)shaded*=0.90;
        shaded*=u_exposure;
        shaded=shaded/(vec3(1.0)+shaded);
        out_color=finishColor(pow(max(shaded,vec3(0.0)),vec3(1.0/2.2)));
        return;
    }
    float diffuse = max(dot(normal, key), 0.0);
    float secondary = max(dot(normal, fill), 0.0);
    float edge = max(dot(normal, rim), 0.0);
    float highlight = pow(max(dot(normal, normalize(key + view)), 0.0), 36.0);
    float fresnel = pow(1.0 - max(dot(normal, view), 0.0), 3.0);
    vec3 light = vec3(0.24, 0.27, 0.32) + diffuse * vec3(0.85, 0.81, 0.75)
               + secondary * vec3(0.15, 0.18, 0.24) + edge * vec3(0.18, 0.20, 0.25);
    vec3 shaded = albedo * light + vec3(highlight * 0.11) + fresnel * vec3(0.025, 0.035, 0.055);
    if (!u_active) shaded *= 0.90;
    out_color = finishColor(pow(max(shaded, vec3(0.0)), vec3(1.0 / 2.2)));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_guide_cache_tracks_immutable_mesh_and_retained_tree_identity() {
        let mut source = Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 0 4 0\nf 1 2 3\n").unwrap();
        source.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        let mesh = Arc::new(source);
        let metadata = SceneObjectMetadata {
            source_center_initialized: true,
            symmetry_offset: 0.25,
            affine: obrush_core::ObjectAffineF32V1 {
                interaction_center: [1., 2., 0.],
                local_to_world: glam::Mat4::IDENTITY.to_cols_array(),
            },
            render: None,
            picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let mut cache = None;
        let object = RenderObject {
            id: 1,
            active: true,
            mesh: &mesh,
            metadata: Some(&metadata),
            source: UploadSource::Shared(&mesh),
            selected_normals: None,
        };
        let first = source_guide_root_loose(&object, &mut cache)
            .unwrap()
            .unwrap();
        assert_eq!(
            source_guide_root_loose(&object, &mut cache).unwrap(),
            Some(first)
        );
        let mut changed = (*mesh).clone();
        changed.positions[0].x = -2.;
        let changed = Arc::new(changed);
        let changed_object = RenderObject {
            mesh: &changed,
            source: UploadSource::Shared(&changed),
            ..object
        };
        let second = source_guide_root_loose(&changed_object, &mut cache)
            .unwrap()
            .unwrap();
        assert!(second[0] < first[0]);
        assert_eq!(second[3], first[3]);
        let retained = Arc::new(obrush_core::SourceGeometryNormalState::seed(&changed).unwrap());
        let retained_object = RenderObject {
            selected_normals: Some(&retained),
            ..changed_object
        };
        assert_eq!(
            source_guide_root_loose(&retained_object, &mut cache).unwrap(),
            Some(retained.root_loose())
        );
    }

    fn render_object(mesh: &Arc<Mesh>, active: bool) -> RenderObject<'_> {
        RenderObject {
            id: 1,
            active,
            mesh,
            metadata: None,
            selected_normals: None,
            source: UploadSource::Shared(mesh),
        }
    }

    #[test]
    fn sparse_selected_upload_reconstructs_exact_packed_rows() {
        let mut old = Mesh::sphere(5);
        old.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        old.masks.fill(1.0);
        let state = obrush_core::SourceGeometryNormalState::seed(&old).unwrap();
        let mut next = old.clone();
        next.positions[0].x += 0.125;
        next.positions[5_000].y += 0.25;
        let mut previous = packed_geometry_selected_rows_after_validation(&old, &state).unwrap();
        let ranges = selected_row_upload_ranges(&previous, &next, &state, None)
            .unwrap()
            .unwrap();
        assert_eq!(ranges, vec![0..2048, 4096..6144]);
        for range in ranges {
            for index in range {
                previous[index] = packed_selected_row(&next, &state, index);
            }
        }
        let expected = packed_geometry_selected_rows_after_validation(&next, &state).unwrap();
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&previous),
            bytemuck::cast_slice::<_, u8>(&expected)
        );
        assert!(
            selected_row_upload_ranges(&expected, &next, &state, None)
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn sparse_selected_upload_detects_signed_zero_and_attribute_changes() {
        let mut old = Mesh::sphere(5);
        old.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        old.masks.fill(1.0);
        old.colors = vec![[1.0, 1.0, 1.0]; old.positions.len()];
        let state = obrush_core::SourceGeometryNormalState::seed(&old).unwrap();
        let mut next = old.clone();
        next.positions[0].x = -0.0;
        old.positions[0].x = 0.0;
        next.colors[5_000][0] = 0.25;
        let previous = packed_geometry_selected_rows_after_validation(&old, &state).unwrap();
        let ranges = selected_row_upload_ranges(&previous, &next, &state, None)
            .unwrap()
            .unwrap();
        assert_eq!(ranges, vec![0..2048, 4096..6144]);
        let mut geometry_hint = [0_u64; 8];
        geometry_hint[0] = 1;
        assert_eq!(
            selected_row_upload_ranges(&previous, &next, &state, Some(&geometry_hint))
                .unwrap()
                .unwrap(),
            ranges,
            "A color edit outside the geometry witness still needs an upload"
        );
    }

    #[test]
    fn controlled_chunk_scan_checks_raw_masks_and_materials_without_losing_distant_edits() {
        let mut old = Mesh::sphere(5);
        old.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        old.masks.fill(0.0);
        old.materials = Some(vec![[0.4, 0.1]; old.positions.len()]);
        let state = obrush_core::SourceGeometryNormalState::seed(&old).unwrap();
        let old_rows = packed_geometry_selected_rows_after_validation(&old, &state).unwrap();
        let raw_masks = SourceMaskSnapshot::capture(&old).unwrap();
        let old_materials = old.materials.clone();
        let mut hint = [0_u64; 8];
        hint[0] = 1;

        let clean = selected_row_upload_scan(
            &old_rows,
            &old,
            &state,
            Some(&hint),
            Some(&raw_masks),
            Some(&old_materials),
        )
        .unwrap();
        assert!(clean.ranges.unwrap().is_empty());
        assert_eq!(clean.raw_masks_match, Some(true));
        assert_eq!(clean.materials_match, Some(true));

        let mut edited = old.clone();
        edited.positions[0].x += 0.125;
        edited.masks[5_000] = f32::from_bits(1);
        edited.materials.as_mut().unwrap()[6_000][1] = -0.0;
        let scan = selected_row_upload_scan(
            &old_rows,
            &edited,
            &state,
            Some(&hint),
            Some(&raw_masks),
            Some(&old_materials),
        )
        .unwrap();
        assert_eq!(scan.ranges.unwrap(), vec![0..2048]);
        assert_eq!(scan.raw_masks_match, Some(false));
        assert_eq!(scan.materials_match, Some(false));

        // Without a controlled chunk witness, independent full-channel checks
        // remain the only authority for mask/material buffer reuse.
        let fallback = selected_row_upload_scan(
            &old_rows,
            &edited,
            &state,
            None,
            Some(&raw_masks),
            Some(&old_materials),
        )
        .unwrap();
        assert_eq!(fallback.raw_masks_match, None);
        assert_eq!(fallback.materials_match, None);
    }

    #[test]
    fn controlled_core_witness_reconstructs_exact_browser_rows() {
        use obrush_core::{
            Document, MaskConvention, SourceGeometryAlphaTracks, SourceGeometryControls,
            SourceGeometryOptions, SourceGeometryPass,
        };
        let mut mesh =
            Mesh::from_obj("v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n")
                .unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let mut document = Document::new(mesh);
        let options = SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        };
        let pass = |x| SourceGeometryPass {
            center: [x, 0.0, 0.0],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        let mut first = document.begin_source_geometry_stroke(options).unwrap();
        document
            .apply_source_geometry_dab(&mut first, pass(0.0), 0.5, true)
            .unwrap();
        document.end_source_geometry_stroke(&first).unwrap();
        let before_mesh = document.require_active_mesh().unwrap().snapshot();
        let before_state = document.source_geometry_state(1).unwrap();
        let before_rows =
            packed_geometry_selected_rows_after_validation(&before_mesh, &before_state).unwrap();
        let mut second = document
            .begin_source_geometry_selection_history(options)
            .unwrap();
        let report = document
            .apply_source_geometry_controlled_dab(
                &mut second,
                [Some(pass(4.0)), None],
                0.5,
                true,
                &SourceGeometryControls {
                    alpha: None,
                    mirror: false,
                },
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        assert_eq!(report.combined.changed_vertices, 1);
        let after_mesh = document.require_active_mesh().unwrap();
        let after_state = document.source_geometry_state(1).unwrap();
        let hint = after_state
            .render_selected_chunks_from(&before_state)
            .unwrap();
        let full = selected_row_upload_ranges(&before_rows, after_mesh, &after_state, None)
            .unwrap()
            .unwrap();
        let hinted =
            selected_row_upload_ranges(&before_rows, after_mesh, &after_state, Some(&hint))
                .unwrap()
                .unwrap();
        assert_eq!(hinted, full);
        let mut published = before_rows.clone();
        for range in hinted {
            for index in range {
                published[index] = packed_selected_row(after_mesh, &after_state, index);
            }
        }
        let expected =
            packed_geometry_selected_rows_after_validation(after_mesh, &after_state).unwrap();
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&published),
            bytemuck::cast_slice::<_, u8>(&expected)
        );
        let mut outside = after_mesh.snapshot();
        outside.colors[3] = [0.125, 0.75, 0.5];
        outside.masks[4] = 0.25;
        let full_with_attributes =
            selected_row_upload_ranges(&before_rows, &outside, &after_state, None)
                .unwrap()
                .unwrap();
        let hinted_with_attributes =
            selected_row_upload_ranges(&before_rows, &outside, &after_state, Some(&hint))
                .unwrap()
                .unwrap();
        assert_eq!(hinted_with_attributes, full_with_attributes);
        document.cancel_source_geometry_stroke(&second).unwrap();
    }

    #[test]
    fn shared_upload_identity_survives_selection_and_detects_replaced_geometry() {
        let original = Arc::new(Mesh::cube());
        let mut identity = UploadIdentity::default();
        identity.update(&render_object(&original, true));
        let unchanged = Arc::clone(&original);
        assert!(identity.matches(&render_object(&unchanged, false)));

        // Equal vertex/face counts alone cannot reuse buffers after an edit.
        let mut edited = original.as_ref().clone();
        edited.positions[0].x += 0.25;
        let replacement = Arc::new(edited);
        assert!(!identity.matches(&render_object(&replacement, true)));
        identity.update(&render_object(&replacement, true));
        assert!(identity.matches(&render_object(&replacement, false)));
        assert!(!identity.matches(&render_object(&original, true)));
    }

    #[test]
    fn upload_cache_does_not_retain_removed_cpu_geometry() {
        let original = Arc::new(Mesh::sphere(1));
        let mut identity = UploadIdentity::default();
        identity.update(&render_object(&original, true));
        assert_eq!(Arc::strong_count(&original), 1);
        drop(original);
        assert!(identity.mesh.as_ref().unwrap().upgrade().is_none());
        // The weak allocation remains until the GPU entry is removed, so its
        // pointer cannot be recycled into a false identity match meanwhile.
    }

    #[test]
    fn arc_copy_on_write_invalidates_upload_identity() {
        let mut mesh = Arc::new(Mesh::cube());
        let mut identity = UploadIdentity::default();
        identity.update(&render_object(&mesh, true));
        Arc::make_mut(&mut mesh).positions[0].x += 0.5;
        assert!(!identity.matches(&render_object(&mesh, true)));
    }

    #[test]
    fn active_depth_tie_pass_follows_every_inactive_object_after_reorder() {
        let mesh = Arc::new(Mesh::cube());
        let mut active = render_object(&mesh, true);
        active.id = 20;
        let mut first = render_object(&mesh, false);
        first.id = 10;
        let mut second = render_object(&mesh, false);
        second.id = 30;
        let objects = [active, second, first];
        let ids: Vec<_> = solid_draw_order(&objects).map(|object| object.id).collect();
        assert_eq!(ids, [30, 10, 20]);
    }

    #[test]
    fn geometry_storage_accounts_for_wires_and_rejects_large_counts() {
        // Eight 40-byte geometry rows plus eight 8-byte material rows, then
        // the independent triangle/wire index allocations.
        assert_eq!(geometry_storage_bytes(8, 12, false), Some(528));
        assert_eq!(geometry_storage_bytes(8, 12, true), Some(816));
        assert_eq!(geometry_storage_bytes(usize::MAX, 12, true), None);
        assert_eq!(geometry_storage_bytes(8, usize::MAX, false), None);
        assert_eq!(
            geometry_storage_bytes(obrush_core::MAX_VERTICES + 1, 1, false),
            None
        );
        assert_eq!(
            geometry_storage_bytes(1, obrush_core::MAX_TRIANGLES + 1, false),
            None
        );
        // Existing core capacity fits the separate scene GPU budget even with
        // every triangle edge retained rather than shared/deduplicated.
        assert!(
            geometry_storage_bytes(obrush_core::MAX_VERTICES, obrush_core::MAX_TRIANGLES, true)
                .unwrap()
                < MAX_SCENE_GPU_BYTES
        );
        let mesh = Arc::new(Mesh::cube());
        assert_eq!(
            scene_storage_bytes(
                &[render_object(&mesh, true), render_object(&mesh, false)],
                true
            ),
            Some(2 * (mesh.positions.len() * 48 + mesh.triangles.len() * (12 + 24)))
        );
        assert_eq!(scene_storage_bytes(&[], true), Some(0));
        let too_many: Vec<_> = (0..=obrush_core::MAX_SUBTOOLS)
            .map(|_| render_object(&mesh, false))
            .collect();
        assert_eq!(scene_storage_bytes(&too_many, true), None);
    }

    #[test]
    fn uv_scene_budget_adds_separate_rows_and_keeps_ordinary_budget_unchanged() {
        let mesh=Arc::new(Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvt 0 0\nvt 0.25 1\nf 1/1 2/2 3/3\nf 1/5 3/6 4/4\n").unwrap().mesh);
        let objects = [render_object(&mesh, true)];
        assert_eq!(scene_render_storage_bytes(&objects, false), Some(264));
        assert_eq!(scene_render_storage_bytes(&objects, true), Some(624));
        let legacy = Arc::new(Mesh::cube());
        let legacy_objects = [render_object(&legacy, true)];
        assert_eq!(
            scene_render_storage_bytes(&legacy_objects, true),
            scene_storage_bytes(&legacy_objects, true)
        );
    }
    #[test]
    fn new_viewport_controls_default_to_prior_opaque_uncurved_geometry_modes() {
        let style = ViewportStyle::default();
        assert_eq!(style.shading, ShadingMode::Studio);
        assert_eq!(style.curvature, 0.0);
        assert_eq!(style.opacity, 1.0);
        assert!(style.uv_texture.is_none());
        assert_eq!(finite_unit_range(f32::NAN, 0.0, 5.0, 0.0), 0.0);
        assert_eq!(finite_unit_range(6.0, 0.0, 5.0, 0.0), 5.0);
        assert_eq!(finite_unit_range(-1.0, 0.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn scene_framing_includes_separated_objects_in_every_aspect() {
        let mut left = Mesh::cube();
        let mut right = Mesh::cube();
        for position in &mut left.positions {
            *position += Vec3::new(-4.0, -1.5, 0.25);
        }
        for position in &mut right.positions {
            *position += Vec3::new(3.0, 2.0, -0.5);
        }
        let objects = [
            SubtoolRenderSnapshot {
                id: 1,
                active: true,
                mesh: Arc::new(left),
                metadata: None,
            },
            SubtoolRenderSnapshot {
                id: 2,
                active: false,
                mesh: Arc::new(right),
                metadata: None,
            },
        ];
        let mut camera = Camera::default();
        camera.frame_scene(&objects);
        assert!(
            camera
                .target
                .abs_diff_eq(Vec3::new(-0.5, 0.25, -0.125), 1.0e-6)
        );
        for aspect in [0.3, 0.62, 1.0, 1.7, 3.0] {
            let projection = camera.view_projection(aspect);
            for object in &objects {
                for &position in &object.mesh.positions {
                    let ndc = projection.project_point3(position);
                    assert!(ndc.is_finite());
                    assert!(ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0);
                    assert!((-1.0..1.0).contains(&ndc.z));
                }
            }
        }
    }

    #[test]
    fn empty_and_single_object_scene_framing_preserve_existing_behavior() {
        let mut camera = Camera::default();
        camera.pan(egui::vec2(10.0, -20.0));
        let original = camera.clone();
        camera.frame_scene(&[]);
        assert_eq!(camera.target, original.target);
        assert_eq!(camera.distance, original.distance);

        let mesh = Arc::new(Mesh::sphere(1));
        let mut single = Camera::default();
        single.frame(&mesh);
        camera.frame_scene(&[SubtoolRenderSnapshot {
            id: 12,
            active: false,
            mesh,
            metadata: None,
        }]);
        assert_eq!(camera.target, single.target);
        assert_eq!(camera.distance, single.distance);
    }

    #[test]
    fn antialias_storage_respects_capabilities_and_memory_budget() {
        assert!(antialias_target_fits(1920, 1080, 4, 4, 8192));
        assert!(!antialias_target_fits(1920, 1080, 4, 2, 8192));
        assert!(!antialias_target_fits(3840, 2160, 4, 4, 8192));
        assert!(antialias_target_fits(3840, 2160, 2, 4, 8192));
        assert!(!antialias_target_fits(8193, 1, 2, 4, 8192));
        assert!(!antialias_target_fits(0, 1080, 4, 4, 8192));
        assert!(!antialias_target_fits(
            i32::MAX,
            i32::MAX,
            i32::MAX,
            i32::MAX,
            i32::MAX
        ));
    }

    #[test]
    fn resolution_storage_has_an_independent_finite_budget() {
        assert!(resolution_target_fits(1920, 1080, 8192));
        assert!(resolution_target_fits(3840, 2160, 8192));
        assert!(!resolution_target_fits(7680, 4320, 8192));
        assert!(!resolution_target_fits(8193, 1, 8192));
        assert!(!resolution_target_fits(0, 1080, 8192));
    }

    #[test]
    fn center_ray_faces_target_after_orbit_and_pan() {
        let mut camera = Camera::default();
        camera.orbit(egui::vec2(183.0, -72.0));
        camera.pan(egui::vec2(25.0, 15.0));
        let ray = camera.ray(Vec2::ZERO, 1.7);
        let expected = (camera.target - camera.position()).normalize();
        assert!(ray.origin.abs_diff_eq(camera.position(), 1.0e-5));
        assert!(ray.direction.dot(expected) > 0.99999);
    }

    #[test]
    fn projection_and_picking_use_the_same_screen_convention() {
        let camera = Camera::default();
        let ndc = Vec2::new(0.65, 0.3);
        for aspect in [0.3, 0.62, 1.0, 1.7] {
            let ray = camera.ray(ndc, aspect);
            let projected = camera
                .view_projection(aspect)
                .project_point3(ray.origin + ray.direction * 3.0);
            assert!((projected.x - ndc.x).abs() < 1.0e-3);
            assert!((projected.y - ndc.y).abs() < 1.0e-3);
        }
    }

    #[test]
    fn zoom_stays_finite_and_orbit_avoids_poles() {
        let mut camera = Camera::default();
        let initial = camera.distance;
        camera.zoom(20.0);
        assert!(camera.distance < initial);
        camera.zoom(1_000_000.0);
        assert_eq!(camera.distance, 0.02);
        camera.zoom(-1_000_000.0);
        assert_eq!(camera.distance, 100_000_000.0);
        camera.orbit(egui::vec2(0.0, 1_000_000.0));
        assert!(camera.view_projection(1.0).is_finite());
    }

    #[test]
    fn raw_wheel_impulse_is_bounded_without_a_following_zoom_tail() {
        for orthographic in [false, true] {
            let mut camera = Camera {
                distance: 3.418441,
                orthographic,
                ..Default::default()
            };
            let initial = camera.distance;
            camera.scroll_zoom(240.0);
            assert!((camera.distance - initial * (-0.25_f32).exp()).abs() < 1e-6);
            assert!(camera.distance > 2.6);
            let endpoint = camera.distance;
            for amount in [0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                camera.scroll_zoom(amount);
                assert_eq!(camera.distance, endpoint);
            }
            camera.scroll_zoom(-240.0);
            assert!((camera.distance - initial).abs() < 1e-6);
        }
    }

    #[test]
    fn orthographic_rays_are_parallel_and_projection_matches_at_multiple_depths() {
        let camera = Camera {
            orthographic: true,
            ..Default::default()
        };
        for aspect in [0.3, 1.0, 1.7, 3.0] {
            let center = camera.ray(Vec2::ZERO, aspect);
            for ndc in [Vec2::new(-0.7, -0.2), Vec2::ZERO, Vec2::new(0.6, 0.8)] {
                let ray = camera.ray(ndc, aspect);
                assert!(ray.direction.abs_diff_eq(center.direction, 1e-6));
                for depth in [0.1, 1.0, 8.0] {
                    let projected = camera
                        .view_projection(aspect)
                        .project_point3(ray.origin + ray.direction * depth);
                    assert!((projected.x - ndc.x).abs() < 2e-5);
                    assert!((projected.y - ndc.y).abs() < 2e-5);
                }
            }
            assert_eq!(
                camera.world_units_per_pixel(1.0, 1000.0, aspect),
                camera.world_units_per_pixel(30.0, 1000.0, aspect)
            );
        }
    }

    #[test]
    fn axis_views_toggle_opposite_and_poles_have_valid_pick_bases() {
        for axis in 0..3 {
            let mut camera = Camera::default();
            camera.axis_view(axis);
            let eye_direction = (camera.position() - camera.target).normalize();
            let expected = [Vec3::Z, -Vec3::X, Vec3::Y][axis];
            assert!(eye_direction.abs_diff_eq(expected, 2e-6));
            let (right, up, forward) = camera.axes();
            assert!((right.length() - 1.0).abs() < 2e-6 && (up.length() - 1.0).abs() < 2e-6);
            assert!(right.dot(up).abs() < 2e-6 && forward.dot(up).abs() < 2e-6);
            assert!(camera.view_projection(1.2).is_finite());
            camera.axis_view(axis);
            assert!(
                (camera.position() - camera.target)
                    .normalize()
                    .abs_diff_eq(-expected, 2e-6)
            );
        }
    }

    #[test]
    fn source_keyboard_camera_steps_change_view_without_touching_mesh() {
        let mut camera = Camera {
            rotation: Some(Quat::IDENTITY),
            distance: 4.0,
            ..Default::default()
        };
        let initial = camera.view64();
        camera.source_keyboard_step(-1, -1);
        assert!((camera.target.x - 0.03).abs() < 1e-6);
        assert!((camera.distance - 3.99625).abs() < 1e-6);
        assert_ne!(camera.view64(), initial);
        camera.source_keyboard_step(1, 1);
        assert!((camera.target.x - 0.000028125).abs() < 1e-6);
        assert!((camera.distance - 4.0).abs() < 1e-6);
    }

    #[test]
    fn alternative_camera_modes_keep_finite_roll_and_project_pick_rays() {
        for mode in [CameraMode::Orbit, CameraMode::Spherical, CameraMode::Plane] {
            let mut camera = Camera {
                mode,
                ..Default::default()
            };
            for i in 0..64 {
                let a = Vec2::new((i as f32 * 0.2).sin() * 0.9, (i as f32 * 0.2).cos() * 0.9);
                let b = Vec2::new(
                    ((i + 1) as f32 * 0.2).sin() * 0.9,
                    ((i + 1) as f32 * 0.2).cos() * 0.9,
                );
                camera.rotate(a, b, egui::vec2(7.0, -3.0));
                let ray = camera.ray(Vec2::new(0.2, -0.3), 1.4);
                let p = camera
                    .view_projection(1.4)
                    .project_point3(ray.origin + ray.direction * 2.0);
                assert!(p.is_finite() && (p.x - 0.2).abs() < 1e-3 && (p.y + 0.3).abs() < 1e-3);
            }
            camera.snap_axis();
            let (right, up, forward) = camera.axes();
            for axis in [right, up, forward] {
                assert!(axis.abs().max_element() > 0.9999);
            }
        }
    }

    #[test]
    fn picked_pivot_preserves_eye_and_fov_range_frames_scene() {
        let mut camera = Camera::default();
        let eye = camera.position();
        camera.set_pivot(Vec3::new(0.4, -0.2, 0.1));
        assert!(camera.position().abs_diff_eq(eye, 2e-6));
        for projection in [false, true] {
            for fov in [10.0, 40.0, 90.0] {
                camera.orthographic = projection;
                camera.fov_degrees = fov;
                let mesh = Mesh::cube();
                camera.frame(&mesh);
                for aspect in [0.3, 1.0, 2.0] {
                    for p in &mesh.positions {
                        let projected = camera.view_projection(aspect).project_point3(*p);
                        assert!(projected.x.abs() < 1.0 && projected.y.abs() < 1.0);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod source_normal_consumer_tests {
    use super::*;
    fn quad() -> Mesh {
        Mesh::import_obj(b"v 0 0 0\nv 2 0 0\nv 2 1 1\nv 0 1 0\nf 1 2 3 4\n")
            .unwrap()
            .mesh
    }
    fn object(mesh: &Arc<Mesh>) -> RenderObject<'_> {
        RenderObject {
            id: 1,
            active: true,
            mesh,
            metadata: None,
            selected_normals: None,
            source: UploadSource::Shared(mesh),
        }
    }
    #[test]
    fn actual_upload_rows_use_raw_original_quad_normals_and_preserve_other_fields() {
        let m = quad();
        let before = serde_json::to_vec(&m).unwrap();
        let field = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        let legacy = packed_geometry_rows(&m, None).unwrap();
        let source = packed_geometry_rows(&m, Some(&field)).unwrap();
        for (a, b) in legacy.iter().zip(&source) {
            assert_eq!(a[..3], b[..3]);
            assert_eq!(a[6..], b[6..]);
            assert_eq!(b[3..6], [-1., -2., 4.]);
        }
        assert_ne!(legacy[1][3..6], source[1][3..6]);
        assert_eq!(before, serde_json::to_vec(&m).unwrap());
    }
    #[test]
    fn all_nonstudio_global_roles_select_source_normals_for_an_ordinary_mesh() {
        for role in [
            ShadingMode::Pbr,
            ShadingMode::Normal,
            ShadingMode::Uv,
            ShadingMode::Matcap,
            ShadingMode::Flat,
        ] {
            let style = ViewportStyle {
                shading: role,
                ..Default::default()
            };
            assert_eq!(
                source_normal_policy(None, &style),
                Some(obrush_core::SourceNormalPolicy::StaticOriginalFaces)
            );
        }
        assert_eq!(source_normal_policy(None, &ViewportStyle::default()), None);
    }
    #[test]
    fn same_arc_role_switch_forces_recipe_refresh_and_studio_gets_original_rows() {
        let m = Arc::new(quad());
        let object = object(&m);
        let mut identity = UploadIdentity::default();
        identity.update(&object);
        let f = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        let pbr = ViewportStyle {
            shading: ShadingMode::Pbr,
            ..Default::default()
        };
        let studio = ViewportStyle::default();
        assert!(identity.matches(&object));
        assert!(!normal_cache_matches_policy(None, &object, &pbr));
        assert!(normal_cache_matches_policy(Some(&f), &object, &pbr));
        assert!(!normal_cache_matches_policy(Some(&f), &object, &studio));
        assert!(normal_cache_matches_policy(None, &object, &studio));
        let rows = packed_geometry_rows(&m, None).unwrap();
        assert_eq!(rows[1][3..6], m.normals[1].to_array());
        assert!(identity.matches(&object));
    }
    #[test]
    fn plain_studio_preserves_legacy_recipe_in_a_mixed_source_scene() {
        let m = Arc::new(quad());
        let ordinary = object(&m);
        let studio = ViewportStyle::default();
        let metadata = SceneObjectMetadata {
            source_center_initialized: false,
            symmetry_offset: 0.0,
            affine: obrush_core::ObjectAffineF32V1::from_local_points(
                Mat4::IDENTITY.to_cols_array(),
                &m.positions,
            )
            .unwrap(),
            render: Some(baseline_source_render_defaults()),
            picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let effective = consumer_style_for_mesh(Some(&metadata), &m, &studio).unwrap();
        assert_eq!(
            source_normal_policy(Some(&metadata), &effective),
            Some(obrush_core::SourceNormalPolicy::StaticOriginalFaces)
        );
        assert_eq!(source_normal_policy(None, &studio), None);
        assert!(normal_cache_matches_policy(None, &ordinary, &studio));
    }
    #[test]
    fn sparse_worker_metadata_admits_only_identity_shader_zero_without_effects() {
        let mesh = quad();
        let mut metadata = SceneObjectMetadata {
            source_center_initialized: false,
            symmetry_offset: 0.0,
            affine: obrush_core::ObjectAffineF32V1::from_local_points(
                Mat4::IDENTITY.to_cols_array(),
                &mesh.positions,
            )
            .unwrap(),
            render: Some(obrush_core::SglRenderSettings {
                shader: 0,
                matcap: 0,
                show_wireframe: false,
                flat_shading: false,
                opacity: 1.0,
            }),
            picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let studio = ViewportStyle::default();
        let effective = consumer_style_for_mesh(Some(&metadata), &mesh, &studio).unwrap();
        assert!(source_worker_preview_style_eligible(
            Some(&metadata),
            &effective
        ));
        assert_eq!(
            source_normal_policy(Some(&metadata), &effective),
            Some(obrush_core::SourceNormalPolicy::StaticOriginalFaces)
        );
        metadata.affine.local_to_world[12] = 0.25;
        assert!(!source_worker_preview_style_eligible(
            Some(&metadata),
            &effective
        ));
        metadata.affine.local_to_world[12] = 0.0;
        metadata.render.as_mut().unwrap().opacity = 0.5;
        assert!(!source_worker_preview_style_eligible(
            Some(&metadata),
            &effective
        ));
        metadata.render.as_mut().unwrap().opacity = 1.0;
        metadata.render.as_mut().unwrap().shader = 5;
        assert!(!source_worker_preview_style_eligible(
            Some(&metadata),
            &effective
        ));
    }
    #[test]
    fn stale_source_upload_field_rejects_instead_of_packing_legacy_fallback() {
        let mut m = quad();
        let field = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        m.positions[1].x = 3.;
        assert!(packed_geometry_rows(&m, Some(&field)).is_err());
        assert!(packed_geometry_rows(&m, None).is_ok());
    }
}

#[cfg(test)]
#[path = "source_geometry_normal_tests.rs"]
mod source_geometry_normal_tests;

#[cfg(test)]
mod boot_source_fov_tests {
    use super::*;
    fn frame_camera(override_fov: Option<f64>) -> Camera {
        let viewport = crate::source_camera_math::ViewportDimensions::checked(
            [1200, 600],
            glam::DVec2::new(600.0, 300.0),
        )
        .unwrap();
        let mut camera = Camera {
            fov_degrees: 45.123456789_f64 as f32,
            boot_source_fov: override_fov,
            source_vertical_fov: true,
            rotation: Some(Quat::IDENTITY),
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        };
        camera.set_source_framing(true, viewport).unwrap();
        camera.set_source_projection_zoom(12.0).unwrap();
        camera.prepare_source_frame(viewport, &[]).unwrap();
        camera
    }
    #[test]
    fn absent_boot_override_keeps_existing_f32_frame_input_domain() {
        let camera = frame_camera(None);
        let frame = camera.source_frame().unwrap();
        assert_eq!(
            frame.vertical_fov_degrees.to_bits(),
            f64::from(camera.fov_degrees).to_bits()
        );
        assert_eq!(frame.eye64.z, 12.0 * 45.0 / f64::from(camera.fov_degrees));
    }
    #[test]
    fn raw_boot_fov_changes_actual_source_frame_and_restores_on_edit() {
        let requested = 45.123456789f64;
        let mut camera = frame_camera(Some(requested));
        let before = camera.source_frame().unwrap().clone();
        assert_eq!(before.vertical_fov_degrees.to_bits(), requested.to_bits());
        assert_eq!(before.eye64.z, 12.0 * 45.0 / requested);
        assert_ne!(
            before.vertical_fov_degrees.to_bits(),
            f64::from(camera.fov_degrees).to_bits()
        );
        camera.boot_source_fov = None;
        let viewport = crate::source_camera_math::ViewportDimensions::checked(
            [1200, 600],
            glam::DVec2::new(600.0, 300.0),
        )
        .unwrap();
        camera.prepare_source_frame(viewport, &[]).unwrap();
        assert_eq!(
            camera
                .source_frame()
                .unwrap()
                .vertical_fov_degrees
                .to_bits(),
            f64::from(camera.fov_degrees).to_bits()
        );
        assert_ne!(camera.source_frame().unwrap().eye64.z, before.eye64.z);
    }
}
