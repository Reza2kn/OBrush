//! Camera-frozen world-axis, world-plane and camera-plane translation gestures.
//!
//! Mouse motion updates a latest absolute offset only. Geometry and history are
//! untouched until the host accepts one core transform transaction on release.

use eframe::egui::{Pos2, Rect};
use glam::{Vec2, Vec3};

use crate::renderer::Camera;

pub const MAX_PREVIEW_COMPONENT: f32 = 1_000_000.0;
pub const MIN_SCALE_FACTOR: f32 = 0.01;
const MIN_AXIS_PLANE_LENGTH: f32 = 0.087_155_74; // sin(5 degrees)
const MIN_RAY_PLANE_DOT: f32 = 1.0e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslationHandle {
    X,
    Y,
    Z,
    Free,
    PlaneX,
    PlaneY,
    PlaneZ,
}

impl TranslationHandle {
    pub fn key(self) -> &'static str {
        match self {
            Self::X => "gizmo.x",
            Self::Y => "gizmo.y",
            Self::Z => "gizmo.z",
            Self::Free => "gizmo.free",
            Self::PlaneX => "gizmo.plane_x",
            Self::PlaneY => "gizmo.plane_y",
            Self::PlaneZ => "gizmo.plane_z",
        }
    }

    pub fn axis(self) -> Option<Vec3> {
        match self {
            Self::X => Some(Vec3::X),
            Self::Y => Some(Vec3::Y),
            Self::Z => Some(Vec3::Z),
            Self::Free => None,
            Self::PlaneX | Self::PlaneY | Self::PlaneZ => None,
        }
    }

    pub fn plane_normal(self) -> Option<Vec3> {
        match self {
            Self::PlaneX => Some(Vec3::X),
            Self::PlaneY => Some(Vec3::Y),
            Self::PlaneZ => Some(Vec3::Z),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TranslationDrag {
    camera: Camera,
    viewport: Rect,
    origin: Vec3,
    plane_normal: Vec3,
    start_intersection: Vec3,
    screen_offset: eframe::egui::Vec2,
    pub handle: TranslationHandle,
    pub translation: Vec3,
}

/// Pinned SculptGL Gizmo._updateScaleEdit screen-distance policy. The origin,
/// projected axis and initial radius are captured on pointer down; sampling
/// the same final pointer through any number of intermediate moves is exact.
#[derive(Clone, Debug)]
pub struct ScaleDrag {
    origin: Pos2,
    axis: Option<eframe::egui::Vec2>,
    start_distance: f32,
    pub factor: f32,
}

/// Pinned SculptGL Gizmo._updateRotateEdit policy: screen travel along the
/// picked torus tangent maps to seven radians per shortest viewport side.
#[derive(Clone, Debug)]
pub struct RotationDrag {
    start: Pos2,
    tangent: eframe::egui::Vec2,
    shortest_side: f32,
    pub degrees: f32,
}

impl RotationDrag {
    pub fn tangent(&self) -> [f32; 2] {
        [self.tangent.x, self.tangent.y]
    }

    pub fn new(viewport: Rect, start: Pos2, tangent: eframe::egui::Vec2) -> Option<Self> {
        let side = viewport.width().min(viewport.height());
        let length = tangent.length();
        if !start.is_finite()
            || !tangent.is_finite()
            || !side.is_finite()
            || side <= 0.0
            || !length.is_finite()
            || length < 1.0e-4
        {
            return None;
        }
        Some(Self {
            start,
            tangent: tangent / length,
            shortest_side: side,
            degrees: 0.0,
        })
    }

    pub fn update(&mut self, pointer: Pos2) -> bool {
        if !pointer.is_finite() {
            return false;
        }
        let distance = (pointer - self.start).dot(self.tangent);
        let radians = (-7.0 * distance / self.shortest_side) % std::f32::consts::TAU;
        if !radians.is_finite() {
            return false;
        }
        self.degrees = radians.to_degrees();
        true
    }
}

impl ScaleDrag {
    pub fn new(origin: Pos2, pointer: Pos2, axis_end: Option<Pos2>) -> Option<Self> {
        if !origin.is_finite() || !pointer.is_finite() {
            return None;
        }
        let axis = if let Some(end) = axis_end {
            let direction = end - origin;
            let length = direction.length();
            if !length.is_finite() || length < 16.0 {
                return None;
            }
            Some(direction / length)
        } else {
            None
        };
        let start_distance = (pointer - origin).length();
        if !start_distance.is_finite() || start_distance < 16.0 {
            return None;
        }
        Some(Self {
            origin,
            axis,
            start_distance,
            factor: 1.0,
        })
    }

    pub fn update(&mut self, pointer: Pos2) -> bool {
        if !pointer.is_finite() {
            return false;
        }
        let vector = pointer - self.origin;
        let distance = if let Some(axis) = self.axis {
            vector.dot(axis).abs()
        } else {
            vector.length()
        };
        if !distance.is_finite() {
            return false;
        }
        self.factor = (distance / self.start_distance).clamp(MIN_SCALE_FACTOR, 1000.0);
        true
    }
}

impl TranslationDrag {
    pub fn new(
        camera: &Camera,
        viewport: Rect,
        pointer: Pos2,
        origin: Vec3,
        handle: TranslationHandle,
    ) -> Option<Self> {
        if !origin.is_finite()
            || !viewport.is_finite()
            || viewport.width() <= 0.0
            || viewport.height() <= 0.0
        {
            return None;
        }
        let forward = (camera.target - camera.position()).normalize_or_zero();
        let plane_normal = if let Some(normal) = handle.plane_normal() {
            normal
        } else if let Some(axis) = handle.axis() {
            let projected = forward - axis * forward.dot(axis);
            if projected.length() < MIN_AXIS_PLANE_LENGTH {
                return None;
            }
            projected.normalize()
        } else {
            forward
        };
        if !plane_normal.is_finite() || plane_normal.length_squared() < 0.99 {
            return None;
        }
        let screen_offset = if handle.plane_normal().is_some() {
            pointer - project_world(camera, viewport, origin)?
        } else {
            eframe::egui::Vec2::ZERO
        };
        let mut drag = Self {
            camera: camera.clone(),
            viewport,
            origin,
            plane_normal,
            start_intersection: Vec3::ZERO,
            screen_offset,
            handle,
            translation: Vec3::ZERO,
        };
        drag.start_intersection = drag.intersection(pointer)?;
        Some(drag)
    }

    fn intersection(&self, pointer: Pos2) -> Option<Vec3> {
        if !pointer.is_finite() {
            return None;
        }
        let ndc = Vec2::new(
            2.0 * (pointer.x - self.screen_offset.x - self.viewport.left()) / self.viewport.width()
                - 1.0,
            1.0 - 2.0 * (pointer.y - self.screen_offset.y - self.viewport.top())
                / self.viewport.height(),
        );
        let ray = self.camera.ray(ndc, self.viewport.aspect_ratio());
        let denominator = ray.direction.dot(self.plane_normal);
        if !denominator.is_finite() || denominator.abs() < MIN_RAY_PLANE_DOT {
            return None;
        }
        let distance = (self.origin - ray.origin).dot(self.plane_normal) / denominator;
        if !distance.is_finite() || distance <= 0.0 {
            return None;
        }
        let position = ray.origin + ray.direction * distance;
        position.is_finite().then_some(position)
    }

    /// Re-evaluate against the pointer-down camera/plane, never integrate deltas.
    /// Invalid/near-parallel rays leave the last valid displayed offset intact.
    pub fn update(&mut self, pointer: Pos2) -> bool {
        let Some(intersection) = self.intersection(pointer) else {
            return false;
        };
        let delta = intersection - self.start_intersection;
        let translation = if let Some(axis) = self.handle.axis() {
            axis * delta.dot(axis)
        } else {
            delta - self.plane_normal * delta.dot(self.plane_normal)
        };
        if !translation.is_finite() || translation.abs().max_element() > MAX_PREVIEW_COMPONENT {
            return false;
        }
        self.translation = translation;
        true
    }
}

pub fn project_world(camera: &Camera, viewport: Rect, point: Vec3) -> Option<Pos2> {
    if !point.is_finite() || viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return None;
    }
    let clip = camera.view_projection(viewport.aspect_ratio()) * point.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !(-1.0..=1.0).contains(&ndc.z) {
        return None;
    }
    Some(Pos2::new(
        viewport.left() + (ndc.x + 1.0) * 0.5 * viewport.width(),
        viewport.top() + (1.0 - ndc.y) * 0.5 * viewport.height(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_scale_screen_distance_is_chunk_invariant_and_bounded() {
        let origin = Pos2::new(400.0, 300.0);
        let start = Pos2::new(500.0, 300.0);
        let end = Pos2::new(550.0, 325.0);
        let mut axis = ScaleDrag::new(origin, start, Some(start)).unwrap();
        let mut chunked = axis.clone();
        assert!(axis.update(end));
        for step in 1..=50 {
            assert!(chunked.update(start.lerp(end, step as f32 / 50.0)));
        }
        assert_eq!(axis.factor, chunked.factor);
        assert_eq!(axis.factor, 1.5);
        assert!(axis.update(origin));
        assert_eq!(axis.factor, MIN_SCALE_FACTOR);
        assert!(axis.update(Pos2::new(300.0, 300.0)));
        assert_eq!(axis.factor, 1.0);
        assert!(axis.update(start));
        assert_eq!(axis.factor, 1.0);
        assert!(!axis.update(Pos2::new(f32::NAN, 0.0)));
        assert_eq!(axis.factor, 1.0);
        let mut ring = ScaleDrag::new(origin, start, None).unwrap();
        ring.update(Pos2::new(400.0, 400.0));
        assert_eq!(ring.factor, 1.0);
        ring.update(Pos2::new(400.0, 450.0));
        assert_eq!(ring.factor, 1.5);
    }

    #[test]
    fn source_rotation_tangent_is_absolute_chunk_invariant_and_returns_to_zero() {
        let viewport = Rect::from_min_size(Pos2::new(50.0, 20.0), eframe::egui::vec2(800.0, 600.0));
        let start = Pos2::new(400.0, 300.0);
        let tangent = eframe::egui::vec2(3.0, 4.0);
        let end = start + tangent.normalized() * 60.0;
        let mut one = RotationDrag::new(viewport, start, tangent).unwrap();
        let mut chunked = one.clone();
        assert!(one.update(end));
        for step in 1..=40 {
            assert!(chunked.update(start.lerp(end, step as f32 / 40.0)));
        }
        assert_eq!(one.degrees, chunked.degrees);
        assert!((one.degrees - (-0.7_f32).to_degrees()).abs() < 1.0e-4);
        assert!(one.update(start));
        assert_eq!(one.degrees, 0.0);
        assert!(!one.update(Pos2::new(f32::NAN, 0.0)));
        assert_eq!(one.degrees, 0.0);
    }

    fn viewport() -> Rect {
        Rect::from_min_max(Pos2::new(210.0, 60.0), Pos2::new(1360.0, 970.0))
    }

    #[test]
    fn plane_drag_preserves_pointer_offset_and_excludes_normal_axis() {
        let camera = Camera::default();
        let rect = viewport();
        let pivot = Vec3::ZERO;
        let center = project_world(&camera, rect, pivot).unwrap();
        let start = center + eframe::egui::vec2(25.0, -18.0);
        for (handle, target) in [
            (TranslationHandle::PlaneX, Vec3::new(0.0, 0.15, -0.12)),
            (TranslationHandle::PlaneY, Vec3::new(0.18, 0.0, -0.12)),
            (TranslationHandle::PlaneZ, Vec3::new(0.18, -0.15, 0.0)),
        ] {
            let moved = start + (project_world(&camera, rect, target).unwrap() - center);
            let mut one = TranslationDrag::new(&camera, rect, start, pivot, handle).unwrap();
            let mut chunked = one.clone();
            assert!(one.update(moved));
            for step in 1..=20 {
                assert!(chunked.update(start.lerp(moved, step as f32 / 20.0)));
            }
            assert_eq!(one.translation, chunked.translation);
            assert!(
                one.translation.abs_diff_eq(target, 3.0e-6),
                "{handle:?}: {:?}",
                one.translation
            );
            assert_eq!(one.translation.dot(handle.plane_normal().unwrap()), 0.0);
            one.update(start);
            assert_eq!(one.translation, Vec3::ZERO);
        }
    }

    #[test]
    fn absolute_axis_drag_is_chunk_invariant_and_has_no_cross_axis_motion() {
        let camera = Camera::default();
        let rect = viewport();
        for handle in [
            TranslationHandle::X,
            TranslationHandle::Y,
            TranslationHandle::Z,
        ] {
            let start = project_world(&camera, rect, Vec3::ZERO).unwrap();
            let end = project_world(&camera, rect, handle.axis().unwrap() * 0.4).unwrap();
            let mut one = TranslationDrag::new(&camera, rect, start, Vec3::ZERO, handle).unwrap();
            let mut many = one.clone();
            one.update(end);
            for index in 1..=64 {
                many.update(start.lerp(end, index as f32 / 64.0));
            }
            assert_eq!(one.translation, many.translation);
            assert!(
                one.translation
                    .abs_diff_eq(handle.axis().unwrap() * 0.4, 2.0e-6)
            );
            assert_eq!(one.translation.cross(handle.axis().unwrap()), Vec3::ZERO);
        }
    }

    #[test]
    fn return_to_pointerdown_is_exact_zero_and_outside_endpoint_is_supported() {
        let camera = Camera::default();
        let rect = viewport();
        let start = rect.center();
        let mut drag =
            TranslationDrag::new(&camera, rect, start, Vec3::ZERO, TranslationHandle::Free)
                .unwrap();
        assert!(drag.update(Pos2::new(rect.right() + 200.0, rect.bottom() + 80.0)));
        assert!(drag.translation.length() > 0.1);
        assert!(drag.update(start));
        assert_eq!(drag.translation, Vec3::ZERO);
    }

    #[test]
    fn free_drag_stays_in_pointerdown_camera_plane() {
        let camera = Camera {
            target: Vec3::new(2.0, 1.0, -0.5),
            ..Default::default()
        };
        let rect = viewport();
        let start = project_world(&camera, rect, camera.target).unwrap();
        let mut drag =
            TranslationDrag::new(&camera, rect, start, camera.target, TranslationHandle::Free)
                .unwrap();
        drag.update(start + eframe::egui::vec2(93.0, -41.0));
        assert!(drag.translation.length() > 0.05);
        let forward = (camera.target - camera.position()).normalize();
        assert!(drag.translation.dot(forward).abs() < 1.0e-7);
    }

    #[test]
    fn near_view_axes_are_disabled_and_invalid_motion_preserves_latest_value() {
        let camera = Camera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        };
        let rect = viewport();
        assert!(
            TranslationDrag::new(
                &camera,
                rect,
                rect.center(),
                Vec3::ZERO,
                TranslationHandle::Z
            )
            .is_none()
        );
        let mut drag = TranslationDrag::new(
            &camera,
            rect,
            rect.center(),
            Vec3::ZERO,
            TranslationHandle::X,
        )
        .unwrap();
        drag.update(rect.center() + eframe::egui::vec2(20.0, 0.0));
        let previous = drag.translation;
        assert!(!drag.update(Pos2::new(f32::NAN, 0.0)));
        assert_eq!(drag.translation, previous);
        assert!(!drag.update(Pos2::new(1.0e20, 1.0e20)));
        assert_eq!(drag.translation, previous);
    }

    #[test]
    fn projection_uses_camera_matrix_in_narrow_and_wide_viewports() {
        let camera = Camera::default();
        for size in [(280.0, 900.0), (1150.0, 910.0), (1500.0, 400.0)] {
            let rect =
                Rect::from_min_size(Pos2::new(210.0, 60.0), eframe::egui::vec2(size.0, size.1));
            let point = Vec3::new(0.15, -0.2, 0.1);
            let screen = project_world(&camera, rect, point).unwrap();
            let start = project_world(&camera, rect, Vec3::ZERO).unwrap();
            let mut drag =
                TranslationDrag::new(&camera, rect, start, Vec3::ZERO, TranslationHandle::X)
                    .unwrap();
            let axis_screen = project_world(&camera, rect, Vec3::X * 0.2).unwrap();
            drag.update(axis_screen);
            assert!(screen.is_finite());
            assert!(drag.translation.abs_diff_eq(Vec3::X * 0.2, 2.0e-6));
        }
    }
}
