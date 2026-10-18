//! Independent literal ordinary Source controls. Authored before compiler/runtime.
use obrush_core::*;
use std::sync::Arc;

fn mesh() -> Mesh {
    let mut m = Mesh::from_obj(
        "v 0 0 0\nv -4 -4 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    )
    .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m
}
fn options(tool: SourceGeometryTool) -> SourceGeometryOptions {
    SourceGeometryOptions {
        tool,
        intensity: 0.625,
        intensity_factor: 0.0,
        clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
        ..Default::default()
    }
}
fn brush() -> SourceGeometryOptions {
    options(SourceGeometryTool::Brush {
        clay: false,
        accumulate: false,
    })
}
fn pass(x: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0.0, 0.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 4.0,
        original_face_index: if x >= 0.0 { 1 } else { 3 },
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    let mut out = serde_json::to_value(m).unwrap();
    out["f32_words"] = serde_json::json!({
        "positions":m.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":m.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":m.masks.iter().map(|p|p.to_bits()).collect::<Vec<_>>(),
        "colors":m.colors.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),
        "materials":m.materials.as_ref().map(|rows|rows.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>()),
        "uv":m.surface.as_ref().and_then(|s|s.uv.as_ref()).map(|u|u.coordinates.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>()),
    });
    out
}
fn identity_track() -> SourceGeometryAlphaTrack {
    SourceGeometryAlphaTrack {
        side: 1.0,
        ..Default::default()
    }
}
fn zero_normal(p: SourceGeometryPass) -> SourcePickedNormalDisposition {
    SourcePickedNormalDisposition::Undefined {
        reason: SourcePickedNormalUndefined::ZeroDirection,
        original_face_index: p.original_face_index,
        local_point64: p.center,
        raw_report: None,
    }
}
fn apply(
    m: &mut Mesh,
    s: &Arc<SourceGeometryNormalState>,
    stroke: &mut SourceGeometryStroke,
    contacts: [Option<SourceGeometryPass>; 2],
    alpha: Option<RasterAlpha>,
    mirror: bool,
) -> (
    SourceGeometryControlledReport,
    Option<Arc<SourceGeometryNormalState>>,
) {
    stroke
        .apply_controlled(
            m,
            s,
            contacts,
            0.5,
            true,
            &SourceGeometryControls { alpha, mirror },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
        .unwrap()
}
#[test]
fn reflect_exact_unit_plane_and_involution() {
    let p = SourceMirrorPlane {
        origin: [1.0, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    };
    assert_eq!(p.reflect([3.0, 2.0, -1.0]).unwrap(), [-1.0, 2.0, -1.0]);
    assert_eq!(
        p.reflect(p.reflect([3.0, 2.0, -1.0]).unwrap()).unwrap(),
        [3.0, 2.0, -1.0]
    );
}
#[test]
fn offset_uses_prescribed_retained_loose_radius() {
    let p = SourceMirrorPlane::from_relative_offset(
        [0.0; 3],
        [1.0, 0.0, 0.0],
        0.5,
        [-2.0, 0.0, 0.0, 2.0, 0.0, 0.0],
    )
    .unwrap();
    assert_eq!(p.origin.map(f32::to_bits), [1.0f32.to_bits(), 0, 0]);
    assert_eq!(p.reflect([3.0, 0.0, 0.0]).unwrap(), [-1.0, 0.0, 0.0]);
}
#[test]
fn plane_nonunit_nan_and_reversed_bounds_reject() {
    for n in [[2.0, 0.0, 0.0], [f64::NAN, 0.0, 0.0], [0.0; 3]] {
        assert!(SourceMirrorPlane {
            origin: [0.0; 3],
            normal: n
        }
        .reflect([1.0, 0.0, 0.0])
        .is_err());
    }
    assert!(SourceMirrorPlane::from_relative_offset(
        [0.0; 3],
        [1.0, 0.0, 0.0],
        0.0,
        [2.0, 0.0, 0.0, -2.0, 0.0, 0.0]
    )
    .is_err());
}
#[test]
fn asymmetric_three_by_two_nearest_and_mirror_direction() {
    let a = RasterAlpha::new(3, 2, vec![0, 64, 255, 255, 128, 0]).unwrap();
    let t = identity_track();
    assert_eq!(
        t.sample(&a, pass(0.5), false, Vec3::ZERO).unwrap(),
        128.0 / 255.0
    );
    assert_eq!(
        t.sample(&a, pass(0.5), false, Vec3::new(0.5, 0.0, 0.0))
            .unwrap(),
        1.0
    );
    assert_eq!(
        t.sample(&a, pass(0.5), false, Vec3::new(-0.5, 0.0, 0.0))
            .unwrap(),
        0.0
    );
    assert_eq!(
        t.sample(&a, pass(0.5), true, Vec3::new(0.5, 0.0, 0.0))
            .unwrap(),
        0.0
    );
    assert_eq!(t.sample(&a, pass(0.5), false, Vec3::Y).unwrap(), 0.0);
}
#[test]
fn exact_x_edge_flat_row_alias_preserved() {
    let a = RasterAlpha::new(3, 2, vec![0, 64, 255, 255, 128, 0]).unwrap();
    assert_eq!(
        identity_track()
            .sample(&a, pass(0.5), false, Vec3::new(-1.0, 0.5, 0.0))
            .unwrap(),
        1.0
    );
    assert_eq!(
        identity_track()
            .sample(&a, pass(0.5), false, Vec3::new(1.0, 0.5, 0.0))
            .unwrap(),
        0.0
    );
}
#[test]
fn exact_x_edge_global_oob_rejects_and_nonfinite_side_rejects() {
    let a = RasterAlpha::new(3, 2, vec![0, 64, 255, 255, 128, 0]).unwrap();
    assert_eq!(
        identity_track()
            .sample(&a, pass(0.5), false, Vec3::new(-1.0, -0.5, 0.0))
            .unwrap_err(),
        SourceGeometryError::UndefinedAlphaLookup
    );
    let t = SourceGeometryAlphaTrack {
        side: f64::NAN,
        ..Default::default()
    };
    assert_eq!(
        t.sample(&a, pass(0.5), false, Vec3::ZERO).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
}
#[test]
fn transposed_alpha_has_vertical_rows_and_horizontal_ratio_support() {
    let a = RasterAlpha::new(2, 3, vec![0, 64, 128, 255, 255, 0]).unwrap();
    let t = identity_track();
    assert_eq!(t.sample(&a, pass(0.5), false, Vec3::ZERO).unwrap(), 1.0);
    assert_eq!(t.sample(&a, pass(0.5), false, Vec3::X).unwrap(), 0.0);
    assert_eq!(
        t.sample(&a, pass(0.5), false, Vec3::new(0.0, 0.75, 0.0))
            .unwrap(),
        64.0 / 255.0
    );
}
#[test]
fn zero_travel_changes_side_and_keeps_exact_old_origin_frame() {
    let mut p = pass(0.5);
    p.center = [0.0; 3];
    let before = SourceGeometryAlphaTrack::default();
    let t = before.updated(p, zero_normal(p)).unwrap();
    assert_eq!(t.origin.map(f64::to_bits), before.origin.map(f64::to_bits));
    assert_eq!(t.frame.map(f32::to_bits), before.frame.map(f32::to_bits));
    assert_eq!(
        t.side.to_bits(),
        (2.0 * std::f64::consts::FRAC_1_SQRT_2).to_bits()
    );
}
#[test]
fn defined_zero_direction_generates_identity_without_fallback() {
    let p = pass(0.5);
    let t = SourceGeometryAlphaTrack::default()
        .updated(p, zero_normal(p))
        .unwrap();
    assert_eq!(t.origin, [0.5, 0.0, 0.0]);
    assert_eq!(t.frame, SourceGeometryAlphaTrack::default().frame);
    assert!(t.undefined_normal.is_none());
}
#[test]
fn undefined_exact_vertex_active_alpha_rejects_but_track_is_typed() {
    let p = pass(0.5);
    let reason = SourcePickedNormalUndefined::ExactVertexContact {
        corner: 0,
        vertex_id: 0,
    };
    let disposition = SourcePickedNormalDisposition::Undefined {
        reason,
        original_face_index: p.original_face_index,
        local_point64: p.center,
        raw_report: None,
    };
    let t = SourceGeometryAlphaTrack::default()
        .updated(p, disposition)
        .unwrap();
    assert_eq!(t.origin, p.center);
    assert_eq!(t.undefined_normal, Some(reason));
    let a = RasterAlpha::new(1, 1, vec![255]).unwrap();
    assert_eq!(
        t.sample(&a, p, false, Vec3::ZERO).unwrap_err(),
        SourceGeometryError::UndefinedPickedNormal(reason)
    );
}
#[test]
fn invalid_frame_contact_provenance_and_radius_reject() {
    let mut p = pass(0.5);
    let picked = zero_normal(p);
    p.center[0] = f64::NAN;
    assert!(identity_track().updated(p, picked).is_err());
    p = pass(0.5);
    p.local_radius_squared = f64::INFINITY;
    assert!(identity_track().updated(p, zero_normal(p)).is_err());
    p = pass(0.5);
    let mut other = p;
    other.original_face_index = 0;
    assert!(identity_track().updated(p, zero_normal(other)).is_err());
}
#[test]
fn rational_overlap_brush_proxy_applies_centerline_twice() {
    let mut m = mesh();
    let initial = words(&m);
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    let (r, next) = apply(
        &mut m,
        &s,
        &mut stroke,
        [Some(pass(0.5)), Some(pass(-0.5))],
        None,
        true,
    );
    assert_eq!(m.positions[0].z.to_bits(), 0x3e730000); //243/1024
    assert_eq!(r.passes[0].as_ref().unwrap().selected_vertices, 1);
    assert_eq!(r.passes[1].as_ref().unwrap().selected_vertices, 1);
    assert_eq!(r.combined.changed_vertices, 1);
    assert_eq!(r.combined.proxy_captured_vertices, 1);
    assert_eq!(stroke.proxy_positions()[0], Vec3::ZERO);
    assert_eq!(next.unwrap().vertex_normals(), s.vertex_normals());
    assert_eq!(&words(&m)["positions"][1], &initial["positions"][1]);
    assert_eq!(words(&m)["masks"], initial["masks"]);
    assert_eq!(words(&m)["colors"], initial["colors"]);
}
#[test]
fn one_pass_white_alpha_literal_brush_response() {
    let mut m = mesh();
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    let (r, _) = apply(
        &mut m,
        &s,
        &mut stroke,
        [Some(pass(0.5)), None],
        Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
        false,
    );
    assert_eq!(m.positions[0].z.to_bits(), 0x3df30000); //243/2048
    assert_eq!(r.combined.changed_vertices, 1);
}
#[test]
fn one_pass_half_byte_alpha_rational_response() {
    let mut m = mesh();
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    apply(
        &mut m,
        &s,
        &mut stroke,
        [Some(pass(0.5)), None],
        Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
        false,
    );
    assert_eq!(
        m.positions[0].z.to_bits(),
        ((81.0f64 / 1360.0) as f32).to_bits()
    );
}
#[test]
fn plain_accumulate_second_radial_distance_uses_live_first_z() {
    let mut m = mesh();
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options(SourceGeometryTool::Brush {
        clay: false,
        accumulate: true,
    }))
    .unwrap();
    apply(
        &mut m,
        &s,
        &mut stroke,
        [Some(pass(0.5)), Some(pass(-0.5))],
        None,
        true,
    );
    let z = 243.0f64 / 2048.0;
    let d = (0.25 + z * z).sqrt() / 2.0;
    let q = d * d;
    let expected = (z + 0.125 * ((3.0 * q * q - 4.0 * q * d) + 1.0)) as f32;
    assert_eq!(m.positions[0].z.to_bits(), expected.to_bits());
    assert!(stroke.proxy_positions().is_empty());
}
#[test]
fn inflate_half_byte_and_negative_preserve_operand_order() {
    for negative in [false, true] {
        let mut m = mesh();
        let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
        let mut o = options(SourceGeometryTool::Inflate);
        o.negative = negative;
        let mut stroke = SourceGeometryStroke::new(o).unwrap();
        apply(
            &mut m,
            &s,
            &mut stroke,
            [Some(pass(0.5)), None],
            Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
            false,
        );
        let sign = if negative { -1.0 } else { 1.0 };
        let polynomial = 243.0f64 / 256.0;
        let weight = (sign * 0.125 * polynomial) / 32.0 * (128.0 / 255.0);
        assert_eq!(
            m.positions[0].z.to_bits(),
            ((32.0 * weight) as f32).to_bits()
        );
    }
}
#[test]
fn clay_alpha_uses_area_plane_and_independent_polynomial() {
    let mut m = mesh();
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(options(SourceGeometryTool::Brush {
        clay: true,
        accumulate: true,
    }))
    .unwrap();
    apply(
        &mut m,
        &s,
        &mut stroke,
        [Some(pass(0.5)), None],
        Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
        false,
    );
    let polynomial = 243.0f64 / 256.0;
    let distance = -0.2f64;
    let expected = -(polynomial * (distance * 0.625 * 1.0 * (128.0 / 255.0)));
    assert_eq!(m.positions[0].z.to_bits(), (expected as f32).to_bits());
}
#[test]
fn all_protected_and_no_contact_keep_prior_real_redo() {
    let mut d = Document::new(mesh());
    let mut stroke = d.begin_source_geometry_stroke(brush()).unwrap();
    d.apply_source_geometry_dab(&mut stroke, pass(0.5), 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let redo = d.can_redo();
    let mut stroke = d.begin_source_geometry_stroke(brush()).unwrap();
    let r = d
        .apply_source_geometry_controlled_dab(
            &mut stroke,
            [Some(pass(0.5)), Some(pass(-0.5))],
            0.5,
            false,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 0);
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.can_redo(), redo);
    assert!(d.redo());
    let mut m = mesh();
    m.masks.fill(0.0);
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let mut stroke = d.begin_source_geometry_stroke(brush()).unwrap();
    let r = d
        .apply_source_geometry_controlled_dab(
            &mut stroke,
            [Some(pass(0.5)), Some(pass(-0.5))],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 0);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn second_pass_undefined_brush_direction_restores_first_pass_every_word() {
    let mut m = mesh();
    let before = words(&m);
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    let mut second = pass(-0.5);
    second.center = [0.0; 3];
    second.original_face_index = 0;
    assert!(stroke
        .apply_controlled(
            &mut m,
            &s,
            [Some(pass(0.5)), Some(second)],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: None,
                mirror: true
            },
            Default::default(),
            MAX_SOURCE_GEOMETRY_STATE_BYTES
        )
        .is_err());
    assert_eq!(words(&m), before);
    assert!(stroke.proxy_positions().is_empty());
    assert_eq!(s.generation(), 0);
    assert_eq!(stroke.visited_leaf_count(), 0);
}
#[test]
fn insufficient_controlled_retained_budget_restores_every_word() {
    let mut m = mesh();
    let before = words(&m);
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    assert_eq!(
        stroke
            .apply_controlled(
                &mut m,
                &s,
                [Some(pass(0.5)), Some(pass(-0.5))],
                0.5,
                true,
                &SourceGeometryControls {
                    alpha: None,
                    mirror: true
                },
                Default::default(),
                0
            )
            .unwrap_err(),
        SourceGeometryError::StorageLimit
    );
    assert_eq!(words(&m), before);
    assert!(stroke.proxy_positions().is_empty());
}
#[test]
fn public_stale_geometry_rejects_before_either_pass() {
    let mut m = mesh();
    let s = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    m.positions[4].z = 0.125;
    let before = words(&m);
    assert_eq!(
        stroke
            .apply_controlled(
                &mut m,
                &s,
                [Some(pass(0.5)), Some(pass(-0.5))],
                0.5,
                true,
                &SourceGeometryControls {
                    alpha: None,
                    mirror: true
                },
                Default::default(),
                MAX_SOURCE_GEOMETRY_STATE_BYTES
            )
            .unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(words(&m), before);
}
#[test]
fn whole_stroke_mirror_history_preserves_seeded_center_and_cache_words() {
    let mut d = Document::new(mesh());
    let before = d.to_json().unwrap();
    let mut stroke = d.begin_source_geometry_stroke(brush()).unwrap();
    d.apply_source_geometry_controlled_dab(
        &mut stroke,
        [Some(pass(0.5)), Some(pass(-0.5))],
        0.5,
        true,
        &SourceGeometryControls {
            alpha: None,
            mirror: true,
        },
        Default::default(),
    )
    .unwrap();
    d.end_source_geometry_stroke(&stroke).unwrap();
    let after = d.to_json().unwrap();
    let state = d.source_geometry_state(1).unwrap();
    let center = state.interaction_center();
    assert!(d.can_undo());
    assert!(d.undo());
    assert!(!d.can_undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.source_geometry_state(1).is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    let restored = d.source_geometry_state(1).unwrap();
    assert_eq!(
        restored.interaction_center().map(f32::to_bits),
        center.map(f32::to_bits)
    );
    assert!(Arc::ptr_eq(&restored, &state));
}
#[test]
fn no_mirror_alpha_none_matches_existing_public_api_whole_output() {
    let mut old = mesh();
    let mut new = old.clone();
    let state = Arc::new(SourceGeometryNormalState::seed(&old).unwrap());
    let mut a = SourceGeometryStroke::new(brush()).unwrap();
    let mut b = SourceGeometryStroke::new(brush()).unwrap();
    let (old_report, old_next) = a.apply(&mut old, &state, pass(0.5), 0.5, true).unwrap();
    let (new_report, new_next) = apply(
        &mut new,
        &state,
        &mut b,
        [Some(pass(0.5)), None],
        None,
        false,
    );
    assert_eq!(words(&old), words(&new));
    assert_eq!(
        old_report.changed_vertices,
        new_report.combined.changed_vertices
    );
    assert_eq!(a.proxy_positions(), b.proxy_positions());
    assert_eq!(
        old_next.unwrap().vertex_normals(),
        new_next.unwrap().vertex_normals()
    );
}
#[test]
fn primary_and_reflected_actual_local_contacts_have_real_face_identity() {
    let m = mesh();
    let view = PaintViewCapture {
        world_to_clip: [
            0.25, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        viewport: [0.0, 0.0, 100.0, 100.0],
    };
    let input = PaintPreparationInput {
        view,
        screen: [56.25, 50.0],
        screen_radius: 25.0,
        pressure: 0.5,
        radius_factor: 0.0,
    };
    let contacts = prepare_source_geometry_contacts(
        &m,
        None,
        input,
        Some(SourceMirrorPlane {
            origin: [0.0; 3],
            normal: [1.0, 0.0, 0.0],
        }),
    )
    .unwrap();
    assert_eq!(contacts.passes[0].unwrap().center, [0.5, 0.0, 0.0]);
    assert_eq!(contacts.passes[1].unwrap().center, [-0.5, 0.0, 0.0]);
    assert!(contacts.triangles.iter().all(Option::is_some));
    assert_eq!(
        contacts.passes[0].unwrap().local_radius_squared,
        contacts.passes[1].unwrap().local_radius_squared
    );
}
#[test]
fn literal_planar_picked_normal_travel_generates_source_frame_axes() {
    let m = mesh();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let p = pass(0.5);
    let t = SourceGeometryAlphaTrack::default()
        .updated(p, state.picked_normal(&m, p).unwrap())
        .unwrap();
    let expected = [
        -0.0f32, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, -0.0, -0.5, -0.0, 1.0,
    ];
    assert_eq!(t.frame.map(f32::to_bits), expected.map(f32::to_bits));
    assert_eq!(t.origin, [0.5, 0.0, 0.0]);
}
#[test]
fn nonplanar_original_quad_freezes_both_picked_normals_before_refresh() {
    let mut m = Mesh::import_obj(b"v -1 -1 0\nv 1 -1 0\nv 1 1 0.5\nv -1 1 0\nf 1 2 3 4\n")
        .unwrap()
        .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let initial = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    assert_eq!(initial.face_normals()[0], Vec3::new(-1.0, -1.0, 8.0));
    let first = SourceGeometryPass {
        center: [0.0, -0.5, 0.125],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 2.0,
        original_face_index: 0,
    };
    let second = SourceGeometryPass {
        center: [0.0, 0.5, 0.25],
        ..first
    };
    let before = initial.picked_normal(&m, second).unwrap();
    let mut one = m.clone();
    let mut single = SourceGeometryStroke::new(brush()).unwrap();
    let (_, one_state) = single.apply(&mut one, &initial, first, 0.5, true).unwrap();
    let one_state = one_state.unwrap();
    let live = one_state.picked_normal(&one, second).unwrap();
    let SourcePickedNormalDisposition::Defined { report: expected } = before else {
        panic!("Independent pre-dab nonvertex contact must define direction");
    };
    let SourcePickedNormalDisposition::Defined { report: changed } = live else {
        panic!("Warm current IDW remains defined");
    };
    assert_ne!(
        expected.normal64.map(f64::to_bits),
        changed.normal64.map(f64::to_bits)
    );
    let mut both = SourceGeometryStroke::new(brush()).unwrap();
    let (r, next) = apply(
        &mut m,
        &initial,
        &mut both,
        [Some(first), Some(second)],
        None,
        true,
    );
    let SourcePickedNormalDisposition::Defined { report: actual } =
        r.passes[1].as_ref().unwrap().picked_normal.unwrap()
    else {
        panic!("Mirror must retain actual pre-dab report");
    };
    assert_eq!(
        actual.normal64.map(f64::to_bits),
        expected.normal64.map(f64::to_bits)
    );
    let final_state = next.unwrap();
    for v in [0, 1] {
        assert_eq!(
            final_state.vertex_normals()[v].to_array().map(f32::to_bits),
            one_state.vertex_normals()[v].to_array().map(f32::to_bits)
        );
    }
    assert!(r.passes.iter().flatten().all(|p| p.changed_vertices > 0));
}
#[test]
fn actual_affine_ray_reflects_after_object_local_inverse() {
    let m = mesh();
    let pose = ObjectAffineF32V1::from_local_points(
        [
            2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        ],
        &m.positions,
    )
    .unwrap();
    let view = PaintViewCapture {
        world_to_clip: [
            0.25, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        viewport: [0.0, 0.0, 100.0, 100.0],
    };
    let input = PaintPreparationInput {
        view,
        screen: [75.0, 50.0],
        screen_radius: 25.0,
        pressure: 0.5,
        radius_factor: 0.0,
    };
    let c = prepare_source_geometry_contacts(
        &m,
        Some(&pose),
        input,
        Some(SourceMirrorPlane {
            origin: [0.0; 3],
            normal: [1.0, 0.0, 0.0],
        }),
    )
    .unwrap();
    assert_eq!(c.passes[0].unwrap().center, [0.5, 0.0, 0.0]);
    assert_eq!(c.passes[1].unwrap().center, [-0.5, 0.0, 0.0]);
    assert_eq!(c.passes[0].unwrap().local_radius_squared, 1.0);
    assert_eq!(c.passes[1].unwrap().local_radius_squared, 1.0);
}
#[test]
fn mirrored_only_actual_hit_keeps_source_zero_primary_radius() {
    let m = Mesh::from_obj("v -1 -1 0\nv 0 -1 0\nv -1 1 0\nf 1 2 3\n").unwrap();
    let view = PaintViewCapture {
        world_to_clip: [
            0.25, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        viewport: [0.0, 0.0, 100.0, 100.0],
    };
    let c = prepare_source_geometry_contacts(
        &m,
        None,
        PaintPreparationInput {
            view,
            screen: [59.375, 50.0],
            screen_radius: 25.0,
            pressure: 0.5,
            radius_factor: 0.0,
        },
        Some(SourceMirrorPlane {
            origin: [0.0; 3],
            normal: [1.0, 0.0, 0.0],
        }),
    )
    .unwrap();
    assert!(c.passes[0].is_none());
    assert_eq!(c.passes[1].unwrap().center, [-0.75, 0.0, 0.0]);
    assert_eq!(c.passes[1].unwrap().local_radius_squared, 0.0);
}
#[test]
fn zero_strength_updates_transient_alpha_track_without_cache_history() {
    let mut d = Document::new(mesh());
    let before = d.to_json().unwrap();
    let mut o = brush();
    o.intensity = 0.0;
    let mut stroke = d.begin_source_geometry_stroke(o).unwrap();
    let r = d
        .apply_source_geometry_controlled_dab(
            &mut stroke,
            [Some(pass(0.5)), Some(pass(-0.5))],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.combined.changed_vertices, 0);
    assert_eq!(r.tracks.primary.origin, [0.5, 0.0, 0.0]);
    assert_eq!(r.tracks.mirror.origin, [-0.5, 0.0, 0.0]);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
#[test]
fn ordered_general_clay_mirror_over_one_hundred_faces_has_independent_scalar_output() {
    let mut obj = String::new();
    let mut vertex = 1;
    for sign in [1.0f64, -1.0] {
        for _ in 0..53 {
            for (x, y) in [(4.0, -0.1), (4.2, -0.1), (4.0, 0.1)] {
                obj.push_str(&format!("v {} {y} 0\n", sign * x));
            }
            let order = if sign > 0.0 {
                [vertex, vertex + 1, vertex + 2]
            } else {
                [vertex, vertex + 2, vertex + 1]
            };
            obj.push_str(&format!("f {} {} {}\n", order[0], order[1], order[2]));
            vertex += 3;
        }
    }
    let mut m = Mesh::from_obj(&obj).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let initial = m.positions.clone();
    let state = Arc::new(SourceGeometryNormalState::seed(&m).unwrap());
    assert_eq!(m.triangles.len(), 106);
    assert!(!state.clay_admitted());
    let first = SourceGeometryPass {
        center: [4.05, 0.0, 0.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 1.0,
        original_face_index: 0,
    };
    let second = SourceGeometryPass {
        center: [-4.05, 0.0, 0.0],
        original_face_index: 53,
        ..first
    };
    let mut stroke = SourceGeometryStroke::new(options(SourceGeometryTool::Brush {
        clay: true,
        accumulate: true,
    }))
    .unwrap();
    let (r, _) = apply(
        &mut m,
        &state,
        &mut stroke,
        [Some(first), Some(second)],
        None,
        true,
    );
    assert_eq!(r.combined.changed_vertices, 318);
    assert!(r.combined.ordered_static_selection);
    // Independently derive literal original-face Float32 raw normal and
    // Source area normalization, rather than replacing rounded normalize with1.
    let raw = ((f64::from(4.2f32) - 4.0) * (f64::from(0.1f32) - f64::from(-0.1f32))) as f32;
    let sum = f64::from(raw) * 159.0;
    let normal = sum * (1.0 / (sum * sum).sqrt());
    for (v, (old, new)) in initial.iter().zip(&m.positions).enumerate() {
        let center = if v < 159 { 4.05 } else { -4.05 };
        let dx = f64::from(old.x) - center;
        let dy = f64::from(old.y);
        let d = (dx * dx + dy * dy).sqrt();
        let q = d * d;
        let distance = -(normal * 0.1) * normal;
        let expected =
            (-normal * (((3.0 * q * q - 4.0 * q * d) + 1.0) * (distance * 0.625))) as f32;
        assert_eq!(
            new.z.to_bits(),
            expected.to_bits(),
            "Literal Source area-plane response vertex{v}"
        );
        assert_eq!(new.x.to_bits(), old.x.to_bits());
        assert_eq!(new.y.to_bits(), old.y.to_bits());
    }
}
#[test]
fn active_alpha_culling_has_real_backside_influence_when_disabled() {
    let mut original = mesh();
    for triangle in &mut original.triangles {
        triangle.swap(1, 2);
    }
    original.recompute_normals();
    let state = Arc::new(SourceGeometryNormalState::seed(&original).unwrap());
    let before = words(&original);
    let mut blocked = original.clone();
    let mut o = brush();
    o.culling = true;
    let mut stroke = SourceGeometryStroke::new(o).unwrap();
    let (r, _) = apply(
        &mut blocked,
        &state,
        &mut stroke,
        [Some(pass(0.5)), None],
        Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
        false,
    );
    assert_eq!(r.combined.changed_vertices, 0);
    assert_eq!(words(&blocked), before);
    let mut stroke = SourceGeometryStroke::new(brush()).unwrap();
    let (r, _) = apply(
        &mut original,
        &state,
        &mut stroke,
        [Some(pass(0.5)), None],
        Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
        false,
    );
    assert_eq!(r.combined.changed_vertices, 1);
    assert_eq!(
        original.positions[0].z.to_bits(),
        ((-81.0f64 / 1360.0) as f32).to_bits()
    );
}
