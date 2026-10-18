//! Synchronous continuous work through the actual ordinary DabQueue path.
//! A frame permit never becomes deferred timer work or a second history engine.
use crate::continuous_host::{HostContinuousClock, Readiness, StopReason};
use crate::dab_queue::{DabQueue, Progress};
use crate::footprint::ScreenFootprint;
#[cfg(test)]
use obrush_core::Ray;
use obrush_core::{BrushSettings, Document, Vec3};

pub(crate) struct ContinuousDab {
    pub frame: u64,
    pub now_micros: u64,
    pub ready: Readiness,
    pub ray: obrush_core::SceneRay64,
    pub settings: BrushSettings,
    pub footprint: Option<ScreenFootprint>,
}

#[derive(Default)]
pub(crate) struct ContinuousDabResult {
    pub attempted: bool,
    pub at_micros: Option<u64>,
    pub progress: Progress,
    pub rejected: bool,
}

pub(crate) fn apply_continuous_dab(
    clock: &mut HostContinuousClock,
    queue: &mut DabQueue,
    document: &mut Document,
    mut dab: ContinuousDab,
) -> ContinuousDabResult {
    // Actual queue state wins over any stale caller readiness sample.
    dab.ready.queue_empty &= queue.is_empty();
    let Some(permit) = clock.poll(dab.frame, dab.now_micros, dab.ready) else {
        return ContinuousDabResult::default();
    };
    let mut result = ContinuousDabResult::default();
    if dab.settings.kind != permit.key().kind
        || dab.settings.pressure.to_bits() != dab.ready.pressure.to_bits()
        || !dab.ray.origin.is_finite()
        || !dab.ray.direction.is_finite()
        || dab.ray.direction.length_squared() == 0.0
    {
        clock.stop(StopReason::Command);
        result.rejected = true;
        result.progress.error =
            Some("Continuous dab preparation does not match the captured input".into());
        return result;
    }
    if let Err(error) = clock.consume(permit, dab.frame, dab.ready) {
        result.rejected = true;
        result.progress.error = Some(error);
        clock.stop(StopReason::Command);
        return result;
    }
    result.attempted = true;
    result.at_micros = Some(permit.at_micros());
    queue.dab_scene_with_footprint(dab.ray, dab.settings, Vec3::ZERO, dab.footprint);
    result.progress = queue.process(document, 1, None);
    if result.progress.error.is_some() {
        result.rejected = true;
        clock.stop(StopReason::Command);
    }
    // Queue was empty and exactly one atomic dab was enqueued and consumed.
    // No timer endpoint, catch-up loop or future timer tail is created.
    debug_assert!(queue.is_empty());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous_host::{ClockPolicy, StrokeKey};
    use obrush_core::{BrushKind, MaskConvention, Mesh};

    fn key(kind: BrushKind) -> StrokeKey {
        StrokeKey {
            serial: 7,
            subtool_id: 1,
            kind,
        }
    }
    fn settings(kind: BrushKind) -> BrushSettings {
        BrushSettings {
            kind,
            radius: 0.6,
            symmetry_x: false,
            adaptive_detail: false,
            draw_accumulate: true,
            ..Default::default()
        }
    }
    fn input(frame: u64, now: u64, kind: BrushKind) -> ContinuousDab {
        ContinuousDab {
            frame,
            now_micros: now,
            ready: Readiness {
                key: Some(key(kind)),
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
            },
            ray: Ray {
                origin: Vec3::new(0.1, 0.0, 3.0),
                direction: -Vec3::Z,
            }
            .into(),
            settings: settings(kind),
            footprint: None,
        }
    }
    fn setup(kind: BrushKind) -> (HostContinuousClock, DabQueue, Document) {
        let mut clock = HostContinuousClock::new(ClockPolicy::default()).unwrap();
        clock.start(key(kind), true, false, 0).unwrap();
        let mut queue = DabQueue::default();
        queue.begin();
        let mut d = Document::new(Mesh::sphere(2));
        queue.process(&mut d, 0, None);
        (clock, queue, d)
    }

    #[test]
    fn actual_queue_stationary_ticks_commit_one_whole_undo_without_future_timer_work() {
        let (mut c, mut q, mut d) = setup(BrushKind::Draw);
        let before = d.to_json().unwrap();
        assert!(
            !apply_continuous_dab(&mut c, &mut q, &mut d, input(0, 0, BrushKind::Draw)).attempted
        );
        assert_eq!(d.to_json().unwrap(), before);
        for (frame, now) in [(1, 16_600), (2, 33_200), (3, 49_800)] {
            let r =
                apply_continuous_dab(&mut c, &mut q, &mut d, input(frame, now, BrushKind::Draw));
            assert!(r.attempted && r.progress.changed && !r.rejected);
            assert_eq!(r.progress.dabs, 1);
            assert!(q.is_empty());
        }
        c.stop(StopReason::Released);
        q.release(&mut d);
        let after = d.to_json().unwrap();
        assert_ne!(before, after);
        for frame in 4..40 {
            assert!(
                !apply_continuous_dab(
                    &mut c,
                    &mut q,
                    &mut d,
                    input(frame, frame * 16_600, BrushKind::Draw)
                )
                .attempted
            );
        }
        assert_eq!(d.to_json().unwrap(), after);
        assert!(q.is_empty());
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }

    #[test]
    fn a_stale_empty_flag_never_appends_timer_work_to_the_actual_busy_queue() {
        let (mut c, mut q, mut d) = setup(BrushKind::Draw);
        let dab = input(1, 16_600, BrushKind::Draw);
        q.dab_scene_with_footprint(dab.ray, dab.settings.clone(), Vec3::ZERO, None);
        let before = q.metrics();
        assert!(!apply_continuous_dab(&mut c, &mut q, &mut d, dab).attempted);
        assert_eq!(q.metrics()["pending_dabs"], before["pending_dabs"]);
        q.process(&mut d, 1, None);
        assert!(q.is_empty());
        assert!(
            !apply_continuous_dab(&mut c, &mut q, &mut d, input(2, 33_199, BrushKind::Draw))
                .attempted
        );
        assert!(
            apply_continuous_dab(&mut c, &mut q, &mut d, input(3, 33_200, BrushKind::Draw))
                .attempted
        );
        assert!(q.is_empty());
    }

    #[test]
    fn ten_second_pause_skips_missed_timer_work_and_waits_a_whole_period() {
        let (mut c, mut q, mut d) = setup(BrushKind::Draw);
        assert!(
            apply_continuous_dab(&mut c, &mut q, &mut d, input(1, 16_600, BrushKind::Draw))
                .attempted
        );
        let before = d.to_json().unwrap();
        assert!(
            !apply_continuous_dab(
                &mut c,
                &mut q,
                &mut d,
                input(2, 10_000_000, BrushKind::Draw)
            )
            .attempted
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(q.is_empty());
        assert!(
            !apply_continuous_dab(
                &mut c,
                &mut q,
                &mut d,
                input(3, 10_016_599, BrushKind::Draw)
            )
            .attempted
        );
        assert!(
            apply_continuous_dab(
                &mut c,
                &mut q,
                &mut d,
                input(4, 10_016_600, BrushKind::Draw)
            )
            .attempted
        );
        assert_eq!(c.report().consumed_ticks, 2);
        assert!(q.is_empty());
    }

    #[test]
    fn release_focus_navigation_contact_and_target_stop_before_actual_queue_application() {
        for scenario in 0..7 {
            let (mut c, mut q, mut d) = setup(BrushKind::Draw);
            let before = d.to_json().unwrap();
            let mut dab = input(1, 16_600, BrushKind::Draw);
            match scenario {
                0 => dab.ready.primary_down = false,
                1 => dab.ready.focused = false,
                2 => dab.ready.navigating = true,
                3 => {
                    dab.ready.pressure = 0.0;
                    dab.settings.pressure = 0.0
                }
                4 => dab.ready.surface_hit = false,
                5 => dab.ready.key = None,
                6 => dab.ready.mode_enabled = false,
                _ => unreachable!(),
            }
            let r = apply_continuous_dab(&mut c, &mut q, &mut d, dab);
            assert!(!r.attempted);
            assert!(q.is_empty());
            assert_eq!(d.to_json().unwrap(), before);
            assert!(
                !apply_continuous_dab(&mut c, &mut q, &mut d, input(2, 33_200, BrushKind::Draw))
                    .attempted
            );
        }
    }

    #[test]
    fn invalid_ray_or_mismatched_captured_fields_reject_atomically_and_do_not_retry() {
        for scenario in 0..4 {
            let (mut c, mut q, mut d) = setup(BrushKind::Draw);
            let before = d.to_json().unwrap();
            let mut dab = input(1, 16_600, BrushKind::Draw);
            match scenario {
                0 => dab.ray.origin.z = f64::NAN,
                1 => dab.ray.direction = Vec3::ZERO.as_dvec3(),
                2 => dab.settings.kind = BrushKind::Paint,
                3 => dab.settings.pressure = 0.5,
                _ => unreachable!(),
            }
            let r = apply_continuous_dab(&mut c, &mut q, &mut d, dab);
            assert!(r.rejected && r.progress.error.is_some());
            assert!(!r.attempted);
            assert!(q.is_empty());
            assert_eq!(d.to_json().unwrap(), before);
            assert!(
                !apply_continuous_dab(&mut c, &mut q, &mut d, input(2, 33_200, BrushKind::Draw))
                    .attempted
            );
            assert_eq!(c.report().consumed_ticks, 0);
        }
    }

    #[test]
    fn timer_noops_keep_redo_and_do_not_materialize_masked_or_disabled_channels() {
        for disabled in [true, false] {
            let mut d = Document::new(Mesh::sphere(2));
            d.begin_stroke();
            d.require_active_mesh_mut().unwrap().masks[0] = 0.5;
            d.end_stroke();
            assert!(d.undo());
            if !disabled {
                d.require_active_mesh_mut().unwrap().masks.fill(1.0);
            }
            let before = d.to_json().unwrap();
            let mut q = DabQueue::default();
            q.begin();
            q.process(&mut d, 0, None);
            let mut c = HostContinuousClock::new(ClockPolicy::default()).unwrap();
            c.start(key(BrushKind::Paint), true, false, 0).unwrap();
            let mut dab = input(1, 16_600, BrushKind::Paint);
            dab.settings.paint_rgb = false;
            dab.settings.paint_roughness = !disabled;
            dab.settings.paint_metallic = !disabled;
            let r = apply_continuous_dab(&mut c, &mut q, &mut d, dab);
            assert!(r.attempted && !r.progress.changed && !r.progress.painted);
            assert!(q.is_empty());
            c.stop(StopReason::Released);
            q.release(&mut d);
            assert_eq!(d.to_json().unwrap(), before);
            assert!(d.can_redo());
            assert!(d.require_active_mesh().unwrap().materials.is_none());
        }
    }

    #[test]
    fn actual_material_timer_path_seeds_canonical_retained_levels_and_keeps_rgb_history_exact() {
        let mut m = Mesh::sphere(2);
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.0);
        let mut d = Document::new(m);
        d.add_subdivision_level().unwrap();
        let original = d.to_json().unwrap();
        let rgb = d.require_active_mesh().unwrap().colors.clone();
        let mut c = HostContinuousClock::new(ClockPolicy::default()).unwrap();
        c.start(key(BrushKind::Paint), true, false, 0).unwrap();
        let mut q = DabQueue::default();
        q.begin();
        q.process(&mut d, 0, None);
        for (frame, now) in [(1, 16_600), (2, 33_200)] {
            let mut dab = input(frame, now, BrushKind::Paint);
            dab.settings.paint_rgb = false;
            dab.settings.paint_roughness = true;
            dab.settings.roughness = 0.8;
            let r = apply_continuous_dab(&mut c, &mut q, &mut d, dab);
            assert!(r.progress.changed && !r.progress.painted);
            assert!(q.is_empty());
        }
        c.stop(StopReason::Released);
        q.release(&mut d);
        assert!(d.take_layer_error().is_none());
        let after = d.to_json().unwrap();
        assert_eq!(d.require_active_mesh().unwrap().colors, rgb);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), original);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
        d.select_level(0).unwrap();
        assert!(d.require_active_mesh().unwrap().materials.is_some());
    }
}
