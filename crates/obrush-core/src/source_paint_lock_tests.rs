//! Independent literal fractions and adversarial complete-transaction witnesses.
use super::*;
use crate::{
    LockTravelRadius, PaintPreparationInput, PaintViewCapture, SourceGeometryPass, SurfaceTopology,
    Vec3, TRIANGLE_CORNER,
};
fn mesh() -> Mesh {
    let mut m = Mesh::from_obj("v 0 0 0\nv .5 0 0\nv 0 .5 0\nf 1 2 3\n").unwrap();
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER]],
        uv: None,
    });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([0.; 3]);
    m.materials = Some(vec![[0.25, 0.75]; 3]);
    m.validate().unwrap();
    m
}
fn options() -> PaintResponseOptions {
    PaintResponseOptions {
        intensity: 0.5,
        hardness: 0.,
        color: [1., 0.5, 0.25],
        roughness: 0.75,
        metallic: 0.25,
        ..Default::default()
    }
}
fn sample(r: f64) -> SourcePaintLockedSample {
    SourcePaintLockedSample::Dab {
        pass: PaintResponsePass {
            center: [0.; 3],
            local_radius_squared: r,
            eye_direction: [0., 0., -1.],
            mirrored_x: false,
            alpha_frame: None,
        },
        face_index: 0,
        pressure: 0.5,
    }
}
fn colors(d: &Document) -> Vec<[u32; 3]> {
    d.require_active_mesh()
        .unwrap()
        .colors
        .iter()
        .map(|c| c.map(f32::to_bits))
        .collect()
}
fn materials(d: &Document) -> Vec<[u32; 2]> {
    d.require_active_mesh()
        .unwrap()
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.map(f32::to_bits))
        .collect()
}
fn first(d: &Document) {
    assert_eq!(
        colors(d),
        [
            [0x3f000000, 0x3e800000, 0x3e000000],
            [0x3e000000, 0x3d800000, 0x3d000000],
            [0x3e000000, 0x3d800000, 0x3d000000]
        ]
    );
    assert_eq!(
        materials(d),
        [
            [0x3f000000, 0x3f000000],
            [0x3ea00000, 0x3f300000],
            [0x3ea00000, 0x3f300000]
        ]
    );
}
fn prior_redo(d: &mut Document) {
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().colors[0] = [0.125; 3];
    d.end_stroke();
    assert!(d.undo());
    assert!(d.can_redo());
}
#[test]
fn l01_literal_absolute_repeated_and_accumulated_rows_are_distinct() {
    for mode in [
        SourcePaintLockedMode::AbsolutePreview,
        SourcePaintLockedMode::ContinuousAccumulation,
    ] {
        let mut d = Document::new(mesh());
        let mut s = d.begin_source_paint_locked(&options(), mode).unwrap();
        assert_eq!(
            d.apply_source_paint_locked(&mut s, sample(1.))
                .unwrap()
                .live
                .changed_vertices,
            3
        );
        first(&d);
        let second = d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        if mode == SourcePaintLockedMode::AbsolutePreview {
            first(&d);
            assert_eq!(second.live.changed_vertices, 0);
        } else {
            assert_eq!(
                colors(&d),
                [
                    [0x3f400000, 0x3ec00000, 0x3e400000],
                    [0x3e700000, 0x3df00000, 0x3d700000],
                    [0x3e700000, 0x3df00000, 0x3d700000]
                ]
            );
            assert_eq!(
                materials(&d),
                [
                    [0x3f200000, 0x3ec00000],
                    [0x3ebc0000, 0x3f220000],
                    [0x3ebc0000, 0x3f220000]
                ]
            );
            assert_eq!(second.live.changed_vertices, 3);
        }
        d.end_source_paint_locked(&mut s).unwrap();
    }
}
#[test]
fn l02_shrinking_preview_restores_outside_rows_and_strict_boundary() {
    let mut d = Document::new(mesh());
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    let r = d.apply_source_paint_locked(&mut s, sample(0.25)).unwrap();
    assert_eq!(r.live.changed_vertices, 2);
    assert_eq!(
        colors(&d),
        [[0x3f000000, 0x3e800000, 0x3e000000], [0; 3], [0; 3]]
    );
    assert_eq!(
        materials(&d),
        [
            [0x3f000000, 0x3f000000],
            [0x3e800000, 0x3f400000],
            [0x3e800000, 0x3f400000]
        ]
    );
    assert_eq!(r.dab.changed_vertices, 1);
    d.end_source_paint_locked(&mut s).unwrap();
}
#[test]
fn l03_zero_travel_restores_only_absolute_and_preserves_material_absence() {
    for mode in [
        SourcePaintLockedMode::AbsolutePreview,
        SourcePaintLockedMode::ContinuousAccumulation,
    ] {
        let mut m = mesh();
        m.materials = None;
        let mut d = Document::new(m);
        let before = d.to_json().unwrap();
        let mut s = d.begin_source_paint_locked(&options(), mode).unwrap();
        d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        let edited = d.to_json().unwrap();
        let r = d
            .apply_source_paint_locked(&mut s, SourcePaintLockedSample::ZeroTravel)
            .unwrap();
        if mode == SourcePaintLockedMode::AbsolutePreview {
            assert_eq!(d.to_json().unwrap(), before);
            assert!(!r.materials_after);
            assert!(r.at_original);
        } else {
            assert_eq!(d.to_json().unwrap(), edited);
            assert_eq!(r.action, SourcePaintLockedAction::HoldZeroTravel);
        }
        d.end_source_paint_locked(&mut s).unwrap();
    }
}
#[test]
fn l04_no_contact_holds_but_positive_zero_effective_intensity_restores_preview() {
    let mut d = Document::new(mesh());
    let mut o = options();
    o.tablet.factor = 1.;
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    let painted = d.to_json().unwrap();
    let r = d
        .apply_source_paint_locked(&mut s, SourcePaintLockedSample::NoContact)
        .unwrap();
    assert_eq!(r.action, SourcePaintLockedAction::HoldNoContact);
    assert_eq!(d.to_json().unwrap(), painted);
    let SourcePaintLockedSample::Dab {
        pass, face_index, ..
    } = sample(1.)
    else {
        panic!()
    };
    // Smallest positive Number is in contact, but 1+(2p-1) rounds exactlyzero.
    let r = d
        .apply_source_paint_locked(
            &mut s,
            SourcePaintLockedSample::Dab {
                pass,
                face_index,
                pressure: f64::from_bits(1),
            },
        )
        .unwrap();
    assert_eq!(r.action, SourcePaintLockedAction::NoContribution);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(r.at_original);
    d.end_source_paint_locked(&mut s).unwrap();
    assert!(!d.can_undo());
}
#[test]
fn l05_miss_undo_preview_but_not_timer_and_explicit_cancel_is_complete() {
    for mode in [
        SourcePaintLockedMode::AbsolutePreview,
        SourcePaintLockedMode::ContinuousAccumulation,
    ] {
        let mut d = Document::new(mesh());
        let before = d.to_json().unwrap();
        let mut s = d.begin_source_paint_locked(&options(), mode).unwrap();
        d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        let edited = d.to_json().unwrap();
        d.apply_source_paint_locked(&mut s, SourcePaintLockedSample::Miss)
            .unwrap();
        assert_eq!(
            d.to_json().unwrap(),
            if mode == SourcePaintLockedMode::AbsolutePreview {
                before.clone()
            } else {
                edited
            }
        );
        d.cancel_source_paint_locked(&mut s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
}
fn view() -> PaintViewCapture {
    PaintViewCapture {
        world_to_clip: [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., 0., 0., 1.,
        ],
        viewport: [0., 0., 1000., 1000.],
    }
}
#[test]
fn l06_preparation_keeps_actual_f64_anchor_and_independent_radius() {
    let mut m = Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    let (pass, hit) = crate::prepare_source_paint_locked_pass(
        &m,
        None,
        PaintPreparationInput {
            view: view(),
            screen: [500.0006, 500.0008],
            screen_radius: 5.,
            pressure: 0.5,
            radius_factor: 0.75,
        },
        LockTravelRadius::new(5.).unwrap(),
    )
    .unwrap()
    .unwrap();
    for (actual, expected) in pass.center.into_iter().zip([1.2e-6, -1.6e-6, 0.]) {
        assert!((actual - expected).abs() <= 1e-16);
    }
    assert!((pass.local_radius_squared - 0.0001).abs() <= 1e-16);
    assert_eq!(hit.triangle_index, 0);
}
#[test]
fn l07_typed_lock_radius_below_and_above_ui_bounds_without_widening_ordinary() {
    let m = mesh();
    for r in [1., 600.] {
        let input = PaintPreparationInput {
            view: view(),
            screen: [500., 500.],
            screen_radius: r,
            pressure: 0.5,
            radius_factor: 0.75,
        };
        assert!(crate::prepare_source_paint_pass(&m, None, input).is_err());
        let (pass, _) = crate::prepare_source_paint_locked_pass(
            &m,
            None,
            input,
            LockTravelRadius::new(r).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert!((pass.local_radius_squared - (r * 0.002).powi(2)).abs() <= 1e-15);
    }
    for r in [0., -1., f64::NAN, f64::INFINITY, 1_000_001.] {
        assert!(LockTravelRadius::new(r).is_err());
    }
}
#[test]
fn l08_invalid_or_mismatched_typed_preparation_never_falls_back() {
    let m = mesh();
    let mut i = PaintPreparationInput {
        view: view(),
        screen: [500., 500.],
        screen_radius: 1.,
        pressure: 0.5,
        radius_factor: 0.75,
    };
    assert!(crate::prepare_source_paint_locked_pass(
        &m,
        None,
        i,
        LockTravelRadius::new(2.).unwrap()
    )
    .is_err());
    i.pressure = f64::NAN;
    assert!(crate::prepare_source_paint_locked_pass(
        &m,
        None,
        i,
        LockTravelRadius::new(1.).unwrap()
    )
    .is_err());
}
#[test]
fn l09_tablet_endpoints_do_not_use_legacy_pressure_or_quarter_multiplier() {
    let m = mesh();
    for (p, r) in [(0., 0.25), (0.5, 1.), (1., 1.75)] {
        let (pass, _) = crate::prepare_source_paint_locked_pass(
            &m,
            None,
            PaintPreparationInput {
                view: view(),
                screen: [500., 500.],
                screen_radius: 10.,
                pressure: p,
                radius_factor: 0.75,
            },
            LockTravelRadius::new(10.).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert!((pass.local_radius_squared - (0.02_f64 * r).powi(2)).abs() <= 1e-16);
    }
    let mut d = Document::new(mesh());
    let mut o = options();
    o.tablet.factor = 1.;
    let mut s = d
        .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let SourcePaintLockedSample::Dab {
        pass, face_index, ..
    } = sample(1.)
    else {
        panic!()
    };
    d.apply_source_paint_locked(
        &mut s,
        SourcePaintLockedSample::Dab {
            pass,
            face_index,
            pressure: 1.,
        },
    )
    .unwrap();
    assert_eq!(colors(&d)[0], [0x3f800000, 0x3f000000, 0x3e800000]);
    d.end_source_paint_locked(&mut s).unwrap();
}
#[test]
fn l10_raw_u_fraction_protection_and_signed_zero_are_preserved() {
    let mut m = mesh();
    m.masks = [-0., 0.5, 1.].to_vec();
    m.colors[0] = [-0., 0., -0.];
    let before_masks = m.masks.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    let mut d = Document::new(m);
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    assert_eq!(colors(&d)[0], [0x80000000, 0, 0x80000000]);
    assert_eq!(colors(&d)[1], [0x3d800000, 0x3d000000, 0x3c800000]);
    assert_eq!(
        d.require_active_mesh()
            .unwrap()
            .masks
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        before_masks
    );
    d.end_source_paint_locked(&mut s).unwrap();
}
#[test]
fn l11_individual_channels_and_none_material_allocation_are_exact() {
    for flags in [1, 2, 4] {
        let mut m = mesh();
        m.materials = None;
        let mut d = Document::new(m);
        let before = d.to_json().unwrap();
        let mut o = options();
        o.write_rgb = flags == 1;
        o.write_roughness = flags == 2;
        o.write_metallic = flags == 4;
        let mut s = d
            .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
            .unwrap();
        d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        assert_eq!(
            d.require_active_mesh().unwrap().materials.is_some(),
            flags != 1
        );
        if flags != 1 {
            assert_eq!(colors(&d), [[0; 3]; 3]);
            let m = d.require_active_mesh().unwrap().materials.as_ref().unwrap();
            for row in m {
                assert_eq!(
                    row[usize::from(flags == 2)].to_bits(),
                    crate::DEFAULT_VERTEX_MATERIAL[usize::from(flags == 2)].to_bits()
                );
            }
        }
        d.cancel_source_paint_locked(&mut s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
    }
}
#[test]
fn l12_no_contribution_keeps_real_redo_and_no_document_cache_or_materials() {
    for case in 0..4 {
        let mut m = mesh();
        m.materials = None;
        if case == 2 {
            m.masks.fill(0.);
        }
        let mut d = Document::new(m);
        prior_redo(&mut d);
        let before = d.to_json().unwrap();
        let mut o = options();
        if case == 0 {
            o.write_rgb = false;
            o.write_roughness = false;
            o.write_metallic = false;
        }
        if case == 1 {
            o.intensity = 0.;
        }
        if case == 3 {
            o.color = [0.; 3];
            o.write_roughness = false;
            o.write_metallic = false;
        }
        let mut s = d
            .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
            .unwrap();
        assert_eq!(
            d.apply_source_paint_locked(&mut s, sample(1.))
                .unwrap()
                .live
                .changed_vertices,
            0
        );
        assert!(d.source_geometry_states.is_empty());
        if case < 3 {
            assert!(s.cold.is_none());
        }
        d.end_source_paint_locked(&mut s).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert!(!d.can_undo());
    }
}
fn warm(d: &mut Document) {
    let mut s = d
        .begin_source_geometry_stroke(crate::SourceGeometryOptions {
            intensity: 0.125,
            ..crate::SourceGeometryOptions::inflate()
        })
        .unwrap();
    assert!(
        d.apply_source_geometry_dab(
            &mut s,
            SourceGeometryPass {
                center: [0.; 3],
                eye_direction: [0., 0., -1.],
                local_radius_squared: 0.01,
                original_face_index: 0
            },
            0.5,
            true
        )
        .unwrap()
        .changed_vertices
            > 0
    );
    d.end_source_geometry_stroke(&s).unwrap();
}
#[test]
fn l13_warm_producer_is_same_arc_outside_rows_and_never_falls_back() {
    let mut d = Document::new(mesh());
    warm(&mut d);
    let state = d.source_geometry_state(1).unwrap();
    let normals = state.vertex_normals().to_vec();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let mut pass = if let SourcePaintLockedSample::Dab { pass, .. } = sample(1.) {
        pass
    } else {
        panic!()
    };
    pass.center = d.require_active_mesh().unwrap().positions[0]
        .to_array()
        .map(f64::from);
    let r = d
        .apply_source_paint_locked(
            &mut s,
            SourcePaintLockedSample::Dab {
                pass,
                face_index: 0,
                pressure: 0.5,
            },
        )
        .unwrap();
    assert_eq!(
        r.producer.unwrap().kind,
        crate::SourcePaintNormalProducerKind::RetainedGeometry
    );
    assert!(s.cold.is_none());
    d.end_source_paint_locked(&mut s).unwrap();
    let after = d.source_geometry_state(1).unwrap();
    assert!(Arc::ptr_eq(&state, &after));
    assert_eq!(after.vertex_normals(), normals);
}
#[test]
fn l14_exact_vertex_idw_undefined_and_zero_normals_do_not_disable_alpha_none() {
    let mut d = Document::new(mesh());
    let mut o = options();
    o.culling = true;
    let mut s = d
        .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let r = d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    assert_eq!(r.live.changed_vertices, 3);
    assert!(matches!(
        r.picked_normal,
        Some(SourcePickedNormalDisposition::Undefined { .. })
    ));
    first(&d);
    d.cancel_source_paint_locked(&mut s).unwrap();
    // Two disconnected opposite faces meeting atv0 have a genuinezero rawmean.
    let mut m = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv -1 0 0\nv 0 -1 0\nf 1 2 3\nf 1 5 4\n")
        .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([0.; 3]);
    // Isolate zero-normal/culling from the independently tested None material
    // representation transition, which changes the live storage-presence count.
    m.materials = Some(vec![crate::DEFAULT_VERTEX_MATERIAL; 5]);
    assert_eq!(
        m.positions.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
        [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.]
        ]
    );
    assert_eq!(m.triangles, [[0, 1, 2], [0, 4, 3]]);
    let field = SourceNormalField::compute_with_budget(
        &m,
        SourceNormalPolicy::StaticOriginalFaces,
        MAX_SOURCE_NORMAL_BYTES,
    )
    .unwrap();
    assert_eq!(
        field
            .face_normals()
            .iter()
            .map(|n| n.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        [[0, 0, 0x3f800000], [0x80000000, 0x80000000, 0xbf800000]]
    );
    assert_eq!(
        field.vertex_normals()[0].to_array().map(f32::to_bits),
        [0; 3]
    );
    let mut d = Document::new(m);
    let mut s = d
        .begin_source_paint_locked(&o, SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let r = d.apply_source_paint_locked(&mut s, sample(0.25)).unwrap();
    assert_eq!(r.dab.first_selected_vertices, 1);
    assert_eq!(r.dab.changed_vertices, 1);
    assert!(!r.dab.materialized);
    assert_eq!(r.live.changed_rgb_vertices, 1);
    assert_eq!(r.live.changed_vertices, 1);
    assert_eq!(colors(&d)[0], [0x3f000000, 0x3e800000, 0x3e000000]);
}
#[test]
fn l15_public_external_edit_rejects_owned_transaction_and_restores_all_words() {
    for which in 0..7 {
        let mut d = Document::new(mesh());
        let before = d.to_json().unwrap();
        let mut s = d
            .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
            .unwrap();
        d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        match which {
            0 => d.require_active_mesh_mut().unwrap().positions[1].x += 0.125,
            1 => d.require_active_mesh_mut().unwrap().normals[1] = Vec3::X,
            2 => d.require_active_mesh_mut().unwrap().masks[1] = 0.5,
            3 => d.require_active_mesh_mut().unwrap().colors[1] = [0.75; 3],
            4 => {
                d.require_active_mesh_mut()
                    .unwrap()
                    .materials
                    .as_mut()
                    .unwrap()[1] = [0.5; 2]
            }
            5 => d.name = "outside slot name change".into(),
            _ => d.require_active_mesh_mut().unwrap().surface = None,
        }
        assert!(d.apply_source_paint_locked(&mut s, sample(1.)).is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(s.closed);
        assert!(d.source_geometry_active.is_none());
        assert!(!d.can_undo());
    }
    let mut d = Document::new(mesh());
    prior_redo(&mut d);
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    d.require_active_mesh_mut().unwrap().colors.clear();
    assert!(d.require_active_mesh().unwrap().validate().is_err());
    d.cancel_source_paint_locked(&mut s).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());

    for edit in 0..4 {
        let raw = crate::SglScene {
            version: 3,
            byte_order: crate::SglByteOrder::LittleEndian,
            display: Some(crate::SglDisplaySettings {
                show_grid: false,
                show_symmetry_line: false,
                show_contour: false,
            }),
            camera: Some(crate::SglCameraSettings {
                projection: 0,
                mode: 0,
                fov_degrees: 30.,
                use_pivot: false,
            }),
            meshes: vec![crate::SglMesh {
                render: Some(crate::SglRenderSettings {
                    shader: 0,
                    matcap: 0,
                    show_wireframe: false,
                    flat_shading: false,
                    opacity: 1.,
                }),
                center: [0.; 3],
                matrix: [
                    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
                ],
                scale: 1.,
                positions: vec![[0., 0., 0.], [0.5, 0., 0.], [0., 0.5, 0.]],
                colors: Some(vec![[0.; 3]; 3]),
                materials: Some(vec![[0.25, 0.75, 1.]; 3]),
                faces: vec![[0, 1, 2, TRIANGLE_CORNER]],
                uv: None,
            }],
        };
        let mut d = Document::new(mesh());
        d.replace_sgl_bytes(
            &raw.to_bytes().unwrap(),
            &crate::SglStageSettings::default(),
        )
        .unwrap();
        prior_redo(&mut d);
        let before = d.to_json().unwrap();
        let mut s = d
            .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
            .unwrap();
        d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
        match edit {
            0 => {
                d.active_metadata
                    .as_mut()
                    .unwrap()
                    .affine
                    .interaction_center[0] = 0.375
            }
            1 => {
                let origin = d.active_metadata.as_mut().unwrap().origin.as_mut().unwrap();
                Arc::make_mut(&mut origin.object.raw)
                    .colors
                    .as_mut()
                    .unwrap()[0] = [0.125; 3];
            }
            2 => {
                d.scene_origin.as_mut().unwrap().camera = Some(crate::SglCameraSettings {
                    projection: 0,
                    mode: 0,
                    fov_degrees: 45.,
                    use_pivot: false,
                })
            }
            _ => d.scene.as_mut().unwrap().selected_ids.clear(),
        }
        assert!(d.apply_source_paint_locked(&mut s, sample(1.)).is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert!(s.at_original);
    }
}
#[test]
fn l16_fresh_restored_foreign_and_expired_owners_cannot_apply_cancel_or_end() {
    let mut a = Document::new(mesh());
    let mut b = Document::new(mesh());
    a.source_geometry_token = 17;
    b.source_geometry_token = 17;
    let mut sa = a
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let mut sb = b
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    assert_ne!(sa.token, sb.token);
    let before_b = b.to_json().unwrap();
    assert!(b.apply_source_paint_locked(&mut sa, sample(1.)).is_err());
    assert!(b.cancel_source_paint_locked(&mut sa).is_err());
    assert!(b.end_source_paint_locked(&mut sa).is_err());
    assert_eq!(b.to_json().unwrap(), before_b);
    let mut restored = Document::from_json(&a.to_json().unwrap()).unwrap();
    let before = restored.to_json().unwrap();
    assert!(restored
        .apply_source_paint_locked(&mut sa, sample(1.))
        .is_err());
    assert!(restored.cancel_source_paint_locked(&mut sa).is_err());
    assert!(restored.end_source_paint_locked(&mut sa).is_err());
    assert_eq!(restored.to_json().unwrap(), before);
    a.cancel_source_paint_locked(&mut sa).unwrap();
    let mut next = a
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    assert!(a.cancel_source_paint_locked(&mut sa).is_err());
    a.cancel_source_paint_locked(&mut next).unwrap();
    b.cancel_source_paint_locked(&mut sb).unwrap();
}
#[test]
fn l17_limits_fail_before_claim_and_owned_late_failure_restores_redo() {
    let mut d = Document::new(mesh());
    prior_redo(&mut d);
    let before = d.to_json().unwrap();
    assert!(d
        .begin_source_paint_locked_with_limit(&options(), SourcePaintLockedMode::AbsolutePreview, 1)
        .is_err());
    assert!(d.stroke_start.is_none());
    assert!(d.source_geometry_active.is_none());
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    s.operation_limit = 1;
    assert!(d.apply_source_paint_locked(&mut s, sample(1.)).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn l18_real_move_keeps_owner_and_closed_cache_shared_docs_get_fresh_claims() {
    let mut d = Document::new(mesh());
    warm(&mut d);
    let mut other = Document::new(d.require_active_mesh().unwrap().snapshot());
    other.source_geometry_states = d.source_geometry_states.clone();
    other.source_geometry_token = d.source_geometry_token;
    let mut sa = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    let mut sb = other
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    assert_ne!(sa.token, sb.token);
    let mut moved = d;
    assert!(other.cancel_source_paint_locked(&mut sa).is_err());
    moved.cancel_source_paint_locked(&mut sa).unwrap();
    other.cancel_source_paint_locked(&mut sb).unwrap();
}
#[test]
fn l19_one_complete_history_step_material_preview_stack_and_other_part_binding() {
    let mut d = Document::new(mesh());
    let id = d.add_subtool(mesh(), "Other").unwrap();
    d.select_subtool(1).unwrap();
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    d.end_source_paint_locked(&mut s).unwrap();
    let edited = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), edited);
    assert!(d.subtool_mesh(id).is_ok());
    let mut d = Document::new(mesh());
    d.require_active_mesh_mut().unwrap().materials = None;
    d.subdivide().unwrap();
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    d.end_source_paint_locked(&mut s).unwrap();
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn l20_release_failure_and_sequence_exhaustion_restore_original_and_generic_refusals_remain() {
    let mut d = Document::new(mesh());
    prior_redo(&mut d);
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    d.apply_source_paint_locked(&mut s, sample(1.)).unwrap();
    d.name = "stale at release".into();
    assert!(d.end_source_paint_locked(&mut s).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    let mut d = Document::new(mesh());
    let before = d.to_json().unwrap();
    let mut s = d
        .begin_source_paint_locked(&options(), SourcePaintLockedMode::AbsolutePreview)
        .unwrap();
    s.sequence = u64::MAX;
    assert!(d.cancel_source_paint_locked(&mut s).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.source_geometry_active.is_none());
    let counter = std::sync::atomic::AtomicU64::new(u64::MAX);
    assert!(super::super::source_geometry_document::allocate_source_claim(&counter).is_err());
    let SourcePaintLockedSample::Dab { pass, .. } = sample(1.) else {
        panic!()
    };
    for op in [
        PaintResponseOperation::LockedPreview,
        PaintResponseOperation::LockedAccumulation,
    ] {
        let mut m = mesh();
        assert!(m
            .apply_paint_response(&PaintResponseRequest {
                kind: BrushKind::Paint,
                operation: op,
                in_contact: true,
                options: options(),
                first: pass,
                mirrored: None
            })
            .is_err());
    }
}
