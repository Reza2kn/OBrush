use crate::{Document, Mesh, StaticSubdivisionSettings, TRIANGLE_CORNER};

#[test]
fn source_seed_matrix_matches_pinned_default_primitive_scales() {
    // Pinned first-add `normalizeSize()` matrices, including the subsequent
    // 0.7 product for Cube/Cylinder. Sphere/Cube have a factor-two different
    // independently authored local seed, so their stored matrix is half of
    // the reference's and their world surfaces are the comparison target.
    let cases: [(Mesh, f32, f32); 4] = [
        (Mesh::cube_quad_grid(0).unwrap(), 1.0, 57.735_027 / 2.0),
        (Mesh::cube_quad_grid(0).unwrap(), 0.7, 40.414_52 / 2.0),
        (Mesh::source_cylinder_base().unwrap(), 0.7, 28.577_381),
        (Mesh::source_torus_base().unwrap(), 1.0, 58.520_573),
    ];
    for (mesh, fraction, expected) in cases {
        let affine = crate::primitives::source_primitive_affine(&mesh, fraction).unwrap();
        for index in [0, 5, 10] {
            assert!((affine.local_to_world[index] - expected).abs() < 0.000_02);
        }
        assert_eq!(affine.local_to_world[12..15], [0.0; 3]);
    }
    assert!(
        crate::primitives::source_primitive_affine(&Mesh::cube_quad_grid(0).unwrap(), 0.9).is_err()
    );
}

type PrimitiveCase = (
    fn() -> Result<Mesh, String>,
    fn() -> Result<Mesh, String>,
    bool,
    usize,
    usize,
    (usize, usize, usize),
);

fn position_keys(mesh: &Mesh) -> Vec<[u32; 3]> {
    let mut keys: Vec<_> = mesh
        .positions
        .iter()
        .map(|p| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()])
        .collect();
    keys.sort_unstable();
    keys
}

fn face_coordinate_keys(mesh: &Mesh) -> Vec<[[u32; 3]; 4]> {
    let mut keys: Vec<_> = mesh
        .surface
        .as_ref()
        .expect("Source primitive has original faces")
        .faces
        .iter()
        .map(|face| {
            let mut corners = face.map(|id| {
                if id == TRIANGLE_CORNER {
                    [u32::MAX; 3]
                } else {
                    let p = mesh.positions[id as usize];
                    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
                }
            });
            corners.sort_unstable();
            corners
        })
        .collect();
    keys.sort_unstable();
    keys
}

#[test]
fn source_default_primitives_retain_clamped_levels_and_history() {
    // Independently selected base/step contracts from pinned Scene.subdivideClamp:
    // cube seeds need seven refinements; cylinder/torus need two.
    let cases: [PrimitiveCase; 4] = [
        (
            || Mesh::cube_quad_grid(0),
            || Mesh::sphere_quad_smooth(7),
            false,
            7,
            4,
            (98_306, 98_304, 196_608),
        ),
        (
            || Mesh::cube_quad_grid(0),
            || Mesh::cube_quad_grid(7),
            true,
            7,
            4,
            (98_306, 98_304, 196_608),
        ),
        (
            Mesh::source_cylinder_base,
            Mesh::source_default_cylinder,
            false,
            2,
            3,
            (66_562, 67_584, 133_120),
        ),
        (
            Mesh::source_torus_base,
            Mesh::source_default_torus,
            false,
            2,
            3,
            (65_536, 65_536, 131_072),
        ),
    ];
    for (index, (base, former_final, linear_edges, steps, retained, counts)) in
        cases.into_iter().enumerate()
    {
        let mut doc = Document::empty();
        doc.add_source_primitive_subtool(
            base().unwrap(),
            &format!("Primitive {index}"),
            &StaticSubdivisionSettings { linear_edges },
            steps,
            if matches!(index, 1 | 2) { 0.7 } else { 1.0 },
        )
        .unwrap();
        assert_eq!(doc.level_count(), retained);
        assert_eq!(doc.current_level(), retained - 1);
        let fine = doc.active_mesh().unwrap();
        assert_eq!(
            (
                fine.positions.len(),
                fine.face_count(),
                fine.triangles.len()
            ),
            counts
        );
        let previous = former_final().unwrap();
        assert_eq!(position_keys(fine), position_keys(&previous));
        assert_eq!(face_coordinate_keys(fine), face_coordinate_keys(&previous));
        let fine_snapshot = fine.snapshot();

        let saved = doc.to_json().unwrap();
        let loaded = Document::from_json(&saved).unwrap();
        assert_eq!(loaded.level_count(), retained);
        assert!(loaded.active_mesh().unwrap().same_surface(&fine_snapshot));
        doc.select_level(0).unwrap();
        assert_eq!(doc.current_level(), 0);
        let low = doc.active_mesh().unwrap();
        assert_eq!(
            low.face_count(),
            if steps == 7 {
                1_536
            } else if index == 2 {
                4_224
            } else {
                4_096
            }
        );
        doc.select_level(retained - 1).unwrap();
        assert!(doc.active_mesh().unwrap().same_surface(&fine_snapshot));
        assert!(doc.undo());
        assert_eq!(doc.current_level(), 0);
        assert!(doc.redo());
        assert!(doc.active_mesh().unwrap().same_surface(&fine_snapshot));
    }
}

#[test]
fn invalid_source_primitive_level_request_leaves_document_and_redo_alone() {
    let mut doc = Document::empty();
    doc.add_subtool(Mesh::sphere(0), "kept").unwrap();
    assert!(doc.undo());
    let before = doc.to_json().unwrap();
    let history = doc.history_depths();
    assert!(doc
        .add_source_primitive_subtool(
            Mesh::cube_quad_grid(0).unwrap(),
            "Rejected",
            &StaticSubdivisionSettings::default(),
            8,
            1.0,
        )
        .is_err());
    assert_eq!(doc.to_json().unwrap(), before);
    assert_eq!(doc.history_depths(), history);
    assert!(doc.redo());
}
