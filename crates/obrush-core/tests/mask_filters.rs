use obrush_core::{
    Document, MaskFilterOperation as Op, MaskFilterSettings as Settings, Mesh, Vec3,
};
use std::sync::Arc;

fn source() -> Mesh {
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 0\nf 1 2 3\nf 1 3 4\n").unwrap();
    mesh.masks = vec![0.1, 0.2, 0.8, 1.0, 0.3];
    mesh
}

fn settings(operation: Op, iterations: u32, strength: f32) -> Settings {
    Settings {
        operation,
        iterations,
        strength,
    }
}

fn exact_masks(a: &[f32], b: &[f32]) {
    assert_eq!(
        a.iter().map(|m| m.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|m| m.to_bits()).collect::<Vec<_>>()
    );
}

fn unchanged_channels(before: &Mesh, after: &Mesh) {
    let before = serde_json::to_value(before).unwrap();
    let after = serde_json::to_value(after).unwrap();
    for key in ["positions", "normals", "colors", "triangles"] {
        assert_eq!(before[key], after[key]);
    }
}

#[test]
fn square_filter_values_match_analytic_targets_without_spatial_welding() {
    let source = source();
    let before = serde_json::to_string(&source).unwrap();
    let cases = [
        (Op::Grow, [0.55, 0.5, 0.9, 1.0, 0.3]),
        (Op::Shrink, [0.1, 0.15, 0.45, 0.55, 0.3]),
        (Op::Blur, [0.3125, 17.0 / 60.0, 0.6625, 49.0 / 60.0, 0.3]),
        (Op::Sharpen, [0.0, 0.05, 0.95, 1.0, 0.2]),
    ];
    for (operation, expected) in cases {
        let (actual, report) = source.filtered_masks(&settings(operation, 1, 0.5)).unwrap();
        let maximum = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| (f64::from(*a) - b).abs())
            .fold(0.0, f64::max);
        assert!(maximum <= 1e-7, "{}: {actual:?}", operation.name());
        assert_eq!(report.vertices, 5);
        let changes = actual
            .iter()
            .zip(&source.masks)
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        assert_eq!(report.changed_vertices, changes);
        let max_change = actual
            .iter()
            .zip(&source.masks)
            .map(|(a, b)| (f64::from(*a) - f64::from(*b)).abs())
            .fold(0.0, f64::max);
        assert_eq!(report.max_abs_change, max_change);
        println!(
            "MASK_FILTER_METRIC {}",
            serde_json::json!({"case":"analytic-square","operation":operation,"iterations":1,"strength":0.5,"max_absolute_error":maximum,"gate":1e-7,"actual":actual,"changed_vertices":changes,"max_abs_change":max_change})
        );
    }
    assert_eq!(before, serde_json::to_string(&source).unwrap());
}

#[test]
fn graph_passes_are_synchronous_and_duplicates_are_not_reweighted() {
    let mut source =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 2 0 0\nv 3 0 0\nv 4 0 0\nf 1 2 3\nf 3 4 5\n").unwrap();
    source.masks = vec![1., 0., 0., 0., 0.];
    let (one, _) = source.filtered_masks(&settings(Op::Grow, 1, 1.)).unwrap();
    exact_masks(&one, &[1., 1., 1., 0., 0.]);
    let (two, _) = source.filtered_masks(&settings(Op::Grow, 2, 1.)).unwrap();
    exact_masks(&two, &[1.; 5]);
    source.masks = vec![0., 1., 1., 1., 1.];
    let (one, _) = source.filtered_masks(&settings(Op::Shrink, 1, 1.)).unwrap();
    exact_masks(&one, &[0., 0., 0., 1., 1.]);
    let baseline = source
        .filtered_masks(&settings(Op::Blur, 3, 0.37))
        .unwrap()
        .0;
    source.triangles.extend_from_within(..);
    source.triangles.push([2, 1, 0]);
    let duplicated = source
        .filtered_masks(&settings(Op::Blur, 3, 0.37))
        .unwrap()
        .0;
    exact_masks(&baseline, &duplicated);
}

#[test]
fn fixed_points_signed_zeros_and_rounded_steps_preserve_bits_and_genuine_redo() {
    let mut fixed = source();
    fixed.masks = vec![-0.0, 0.5, 1.0, 0.0, -0.0];
    for operation in Op::ALL {
        let (zero, report) = fixed.filtered_masks(&settings(operation, 16, 0.0)).unwrap();
        exact_masks(&zero, &fixed.masks);
        assert_eq!(report.changed_vertices, 0);
    }
    let (sharpen, report) = fixed
        .filtered_masks(&settings(Op::Sharpen, 16, 1.0))
        .unwrap();
    exact_masks(&sharpen, &fixed.masks);
    assert_eq!(report.changed_vertices, 0);
    fixed.masks.fill(-0.0);
    for operation in [Op::Grow, Op::Shrink, Op::Blur] {
        let (constant, report) = fixed.filtered_masks(&settings(operation, 16, 1.0)).unwrap();
        exact_masks(&constant, &fixed.masks);
        assert_eq!(report.changed_vertices, 0);
    }
    let mut base = Mesh::sphere(1);
    for (i, mask) in base.masks.iter_mut().enumerate() {
        *mask = if i % 2 == 0 { 0.25 } else { 0.75 };
    }
    let mut document = Document::new(base);
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_subtool_name(1, "Genuine redo").unwrap();
    let named = document.to_json().unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    let report = document
        .filter_mask(&settings(Op::Blur, 16, 1e-10))
        .unwrap();
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(report.max_abs_change, 0.0);
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), named);
}

#[test]
fn rich_mask_edit_preserves_geometry_precise_layers_targets_and_inactive_arcs() {
    let mut base = Mesh::sphere(1);
    for (i, mask) in base.masks.iter_mut().enumerate() {
        *mask = if i % 2 == 0 { 0.1 } else { 0.9 };
    }
    let mut document = Document::new(base);
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.store_morph_target().unwrap();
    document.add_layer("Hidden sculpt").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.add_subtool(Mesh::sphere(0), "Inactive").unwrap();
    document.select_subtool(1).unwrap();
    let inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == 2)
        .unwrap()
        .mesh;
    let before = document.to_json().unwrap();
    let previous = document.active_mesh().unwrap().snapshot();
    let value: serde_json::Value = serde_json::from_str(&before).unwrap();
    let report = document.filter_mask(&settings(Op::Blur, 2, 0.5)).unwrap();
    assert!(report.changed_vertices > 0);
    unchanged_channels(&previous, document.active_mesh().unwrap());
    assert_eq!(document.current_level(), 0);
    let after = document.to_json().unwrap();
    let next: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(next["layers"], value["layers"]);
    assert_eq!(next["morph_target"], value["morph_target"]);
    let next_inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == 2)
        .unwrap()
        .mesh;
    assert!(Arc::ptr_eq(&inactive, &next_inactive));
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
fn retained_level_mask_transport_keeps_fine_geometry_and_morph_binding() {
    let mut base = Mesh::sphere(1);
    for (i, mask) in base.masks.iter_mut().enumerate() {
        *mask = if i % 2 == 0 { 0.25 } else { 0.75 };
    }
    let mut document = Document::new(base.snapshot());
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[100] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    let fine = document.active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    let mut old_prediction = base.snapshot();
    old_prediction.subdivide_smooth().unwrap();
    let before = document.to_json().unwrap();
    document.filter_mask(&settings(Op::Grow, 1, 0.4)).unwrap();
    let mut new_prediction = document.active_mesh().unwrap().snapshot();
    new_prediction.subdivide_smooth().unwrap();
    let expected: Vec<_> = fine
        .masks
        .iter()
        .zip(&new_prediction.masks)
        .zip(&old_prediction.masks)
        .map(|((old, new), prediction)| (old + new - prediction).clamp(0.0, 1.0))
        .collect();
    document.select_level(1).unwrap();
    unchanged_channels(&fine, document.active_mesh().unwrap());
    exact_masks(&document.active_mesh().unwrap().masks, &expected);
    assert!(document.has_morph_target());
    assert!(!document.morph_target_summary().unwrap().available);
    document.select_level(0).unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn invalid_settings_channels_hidden_or_pending_edits_are_atomic() {
    let source = source();
    for input in [
        settings(Op::Grow, 0, 1.),
        settings(Op::Blur, 17, 1.),
        settings(Op::Shrink, 1, -0.1),
        settings(Op::Sharpen, 1, f32::NAN),
    ] {
        assert!(source.filtered_masks(&input).is_err());
    }
    let mut duplicate_id = source.snapshot();
    duplicate_id.triangles[0] = [0, 0, 1];
    assert!(duplicate_id.filtered_masks(&Settings::default()).is_err());
    let mut invalid = source.snapshot();
    invalid.normals[0].x = f32::NAN;
    assert!(invalid.filtered_masks(&settings(Op::Blur, 1, 0.)).is_err());
    let mut document = Document::new(source);
    document.set_subtool_name(1, "Redo").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[0] = f32::INFINITY;
    assert!(document.filter_mask(&Settings::default()).is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.take_layer_error().is_some());
    assert!(document.redo());
    document.set_subtool_visible(1, false).unwrap();
    let before = document.to_json().unwrap();
    assert!(document.filter_mask(&Settings::default()).is_err());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn caps_reject_before_graph_scratch() {
    let mut excessive = source();
    excessive.positions.resize(1_000_001, Vec3::ZERO);
    assert!(excessive
        .filtered_masks(&Settings::default())
        .unwrap_err()
        .contains("vertices"));
    let mut excessive = source();
    excessive.triangles.resize(2_000_001, [0, 1, 2]);
    assert!(excessive
        .filtered_masks(&Settings::default())
        .unwrap_err()
        .contains("triangles"));
}
