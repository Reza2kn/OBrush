//! Independent analytic mask-extraction contracts; no reference behavior is
//! inferred. Unit-scale gates were frozen before the first executable run.
use obrush_core::{Document, MaskExtractReport, MaskExtractSettings, Mesh, Vec3};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const POINT_GATE: f64 = 2e-6;
const COLOR_GATE: f64 = 1e-6;
const VOLUME_GATE: f64 = 2e-6;
type DirectedFaceUse = (u32, u32, usize);
type EdgeUses = BTreeMap<(u32, u32), Vec<DirectedFaceUse>>;

fn fixture(points: &[[f32; 3]], faces: &[[u32; 3]], masks: &[f32]) -> Mesh {
    let mut obj = String::new();
    for point in points {
        obj.push_str(&format!("v {} {} {}\n", point[0], point[1], point[2]));
    }
    for face in faces {
        obj.push_str(&format!(
            "f {} {} {}\n",
            face[0] + 1,
            face[1] + 1,
            face[2] + 1
        ));
    }
    let mut mesh = Mesh::from_obj(&obj).unwrap();
    mesh.masks.clone_from_slice(masks);
    mesh
}

fn triangle(masks: [f32; 3]) -> Mesh {
    let mut mesh = fixture(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &masks,
    );
    mesh.colors = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    mesh
}

fn square(masks: [f32; 4]) -> Mesh {
    let mut mesh = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
        &masks,
    );
    mesh.colors = mesh.positions.iter().map(|p| [p.x, p.y, 0.25]).collect();
    mesh
}

fn bits(mesh: &Mesh) -> Value {
    json!({
        "positions":mesh.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":mesh.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":mesh.masks.iter().map(|p|p.to_bits()).collect::<Vec<_>>(),
        "colors":mesh.colors.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),
        "triangles":mesh.triangles,
    })
}

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn payload(document: &Document) -> Value {
    let project: Value = serde_json::from_str(&state(document)).unwrap();
    let mut result = serde_json::Map::new();
    for key in ["mesh", "multiresolution", "layers", "morph_target"] {
        if let Some(value) = project.get(key) {
            result.insert(key.into(), value.clone());
        }
    }
    Value::Object(result)
}

fn metric(case: &str, family: &str, error: f64, gate: f64) {
    eprintln!(
        "mask_extract_contract_metric: {}",
        json!({"case":case,"family":family,"max_absolute_error":error,"gate":gate})
    );
    assert!(error <= gate, "{case} {family}: {error} exceeds {gate}");
}

fn maximum(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

fn triple(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    a[0] * (b[1] * c[2] - b[2] * c[1])
        + a[1] * (b[2] * c[0] - b[0] * c[2])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

fn volume(mesh: &Mesh) -> f64 {
    let origin = mesh.positions[0].to_array().map(f64::from);
    mesh.triangles
        .iter()
        .map(|face| {
            let points = face.map(|index| {
                let point = mesh.positions[index as usize].to_array().map(f64::from);
                std::array::from_fn(|i| point[i] - origin[i])
            });
            triple(points[0], points[1], points[2]) / 6.0
        })
        .sum()
}

fn closed_shell(mesh: &Mesh) -> usize {
    mesh.validate().unwrap();
    assert!(mesh.masks.iter().all(|m| m.to_bits() == 0.0_f32.to_bits()));
    let mut edges = EdgeUses::new();
    let mut faces = BTreeSet::new();
    let mut incident = vec![Vec::new(); mesh.positions.len()];
    let mut neighbors = vec![BTreeSet::new(); mesh.positions.len()];
    for (index, &face) in mesh.triangles.iter().enumerate() {
        let mut sorted = face;
        sorted.sort_unstable();
        assert!(faces.insert(sorted), "duplicate unordered triangle");
        let [a, b, c] = face.map(|v| mesh.positions[v as usize].as_dvec3());
        assert!(
            (b - a).cross(c - a).length_squared() > 0.0,
            "collapsed face"
        );
        for vertex in face {
            incident[vertex as usize].push(index);
        }
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((a, b, index));
            neighbors[a as usize].insert(b as usize);
            neighbors[b as usize].insert(a as usize);
        }
    }
    for uses in edges.values() {
        assert_eq!(uses.len(), 2, "shell edge must have exactly two uses");
        assert_eq!(
            (uses[0].0, uses[0].1),
            (uses[1].1, uses[1].0),
            "edge winding must oppose"
        );
    }
    for (vertex, local) in incident.iter().enumerate() {
        assert!(!local.is_empty(), "orphan output point");
        let mut seen = BTreeSet::new();
        let mut pending = vec![local[0]];
        while let Some(face) = pending.pop() {
            if !seen.insert(face) {
                continue;
            }
            for (&(a, b), uses) in &edges {
                if a as usize != vertex && b as usize != vertex {
                    continue;
                }
                if uses.iter().any(|entry| entry.2 == face) {
                    pending.extend(uses.iter().map(|entry| entry.2));
                }
            }
        }
        assert_eq!(seen.len(), local.len(), "vertex face fan must be connected");
    }
    let mut seen = BTreeSet::new();
    let mut components = 0;
    for start in 0..mesh.positions.len() {
        if seen.contains(&start) {
            continue;
        }
        components += 1;
        let mut pending = vec![start];
        while let Some(vertex) = pending.pop() {
            if seen.insert(vertex) {
                pending.extend(neighbors[vertex].iter().copied());
            }
        }
    }
    components
}

fn extract(source: &Mesh, settings: MaskExtractSettings) -> (Mesh, MaskExtractReport) {
    let before = bits(source);
    let result = source.extract_mask(&settings).unwrap();
    assert_eq!(
        bits(source),
        before,
        "Mesh extraction must leave all source channels exact"
    );
    assert_eq!(result.1.input_vertices, source.positions.len());
    assert_eq!(result.1.input_triangles, source.triangles.len());
    assert_eq!(result.1.output_vertices, result.0.positions.len());
    assert_eq!(result.1.output_triangles, result.0.triangles.len());
    assert_eq!(result.1.threshold.to_bits(), settings.threshold.to_bits());
    assert_eq!(result.1.thickness.to_bits(), settings.thickness.to_bits());
    result
}

fn planar_points(
    mesh: &Mesh,
    expected: &[[f64; 2]],
    thickness: f32,
    color: impl Fn(f64, f64) -> [f64; 3],
    case: &str,
) {
    assert_eq!(mesh.positions.len(), expected.len() * 2);
    let mut position_error = 0.0_f64;
    let mut color_error = 0.0_f64;
    let mut matched = BTreeSet::new();
    for (index, point) in mesh.positions.iter().enumerate() {
        let actual = point.to_array().map(f64::from);
        let best = expected
            .iter()
            .enumerate()
            .flat_map(|(id, xy)| {
                [0.0, f64::from(thickness)]
                    .into_iter()
                    .enumerate()
                    .map(move |(skin, z)| (id * 2 + skin, [xy[0], xy[1], z]))
            })
            .min_by(|(_, a), (_, b)| maximum(actual, *a).total_cmp(&maximum(actual, *b)))
            .unwrap();
        position_error = position_error.max(maximum(actual, best.1));
        assert!(matched.insert(best.0), "duplicate cap/cut/endpoint point");
        color_error = color_error.max(maximum(
            mesh.colors[index].map(f64::from),
            color(best.1[0], best.1[1]),
        ));
    }
    assert_eq!(matched.len(), expected.len() * 2);
    metric(case, "positions", position_error, POINT_GATE);
    metric(case, "colors", color_error, COLOR_GATE);
}

fn assert_report(
    report: &MaskExtractReport,
    patch: (usize, usize, usize),
    boundary: (usize, usize),
) {
    assert_eq!(
        (
            report.patch_vertices,
            report.patch_triangles,
            report.patch_components
        ),
        patch
    );
    assert_eq!((report.boundary_edges, report.boundary_loops), boundary);
}

#[test]
fn defaults_and_report_preserve_exact_requested_parameters() {
    let settings = MaskExtractSettings::default();
    assert_eq!(settings.threshold, 0.5);
    assert_eq!(settings.thickness, 0.02);
    let (mesh, report) = extract(&triangle([1.0; 3]), settings);
    assert_report(&report, (3, 1, 1), (3, 1));
    assert_eq!(report.selected_source_triangles, 1);
    assert_eq!((mesh.positions.len(), mesh.triangles.len()), (6, 8));
    assert_eq!(closed_shell(&mesh), 1);
    let encoded = serde_json::to_string(&report).unwrap();
    let decoded: MaskExtractReport = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
}

#[test]
fn one_selected_corner_clips_to_analytic_triangle_and_rgb() {
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&triangle([1.0, 0.0, 0.0]), settings);
    assert_report(&report, (3, 1, 1), (3, 1));
    assert_eq!(closed_shell(&mesh), 1);
    planar_points(
        &mesh,
        &[[0.0, 0.0], [0.5, 0.0], [0.0, 0.5]],
        settings.thickness,
        |x, y| [1.0 - x - y, x, y],
        "one-corner",
    );
    metric(
        "one-corner",
        "signed-volume",
        (volume(&mesh) - 0.125 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn two_selected_corners_clip_to_analytic_quadrilateral() {
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&triangle([1.0, 1.0, 0.0]), settings);
    assert_report(&report, (4, 2, 1), (4, 1));
    assert_eq!(closed_shell(&mesh), 1);
    planar_points(
        &mesh,
        &[[0.0, 0.0], [1.0, 0.0], [0.5, 0.5], [0.0, 0.5]],
        settings.thickness,
        |x, y| [1.0 - x - y, x, y],
        "two-corners",
    );
    metric(
        "two-corners",
        "signed-volume",
        (volume(&mesh) - 0.375 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn shared_diagonal_cut_is_welded_and_runs_are_deterministic() {
    let source = square([0.0, 1.0, 1.0, 0.0]);
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&source, settings);
    assert_report(&report, (5, 3, 1), (5, 1));
    assert_eq!(report.selected_source_triangles, 2);
    assert_eq!((mesh.positions.len(), mesh.triangles.len()), (10, 16));
    assert_eq!(closed_shell(&mesh), 1);
    planar_points(
        &mesh,
        &[[0.5, 0.0], [1.0, 0.0], [1.0, 1.0], [0.5, 1.0], [0.5, 0.5]],
        settings.thickness,
        |x, y| [x, y, 0.25],
        "welded-square",
    );
    metric(
        "welded-square",
        "signed-volume",
        (volume(&mesh) - 0.5 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
    for _ in 0..3 {
        let (again, again_report) = extract(&source, settings);
        assert_eq!(bits(&again), bits(&mesh));
        assert_eq!(
            serde_json::to_value(again_report).unwrap(),
            serde_json::to_value(&report).unwrap()
        );
    }
}

#[test]
fn exact_threshold_vertex_is_reused_without_duplicate_cuts() {
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&triangle([0.5, 1.0, 0.0]), settings);
    assert_report(&report, (3, 1, 1), (3, 1));
    assert_eq!(closed_shell(&mesh), 1);
    planar_points(
        &mesh,
        &[[0.0, 0.0], [1.0, 0.0], [0.5, 0.5]],
        settings.thickness,
        |x, y| [1.0 - x - y, x, y],
        "threshold-endpoint",
    );
    metric(
        "threshold-endpoint",
        "signed-volume",
        (volume(&mesh) - 0.25 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn full_exact_threshold_face_is_kept_but_zero_area_touches_are_empty() {
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&triangle([0.5; 3]), settings);
    assert_report(&report, (3, 1, 1), (3, 1));
    assert_eq!(closed_shell(&mesh), 1);
    for masks in [[0.5, 0.0, 0.0], [0.5, 0.5, 0.0], [0.0; 3]] {
        let source = triangle(masks);
        let before = bits(&source);
        assert!(source.extract_mask(&settings).is_err());
        assert_eq!(bits(&source), before);
    }
}

#[test]
fn threshold_and_thickness_boundaries_follow_analytic_patch_area() {
    for threshold in [0.01, 0.25, 0.5, 0.75, 0.99] {
        for thickness in [0.0001, 0.02, 1.0] {
            let settings = MaskExtractSettings {
                threshold,
                thickness,
            };
            let (mesh, report) = extract(&triangle([1.0, 0.0, 0.0]), settings);
            assert_report(&report, (3, 1, 1), (3, 1));
            assert_eq!(closed_shell(&mesh), 1);
            let edge = 1.0 - f64::from(threshold);
            metric(
                "parameter-boundaries",
                "signed-volume",
                (volume(&mesh) - 0.5 * edge * edge * f64::from(thickness)).abs(),
                VOLUME_GATE,
            );
            planar_points(
                &mesh,
                &[[0.0, 0.0], [edge, 0.0], [0.0, edge]],
                thickness,
                |x, y| [1.0 - x - y, x, y],
                "parameter-boundaries",
            );
        }
    }
}

#[test]
fn disconnected_patches_form_separate_closed_solid_components() {
    let source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [3, 4, 5]],
        &[1.0; 6],
    );
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&source, settings);
    assert_report(&report, (6, 2, 2), (6, 2));
    assert_eq!(closed_shell(&mesh), 2);
    metric(
        "disconnected-islands",
        "signed-volume",
        (volume(&mesh) - f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn annulus_keeps_inner_hole_and_closes_both_boundary_loops() {
    let points = [
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        [3.0, 3.0, 0.0],
        [0.0, 3.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
        [2.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
    ];
    let faces = [
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let source = fixture(&points, &faces, &[1.0; 8]);
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&source, settings);
    assert_report(&report, (8, 8, 1), (8, 2));
    assert_eq!((mesh.positions.len(), mesh.triangles.len()), (16, 32));
    assert_eq!(closed_shell(&mesh), 1);
    assert!(mesh
        .positions
        .iter()
        .all(|p| !(p.x > 1.0 && p.x < 2.0 && p.y > 1.0 && p.y < 2.0)));
    metric(
        "annulus",
        "signed-volume",
        (volume(&mesh) - 8.0 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn full_closed_sphere_has_nested_skin_components_and_no_side_faces() {
    let mut source = Mesh::sphere(0);
    source.masks.fill(1.0);
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&source, settings);
    assert_report(
        &report,
        (source.positions.len(), source.triangles.len(), 1),
        (0, 0),
    );
    assert_eq!(mesh.triangles.len(), source.triangles.len() * 2);
    assert_eq!(closed_shell(&mesh), 2);
    let mut error = 0.0_f64;
    for point in &source.positions {
        for expected in [*point, *point + point.normalize() * settings.thickness] {
            error = error.max(
                mesh.positions
                    .iter()
                    .map(|p| {
                        maximum(
                            p.to_array().map(f64::from),
                            expected.to_array().map(f64::from),
                        )
                    })
                    .fold(f64::INFINITY, f64::min),
            );
        }
    }
    metric(
        "closed-sphere",
        "radial-offset-positions",
        error,
        POINT_GATE,
    );
    let expected = volume(&source) * ((1.0 + f64::from(settings.thickness)).powi(3) - 1.0);
    metric(
        "closed-sphere",
        "signed-volume",
        (volume(&mesh) - expected).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn selected_geometric_normals_ignore_stale_source_shading_normals() {
    let mut source = triangle([1.0; 3]);
    source.normals.fill(Vec3::X);
    let settings = MaskExtractSettings::default();
    let (mesh, _) = extract(&source, settings);
    assert_eq!(closed_shell(&mesh), 1);
    planar_points(
        &mesh,
        &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        settings.thickness,
        |x, y| [1.0 - x - y, x, y],
        "geometric-not-shading-normals",
    );
}

#[test]
fn right_angle_patch_offsets_shared_vertices_by_angle_weighted_normal() {
    let source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        &[[0, 1, 2], [0, 3, 1]],
        &[1.0; 4],
    );
    let settings = MaskExtractSettings::default();
    let (mesh, report) = extract(&source, settings);
    assert_report(&report, (4, 2, 1), (4, 1));
    assert_eq!(closed_shell(&mesh), 1);
    let diagonal = f64::from(settings.thickness) / 2.0_f64.sqrt();
    let expected = [
        [0.0, diagonal, diagonal],
        [1.0, diagonal, diagonal],
        [0.0, 1.0, f64::from(settings.thickness)],
        [0.0, f64::from(settings.thickness), 1.0],
    ];
    let error = expected
        .iter()
        .map(|point| {
            mesh.positions
                .iter()
                .map(|p| maximum(p.to_array().map(f64::from), *point))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    metric(
        "right-angle-patch",
        "angle-weighted-positions",
        error,
        POINT_GATE,
    );
}

#[test]
fn reversing_source_winding_reverses_offset_direction_but_keeps_positive_volume() {
    let mut source = triangle([1.0; 3]);
    source.triangles[0].swap(1, 2);
    source.recompute_normals();
    let settings = MaskExtractSettings::default();
    let (mesh, _) = extract(&source, settings);
    assert_eq!(closed_shell(&mesh), 1);
    assert!(mesh
        .positions
        .iter()
        .all(|p| p.z == 0.0 || p.z == -settings.thickness));
    metric(
        "reversed-source",
        "signed-volume",
        (volume(&mesh) - 0.5 * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

fn with_redo(mesh: Mesh) -> Document {
    let mut document = Document::new(mesh);
    document.set_subtool_name(1, "Prior genuine redo").unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    document
}

fn rejected_document(mut document: Document, settings: MaskExtractSettings, case: &str) {
    let before = state(&document);
    let mesh = bits(document.active_mesh().unwrap());
    let can_redo = document.can_redo();
    let result = document.extract_mask_as_subtool(&settings);
    assert!(result.is_err(), "{case} should reject");
    assert_eq!(
        state(&document),
        before,
        "{case} must preserve complete scene"
    );
    assert_eq!(
        bits(document.active_mesh().unwrap()),
        mesh,
        "{case} must preserve source channels"
    );
    assert_eq!(document.can_redo(), can_redo, "{case} must preserve redo");
    eprintln!(
        "mask_extract_contract_rejection: {}",
        json!({"case":case,"error":result.err(),"scene_exact":true,"redo_preserved":true})
    );
}

#[test]
fn invalid_nonfinite_and_out_of_range_settings_preserve_scene_and_redo() {
    for threshold in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        0.0,
        0.009,
        0.991,
        1.0,
    ] {
        rejected_document(
            with_redo(triangle([1.0; 3])),
            MaskExtractSettings {
                threshold,
                ..Default::default()
            },
            "invalid-threshold",
        );
    }
    for thickness in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -5.001, 5.001] {
        rejected_document(
            with_redo(triangle([1.0; 3])),
            MaskExtractSettings {
                thickness,
                ..Default::default()
            },
            "invalid-thickness",
        );
    }
}

#[test]
fn empty_selected_area_preserves_scene_and_redo() {
    rejected_document(
        with_redo(triangle([0.0; 3])),
        MaskExtractSettings::default(),
        "empty-mask",
    );
}

#[test]
fn nonmanifold_shared_edge_and_inconsistent_winding_reject_atomically() {
    let edge = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        &[[0, 1, 2], [1, 0, 3], [0, 1, 4]],
        &[1.0; 5],
    );
    rejected_document(
        with_redo(edge),
        MaskExtractSettings::default(),
        "three-face-edge",
    );
    let mut wrong = square([1.0; 4]);
    wrong.triangles[1].swap(1, 2);
    wrong.recompute_normals();
    rejected_document(
        with_redo(wrong),
        MaskExtractSettings::default(),
        "same-direction-shared-edge",
    );
}

#[test]
fn disconnected_source_vertex_fan_and_branching_threshold_boundary_are_rejected() {
    let fan = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ],
        &[[0, 1, 2], [0, 3, 4]],
        &[1.0; 5],
    );
    rejected_document(
        with_redo(fan),
        MaskExtractSettings::default(),
        "disconnected-vertex-fan",
    );
    let branch = fixture(
        &[
            [0.5, 0.5, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]],
        &[0.5, 1.0, 0.0, 1.0, 0.0],
    );
    rejected_document(
        with_redo(branch),
        MaskExtractSettings::default(),
        "threshold-touching-regions",
    );
}

#[test]
fn duplicate_and_geometrically_degenerate_faces_are_rejected() {
    let mut duplicate = triangle([1.0; 3]);
    duplicate.triangles.push([0, 1, 2]);
    rejected_document(
        with_redo(duplicate),
        MaskExtractSettings::default(),
        "duplicate-face",
    );
    let flat = fixture(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        &[[0, 1, 2]],
        &[1.0; 3],
    );
    rejected_document(
        with_redo(flat),
        MaskExtractSettings::default(),
        "zero-area-source-face",
    );
}

#[test]
fn invalid_source_channels_indices_and_face_ids_do_not_mutate_mesh() {
    for mutation in 0..8 {
        let mut source = triangle([1.0; 3]);
        match mutation {
            0 => source.positions[0].x = f32::NAN,
            1 => source.positions[0].x = 1_000_001.0,
            2 => source.normals[0].x = f32::INFINITY,
            3 => source.masks[0] = f32::NAN,
            4 => source.masks[0] = 1.01,
            5 => source.colors[0][1] = -0.01,
            6 => source.triangles[0][2] = 99,
            _ => source.triangles[0] = [0, 0, 1],
        }
        let before = bits(&source);
        assert!(
            source
                .extract_mask(&MaskExtractSettings::default())
                .is_err(),
            "invalid source case {mutation}"
        );
        assert_eq!(bits(&source), before);
    }
}

#[test]
fn f32_thickness_collapse_is_rejected_with_scene_and_redo_intact() {
    let source = fixture(
        &[
            [0.0, 0.0, 100_000.0],
            [1.0, 0.0, 100_000.0],
            [0.0, 1.0, 100_000.0],
        ],
        &[[0, 1, 2]],
        &[1.0; 3],
    );
    rejected_document(
        with_redo(source),
        MaskExtractSettings {
            thickness: 0.0001,
            ..Default::default()
        },
        "rounded-thickness-collapse",
    );
}

#[test]
fn positive_f64_cut_that_collapses_in_f32_is_rejected() {
    let source = fixture(
        &[
            [100_000.0, 100_000.0, 0.0],
            [100_001.0, 100_000.0, 0.0],
            [100_000.0, 100_001.0, 0.0],
        ],
        &[[0, 1, 2]],
        &[0.50000006, 0.0, 0.0],
    );
    rejected_document(
        with_redo(source),
        MaskExtractSettings::default(),
        "rounded-cut-collapse",
    );
}

#[test]
fn world_coordinate_overflow_is_rejected_before_document_installation() {
    let source = fixture(
        &[
            [0.0, 0.0, 1_000_000.0],
            [1.0, 0.0, 1_000_000.0],
            [0.0, 1.0, 1_000_000.0],
        ],
        &[[0, 1, 2]],
        &[1.0; 3],
    );
    rejected_document(
        with_redo(source),
        MaskExtractSettings::default(),
        "outward-coordinate-overflow",
    );
}

#[test]
fn malformed_over_limit_source_counts_reject_without_output_allocation() {
    // Extraction has its own scratch-budget guard below general scene caps.
    let mut extraction_points = triangle([1.0; 3]);
    extraction_points.positions.resize(250_001, Vec3::ZERO);
    let error = extraction_points
        .extract_mask(&MaskExtractSettings::default())
        .unwrap_err();
    assert!(error.contains("250000"));
    assert_eq!(extraction_points.positions.len(), 250_001);
    let mut extraction_faces = triangle([1.0; 3]);
    extraction_faces.triangles.resize(500_001, [0, 1, 2]);
    let error = extraction_faces
        .extract_mask(&MaskExtractSettings::default())
        .unwrap_err();
    assert!(error.contains("500000"));
    assert_eq!(extraction_faces.triangles.len(), 500_001);
    let mut too_many_points = triangle([1.0; 3]);
    too_many_points
        .positions
        .resize(obrush_core::MAX_VERTICES + 1, Vec3::ZERO);
    assert!(too_many_points
        .extract_mask(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(
        too_many_points.positions.len(),
        obrush_core::MAX_VERTICES + 1
    );
    let mut too_many_faces = triangle([1.0; 3]);
    too_many_faces
        .triangles
        .resize(obrush_core::MAX_TRIANGLES + 1, [0, 1, 2]);
    assert!(too_many_faces
        .extract_mask(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(
        too_many_faces.triangles.len(),
        obrush_core::MAX_TRIANGLES + 1
    );
}

#[test]
fn full_subtool_count_rejects_extraction_without_consuming_redo_or_identity() {
    let mut document = Document::new(triangle([1.0; 3]));
    for index in 1..obrush_core::MAX_SUBTOOLS {
        document
            .add_subtool(triangle([1.0; 3]), &format!("Part {index}"))
            .unwrap();
    }
    let active = document.active_subtool_id().unwrap();
    document
        .set_subtool_name(active, "Genuine prior redo")
        .unwrap();
    assert!(document.undo());
    assert_eq!(document.subtool_count(), obrush_core::MAX_SUBTOOLS);
    rejected_document(
        document,
        MaskExtractSettings::default(),
        "subtool-count-cap",
    );
}

fn rich_document() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "Independent extraction source".into();
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    document.add_layer("Hidden offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[9] += Vec3::new(0.007, -0.005, 0.009);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.add_layer("Faded offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[21] += Vec3::new(-0.003, 0.008, 0.005);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_intensity(1, 0.37).unwrap();
    document.add_layer("Recording offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[17] += Vec3::new(0.002, -0.004, 0.003);
    document.active_mesh_mut().unwrap().masks.fill(1.0);
    for (index, color) in document
        .active_mesh_mut()
        .unwrap()
        .colors
        .iter_mut()
        .enumerate()
    {
        *color = [
            (index % 7) as f32 / 7.0,
            (index % 11) as f32 / 11.0,
            (index % 13) as f32 / 13.0,
        ];
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    document
}

#[test]
fn extracted_part_preserves_rich_source_inactive_arc_and_exact_scene_history() {
    let mut document = rich_document();
    let source_id = document.active_subtool_id().unwrap();
    let other_id = document
        .add_subtool(triangle([0.0; 3]), "Unrelated original part")
        .unwrap();
    document.select_subtool(source_id).unwrap();
    let inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == other_id)
        .unwrap()
        .mesh;
    let source_payload = payload(&document);
    let source_bits = bits(document.active_mesh().unwrap());
    let before = state(&document);
    let summaries = document.subtool_summaries();
    let (new_id, report) = document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap();
    assert_ne!(new_id, source_id);
    assert_ne!(new_id, other_id);
    assert_eq!(document.active_subtool_id().unwrap(), new_id);
    assert_eq!(document.subtool_count(), 3);
    assert_eq!(
        (
            document.level_count(),
            document.current_level(),
            document.layer_count()
        ),
        (1, 0, 0)
    );
    assert!(!document.has_morph_target());
    assert!(document.active_subtool_visible());
    assert_eq!(closed_shell(document.active_mesh().unwrap()), 2);
    assert_eq!(
        report.input_vertices,
        source_bits["positions"].as_array().unwrap().len()
    );
    let now = document.subtool_render_snapshots();
    let retained = now.iter().find(|s| s.id == other_id).unwrap();
    assert!(Arc::ptr_eq(&inactive, &retained.mesh));
    let retained_source = now.iter().find(|s| s.id == source_id).unwrap();
    assert_eq!(bits(&retained_source.mesh), source_bits);
    let new_summaries = document.subtool_summaries();
    for old in summaries {
        let new = new_summaries.iter().find(|s| s.id == old.id).unwrap();
        assert_eq!((&new.name, new.visible), (&old.name, old.visible));
    }
    let extracted = state(&document);
    assert_eq!(
        serde_json::from_str::<Value>(&extracted).unwrap()["version"],
        5
    );
    assert_eq!(
        Document::from_json(&extracted).unwrap().to_json().unwrap(),
        extracted
    );
    document.select_subtool(source_id).unwrap();
    assert_eq!(payload(&document), source_payload);
    assert_eq!(bits(document.active_mesh().unwrap()), source_bits);
    document.select_subtool(new_id).unwrap();
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), extracted);
}

#[test]
fn later_edits_to_extracted_part_are_independent_of_source() {
    let mut document = Document::new(square([0.0, 1.0, 1.0, 0.0]));
    let source = payload(&document);
    let (new_id, _) = document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x += 0.007;
    document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.3];
    document.active_mesh_mut().unwrap().masks[0] = 0.77;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert_eq!(document.active_subtool_id().unwrap(), new_id);
    document.select_subtool(1).unwrap();
    assert_eq!(payload(&document), source);
}

#[test]
fn solo_selection_is_retained_and_new_part_becomes_effectively_visible() {
    let mut document = Document::new(triangle([1.0; 3]));
    document.set_solo(true).unwrap();
    let before = state(&document);
    let (id, _) = document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap();
    assert!(document.solo());
    assert_eq!(document.active_subtool_id().unwrap(), id);
    assert!(document.active_subtool_visible());
    assert_eq!(document.subtool_render_snapshots().len(), 1);
    assert!(document.undo());
    assert_eq!(state(&document), before);
}

#[test]
fn hidden_active_source_rejects_extraction_without_changing_redo() {
    let mut document = Document::new(triangle([1.0; 3]));
    document.set_subtool_visible(1, false).unwrap();
    document.set_subtool_name(1, "Prior rename").unwrap();
    assert!(document.undo());
    assert!(!document.active_subtool_visible());
    rejected_document(
        document,
        MaskExtractSettings::default(),
        "hidden-active-source",
    );
}

#[test]
fn pending_invalid_stroke_restores_complete_scene_and_preserves_redo() {
    let mut document = rich_document();
    document.set_subtool_name(1, "Genuine prior redo").unwrap();
    assert!(document.undo());
    let before = state(&document);
    let old = bits(document.active_mesh().unwrap());
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x = f32::NAN;
    assert!(document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(state(&document), before);
    assert_eq!(bits(document.active_mesh().unwrap()), old);
    assert!(document.can_redo());
    assert!(document.take_layer_error().is_some());
}

#[test]
fn nonbinary_masks_use_general_scalar_interpolation_with_barycentric_rgb() {
    let source = triangle([0.9, 0.2, 0.1]);
    let settings = MaskExtractSettings {
        threshold: 0.6,
        thickness: 0.023,
    };
    let (mesh, report) = extract(&source, settings);
    assert_report(&report, (3, 1, 1), (3, 1));
    assert_eq!(closed_shell(&mesh), 1);
    let t = f64::from(settings.threshold);
    let a = f64::from(source.masks[0]);
    let x = (a - t) / (a - f64::from(source.masks[1]));
    let y = (a - t) / (a - f64::from(source.masks[2]));
    planar_points(
        &mesh,
        &[[0.0, 0.0], [x, 0.0], [0.0, y]],
        settings.thickness,
        |x, y| [1.0 - x - y, x, y],
        "nonbinary-masks",
    );
    metric(
        "nonbinary-masks",
        "signed-volume",
        (volume(&mesh) - 0.5 * x * y * f64::from(settings.thickness)).abs(),
        VOLUME_GATE,
    );
}

#[test]
fn unselected_degenerate_component_still_rejects_the_complete_source() {
    let source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
        &[[0, 1, 2], [3, 4, 5]],
        &[1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
    );
    rejected_document(
        with_redo(source),
        MaskExtractSettings::default(),
        "unselected-zero-area-source-face",
    );
}

#[test]
fn pending_valid_stroke_finishes_before_one_separate_extraction_undo_step() {
    let mut document = Document::new(triangle([1.0; 3]));
    let initial = state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x -= 0.03;
    document.active_mesh_mut().unwrap().colors[0] = [0.2, 0.3, 0.4];
    document.active_mesh_mut().unwrap().recompute_normals();
    let finished_stroke = state(&document);
    let (id, _) = document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap();
    assert_eq!(document.active_subtool_id().unwrap(), id);
    let extracted = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), finished_stroke);
    assert!(document.undo());
    assert_eq!(state(&document), initial);
    assert!(document.redo());
    assert_eq!(state(&document), finished_stroke);
    assert!(document.redo());
    assert_eq!(state(&document), extracted);
}
