use crate::continuous_host::*;
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};

fn key() -> StrokeKey {
    StrokeKey {
        serial: 7,
        subtool_id: 1,
        kind: BrushKind::Draw,
    }
}
fn ready(key: StrokeKey) -> Readiness {
    Readiness {
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
    }
}
fn clock() -> HostContinuousClock {
    HostContinuousClock::new(ClockPolicy::default()).unwrap()
}
fn started() -> HostContinuousClock {
    let mut c = clock();
    c.start(key(), true, false, 0).unwrap();
    c
}

#[test]
fn global_mode_and_source_tool_capability_do_not_invent_locked_support() {
    let expected = [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Smooth,
        BrushKind::Flatten,
        BrushKind::Crease,
        BrushKind::Pinch,
        BrushKind::Mask,
        BrushKind::Paint,
    ];
    for kind in BrushKind::ALL {
        assert_eq!(supports_continuous(kind, false), expected.contains(&kind));
        assert!(!supports_continuous(kind, true));
        let mut c = clock();
        let k = StrokeKey { kind, ..key() };
        assert_eq!(c.start(k, true, false, 0).is_ok(), expected.contains(&kind));
        c.stop(StopReason::Command);
        assert!(c.start(k, true, true, 0).is_err());
        assert_eq!(c.report().active_key, None);
    }
    let mut c = clock();
    assert!(c.start(key(), false, false, 0).is_err());
    assert_eq!(c.report().active_key, None);
}

#[test]
fn stationary_frames_emit_only_declared_due_times_and_same_frame_cannot_repeat() {
    let mut c = started();
    let r = ready(key());
    let mut times = Vec::new();
    for (frame, now) in [0, 10_000, 16_600, 17_000, 33_200, 49_800]
        .into_iter()
        .enumerate()
    {
        if let Some(p) = c.poll(frame as u64, now, r) {
            assert!(c.poll(frame as u64, now, r).is_none());
            c.consume(p, frame as u64, r).unwrap();
            times.push(p.at_micros());
            assert!(c.consume(p, frame as u64, r).is_err());
        }
    }
    assert_eq!(times, [16_600, 33_200, 49_800]);
    assert_eq!(c.report().consumed_ticks, 3);
    assert!(!c.report().outstanding_permit);
}

#[test]
fn a_long_pause_never_catches_up_missed_ticks() {
    let mut c = started();
    let r = ready(key());
    assert!(c.poll(1, 10_000_000, r).is_none());
    assert!(c.poll(2, 10_016_599, r).is_none());
    let p = c.poll(3, 10_016_600, r).unwrap();
    c.consume(p, 3, r).unwrap();
    assert_eq!(c.report().consumed_ticks, 1);
    assert_eq!(c.report().skipped_polls, 1);
}

#[test]
fn any_pending_work_skips_a_full_period_and_never_holds_a_timer_permit() {
    let mut c = started();
    let mut r = ready(key());
    r.queue_empty = false;
    assert!(c.poll(1, 16_600, r).is_none());
    assert!(!c.report().outstanding_permit);
    r.queue_empty = true;
    assert!(c.poll(2, 33_199, r).is_none());
    let p = c.poll(3, 33_200, r).unwrap();
    c.consume(p, 3, r).unwrap();
    r.operation_busy = true;
    assert!(c.poll(4, 49_800, r).is_none());
    assert_eq!(c.report().consumed_ticks, 1);
    assert_eq!(c.report().skipped_polls, 2);
}

#[test]
fn an_unconsumed_permit_expires_on_next_frame_and_requires_a_fresh_full_period() {
    let mut c = started();
    let r = ready(key());
    let p = c.poll(1, 16_600, r).unwrap();
    assert!(c.poll(2, 33_200, r).is_none());
    assert!(c.consume(p, 2, r).is_err());
    assert_eq!(c.report().expired_permits, 1);
    let fresh = c.poll(3, 49_800, r).unwrap();
    c.consume(fresh, 3, r).unwrap();
    assert_eq!(c.report().consumed_ticks, 1);
}

#[test]
fn a_delayed_current_permit_is_rejected_without_any_future_timer_tail() {
    let mut c = started();
    let r = ready(key());
    let p = c.poll(1, 16_600, r).unwrap();
    assert!(c.consume(p, 2, r).is_err());
    assert_eq!(c.report().last_stop, Some(StopReason::ExpiredPermit));
    assert_eq!(c.report().consumed_ticks, 0);
    for frame in 3..100 {
        assert!(c.poll(frame, frame * 16_600, r).is_none());
    }
}

#[test]
fn release_contact_focus_navigation_visibility_and_mode_interrupt_before_consumption() {
    let base = ready(key());
    let cases = [
        (
            Readiness {
                primary_down: false,
                ..base
            },
            StopReason::Released,
        ),
        (
            Readiness {
                pressure: 0.0,
                ..base
            },
            StopReason::NoContact,
        ),
        (
            Readiness {
                pressure: f32::NAN,
                ..base
            },
            StopReason::InvalidContact,
        ),
        (
            Readiness {
                pressure: 1.25,
                ..base
            },
            StopReason::InvalidContact,
        ),
        (
            Readiness {
                focused: false,
                ..base
            },
            StopReason::FocusLost,
        ),
        (
            Readiness {
                navigating: true,
                ..base
            },
            StopReason::Navigation,
        ),
        (
            Readiness {
                target_visible: false,
                ..base
            },
            StopReason::TargetLost,
        ),
        (
            Readiness {
                pointer_in_viewport: false,
                ..base
            },
            StopReason::TargetLost,
        ),
        (
            Readiness {
                surface_hit: false,
                ..base
            },
            StopReason::TargetLost,
        ),
        (
            Readiness {
                mode_enabled: false,
                ..base
            },
            StopReason::ModeChanged,
        ),
        (Readiness { key: None, ..base }, StopReason::StrokeChanged),
    ];
    for (r, reason) in cases {
        let mut c = started();
        let p = c.poll(1, 16_600, base).unwrap();
        assert!(c.consume(p, 1, r).is_err());
        assert_eq!(c.report().last_stop, Some(reason));
        assert_eq!(c.report().consumed_ticks, 0);
        assert!(c.poll(2, 33_200, base).is_none());
    }
}

#[test]
fn interruption_is_checked_even_on_a_second_poll_of_the_same_frame() {
    let mut c = started();
    let r = ready(key());
    let p = c.poll(1, 16_600, r).unwrap();
    assert!(
        c.poll(
            1,
            16_600,
            Readiness {
                navigating: true,
                ..r
            }
        )
        .is_none()
    );
    assert!(c.consume(p, 1, r).is_err());
    assert_eq!(c.report().last_stop, Some(StopReason::Navigation));
}

#[test]
fn readiness_changed_after_poll_never_adds_work_to_an_already_busy_queue() {
    let mut c = started();
    let r = ready(key());
    let p = c.poll(1, 16_600, r).unwrap();
    assert!(
        c.consume(
            p,
            1,
            Readiness {
                queue_empty: false,
                ..r
            }
        )
        .is_err()
    );
    assert!(!c.report().outstanding_permit);
    assert_eq!(c.report().consumed_ticks, 0);
    assert_eq!(c.report().skipped_polls, 1);
}

#[test]
fn stale_old_stroke_permits_do_not_cancel_a_newer_current_stroke() {
    let mut c = started();
    let r = ready(key());
    let old = c.poll(1, 16_600, r).unwrap();
    c.stop(StopReason::Command);
    let new_key = StrokeKey {
        serial: 8,
        subtool_id: 2,
        kind: BrushKind::Paint,
    };
    c.start(new_key, true, false, 20_000).unwrap();
    let new_ready = ready(new_key);
    let fresh = c.poll(2, 36_600, new_ready).unwrap();
    let before = c.report();
    assert!(c.consume(old, 2, new_ready).is_err());
    assert_eq!(c.report(), before);
    assert_eq!(c.consume(fresh, 2, new_ready).unwrap(), new_key);
}

#[test]
fn clock_frame_overflow_and_invalid_policy_never_emit_a_permit() {
    for policy in [
        ClockPolicy {
            period_micros: 0,
            pause_gap_micros: 100_000,
        },
        ClockPolicy {
            period_micros: 100_001,
            pause_gap_micros: 100_001,
        },
        ClockPolicy {
            period_micros: 16_600,
            pause_gap_micros: 16_599,
        },
        ClockPolicy {
            period_micros: 16_600,
            pause_gap_micros: 1_000_001,
        },
    ] {
        assert!(HostContinuousClock::new(policy).is_err());
    }
    let mut c = clock();
    assert!(c.start(key(), true, false, u64::MAX).is_err());
    assert_eq!(c.report().active_key, None);
    let r = ready(key());
    c.start(key(), true, false, 0).unwrap();
    assert!(c.poll(1, 0, r).is_none());
    assert!(c.poll(2, u64::MAX, r).is_none());
    assert_eq!(c.report().last_stop, Some(StopReason::ClockOverflow));
    c.start(key(), true, false, 100).unwrap();
    assert!(c.poll(3, 99, r).is_none());
    assert_eq!(c.report().last_stop, Some(StopReason::ClockReversed));
    c.start(key(), true, false, 0).unwrap();
    assert!(c.poll(10, 1, r).is_none());
    assert!(c.poll(9, 2, r).is_none());
    assert_eq!(c.report().last_stop, Some(StopReason::FrameReversed));
}

#[test]
fn consumed_ticks_drive_one_real_stroke_and_release_keeps_redo_without_later_edits() {
    let mut d = Document::new(Mesh::sphere(2));
    let before = d.to_json().unwrap();
    let settings = BrushSettings {
        radius: 0.6,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    };
    let ray = Ray {
        origin: Vec3::new(0.0, 0.0, 4.0),
        direction: -Vec3::Z,
    };
    let mut c = started();
    let r = ready(key());
    d.begin_stroke();
    for (frame, now) in [(1, 16_600), (2, 33_200), (3, 49_800)] {
        let p = c.poll(frame, now, r).unwrap();
        c.consume(p, frame, r).unwrap();
        let hit = d.require_active_mesh().unwrap().raycast(ray).unwrap();
        assert!(
            d.require_active_mesh_mut().unwrap().apply_brush(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings
            ) > 0
        );
    }
    c.stop(StopReason::Released);
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert_ne!(before, after);
    for frame in 4..80 {
        assert!(c.poll(frame, frame * 16_600, r).is_none());
    }
    assert_eq!(d.to_json().unwrap(), after);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.can_redo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn short_released_contact_has_no_tick_history_or_redo_invalidation() {
    let mut d = Document::new(Mesh::sphere(2));
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().masks[0] = 0.5;
    d.end_stroke();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let mut c = started();
    let r = ready(key());
    d.begin_stroke();
    assert!(c.poll(1, 10_000, r).is_none());
    assert!(
        c.poll(
            2,
            12_000,
            Readiness {
                primary_down: false,
                ..r
            }
        )
        .is_none()
    );
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(!d.can_undo());
    assert_eq!(c.report().consumed_ticks, 0);
}

#[test]
fn a_second_start_cannot_replace_an_active_stroke_or_its_outstanding_permit() {
    let mut c = started();
    let r = ready(key());
    let permit = c.poll(1, 16_600, r).unwrap();
    let before = c.report();
    assert!(
        c.start(StrokeKey { serial: 8, ..key() }, true, false, 16_700)
            .is_err()
    );
    assert_eq!(c.report(), before);
    c.consume(permit, 1, r).unwrap();
    assert_eq!(c.report().consumed_ticks, 1);
}

#[test]
fn rejected_atomic_application_stops_new_permissions_and_preserves_complete_history() {
    // An existing checked core operation supplies a real atomic rejection.
    // This does not substitute for the future normal queue integration test.
    for accepted_first in [false, true] {
        let mut d = Document::new(Mesh::sphere(2));
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().masks[0] = 0.5;
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut c = started();
        let r = ready(key());
        d.begin_stroke();
        if accepted_first {
            let permit = c.poll(1, 16_600, r).unwrap();
            c.consume(permit, 1, r).unwrap();
            let ray = Ray {
                origin: Vec3::new(0.0, 0.0, 4.0),
                direction: -Vec3::Z,
            };
            let hit = d.require_active_mesh().unwrap().raycast(ray).unwrap();
            assert!(
                d.require_active_mesh_mut().unwrap().apply_brush(
                    hit.position,
                    hit.normal,
                    Vec3::ZERO,
                    &BrushSettings {
                        radius: 0.6,
                        symmetry_x: false,
                        adaptive_detail: false,
                        ..Default::default()
                    }
                ) > 0
            );
        }
        let permit = c.poll(2, 33_200, r).unwrap();
        c.consume(permit, 2, r).unwrap();
        let immediately_before_rejection = d.to_json().unwrap();
        assert!(
            d.require_active_mesh_mut()
                .unwrap()
                .install_attributes(Some(vec![[0.5, 0.5]]), None)
                .is_err()
        );
        assert_eq!(d.to_json().unwrap(), immediately_before_rejection);
        c.stop(StopReason::Command);
        d.end_stroke();
        let after = d.to_json().unwrap();
        assert!(!c.report().outstanding_permit);
        for frame in 3..100 {
            assert!(c.poll(frame, frame * 16_600, r).is_none());
        }
        assert_eq!(d.to_json().unwrap(), after);
        if accepted_first {
            assert!(!d.can_redo());
            assert!(d.undo());
            assert_eq!(d.to_json().unwrap(), before);
            assert!(!d.can_undo());
            assert!(d.redo());
            assert_eq!(d.to_json().unwrap(), after);
        } else {
            assert_eq!(after, before);
            assert!(d.can_redo());
            assert!(!d.can_undo());
        }
    }
}
