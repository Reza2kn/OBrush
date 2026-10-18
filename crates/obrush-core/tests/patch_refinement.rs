//! Independently authored planar and spherical fixtures for conforming local
//! density changes. These checks do not establish remeshing or reference parity.
use obrush_core::{Document, Mesh, Vec3, MAX_TRIANGLES, MAX_VERTICES};
use std::collections::{HashMap, HashSet};

fn plane() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n").unwrap()
}

fn assert_equal(first: &Mesh, second: &Mesh) {
    assert_eq!(first.positions, second.positions);
    assert_eq!(first.triangles, second.triangles);
    assert_eq!(first.normals, second.normals);
    assert_eq!(first.masks, second.masks);
    assert_eq!(first.colors, second.colors);
}

fn assert_planar_conforming(mesh: &Mesh, extent: (f64, f64), expected_area: f64) {
    mesh.validate().unwrap();
    let mut edges: HashMap<(u32, u32), Vec<bool>> = HashMap::new();
    let mut area = 0.0;
    for &[a, b, c] in &mesh.triangles {
        let [p, q, r] = [a, b, c].map(|vertex| mesh.positions[vertex as usize].as_dvec3());
        let cross = (q - p).cross(r - p);
        assert!(cross.z > 0.0);
        assert_eq!(cross.x, 0.0);
        assert_eq!(cross.y, 0.0);
        area += cross.z * 0.5;
        for (a, b) in [(a, b), (b, c), (c, a)] {
            edges.entry((a.min(b), a.max(b))).or_default().push(a < b);
        }
    }
    assert!((area - expected_area).abs() < 1e-10);
    for ((a, b), incident) in &edges {
        let p = mesh.positions[*a as usize].as_dvec3();
        let q = mesh.positions[*b as usize].as_dvec3();
        match incident.as_slice() {
            [first, second] => assert_ne!(first, second),
            [_] => assert!(
                (p.x == extent.0 && q.x == extent.0)
                    || (p.x == extent.1 && q.x == extent.1)
                    || (p.y == extent.0 && q.y == extent.0)
                    || (p.y == extent.1 && q.y == extent.1),
                "an internal open edge indicates a crack"
            ),
            _ => panic!("non-manifold refined edge"),
        }
        let direction = q - p;
        for (vertex, position) in mesh.positions.iter().enumerate() {
            if vertex == *a as usize || vertex == *b as usize {
                continue;
            }
            let offset = position.as_dvec3() - p;
            let parameter = offset.dot(direction) / direction.length_squared();
            assert!(
                !(parameter > 0.0
                    && parameter < 1.0
                    && offset.cross(direction).length_squared() == 0.0),
                "vertex {vertex} lies inside an unsplit edge: a T junction"
            );
        }
    }
}

#[test]
fn full_patch_preserves_surface_boundary_and_expected_adjacency_counts() {
    let mut mesh = plane();
    let original = mesh.clone();
    let result = mesh
        .refine_brush_patch(Vec3::new(0.5, 0.5, 0.0), 3.0, false)
        .unwrap();
    assert_eq!(result.vertices_added, 21);
    assert_eq!(result.triangles_added, 30);
    assert!(!result.limited);
    assert_eq!(mesh.positions.len(), 25);
    assert_eq!(mesh.triangles.len(), 32);
    assert_eq!(&mesh.positions[..4], original.positions.as_slice());
    assert!(mesh
        .normals
        .iter()
        .all(|normal| normal.distance(Vec3::Z) < 1e-6));
    assert_planar_conforming(&mesh, (0.0, 1.0), 1.0);
}

#[test]
fn partial_patch_splits_incident_faces_across_its_boundary_without_cracks() {
    let mut mesh = Mesh::from_obj(
        "v -1 -1 0\nv 0 -1 0\nv 1 -1 0\nv -1 0 0\nv 0 0 0\nv 1 0 0\n\
         v -1 1 0\nv 0 1 0\nv 1 1 0\nf 1 2 5\nf 1 5 4\nf 2 3 6\nf 2 6 5\n\
         f 4 5 8\nf 4 8 7\nf 5 6 9\nf 5 9 8\n",
    )
    .unwrap();
    let original = mesh.clone();
    let center = Vec3::new(0.2, 0.1, 0.0);
    let result = mesh.refine_brush_patch(center, 0.35, false).unwrap();
    assert!(result.vertices_added > 0 && result.vertices_added <= 1024);
    assert_eq!(
        &mesh.positions[..original.positions.len()],
        original.positions.as_slice()
    );
    assert!(mesh.positions[original.positions.len()..]
        .iter()
        .any(|position| position.distance(center) > 0.35));
    assert_planar_conforming(&mesh, (-1.0, 1.0), 4.0);
}

#[test]
fn midpoint_attributes_follow_authored_linear_fields_and_history_roundtrips() {
    let mut mesh = plane();
    for (vertex, position) in mesh.positions.iter().enumerate() {
        mesh.masks[vertex] = (position.x + position.y) * 0.5;
        mesh.colors[vertex] = [position.x, position.y, (position.x + position.y) * 0.5];
    }
    let original = mesh.clone();
    let mut document = Document::new(mesh);
    document.begin_stroke();
    document
        .active_mesh_mut()
        .unwrap()
        .refine_brush_patch(Vec3::new(0.5, 0.5, 0.0), 3.0, false)
        .unwrap();
    document.end_stroke();
    for (vertex, position) in document.active_mesh().unwrap().positions.iter().enumerate() {
        assert_eq!(
            document.active_mesh().unwrap().masks[vertex],
            (position.x + position.y) * 0.5
        );
        assert_eq!(
            document.active_mesh().unwrap().colors[vertex],
            [position.x, position.y, (position.x + position.y) * 0.5]
        );
    }
    let refined = document.active_mesh().unwrap().clone();
    let loaded = Document::from_json(&document.to_json().unwrap()).unwrap();
    assert_equal(loaded.active_mesh().unwrap(), &refined);
    assert!(document.undo());
    assert_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_equal(document.active_mesh().unwrap(), &refined);
}

#[test]
fn a_fully_masked_region_and_a_distant_brush_are_exact_no_ops() {
    let mut masked = plane();
    masked.masks.fill(1.0);
    let original = masked.clone();
    assert_eq!(
        masked.refine_brush_patch(Vec3::ZERO, 3.0, false).unwrap(),
        Default::default()
    );
    assert_equal(&masked, &original);
    let mut distant = plane();
    let original = distant.clone();
    assert_eq!(
        distant
            .refine_brush_patch(Vec3::new(100.0, 100.0, 100.0), 0.1, true)
            .unwrap(),
        Default::default()
    );
    assert_equal(&distant, &original);
}

#[test]
fn a_protected_shared_edge_refines_conformingly_without_generating_boundary_wedges() {
    let mut mesh = plane();
    mesh.masks = vec![1.0, 0.0, 1.0, 1.0];
    let original = mesh.snapshot();
    let mut added = 0;
    for _ in 0..8 {
        added += mesh
            .refine_brush_patch(Vec3::new(0.5, 0.5, 0.0), 0.13, false)
            .unwrap()
            .vertices_added;
        for &triangle in &mesh.triangles {
            let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
            let quality = (b - a).cross(c - a).length()
                / a.distance_squared(b)
                    .max(b.distance_squared(c))
                    .max(c.distance_squared(a));
            assert!(
                quality >= 0.05 * (1.0 - 1e-5),
                "a repeated transition created a wedge: {quality}"
            );
        }
    }
    assert!(added > 0);
    assert_eq!(&mesh.positions[..4], original.positions);
    assert_eq!(&mesh.masks[..4], original.masks);
    assert!(mesh
        .positions
        .iter()
        .zip(&mesh.masks)
        .any(|(position, &mask)| position.x == 0.5 && position.y == 0.5 && mask == 1.0));
    assert_planar_conforming(&mesh, (0.0, 1.0), 1.0);
}

#[test]
fn a_skinny_transition_is_limited_instead_of_making_the_authored_face_thinner() {
    let mut mesh = Mesh::from_obj("v -1 0 0\nv 1 0 0\nv 0 0.04 0\nf 1 2 3\n").unwrap();
    let original = mesh.snapshot();
    // Only the first long side crosses the brush. Splitting it would halve the
    // quality of an already thin but nonzero-area authored triangle.
    let result = mesh
        .refine_brush_patch(Vec3::new(-0.5, 0.02, 0.0), 0.005, false)
        .unwrap();
    assert_eq!(result.vertices_added, 0);
    assert!(result.limited);
    assert_equal(&mesh, &original);
}

fn point_key(position: Vec3) -> [u32; 3] {
    position
        .to_array()
        .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
}

#[test]
fn work_budget_is_deterministic_and_keeps_exact_mirrored_vertex_groups() {
    let mut first = Mesh::sphere(5);
    let original = first.clone();
    let mut second = original.clone();
    let center = Vec3::new(0.3, 0.0, (1.0_f32 - 0.09).sqrt());
    let first_result = first.refine_brush_patch(center, 0.23, true).unwrap();
    let second_result = second.refine_brush_patch(center, 0.23, true).unwrap();
    assert_eq!(first_result, second_result);
    assert_equal(&first, &second);
    assert!(first_result.limited);
    assert!(first_result.vertices_added > 0 && first_result.vertices_added <= 1024);
    assert_eq!(
        &first.positions[..original.positions.len()],
        original.positions.as_slice()
    );
    let positions: HashSet<_> = first.positions.iter().copied().map(point_key).collect();
    for &position in &first.positions {
        let reflected = Vec3::new(-position.x, position.y, position.z);
        assert!(
            positions.contains(&point_key(reflected)),
            "missing mirror of {position:?}; nearest={:?}",
            first
                .positions
                .iter()
                .min_by(|a, b| a
                    .distance_squared(reflected)
                    .total_cmp(&b.distance_squared(reflected)))
                .unwrap()
        );
    }
    first.validate().unwrap();
}

#[test]
fn a_self_mirrored_triangle_keeps_symmetric_samples_at_a_partial_patch_boundary() {
    let mut mesh = Mesh::from_obj("v -1 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    let original = mesh.clone();
    let result = mesh.refine_brush_patch(Vec3::Y, 0.7, true).unwrap();
    assert!(result.vertices_added > 0 && result.vertices_added <= 1024);
    assert_eq!(&mesh.positions[..3], original.positions.as_slice());
    let positions: HashSet<_> = mesh.positions.iter().copied().map(point_key).collect();
    for &position in &mesh.positions {
        assert!(positions.contains(&point_key(Vec3::new(-position.x, position.y, position.z))));
    }
    let area: f64 = mesh
        .triangles
        .iter()
        .map(|&triangle| {
            let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
            let area = (b - a).cross(c - a).z * 0.5;
            assert!(area > 0.0);
            area
        })
        .sum();
    assert!((area - 1.0).abs() < 1e-8);
}

#[test]
fn tiny_and_large_coordinate_patches_use_scale_independent_distance_checks() {
    for (origin, size) in [(0.0, 1e-10), (999_999.0, 1.0)] {
        let mut mesh = plane();
        for position in &mut mesh.positions {
            position.x = origin + position.x * size;
            position.y = origin + position.y * size;
        }
        mesh.recompute_normals();
        let original = mesh.clone();
        let result = mesh
            .refine_brush_patch(
                Vec3::new(origin + size * 0.5, origin + size * 0.5, 0.0),
                size * 3.0,
                false,
            )
            .unwrap();
        assert_eq!(result.vertices_added, 21);
        assert_eq!(result.triangles_added, 30);
        assert_eq!(&mesh.positions[..4], original.positions.as_slice());
        mesh.validate().unwrap();
        assert!(mesh
            .normals
            .iter()
            .all(|normal| normal.distance(Vec3::Z) < 1e-6));
    }
}

#[test]
fn malformed_selected_topology_and_unrepresentable_midpoints_are_atomic_errors() {
    let mut invalid_index = plane();
    invalid_index.triangles[0][0] = u32::MAX;
    let mut mismatched_attributes = plane();
    mismatched_attributes.colors.pop();
    let mut duplicate = plane();
    duplicate.triangles.push(duplicate.triangles[0]);
    let non_manifold = Mesh::from_obj(
        "v -1 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 0 0 1\nf 1 2 3\nf 2 1 4\nf 1 2 5\n",
    )
    .unwrap();
    let degenerate = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 2 0 0\nf 1 2 3\n").unwrap();
    for mut mesh in [
        invalid_index,
        mismatched_attributes,
        duplicate,
        non_manifold,
        degenerate,
    ] {
        let original = mesh.clone();
        assert!(mesh.refine_brush_patch(Vec3::ZERO, 3.0, true).is_err());
        assert_equal(&mesh, &original);
    }
    let mut precision =
        Mesh::from_obj("v 999999.9375 0 0\nv 1000000 0 0\nv 999999.9375 0.0625 0\nf 1 2 3\n")
            .unwrap();
    let original = precision.clone();
    assert!(precision
        .refine_brush_patch(Vec3::new(1_000_000.0, 0.0, 0.0), 0.1, false)
        .is_err());
    assert_equal(&precision, &original);
}

#[test]
fn bad_brush_parameters_leave_the_document_unchanged() {
    for (center, radius) in [
        (Vec3::ZERO, 0.0),
        (Vec3::ZERO, -1.0),
        (Vec3::ZERO, f32::INFINITY),
        (Vec3::ZERO, f32::NAN),
        (Vec3::new(f32::NAN, 0.0, 0.0), 1.0),
    ] {
        let mut mesh = plane();
        let original = mesh.clone();
        assert!(mesh.refine_brush_patch(center, radius, false).is_err());
        assert_equal(&mesh, &original);
    }
}

#[test]
fn output_vertex_limit_is_preflighted_without_partial_refinement() {
    let mut mesh = plane();
    mesh.positions.resize(MAX_VERTICES, Vec3::ZERO);
    mesh.normals.resize(MAX_VERTICES, Vec3::ZERO);
    mesh.masks.resize(MAX_VERTICES, 1.0);
    mesh.colors.resize(MAX_VERTICES, [0.5; 3]);
    let positions_pointer = mesh.positions.as_ptr();
    let triangles = mesh.triangles.clone();
    let message = mesh.refine_brush_patch(Vec3::ZERO, 3.0, false).unwrap_err();
    assert!(message.contains("limit"));
    assert_eq!(mesh.positions.as_ptr(), positions_pointer);
    assert_eq!(mesh.positions.len(), MAX_VERTICES);
    assert_eq!(mesh.triangles, triangles);
}

#[test]
fn output_triangle_limit_is_preflighted_before_any_replacement() {
    let mut mesh = Mesh::from_obj(
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 100 0 0\nv 101 0 0\nv 100 1 0\nf 1 2 3\nf 4 5 6\n",
    )
    .unwrap();
    // Structurally valid distant triangles do not enter the selected patch;
    // this intentionally tests the global allocation ceiling independently.
    mesh.triangles = vec![[3, 4, 5]; MAX_TRIANGLES - 1];
    mesh.triangles.push([0, 1, 2]);
    let positions_pointer = mesh.positions.as_ptr();
    let triangles_pointer = mesh.triangles.as_ptr();
    let message = mesh.refine_brush_patch(Vec3::ZERO, 3.0, false).unwrap_err();
    assert!(message.contains("limit"));
    assert_eq!(mesh.positions.as_ptr(), positions_pointer);
    assert_eq!(mesh.triangles.as_ptr(), triangles_pointer);
    assert_eq!(mesh.triangles.len(), MAX_TRIANGLES);
    assert_eq!(mesh.positions.len(), 6);
}

#[test]
fn a_tangent_long_edge_refines_all_incident_faces_with_no_vertices_in_the_brush() {
    let mut mesh =
        Mesh::from_obj("v -2 1 0\nv 2 1 0\nv 0 3 0\nv 0 -3 0\nf 1 2 3\nf 2 1 4\n").unwrap();
    assert!(mesh
        .positions
        .iter()
        .all(|position| position.length() > 1.0));
    let original = mesh.clone();
    let result = mesh.refine_brush_patch(Vec3::ZERO, 1.0, false).unwrap();
    assert!(result.vertices_added > 0);
    assert_eq!(&mesh.positions[..4], original.positions.as_slice());
    assert!(mesh.positions.contains(&Vec3::new(0.0, 1.0, 0.0)));
    let mut edges = HashSet::new();
    for &[a, b, c] in &mesh.triangles {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    assert!(!edges.contains(&(0, 1)));
    for (a, b) in edges {
        let p = mesh.positions[a as usize].as_dvec3();
        let direction = mesh.positions[b as usize].as_dvec3() - p;
        for (vertex, position) in mesh.positions.iter().enumerate() {
            if vertex == a as usize || vertex == b as usize {
                continue;
            }
            let offset = position.as_dvec3() - p;
            let parameter = offset.dot(direction) / direction.length_squared();
            assert!(
                !(parameter > 0.0
                    && parameter < 1.0
                    && offset.cross(direction).length_squared() == 0.0),
                "tangent patch left a T junction"
            );
        }
    }
    mesh.validate().unwrap();
}

#[test]
fn public_refinement_rebuilds_stale_normals_outside_the_refined_patch() {
    let mut mesh = Mesh::from_obj(
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 100 0 0\nv 101 0 0\nv 100 1 0\nf 1 2 3\nf 4 5 6\n",
    )
    .unwrap();
    mesh.normals.fill(Vec3::X);
    let result = mesh
        .refine_brush_patch(Vec3::new(0.25, 0.25, 0.0), 0.4, false)
        .unwrap();
    assert!(result.vertices_added > 0);
    assert!(mesh
        .normals
        .iter()
        .all(|normal| normal.distance(Vec3::Z) < 1e-6));
}
