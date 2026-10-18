//! Independent fixed-topology Morph Target contracts. Neither these fixtures
//! nor familiar controls establish ZBrush Morph Target or brush equivalence.
use obrush_core::{
    BrushKind, BrushSettings, DecimateSettings, Document, Mesh, RasterAlpha, RemeshSettings, Vec3,
};
use serde_json::{json, Value};

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn positions_bits(mesh: &Mesh) -> Vec<[u32; 3]> {
    mesh.positions
        .iter()
        .map(|point| point.to_array().map(f32::to_bits))
        .collect()
}

fn attributes_bits(mesh: &Mesh) -> (Vec<u32>, Vec<[u32; 3]>) {
    (
        mesh.masks.iter().map(|value| value.to_bits()).collect(),
        mesh.colors
            .iter()
            .map(|color| color.map(f32::to_bits))
            .collect(),
    )
}

fn exact_surface(actual: &Mesh, expected: &Mesh) {
    assert_eq!(positions_bits(actual), positions_bits(expected));
    assert_eq!(actual.triangles, expected.triangles);
    assert_eq!(attributes_bits(actual), attributes_bits(expected));
}

fn edit_vertex(document: &mut Document, index: usize, offset: Vec3) {
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[index] += offset;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn translate(document: &mut Document, offset: Vec3) {
    document.begin_stroke();
    for position in &mut document.active_mesh_mut().unwrap().positions {
        *position += offset;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn target(document: &Document) -> Value {
    serde_json::from_str::<Value>(&state(document)).unwrap()["morph_target"].clone()
}

fn sheet() -> Mesh {
    let mut obj = String::new();
    const SIDE: usize = 21;
    for y in 0..SIDE {
        for x in 0..SIDE {
            obj.push_str(&format!(
                "v {} {} 0\n",
                (x as f32 - 10.0) * 0.05,
                (y as f32 - 10.0) * 0.05
            ));
        }
    }
    for y in 0..SIDE - 1 {
        for x in 0..SIDE - 1 {
            let a = y * SIDE + x + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            obj.push_str(&format!("f {a} {b} {c}\nf {b} {d} {c}\n"));
        }
    }
    Mesh::from_obj(&obj).unwrap()
}

fn vertex(mesh: &Mesh, x: f32, y: f32) -> usize {
    mesh.positions
        .iter()
        .position(|p| (p.x - x).abs() < 1e-6 && (p.y - y).abs() < 1e-6)
        .unwrap()
}

fn morph_settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Morph,
        radius: 0.3,
        strength: 0.5,
        symmetry_x: false,
        adaptive_detail: true,
        alpha_tangent: Some(Vec3::X),
        ..BrushSettings::default()
    }
}

fn displaced_sheet() -> Document {
    let mut document = Document::new(sheet());
    document.store_morph_target().unwrap();
    translate(&mut document, Vec3::Z * 0.025);
    document
}

#[test]
fn store_delete_and_versions_are_exact_undoable_whole_document_actions() {
    let mut document = Document::new(Mesh::sphere(1));
    document.active_mesh_mut().unwrap().positions[4].x = -0.0;
    document.active_mesh_mut().unwrap().recompute_normals();
    let before = state(&document);
    let surface = document.active_mesh().unwrap().snapshot();
    document.store_morph_target().unwrap();
    let stored = state(&document);
    let summary = document.morph_target_summary().unwrap();
    assert_eq!(
        (
            summary.creation_level,
            summary.available,
            summary.vertices,
            summary.triangles
        ),
        (0, true, 42, 80)
    );
    exact_surface(document.active_mesh().unwrap(), &surface);
    assert_eq!(
        serde_json::from_str::<Value>(&stored).unwrap()["version"],
        4
    );
    let restored = Document::from_json(&stored).unwrap();
    assert_eq!(state(&restored), stored);
    assert_eq!(
        target(&restored)["positions"][4][0]
            .as_f64()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), stored);
    document.delete_morph_target().unwrap();
    assert!(!document.has_morph_target());
    assert_eq!(state(&document), before);
    assert!(document.undo());
    assert_eq!(state(&document), stored);
}

#[test]
fn swap_twice_restores_exact_geometry_target_and_unlayered_attributes() {
    let mut document = Document::new(Mesh::sphere(1));
    document.active_mesh_mut().unwrap().positions[4].x = -0.0;
    document.active_mesh_mut().unwrap().recompute_normals();
    let base = document.active_mesh().unwrap().snapshot();
    document.store_morph_target().unwrap();
    edit_vertex(&mut document, 5, Vec3::new(0.021, -0.017, 0.033));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[5] = 0.8;
    document.active_mesh_mut().unwrap().colors[5] = [0.13, 0.29, 0.91];
    document.end_stroke();
    let changed = document.active_mesh().unwrap().snapshot();
    let before = state(&document);
    document.swap_morph_target().unwrap();
    assert_eq!(
        positions_bits(document.active_mesh().unwrap()),
        positions_bits(&base)
    );
    assert_eq!(
        attributes_bits(document.active_mesh().unwrap()),
        attributes_bits(&changed)
    );
    let swapped = state(&document);
    document.swap_morph_target().unwrap();
    assert_eq!(state(&document), before);
    assert!(document.undo());
    assert_eq!(state(&document), swapped);
    assert!(document.redo());
    assert_eq!(state(&document), before);
    assert_eq!(state(&Document::from_json(&before).unwrap()), before);
}

#[test]
fn whole_restore_ignores_protection_preserves_attributes_and_keeps_the_target() {
    let mut document = Document::new(Mesh::sphere(0));
    let original = document.active_mesh().unwrap().snapshot();
    document.store_morph_target().unwrap();
    let stored = target(&document);
    translate(&mut document, Vec3::new(0.03, -0.02, 0.01));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks.fill(1.0);
    document.active_mesh_mut().unwrap().colors[3] = [0.2, 0.4, 0.8];
    document.end_stroke();
    let before = state(&document);
    let attributes = attributes_bits(document.active_mesh().unwrap());
    document.restore_morph_target().unwrap();
    assert_eq!(
        positions_bits(document.active_mesh().unwrap()),
        positions_bits(&original)
    );
    assert_eq!(attributes_bits(document.active_mesh().unwrap()), attributes);
    assert_eq!(target(&document), stored);
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn no_op_actions_and_no_effect_brush_preserve_available_redo() {
    let mut document = Document::new(Mesh::sphere(0));
    document.store_morph_target().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().colors[0] = [0.2, 0.3, 0.4];
    document.end_stroke();
    let edited = state(&document);
    assert!(document.undo());
    let before = state(&document);
    document.store_morph_target().unwrap();
    document.restore_morph_target().unwrap();
    document.swap_morph_target().unwrap();
    document.begin_stroke();
    document.end_stroke();
    let result = document
        .apply_morph_brush_with_result(Vec3::Z, Vec3::Z, &morph_settings())
        .unwrap();
    assert_eq!(result.changed_vertices, 0);
    assert_eq!(result.refined_vertices, 0);
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn missing_target_failures_and_empty_delete_preserve_redo() {
    let mut document = Document::new(Mesh::sphere(0));
    translate(&mut document, Vec3::Z * 0.02);
    let edited = state(&document);
    assert!(document.undo());
    let before = state(&document);
    assert!(document.restore_morph_target().is_err());
    assert!(document.swap_morph_target().is_err());
    assert!(document
        .apply_morph_brush_with_result(Vec3::Z, Vec3::Z, &morph_settings())
        .is_err());
    document.delete_morph_target().unwrap();
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn target_stays_bound_to_creation_level_across_navigation_and_new_levels() {
    let mut document = Document::new(Mesh::sphere(1));
    document.add_subdivision_level().unwrap();
    document.store_morph_target().unwrap();
    let stored = target(&document);
    edit_vertex(&mut document, 17, Vec3::new(0.019, -0.011, 0.023));
    document.select_level(0).unwrap();
    let lower = state(&document);
    let summary = document.morph_target_summary().unwrap();
    assert_eq!((summary.creation_level, summary.available), (1, false));
    assert!(document.restore_morph_target().is_err());
    assert!(document.swap_morph_target().is_err());
    assert!(document
        .apply_morph_brush_with_result(Vec3::Z, Vec3::Z, &morph_settings())
        .is_err());
    assert_eq!(state(&document), lower);
    assert_eq!(target(&document), stored);
    document.select_level(1).unwrap();
    assert!(document.morph_target_summary().unwrap().available);
    document.add_subdivision_level().unwrap();
    assert_eq!((document.current_level(), document.level_count()), (2, 3));
    assert!(!document.morph_target_summary().unwrap().available);
    assert_eq!(target(&document), stored);
    let bytes = state(&document);
    let mut restored = Document::from_json(&bytes).unwrap();
    assert_eq!(state(&restored), bytes);
    restored.select_level(1).unwrap();
    assert!(restored.morph_target_summary().unwrap().available);
    restored.restore_morph_target().unwrap();
    assert_eq!(target(&restored), stored);
}

#[test]
fn storing_at_another_level_replaces_one_target_with_exact_undo() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    let before = state(&document);
    assert!(!document.morph_target_summary().unwrap().available);
    document.store_morph_target().unwrap();
    assert_eq!(
        (
            document.morph_target_summary().unwrap().creation_level,
            document.morph_target_summary().unwrap().available
        ),
        (1, true)
    );
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn low_level_restore_propagates_into_fine_shape_without_losing_fine_residual() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    edit_vertex(&mut document, 17, Vec3::new(0.021, -0.017, 0.033));
    let fine = document.active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    translate(&mut document, Vec3::new(0.08, -0.04, 0.06));
    document.restore_morph_target().unwrap();
    document.select_level(1).unwrap();
    assert_eq!(document.active_mesh().unwrap().triangles, fine.triangles);
    assert_eq!(
        attributes_bits(document.active_mesh().unwrap()),
        attributes_bits(&fine)
    );
    for (actual, expected) in document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .zip(&fine.positions)
    {
        assert!((*actual - *expected).length() < 3e-6);
    }
    assert!(document.has_morph_target());
    assert!(!document.morph_target_summary().unwrap().available);
}

#[test]
fn topology_actions_and_raw_refinement_invalidate_with_whole_state_undo() {
    for operation in 0..4 {
        let mut document = Document::new(Mesh::sphere(1));
        document.store_morph_target().unwrap();
        let before = state(&document);
        match operation {
            0 => document.replace_mesh(Mesh::cube()).unwrap(),
            1 => document.subdivide().unwrap(),
            2 => document.subdivide_smooth().unwrap(),
            _ => {
                document.begin_stroke();
                document.active_mesh_mut().unwrap().subdivide().unwrap();
                document.end_stroke();
            }
        }
        assert!(!document.has_morph_target());
        let after = state(&document);
        assert!(document.undo());
        assert_eq!(state(&document), before, "topology operation {operation}");
        assert!(document.redo());
        assert_eq!(state(&document), after);
    }
}

#[test]
fn adaptive_draw_invalidates_target_only_with_an_accepted_topology_change() {
    let mut document = Document::new(Mesh::sphere(3));
    document.store_morph_target().unwrap();
    let before = state(&document);
    document.begin_stroke();
    let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
        Vec3::Z,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            symmetry_x: false,
            adaptive_detail: true,
            ..BrushSettings::default()
        },
    );
    assert!(result.changed_vertices > 0);
    assert!(
        result.refined_vertices > 0,
        "fixture must exercise adaptive topology rather than only geometry"
    );
    document.end_stroke();
    assert!(!document.has_morph_target());
    assert!(document.undo());
    assert_eq!(state(&document), before);
}

#[test]
fn explicit_identical_replacement_invalidates_target_but_failed_and_zero_work_operations_do_not() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    edit_vertex(&mut document, 0, Vec3::new(0.02, 0.01, 0.03));
    assert!(document.undo());
    let before = state(&document);
    assert_eq!(
        document
            .decimate(&DecimateSettings {
                target_ratio: 1.0,
                ..DecimateSettings::default()
            })
            .unwrap()
            .collapses,
        0
    );
    assert!(document
        .decimate(&DecimateSettings {
            target_ratio: f32::NAN,
            ..DecimateSettings::default()
        })
        .is_err());
    assert!(document
        .remesh(&RemeshSettings {
            resolution: 0,
            ..RemeshSettings::default()
        })
        .is_err());
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
    document
        .replace_mesh(document.active_mesh().unwrap().snapshot())
        .unwrap();
    assert!(!document.has_morph_target());
    assert!(!document.can_redo());
    assert!(document.undo());
    assert_eq!(state(&document), before);
}

#[test]
fn centered_morph_has_independent_pressure_oracle_and_fixed_topology() {
    for pressure in [0.25, 1.0] {
        let mut document = displaced_sheet();
        let before = state(&document);
        let stored = target(&document);
        let original = document.active_mesh().unwrap().snapshot();
        let index = vertex(document.active_mesh().unwrap(), 0.0, 0.0);
        let result = document
            .apply_morph_brush_with_result(
                Vec3::Z * 0.025,
                Vec3::Z,
                &BrushSettings {
                    pressure,
                    ..morph_settings()
                },
            )
            .unwrap();
        assert!(result.changed_vertices > 0);
        assert_eq!(result.refined_vertices, 0);
        assert_eq!(
            document.active_mesh().unwrap().triangles,
            original.triangles
        );
        assert_eq!(
            attributes_bits(document.active_mesh().unwrap()),
            attributes_bits(&original)
        );
        let expected = f64::from(original.positions[index].z) * (1.0 - 0.5 * f64::from(pressure));
        assert!(
            (f64::from(document.active_mesh().unwrap().positions[index].z) - expected).abs() < 2e-7
        );
        assert_eq!(target(&document), stored);
        let after = state(&document);
        assert!(document.undo());
        assert_eq!(state(&document), before);
        assert!(document.redo());
        assert_eq!(state(&document), after);
    }
}

#[test]
fn morph_mask_pressure_radius_alpha_and_zero_contact_preserve_protection() {
    let mut masked = displaced_sheet();
    let center = vertex(masked.active_mesh().unwrap(), 0.0, 0.0);
    masked.active_mesh_mut().unwrap().masks[center] = 1.0;
    masked.active_mesh_mut().unwrap().colors[center] = [0.11, 0.29, 0.83];
    let point = masked.active_mesh().unwrap().positions[center];
    let attrs = attributes_bits(masked.active_mesh().unwrap());
    masked
        .apply_morph_brush_with_result(Vec3::Z * 0.025, Vec3::Z, &morph_settings())
        .unwrap();
    assert_eq!(
        masked.active_mesh().unwrap().positions[center]
            .to_array()
            .map(f32::to_bits),
        point.to_array().map(f32::to_bits)
    );
    assert_eq!(attributes_bits(masked.active_mesh().unwrap()), attrs);
    let mut full = displaced_sheet();
    let full_effect = full
        .apply_morph_brush_with_result(Vec3::Z * 0.025, Vec3::Z, &morph_settings())
        .unwrap();
    let mut narrow = displaced_sheet();
    let far = vertex(narrow.active_mesh().unwrap(), 0.2, 0.0);
    let far_point = narrow.active_mesh().unwrap().positions[far];
    let effect = narrow
        .apply_morph_brush_with_result(
            Vec3::Z * 0.025,
            Vec3::Z,
            &BrushSettings {
                pressure: 0.5,
                pressure_strength: false,
                pressure_radius: true,
                ..morph_settings()
            },
        )
        .unwrap();
    assert!(effect.changed_vertices > 0 && effect.changed_vertices < full_effect.changed_vertices);
    assert_eq!(
        narrow.active_mesh().unwrap().positions[far]
            .to_array()
            .map(f32::to_bits),
        far_point.to_array().map(f32::to_bits)
    );
    let mut white = displaced_sheet();
    white
        .apply_morph_brush_with_result(
            Vec3::Z * 0.025,
            Vec3::Z,
            &BrushSettings {
                alpha: Some(RasterAlpha::new(2, 2, vec![255; 4]).unwrap()),
                ..morph_settings()
            },
        )
        .unwrap();
    exact_surface(white.active_mesh().unwrap(), full.active_mesh().unwrap());
    let mut no_op = displaced_sheet();
    no_op.begin_stroke();
    no_op.active_mesh_mut().unwrap().colors[0] = [0.2, 0.4, 0.6];
    no_op.end_stroke();
    assert!(no_op.undo());
    let before = state(&no_op);
    for settings in [
        BrushSettings {
            pressure: 0.0,
            pressure_strength: false,
            ..morph_settings()
        },
        BrushSettings {
            alpha: Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap()),
            ..morph_settings()
        },
    ] {
        let effect = no_op
            .apply_morph_brush_with_result(Vec3::Z * 0.025, Vec3::Z, &settings)
            .unwrap();
        assert_eq!(effect.changed_vertices, 0);
        assert_eq!(state(&no_op), before);
        assert!(no_op.can_redo());
    }
}

#[test]
fn local_symmetry_uses_max_influence_and_asymmetric_alpha_rotation_is_effective() {
    let mut mirrored = displaced_sheet();
    mirrored
        .apply_morph_brush_with_result(
            Vec3::new(0.2, 0.0, 0.025),
            Vec3::Z,
            &BrushSettings {
                radius: 0.15,
                symmetry_x: true,
                ..morph_settings()
            },
        )
        .unwrap();
    let left = vertex(mirrored.active_mesh().unwrap(), -0.2, 0.0);
    let right = vertex(mirrored.active_mesh().unwrap(), 0.2, 0.0);
    assert!(mirrored.active_mesh().unwrap().positions[left].z < 0.024);
    assert!(
        (mirrored.active_mesh().unwrap().positions[left].z
            - mirrored.active_mesh().unwrap().positions[right].z)
            .abs()
            < 2e-7
    );
    let mut one = displaced_sheet();
    let mut overlap = displaced_sheet();
    one.apply_morph_brush_with_result(Vec3::new(0.05, 0.0, 0.025), Vec3::Z, &morph_settings())
        .unwrap();
    overlap
        .apply_morph_brush_with_result(
            Vec3::new(0.05, 0.0, 0.025),
            Vec3::Z,
            &BrushSettings {
                symmetry_x: true,
                ..morph_settings()
            },
        )
        .unwrap();
    let seam = vertex(one.active_mesh().unwrap(), 0.0, 0.0);
    assert_eq!(
        one.active_mesh().unwrap().positions[seam].z.to_bits(),
        overlap.active_mesh().unwrap().positions[seam].z.to_bits()
    );
    let alpha = RasterAlpha::new(2, 2, vec![0, 255, 0, 255]).unwrap();
    let mut stamp = displaced_sheet();
    let mut rotated = displaced_sheet();
    stamp
        .apply_morph_brush_with_result(
            Vec3::Z * 0.025,
            Vec3::Z,
            &BrushSettings {
                alpha: Some(alpha.clone()),
                ..morph_settings()
            },
        )
        .unwrap();
    rotated
        .apply_morph_brush_with_result(
            Vec3::Z * 0.025,
            Vec3::Z,
            &BrushSettings {
                alpha: Some(alpha),
                alpha_rotation: std::f32::consts::PI,
                ..morph_settings()
            },
        )
        .unwrap();
    let a = vertex(stamp.active_mesh().unwrap(), -0.1, 0.0);
    let b = vertex(stamp.active_mesh().unwrap(), 0.1, 0.0);
    assert!(
        stamp.active_mesh().unwrap().positions[b].z < stamp.active_mesh().unwrap().positions[a].z
    );
    assert!(
        rotated.active_mesh().unwrap().positions[a].z
            < rotated.active_mesh().unwrap().positions[b].z
    );
}

#[test]
fn inverted_morph_moves_away_and_generic_mesh_cannot_restore_without_document_target() {
    let mut document = displaced_sheet();
    let stored = target(&document);
    let center = vertex(document.active_mesh().unwrap(), 0.0, 0.0);
    let before = document.active_mesh().unwrap().positions[center];
    document
        .apply_morph_brush_with_result(
            Vec3::Z * 0.025,
            Vec3::Z,
            &BrushSettings {
                invert: true,
                ..morph_settings()
            },
        )
        .unwrap();
    assert!(document.active_mesh().unwrap().positions[center].z > before.z);
    assert_eq!(target(&document), stored);
    let mut mesh = sheet();
    let original = mesh.snapshot();
    assert_eq!(
        mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &morph_settings())
            .changed_vertices,
        0
    );
    exact_surface(&mesh, &original);
}

#[test]
fn restore_and_swap_reconcile_recorded_layers_preserving_full_history() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    let base = document.active_mesh().unwrap().snapshot();
    let stored = target(&document);
    document.add_layer("Morph interaction").unwrap();
    edit_vertex(&mut document, 5, Vec3::new(0.021, -0.017, 0.033));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[5] = 0.8;
    document.active_mesh_mut().unwrap().colors[5] = [0.13, 0.29, 0.91];
    document.end_stroke();
    let before = state(&document);
    let attrs = attributes_bits(document.active_mesh().unwrap());
    document.restore_morph_target().unwrap();
    assert_eq!(
        positions_bits(document.active_mesh().unwrap()),
        positions_bits(&base)
    );
    assert_eq!(attributes_bits(document.active_mesh().unwrap()), attrs);
    assert_eq!(target(&document), stored);
    assert_eq!(document.recording_layer(), Some(0));
    assert!(document.undo());
    assert_eq!(state(&document), before);
    document.swap_morph_target().unwrap();
    document.swap_morph_target().unwrap();
    assert_eq!(state(&document), before);
    assert_eq!(state(&Document::from_json(&before).unwrap()), before);
}

#[test]
fn local_morph_participates_in_one_open_stroke_and_captures_layer_edits() {
    let mut document = displaced_sheet();
    document.add_layer("Local restoration").unwrap();
    let stored = target(&document);
    let before = state(&document);
    document.begin_stroke();
    for x in [-0.1, 0.0, 0.1] {
        let result = document
            .apply_morph_brush_with_result(Vec3::new(x, 0.0, 0.025), Vec3::Z, &morph_settings())
            .unwrap();
        assert!(result.changed_vertices > 0);
        assert_eq!(result.refined_vertices, 0);
    }
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    assert!(document.layer_summaries()[0].has_geometry);
    assert_eq!(target(&document), stored);
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
    document.set_layer_visibility(0, false).unwrap();
    assert_eq!(
        positions_bits(document.active_mesh().unwrap()),
        positions_bits(Document::from_json(&before).unwrap().active_mesh().unwrap())
    );
}

#[test]
fn store_finishes_pending_valid_stroke_before_its_own_target_history() {
    let mut document = Document::new(Mesh::sphere(0));
    document.store_morph_target().unwrap();
    let original = state(&document);
    let old_target = target(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] += Vec3::new(0.021, -0.017, 0.033);
    document.active_mesh_mut().unwrap().recompute_normals();
    let old_target_new_geometry = state(&document);
    document.store_morph_target().unwrap();
    let stored_new = state(&document);
    assert_ne!(target(&document), old_target);
    assert!(document.undo());
    assert_eq!(state(&document), old_target_new_geometry);
    assert!(document.undo());
    assert_eq!(state(&document), original);
    assert!(document.redo());
    assert!(document.redo());
    assert_eq!(state(&document), stored_new);
}

#[test]
fn whole_document_import_preserves_target_layers_levels_and_receiving_history() {
    let mut existing = Document::new(Mesh::cube());
    existing.store_morph_target().unwrap();
    translate(&mut existing, Vec3::new(0.02, 0.03, 0.01));
    assert!(existing.undo());
    let before = state(&existing);
    let mut incoming = Document::new(Mesh::sphere(1));
    incoming.name = "Imported morph hierarchy".into();
    incoming.add_subdivision_level().unwrap();
    incoming.store_morph_target().unwrap();
    incoming.add_layer("Imported target edits").unwrap();
    edit_vertex(&mut incoming, 17, Vec3::new(0.017, -0.019, 0.031));
    let imported = state(&incoming);
    assert_eq!(
        serde_json::from_str::<Value>(&imported).unwrap()["version"],
        4
    );
    let decoded = Document::from_json(&imported).unwrap();
    assert!(!decoded.can_undo());
    existing.replace_document(decoded).unwrap();
    assert_eq!(state(&existing), imported);
    assert!(existing.undo());
    assert_eq!(state(&existing), before);
    assert!(existing.redo());
    assert_eq!(state(&existing), imported);
}

#[test]
fn malformed_target_versions_dimensions_coordinates_topology_and_level_are_rejected() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.add_subdivision_level().unwrap();
    let source: Value = serde_json::from_str(&state(&document)).unwrap();
    for corruption in 0..13 {
        let mut payload = source.clone();
        match corruption {
            0..=2 => payload["version"] = json!(corruption + 1),
            3 => {
                payload.as_object_mut().unwrap().remove("morph_target");
            }
            4 => payload["morph_target"] = Value::Null,
            5 => payload["morph_target"]["scheme"] = json!("unknown_scheme"),
            6 => {
                payload["morph_target"]["positions"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            7 => payload["morph_target"]["positions"][0] = json!([1e9, 0.0, 0.0]),
            8 => payload["morph_target"]["positions"][0] = json!(["NaN", 0.0, 0.0]),
            9 => payload["morph_target"]["triangles"][0] = json!([0, 1, 2]),
            10 => payload["morph_target"]["creation_level"] = json!(99),
            11 => payload["morph_target"]["creation_level"] = json!(1),
            _ => payload["morph_target"]["extra"] = json!(true),
        }
        assert!(
            Document::from_json(&payload.to_string()).is_err(),
            "morph corruption {corruption} accepted"
        );
    }
    let metadata = source["morph_target"].to_string();
    let duplicate = format!("{{\"version\":4,\"name\":\"Duplicate target field\",\"mesh\":{},\"multiresolution\":{},\"morph_target\":{{\"scheme\":\"positions_f32_v1\",{}}}}}", source["mesh"], source["multiresolution"], &metadata[1..metadata.len()-1]);
    assert!(Document::from_json(&duplicate)
        .unwrap_err()
        .contains("duplicate"));
}

#[test]
fn target_decode_enforces_point_allocation_limit_before_shape_binding() {
    let mesh = serde_json::to_string(&Mesh::sphere(0)).unwrap();
    let positions = format!("[{}[0,0,0]]", "[0,0,0],".repeat(1_000_000));
    let bytes = format!("{{\"version\":4,\"name\":\"Bounded target decoder\",\"mesh\":{mesh},\"morph_target\":{{\"scheme\":\"positions_f32_v1\",\"creation_level\":0,\"positions\":{positions},\"triangles\":[[0,1,2]]}}}}");
    let error = Document::from_json(&bytes).unwrap_err().to_lowercase();
    assert!(
        error.contains("limit")
            || error.contains("exceed")
            || error.contains("allocation")
            || error.contains("many"),
        "target oversized field reached ordinary shape binding: {error}"
    );
}
