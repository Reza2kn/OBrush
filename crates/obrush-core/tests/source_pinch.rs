//! Independent Pinch literals and shared transaction contracts. Authored UNRUN.
use obrush_core::*;
use std::sync::Arc;

fn mesh() -> Mesh {
    let mut m = Mesh::import_obj(
        b"v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    )
    .unwrap()
    .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    // This fixture explicitly retains its four literal original triangles.
    m.surface = Some(SurfaceTopology {
        faces: vec![
            [0, 1, 2, TRIANGLE_CORNER],
            [0, 2, 3, TRIANGLE_CORNER],
            [0, 3, 4, TRIANGLE_CORNER],
            [0, 4, 1, TRIANGLE_CORNER],
        ],
        uv: None,
    });
    m.validate().unwrap();
    m
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.625,
        intensity_factor: 0.0,
        ..SourceGeometryOptions::pinch()
    }
}
fn pass(x: f64, r2: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0.0, 0.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: r2,
        original_face_index: if x >= 0.0 { 0 } else { 2 },
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    let mut v = serde_json::to_value(m).unwrap();
    v["raw_f32"] = serde_json::json!({
        "positions":m.positions.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":m.normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":m.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
        "colors":m.colors.iter().map(|v|v.map(f32::to_bits)).collect::<Vec<_>>(),
        "materials":m.materials.as_ref().map(|v|v.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>()),
    });
    v
}
fn polynomial(d: f64) -> f64 {
    let q = d * d;
    (3.0 * q * q - 4.0 * q * d) + 1.0
}
fn single(
    m: &mut Mesh,
    o: SourceGeometryOptions,
    p: SourceGeometryPass,
    pressure: f64,
) -> SourceGeometryReport {
    let state = SourceGeometryNormalState::seed(m).unwrap();
    let mut stroke = SourceGeometryStroke::new(o).unwrap();
    let (report, _) = stroke.apply(m, &state, p, pressure, true).unwrap();
    assert!(stroke.proxy_positions().is_empty());
    report
}

#[test]
fn declared_source_defaults_are_separate_from_legacy() {
    let p = SourceGeometryOptions::pinch();
    assert_eq!(p.tool, SourceGeometryTool::Pinch);
    assert_eq!(p.intensity.to_bits(), 0.75f64.to_bits());
    assert_eq!(p.intensity_factor, 0.0);
    assert!(!p.negative && !p.culling);
    assert_eq!(p.clay_order, SourceClayOrderPolicy::OrderedStaticOctree);
    assert_eq!(SourceGeometryOptions::default().intensity, 0.5);
    assert_eq!(SourceGeometryOptions::inflate().intensity, 0.3);
}
#[test]
fn rational_gather_and_negative_have_literal_float32_words() {
    for (negative, expected) in [(false, 0x3c730000), (true, 0xbc730000)] {
        let mut m = mesh();
        let before = words(&m);
        let mut o = options();
        o.negative = negative;
        let report = single(&mut m, o, pass(0.5, 4.0), 0.5);
        assert_eq!(
            m.positions[0].to_array().map(f32::to_bits),
            [expected, 0, 0]
        );
        assert_eq!(report.changed_vertices, 1);
        assert_eq!(report.displacement_scale, 1.0);
        assert_eq!(report.proxy_captured_vertices, 0);
        for index in 1..5 {
            assert_eq!(
                words(&m)["raw_f32"]["positions"][index],
                before["raw_f32"]["positions"][index]
            );
        }
        for channel in ["masks", "colors", "materials"] {
            assert_eq!(words(&m)["raw_f32"][channel], before["raw_f32"][channel]);
        }
        assert_eq!(m.surface.as_ref().unwrap().faces.len(), 4);
    }
}
#[test]
fn relative_radius_has_no_extra_brush_radius_amplitude() {
    for (x, r2, expected) in [
        (0.5, 4.0, 0x3c730000),
        (1.0, 16.0, 0x3cf30000),
        (0.5, 1.0, 0x3c300000),
    ] {
        let mut m = mesh();
        let r = single(&mut m, options(), pass(x, r2), 0.5);
        assert_eq!(m.positions[0].x.to_bits(), expected);
        assert_eq!(r.displacement_scale, 1.0);
    }
}
#[test]
fn raw_unprotected_half_and_byte128_alpha_have_independent_rational_output() {
    let mut m = mesh();
    m.masks[0] = 0.5;
    let before = words(&m);
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let (r, _) = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5, 4.0)), None],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
                mirror: false,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    assert_eq!(
        m.positions[0].x.to_bits(),
        ((81.0f64 / 21760.0) as f32).to_bits()
    );
    assert_eq!(r.combined.changed_vertices, 1);
    assert_eq!(r.combined.displacement_scale, 1.0);
    assert_eq!(words(&m)["raw_f32"]["masks"], before["raw_f32"]["masks"]);
}
#[test]
fn white_alpha_and_none_share_the_literal_gather_words() {
    for alpha in [None, Some(RasterAlpha::new(1, 1, vec![255]).unwrap())] {
        let mut m = mesh();
        let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
        let mut s = SourceGeometryStroke::new(options()).unwrap();
        let (r, _) = s
            .apply_controlled(
                &mut m,
                &state,
                [Some(pass(0.5, 4.0)), None],
                0.5,
                true,
                &SourceGeometryControls {
                    alpha,
                    mirror: false,
                },
                Default::default(),
                MAX_SOURCE_GEOMETRY_STATE_BYTES,
            )
            .unwrap();
        assert_eq!(m.positions[0].x.to_bits(), 0x3c730000);
        assert_eq!(r.combined.displacement_scale, 1.0);
    }
}
#[test]
fn source_pressure_factor_uses_effective_intensity_before_point05() {
    let mut m = mesh();
    let mut o = options();
    o.intensity_factor = 0.5;
    let r = single(&mut m, o, pass(0.5, 4.0), 0.25);
    assert_eq!(
        m.positions[0].x.to_bits(),
        ((729.0f64 / 65536.0) as f32).to_bits()
    );
    assert_eq!(r.displacement_scale, 1.0);
}
#[test]
fn repeat_uses_float32_live_position_and_never_captures_proxy() {
    let mut m = mesh();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    let (_, next) = s.apply(&mut m, &state, pass(0.5, 4.0), 0.5, true).unwrap();
    let next = next.unwrap();
    let first = f64::from(f32::from_bits(0x3c730000));
    let d = (0.5 - first) / 2.0;
    let expected = (first + (0.5 - first) * polynomial(d) * (1.0 / 32.0)) as f32;
    let (r, _) = s.apply(&mut m, &next, pass(0.5, 4.0), 0.5, true).unwrap();
    assert_eq!(m.positions[0].x.to_bits(), expected.to_bits());
    assert_ne!(
        m.positions[0].x.to_bits(),
        ((243.0f64 / 8192.0) as f32).to_bits()
    );
    assert!(s.proxy_positions().is_empty());
    assert_eq!(r.proxy_captured_vertices, 0);
}
#[test]
fn mirrored_overlap_is_sequential_live_gather_not_cancelled_sum() {
    let mut m = mesh();
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    let (r, _) = s
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5, 4.0)), Some(pass(-0.5, 4.0))],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    let x = f64::from(f32::from_bits(0x3c730000));
    let d = (0.5 + x) / 2.0;
    let expected = (x + (-0.5 - x) * polynomial(d) * (1.0 / 32.0)) as f32;
    assert_eq!(m.positions[0].x.to_bits(), expected.to_bits());
    assert_ne!(m.positions[0].x, 0.0);
    for p in r.passes.into_iter().flatten() {
        assert_eq!(p.selected_vertices, 1);
        assert_eq!(p.changed_vertices, 1);
        assert_eq!(p.displacement_scale, 1.0);
    }
    assert!(s.proxy_positions().is_empty());
}
#[test]
fn mirrored_preselection_evaluates_live_polynomial_beyond_radius_one() {
    let mut m=Mesh::import_obj(b"v 0.609375 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n").unwrap().mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let mut o = options();
    o.intensity = 1.0;
    o.intensity_factor = 1.0;
    o.negative = true;
    let mut first = pass(0.375, 1.0);
    first.original_face_index = 2;
    let second = pass(-0.375, 1.0);
    let starting = f64::from(m.positions[0].x);
    assert_eq!(starting, 39.0 / 64.0);
    assert_eq!(starting + 0.375, 63.0 / 64.0);
    assert!(starting + 0.375 < 1.0);
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut s = SourceGeometryStroke::new(o).unwrap();
    let (r, _) = s
        .apply_controlled(
            &mut m,
            &state,
            [Some(first), Some(second)],
            1.0,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    let x0 = 39.0f64 / 64.0;
    let d = 15.0f64 / 64.0;
    let deformation = -((1.0 * 2.0) * 0.05);
    let x1 = (x0 + (-d) * (polynomial(d) * (deformation * 1.0))) as f32;
    let dm = f64::from(x1) + 3.0 / 8.0;
    assert!(dm > 1.0);
    let expected = (f64::from(x1) + (-dm) * (polynomial(dm) * (deformation * 1.0))) as f32;
    assert_eq!(m.positions[0].x.to_bits(), expected.to_bits());
    assert!(m.positions[0].x.to_bits() > x1.to_bits() + 1);
    for p in r.passes.into_iter().flatten() {
        assert_eq!(p.selected_vertices, 1);
        assert_eq!(p.changed_vertices, 1);
        assert_eq!(p.displacement_scale, 1.0);
    }
}
#[test]
fn culling_is_optional_source_raw_normal_dot_policy() {
    for culling in [false, true] {
        let mut m = mesh();
        let before = words(&m);
        let mut o = options();
        o.culling = culling;
        let mut p = pass(0.5, 4.0);
        p.eye_direction = [0.0, 0.0, 1.0];
        let r = single(&mut m, o, p, 0.5);
        assert_eq!(r.front_vertices, 0);
        if culling {
            assert_eq!(r.changed_vertices, 0);
            assert_eq!(words(&m), before);
        } else {
            assert_eq!(m.positions[0].x.to_bits(), 0x3c730000);
        }
    }
}
#[test]
fn zero_raw_normal_is_front_facing_and_pinching_needs_no_direction() {
    let mut m =
        Mesh::import_obj(b"v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 5 4\n")
            .unwrap()
            .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(state.vertex_normals()[0], Vec3::ZERO);
    let mut o = options();
    o.culling = true;
    let mut s = SourceGeometryStroke::new(o).unwrap();
    let (r, _) = s.apply(&mut m, &state, pass(0.5, 4.0), 0.5, true).unwrap();
    assert_eq!(m.positions[0].x.to_bits(), 0x3c730000);
    assert_eq!(r.front_vertices, 1);
    assert_eq!(r.changed_vertices, 1);
}
#[test]
fn exact_vertex_no_alpha_has_typed_undefined_report_and_useful_neighbours() {
    let mut m = mesh();
    let r = single(&mut m, options(), pass(0.0, 81.0), 0.5);
    assert_eq!(r.changed_vertices, 4);
    assert_eq!(m.positions[0], Vec3::ZERO);
    assert!(matches!(
        r.picked_normal,
        Some(SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ExactVertexContact { .. },
            ..
        })
    ));
}
#[test]
fn zero_pressure_factor_and_protected_components_preserve_all_words() {
    for protected in [false, true] {
        let mut m = mesh();
        m.positions[0].y = -0.0;
        let mut o = options();
        o.intensity_factor = 1.0;
        if protected {
            m.masks.fill(0.0);
        }
        let before = words(&m);
        let r = single(&mut m, o, pass(0.5, 4.0), 0.0);
        assert_eq!(r.changed_vertices, 0);
        assert_eq!(words(&m), before);
    }
}
#[test]
fn noops_preserve_prior_genuine_redo_and_absent_persistent_cache() {
    for mode in 0..4 {
        let mut m = mesh();
        if mode == 0 {
            m.masks.fill(0.0);
        }
        let mut d = Document::new(m);
        d.begin_stroke();
        d.name = "genuine future".into();
        d.end_stroke();
        let future = d.to_json().unwrap();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut o = options();
        if mode == 1 {
            o.intensity = 0.0;
        }
        let mut s = d.begin_source_geometry_stroke(o).unwrap();
        let alpha = (mode == 3).then(|| RasterAlpha::new(1, 1, vec![0]).unwrap());
        let r = d
            .apply_source_geometry_controlled_dab(
                &mut s,
                [Some(pass(0.5, 4.0)), None],
                0.5,
                mode != 2,
                &SourceGeometryControls {
                    alpha,
                    mirror: false,
                },
                Default::default(),
            )
            .unwrap();
        assert_eq!(r.combined.changed_vertices, 0);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
        d.end_source_geometry_stroke(&s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}
#[test]
fn rejected_second_alpha_pass_restores_first_pass_and_staged_stroke() {
    let mut m = mesh();
    let before = words(&m);
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    let second = SourceGeometryPass {
        center: [-4.0, 4.0, 0.0],
        original_face_index: 1,
        ..pass(-0.5, 4.0)
    };
    let e = s
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5, 4.0)), Some(second)],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
                mirror: true,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap_err();
    assert!(matches!(
        e,
        SourceGeometryError::UndefinedPickedNormal(
            SourcePickedNormalUndefined::ExactVertexContact { .. }
        )
    ));
    assert_eq!(words(&m), before);
    assert_eq!(state.generation(), 0);
    assert!(s.proxy_positions().is_empty());
    assert_eq!(s.visited_leaf_count(), 0);
    let (retry, _) = s.apply(&mut m, &state, pass(0.5, 4.0), 0.5, true).unwrap();
    assert_eq!(retry.changed_vertices, 1);
    assert_eq!(m.positions[0].x.to_bits(), 0x3c730000);
}
#[test]
fn whole_mirrored_stroke_undo_redo_exchanges_exact_cache_and_project() {
    let mut d = Document::new(mesh());
    let before = d.to_json().unwrap();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    let r = d
        .apply_source_geometry_controlled_dab(
            &mut s,
            [Some(pass(0.5, 4.0)), Some(pass(-0.5, 4.0))],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 1);
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
fn ordered_large_scope_has_no_clay_single_leaf_limit() {
    let mut source = String::from(
        "v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    );
    for i in 0..104 {
        let x = 20 + i * 3;
        let first = 6 + i * 3;
        source.push_str(&format!(
            "v {x} 0 0\nv {} 0 0\nv {x} 1 0\nf {first} {} {}\n",
            x + 1,
            first + 1,
            first + 2
        ));
    }
    let mut m = Mesh::import_obj(source.as_bytes()).unwrap().mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    // Triangle-only OBJ intentionally omits retained metadata; this fixture
    // explicitly supplies its authored original rows without a fallback gate.
    m.surface = Some(SurfaceTopology {
        faces: m
            .triangles
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect(),
        uv: None,
    });
    m.validate().unwrap();
    assert!(m.surface.as_ref().unwrap().faces.len() > 100);
    let before = words(&m);
    let r = single(&mut m, options(), pass(0.5, 4.0), 0.5);
    assert!(r.ordered_static_selection);
    assert!(!r.clay_single_leaf);
    assert_eq!(m.positions[0].x.to_bits(), 0x3c730000);
    for index in 5..m.positions.len() {
        assert_eq!(
            words(&m)["raw_f32"]["positions"][index],
            before["raw_f32"]["positions"][index]
        );
    }
}
#[test]
fn nonplanar_quad_refreshes_only_selected_raw_cache_rows() {
    let mut m = Mesh::import_obj(b"v -1 -1 0\nv 1 -1 0\nv 1 1 0.5\nv -1 1 0\nf 1 2 3 4\n")
        .unwrap()
        .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(state.vertex_normals(), &[Vec3::new(-1.0, -1.0, 8.0); 4]);
    let p = SourceGeometryPass {
        center: [0.0, -0.5, 0.125],
        original_face_index: 0,
        ..pass(0.5, 2.0)
    };
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    let (r, next) = s.apply(&mut m, &state, p, 0.5, true).unwrap();
    let next = next.unwrap();
    assert_eq!(r.selected_vertices, 2);
    assert_eq!(r.changed_vertices, 2);
    assert_eq!(r.displacement_scale, 1.0);
    let a = m.positions[0].to_array().map(f64::from);
    let b = m.positions[1].to_array().map(f64::from);
    let c = m.positions[2].to_array().map(f64::from);
    let e = m.positions[3].to_array().map(f64::from);
    let cross = |x: [f64; 3], y: [f64; 3]| {
        [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ]
    };
    let ac = std::array::from_fn(|i| c[i] - a[i]);
    let n1 = cross(std::array::from_fn(|i| b[i] - a[i]), ac);
    let n2 = cross(std::array::from_fn(|i| c[i] - e[i]), ac);
    let expected = std::array::from_fn(|i| ((n1[i] + n2[i]) as f32).to_bits());
    for i in [0, 1] {
        assert_eq!(
            next.vertex_normals()[i].to_array().map(f32::to_bits),
            expected
        );
    }
    for i in [2, 3] {
        assert_eq!(next.vertex_normals()[i], Vec3::new(-1.0, -1.0, 8.0));
    }
    assert_ne!(expected, [-1.0f32, -1.0, 8.0].map(f32::to_bits));
    assert!(next.matches_mesh(&m));
    assert_eq!(
        next.normal_report().policy,
        SourceNormalPolicy::StaticSelectedRefresh
    );
}
