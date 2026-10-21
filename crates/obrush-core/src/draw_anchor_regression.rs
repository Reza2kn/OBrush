//! Authored near-edge Draw anchor fixtures; include as a child of `brush.rs` in tests.
use super::*;
use crate::Ray;

fn steep_triangle() -> Mesh {
    let mut mesh = Mesh::new(
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.001, 0.0, 0.01),
            Vec3::new(0.0, 0.001, 0.01),
        ],
        vec![[0, 1, 2]],
    );
    // Shading normals can differ sharply from the geometric face normal on a
    // sculpted crease. The camera ray, not a normal-plane projection, picks.
    mesh.normals.fill(Vec3::Z);
    mesh.ensure_adjacency();
    mesh
}

#[test]
fn camera_ray_anchor_accepts_valid_near_edge_hit() {
    let mesh = steep_triangle();
    for (x, y) in [(0.000_000_002, 0.000_002), (0.000_002, 0.000_000_002)] {
        let ray = Ray {
            origin: Vec3::new(x, y, 1.0),
            direction: -Vec3::Z,
        };
        let (_, hit) = mesh.raycast_indexed(ray).expect("authored interior ray");
        assert_eq!(hit.triangle, [0, 1, 2]);
        let anchor = mesh
            .draw_contact_anchor(&mesh.positions, hit.position, Vec3::Z, 0.05)
            .expect("near-edge picked face remains eligible");
        assert_eq!(anchor.ids, [0, 1, 2]);
        assert!(anchor.bary.iter().all(|&weight| weight >= 0.0));
        assert!(anchor.bary.iter().any(|&weight| weight < 0.001));
        assert!((anchor.bary.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        let repeated = mesh
            .draw_contact_anchor(&mesh.positions, hit.position, Vec3::Z, 0.05)
            .unwrap();
        assert_eq!(anchor.ids, repeated.ids);
        assert_eq!(
            anchor.bary.map(f64::to_bits),
            repeated.bary.map(f64::to_bits)
        );
    }
    mesh.validate().unwrap();
}

#[test]
fn camera_ray_anchor_rejects_true_outside_hit() {
    let mesh = steep_triangle();
    let ray = Ray {
        origin: Vec3::new(-0.000_002, 0.000_002, 1.0),
        direction: -Vec3::Z,
    };
    assert!(mesh.raycast(ray).is_none());
    // A hand supplied center just outside the face must not manufacture an
    // anchor even when the nearest vertex is within the nominal brush radius.
    assert!(mesh
        .draw_contact_anchor(
            &mesh.positions,
            Vec3::new(-0.000_002, 0.000_002, 0.0),
            Vec3::Z,
            0.05
        )
        .is_none());
}
