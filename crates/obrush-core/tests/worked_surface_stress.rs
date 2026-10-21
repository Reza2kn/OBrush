//! Diagnostic for surface quality after overlapping Crease and repeated Move.
//!
//! This is an independently generated shared-core fixture, not a rendered
//! comparison or a SculptGL/ZBrush parity test. The geometric thresholds
//! below were frozen before the first baseline run. That run passed geometry
//! but exposed ten no-op Move contacts; its v1 receipt is retained separately.
//! The eight-of-twelve usefulness gate was then added before any product edit.
//! Run explicitly with
//! `cargo test --release -p obrush-core --test worked_surface_stress -- --ignored --nocapture`.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

const DESIRED_MIN_STANDARD_QUALITY: f64 = 0.15;
const DESIRED_MAX_FOLD_DEGREES: f64 = 45.0;
const DESIRED_MAX_ORIGINAL_EDGE_STRETCH: f64 = 2.0;
const DESIRED_EFFECTIVE_MOVE_CONTACTS: usize = 8;

fn settings(kind: BrushKind, radius: f32, strength: f32) -> BrushSettings {
    BrushSettings {
        kind,
        radius,
        strength,
        symmetry_x: true,
        adaptive_detail: true,
        ..BrushSettings::default()
    }
}

fn picked(mesh: &Mesh, x: f32, y: f32) -> obrush_core::Hit {
    mesh.raycast(Ray {
        origin: Vec3::new(x, y, 3.0),
        direction: -Vec3::Z,
    })
    .expect("fixture ray reaches the current sphere")
}

fn state_hash(mesh: &Mesh) -> String {
    let mut h = blake3::Hasher::new();
    for position in &mesh.positions {
        for coordinate in position.to_array() {
            h.update(&coordinate.to_bits().to_le_bytes());
        }
    }
    for triangle in &mesh.triangles {
        for vertex in triangle {
            h.update(&vertex.to_le_bytes());
        }
    }
    for normal in &mesh.normals {
        for coordinate in normal.to_array() {
            h.update(&coordinate.to_bits().to_le_bytes());
        }
    }
    for mask in &mesh.masks {
        h.update(&mask.to_bits().to_le_bytes());
    }
    for color in &mesh.colors {
        for channel in color {
            h.update(&channel.to_bits().to_le_bytes());
        }
    }
    h.finalize().to_hex().to_string()
}

fn original_edges(mesh: &Mesh) -> HashMap<(u32, u32), f64> {
    let mut result = HashMap::new();
    for &[a, b, c] in &mesh.triangles {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            result.entry((a.min(b), a.max(b))).or_insert_with(|| {
                f64::from(mesh.positions[a as usize].distance(mesh.positions[b as usize]))
            });
        }
    }
    result
}

fn metrics(mesh: &Mesh, baseline_edges: &HashMap<(u32, u32), f64>) -> Value {
    mesh.validate().expect("finite, valid shared-core mesh");
    let mut minimum_standard_quality = f64::INFINITY;
    let mut minimum_guard_quality = f64::INFINITY;
    let mut maximum_fold_degrees = 0.0_f64;
    let mut folds_over_30 = 0_usize;
    let mut folds_over_60 = 0_usize;
    let mut folds_over_90 = 0_usize;
    let mut maximum_original_edge_stretch = 1.0_f64;
    let mut retained_original_edges = 0_usize;
    let mut edges: HashMap<(u32, u32), (bool, glam::DVec3, u8)> = HashMap::new();
    for &[a, b, c] in &mesh.triangles {
        let [p, q, r] = [a, b, c].map(|id| mesh.positions[id as usize].as_dvec3());
        let cross = (q - p).cross(r - p);
        let area2 = cross.length();
        let ab = p.distance_squared(q);
        let bc = q.distance_squared(r);
        let ca = r.distance_squared(p);
        assert!(area2 > 0.0 && area2.is_finite());
        minimum_standard_quality =
            minimum_standard_quality.min(2.0 * 3.0_f64.sqrt() * area2 / (ab + bc + ca));
        minimum_guard_quality = minimum_guard_quality.min(area2 / ab.max(bc).max(ca));
        let normal = cross / area2;
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let edge = (u.min(v), u.max(v));
            if let Some((forward, other, count)) = edges.get_mut(&edge) {
                assert_eq!(*count, 1, "nonmanifold edge has over two faces");
                assert_ne!(*forward, u < v, "consistent shared-edge winding");
                let fold = normal.dot(*other).clamp(-1.0, 1.0).acos().to_degrees();
                maximum_fold_degrees = maximum_fold_degrees.max(fold);
                folds_over_30 += usize::from(fold > 30.0);
                folds_over_60 += usize::from(fold > 60.0);
                folds_over_90 += usize::from(fold > 90.0);
                *count = 2;
            } else {
                edges.insert(edge, (u < v, normal, 1));
                if let Some(original) = baseline_edges.get(&edge) {
                    retained_original_edges += 1;
                    let length =
                        f64::from(mesh.positions[u as usize].distance(mesh.positions[v as usize]));
                    maximum_original_edge_stretch =
                        maximum_original_edge_stretch.max(length / original);
                }
            }
        }
    }
    // The sphere and its adaptive children should stay closed and manifold.
    assert!(edges.values().all(|edge| edge.2 == 2));
    json!({
        "vertices": mesh.positions.len(),
        "triangles": mesh.triangles.len(),
        "state_hash": state_hash(mesh),
        "minimum_standard_quality": minimum_standard_quality,
        "minimum_guard_quality": minimum_guard_quality,
        "maximum_fold_degrees": maximum_fold_degrees,
        "folds_over_30": folds_over_30,
        "folds_over_60": folds_over_60,
        "folds_over_90": folds_over_90,
        "retained_original_edges": retained_original_edges,
        "maximum_original_edge_stretch": maximum_original_edge_stretch,
    })
}

fn exact(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.masks, b.masks);
    assert_eq!(a.colors, b.colors);
}

fn check_history(document: &mut Document, history: &[Mesh]) {
    for expected in history.iter().rev().skip(1) {
        assert!(document.undo(), "each changed contact has one Undo step");
        exact(document.active_mesh().unwrap(), expected);
    }
    assert!(
        !document.can_undo(),
        "no extra Undo step for a no-op contact"
    );
    for expected in history.iter().skip(1) {
        assert!(document.redo(), "each changed contact has one Redo step");
        exact(document.active_mesh().unwrap(), expected);
    }
    assert!(!document.can_redo());
}

fn run_crease() -> Value {
    let mut document = Document::new(Mesh::sphere(6));
    let initial = document.active_mesh().unwrap().snapshot();
    let initial_edges = original_edges(&initial);
    let settings = settings(BrushKind::Crease, 0.18, 0.35);
    let mut history = vec![initial];
    let mut stages = vec![metrics(document.active_mesh().unwrap(), &initial_edges)];
    let mut contacts = Vec::new();
    for stroke in 0..8 {
        document.begin_stroke();
        let mut changed = 0_usize;
        let mut refined = 0_usize;
        let mut detail_limited = false;
        for step in 0..3 {
            let x = 0.18 + 0.018 * step as f32;
            let y = 0.085 + 0.004 * (stroke % 2) as f32;
            let hit = picked(document.active_mesh().unwrap(), x, y);
            let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings,
            );
            changed += result.changed_vertices;
            refined += result.refined_vertices;
            detail_limited |= result.detail_limited;
        }
        document.end_stroke();
        let mesh = document.active_mesh().unwrap();
        let stage = metrics(mesh, &initial_edges);
        if state_hash(mesh) != state_hash(history.last().unwrap()) {
            history.push(mesh.snapshot());
        }
        contacts.push(json!({"stroke":stroke+1,"changed_vertex_sum":changed,"refined_vertex_sum":refined,"detail_limited":detail_limited,"metrics":stage}));
        stages.push(stage);
    }
    let expected = document.active_mesh().unwrap().snapshot();
    check_history(&mut document, &history);
    exact(document.active_mesh().unwrap(), &expected);
    json!({"tool":"Crease","settings":{"radius":0.18,"strength":0.35,"x_symmetry":true,"adaptive_detail":true,"pressure":1.0},"effective_history_steps":history.len()-1,"initial":stages.first().unwrap(),"final":stages.last().unwrap(),"contacts":contacts,"history_exact":true})
}

fn run_move() -> Value {
    let mut document = Document::new(Mesh::sphere(6));
    let initial = document.active_mesh().unwrap().snapshot();
    let initial_edges = original_edges(&initial);
    let settings = settings(BrushKind::Move, 0.037, 0.79);
    let mut history = vec![initial];
    let initial_metrics = metrics(document.active_mesh().unwrap(), &initial_edges);
    let mut contacts = Vec::new();
    for contact in 0..12 {
        let hit = picked(document.active_mesh().unwrap(), 0.23, 0.09);
        let mut moving = document
            .active_mesh()
            .unwrap()
            .prepare_move_stroke(hit.position, hit.normal, &settings)
            .unwrap();
        document.begin_stroke();
        let mut changed = 0_usize;
        let mut refined = 0_usize;
        let mut detail_limited = false;
        for sample in 1..=6 {
            let target = Vec3::X * (0.018 * sample as f32 / 6.0);
            let result = moving.apply(document.active_mesh_mut().unwrap(), target, &settings);
            changed += result.changed_vertices;
            refined += result.refined_vertices;
            detail_limited |= result.detail_limited;
        }
        document.end_stroke();
        let mesh = document.active_mesh().unwrap();
        let stage = metrics(mesh, &initial_edges);
        if state_hash(mesh) != state_hash(history.last().unwrap()) {
            history.push(mesh.snapshot());
        }
        contacts.push(json!({"contact":contact+1,"changed_vertex_sum":changed,"refined_vertex_sum":refined,"detail_limited":detail_limited,"metrics":stage}));
    }
    let expected = document.active_mesh().unwrap().snapshot();
    let final_metrics = metrics(&expected, &initial_edges);
    check_history(&mut document, &history);
    exact(document.active_mesh().unwrap(), &expected);
    json!({"tool":"Move","settings":{"radius":0.037,"strength":0.79,"x_symmetry":true,"adaptive_detail":true,"pressure":1.0,"target_delta":[0.018,0.0,0.0],"samples_per_contact":6},"effective_history_steps":history.len()-1,"initial":initial_metrics,"final":final_metrics,"contacts":contacts,"history_exact":true})
}

fn desired(result: &Value) -> bool {
    let final_metrics = &result["final"];
    final_metrics["minimum_standard_quality"].as_f64().unwrap() >= DESIRED_MIN_STANDARD_QUALITY
        && final_metrics["maximum_fold_degrees"].as_f64().unwrap() <= DESIRED_MAX_FOLD_DEGREES
        && final_metrics["maximum_original_edge_stretch"]
            .as_f64()
            .unwrap()
            <= DESIRED_MAX_ORIGINAL_EDGE_STRETCH
        && (result["tool"] != "Move"
            || result["effective_history_steps"].as_u64().unwrap()
                >= DESIRED_EFFECTIVE_MOVE_CONTACTS as u64)
}

#[test]
#[ignore = "diagnostic quality gate: retain baseline measurements and failures"]
fn overlapping_crease_and_repeated_small_move_keep_worked_surfaces_usable() {
    let crease = run_crease();
    let moving = run_move();
    let crease_pass = desired(&crease);
    let move_pass = desired(&moving);
    let report = json!({
        "fixture":"independent sphere(6), separate fresh documents per tool",
        "scope":"shared core geometry and history only; no rendered visual or responsiveness verdict",
        "desired_gate":{"minimum_standard_quality":DESIRED_MIN_STANDARD_QUALITY,"maximum_fold_degrees":DESIRED_MAX_FOLD_DEGREES,"maximum_original_edge_stretch":DESIRED_MAX_ORIGINAL_EDGE_STRETCH,"minimum_effective_move_contacts":DESIRED_EFFECTIVE_MOVE_CONTACTS},
        "crease_pass":crease_pass,"move_pass":move_pass,"crease":crease,"move":moving,
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.artifacts/worked-surface-core-baseline-v2.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    eprintln!("worked-surface diagnostic: {}", path.display());
    assert!(
        crease_pass && move_pass,
        "worked-surface desired gate failed; see retained JSON"
    );
}
