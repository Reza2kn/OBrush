//! Independently chosen precision/ray/camera/original-face gates; no source runtime claim.
use glam::{DMat4, DVec3};
use obrush_core::{
    prepare_source_paint_hit, prepare_source_paint_pass, Mesh, ObjectAffineF32V1,
    PaintPreparationInput, PaintViewCapture, SceneRay64,
};
fn mesh() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()
}
fn ray(x: f64, y: f64) -> SceneRay64 {
    SceneRay64 {
        origin: DVec3::new(x, y, 1.),
        direction: -DVec3::Z,
    }
}
fn view() -> PaintViewCapture {
    PaintViewCapture {
        world_to_clip: DMat4::IDENTITY.to_cols_array(),
        viewport: [0., 0., 100., 100.],
    }
}
#[test]
fn source_center_preserves_independent_nonrepresentable_double() {
    let h = prepare_source_paint_hit(&mesh(), None, ray(0.99999998, 0.))
        .unwrap()
        .unwrap();
    assert_eq!(h.local_position, [0.99999998, 0., 0.]);
    assert_ne!(h.local_position[0], f64::from(h.local_position[0] as f32));
}
#[test]
fn source_eye_is_normalized_double_and_not_cast_hit_metadata() {
    let r = SceneRay64 {
        origin: DVec3::new(0., 0., 1.),
        direction: DVec3::new(0.1, 0.2, -1.),
    };
    let h = prepare_source_paint_hit(&mesh(), None, r).unwrap().unwrap();
    assert!((h.local_position[0] - 0.1).abs() < 2e-16);
    assert!((h.local_position[1] - 0.2).abs() < 2e-16);
    assert_eq!(h.local_eye_direction, r.direction.normalize().to_array());
}
#[test]
fn source_barycentric_boundary_accepts_tiny_negative_margin() {
    let h = prepare_source_paint_hit(&mesh(), None, ray(0.5, -5e-16))
        .unwrap()
        .unwrap();
    assert_eq!(h.local_position[1], -5e-16);
}
#[test]
fn source_boundary_rejects_outside_preregistered_epsilon() {
    assert!(prepare_source_paint_hit(&mesh(), None, ray(0.5, -2e-15))
        .unwrap()
        .is_none());
}
#[test]
fn original_face_identity_is_ordered_for_duplicate_quads() {
    let m =
        Mesh::import_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\nf 1 2 3 4\n".as_bytes())
            .unwrap()
            .mesh;
    assert_eq!(m.surface.as_ref().unwrap().faces, vec![[0, 1, 2, 3]; 2]);
    let h = prepare_source_paint_hit(&m, None, ray(0.75, 0.25))
        .unwrap()
        .unwrap();
    assert_eq!(h.original_face_index, Some(0));
    assert_eq!(h.triangle_index, 0);
}
#[test]
fn second_quad_triangle_retains_its_real_row_and_face() {
    let m = Mesh::import_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n".as_bytes())
        .unwrap()
        .mesh;
    assert_eq!(m.surface.as_ref().unwrap().faces, vec![[0, 1, 2, 3]; 1]);
    let h = prepare_source_paint_hit(&m, None, ray(0.25, 0.75))
        .unwrap()
        .unwrap();
    assert_eq!(h.original_face_index, Some(0));
    assert_eq!(h.triangle_index, 1);
}
#[test]
fn f64_affine_local_pick_preserves_editable_position_and_matrix_bits() {
    let m = mesh();
    let a = ObjectAffineF32V1::from_local_points(
        [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 100., 20., 0., 1.,
        ],
        &m.positions,
    )
    .unwrap();
    let before = serde_json::to_string(&m).unwrap();
    let h = prepare_source_paint_hit(&m, Some(&a), ray(100.2, 20.3))
        .unwrap()
        .unwrap();
    assert!((h.local_position[0] - 0.2).abs() < 2e-14);
    assert!((h.local_position[1] - 0.3).abs() < 2e-14);
    assert_eq!(serde_json::to_string(&m).unwrap(), before);
}
#[test]
fn project_unproject_are_independent_identity_view_coordinates() {
    let v = view();
    assert_eq!(
        v.unproject([75., 25.], 0.5).unwrap(),
        DVec3::new(0.5, 0.5, 0.)
    );
    assert_eq!(
        v.project(DVec3::new(0.5, 0.5, 0.)).unwrap(),
        [75., 25., 0.5]
    );
}
#[test]
fn source_radius_pressure_is_applied_once_to_squared_local_radius() {
    let m = mesh();
    let input = PaintPreparationInput {
        view: view(),
        screen: [62.5, 37.5],
        screen_radius: 10.,
        pressure: 0.25,
        radius_factor: 0.75,
    };
    let (pass, _) = prepare_source_paint_pass(&m, None, input).unwrap().unwrap();
    let expected = (0.2_f64 * 0.625).powi(2);
    assert!((pass.local_radius_squared - expected).abs() < 2e-17);
    assert_eq!(pass.center, [0.25, 0.25, 0.]);
    assert_eq!(pass.eye_direction, [0., 0., 1.]);
}
#[test]
fn affine_source_scale_row_divides_world_radius_squared_once() {
    let m = mesh();
    let a = ObjectAffineF32V1::from_local_points(
        [
            2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1.,
        ],
        &m.positions,
    )
    .unwrap();
    let (pass, _) = prepare_source_paint_pass(
        &m,
        Some(&a),
        PaintPreparationInput {
            view: view(),
            screen: [75., 25.],
            screen_radius: 10.,
            pressure: 0.5,
            radius_factor: 0.75,
        },
    )
    .unwrap()
    .unwrap();
    assert!((pass.local_radius_squared - 0.01).abs() < 2e-17);
    assert_eq!(pass.center, [0.25, 0.25, 0.]);
}
#[test]
fn invalid_camera_and_source_tablet_reject_without_fallback() {
    let mut v = view();
    v.world_to_clip.fill(0.);
    assert!(v.ray([50., 50.]).is_err());
    let mut input = PaintPreparationInput {
        view: view(),
        screen: [62.5, 37.5],
        screen_radius: 10.,
        pressure: f64::NAN,
        radius_factor: 0.75,
    };
    assert!(prepare_source_paint_pass(&mesh(), None, input).is_err());
    input.pressure = 0.5;
    input.radius_factor = 1.1;
    assert!(prepare_source_paint_pass(&mesh(), None, input).is_err());
}
#[test]
fn behind_camera_ray_is_a_no_hit_and_never_fabricates_a_center() {
    assert!(prepare_source_paint_hit(
        &mesh(),
        None,
        SceneRay64 {
            origin: DVec3::new(0.25, 0.25, -1.),
            direction: -DVec3::Z
        }
    )
    .unwrap()
    .is_none());
}
