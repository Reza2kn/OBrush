//! Regression from the actual CW02 Source normal-history characterization.
//! Source geometry history refreshes captured rows from the CURRENT field;
//! restoring an old normal snapshot changes subsequent brush directions.
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::RawPressureSample;
use obrush_core::*;
use std::sync::{Arc, Mutex};

fn document() -> Document {
    let mut m = Mesh::from_obj(
        "v -1 0 0.5\nv 1 0 0.5\nv 0 1 0.5\nv -1 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 4 5 6\n",
    )
    .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]],
        uv: None,
    });
    Document::new(m)
}
fn input() -> host::SourceGeometryInput {
    host::SourceGeometryInput {
        view: PaintViewCapture {
            world_to_clip: [
                0.4, 0.0, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [49.8, 45.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn apply(d: &mut Document, radius: f64, intensity: f64) {
    apply_controls(d, radius, intensity, true);
}
fn apply_controls(d: &mut Document, radius: f64, intensity: f64, mirror: bool) {
    let preferences = SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius,
        radius_factor: 0.0,
        options: SourceGeometryOptions {
            intensity,
            prevent_folds: false,
            ..SourceGeometryOptions::crease()
        },
        ..SourceGeometryPreferences::crease()
    };
    let settings = BrushSettings {
        kind: BrushKind::Crease,
        adaptive_detail: false,
        symmetry_x: mirror,
        alpha: None,
        ..Default::default()
    };
    let session = SourceGeometrySession::begin_with_controls(
        d,
        &settings,
        false,
        preferences,
        input(),
        Arc::new(Mutex::new(Default::default())),
    )
    .unwrap();
    let report = host::apply_shared(d, &session, input()).unwrap();
    assert!(report.changed_vertices > 0);
    assert_eq!(
        report.selected_vertices,
        if radius == 25.0 { 6 } else { 2 } * if mirror { 2 } else { 1 }
    );
    session.lock().unwrap().finish(d).unwrap();
}
fn fresh_field(m: &Mesh) -> Vec<Vec3> {
    // Independent original triangle cross products; this fixture has exactly
    // one incident face per vertex, so no averaging/order ambiguity exists.
    let mut field = vec![Vec3::ZERO; m.positions.len()];
    for [a, b, c] in &m.triangles {
        let origin = m.positions[*a as usize];
        let u = std::array::from_fn::<_, 3, _>(|k| {
            f64::from(m.positions[*b as usize][k]) - f64::from(origin[k])
        });
        let v = std::array::from_fn::<_, 3, _>(|k| {
            f64::from(m.positions[*c as usize][k]) - f64::from(origin[k])
        });
        let n = Vec3::new(
            (u[1] * v[2] - u[2] * v[1]) as f32,
            (u[2] * v[0] - u[0] * v[2]) as f32,
            (u[0] * v[1] - u[1] * v[0]) as f32,
        );
        // Vertex accumulation starts at +0 even with one incident face, so
        // a -0 face component becomes +0 before the vertex Float32 store.
        let mut accumulated = [0.0f64; 3];
        for k in 0..3 {
            accumulated[k] += f64::from(n[k]);
        }
        let n = Vec3::from_array(accumulated.map(|x| x as f32));
        // This authored disconnected-triangle incidence is explicit.
        for vertex in [*a, *b, *c] {
            field[vertex as usize] = n;
        }
    }
    field
}
fn field(d: &Document) -> Vec<Vec3> {
    d.source_geometry_state(d.active_subtool_id().unwrap())
        .unwrap()
        .vertex_normals()
        .to_vec()
}
fn bits(rows: &[Vec3]) -> Vec<[u32; 3]> {
    rows.iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect()
}

#[test]
fn source_ordinary_undo_large_stroke_refreshes_all_captured_normals_from_current_field() {
    let mut d = document();
    apply(&mut d, 25.0, 0.25);
    apply(&mut d, 18.5, 0.75);
    let before_c = d.to_json().unwrap();
    let original_middle = field(&d);
    let rebuilt_middle = fresh_field(d.require_active_mesh().unwrap());
    assert_ne!(
        bits(&original_middle),
        bits(&rebuilt_middle),
        "Fixture must expose stale outside rows"
    );
    apply(&mut d, 25.0, 0.75);
    assert!(d.undo());
    assert_eq!(
        d.to_json().unwrap(),
        before_c,
        "Whole geometry/attributes restore exactly"
    );
    assert_eq!(
        bits(&field(&d)),
        bits(&rebuilt_middle),
        "SculptGL StateGeometry.undo refreshes the large stroke's full captured vertex union"
    );
}

#[test]
fn source_ordinary_second_undo_keeps_current_outside_rows_instead_of_old_snapshot() {
    let mut d = document();
    apply(&mut d, 25.0, 0.25);
    let after_a = d.to_json().unwrap();
    let field_a = field(&d);
    apply(&mut d, 18.5, 0.75);
    let mut expected = fresh_field(d.require_active_mesh().unwrap());
    // UndoC refreshes all rows to B. UndoB restores A geometry and refreshes
    // only the middle stroke's two captured rows; other CURRENT B rows remain.
    for v in [2, 5] {
        expected[v] = field_a[v];
    }
    assert_ne!(bits(&expected), bits(&field_a));
    apply(&mut d, 25.0, 0.75);
    assert!(d.undo() && d.undo());
    assert_eq!(d.to_json().unwrap(), after_a);
    assert_eq!(
        bits(&field(&d)),
        bits(&expected),
        "Undo must refresh only this history event's captured rows"
    );
}

#[test]
fn no_alpha_no_mirror_host_uses_current_history_field_for_a_new_stroke_after_undo() {
    let mut d = document();
    apply_controls(&mut d, 25.0, 0.25, false);
    apply_controls(&mut d, 18.5, 0.75, false);
    let before_c = d.to_json().unwrap();
    let warm_b = field(&d);
    let refreshed_b = fresh_field(d.require_active_mesh().unwrap());
    assert_ne!(bits(&warm_b), bits(&refreshed_b));
    apply_controls(&mut d, 25.0, 0.75, false);
    let original_c = d.to_json().unwrap();
    let original_c_positions = d.require_active_mesh().unwrap().positions.clone();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_c);
    assert_eq!(bits(&field(&d)), bits(&refreshed_b));
    assert!(d.can_redo());
    apply_controls(&mut d, 25.0, 0.75, false);
    let new_d = d.to_json().unwrap();
    assert_ne!(new_d, original_c);
    assert_ne!(
        bits(&d.require_active_mesh().unwrap().positions),
        bits(&original_c_positions)
    );
    assert!(
        !d.can_redo(),
        "New useful post-undo stroke replaces old redo"
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_c);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), new_d);
}
