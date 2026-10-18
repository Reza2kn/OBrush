//! Actual shared Source session -> bounded queue -> Document history contracts.
//! These are CPU/egui paths, not claimed OBrushApp/GL/browser executions.
use super::source_geometry_app::GeometryPreferenceStore;
use super::source_geometry_ui::{GeometryCommit, commit_candidate};
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as geometry, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::{RawPressureSample, SourcePaintInput};
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, Mesh, PaintViewCapture,
    SourceGeometryError, SourceGeometryOptions, SourceGeometryPass,
};
fn doc() -> Document {
    let mut m = Mesh::from_obj("v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    Document::new(m)
}
fn input() -> SourcePaintInput {
    SourcePaintInput {
        view: PaintViewCapture {
            world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        },
        screen: [50., 50.],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Inflate,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    }
}
fn prefs() -> SourceGeometryPreferences {
    SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius: 75.,
        radius_factor: 0.,
        ..SourceGeometryPreferences::inflate()
    }
}
#[test]
fn per_tool_source_options_do_not_change_legacy_or_other_tool_defaults() {
    let mut store = GeometryPreferenceStore::default();
    let draw = store.get(BrushKind::Draw).unwrap();
    let mut inflate = store.get(BrushKind::Inflate).unwrap();
    inflate.mode = SourceGeometryMode::SourceOrdinary;
    inflate.options.intensity = 0.875;
    store.set(BrushKind::Inflate, inflate).unwrap();
    assert_eq!(store.get(BrushKind::Draw).unwrap(), draw);
    assert_eq!(
        store.get(BrushKind::Clay).unwrap().mode,
        SourceGeometryMode::Legacy
    );
    assert_eq!(
        super::initial_brush_settings().strength.to_bits(),
        0.35f32.to_bits()
    );
    assert_eq!(
        store.get(BrushKind::Inflate).unwrap().options.intensity,
        0.875
    );
}
#[test]
fn source_queue_release_owns_one_transaction_and_has_no_generic_end_tail() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let session =
        SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).unwrap();
    let mut queue = DabQueue::default();
    for _ in 0..200 {
        queue.source_geometry_dab(session.clone(), input());
    }
    let p = queue.release_source_geometry(&mut d);
    assert_eq!(p.dabs, 1);
    assert!(p.changed);
    assert!(queue.is_empty());
    session.lock().unwrap().finish(&mut d).unwrap();
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert_eq!(queue.metrics()["pending_dabs"], 0);
    assert_eq!(queue.metrics()["peak_pending_dabs"], 32);
    assert_eq!(queue.metrics()["released_strokes"], 1);
}
#[test]
fn actual_numeric_candidate_releases_old_captured_geometry_then_installs_new_prefs() {
    let mut d = doc();
    let old = prefs();
    let session = SourceGeometrySession::begin(&mut d, &settings(), false, old, input()).unwrap();
    let mut queue = DabQueue::default();
    queue.source_geometry_dab(session.clone(), input());
    let before = d.to_json().unwrap();
    let mut candidate = old;
    candidate.options.intensity = 0.;
    let commit = commit_candidate(old, candidate, || {
        let p = queue.release_source_geometry(&mut d);
        assert!(p.changed);
        session.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    let GeometryCommit::Accepted(new) = commit else {
        panic!("valid released intent must install")
    };
    assert_eq!(new.options.intensity, 0.);
    assert_eq!(session.lock().unwrap().preferences().options.intensity, 0.3);
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn failed_in_stroke_input_cancels_accepted_work_and_preserves_original_complete_document() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let session =
        SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).unwrap();
    let mut queue = DabQueue::default();
    queue.source_geometry_dab(session.clone(), input());
    assert!(queue.process(&mut d, 1, None).changed);
    let mut invalid = input();
    invalid.raw.pressure = f64::NAN;
    queue.source_geometry_dab(session.clone(), invalid);
    assert!(queue.release_source_geometry(&mut d).error.is_some());
    session.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(queue.is_empty());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}
#[test]
fn warm_cache_only_queue_progress_is_visual_not_geometry_or_history() {
    let mut m =
        Mesh::from_obj("v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let mut d = Document::new(m);
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let p = SourceGeometryPass {
        center: [0.; 3],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    };
    let mut first = d.begin_source_geometry_stroke(o).unwrap();
    d.apply_source_geometry_dab(&mut first, p, 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&first).unwrap();
    let old = d.to_json().unwrap();
    let mut i = input();
    i.view.viewport = [0., 0., 1000., 1000.];
    i.view.world_to_clip = glam::DMat4::from_scale(glam::DVec3::new(0.1, 0.1, 1.)).to_cols_array();
    i.screen = [700., 500.];
    let mut pref = prefs();
    pref.radius = 100.;
    pref.options.intensity = 0.;
    let s = SourceGeometrySession::begin(&mut d, &settings(), false, pref, i).unwrap();
    let mut q = DabQueue::default();
    q.source_geometry_dab(s.clone(), i);
    let progress = q.release_source_geometry(&mut d);
    assert!(!progress.changed);
    assert!(progress.derived_cache_changed);
    s.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), old);
}
#[test]
fn unsupported_mirror_lock_and_adaptive_are_real_pre_begin_blockers() {
    for which in 0..3 {
        let d = doc();
        let mut s = settings();
        if which == 0 {
            s.symmetry_x = true;
        }
        if which == 1 {
            s.adaptive_detail = true;
        }
        assert!(geometry::blocker(&d, &s, which == 2, prefs()).is_err());
        assert!(!d.can_undo());
        assert_eq!(d.source_geometry_retained_bytes(), 0);
    }
}
#[test]
fn returned_to_release_cannot_accept_new_timer_or_queued_work() {
    let mut d = doc();
    let s = SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).unwrap();
    s.lock().unwrap().finish(&mut d).unwrap();
    let before = d.to_json().unwrap();
    assert_eq!(
        geometry::apply_shared(&mut d, &s, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}

#[test]
fn owned_source_panel_does_not_repeat_full_preflight_but_idle_panel_does() {
    let mut d = doc();
    let session =
        SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).unwrap();
    let mut calls = 0;
    for _ in 0..128 {
        super::source_geometry_ui::panel_preflight(Some(&session), || {
            calls += 1;
            geometry::blocker(&d, &settings(), false, prefs()).map_err(|e| e.to_string())
        })
        .unwrap();
    }
    assert_eq!(
        calls, 0,
        "An owned Source stroke must not scan its mesh just to redraw its panel"
    );
    super::source_geometry_ui::panel_preflight(None, || {
        calls += 1;
        geometry::blocker(&d, &settings(), false, prefs()).map_err(|e| e.to_string())
    })
    .unwrap();
    assert_eq!(calls, 1);
    session.lock().unwrap().finish(&mut d).unwrap();
}

#[test]
fn display_preflight_skip_never_authorizes_invalid_or_stale_source_dabs() {
    for edit in 0..3 {
        let mut d = doc();
        let session =
            SourceGeometrySession::begin(&mut d, &settings(), false, prefs(), input()).unwrap();
        session.lock().unwrap().apply(&mut d, input()).unwrap();
        let id = d.active_subtool_id().unwrap();
        let state = d.source_geometry_state(id).unwrap();
        let m = d.mesh.as_mut().unwrap();
        match edit {
            0 => m.colors[0][0] = f32::from_bits(0x7fc01234),
            1 => m.positions[0].x = f32::from_bits(0x7fc05678),
            _ => m.positions[0].x += 0.01,
        }
        let positions: Vec<_> = m
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect();
        let colors: Vec<_> = m.colors.iter().map(|p| p.map(f32::to_bits)).collect();
        super::source_geometry_ui::panel_preflight(Some(&session), || {
            panic!("Display-only redundant preflight")
        })
        .unwrap();
        assert!(session.lock().unwrap().apply(&mut d, input()).is_err());
        assert_eq!(
            d.mesh
                .as_ref()
                .unwrap()
                .positions
                .iter()
                .map(|p| p.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            positions
        );
        assert_eq!(
            d.mesh
                .as_ref()
                .unwrap()
                .colors
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect::<Vec<_>>(),
            colors
        );
        assert!(std::sync::Arc::ptr_eq(
            &state,
            &d.source_geometry_state(id).unwrap()
        ));
        assert!(
            super::source_geometry_ui::panel_preflight(None, || geometry::blocker(
                &d,
                &settings(),
                false,
                prefs()
            )
            .map_err(|e| e.to_string()))
            .is_err()
        );
    }
}
