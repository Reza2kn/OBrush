//! Independent geometric history contracts, including pre-culling selection,
//! exact public data, derived fields, attribute history and atomic rejection.
use super::*;
use crate::*;

fn six() -> Document {
    let mut mesh = Mesh::from_obj(
        "v -1 0 0.5\nv 1 0 0.5\nv 0 1 0.5\nv -1 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 4 5 6\n",
    )
    .unwrap();
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.0);
    Document::new(mesh)
}
fn options() -> SourceGeometryOptions {
    SourceGeometryOptions {
        intensity: 0.25,
        prevent_folds: false,
        ..SourceGeometryOptions::crease()
    }
}
fn contact(d: &Document, radius_squared: f64) -> SourceGeometryPass {
    let m = d.require_active_mesh().unwrap();
    let [a, b, c] = m.triangles[0].map(|v| m.positions[v as usize].as_dvec3());
    let n = (b - a).cross(c - a);
    let (x, y) = (-0.01, 0.25);
    let z = a.z - (n.x * (x - a.x) + n.y * (y - a.y)) / n.z;
    SourceGeometryPass {
        center: [x, y, z],
        eye_direction: [0., 0., -1.],
        local_radius_squared: radius_squared,
        original_face_index: 0,
    }
}
fn dab(
    d: &mut Document,
    s: &mut SourceGeometryStroke,
    p: SourceGeometryPass,
) -> SourceGeometryReport {
    d.apply_source_geometry_controlled_dab(
        s,
        [Some(p), None],
        0.5,
        true,
        &SourceGeometryControls {
            alpha: None,
            mirror: false,
        },
        Default::default(),
    )
    .unwrap()
    .combined
}
fn stroke(d: &mut Document, radius_squared: f64, reference: bool) {
    let p = contact(d, radius_squared);
    let mut s = if reference {
        d.begin_source_geometry_selection_history(options())
    } else {
        d.begin_source_geometry_stroke(options())
    }
    .unwrap();
    assert!(dab(d, &mut s, p).changed_vertices > 0);
    d.end_source_geometry_stroke(&s).unwrap();
}
fn field(d: &Document) -> Vec<Vec3> {
    d.source_geometry_state(1)
        .unwrap()
        .vertex_normals()
        .to_vec()
}
fn words(rows: &[Vec3]) -> Vec<[u32; 3]> {
    rows.iter()
        .map(|n| n.to_array().map(f32::to_bits))
        .collect()
}
fn fresh(m: &Mesh) -> Vec<Vec3> {
    let mut sums = vec![[0.0f64; 3]; m.positions.len()];
    let mut counts = vec![0; m.positions.len()];
    for ids in m.triangles.iter().rev() {
        let [a, b, c] = ids.map(|v| m.positions[v as usize].as_dvec3());
        let cross = (b - a).cross(c - a).as_vec3();
        for &v in ids {
            for k in 0..3 {
                sums[v as usize][k] += f64::from(cross[k]);
            }
            counts[v as usize] += 1;
        }
    }
    sums.into_iter()
        .zip(counts)
        .map(|(s, n)| Vec3::from_array(s.map(|v| (v * (1.0 / f64::from(n))) as f32)))
        .collect()
}
fn selected(d: &Document) -> Vec<usize> {
    match d.undo.back().unwrap().source_history.as_ref().unwrap() {
        SourceHistoryEvent::Geometry(event) => event
            .selected
            .iter()
            .enumerate()
            .filter_map(|(v, yes)| yes.then_some(v))
            .collect(),
        _ => panic!("Expected captured geometry event"),
    }
}

#[test]
fn generic_geometry_api_retains_original_normal_snapshot_history() {
    let mut d = six();
    stroke(&mut d, 1.5625, false);
    stroke(&mut d, 0.855625, false);
    let before = d.to_json().unwrap();
    let old = field(&d);
    assert_ne!(words(&old), words(&fresh(d.require_active_mesh().unwrap())));
    stroke(&mut d, 1.5625, false);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&old));
}

#[test]
fn geometry_history_survives_partial_redo_and_keeps_current_outside_rows() {
    let mut d = six();
    stroke(&mut d, 1.5625, true);
    stroke(&mut d, 0.855625, true);
    let before = d.to_json().unwrap();
    let old = field(&d);
    let rebuilt = fresh(d.require_active_mesh().unwrap());
    assert_ne!(words(&old), words(&rebuilt));
    stroke(&mut d, 1.5625, true);
    assert!(d.undo() && d.undo() && d.redo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&rebuilt));
}

#[test]
fn protected_selected_vertices_are_captured_even_when_they_never_move() {
    let mut d = six();
    d.mesh.as_mut().unwrap().masks[0] = 0.0;
    d.mesh.as_mut().unwrap().masks[3] = 0.0;
    let before = d.to_json().unwrap();
    let initial = fresh(d.require_active_mesh().unwrap());
    stroke(&mut d, 1.5625, true);
    assert_eq!(selected(&d), [0, 1, 2, 3, 4, 5]);
    stroke(&mut d, 1.5625, true);
    assert!(d.undo() && d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&initial));
}

#[test]
fn back_facing_selected_vertices_are_captured_before_optional_culling() {
    let mut m =
        Mesh::from_obj("v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -3 3 0\nv -3 -3 0\nf 1 2 3\nf 1 5 4\n")
            .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    let initial = fresh(d.require_active_mesh().unwrap());
    let p = SourceGeometryPass {
        center: [0.5, 0., 0.],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 64.,
        original_face_index: 0,
    };
    let mut s = d
        .begin_source_geometry_selection_history(SourceGeometryOptions {
            culling: true,
            ..options()
        })
        .unwrap();
    let r = dab(&mut d, &mut s, p);
    assert_eq!(r.selected_vertices, 5);
    assert_eq!(r.front_vertices, 3);
    assert_eq!(r.changed_vertices, 3);
    d.end_source_geometry_stroke(&s).unwrap();
    assert_eq!(selected(&d), [0, 1, 2, 3, 4]);
    let mut next = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    assert!(dab(&mut d, &mut next, p).changed_vertices > 0);
    d.end_source_geometry_stroke(&next).unwrap();
    assert!(d.undo() && d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&initial));
}

#[test]
fn brush_culling_on_two_opposed_sheets_includes_zero_normal_and_preserves_history() {
    // The shared center receives equal and opposite face normals. Its zero
    // normal is front-facing under SculptGL's inclusive <= 0 predicate.
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 1 5 4\n").unwrap();
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.0);
    let original = Document::new(mesh);
    let initial_positions = original.require_active_mesh().unwrap().positions.clone();
    let normals = SourceGeometryNormalState::seed(original.require_active_mesh().unwrap()).unwrap();
    assert_eq!(normals.vertex_normals()[0], Vec3::ZERO);
    let pass = SourceGeometryPass {
        center: [0.2, 0.2, 0.],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    };
    for clay in [false, true] {
        for culling in [false, true] {
            let mut document = Document::from_json(&original.to_json().unwrap()).unwrap();
            let before = document.to_json().unwrap();
            let mut stroke = document
                .begin_source_geometry_selection_history(SourceGeometryOptions {
                    tool: SourceGeometryTool::Brush {
                        clay,
                        accumulate: true,
                    },
                    culling,
                    clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
                    prevent_folds: false,
                    ..Default::default()
                })
                .unwrap();
            let report = dab(&mut document, &mut stroke, pass);
            document.end_source_geometry_stroke(&stroke).unwrap();
            let after = document.to_json().unwrap();
            let moved: Vec<_> = document
                .require_active_mesh()
                .unwrap()
                .positions
                .iter()
                .zip(&initial_positions)
                .enumerate()
                .filter_map(|(id, (now, old))| (now != old).then_some(id))
                .collect();
            assert_eq!(report.selected_vertices, 5, "clay={clay} culling={culling}");
            assert_eq!(report.front_vertices, 3, "clay={clay} culling={culling}");
            assert_eq!(
                moved,
                if culling {
                    vec![0, 1, 2]
                } else {
                    vec![0, 1, 2, 3, 4]
                },
                "clay={clay} culling={culling}"
            );
            assert_eq!(selected(&document), [0, 1, 2, 3, 4]);
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), before);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), after);
        }
    }
}

#[test]
fn cold_protected_noop_selection_joins_later_useful_history_without_losing_redo() {
    let mut d = six();
    d.mesh.as_mut().unwrap().masks.fill(0.0);
    d.mesh.as_mut().unwrap().masks[2] = 1.0;
    d.mesh.as_mut().unwrap().masks[5] = 1.0;
    let initial = fresh(d.require_active_mesh().unwrap());
    let before = d.to_json().unwrap();
    let p = contact(&d, 0.855625);
    let mut s = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    let noop = dab(
        &mut d,
        &mut s,
        SourceGeometryPass {
            center: [-0.98, 0.005, 0.5],
            local_radius_squared: 0.0025,
            ..p
        },
    );
    assert_eq!(noop.changed_vertices, 0);
    assert!(d.source_geometry_state(1).is_none());
    assert!(dab(&mut d, &mut s, p).changed_vertices > 0);
    d.end_source_geometry_stroke(&s).unwrap();
    assert_eq!(selected(&d), [0, 2, 5]);
    let mut expected = fresh(d.require_active_mesh().unwrap());
    for v in [0, 2, 5] {
        expected[v] = initial[v];
    }
    stroke(&mut d, 1.5625, true);
    assert!(d.undo() && d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&expected));
}

#[test]
fn source_attribute_undo_preserves_current_field_after_a_later_geometry_undo() {
    let mut d = six();
    stroke(&mut d, 0.855625, true);
    let original = field(&d);
    let rebuilt = fresh(d.require_active_mesh().unwrap());
    assert_ne!(words(&original), words(&rebuilt));
    let before_paint = d.to_json().unwrap();
    d.begin_source_attribute_history(1).unwrap();
    d.mesh.as_mut().unwrap().colors[2] = [0.1, 0.2, 0.3];
    d.end_stroke();
    stroke(&mut d, 1.5625, true);
    assert!(d.undo());
    assert_eq!(words(&field(&d)), words(&rebuilt));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_paint);
    assert_eq!(words(&field(&d)), words(&rebuilt));
    let initial_positions = six();
    let initial = fresh(initial_positions.require_active_mesh().unwrap());
    let mut expected = rebuilt;
    for v in [2, 5] {
        expected[v] = initial[v];
    }
    assert!(d.undo());
    assert_eq!(words(&field(&d)), words(&expected));
    assert!(
        d.source_geometry_state(1).is_some(),
        "Nonfresh outside rows must survive even if old snapshot had no field"
    );
}

#[test]
fn source_history_memory_rejection_does_not_pop_either_stack_or_publish_normals() {
    let mut d = six();
    stroke(&mut d, 1.5625, true);
    let before = d.to_json().unwrap();
    let current = d.source_geometry_state(1).unwrap();
    let counts = (d.undo.len(), d.redo.len());
    assert!(!d.undo_with_source_history_limit(0));
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!((d.undo.len(), d.redo.len()), counts);
    assert!(Arc::ptr_eq(&current, &d.source_geometry_state(1).unwrap()));
    assert!(d.take_layer_error().is_some());
    assert!(d.undo());
    let before_redo = d.to_json().unwrap();
    let current = d.source_geometry_state(1).unwrap();
    let counts = (d.undo.len(), d.redo.len());
    assert!(!d.redo_with_source_history_limit(0));
    assert_eq!(d.to_json().unwrap(), before_redo);
    assert_eq!((d.undo.len(), d.redo.len()), counts);
    assert!(Arc::ptr_eq(&current, &d.source_geometry_state(1).unwrap()));
    assert!(d.redo());
}

#[test]
fn reference_controlled_constructor_refuses_untracked_single_dab_api_atomically() {
    let mut d = six();
    let before = d.to_json().unwrap();
    let p = contact(&d, 1.5625);
    let mut s = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut s, p, 0.5, true)
            .unwrap_err(),
        SourceGeometryError::UnsupportedDomain
    );
    assert_eq!(d.to_json().unwrap(), before);
    d.cancel_source_geometry_stroke(&s).unwrap();
    assert!(!d.can_undo());
}

#[test]
fn cancelled_reference_stroke_and_empty_attribute_stroke_preserve_prior_redo() {
    let mut d = six();
    d.begin_stroke();
    d.name = "future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let p = contact(&d, 1.5625);
    let mut s = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    assert!(dab(&mut d, &mut s, p).changed_vertices > 0);
    d.cancel_source_geometry_stroke(&s).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.source_geometry_state(1).is_none());
    d.begin_source_attribute_history(1).unwrap();
    d.end_stroke();
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}

#[test]
fn undo_of_open_reference_stroke_uses_the_already_published_selection_union() {
    let mut d = six();
    stroke(&mut d, 1.5625, true);
    stroke(&mut d, 0.855625, true);
    let before = d.to_json().unwrap();
    let rebuilt = fresh(d.require_active_mesh().unwrap());
    let p = contact(&d, 1.5625);
    let mut active = d
        .begin_source_geometry_selection_history(options())
        .unwrap();
    assert!(dab(&mut d, &mut active, p).changed_vertices > 0);
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(words(&field(&d)), words(&rebuilt));
    assert_eq!(
        d.end_source_geometry_stroke(&active).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn undo_on_another_part_preserves_current_unrelated_normal_field_after_partial_history() {
    let mut d = six();
    let other = d.duplicate_subtool(1).unwrap();
    d.select_subtool(other).unwrap();
    stroke(&mut d, 1.5625, true);
    stroke(&mut d, 0.855625, true);
    let warm = d.source_geometry_state(other).unwrap();
    let rebuilt = fresh(d.require_active_mesh().unwrap());
    assert_ne!(words(warm.vertex_normals()), words(&rebuilt));
    d.select_subtool(1).unwrap();
    stroke(&mut d, 1.5625, true);
    d.select_subtool(other).unwrap();
    stroke(&mut d, 1.5625, true);
    assert!(d.undo());
    let current = d.source_geometry_state(other).unwrap();
    assert_eq!(words(current.vertex_normals()), words(&rebuilt));
    assert!(
        d.undo(),
        "Undo the stroke on the other, currently inactive part"
    );
    assert_eq!(d.active_subtool_id(), Some(1));
    let preserved = d.source_geometry_state(other).unwrap();
    assert_eq!(words(preserved.vertex_normals()), words(&rebuilt));
    assert!(Arc::ptr_eq(&current, &preserved));
    assert!(d.redo());
    assert!(Arc::ptr_eq(
        &current,
        &d.source_geometry_state(other).unwrap()
    ));
}
