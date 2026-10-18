//! Independent Smooth guarded transactions, exact attributes/history and warm producer.
use obrush_core::*;
use std::sync::Arc;

fn literal(text: &str, faces: &[[u32; 3]]) -> Mesh {
    let mut m = Mesh::import_obj(text.as_bytes()).unwrap().mesh;
    assert_eq!(m.triangles, faces);
    m.surface = Some(SurfaceTopology {
        faces: faces
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect(),
        uv: None,
    });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.colors.fill([1.0; 3]);
    m.validate().unwrap();
    m
}
fn tetra() -> Mesh {
    literal(
        include_str!("fixtures/independent-smooth-tetra.obj"),
        &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]],
    )
}
fn zero() -> Mesh {
    literal(
        include_str!("fixtures/independent-smooth-zero-two-fans.obj"),
        &[[0, 1, 2], [0, 4, 3]],
    )
}
fn options(intensity: f64, tangent: bool) -> SourceGeometryOptions {
    SourceGeometryOptions {
        tool: SourceGeometryTool::Smooth { tangent },
        intensity,
        ..SourceGeometryOptions::smooth()
    }
}
fn pass(center: [f64; 3], r2: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center,
        local_radius_squared: r2,
        eye_direction: [0.0, 0.0, -1.0],
        original_face_index: 0,
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    let mut v = serde_json::to_value(m).unwrap();
    v["raw_words"] = serde_json::json!({"p":m.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "n":m.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "u":m.masks.iter().map(|p|p.to_bits()).collect::<Vec<_>>(),
        "c":m.colors.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),
        "rm":m.materials.as_ref().map(|p|p.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>())});
    v
}
fn redo(d: &mut Document) -> String {
    d.begin_stroke();
    d.name = "genuine prior future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    future
}
fn single(m: &mut Mesh, o: SourceGeometryOptions, p: SourceGeometryPass) -> SourceGeometryReport {
    let state = SourceGeometryNormalState::seed(m).unwrap();
    let mut s = SourceGeometryStroke::new(o).unwrap();
    let (report, _) = s.apply(m, &state, p, 0.5, true).unwrap();
    assert!(s.proxy_positions().is_empty());
    report
}

#[test]
fn genuine_smooth_defaults_and_only_new_option_refusals_are_typed() {
    let o = SourceGeometryOptions::smooth();
    assert_eq!(o.tool, SourceGeometryTool::Smooth { tangent: false });
    assert_eq!(o.intensity, 0.75);
    assert_eq!(o.intensity_factor, 0.0);
    assert!(!o.negative && !o.culling);
    assert_eq!(o.clay_order, SourceClayOrderPolicy::OrderedStaticOctree);
    assert_eq!(
        SourceGeometryStroke::new(SourceGeometryOptions {
            negative: true,
            ..o
        })
        .err()
        .unwrap(),
        SourceGeometryError::UnsupportedSmoothNegative
    );
    assert_eq!(
        SourceGeometryStroke::new(SourceGeometryOptions {
            clay_order: SourceClayOrderPolicy::SingleLeaf,
            ..o
        })
        .err()
        .unwrap(),
        SourceGeometryError::UnsupportedSmoothOrder
    );
    for intensity in [f64::NAN, -0.1, 1.1] {
        assert!(SourceGeometryStroke::new(SourceGeometryOptions { intensity, ..o }).is_err());
    }
    assert!(SourceGeometryStroke::new(SourceGeometryOptions {
        negative: true,
        ..SourceGeometryOptions::pinch()
    })
    .is_ok());
}

#[test]
fn single_vertex_plain_and_tangent_raw_candidates_are_guard_rejected_with_real_redo() {
    for tangent in [false, true] {
        let mut d = Document::new(tetra());
        let future = redo(&mut d);
        let before = d.to_json().unwrap();
        let prior = words(d.require_active_mesh().unwrap());
        let mut s = d
            .begin_source_geometry_stroke(options(0.625, tangent))
            .unwrap();
        let report = d
            .apply_source_geometry_dab(&mut s, pass([0.0, 0.0, 1.0], 0.25), 0.5, true)
            .unwrap();
        assert_eq!(report.noop, Some(SourceGeometryNoop::GuardRejected));
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(report.displacement_scale, 0.0);
        assert_eq!(words(d.require_active_mesh().unwrap()), prior);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
        d.end_source_geometry_stroke(&s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}

#[test]
fn high_intensity_all_selected_projection_and_rounded_fold_guard_preserve_original() {
    let mut m = tetra();
    let before = words(&m);
    let report = single(&mut m, options(0.625, false), pass([0.0, 0.0, 1.0], 4.0));
    assert_eq!(report.selected_vertices, 4);
    assert_eq!(report.noop, Some(SourceGeometryNoop::GuardRejected));
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(report.displacement_scale, 0.0);
    assert_eq!(words(&m), before);
}

#[test]
fn lower_intensity_all_four_useful_full_scale_and_whole_cache_history() {
    let mut m = tetra();
    m.materials = Some(vec![[0.18, 0.08]; 4]);
    let attrs = (
        m.masks.clone(),
        m.colors.clone(),
        m.materials.clone(),
        m.surface.clone(),
        m.triangles.clone(),
    );
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_geometry_stroke(options(0.375, false))
        .unwrap();
    let r = d
        .apply_source_geometry_dab(&mut s, pass([0.0, 0.0, 1.0], 4.0), 0.5, true)
        .unwrap();
    assert_eq!(r.changed_vertices, 4);
    assert_eq!(r.displacement_scale, 1.0);
    assert!(r.noop.is_none());
    assert_eq!(
        d.require_active_mesh()
            .unwrap()
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        [
            [0x3e000000, 0, 0x3f200000],
            [0x3f200000, 0, 0x3e000000],
            [0x3e000000, 0x3f000000, 0x3e000000],
            [0x3e000000, 0xbf000000, 0x3e000000]
        ]
    );
    let m = d.require_active_mesh().unwrap();
    assert_eq!(
        (
            m.masks.clone(),
            m.colors.clone(),
            m.materials.clone(),
            m.surface.clone(),
            m.triangles.clone()
        ),
        attrs
    );
    d.end_source_geometry_stroke(&s).unwrap();
    let after = d.to_json().unwrap();
    let id = d.active_subtool_id().unwrap();
    let cache = d.source_geometry_state(id).unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.source_geometry_state(id).is_none());
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(Arc::ptr_eq(&cache, &d.source_geometry_state(id).unwrap()));
}

#[test]
fn two_prepared_overlap_passes_reaverage_live_positions_and_keep_one_history() {
    let mut d = Document::new(tetra());
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_geometry_stroke(options(0.375, false))
        .unwrap();
    let p = pass([0.0, 0.0, 1.0], 4.0);
    let r = d
        .apply_source_geometry_controlled_dab(
            &mut s,
            [Some(p), Some(p)],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 4);
    assert_eq!(r.contacts[0].unwrap().center, p.center);
    assert_eq!(r.contacts[1].unwrap().center, p.center);
    for p in r.passes.iter().flatten() {
        assert_eq!(p.displacement_scale, 1.0);
        assert_eq!(p.source_normal_rows_refreshed, 4);
    }
    assert_eq!(
        d.require_active_mesh()
            .unwrap()
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        [
            [0x3e400000, 0, 0x3ee00000],
            [0x3ee00000, 0, 0x3e400000],
            [0x3e400000, 0x3e800000, 0x3e400000],
            [0x3e400000, 0xbe800000, 0x3e400000]
        ]
    );
    d.end_source_geometry_stroke(&s).unwrap();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn zero_raw_normal_plain_is_useful_and_tangent_is_exact_noop() {
    for tangent in [false, true] {
        let mut m = zero();
        let before = words(&m);
        let r = single(&mut m, options(0.5, tangent), pass([0.0; 3], 0.25));
        if tangent {
            assert_eq!(r.changed_vertices, 0);
            assert_eq!(words(&m), before);
        } else {
            assert_eq!(r.changed_vertices, 1);
            assert_eq!(r.displacement_scale, 1.0);
            assert_eq!(
                m.positions[0].to_array().map(f32::to_bits),
                [0xbf800000, 0, 0]
            );
        }
    }
}

#[test]
fn no_radial_polynomial_or_extra_radius_amplitude_when_selection_matches() {
    let mut a = tetra();
    let mut b = tetra();
    let ra = single(&mut a, options(0.375, false), pass([0.0, 0.0, 1.0], 4.0));
    let rb = single(&mut b, options(0.375, false), pass([0.0, 0.0, 1.0], 9.0));
    assert_eq!(ra.selected_vertices, 4);
    assert_eq!(rb.selected_vertices, 4);
    assert_eq!(words(&a), words(&b));
}

#[test]
fn source_zero_normal_culling_is_inclusive_without_facing_fade() {
    let mut m = zero();
    let r = single(
        &mut m,
        SourceGeometryOptions {
            culling: true,
            ..options(0.5, false)
        },
        pass([0.0; 3], 0.25),
    );
    assert_eq!(r.front_vertices, 1);
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(m.positions[0].x.to_bits(), 0xbf800000);
}

#[test]
fn zero_protected_no_contact_and_pressure_curve_keep_real_redo_without_cache() {
    for mode in 0..4 {
        let mut m = tetra();
        if mode == 0 {
            m.masks.fill(0.0);
            m.positions[0].y = -0.0;
        }
        let mut d = Document::new(m);
        let future = redo(&mut d);
        let before = d.to_json().unwrap();
        let original = words(d.require_active_mesh().unwrap());
        let mut o = options(0.375, false);
        if mode == 1 {
            o.intensity = 0.0;
        }
        if mode == 3 {
            o.intensity_factor = 1.0;
        }
        let mut s = d.begin_source_geometry_stroke(o).unwrap();
        let r = d
            .apply_source_geometry_dab(
                &mut s,
                pass([0.0, 0.0, 1.0], 4.0),
                if mode == 3 { 0.0 } else { 0.5 },
                mode != 2,
            )
            .unwrap();
        assert_eq!(r.changed_vertices, 0);
        assert_eq!(words(d.require_active_mesh().unwrap()), original);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
        d.end_source_geometry_stroke(&s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}

#[test]
fn boundary_fallback_candidate_does_not_bypass_nonmanifold_guard() {
    let mut m = literal(
        include_str!("fixtures/independent-smooth-boundary-fallback.obj"),
        &[
            [0, 1, 2],
            [0, 2, 3],
            [1, 2, 3],
            [1, 2, 4],
            [1, 3, 4],
            [2, 3, 4],
        ],
    );
    let before = words(&m);
    let r = single(&mut m, options(0.5, false), pass([0.0, 0.0, 1.0], 0.25));
    assert_eq!(r.noop, Some(SourceGeometryNoop::GuardRejected));
    assert_eq!(words(&m), before);
}

#[test]
fn stale_positions_and_raw_domain_changes_reject_atomically() {
    let mut m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let mut s = SourceGeometryStroke::new(options(0.375, false)).unwrap();
    m.positions[0].x = 0.125;
    let edited = words(&m);
    assert_eq!(
        s.apply(&mut m, &state, pass([0.0, 0.0, 1.0], 4.0), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(words(&m), edited);
    let mut m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    s.apply(&mut m, &state, pass([0.0, 0.0, 1.0], 4.0), 0.0, false)
        .unwrap();
    m.surface.as_mut().unwrap().faces[0] = [0, 2, 1, TRIANGLE_CORNER];
    let edited = words(&m);
    assert!(s
        .apply(&mut m, &state, pass([0.0, 0.0, 1.0], 4.0), 0.5, true)
        .is_err());
    assert_eq!(words(&m), edited);
}

#[test]
fn warm_geometry_smooth_then_paint_uses_retained_selected_refresh_rows() {
    let m = literal(
        "v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n",
        &[[0, 1, 2], [2, 4, 3]],
    );
    let mut d = Document::new(m);
    let mut inflate = d
        .begin_source_geometry_stroke(SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        })
        .unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut inflate, pass([0.0; 3], 4.0), 0.5, true)
            .unwrap()
            .changed_vertices,
        1
    );
    d.end_source_geometry_stroke(&inflate).unwrap();
    let mut smooth = d
        .begin_source_geometry_stroke(options(0.375, false))
        .unwrap();
    let r = d
        .apply_source_geometry_dab(&mut smooth, pass([0.0, 4.0, 0.0], 1.0), 0.5, true)
        .unwrap();
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(r.displacement_scale, 1.0);
    d.end_source_geometry_stroke(&smooth).unwrap();
    let state = d
        .source_geometry_state(d.active_subtool_id().unwrap())
        .unwrap();
    assert_eq!(
        d.require_active_mesh().unwrap().positions[2],
        Vec3::new(0.0, 3.625, 0.01171875)
    );
    assert_eq!(state.vertex_normals()[2], Vec3::new(0.203125, 0.25, 16.0));
    assert_eq!(state.vertex_normals()[1], Vec3::new(0.0, 0.0, 16.0));
    let request = PaintResponseRequest {
        kind: BrushKind::Paint,
        operation: PaintResponseOperation::OrdinaryDab,
        in_contact: true,
        options: PaintResponseOptions {
            intensity: 0.5,
            hardness: 1.0,
            color: [0.0, 0.5, 1.0],
            write_rgb: true,
            write_roughness: false,
            write_metallic: false,
            culling: true,
            alpha: None,
            ..Default::default()
        },
        first: PaintResponsePass {
            center: [4.0, 0.0, 0.0],
            local_radius_squared: 1.0,
            eye_direction: [1.0, 0.0, -1.0 / 128.0],
            mirrored_x: false,
            alpha_frame: None,
        },
        mirrored: None,
    };
    let fresh = SourceNormalField::compute(
        d.require_active_mesh().unwrap(),
        SourceNormalPolicy::StaticOriginalFaces,
    )
    .unwrap();
    assert_eq!(
        fresh.vertex_normals()[1],
        Vec3::new(0.453125, 0.453125, 14.5)
    );
    let mut cold = d.require_active_mesh().unwrap().snapshot();
    assert_eq!(
        cold.apply_paint_response_with_source_normals(&request, &fresh)
            .unwrap()
            .changed_vertices,
        0
    );
    let before_positions = d.require_active_mesh().unwrap().positions.clone();
    let r = d
        .require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_normals(&request, &state)
        .unwrap();
    assert_eq!(r.first_selected_vertices, 1);
    assert_eq!(r.changed_rgb_vertices, 1);
    assert_eq!(r.changed_material_vertices, 0);
    assert_eq!(
        d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
        [0x3f000000, 0x3f400000, 0x3f800000]
    );
    assert_eq!(d.require_active_mesh().unwrap().positions, before_positions);
    assert!(Arc::ptr_eq(
        &state,
        &d.source_geometry_state(d.active_subtool_id().unwrap())
            .unwrap()
    ));
}

#[test]
fn smooth_defined_nearest_alpha_is_real_and_undefined_vertex_frame_still_rejects() {
    let controls = SourceGeometryControls {
        alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
        mirror: false,
    };
    let mut m = tetra();
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options(0.375, false)).unwrap();
    let contact = pass([0.25, 0.0, 0.75], 4.0);
    let (r, _) = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(contact), None],
            0.5,
            true,
            &controls,
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 4);
    assert_eq!(r.combined.displacement_scale, 1.0);
    assert_eq!(
        m.positions[0].to_array().map(f32::to_bits),
        [0x3e000000, 0, 0x3f200000]
    );
    let mut m = tetra();
    let before = words(&m);
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options(0.375, true)).unwrap();
    let error = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass([0.0, 0.0, 1.0], 4.0)), None],
            0.5,
            true,
            &controls,
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        SourceGeometryError::UndefinedPickedNormal(
            SourcePickedNormalUndefined::ExactVertexContact { .. }
        )
    ));
    assert_eq!(words(&m), before);
}

#[test]
fn explicit_reference_tangent_matches_all_literal_words_and_retained_history() {
    let mut d = Document::new(tetra());
    let original = d.to_json().unwrap();
    let mut opts = options(0.13, true);
    assert!(opts.prevent_folds);
    let mut guarded = d.begin_source_geometry_stroke(opts).unwrap();
    let p = pass(
        [0.1268937959470357, 0.06249996161826249, 0.8106062424346874],
        0.11794690396893492,
    );
    let rejected = d
        .apply_source_geometry_dab(&mut guarded, p, 0.5, true)
        .unwrap();
    assert_eq!(rejected.noop, Some(SourceGeometryNoop::GuardRejected));
    d.end_source_geometry_stroke(&guarded).unwrap();
    assert_eq!(d.to_json().unwrap(), original);
    opts.prevent_folds = false;
    let mut stroke = d.begin_source_geometry_stroke(opts).unwrap();
    let accepted = d
        .apply_source_geometry_dab(&mut stroke, p, 0.5, true)
        .unwrap();
    assert_eq!(accepted.noop, None);
    assert_eq!(accepted.changed_vertices, 1);
    assert_eq!(accepted.displacement_scale, 1.0);
    let m = d.require_active_mesh().unwrap();
    assert_eq!(
        m.positions[0].to_array().map(f32::to_bits),
        [1026653771, 0, 1065353216]
    );
    assert_eq!(&m.positions[1..], &tetra().positions[1..]);
    d.end_source_geometry_stroke(&stroke).unwrap();
    let finished = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), finished);
    assert!(d.source_geometry_retained_bytes() > 0);
}

#[test]
fn reference_response_keeps_zero_protected_and_nonfinite_rejection_contracts() {
    let mut m = tetra();
    m.masks.fill(0.0);
    let before = words(&m);
    let opts = SourceGeometryOptions {
        prevent_folds: false,
        ..options(0.13, true)
    };
    let report = single(&mut m, opts, pass([0., 0., 1.], 4.));
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(words(&m), before);
    for intensity in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(SourceGeometryStroke::new(SourceGeometryOptions { intensity, ..opts }).is_err());
    }
}
