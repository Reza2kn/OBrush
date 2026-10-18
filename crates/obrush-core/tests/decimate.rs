use obrush_core::{DecimateSettings, Mesh, Vec3};
use std::collections::{BTreeMap, BTreeSet};

fn exact(mesh: &Mesh) -> String {
    serde_json::to_string(mesh).unwrap()
}

fn edge_counts(mesh: &Mesh) -> BTreeMap<[u32; 2], Vec<bool>> {
    let mut edges = BTreeMap::<_, Vec<_>>::new();
    let mut faces = BTreeSet::new();
    for triangle in &mesh.triangles {
        let mut face = *triangle;
        face.sort_unstable();
        assert!(faces.insert(face), "duplicate face");
        for [a, b] in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            edges.entry([a.min(b), a.max(b)]).or_default().push(a < b);
        }
    }
    for faces in edges.values() {
        assert!(faces.len() <= 2);
        if faces.len() == 2 {
            assert_ne!(faces[0], faces[1], "inconsistent edge winding");
        }
    }
    edges
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn sheet(size: usize) -> Mesh {
    let mut obj = String::new();
    for y in 0..=size {
        for x in 0..=size {
            obj.push_str(&format!(
                "v {} {} 0\n",
                x as f32 / size as f32,
                y as f32 / size as f32
            ));
        }
    }
    for y in 0..size {
        for x in 0..size {
            let a = y * (size + 1) + x + 1;
            let b = a + 1;
            let c = a + size + 1;
            let d = c + 1;
            obj.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }
    Mesh::from_obj(&obj).unwrap()
}

#[test]
fn sphere_reduces_deterministically_without_boundary_or_large_shape_loss() {
    let input = Mesh::sphere(4);
    let snapshot = exact(&input);
    let settings = DecimateSettings {
        max_error: 0.1,
        ..Default::default()
    };
    let (output, report) = input.decimated(&settings).unwrap();
    assert!(report.reached_target, "{report:?}");
    assert_eq!(report.output_triangles, input.triangles.len() / 2);
    assert!(report.output_vertices < input.positions.len());
    assert_eq!(report.boundary_vertices, 0);
    assert!(report.max_vertex_displacement_bound <= settings.max_error as f64);
    output.validate().unwrap();
    let edges = edge_counts(&output);
    assert!(edges.values().all(|incident| incident.len() == 2));
    assert_eq!(
        output.positions.len() as i64 - edges.len() as i64 + output.triangles.len() as i64,
        2
    );
    let relative_volume_change = (volume(&output) / volume(&input) - 1.0).abs();
    assert!(
        relative_volume_change < 0.01,
        "volume change {relative_volume_change}"
    );
    let radial_error = output
        .positions
        .iter()
        .map(|position| (position.as_dvec3().length() - 1.0).abs())
        .fold(0.0, f64::max);
    assert!(radial_error < 0.01, "radial error {radial_error}");
    assert_eq!(snapshot, exact(&input), "source changed");
    assert_eq!(
        exact(&output),
        exact(&input.decimated(&settings).unwrap().0)
    );
    for point in &input.positions {
        let nearest_representative = output
            .positions
            .iter()
            .map(|other| point.as_dvec3().distance(other.as_dvec3()))
            .fold(f64::INFINITY, f64::min);
        assert!(nearest_representative <= report.max_vertex_displacement_bound + 1.0e-12);
    }
}

#[test]
fn boundary_curve_and_open_sheet_winding_remain_exact() {
    let input = sheet(24);
    let original_boundary: BTreeSet<_> = edge_counts(&input)
        .iter()
        .filter(|(_, faces)| faces.len() == 1)
        .flat_map(|(edge, _)| {
            edge.map(|vertex| {
                input.positions[vertex as usize]
                    .to_array()
                    .map(f32::to_bits)
            })
        })
        .collect();
    let (output, report) = input
        .decimated(&DecimateSettings {
            max_error: 0.2,
            ..Default::default()
        })
        .unwrap();
    assert!(report.reached_target, "{report:?}");
    let output_edges = edge_counts(&output);
    let boundary: BTreeSet<_> = output_edges
        .iter()
        .filter(|(_, faces)| faces.len() == 1)
        .flat_map(|(edge, _)| {
            edge.map(|vertex| {
                output.positions[vertex as usize]
                    .to_array()
                    .map(f32::to_bits)
            })
        })
        .collect();
    assert_eq!(boundary, original_boundary);
    assert_eq!(report.boundary_vertices, 96);
    for triangle in &output.triangles {
        let [a, b, c] = triangle.map(|vertex| output.positions[vertex as usize]);
        assert!((b - a).cross(c - a).z > 0.0);
    }
    assert!(output.positions.iter().all(|position| position.z == 0.0));
}

#[test]
fn sharp_box_corners_and_edges_survive_reduction() {
    let input = Mesh::cube();
    let edge_positions: Vec<_> = input
        .positions
        .iter()
        .filter(|point| {
            point
                .to_array()
                .iter()
                .filter(|value| value.abs() == 1.0)
                .count()
                >= 2
        })
        .copied()
        .collect();
    let (output, report) = input
        .decimated(&DecimateSettings {
            max_error: 0.3,
            target_ratio: 0.4,
            ..Default::default()
        })
        .unwrap();
    assert!(report.reached_target, "{report:?}");
    for point in edge_positions {
        assert!(output.positions.contains(&point));
    }
    assert!((volume(&output) - volume(&input)).abs() < 1.0e-10);
    assert!(edge_counts(&output).values().all(|faces| faces.len() == 2));
}

#[test]
fn nonzero_masks_pin_geometry_and_attributes() {
    let mut input = Mesh::sphere(4);
    for (vertex, position) in input.positions.iter().enumerate() {
        input.colors[vertex] = [
            (position.x + 1.0) * 0.5,
            (position.y + 1.0) * 0.5,
            (position.z + 1.0) * 0.5,
        ];
        if position.z > 0.7 {
            input.masks[vertex] = if vertex % 2 == 0 { 1.0 } else { 0.25 };
        }
    }
    let protected: Vec<_> = input
        .positions
        .iter()
        .copied()
        .zip(input.masks.iter().copied())
        .zip(input.colors.iter().copied())
        .filter(|((_, mask), _)| *mask > 0.0)
        .collect();
    let (output, report) = input.decimated(&DecimateSettings::default()).unwrap();
    assert!(report.output_triangles < input.triangles.len() * 3 / 4);
    assert!(report.protected_vertices >= protected.len());
    for ((position, mask), color) in protected {
        let index = output
            .positions
            .iter()
            .position(|value| *value == position)
            .unwrap();
        assert_eq!(output.masks[index], mask);
        assert_eq!(output.colors[index], color);
    }
    output.validate().unwrap();
    assert!(output.colors.iter().any(|color| color != &output.colors[0]));
}

#[test]
fn impossible_target_and_work_limit_are_reported_without_corruption() {
    let input = Mesh::sphere(3);
    let (unchanged, report) = input
        .decimated(&DecimateSettings {
            max_error: 0.0,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(exact(&unchanged), exact(&input));
    assert!(!report.reached_target);
    assert_eq!(report.collapses, 0);
    let (partial, report) = input
        .decimated(&DecimateSettings {
            max_error: 0.3,
            max_candidate_pops: 1,
            ..Default::default()
        })
        .unwrap();
    assert!(!report.reached_target);
    assert_eq!(report.candidate_pops, 1);
    assert_eq!(report.termination, "Work limit reached");
    partial.validate().unwrap();
    edge_counts(&partial);
}

#[test]
fn invalid_geometry_settings_and_nonmanifold_vertices_leave_source_unchanged() {
    let input = Mesh::sphere(2);
    let before = exact(&input);
    for settings in [
        DecimateSettings {
            target_ratio: 0.0,
            ..Default::default()
        },
        DecimateSettings {
            target_ratio: f32::NAN,
            ..Default::default()
        },
        DecimateSettings {
            max_error: f32::INFINITY,
            ..Default::default()
        },
        DecimateSettings {
            max_candidate_pops: 0,
            ..Default::default()
        },
    ] {
        assert!(input.decimated(&settings).is_err());
        assert_eq!(exact(&input), before);
    }
    let mut invalid = input.clone();
    invalid.triangles.push(invalid.triangles[0]);
    assert!(invalid
        .decimated(&Default::default())
        .unwrap_err()
        .contains("duplicate"));
    let bow_tie =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv -1 0 0\nv 0 -1 0\nf 1 2 3\nf 1 4 5\n")
            .unwrap();
    assert!(bow_tie
        .decimated(&Default::default())
        .unwrap_err()
        .contains("fan"));
    let mut zero_area = input.clone();
    let face = zero_area.triangles[0];
    zero_area.positions[face[1] as usize] = zero_area.positions[face[0] as usize];
    assert!(zero_area
        .decimated(&Default::default())
        .unwrap_err()
        .contains("degenerate"));
    let mut nonfinite = input;
    nonfinite.positions[0] = Vec3::splat(f32::NAN);
    assert!(nonfinite.decimated(&Default::default()).is_err());
}

#[test]
fn inconsistent_winding_and_nonmanifold_edge_are_rejected() {
    let mut winding = Mesh::sphere(2);
    winding.triangles[0].swap(1, 2);
    assert!(winding
        .decimated(&Default::default())
        .unwrap_err()
        .contains("winding"));
    let edge =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 0 0 1\nf 1 2 3\nf 2 1 4\nf 1 2 5\n")
            .unwrap();
    assert!(edge
        .decimated(&Default::default())
        .unwrap_err()
        .contains("manifold"));
}

#[test]
fn cancelling_collapse_and_compaction_preserves_source_exactly() {
    let input = Mesh::sphere(4);
    let before = exact(&input);
    let error = input
        .decimated_with_progress(&DecimateSettings::default(), |progress| {
            matches!(progress.phase, obrush_core::DecimatePhase::Collapse)
                && progress.completed >= 256
        })
        .unwrap_err();
    assert_eq!(error, "Decimation cancelled");
    assert_eq!(exact(&input), before);
    let error = input
        .decimated_with_progress(&DecimateSettings::default(), |progress| {
            matches!(progress.phase, obrush_core::DecimatePhase::Compact)
        })
        .unwrap_err();
    assert_eq!(error, "Decimation cancelled");
    assert_eq!(exact(&input), before);
}

#[test]
fn tetrahedron_cannot_collapse_to_duplicate_faces() {
    let input = Mesh::from_obj(
        "v 1 1 1\nv -1 -1 1\nv -1 1 -1\nv 1 -1 -1\nf 1 3 2\nf 1 2 4\nf 1 4 3\nf 2 3 4\n",
    )
    .unwrap();
    let (output, report) = input
        .decimated(&DecimateSettings {
            max_error: 10.0,
            preserve_sharp_features: false,
            ..Default::default()
        })
        .unwrap();
    assert!(!report.reached_target);
    assert_eq!(report.collapses, 0);
    assert_eq!(exact(&input), exact(&output));
}

#[test]
fn ratio_one_preserves_exact_existing_normals_and_attributes() {
    let mut input = Mesh::sphere(2);
    input.normals[0] = Vec3::Y;
    input.colors[2] = [1.0, 0.0, 0.5];
    let (output, report) = input
        .decimated(&DecimateSettings {
            target_ratio: 1.0,
            ..Default::default()
        })
        .unwrap();
    assert!(report.reached_target);
    assert_eq!(report.collapses, 0);
    assert_eq!(exact(&input), exact(&output));
}

#[test]
fn operation_allocation_limit_is_checked_before_topology_work() {
    let mut input = Mesh::sphere(1);
    input.positions.resize(250_001, Vec3::ZERO);
    input.normals.resize(250_001, Vec3::ZERO);
    input.masks.resize(250_001, 0.0);
    input.colors.resize(250_001, [0.5; 3]);
    let error = input.decimated(&Default::default()).unwrap_err();
    assert!(error.contains("250000 vertices"));
    assert_eq!(input.positions.len(), 250_001);
}
