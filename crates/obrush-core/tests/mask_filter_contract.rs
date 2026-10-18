//! Independent scalar mask-graph contracts. These fixtures establish OBrush
//! invariants, not reference mask-filter behavior. Gates were preregistered.
use obrush_core::{
    Document, MaskFilterOperation, MaskFilterReport, MaskFilterSettings, Mesh, Vec3,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;

const MASK_GATE: f64 = 1e-7;
const OPERATIONS: [MaskFilterOperation; 4] = [
    MaskFilterOperation::Grow,
    MaskFilterOperation::Shrink,
    MaskFilterOperation::Blur,
    MaskFilterOperation::Sharpen,
];

fn fixture(points: &[[f32; 3]], faces: &[[u32; 3]], masks: &[f32]) -> Mesh {
    let mut obj = String::new();
    for p in points {
        obj.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
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
    for (index, color) in mesh.colors.iter_mut().enumerate() {
        *color = [
            (index % 7) as f32 / 7.0,
            (index % 11) as f32 / 11.0,
            (index % 13) as f32 / 13.0,
        ];
    }
    mesh
}

fn triangle(masks: [f32; 3]) -> Mesh {
    fixture(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &masks,
    )
}

fn chain(masks: [f32; 6]) -> Mesh {
    fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
            [3.0, 2.0, 0.0],
        ],
        &[[0, 1, 2], [2, 3, 4]],
        &masks,
    )
}

fn settings(operation: MaskFilterOperation, iterations: u32, strength: f32) -> MaskFilterSettings {
    MaskFilterSettings {
        operation,
        iterations,
        strength,
    }
}

fn bits(mesh: &Mesh) -> Value {
    json!({
        "positions":mesh.positions.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":mesh.normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":mesh.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
        "colors":mesh.colors.iter().map(|v|v.map(f32::to_bits)).collect::<Vec<_>>(),
        "triangles":mesh.triangles,
    })
}

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn strip_masks(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.remove("masks");
            for value in fields.values_mut() {
                strip_masks(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                strip_masks(value);
            }
        }
        _ => (),
    }
}

fn nonmask_document(document: &Document) -> Value {
    let mut value: Value = serde_json::from_str(&state(document)).unwrap();
    strip_masks(&mut value);
    value
}

fn neighbors(mesh: &Mesh) -> Vec<BTreeSet<usize>> {
    let mut graph = vec![BTreeSet::new(); mesh.positions.len()];
    for &[a, b, c] in &mesh.triangles {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            if a != b {
                graph[a as usize].insert(b as usize);
                graph[b as usize].insert(a as usize);
            }
        }
    }
    graph
}

fn oracle(mesh: &Mesh, settings: MaskFilterSettings) -> Vec<f32> {
    let graph = neighbors(mesh);
    let mut previous = mesh.masks.clone();
    for _ in 0..settings.iterations {
        let mut next = previous.clone();
        for (index, &old) in previous.iter().enumerate() {
            if settings.strength == 0.0 {
                continue;
            }
            let m = f64::from(old);
            let s = f64::from(settings.strength);
            let raw = match settings.operation {
                MaskFilterOperation::Sharpen => (0.5 + (m - 0.5) * (1.0 + s)).clamp(0.0, 1.0),
                _ if graph[index].is_empty() => m,
                MaskFilterOperation::Grow => {
                    let target = graph[index]
                        .iter()
                        .map(|&v| f64::from(previous[v]))
                        .fold(m, f64::max);
                    m + s * (target - m)
                }
                MaskFilterOperation::Shrink => {
                    let target = graph[index]
                        .iter()
                        .map(|&v| f64::from(previous[v]))
                        .fold(m, f64::min);
                    m + s * (target - m)
                }
                MaskFilterOperation::Blur => {
                    let target = (m + graph[index]
                        .iter()
                        .map(|&v| f64::from(previous[v]))
                        .sum::<f64>())
                        / (graph[index].len() + 1) as f64;
                    m + s * (target - m)
                }
            };
            let rounded = raw as f32;
            // Preserve signed zero and every exact representational fixedpoint.
            if rounded != old {
                next[index] = rounded;
            }
        }
        previous = next;
    }
    previous
}

fn metric(case: &str, actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    let maximum = actual
        .iter()
        .zip(expected)
        .map(|(&a, &b)| (f64::from(a) - f64::from(b)).abs())
        .fold(0.0, f64::max);
    eprintln!(
        "mask_filter_contract_metric: {}",
        json!({"case":case,"max_absolute_mask_error":maximum,"gate":MASK_GATE})
    );
    assert!(
        maximum <= MASK_GATE,
        "{case}: {maximum} exceeds {MASK_GATE}"
    );
}

fn check_report(
    report: &MaskFilterReport,
    source: &[f32],
    output: &[f32],
    requested: MaskFilterSettings,
) {
    assert_eq!(
        serde_json::to_value(report.operation).unwrap(),
        serde_json::to_value(requested.operation).unwrap()
    );
    assert_eq!(report.iterations, requested.iterations);
    assert_eq!(report.strength.to_bits(), requested.strength.to_bits());
    assert_eq!(report.vertices, source.len());
    let changed = source
        .iter()
        .zip(output)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    let maximum = source
        .iter()
        .zip(output)
        .map(|(&a, &b)| (f64::from(a) - f64::from(b)).abs())
        .fold(0.0, f64::max);
    assert_eq!(report.changed_vertices, changed);
    assert_eq!(report.max_abs_change.to_bits(), maximum.to_bits());
}

fn filtered(
    mesh: &Mesh,
    requested: MaskFilterSettings,
    case: &str,
) -> (Vec<f32>, MaskFilterReport) {
    let before = bits(mesh);
    let expected = oracle(mesh, requested);
    let (output, report) = mesh.filtered_masks(&requested).unwrap();
    assert_eq!(
        bits(mesh),
        before,
        "filter must leave source mesh immutable"
    );
    metric(case, &output, &expected);
    assert!(output
        .iter()
        .all(|m| m.is_finite() && (0.0..=1.0).contains(m)));
    check_report(&report, &mesh.masks, &output, requested);
    (output, report)
}

#[test]
fn defaults_and_report_serialization_match_declared_contract() {
    let requested = MaskFilterSettings::default();
    assert_eq!(requested.iterations, 1);
    assert_eq!(requested.strength, 1.0);
    assert_eq!(serde_json::to_value(requested.operation).unwrap(), "blur");
    let (_, report) = filtered(&triangle([0.0, 0.25, 1.0]), requested, "defaults");
    let encoded = serde_json::to_string(&report).unwrap();
    let decoded: MaskFilterReport = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
    let settings_json = serde_json::to_string(&requested).unwrap();
    let decoded: MaskFilterSettings = serde_json::from_str(&settings_json).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), settings_json);
}

#[test]
fn analytic_triangle_grow_shrink_and_blur_include_self_once() {
    let source = triangle([0.0, 0.25, 1.0]);
    let (grow, _) = filtered(
        &source,
        settings(MaskFilterOperation::Grow, 1, 1.0),
        "triangle-grow",
    );
    assert_eq!(grow, vec![1.0; 3]);
    let (shrink, _) = filtered(
        &source,
        settings(MaskFilterOperation::Shrink, 1, 1.0),
        "triangle-shrink",
    );
    assert_eq!(shrink, vec![0.0; 3]);
    let (blur, _) = filtered(
        &source,
        settings(MaskFilterOperation::Blur, 1, 1.0),
        "triangle-blur",
    );
    assert_eq!(blur, vec![(1.25_f64 / 3.0) as f32; 3]);
}

#[test]
fn fractional_strength_blends_graph_target_once() {
    let source = triangle([0.0, 0.25, 1.0]);
    for operation in [
        MaskFilterOperation::Grow,
        MaskFilterOperation::Shrink,
        MaskFilterOperation::Blur,
    ] {
        let (output, _) = filtered(
            &source,
            settings(operation, 1, 0.5),
            "triangle-half-strength",
        );
        let target = match operation {
            MaskFilterOperation::Grow => 1.0,
            MaskFilterOperation::Shrink => 0.0,
            _ => 1.25 / 3.0,
        };
        for (&old, &new) in source.masks.iter().zip(&output) {
            assert_eq!(
                new,
                (f64::from(old) + 0.5 * (target - f64::from(old))) as f32
            );
        }
    }
}

#[test]
fn propagation_uses_previous_pass_not_in_place_vertex_order() {
    let source = chain([1.0, 0.0, 0.0, 0.0, 0.0, 0.3]);
    let (one, _) = filtered(
        &source,
        settings(MaskFilterOperation::Grow, 1, 1.0),
        "synchronous-grow-one",
    );
    assert_eq!(one, vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.3]);
    let (two, _) = filtered(
        &source,
        settings(MaskFilterOperation::Grow, 2, 1.0),
        "synchronous-grow-two",
    );
    assert_eq!(two, vec![1.0, 1.0, 1.0, 1.0, 1.0, 0.3]);
    let source = chain([0.0, 1.0, 1.0, 1.0, 1.0, 0.7]);
    let (one, _) = filtered(
        &source,
        settings(MaskFilterOperation::Shrink, 1, 1.0),
        "synchronous-shrink-one",
    );
    assert_eq!(one, vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.7]);
    let (two, _) = filtered(
        &source,
        settings(MaskFilterOperation::Shrink, 2, 1.0),
        "synchronous-shrink-two",
    );
    assert_eq!(two, vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.7]);
}

#[test]
fn blur_on_irregular_graph_uses_unique_degree_not_face_incidence() {
    let source = chain([1.0, 0.0, 0.0, 0.0, 0.0, 0.7]);
    let (output, _) = filtered(
        &source,
        settings(MaskFilterOperation::Blur, 1, 1.0),
        "irregular-blur",
    );
    assert_eq!(
        output,
        vec![
            (1.0_f64 / 3.0) as f32,
            (1.0_f64 / 3.0) as f32,
            0.2,
            0.0,
            0.0,
            0.7
        ]
    );
}

#[test]
fn square_diagonal_changes_neighbors_but_duplicate_faces_do_not_change_weights() {
    let mut source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
        &[0.1, 0.4, 0.7, 0.9],
    );
    let baseline = source.clone();
    source
        .triangles
        .extend([[0, 1, 2], [0, 2, 3], [2, 1, 0], [3, 2, 0]]);
    source.recompute_normals();
    for operation in OPERATIONS {
        let requested = settings(operation, 3, 0.37);
        let (a, _) = filtered(&baseline, requested, "square-baseline");
        let (b, _) = filtered(&source, requested, "square-duplicate-faces");
        assert_eq!(
            a.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn face_order_and_winding_are_irrelevant_to_mask_graph() {
    let source = chain([0.13, 0.29, 0.41, 0.57, 0.83, 0.73]);
    let mut reordered = source.clone();
    reordered.triangles.reverse();
    for face in &mut reordered.triangles {
        face.swap(1, 2);
    }
    for operation in OPERATIONS {
        let requested = settings(operation, 5, 0.63);
        let (a, _) = filtered(&source, requested, "ordered-faces");
        let (b, _) = filtered(&reordered, requested, "reordered-reversed-faces");
        assert_eq!(
            a.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn coincident_distinct_vertex_ids_do_not_weld_disconnected_graphs() {
    let source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [3, 4, 5]],
        &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    );
    let (output, _) = filtered(
        &source,
        settings(MaskFilterOperation::Grow, 16, 1.0),
        "coincident-distinct-ids",
    );
    assert_eq!(output, vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn geometric_degeneracy_and_nonmanifold_edges_are_valid_mask_graphs() {
    let source = fixture(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
        &[[0, 1, 2], [0, 1, 3], [0, 1, 4]],
        &[0.11, 0.29, 0.43, 0.71, 0.91],
    );
    source.validate().unwrap();
    for operation in OPERATIONS {
        filtered(
            &source,
            settings(operation, 4, 0.35),
            "degenerate-nonmanifold-graph",
        );
    }
}

#[test]
fn isolated_vertices_keep_graph_filters_and_receive_pointwise_sharpen() {
    let source = chain([0.2, 0.3, 0.4, 0.5, 0.6, 0.75]);
    for operation in [
        MaskFilterOperation::Grow,
        MaskFilterOperation::Shrink,
        MaskFilterOperation::Blur,
    ] {
        let (output, _) = filtered(
            &source,
            settings(operation, 16, 1.0),
            "isolated-graph-point",
        );
        assert_eq!(output[5].to_bits(), source.masks[5].to_bits());
    }
    let (output, _) = filtered(
        &source,
        settings(MaskFilterOperation::Sharpen, 1, 0.5),
        "isolated-sharpen",
    );
    assert_eq!(output[5], 0.875);
}

#[test]
fn sharpen_is_pointwise_contrast_with_clamping_before_no_extra_blend() {
    let source = chain([0.1, 0.25, 0.5, 0.75, 0.9, 0.2]);
    let (output, _) = filtered(
        &source,
        settings(MaskFilterOperation::Sharpen, 1, 0.5),
        "pointwise-sharpen",
    );
    assert_eq!(output, vec![0.0, 0.125, 0.5, 0.875, 1.0, 0.050000004]);
    let (output, _) = filtered(
        &source,
        settings(MaskFilterOperation::Sharpen, 2, 0.5),
        "pointwise-sharpen-two",
    );
    assert_eq!(output[1], 0.0);
    assert_eq!(output[3], 1.0);
}

#[test]
fn repeated_passes_match_separate_calls_and_independent_scalar_oracle() {
    let source = chain([0.17, 0.23, 0.41, 0.53, 0.79, 0.61]);
    for operation in OPERATIONS {
        for iterations in [1, 2, 3, 8, 16] {
            for strength in [0.0, 0.13, 0.5, 1.0] {
                let requested = settings(operation, iterations, strength);
                let (batch, _) = filtered(&source, requested, "iteration-strength-grid");
                let mut separate = source.clone();
                for _ in 0..iterations {
                    separate.masks = separate
                        .filtered_masks(&settings(operation, 1, strength))
                        .unwrap()
                        .0;
                }
                assert_eq!(
                    batch.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    separate
                        .masks
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn fixedpoints_signed_zero_and_zero_strength_preserve_every_mask_bit() {
    let signed = triangle([-0.0, 0.0, -0.0]);
    for operation in OPERATIONS {
        let (output, report) = filtered(
            &signed,
            settings(operation, 16, 1.0),
            "signed-zero-fixedpoints",
        );
        assert_eq!(
            output.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            signed.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(report.changed_vertices, 0);
    }
    let source = triangle([0.0, 0.5, 1.0]);
    let (output, report) = filtered(
        &source,
        settings(MaskFilterOperation::Sharpen, 16, 1.0),
        "sharpen-fixedpoints",
    );
    assert_eq!(output, source.masks);
    assert_eq!(report.changed_vertices, 0);
    for operation in OPERATIONS {
        let source = chain([0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
        let (output, report) = filtered(
            &source,
            settings(operation, 16, -0.0),
            "zero-strength-fixedpoints",
        );
        assert_eq!(
            output.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            source.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(report.changed_vertices, 0);
    }
}

#[test]
fn rounded_away_blend_is_a_true_reported_noop() {
    let source = chain([0.25, 0.375, 0.5, 0.625, 0.75, 0.875]);
    for operation in OPERATIONS {
        let (output, report) =
            filtered(&source, settings(operation, 16, 1e-20), "sub-f32-strength");
        assert_eq!(
            output.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            source.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(report.max_abs_change, 0.0);
    }
}

fn with_redo(mesh: Mesh) -> Document {
    let mut document = Document::new(mesh);
    document.set_subtool_name(1, "Prior genuine redo").unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    document
}

fn reject_document(mut document: Document, requested: MaskFilterSettings, case: &str) {
    let before = state(&document);
    let source = bits(document.active_mesh().unwrap());
    let redo = document.can_redo();
    let result = document.filter_mask(&requested);
    assert!(result.is_err(), "{case} must reject");
    assert_eq!(state(&document), before, "{case} serialized scene");
    assert_eq!(
        bits(document.active_mesh().unwrap()),
        source,
        "{case} exact source bits"
    );
    assert_eq!(document.can_redo(), redo, "{case} redo");
    eprintln!(
        "mask_filter_contract_rejection: {}",
        json!({"case":case,"error":result.err(),"scene_exact":true,"redo_preserved":true})
    );
}

#[test]
fn invalid_iterations_strength_and_nonfinite_settings_reject_atomically() {
    for operation in OPERATIONS {
        for iterations in [0, 17, u32::MAX] {
            reject_document(
                with_redo(triangle([0.1, 0.4, 0.9])),
                settings(operation, iterations, 0.5),
                "invalid-iterations",
            );
        }
        for strength in [-0.001, 1.001, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            reject_document(
                with_redo(triangle([0.1, 0.4, 0.9])),
                settings(operation, 1, strength),
                "invalid-strength",
            );
        }
    }
}

#[test]
fn invalid_source_masks_indices_channels_and_self_edges_are_not_mutated() {
    for operation in OPERATIONS {
        for mutation in 0..11 {
            let mut source = triangle([0.1, 0.4, 0.9]);
            match mutation {
                0 => source.masks[0] = f32::NAN,
                1 => source.masks[0] = f32::INFINITY,
                2 => source.masks[0] = -0.01,
                3 => source.masks[0] = 1.01,
                4 => {
                    source.masks.pop();
                }
                5 => source.triangles[0][1] = 9,
                6 => source.triangles[0] = [0, 0, 1],
                7 => source.positions[0].x = f32::NAN,
                8 => source.normals[0].z = f32::INFINITY,
                9 => source.colors[0][1] = 1.1,
                _ => source.positions[0].x = 1_000_001.0,
            }
            let before = bits(&source);
            for strength in [0.0, 0.5] {
                assert!(
                    source
                        .filtered_masks(&settings(operation, 1, strength))
                        .is_err(),
                    "invalid source {mutation}"
                );
                assert_eq!(bits(&source), before);
            }
        }
    }
}

#[test]
fn empty_mesh_and_empty_triangle_input_reject_before_filter_work() {
    let mut source = triangle([0.1, 0.4, 0.9]);
    source.triangles.clear();
    for operation in OPERATIONS {
        assert!(source.filtered_masks(&settings(operation, 1, 0.5)).is_err());
    }
    source.positions.clear();
    source.normals.clear();
    source.masks.clear();
    source.colors.clear();
    for operation in OPERATIONS {
        assert!(source.filtered_masks(&settings(operation, 1, 0.5)).is_err());
    }
}

#[test]
fn malformed_count_caps_reject_without_adjacency_allocation() {
    let mut points = triangle([0.1, 0.4, 0.9]);
    points
        .positions
        .resize(obrush_core::MAX_VERTICES + 1, Vec3::ZERO);
    let mut faces = triangle([0.1, 0.4, 0.9]);
    faces
        .triangles
        .resize(obrush_core::MAX_TRIANGLES + 1, [0, 1, 2]);
    for operation in OPERATIONS {
        assert!(points.filtered_masks(&settings(operation, 1, 0.5)).is_err());
        assert!(faces.filtered_masks(&settings(operation, 1, 0.5)).is_err());
    }
    assert_eq!(points.positions.len(), obrush_core::MAX_VERTICES + 1);
    assert_eq!(faces.triangles.len(), obrush_core::MAX_TRIANGLES + 1);
}

#[test]
fn hidden_active_document_rejects_without_consuming_redo() {
    let mut document = Document::new(triangle([0.1, 0.4, 0.9]));
    document.set_subtool_visible(1, false).unwrap();
    document.set_subtool_name(1, "Prior rename").unwrap();
    assert!(document.undo());
    for operation in OPERATIONS {
        let requested = settings(operation, 1, 0.5);
        let before = state(&document);
        assert!(document.filter_mask(&requested).is_err());
        assert_eq!(state(&document), before);
        assert!(document.can_redo());
    }
}

#[test]
fn plain_document_mask_filter_has_one_exact_undo_redo_and_keeps_project_version() {
    for operation in OPERATIONS {
        let mut document = with_redo(chain([0.1, 0.3, 0.4, 0.6, 0.9, 0.2]));
        let before = state(&document);
        let shape = nonmask_document(&document);
        let requested = settings(operation, 3, 0.37);
        let expected = oracle(document.active_mesh().unwrap(), requested);
        let old = document.active_mesh().unwrap().masks.clone();
        let report = document.filter_mask(&requested).unwrap();
        assert!(report.changed_vertices > 0);
        assert!(!document.can_redo());
        metric(
            "document-filter",
            &document.active_mesh().unwrap().masks,
            &expected,
        );
        check_report(
            &report,
            &old,
            &document.active_mesh().unwrap().masks,
            requested,
        );
        assert_eq!(nonmask_document(&document), shape);
        let after = state(&document);
        assert_eq!(serde_json::from_str::<Value>(&after).unwrap()["version"], 2);
        assert_eq!(
            Document::from_json(&after).unwrap().to_json().unwrap(),
            after
        );
        assert!(document.undo());
        assert_eq!(state(&document), before);
        assert!(document.redo());
        assert_eq!(state(&document), after);
    }
}

fn rich() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "Independent filter source".into();
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    document.add_layer("Hidden offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[7] += Vec3::new(0.005, -0.003, 0.007);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.add_layer("Faded offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[19] += Vec3::new(-0.002, 0.006, 0.004);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_intensity(1, 0.37).unwrap();
    document.add_layer("Recording offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[13] += Vec3::new(0.002, -0.004, 0.003);
    for (index, mask) in document
        .active_mesh_mut()
        .unwrap()
        .masks
        .iter_mut()
        .enumerate()
    {
        *mask = 0.1 + (index % 13) as f32 / 16.0;
    }
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
fn rich_scene_filter_keeps_geometry_layers_target_inactive_arc_and_one_history_step() {
    let mut document = rich();
    let source_id = document.active_subtool_id().unwrap();
    let inactive_id = document
        .add_subtool(triangle([0.2, 0.4, 0.8]), "Unrelated part")
        .unwrap();
    document.select_subtool(source_id).unwrap();
    let inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == inactive_id)
        .unwrap()
        .mesh;
    let before = state(&document);
    let source_bits = bits(document.active_mesh().unwrap());
    let nonmask = nonmask_document(&document);
    let requested = settings(MaskFilterOperation::Blur, 3, 0.63);
    let expected = oracle(document.active_mesh().unwrap(), requested);
    let report = document.filter_mask(&requested).unwrap();
    assert!(report.changed_vertices > 0);
    metric(
        "rich-layer-scene",
        &document.active_mesh().unwrap().masks,
        &expected,
    );
    assert_eq!(nonmask_document(&document), nonmask);
    let updated = bits(document.active_mesh().unwrap());
    for key in ["positions", "normals", "colors", "triangles"] {
        assert_eq!(updated[key], source_bits[key]);
    }
    let now = document.subtool_render_snapshots();
    let unchanged = now.iter().find(|s| s.id == inactive_id).unwrap();
    assert!(Arc::ptr_eq(&inactive, &unchanged.mesh));
    assert_eq!(document.active_subtool_id().unwrap(), source_id);
    assert_eq!(
        (
            document.level_count(),
            document.current_level(),
            document.layer_count()
        ),
        (2, 1, 3)
    );
    assert!(document.has_morph_target());
    let after = state(&document);
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn true_noops_after_coarse_sculpt_preserve_unreconciled_hierarchy_and_genuine_redo() {
    let mut document = Document::new(triangle([0.25, 0.5, 0.75]));
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] += Vec3::new(-0.007, 0.003, 0.002);
    document.active_mesh_mut().unwrap().masks[0] = 0.375;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_subtool_name(1, "Genuine prior redo").unwrap();
    let renamed = state(&document);
    assert!(document.undo());
    let before = state(&document);
    let source = bits(document.active_mesh().unwrap());
    for operation in OPERATIONS {
        for strength in [0.0, 1e-20] {
            let report = document
                .filter_mask(&settings(operation, 16, strength))
                .unwrap();
            assert_eq!(report.changed_vertices, 0);
            assert_eq!(report.max_abs_change, 0.0);
            assert_eq!(state(&document), before);
            assert_eq!(bits(document.active_mesh().unwrap()), source);
            assert!(document.can_redo());
        }
    }
    assert!(document.redo());
    assert_eq!(state(&document), renamed);
}

#[test]
fn constant_grow_shrink_blur_and_sharpen_fixedpoints_preserve_redo() {
    for operation in OPERATIONS {
        let value = if matches!(operation, MaskFilterOperation::Sharpen) {
            0.5
        } else {
            0.375
        };
        let mut document = with_redo(triangle([value; 3]));
        let before = state(&document);
        let report = document.filter_mask(&settings(operation, 16, 1.0)).unwrap();
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(state(&document), before);
        assert!(document.can_redo());
    }
}

#[test]
fn coarse_mask_filter_propagates_existing_loop_masks_and_retains_fine_residual() {
    let mut document = Document::new(triangle([0.2, 0.4, 0.8]));
    document.add_subdivision_level().unwrap();
    let fine_index = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .position(|p| p.x == 0.5 && p.y == 0.0)
        .unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[fine_index] = 0.9;
    document.end_stroke();
    let old_child = document.active_mesh().unwrap().masks.clone();
    document.select_level(0).unwrap();
    let geometry = nonmask_document(&document);
    let report = document
        .filter_mask(&settings(MaskFilterOperation::Grow, 1, 1.0))
        .unwrap();
    assert_eq!(report.changed_vertices, 2);
    assert_eq!(document.active_mesh().unwrap().masks, vec![0.8; 3]);
    assert_eq!(nonmask_document(&document), geometry);
    document.select_level(1).unwrap();
    // This fixture's EXISTING Loop mask transport uses f32 stencils and
    // staged f32 add/subtract. A constant coarse value is not a promised
    // bit-exact constant fine value under that earlier contract. Derive the
    // three boundary-even and three edge-midpoint predictions explicitly.
    let predict = |m: [f32; 3]| {
        [
            (m[0] * 0.75 + m[1] * 0.125) + m[2] * 0.125,
            (m[1] * 0.75 + m[0] * 0.125) + m[2] * 0.125,
            (m[2] * 0.75 + m[1] * 0.125) + m[0] * 0.125,
            (m[0] + m[1]) * 0.5,
            (m[1] + m[2]) * 0.5,
            (m[0] + m[2]) * 0.5,
        ]
    };
    let old_prediction = predict([0.2, 0.4, 0.8]);
    let new_prediction = predict([0.8; 3]);
    let expected: Vec<_> = old_child
        .iter()
        .enumerate()
        .map(|(index, &old)| {
            if old_prediction[index] == new_prediction[index] {
                old
            } else {
                (old + new_prediction[index] - old_prediction[index]).clamp(0.0, 1.0)
            }
        })
        .collect();
    metric(
        "coarse-loop-mask-transport",
        &document.active_mesh().unwrap().masks,
        &expected,
    );
    assert_eq!(document.active_mesh().unwrap().masks[fine_index], 1.0);
}

#[test]
fn pending_invalid_stroke_restores_scene_and_blocks_dependent_filter() {
    for operation in OPERATIONS {
        let mut document = rich();
        document.set_subtool_name(1, "Prior redo").unwrap();
        assert!(document.undo());
        let before = state(&document);
        let source = bits(document.active_mesh().unwrap());
        document.begin_stroke();
        document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
        assert!(document.filter_mask(&settings(operation, 1, 0.5)).is_err());
        assert_eq!(state(&document), before);
        assert_eq!(bits(document.active_mesh().unwrap()), source);
        assert!(document.can_redo());
        assert!(document.take_layer_error().is_some());
    }
}

#[test]
fn pending_valid_mask_stroke_finishes_before_a_separate_filter_history_step() {
    let mut document = Document::new(triangle([0.1, 0.4, 0.9]));
    let initial = state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[1] = 0.7;
    let stroke = state(&document);
    let report = document
        .filter_mask(&settings(MaskFilterOperation::Blur, 2, 0.5))
        .unwrap();
    assert!(report.changed_vertices > 0);
    let filtered = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), stroke);
    assert!(document.undo());
    assert_eq!(state(&document), initial);
    assert!(document.redo());
    assert_eq!(state(&document), stroke);
    assert!(document.redo());
    assert_eq!(state(&document), filtered);
}

#[test]
fn invalid_source_public_edit_without_open_stroke_rejects_and_keeps_redo() {
    for operation in OPERATIONS {
        let mut document = with_redo(triangle([0.1, 0.4, 0.9]));
        document.active_mesh_mut().unwrap().masks[1] = f32::NAN;
        let source = bits(document.active_mesh().unwrap());
        assert!(document.filter_mask(&settings(operation, 1, 0.5)).is_err());
        assert_eq!(bits(document.active_mesh().unwrap()), source);
        assert!(document.can_redo());
    }
}

#[test]
fn invalid_settings_are_rejected_even_when_strength_or_masks_would_be_noops() {
    for operation in OPERATIONS {
        let source = triangle([0.5; 3]);
        assert!(source.filtered_masks(&settings(operation, 0, 0.0)).is_err());
        assert!(source
            .filtered_masks(&settings(operation, 17, 0.0))
            .is_err());
        assert!(source
            .filtered_masks(&settings(operation, 1, f32::NAN))
            .is_err());
        reject_document(
            with_redo(source),
            settings(operation, 17, 0.0),
            "invalid-noop-settings",
        );
    }
}
