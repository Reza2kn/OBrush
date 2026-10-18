//! Independent whole-word equivalence, entry-boundary and structural validation
//! witnesses; no timing or runtime/reference parity inferred from counts.
use super::*;
use crate::{Document, MaskConvention};
use std::sync::Arc;
fn fixture() -> Mesh {
    let mut m=Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 0.25 0.75\nvt 0.5 1\nvt 0.5 0.5\nf 1/1 2/2 3/3\nf 3/4 5/5 4/6\n").unwrap().mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([1.; 3]);
    m
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.625,
        ..SourceGeometryOptions::inflate()
    }
}
fn pass(x: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0., 0.],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    }
}
fn warm() -> Document {
    let mut d = Document::new(fixture());
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, pass(0.), 0.5, true)
            .unwrap()
            .changed_vertices,
        1
    );
    d.end_source_geometry_stroke(&s).unwrap();
    d
}
fn cache_words(s: &SourceGeometryNormalState) -> serde_json::Value {
    serde_json::json!({"normals":s.vertex_normals().iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "faces":s.face_normals().iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"tree":s.ordered_tree_snapshot().unwrap(),"generation":s.generation()})
}
fn real_redo(d: &mut Document) -> String {
    d.begin_stroke();
    d.name = "real future state".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    future
}
#[test]
fn each_warm_document_call_full_checks_entry_once_without_cross_dab_reuse() {
    let mut d = warm();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
        .unwrap();
    for _ in 0..2 {
        validation_probe::reset();
        d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
            .unwrap();
        assert_eq!(validation_probe::counts(), [1, 1, 1]);
    }
    d.cancel_source_geometry_stroke(&s).unwrap();
}
#[test]
fn document_and_standalone_inflate_keep_literal_whole_words_proxy_cache_report() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let mut standalone = d.require_active_mesh().unwrap().snapshot();
    let mut a = SourceGeometryStroke::new(options()).unwrap();
    let mut b = d.begin_source_geometry_stroke(options()).unwrap();
    let (ra, next) = a
        .apply(&mut standalone, &state, pass(4.), 0.5, true)
        .unwrap();
    let rb = d
        .apply_source_geometry_dab(&mut b, pass(4.), 0.5, true)
        .unwrap();
    assert_eq!(ra.changed_vertices, 1);
    assert_eq!(standalone.positions[1].z.to_bits(), 0.125f32.to_bits());
    assert_eq!(
        serde_json::to_vec(&standalone).unwrap(),
        serde_json::to_vec(d.require_active_mesh().unwrap()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&ra).unwrap(),
        serde_json::to_vec(&rb).unwrap()
    );
    assert_eq!(a.proxy_positions(), b.proxy_positions());
    assert_eq!(
        cache_words(&next.unwrap()),
        cache_words(&d.source_geometry_state(1).unwrap())
    );
    d.end_source_geometry_stroke(&b).unwrap();
}
#[test]
fn general_ordered_clay_document_standalone_and_independent_plane_words_match() {
    let mut m =
        Mesh::from_obj(include_str!("../tests/fixtures/independent-octants104.obj")).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let before = m.snapshot();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let opts = SourceGeometryOptions {
        clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
        ..Default::default()
    };
    let p = SourceGeometryPass {
        center: [0.; 3],
        local_radius_squared: 1600.,
        ..pass(0.)
    };
    let mut a = SourceGeometryStroke::new(opts).unwrap();
    let mut d = Document::new(m.snapshot());
    let mut b = d.begin_source_geometry_stroke(opts).unwrap();
    let (ra, next) = a.apply(&mut m, &state, p, 0.5, true).unwrap();
    let rb = d.apply_source_geometry_dab(&mut b, p, 0.5, true).unwrap();
    assert_eq!(ra.changed_vertices, 156);
    assert!(ra.ordered_static_selection);
    assert_eq!(
        serde_json::to_vec(&ra).unwrap(),
        serde_json::to_vec(&rb).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&m).unwrap(),
        serde_json::to_vec(d.require_active_mesh().unwrap()).unwrap()
    );
    for (old, new) in before.positions.iter().zip(&m.positions) {
        let p = old.to_array().map(f64::from);
        let dist = ((p[0] * p[0] + p[1] * p[1]) + p[2] * p[2]).sqrt() / 40.;
        let q = dist * dist;
        let expected = if old.z == 4. {
            4f32
        } else {
            (-4. + 4. * (3. * q * q - 4. * q * dist + 1.)) as f32
        };
        assert_eq!(new.z.to_bits(), expected.to_bits());
    }
    assert_eq!(
        cache_words(&next.unwrap()),
        cache_words(&d.source_geometry_state(1).unwrap())
    );
    d.end_source_geometry_stroke(&b).unwrap();
}
#[test]
fn standalone_entry_and_public_picked_query_retain_their_full_checks() {
    let d = warm();
    let mut m = d.require_active_mesh().unwrap().snapshot();
    let state = d.source_geometry_state(1).unwrap();
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    let (_, next) = s.apply(&mut m, &state, pass(4.), 0.5, true).unwrap();
    let current = next.unwrap();
    validation_probe::reset();
    let _ = s.apply(&mut m, &current, pass(4.), 0.5, true).unwrap();
    assert_eq!(validation_probe::counts(), [1, 1, 1]);
    for _ in 0..2 {
        validation_probe::reset();
        state
            .picked_normal(
                d.require_active_mesh().unwrap(),
                SourceGeometryPass {
                    center: [0.5, 0.5, 0.09375],
                    ..pass(0.)
                },
            )
            .unwrap();
        assert_eq!(validation_probe::counts(), [0, 1, 0]);
    }
}
#[test]
fn public_position_or_original_face_edit_between_dabs_rejects_and_keeps_prior_redo() {
    for face in [false, true] {
        let mut d = warm();
        let future = real_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let mut s = d.begin_source_geometry_stroke(options()).unwrap();
        d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
            .unwrap();
        let m = d.require_active_mesh_mut().unwrap();
        if face {
            m.triangles[0] = [0, 2, 1];
            m.surface.as_mut().unwrap().faces[0] = [0, 2, 1, TRIANGLE_CORNER];
        } else {
            m.positions[2].z = 0.25;
        }
        let mutated = d.to_json().unwrap();
        validation_probe::reset();
        assert_eq!(
            d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::StaleNormalState
        );
        assert_eq!(validation_probe::counts(), [1, 1, 0]);
        assert_eq!(d.to_json().unwrap(), mutated);
        d.cancel_source_geometry_stroke(&s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}
#[test]
fn every_public_attribute_domain_edit_still_rejects_before_math() {
    for kind in 0..6 {
        let mut d = warm();
        let mut s = d.begin_source_geometry_stroke(options()).unwrap();
        d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
            .unwrap();
        let m = d.require_active_mesh_mut().unwrap();
        match kind {
            0 => m.normals[1].x += 0.125,
            1 => m.masks[1] = 0.5,
            2 => m.colors[1] = [0.5, 0.75, 1.],
            3 => m.materials = Some(vec![crate::DEFAULT_VERTEX_MATERIAL; 5]),
            4 => m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[0][0] = 0.25,
            _ => {
                m.surface
                    .as_mut()
                    .unwrap()
                    .uv
                    .as_mut()
                    .unwrap()
                    .face_indices[0][0] = 1
            }
        }
        let before = d.to_json().unwrap();
        validation_probe::reset();
        assert_eq!(
            d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(validation_probe::counts(), [1, 1, 1]);
        assert_eq!(d.to_json().unwrap(), before);
        d.cancel_source_geometry_stroke(&s).unwrap();
    }
}
#[test]
fn invalid_mesh_input_is_not_accepted_by_a_previous_successful_entry() {
    let mut d = warm();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
        .unwrap();
    d.require_active_mesh_mut().unwrap().colors[1][0] = f32::NAN;
    validation_probe::reset();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::InvalidMesh
    );
    assert_eq!(validation_probe::counts(), [1, 0, 0]);
    d.cancel_source_geometry_stroke(&s).unwrap();
}
#[test]
fn cold_no_contact_zero_and_all_protected_do_not_allocate_cache_or_drop_redo() {
    for kind in 0..3 {
        let mut m = fixture();
        if kind == 2 {
            m.masks.fill(0.);
        }
        let mut d = Document::new(m);
        let future = real_redo(&mut d);
        let before = d.to_json().unwrap();
        let mut o = options();
        if kind == 1 {
            o.intensity = 0.;
        }
        let mut s = d.begin_source_geometry_stroke(o).unwrap();
        assert_eq!(
            d.apply_source_geometry_dab(&mut s, pass(0.), 0.5, kind != 0)
                .unwrap()
                .changed_vertices,
            0
        );
        d.end_source_geometry_stroke(&s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.source_geometry_state(1).is_none());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}
#[test]
fn generation_mismatch_at_original_core_boundary_is_still_atomic() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    s.expected_generation = Some(state.generation() + 1);
    let before = d.to_json().unwrap();
    validation_probe::reset();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(validation_probe::counts(), [1, 1, 0]);
    assert_eq!(d.to_json().unwrap(), before);
    d.cancel_source_geometry_stroke(&s).unwrap();
}
#[test]
fn existing_state_limit_and_release_validations_are_not_bypassed() {
    let d = warm();
    let mut m = d.require_active_mesh().unwrap().snapshot();
    let state = d.source_geometry_state(1).unwrap();
    let before = serde_json::to_vec(&m).unwrap();
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    assert_eq!(
        s.apply_with_state_limit(&mut m, &state, pass(4.), 0.5, true, 0)
            .unwrap_err(),
        SourceGeometryError::StorageLimit
    );
    assert_eq!(serde_json::to_vec(&m).unwrap(), before);
    let mut d = warm();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    d.apply_source_geometry_dab(&mut s, pass(4.), 0.5, true)
        .unwrap();
    validation_probe::reset();
    d.end_source_geometry_stroke(&s).unwrap();
    // The end caller admits the exact retained mesh once; balancing reuses
    // that same-call proof while the mesh and domain checks remain active.
    assert_eq!(validation_probe::counts(), [1, 1, 2]);
}

#[test]
fn controlled_document_checks_current_mesh_state_and_domain_once_per_warm_dab() {
    let mut d = warm();
    let mut s = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    let controls = crate::SourceGeometryControls {
        alpha: None,
        mirror: false,
    };
    let mut tracks = crate::SourceGeometryAlphaTracks::default();
    let contacts = [Some(pass(4.)), None];
    tracks = d
        .apply_source_geometry_controlled_dab(&mut s, contacts, 0.5, true, &controls, tracks)
        .unwrap()
        .tracks;
    for _ in 0..2 {
        validation_probe::reset();
        tracks = d
            .apply_source_geometry_controlled_dab(&mut s, contacts, 0.5, true, &controls, tracks)
            .unwrap()
            .tracks;
        // Every public entry must recheck mutable public data; the same
        // exclusive entry must not rescan it again at the inner math boundary.
        assert_eq!(validation_probe::counts(), [1, 1, 1]);
    }
    d.cancel_source_geometry_stroke(&s).unwrap();
}

#[test]
fn controlled_state_exposes_only_exact_predecessor_selected_chunks() {
    let mut document = warm();
    let before = document.source_geometry_state(1).unwrap();
    let unrelated =
        Arc::new(SourceGeometryNormalState::seed(document.require_active_mesh().unwrap()).unwrap());
    let mut stroke = document
        .begin_source_geometry_selection_history(options())
        .unwrap();
    let result = document
        .apply_source_geometry_controlled_dab(
            &mut stroke,
            [Some(pass(4.)), None],
            0.5,
            true,
            &crate::SourceGeometryControls {
                alpha: None,
                mirror: false,
            },
            crate::SourceGeometryAlphaTracks::default(),
        )
        .unwrap();
    assert_eq!(result.combined.changed_vertices, 1);
    let after = document.source_geometry_state(1).unwrap();
    let chunks = after.render_selected_chunks_from(&before).unwrap();
    assert_eq!(chunks[0] & 1, 1);
    assert!(chunks.iter().skip(1).all(|word| *word == 0));
    assert!(after.render_selected_chunks_from(&unrelated).is_none());
    document.cancel_source_geometry_stroke(&stroke).unwrap();
}

#[test]
fn controlled_document_matches_standalone_whole_words_and_current_normal_history() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let mut m = d.require_active_mesh().unwrap().snapshot();
    let mut standalone = SourceGeometryStroke::new(options()).unwrap();
    standalone.history_selected = Some(Arc::new(Vec::new()));
    let mut owned = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    let controls = crate::SourceGeometryControls {
        alpha: None,
        mirror: false,
    };
    let tracks = crate::SourceGeometryAlphaTracks::default();
    let contacts = [Some(pass(4.)), None];
    let (a, next) = standalone
        .apply_controlled(
            &mut m,
            &state,
            contacts,
            0.5,
            true,
            &controls,
            tracks,
            crate::MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    let b = d
        .apply_source_geometry_controlled_dab(&mut owned, contacts, 0.5, true, &controls, tracks)
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&m).unwrap(),
        serde_json::to_vec(d.require_active_mesh().unwrap()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
    assert_eq!(
        cache_words(next.as_ref().unwrap()),
        cache_words(&d.source_geometry_state(1).unwrap())
    );
    assert_eq!(
        standalone.history_selected.as_deref(),
        owned.history_selected.as_deref()
    );
    d.cancel_source_geometry_stroke(&owned).unwrap();
}

#[test]
fn controlled_public_entry_rechecks_external_edits_between_dabs_without_mutation() {
    for mutation in 0..7 {
        let mut d = warm();
        let mut s = d
            .begin_source_geometry_selection_history(options())
            .unwrap();
        let controls = crate::SourceGeometryControls {
            alpha: None,
            mirror: false,
        };
        let contacts = [Some(pass(4.)), None];
        let tracks = d
            .apply_source_geometry_controlled_dab(
                &mut s,
                contacts,
                0.5,
                true,
                &controls,
                crate::SourceGeometryAlphaTracks::default(),
            )
            .unwrap()
            .tracks;
        let m = d.mesh.as_mut().unwrap();
        let expected = match mutation {
            0 => {
                m.positions[0].x = f32::from_bits(0x7fc0_0042);
                SourceGeometryError::InvalidMesh
            }
            1 => {
                m.positions[0].x += 0.125;
                SourceGeometryError::StaleNormalState
            }
            2 => {
                m.normals[0].x += 0.125;
                SourceGeometryError::StrokeBindingChanged
            }
            3 => {
                m.masks[0] = 0.5;
                SourceGeometryError::StrokeBindingChanged
            }
            4 => {
                m.colors[0][0] = 0.5;
                SourceGeometryError::StrokeBindingChanged
            }
            5 => {
                m.materials = Some(vec![[0.5, 0.25]; m.positions.len()]);
                SourceGeometryError::StrokeBindingChanged
            }
            _ => {
                m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[0][0] = 0.375;
                SourceGeometryError::StrokeBindingChanged
            }
        };
        let before = serde_json::to_vec(d.require_active_mesh().unwrap()).unwrap();
        let words = d
            .require_active_mesh()
            .unwrap()
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>();
        let state = d.source_geometry_state(1).unwrap();
        assert_eq!(
            d.apply_source_geometry_controlled_dab(&mut s, contacts, 0.5, true, &controls, tracks)
                .unwrap_err(),
            expected,
            "mutation{mutation}"
        );
        assert_eq!(
            serde_json::to_vec(d.require_active_mesh().unwrap()).unwrap(),
            before
        );
        assert_eq!(
            d.require_active_mesh()
                .unwrap()
                .positions
                .iter()
                .map(|p| p.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            words
        );
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
    }
}
