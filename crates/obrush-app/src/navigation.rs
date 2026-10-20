//! Ordered, bounded pointer navigation capture. No document or GPU state.

use std::collections::VecDeque;

const MAX_MOTIONS: usize = 32;
const MAX_COORDINATE: f32 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationButton {
    Orbit,
    Pan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavigationMotion {
    pub button: NavigationButton,
    pub origin: [f32; 2],
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub shift: bool,
    pub first: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavigationPress {
    pub button: NavigationButton,
    pub origin: [f32; 2],
}

#[derive(Clone, Copy)]
struct Contact {
    button: NavigationButton,
    origin: [f32; 2],
    point: [f32; 2],
    first: bool,
}

#[derive(Default)]
pub struct NavigationCapture {
    contact: Option<Contact>,
    presses: VecDeque<NavigationPress>,
    motions: VecDeque<NavigationMotion>,
    skipped_segments: u64,
}

impl NavigationCapture {
    /// A release retains its final displacement using the old held button.
    /// Chord changes terminate that old contact before capturing the new one.
    pub fn pointer(&mut self, buttons: u16, point: [f32; 2], shift: bool) {
        if point
            .iter()
            .any(|v| !v.is_finite() || v.abs() > MAX_COORDINATE)
        {
            self.cancel();
            return;
        }
        let button = if buttons & 4 != 0 {
            Some(NavigationButton::Pan)
        } else if buttons & 2 != 0 {
            Some(NavigationButton::Orbit)
        } else {
            None
        };
        if let Some(mut contact) = self.contact.take() {
            if point != contact.point {
                self.push(NavigationMotion {
                    button: contact.button,
                    origin: contact.origin,
                    start: contact.point,
                    end: point,
                    shift,
                    first: contact.first,
                });
                contact.point = point;
                contact.first = false;
            }
            if button == Some(contact.button) {
                self.contact = Some(contact);
                return;
            }
        }
        if let Some(button) = button {
            if self.presses.len() == MAX_MOTIONS {
                self.presses.pop_front();
                self.skipped_segments = self.skipped_segments.saturating_add(1);
            }
            self.presses.push_back(NavigationPress {
                button,
                origin: point,
            });
            self.contact = Some(Contact {
                button,
                origin: point,
                point,
                first: true,
            });
        }
    }

    fn push(&mut self, motion: NavigationMotion) {
        // Compress only consecutive samples of the same contact and modifiers.
        // Keeping the old start and new end preserves total pan travel.
        if let Some(previous) = self.motions.back_mut()
            && !motion.first
            && previous.button == motion.button
            && previous.origin == motion.origin
            && previous.end == motion.start
            && previous.shift == motion.shift
        {
            previous.end = motion.end;
            return;
        }
        if self.motions.len() == MAX_MOTIONS {
            self.motions.pop_front();
            self.skipped_segments = self.skipped_segments.saturating_add(1);
        }
        self.motions.push_back(motion);
    }

    pub fn pending(&self) -> bool {
        !self.presses.is_empty() || !self.motions.is_empty()
    }
    pub fn held(&self) -> bool {
        self.contact.is_some()
    }
    pub fn take(&mut self) -> VecDeque<NavigationMotion> {
        std::mem::take(&mut self.motions)
    }
    pub fn take_presses(&mut self) -> VecDeque<NavigationPress> {
        std::mem::take(&mut self.presses)
    }
    pub fn skipped_segments(&self) -> u64 {
        self.skipped_segments
    }
    pub fn cancel(&mut self) {
        self.contact = None;
        self.presses.clear();
        self.motions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_move_release_within_one_frame_retains_pan_endpoint_once() {
        let mut capture = NavigationCapture::default();
        capture.pointer(4, [120., 80.], false);
        capture.pointer(4, [130., 85.], false);
        capture.pointer(0, [145., 91.], false);
        assert!(!capture.held());
        assert!(capture.pending());
        assert_eq!(
            capture.take(),
            VecDeque::from([NavigationMotion {
                button: NavigationButton::Pan,
                origin: [120., 80.],
                start: [120., 80.],
                end: [145., 91.],
                shift: false,
                first: true,
            }])
        );
        assert_eq!(
            capture.take_presses(),
            VecDeque::from([NavigationPress {
                button: NavigationButton::Pan,
                origin: [120., 80.],
            }])
        );
        assert!(!capture.pending());
        capture.pointer(0, [160., 90.], false);
        assert!(!capture.pending());
    }

    #[test]
    fn stationary_orbit_press_is_delivered_once_after_release() {
        let mut capture = NavigationCapture::default();
        capture.pointer(2, [240.0, 150.0], false);
        capture.pointer(0, [240.0, 150.0], false);
        assert!(capture.pending());
        assert!(!capture.held());
        assert!(capture.take().is_empty());
        assert_eq!(
            capture.take_presses(),
            VecDeque::from([NavigationPress {
                button: NavigationButton::Orbit,
                origin: [240.0, 150.0],
            }])
        );
        assert!(!capture.pending());
        assert!(capture.take_presses().is_empty());
    }

    #[test]
    fn frame_partition_preserves_total_pan_travel_and_first_contact() {
        let mut capture = NavigationCapture::default();
        capture.pointer(4, [20., 20.], false);
        let mut total = [0.; 2];
        let mut first_count = 0;
        for i in 1..=20 {
            capture.pointer(
                if i == 20 { 0 } else { 4 },
                [20. + i as f32, 20. - i as f32],
                false,
            );
            if i % 3 == 0 || i == 20 {
                for motion in capture.take() {
                    total[0] += motion.end[0] - motion.start[0];
                    total[1] += motion.end[1] - motion.start[1];
                    first_count += usize::from(motion.first);
                    assert_eq!(motion.origin, [20., 20.]);
                }
            }
        }
        assert_eq!(total, [20., -20.]);
        assert_eq!(first_count, 1);
    }

    #[test]
    fn chord_and_shift_changes_do_not_relabel_old_motion() {
        let mut capture = NavigationCapture::default();
        capture.pointer(2, [10., 10.], false);
        capture.pointer(2, [15., 10.], false);
        capture.pointer(6, [20., 10.], true);
        capture.pointer(4, [25., 10.], true);
        capture.pointer(0, [30., 10.], false);
        let motions = capture.take();
        assert_eq!(motions.len(), 4);
        assert_eq!(motions[0].button, NavigationButton::Orbit);
        assert!(!motions[0].shift);
        assert_eq!(motions[1].button, NavigationButton::Orbit);
        assert!(motions[1].shift);
        assert_eq!(motions[2].button, NavigationButton::Pan);
        assert_eq!(motions[2].origin, [20., 10.]);
        assert_eq!(motions[3].button, NavigationButton::Pan);
        assert!(!motions[3].shift);
    }

    #[test]
    fn overload_is_bounded_and_retains_latest_endpoint() {
        let mut capture = NavigationCapture::default();
        capture.pointer(4, [0., 0.], false);
        for i in 1..=1000 {
            capture.pointer(4, [i as f32, 0.], i % 2 == 0);
        }
        capture.pointer(0, [1001., 0.], false);
        let skipped = capture.skipped_segments();
        let motions = capture.take();
        assert_eq!(motions.len(), MAX_MOTIONS);
        assert!(skipped > 900);
        assert_eq!(motions.back().unwrap().end, [1001., 0.]);
        assert!(!capture.held());
    }

    #[test]
    fn cancellation_and_invalid_coordinates_drop_unapplied_motion() {
        for point in [[f32::NAN, 0.], [f32::INFINITY, 0.], [1_000_001., 0.]] {
            let mut capture = NavigationCapture::default();
            capture.pointer(2, [10., 10.], false);
            capture.pointer(2, [30., 10.], false);
            capture.pointer(0, point, false);
            assert!(!capture.held() && !capture.pending());
        }
        let mut capture = NavigationCapture::default();
        capture.pointer(4, [10., 10.], false);
        capture.pointer(4, [30., 10.], false);
        capture.cancel();
        assert!(!capture.held() && !capture.pending());
    }

    #[test]
    fn captured_quick_pan_changes_camera_by_full_delta_and_stays_quiet_after_release() {
        let mut capture = NavigationCapture::default();
        capture.pointer(4, [120., 80.], false);
        capture.pointer(4, [140., 90.], false);
        capture.pointer(0, [152., 96.], false);
        let mut camera = crate::renderer::Camera::default();
        let (right, up, _) = camera.axes();
        let expected =
            camera.target + (-right * 32.0 + up * 16.0) * camera.distance * 0.0015 * camera.speed;
        for motion in capture.take() {
            assert_eq!(motion.button, NavigationButton::Pan);
            camera.pan(eframe::egui::vec2(
                motion.end[0] - motion.start[0],
                motion.end[1] - motion.start[1],
            ));
        }
        assert_eq!(camera.target, expected);
        let released = camera.target;
        for _ in 0..10 {
            assert!(capture.take().is_empty());
            assert_eq!(camera.target, released);
        }
    }
}
