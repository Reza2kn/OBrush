//! Independent actual widget/captured-host/old-settings barrier contracts.
//! Source-only authoring is not browser or Source reference parity.
use super::*;
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::RawPressureSample;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, Mesh, PaintViewCapture,
    SourceClayOrderPolicy, SourceGeometryError, SourceGeometryOptions,
};
use std::cell::Cell;
const ORDERED: &str = "source_geometry.selection.ordered";
const SINGLE: &str = "source_geometry.selection.single_leaf";
fn store() -> super::super::source_geometry_app::GeometryPreferenceStore {
    Default::default()
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Clay,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    }
}
fn large() -> Document {
    let mut m = Mesh::from_obj(include_str!(
        "../../obrush-core/tests/fixtures/independent-octants104.obj"
    ))
    .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    Document::new(m)
}
fn prefs() -> SourceGeometryPreferences {
    let mut p = store().get(BrushKind::Clay).unwrap();
    p.mode = SourceGeometryMode::SourceOrdinary;
    p.radius = 250.;
    p.radius_factor = 0.;
    p
}
fn input() -> host::SourceGeometryInput {
    host::SourceGeometryInput {
        view: PaintViewCapture {
            world_to_clip: [
                0.125, 0., 0., 0., 0., 0.125, 0., 0., 0., 0., -0.1, 0., 0., 0., 0., 1.,
            ],
            viewport: [0., 0., 100., 100.],
        },
        screen: [25., 75.],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn frame(
    ctx: &egui::Context,
    width: f32,
    current: SourceClayOrderPolicy,
    events: Vec<Event>,
    time: f64,
) -> (GeometrySelectionUi, Rect) {
    let mut out = None;
    let mut clip = None;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(width + 16., 180.),
            )),
            events,
            time: Some(time),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(width);
                clip = Some(ui.clip_rect());
                out = Some(render_selection_order(ui, current));
            });
        },
    );
    (out.unwrap(), clip.unwrap())
}
fn click(pos: Pos2, pressed: bool) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        },
    ]
}

#[test]
fn app_default_source_brush_clay_is_ordered_but_modes_and_other_words_unchanged() {
    let s = store();
    let original = SourceGeometryPreferences::default();
    for kind in [BrushKind::Draw, BrushKind::Clay] {
        let p = s.get(kind).unwrap();
        assert_eq!(p.mode, SourceGeometryMode::Legacy);
        assert_eq!(
            p.options.clay_order,
            SourceClayOrderPolicy::OrderedStaticOctree
        );
        assert_eq!(p.radius.to_bits(), 50f64.to_bits());
        assert_eq!(p.radius_factor.to_bits(), 0.75f64.to_bits());
        assert_eq!(p.options.tool, original.options.tool);
        assert_eq!(p.options.intensity.to_bits(), 0.5f64.to_bits());
        assert_eq!(p.options.intensity_factor.to_bits(), 0f64.to_bits());
        assert!(!p.options.negative && !p.options.culling);
    }
    assert_eq!(
        s.get(BrushKind::Inflate).unwrap(),
        SourceGeometryPreferences::inflate()
    );
    assert_eq!(
        SourceGeometryOptions::default().clay_order,
        SourceClayOrderPolicy::SingleLeaf
    );
    assert_eq!(
        SourceGeometryPreferences::default().options.clay_order,
        SourceClayOrderPolicy::SingleLeaf
    );
}
#[test]
fn selection_is_per_tool_and_core_single_leaf_refusal_is_retained() {
    let mut s = store();
    let draw = s.get(BrushKind::Draw).unwrap();
    let inflate = s.get(BrushKind::Inflate).unwrap();
    let mut clay = s.get(BrushKind::Clay).unwrap();
    clay.options.clay_order = SourceClayOrderPolicy::SingleLeaf;
    s.set(BrushKind::Clay, clay).unwrap();
    assert_eq!(s.get(BrushKind::Draw).unwrap(), draw);
    assert_eq!(s.get(BrushKind::Inflate).unwrap(), inflate);
    assert_eq!(
        s.get(BrushKind::Clay).unwrap().options.clay_order,
        SourceClayOrderPolicy::SingleLeaf
    );
    assert_eq!(
        large()
            .source_geometry_blocker(SourceGeometryOptions::default())
            .unwrap_err(),
        SourceGeometryError::UnsupportedClayOrder
    );
}
#[test]
fn actual_order_controls_fit_narrow_and_wide_panels_without_idle_candidate() {
    for width in [160., 280.] {
        let ctx = egui::Context::default();
        super::super::configure_theme(&ctx);
        for policy in [
            SourceClayOrderPolicy::SingleLeaf,
            SourceClayOrderPolicy::OrderedStaticOctree,
        ] {
            let (out, clip) = frame(&ctx, width, policy, Vec::new(), 0.);
            assert!(out.candidate.is_none());
            assert_eq!(out.controls.len(), 2);
            for key in [ORDERED, SINGLE] {
                let r = &out.controls[key];
                assert!(r.enabled());
                assert!(r.interact_rect.is_positive());
                assert!(clip.contains_rect(r.interact_rect));
            }
        }
    }
}
#[test]
fn actual_pointer_press_release_changes_only_the_chosen_order() {
    for (from, key, to) in [
        (
            SourceClayOrderPolicy::SingleLeaf,
            ORDERED,
            SourceClayOrderPolicy::OrderedStaticOctree,
        ),
        (
            SourceClayOrderPolicy::OrderedStaticOctree,
            SINGLE,
            SourceClayOrderPolicy::SingleLeaf,
        ),
    ] {
        let ctx = egui::Context::default();
        super::super::configure_theme(&ctx);
        let (idle, _) = frame(&ctx, 180., from, Vec::new(), 0.);
        let center = idle.controls[key].interact_rect.center();
        let (pressed, _) = frame(&ctx, 180., from, click(center, true), 0.016);
        assert!(pressed.candidate.is_none());
        let (released, _) = frame(&ctx, 180., from, click(center, false), 0.032);
        assert_eq!(released.candidate, Some(to));
    }
}
#[test]
fn same_order_noop_avoids_release_and_changed_order_releases_exactly_once() {
    let current = prefs();
    let n = Cell::new(0);
    assert!(matches!(
        commit_candidate(current, current, || {
            n.set(n.get() + 1);
            true
        })
        .unwrap(),
        GeometryCommit::Unchanged
    ));
    assert_eq!(n.get(), 0);
    let mut candidate = current;
    candidate.options.clay_order = SourceClayOrderPolicy::SingleLeaf;
    assert!(!same_words(current, candidate));
    let accepted = commit_candidate(current, candidate, || {
        n.set(n.get() + 1);
        true
    })
    .unwrap();
    assert_eq!(n.get(), 1);
    let GeometryCommit::Accepted(p) = accepted else {
        panic!("Actual changed order needs a released preference candidate");
    };
    assert_eq!(p, candidate);
}
#[test]
fn rejected_release_preserves_saved_order_project_and_genuine_redo() {
    let mut d = large();
    d.begin_stroke();
    d.name = "real future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let mut s = store();
    let current = s.get(BrushKind::Clay).unwrap();
    let mut candidate = current;
    candidate.options.clay_order = SourceClayOrderPolicy::SingleLeaf;
    let calls = Cell::new(0);
    assert!(matches!(
        commit_candidate(current, candidate, || {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap(),
        GeometryCommit::ReleaseRejected
    ));
    assert_eq!(calls.get(), 1);
    assert_eq!(s.get(BrushKind::Clay).unwrap(), current);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
    // Re-applying the old saved value does not introduce a state change.
    s.set(BrushKind::Clay, current).unwrap();
    assert_eq!(s.get(BrushKind::Clay).unwrap(), current);
}
#[test]
fn actual_large_default_capture_queue_and_source_owned_release_make_useful_clay() {
    let mut d = large();
    let before = d.to_json().unwrap();
    let p = prefs();
    assert!(host::blocker(&d, &settings(), false, p).is_ok());
    let session = SourceGeometrySession::begin(&mut d, &settings(), false, p, input()).unwrap();
    assert_eq!(
        session.lock().unwrap().preferences().options.clay_order,
        SourceClayOrderPolicy::OrderedStaticOctree
    );
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), input());
    let r = q.process(&mut d, 1, None);
    assert!(r.error.is_none() && r.changed);
    assert_eq!(r.dabs, 1);
    q.release_source_geometry(&mut d);
    assert!(q.is_empty());
    session.lock().unwrap().finish(&mut d).unwrap();
    let report = session.lock().unwrap().last_report.clone().unwrap();
    assert!(report.ordered_static_selection);
    assert!(!report.clay_single_leaf);
    assert_eq!(report.selected_vertices, 312);
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.source_geometry_state(1).is_some());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    let mut single = p;
    single.options.clay_order = SourceClayOrderPolicy::SingleLeaf;
    assert_eq!(
        host::blocker(&large(), &settings(), false, single).unwrap_err(),
        SourceGeometryError::UnsupportedClayOrder
    );
}
#[test]
fn active_order_is_captured_until_old_settings_finish_before_new_install() {
    let mut d = large();
    let current = prefs();
    let session =
        SourceGeometrySession::begin(&mut d, &settings(), false, current, input()).unwrap();
    let mut candidate = current;
    candidate.options.clay_order = SourceClayOrderPolicy::SingleLeaf;
    assert_eq!(session.lock().unwrap().preferences(), current);
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), input());
    assert!(q.process(&mut d, 1, None).changed);
    assert_eq!(
        session.lock().unwrap().preferences().options.clay_order,
        SourceClayOrderPolicy::OrderedStaticOctree
    );
    let calls = Cell::new(0);
    let result = commit_candidate(current, candidate, || {
        calls.set(calls.get() + 1);
        q.release_source_geometry(&mut d);
        session.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
    let GeometryCommit::Accepted(next) = result else {
        panic!("Finished captured order should accept next saved preference");
    };
    assert_eq!(next.options.clay_order, SourceClayOrderPolicy::SingleLeaf);
    assert_eq!(session.lock().unwrap().preferences(), current);
    assert_eq!(
        host::blocker(&d, &settings(), false, next).unwrap_err(),
        SourceGeometryError::UnsupportedClayOrder
    );
    assert!(q.is_empty());
}
