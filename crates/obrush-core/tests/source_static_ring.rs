use obrush_core::{
    source_ring::SourceStaticRingMap, Document, Mesh, StaticSubdivisionSettings, Vec3,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Expected {
    parent_face_visit: Vec<u32>,
    first_neighbor: Vec<u32>,
    coarse_vertex: usize,
    edited_fine: Vec<[f32; 3]>,
}

#[derive(Deserialize)]
struct ChainedExpected {
    coarse_vertex: usize,
    second_first_neighbor: Vec<u32>,
    edited_second_positions: Vec<[f32; 3]>,
}

#[test]
fn uv_seamed_reference_first_neighbors_match_from_parent_alone() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/source-static-ring-reference.json")).unwrap();
    for (name, source) in [
        (
            "independent-seamed-tetra",
            include_str!("fixtures/independent-seamed-tetra.obj"),
        ),
        (
            "independent-seamed-quad-cube",
            include_str!("fixtures/independent-seamed-quad-cube.obj"),
        ),
        (
            "independent-seamed-pyramid",
            include_str!("fixtures/independent-seamed-pyramid.obj"),
        ),
    ] {
        let parent = Mesh::import_obj(source.as_bytes()).unwrap().mesh;
        let child = parent
            .subdivided_static(&StaticSubdivisionSettings::default())
            .unwrap()
            .mesh;
        let observed = SourceStaticRingMap::build(&parent, &child).unwrap();
        let expected: Expected = serde_json::from_value(expected[name].clone()).unwrap();
        assert_eq!(
            observed.parent_face_visit(),
            expected.parent_face_visit,
            "{name}"
        );
        assert_eq!(observed.first_neighbor(), expected.first_neighbor, "{name}");

        let mut document = Document::new(parent);
        document
            .add_source_static_subdivision_level(&StaticSubdivisionSettings::default())
            .unwrap();
        let old_child = document.require_active_mesh().unwrap().snapshot();
        document.select_level(0).unwrap();
        let old_parent = document.require_active_mesh().unwrap().snapshot();
        let old_prediction = old_parent
            .subdivided_static(&StaticSubdivisionSettings::default())
            .unwrap()
            .mesh;
        let edited = document.require_active_mesh_mut().unwrap();
        edited.positions[expected.coarse_vertex] += Vec3::new(0.125, -0.09375, 0.0625);
        edited.recompute_normals();
        let new_prediction = edited
            .subdivided_static(&StaticSubdivisionSettings::default())
            .unwrap()
            .mesh;
        let moved = observed
            .transport_positions(&old_child, &old_prediction, &new_prediction)
            .unwrap();
        let max_error = moved
            .iter()
            .zip(expected.edited_fine)
            .flat_map(|(observed, expected)| {
                observed
                    .to_array()
                    .into_iter()
                    .zip(expected)
                    .map(|(observed, expected)| (observed - expected).abs())
            })
            .fold(0.0f32, f32::max);
        println!("{name}: max transported component error {max_error:.9e}");
        assert!(max_error < 1e-6, "{name}: max error {max_error}");
    }
}

#[test]
fn map_rejects_a_child_from_another_parent() {
    let tetra = Mesh::import_obj(include_bytes!("fixtures/independent-seamed-tetra.obj"))
        .unwrap()
        .mesh;
    let cube = Mesh::import_obj(include_bytes!("fixtures/independent-seamed-quad-cube.obj"))
        .unwrap()
        .mesh;
    let child = cube
        .subdivided_static(&StaticSubdivisionSettings::default())
        .unwrap()
        .mesh;
    assert!(SourceStaticRingMap::build(&tetra, &child).is_err());
}

#[test]
fn chained_source_faces_and_edited_second_level_match_pinned_reference() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/source-two-level-ring-reference.json"
    ))
    .unwrap();
    for (name, source) in [
        (
            "independent-seamed-tetra",
            include_bytes!("fixtures/independent-seamed-tetra.obj").as_slice(),
        ),
        (
            "independent-seamed-quad-cube",
            include_bytes!("fixtures/independent-seamed-quad-cube.obj").as_slice(),
        ),
        (
            "independent-seamed-pyramid",
            include_bytes!("fixtures/independent-seamed-pyramid.obj").as_slice(),
        ),
    ] {
        let expected: ChainedExpected =
            serde_json::from_value(expected["cases"][name].clone()).unwrap();
        let parent = Mesh::import_obj(source).unwrap().mesh;
        let settings = StaticSubdivisionSettings::default();
        let first = parent.subdivided_static(&settings).unwrap().mesh;
        let second = first.subdivided_static(&settings).unwrap().mesh;
        let first_ring = SourceStaticRingMap::build(&parent, &first).unwrap();
        let second_ring = SourceStaticRingMap::build_after(&first, &second, &first_ring).unwrap();
        assert_eq!(
            second_ring.first_neighbor(),
            expected.second_first_neighbor,
            "{name}"
        );

        let mut document = Document::new(parent);
        document
            .add_source_static_subdivision_level(&settings)
            .unwrap();
        document
            .add_source_static_subdivision_level(&settings)
            .unwrap();
        document.select_level(0).unwrap();
        let json = document.to_json().unwrap();
        let mut document = Document::from_json(&json).unwrap();
        document.require_active_mesh_mut().unwrap().positions[expected.coarse_vertex] +=
            Vec3::new(0.125, -0.09375, 0.0625);
        document
            .require_active_mesh_mut()
            .unwrap()
            .recompute_normals();
        document.select_level(2).unwrap();
        let actual = &document.require_active_mesh().unwrap().positions;
        assert_eq!(actual.len(), expected.edited_second_positions.len());
        let max_error = actual
            .iter()
            .zip(expected.edited_second_positions)
            .flat_map(|(actual, expected)| {
                actual
                    .to_array()
                    .into_iter()
                    .zip(expected)
                    .map(|(actual, expected)| (actual - expected).abs())
            })
            .fold(0.0f32, f32::max);
        println!("{name}: second-level max component error {max_error:.9e}");
        assert!(max_error < 1e-6, "{name}: max error {max_error}");
    }
}

#[test]
fn deleting_lower_level_keeps_chained_source_order_after_reload() {
    let source = include_bytes!("fixtures/independent-seamed-pyramid.obj");
    let settings = StaticSubdivisionSettings::default();
    let mut document = Document::new(Mesh::import_obj(source).unwrap().mesh);
    document
        .add_source_static_subdivision_level(&settings)
        .unwrap();
    document
        .add_source_static_subdivision_level(&settings)
        .unwrap();
    document.select_level(1).unwrap();
    document.delete_lower_subdivision_levels().unwrap();
    let encoded = document.to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        value["multiresolution"]["source_root_order"]
            .as_array()
            .unwrap()
            .len(),
        document
            .require_active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .faces
            .len()
    );
    let mut restored = Document::from_json(&encoded).unwrap();
    let mut malformed: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    malformed["multiresolution"]["source_root_order"][0][0] = serde_json::Value::from(u32::MAX);
    assert!(Document::from_json(&malformed.to_string()).is_err());
    for candidate in [&mut document, &mut restored] {
        candidate.require_active_mesh_mut().unwrap().positions[0] +=
            Vec3::new(0.125, -0.09375, 0.0625);
        candidate
            .require_active_mesh_mut()
            .unwrap()
            .recompute_normals();
        candidate.select_level(1).unwrap();
    }
    assert_eq!(
        document.require_active_mesh().unwrap().positions,
        restored.require_active_mesh().unwrap().positions
    );
}

#[test]
fn ring_map_covers_non_uv_sphere_and_larger_seamed_sphere() {
    let plain = Mesh::sphere(2);
    let seamed = Mesh::import_obj(include_bytes!("fixtures/independent-seamed-sphere.obj"))
        .unwrap()
        .mesh;
    for parent in [&plain, &seamed] {
        let child = parent
            .subdivided_static(&StaticSubdivisionSettings::default())
            .unwrap()
            .mesh;
        let ring = SourceStaticRingMap::build(parent, &child).unwrap();
        assert_eq!(ring.first_neighbor().len(), child.positions.len());
        assert_eq!(ring.parent_face_visit().len(), parent.triangles.len());
        assert!(ring
            .first_neighbor()
            .iter()
            .enumerate()
            .all(|(vertex, &neighbor)| neighbor as usize != vertex
                && (neighbor as usize) < child.positions.len()));
    }
}
