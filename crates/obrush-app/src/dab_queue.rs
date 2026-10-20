//! Recent sculpt input with bounded latency between viewport redraws.
use std::collections::VecDeque;
use std::time::Duration;

use crate::footprint::ScreenFootprint;
#[cfg(test)]
use obrush_core::Ray;
use obrush_core::{BrushKind, BrushSettings, Document, DrawStroke, SceneRay64, Vec3};
use web_time::Instant;

const MAX_PENDING_DABS: usize = 32;
const MAX_INPUT_AGE: Duration = Duration::from_millis(50);

enum Work {
    Begin,
    BeginDrag {
        center: Vec3,
        normal: Vec3,
    },
    Dab {
        ray: SceneRay64,
        settings: Box<BrushSettings>,
        delta: Vec3,
        received: Instant,
        footprint: Option<ScreenFootprint>,
    },
    SourcePaint {
        session: crate::source_paint_host::SharedSourcePaint,
        input: crate::source_paint_host::SourcePaintInput,
        received: Instant,
    },
    SourceGeometry {
        session: crate::source_geometry_host::SharedSourceGeometry,
        input: crate::source_geometry_host::SourceGeometryInput,
        received: Instant,
    },
    End,
}

struct DragPatch {
    center: Vec3,
    normal: Vec3,
    subtool_id: Option<u32>,
}

#[derive(Default)]
pub(crate) struct DabTiming {
    samples: u64,
    total_ns: u64,
    last_ns: u64,
    max_ns: u64,
}

impl DabTiming {
    pub(crate) fn record(&mut self, elapsed: Duration) {
        let ns = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.samples = self.samples.saturating_add(1);
        self.total_ns = self.total_ns.saturating_add(ns);
        self.last_ns = ns;
        self.max_ns = self.max_ns.max(ns);
    }

    pub(crate) fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "samples": self.samples,
            "total_ms": self.total_ns as f64 / 1_000_000.0,
            "last_ms": self.last_ns as f64 / 1_000_000.0,
            "max_ms": self.max_ns as f64 / 1_000_000.0,
            "mean_ms": if self.samples == 0 { 0.0 } else { self.total_ns as f64 / self.samples as f64 / 1_000_000.0 },
        })
    }
}

#[derive(Default)]
pub(crate) struct DabQueue {
    work: VecDeque<Work>,
    pending: usize,
    peak_pending: usize,
    applied: u64,
    processed: u64,
    coalesced: u64,
    releases: u64,
    draw_proxy: Option<DrawStroke>,
    drag: Option<DragPatch>,
    ordinary_timing: DabTiming,
    source_paint_timing: DabTiming,
    source_geometry_timing: DabTiming,
    source_geometry_preparation_timing: DabTiming,
    source_geometry_retained_match_timing: DabTiming,
    source_geometry_contact_prepare_timing: DabTiming,
    source_geometry_deformation_timing: DabTiming,
    source_geometry_other_timing: DabTiming,
    source_selection_timing: DabTiming,
    source_old_copy_timing: DabTiming,
    source_candidate_timing: DabTiming,
    source_advance_timing: DabTiming,
    source_normals_timing: DabTiming,
    source_publish_timing: DabTiming,
    release_timing: DabTiming,
}

#[derive(Default)]
pub(crate) struct Progress {
    pub changed: bool,
    pub painted: bool,
    pub derived_cache_changed: bool,
    pub dabs: usize,
    pub error: Option<String>,
    pub render_snapshot: Option<obrush_core::CertifiedSourceRenderSnapshot>,
}

impl DabQueue {
    pub fn source_geometry_dab(
        &mut self,
        session: crate::source_geometry_host::SharedSourceGeometry,
        input: crate::source_geometry_host::SourceGeometryInput,
    ) {
        if self.pending == MAX_PENDING_DABS
            && let Some(index) = self.work.iter().position(|w| {
                matches!(
                    w,
                    Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. }
                )
            })
        {
            self.work.remove(index);
            self.pending -= 1;
            self.coalesced += 1;
        }
        self.work.push_back(Work::SourceGeometry {
            session,
            input,
            received: Instant::now(),
        });
        self.pending += 1;
        self.peak_pending = self.peak_pending.max(self.pending);
    }
    fn apply_source_geometry(
        &mut self,
        document: &mut Document,
        session: crate::source_geometry_host::SharedSourceGeometry,
        input: crate::source_geometry_host::SourceGeometryInput,
        progress: &mut Progress,
    ) {
        self.pending -= 1;
        self.processed += 1;
        progress.dabs += 1;
        if session.lock().is_ok_and(|s| s.failed.is_some()) {
            self.coalesced += 1;
            return;
        }
        let started = Instant::now();
        let (result, stages, render_snapshot) =
            crate::source_geometry_host::apply_shared_with_timing(document, &session, input);
        let elapsed = started.elapsed();
        self.source_geometry_timing.record(elapsed);
        if let Some(stages) = stages {
            self.source_geometry_preparation_timing
                .record(stages.preparation);
            self.source_geometry_retained_match_timing
                .record(stages.retained_match);
            self.source_geometry_contact_prepare_timing
                .record(stages.contact_prepare);
            self.source_geometry_deformation_timing
                .record(stages.deformation);
            self.source_geometry_other_timing.record(
                elapsed
                    .saturating_sub(stages.preparation)
                    .saturating_sub(stages.deformation),
            );
        }
        match result {
            Ok(r) => {
                if let Some(phase) = r.phase_time {
                    self.source_selection_timing.record(phase.selection);
                    self.source_old_copy_timing.record(phase.old_copy);
                    self.source_candidate_timing
                        .record(phase.candidate_and_guard);
                    self.source_advance_timing.record(phase.state_advance);
                    self.source_normals_timing.record(phase.normal_refresh);
                    self.source_publish_timing.record(phase.publish);
                }
                let changed = r.changed_vertices > 0;
                self.applied += u64::from(changed);
                progress.changed |= changed;
                progress.derived_cache_changed |= r.derived_cache_changed;
                if changed {
                    progress.render_snapshot = render_snapshot;
                }
            }
            Err(e) => {
                progress.error.get_or_insert(e.to_string());
            }
        }
    }
    /// SourceGeometrySession already opened and owns the document transaction.
    /// Drain only the latest endpoint; never emit generic End before its finish.
    pub fn release_source_geometry(&mut self, document: &mut Document) -> Progress {
        self.release_work(document)
    }

    pub fn source_paint_dab(
        &mut self,
        session: crate::source_paint_host::SharedSourcePaint,
        input: crate::source_paint_host::SourcePaintInput,
    ) {
        if self.pending == MAX_PENDING_DABS
            && let Some(index) = self.work.iter().position(|item| {
                matches!(
                    item,
                    Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. }
                )
            })
        {
            self.work.remove(index);
            self.pending -= 1;
            self.coalesced += 1;
        }
        self.work.push_back(Work::SourcePaint {
            session,
            input,
            received: Instant::now(),
        });
        self.pending += 1;
        self.peak_pending = self.peak_pending.max(self.pending);
    }
    fn apply_source_paint(
        &mut self,
        document: &mut Document,
        session: crate::source_paint_host::SharedSourcePaint,
        input: crate::source_paint_host::SourcePaintInput,
        progress: &mut Progress,
    ) {
        // A later paint operation can change the mesh after an earlier
        // geometry proof was captured in the same queue drain.
        progress.render_snapshot = None;
        self.pending -= 1;
        self.processed += 1;
        progress.dabs += 1;
        if session.lock().is_ok_and(|s| s.failed.is_some()) {
            self.coalesced += 1;
            return;
        }
        let started = Instant::now();
        let result = crate::source_paint_host::apply_shared(document, &session, input);
        self.source_paint_timing.record(started.elapsed());
        match result {
            Ok(r) => {
                let changed = r.changed_vertices > 0;
                self.applied += u64::from(changed);
                progress.changed |= changed;
                progress.painted |= r.changed_rgb_vertices > 0;
            }
            Err(e) => {
                progress.error.get_or_insert(e);
            }
        }
    }
    pub fn begin(&mut self) {
        self.work.push_back(Work::Begin);
    }
    pub fn begin_drag(&mut self, center: Vec3, normal: Vec3) {
        self.work.push_back(Work::BeginDrag { center, normal });
    }
    pub fn end(&mut self) {
        self.work.push_back(Work::End);
    }
    #[cfg(test)]
    pub fn dab(&mut self, ray: Ray, settings: BrushSettings, delta: Vec3) {
        self.dab_with_footprint(ray, settings, delta, None);
    }
    #[cfg(test)]
    pub fn dab_with_footprint(
        &mut self,
        ray: Ray,
        settings: BrushSettings,
        delta: Vec3,
        footprint: Option<ScreenFootprint>,
    ) {
        self.dab_scene_with_footprint(ray.into(), settings, delta, footprint);
    }
    pub fn dab_scene_with_footprint(
        &mut self,
        ray: SceneRay64,
        settings: BrushSettings,
        delta: Vec3,
        footprint: Option<ScreenFootprint>,
    ) {
        // Overload must not turn physical movement into seconds of future work.
        // Retain history markers and the most recent movement, coalescing older
        // samples only when the fixed interactive capacity is exhausted.
        if self.pending == MAX_PENDING_DABS
            && let Some(index) = self.work.iter().position(|item| {
                matches!(
                    item,
                    Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. }
                )
            })
        {
            self.work.remove(index);
            self.pending -= 1;
            self.coalesced += 1;
        }
        // The queue retains at most32 captured payloads. Keep history markers
        // compact while owning each unchanged settings snapshot separately.
        self.work.push_back(Work::Dab {
            ray,
            settings: Box::new(settings),
            delta,
            received: Instant::now(),
            footprint,
        });
        self.pending += 1;
        self.peak_pending = self.peak_pending.max(self.pending);
    }
    pub fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "pending_dabs":self.pending,"peak_pending_dabs":self.peak_pending,"maximum_pending_dabs":MAX_PENDING_DABS,
            "maximum_input_age_ms":MAX_INPUT_AGE.as_millis(),
            "processed_dabs":self.processed,"applied_dabs":self.applied,"coalesced_dabs":self.coalesced,"released_strokes":self.releases,
            "oldest_pending_age_ms":self.work.iter().find_map(|work| match work {
                Work::Dab { received, .. } | Work::SourcePaint { received, .. } | Work::SourceGeometry { received, .. } => Some(received.elapsed().as_secs_f64()*1000.0),
                _ => None,
            }).unwrap_or(0.0),
            "cpu_dab_timing":{"ordinary":self.ordinary_timing.metrics(),"source_paint":self.source_paint_timing.metrics(),
                "source_geometry":self.source_geometry_timing.metrics(),
                "source_geometry_preparation":self.source_geometry_preparation_timing.metrics(),
                "source_geometry_retained_match":self.source_geometry_retained_match_timing.metrics(),
                "source_geometry_contact_prepare":self.source_geometry_contact_prepare_timing.metrics(),
                "source_geometry_deformation":self.source_geometry_deformation_timing.metrics(),
                "source_geometry_other":self.source_geometry_other_timing.metrics(),
                "source_selection":self.source_selection_timing.metrics(),
                "source_old_copy":self.source_old_copy_timing.metrics(),
                "source_candidate_guard":self.source_candidate_timing.metrics(),
                "source_state_advance":self.source_advance_timing.metrics(),
                "source_normal_refresh":self.source_normals_timing.metrics(),
                "source_publish":self.source_publish_timing.metrics(),
                "release":self.release_timing.metrics(),
                "scope":"WASM/native wall time inside queued dab apply and release_work; excludes input delivery, paint callback, GPU upload/draw, and browser input-to-photon latency"}
        })
    }

    fn apply_timed(
        &mut self,
        document: &mut Document,
        ray: SceneRay64,
        settings: BrushSettings,
        delta: Vec3,
        footprint: Option<ScreenFootprint>,
        progress: &mut Progress,
    ) {
        progress.render_snapshot = None;
        let started = Instant::now();
        self.apply(document, ray, settings, delta, footprint, progress);
        self.ordinary_timing.record(started.elapsed());
    }
    fn apply(
        &mut self,
        document: &mut Document,
        ray: SceneRay64,
        mut settings: BrushSettings,
        delta: Vec3,
        footprint: Option<ScreenFootprint>,
        progress: &mut Progress,
    ) {
        self.pending -= 1;
        self.processed += 1;
        progress.dabs += 1;
        if settings.kind == BrushKind::Drag {
            if let Some(patch) = self.drag.as_mut()
                && patch.subtool_id == document.active_subtool_id()
                && document.active_subtool_visible()
            {
                // Coalescing drops intermediate intent, not displacement. The
                // latest ray advances from the last *applied* patch center.
                if !ray.origin.is_finite()
                    || !ray.direction.is_finite()
                    || ray.direction.length_squared() == 0.0
                {
                    progress
                        .error
                        .get_or_insert("Invalid Drag picking ray".into());
                    return;
                }
                let direction = ray.direction.normalize();
                let origin = ray.origin;
                let world_center = match crate::scene_space::world_point(document, patch.center) {
                    Ok(v) => v,
                    Err(e) => {
                        progress.error.get_or_insert(e);
                        return;
                    }
                };
                let projected = origin + direction * (world_center - origin).dot(direction);
                let center = match crate::scene_space::local_point(document, projected) {
                    Ok(v) => v,
                    Err(e) => {
                        progress.error.get_or_insert(e);
                        return;
                    }
                };
                if !center.is_finite() || !patch.normal.is_finite() {
                    progress
                        .error
                        .get_or_insert("Invalid captured Drag patch".into());
                    return;
                }
                if let Some(footprint) = footprint {
                    let Some(radius) = footprint.radius_at_world(document, center) else {
                        progress
                            .error
                            .get_or_insert("Invalid screen-space Drag radius".into());
                        return;
                    };
                    settings.radius = radius;
                }
                let mesh = match document.require_active_mesh_mut() {
                    Ok(v) => v,
                    Err(e) => {
                        progress.error.get_or_insert(e);
                        return;
                    }
                };
                let result = mesh.apply_brush_with_result(
                    center,
                    patch.normal,
                    center - patch.center,
                    &settings,
                );
                patch.center = center;
                let changed = result.changed_vertices > 0;
                self.applied += u64::from(changed);
                progress.changed |= changed;
            }
            return;
        }
        let policy = match crate::scene_space::picking_policy(document) {
            Ok(p) => p,
            Err(error) => {
                progress.error.get_or_insert(error);
                return;
            }
        };
        let picked = match document.raycast_scene(ray, policy) {
            Ok(v) => v,
            Err(e) => {
                progress.error.get_or_insert(e);
                return;
            }
        };
        if let Some(scene_hit) = picked
            && Some(scene_hit.id) == document.active_subtool_id()
            && document.active_subtool_visible()
        {
            let (hit, _rounding) = match scene_hit.local_brush_hit() {
                Ok(v) => v,
                Err(e) => {
                    progress.error.get_or_insert(e);
                    return;
                }
            };
            if let Some(footprint) = footprint {
                let Some(radius) = footprint.radius_at_world(document, hit.position) else {
                    progress.error.get_or_insert(
                        "Invalid screen-space brush radius at the picked surface".into(),
                    );
                    return;
                };
                settings.radius = radius;
            }
            if settings.kind == BrushKind::Paint
                && (settings.paint_roughness || settings.paint_metallic)
            {
                document.begin_material_paint_stroke();
            }
            let changed = if settings.kind == BrushKind::Morph {
                match document.apply_morph_brush_with_result(hit.position, hit.normal, &settings) {
                    Ok(result) => result.changed_vertices > 0,
                    Err(error) => {
                        progress.error.get_or_insert(error);
                        false
                    }
                }
            } else if settings.kind == BrushKind::Draw && !settings.draw_accumulate {
                if self.draw_proxy.is_none() {
                    match document
                        .require_active_mesh()
                        .and_then(|mesh| mesh.prepare_draw_stroke(&settings))
                    {
                        Ok(stroke) => self.draw_proxy = Some(stroke),
                        Err(error) => {
                            progress.error.get_or_insert(error);
                            return;
                        }
                    }
                }
                let mesh = match document.require_active_mesh_mut() {
                    Ok(v) => v,
                    Err(e) => {
                        progress.error.get_or_insert(e);
                        return;
                    }
                };
                self.draw_proxy
                    .as_mut()
                    .expect("proxy prepared above")
                    .apply(mesh, hit.position, hit.normal, &settings)
                    .changed_vertices
                    > 0
            } else {
                let mesh = match document.require_active_mesh_mut() {
                    Ok(v) => v,
                    Err(e) => {
                        progress.error.get_or_insert(e);
                        return;
                    }
                };
                mesh.apply_brush(hit.position, hit.normal, delta, &settings) > 0
            };
            self.applied += u64::from(changed);
            progress.changed |= changed;
            progress.painted |= changed && settings.kind == BrushKind::Paint && settings.paint_rgb;
        }
    }

    /// Mouse release commits the visible stroke and at most its final endpoint.
    /// No obsolete geometry edits remain to run after the hand has stopped.
    pub fn release(&mut self, document: &mut Document) -> Progress {
        self.end();
        self.release_work(document)
    }
    /// This Source Lock already owns Begin. Drain only its last endpoint, then
    /// run the owned fullbinding/release gate before any generic end transaction.
    pub fn release_source_paint_locked(
        &mut self,
        document: &mut Document,
        session: &crate::source_paint_host::SharedSourcePaint,
    ) -> Progress {
        let mut progress = self.release_work(document);
        let ended = session
            .lock()
            .map_err(|_| "Source Paint Lock release mutex failed".to_owned())
            .and_then(|mut s| s.finish_locked(document));
        if let Err(error) = ended {
            progress.error.get_or_insert(error);
        }
        progress
    }

    fn release_work(&mut self, document: &mut Document) -> Progress {
        let release_started = Instant::now();
        let last = self.work.iter().rposition(|item| {
            matches!(
                item,
                Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. }
            )
        });
        let mut progress = Progress::default();
        let mut index = 0;
        while let Some(work) = self.work.pop_front() {
            match work {
                Work::Begin => {
                    self.draw_proxy = None;
                    self.drag = None;
                    document.begin_stroke();
                }
                Work::BeginDrag { center, normal } => {
                    self.draw_proxy = None;
                    self.drag = Some(DragPatch {
                        center,
                        normal,
                        subtool_id: document.active_subtool_id(),
                    });
                    document.begin_stroke();
                }
                Work::End => {
                    document.end_stroke();
                    self.draw_proxy = None;
                    self.drag = None;
                }
                Work::Dab {
                    ray,
                    settings,
                    delta,
                    footprint,
                    ..
                } if Some(index) == last => {
                    self.apply_timed(document, ray, *settings, delta, footprint, &mut progress);
                }
                Work::SourcePaint { session, input, .. } if Some(index) == last => {
                    self.apply_source_paint(document, session, input, &mut progress);
                }
                Work::SourceGeometry { session, input, .. } if Some(index) == last => {
                    self.apply_source_geometry(document, session, input, &mut progress);
                }
                Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. } => {
                    self.pending -= 1;
                    self.coalesced += 1;
                }
            }
            index += 1;
        }
        self.releases += 1;
        self.release_timing.record(release_started.elapsed());
        progress
    }
    pub fn is_empty(&self) -> bool {
        self.work.is_empty()
    }

    /// The document owner rolls its open snapshot back separately. Drop every
    /// pending sample and marker from the cancelled active gesture first.
    pub fn discard_active_stroke(&mut self) {
        self.coalesced = self.coalesced.saturating_add(self.pending as u64);
        self.work.clear();
        self.pending = 0;
        self.draw_proxy = None;
        self.drag = None;
    }

    /// A dab is atomic; the budget is checked between dabs. History markers
    /// remain ordered even when a mouse release precedes completion of its work.
    pub fn process(
        &mut self,
        document: &mut Document,
        maximum: usize,
        budget: Option<Duration>,
    ) -> Progress {
        let start = Instant::now();
        let mut progress = Progress::default();
        while let Some(next) = self.work.front() {
            if budget.is_some()
                && self.pending > 1
                && matches!(next, Work::Dab { received, .. } | Work::SourcePaint { received, .. } | Work::SourceGeometry { received, .. } if received.elapsed() > MAX_INPUT_AGE)
            {
                self.work.pop_front();
                self.pending -= 1;
                self.coalesced += 1;
                continue;
            }
            if matches!(
                next,
                Work::Dab { .. } | Work::SourcePaint { .. } | Work::SourceGeometry { .. }
            ) && (progress.dabs >= maximum
                || (progress.dabs > 0 && budget.is_some_and(|limit| start.elapsed() >= limit)))
            {
                break;
            }
            match self.work.pop_front().expect("front exists") {
                Work::Begin => {
                    self.draw_proxy = None;
                    self.drag = None;
                    document.begin_stroke();
                }
                Work::BeginDrag { center, normal } => {
                    self.draw_proxy = None;
                    self.drag = Some(DragPatch {
                        center,
                        normal,
                        subtool_id: document.active_subtool_id(),
                    });
                    document.begin_stroke();
                }
                Work::End => {
                    document.end_stroke();
                    self.draw_proxy = None;
                    self.drag = None;
                }
                Work::SourcePaint { session, input, .. } => {
                    self.apply_source_paint(document, session, input, &mut progress);
                }
                Work::SourceGeometry { session, input, .. } => {
                    self.apply_source_geometry(document, session, input, &mut progress)
                }
                Work::Dab {
                    ray,
                    settings,
                    delta,
                    footprint,
                    ..
                } => {
                    self.apply_timed(document, ray, *settings, delta, footprint, &mut progress);
                }
            }
        }
        progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::Mesh;

    #[test]
    fn discard_clears_pending_samples_without_committing_an_open_stroke() {
        let mut document = Document::new(Mesh::sphere(1));
        let before = document.to_json().unwrap();
        let history = document.history_depths();
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings::default(),
            Vec3::ZERO,
        );
        let applied = queue.process(&mut document, 1, None);
        assert!(applied.changed);
        queue.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings::default(),
            Vec3::ZERO,
        );
        queue.discard_active_stroke();
        assert!(queue.is_empty());
        assert_eq!(queue.metrics()["pending_dabs"], 0);
        assert!(document.cancel_stroke());
        assert_eq!(document.to_json().unwrap(), before);
        assert_eq!(document.history_depths(), history);
    }

    #[test]
    fn timing_observes_processed_dabs_and_release_without_changing_history() {
        let mut document = Document::new(Mesh::sphere(2));
        let before = document.to_json().unwrap();
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings {
                kind: BrushKind::Draw,
                symmetry_x: false,
                adaptive_detail: false,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let progress = queue.process(&mut document, 1, None);
        assert!(progress.changed && progress.error.is_none());
        let released = queue.release(&mut document);
        assert!(released.error.is_none());
        let metrics = queue.metrics();
        assert_eq!(metrics["cpu_dab_timing"]["ordinary"]["samples"], 1);
        assert_eq!(metrics["cpu_dab_timing"]["release"]["samples"], 1);
        assert_eq!(metrics["pending_dabs"], 0);
        let after = document.to_json().unwrap();
        assert_ne!(before, after);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }

    #[test]
    fn material_only_queue_seeds_old_levels_and_keeps_rgb_display_unchanged() {
        let mut document = Document::new(Mesh::sphere(2));
        document.add_subdivision_level().unwrap();
        let before = document.to_json().unwrap();
        let colors = document.require_active_mesh().unwrap().colors.clone();
        let settings = BrushSettings {
            kind: BrushKind::Paint,
            radius: 0.5,
            strength: 0.6,
            symmetry_x: false,
            adaptive_detail: false,
            paint_rgb: false,
            paint_roughness: true,
            roughness: 0.8,
            ..Default::default()
        };
        let ray = Ray {
            origin: Vec3::new(0.1, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(ray, settings.clone(), Vec3::ZERO);
        let progress = queue.process(&mut document, 1, None);
        assert!(progress.changed && !progress.painted && progress.error.is_none());
        // The normal render path can show a held first-material stroke without
        // prematurely reconciling its old implicit hierarchy.
        let rendered = document.subtool_render_snapshots();
        assert_eq!(rendered.len(), 1);
        assert_eq!(
            rendered[0].mesh.materials,
            document.require_active_mesh().unwrap().materials
        );
        assert!(document.require_active_mesh().unwrap().materials.is_some());
        let release = queue.release(&mut document);
        assert!(!release.painted && queue.is_empty());
        assert_eq!(document.require_active_mesh().unwrap().colors, colors);
        let painted = document.to_json().unwrap();
        assert_ne!(painted, before);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), painted);
    }

    #[test]
    fn disabled_material_queue_preserves_noop_redo_and_absent_channels() {
        let mut document = Document::new(Mesh::sphere(2));
        document.begin_stroke();
        document.require_active_mesh_mut().unwrap().masks[0] = 0.5;
        document.end_stroke();
        assert!(document.undo());
        let before = document.to_json().unwrap();
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            Ray {
                origin: Vec3::new(0.1, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings {
                kind: BrushKind::Paint,
                paint_rgb: false,
                paint_roughness: false,
                paint_metallic: false,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let progress = queue.release(&mut document);
        assert!(!progress.changed && !progress.painted && progress.error.is_none());
        assert!(document.require_active_mesh().unwrap().materials.is_none() && document.can_redo());
        assert_eq!(document.to_json().unwrap(), before);
    }

    #[test]
    fn queued_material_targets_are_captured_and_rgb_flag_controls_only_display() {
        let mut document = Document::new(Mesh::sphere(2));
        let original_rgb = document.require_active_mesh().unwrap().colors.clone();
        let ray = Ray {
            origin: Vec3::new(0.1, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        let mut settings = BrushSettings {
            kind: BrushKind::Paint,
            radius: 0.5,
            strength: 1.0,
            symmetry_x: false,
            adaptive_detail: false,
            paint_rgb: false,
            paint_metallic: true,
            metallic: 0.9,
            ..Default::default()
        };
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(ray, settings.clone(), Vec3::ZERO);
        settings.metallic = 0.0;
        assert!(!queue.release(&mut document).painted);
        assert_eq!(document.require_active_mesh().unwrap().colors, original_rgb);
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .materials
                .as_ref()
                .unwrap()
                .iter()
                .any(|m| m[1] > 0.08)
        );
        let material = document.require_active_mesh().unwrap().materials.clone();
        settings.paint_metallic = false;
        settings.paint_rgb = true;
        settings.color = [0.0, 1.0, 0.0];
        queue.begin();
        queue.dab(ray, settings, Vec3::ZERO);
        assert!(queue.release(&mut document).painted);
        assert_eq!(document.require_active_mesh().unwrap().materials, material);
    }

    #[test]
    fn overloaded_drag_keeps_total_latest_intent_and_has_no_post_release_work() {
        let mut document = Document::new(Mesh::sphere(4));
        let original = document.to_json().unwrap();
        let start = Ray {
            origin: Vec3::new(0.2, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        let hit = document
            .require_active_mesh()
            .unwrap()
            .raycast(start)
            .unwrap();
        let settings = BrushSettings {
            kind: BrushKind::Drag,
            radius: 0.45,
            strength: 0.25,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        };
        let mut oracle = document.require_active_mesh().unwrap().snapshot();
        let center = Vec3::new(0.35, hit.position.y, hit.position.z);
        let expected =
            oracle.apply_brush_with_result(center, hit.normal, center - hit.position, &settings);
        assert!(expected.changed_vertices > 0);
        let mut queue = DabQueue::default();
        queue.begin_drag(hit.position, hit.normal);
        for step in 1..=100 {
            let x = if step == 100 {
                0.35
            } else {
                0.2 + 0.0015 * step as f32
            };
            queue.dab(
                Ray {
                    origin: Vec3::new(x, 0.0, 3.0),
                    direction: -Vec3::Z,
                },
                settings.clone(),
                Vec3::ZERO,
            );
        }
        assert_eq!(queue.pending, MAX_PENDING_DABS);
        let result = queue.release(&mut document);
        assert!(result.changed && result.error.is_none());
        assert_eq!(result.dabs, 1);
        assert_eq!(
            document.require_active_mesh().unwrap().positions,
            oracle.positions
        );
        assert_eq!(
            document.require_active_mesh().unwrap().normals,
            oracle.normals
        );
        let drawn = document.to_json().unwrap();
        assert!(queue.is_empty() && queue.drag.is_none());
        assert!(!queue.process(&mut document, usize::MAX, None).changed);
        assert_eq!(document.to_json().unwrap(), drawn);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), drawn);
    }

    #[test]
    fn drag_press_without_motion_and_invalid_intent_preserve_redo() {
        let mut document = Document::new(Mesh::sphere(3));
        document.begin_stroke();
        document.require_active_mesh_mut().unwrap().masks[0] = 0.5;
        document.end_stroke();
        assert!(document.undo());
        let original = document.to_json().unwrap();
        let hit = document
            .require_active_mesh()
            .unwrap()
            .raycast(Ray {
                origin: Vec3::new(0.2, 0.0, 3.0),
                direction: -Vec3::Z,
            })
            .unwrap();
        let mut queue = DabQueue::default();
        queue.begin_drag(hit.position, hit.normal);
        assert!(!queue.release(&mut document).changed);
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.can_redo());
        queue.begin_drag(hit.position, hit.normal);
        queue.dab(
            Ray {
                origin: Vec3::ZERO,
                direction: Vec3::ZERO,
            },
            BrushSettings {
                kind: BrushKind::Drag,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let result = queue.release(&mut document);
        assert!(result.error.is_some() && !result.changed && queue.is_empty());
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.can_redo());
    }

    #[test]
    fn queued_screen_radius_resolves_on_current_surface_and_preserves_one_undo() {
        for orthographic in [false, true] {
            let camera = crate::renderer::Camera {
                orthographic,
                ..Default::default()
            };
            let footprint = ScreenFootprint::capture(&camera, 75.0, 1000.0, 1.6).unwrap();
            let ray = camera.ray(glam::Vec2::ZERO, 1.6);
            let settings = BrushSettings {
                radius: 0.999, // Must be overridden at each real picked depth.
                strength: 0.35,
                symmetry_x: false,
                adaptive_detail: false,
                ..Default::default()
            };
            let mut document = Document::new(Mesh::sphere(4));
            let original = document.to_json().unwrap();
            let mut oracle = document.require_active_mesh().unwrap().snapshot();
            let mut radii = Vec::new();
            let mut queue = DabQueue::default();
            queue.begin();
            for _ in 0..12 {
                let hit = oracle.raycast(ray).unwrap();
                let mut sample = settings.clone();
                sample.radius = footprint.radius_at(hit.position).unwrap();
                radii.push(sample.radius);
                oracle.apply_brush(hit.position, hit.normal, Vec3::ZERO, &sample);
                queue.dab_with_footprint(ray, settings.clone(), Vec3::ZERO, Some(footprint));
                assert!(queue.process(&mut document, 1, None).error.is_none());
                assert_eq!(
                    document.require_active_mesh().unwrap().positions,
                    oracle.positions
                );
            }
            if orthographic {
                assert_eq!(radii[0], radii[11]);
            } else {
                assert!(radii[11] < radii[0]);
            }
            queue.release(&mut document);
            let drawn = document.to_json().unwrap();
            assert_ne!(drawn, original);
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), original);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), drawn);
            assert!(queue.is_empty());
        }
    }

    #[test]
    fn queued_proxy_draw_matches_one_captured_stroke_with_adaptive_ancestry_and_undo() {
        for adaptive_detail in [false, true] {
            let mut document = Document::new(Mesh::sphere(3));
            let original = document.to_json().unwrap();
            let mut oracle = document.require_active_mesh().unwrap().snapshot();
            let settings = BrushSettings {
                radius: 0.18,
                strength: 0.35,
                symmetry_x: false,
                draw_accumulate: false,
                adaptive_detail,
                ..Default::default()
            };
            let mut proxy = oracle.prepare_draw_stroke(&settings).unwrap();
            let mut queue = DabQueue::default();
            queue.begin();
            for step in 0..12 {
                let ray = Ray {
                    origin: Vec3::new(0.08 + step as f32 * 0.003, 0.0, 3.0),
                    direction: -Vec3::Z,
                };
                let hit = oracle.raycast(ray).unwrap();
                proxy.apply(&mut oracle, hit.position, hit.normal, &settings);
                queue.dab(ray, settings.clone(), Vec3::ZERO);
                let progress = queue.process(&mut document, 1, None);
                assert!(progress.error.is_none());
                assert_eq!(
                    document.require_active_mesh().unwrap().positions,
                    oracle.positions
                );
                assert_eq!(
                    document.require_active_mesh().unwrap().normals,
                    oracle.normals
                );
                assert_eq!(
                    document.require_active_mesh().unwrap().triangles,
                    oracle.triangles
                );
            }
            let release = queue.release(&mut document);
            assert!(release.error.is_none() && queue.is_empty() && queue.draw_proxy.is_none());
            let drawn = document.to_json().unwrap();
            assert_ne!(drawn, original);
            if adaptive_detail {
                assert!(document.require_active_mesh().unwrap().positions.len() > 642);
            }
            assert!(!queue.process(&mut document, usize::MAX, None).changed);
            assert_eq!(document.to_json().unwrap(), drawn);
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), original);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), drawn);

            // The next gesture captures the drawn surface, not the first proxy.
            oracle = document.require_active_mesh().unwrap().snapshot();
            proxy = oracle.prepare_draw_stroke(&settings).unwrap();
            let ray = Ray {
                origin: Vec3::new(0.1, 0.0, 3.0),
                direction: -Vec3::Z,
            };
            let hit = oracle.raycast(ray).unwrap();
            proxy.apply(&mut oracle, hit.position, hit.normal, &settings);
            queue.begin();
            queue.dab(ray, settings.clone(), Vec3::ZERO);
            queue.release(&mut document);
            assert_eq!(
                document.require_active_mesh().unwrap().positions,
                oracle.positions
            );
            assert_eq!(
                document.require_active_mesh().unwrap().triangles,
                oracle.triangles
            );
        }
    }

    #[test]
    fn queued_morph_uses_target_and_release_commits_one_restoration() {
        let mut document = Document::new(Mesh::sphere(4));
        document.store_morph_target().unwrap();
        let target = document.require_active_mesh().unwrap().positions.clone();
        let ray = Ray {
            origin: Vec3::new(0.2, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        let hit = document
            .require_active_mesh()
            .unwrap()
            .raycast(ray)
            .unwrap();
        document.begin_stroke();
        document.require_active_mesh_mut().unwrap().apply_brush(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &BrushSettings {
                radius: 0.28,
                strength: 0.35,
                symmetry_x: false,
                adaptive_detail: false,
                ..Default::default()
            },
        );
        document.end_stroke();
        let drawn = document.to_json().unwrap();
        let target_error = |document: &Document| {
            document
                .require_active_mesh()
                .unwrap()
                .positions
                .iter()
                .zip(&target)
                .map(|(a, b)| a.distance_squared(*b) as f64)
                .sum::<f64>()
        };
        let before = target_error(&document);
        assert!(before > 0.0);
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            ray,
            BrushSettings {
                kind: BrushKind::Morph,
                radius: 0.28,
                strength: 1.0,
                symmetry_x: false,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let progress = queue.release(&mut document);
        assert!(progress.changed);
        assert!(progress.error.is_none());
        assert!(target_error(&document) < before);
        assert!(queue.is_empty());
        let restored = document.to_json().unwrap();
        assert!(!queue.process(&mut document, usize::MAX, None).changed);
        assert_eq!(document.to_json().unwrap(), restored);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), drawn);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), restored);
    }

    #[test]
    fn scene_dabs_respect_visibility_occlusion_and_active_ties() {
        let mut document = Document::new(Mesh::sphere(3));
        let original_id = document
            .active_subtool_id()
            .expect("nonempty fixture active id");
        let duplicate_id = document.duplicate_subtool(original_id).unwrap();
        let settings = BrushSettings {
            kind: BrushKind::Mask,
            radius: 0.3,
            symmetry_x: false,
            ..Default::default()
        };
        let ray = Ray {
            origin: Vec3::new(0.2, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(ray, settings.clone(), Vec3::ZERO);
        assert!(queue.release(&mut document).changed);
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .masks
                .iter()
                .any(|&value| value > 0.0)
        );
        document.select_subtool(original_id).unwrap();
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .masks
                .iter()
                .all(|&value| value == 0.0)
        );
        document.set_subtool_visible(original_id, false).unwrap();
        let before = document.to_json().unwrap();
        queue.begin();
        queue.dab(ray, settings.clone(), Vec3::ZERO);
        assert!(!queue.release(&mut document).changed);
        assert_eq!(document.to_json().unwrap(), before);
        document.set_subtool_visible(original_id, true).unwrap();
        // A nearer visible inactive part blocks strokes through it.
        document.select_subtool(duplicate_id).unwrap();
        document.begin_stroke();
        for point in &mut document.require_active_mesh_mut().unwrap().positions {
            point.z += 0.2;
        }
        document.end_stroke();
        document.select_subtool(original_id).unwrap();
        let before = document.to_json().unwrap();
        queue.begin();
        queue.dab(ray, settings, Vec3::ZERO);
        assert!(!queue.release(&mut document).changed);
        assert_eq!(document.to_json().unwrap(), before);
    }

    #[test]
    fn queued_morph_without_target_preserves_document_and_redo() {
        let mut document = Document::new(Mesh::sphere(3));
        document.clear_mask();
        document.begin_stroke();
        document.require_active_mesh_mut().unwrap().masks[0] = 0.5;
        document.end_stroke();
        assert!(document.undo());
        let before = document.to_json().unwrap();
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            Ray {
                origin: Vec3::new(0.0, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings {
                kind: BrushKind::Morph,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let progress = queue.release(&mut document);
        assert!(!progress.changed);
        assert!(progress.error.is_some());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.can_redo());
        assert!(queue.is_empty());
    }

    #[test]
    fn expired_motion_keeps_only_the_latest_intent() {
        let mut document = Document::new(Mesh::sphere(3));
        let mut direct = Document::new(document.require_active_mesh().unwrap().snapshot());
        let settings = BrushSettings {
            kind: BrushKind::Mask,
            radius: 0.25,
            symmetry_x: false,
            ..Default::default()
        };
        let mut queue = DabQueue::default();
        queue.begin();
        let mut last = Ray {
            origin: Vec3::new(-0.3, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        for index in 0..10 {
            last.origin.x = -0.3 + index as f32 * 0.06;
            queue.dab(last, settings.clone(), Vec3::ZERO);
        }
        let expired = Instant::now() - Duration::from_secs(1);
        for work in &mut queue.work {
            if let Work::Dab { received, .. } = work {
                *received = expired;
            }
        }
        let progress = queue.process(&mut document, 64, Some(Duration::from_millis(8)));
        assert_eq!(progress.dabs, 1);
        assert_eq!(queue.coalesced, 9);
        let hit = direct.require_active_mesh().unwrap().raycast(last).unwrap();
        direct.require_active_mesh_mut().unwrap().apply_brush(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &settings,
        );
        assert_eq!(
            document.require_active_mesh().unwrap().masks,
            direct.require_active_mesh().unwrap().masks
        );
        queue.release(&mut document);
        assert!(document.undo());
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .masks
                .iter()
                .all(|&mask| mask == 0.0)
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn sustained_overload_is_bounded_and_release_stops_all_future_edits() {
        let original = Mesh::sphere(3);
        let mut document = Document::new(original.snapshot());
        let mut queue = DabQueue::default();
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            ..Default::default()
        };
        queue.begin();
        for index in 0..2000 {
            let x = -0.3 + 0.6 * (index % 101) as f32 / 100.0;
            queue.dab(
                Ray {
                    origin: Vec3::new(x, 0.09, 3.0),
                    direction: -Vec3::Z,
                },
                settings.clone(),
                Vec3::ZERO,
            );
            assert!(queue.pending <= MAX_PENDING_DABS);
        }
        let progress = queue.release(&mut document);
        assert_eq!(progress.dabs, 1);
        assert!(progress.changed);
        assert_eq!(queue.pending, 0);
        assert!(queue.is_empty());
        assert_eq!(queue.coalesced, 1999);
        let stopped = document.to_json().unwrap();
        for _ in 0..20 {
            assert_eq!(
                queue
                    .process(&mut document, 64, Some(Duration::from_millis(8)))
                    .dabs,
                0
            );
            assert_eq!(document.to_json().unwrap(), stopped);
        }
        assert!(document.undo());
        assert_eq!(
            document.require_active_mesh().unwrap().positions,
            original.positions
        );
        assert_eq!(
            document.require_active_mesh().unwrap().triangles,
            original.triangles
        );
        assert!(!document.undo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), stopped);
    }

    #[test]
    fn a_same_frame_click_applies_once_and_commits_one_history_entry() {
        let mut document = Document::new(Mesh::sphere(3));
        let mut queue = DabQueue::default();
        queue.begin();
        queue.dab(
            Ray {
                origin: Vec3::new(0.2, 0.0, 3.0),
                direction: -Vec3::Z,
            },
            BrushSettings {
                radius: 0.3,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        assert!(queue.release(&mut document).changed);
        assert!(document.undo());
        assert!(!document.undo());
        assert!(queue.is_empty());
    }

    #[test]
    fn batching_preserves_every_dab_and_whole_stroke_undo() {
        let original = Mesh::sphere(3);
        let mut queued = Document::new(original.snapshot());
        let mut direct = Document::new(original.snapshot());
        let mut queue = DabQueue::default();
        let settings = BrushSettings {
            radius: 0.18,
            strength: 0.35,
            ..Default::default()
        };
        queue.begin();
        direct.begin_stroke();
        for i in 0..25 {
            let ray = Ray {
                origin: Vec3::new(-0.25 + i as f32 * 0.02, 0.09, 3.0),
                direction: -Vec3::Z,
            };
            let hit = direct.require_active_mesh().unwrap().raycast(ray).unwrap();
            direct.require_active_mesh_mut().unwrap().apply_brush(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings,
            );
            queue.dab(ray, settings.clone(), Vec3::ZERO);
        }
        queue.end();
        direct.end_stroke();
        let mut dabs = 0;
        while !queue.is_empty() {
            let result = queue.process(&mut queued, 2, None);
            assert!(result.dabs <= 2);
            dabs += result.dabs;
        }
        assert_eq!(dabs, 25);
        assert_eq!(queued.to_json().unwrap(), direct.to_json().unwrap());
        let sculpt = queued.to_json().unwrap();
        assert!(queued.undo());
        assert_eq!(
            queued.require_active_mesh().unwrap().positions,
            original.positions
        );
        assert_eq!(
            queued.require_active_mesh().unwrap().triangles,
            original.triangles
        );
        assert!(!queued.undo());
        assert!(queued.redo());
        assert_eq!(queued.to_json().unwrap(), sculpt);
    }

    #[test]
    fn partial_processing_then_release_preserves_two_distinct_undo_steps() {
        let mut document = Document::new(Mesh::sphere(3));
        let original = document.to_json().unwrap();
        let original_colors = document.require_active_mesh().unwrap().colors.clone();
        let mut queue = DabQueue::default();
        let first = BrushSettings {
            kind: BrushKind::Mask,
            radius: 0.3,
            symmetry_x: false,
            ..Default::default()
        };
        let second = BrushSettings {
            kind: BrushKind::Paint,
            color: [1.0, 0.0, 0.0],
            ..first.clone()
        };
        for settings in [first, second] {
            queue.begin();
            for x in [0.15, 0.2, 0.25] {
                queue.dab(
                    Ray {
                        origin: Vec3::new(x, 0.0, 3.0),
                        direction: -Vec3::Z,
                    },
                    settings.clone(),
                    Vec3::ZERO,
                );
            }
            assert_eq!(queue.process(&mut document, 1, None).dabs, 1);
            assert_eq!(queue.release(&mut document).dabs, 1);
            assert_eq!(queue.pending, 0);
            assert!(queue.is_empty());
        }
        let final_state = document.to_json().unwrap();
        let masks = document.require_active_mesh().unwrap().masks.clone();
        assert!(document.undo());
        assert_eq!(document.require_active_mesh().unwrap().masks, masks);
        assert_eq!(
            document.require_active_mesh().unwrap().colors,
            original_colors
        );
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), original);
        assert!(!document.undo());
        assert!(document.redo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), final_state);
    }

    #[test]
    fn queued_releases_and_new_strokes_remain_separate_and_flushable() {
        let mut document = Document::new(Mesh::sphere(3));
        let mut queue = DabQueue::default();
        let first = BrushSettings {
            kind: BrushKind::Mask,
            radius: 0.3,
            symmetry_x: false,
            ..Default::default()
        };
        let second = BrushSettings {
            kind: BrushKind::Paint,
            radius: 0.3,
            color: [1.0, 0.0, 0.0],
            ..first.clone()
        };
        let ray = Ray {
            origin: Vec3::new(0.2, 0.0, 3.0),
            direction: -Vec3::Z,
        };
        queue.begin();
        queue.dab(ray, first, Vec3::ZERO);
        queue.end();
        queue.begin();
        queue.dab(ray, second, Vec3::ZERO);
        queue.end();
        assert_eq!(queue.process(&mut document, 1, None).dabs, 1);
        assert!(!queue.is_empty());
        let masks = document.require_active_mesh().unwrap().masks.clone();
        let before_paint = document.require_active_mesh().unwrap().colors.clone();
        assert!(queue.process(&mut document, usize::MAX, None).painted);
        assert!(queue.is_empty());
        let final_state = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.require_active_mesh().unwrap().masks, masks);
        assert_eq!(document.require_active_mesh().unwrap().colors, before_paint);
        assert!(document.undo());
        assert!(
            document
                .require_active_mesh()
                .unwrap()
                .masks
                .iter()
                .all(|&mask| mask == 0.0)
        );
        assert!(document.redo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), final_state);
    }
}
