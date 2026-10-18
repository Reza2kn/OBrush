//! Source-only f64 screen preparation; no camera, alpha or normal UI claim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockedAnchor {
    press: [f64; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockedFrame {
    Hold,
    Dab {
        screen_radius: f64,
        anchored_pick: [f64; 2],
        direction: [f64; 2],
    },
}
impl LockedAnchor {
    pub fn press(&self) -> [f64; 2] {
        self.press
    }
    pub fn new(press: [f64; 2]) -> Result<Self, String> {
        validate_screen(press)?;
        Ok(Self { press })
    }
    pub fn sample(&self, current: [f64; 2]) -> Result<LockedFrame, String> {
        validate_screen(current)?;
        let dx = current[0] - self.press[0];
        let dy = current[1] - self.press[1];
        // Preserve source sqrt(dx*dx+dy*dy) grouping, rather than hypot.
        let radius = (dx * dx + dy * dy).sqrt();
        if radius > 1_000_000.0 {
            return Err("Locked travel exceeds its bounded screen policy".into());
        }
        if radius == 0.0 {
            return Ok(LockedFrame::Hold);
        }
        let direction = [dx / radius, dy / radius];
        Ok(LockedFrame::Dab {
            screen_radius: radius,
            anchored_pick: [
                self.press[0] + direction[0] * 1e-3,
                self.press[1] + direction[1] * 1e-3,
            ],
            direction,
        })
    }
}
fn validate_screen(p: [f64; 2]) -> Result<(), String> {
    if p.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.0) {
        Err("Locked screen input must be bounded finite f64 data".into())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const GATE: f64 = 2e-13;
    #[test]
    fn independent_three_four_five_anchor_perturbation() {
        let LockedFrame::Dab {
            screen_radius,
            anchored_pick,
            direction,
        } = LockedAnchor::new([100., 200.])
            .unwrap()
            .sample([103., 204.])
            .unwrap()
        else {
            panic!("positive travel");
        };
        assert_eq!(screen_radius, 5.);
        assert_eq!(direction, [0.6, 0.8]);
        for (a, e) in anchored_pick.into_iter().zip([100.0006, 200.0008]) {
            assert!((a - e).abs() <= GATE);
        }
    }
    #[test]
    fn independent_cardinal_directions_remain_one_thousandth_pixel() {
        for (pointer, expected) in [
            ([160., 200.], [100.001, 200.]),
            ([40., 200.], [99.999, 200.]),
            ([100., 260.], [100., 200.001]),
            ([100., 140.], [100., 199.999]),
        ] {
            let LockedFrame::Dab {
                screen_radius,
                anchored_pick,
                ..
            } = LockedAnchor::new([100., 200.])
                .unwrap()
                .sample(pointer)
                .unwrap()
            else {
                panic!();
            };
            assert_eq!(screen_radius, 60.);
            for (a, e) in anchored_pick.into_iter().zip(expected) {
                assert!((a - e).abs() <= GATE);
            }
        }
    }
    #[test]
    fn growing_travel_changes_radius_but_not_normalized_anchor_pick() {
        let a = LockedAnchor::new([100., 200.]).unwrap();
        let LockedFrame::Dab {
            screen_radius: r1,
            anchored_pick: p1,
            ..
        } = a.sample([103., 204.]).unwrap()
        else {
            panic!();
        };
        let LockedFrame::Dab {
            screen_radius: r2,
            anchored_pick: p2,
            ..
        } = a.sample([130., 240.]).unwrap()
        else {
            panic!();
        };
        assert_eq!(r2, 10. * r1);
        assert_eq!(p1, p2);
        assert_ne!(p2, [130., 240.]);
    }
    #[test]
    fn travel_can_exceed_saved_five_hundred_pixel_ui_radius() {
        let LockedFrame::Dab { screen_radius, .. } = LockedAnchor::new([0., 0.])
            .unwrap()
            .sample([900., 1200.])
            .unwrap()
        else {
            panic!();
        };
        assert_eq!(screen_radius, 1500.);
    }
    #[test]
    fn zero_travel_is_explicit_hold_without_nan_offset() {
        let a = LockedAnchor::new([100., 200.]).unwrap();
        assert_eq!(a.sample([100., 200.]).unwrap(), LockedFrame::Hold);
        assert_eq!(a.press, [100., 200.]);
    }
    #[test]
    fn f64_anchor_witness_would_be_lost_through_host_float32_position() {
        let LockedFrame::Dab { anchored_pick, .. } = LockedAnchor::new([32768., 4.])
            .unwrap()
            .sample([32769., 4.])
            .unwrap()
        else {
            panic!();
        };
        assert!((anchored_pick[0] - 32768.001).abs() <= GATE);
        assert!((f64::from(anchored_pick[0] as f32) - anchored_pick[0]).abs() >= 0.0009);
    }
    #[test]
    fn invalid_or_excessive_screen_inputs_reject_without_fallback() {
        for p in [[f64::NAN, 0.], [0., f64::INFINITY], [1_000_001., 0.]] {
            assert!(LockedAnchor::new(p).is_err());
        }
        let a = LockedAnchor::new([-1_000_000., 0.]).unwrap();
        assert!(a.sample([1_000_000., 0.]).is_err());
        assert!(a.sample([f64::NEG_INFINITY, 0.]).is_err());
    }
}
