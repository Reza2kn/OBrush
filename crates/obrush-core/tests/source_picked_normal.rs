//! Independent rational contacts and literal scalar bounds; source runtime open.
use obrush_core::{
    prepare_source_paint_pass_with_normals, prepare_source_paint_pass_with_picked_normal,
    prepare_source_picked_normal, Mesh, ObjectAffineF32V1, PaintPreparationInput, PaintViewCapture,
    SourceNormalField, SourceNormalPolicy, SourceOriginalFaceContact,
};
fn quad() -> Mesh {
    Mesh::import_obj(b"v 0 0 0\nv 2 0 0\nv 2 1 1\nv 0 1 0\nf 1 2 3 4\n")
        .unwrap()
        .mesh
}
fn varied() -> Mesh {
    Mesh::import_obj(b"v 0 0 0\nv 2 0 0\nv 2 1 1\nv 0 1 0\nv 2 1 0\nv 2 0 1\nf 1 2 3 4\nf 2 5 6\n")
        .unwrap()
        .mesh
}
fn field(m: &Mesh) -> SourceNormalField {
    SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap()
}
fn pick(
    m: &Mesh,
    face: usize,
    point: [f64; 3],
) -> Result<obrush_core::SourcePickedNormalReport, String> {
    prepare_source_picked_normal(
        m,
        &field(m),
        SourceOriginalFaceContact {
            face_index: face,
            local_point: point,
        },
    )
}
fn near(actual: [f64; 3], expected: [f64; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() <= 2e-15),
        "actual{actual:?} expected{expected:?}"
    );
}
fn view_input(screen: [f64; 2]) -> PaintPreparationInput {
    PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        },
        screen,
        screen_radius: 10.,
        pressure: 0.5,
        radius_factor: 0.,
    }
}
#[test]
fn rational_nonplanar_quad_normal_stays_number_precision_without_a_float32_cast() {
    let m = quad();
    let before = serde_json::to_vec(&m).unwrap();
    let r = pick(&m, 0, [1., 0.5, 0.5]).unwrap();
    assert_eq!(r.corner_count, 4);
    near(r.interpolated64, [-1., -2., 4.]);
    let denominator = 21.0_f64.sqrt();
    near(
        r.normal64,
        [-1. / denominator, -2. / denominator, 4. / denominator],
    );
    assert_ne!(r.normal64[0], f64::from(r.normal64[0] as f32));
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn all_four_original_corners_contribute_instead_of_canonical_barycentrics() {
    let m = varied();
    let r = pick(&m, 0, [1., 0.5, 0.5]).unwrap();
    near(r.interpolated64, [-0.75, -1.75, 3.5]);
    let d = 254.0_f64.sqrt();
    near(r.normal64, [-3. / d, -7. / d, 14. / d]);
    let bary = [
        -1. / 21.0_f64.sqrt(),
        -2. / 21.0_f64.sqrt(),
        4. / 21.0_f64.sqrt(),
    ];
    assert!((r.normal64[0] - bary[0]).abs() > 0.01);
}
#[test]
fn independent_three_four_five_distances_and_triangle_zero_fourth_weight() {
    let m = Mesh::import_obj(b"v 0 0 0\nv 6 0 0\nv 0 8 0\nf 1 2 3\n")
        .unwrap()
        .mesh;
    let r = pick(&m, 0, [3., 4., 0.]).unwrap();
    assert_eq!(r.distances64, [Some(5.), Some(5.), Some(5.), None]);
    assert_eq!(r.weights64[3], 0.);
    near(r.normal64, [0., 0., 1.]);
    assert!(r.weights64[..3]
        .iter()
        .all(|w| (w - 1. / 3.).abs() <= 2e-16));
}
#[test]
fn given_duplicate_original_face_identity_is_never_guessed_from_a_triangle() {
    let m =
        Mesh::import_obj(b"v 0 0 0\nv 2 0 0\nv 2 1 1\nv 0 1 0\nv 0 2 0\nf 1 2 3 4\nf 1 2 3 5\n")
            .unwrap()
            .mesh;
    let a = pick(&m, 0, [1., 0.5, 0.5]).unwrap();
    let b = pick(&m, 1, [1., 0.5, 0.5]).unwrap();
    assert_eq!(a.vertex_ids, [0, 1, 2, 3]);
    assert_eq!(b.vertex_ids, [0, 1, 2, 4]);
    assert_ne!(a.normal64, b.normal64);
}
#[test]
fn every_exact_quad_vertex_contact_rejects_the_source_nan_weight_state() {
    let m = quad();
    let before = serde_json::to_vec(&m).unwrap();
    for p in &m.positions {
        assert!(pick(&m, 0, p.to_array().map(f64::from))
            .unwrap_err()
            .contains("undefined"));
    }
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn near_vertex_number_distance_survives_where_float32_contact_would_be_exact() {
    let m = quad();
    let eps = 2.0_f64.powi(-40);
    let point = [2. + eps, 0., 0.];
    let r = pick(&m, 0, point).unwrap();
    assert_eq!(r.local_point64, point);
    assert_eq!(r.distances64[1], Some(eps));
    assert_eq!(point[0] as f32, 2.);
}
#[test]
fn hypot_preserves_tiny_distance_that_squared_distance_would_underflow() {
    let m = quad();
    let point = [1e-300, 0., 0.];
    let r = pick(&m, 0, point).unwrap();
    assert_eq!(r.distances64[0], Some(1e-300));
    assert_eq!(point[0] * point[0], 0.);
    assert!(r.direction_defined);
}
#[test]
fn inverse_distance_overflow_is_rejected_atomically_without_corner_snap() {
    let m = quad();
    let before = serde_json::to_vec(&m).unwrap();
    assert!(pick(&m, 0, [f64::from_bits(1), 0., 0.]).is_err());
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn zero_interpolated_normal_is_reported_not_replaced_with_a_unit_axis() {
    let m = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 2 0 0\nf 1 2 3\n")
        .unwrap()
        .mesh;
    let r = pick(&m, 0, [0.5, 1., 0.]).unwrap();
    assert_eq!(r.interpolated64, [0.; 3]);
    assert_eq!(r.normal64, [0.; 3]);
    assert!(!r.direction_defined);
}
#[test]
fn nonfinite_excessive_and_missing_original_face_inputs_are_real_errors() {
    let m = quad();
    for x in [f64::NAN, f64::INFINITY, 1_000_001., -1_000_001.] {
        assert!(pick(&m, 0, [x, 0.5, 0.5]).is_err());
    }
    assert!(pick(&m, usize::MAX, [1., 0.5, 0.5]).is_err());
}
#[test]
fn stale_position_words_or_original_canonical_binding_cannot_supply_a_normal() {
    let mut m = quad();
    let f = field(&m);
    m.positions[1].x = 3.;
    assert!(prepare_source_picked_normal(
        &m,
        &f,
        SourceOriginalFaceContact {
            face_index: 0,
            local_point: [1., 0.5, 0.5]
        }
    )
    .is_err());
    let mut m = quad();
    let f = field(&m);
    m.triangles[1] = [0, 3, 2];
    assert!(prepare_source_picked_normal(
        &m,
        &f,
        SourceOriginalFaceContact {
            face_index: 0,
            local_point: [1., 0.5, 0.5]
        }
    )
    .is_err());
}
#[test]
fn protection_rgb_and_materials_do_not_weight_original_face_normal_interpolation() {
    let mut m = varied();
    let f = field(&m);
    let contact = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [1., 0.5, 0.5],
    };
    let a = prepare_source_picked_normal(&m, &f, contact).unwrap();
    m.masks.fill(1.);
    m.colors.fill([0.25; 3]);
    m.materials = Some(vec![[0.1, 0.9]; 6]);
    let b = prepare_source_picked_normal(&m, &f, contact).unwrap();
    assert_eq!(a.normal64, b.normal64);
    assert_eq!(a.weights64, b.weights64);
}
#[test]
fn affine_preparation_preserves_local_number_normal_and_old_pass_parameters() {
    let m = varied();
    let matrix = [
        0.5, 0., 0., 0., 0., 0.5, 0., 0., 0., 0., 0.5, 0., -0.25, -0.125, 0., 1.,
    ];
    let pose = ObjectAffineF32V1::from_local_points(matrix, &m.positions).unwrap();
    let input = view_input([62.5, 43.75]);
    let mut old_cache = None;
    let old = prepare_source_paint_pass_with_normals(&m, Some(&pose), input, &mut old_cache)
        .unwrap()
        .unwrap();
    let mut cache = None;
    let new = prepare_source_paint_pass_with_picked_normal(&m, Some(&pose), input, &mut cache)
        .unwrap()
        .unwrap();
    assert_eq!(new.pass.center, old.0.center);
    assert_eq!(new.pass.local_radius_squared, old.0.local_radius_squared);
    assert_eq!(new.hit.triangle_index, old.1.triangle_index);
    near(new.hit.local_position, [1., 0.5, 0.5]);
    let d = 254.0_f64.sqrt();
    near(new.picked_normal.normal64, [-3. / d, -7. / d, 14. / d]);
}
#[test]
fn reflection_and_shear_do_not_bake_world_normal_into_local_picking_output() {
    let m = varied();
    let matrix = [
        0.5, 0., 0., 0., 0.25, 0.5, 0., 0., 0., 0., -0.5, 0., -0.25, -0.125, 0.5, 1.,
    ];
    let pose = ObjectAffineF32V1::from_local_points(matrix, &m.positions).unwrap();
    let mut cache = None;
    let new = prepare_source_paint_pass_with_picked_normal(
        &m,
        Some(&pose),
        view_input([68.75, 43.75]),
        &mut cache,
    )
    .unwrap()
    .unwrap();
    near(new.hit.local_position, [1., 0.5, 0.5]);
    let expected = pick(&m, 0, [1., 0.5, 0.5]).unwrap();
    near(new.picked_normal.normal64, expected.normal64);
}
#[test]
fn missed_preparation_does_not_allocate_a_field_or_manufacture_a_picked_normal() {
    let m = quad();
    let mut cache = None;
    assert!(prepare_source_paint_pass_with_picked_normal(
        &m,
        None,
        view_input([0., 100.]),
        &mut cache
    )
    .unwrap()
    .is_none());
    assert!(cache.is_none());
}
#[test]
fn read_only_normal_query_keeps_document_redo_and_complete_model_bytes() {
    let mut d = obrush_core::Document::new(varied());
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().colors[0] = [0.1; 3];
    d.end_stroke();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let m = d.require_active_mesh().unwrap();
    pick(m, 0, [1., 0.5, 0.5]).unwrap();
    assert_eq!(before, d.to_json().unwrap());
    assert!(d.redo());
}
