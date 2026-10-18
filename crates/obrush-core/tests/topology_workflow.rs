use obrush_core::{
    execute_topology_project, DecimateSettings, Document, Mesh, RemeshSettings, TopologyOperation,
};

#[test]
fn topology_changes_are_single_undo_steps_and_json_roundtrips() {
    for remesh in [false, true] {
        let mut doc = Document::new(Mesh::sphere(3));
        let before = doc.to_json().unwrap();
        if remesh {
            doc.remesh(&RemeshSettings {
                resolution: 24,
                ..Default::default()
            })
            .unwrap();
        } else {
            doc.decimate(&DecimateSettings {
                max_error: 0.2,
                ..Default::default()
            })
            .unwrap();
        }
        let after = doc.to_json().unwrap();
        assert_ne!(before, after);
        assert_eq!(
            Document::from_json(&after).unwrap().to_json().unwrap(),
            after
        );
        assert!(doc.undo());
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(!doc.undo());
        assert!(doc.redo());
        assert_eq!(doc.to_json().unwrap(), after);
    }
}

#[test]
fn failed_topology_settings_preserve_mesh_and_redo() {
    let mut doc = Document::new(Mesh::sphere(2));
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().masks[0] = 0.5;
    doc.end_stroke();
    assert!(doc.undo());
    let before = doc.to_json().unwrap();
    assert!(doc
        .remesh(&RemeshSettings {
            resolution: 0,
            ..Default::default()
        })
        .is_err());
    assert!(doc
        .decimate(&DecimateSettings {
            target_ratio: f32::NAN,
            ..Default::default()
        })
        .is_err());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(doc.can_redo());
    assert!(doc.redo());
    assert_eq!(doc.active_mesh().unwrap().masks[0], 0.5);
}

#[test]
fn worker_protocol_uses_validated_projects_and_reports() {
    let doc = Document::new(Mesh::sphere(3));
    let project = doc.to_json().unwrap();
    let operation = serde_json::to_string(&TopologyOperation::Decimate(DecimateSettings {
        max_error: 0.2,
        ..Default::default()
    }))
    .unwrap();
    let result = execute_topology_project(&project, &operation).unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let output = Document::from_json(value["project"].as_str().unwrap()).unwrap();
    assert!(
        output.active_mesh().unwrap().triangles.len() < doc.active_mesh().unwrap().triangles.len()
    );
    assert_eq!(value["report"]["kind"], "decimate");
    assert!(execute_topology_project("{}", &operation).is_err());
    assert!(execute_topology_project(&project, "{\"kind\":\"unknown\"}").is_err());
    assert_eq!(doc.to_json().unwrap(), project);
}
