//! Independently authored Crease literals, proxy lifecycle and transaction tests.
//! Actual reference/browser behavior is a separate acceptance gate.
use obrush_core::*;
use std::sync::Arc;

fn mesh() -> Mesh {
    let mut m =
        Mesh::from_obj("v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -3 3 0\nv -3 -3 0\nf 1 2 3\nf 1 4 5\n")
            .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER], [0, 3, 4, TRIANGLE_CORNER]],
        uv: None,
    });
    m.validate().unwrap();
    m
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.625,
        prevent_folds: false,
        ..SourceGeometryOptions::crease()
    }
}
fn pass(x: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0.0, 0.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 4.0,
        original_face_index: usize::from(x < 0.0),
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    let mut value = serde_json::to_value(m).unwrap();
    value["position_words"] = serde_json::json!(m
        .positions
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect::<Vec<_>>());
    value
}
fn apply_once(
    m: &mut Mesh,
    o: SourceGeometryOptions,
    p: SourceGeometryPass,
) -> SourceGeometryReport {
    let state = SourceGeometryNormalState::seed(m).unwrap();
    SourceGeometryStroke::new(o)
        .unwrap()
        .apply(m, &state, p, 0.5, true)
        .unwrap()
        .0
}
fn direction(report: &SourceGeometryReport) -> [f64; 3] {
    match report.picked_normal.unwrap() {
        SourcePickedNormalDisposition::Defined { report } => report.normal64,
        other => panic!("Expected independently defined direction: {other:?}"),
    }
}

#[test]
fn declared_defaults_and_order_do_not_change_other_tools() {
    let o = SourceGeometryOptions::crease();
    assert_eq!(o.tool, SourceGeometryTool::Crease);
    assert_eq!(o.intensity, 0.75);
    assert!(o.negative && o.prevent_folds);
    assert!(!o.culling);
    assert_eq!(o.intensity_factor, 0.0);
    assert_eq!(o.clay_order, SourceClayOrderPolicy::OrderedStaticOctree);
    let invalid = SourceGeometryOptions {
        clay_order: SourceClayOrderPolicy::SingleLeaf,
        ..o
    };
    assert_eq!(
        invalid.validate(),
        Err(SourceGeometryError::UnsupportedCreaseOrder)
    );
    assert_eq!(SourceGeometryOptions::default().intensity, 0.5);
    assert_eq!(
        SourceGeometryOptions::pinch().tool,
        SourceGeometryTool::Pinch
    );
}

#[test]
fn rational_quarter_radius_has_literal_pinch_and_signed_fifth_power_depth() {
    for (negative, depth) in [(false, 0x3d8a17a3), (true, 0xbd8a17a3)] {
        let mut m = mesh();
        let before = m.clone();
        let report = apply_once(
            &mut m,
            SourceGeometryOptions {
                negative,
                ..options()
            },
            pass(0.5),
        );
        // d=1/4, falloff=243/256, deformation=0.625*0.07, radius=2.
        assert_eq!(
            m.positions[0].to_array().map(f32::to_bits),
            [0x3caa199a, 0, depth]
        );
        assert_eq!(report.changed_vertices, 1);
        assert_eq!(report.proxy_captured_vertices, 1);
        assert_eq!(report.source_normal_rows_refreshed, 1);
        assert_eq!(report.displacement_scale, 1.0);
        assert_eq!(direction(&report), [0.0, 0.0, 1.0]);
        assert_eq!(&m.positions[1..], &before.positions[1..]);
        assert_eq!(m.masks, before.masks);
        assert_eq!(m.colors, before.colors);
        assert_eq!(m.materials, before.materials);
        assert_eq!(m.surface, before.surface);
    }
}

#[test]
fn soft_mask_and_gray_alpha_enter_depth_before_its_fifth_power() {
    let mut m = mesh();
    m.masks[0] = 0.5;
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let (report, _) = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5)), None],
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
    let falloff: f64 = (243.0 / 256.0) * (0.5 * (128.0 / 255.0));
    let deformation = 0.625 * 0.07;
    let expected_x = (0.5 * (falloff * deformation)) as f32;
    let depth = (-falloff.powf(5.0) * (deformation * 2.0)) as f32;
    assert_eq!(m.positions[0].x.to_bits(), expected_x.to_bits());
    assert_eq!(m.positions[0].z.to_bits(), depth.to_bits());
    assert_eq!(report.combined.changed_vertices, 1);
    let wrong_linear_mask_depth =
        (-((243.0f64 / 256.0).powf(5.0)) * deformation * 2.0 * 0.5 * (128.0 / 255.0)) as f32;
    assert_ne!(depth.to_bits(), wrong_linear_mask_depth.to_bits());
}

#[test]
fn repeated_dabs_keep_first_touch_proxy_and_a_new_stroke_recaptures() {
    let mut m = mesh();
    let initial = m.clone();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let (_, next) = stroke.apply(&mut m, &state, pass(0.5), 0.5, true).unwrap();
    let first = m.positions[0];
    assert_eq!(stroke.proxy_positions()[0], initial.positions[0]);
    let next = next.unwrap();
    let (report, _) = stroke.apply(&mut m, &next, pass(0.5), 0.5, true).unwrap();
    assert_eq!(report.proxy_captured_vertices, 0);
    assert_eq!(stroke.proxy_positions()[0], initial.positions[0]);
    let dx = 0.5;
    let falloff: f64 = 243.0 / 256.0;
    let n = direction(&report);
    let deformation = 0.625 * 0.07;
    let depth = -falloff.powf(5.0) * (deformation * 2.0);
    let expected_x = ((f64::from(first.x) + dx * (falloff * deformation)) + n[0] * depth) as f32;
    assert_eq!(m.positions[0].x.to_bits(), expected_x.to_bits());
    let fresh_state = SourceGeometryNormalState::seed(&m).unwrap();
    let before_new = m.positions[0];
    let mut new_stroke = SourceGeometryStroke::new(options()).unwrap();
    new_stroke
        .apply(&mut m, &fresh_state, pass(0.5), 0.5, true)
        .unwrap();
    assert_eq!(new_stroke.proxy_positions()[0], before_new);
    assert_ne!(new_stroke.proxy_positions()[0], stroke.proxy_positions()[0]);
}

#[test]
fn mirror_keeps_pre_dab_direction_and_proxy_while_reading_live_primary_positions() {
    let mut primary = mesh();
    let initial = primary.clone();
    let primary_report = apply_once(&mut primary, options(), pass(0.5));
    assert_eq!(direction(&primary_report), [0., 0., 1.]);
    let mut m = mesh();
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let (r, next) = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5)), Some(pass(-0.5))],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
                mirror: true,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap();
    let mirror = direction(r.passes[1].as_ref().unwrap());
    assert_eq!(mirror, [0.0, 0.0, 1.0]);
    // Independently reconstruct the post-primary raw face/vertex field. A
    // mistaken mirror-normal recomputation must produce a different x value.
    let cross = |a: Vec3, b: Vec3, c: Vec3| {
        let u = std::array::from_fn::<_, 3, _>(|k| f64::from(b[k]) - f64::from(a[k]));
        let v = std::array::from_fn::<_, 3, _>(|k| f64::from(c[k]) - f64::from(a[k]));
        [
            (u[1] * v[2] - u[2] * v[1]) as f32,
            (u[2] * v[0] - u[0] * v[2]) as f32,
            (u[0] * v[1] - u[1] * v[0]) as f32,
        ]
    };
    let a = cross(
        primary.positions[0],
        primary.positions[1],
        primary.positions[2],
    );
    let b = cross(
        primary.positions[0],
        primary.positions[3],
        primary.positions[4],
    );
    let refreshed: [f32; 3] =
        std::array::from_fn(|k| ((f64::from(a[k]) + f64::from(b[k])) / 2.0) as f32);
    let mut weighted = [0.0; 3];
    for (id, normal) in [(0, refreshed), (3, [0.0, 0.0, 18.0]), (4, [0.0, 0.0, 18.0])] {
        let delta = std::array::from_fn::<_, 3, _>(|k| {
            f64::from(primary.positions[id][k]) - pass(-0.5).center[k]
        });
        let weight = 1.0 / delta[0].hypot(delta[1]).hypot(delta[2]);
        for k in 0..3 {
            weighted[k] += weight * f64::from(normal[k]);
        }
    }
    let norm = weighted[0].hypot(weighted[1]).hypot(weighted[2]);
    let wrong_post_primary = weighted.map(|v| v / norm);
    assert!(wrong_post_primary[0].abs() > 0.001);
    assert_eq!(stroke.proxy_positions()[0], initial.positions[0]);
    assert_eq!(r.passes[0].as_ref().unwrap().proxy_captured_vertices, 1);
    assert_eq!(r.passes[1].as_ref().unwrap().proxy_captured_vertices, 0);
    let f: f64 = 243.0 / 256.0;
    let deformation = 0.625 * 0.07;
    let depth = -f.powf(5.0) * (deformation * 2.0);
    let expected_x =
        ((f64::from(primary.positions[0].x) - 0.5 * (f * deformation)) + mirror[0] * depth) as f32;
    assert_eq!(m.positions[0].x.to_bits(), expected_x.to_bits());
    let wrong_x = ((f64::from(primary.positions[0].x) - 0.5 * (f * deformation))
        + wrong_post_primary[0] * depth) as f32;
    assert_ne!(m.positions[0].x.to_bits(), wrong_x.to_bits());
    assert_eq!(
        m.positions[0].z.to_bits(),
        ((f64::from(primary.positions[0].z) + depth) as f32).to_bits()
    );
    assert_ne!(m.positions[0].x.to_bits(), 0.0f32.to_bits());
    assert_eq!(next.unwrap().generation(), 2);
}

#[test]
fn optional_culling_preserves_back_facing_points_and_their_attributes() {
    for culling in [false, true] {
        let mut m = mesh();
        let before = words(&m);
        let report = apply_once(
            &mut m,
            SourceGeometryOptions {
                culling,
                ..options()
            },
            SourceGeometryPass {
                eye_direction: [0.0, 0.0, 1.0],
                ..pass(0.5)
            },
        );
        assert_eq!(report.front_vertices, 0);
        if culling {
            assert_eq!(report.changed_vertices, 0);
            assert_eq!(words(&m), before);
        } else {
            assert_eq!(report.changed_vertices, 1);
            assert_eq!(m.positions[0].x.to_bits(), 0x3caa199a);
        }
    }
}

#[test]
fn finite_zero_picked_direction_still_pinches_without_inventing_a_depth_normal() {
    let mut m = Mesh::from_obj("v 0 0 0\nv 4 -4 0\nv 4 4 0\nf 1 2 3\nf 1 3 2\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert!(state.vertex_normals().iter().all(|n| *n == Vec3::ZERO));
    let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions {
        culling: true,
        ..options()
    })
    .unwrap();
    let (report, _) = stroke.apply(&mut m, &state, pass(0.5), 0.5, true).unwrap();
    assert!(matches!(
        report.picked_normal,
        Some(SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ZeroDirection,
            ..
        })
    ));
    assert_eq!(report.front_vertices, 1);
    assert_eq!(report.changed_vertices, 1);
    assert_eq!(
        m.positions[0].to_array().map(f32::to_bits),
        [0x3caa199a, 0, 0]
    );
}

#[test]
fn second_pass_undefined_direction_rolls_back_geometry_normals_and_proxy() {
    let mut m = mesh();
    let before = words(&m);
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let bad = SourceGeometryPass {
        center: [-3., 3., 0.],
        ..pass(-0.5)
    };
    let e = stroke
        .apply_controlled(
            &mut m,
            &state,
            [Some(pass(0.5)), Some(bad)],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
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
    assert!(stroke.proxy_positions().is_empty());
    assert!(state.matches_mesh(&m));
}

#[test]
fn optional_fold_guard_limits_extreme_crease_without_changing_full_response_mode() {
    let mut guarded = mesh();
    guarded.masks[1..].fill(0.0);
    let before = guarded.clone();
    let mut full = guarded.clone();
    let contact = SourceGeometryPass {
        local_radius_squared: 1_000_000_000_000.0,
        ..pass(0.5)
    };
    let g = apply_once(
        &mut guarded,
        SourceGeometryOptions {
            prevent_folds: true,
            ..options()
        },
        contact,
    );
    let f = apply_once(&mut full, options(), contact);
    assert_eq!(f.displacement_scale, 1.0);
    assert!(g.displacement_scale >= 0.0 && g.displacement_scale < 1.0);
    assert!(full.positions[0].z.abs() > 10_000.0);
    assert!(guarded.positions[0].z.abs() < full.positions[0].z.abs());
    assert_eq!(&guarded.positions[1..], &before.positions[1..]);
    assert_eq!(guarded.masks, before.masks);
    assert_eq!(guarded.colors, before.colors);
    assert_eq!(guarded.materials, before.materials);
    guarded.validate().unwrap();
    for triangle in &guarded.triangles {
        let [a, b, c] = triangle.map(|v| guarded.positions[v as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let longest = (b - a)
            .length_squared()
            .max((c - a).length_squared())
            .max((c - b).length_squared());
        assert!(cross.length() / longest >= 0.1);
        assert!(cross.z > 0.0);
    }
}

#[test]
fn pressure_protection_and_no_contact_noops_preserve_real_redo() {
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
            o.intensity_factor = 1.0;
        }
        let mut stroke = d.begin_source_geometry_stroke(o).unwrap();
        let r = d
            .apply_source_geometry_controlled_dab(
                &mut stroke,
                [Some(pass(0.5)), None],
                if mode == 1 { 0.0 } else { 0.5 },
                mode != 2,
                &SourceGeometryControls {
                    alpha: (mode == 3).then(|| RasterAlpha::new(1, 1, vec![0]).unwrap()),
                    mirror: false,
                },
                Default::default(),
            )
            .unwrap();
        assert_eq!(r.combined.changed_vertices, 0);
        d.end_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}

#[test]
fn useful_two_dab_document_history_restores_entire_mesh_and_retained_field() {
    let mut d = Document::new(mesh());
    let before = d.to_json().unwrap();
    let id = d.active_subtool_id().unwrap();
    let mut stroke = d.begin_source_geometry_stroke(options()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            d.apply_source_geometry_dab(&mut stroke, pass(0.5), 0.5, true)
                .unwrap()
                .changed_vertices,
            1
        );
    }
    d.end_source_geometry_stroke(&stroke).unwrap();
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert_eq!(d.source_geometry_state(id).unwrap().generation(), 2);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.source_geometry_state(id).is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert_eq!(d.source_geometry_state(id).unwrap().generation(), 2);
}
