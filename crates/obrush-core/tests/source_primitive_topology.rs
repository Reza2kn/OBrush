//! Independent topological contracts for the default Source-profile solids.
//! These check actual adjacency, face winding, serialization and history; the
//! pinned browser comparison is recorded separately in docs.
use obrush_core::{Document, Mesh, Vec3, TRIANGLE_CORNER};
use std::collections::HashMap;

fn closed_oriented_edges(mesh: &Mesh, expected_euler: i64) {
    let mut edges = HashMap::<(u32, u32), (u8, i8)>::new();
    for &[a, b, c] in &mesh.triangles {
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let edge = edges.entry((from.min(to), from.max(to))).or_default();
            edge.0 += 1;
            edge.1 += if from < to { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|edge| *edge == (2, 0)));
    assert_eq!(
        mesh.positions.len() as i64 - edges.len() as i64 + mesh.triangles.len() as i64,
        expected_euler
    );
}

fn finite_unit_normals(mesh: &Mesh) {
    assert!(mesh
        .normals
        .iter()
        .all(|normal| { normal.is_finite() && (normal.length() - 1.0).abs() < 2e-4 }));
}

#[test]
fn source_default_cylinder_has_mixed_original_faces_and_closed_genus_zero() {
    let mesh = Mesh::source_default_cylinder().unwrap();
    mesh.validate().unwrap();
    assert_eq!(mesh.positions.len(), 66_562);
    assert_eq!(mesh.face_count(), 67_584);
    assert_eq!(mesh.triangles.len(), 133_120);
    let surface = mesh.surface.as_ref().unwrap();
    let quads = surface
        .faces
        .iter()
        .filter(|face| face[3] != TRIANGLE_CORNER)
        .count();
    assert_eq!(quads, 65_536);
    assert_eq!(surface.faces.len() - quads, 2_048);
    closed_oriented_edges(&mesh, 2);
    finite_unit_normals(&mesh);
    let bounds = mesh.positions.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(min, max), &point| (min.min(point), max.max(point)),
    );
    assert!(bounds.0.x < -0.45 && bounds.1.x > 0.45);
    assert!(bounds.0.z < -0.45 && bounds.1.z > 0.45);
    assert!(bounds.0.x >= -0.5 && bounds.1.x <= 0.5);
    assert!(bounds.0.z >= -0.5 && bounds.1.z <= 0.5);
    assert_eq!((bounds.0.y, bounds.1.y), (-1.0, 1.0));
}

#[test]
fn source_default_torus_keeps_quads_and_genus_one_through_project_history() {
    let mesh = Mesh::source_default_torus().unwrap();
    mesh.validate().unwrap();
    assert_eq!(mesh.positions.len(), 65_536);
    assert_eq!(mesh.face_count(), 65_536);
    assert_eq!(mesh.triangles.len(), 131_072);
    assert!(mesh
        .surface
        .as_ref()
        .unwrap()
        .faces
        .iter()
        .all(|face| face[3] != TRIANGLE_CORNER));
    closed_oriented_edges(&mesh, 0);
    finite_unit_normals(&mesh);

    let mut document = Document::new(Mesh::sphere(0));
    let before = document.to_json().unwrap();
    document.add_subtool(mesh, "Torus").unwrap();
    let after = document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn ordinary_primitives_keep_prior_triangle_only_topology() {
    let cylinder = Mesh::cylinder(&Default::default()).unwrap();
    let torus = Mesh::torus(&Default::default()).unwrap();
    assert!(cylinder.surface.is_none());
    assert!(torus.surface.is_none());
    assert_eq!(
        (cylinder.positions.len(), cylinder.triangles.len()),
        (4_162, 8_320)
    );
    assert_eq!(
        (torus.positions.len(), torus.triangles.len()),
        (4_096, 8_192)
    );
}
