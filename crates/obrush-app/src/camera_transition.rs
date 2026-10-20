//! Display-only camera motion. The caller supplies elapsed time so native and
//! browser hosts share the same pose curve without depending on timer cadence.
use crate::renderer::Camera;
use glam::{DVec3, Quat};

pub(crate) const SNAP_DURATION_MS: f64 = 200.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraTransitionPose {
    pub orientation: Quat,
    pub target: DVec3,
    pub distance: f64,
    pub source_zoom: Option<f64>,
    pub source_pivot_offset: Option<DVec3>,
}

impl CameraTransitionPose {
    pub fn capture(camera: &Camera) -> Self {
        let orientation = camera.rotation.unwrap_or_else(|| {
            Quat::from_rotation_y(camera.yaw) * Quat::from_rotation_x(-camera.pitch)
        });
        Self {
            orientation,
            target: camera
                .precise_target
                .unwrap_or_else(|| camera.target.as_dvec3()),
            distance: f64::from(camera.distance),
            source_zoom: camera.source_projection_zoom(),
            source_pivot_offset: camera.source_pivot_offset(),
        }
    }

    pub fn apply(self, camera: &mut Camera) -> Result<(), String> {
        if !self.valid()
            || !self.target.as_vec3().is_finite()
            || !(self.distance as f32).is_finite()
        {
            return Err("Camera transition sample exceeds finite display coordinates".into());
        }
        if camera.source_enabled() != self.source_zoom.is_some()
            || camera.source_enabled() != self.source_pivot_offset.is_some()
        {
            return Err("Camera transition projection mode changed during animation".into());
        }
        // Check source zoom before mutating the visible pose.
        if let Some(zoom) = self.source_zoom {
            camera.set_source_projection_zoom(zoom)?;
        }
        if let Some(offset) = self.source_pivot_offset {
            camera.set_source_pivot_offset(offset)?;
        }
        camera.rotation = Some(self.orientation.normalize());
        camera.target = self.target.as_vec3();
        camera.precise_target = Some(self.target);
        camera.distance = self.distance as f32;
        Ok(())
    }

    fn valid(self) -> bool {
        self.orientation.is_finite()
            && self.orientation.length_squared() > 1e-12
            && self.target.is_finite()
            && self.distance.is_finite()
            && self.distance > 0.0
            && self.source_zoom.is_none_or(|z| z.is_finite() && z > 0.0)
            && self.source_pivot_offset.is_none_or(|offset| offset.is_finite())
            && self.source_zoom.is_some() == self.source_pivot_offset.is_some()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraTransition {
    start: CameraTransitionPose,
    end: CameraTransitionPose,
}

impl CameraTransition {
    pub fn new(start: CameraTransitionPose, end: CameraTransitionPose) -> Result<Self, String> {
        if !start.valid()
            || !end.valid()
            || start.source_zoom.is_some() != end.source_zoom.is_some()
            || start.source_pivot_offset.is_some() != end.source_pivot_offset.is_some()
        {
            return Err("Camera transition requires compatible finite poses".into());
        }
        Ok(Self { start, end })
    }

    pub fn sample(self, elapsed_ms: f64) -> CameraTransitionPose {
        if !elapsed_ms.is_finite() || elapsed_ms <= 0.0 {
            return self.start;
        }
        if elapsed_ms >= SNAP_DURATION_MS {
            return self.end;
        }
        let t = ease_out_quart(elapsed_ms / SNAP_DURATION_MS);
        let orientation = self
            .start
            .orientation
            .normalize()
            .slerp(self.end.orientation.normalize(), t as f32)
            .normalize();
        CameraTransitionPose {
            orientation,
            target: self.start.target.lerp(self.end.target, t),
            distance: self.start.distance + (self.end.distance - self.start.distance) * t,
            source_zoom: self
                .start
                .source_zoom
                .zip(self.end.source_zoom)
                .map(|(a, b)| a + (b - a) * t),
            source_pivot_offset: self
                .start
                .source_pivot_offset
                .zip(self.end.source_pivot_offset)
                .map(|(a, b)| a.lerp(b, t)),
        }
    }

    pub fn finished(elapsed_ms: f64) -> bool {
        elapsed_ms.is_finite() && elapsed_ms >= SNAP_DURATION_MS
    }
}

pub(crate) fn ease_out_quart(fraction: f64) -> f64 {
    let remaining = 1.0 - fraction.clamp(0.0, 1.0);
    1.0 - remaining * remaining * remaining * remaining
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CameraTransition {
        CameraTransition::new(
            CameraTransitionPose {
                orientation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_3),
                target: DVec3::new(1.25, -0.5, 2.0),
                distance: 8.0,
                source_zoom: Some(8.0),
                source_pivot_offset: Some(DVec3::ZERO),
            },
            CameraTransitionPose {
                orientation: Quat::IDENTITY,
                target: DVec3::new(-0.75, 1.5, 0.0),
                distance: 4.0,
                source_zoom: Some(4.0),
                source_pivot_offset: Some(DVec3::ZERO),
            },
        )
        .unwrap()
    }

    #[test]
    fn pinned_quartic_samples_match_authored_off_axis_pose() {
        for (ms, eased, x, distance, angle) in [
            (0.0, 0.0, 1.25, 8.0, 60.0),
            (50.0, 0.68359375, -0.1171875, 5.265625, 18.984375),
            (100.0, 0.9375, -0.625, 4.25, 3.75),
            (150.0, 0.99609375, -0.7421875, 4.015625, 0.234375),
            (200.0, 1.0, -0.75, 4.0, 0.0),
        ] {
            assert!((ease_out_quart(ms / SNAP_DURATION_MS) - eased).abs() < 1e-12);
            let pose = fixture().sample(ms);
            assert!((pose.target.x - x).abs() < 1e-6);
            assert!((pose.distance - distance).abs() < 1e-6);
            assert!((pose.source_zoom.unwrap() - distance).abs() < 1e-6);
            let current_degrees = pose
                .orientation
                .to_euler(glam::EulerRot::YXZ)
                .0
                .to_degrees();
            assert!((f64::from(current_degrees) - angle).abs() < 1e-5);
            assert!((pose.orientation.length() - 1.0).abs() < 1e-6);
        }
        assert_eq!(fixture().sample(-1.0).target, fixture().sample(0.0).target);
        assert_eq!(
            fixture().sample(1000.0).target,
            fixture().sample(200.0).target
        );
    }

    #[test]
    fn retarget_starts_from_current_sample_and_invalid_pose_rejects() {
        let live = fixture().sample(50.0);
        let next = CameraTransitionPose {
            orientation: Quat::from_rotation_x(0.3),
            target: DVec3::new(3.0, 2.0, -1.0),
            distance: 6.0,
            source_zoom: Some(6.0),
            source_pivot_offset: Some(DVec3::ZERO),
        };
        let transition = CameraTransition::new(live, next).unwrap();
        let at_start = transition.sample(0.0);
        assert_eq!(at_start.target, live.target);
        assert_eq!(at_start.distance, live.distance);
        assert_eq!(at_start.orientation.to_array(), live.orientation.to_array());
        assert_eq!(transition.sample(200.0).target, next.target);
        assert!(CameraTransition::finished(200.0));
        assert!(!CameraTransition::finished(199.0));
        let invalid = CameraTransitionPose {
            distance: f64::NAN,
            ..next
        };
        assert!(CameraTransition::new(live, invalid).is_err());
    }

    #[test]
    fn actual_legacy_camera_capture_apply_is_display_only() {
        let mut camera = Camera::default();
        let source = CameraTransitionPose::capture(&camera);
        let destination = CameraTransitionPose {
            orientation: Quat::from_rotation_y(0.4),
            target: DVec3::new(-1.0, 0.5, 2.0),
            distance: 6.0,
            source_zoom: None,
            source_pivot_offset: None,
        };
        let middle = CameraTransition::new(source, destination)
            .unwrap()
            .sample(100.0);
        middle.apply(&mut camera).unwrap();
        assert_eq!(camera.target.as_dvec3(), middle.target);
        assert_eq!(f64::from(camera.distance), middle.distance);
        assert!(camera.rotation.unwrap().dot(middle.orientation).abs() > 1.0 - 1e-6);
        assert!(
            CameraTransitionPose {
                source_zoom: Some(2.0),
                ..middle
            }
            .apply(&mut camera)
            .is_err()
        );
    }
}
