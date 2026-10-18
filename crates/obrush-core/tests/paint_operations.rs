// The independent shared fixture also defines the separate baseline driver.
#[allow(dead_code)]
#[path = "common/paint_fixture.rs"]
mod fixture;
use obrush_core::{
    BrushKind, BrushSettings, CornerUv, Document, Hit, Mesh, PaintPickInterpolation as Mode, Ray,
    SurfaceTopology, Vec3, DEFAULT_VERTEX_MATERIAL,
};

const ERROR: f64 = 2e-7;
fn quad() -> Mesh {
    let mut m = Mesh::cube();
    m.positions = vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
    m.triangles = vec![[0, 1, 2], [0, 2, 3]];
    m.colors = vec![
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    m.masks = vec![0.0, 0.25, 0.5, 1.0];
    m.materials = Some(vec![[0.0, 1.0], [0.25, 0.5], [0.5, 0.25], [1.0, 0.0]]);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3]],
        uv: Some(CornerUv {
            coordinates: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            face_indices: vec![[0, 1, 2, 3]],
        }),
    });
    m.recompute_normals();
    m.validate().unwrap();
    m
}
fn options() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Paint,
        paint_rgb: true,
        paint_roughness: true,
        paint_metallic: true,
        color: [0.375, 0.625, 0.75],
        roughness: 0.8,
        metallic: 0.6,
        ..Default::default()
    }
}
fn hit(m: &Mesh, index: usize, weights: [f32; 3]) -> Hit {
    let triangle = m.triangles[index];
    let point = triangle
        .into_iter()
        .zip(weights)
        .fold(Vec3::ZERO, |p, (v, w)| p + m.positions[v as usize] * w);
    Hit {
        position: point,
        normal: Vec3::Z,
        distance: 1.0,
        triangle,
        barycentric: weights,
    }
}
fn scalar(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() <= ERROR,
        "actual{actual}, expected{expected}"
    );
}
fn fill_history(d: &mut Document, s: &BrushSettings) {
    d.begin_material_paint_stroke();
    assert!(
        d.require_active_mesh_mut()
            .unwrap()
            .fill_paint_channels(s)
            .unwrap()
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
}

#[test]
fn all_eight_fill_channel_choices_follow_known_mask_only_formula_and_union_counts() {
    for flags in 0..8 {
        let mut m = quad();
        let old = m.snapshot();
        let mut s = options();
        s.paint_rgb = flags & 1 != 0;
        s.paint_roughness = flags & 2 != 0;
        s.paint_metallic = flags & 4 != 0;
        let r = m.fill_paint_channels(&s).unwrap();
        assert_eq!(r.changed_vertices, if flags == 0 { 0 } else { 3 });
        assert_eq!(r.changed_rgb_vertices, if s.paint_rgb { 3 } else { 0 });
        assert_eq!(
            r.changed_material_vertices,
            if s.paint_roughness || s.paint_metallic {
                3
            } else {
                0
            }
        );
        assert!(!r.materialized);
        for i in 0..4 {
            let w = 1.0 - f64::from(old.masks[i]);
            for c in 0..3 {
                if s.paint_rgb {
                    scalar(
                        m.colors[i][c],
                        f64::from(old.colors[i][c]) * (1.0 - w) + f64::from(s.color[c]) * w,
                    );
                } else {
                    assert_eq!(m.colors[i][c].to_bits(), old.colors[i][c].to_bits());
                }
            }
            for (c, enabled) in [s.paint_roughness, s.paint_metallic]
                .into_iter()
                .enumerate()
            {
                let before = old.materials.as_ref().unwrap()[i][c];
                let after = m.materials.as_ref().unwrap()[i][c];
                if enabled {
                    scalar(
                        after,
                        f64::from(before) * (1.0 - w) + f64::from([s.roughness, s.metallic][c]) * w,
                    );
                } else {
                    assert_eq!(after.to_bits(), before.to_bits());
                }
            }
        }
        assert_eq!(m.positions, old.positions);
        assert_eq!(m.normals, old.normals);
        assert_eq!(m.triangles, old.triangles);
        assert_eq!(m.surface, old.surface);
        assert_eq!(m.masks, old.masks);
    }
}

#[test]
fn fill_ignores_radius_strength_pressure_alpha_hardness_culling_and_symmetry() {
    let mut a = quad();
    let mut b = a.snapshot();
    let base = options();
    let mut ignored = base.clone();
    ignored.radius = f32::NAN;
    ignored.strength = 0.0;
    ignored.pressure = 0.0;
    ignored.hardness = f32::NAN;
    ignored.symmetry_offset = f32::INFINITY;
    ignored.alpha_rotation = f32::NAN;
    ignored.culling_axis = Some(Vec3::ZERO);
    ignored.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![0]).unwrap());
    assert_eq!(
        a.fill_paint_channels(&base).unwrap(),
        b.fill_paint_channels(&ignored).unwrap()
    );
    assert_eq!(fixture::bits(&a), fixture::bits(&b));
}

#[test]
fn all_masked_and_default_target_fills_keep_none_storage_and_redo() {
    for scenario in 0..2 {
        let mut d = Document::new(fixture::sheet());
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[0][0] = 0.1;
        d.end_stroke();
        assert!(d.undo());
        let mut s = options();
        s.paint_rgb = false;
        if scenario == 0 {
            d.require_active_mesh_mut().unwrap().masks.fill(1.0);
        } else {
            s.roughness = DEFAULT_VERTEX_MATERIAL[0];
            s.metallic = DEFAULT_VERTEX_MATERIAL[1];
        }
        let before = d.to_json().unwrap();
        d.begin_material_paint_stroke();
        assert_eq!(
            d.require_active_mesh_mut()
                .unwrap()
                .fill_paint_channels(&s)
                .unwrap()
                .changed_vertices,
            0
        );
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(d.can_redo());
    }
}

#[test]
fn no_channels_fill_is_an_exact_noop() {
    let mut m = quad();
    let before = fixture::bits(&m);
    let mut s = options();
    s.paint_rgb = false;
    s.paint_roughness = false;
    s.paint_metallic = false;
    assert_eq!(m.fill_paint_channels(&s).unwrap().changed_vertices, 0);
    assert_eq!(fixture::bits(&m), before);
}

#[test]
fn fill_uses_canonical_source_weights_without_losing_tiny_unprotected_values() {
    let mut m = quad();
    m.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    m.masks.fill(0.0);
    let tiny = 1e-9_f32;
    m.masks[0] = tiny;
    m.colors[0] = [0.0; 3];
    m.materials.as_mut().unwrap()[0] = [0.0; 2];
    let before = m.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
    let mut s = options();
    s.color = [1.0; 3];
    s.roughness = 1.0;
    s.metallic = 1.0;
    let r = m.fill_paint_channels(&s).unwrap();
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(m.colors[0].map(f32::to_bits), [tiny.to_bits(); 3]);
    assert_eq!(
        m.materials.as_ref().unwrap()[0].map(f32::to_bits),
        [tiny.to_bits(); 2]
    );
    assert_eq!(
        m.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        before
    );
}

#[test]
fn fill_on_exactly_equivalent_mask_conventions_has_equal_paint_and_keeps_both_raw_channels() {
    let mut protected = quad();
    let mut source = protected.snapshot();
    source.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    source.masks = protected.masks.iter().map(|v| 1.0 - v).collect();
    let protected_masks = protected.masks.clone();
    let source_masks = source.masks.clone();
    let a = protected.fill_paint_channels(&options()).unwrap();
    let b = source.fill_paint_channels(&options()).unwrap();
    assert_eq!(a, b);
    assert_eq!(protected.colors, source.colors);
    assert_eq!(protected.materials, source.materials);
    assert_eq!(protected.masks, protected_masks);
    assert_eq!(source.masks, source_masks);
}

#[test]
fn equal_and_disabled_signed_zero_scalar_channels_survive_other_fill_edits() {
    for selected in [false, true] {
        let mut m = quad();
        for p in m.materials.as_mut().unwrap() {
            p[0] = -0.0;
        }
        let mut s = options();
        s.paint_rgb = false;
        s.paint_roughness = selected;
        s.roughness = 0.0;
        assert!(m.fill_paint_channels(&s).unwrap().changed_vertices > 0);
        assert!(m
            .materials
            .unwrap()
            .iter()
            .all(|p| p[0].to_bits() == (-0.0_f32).to_bits()));
    }
}

#[test]
fn first_material_fill_allocates_only_after_real_delta_with_default_protected_points() {
    let mut m = quad();
    m.materials = None;
    let mut s = options();
    s.paint_rgb = false;
    s.paint_metallic = false;
    let r = m.fill_paint_channels(&s).unwrap();
    assert!(r.materialized);
    assert_eq!(r.changed_material_vertices, 3);
    let values = m.materials.unwrap();
    assert_eq!(
        values[3].map(f32::to_bits),
        DEFAULT_VERTEX_MATERIAL.map(f32::to_bits)
    );
    assert!(values
        .iter()
        .all(|p| p[1].to_bits() == DEFAULT_VERTEX_MATERIAL[1].to_bits()));
}

#[test]
fn invalid_targets_and_nonpaint_kind_reject_before_any_fill_mutation() {
    for scenario in 0..6 {
        let mut m = quad();
        let before = fixture::bits(&m);
        let mut s = options();
        match scenario {
            0 => s.roughness = f32::NAN,
            1 => s.metallic = 1.1,
            2 => s.color[0] = f32::INFINITY,
            3 => s.kind = BrushKind::Draw,
            4 => {
                s.paint_roughness = false;
                s.roughness = -0.1
            }
            5 => {
                s.paint_metallic = false;
                s.metallic = f32::NEG_INFINITY
            }
            _ => unreachable!(),
        };
        assert!(m.fill_paint_channels(&s).is_err());
        assert_eq!(fixture::bits(&m), before);
    }
}

#[test]
fn malformed_mesh_channels_reject_atomically_before_fill() {
    for scenario in 0..3 {
        let mut m = quad();
        match scenario {
            0 => {
                m.materials.as_mut().unwrap().pop();
            }
            1 => m.masks[0] = f32::NAN,
            2 => m.surface.as_mut().unwrap().faces[0][3] = 99,
            _ => unreachable!(),
        };
        let before = fixture::bits(&m);
        assert!(m.fill_paint_channels(&options()).is_err());
        assert_eq!(fixture::bits(&m), before);
    }
}

#[test]
fn fill_storage_limit_precedes_rgb_and_material_edits() {
    let mut m = fixture::sheet();
    let noncolor = std::mem::size_of::<Mesh>()
        + m.positions.capacity() * std::mem::size_of::<Vec3>()
        + m.normals.capacity() * std::mem::size_of::<Vec3>()
        + m.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
        + m.masks.capacity() * std::mem::size_of::<f32>();
    let bytes = m.positions.len() * 8;
    let mut colors = Vec::with_capacity(
        (obrush_core::resource_limits::MAX_SCENE_STORAGE_BYTES - noncolor - bytes / 2) / 12,
    );
    colors.extend_from_slice(&m.colors);
    m.colors = colors;
    m.validate().unwrap();
    let before = fixture::bits(&m);
    assert!(m.fill_paint_channels(&options()).is_err());
    assert_eq!(fixture::bits(&m), before);
    assert!(m.materials.is_none());
}

#[test]
fn inverse_fill_is_explicit_legacy_grey_and_material_default_erasure() {
    let mut m = quad();
    let old = m.snapshot();
    let mut s = options();
    s.invert = true;
    m.fill_paint_channels(&s).unwrap();
    for i in 0..4 {
        let w = 1.0 - f64::from(old.masks[i]);
        for (c, t) in [0.65_f32, 0.69, 0.74].into_iter().enumerate() {
            scalar(
                m.colors[i][c],
                f64::from(old.colors[i][c]) * (1.0 - w) + f64::from(t) * w,
            );
        }
        for (c, &t) in DEFAULT_VERTEX_MATERIAL.iter().enumerate() {
            scalar(
                m.materials.as_ref().unwrap()[i][c],
                f64::from(old.materials.as_ref().unwrap()[i][c]) * (1.0 - w) + f64::from(t) * w,
            );
        }
    }
}

#[test]
fn one_fill_transaction_restores_rich_quad_attributes_exactly() {
    let mut d = Document::new(quad());
    let before = d.to_json().unwrap();
    fill_history(&mut d, &options());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn first_retained_material_fill_preserves_levels_and_one_whole_history_entry() {
    let mut d = Document::new(fixture::sheet());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    fill_history(&mut d, &options());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_level(0).unwrap();
    assert!(d
        .require_active_mesh()
        .unwrap()
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .all(|p| *p == DEFAULT_VERTEX_MATERIAL));
}

#[test]
fn retained_layer_scene_fill_preserves_other_parts_and_complete_history() {
    let mut d = Document::new(fixture::sheet());
    d.add_subdivision_level().unwrap();
    let layer = d.add_layer("shape").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    let active = d.active_subtool_id();
    let other = d.add_subtool(quad(), "other").unwrap();
    d.select_subtool(active.expect("nonempty fixture active id"))
        .unwrap();
    let before = d.to_json().unwrap();
    fill_history(&mut d, &options());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_subtool(other).unwrap();
    assert_eq!(
        fixture::bits(d.require_active_mesh().unwrap()),
        fixture::bits(&quad())
    );
}

#[test]
fn triangle_barycentric_picker_has_independent_rgb_and_material_oracle() {
    let m = quad();
    let h = hit(&m, 0, [0.25, 0.25, 0.5]);
    let before = fixture::bits(&m);
    let p = m
        .pick_paint_channels(&h, Mode::TriangleBarycentric)
        .unwrap();
    assert_eq!(p.rgb, [0.25, 0.25, 0.5]);
    assert_eq!(p.roughness, 0.3125);
    assert_eq!(p.metallic, 0.5);
    assert_eq!(p.triangle_index, 0);
    assert_eq!(p.original_face_index, Some(0));
    assert_eq!(fixture::bits(&m), before);
}

#[test]
fn original_quad_inverse_distance_uses_fourth_corner_and_differs_from_barycentric() {
    let m = quad();
    let h = hit(&m, 0, [0.5, 0.0, 0.5]);
    let b = m
        .pick_paint_channels(&h, Mode::TriangleBarycentric)
        .unwrap();
    let p = m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_eq!(p.rgb, [0.5, 0.5, 0.5]);
    assert_eq!(p.roughness, 0.4375);
    assert_eq!(p.metallic, 0.4375);
    assert_ne!(p.rgb, b.rgb);
    assert_ne!(p.roughness, b.roughness);
}

#[test]
fn asymmetric_quad_inverse_distance_matches_independent_f64_distance_oracle() {
    let m = quad();
    let h = hit(&m, 0, [0.5, 0.25, 0.25]);
    let p = m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .unwrap();
    let distances = [
        (0.5_f64 * 0.5 + 0.25 * 0.25).sqrt(),
        (0.5_f64 * 0.5 + 0.25 * 0.25).sqrt(),
        (0.5_f64 * 0.5 + 0.75 * 0.75).sqrt(),
        (0.5_f64 * 0.5 + 0.75 * 0.75).sqrt(),
    ];
    let weights = distances.map(|d| 1.0 / d);
    let sum: f64 = weights.iter().sum();
    for c in 0..3 {
        scalar(
            p.rgb[c],
            (0..4)
                .map(|v| f64::from(m.colors[v][c]) * weights[v] / sum)
                .sum(),
        );
    }
    scalar(
        p.roughness,
        (0..4)
            .map(|v| f64::from(m.materials.as_ref().unwrap()[v][0]) * weights[v] / sum)
            .sum(),
    );
    scalar(
        p.metallic,
        (0..4)
            .map(|v| f64::from(m.materials.as_ref().unwrap()[v][1]) * weights[v] / sum)
            .sum(),
    );
}

#[test]
fn picker_defaults_do_not_materialize_none_or_change_document_history() {
    let mut m = quad();
    m.materials = None;
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    d.begin_stroke();
    for mode in [Mode::TriangleBarycentric, Mode::OriginalFaceInverseDistance] {
        let p = d
            .require_active_mesh()
            .unwrap()
            .pick_paint_channels(
                &hit(d.require_active_mesh().unwrap(), 0, [0.25, 0.25, 0.5]),
                mode,
            )
            .unwrap();
        assert_eq!([p.roughness, p.metallic], DEFAULT_VERTEX_MATERIAL);
        assert!(!p.materials_explicit);
    }
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.require_active_mesh().unwrap().materials.is_none());
    assert!(!d.can_undo());
}

#[test]
fn exact_corner_picker_is_finite_and_preserves_original_signed_zero_bits() {
    let mut m = quad();
    m.colors[0][1] = -0.0;
    m.materials.as_mut().unwrap()[0][0] = -0.0;
    let h = hit(&m, 0, [1.0, 0.0, 0.0]);
    for mode in [Mode::TriangleBarycentric, Mode::OriginalFaceInverseDistance] {
        let p = m.pick_paint_channels(&h, mode).unwrap();
        assert_eq!(p.rgb.map(f32::to_bits), m.colors[0].map(f32::to_bits));
        assert_eq!(p.roughness.to_bits(), (-0.0_f32).to_bits());
        assert!(p.metallic.is_finite());
    }
}

#[test]
fn picker_follows_current_barycentric_geometry_after_vertex_edit() {
    let mut m = quad();
    let h = hit(&m, 0, [0.5, 0.0, 0.5]);
    for v in &mut m.positions {
        v.x += 2.0;
        v.y += 4.0;
    }
    let p = m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_eq!(p.rgb, [0.5, 0.5, 0.5]);
}

#[test]
fn invalid_hit_weights_metadata_and_stale_triangles_reject_without_mutation() {
    let m = quad();
    let before = fixture::bits(&m);
    for scenario in 0..7 {
        let mut h = hit(&m, 0, [0.25, 0.25, 0.5]);
        match scenario {
            0 => h.barycentric[0] = f32::NAN,
            1 => h.barycentric = [-0.1, 0.6, 0.5],
            2 => h.barycentric = [0.1, 0.2, 0.3],
            3 => h.position.x = f32::INFINITY,
            4 => h.triangle = [0, 1, 3],
            5 => h.distance = -1.0,
            6 => h.normal.x = f32::NAN,
            _ => unreachable!(),
        };
        assert!(m
            .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
            .is_err());
        assert_eq!(fixture::bits(&m), before);
    }
}

#[test]
fn explicit_triangle_index_must_match_the_canonical_hit_indices() {
    let m = quad();
    let h = hit(&m, 0, [0.25, 0.25, 0.5]);
    assert!(m
        .pick_paint_channels_at_triangle(&h, 1, Mode::TriangleBarycentric)
        .is_err());
    assert!(m
        .pick_paint_channels_at_triangle(&h, 99, Mode::TriangleBarycentric)
        .is_err());
}

#[test]
fn duplicated_canonical_triangles_require_explicit_original_face_identity() {
    let mut m = quad();
    m.positions.push(Vec3::new(-1.0, 1.0, 0.0));
    m.colors.push([0.0, 0.0, 0.0]);
    m.masks.push(0.0);
    m.materials.as_mut().unwrap().push([0.0, 0.0]);
    m.triangles.extend([[0, 1, 2], [0, 2, 4]]);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3], [0, 1, 2, 4]],
        uv: None,
    });
    m.recompute_normals();
    m.validate().unwrap();
    let h = hit(&m, 0, [0.5, 0.0, 0.5]);
    assert!(m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .is_err());
    assert!(m
        .pick_paint_channels(&h, Mode::TriangleBarycentric)
        .is_err());
    let first = m
        .pick_paint_channels_at_triangle(&h, 0, Mode::OriginalFaceInverseDistance)
        .unwrap();
    let second = m
        .pick_paint_channels_at_triangle(&h, 2, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_eq!(first.original_face_index, Some(0));
    assert_eq!(second.original_face_index, Some(1));
    assert_ne!(first.rgb, second.rgb);
}

#[test]
fn raycast_tie_returns_indices_without_unique_render_row_provenance() {
    let mut m = quad();
    m.surface = None;
    m.triangles.push(m.triangles[0]);
    let h = m
        .raycast(Ray {
            origin: Vec3::new(0.75, 0.25, 1.0),
            direction: -Vec3::Z,
        })
        .unwrap();
    assert_eq!(h.triangle, [0, 1, 2]);
    assert!(m
        .pick_paint_channels(&h, Mode::TriangleBarycentric)
        .is_err());
    assert!(m
        .pick_paint_channels_at_triangle(&h, 0, Mode::TriangleBarycentric)
        .is_ok());
}

#[test]
fn indexed_raycast_resolves_original_face_identity_without_changing_legacy_pick_bits() {
    let mut m = quad();
    m.positions.push(Vec3::new(-1.0, 1.0, 0.0));
    m.colors.push([0.0, 0.0, 0.0]);
    m.masks.push(0.0);
    m.materials.as_mut().unwrap().push([0.0, 0.0]);
    m.triangles.extend([[0, 1, 2], [0, 2, 4]]);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3], [0, 1, 2, 4]],
        uv: None,
    });
    m.recompute_normals();
    m.validate().unwrap();
    let ray = Ray {
        origin: Vec3::new(0.7, 0.2, 1.0),
        direction: -Vec3::Z,
    };
    let (row, indexed) = m.raycast_indexed(ray).unwrap();
    let legacy = m.raycast(ray).unwrap();
    assert_eq!(row, 0);
    assert_eq!(
        indexed.position.to_array().map(f32::to_bits),
        legacy.position.to_array().map(f32::to_bits)
    );
    assert_eq!(
        indexed.barycentric.map(f32::to_bits),
        legacy.barycentric.map(f32::to_bits)
    );
    assert_eq!(indexed.triangle, legacy.triangle);
    assert!(m
        .pick_paint_channels(&legacy, Mode::OriginalFaceInverseDistance)
        .is_err());
    let picked = m
        .pick_paint_channels_at_triangle(&indexed, row, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_eq!(picked.original_face_index, Some(0));
    let other = m
        .pick_paint_channels_at_triangle(&indexed, 2, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_ne!(picked.rgb, other.rgb);
    let document = Document::new(m);
    let (document_row, indexed_part) = document
        .raycast_subtools_indexed(ray)
        .unwrap()
        .expect("fixture hit");
    let legacy_part = document
        .raycast_subtools(ray)
        .unwrap()
        .expect("fixture hit");
    assert_eq!(document_row, row);
    assert_eq!(indexed_part.id, legacy_part.id);
    assert_eq!(indexed_part.hit.triangle, legacy_part.hit.triangle);
    assert_eq!(
        indexed_part.hit.barycentric.map(f32::to_bits),
        legacy_part.hit.barycentric.map(f32::to_bits)
    );
}

#[test]
fn coincident_original_quad_corners_reject_inverse_distance_singularity() {
    let mut m = quad();
    m.positions[3] = m.positions[0];
    m.recompute_normals();
    let h = hit(&m, 0, [1.0, 0.0, 0.0]);
    assert!(m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .is_err());
    assert!(m.pick_paint_channels(&h, Mode::TriangleBarycentric).is_ok());
}

#[test]
fn collapsed_selected_triangle_is_rejected_by_both_picker_modes() {
    let mut m = quad();
    m.positions[2] = m.positions[1];
    m.recompute_normals();
    let h = hit(&m, 0, [0.25, 0.25, 0.5]);
    for mode in [Mode::TriangleBarycentric, Mode::OriginalFaceInverseDistance] {
        assert!(m.pick_paint_channels(&h, mode).is_err());
    }
}

#[test]
fn triangular_surface_without_original_metadata_has_defined_inverse_distance_mode() {
    let mut m = quad();
    m.surface = None;
    let h = hit(&m, 0, [1.0 / 3.0; 3]);
    let p = m
        .pick_paint_channels(&h, Mode::OriginalFaceInverseDistance)
        .unwrap();
    assert_eq!(p.original_face_index, None);
    assert!(p.rgb.iter().all(|v| v.is_finite()));
    assert!((p.rgb.iter().sum::<f32>() - 1.0).abs() < 1e-6);
}
