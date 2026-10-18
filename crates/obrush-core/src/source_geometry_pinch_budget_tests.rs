//! Original integration cap/stale gate relocated into private core test scope.
use crate::*;

fn mesh() -> Mesh {
    let mut m = Mesh::import_obj(
        b"v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nv -4 -4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    )
    .unwrap()
    .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.625,
        intensity_factor: 0.0,
        ..SourceGeometryOptions::pinch()
    }
}
fn pass(x: f64, r2: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0.0, 0.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: r2,
        original_face_index: if x >= 0.0 { 0 } else { 2 },
    }
}
fn words(m: &Mesh) -> serde_json::Value {
    let mut v = serde_json::to_value(m).unwrap();
    v["raw_f32"] = serde_json::json!({
        "positions":m.positions.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":m.normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":m.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
        "colors":m.colors.iter().map(|v|v.map(f32::to_bits)).collect::<Vec<_>>(),
        "materials":m.materials.as_ref().map(|v|v.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>()),
    });
    v
}
#[test]
fn insufficient_retained_budget_and_stale_positions_reject_atomically() {
    let mut m = mesh();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let before = words(&m);
    let mut s = SourceGeometryStroke::new(options()).unwrap();
    assert_eq!(
        s.apply_with_state_limit(&mut m, &state, pass(0.5, 4.0), 0.5, true, 0)
            .unwrap_err(),
        SourceGeometryError::StorageLimit
    );
    assert_eq!(words(&m), before);
    m.positions[0].x = 0.125;
    let externally_edited = words(&m);
    assert_eq!(
        s.apply(&mut m, &state, pass(0.5, 4.0), 0.5, true)
            .unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(words(&m), externally_edited);
}
