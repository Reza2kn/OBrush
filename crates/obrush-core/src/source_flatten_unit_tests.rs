//! Independent predeclared Flatten literals, candidate/guard distinction and
//! real shared transactions. Private budgets stay private. All authored UNRUN.
use super::*;
use crate::{
    Document, RasterAlpha, SourceGeometryAlphaTrack, SourceGeometryAlphaTracks,
    SourceGeometryControlledReport, SourceGeometryControls, SurfaceTopology,
};
use std::cell::Cell;

fn fixture(name: &str) -> Mesh {
    let (text, faces) = match name {
        "corners" => (
            include_str!("../tests/fixtures/independent-flatten-corners.obj"),
            vec![[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]],
        ),
        "ring" => (
            include_str!("../tests/fixtures/independent-flatten-ring-sheets.obj"),
            vec![[0, 2, 1, TRIANGLE_CORNER], [3, 5, 4, TRIANGLE_CORNER]],
        ),
        "back" => (
            include_str!("../tests/fixtures/independent-flatten-front-back.obj"),
            vec![[0, 1, 2, TRIANGLE_CORNER], [3, 5, 4, TRIANGLE_CORNER]],
        ),
        _ => panic!("unknown independent fixture"),
    };
    let mut m = Mesh::import_obj(text.as_bytes()).unwrap().mesh;
    m.surface = Some(SurfaceTopology { faces, uv: None });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.materials = Some(vec![[0.18, 0.08]; m.positions.len()]);
    m.validate().unwrap();
    m
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.25,
        ..SourceGeometryOptions::flatten()
    }
}
fn pass() -> SourceGeometryPass {
    SourceGeometryPass {
        center: [0.0, 0.0, 1.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 4.0,
        original_face_index: 0,
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    serde_json::json!({"mesh":m,"positions":m.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":m.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":m.masks.iter().map(|p|p.to_bits()).collect::<Vec<_>>()})
}
fn apply(
    m: &mut Mesh,
    o: SourceGeometryOptions,
    p: SourceGeometryPass,
    pressure: f64,
) -> SourceGeometryReport {
    let state = SourceGeometryNormalState::seed(m).unwrap();
    SourceGeometryStroke::new(o)
        .unwrap()
        .apply(m, &state, p, pressure, true)
        .unwrap()
        .0
}
fn scalar_z(z: f64, center_z: f64, plane_z: f64, r2: f64, i: f64, alpha: f64, xy2: f64) -> f32 {
    let dz = z - center_z;
    let d = (xy2 + dz * dz).sqrt() / r2.sqrt();
    let q = d * d;
    let w = (3.0 * q * q - 4.0 * q * d) + 1.0;
    (z - w * ((z - plane_z) * i) * alpha) as f32
}
fn controlled(
    m: &mut Mesh,
    o: SourceGeometryOptions,
    contacts: [Option<SourceGeometryPass>; 2],
    alpha: Option<RasterAlpha>,
    tracks: SourceGeometryAlphaTracks,
) -> SourceGeometryControlledReport {
    let state = Arc::new(SourceGeometryNormalState::seed(m).unwrap());
    SourceGeometryStroke::new(o)
        .unwrap()
        .apply_controlled(
            m,
            &state,
            contacts,
            0.5,
            true,
            &SourceGeometryControls {
                alpha,
                mirror: contacts[1].is_some(),
            },
            tracks,
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap()
        .0
}
fn saved_tracks() -> SourceGeometryAlphaTracks {
    SourceGeometryAlphaTracks {
        primary: SourceGeometryAlphaTrack {
            origin: [0.25, 0.0, 1.0],
            side: 3.0,
            ..Default::default()
        },
        mirror: SourceGeometryAlphaTrack {
            origin: [0.5, 0.0, 1.0],
            side: 5.0,
            ..Default::default()
        },
    }
}

#[test]
fn f01_default_negative_projects_only_positive_side_with_literal_full_guard() {
    let mut m = fixture("corners");
    let before = m.clone();
    let r = apply(&mut m, options(), pass(), 0.5);
    assert_eq!(m.positions[0].z.to_bits(), 0x3f600000);
    assert_eq!(m.positions[3], before.positions[3]);
    for v in [1, 2, 4, 5] {
        assert_eq!(m.positions[v], before.positions[v]);
    }
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(r.displacement_scale, 1.0);
    assert_eq!(r.proxy_captured_vertices, 0);
    let p = r.flatten_plane.unwrap();
    assert_eq!(p.center64, [0.0, 0.0, 0.5]);
    assert_eq!(p.normal64, [0.0, 0.0, 1.0]);
    assert_eq!(p.weight_sum, 2.0);
    assert_eq!(m.colors, before.colors);
    assert_eq!(m.materials, before.materials);
}
#[test]
fn f02_negative_false_has_distinct_opposite_side_word() {
    let mut m = fixture("corners");
    let r = apply(
        &mut m,
        SourceGeometryOptions {
            negative: false,
            ..options()
        },
        pass(),
        0.5,
    );
    assert_eq!(m.positions[0].z.to_bits(), 0x3f800000);
    assert_eq!(m.positions[3].z.to_bits(), 0x3db00000);
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(r.displacement_scale, 1.0);
}
#[test]
fn f03_raw_u_changes_plane_and_each_channel_of_influence() {
    for (negative, v, word) in [(true, 0, 0x3f6aaaab), (false, 3, 0x3d6aaaab)] {
        let mut m = fixture("corners");
        m.masks[3] = 0.5;
        let before = m.masks.clone();
        let r = apply(
            &mut m,
            SourceGeometryOptions {
                negative,
                ..options()
            },
            pass(),
            0.5,
        );
        assert_eq!(r.flatten_plane.unwrap().center64[2], 2.0 / 3.0);
        assert_eq!(m.positions[v].z.to_bits(), word);
        assert_eq!(m.masks, before);
        assert_eq!(r.displacement_scale, 1.0);
    }
}
#[test]
fn f04_disabled_culling_writes_backside_but_plane_stays_front_only() {
    for culling in [false, true] {
        let mut m = fixture("back");
        let before = words(&m);
        let r = apply(
            &mut m,
            SourceGeometryOptions {
                negative: false,
                culling,
                ..options()
            },
            pass(),
            0.5,
        );
        assert_eq!(r.front_vertices, 1);
        assert_eq!(r.flatten_plane.unwrap().center64, [0.0, 0.0, 1.0]);
        if culling {
            assert_eq!(words(&m), before);
            assert_eq!(r.changed_vertices, 0);
        } else {
            assert_eq!(m.positions[3].z.to_bits(), 0x3e300000);
            assert_eq!(r.changed_vertices, 1);
        }
    }
}
#[test]
fn f05_second_stroke_uses_live_rehit_and_no_proxy_exact_dyadic_words() {
    let mut m = fixture("ring");
    let mut state = SourceGeometryNormalState::seed(&m).unwrap();
    let mut stroke = SourceGeometryStroke::new(options()).unwrap();
    let (r, next) = stroke.apply(&mut m, &state, pass(), 0.5, true).unwrap();
    state = next.unwrap();
    assert_eq!(r.changed_vertices, 3);
    for v in 0..3 {
        assert_eq!(m.positions[v].z.to_bits(), 0x3f6a0000);
    }
    let p = SourceGeometryPass {
        center: [0.0, 0.0, 117.0 / 128.0],
        ..pass()
    };
    let (r, _) = stroke.apply(&mut m, &state, p, 0.5, true).unwrap();
    for v in 0..3 {
        assert_eq!(m.positions[v].z.to_bits(), 0x3f55e400);
    }
    assert_eq!(r.displacement_scale, 1.0);
    assert_eq!(r.flatten_plane.unwrap().center64[2], 117.0 / 256.0);
    assert!(stroke.proxy_positions().is_empty());
    assert_eq!(r.proxy_captured_vertices, 0);
    assert_eq!(m.positions[3].z, 0.0);
}
#[test]
fn f06_no_clay_plane_offset_or_separate_radius_amplitude() {
    for r2 in [4.0, 16.0] {
        let mut m = fixture("corners");
        let r = apply(
            &mut m,
            options(),
            SourceGeometryPass {
                local_radius_squared: r2,
                ..pass()
            },
            0.5,
        );
        assert_eq!(r.flatten_plane.unwrap().center64[2], 0.5);
        assert_eq!(m.positions[0].z.to_bits(), 0x3f600000);
    }
    let mut flat = fixture("ring");
    let mut clay = flat.clone();
    apply(&mut flat, options(), pass(), 0.5);
    apply(
        &mut clay,
        SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: true,
                accumulate: true,
            },
            ..options()
        },
        pass(),
        0.5,
    );
    assert_eq!(flat.positions[0].z.to_bits(), 0x3f6a0000);
    assert_ne!(clay.positions[0].z.to_bits(), flat.positions[0].z.to_bits());
}
#[test]
fn f07_shared_nearest_alpha_preserves_independent_byte_multiplier() {
    for byte in [0, 128, 255] {
        let mut m = fixture("ring");
        let before = words(&m);
        let r = controlled(
            &mut m,
            options(),
            [Some(pass()), None],
            Some(RasterAlpha::new(1, 1, vec![byte]).unwrap()),
            saved_tracks(),
        );
        let expected = if byte == 128 {
            (244.0_f64 / 255.0) as f32
        } else if byte == 255 {
            117.0 / 128.0
        } else {
            1.0
        };
        for v in 0..3 {
            assert_eq!(m.positions[v].z.to_bits(), expected.to_bits());
        }
        if byte == 0 {
            assert_eq!(words(&m), before);
            assert_eq!(r.combined.changed_vertices, 0);
        } else {
            assert_eq!(r.combined.changed_vertices, 3);
            assert_eq!(r.combined.displacement_scale, 1.0);
        }
    }
}
#[test]
fn f08_no_area_preserves_incoming_tracks_in_both_passes() {
    for zero_radius in [false, true] {
        let mut m = fixture("corners");
        m.masks.fill(0.0);
        let before = words(&m);
        let tracks = saved_tracks();
        let p = SourceGeometryPass {
            local_radius_squared: if zero_radius { 0.0 } else { 4.0 },
            ..pass()
        };
        let r = controlled(
            &mut m,
            options(),
            [Some(p), Some(p)],
            Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
            tracks,
        );
        assert_eq!(words(&m), before);
        assert_eq!(r.combined.noop, Some(SourceGeometryNoop::NoAreaDirection));
        assert_eq!(
            serde_json::to_value(r.tracks).unwrap(),
            serde_json::to_value(tracks).unwrap()
        );
    }
}
#[test]
fn f09_zero_intensity_preserves_genuine_prior_redo_and_updates_valid_track() {
    let mut d = Document::new(fixture("corners"));
    let before = d.to_json().unwrap();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    d.apply_source_geometry_dab(&mut s, pass(), 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&s).unwrap();
    let useful = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    let mut zero = d
        .begin_source_geometry_stroke(SourceGeometryOptions {
            intensity: 0.0,
            ..options()
        })
        .unwrap();
    let r = d
        .apply_source_geometry_dab(&mut zero, pass(), 0.5, true)
        .unwrap();
    assert_eq!(r.changed_vertices, 0);
    d.end_source_geometry_stroke(&zero).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), useful);
    let mut m = fixture("ring");
    let tracks = saved_tracks();
    let r = controlled(
        &mut m,
        SourceGeometryOptions {
            intensity: 0.0,
            ..options()
        },
        [Some(pass()), None],
        None,
        tracks,
    );
    assert_eq!(r.tracks.primary.origin, pass().center);
    assert_ne!(r.tracks.primary.side, tracks.primary.side);
}
#[test]
fn f10_raw_pressure_has_source_curve_and_invalid_is_atomic() {
    for (factor, p, word) in [
        (1.0, 0.0, 0x3f800000),
        (1.0, 0.5, 0x3f600000),
        (1.0, 1.0, 0x3f400000),
        (0.5, 0.0, 0x3f700000),
        (0.5, 1.0, 0x3f500000),
    ] {
        let mut m = fixture("corners");
        apply(
            &mut m,
            SourceGeometryOptions {
                intensity_factor: factor,
                ..options()
            },
            pass(),
            p,
        );
        assert_eq!(m.positions[0].z.to_bits(), word);
    }
    let mut m = fixture("corners");
    let before = words(&m);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(
        SourceGeometryStroke::new(options())
            .unwrap()
            .apply(&mut m, &state, pass(), f64::NAN, true)
            .unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    assert_eq!(words(&m), before);
}
#[test]
fn f11_plane_keeps_input_order_and_large_static_scope_is_genuine() {
    let m = fixture("corners");
    let n = vec![
        Vec3::new(1073741824.0, 0.0, 1.0),
        Vec3::new(1.0 / 1073741824.0, 0.0, 1.0),
        Vec3::new(-1073741824.0, 0.0, 1.0),
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::ZERO,
    ];
    let a = source_flatten::plane(&m, &n, &[0, 1, 2]).unwrap().unwrap();
    let b = source_flatten::plane(&m, &n, &[0, 2, 1]).unwrap().unwrap();
    assert_eq!(a.normal64, [0.0, 0.0, 1.0]);
    assert!(b.normal64[0] > 0.0);
    assert_eq!(a.weight_sum, 3.0);
    let mut m = fixture("corners");
    let mut faces = m.surface.as_ref().unwrap().faces.clone();
    for k in 0..128 {
        let v = m.positions.len() as u32;
        let x = 4.0 + k as f32 * 2.0;
        m.positions.extend([
            Vec3::new(x, 4.0, 0.0),
            Vec3::new(x + 1.0, 4.0, 0.0),
            Vec3::new(x, 5.0, 0.0),
        ]);
        m.triangles.push([v, v + 1, v + 2]);
        faces.push([v, v + 1, v + 2, TRIANGLE_CORNER]);
        m.normals.extend([Vec3::Z; 3]);
        m.colors.extend([[0.5; 3]; 3]);
        m.masks.extend([1.0; 3]);
        m.materials.as_mut().unwrap().extend([[0.18, 0.08]; 3]);
    }
    m.surface.as_mut().unwrap().faces = faces;
    m.validate().unwrap();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let ids = state
        .ordered_tree
        .select(&m.positions, &state.faces, pass().center, 4.0)
        .unwrap()
        .selected_vertices;
    assert_eq!(ids, [0, 3]);
    let r = apply(&mut m, options(), pass(), 0.5);
    assert!(r.ordered_static_selection);
    assert_eq!(r.selected_vertices, 2);
    assert_eq!(m.positions[0].z.to_bits(), 0x3f600000);
}
#[test]
fn f12_overlap_uses_precontacts_but_live_post_primary_plane_and_footprint() {
    let mut m = fixture("ring");
    let r = controlled(
        &mut m,
        options(),
        [Some(pass()), Some(pass())],
        None,
        Default::default(),
    );
    let expected = scalar_z(117.0 / 128.0, 1.0, 117.0 / 256.0, 4.0, 0.25, 1.0, 1.0);
    for v in 0..3 {
        assert_eq!(m.positions[v].z.to_bits(), expected.to_bits());
    }
    assert_eq!(r.contacts[1].unwrap().center, [0.0, 0.0, 1.0]);
    assert_eq!(
        r.passes[0]
            .as_ref()
            .unwrap()
            .flatten_plane
            .unwrap()
            .center64[2],
        0.5
    );
    assert_eq!(
        r.passes[1]
            .as_ref()
            .unwrap()
            .flatten_plane
            .unwrap()
            .center64[2],
        117.0 / 256.0
    );
    assert_ne!(expected.to_bits(), 0x3f55e400);
    assert_eq!(r.combined.displacement_scale, 1.0);
}
#[test]
fn f13_zero_area_and_zero_row_normal_have_distinct_behavior() {
    let mut m = fixture("back");
    let before = words(&m);
    let tracks = saved_tracks();
    let p = SourceGeometryPass {
        eye_direction: [1.0, 0.0, 0.0],
        ..pass()
    };
    let r = controlled(&mut m, options(), [Some(p), None], None, tracks);
    assert_eq!(r.combined.noop, Some(SourceGeometryNoop::NoAreaDirection));
    assert_eq!(words(&m), before);
    assert_eq!(
        serde_json::to_value(r.tracks).unwrap(),
        serde_json::to_value(tracks).unwrap()
    );
    let m = fixture("corners");
    let mut normals = vec![Vec3::ZERO; 6];
    normals[3] = Vec3::new(0.0, 0.0, 4.0);
    let plane = source_flatten::plane(&m, &normals, &[0, 3])
        .unwrap()
        .unwrap();
    assert_eq!(plane.center64, [0.0, 0.0, 0.5]);
    let out =
        source_flatten::candidate(m.positions[0], pass(), 0.25, true, 1.0, plane, |_| Ok(1.0))
            .unwrap();
    assert_eq!(out.z.to_bits(), 0x3f600000);
}
#[test]
fn f14_raw_candidate_is_separate_from_unchanged_quality_guard() {
    let mut m =
        Mesh::from_obj("v 0 0 1\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n")
            .unwrap();
    let old = m.positions.clone();
    let plane = SourceFlattenPlane {
        center64: [0.0; 3],
        normal64: [0.0, 0.0, 1.0],
        weight_sum: 1.0,
        front_vertices: 1,
    };
    let candidate = source_flatten::candidate(
        old[0],
        SourceGeometryPass {
            center: [0.0; 3],
            ..pass()
        },
        2.0,
        true,
        1.0,
        plane,
        |_| Ok(1.0),
    )
    .unwrap();
    assert_eq!(candidate.z.to_bits(), 0xbec00000);
    m.positions[0] = candidate;
    let scale = m.accept_source_geometry_candidate(&old);
    assert!(scale < 1.0);
    assert!(scale >= 0.0);
    assert_ne!(m.positions[0].z.to_bits(), candidate.z.to_bits());
    m.validate().unwrap();
}
#[test]
fn f15_private_budget_stale_domain_and_second_pass_failure_are_atomic() {
    let mut m = fixture("ring");
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let before = words(&m);
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    assert_eq!(
        s.apply_with_state_limit(&mut m, &state, pass(), 0.5, true, 1)
            .unwrap_err(),
        SourceGeometryError::StorageLimit
    );
    assert_eq!(words(&m), before);
    let mut m = fixture("corners");
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let before = words(&m);
    let primary = SourceGeometryPass {
        center: [0.5, 0.5, 1.0],
        local_radius_squared: 16.0,
        ..pass()
    };
    let result = SourceGeometryStroke::new(options())
        .unwrap()
        .apply_controlled(
            &mut m,
            &state,
            [Some(primary), Some(pass())],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
                mirror: true,
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        );
    assert!(matches!(
        result,
        Err(SourceGeometryError::UndefinedPickedNormal(
            SourcePickedNormalUndefined::ExactVertexContact { .. }
        ))
    ));
    assert_eq!(words(&m), before);
    m.positions[0].z = 0.9;
    let edited = words(&m);
    assert_eq!(
        SourceGeometryStroke::new(options())
            .unwrap()
            .apply(&mut m, &state, pass(), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(words(&m), edited);
}
#[test]
fn f16_strict_boundary_side_and_protected_signed_zero_are_preserved() {
    let calls = Cell::new(0);
    let plane = SourceFlattenPlane {
        center64: [0.0; 3],
        normal64: [0.0, 0.0, 1.0],
        weight_sum: 1.0,
        front_vertices: 1,
    };
    for (p, u) in [
        (Vec3::new(-0.0, 0.0, 1.0), 0.0),
        (Vec3::Z, 1.0),
        (Vec3::new(0.0, 0.0, -0.5), 1.0),
    ] {
        let out = source_flatten::candidate(
            p,
            SourceGeometryPass {
                center: [0.0; 3],
                local_radius_squared: 1.0,
                ..pass()
            },
            0.25,
            true,
            u,
            plane,
            |_| {
                calls.set(calls.get() + 1);
                Ok(1.0)
            },
        )
        .unwrap();
        assert_eq!(
            out.to_array().map(f32::to_bits),
            p.to_array().map(f32::to_bits)
        );
    }
    assert_eq!(calls.get(), 0);
    let mut m = fixture("corners");
    let r = apply(
        &mut m,
        options(),
        SourceGeometryPass {
            local_radius_squared: 1.0,
            ..pass()
        },
        0.5,
    );
    assert_eq!(r.selected_vertices, 1);
    assert_eq!(r.changed_vertices, 0);
}
#[test]
fn f17_warm_selected_normal_rows_and_whole_document_history_are_exact() {
    let mut d = Document::new(fixture("corners"));
    let before = d.to_json().unwrap();
    let mut s = d.begin_source_geometry_stroke(options()).unwrap();
    let r = d
        .apply_source_geometry_dab(&mut s, pass(), 0.5, true)
        .unwrap();
    assert_eq!(r.source_normal_rows_refreshed, 2);
    let id = d.active_subtool_id().unwrap();
    let state = d.source_geometry_state(id).unwrap();
    assert_eq!(state.vertex_normals()[0], Vec3::new(-0.25, -0.25, 4.0));
    assert_eq!(state.vertex_normals()[1], Vec3::new(0.0, 0.0, 4.0));
    let warm = state.vertex_normals().to_vec();
    d.end_source_geometry_stroke(&s).unwrap();
    let useful = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), useful);
    assert_eq!(d.source_geometry_state(id).unwrap().vertex_normals(), warm);
}
#[test]
fn f18_report_json_current_policy_caps_and_factory_scopes_stay_truthful() {
    let r = SourceGeometryReport::noop(SourceGeometryNoop::NoContact);
    assert!(serde_json::to_value(r)
        .unwrap()
        .get("flatten_plane")
        .is_none());
    assert_eq!(MAX_SOURCE_GEOMETRY_STATE_BYTES, 256 * 1024 * 1024);
    assert_eq!(MAX_SOURCE_GEOMETRY_SCRATCH_BYTES, 768 * 1024 * 1024);
    assert_eq!(MAX_SOURCE_GEOMETRY_OPERATION_BYTES, 2048 * 1024 * 1024);
    assert_eq!(crate::MAX_SOURCE_ORDERED_TREE_BYTES, 128 * 1024 * 1024);
    assert_eq!(SourceGeometryOptions::default().intensity, 0.5);
    assert_eq!(
        SourceGeometryOptions {
            clay_order: SourceClayOrderPolicy::SingleLeaf,
            ..options()
        }
        .validate()
        .unwrap_err(),
        SourceGeometryError::UnsupportedFlattenOrder
    );
    assert_eq!(SourceGeometryOptions::flatten().intensity, 0.75);
    assert!(SourceGeometryOptions::flatten().negative);
}
