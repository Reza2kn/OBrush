//! Source-only prototype: permits keyed to the actual host stroke, not a second
//! gesture/history engine. A permit is consumed synchronously in its poll frame.
use obrush_core::BrushKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokeKey {
    pub serial: u64,
    pub subtool_id: u32,
    pub kind: BrushKind,
}

pub fn supports_continuous(kind: BrushKind, locked: bool) -> bool {
    !locked
        && matches!(
            kind,
            BrushKind::Draw
                | BrushKind::Clay
                | BrushKind::Inflate
                | BrushKind::Smooth
                | BrushKind::Flatten
                | BrushKind::Crease
                | BrushKind::Pinch
                | BrushKind::Mask
                | BrushKind::Paint
        )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousMode {
    Ordinary,
    LockedAccumulation,
}

pub fn supports_continuous_locked(kind: BrushKind) -> bool {
    obrush_core::LockedAccumulator::supports_kind(kind)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockPolicy {
    pub period_micros: u64,
    pub pause_gap_micros: u64,
}
impl Default for ClockPolicy {
    fn default() -> Self {
        Self {
            period_micros: 16_600,
            pause_gap_micros: 100_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Released,
    NoContact,
    InvalidContact,
    FocusLost,
    Navigation,
    TargetLost,
    StrokeChanged,
    ModeChanged,
    ClockReversed,
    FrameReversed,
    ClockOverflow,
    PermitExhausted,
    ExpiredPermit,
    Command,
}

#[derive(Clone, Copy, Debug)]
pub struct Readiness {
    pub key: Option<StrokeKey>,
    pub mode_enabled: bool,
    pub primary_down: bool,
    pub pressure: f32,
    pub focused: bool,
    pub navigating: bool,
    pub target_visible: bool,
    pub pointer_in_viewport: bool,
    pub surface_hit: bool,
    /// Require no waiting work, not merely space remaining in the queue.
    pub queue_empty: bool,
    pub operation_busy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickPermit {
    key: StrokeKey,
    mode: ContinuousMode,
    frame: u64,
    serial: u64,
    at_micros: u64,
}
impl TickPermit {
    pub fn mode(self) -> ContinuousMode {
        self.mode
    }
    pub fn key(self) -> StrokeKey {
        self.key
    }
    pub fn at_micros(self) -> u64 {
        self.at_micros
    }
}

#[derive(Clone, Copy, Debug)]
struct ActiveClock {
    key: StrokeKey,
    mode: ContinuousMode,
    last_poll: u64,
    last_frame: Option<u64>,
    next_due: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockReport {
    pub active_key: Option<StrokeKey>,
    pub active_mode: Option<ContinuousMode>,
    pub outstanding_permit: bool,
    pub consumed_ticks: u64,
    pub skipped_polls: u64,
    pub expired_permits: u64,
    pub last_stop: Option<StopReason>,
}

pub struct HostContinuousClock {
    policy: ClockPolicy,
    state: Option<ActiveClock>,
    outstanding: Option<TickPermit>,
    issued: u64,
    consumed: u64,
    skipped: u64,
    expired: u64,
    last_stop: Option<StopReason>,
}
impl HostContinuousClock {
    pub fn new(policy: ClockPolicy) -> Result<Self, String> {
        if !(1_000..=100_000).contains(&policy.period_micros)
            || policy.pause_gap_micros < policy.period_micros
            || policy.pause_gap_micros > 1_000_000
        {
            return Err("Continuous period/pause policy exceeds its bounds".into());
        }
        Ok(Self {
            policy,
            state: None,
            outstanding: None,
            issued: 0,
            consumed: 0,
            skipped: 0,
            expired: 0,
            last_stop: None,
        })
    }
    pub fn start(
        &mut self,
        key: StrokeKey,
        mode_enabled: bool,
        locked: bool,
        now_micros: u64,
    ) -> Result<(), String> {
        if self.state.is_some() || !mode_enabled || !supports_continuous(key.kind, locked) {
            return Err(
                "Continuous start requires an idle clock and a supported ordinary stroke".into(),
            );
        }
        self.start_mode(key, ContinuousMode::Ordinary, now_micros)
    }
    /// New typed branch; the ordinary start(...locked=true) contract remains
    /// rejected. Both modes share the same absolute no-backlog permit policy.
    pub fn start_locked_accumulation(
        &mut self,
        key: StrokeKey,
        mode_enabled: bool,
        now_micros: u64,
    ) -> Result<(), String> {
        if self.state.is_some() || !mode_enabled || !supports_continuous_locked(key.kind) {
            return Err(
                "Locked accumulation needs an idle clock and a supported captured tool".into(),
            );
        }
        self.start_mode(key, ContinuousMode::LockedAccumulation, now_micros)
    }
    fn start_mode(
        &mut self,
        key: StrokeKey,
        mode: ContinuousMode,
        now_micros: u64,
    ) -> Result<(), String> {
        let next_due = now_micros
            .checked_add(self.policy.period_micros)
            .ok_or("Continuous timestamp overflow")?;
        self.state = Some(ActiveClock {
            key,
            mode,
            last_poll: now_micros,
            last_frame: None,
            next_due,
        });
        self.outstanding = None;
        self.last_stop = None;
        Ok(())
    }
    pub fn stop(&mut self, reason: StopReason) {
        self.state = None;
        self.outstanding = None;
        self.last_stop = Some(reason);
    }
    pub fn report(&self) -> ClockReport {
        ClockReport {
            active_key: self.state.map(|state| state.key),
            active_mode: self.state.map(|state| state.mode),
            outstanding_permit: self.outstanding.is_some(),
            consumed_ticks: self.consumed,
            skipped_polls: self.skipped,
            expired_permits: self.expired,
            last_stop: self.last_stop,
        }
    }
    /// Repaint against the absolute next deadline, without moving it forward
    /// merely because another UI/input frame happened before the timer was due.
    pub fn micros_until_due(&self, now_micros: u64) -> Option<u64> {
        self.state
            .map(|state| state.next_due.saturating_sub(now_micros))
    }
    /// At most one permit for a frame; a pause, pending work or unconsumed old
    /// permit resets the full period. There is no elapsed-tick loop or backlog.
    pub fn poll(&mut self, frame: u64, now_micros: u64, ready: Readiness) -> Option<TickPermit> {
        let mut state = self.state?;
        if let Some(reason) = readiness_stop(state.key, ready) {
            self.stop(reason);
            return None;
        }
        if now_micros < state.last_poll {
            self.stop(StopReason::ClockReversed);
            return None;
        }
        if state.last_frame.is_some_and(|last| frame < last) {
            self.stop(StopReason::FrameReversed);
            return None;
        }
        if state.last_frame == Some(frame) {
            return None;
        }
        let next = match now_micros.checked_add(self.policy.period_micros) {
            Some(next) => next,
            None => {
                self.stop(StopReason::ClockOverflow);
                return None;
            }
        };
        let paused = now_micros - state.last_poll > self.policy.pause_gap_micros;
        let expired = self.outstanding.take().is_some();
        state.last_poll = now_micros;
        state.last_frame = Some(frame);
        if paused || expired || !ready.queue_empty || ready.operation_busy {
            state.next_due = next;
            self.skipped = self.skipped.saturating_add(1);
            self.expired = self.expired.saturating_add(u64::from(expired));
            self.state = Some(state);
            return None;
        }
        if now_micros < state.next_due {
            self.state = Some(state);
            return None;
        }
        let serial = match self.issued.checked_add(1) {
            Some(serial) => serial,
            None => {
                self.stop(StopReason::PermitExhausted);
                return None;
            }
        };
        self.issued = serial;
        let permit = TickPermit {
            key: state.key,
            mode: state.mode,
            frame,
            serial,
            at_micros: now_micros,
        };
        state.next_due = next;
        self.state = Some(state);
        self.outstanding = Some(permit);
        Some(permit)
    }
    /// Consume immediately before the atomic dab. Re-read current readiness;
    /// stale permits do not disturb a newer stroke. No geometry/history lives here.
    pub fn consume(
        &mut self,
        permit: TickPermit,
        frame: u64,
        ready: Readiness,
    ) -> Result<StrokeKey, String> {
        if self.outstanding != Some(permit) {
            return Err("Continuous permit is stale or already consumed".into());
        }
        let state = self.state.ok_or("Continuous stroke is inactive")?;
        if state.mode != permit.mode {
            return Err("Continuous permit belongs to a different gesture mode".into());
        }
        if let Some(reason) = readiness_stop(state.key, ready) {
            self.stop(reason);
            return Err("Continuous input was interrupted before the dab".into());
        }
        if frame != permit.frame {
            self.expired = self.expired.saturating_add(1);
            self.stop(StopReason::ExpiredPermit);
            return Err("Continuous permit cannot outlive its frame".into());
        }
        if !ready.queue_empty || ready.operation_busy {
            self.outstanding = None;
            self.skipped = self.skipped.saturating_add(1);
            return Err("Continuous dab was skipped because work is already pending".into());
        }
        self.outstanding = None;
        self.consumed = self.consumed.saturating_add(1);
        Ok(permit.key)
    }
}

fn readiness_stop(key: StrokeKey, ready: Readiness) -> Option<StopReason> {
    if ready.key != Some(key) {
        Some(StopReason::StrokeChanged)
    } else if !ready.mode_enabled {
        Some(StopReason::ModeChanged)
    } else if !ready.primary_down {
        Some(StopReason::Released)
    } else if !ready.pressure.is_finite() || ready.pressure > 1.0 {
        Some(StopReason::InvalidContact)
    } else if ready.pressure <= 0.0 {
        Some(StopReason::NoContact)
    } else if !ready.focused {
        Some(StopReason::FocusLost)
    } else if ready.navigating {
        Some(StopReason::Navigation)
    } else if !ready.target_visible || !ready.pointer_in_viewport || !ready.surface_hit {
        Some(StopReason::TargetLost)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permit_serial_exhaustion_stops_without_wrapping_a_valid_old_id() {
        let key = StrokeKey {
            serial: 1,
            subtool_id: 1,
            kind: BrushKind::Draw,
        };
        let mut c = HostContinuousClock::new(ClockPolicy::default()).unwrap();
        c.issued = u64::MAX;
        c.start(key, true, false, 0).unwrap();
        let ready = Readiness {
            key: Some(key),
            mode_enabled: true,
            primary_down: true,
            pressure: 1.0,
            focused: true,
            navigating: false,
            target_visible: true,
            pointer_in_viewport: true,
            surface_hit: true,
            queue_empty: true,
            operation_busy: false,
        };
        assert!(c.poll(1, 16_600, ready).is_none());
        assert_eq!(c.report().last_stop, Some(StopReason::PermitExhausted));
        assert_eq!(c.report().consumed_ticks, 0);
        assert!(!c.report().outstanding_permit);
    }
    #[test]
    fn repaint_countdown_keeps_the_absolute_deadline_and_stops_with_the_clock() {
        let mut c = HostContinuousClock::new(ClockPolicy::default()).unwrap();
        let key = StrokeKey {
            serial: 1,
            subtool_id: 1,
            kind: BrushKind::Draw,
        };
        c.start(key, true, false, 0).unwrap();
        assert_eq!(c.micros_until_due(0), Some(16_600));
        assert_eq!(c.micros_until_due(8_000), Some(8_600));
        assert_eq!(c.micros_until_due(16_599), Some(1));
        assert_eq!(c.micros_until_due(16_600), Some(0));
        c.stop(StopReason::Released);
        assert_eq!(c.micros_until_due(16_600), None);
    }
}

#[cfg(test)]
mod locked_mode_tests {
    use super::*;
    fn key() -> StrokeKey {
        StrokeKey {
            serial: 41,
            subtool_id: 1,
            kind: BrushKind::Paint,
        }
    }
    fn ready() -> Readiness {
        Readiness {
            key: Some(key()),
            mode_enabled: true,
            primary_down: true,
            pressure: 1.0,
            focused: true,
            navigating: false,
            target_visible: true,
            pointer_in_viewport: true,
            surface_hit: true,
            queue_empty: true,
            operation_busy: false,
        }
    }
    fn clock() -> HostContinuousClock {
        HostContinuousClock::new(ClockPolicy::default()).unwrap()
    }
    #[test]
    fn old_ordinary_locked_argument_still_rejects_without_changing_clock() {
        let mut c = clock();
        assert!(c.start(key(), true, true, 0).is_err());
        assert_eq!(c.report().active_mode, None);
        c.start_locked_accumulation(key(), true, 0).unwrap();
        assert_eq!(
            c.report().active_mode,
            Some(ContinuousMode::LockedAccumulation)
        );
    }
    #[test]
    fn every_exposed_nine_tool_can_start_typed_lock_and_transform_kinds_cannot() {
        for kind in BrushKind::ALL {
            let k = StrokeKey { kind, ..key() };
            let mut c = clock();
            let result = c.start_locked_accumulation(k, true, 0);
            assert_eq!(
                result.is_ok(),
                matches!(
                    kind,
                    BrushKind::Draw
                        | BrushKind::Clay
                        | BrushKind::Inflate
                        | BrushKind::Smooth
                        | BrushKind::Flatten
                        | BrushKind::Crease
                        | BrushKind::Pinch
                        | BrushKind::Mask
                        | BrushKind::Paint
                )
            );
        }
    }
    #[test]
    fn typed_lock_first_full_delay_and_one_permission_in_each_actual_frame() {
        let mut c = clock();
        c.start_locked_accumulation(key(), true, 0).unwrap();
        assert!(c.poll(1, 16_599, ready()).is_none());
        let p = c.poll(2, 16_600, ready()).unwrap();
        assert_eq!(p.mode(), ContinuousMode::LockedAccumulation);
        c.consume(p, 2, ready()).unwrap();
        assert!(c.poll(2, 33_200, ready()).is_none());
        assert_eq!(c.report().consumed_ticks, 1);
    }
    #[test]
    fn long_paused_locked_frame_never_catches_up_old_ticks() {
        let mut c = clock();
        c.start_locked_accumulation(key(), true, 0).unwrap();
        let p = c.poll(1, 16_600, ready()).unwrap();
        c.consume(p, 1, ready()).unwrap();
        assert!(c.poll(2, 200_000, ready()).is_none());
        assert_eq!(c.micros_until_due(200_000), Some(16_600));
        let p = c.poll(3, 216_600, ready()).unwrap();
        c.consume(p, 3, ready()).unwrap();
        assert_eq!(c.report().consumed_ticks, 2);
    }
    #[test]
    fn actual_locked_release_before_dab_invalidates_permission_without_a_tail() {
        let mut c = clock();
        c.start_locked_accumulation(key(), true, 0).unwrap();
        let p = c.poll(1, 16_600, ready()).unwrap();
        let mut r = ready();
        r.primary_down = false;
        assert!(c.consume(p, 1, r).is_err());
        assert_eq!(c.report().last_stop, Some(StopReason::Released));
        assert!(!c.report().outstanding_permit);
        assert_eq!(c.report().consumed_ticks, 0);
    }
    #[test]
    fn busy_or_queued_locked_work_resets_period_instead_of_deferring_a_dab() {
        let mut c = clock();
        c.start_locked_accumulation(key(), true, 0).unwrap();
        let mut r = ready();
        r.queue_empty = false;
        assert!(c.poll(1, 16_600, r).is_none());
        assert_eq!(c.micros_until_due(16_600), Some(16_600));
        assert!(c.poll(2, 33_199, ready()).is_none());
        assert!(c.poll(3, 33_200, ready()).is_some());
    }
    #[test]
    fn old_locked_permission_cannot_consume_new_ordinary_gesture() {
        let mut c = clock();
        c.start_locked_accumulation(key(), true, 0).unwrap();
        let p = c.poll(1, 16_600, ready()).unwrap();
        c.stop(StopReason::Command);
        let other = StrokeKey {
            serial: 42,
            ..key()
        };
        c.start(other, true, false, 20_000).unwrap();
        let mut r = ready();
        r.key = Some(other);
        assert!(c.consume(p, 1, r).is_err());
        assert_eq!(c.report().active_key, Some(other));
        assert_eq!(c.report().active_mode, Some(ContinuousMode::Ordinary));
    }
    #[test]
    fn backward_and_overflowed_locked_clocks_stop_without_new_permission() {
        let mut c = clock();
        assert!(c.start_locked_accumulation(key(), true, u64::MAX).is_err());
        assert_eq!(c.report().active_key, None);
        c.start_locked_accumulation(key(), true, 20_000).unwrap();
        assert!(c.poll(1, 19_999, ready()).is_none());
        assert_eq!(c.report().last_stop, Some(StopReason::ClockReversed));
    }
}
