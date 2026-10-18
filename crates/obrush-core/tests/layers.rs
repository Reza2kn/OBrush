//! Bit-level composition checks for the geometry-only layer foundation.
use obrush_core::{Document, Mesh};

fn signed_zero_sheet() -> Mesh {
    Mesh::from_obj("v -0.0 0.0 0.0\nv 1.0 0.0 -0.0\nv 0.0 1.0 0.0\nf 1 2 3\n").unwrap()
}

fn position_bits(mesh: &Mesh) -> Vec<[u32; 3]> {
    mesh.positions
        .iter()
        .map(|position| position.to_array().map(f32::to_bits))
        .collect()
}

#[test]
fn signed_zero_is_preserved_by_empty_layers_recording_and_blend_endpoints() {
    let mut document = Document::new(signed_zero_sheet());
    let base = position_bits(document.active_mesh().unwrap());
    assert_eq!(base[0][0], (-0.0_f32).to_bits());
    assert_eq!(base[1][2], (-0.0_f32).to_bits());
    let index = document.add_layer("Signed zero").unwrap();
    assert_eq!(position_bits(document.active_mesh().unwrap()), base);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].z = 0.15;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    let recorded = position_bits(document.active_mesh().unwrap());
    assert_eq!(recorded[0][0], base[0][0]);
    assert_eq!(recorded[1][2], base[1][2]);
    document.set_recording_layer(None).unwrap();
    for _ in 0..16 {
        document.set_layer_visibility(index, false).unwrap();
        assert_eq!(position_bits(document.active_mesh().unwrap()), base);
        document.set_layer_visibility(index, true).unwrap();
        document.set_layer_intensity(index, 0.37).unwrap();
        document.set_layer_intensity(index, 1.0).unwrap();
        assert_eq!(position_bits(document.active_mesh().unwrap()), recorded);
    }
    let project = document.to_json().unwrap();
    let loaded = Document::from_json(&project).unwrap();
    assert_eq!(position_bits(loaded.active_mesh().unwrap()), recorded);
    assert_eq!(loaded.to_json().unwrap(), project);
}

#[test]
fn signed_zero_base_edit_and_history_match_the_accepted_mesh_bits() {
    let mut document = Document::new(signed_zero_sheet());
    document.add_layer("Geometry").unwrap();
    document.set_recording_layer(None).unwrap();
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x = 0.0;
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    assert_eq!(
        document.active_mesh().unwrap().positions[0].x.to_bits(),
        0.0_f32.to_bits()
    );
    let after = document.to_json().unwrap();
    assert_ne!(after, before);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    document.begin_stroke();
    document.end_stroke();
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn imported_composition_cannot_disagree_only_in_zero_sign() {
    let mut document = Document::new(signed_zero_sheet());
    document.add_layer("Geometry").unwrap();
    let valid = document.to_json().unwrap();
    let mut project: serde_json::Value = serde_json::from_str(&valid).unwrap();
    project["mesh"]["positions"][0][0] = serde_json::json!(0.0);
    let corrupt = serde_json::to_string(&project).unwrap();
    assert!(Document::from_json(&corrupt).is_err());
    assert_eq!(document.to_json().unwrap(), valid);
}
