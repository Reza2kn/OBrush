//! Captured projection for a brush radius expressed in logical screen pixels.
//! A queued dab resolves its radius at the actual picked surface depth, rather
//! than at an earlier input frame's geometry. No display or document mutation.
use glam::Vec3;

use crate::renderer::Camera;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScreenFootprint {
    pixels: f32,
    eye: Vec3,
    precise_eye: glam::DVec3,
    precise_forward: glam::DVec3,
    forward: Vec3,
    units: f32,
    /// Optional measured source scale; legacy calculations remain f32 exact.
    source_units: Option<f64>,
    orthographic: bool,
}

impl ScreenFootprint {
    pub(crate) fn capture(camera: &Camera, pixels: f32, height: f32, aspect: f32) -> Option<Self> {
        if !pixels.is_finite()
            || !(5.0..=500.0).contains(&pixels)
            || !height.is_finite()
            || height <= 0.0
            || !aspect.is_finite()
            || aspect <= 0.0
        {
            return None;
        }
        let eye = camera.position();
        let forward = camera.axes().2;
        let units = camera.world_units_per_pixel(1.0, height, aspect);
        (eye.is_finite()
            && forward.is_finite()
            && forward.length_squared() > 0.0
            && units.is_finite()
            && units > 0.0)
            .then_some(Self {
                pixels,
                eye,
                precise_eye: camera.position64(),
                precise_forward: camera.axes64().2,
                forward,
                units,
                source_units: camera
                    .source_frame()
                    .and_then(|frame| frame.horizontal_world_units_per_user_point(1.0).ok()),
                orthographic: camera.orthographic,
            })
    }

    /// Imported local samples are projected in retained world f64 coordinates,
    /// then converted through the explicitly selected source radius scale.
    pub(crate) fn radius_at_world(
        self,
        document: &obrush_core::Document,
        point: Vec3,
    ) -> Option<f32> {
        if crate::scene_space::active_pose(document).is_none() {
            return self.radius_at(point);
        }
        let world = crate::scene_space::world_point(document, point).ok()?;
        let depth = if self.orthographic {
            1.0
        } else {
            (world - self.precise_eye).dot(self.precise_forward)
        };
        let radius = (f64::from(self.pixels)
            * self.source_units.unwrap_or(f64::from(self.units))
            * depth) as f32;
        if !depth.is_finite() || depth <= 0.0 || !radius.is_finite() {
            return None;
        }
        crate::scene_space::local_radius(document, radius).ok()
    }
    pub(crate) fn radius_at(self, point: Vec3) -> Option<f32> {
        if let Some(units) = self.source_units {
            let depth = if self.orthographic {
                1.0
            } else {
                (point.as_dvec3() - self.precise_eye).dot(self.precise_forward)
            };
            let radius = (f64::from(self.pixels) * units * depth) as f32;
            return (point.is_finite()
                && depth.is_finite()
                && depth > 0.0
                && radius.is_finite()
                && radius > 1e-6)
                .then_some(radius);
        }
        let depth = if self.orthographic {
            1.0
        } else {
            (point - self.eye).dot(self.forward)
        };
        let radius = self.pixels * self.units * depth;
        (point.is_finite()
            && depth.is_finite()
            && depth > 0.0
            && radius.is_finite()
            && radius > 1e-6)
            .then_some(radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_reprojects_to_requested_pixels_at_multiple_depths_and_aspects() {
        for orthographic in [false, true] {
            for fov in [10.0, 40.0, 90.0] {
                for aspect in [0.3, 1.0, 2.5] {
                    let camera = Camera {
                        orthographic,
                        fov_degrees: fov,
                        ..Default::default()
                    };
                    let (right, _, forward) = camera.axes();
                    // Evaluate the actual f32 uniforms independently in f64.
                    // f32 project_point3 adds an extra CPU rounding chain which
                    // is not the brush projection contract or a GPU measurement.
                    let projection = camera.view_projection(aspect).as_dmat4();
                    for pixels in [5.0, 75.0, 500.0] {
                        let footprint =
                            ScreenFootprint::capture(&camera, pixels, 1000.0, aspect).unwrap();
                        for depth in [0.5, 2.0, 12.0] {
                            let center = camera.position() + forward * depth;
                            let radius = footprint.radius_at(center).unwrap();
                            let p = projection.project_point3(center.as_dvec3());
                            let q = projection.project_point3((center + right * radius).as_dvec3());
                            let measured = (q.x - p.x).abs() * 1000.0 * f64::from(aspect) * 0.5;
                            assert!(
                                (measured - f64::from(pixels)).abs() < 0.002,
                                "pixels={pixels} measured={measured}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_or_behind_eye_points_do_not_produce_a_radius() {
        let camera = Camera::default();
        let footprint = ScreenFootprint::capture(&camera, 75.0, 1000.0, 1.0).unwrap();
        assert!(
            footprint
                .radius_at(camera.position() - camera.axes().2)
                .is_none()
        );
        assert!(footprint.radius_at(Vec3::splat(f32::NAN)).is_none());
        for pixels in [0.0, 4.0, 501.0, f32::INFINITY] {
            assert!(ScreenFootprint::capture(&camera, pixels, 1000.0, 1.0).is_none());
        }
        assert!(ScreenFootprint::capture(&camera, 75.0, 0.0, 1.0).is_none());
        assert!(ScreenFootprint::capture(&camera, 75.0, 1000.0, f32::NAN).is_none());
    }
}
