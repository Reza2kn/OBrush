use obrush_core::{Document, Mesh, Vec3};
use std::collections::{HashMap, HashSet};

// Independent integer-lattice fixture for the pinned reference's linear
// seven-level cube. The reference count is 6 * 4^7 original quads; cube
// topology gives V = F + 2 and two render triangles per quad.
#[test]
fn seven_level_cube_retains_source_density_quads_and_exact_lattice() {
    let mesh = Mesh::cube_quad_grid(7).unwrap();
    assert_eq!(mesh.positions.len(), 98_306);
    assert_eq!(mesh.surface.as_ref().unwrap().faces.len(), 98_304);
    assert_eq!(mesh.triangles.len(), 196_608);
    mesh.validate().unwrap();

    let mut lattice = HashSet::with_capacity(mesh.positions.len());
    for p in &mesh.positions {
        let key = [p.x, p.y, p.z].map(|value| {
            let scaled = (value + 1.0) * 64.0;
            assert_eq!(scaled.fract(), 0.0);
            assert!((0.0..=128.0).contains(&scaled));
            scaled as u8
        });
        assert!(key.contains(&0) || key.contains(&128));
        assert!(lattice.insert(key));
    }
    assert!(lattice.contains(&[0, 0, 0]));
    assert!(lattice.contains(&[128, 128, 128]));
    assert!(lattice.contains(&[64, 64, 128]));
    assert!(lattice.contains(&[127, 64, 0]));

    let mut edge_use = HashMap::<(u32, u32), u8>::new();
    for &[a, b, c] in &mesh.triangles {
        for (from, to) in [(a, b), (b, c), (c, a)] {
            *edge_use.entry((from.min(to), from.max(to))).or_default() += 1;
        }
        let a = mesh.positions[a as usize];
        let b = mesh.positions[b as usize];
        let c = mesh.positions[c as usize];
        let normal = (b - a).cross(c - a);
        assert!(normal.dot(a + b + c) > 0.0);
    }
    assert_eq!(edge_use.len(), 294_912);
    assert!(edge_use.values().all(|&count| count == 2));
    assert_eq!(
        mesh.positions.len() + mesh.triangles.len(),
        edge_use.len() + 2
    );
    assert!(mesh
        .normals
        .iter()
        .all(|n| n.is_finite() && n.length() > 0.9));
}

#[test]
fn generated_cube_is_bounded_and_undo_restores_scene() {
    assert!(Mesh::cube_quad_grid(9).is_err());
    let original = Mesh::sphere(0);
    let mut document = Document::new(original);
    let before = document.to_json().unwrap();
    let cube = Mesh::cube_quad_grid(2).unwrap();
    assert_eq!(cube.positions.len(), 98);
    assert_eq!(cube.surface.as_ref().unwrap().faces.len(), 96);
    assert_eq!(cube.triangles.len(), 192);
    assert_eq!(cube.positions[0], Vec3::new(1.0, -1.0, -1.0));
    document.add_subtool(cube, "Cube").unwrap();
    let after = document.to_json().unwrap();
    assert_ne!(before, after);
    assert!(document.undo());
    assert_eq!(before, document.to_json().unwrap());
    assert!(document.redo());
    assert_eq!(after, document.to_json().unwrap());
}
