//! Independently declared alphaNone corner/zero acceptance; reference runtime open.
use obrush_core::{
    prepare_source_paint_pass_alpha_none, prepare_source_paint_pass_with_picked_normal,
    prepare_source_picked_normal, prepare_source_picked_normal_alpha_none,
    prepare_source_picked_normal_typed, BrushKind, Document, MaskConvention, Mesh,
    PaintPreparationInput, PaintResponseOperation, PaintResponseOptions, PaintResponseRequest,
    PaintViewCapture, SourceNormalField, SourceNormalPolicy, SourceOriginalFaceContact,
    SourcePickedNormalDisposition, SourcePickedNormalError, SourcePickedNormalRejection,
    SourcePickedNormalUndefined,
};
fn mesh() -> Mesh {
    let mut m = Mesh::import_obj(b"v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n")
        .unwrap()
        .mesh;
    m.colors.fill([0.; 3]);
    m.install_mask_channel(vec![1.; 3], MaskConvention::UnprotectedF32V1)
        .unwrap();
    m
}
fn field(m: &Mesh) -> SourceNormalField {
    SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap()
}
fn corner() -> SourceOriginalFaceContact {
    SourceOriginalFaceContact {
        face_index: 0,
        local_point: [-0.5, -0.5, 0.],
    }
}
fn input() -> PaintPreparationInput {
    PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        },
        screen: [25., 75.],
        screen_radius: 5.,
        pressure: 0.5,
        radius_factor: 0.,
    }
}
fn request(pass: obrush_core::PaintResponsePass) -> PaintResponseRequest {
    PaintResponseRequest {
        kind: BrushKind::Paint,
        operation: PaintResponseOperation::OrdinaryDab,
        in_contact: true,
        first: pass,
        mirrored: None,
        options: PaintResponseOptions {
            intensity: 0.25,
            hardness: 1.,
            color: [1., 0.5, 0.25],
            roughness: 0.5,
            metallic: 0.75,
            culling: false,
            ..Default::default()
        },
    }
}
#[test]
fn strict_query_error_and_alpha_none_corner_disposition_are_distinct_typed_states() {
    let m = mesh();
    let f = field(&m);
    let before = serde_json::to_vec(&m).unwrap();
    assert!(prepare_source_picked_normal(&m, &f, corner()).is_err());
    assert_eq!(
        prepare_source_picked_normal_typed(&m, &f, corner()).unwrap_err(),
        SourcePickedNormalError::Undefined(SourcePickedNormalUndefined::ExactVertexContact {
            corner: 0,
            vertex_id: 0
        })
    );
    let admitted = prepare_source_picked_normal_alpha_none(&m, &f, corner()).unwrap();
    assert!(matches!(
        admitted,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ExactVertexContact {
                corner: 0,
                vertex_id: 0
            },
            raw_report: None,
            ..
        }
    ));
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn inverse_distance_overflow_is_undefined_without_fallback_or_nonfinite_report() {
    let m = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
        .unwrap()
        .mesh;
    let f = field(&m);
    let c = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [f64::from_bits(1), 0., 0.],
    };
    assert!(prepare_source_picked_normal(&m, &f, c).is_err());
    assert_eq!(
        prepare_source_picked_normal_typed(&m, &f, c).unwrap_err(),
        SourcePickedNormalError::Undefined(SourcePickedNormalUndefined::InverseDistanceOverflow {
            corner: 0,
            vertex_id: 0
        })
    );
    let disposition = prepare_source_picked_normal_alpha_none(&m, &f, c).unwrap();
    assert!(matches!(
        disposition,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::InverseDistanceOverflow { .. },
            raw_report: None,
            ..
        }
    ));
    let json = serde_json::to_value(disposition).unwrap();
    assert!(json["raw_report"].is_null());
    assert_eq!(json["reason"]["reason"], "inverse_distance_overflow");
}
#[test]
fn zero_direction_retains_finite_raw_zero_report_but_cannot_claim_a_frame() {
    let m = Mesh::import_obj(b"v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\nf 1 3 2\n")
        .unwrap()
        .mesh;
    let f = field(&m);
    let c = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [0., 0., 0.],
    };
    let old = prepare_source_picked_normal(&m, &f, c).unwrap();
    assert_eq!(old.normal64, [0.; 3]);
    assert!(!old.direction_defined);
    let d = prepare_source_picked_normal_alpha_none(&m, &f, c).unwrap();
    assert!(matches!(
        d,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ZeroDirection,
            raw_report: Some(_),
            ..
        }
    ));
    assert_eq!(
        serde_json::to_vec(&d.raw_report().unwrap()).unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
}
#[test]
fn alpha_none_does_not_swallow_stale_invalid_contact_or_missing_face_rejections() {
    let mut m = mesh();
    let f = field(&m);
    m.positions[0].x = -0.75;
    assert_eq!(
        prepare_source_picked_normal_alpha_none(&m, &f, corner()).unwrap_err(),
        SourcePickedNormalRejection::StaleOrNonstaticField
    );
    let m = mesh();
    let f = field(&m);
    for value in [f64::NAN, f64::INFINITY, 1_000_001.] {
        let mut c = corner();
        c.local_point[0] = value;
        assert_eq!(
            prepare_source_picked_normal_alpha_none(&m, &f, c).unwrap_err(),
            SourcePickedNormalRejection::InvalidContact
        );
    }
    let mut c = corner();
    c.face_index = usize::MAX;
    assert_eq!(
        prepare_source_picked_normal_alpha_none(&m, &f, c).unwrap_err(),
        SourcePickedNormalRejection::MissingOriginalFace
    );
}
#[test]
fn real_precise_vertex_preparation_admits_only_alpha_none_and_preserves_old_guard() {
    let m = mesh();
    let before = serde_json::to_vec(&m).unwrap();
    assert!(prepare_source_paint_pass_with_picked_normal(&m, None, input(), &mut None).is_err());
    let pass = prepare_source_paint_pass_alpha_none(&m, None, input(), &mut None)
        .unwrap()
        .unwrap();
    assert_eq!(pass.hit.local_position, [-0.5, -0.5, 0.]);
    assert_eq!(pass.pass.center, pass.hit.local_position);
    assert!(pass.pass.alpha_frame.is_none());
    assert!(matches!(
        pass.picked_normal,
        SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ExactVertexContact { .. },
            ..
        }
    ));
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn alpha_none_corner_executes_shared_kernel_with_literal_rgb_material_words() {
    let mut m = mesh();
    let original = m.clone();
    let f = field(&m);
    let prepared = prepare_source_paint_pass_alpha_none(&m, None, input(), &mut None)
        .unwrap()
        .unwrap();
    let r = m
        .apply_paint_response_with_source_normals(&request(prepared.pass), &f)
        .unwrap();
    assert_eq!(r.changed_vertices, 1);
    assert_eq!(r.maximum_weight, 0.25);
    assert!(r.materialized);
    assert_eq!(
        m.colors[0].map(f32::to_bits),
        [0x3e800000, 0x3e000000, 0x3d800000]
    );
    assert_eq!(
        m.materials.as_ref().unwrap()[0].map(f32::to_bits),
        [0x3e851eb8, 0x3e7d70a4]
    );
    assert_eq!(&m.colors[1..], &original.colors[1..]);
    assert!(m.materials.as_ref().unwrap()[1..]
        .iter()
        .all(|r| *r == [0.18, 0.08]));
    m.colors = original.colors.clone();
    m.materials = original.materials.clone();
    assert_eq!(
        serde_json::to_vec(&m).unwrap(),
        serde_json::to_vec(&original).unwrap()
    );
}
#[test]
fn protected_or_no_channels_vertex_noops_preserve_lazy_storage_and_useful_redo() {
    for protected in [false, true] {
        let mut m = mesh();
        if protected {
            m.masks.fill(0.);
        }
        let mut d = Document::new(m);
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[1] = [0.5; 3];
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let f = field(d.require_active_mesh().unwrap());
        let prepared = prepare_source_paint_pass_alpha_none(
            d.require_active_mesh().unwrap(),
            None,
            input(),
            &mut None,
        )
        .unwrap()
        .unwrap();
        let mut req = request(prepared.pass);
        if !protected {
            req.options.write_rgb = false;
            req.options.write_roughness = false;
            req.options.write_metallic = false;
        }
        d.begin_material_paint_stroke();
        assert_eq!(
            d.require_active_mesh_mut()
                .unwrap()
                .apply_paint_response_with_source_normals(&req, &f)
                .unwrap()
                .changed_vertices,
            0
        );
        d.end_stroke();
        assert_eq!(before, d.to_json().unwrap());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(d.redo());
    }
}
#[test]
fn defined_disposition_keeps_exact_old_number_report_and_readonly_mesh() {
    let m = mesh();
    let f = field(&m);
    let before = serde_json::to_vec(&m).unwrap();
    let c = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [0., 0., 0.],
    };
    let old = prepare_source_picked_normal(&m, &f, c).unwrap();
    let new = prepare_source_picked_normal_alpha_none(&m, &f, c).unwrap();
    assert!(matches!(new, SourcePickedNormalDisposition::Defined { .. }));
    assert_eq!(
        serde_json::to_vec(&old).unwrap(),
        serde_json::to_vec(&new.raw_report().unwrap()).unwrap()
    );
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
