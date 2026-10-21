//! Independent meaningful-motion gate for repeated tiny Move contacts.
//!
//! This supplements, and does not alter, `worked_surface_stress`. Its first
//! baseline is expected to fail. A contact must move an original surface
//! vertex by at least 0.001 world units (about 2.7% of the 0.037 brush radius),
//! not merely change one floating-point word. This is a local usability gate,
//! not a reference-product or rendered-appearance tolerance.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use serde_json::json;
use std::path::PathBuf;

const MIN_USEFUL_DELTA: f32 = 0.001;
const MIN_USEFUL_CONTACTS: usize = 8;

#[test]
#[ignore = "known failure: repeated tiny Move contacts stop making useful progress"]
fn repeated_tiny_move_contacts_remain_meaningful() {
    let settings = BrushSettings {
        kind: BrushKind::Move,
        radius: 0.037,
        strength: 0.79,
        symmetry_x: true,
        adaptive_detail: true,
        ..BrushSettings::default()
    };
    let mut document = Document::new(Mesh::sphere(6));
    let original_count = document.active_mesh().unwrap().positions.len();
    let mut deltas = Vec::new();
    let mut history = vec![document.active_mesh().unwrap().snapshot()];
    for _ in 0..12 {
        let before = document.active_mesh().unwrap().snapshot();
        let hit = before
            .raycast(Ray {
                origin: Vec3::new(0.23, 0.09, 3.0),
                direction: -Vec3::Z,
            })
            .unwrap();
        let mut moving = before
            .prepare_move_stroke(hit.position, hit.normal, &settings)
            .unwrap();
        document.begin_stroke();
        for sample in 1..=6 {
            moving.apply(
                document.active_mesh_mut().unwrap(),
                Vec3::X * (0.018 * sample as f32 / 6.0),
                &settings,
            );
        }
        document.end_stroke();
        let after = document.active_mesh().unwrap();
        after.validate().unwrap();
        let max_original_delta = before.positions[..original_count]
            .iter()
            .zip(&after.positions[..original_count])
            .map(|(&a, &b)| a.distance(b))
            .fold(0.0_f32, f32::max);
        deltas.push(max_original_delta);
        if after.positions != before.positions || after.triangles != before.triangles {
            history.push(after.snapshot());
        }
    }
    let final_mesh = document.active_mesh().unwrap().snapshot();
    for expected in history.iter().rev().skip(1) {
        assert!(document.undo());
        assert_eq!(
            document.active_mesh().unwrap().positions,
            expected.positions
        );
        assert_eq!(
            document.active_mesh().unwrap().triangles,
            expected.triangles
        );
    }
    assert!(!document.can_undo());
    for expected in history.iter().skip(1) {
        assert!(document.redo());
        assert_eq!(
            document.active_mesh().unwrap().positions,
            expected.positions
        );
        assert_eq!(
            document.active_mesh().unwrap().triangles,
            expected.triangles
        );
    }
    assert_eq!(
        document.active_mesh().unwrap().positions,
        final_mesh.positions
    );
    assert_eq!(
        document.active_mesh().unwrap().triangles,
        final_mesh.triangles
    );
    let useful = deltas
        .iter()
        .filter(|&&delta| delta >= MIN_USEFUL_DELTA)
        .count();
    let report = json!({
        "fixture":"independent sphere(6), 12 separate Move contacts",
        "settings":{"radius":0.037,"strength":0.79,"x_symmetry":true,"adaptive_detail":true,"target_delta":[0.018,0.0,0.0],"samples_per_contact":6},
        "minimum_useful_delta":MIN_USEFUL_DELTA,
        "minimum_useful_contacts":MIN_USEFUL_CONTACTS,
        "actual_useful_contacts":useful,
        "max_original_vertex_delta_per_contact":deltas,
        "exact_position_triangle_history":true,
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.artifacts/move-contact-usefulness.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    eprintln!("Move usefulness: {useful}/12; report at {}", path.display());
    assert!(useful >= MIN_USEFUL_CONTACTS);
}
