//! Independent literal mixed-normal witnesses. Authored gates, not a paired
//! Source camera/history/optimizer or physical browser comparison.
use obrush_core::*;
use std::sync::Arc;

fn fixture() -> Mesh {
    let mut m =
        Mesh::from_obj("v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([1.; 3]);
    m
}
fn geometry_pass() -> SourceGeometryPass {
    SourceGeometryPass {
        center: [0.; 3],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    }
}
fn warm() -> Document {
    let mut d = Document::new(fixture());
    let options = SourceGeometryOptions {
        intensity: 0.625,
        intensity_factor: 0.,
        ..SourceGeometryOptions::inflate()
    };
    let mut s = d.begin_source_geometry_stroke(options).unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, geometry_pass(), 0.5, true)
            .unwrap()
            .changed_vertices,
        1
    );
    d.end_source_geometry_stroke(&s).unwrap();
    d
}
fn paint() -> PaintResponseRequest {
    PaintResponseRequest {
        kind: BrushKind::Paint,
        operation: PaintResponseOperation::OrdinaryDab,
        in_contact: true,
        options: PaintResponseOptions {
            intensity: 0.5,
            hardness: 1.,
            color: [0., 0.5, 1.],
            write_rgb: true,
            write_roughness: false,
            write_metallic: false,
            culling: true,
            tablet: PaintTabletIntensity {
                pressure: 0.5,
                factor: 0.,
            },
            alpha: None,
            ..Default::default()
        },
        first: PaintResponsePass {
            center: [4., 0., 0.],
            local_radius_squared: 4.,
            eye_direction: [1., 0., -1. / 128.],
            mirrored_x: false,
            alpha_frame: None,
        },
        mirrored: None,
    }
}
fn geometry_words(m: &Mesh) -> serde_json::Value {
    serde_json::json!({"p":m.positions.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "n":m.normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"t":m.triangles,
        "u":m.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"convention":m.mask_convention,"surface":m.surface})
}
fn cache_words(s: &SourceGeometryNormalState) -> serde_json::Value {
    serde_json::json!({"n":s.vertex_normals().iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "f":s.face_normals().iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "tree":s.ordered_tree_snapshot().unwrap(),"generation":s.generation()})
}
fn redo_witness(d: &mut Document) -> String {
    d.begin_stroke();
    d.name = "real future state".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    future
}

#[test]
fn w01_warm_fresh_literal_culling_difference_and_exact_rgb_words() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let m = d.require_active_mesh().unwrap();
    assert_eq!(m.positions[0].z.to_bits(), 0.125f32.to_bits());
    assert_eq!(state.vertex_normals()[1], Vec3::new(0., 0., 16.));
    let fresh = SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap();
    assert_eq!(fresh.vertex_normals()[1], Vec3::new(0.5, 0.5, 16.));
    let mut cold = m.snapshot();
    let r = cold
        .apply_paint_response_with_source_normals(&paint(), &fresh)
        .unwrap();
    assert_eq!(r.changed_vertices, 0);
    let r = d
        .require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_normals(&paint(), &state)
        .unwrap();
    assert_eq!(r.first_selected_vertices, 1);
    assert_eq!(r.changed_rgb_vertices, 1);
    assert_eq!(r.changed_material_vertices, 0);
    assert_eq!(
        d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
        [0x3f000000, 0x3f400000, 0x3f800000]
    );
    assert_eq!(cold.colors[1].map(f32::to_bits), [0x3f800000; 3]);
}
#[test]
fn w02_borrowed_actual_rows_and_readonly_cache_geometry_generation() {
    let mut d = warm();
    let s = d.source_geometry_state(1).unwrap();
    let before = cache_words(&s);
    let geometry = geometry_words(d.require_active_mesh().unwrap());
    let view = SourcePaintNormalView::RetainedGeometry(&s);
    view.validate(d.require_active_mesh().unwrap()).unwrap();
    assert_eq!(view.vertex_normals().as_ptr(), s.vertex_normals().as_ptr());
    let report = view.report();
    assert_eq!(report.kind, SourcePaintNormalProducerKind::RetainedGeometry);
    assert_eq!(
        report.normal_policy.policy,
        SourceNormalPolicy::StaticSelectedRefresh
    );
    assert_eq!(report.generation, Some(s.generation()));
    assert_eq!(
        report.normal_policy.storage.retained_bytes,
        s.approximate_bytes()
    );
    d.require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_normals(&paint(), &s)
        .unwrap();
    assert_eq!(geometry_words(d.require_active_mesh().unwrap()), geometry);
    assert_eq!(cache_words(&s), before);
    assert!(Arc::ptr_eq(&s, &d.source_geometry_state(1).unwrap()));
    assert!(d.require_active_mesh().unwrap().materials.is_none());
}
#[test]
fn w03_rational_idw_binds_warm_raw_rows_not_fresh_normals() {
    let d = warm();
    let s = d.source_geometry_state(1).unwrap();
    let view = SourcePaintNormalView::RetainedGeometry(&s);
    let disposition = view
        .picked_normal_alpha_none(
            d.require_active_mesh().unwrap(),
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: [0.5, 0.5, 0.09375],
            },
        )
        .unwrap();
    let SourcePickedNormalDisposition::Defined { report } = disposition else {
        panic!("Independent interior contact must be defined");
    };
    let inverse0 = 1. / 0.5009765625f64.sqrt();
    let inverse1 = 1. / 12.5087890625f64.sqrt();
    let weight0 = inverse0 / (inverse0 + inverse1 + inverse1);
    let expected = [0.5 * weight0, 0.5 * weight0, 16.];
    for (actual, expected) in report.interpolated64.into_iter().zip(expected) {
        assert!((actual - expected).abs() <= 2e-15);
    }
    assert_eq!(report.policy, SourceNormalPolicy::StaticSelectedRefresh);
    assert_eq!(report.vertex_ids, [0, 1, 2, TRIANGLE_CORNER]);
}
#[test]
fn w04_exact_vertex_undefined_alpha_none_still_permits_useful_paint() {
    let mut d = warm();
    let s = d.source_geometry_state(1).unwrap();
    let before = geometry_words(d.require_active_mesh().unwrap());
    let disposition = SourcePaintNormalView::RetainedGeometry(&s)
        .picked_normal_alpha_none(
            d.require_active_mesh().unwrap(),
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: [4., 0., 0.],
            },
        )
        .unwrap();
    assert!(matches!(
        disposition,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ExactVertexContact {
                corner: 1,
                vertex_id: 1
            },
            raw_report: None,
            ..
        }
    ));
    assert_eq!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response_with_retained_source_normals(&paint(), &s)
            .unwrap()
            .changed_vertices,
        1
    );
    assert_eq!(geometry_words(d.require_active_mesh().unwrap()), before);
}
#[test]
fn w05_noops_preserve_genuine_redo_and_no_material_allocation() {
    for case in 0..4 {
        let mut d = warm();
        if case == 3 {
            d.require_active_mesh_mut().unwrap().masks.fill(0.);
        }
        let future = redo_witness(&mut d);
        let before = d.to_json().unwrap();
        let s = d.source_geometry_state(1).unwrap();
        let mut request = paint();
        request.options.write_roughness = true;
        request.options.write_metallic = true;
        match case {
            0 => request.in_contact = false,
            1 => request.options.intensity = 0.,
            2 => {
                request.options.write_rgb = false;
                request.options.write_roughness = false;
                request.options.write_metallic = false;
            }
            _ => {}
        }
        d.begin_stroke();
        let owner = d.claim_retained_source_paint_stroke(1, 0, &s).unwrap();
        assert_eq!(
            d.require_active_mesh_mut()
                .unwrap()
                .apply_paint_response_with_retained_source_normals(&request, &s)
                .unwrap()
                .changed_vertices,
            0
        );
        d.validate_retained_source_paint_stroke(&owner).unwrap();
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(Arc::ptr_eq(&s, &d.source_geometry_state(1).unwrap()));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
}
#[test]
fn w06_material_only_lazy_storage_and_complete_history_same_cache() {
    let mut d = warm();
    let before = d.to_json().unwrap();
    let s = d.source_geometry_state(1).unwrap();
    let cache = cache_words(&s);
    let mut request = paint();
    request.options.intensity = 1.;
    request.options.write_rgb = false;
    request.options.write_roughness = true;
    request.options.write_metallic = true;
    request.options.roughness = 0.5;
    request.options.metallic = 1.;
    d.begin_material_paint_stroke();
    let owner = d.claim_retained_source_paint_stroke(1, 0, &s).unwrap();
    let r = d
        .require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_normals(&request, &s)
        .unwrap();
    assert_eq!(r.changed_rgb_vertices, 0);
    assert_eq!(r.changed_material_vertices, 1);
    assert!(r.materialized);
    let m = d.require_active_mesh().unwrap();
    assert_eq!(
        m.materials.as_ref().unwrap()[1].map(f32::to_bits),
        [0x3f000000, 0x3f800000]
    );
    for i in [0, 2, 3, 4] {
        assert_eq!(
            m.materials.as_ref().unwrap()[i].map(f32::to_bits),
            DEFAULT_VERTEX_MATERIAL.map(f32::to_bits)
        );
    }
    assert_eq!(m.colors, vec![[1.; 3]; 5]);
    d.validate_retained_source_paint_stroke(&owner).unwrap();
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert_eq!(cache_words(&s), cache);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.require_active_mesh().unwrap().materials.is_none());
    assert!(Arc::ptr_eq(&s, &d.source_geometry_state(1).unwrap()));
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(Arc::ptr_eq(&s, &d.source_geometry_state(1).unwrap()));
}
#[test]
fn w07_stale_geometry_face_and_invalid_contact_do_not_fallback_or_write() {
    let d = warm();
    let s = d.source_geometry_state(1).unwrap();
    let view = SourcePaintNormalView::RetainedGeometry(&s);
    for face_edit in [false, true] {
        let mut m = d.require_active_mesh().unwrap().snapshot();
        if face_edit {
            m.triangles[0] = [0, 2, 1];
        } else {
            m.positions[2].z = 0.25;
        }
        let before = serde_json::to_vec(&m).unwrap();
        assert_eq!(
            view.validate(&m).unwrap_err(),
            SourcePaintNormalProducerError::StaleField
        );
        assert!(m
            .apply_paint_response_with_retained_source_normals(&paint(), &s)
            .is_err());
        assert_eq!(serde_json::to_vec(&m).unwrap(), before);
    }
    assert_eq!(
        view.picked_normal_alpha_none(
            d.require_active_mesh().unwrap(),
            SourceOriginalFaceContact {
                face_index: usize::MAX,
                local_point: [1., 1., 0.]
            }
        )
        .unwrap_err(),
        SourcePaintNormalProducerError::PickedNormal(
            SourcePickedNormalRejection::MissingOriginalFace
        )
    );
}
#[test]
fn w08_future_alpha_mirror_locked_and_nonpaint_are_explicit_rejections() {
    let d = warm();
    let s = d.source_geometry_state(1).unwrap();
    for case in 0..5 {
        let mut m = d.require_active_mesh().unwrap().snapshot();
        let before = serde_json::to_vec(&m).unwrap();
        let mut r = paint();
        match case {
            0 => r.options.alpha = Some(RasterAlpha::new(2, 2, vec![255; 4]).unwrap()),
            1 => r.first.mirrored_x = true,
            2 => {
                r.mirrored = Some(PaintResponsePass {
                    mirrored_x: true,
                    ..r.first
                })
            }
            3 => r.operation = PaintResponseOperation::LockedAccumulation,
            _ => r.kind = BrushKind::Draw,
        }
        assert!(m
            .apply_paint_response_with_retained_source_normals(&r, &s)
            .is_err());
        assert_eq!(serde_json::to_vec(&m).unwrap(), before);
    }
}
#[test]
fn w09_zero_raw_normal_has_explicit_alpha_none_disposition_and_no_direction() {
    let mut m = fixture();
    for (i, p) in m.positions.iter_mut().enumerate() {
        *p = Vec3::new(i as f32, 0., 0.);
    }
    m.validate().unwrap();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(state.vertex_normals(), vec![Vec3::ZERO; 5]);
    let disposition = SourcePaintNormalView::RetainedGeometry(&state)
        .picked_normal_alpha_none(
            &m,
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: [0.5, 0.25, 0.],
            },
        )
        .unwrap();
    assert!(matches!(
        disposition,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ZeroDirection,
            raw_report: Some(_),
            ..
        }
    ));
    let report = disposition.raw_report().unwrap();
    assert_eq!(report.interpolated64, [0.; 3]);
    assert_eq!(report.normal64, [0.; 3]);
    assert!(!report.direction_defined);
}
#[test]
fn w10_useful_geometry_paint_geometry_sequence_retains_and_then_refreshes_cache() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_normals(&paint(), &state)
        .unwrap();
    d.end_stroke();
    assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
    let before = d.to_json().unwrap();
    let mut next = d
        .begin_source_geometry_stroke(SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        })
        .unwrap();
    let r = d
        .apply_source_geometry_dab(
            &mut next,
            SourceGeometryPass {
                center: [4., 0., 0.],
                ..geometry_pass()
            },
            0.5,
            true,
        )
        .unwrap();
    assert_eq!(r.changed_vertices, 1);
    d.end_source_geometry_stroke(&next).unwrap();
    let changed = d.source_geometry_state(1).unwrap();
    assert!(!Arc::ptr_eq(&state, &changed));
    assert!(changed.generation() > state.generation());
    assert_eq!(
        d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
        [0x3f000000, 0x3f400000, 0x3f800000]
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
}
#[test]
fn w11_fresh_producer_query_preserves_original_alpha_none_api_values() {
    let d = warm();
    let m = d.require_active_mesh().unwrap();
    let f = SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap();
    let view = SourcePaintNormalView::FreshStatic(&f);
    view.validate(m).unwrap();
    assert_eq!(view.report().generation, None);
    assert_eq!(
        view.report().kind,
        SourcePaintNormalProducerKind::FreshStatic
    );
    assert_eq!(view.vertex_normals().as_ptr(), f.vertex_normals().as_ptr());
    let contact = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [0.5, 0.5, 0.09375],
    };
    let before = prepare_source_picked_normal_alpha_none(m, &f, contact).unwrap();
    let after = view.picked_normal_alpha_none(m, contact).unwrap();
    assert_eq!(
        serde_json::to_vec(&before).unwrap(),
        serde_json::to_vec(&after).unwrap()
    );
}
#[test]
fn w12_original_face_uv_seam_and_complete_history_survive_warm_paint() {
    let mut m=Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 0.25 0.75\nvt 0.5 1\nvt 0.5 0.5\nf 1/1 2/2 3/3\nf 3/4 5/5 4/6\n").unwrap().mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([1.; 3]);
    let uv = m.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates.len(), 6);
    assert_eq!(
        uv.face_indices,
        [[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]]
    );
    let mut d = Document::new(m);
    let mut g = d
        .begin_source_geometry_stroke(SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        })
        .unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut g, geometry_pass(), 0.5, true)
            .unwrap()
            .changed_vertices,
        1
    );
    d.end_source_geometry_stroke(&g).unwrap();
    let state = d.source_geometry_state(1).unwrap();
    let before = d.to_json().unwrap();
    let geometry = geometry_words(d.require_active_mesh().unwrap());
    let cache = cache_words(&state);
    d.begin_stroke();
    assert_eq!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response_with_retained_source_normals(&paint(), &state)
            .unwrap()
            .changed_vertices,
        1
    );
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert_eq!(geometry_words(d.require_active_mesh().unwrap()), geometry);
    assert_eq!(cache_words(&state), cache);
    assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert_eq!(geometry_words(d.require_active_mesh().unwrap()), geometry);
}

#[test]
fn controlled_warm_alpha_mirror_keeps_stale_culling_and_exact_sequential_words() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let geometry = geometry_words(d.require_active_mesh().unwrap());
    let cache = cache_words(&state);
    let before = d.to_json().unwrap();
    let mut request = paint();
    request.options.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    request.options.write_roughness = true;
    request.options.write_metallic = true;
    let frame = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -4., 0., 0., 1.,
    ];
    request.first.alpha_frame = Some(PaintResponseAlphaFrame { look_at: frame });
    request.mirrored = Some(PaintResponsePass {
        mirrored_x: true,
        ..request.first
    });
    let fresh = SourceNormalField::compute(
        d.require_active_mesh().unwrap(),
        SourceNormalPolicy::StaticOriginalFaces,
    )
    .unwrap();
    let mut wrong = d.require_active_mesh().unwrap().snapshot();
    assert_eq!(
        wrong
            .apply_paint_response_with_source_normals(&request, &fresh)
            .unwrap()
            .changed_vertices,
        0
    );
    d.begin_material_paint_stroke();
    let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
    let report = d
        .require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_retained_source_controls(&request, &state)
        .unwrap();
    assert_eq!(report.changed_vertices, 1);
    assert_eq!(report.first_selected_vertices, 1);
    assert_eq!(report.mirrored_selected_vertices, 1);
    let w = 0.5 * (128. / 255.);
    let mut rgb = [1f32; 3];
    let mut material = DEFAULT_VERTEX_MATERIAL;
    for _ in 0..2 {
        rgb = std::array::from_fn(|k| {
            (f64::from(rgb[k]) * (1. - w) + f64::from(request.options.color[k]) * w) as f32
        });
        material = std::array::from_fn(|k| {
            (f64::from(material[k]) * (1. - w)
                + f64::from(if k == 0 {
                    request.options.roughness
                } else {
                    request.options.metallic
                }) * w) as f32
        });
    }
    assert_eq!(
        d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
        rgb.map(f32::to_bits)
    );
    assert_eq!(
        d.require_active_mesh().unwrap().materials.as_ref().unwrap()[1].map(f32::to_bits),
        material.map(f32::to_bits)
    );
    assert_eq!(geometry_words(d.require_active_mesh().unwrap()), geometry);
    assert_eq!(cache_words(&state), cache);
    d.validate_retained_source_paint_stroke(&owner).unwrap();
    d.end_stroke();
    assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn controlled_warm_invalid_or_stale_second_pass_rejects_before_any_mutation() {
    let d = warm();
    let state = d.source_geometry_state(1).unwrap();
    for case in 0..5 {
        let mut m = d.require_active_mesh().unwrap().snapshot();
        let mut request = paint();
        match case {
            0 => {
                request.options.alpha = Some(RasterAlpha::new(1, 1, vec![255]).unwrap());
            }
            1 => {
                request.mirrored = Some(PaintResponsePass {
                    mirrored_x: true,
                    local_radius_squared: 3.,
                    ..request.first
                });
            }
            2 => request.kind = BrushKind::Draw,
            3 => request.operation = PaintResponseOperation::LockedPreview,
            _ => m.positions[0].x += 0.25,
        }
        let before = serde_json::to_vec(&m).unwrap();
        assert!(m
            .apply_paint_response_with_retained_source_controls(&request, &state)
            .is_err());
        assert_eq!(serde_json::to_vec(&m).unwrap(), before);
    }
}
#[test]
fn controlled_warm_transparent_alpha_and_mirror_preserve_real_redo_without_materialization() {
    let mut d = warm();
    let state = d.source_geometry_state(1).unwrap();
    let future = redo_witness(&mut d);
    let before = d.to_json().unwrap();
    let mut request = paint();
    request.options.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
    request.options.write_roughness = true;
    request.options.write_metallic = true;
    request.first.alpha_frame = Some(PaintResponseAlphaFrame {
        look_at: [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -4., 0., 0., 1.,
        ],
    });
    request.mirrored = Some(PaintResponsePass {
        mirrored_x: true,
        ..request.first
    });
    d.begin_material_paint_stroke();
    let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
    assert_eq!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response_with_retained_source_controls(&request, &state)
            .unwrap()
            .changed_vertices,
        0
    );
    d.validate_retained_source_paint_stroke(&owner).unwrap();
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.require_active_mesh().unwrap().materials.is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}
