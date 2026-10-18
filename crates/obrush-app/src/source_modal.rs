//! Ordered, bounded transient brush-setting ownership. No geometry/history/UI.
//! Source radius uses sqrt(dx*dx+dy*dy), and sliders round positive halves up.
use crate::shortcut_profile::Modal;
pub const MAX_MODAL_QUERIES: usize = 256;
pub const MAX_MODAL_LEDGER_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickQuery {
    pub screen: [f64; 2],
    pub ordinal: u32,
    pub epoch: u64,
}
#[derive(Clone)]
pub struct QueryLedger {
    rows: [PickQuery; MAX_MODAL_QUERIES],
    len: usize,
}
impl Default for QueryLedger {
    fn default() -> Self {
        Self {
            rows: [PickQuery::default(); MAX_MODAL_QUERIES],
            len: 0,
        }
    }
}
impl QueryLedger {
    pub fn rows(&self) -> &[PickQuery] {
        &self.rows[..self.len]
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
    fn push(&mut self, row: PickQuery) -> bool {
        if self.len == MAX_MODAL_QUERIES {
            return false;
        }
        self.rows[self.len] = row;
        self.len += 1;
        true
    }
}
const _: () = assert!(std::mem::size_of::<QueryLedger>() <= MAX_MODAL_LEDGER_BYTES);
#[derive(Default)]
pub struct ModalState {
    pub radius: bool,
    pub intensity: bool,
    pub picker: bool,
    pub primary_down: bool,
    /// An actual focus loss interrupted a contact; only real up or a fresh
    /// admitted primary press closes its stale ownership. No input is forged.
    pub focus_contact_recovery: bool,
    /// None preserves the old first-boot focused admission. After actual loss,
    /// only a processed WindowFocused(true), never final frame focus, gains it.
    pub ordered_window_focus: Option<bool>,
    pub suppress_contact: bool,
    pub suppress_frame: bool,
    pub last_pointer: Option<[f64; 2]>,
    pub radius_anchor: Option<[f64; 2]>,
    pub epoch: u64,
    pub ordinal: u32,
    pub queries: QueryLedger,
    pub last_stop: Option<&'static str>,
}
const _: () = assert!(std::mem::size_of::<Option<bool>>() <= std::mem::size_of::<bool>());
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Movement {
    pub radius: Option<f64>,
    pub intensity: Option<f64>,
}
pub fn positive_round(value: f64) -> f64 {
    (value + 0.5).floor()
}
impl ModalState {
    pub fn active(&self) -> bool {
        self.radius || self.intensity || self.picker
    }
    pub fn settings_active(&self) -> bool {
        self.radius || self.intensity
    }
    pub fn begin_frame(&mut self) {
        // A previous frame must consume/discard its complete ledger. There is no
        // delayed continuation if a host skips its viewport for any reason.
        if !self.queries.rows().is_empty() {
            self.cancel("unconsumed prior frame");
        }
        self.suppress_frame = self.active() || self.suppress_contact;
        self.ordinal = 0;
    }
    pub fn next_event(&mut self) {
        self.ordinal = self.ordinal.saturating_add(1);
    }
    pub fn start(&mut self, modal: Modal, radius: Option<f64>, eligible: bool) -> bool {
        if !eligible
            || self.primary_down
            || (modal == Modal::Picker && self.settings_active())
            || (modal != Modal::Picker && self.picker)
        {
            return false;
        }
        match modal {
            Modal::Radius => {
                let Some((p, r)) = self
                    .last_pointer
                    .zip(radius)
                    .filter(|(_, r)| r.is_finite() && (5.0..=500.0).contains(r))
                else {
                    return false;
                };
                if !self.radius {
                    self.radius_anchor = Some([p[0] - r, p[1]]);
                }
                self.radius = true;
            }
            Modal::Intensity => self.intensity = true,
            Modal::Picker => {
                if !self.picker {
                    self.epoch = self.epoch.wrapping_add(1);
                }
                self.picker = true;
            }
        }
        self.suppress_frame = true;
        self.last_stop = None;
        true
    }
    pub fn end(&mut self, modal: Modal) {
        match modal {
            Modal::Radius => {
                self.radius = false;
                self.radius_anchor = None;
            }
            Modal::Intensity => self.intensity = false,
            Modal::Picker => self.picker = false,
        }
        // End does not discard earlier accepted owned moves in this same frame.
        // The host consumes them once before ordinary viewport contact handling.
    }
    pub fn cancel(&mut self, reason: &'static str) {
        self.radius = false;
        self.intensity = false;
        self.picker = false;
        self.radius_anchor = None;
        self.queries.clear();
        self.epoch = self.epoch.wrapping_add(1);
        self.last_stop = Some(reason);
    }
    pub fn close_picker(&mut self, reason: &'static str, discard: bool) {
        self.picker = false;
        self.last_stop = Some(reason);
        if discard {
            self.queries.clear();
            self.epoch = self.epoch.wrapping_add(1);
        }
    }
    pub fn primary(&mut self, down: bool) {
        if down && (self.active() || self.suppress_frame) {
            self.suppress_contact = true;
            self.suppress_frame = true;
        }
        self.primary_down = down;
        if !down {
            self.suppress_contact = false;
        }
    }
    pub fn move_pointer(
        &mut self,
        p: [f64; 2],
        current_intensity: Option<f64>,
        allow_radius: bool,
    ) -> Movement {
        if p.iter().any(|v| !v.is_finite()) {
            self.cancel("invalid pointer");
            return Movement::default();
        }
        let mut out = Movement::default();
        if self.radius
            && allow_radius
            && let Some(anchor) = self.radius_anchor
        {
            let dx = p[0] - anchor[0];
            let dy = p[1] - anchor[1];
            let value = (dx * dx + dy * dy).sqrt();
            if value.is_finite() {
                out.radius = Some(positive_round(value.clamp(5., 500.)));
            }
        }
        if self.intensity
            && let Some((last, value)) = self.last_pointer.zip(current_intensity)
            && value.is_finite()
            && (0.0..=1.0).contains(&value)
        {
            let percent = positive_round(value * 100.);
            out.intensity = Some(positive_round((percent + p[0] - last[0]).clamp(0., 100.)) / 100.);
        }
        self.last_pointer = Some(p);
        if self.picker {
            let query = PickQuery {
                screen: p,
                ordinal: self.ordinal,
                epoch: self.epoch,
            };
            if !self.queries.push(query) {
                // Keep the already admitted prefix, ending ownership now. The
                // rejected tail is never silently coalesced or replayed later.
                self.close_picker("event ledger budget", false);
            }
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> ModalState {
        ModalState {
            last_pointer: Some([200., 100.]),
            ..Default::default()
        }
    }
    #[test]
    fn m01_ordered_radius_moves_and_release() {
        let mut s = state();
        assert!(s.start(Modal::Radius, Some(50.), true));
        assert_eq!(s.move_pointer([200., 140.], None, true).radius, Some(64.));
        assert_eq!(s.move_pointer([180., 140.], None, true).radius, Some(50.));
        s.end(Modal::Radius);
        assert!(!s.radius);
    }
    #[test]
    fn m02_reentry_reanchors_current_radius() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.end(Modal::Radius);
        s.last_pointer = Some([300., 200.]);
        s.start(Modal::Radius, Some(50.), true);
        assert_eq!(s.move_pointer([300., 200.], None, true).radius, Some(50.));
    }
    #[test]
    fn m03_clamp_positive_half_and_squared_sqrt() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        for (p, v) in [
            ([150., 100.], 5.),
            ([1150., 100.], 500.),
            ([160.5, 100.], 11.),
        ] {
            assert_eq!(s.move_pointer(p, None, true).radius, Some(v));
        }
    }
    #[test]
    fn m04_no_move_no_setting_proposal() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        s.end(Modal::Intensity);
        assert_eq!(s.last_pointer, Some([200., 100.]));
    }
    #[test]
    fn m05_y_only_and_zero_delta_quantize_current_percent() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        assert_eq!(
            s.move_pointer([200., 140.], Some(0.354), false).intensity,
            Some(0.35)
        );
        assert_eq!(s.last_pointer, Some([200., 140.]));
        assert_eq!(
            s.move_pointer([200., 140.], Some(0.355), false).intensity,
            Some(0.36)
        );
    }
    #[test]
    fn m06_ordered_percent_does_not_accumulate_fraction() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        let mut value = 0.35;
        for (p, want) in [
            ([204.5, 100.], 0.40),
            ([203.5, 100.], 0.39),
            ([203.75, 100.], 0.39),
        ] {
            value = s.move_pointer(p, Some(value), false).intensity.unwrap();
            assert_eq!(value, want);
        }
    }
    #[test]
    fn m07_logical_alias_release_is_not_physical_holdcount() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.end(Modal::Radius);
        assert!(!s.radius);
    }
    #[test]
    fn m08_existing_hold_does_not_reanchor() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.last_pointer = Some([400., 400.]);
        s.start(Modal::Radius, Some(100.), true);
        assert_eq!(s.radius_anchor, Some([150., 100.]));
    }
    #[test]
    fn m09_ineligible_editor_start_changes_nothing() {
        let mut s = state();
        assert!(!s.start(Modal::Picker, None, false));
        assert!(!s.active());
        assert!(s.queries.rows().is_empty());
    }
    #[test]
    fn m10_end_clears_owned_flag_without_idle_admission() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        s.end(Modal::Intensity);
        assert!(!s.intensity);
    }
    #[test]
    fn m11_earlier_primary_down_blocks_modal() {
        let mut s = state();
        s.primary(true);
        assert!(!s.start(Modal::Radius, Some(50.), true));
    }
    #[test]
    fn m12_suppressed_primary_survives_setting_keyup() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.primary(true);
        s.end(Modal::Radius);
        assert!(s.suppress_contact);
        s.primary(false);
        assert!(!s.suppress_contact);
        assert!(s.suppress_frame);
    }
    #[test]
    fn m13_independent_flags_retain_other_owner() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.start(Modal::Intensity, None, true);
        s.end(Modal::Intensity);
        assert!(s.radius);
        assert!(!s.intensity);
    }
    #[test]
    fn m14_cancel_is_transient_and_discards_queries() {
        let mut s = state();
        s.start(Modal::Picker, None, true);
        s.move_pointer([201., 100.], None, false);
        s.cancel("Escape");
        assert!(!s.active());
        assert!(s.queries.rows().is_empty());
        assert_eq!(s.last_pointer, Some([201., 100.]));
    }
    #[test]
    fn m15_next_move_uses_current_owner_intensity() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        assert_eq!(
            s.move_pointer([205., 100.], Some(0.8), false).intensity,
            Some(0.85)
        );
    }
    #[test]
    fn m16_world_radius_has_no_implicit_conversion() {
        let mut s = state();
        assert!(!s.start(Modal::Radius, None, true));
        assert!(!s.radius);
    }
    #[test]
    fn m17_source_fractional_anchor_uses_number() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.25), true);
        assert_eq!(s.radius_anchor, Some([149.75, 100.]));
    }
    #[test]
    fn m18_radius_only_tool_has_no_intensity_proposal() {
        let mut s = state();
        s.start(Modal::Intensity, None, true);
        assert_eq!(s.move_pointer([220., 100.], None, false).intensity, None);
    }
    #[test]
    fn s02_release_keeps_sameframe_owned_prefix() {
        let mut s = state();
        s.start(Modal::Picker, None, true);
        s.next_event();
        s.move_pointer([201., 100.], None, false);
        s.end(Modal::Picker);
        s.move_pointer([202., 100.], None, false);
        assert_eq!(s.queries.rows().len(), 1);
        assert_eq!(s.queries.rows()[0].screen, [201., 100.]);
    }
    #[test]
    fn s12_mouseup_ends_picker_but_keeps_prefix() {
        let mut s = state();
        s.start(Modal::Picker, None, true);
        s.move_pointer([201., 100.], None, false);
        s.close_picker("mouse up", false);
        assert!(!s.picker);
        assert_eq!(s.queries.rows().len(), 1);
    }
    #[test]
    fn s16_ledger_ends_without_latest_only_coalescing() {
        let mut s = state();
        s.start(Modal::Picker, None, true);
        for i in 0..257 {
            s.next_event();
            s.move_pointer([f64::from(i), 100.], None, false);
        }
        assert!(!s.picker);
        assert_eq!(s.queries.rows().len(), 256);
        assert_eq!(s.queries.rows()[255].screen, [255., 100.]);
    }
    #[test]
    fn b11_no_unconsumed_prior_frame_catchup() {
        let mut s = state();
        s.start(Modal::Picker, None, true);
        s.move_pointer([201., 100.], None, false);
        s.begin_frame();
        assert!(!s.picker);
        assert!(s.queries.rows().is_empty());
    }
    #[test]
    fn b15_picker_and_settings_are_exclusive() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        assert!(!s.start(Modal::Picker, None, true));
    }
    #[test]
    fn m12_multiple_primary_epochs_in_a_suppressed_frame_stay_suppressed() {
        let mut s = state();
        s.start(Modal::Radius, Some(50.), true);
        s.primary(true);
        s.end(Modal::Radius);
        s.primary(false);
        s.primary(true);
        s.begin_frame();
        assert!(s.suppress_contact);
        assert!(s.suppress_frame);
        s.primary(false);
        assert!(!s.suppress_contact);
    }
}
