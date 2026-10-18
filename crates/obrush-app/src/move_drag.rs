//! A Move gesture uses the camera plane and depth captured at pointer-down.
use eframe::egui::Pos2;
use glam::{DVec2, Vec3};

pub(crate) struct MoveDrag {
    right: Vec3,
    up: Vec3,
    units_per_pixel: f64,
    pixels: DVec2,
}

impl MoveDrag {
    pub(crate) fn new(right: Vec3, up: Vec3, units_per_pixel: f32) -> Self {
        Self {
            right,
            up,
            units_per_pixel: f64::from(units_per_pixel),
            pixels: DVec2::ZERO,
        }
    }

    pub(crate) fn advance(&mut self, from: Pos2, to: Pos2) -> Vec3 {
        if from.x.is_finite() && from.y.is_finite() && to.x.is_finite() && to.y.is_finite() {
            self.pixels += DVec2::new(
                f64::from(to.x) - f64::from(from.x),
                f64::from(to.y) - f64::from(from.y),
            );
        }
        ((self.right.as_dvec3() * self.pixels.x - self.up.as_dvec3() * self.pixels.y)
            * self.units_per_pixel)
            .as_vec3()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_chunking_and_round_trip_keep_the_same_camera_plane_target() {
        let mut coarse = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        let target = coarse.advance(Pos2::ZERO, Pos2::new(160.0, -80.0));
        let mut fine = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        for step in 0..160 {
            fine.advance(
                Pos2::new(step as f32, -0.5 * step as f32),
                Pos2::new((step + 1) as f32, -0.5 * (step + 1) as f32),
            );
        }
        assert_eq!(fine.advance(Pos2::ZERO, Pos2::ZERO), target);
        assert_eq!(
            fine.advance(Pos2::new(160.0, -80.0), Pos2::ZERO),
            Vec3::ZERO
        );

        let mut wide = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        let start = Pos2::new(1111.1234, 1200.4321);
        let middle = Pos2::new(16.9876, 17.6543);
        let end = Pos2::new(1777.8765, 9.8765);
        wide.advance(start, middle);
        wide.advance(middle, end);
        assert_eq!(wide.advance(end, start), Vec3::ZERO);
    }

    #[test]
    fn invalid_input_does_not_change_the_target() {
        let mut drag = MoveDrag::new(Vec3::X, Vec3::Z, 0.003);
        let target = drag.advance(Pos2::ZERO, Pos2::new(3.0, 2.0));
        assert_eq!(drag.advance(Pos2::ZERO, Pos2::new(f32::NAN, 10.0)), target);
        assert_eq!(
            drag.advance(Pos2::new(1.0, f32::INFINITY), Pos2::ZERO),
            target
        );
    }
}
