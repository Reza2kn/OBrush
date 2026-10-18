//! Normal input adapter for the bounded held-setting foundation. No worker,
//! deferred S contact, hidden model setter or GPU readback is used here.
use super::*;
use crate::app_preferences::PreferenceProfile;
use crate::shortcut_profile::Modal;
use crate::source_modal::{ModalState, Movement};
use crate::source_paint_host::{PaintMode, SourcePaintPreferences};
use obrush_core::{
    PaintViewCapture, SourceOriginalFaceContact, SourcePickQueryAdmission, SourcePickQueryBudget,
    SourcePreparedPickQuery,
};

#[cfg(test)]
use obrush_core::SourcePickSnapshotAdmission;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Publication {
    revision: u64,
    subtool: u32,
    level: usize,
    pose: Option<([u32; 16], [u32; 3])>,
}
struct QueryOwner {
    publication: Publication,
    preferences: [u64; 16],
    view: ([u64; 16], [u64; 4]),
    epoch: u64,
}
pub(super) struct HostModal {
    pub(super) state: ModalState,
    publication: Option<Publication>,
    admission: Option<SourcePickQueryAdmission>,
    ready: Option<Arc<SourcePreparedPickQuery>>,
    rejected_snapshot: Option<Arc<Mesh>>,
    owner: Option<QueryOwner>,
    availability: &'static str,
    event_focused: bool,
    admission_progress: Option<obrush_core::SourcePickQueryProgress>,
    last_sample: Option<obrush_core::SourceAttributePickReport>,
    last_failure: Option<String>,
    pub(super) observation: serde_json::Value,
    pub(super) cursor_observation: serde_json::Value,
}
impl Default for HostModal {
    fn default() -> Self {
        Self {
            state: Default::default(),
            publication: None,
            admission: None,
            ready: None,
            rejected_snapshot: None,
            owner: None,
            availability: "Source Paint response required",
            event_focused: true,
            admission_progress: None,
            last_sample: None,
            last_failure: None,
            observation: serde_json::Value::Null,
            cursor_observation: serde_json::Value::Null,
        }
    }
}
fn words(p: &SourcePaintPreferences) -> [u64; 16] {
    let o = &p.options;
    [
        p.radius.to_bits(),
        p.radius_factor.to_bits(),
        o.intensity.to_bits(),
        o.hardness.to_bits(),
        u64::from(o.color[0].to_bits()),
        u64::from(o.color[1].to_bits()),
        u64::from(o.color[2].to_bits()),
        u64::from(o.roughness.to_bits()),
        u64::from(o.metallic.to_bits()),
        o.tablet.pressure.to_bits(),
        o.tablet.factor.to_bits(),
        u64::from(o.write_rgb),
        u64::from(o.write_roughness),
        u64::from(o.write_metallic),
        u64::from(o.culling),
        u64::from(o.alpha.is_some()),
    ]
}
fn frame_words(view: PaintViewCapture) -> ([u64; 16], [u64; 4]) {
    (
        view.world_to_clip.map(f64::to_bits),
        view.viewport.map(f64::to_bits),
    )
}
fn intensity_exposed(kind: BrushKind) -> bool {
    !matches!(
        kind,
        BrushKind::Twist | BrushKind::LocalScale | BrushKind::Drag | BrushKind::Morph
    )
}
fn paint_candidate(
    p: &SourcePaintPreferences,
    s: obrush_core::SourceAttributePickSample,
) -> Result<SourcePaintPreferences, String> {
    let mut candidate = p.clone();
    candidate.options.color = s.rgb;
    candidate.options.roughness = s.roughness;
    candidate.options.metallic = s.metallic;
    candidate.validate()?;
    Ok(candidate)
}

// These adapters are used by the actual update/ordered input path, and by the
// normal Egui event regressions. They never change a brush setting or focus.
fn observe_primary_event(state: &mut ModalState, event: &egui::Event) {
    if let egui::Event::PointerButton {
        button: egui::PointerButton::Primary,
        pressed,
        ..
    } = event
    {
        state.primary(*pressed);
        if !*pressed {
            state.focus_contact_recovery = false;
        }
    }
}
fn latch_focus_contact(state: &mut ModalState) {
    if state.primary_down || state.suppress_contact {
        state.focus_contact_recovery = true;
    }
}
/// This is called only while walking an actual Egui delivery, before the same
/// delivery reaches the unchanged primary observer. It never changes Egui
/// PointerState or emits a missing release. Focus return/movement are not down.
fn recover_fresh_primary(
    state: &mut ModalState,
    event: &egui::Event,
    final_focused: bool,
    event_focused: bool,
    owner_empty: bool,
) -> bool {
    if !matches!(
        event,
        egui::Event::PointerButton {
            button: egui::PointerButton::Primary,
            pressed: true,
            ..
        }
    ) || !state.focus_contact_recovery
        || !final_focused
        || !event_focused
        || state.active()
        || !state.queries.rows().is_empty()
        || !owner_empty
    {
        return false;
    }
    state.primary_down = false;
    state.suppress_contact = false;
    state.suppress_frame = false;
    state.focus_contact_recovery = false;
    true
}
/// Keep actual ordered admission across frames and profile walks. A final
/// focused=true cannot move a gain ahead of its delivery. None is boot-only.
fn ordered_frame_focus(state: &mut ModalState, final_focused: bool) -> bool {
    if !final_focused {
        state.ordered_window_focus = Some(false);
    }
    state.ordered_window_focus.unwrap_or(true)
}
fn reconcile_legacy_primary(
    state: &mut ModalState,
    ctx: &egui::Context,
    profile: PreferenceProfile,
) {
    if profile == PreferenceProfile::SculptGl {
        return;
    }
    let (final_focused, events) = ctx.input(|i| (i.focused, i.events.clone()));
    let mut event_focused = ordered_frame_focus(state, final_focused);
    if !final_focused {
        latch_focus_contact(state);
        state.cancel("unfocused delivery");
    }
    for event in &events {
        // The actual host clears its Source owner on profile cancellation
        // before this Legacy walk; Legacy cannot admit a Source query owner.
        observe_modal_contact_event(state, event, final_focused, &mut event_focused, true);
    }
}
fn observe_window_focus(state: &mut ModalState, event: &egui::Event, event_focused: &mut bool) {
    if let egui::Event::WindowFocused(focused) = event {
        *event_focused = *focused;
        state.ordered_window_focus = Some(*focused);
        if !*focused {
            latch_focus_contact(state);
            state.cancel("ordered window focus lost");
        }
    }
}
/// Shared production adapter for the Source and Legacy ordered event walks.
/// The caller clears a host query owner on focus loss; an old nonempty owner
/// can never be treated as recoverable by a fresh primary delivery.
fn observe_modal_contact_event(
    state: &mut ModalState,
    event: &egui::Event,
    final_focused: bool,
    event_focused: &mut bool,
    owner_empty: bool,
) -> bool {
    *event_focused = ordered_frame_focus(state, final_focused);
    observe_window_focus(state, event, event_focused);
    if !final_focused {
        // A final unfocused snapshot is failclosed even for an inconsistent
        // queued gain; no later frame may inherit an unadmitted focused flag.
        state.ordered_window_focus = Some(false);
        *event_focused = false;
    }
    if !final_focused || !*event_focused {
        state.cancel("unfocused delivery");
    }
    let recovered = recover_fresh_primary(state, event, final_focused, *event_focused, owner_empty);
    observe_primary_event(state, event);
    recovered
}
fn delivery_focus_admitted(ctx: &egui::Context, event_focused: bool) -> bool {
    event_focused && ctx.input(|i| i.focused)
}
fn picker_epoch_available(state: &ModalState, owner_present: bool) -> bool {
    state.queries.rows().is_empty() && !owner_present
}
fn contact_owned(state: &ModalState) -> bool {
    state.suppress_frame || state.suppress_contact || state.active()
}

impl OBrushApp {
    fn modal_publication(&self) -> Option<Publication> {
        let subtool = self.document.active_subtool_id()?;
        self.rendered_scene
            .iter()
            .find(|s| s.active && s.id == subtool)?;
        Some(Publication {
            revision: self.revision,
            subtool,
            level: self.document.current_level(),
            pose: self.document.active_object_metadata().map(|m| {
                (
                    m.affine.local_to_world.map(f32::to_bits),
                    m.affine.interaction_center.map(f32::to_bits),
                )
            }),
        })
    }
    fn modal_view(&self, rect: egui::Rect) -> Result<PaintViewCapture, String> {
        if self.egui_ctx.zoom_factor() != 1.0 {
            return Err("Held Source picker requires UI scale 100%.".into());
        }
        let view = PaintViewCapture {
            world_to_clip: (glam::DMat4::from_cols_array(
                &self
                    .camera
                    .projection(rect.aspect_ratio())
                    .to_cols_array()
                    .map(f64::from),
            ) * self.camera.view64())
            .to_cols_array(),
            viewport: [
                f64::from(rect.left()),
                f64::from(rect.top()),
                f64::from(rect.width()),
                f64::from(rect.height()),
            ],
        };
        view.validate()?;
        Ok(view)
    }
    fn modal_idle(&self) -> bool {
        self.stroke.is_none()
            && self.pending_dabs.is_empty()
            && !self.camera_navigation_active
            && !self.transform_mode
            && self.transform_drag.is_none()
            && self.topology_job.is_none()
            && self.pending_sgl_export.is_none()
            && self.source_sgl_import.pending.is_none()
            && self.mask_region_gesture.is_none()
            && self.mask_selection_mode == MaskSelectionMode::Brush
            && !self.paint_pick_armed
            && !self.paint_pick_dragging
    }
    fn modal_values(&self) -> (Option<f64>, Option<f64>) {
        if self.transform_mode || self.brush.kind == BrushKind::Morph {
            return (None, None);
        }
        if self.brush.kind == BrushKind::Paint
            && self.source_paint_preferences.mode == PaintMode::SourceOrdinary
        {
            return (
                Some(self.source_paint_preferences.radius),
                Some(self.source_paint_preferences.options.intensity),
            );
        }
        if let Some(p) = self.source_geometry_preferences.get(self.brush.kind)
            && p.mode == crate::source_geometry_host::SourceGeometryMode::SourceOrdinary
        {
            return (Some(p.radius), Some(p.options.intensity));
        }
        (
            self.brush_screen_space
                .then_some(f64::from(self.brush_screen_radius)),
            intensity_exposed(self.brush.kind).then_some(f64::from(self.brush.strength)),
        )
    }
    fn apply_modal_movement(&mut self, m: Movement) {
        if m == Movement::default() {
            return;
        }
        if self.brush.kind == BrushKind::Paint
            && self.source_paint_preferences.mode == PaintMode::SourceOrdinary
        {
            let mut candidate = self.source_paint_preferences.clone();
            if let Some(v) = m.radius {
                candidate.radius = v;
            }
            if let Some(v) = m.intensity {
                candidate.options.intensity = v;
            }
            match crate::source_paint_ui::commit_candidate(Some(candidate), || {
                self.finish_stroke();
                !self.stroke_rejected
            }) {
                crate::source_paint_ui::PreferenceCommit::Accepted(p) => {
                    self.source_paint_preferences = p
                }
                crate::source_paint_ui::PreferenceCommit::Invalid(e) => self.message = e,
                crate::source_paint_ui::PreferenceCommit::ReleaseRejected => self
                    .source_modal
                    .state
                    .cancel("preference release rejected"),
                crate::source_paint_ui::PreferenceCommit::Unchanged => {}
            }
            return;
        }
        if let Some(mut p) = self.source_geometry_preferences.get(self.brush.kind)
            && p.mode == crate::source_geometry_host::SourceGeometryMode::SourceOrdinary
        {
            if let Some(v) = m.radius {
                p.radius = v;
            }
            if let Some(v) = m.intensity {
                p.options.intensity = v;
            }
            if !self.commit_source_geometry_preferences(self.brush.kind, p) {
                self.source_modal
                    .state
                    .cancel("preference release rejected");
            }
            return;
        }
        let old_radius = self.brush_screen_radius;
        let old_strength = self.brush.strength;
        if let Some(v) = m.radius {
            self.brush_screen_radius = v as f32;
        }
        if let Some(v) = m.intensity {
            self.brush.strength = v as f32;
        }
        if !self.commit_brush_view() {
            self.brush_screen_radius = old_radius;
            self.brush.strength = old_strength;
            self.source_modal
                .state
                .cancel("preference release rejected");
        }
    }
    pub(super) fn modal_begin_frame(&mut self, ctx: &egui::Context) {
        self.source_modal.state.begin_frame();
        self.source_modal.event_focused =
            ordered_frame_focus(&mut self.source_modal.state, ctx.input(|i| i.focused));
        if !self.source_modal.event_focused {
            latch_focus_contact(&mut self.source_modal.state);
        }
        // End the Source owner before a Legacy fresh-contact event can be
        // admitted. The old frame/contact quarantine remains until real input.
        if self.shortcut_resolver.profile() != PreferenceProfile::SculptGl
            || !self.source_modal.event_focused
        {
            self.modal_cancel("profile or focus changed");
        }
        reconcile_legacy_primary(
            &mut self.source_modal.state,
            ctx,
            self.shortcut_resolver.profile(),
        );
        self.source_modal.event_focused = self
            .source_modal
            .state
            .ordered_window_focus
            .unwrap_or(self.source_modal.event_focused);
        if !self.source_modal.state.picker && self.source_modal.state.queries.rows().is_empty() {
            self.source_modal.owner = None;
        }
        if self.source_modal.state.active() && ctx.input(|i| i.modifiers.any()) {
            self.modal_cancel("mixed modifiers unsupported");
        }
        if self.source_modal.publication.as_ref() != self.modal_publication().as_ref()
            || self.snapshot_revision != self.revision
        {
            self.source_modal.ready = None;
            self.source_modal.admission = None;
            self.source_modal.rejected_snapshot = None;
            self.source_modal.publication = None;
            self.source_modal.admission_progress = None;
            self.source_modal
                .state
                .close_picker("publication changed", true);
            self.source_modal.owner = None;
        }
    }
    pub(super) fn modal_cancel(&mut self, reason: &'static str) {
        self.source_modal.state.cancel(reason);
        self.source_modal.owner = None;
    }
    pub(super) fn modal_invalidate_publication(&mut self, reason: &'static str) {
        self.modal_cancel(reason);
        self.source_modal.ready = None;
        self.source_modal.admission = None;
        self.source_modal.rejected_snapshot = None;
        self.source_modal.publication = None;
        self.source_modal.admission_progress = None;
    }
    pub(super) fn modal_close_picker(&mut self, reason: &'static str) {
        let active = self.source_modal.state.picker;
        self.source_modal.state.close_picker(reason, true);
        self.source_modal.owner = None;
        if active {
            self.message = "Paint picker stopped.".into();
        }
    }
    pub(super) fn modal_end(&mut self, kind: Modal) {
        self.source_modal.state.end(kind);
        if kind == Modal::Picker {
            self.message = "Paint picker released.".into();
        }
    }
    pub(super) fn modal_shortcut_idle(&self) -> bool {
        self.modal_idle()
            && !self.source_modal.state.primary_down
            && !self.source_modal.state.picker
    }
    pub(super) fn modal_start(&mut self, kind: Modal, modifiers: egui::Modifiers) {
        if !delivery_focus_admitted(&self.egui_ctx, self.source_modal.event_focused) {
            return;
        }
        if kind == Modal::Picker
            && !picker_epoch_available(&self.source_modal.state, self.source_modal.owner.is_some())
        {
            self.message =
                "Held picker retains its first contact this frame; use a fresh S press next frame."
                    .into();
            return;
        }
        let (radius, intensity) = self.modal_values();
        let eligible = self.modal_idle()
            && !modifiers.ctrl
            && !modifiers.shift
            && !modifiers.alt
            && !modifiers.command
            && !self.egui_ctx.input(|i| i.modifiers.any());
        if kind == Modal::Picker {
            let ready = self.source_modal.ready.as_ref().is_some();
            let eligible = eligible
                && self.brush.kind == BrushKind::Paint
                && self.source_paint_preferences.mode == PaintMode::SourceOrdinary
                && ready
                && self.snapshot_revision == self.revision
                && self.source_paint_preferences.validate().is_ok();
            if !eligible {
                self.message = format!(
                    "Held picker: {}. Use a fresh S press when ready.",
                    self.source_modal.availability
                );
                return;
            }
            let Some(publication) = self.modal_publication() else {
                return;
            };
            let Ok(view) = self.modal_view(self.last_viewport) else {
                self.message =
                    "Held picker requires a valid prepared viewport at UI scale 100%.".into();
                return;
            };
            if self.source_modal.state.start(kind, None, true) {
                self.source_modal.owner = Some(QueryOwner {
                    publication,
                    preferences: words(&self.source_paint_preferences),
                    view: frame_words(view),
                    epoch: self.source_modal.state.epoch,
                });
                self.message = "Paint picker active: move over the surface to sample.".into();
            }
        } else {
            let eligible = eligible
                && match kind {
                    Modal::Radius => radius.is_some(),
                    Modal::Intensity => intensity.is_some(),
                    Modal::Picker => false,
                };
            if !self.source_modal.state.start(kind, radius, eligible) {
                self.message=if kind==Modal::Radius {"Held radius needs an idle exposed screen-radius owner; world-radius units are retained."} else {"Held intensity needs an idle tool with an intensity control."}.into();
            }
        }
    }
    /// Called exactly once for each event in the normal Source shortcut walk.
    pub(super) fn modal_pointer_event(&mut self, event: &egui::Event, editor: bool) {
        self.source_modal.state.next_event();
        let final_focused = self.egui_ctx.input(|i| i.focused);
        observe_modal_contact_event(
            &mut self.source_modal.state,
            event,
            final_focused,
            &mut self.source_modal.event_focused,
            self.source_modal.owner.is_none(),
        );
        if !final_focused || !self.source_modal.event_focused {
            self.source_modal.owner = None;
        }
        if editor && self.source_modal.state.active() {
            self.modal_cancel("editor acquired input");
        }
        match event {
            egui::Event::PointerMoved(p) => {
                let (radius, intensity) = self.modal_values();
                let movement = self.source_modal.state.move_pointer(
                    [f64::from(p.x), f64::from(p.y)],
                    intensity,
                    radius.is_some(),
                );
                self.apply_modal_movement(movement);
            }
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                ..
            } => {
                self.source_modal.state.last_pointer = Some([f64::from(pos.x), f64::from(pos.y)]);
                if *button != egui::PointerButton::Primary && *pressed {
                    self.modal_cancel("navigation contact");
                }
                if !*pressed {
                    self.source_modal.state.close_picker("mouse up", false);
                }
            }
            egui::Event::PointerGone => self.modal_cancel("pointer left window"),
            egui::Event::MouseWheel { .. } => self.modal_cancel("wheel navigation"),
            _ => {}
        }
    }
    pub(super) fn modal_suppresses_contact(&self) -> bool {
        contact_owned(&self.source_modal.state)
    }
    /// Runs only after the real measured frame, before ordinary contact work.
    pub(super) fn modal_flush_picker(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let ledger = std::mem::take(&mut self.source_modal.state.queries);
        if ledger.rows().is_empty() {
            return;
        }
        let Some(owner) = self.source_modal.owner.take() else {
            return;
        };
        let view = match self.modal_view(rect) {
            Ok(v) => v,
            Err(e) => {
                self.source_modal.last_failure = Some(e);
                self.modal_close_picker("invalid frame");
                return;
            }
        };
        if !ctx.input(|i| i.focused)
            || (ctx.wants_keyboard_input()
                && ctx
                    .memory(|m| m.focused())
                    .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some()))
            || !self.modal_idle()
            || self.brush.kind != BrushKind::Paint
            || self.source_paint_preferences.mode != PaintMode::SourceOrdinary
            || self.modal_publication().as_ref() != Some(&owner.publication)
            || owner.preferences != words(&self.source_paint_preferences)
            || owner.view != frame_words(view)
            || owner.epoch != self.source_modal.state.epoch
            || self.snapshot_revision != self.revision
        {
            self.modal_close_picker("stale owned query");
            return;
        }
        let mut budget = SourcePickQueryBudget::default();
        for query in ledger.rows() {
            if query.epoch != owner.epoch
                || !rect.contains(Pos2::new(query.screen[0] as f32, query.screen[1] as f32))
            {
                continue;
            }
            let ray = match view.ray(query.screen) {
                Ok(r) => r,
                Err(e) => {
                    self.source_modal.last_failure = Some(e);
                    continue;
                }
            };
            let Some(snapshot) = self.source_modal.ready.as_ref() else {
                self.modal_close_picker("snapshot not ready");
                break;
            };
            let pose = self.document.active_object_metadata().map(|m| &m.affine);
            let hit = match snapshot.ray(pose, ray, &mut budget) {
                Ok(Some(hit)) => hit,
                Ok(None) => continue,
                Err(e) => {
                    self.source_modal.last_failure =
                        Some(format!("{e:?} at input{}", query.ordinal));
                    self.modal_close_picker("ray budget or invalid input");
                    break;
                }
            };
            let report = match snapshot.sample_with_budget(
                SourceOriginalFaceContact {
                    face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
                    local_point: hit.local_position,
                },
                &mut budget,
            ) {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    self.source_modal.last_failure = Some(e.to_string());
                    continue;
                }
                Err(e) => {
                    self.source_modal.last_failure =
                        Some(format!("{e:?} at input{}", query.ordinal));
                    self.modal_close_picker("corner budget");
                    break;
                }
            };
            match paint_candidate(&self.source_paint_preferences, report.sample) {
                Ok(candidate) => {
                    // Idle-only preference ownership: all five targets replace
                    // atomically, with no Document/history/appearance mutation.
                    self.source_paint_preferences = candidate;
                    self.message = "Paint color and material sampled.".into();
                    self.source_modal.last_sample = Some(report);
                    self.source_modal.last_failure = None;
                }
                Err(e) => self.source_modal.last_failure = Some(e),
            }
        }
        if self.source_modal.state.picker {
            self.source_modal.owner = Some(QueryOwner {
                preferences: words(&self.source_paint_preferences),
                ..owner
            });
        }
    }
    /// Reuses the published Arc; does not call subtool_render_snapshots per move.
    pub(super) fn modal_prepare_snapshot(&mut self, ctx: &egui::Context) {
        if self.brush.kind != BrushKind::Paint
            || self.source_paint_preferences.mode != PaintMode::SourceOrdinary
        {
            self.source_modal.availability = "Source Paint response required";
            self.source_modal.admission = None;
            self.source_modal.rejected_snapshot = None;
            self.source_modal.ready = None;
            self.source_modal.publication = None;
            self.modal_close_picker("Paint response changed");
            return;
        }
        if self
            .stroke
            .as_ref()
            .is_some_and(|s| s.source_paint.is_some())
        {
            self.source_modal.availability = "Surface picker prepares after the current stroke";
            return;
        }
        if self.snapshot_revision != self.revision {
            return;
        }
        let Some(publication) = self.modal_publication() else {
            self.source_modal.availability = "Active visible mesh required";
            self.modal_close_picker("no active visible mesh");
            return;
        };
        let Some(published) = self
            .rendered_scene
            .iter()
            .find(|s| s.active && s.id == publication.subtool)
            .map(|s| s.mesh.clone())
        else {
            return;
        };
        if self.source_modal.publication.as_ref() == Some(&publication)
            && self
                .source_modal
                .rejected_snapshot
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &published))
        {
            // This exact immutable publication already failed. Do not restart
            // the same validation/index preparation on every idle frame.
            return;
        }
        let same = self.source_modal.publication.as_ref() == Some(&publication)
            && (self
                .source_modal
                .ready
                .as_ref()
                .is_some_and(|r| r.matches(&published))
                || self
                    .source_modal
                    .admission
                    .as_ref()
                    .is_some_and(|a| a.matches(&published)));
        if !same {
            self.modal_close_picker("new snapshot publication");
            self.source_modal.ready = None;
            self.source_modal.admission = None;
            self.source_modal.rejected_snapshot = None;
            self.source_modal.admission_progress = None;
            self.source_modal.publication = Some(publication);
            match SourcePickQueryAdmission::new_for_batch(Arc::clone(&published), 2) {
                Ok(a) => self.source_modal.admission = Some(a),
                Err(e) => {
                    self.source_modal.rejected_snapshot = Some(Arc::clone(&published));
                    self.source_modal.last_failure = Some(format!("Admission {e:?}"));
                    self.source_modal.availability = "Unavailable: invalid snapshot";
                    return;
                }
            }
        }
        if let Some(admission) = self.source_modal.admission.as_mut() {
            match admission.step(obrush_core::MAX_SOURCE_PICK_ADMISSION_OPERATIONS) {
                Ok(progress) => {
                    self.source_modal.admission_progress = Some(progress);
                    if progress.ready() {
                        if let Some(a) = self.source_modal.admission.take() {
                            match a.finish() {
                                Ok(v) => self.source_modal.ready = Some(Arc::new(v)),
                                Err(e) => {
                                    self.source_modal.last_failure =
                                        Some(format!("Admission {e:?}"))
                                }
                            }
                        }
                    } else {
                        ctx.request_repaint();
                    }
                }
                Err(e) => {
                    self.source_modal.admission = None;
                    self.source_modal.rejected_snapshot = None;
                    self.source_modal.rejected_snapshot = Some(Arc::clone(&published));
                    self.source_modal.last_failure = Some(format!("Admission {e:?}"));
                }
            }
        }
        self.source_modal.availability = if self.source_modal.ready.is_some() {
            "Ready: fresh S press samples active Paint surface"
        } else if self.source_modal.admission.is_some() {
            "Preparing: S is not armed"
        } else {
            "Unavailable: invalid snapshot"
        };
    }
    pub(super) fn modal_status(&self, ui: &mut egui::Ui) {
        if self.shortcut_resolver.profile() == PreferenceProfile::SculptGl {
            ui.label(egui::RichText::new(self.source_modal.availability).small());
            ui.label(egui::RichText::new("C: exposed intensity. X: exposed screen radius. S samples a prepared active Paint surface. Source cursor appearance remains unverified.").small());
        }
    }
    pub(super) fn paint_cursor_capture(&self) -> Option<Arc<SourcePreparedPickQuery>> {
        if self.snapshot_revision != self.revision
            || self.source_modal.publication.as_ref() != self.modal_publication().as_ref()
        {
            return None;
        }
        let part = self.document.active_subtool_id()?;
        let mesh = &self
            .rendered_scene
            .iter()
            .find(|s| s.active && s.id == part)?
            .mesh;
        self.source_modal
            .ready
            .as_ref()
            .filter(|q| q.matches(mesh))
            .cloned()
    }
    /// Actual normal Source Paint hover uses the sealed publication prepared
    /// above; it never allocates normals, opens a stroke or claims alpha tracks.
    pub(super) fn paint_surface_cursor(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        response: &egui::Response,
        rect: egui::Rect,
    ) -> bool {
        let modifiers = ctx.input(|i| i.modifiers);
        let preview = self.preferences.preview(
            PreferenceModifiers {
                shift: modifiers.shift,
                control: modifiers.ctrl,
                alt: modifiers.alt,
            },
            ContactInput::Pen(self.brush.pressure),
            None,
            None,
        );
        let source_paint = preview
            .as_ref()
            .is_ok_and(|p| p.settings.kind == BrushKind::Paint)
            && self.source_paint_preferences.mode == PaintMode::SourceOrdinary;
        if !source_paint {
            return false;
        }
        self.source_modal.cursor_observation = serde_json::json!({"status":"No current Source Paint surface hover","revision":self.revision});
        if self.camera_navigation_active
            || self.source_modal.state.radius
            || self.source_modal.state.intensity
        {
            return true;
        }
        let point = response.hover_pos().or_else(|| {
            self.stroke.as_ref().filter(|s| s.source_paint.is_some())?;
            ctx.input(|i| i.pointer.latest_pos())
                .filter(|p| rect.contains(*p) && ctx.layer_id_at(*p) == Some(response.layer_id))
        });
        let Some(pos) = point else {
            return true;
        };
        let result = (|| -> Result<crate::source_paint_cursor::Frame, String> {
            let active_capture = self
                .stroke
                .as_ref()
                .and_then(|s| s.source_paint.as_ref())
                .and_then(|s| s.lock().ok()?.cursor_capture(&self.document));
            if let Some(capture) = active_capture {
                let input = self.capture_source_paint_input(pos, rect)?;
                return crate::source_paint_cursor::prepare(
                    &capture.query,
                    capture.pose.as_ref(),
                    input.view,
                    input.screen,
                    capture.plane,
                );
            }
            if self.snapshot_revision != self.revision {
                return Err("Cursor publication is not current".into());
            }
            let publication = self
                .modal_publication()
                .ok_or("Cursor active part is unavailable")?;
            if self.source_modal.publication.as_ref() != Some(&publication) {
                return Err("Cursor active publication changed".into());
            }
            let published = self
                .rendered_scene
                .iter()
                .find(|s| s.active && s.id == publication.subtool)
                .ok_or("Cursor active publication missing")?;
            let ready = self
                .source_modal
                .ready
                .as_ref()
                .filter(|q| q.matches(&published.mesh))
                .ok_or("Cursor surface is preparing")?;
            let pose = published.metadata.as_ref().map(|m| &m.affine);
            let plane = if self.brush.symmetry_x {
                let warm = self.document.source_geometry_state(publication.subtool);
                let bounds = warm
                    .as_ref()
                    .map_or_else(|| ready.source_root_loose_bounds(), |s| s.root_loose());
                let center = pose
                    .map(|p| p.interaction_center)
                    .or_else(|| warm.as_ref().map(|s| s.interaction_center()))
                    .unwrap_or_else(|| {
                        std::array::from_fn(|i| ((bounds[i] + bounds[i + 3]) * 0.5) as f32)
                    });
                Some(
                    obrush_core::SourceMirrorPlane::from_relative_offset(
                        center,
                        [1., 0., 0.],
                        self.source_paint_preferences.mirror_offset,
                        bounds,
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else {
                None
            };
            let input = self.capture_source_paint_input(pos, rect)?;
            crate::source_paint_cursor::prepare(ready, pose, input.view, input.screen, plane)
        })();
        match result {
            Ok(frame) => {
                let radius = self.source_paint_raw.sample().map_or(0., |raw| {
                    self.source_paint_preferences.radius
                        * (1.
                            + self.source_paint_preferences.radius_factor
                                * (raw.pressure * 2. - 1.))
                }) as f32;
                self.brush_radius_px = radius;
                let painter = painter.with_clip_rect(rect);
                let mut dots = 0;
                for (i, contact) in [frame.primary, frame.mirrored].into_iter().enumerate() {
                    if let Some(dot) = contact
                        && (0.0..=1.0).contains(&dot.screen[2])
                    {
                        let point = Pos2::new(dot.screen[0] as f32, dot.screen[1] as f32);
                        if i == 0 && self.stroke.is_none() {
                            painter.circle_stroke(point, radius, Stroke::new(1.2, ACCENT));
                        }
                        painter.circle_stroke(point, 2., Stroke::new(1., ACCENT));
                        dots += 1;
                    }
                }
                self.source_modal.cursor_observation = serde_json::json!({"status":"Completed read-only cursor contacts","revision":self.revision,"frame":frame.diagnostics(),"dots_submitted":dots,"radius":radius,"clip":[rect.left(),rect.top(),rect.right(),rect.bottom()],"input_screen":self.source_paint_raw.position(pos),"scope":"Actual sealed active-surface picks; shared8192face/triangle4096node budget; projected markers only, Source oriented ring/shader/appearance remains unverified"});
                if dots > 0 {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
            }
            Err(error) => {
                self.source_modal.cursor_observation = serde_json::json!({"status":"Source Paint cursor unavailable","reason":error,"revision":self.revision});
            }
        }
        true
    }
    pub(super) fn modal_cursor(&self, ctx: &egui::Context, rect: egui::Rect) {
        if self.source_modal.state.picker {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        if self.source_modal.state.radius
            && let Some(anchor) = self.source_modal.state.radius_anchor
            && let Some(radius) = self.modal_values().0
        {
            let p = Pos2::new(anchor[0] as f32, anchor[1] as f32);
            if p.is_finite() && rect.contains(p) {
                let painter = ctx
                    .layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        egui::Id::new("held_radius_foundation"),
                    ))
                    .with_clip_rect(rect);
                painter.circle_stroke(
                    p,
                    radius as f32,
                    Stroke::new(1., Color32::from_rgb(240, 140, 70)),
                );
            }
        }
    }
    pub(super) fn modal_observe_end_update(&mut self) {
        let s = &self.source_modal.state;
        let targets = &self.source_paint_preferences.options;
        self.source_modal.observation = serde_json::json!({"radius":s.radius,"intensity":s.intensity,"picker":s.picker,
            "paint_targets":{"rgb":targets.color,"roughness":targets.roughness,"metallic":targets.metallic,
                "words":[targets.color[0].to_bits(),targets.color[1].to_bits(),targets.color[2].to_bits(),targets.roughness.to_bits(),targets.metallic.to_bits()]},
            "epoch":s.epoch.to_string(),"ordinal":s.ordinal,"primary_down":s.primary_down,"suppressed_contact":s.suppress_contact,
            "focus_contact_recovery":s.focus_contact_recovery,
            "ordered_window_focus":s.ordered_window_focus,
            "suppressed_frame":s.suppress_frame,"event_focused":self.source_modal.event_focused,"pending_queries":s.queries.rows().len(),"availability":self.source_modal.availability,
            "admission":self.source_modal.admission_progress,"last_stop":s.last_stop,"last_failure":self.source_modal.last_failure,
            "publication":self.source_modal.publication.as_ref().map(|p|serde_json::json!({"revision":p.revision.to_string(),"subtool":p.subtool,"level":p.level,"pose_words":p.pose})),
            "last_sample":self.source_modal.last_sample,"budgets":{"admission_operations":16384,"face_tests":8192,"triangle_tests":8192,"corner_reads":1024,"node_certificates":4096,"index_bytes":67108864,"combined_bytes":134217728,"dense_faces":obrush_core::MAX_SOURCE_PICK_INDEX_FACES,"events":256,"ledger_bytes":65536},
            "scope":"active-visible Source Paint; sealed publication; bounded small-linear or indexed dense query; Egui f32 coordinates lifted; dense local-origin/direction limits; physical latency, Source PNG cursor and selection parity unverified"});
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{SourcePickSnapshotAdmission, SourcePickWorkBudget};
    fn sample() -> obrush_core::SourceAttributePickSample {
        obrush_core::SourceAttributePickSample {
            rgb: [0.25, 0.5, 0.75],
            roughness: 0.125,
            metallic: 0.875,
            sampled_unprotected_mask: 0.0,
        }
    }
    #[test]
    fn s03_all_five_targets_are_replaced_together_and_mask_not_owned() {
        let p = SourcePaintPreferences::default();
        let n = paint_candidate(&p, sample()).unwrap();
        assert_eq!(n.options.color, [0.25, 0.5, 0.75]);
        assert_eq!(n.options.roughness, 0.125);
        assert_eq!(n.options.metallic, 0.875);
        assert_eq!(n.options.intensity.to_bits(), p.options.intensity.to_bits());
        assert_eq!(n.options.write_rgb, p.options.write_rgb);
    }
    #[test]
    fn s05_write_toggles_and_zero_intensity_do_not_block_sampling() {
        let mut p = SourcePaintPreferences::default();
        p.options.write_rgb = false;
        p.options.write_roughness = false;
        p.options.write_metallic = false;
        p.options.intensity = 0.;
        let n = paint_candidate(&p, sample()).unwrap();
        assert_eq!(n.options.color, sample().rgb);
        assert!(!n.options.write_rgb);
        assert_eq!(n.options.intensity, 0.);
    }
    #[test]
    fn s07_defined_zero_sample_is_eligible() {
        let s = obrush_core::SourceAttributePickSample {
            rgb: [0.; 3],
            roughness: 0.,
            metallic: 0.,
            sampled_unprotected_mask: 0.,
        };
        assert_eq!(
            paint_candidate(&SourcePaintPreferences::default(), s)
                .unwrap()
                .options
                .color
                .map(f32::to_bits),
            [0; 3]
        );
    }
    #[test]
    fn s08_out_of_domain_candidate_is_atomic() {
        let p = SourcePaintPreferences::default();
        let before = words(&p);
        let mut s = sample();
        s.metallic = 1.01;
        assert!(paint_candidate(&p, s).is_err());
        assert_eq!(words(&p), before);
    }
    #[test]
    fn s10_sequential_sample_candidates_use_new_complete_owner() {
        let p = SourcePaintPreferences::default();
        let a = paint_candidate(&p, sample()).unwrap();
        let mut s = sample();
        s.rgb = [0.5, 0.25, 0.125];
        let b = paint_candidate(&a, s).unwrap();
        assert_eq!(b.options.color, s.rgb);
        assert_eq!(b.options.roughness, s.roughness);
        assert_ne!(words(&a), words(&b));
    }
    #[test]
    fn m18_primary_exposed_tool_roster_is_exact() {
        for kind in [
            BrushKind::Twist,
            BrushKind::LocalScale,
            BrushKind::Drag,
            BrushKind::Morph,
        ] {
            assert!(!intensity_exposed(kind));
        }
        for kind in [
            BrushKind::Move,
            BrushKind::Smooth,
            BrushKind::Mask,
            BrushKind::Draw,
            BrushKind::Paint,
        ] {
            assert!(intensity_exposed(kind));
        }
    }
    #[test]
    fn owner_words_retain_signed_zero_and_all_target_channels() {
        let mut p = SourcePaintPreferences::default();
        let old = words(&p);
        p.options.color[0] = -0.;
        let zero = words(&p);
        p.options.color[0] = 0.;
        assert_ne!(zero, words(&p));
        assert_ne!(old, zero);
    }
    #[test]
    fn p02_idle_candidates_leave_real_document_and_prior_redo_unchanged() {
        let mut doc = Document::new(Mesh::sphere(0));
        doc.replace_mesh(Mesh::cube()).unwrap();
        let redo_endpoint = doc.to_json().unwrap();
        assert!(doc.undo());
        let before = doc.to_json().unwrap();
        let mut s = ModalState {
            last_pointer: Some([200., 100.]),
            ..Default::default()
        };
        s.start(Modal::Radius, Some(50.), true);
        assert_eq!(s.move_pointer([180., 140.], None, true).radius, Some(50.));
        s.end(Modal::Radius);
        let candidate = paint_candidate(&SourcePaintPreferences::default(), sample()).unwrap();
        assert_eq!(candidate.options.color, sample().rgb);
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(doc.redo());
        assert_eq!(doc.to_json().unwrap(), redo_endpoint);
    }
    #[test]
    fn b07_valid_then_complete_miss_then_keyup_retains_last_targets() {
        let mesh=Mesh::import_obj(b"v -1 0 0 0.5 0 0\nv 0 -1 0 0.0000000298023223876953125 0 0\nv 1 0 0 0.0000000298023223876953125 0 0\nv 0 1 0 0 0 0\nf 1 2 3 4\n").unwrap().mesh;
        let mut admission = SourcePickSnapshotAdmission::new(Arc::new(mesh)).unwrap();
        while !admission.step(16384).unwrap().ready {}
        let ready = admission.finish().unwrap();
        let mut budget = SourcePickWorkBudget::default();
        let mut state = ModalState {
            last_pointer: Some([0., 0.]),
            ..Default::default()
        };
        state.start(Modal::Picker, None, true);
        state.move_pointer([0., 0.], None, false);
        state.move_pointer([3., 3.], None, false);
        state.end(Modal::Picker);
        let mut preferences = SourcePaintPreferences::default();
        let mut accepted = 0;
        for q in state.queries.rows() {
            let ray = obrush_core::SceneRay64 {
                origin: glam::DVec3::new(q.screen[0], q.screen[1], 2.),
                direction: -glam::DVec3::Z,
            };
            if let Some(hit) = ready.borrow().ray(None, ray, &mut budget).unwrap() {
                let r = ready
                    .borrow()
                    .sample_with_budget(
                        SourceOriginalFaceContact {
                            face_index: hit.original_face_index.unwrap(),
                            local_point: hit.local_position,
                        },
                        &mut budget,
                    )
                    .unwrap()
                    .unwrap();
                preferences = paint_candidate(&preferences, r.sample).unwrap();
                accepted += 1;
            }
        }
        assert_eq!(accepted, 1);
        assert!(!state.picker);
        assert_eq!(preferences.options.color[0].to_bits(), 0x3e000000);
    }
}

#[cfg(test)]
#[path = "source_modal_profile_tests.rs"]
mod profile_regressions;

#[cfg(test)]
#[path = "source_modal_outside_release_tests.rs"]
mod outside_release_regressions;

#[cfg(test)]
#[path = "source_modal_crossframe_focus_tests.rs"]
mod crossframe_focus_regressions;
