//! Core conversion foundation; normal UI and reference response remain open.
use obrush_core::{Document, Mesh, StaticSubdivisionSettings};

fn fixture() -> Mesh {
    let mut mesh = Mesh::import_obj(include_bytes!("fixtures/independent-dynamic-quad.obj"))
        .unwrap()
        .mesh;
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.triangles.len(), 2);
    assert_eq!(mesh.surface.as_ref().unwrap().faces.len(), 1);
    assert_eq!(
        mesh.surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .face_indices
            .len(),
        1
    );
    mesh.masks = vec![0.0, 0.25, 0.5, 1.0];
    mesh.install_attributes(
        Some(vec![[0.11, 0.22], [0.33, 0.44], [0.55, 0.66], [0.77, 0.88]]),
        mesh.surface.clone(),
    )
    .unwrap();
    mesh
}

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

#[test]
fn dynamic_preparation_preserves_visible_vertices_and_channels_and_is_undoable() {
    let mut document = Document::new(fixture());
    document
        .add_source_static_subdivision_level(&StaticSubdivisionSettings::default())
        .unwrap();
    let before = state(&document);
    let visible = document
        .require_active_mesh()
        .unwrap()
        .try_snapshot()
        .unwrap();
    let uv = visible.surface.as_ref().unwrap().uv.as_ref().unwrap();
    let report = document.prepare_dynamic_topology().unwrap();
    assert_eq!(report.discarded_retained_levels, 1);
    assert_eq!(
        report.discarded_original_faces,
        visible.surface.as_ref().unwrap().faces.len()
    );
    assert_eq!(report.discarded_uv_coordinates, uv.coordinates.len());
    assert_eq!(report.discarded_uv_faces, uv.face_indices.len());
    assert_eq!(
        (report.vertices, report.triangles),
        (visible.positions.len(), visible.triangles.len())
    );
    assert_eq!(document.level_count(), 1);
    assert!(!document.has_retained_subdivision_hierarchy());
    assert!(document.supports_adaptive_detail());
    let converted = document.require_active_mesh().unwrap();
    assert!(converted.surface.is_none());
    assert_eq!(converted.positions, visible.positions);
    assert_eq!(converted.triangles, visible.triangles);
    assert_eq!(converted.colors, visible.colors);
    assert_eq!(converted.masks, visible.masks);
    assert_eq!(converted.materials, visible.materials);
    let after = state(&document);
    assert_eq!(state(&Document::from_json(&after).unwrap()), after);
    assert_eq!(
        document.prepare_dynamic_topology().unwrap(),
        obrush_core::DynamicTopologyPreparationReport {
            discarded_retained_levels: 0,
            discarded_original_faces: 0,
            discarded_uv_coordinates: 0,
            discarded_uv_faces: 0,
            vertices: visible.positions.len(),
            triangles: visible.triangles.len(),
        }
    );
    assert_eq!(state(&document), after);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert_eq!(state(&Document::from_json(&before).unwrap()), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn dynamic_preparation_rejects_bound_target_layer_and_open_stroke() {
    let mut target = Document::new(fixture());
    target.store_morph_target().unwrap();
    target.begin_stroke();
    target.require_active_mesh_mut().unwrap().colors[0] = [0.13, 0.27, 0.41];
    target.end_stroke();
    assert!(target.undo());
    assert!(target.can_redo());
    let before = state(&target);
    assert!(target
        .prepare_dynamic_topology()
        .unwrap_err()
        .contains("Morph Target"));
    assert_eq!(state(&target), before);
    assert!(target.can_redo());

    let mut layered = Document::new(fixture());
    layered.add_layer("Unbaked layer").unwrap();
    let before = state(&layered);
    assert!(layered
        .prepare_dynamic_topology()
        .unwrap_err()
        .contains("Bake"));
    assert_eq!(state(&layered), before);

    let mut stroke = Document::new(fixture());
    stroke.begin_stroke();
    let before = state(&stroke);
    assert!(stroke
        .prepare_dynamic_topology()
        .unwrap_err()
        .contains("Finish"));
    assert_eq!(state(&stroke), before);
    assert!(stroke.cancel_stroke());
}
