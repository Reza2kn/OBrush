//! Independent literal geometry/normal/cache/history fixtures. Source-only
//! authored against frozen34-gate proposal; no product output defines expected
//! coordinates. Actual UI/reference/pressure/GPU gates remain separate.
use obrush_core::*;
use std::sync::Arc;

fn mesh(points: &[[f32; 3]], faces: &[[u32; 3]], u: &[f32]) -> Mesh {
    let obj = points
        .iter()
        .map(|p| format!("v {} {} {}\n", p[0], p[1], p[2]))
        .collect::<String>()
        + &faces
            .iter()
            .map(|f| format!("f {} {} {}\n", f[0] + 1, f[1] + 1, f[2] + 1))
            .collect::<String>();
    let mut m = Mesh::from_obj(&obj).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.copy_from_slice(u);
    m
}
fn radial() -> Mesh {
    let mut p = vec![[-0.25, -0.25, 0.], [0.25, -0.25, 0.], [0., 0.25, 0.]];
    let mut f = vec![[0, 1, 2]];
    let mut u = vec![0.; 3];
    for x in [0., 0.5, 1., 1.5, 2.] {
        let i = p.len() as u32;
        p.extend([[x, 0., 0.], [x + 4., 0., 0.], [x, 0.25, 0.]]);
        f.push([i, i + 1, i + 2]);
        u.extend([1., 0., 0.]);
    }
    mesh(&p, &f, &u)
}
fn cache_fixture() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [4., 0., 0.],
            [0., 4., 0.],
            [-4., 4., 0.],
            [0., 8., 0.],
        ],
        &[[0, 1, 2], [2, 4, 3]],
        &[1.; 5],
    )
}
fn clay() -> Mesh {
    mesh(
        &[
            [-1.25, 0., 0.],
            [2.75, 0., 0.],
            [-1.25, 0.5, 0.],
            [1.25, 0., 0.],
            [5.25, 0., 0.],
            [1.25, 1., 0.],
            [0., 0., -0.625],
            [4., 0., -0.625],
            [0., -0.75, -0.625],
        ],
        &[[0, 1, 2], [3, 4, 5], [6, 7, 8]],
        &[1., 0., 0., 0.5, 0., 0., 1., 0., 0.],
    )
}
fn options(accumulate: bool) -> SourceGeometryOptions {
    SourceGeometryOptions {
        tool: SourceGeometryTool::Brush {
            clay: false,
            accumulate,
        },
        intensity: 0.625,
        ..Default::default()
    }
}
fn pass() -> SourceGeometryPass {
    SourceGeometryPass {
        center: [0.; 3],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    }
}
fn words(m: &Mesh) -> Vec<[u32; 3]> {
    m.positions
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect()
}
fn apply(
    d: &mut Document,
    s: &mut SourceGeometryStroke,
    p: SourceGeometryPass,
) -> SourceGeometryReport {
    d.apply_source_geometry_dab(s, p, 0.5, true).unwrap()
}

#[test]
fn opt_in_defaults_and_legacy_are_distinct() {
    assert_eq!(
        SourceGeometryOptions::default().tool,
        SourceGeometryTool::Brush {
            clay: true,
            accumulate: true
        }
    );
    assert_eq!(SourceGeometryOptions::default().intensity, 0.5);
    assert!(!SourceGeometryOptions::default().culling);
    assert_eq!(SourceGeometryOptions::inflate().intensity, 0.3);
    assert_eq!(BrushSettings::default().kind, BrushKind::Draw);
    assert!(!BrushSettings::default().draw_clay);
    assert!(BrushSettings::default().backface_culling);
}
#[test]
fn polynomial_literal_f32_words_and_protected_boundary() {
    let mut d = Document::new(radial());
    let old = words(d.active_mesh().unwrap());
    let old_masks = d
        .active_mesh()
        .unwrap()
        .masks
        .iter()
        .map(|v| v.to_bits())
        .collect::<Vec<_>>();
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    let r = apply(&mut d, &mut s, pass());
    assert_eq!(r.changed_vertices, 4);
    assert_eq!(r.displacement_scale, 1.);
    let m = d.active_mesh().unwrap();
    let expected = [0x3e000000, 0x3df30000, 0x3db00000, 0x3d060000, 0];
    for (v, w) in [3, 6, 9, 12, 15].into_iter().zip(expected) {
        assert_eq!(m.positions[v].z.to_bits(), w);
    }
    for (v, p) in old.iter().enumerate() {
        if ![3, 6, 9, 12].contains(&v) {
            assert_eq!(m.positions[v].to_array().map(f32::to_bits), *p);
        }
    }
    assert_eq!(
        m.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        old_masks
    );
    assert!(m.materials.is_none());
    d.end_source_geometry_stroke(&s).unwrap();
}
#[test]
fn inverse_plain_brush_reverses_literal_displacements() {
    let mut d = Document::new(radial());
    let mut o = options(true);
    o.negative = true;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    assert_eq!(apply(&mut d, &mut s, pass()).displacement_scale, 1.);
    assert_eq!(
        d.active_mesh().unwrap().positions[3].z.to_bits(),
        0xbe000000
    );
    assert_eq!(
        d.active_mesh().unwrap().positions[6].z.to_bits(),
        0xbdf30000
    );
}
#[test]
fn non_accumulate_freezes_distance_but_still_adds_to_live_surface() {
    let mut d = Document::new(radial());
    let mut s = d.begin_source_geometry_stroke(options(false)).unwrap();
    apply(&mut d, &mut s, pass());
    apply(&mut d, &mut s, pass());
    assert_eq!(
        d.active_mesh().unwrap().positions[3].z.to_bits(),
        0x3e800000
    );
    assert_eq!(s.proxy_positions()[3].z.to_bits(), 0);
    d.end_source_geometry_stroke(&s).unwrap();
}
#[test]
fn accumulate_measures_live_radius_with_distinct_second_literal() {
    let mut d = Document::new(radial());
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut s, pass());
    apply(&mut d, &mut s, pass());
    assert_eq!(
        d.active_mesh().unwrap().positions[3].z.to_bits(),
        0x3e7fe180
    );
}
#[test]
fn new_stroke_resets_proxy_to_current_positions() {
    let mut d = Document::new(radial());
    let mut first = d.begin_source_geometry_stroke(options(false)).unwrap();
    apply(&mut d, &mut first, pass());
    apply(&mut d, &mut first, pass());
    d.end_source_geometry_stroke(&first).unwrap();
    let mut next = d.begin_source_geometry_stroke(options(false)).unwrap();
    apply(&mut d, &mut next, pass());
    assert_eq!(next.proxy_positions()[3].z, 0.25);
    assert_eq!(d.active_mesh().unwrap().positions[3].z, 12259. / 32768.);
}
#[test]
fn clay_front_plane_and_backside_edit_when_culling_is_off() {
    let mut d = Document::new(clay());
    let old = d.active_mesh().unwrap().positions.clone();
    let mut s = d
        .begin_source_geometry_stroke(SourceGeometryOptions::default())
        .unwrap();
    let p = SourceGeometryPass {
        local_radius_squared: 25.,
        ..pass()
    };
    let r = apply(&mut d, &mut s, p);
    assert_eq!(r.changed_vertices, 3);
    assert_eq!(r.displacement_scale, 1.);
    assert_eq!(d.active_mesh().unwrap().positions[0].z, 243. / 1024.);
    assert_eq!(d.active_mesh().unwrap().positions[3].z, 243. / 2048.);
    assert_eq!(
        d.active_mesh().unwrap().positions[6].z - old[6].z,
        36603. / 65536.
    );
    for v in [1, 2, 4, 5, 7, 8] {
        assert_eq!(d.active_mesh().unwrap().positions[v], old[v]);
    }
}
#[test]
fn clay_culling_on_retains_backside_and_same_front_plane() {
    let mut d = Document::new(clay());
    let o = SourceGeometryOptions {
        culling: true,
        ..Default::default()
    };
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    assert_eq!(
        apply(
            &mut d,
            &mut s,
            SourceGeometryPass {
                local_radius_squared: 25.,
                ..pass()
            }
        )
        .changed_vertices,
        2
    );
    assert_eq!(d.active_mesh().unwrap().positions[6].z, -0.625);
    assert_eq!(d.active_mesh().unwrap().positions[0].z, 243. / 1024.);
}
#[test]
fn negative_clay_is_an_opposite_one_sided_plane() {
    let mut d = Document::new(clay());
    let o = SourceGeometryOptions {
        negative: true,
        ..Default::default()
    };
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    assert_eq!(
        apply(
            &mut d,
            &mut s,
            SourceGeometryPass {
                local_radius_squared: 25.,
                ..pass()
            }
        )
        .changed_vertices,
        2
    );
    assert_eq!(d.active_mesh().unwrap().positions[0].z, -243. / 1024.);
    assert_eq!(d.active_mesh().unwrap().positions[6].z, -0.625);
}
#[test]
fn inflate_selected_refresh_preserves_unselected_raw_rows() {
    let mut d = Document::new(cache_fixture());
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    let r = apply(&mut d, &mut s, pass());
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(d.active_mesh().unwrap().positions[0].z, 0.125);
    let id = d.active_subtool_id().unwrap();
    let field = d.source_geometry_state(id).unwrap();
    assert_eq!(field.face_normals()[0], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(field.vertex_normals()[0], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(field.vertex_normals()[1], Vec3::new(0., 0., 16.));
    assert_eq!(field.vertex_normals()[2], Vec3::new(0., 0., 16.));
    let fresh = SourceNormalField::compute(
        d.active_mesh().unwrap(),
        SourceNormalPolicy::StaticOriginalFaces,
    )
    .unwrap();
    assert_eq!(fresh.vertex_normals()[1], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(fresh.vertex_normals()[2], Vec3::new(0.25, 0.25, 16.));
    let second = SourceGeometryPass {
        center: [4., 0., 0.],
        ..pass()
    };
    assert_eq!(apply(&mut d, &mut s, second).changed_vertices, 1);
    let next = d.source_geometry_state(id).unwrap();
    assert_eq!(d.active_mesh().unwrap().positions[1].z, 0.125);
    assert_eq!(next.vertex_normals()[1], Vec3::new(0., 0.5, 16.));
    assert_eq!(next.vertex_normals()[0], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(next.vertex_normals()[2], Vec3::new(0., 0., 16.));
}
#[test]
fn warm_cache_survives_release_next_press_and_exact_history_swap() {
    let mut d = Document::new(cache_fixture());
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    apply(&mut d, &mut s, pass());
    d.end_source_geometry_stroke(&s).unwrap();
    let id = d.active_subtool_id().unwrap();
    let old = d.source_geometry_state(id).unwrap();
    let before = d.to_json().unwrap();
    let mut next = d.begin_source_geometry_stroke(o).unwrap();
    apply(
        &mut d,
        &mut next,
        SourceGeometryPass {
            center: [4., 0., 0.],
            ..pass()
        },
    );
    d.end_source_geometry_stroke(&next).unwrap();
    let after = d.to_json().unwrap();
    let newer = d.source_geometry_state(id).unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(Arc::ptr_eq(&old, &d.source_geometry_state(id).unwrap()));
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(Arc::ptr_eq(&newer, &d.source_geometry_state(id).unwrap()));
}
#[test]
fn cancelling_restores_exact_document_and_cached_neighbor_rows() {
    let mut d = Document::new(cache_fixture());
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    apply(&mut d, &mut s, pass());
    d.end_source_geometry_stroke(&s).unwrap();
    let id = d.active_subtool_id().unwrap();
    let field = d.source_geometry_state(id).unwrap();
    let before = d.to_json().unwrap();
    let mut next = d.begin_source_geometry_stroke(o).unwrap();
    apply(
        &mut d,
        &mut next,
        SourceGeometryPass {
            center: [4., 0., 0.],
            ..pass()
        },
    );
    d.cancel_source_geometry_stroke(&next).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(Arc::ptr_eq(&field, &d.source_geometry_state(id).unwrap()));
}
#[test]
fn zero_strength_no_contact_and_all_protected_preserve_real_redo() {
    for which in 0..3 {
        let mut d = Document::new(radial());
        let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
        apply(&mut d, &mut s, pass());
        d.end_source_geometry_stroke(&s).unwrap();
        let after = d.to_json().unwrap();
        assert!(d.undo());
        if which == 2 {
            d.active_mesh_mut().unwrap().masks.fill(0.);
        }
        let before = d.to_json().unwrap();
        let mut o = options(true);
        if which == 0 {
            o.intensity = 0.;
        }
        let mut noop = d.begin_source_geometry_stroke(o).unwrap();
        assert_eq!(
            d.apply_source_geometry_dab(&mut noop, pass(), 0.5, which != 1)
                .unwrap()
                .changed_vertices,
            0
        );
        d.end_source_geometry_stroke(&noop).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
        assert!(d.can_redo());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
}
#[test]
fn exact_vertex_plain_brush_rejects_without_barycentric_fallback() {
    let mut d = Document::new(cache_fixture());
    let before = d.to_json().unwrap();
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    assert!(matches!(
        d.apply_source_geometry_dab(&mut s, pass(), 0.5, true),
        Err(SourceGeometryError::UndefinedPickedNormal(
            SourcePickedNormalUndefined::ExactVertexContact {
                corner: 0,
                vertex_id: 0
            }
        ))
    ));
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    d.cancel_source_geometry_stroke(&s).unwrap();
}
#[test]
fn source_raw_tiny_u_is_not_reinterpreted_as_legacy_protection() {
    let mut m = radial();
    m.masks.fill(0.);
    m.masks[3] = f32::from_bits(1);
    let mut d = Document::new(m);
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    let r = apply(
        &mut d,
        &mut s,
        SourceGeometryPass {
            local_radius_squared: 1_000_000_000_000.,
            ..pass()
        },
    );
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(d.active_mesh().unwrap().positions[3].z.to_bits(), 62500);
    assert_eq!(d.active_mesh().unwrap().masks[3].to_bits(), 1);
}
#[test]
fn source_cache_is_transient_on_serialized_reload() {
    let mut d = Document::new(cache_fixture());
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    apply(&mut d, &mut s, pass());
    d.end_source_geometry_stroke(&s).unwrap();
    let saved = d.to_json().unwrap();
    assert!(d.source_geometry_retained_bytes() > 0);
    let reloaded = Document::from_json(&saved).unwrap();
    assert_eq!(reloaded.source_geometry_retained_bytes(), 0);
    assert_eq!(reloaded.to_json().unwrap(), saved);
}
#[test]
fn stale_position_field_rejects_and_explicit_reset_is_required() {
    let mut d = Document::new(radial());
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut s, pass());
    d.end_source_geometry_stroke(&s).unwrap();
    d.active_mesh_mut().unwrap().positions[3].x = 0.0625;
    let before = d.to_json().unwrap();
    assert_eq!(
        d.begin_source_geometry_stroke(options(true)).err(),
        Some(SourceGeometryError::StaleNormalState)
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.reset_source_geometry_normal_state().unwrap());
    let s = d.begin_source_geometry_stroke(options(true)).unwrap();
    d.cancel_source_geometry_stroke(&s).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn in_stroke_attributes_and_legacy_normal_words_are_not_swallowed() {
    for which in 0..3 {
        let mut d = Document::new(radial());
        let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
        apply(&mut d, &mut s, pass());
        match which {
            0 => d.active_mesh_mut().unwrap().masks[3] = 0.5,
            1 => d.active_mesh_mut().unwrap().colors[3][0] = 0.5,
            _ => d.active_mesh_mut().unwrap().normals[3].x = 0.25,
        };
        let before = d.to_json().unwrap();
        assert_eq!(
            d.apply_source_geometry_dab(&mut s, pass(), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(d.to_json().unwrap(), before);
    }
}
#[test]
fn ended_handle_cannot_mutate_a_later_open_stroke() {
    let mut d = Document::new(radial());
    let mut old = d.begin_source_geometry_stroke(options(true)).unwrap();
    d.end_source_geometry_stroke(&old).unwrap();
    let next = d.begin_source_geometry_stroke(options(true)).unwrap();
    let before = d.to_json().unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut old, pass(), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(d.to_json().unwrap(), before);
    d.cancel_source_geometry_stroke(&next).unwrap();
}
#[test]
fn malformed_input_and_original_face_are_atomic() {
    for which in 0..4 {
        let mut d = Document::new(radial());
        let before = d.to_json().unwrap();
        let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
        let mut p = pass();
        match which {
            0 => p.center[0] = f64::NAN,
            1 => p.eye_direction = [0.; 3],
            2 => p.local_radius_squared = f64::INFINITY,
            _ => p.original_face_index = 99,
        };
        assert!(d.apply_source_geometry_dab(&mut s, p, 0.5, true).is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
    }
}
#[test]
fn raw_tablet_intensity_factor_uses_source_mouse_midpoint() {
    let mut d = Document::new(radial());
    let mut o = options(true);
    o.intensity_factor = 1.;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, pass(), 0.25, true)
            .unwrap()
            .changed_vertices,
        4
    );
    assert_eq!(
        d.active_mesh().unwrap().positions[3].z.to_bits(),
        0x3d800000
    );
}
#[test]
fn geometry_quality_guard_limits_extreme_proposed_source_displacement() {
    let mut d = Document::new(radial());
    let old = d.active_mesh().unwrap().positions.clone();
    let mut o = options(true);
    o.intensity = 1.;
    let mut s = d.begin_source_geometry_stroke(o).unwrap();
    let r = apply(
        &mut d,
        &mut s,
        SourceGeometryPass {
            local_radius_squared: 1_000_000_000_000.,
            ..pass()
        },
    );
    assert!(r.displacement_scale < 1.);
    assert!(r.displacement_scale >= 0.);
    d.active_mesh().unwrap().validate().unwrap();
    for v in [0, 1, 2, 4, 5, 7, 8, 10, 11, 13, 14, 16, 17] {
        assert_eq!(d.active_mesh().unwrap().positions[v], old[v]);
    }
    for t in &d.active_mesh().unwrap().triangles {
        let [a, b, c] = t.map(|v| old[v as usize].as_dvec3());
        let before = (b - a).cross(c - a);
        let [a, b, c] = t.map(|v| d.active_mesh().unwrap().positions[v as usize].as_dvec3());
        let after = (b - a).cross(c - a);
        assert!(after.dot(before) >= before.length_squared() * 0.05);
    }
}
#[test]
fn clay_scope_larger_than_single_leaf_is_a_visible_typed_blocker() {
    let mut m = radial();
    while m.triangles.len() <= 100 {
        m.triangles.push(m.triangles[0]);
    }
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    assert_eq!(
        d.begin_source_geometry_stroke(SourceGeometryOptions::default())
            .err(),
        Some(SourceGeometryError::UnsupportedClayOrder)
    );
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn non_source_conventions_are_not_implicitly_converted() {
    let mut m = radial();
    m.mask_convention = MaskConvention::ProtectedF32V1;
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    assert_eq!(
        d.begin_source_geometry_stroke(options(true)).err(),
        Some(SourceGeometryError::MaskConvention)
    );
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn cache_storage_is_charged_and_undo_restores_absence() {
    let mut d = Document::new(radial());
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut s, pass());
    d.end_source_geometry_stroke(&s).unwrap();
    let id = d.active_subtool_id().unwrap();
    let cache = d.source_geometry_state(id).unwrap();
    assert!(d.source_geometry_retained_bytes() >= cache.approximate_bytes());
    assert!(d.source_geometry_retained_bytes() <= MAX_SOURCE_GEOMETRY_STATE_BYTES);
    assert!(d.undo());
    assert!(d.source_geometry_state(id).is_none());
    assert!(d.redo());
    assert!(Arc::ptr_eq(&cache, &d.source_geometry_state(id).unwrap()));
}

#[test]
fn warm_cache_only_refresh_publishes_new_identity_but_preserves_real_redo() {
    let mut d = Document::new(cache_fixture());
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut first = d.begin_source_geometry_stroke(o).unwrap();
    apply(&mut d, &mut first, pass());
    d.end_source_geometry_stroke(&first).unwrap();
    let second_pass = SourceGeometryPass {
        center: [4., 0., 0.],
        ..pass()
    };
    let mut second = d.begin_source_geometry_stroke(o).unwrap();
    apply(&mut d, &mut second, second_pass);
    d.end_source_geometry_stroke(&second).unwrap();
    let genuine_redo = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let id = d.active_subtool_id().unwrap();
    let old = d.source_geometry_state(id).unwrap();
    assert_eq!(old.vertex_normals()[1], Vec3::new(0., 0., 16.));
    assert!(d.can_redo());
    o.intensity = 0.;
    let mut noop = d.begin_source_geometry_stroke(o).unwrap();
    let report = apply(&mut d, &mut noop, second_pass);
    assert_eq!(report.changed_vertices, 0);
    assert!(report.derived_cache_changed);
    assert_eq!(report.noop, Some(SourceGeometryNoop::ZeroIntensity));
    assert!(noop.proxy_positions().is_empty());
    let refreshed = d.source_geometry_state(id).unwrap();
    assert!(!Arc::ptr_eq(&old, &refreshed));
    assert!(refreshed.generation() > old.generation());
    assert_eq!(refreshed.vertex_normals()[1], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(refreshed.vertex_normals()[2], Vec3::new(0., 0., 16.));
    assert_eq!(old.vertex_normals()[1], Vec3::new(0., 0., 16.));
    d.end_source_geometry_stroke(&noop).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), genuine_redo);
}

#[test]
fn finite_zero_picked_direction_is_a_noop_not_a_fallback_normal() {
    let m = mesh(
        &[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &[[0, 1, 2], [0, 2, 1]],
        &[1.; 3],
    );
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    let report = apply(&mut d, &mut s, pass());
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(report.noop, Some(SourceGeometryNoop::ZeroPickedDirection));
    assert!(matches!(
        report.picked_normal,
        Some(SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ZeroDirection,
            ..
        })
    ));
    d.end_source_geometry_stroke(&s).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    assert!(s.proxy_positions().is_empty());
}

#[test]
fn zero_selected_inflate_normal_rejects_atomically() {
    let m = mesh(
        &[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &[[0, 1, 2], [0, 2, 1]],
        &[1.; 3],
    );
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
        .unwrap();
    assert!(matches!(
        d.apply_source_geometry_dab(&mut s, pass(), 0.5, true),
        Err(SourceGeometryError::UndefinedInflateNormal { .. })
    ));
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    d.cancel_source_geometry_stroke(&s).unwrap();
}

#[test]
fn clay_without_weighted_area_direction_does_not_refresh_or_edit() {
    let m = mesh(
        &[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &[[0, 1, 2], [0, 2, 1]],
        &[1.; 3],
    );
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_geometry_stroke(SourceGeometryOptions::default())
        .unwrap();
    let r = apply(&mut d, &mut s, pass());
    assert_eq!(r.noop, Some(SourceGeometryNoop::NoAreaDirection));
    assert_eq!(r.source_normal_rows_refreshed, 0);
    assert!(!r.derived_cache_changed);
    d.end_source_geometry_stroke(&s).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}

#[test]
fn normal_caches_are_per_object_and_selection_history_preserves_them() {
    let mut d = Document::new(radial());
    let first = d.active_subtool_id().unwrap();
    let mut a = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut a, pass());
    d.end_source_geometry_stroke(&a).unwrap();
    let old = d.source_geometry_state(first).unwrap();
    let second = d
        .add_subtool_preserving_normals(radial(), "Second")
        .unwrap();
    let mut b = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut b, pass());
    d.end_source_geometry_stroke(&b).unwrap();
    let newer = d.source_geometry_state(second).unwrap();
    assert!(!Arc::ptr_eq(&old, &newer));
    d.select_subtool(first).unwrap();
    assert!(Arc::ptr_eq(&old, &d.source_geometry_state(first).unwrap()));
    d.select_subtool(second).unwrap();
    assert!(Arc::ptr_eq(
        &newer,
        &d.source_geometry_state(second).unwrap()
    ));
}

#[test]
fn source_geometry_keeps_original_faces_uv_material_and_mask_words() {
    let mut m = radial();
    let raw_faces = m
        .triangles
        .iter()
        .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
        .collect::<Vec<_>>();
    m.surface = Some(SurfaceTopology {
        faces: raw_faces.clone(),
        uv: Some(CornerUv {
            coordinates: vec![[0., 0.], [1., 0.], [0., 1.]],
            face_indices: raw_faces
                .iter()
                .map(|_| [0, 1, 2, TRIANGLE_CORNER])
                .collect(),
        }),
    });
    m.materials = Some(vec![[0.18, 0.08]; m.positions.len()]);
    let attrs = serde_json::to_value(&m).unwrap();
    let mut d = Document::new(m);
    let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut s, pass());
    let after = serde_json::to_value(d.active_mesh().unwrap()).unwrap();
    for key in [
        "masks",
        "mask_convention",
        "colors",
        "materials",
        "surface",
        "triangles",
    ] {
        assert_eq!(after[key], attrs[key]);
    }
}

#[test]
fn before_first_dab_public_word_mutations_are_rejected() {
    for which in 0..3 {
        let mut d = Document::new(radial());
        let mut s = d.begin_source_geometry_stroke(options(true)).unwrap();
        match which {
            0 => d.active_mesh_mut().unwrap().positions[3].x = 0.125,
            1 => d.active_mesh_mut().unwrap().normals[3].x = 0.25,
            _ => d.active_mesh_mut().unwrap().colors[3][0] = 0.5,
        };
        let before = d.to_json().unwrap();
        assert_eq!(
            d.apply_source_geometry_dab(&mut s, pass(), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(d.to_json().unwrap(), before);
    }
}

#[test]
fn removed_and_cleared_cache_rows_restore_exactly_through_whole_history() {
    let mut d = Document::new(radial());
    let first = d.active_subtool_id().unwrap();
    let mut a = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut a, pass());
    d.end_source_geometry_stroke(&a).unwrap();
    let first_cache = d.source_geometry_state(first).unwrap();
    let second = d
        .add_subtool_preserving_normals(radial(), "Second")
        .unwrap();
    let mut b = d.begin_source_geometry_stroke(options(true)).unwrap();
    apply(&mut d, &mut b, pass());
    d.end_source_geometry_stroke(&b).unwrap();
    let second_cache = d.source_geometry_state(second).unwrap();
    let before_delete = d.to_json().unwrap();
    d.delete_subtool(first).unwrap();
    assert!(d.source_geometry_state(first).is_none());
    assert!(Arc::ptr_eq(
        &second_cache,
        &d.source_geometry_state(second).unwrap()
    ));
    let after_delete = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_delete);
    assert!(Arc::ptr_eq(
        &first_cache,
        &d.source_geometry_state(first).unwrap()
    ));
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after_delete);
    assert!(d.source_geometry_state(first).is_none());
    assert!(d.clear_scene().unwrap());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    assert!(d.source_geometry_state(second).is_none());
    let empty = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), after_delete);
    assert!(Arc::ptr_eq(
        &second_cache,
        &d.source_geometry_state(second).unwrap()
    ));
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), empty);
    let new_id = d.add_subtool_preserving_normals(radial(), "New").unwrap();
    assert_ne!(new_id, second);
    assert!(d.source_geometry_state(new_id).is_none());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}
