//! Independent perspective-depth fixture; source-authored, runtime UNRUN.
use obrush_core::*;

fn fixture() -> (
    Mesh,
    ObjectAffineF32V1,
    PaintPreparationInput,
    SourceMirrorPlane,
) {
    let mesh=Mesh::import_obj(b"v -1 -2 -2\nv -1 2 -2\nv -1 2 2\nv -1 -2 2\nv 1 -2 -2\nv 1 2 -2\nv 1 2 2\nv 1 -2 2\nf 1 2 3 4\nf 5 6 7 8\n").unwrap().mesh;
    let pose = ObjectAffineF32V1::from_local_points(
        [
            0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        &mesh.positions,
    )
    .unwrap();
    let matrix = glam::DMat4::perspective_rh_gl(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 100.0)
        * glam::DMat4::from_translation(glam::DVec3::new(0.0, 0.0, -10.0));
    let input = PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: matrix.to_cols_array(),
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [50.0, 50.0],
        screen_radius: 10.0,
        pressure: 0.5,
        radius_factor: 0.0,
    };
    (
        mesh,
        pose,
        input,
        SourceMirrorPlane {
            origin: [0.0; 3],
            normal: [1.0, 0.0, 0.0],
        },
    )
}

#[test]
fn asymmetric_depth_press_radius_has_independent_primary_and_mirror_values() {
    let (mesh, pose, input, plane) = fixture();
    let before = serde_json::to_string(&mesh).unwrap();
    let contacts =
        prepare_source_geometry_press_contacts(&mesh, Some(&pose), input, Some(plane)).unwrap();
    for (index, (x, squared, face)) in [(-1.0, 3.24, 0), (1.0, 4.84, 1)].into_iter().enumerate() {
        let pass = contacts.passes[index].unwrap();
        assert!((pass.center[0] - x).abs() <= 1e-12);
        assert!(pass.center[1].abs() <= 1e-12 && pass.center[2].abs() <= 1e-12);
        assert!(
            (pass.local_radius_squared - squared).abs() <= 1e-12,
            "Own-depth press radius at local X={x}: {} vs independent {squared}",
            pass.local_radius_squared
        );
        assert_eq!(pass.original_face_index, face);
        assert!(matches!(contacts.triangles[index],Some(row) if row/2==face));
    }
    assert!(
        contacts.passes[1].unwrap().local_radius_squared
            > contacts.passes[0].unwrap().local_radius_squared
    );
    assert_eq!(serde_json::to_string(&mesh).unwrap(), before);
}

#[test]
fn asymmetric_depth_ordinary_dab_still_copies_primary_radius() {
    let (mesh, pose, input, plane) = fixture();
    let press =
        prepare_source_geometry_press_contacts(&mesh, Some(&pose), input, Some(plane)).unwrap();
    let dab = prepare_source_geometry_contacts(&mesh, Some(&pose), input, Some(plane)).unwrap();
    assert_eq!(
        dab.passes[0].unwrap().local_radius_squared.to_bits(),
        press.passes[0].unwrap().local_radius_squared.to_bits()
    );
    assert_eq!(
        dab.passes[1].unwrap().local_radius_squared.to_bits(),
        dab.passes[0].unwrap().local_radius_squared.to_bits()
    );
    assert_ne!(
        dab.passes[1].unwrap().local_radius_squared.to_bits(),
        press.passes[1].unwrap().local_radius_squared.to_bits()
    );
    assert_eq!(dab.triangles, press.triangles);
    for index in 0..2 {
        assert_eq!(
            dab.passes[index].unwrap().center.map(f64::to_bits),
            press.passes[index].unwrap().center.map(f64::to_bits)
        );
    }
}
