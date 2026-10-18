//! SculptGL-style Ctrl gesture that starts in empty viewport space.
//! Geometry and the undoable mask transaction stay in `obrush-core`.
use eframe::egui::{Pos2, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMaskAction {
    Invert,
    Clear,
}

#[derive(Clone, Copy, Debug)]
pub struct BackgroundMaskGesture {
    start: Pos2,
    last: Pos2,
    viewport: Rect,
    subtool_id: u32,
    revision: u64,
}

impl BackgroundMaskGesture {
    pub fn new(start: Pos2, viewport: Rect, subtool_id: u32, revision: u64) -> Option<Self> {
        (start.is_finite() && viewport.contains(start)).then_some(Self {
            start,
            last: start,
            viewport,
            subtool_id,
            revision,
        })
    }

    pub fn matches(&self, viewport: Rect, subtool_id: Option<u32>, revision: u64) -> bool {
        self.viewport == viewport
            && subtool_id == Some(self.subtool_id)
            && self.revision == revision
    }

    /// The pinned reference compares its last mouse-move coordinates with
    /// the press coordinates at release. Moving away and back is a click.
    pub fn move_to(&mut self, position: Pos2) {
        if position.is_finite() {
            self.last = position;
        }
    }

    pub fn release(self) -> BackgroundMaskAction {
        if self.last == self.start {
            BackgroundMaskAction::Invert
        } else {
            BackgroundMaskAction::Clear
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_click_inverts_and_background_drag_clears() {
        let rect = Rect::from_min_max(Pos2::new(0., 0.), Pos2::new(100., 100.));
        let original = BackgroundMaskGesture::new(Pos2::new(5., 7.), rect, 3, 4).unwrap();
        assert_eq!(original.release(), BackgroundMaskAction::Invert);
        let mut dragged = original;
        dragged.move_to(Pos2::new(6., 7.));
        assert_eq!(dragged.release(), BackgroundMaskAction::Clear);
        dragged.move_to(Pos2::new(5., 7.));
        assert_eq!(dragged.release(), BackgroundMaskAction::Invert);
    }

    #[test]
    fn stale_document_viewport_and_nonfinite_input_are_not_admitted() {
        let rect = Rect::from_min_max(Pos2::new(0., 0.), Pos2::new(100., 100.));
        let gesture = BackgroundMaskGesture::new(Pos2::new(5., 7.), rect, 3, 4).unwrap();
        assert!(gesture.matches(rect, Some(3), 4));
        assert!(!gesture.matches(rect, Some(4), 4));
        assert!(!gesture.matches(rect, Some(3), 5));
        assert!(!gesture.matches(rect.translate(eframe::egui::Vec2::X), Some(3), 4));
        assert!(BackgroundMaskGesture::new(Pos2::new(f32::NAN, 7.), rect, 3, 4).is_none());
        assert!(BackgroundMaskGesture::new(Pos2::new(105., 7.), rect, 3, 4).is_none());
    }
}
