use obrush_core::{Mesh, Vec3};
use std::collections::HashMap;

#[test]
fn quad_sphere_levels_are_closed_and_outward_with_expected_density() {
    for levels in [0_u32, 1, 3, 7] {
        let mesh = Mesh::sphere_quad_smooth(levels).unwrap();
        let faces = 6 * 4_usize.pow(levels);
        assert_eq!(mesh.positions.len(), faces + 2);
        assert_eq!(mesh.surface.as_ref().unwrap().faces.len(), faces);
        assert_eq!(mesh.triangles.len(), faces * 2);
        mesh.validate().unwrap();

        let mut edges = HashMap::<[u32; 2], usize>::new();
        for face in &mesh.surface.as_ref().unwrap().faces {
            for i in 0..4 {
                let a = face[i];
                let b = face[(i + 1) % 4];
                let edge = [a.min(b), a.max(b)];
                *edges.entry(edge).or_default() += 1;
            }
        }
        assert_eq!(edges.len(), faces * 2);
        assert!(edges.values().all(|&uses| uses == 2));
        assert_eq!(
            mesh.positions.len() as isize - edges.len() as isize + faces as isize,
            2
        );
        assert!(mesh.positions.iter().all(|position| position.is_finite()));
        assert!(mesh
            .positions
            .iter()
            .zip(&mesh.normals)
            .all(|(position, normal)| normal.is_finite() && normal.dot(*position) > 0.0));
    }
}

#[test]
fn first_smooth_level_moves_cube_corner_and_preserves_axis_symmetry() {
    let mesh = Mesh::sphere_quad_smooth(1).unwrap();
    let corner = Vec3::splat(5.0 / 9.0);
    assert!(mesh
        .positions
        .iter()
        .any(|position| (*position - corner).length() < 1e-6));
    for position in &mesh.positions {
        assert!(mesh
            .positions
            .iter()
            .any(|other| (*other + *position).length() < 1e-6));
    }
    assert!(Mesh::sphere_quad_smooth(8).is_err());
}
