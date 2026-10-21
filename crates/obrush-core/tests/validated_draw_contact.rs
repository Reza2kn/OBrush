//! Independent ordinary-contact equivalence and rejection checks.

use obrush_core::{
    BrushKind, BrushSettings, Document, Mesh, Ray, SceneRay64, SglPickingPolicy, Vec3,
};

fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..BrushSettings::default()
    }
}

fn ray() -> SceneRay64 {
    Ray {
        origin: Vec3::new(0.04, 0.02, 3.0),
        direction: -Vec3::Z,
    }
    .into()
}

fn compare_contact_with_ordinary_route(mesh: Mesh) {
    let mut contact_doc = Document::new(mesh.snapshot());
    let mut ordinary_doc = Document::new(mesh);
    let original = contact_doc.to_json().unwrap();
    assert_eq!(original, ordinary_doc.to_json().unwrap());
    let brush = settings();

    for _ in 0..2 {
        contact_doc.begin_stroke();
        ordinary_doc.begin_stroke();
        let picked = ordinary_doc
            .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap();
        let (hit, _) = picked.local_brush_hit().unwrap();
        let ordinary_result = ordinary_doc
            .require_active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &brush);
        let contact = contact_doc
            .pick_active_draw_contact(ray(), SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap();
        let retained = contact.hit();
        assert_eq!(retained.position, hit.position);
        assert_eq!(retained.normal, hit.normal);
        assert_eq!(retained.triangle, hit.triangle);
        assert_eq!(retained.barycentric, hit.barycentric);
        let contact_result = contact.apply(&brush).unwrap();
        assert_eq!(contact_result, ordinary_result);
        contact_doc.end_stroke();
        ordinary_doc.end_stroke();
        assert_eq!(
            contact_doc.to_json().unwrap(),
            ordinary_doc.to_json().unwrap()
        );
    }

    let edited = contact_doc.to_json().unwrap();
    assert_ne!(edited, original, "the fixture must exercise a real edit");
    assert!(contact_doc.undo());
    assert!(ordinary_doc.undo());
    assert_eq!(
        contact_doc.to_json().unwrap(),
        ordinary_doc.to_json().unwrap()
    );
    assert!(contact_doc.undo());
    assert!(ordinary_doc.undo());
    assert_eq!(contact_doc.to_json().unwrap(), original);
    assert_eq!(ordinary_doc.to_json().unwrap(), original);
    for _ in 0..2 {
        assert!(contact_doc.redo());
        assert!(ordinary_doc.redo());
        assert_eq!(
            contact_doc.to_json().unwrap(),
            ordinary_doc.to_json().unwrap()
        );
    }
    assert_eq!(contact_doc.to_json().unwrap(), edited);
}

#[test]
fn validated_contact_matches_ordinary_fallback_and_history() {
    compare_contact_with_ordinary_route(Mesh::sphere(4));
}

#[test]
fn validated_contact_matches_large_draw_window_and_history() {
    // The window only activates above 100k vertices; level seven has 163,842.
    let mesh = Mesh::sphere(7);
    assert!(mesh.positions.len() > 100_000);
    compare_contact_with_ordinary_route(mesh);
}

#[test]
fn invalid_source_is_rejected_before_mutation_and_wrong_tool_cannot_consume() {
    let mut doc = Document::new(Mesh::sphere(4));
    let before = doc.to_json().unwrap();
    let contact = doc
        .pick_active_draw_contact(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    let mut wrong = settings();
    wrong.kind = BrushKind::Clay;
    assert!(contact.apply(&wrong).is_err());
    assert_eq!(doc.to_json().unwrap(), before);

    doc.require_active_mesh_mut().unwrap().normals.pop();
    let damaged = doc.require_active_mesh().unwrap().snapshot();
    let expected_error = doc
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap_err();
    let actual_error = doc
        .pick_active_draw_contact(ray(), SglPickingPolicy::WorldDistanceV1)
        .err()
        .expect("invalid source must fail before yielding a contact");
    assert_eq!(actual_error, expected_error);
    assert_eq!(
        doc.require_active_mesh().unwrap().positions,
        damaged.positions
    );
    assert_eq!(
        doc.require_active_mesh().unwrap().triangles,
        damaged.triangles
    );
    assert_eq!(doc.require_active_mesh().unwrap().normals, damaged.normals);
}
