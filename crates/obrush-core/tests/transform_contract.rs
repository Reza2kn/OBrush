//! Independent baked whole-object affine contracts, without a shared matrix
//! helper as oracle. These tests do not establish Gizmo/Transpose equivalence.
use obrush_core::{Document, Mesh, ObjectTransform, Vec3};
use serde_json::{json, Value};
use std::sync::Arc;

const POINT_GATE: f64 = 2e-6;
const NORMAL_GATE: f64 = 2e-5;
const LAYER_GATE: f64 = 2e-13;

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn value(document: &Document) -> Value {
    serde_json::from_str(&state(document)).unwrap()
}

fn coordinates(value: &Value) -> Vec<[f64; 3]> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let row = row.as_array().unwrap();
            [
                row[0].as_f64().unwrap(),
                row[1].as_f64().unwrap(),
                row[2].as_f64().unwrap(),
            ]
        })
        .collect()
}

fn coordinates_f32(value: &Value) -> Vec<[f64; 3]> {
    // Mesh and target JSON encode shortest roundtripping f32 decimals. Cast
    // back to the original f32 bits before widening to the f64 oracle; layer
    // base/offset arrays deliberately keep their actual f64 representation.
    coordinates(value)
        .into_iter()
        .map(|row| row.map(|v| f64::from(v as f32)))
        .collect()
}

fn rotate(mut point: [f64; 3], degrees: Vec3) -> [f64; 3] {
    // Final contract is fixed world X, then Y, then Z: Rz * Ry * Rx.
    // This scalar derivation deliberately never calls glam rotation/matrix API.
    let radians = degrees
        .to_array()
        .map(|angle| f64::from(angle) * std::f64::consts::PI / 180.0);
    let (sx, cx) = radians[0].sin_cos();
    point = [
        point[0],
        cx * point[1] - sx * point[2],
        sx * point[1] + cx * point[2],
    ];
    let (sy, cy) = radians[1].sin_cos();
    point = [
        cy * point[0] + sy * point[2],
        point[1],
        -sy * point[0] + cy * point[2],
    ];
    let (sz, cz) = radians[2].sin_cos();
    [
        cz * point[0] - sz * point[1],
        sz * point[0] + cz * point[1],
        point[2],
    ]
}

fn point_oracle(point: [f64; 3], transform: &ObjectTransform) -> [f64; 3] {
    let pivot = transform.pivot.to_array().map(f64::from);
    let translation = transform.translation.to_array().map(f64::from);
    let relative = std::array::from_fn(|i| point[i] - pivot[i]);
    let rotated = rotate(relative, transform.rotation_degrees);
    std::array::from_fn(|i| pivot[i] + f64::from(transform.scale) * rotated[i] + translation[i])
}

fn vector_oracle(vector: [f64; 3], transform: &ObjectTransform) -> [f64; 3] {
    rotate(vector, transform.rotation_degrees).map(|v| v * f64::from(transform.scale))
}

fn max_error(actual: &[[f64; 3]], expected: &[[f64; 3]]) -> f64 {
    assert_eq!(actual.len(), expected.len());
    actual
        .iter()
        .zip(expected)
        .flat_map(|(a, b)| (0..3).map(move |i| (a[i] - b[i]).abs()))
        .fold(0.0, f64::max)
}

fn metric(case: &str, family: &str, maximum: f64, gate: f64) {
    eprintln!(
        "transform_contract_metric: {}",
        json!({"case":case,"family":family,"max_component_absolute_error":maximum,"gate":gate})
    );
    assert!(
        maximum <= gate,
        "{case} {family}: error {maximum} exceeds {gate}"
    );
}

fn check_mesh(before: &Value, after: &Value, transform: &ObjectTransform, case: &str) {
    let expected: Vec<_> = coordinates_f32(&before["positions"])
        .into_iter()
        .map(|p| point_oracle(p, transform))
        .collect();
    metric(
        case,
        "positions_f32_vs_f64_affine",
        max_error(&coordinates_f32(&after["positions"]), &expected),
        POINT_GATE,
    );
    let expected_normals: Vec<_> = coordinates_f32(&before["normals"])
        .into_iter()
        .map(|p| rotate(p, transform.rotation_degrees))
        .collect();
    metric(
        case,
        "recomputed_normals_vs_rotated_original",
        max_error(&coordinates_f32(&after["normals"]), &expected_normals),
        NORMAL_GATE,
    );
    for key in ["triangles", "masks", "colors"] {
        assert_eq!(after[key], before[key], "changed {case} {key}");
    }
}

fn mesh_at(project: &Value, level: usize) -> &Value {
    if project["multiresolution"].is_null()
        || project["multiresolution"]["active_level"].as_u64().unwrap() as usize == level
    {
        &project["mesh"]
    } else {
        &project["multiresolution"]["levels"][level]
    }
}

fn edit(document: &mut Document, index: usize, offset: Vec3) {
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[index] += offset;
    document.active_mesh_mut().unwrap().masks[index] = 0.37;
    document.active_mesh_mut().unwrap().colors[index] = [0.13, 0.29, 0.91];
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn translate_stroke(document: &mut Document, translation: Vec3) {
    document.begin_stroke();
    for point in &mut document.active_mesh_mut().unwrap().positions {
        *point += translation;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn rich() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    edit(&mut document, 11, Vec3::new(0.013, -0.019, 0.007));
    document.add_layer("Hidden offsets").unwrap();
    edit(&mut document, 23, Vec3::new(0.027, -0.018, 0.009));
    document.set_layer_visibility(0, false).unwrap();
    document.add_layer("Faded offsets").unwrap();
    edit(&mut document, 30, Vec3::new(-0.011, 0.017, 0.023));
    document.set_layer_intensity(1, 0.37).unwrap();
    document.add_layer("Recording offsets").unwrap();
    edit(&mut document, 17, Vec3::new(0.021, 0.014, -0.013));
    document
}

fn arbitrary() -> ObjectTransform {
    ObjectTransform {
        translation: Vec3::new(0.27, -0.19, 0.31),
        rotation_degrees: Vec3::new(23.0, -41.0, 17.0),
        scale: 1.7,
        pivot: Vec3::new(0.14, -0.23, 0.09),
    }
}

#[test]
fn identity_is_bit_exact_including_signed_zero_and_does_not_create_history() {
    let mut document = Document::new(Mesh::sphere(0));
    document.active_mesh_mut().unwrap().positions[4].x = -0.0;
    document.active_mesh_mut().unwrap().masks[4] = -0.0;
    document.active_mesh_mut().unwrap().colors[4][1] = -0.0;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.store_morph_target().unwrap();
    let mut document = Document::from_json(&state(&document)).unwrap();
    let before = state(&document);
    assert!(!document
        .transform_active_subtool(&ObjectTransform::default())
        .unwrap());
    let signed_identity = ObjectTransform {
        translation: Vec3::splat(-0.0),
        rotation_degrees: Vec3::splat(-0.0),
        pivot: Vec3::new(0.3, -0.2, 0.1),
        ..ObjectTransform::default()
    };
    assert!(!document.transform_active_subtool(&signed_identity).unwrap());
    assert_eq!(state(&document), before);
    assert_eq!(
        document.active_mesh().unwrap().positions[4].x.to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        document.active_mesh().unwrap().masks[4].to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        document.active_mesh().unwrap().colors[4][1].to_bits(),
        (-0.0_f32).to_bits()
    );
    assert!(!document.undo());
}

#[test]
fn active_center_is_read_only_bounds_center_and_ignores_other_objects() {
    let mut document = Document::new(Mesh::sphere(0));
    let a = document.active_subtool_id().unwrap();
    let mut other = Mesh::sphere(0);
    for point in &mut other.positions {
        *point += Vec3::new(37.0, -19.0, 11.0);
    }
    let b = document.add_subtool(other, "Far object").unwrap();
    let before = state(&document);
    assert!(
        document
            .active_subtool_center()
            .unwrap()
            .distance(Vec3::new(37.0, -19.0, 11.0))
            < 1e-5
    );
    assert_eq!(state(&document), before);
    document.select_subtool(a).unwrap();
    assert!(document.active_subtool_center().unwrap().length() < 1e-6);
    document.select_subtool(b).unwrap();
    assert!(document.undo());
    assert_eq!(document.subtool_count(), 1);
}

#[test]
fn translation_uses_all_vertices_ignores_masks_and_preserves_channels() {
    let mut document = Document::new(Mesh::sphere(1));
    document.active_mesh_mut().unwrap().masks.fill(1.0);
    let before = value(&document);
    let transform = ObjectTransform {
        translation: Vec3::new(0.25, -0.5, 0.75),
        ..ObjectTransform::default()
    };
    assert!(document.transform_active_subtool(&transform).unwrap());
    let after = value(&document);
    check_mesh(&before["mesh"], &after["mesh"], &transform, "translation");
    assert_eq!(after["version"], 2);
    assert!(after.get("scene").is_none());
    assert_eq!(
        after.as_object().unwrap().keys().collect::<Vec<_>>(),
        before.as_object().unwrap().keys().collect::<Vec<_>>()
    );
}

#[test]
fn cardinal_rotations_match_independent_world_axis_order() {
    for rotation in [
        Vec3::new(90.0, 0.0, 0.0),
        Vec3::new(0.0, 90.0, 0.0),
        Vec3::new(0.0, 0.0, 90.0),
        Vec3::new(90.0, 90.0, 90.0),
    ] {
        let mut document = Document::new(Mesh::sphere(0));
        let before = value(&document);
        let transform = ObjectTransform {
            rotation_degrees: rotation,
            ..ObjectTransform::default()
        };
        assert!(document.transform_active_subtool(&transform).unwrap());
        check_mesh(
            &before["mesh"],
            &value(&document)["mesh"],
            &transform,
            &format!("cardinal-{rotation:?}"),
        );
    }
}

#[test]
fn arbitrary_xyz_rotation_translation_scale_and_pivot_match_scalar_oracle() {
    let mut document = Document::new(Mesh::sphere(1));
    edit(&mut document, 17, Vec3::new(0.017, -0.023, 0.011));
    let before = value(&document);
    let transform = arbitrary();
    assert!(document.transform_active_subtool(&transform).unwrap());
    check_mesh(
        &before["mesh"],
        &value(&document)["mesh"],
        &transform,
        "arbitrary-XYZ-pivot-scale",
    );
}

#[test]
fn point_at_pivot_is_fixed_except_for_translation() {
    let mut document = Document::new(Mesh::sphere(0));
    let pivot = document.active_mesh().unwrap().positions[5];
    let transform = ObjectTransform {
        pivot,
        ..arbitrary()
    };
    assert!(document.transform_active_subtool(&transform).unwrap());
    let actual = document.active_mesh().unwrap().positions[5]
        .to_array()
        .map(f64::from);
    let expected = point_oracle(pivot.to_array().map(f64::from), &transform);
    metric(
        "pivot-vertex",
        "position",
        max_error(&[actual], &[expected]),
        POINT_GATE,
    );
}

#[test]
fn all_retained_levels_and_unavailable_origin_target_are_transformed() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    edit(&mut document, 17, Vec3::new(0.021, -0.017, 0.013));
    document.select_level(1).unwrap();
    let before = value(&document);
    assert!(!document.morph_target_summary().unwrap().available);
    let transform = arbitrary();
    assert!(document.transform_active_subtool(&transform).unwrap());
    let after = value(&document);
    assert_eq!((document.current_level(), document.level_count()), (1, 3));
    for level in 0..3 {
        check_mesh(
            mesh_at(&before, level),
            mesh_at(&after, level),
            &transform,
            &format!("retained-level-{level}"),
        );
    }
    let target = coordinates_f32(&before["morph_target"]["positions"])
        .into_iter()
        .map(|p| point_oracle(p, &transform))
        .collect::<Vec<_>>();
    metric(
        "origin-target",
        "positions",
        max_error(
            &coordinates_f32(&after["morph_target"]["positions"]),
            &target,
        ),
        POINT_GATE,
    );
    assert_eq!(after["morph_target"]["creation_level"], 0);
    assert_eq!(
        after["morph_target"]["triangles"],
        before["morph_target"]["triangles"]
    );
    document.select_level(0).unwrap();
    assert!(document.morph_target_summary().unwrap().available);
    assert_eq!(
        state(&Document::from_json(&state(&document)).unwrap()),
        state(&document)
    );
}

#[test]
fn rich_layers_use_affine_base_linear_offsets_and_keep_recording_metadata() {
    let mut document = rich();
    let before = value(&document);
    let summaries = document.layer_summaries();
    let transform = arbitrary();
    assert!(document.transform_active_subtool(&transform).unwrap());
    let after = value(&document);
    assert_eq!(document.layer_summaries(), summaries);
    assert_eq!(document.recording_layer(), Some(2));
    for level in 0..3 {
        check_mesh(
            mesh_at(&before, level),
            mesh_at(&after, level),
            &transform,
            &format!("rich-level-{level}"),
        );
    }
    let expected = coordinates(&before["layers"]["base_positions"])
        .into_iter()
        .map(|p| point_oracle(p, &transform))
        .collect::<Vec<_>>();
    metric(
        "rich-layers",
        "f64-affine-base",
        max_error(&coordinates(&after["layers"]["base_positions"]), &expected),
        LAYER_GATE,
    );
    for (index, (old, new)) in before["layers"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["layers"]["entries"].as_array().unwrap())
        .enumerate()
    {
        for key in ["name", "intensity", "visible"] {
            assert_eq!(old[key], new[key]);
        }
        let expected = coordinates(&old["deltas"])
            .into_iter()
            .map(|p| vector_oracle(p, &transform))
            .collect::<Vec<_>>();
        metric(
            &format!("layer-{index}"),
            "f64-linear-offset",
            max_error(&coordinates(&new["deltas"]), &expected),
            LAYER_GATE,
        );
    }
    assert_eq!(after["layers"]["triangles"], before["layers"]["triangles"]);
    let target = coordinates_f32(&before["morph_target"]["positions"])
        .into_iter()
        .map(|p| point_oracle(p, &transform))
        .collect::<Vec<_>>();
    metric(
        "rich-origin-target",
        "positions",
        max_error(
            &coordinates_f32(&after["morph_target"]["positions"]),
            &target,
        ),
        POINT_GATE,
    );
    // Independent ordered additive composition must reproduce every saved f32
    // visible coordinate exactly; hidden/faded/recording offsets all remain.
    let mut composed = coordinates(&after["layers"]["base_positions"]);
    for entry in after["layers"]["entries"].as_array().unwrap() {
        if !entry["visible"].as_bool().unwrap() {
            continue;
        }
        let weight = entry["intensity"].as_f64().unwrap();
        for (point, delta) in composed.iter_mut().zip(coordinates(&entry["deltas"])) {
            for axis in 0..3 {
                if delta[axis] != 0.0 {
                    point[axis] += weight * delta[axis];
                }
            }
        }
    }
    for (actual, expected) in coordinates(&after["mesh"]["positions"])
        .iter()
        .zip(composed)
    {
        assert_eq!(
            actual.map(|v| (v as f32).to_bits()),
            expected.map(|v| (v as f32).to_bits())
        );
    }
    assert_eq!(
        state(&Document::from_json(&state(&document)).unwrap()),
        state(&document)
    );
}

#[test]
fn revealing_hidden_and_faded_layers_after_transform_matches_transformed_original_shapes() {
    let source = state(&rich());
    for (index, intensity) in [(0, 1.0), (1, 1.0), (2, 0.25)] {
        let mut before = Document::from_json(&source).unwrap();
        before.set_layer_visibility(index, true).unwrap();
        before.set_layer_intensity(index, intensity).unwrap();
        let original = value(&before);
        let mut after = Document::from_json(&source).unwrap();
        let transform = arbitrary();
        after.transform_active_subtool(&transform).unwrap();
        after.set_layer_visibility(index, true).unwrap();
        after.set_layer_intensity(index, intensity).unwrap();
        check_mesh(
            &original["mesh"],
            &value(&after)["mesh"],
            &transform,
            &format!("reveal-layer-{index}"),
        );
    }
}

#[test]
fn active_scene_transform_preserves_inactive_arc_bits_ids_visibility_and_whole_scene_undo() {
    let mut document = rich();
    let a = document.active_subtool_id().unwrap();
    let b = document.add_subtool(Mesh::sphere(0), "Untouched").unwrap();
    document.select_subtool(a).unwrap();
    let before = state(&document);
    let held = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == b)
        .unwrap()
        .mesh;
    let frozen = serde_json::to_string(held.as_ref()).unwrap();
    let summaries = document.subtool_summaries();
    assert!(document.transform_active_subtool(&arbitrary()).unwrap());
    let after = state(&document);
    assert_eq!(document.active_subtool_id().unwrap(), a);
    assert_eq!(document.subtool_summaries(), summaries);
    let inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == b)
        .unwrap()
        .mesh;
    assert!(Arc::ptr_eq(&held, &inactive));
    assert_eq!(serde_json::to_string(inactive.as_ref()).unwrap(), frozen);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
    document.select_subtool(b).unwrap();
    assert_eq!(
        serde_json::to_string(document.active_mesh().unwrap()).unwrap(),
        frozen
    );
}

#[test]
fn v2_v3_v4_v5_formats_keep_versions_and_roundtrip_after_transform() {
    for version in 2..=5 {
        let mut document = Document::new(Mesh::sphere(1));
        if version == 3 {
            document.add_layer("Geometry").unwrap();
            edit(&mut document, 5, Vec3::X * 0.011);
        }
        if version >= 4 {
            document.store_morph_target().unwrap();
        }
        if version == 5 {
            document.set_subtool_name(1, "Explicit scene").unwrap();
        }
        assert_eq!(value(&document)["version"], version);
        assert!(document.transform_active_subtool(&arbitrary()).unwrap());
        let after = state(&document);
        assert_eq!(value(&document)["version"], version);
        assert_eq!(state(&Document::from_json(&after).unwrap()), after);
    }
}

#[test]
fn exact_single_transform_undo_redo_and_identity_preserve_genuine_redo() {
    let mut document = Document::from_json(&state(&rich())).unwrap();
    let before = state(&document);
    assert!(document.transform_active_subtool(&arbitrary()).unwrap());
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(!document
        .transform_active_subtool(&ObjectTransform::default())
        .unwrap());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
    assert!(!document.redo());
    assert!(document.undo());
    assert!(!document.undo());
}

#[test]
fn forward_and_inverse_single_axis_transform_has_bounded_measured_drift() {
    let mut document = Document::new(Mesh::sphere(1));
    let before = value(&document);
    let forward = ObjectTransform {
        rotation_degrees: Vec3::new(0.0, 27.0, 0.0),
        scale: 1.5,
        translation: Vec3::new(0.13, -0.21, 0.17),
        pivot: Vec3::new(-0.2, 0.1, 0.3),
    };
    document.transform_active_subtool(&forward).unwrap();
    let rotated_translation = rotate(
        forward.translation.to_array().map(f64::from),
        -forward.rotation_degrees,
    );
    let inverse = ObjectTransform {
        rotation_degrees: -forward.rotation_degrees,
        scale: 1.0 / forward.scale,
        translation: Vec3::from_array(
            rotated_translation.map(|v| (-v / f64::from(forward.scale)) as f32),
        ),
        pivot: forward.pivot,
    };
    document.transform_active_subtool(&inverse).unwrap();
    metric(
        "forward-inverse",
        "positions",
        max_error(
            &coordinates_f32(&value(&document)["mesh"]["positions"]),
            &coordinates_f32(&before["mesh"]["positions"]),
        ),
        POINT_GATE,
    );
}

fn with_redo(mut document: Document) -> (Document, String, String) {
    document.begin_stroke();
    document.active_mesh_mut().unwrap().colors[0] = [0.17, 0.31, 0.53];
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    let redo = state(&document);
    assert!(document.undo());
    let before = state(&document);
    (document, before, redo)
}

fn reject_preserving_redo(document: Document, transform: &ObjectTransform, label: &str) {
    let (mut document, before, redo) = with_redo(document);
    assert!(
        document.transform_active_subtool(transform).is_err(),
        "accepted {label}"
    );
    assert_eq!(state(&document), before, "changed after rejecting {label}");
    assert!(document.redo(), "lost redo after rejecting {label}");
    assert_eq!(state(&document), redo);
}

#[test]
fn invalid_parameters_are_rejected_atomically_with_redo_intact() {
    let mut invalid = Vec::new();
    for component in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1_000_001.0,
        -1_000_001.0,
    ] {
        invalid.push(ObjectTransform {
            translation: Vec3::new(component, 0.0, 0.0),
            ..ObjectTransform::default()
        });
        invalid.push(ObjectTransform {
            pivot: Vec3::new(0.0, component, 0.0),
            ..ObjectTransform::default()
        });
    }
    for angle in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        36_001.0,
        -36_001.0,
    ] {
        invalid.push(ObjectTransform {
            rotation_degrees: Vec3::new(0.0, 0.0, angle),
            ..ObjectTransform::default()
        });
    }
    for scale in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        0.0,
        0.0009,
        1000.1,
    ] {
        invalid.push(ObjectTransform {
            scale,
            ..ObjectTransform::default()
        });
    }
    assert_eq!(invalid.len(), 22);
    for (index, transform) in invalid.into_iter().enumerate() {
        reject_preserving_redo(
            Document::new(Mesh::sphere(0)),
            &transform,
            &format!("invalid-parameter-{index}"),
        );
    }
}

#[test]
fn documented_parameter_boundaries_are_accepted_when_geometry_stays_healthy() {
    for transform in [
        ObjectTransform {
            scale: 0.001,
            ..ObjectTransform::default()
        },
        ObjectTransform {
            scale: 1000.0,
            ..ObjectTransform::default()
        },
        ObjectTransform {
            rotation_degrees: Vec3::new(36_000.0, -36_000.0, 36_000.0),
            ..ObjectTransform::default()
        },
        ObjectTransform {
            pivot: Vec3::splat(1_000_000.0),
            ..ObjectTransform::default()
        },
    ] {
        let mut document = Document::new(Mesh::sphere(0));
        assert!(document.transform_active_subtool(&transform).is_ok());
        document.active_mesh().unwrap().validate().unwrap();
    }
}

#[test]
fn coordinate_overflow_in_current_geometry_is_atomic() {
    reject_preserving_redo(
        Document::new(Mesh::sphere(0)),
        &ObjectTransform {
            translation: Vec3::X * 1_000_000.0,
            ..ObjectTransform::default()
        },
        "active-coordinate-overflow",
    );
}

#[test]
fn inactive_origin_target_coordinate_overflow_rejects_whole_operation() {
    let mut document = Document::new(Mesh::sphere(0));
    translate_stroke(&mut document, Vec3::X * 999_999.0);
    document.store_morph_target().unwrap();
    translate_stroke(&mut document, -Vec3::X * 999_999.0);
    assert!(document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .all(|p| p.x.abs() < 2.0));
    reject_preserving_redo(
        document,
        &ObjectTransform {
            translation: Vec3::X * 2.0,
            ..ObjectTransform::default()
        },
        "target-coordinate-overflow",
    );
}

#[test]
fn hidden_hierarchy_level_coordinate_overflow_rejects_whole_operation() {
    let mut document = Document::new(Mesh::sphere(0));
    translate_stroke(&mut document, Vec3::X * 999_999.0);
    document.add_subdivision_level().unwrap();
    translate_stroke(&mut document, -Vec3::X * 999_999.0);
    assert_eq!(document.current_level(), 1);
    assert!(document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .all(|p| p.x.abs() < 2.0));
    reject_preserving_redo(
        document,
        &ObjectTransform {
            translation: Vec3::X * 2.0,
            ..ObjectTransform::default()
        },
        "inactive-level-coordinate-overflow",
    );
}

#[test]
fn hidden_layer_offset_and_cancelling_base_overflow_are_not_ignored() {
    let mut hidden = Document::new(Mesh::sphere(0));
    hidden.add_layer("Hidden large translation").unwrap();
    translate_stroke(&mut hidden, Vec3::X * 2001.0);
    hidden.set_layer_visibility(0, false).unwrap();
    assert!(hidden
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .all(|p| p.x.abs() < 2.0));
    reject_preserving_redo(
        hidden,
        &ObjectTransform {
            scale: 1000.0,
            ..ObjectTransform::default()
        },
        "hidden-layer-offset-overflow",
    );
    let mut base = Document::new(Mesh::sphere(0));
    translate_stroke(&mut base, Vec3::X * 999_999.0);
    base.add_layer("Cancelling translation").unwrap();
    translate_stroke(&mut base, -Vec3::X * 999_999.0);
    assert!(base
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .all(|p| p.x.abs() < 2.0));
    reject_preserving_redo(
        base,
        &ObjectTransform {
            translation: Vec3::X * 2.0,
            ..ObjectTransform::default()
        },
        "layer-base-coordinate-overflow",
    );
}

#[test]
fn f32_rounding_collapse_and_orientation_flip_are_rejected_before_commit() {
    let collapsed = ObjectTransform {
        translation: Vec3::splat(999_999.0),
        scale: 0.001,
        ..ObjectTransform::default()
    };
    reject_preserving_redo(
        Document::new(Mesh::sphere(0)),
        &collapsed,
        "f32-collapsed-sphere",
    );
    // Continuous CCW triangle becomes CW after translation and f32 grid
    // rounding at 999999 (grid width .0625). All final coords remain legal.
    let mesh = Mesh::from_obj(
        "v -0.030625 0.030625 0\nv 0.093125 0.031875 0\nv 0.094375 0.093125 0\nf 1 2 3\n",
    )
    .unwrap();
    let transform = ObjectTransform {
        translation: Vec3::new(999_999.0, 999_999.0, 0.0),
        ..ObjectTransform::default()
    };
    let before = mesh
        .positions
        .iter()
        .map(|p| p.to_array().map(f64::from))
        .collect::<Vec<_>>();
    let after = before
        .iter()
        .map(|p| point_oracle(*p, &transform).map(|v| f64::from(v as f32)))
        .collect::<Vec<_>>();
    let area = |points: &[[f64; 3]]| {
        (points[1][0] - points[0][0]) * (points[2][1] - points[0][1])
            - (points[1][1] - points[0][1]) * (points[2][0] - points[0][0])
    };
    let old_area = area(&before);
    let new_area = area(&after);
    eprintln!(
        "transform_contract_metric: {}",
        json!({"case":"rounding-flipped-triangle","signed_double_area_before":old_area,"signed_double_area_after_f32_oracle":new_area})
    );
    assert!(old_area > 0.0 && new_area < 0.0);
    reject_preserving_redo(Document::new(mesh), &transform, "f32-flipped-triangle");
}

#[test]
fn pending_invalid_stroke_aborts_transform_even_identity_and_preserves_redo() {
    for transform in [ObjectTransform::default(), arbitrary()] {
        let (mut document, before, redo) = with_redo(Document::new(Mesh::sphere(0)));
        document.begin_stroke();
        document.active_mesh_mut().unwrap().positions[0].x = f32::NAN;
        assert!(document.transform_active_subtool(&transform).is_err());
        assert_eq!(state(&document), before);
        assert!(document.redo());
        assert_eq!(state(&document), redo);
    }
}
