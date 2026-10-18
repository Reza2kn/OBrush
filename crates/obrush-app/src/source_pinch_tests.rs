//! Shared normal Pinch session/queue and real numeric widgets; browser UNRUN.
use super::super::source_geometry_ui::{self, GeometryCommit, GeometryEditorDrafts};
use super::GeometryPreferenceStore;
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::{
    self as paint, PaintMode, RawPressureSample, SourcePaintPreferences, SourcePaintSession,
};
use eframe::egui::{self, Pos2, Rect};
use obrush_core::*;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

fn doc() -> Document {
    let mut m = Mesh::import_obj(
        b"v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    )
    .unwrap()
    .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.colors.fill([1.0; 3]);
    Document::new(m)
}
fn prefs() -> SourceGeometryPreferences {
    SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius: 25.0,
        radius_factor: 0.0,
        options: SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::pinch()
        },
        ..SourceGeometryPreferences::pinch()
    }
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Pinch,
        symmetry_x: false,
        adaptive_detail: false,
        alpha: None,
        ..Default::default()
    }
}
fn input() -> host::SourceGeometryInput {
    host::SourceGeometryInput {
        view: PaintViewCapture {
            world_to_clip: [
                0.25, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [56.25, 50.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn tracks() -> host::SharedSourceGeometryAlphaTracks {
    Arc::new(Mutex::new(Default::default()))
}

#[test]
fn saved_pinch_source_defaults_leave_three_old_tools_and_legacy_words_exact() {
    let mut s = GeometryPreferenceStore::default();
    let old = [
        s.get(BrushKind::Draw).unwrap(),
        s.get(BrushKind::Clay).unwrap(),
        s.get(BrushKind::Inflate).unwrap(),
    ];
    let p = s.get(BrushKind::Pinch).unwrap();
    assert_eq!(p.mode, SourceGeometryMode::Legacy);
    assert_eq!(p.radius, 50.0);
    assert_eq!(p.radius_factor, 0.75);
    assert_eq!(p.options.tool, SourceGeometryTool::Pinch);
    assert_eq!(p.options.intensity, 0.75);
    assert!(!p.options.negative && !p.options.culling);
    s.set(BrushKind::Pinch, prefs()).unwrap();
    for (i, k) in [BrushKind::Draw, BrushKind::Clay, BrushKind::Inflate]
        .into_iter()
        .enumerate()
    {
        assert_eq!(s.get(k).unwrap(), old[i]);
    }
    assert_eq!(
        super::super::initial_brush_settings().strength.to_bits(),
        0.35f32.to_bits()
    );
    let pinch = s.get(BrushKind::Pinch).unwrap();
    let smooth = s.get(BrushKind::Smooth).unwrap();
    assert_eq!(smooth.mode, SourceGeometryMode::Legacy);
    assert_eq!(smooth.radius, 50.0);
    assert_eq!(smooth.radius_factor, 0.75);
    assert_eq!(smooth.options, SourceGeometryOptions::smooth());
    assert_eq!(smooth.options.intensity, 0.75);
    assert_eq!(smooth.options.intensity_factor, 0.0);
    assert!(!smooth.options.negative && !smooth.options.culling);
    let changed = SourceGeometryPreferences {
        radius: 64.0,
        options: SourceGeometryOptions {
            tool: SourceGeometryTool::Smooth { tangent: true },
            intensity: 0.375,
            ..smooth.options
        },
        ..smooth
    };
    s.set(BrushKind::Smooth, changed).unwrap();
    assert_eq!(s.get(BrushKind::Smooth).unwrap(), changed);
    assert_eq!(s.get(BrushKind::Pinch).unwrap(), pinch);
    for (i, k) in [BrushKind::Draw, BrushKind::Clay, BrushKind::Inflate]
        .into_iter()
        .enumerate()
    {
        assert_eq!(s.get(k).unwrap(), old[i]);
    }
}
#[test]
fn new_controls_admit_only_typed_pinch_and_old_constructor_keeps_refusal() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    assert!(host::blocker_controls(&d, &settings(), false, prefs()).is_ok());
    assert_eq!(
        host::blocker(&d, &settings(), false, prefs()).unwrap_err(),
        SourceGeometryError::UnsupportedDomain
    );
    assert!(SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).is_err());
    let mut wrong = settings();
    wrong.kind = BrushKind::Draw;
    assert!(host::blocker_controls(&d, &wrong, false, prefs()).is_err());
    let mut wrong = settings();
    wrong.invert = true;
    assert!(host::blocker_controls(&d, &wrong, false, prefs()).is_err());
    assert!(host::blocker_controls(&d, &settings(), true, prefs()).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    assert!(!d.can_undo());
}
#[test]
fn ordinary_queue_capture_has_no_initial_dab_then_one_useful_release_and_history() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(),
        input(),
        tracks(),
    )
    .unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    assert!(!d.can_undo());
    let mut q = DabQueue::default();
    for _ in 0..200 {
        q.source_geometry_dab(s.clone(), input());
    }
    let p = q.release_source_geometry(&mut d);
    assert!(p.changed && p.error.is_none());
    assert_eq!(p.dabs, 1);
    assert!(q.is_empty());
    s.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(
        d.active_mesh().unwrap().positions[0].x.to_bits(),
        0x3c730000
    );
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert_eq!(
        host::apply_shared(&mut d, &s, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
}
#[test]
fn captured_negative_culling_radius_and_alpha_survive_caller_edits() {
    let mut d = doc();
    let mut p = prefs();
    p.options.negative = true;
    let mut b = settings();
    b.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    let s = SourceGeometrySession::begin_with_controls(&mut d, &b, false, p, input(), tracks())
        .unwrap();
    p.options.negative = false;
    p.options.culling = true;
    p.radius = 100.0;
    p.options.intensity = 0.0;
    b.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
    b.symmetry_x = true;
    assert!(p.options.culling && !p.options.negative && b.symmetry_x);
    assert!(!b.alpha.unwrap().has_signal());
    let r = host::apply_shared(&mut d, &s, input()).unwrap();
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(
        d.active_mesh().unwrap().positions[0].x.to_bits(),
        (-(81.0f64 / 10880.0) as f32).to_bits()
    );
    let lock = s.lock().unwrap();
    assert!(lock.preferences().options.negative);
    assert_eq!(lock.preferences().radius, 25.0);
    assert_eq!(lock.preferences().options.intensity, 0.625);
    drop(lock);
    s.lock().unwrap().finish(&mut d).unwrap();
}
#[test]
fn candidate_barrier_drains_old_pinch_before_new_saved_negative() {
    let mut d = doc();
    let old = prefs();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        old,
        input(),
        tracks(),
    )
    .unwrap();
    let mut q = DabQueue::default();
    q.source_geometry_dab(s.clone(), input());
    let mut next = old;
    next.options.negative = true;
    next.mirror_offset = 0.25;
    let calls = Cell::new(0);
    let c = source_geometry_ui::commit_candidate(old, next, || {
        calls.set(calls.get() + 1);
        let p = q.release_source_geometry(&mut d);
        assert!(p.error.is_none());
        s.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    let GeometryCommit::Accepted(accepted) = c else {
        panic!("Normal old-setting release must succeed")
    };
    assert!(accepted.options.negative);
    assert_eq!(accepted.mirror_offset, 0.25);
    assert_eq!(calls.get(), 1);
    assert!(q.is_empty());
    assert_eq!(
        d.active_mesh().unwrap().positions[0].x.to_bits(),
        0x3c730000
    );
    assert!(!s.lock().unwrap().preferences().options.negative);
    let mut invalid = next;
    invalid.options.intensity = f64::NAN;
    assert!(
        source_geometry_ui::commit_candidate(next, invalid, || {
            calls.set(calls.get() + 1);
            true
        })
        .is_err()
    );
    assert_eq!(calls.get(), 1);
    assert!(matches!(
        source_geometry_ui::commit_candidate(old, next, || false).unwrap(),
        GeometryCommit::ReleaseRejected
    ));
}
#[test]
fn invalid_after_useful_pinch_restores_project_track_owner_and_real_redo() {
    let mut d = doc();
    d.begin_stroke();
    d.name = "real future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let global = tracks();
    let prior = serde_json::to_value(global.lock().unwrap().snapshot()).unwrap();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(),
        input(),
        global.clone(),
    )
    .unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &s, input())
            .unwrap()
            .changed_vertices,
        1
    );
    let mut bad = input();
    bad.view.viewport[2] = 0.0;
    assert!(host::apply_shared(&mut d, &s, bad).is_err());
    s.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(
        serde_json::to_value(global.lock().unwrap().snapshot()).unwrap(),
        prior
    );
    assert!(global.lock().unwrap().active_claim().is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}
#[test]
fn no_contact_and_foreign_document_do_not_consume_live_owned_session() {
    let mut owner = doc();
    let mut foreign = doc();
    let global = tracks();
    let s = SourceGeometrySession::begin_with_controls(
        &mut owner,
        &settings(),
        false,
        prefs(),
        input(),
        global.clone(),
    )
    .unwrap();
    let claim = global.lock().unwrap().active_claim();
    assert_eq!(
        host::apply_shared(&mut foreign, &s, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert!(s.lock().unwrap().failed.is_none());
    assert_eq!(
        s.lock().unwrap().cancel(&mut foreign).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(global.lock().unwrap().active_claim(), claim);
    let before = owner.to_json().unwrap();
    let frame = serde_json::to_value(global.lock().unwrap().snapshot()).unwrap();
    let mut none = input();
    none.raw = RawPressureSample {
        pressure: 0.0,
        pen: true,
        browser_number: true,
        absent: false,
    };
    none.in_contact = false;
    let r = host::apply_shared(&mut owner, &s, none).unwrap();
    assert_eq!(r.noop, Some(SourceGeometryNoop::NoContact));
    assert_eq!(owner.to_json().unwrap(), before);
    assert_eq!(
        serde_json::to_value(global.lock().unwrap().snapshot()).unwrap(),
        frame
    );
    assert!(s.lock().unwrap().last_pass.is_none());
    assert_eq!(
        host::apply_shared(&mut owner, &s, input())
            .unwrap()
            .changed_vertices,
        1
    );
    s.lock().unwrap().finish(&mut owner).unwrap();
    assert!(global.lock().unwrap().active_claim().is_none());
}
#[test]
fn source_pinch_numeric_rows_are_visible_narrow_wide_and_idle_exact() {
    for width in [160.0, 280.0] {
        let ctx = egui::Context::default();
        super::super::configure_theme(&ctx);
        let mut drafts = GeometryEditorDrafts::default();
        let p = prefs();
        for _ in 0..3 {
            let mut result = None;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(width + 16.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.set_width(width);
                        let out = source_geometry_ui::render(ui, p, &mut drafts);
                        result = Some((out, ui.clip_rect()));
                    });
                },
            );
            let (out, clip) = result.unwrap();
            assert!(out.candidate.is_none());
            assert_eq!(out.controls.len(), 8);
            for r in out.controls.values() {
                assert!(r.enabled());
                assert!(r.interact_rect.is_positive());
                assert!(clip.contains_rect(r.interact_rect));
            }
            assert_eq!(p.options.intensity, 0.625);
        }
    }
}
#[test]
fn actual_pinch_warm_producer_is_reused_by_normal_paint_queue_and_history() {
    let mut d = doc();
    let initial = d.to_json().unwrap();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(),
        input(),
        tracks(),
    )
    .unwrap();
    host::apply_shared(&mut d, &s, input()).unwrap();
    s.lock().unwrap().finish(&mut d).unwrap();
    let before_paint = d.to_json().unwrap();
    let id = d.active_subtool_id().unwrap();
    let warm = d.source_geometry_state(id).unwrap();
    let paint_prefs = SourcePaintPreferences {
        mode: PaintMode::SourceOrdinary,
        radius: 25.0,
        radius_factor: 0.0,
        mirror_offset: 0.0,
        options: PaintResponseOptions {
            intensity: 0.5,
            hardness: 1.0,
            color: [0.0, 0.5, 1.0],
            culling: false,
            write_rgb: true,
            write_roughness: false,
            write_metallic: false,
            ..Default::default()
        },
    };
    let paint_settings = BrushSettings {
        kind: BrushKind::Paint,
        symmetry_x: false,
        adaptive_detail: false,
        alpha: None,
        ..Default::default()
    };
    let painter =
        SourcePaintSession::prepare(&d, &paint_prefs, &paint_settings, false, input()).unwrap();
    assert!(Arc::ptr_eq(
        &warm,
        &painter.lock().unwrap().retained_normal_state().unwrap()
    ));
    let mut q = DabQueue::default();
    q.begin();
    q.source_paint_dab(painter.clone(), input());
    let r = q.process(&mut d, 1, None);
    assert!(r.changed && r.painted && r.error.is_none());
    q.release(&mut d);
    assert!(q.is_empty());
    assert!(Arc::ptr_eq(&warm, &d.source_geometry_state(id).unwrap()));
    assert_eq!(
        painter.lock().unwrap().diagnostics()["cold_fresh_producer"],
        false
    );
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_paint);
    assert!(Arc::ptr_eq(&warm, &d.source_geometry_state(id).unwrap()));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), initial);
    // Source geometry Undo refreshes the captured rows, retaining a usable
    // current normal producer even when no producer existed before the stroke.
    let restored = d.source_geometry_state(id).unwrap();
    let initial_field = SourceNormalField::compute(
        d.require_active_mesh().unwrap(),
        SourceNormalPolicy::StaticOriginalFaces,
    )
    .unwrap();
    assert_eq!(
        restored
            .vertex_normals()
            .iter()
            .map(|n| n.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        initial_field
            .vertex_normals()
            .iter()
            .map(|n| n.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), before_paint);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(paint::apply_shared(&mut d, &painter, input()).is_err());
}
