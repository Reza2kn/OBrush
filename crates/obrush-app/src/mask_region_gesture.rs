//! Bounded, nonmutating screen-region gesture capture for both application hosts.

use eframe::egui::{Pos2, Rect};
use obrush_core::{MaskRegion, MaskRegionSettings};

pub const MAX_GESTURE_POINTS: usize = obrush_core::MAX_MASK_REGION_POINTS;
const MAX_SCREEN_COORDINATE: f32 = 1_000_000.0;

/// DOM chords can arrive as pointer moves instead of button-down events. Keep
/// only the latest navigation buttons and an interruption latch until consumed.
#[derive(Default)]
pub struct BrowserNavigationButtons {
    held: u16,
    pending_cancel: bool,
}

impl BrowserNavigationButtons {
    pub fn update(&mut self, buttons: u16) {
        let held = buttons & 6; // DOM Secondary=2, Middle=4.
        self.pending_cancel |= held & !self.held != 0;
        self.held = held;
    }

    pub fn navigation_down(&self) -> bool {
        self.held != 0
    }

    pub fn secondary_down(&self) -> bool {
        self.held & 2 != 0
    }

    pub fn middle_down(&self) -> bool {
        self.held & 4 != 0
    }

    pub fn take_cancel_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_cancel)
    }

    pub fn cancel_contact(&mut self) {
        self.held = 0;
        self.pending_cancel = true;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MaskSelectionMode {
    #[default]
    Brush,
    Rectangle,
    Lasso,
}

impl MaskSelectionMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Rectangle => "Rectangle",
            Self::Lasso => "Lasso",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MaskRegionGesture {
    pub mode: MaskSelectionMode,
    pub viewport: Rect,
    pub view_projection: [f32; 16],
    pub subtool_id: u32,
    pub revision: u64,
    pub erase: bool,
    pub start: Pos2,
    pub end: Pos2,
    pub points: Vec<Pos2>,
    pub max_travel_sq: f32,
}

impl MaskRegionGesture {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: MaskSelectionMode,
        viewport: Rect,
        view_projection: [f32; 16],
        subtool_id: u32,
        revision: u64,
        start: Pos2,
        erase: bool,
    ) -> Result<Self, String> {
        if mode == MaskSelectionMode::Brush
            || !viewport.is_finite()
            || viewport.width() <= 0.0
            || viewport.height() <= 0.0
            || viewport.min.x.abs() > MAX_SCREEN_COORDINATE
            || viewport.min.y.abs() > MAX_SCREEN_COORDINATE
            || viewport.max.x.abs() > MAX_SCREEN_COORDINATE
            || viewport.max.y.abs() > MAX_SCREEN_COORDINATE
            || viewport.width() > MAX_SCREEN_COORDINATE
            || viewport.height() > MAX_SCREEN_COORDINATE
            || view_projection.iter().any(|value| !value.is_finite())
        {
            return Err("This viewport cannot capture a mask region.".into());
        }
        let start = clamped_pointer(viewport, start)?;
        Ok(Self {
            mode,
            viewport,
            view_projection,
            subtool_id,
            revision,
            erase,
            start,
            end: start,
            points: vec![start],
            max_travel_sq: 0.0,
        })
    }

    /// Every observed lasso move is retained, including repeated points. Only
    /// the final release endpoint is omitted when it equals the last move.
    pub fn move_to(&mut self, point: Pos2) -> Result<(), String> {
        let point = clamped_pointer(self.viewport, point)?;
        if self.mode == MaskSelectionMode::Lasso {
            if self.points.len() >= MAX_GESTURE_POINTS {
                return Err(
                    "Lasso cancelled: too many points. Draw a shorter gesture (up to 512 points)."
                        .into(),
                );
            }
            self.points.push(point);
        }
        self.end = point;
        self.max_travel_sq = self.max_travel_sq.max(self.start.distance_sq(point));
        Ok(())
    }

    pub fn release_at(&mut self, point: Pos2) -> Result<(), String> {
        let point = clamped_pointer(self.viewport, point)?;
        if point != self.end {
            self.move_to(point)?;
        }
        Ok(())
    }

    pub fn tiny_click(&self) -> bool {
        self.max_travel_sq <= 1.0
    }

    pub fn settings(&self) -> MaskRegionSettings {
        let coordinate = |point: Pos2| [f64::from(point.x), f64::from(point.y)];
        let region = match self.mode {
            MaskSelectionMode::Rectangle | MaskSelectionMode::Brush => MaskRegion::Rectangle {
                start: coordinate(self.start),
                end: coordinate(self.end),
            },
            MaskSelectionMode::Lasso => MaskRegion::Lasso {
                points: self.points.iter().copied().map(coordinate).collect(),
            },
        };
        MaskRegionSettings {
            region,
            view_projection: self.view_projection,
            viewport: [
                f64::from(self.viewport.left()),
                f64::from(self.viewport.top()),
                f64::from(self.viewport.width()),
                f64::from(self.viewport.height()),
            ],
            erase: self.erase,
        }
    }
}

fn clamped_pointer(viewport: Rect, point: Pos2) -> Result<Pos2, String> {
    if !point.is_finite()
        || point.x.abs() > MAX_SCREEN_COORDINATE
        || point.y.abs() > MAX_SCREEN_COORDINATE
    {
        return Err("Selection cancelled: pointer coordinates exceed mask-region limits.".into());
    }
    Ok(Pos2::new(
        point.x.clamp(viewport.left(), viewport.right()),
        point.y.clamp(viewport.top(), viewport.bottom()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gesture(mode: MaskSelectionMode) -> MaskRegionGesture {
        let viewport = Rect::from_min_max(Pos2::new(212.0, 60.0), Pos2::new(1365.0, 970.0));
        MaskRegionGesture::new(
            mode,
            viewport,
            glam::Mat4::IDENTITY.to_cols_array(),
            7,
            19,
            Pos2::new(300.0, 200.0),
            true,
        )
        .unwrap()
    }

    #[test]
    fn rectangle_capture_and_outside_release_use_frozen_coordinates() {
        let mut region = gesture(MaskSelectionMode::Rectangle);
        region.move_to(Pos2::new(600.0, 500.0)).unwrap();
        region.release_at(Pos2::new(2000.0, -20.0)).unwrap();
        assert_eq!(region.start, Pos2::new(300.0, 200.0));
        assert_eq!(region.end, Pos2::new(1365.0, 60.0));
        assert_eq!(region.subtool_id, 7);
        assert_eq!(region.revision, 19);
        assert!(region.erase);
        let settings = region.settings();
        assert_eq!(settings.viewport, [212.0, 60.0, 1153.0, 910.0]);
        assert_eq!(
            settings.view_projection,
            glam::Mat4::IDENTITY.to_cols_array()
        );
    }

    #[test]
    fn lasso_retains_revisits_and_rejects_overflow_without_partial_append() {
        let mut region = gesture(MaskSelectionMode::Lasso);
        for _ in 1..MAX_GESTURE_POINTS {
            region.move_to(region.start).unwrap();
        }
        assert_eq!(region.points.len(), MAX_GESTURE_POINTS);
        let previous = region.points.clone();
        assert!(region.move_to(Pos2::new(310.0, 210.0)).is_err());
        assert_eq!(region.points, previous);
        assert_eq!(region.end, region.start);
        region.release_at(region.end).unwrap();
        assert_eq!(region.points.len(), MAX_GESTURE_POINTS);
    }

    #[test]
    fn invalid_motion_is_rejected_and_zero_click_remains_tiny() {
        let mut region = gesture(MaskSelectionMode::Lasso);
        assert!(region.tiny_click());
        assert!(region.move_to(Pos2::new(f32::NAN, 200.0)).is_err());
        assert_eq!(region.points.len(), 1);
        region.move_to(Pos2::new(301.0, 200.0)).unwrap();
        assert!(region.tiny_click());
        region.move_to(Pos2::new(302.0, 200.0)).unwrap();
        assert!(!region.tiny_click());
    }

    #[test]
    fn large_closed_lasso_is_not_treated_as_a_tiny_click() {
        let mut region = gesture(MaskSelectionMode::Lasso);
        region.move_to(Pos2::new(500.0, 200.0)).unwrap();
        region.move_to(Pos2::new(500.0, 400.0)).unwrap();
        region.move_to(Pos2::new(300.0, 400.0)).unwrap();
        region.release_at(region.start).unwrap();
        assert_eq!(region.start, region.end);
        assert!(!region.tiny_click());
        assert_eq!(region.points.len(), 5);
    }

    #[test]
    fn dom_navigation_buttons_cover_secondary_middle_both_and_primary_only() {
        for buttons in [2, 4, 6, 3, 5, 7] {
            let mut input = BrowserNavigationButtons::default();
            input.update(buttons);
            assert!(input.navigation_down());
            assert!(input.take_cancel_request());
            assert!(!input.take_cancel_request());
            assert!(input.navigation_down());
        }
        for buttons in [0, 1, 8, 16] {
            let mut input = BrowserNavigationButtons::default();
            input.update(buttons);
            assert!(!input.navigation_down());
            assert!(!input.take_cancel_request());
        }
    }

    #[test]
    fn dom_fast_chord_release_preserves_latch_then_allows_a_fresh_gesture() {
        for chord in [3, 5, 7] {
            let mut input = BrowserNavigationButtons::default();
            input.update(1);
            input.update(chord);
            input.update(1);
            input.update(0);
            assert!(!input.navigation_down());
            assert!(input.take_cancel_request());
            assert!(!input.take_cancel_request());
            input.update(1);
            assert!(!input.navigation_down());
            assert!(!input.take_cancel_request());
            // A later chord is independently detected after consumption.
            input.update(chord);
            assert!(input.navigation_down());
            assert!(input.take_cancel_request());
        }
    }

    #[test]
    fn dom_cancel_or_blur_reset_keeps_the_pending_cancel_without_stale_buttons() {
        let mut input = BrowserNavigationButtons::default();
        input.update(7);
        input.cancel_contact();
        assert!(!input.navigation_down());
        assert!(input.take_cancel_request());
        assert!(!input.take_cancel_request());
        input.update(1);
        assert!(!input.navigation_down());
        assert!(!input.take_cancel_request());
    }

    #[test]
    fn dom_primary_contact_cancel_latches_without_prior_navigation_buttons() {
        let mut input = BrowserNavigationButtons::default();
        input.update(1);
        assert!(!input.navigation_down());
        assert!(!input.take_cancel_request());
        input.cancel_contact();
        assert!(!input.navigation_down());
        assert!(
            input.take_cancel_request(),
            "primary-only pointercancel must cancel a region"
        );
        assert!(!input.take_cancel_request());
        input.update(1);
        assert!(!input.navigation_down());
        assert!(!input.take_cancel_request());
    }

    #[test]
    fn dom_primary_contact_normal_pointerup_does_not_latch_cancellation() {
        let mut input = BrowserNavigationButtons::default();
        input.update(1);
        input.update(0);
        assert!(!input.navigation_down());
        assert!(!input.take_cancel_request());
    }
}
