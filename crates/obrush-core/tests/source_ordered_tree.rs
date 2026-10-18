//! Independently parameterized dyadic/literal order and geometry gates. These
//! do not establish Source optimizer mapping, ray tie/contact, or runtime parity.
use obrush_core::*;
use std::sync::Arc;

fn octants() -> Mesh {
    let mut m = Mesh::from_obj(include_str!("fixtures/independent-octants104.obj")).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m
}
fn clay_mesh() -> Mesh {
    let text = include_str!("fixtures/independent-octants104.obj").to_owned()
        + "v -0.25 -0.25 0\nv 0.25 -0.25 0\nv 0 0.25 0\nf 313 314 315\n";
    let mut m = Mesh::from_obj(&text).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.masks[312..].fill(0.);
    m
}
fn clay_pass() -> SourceGeometryPass {
    SourceGeometryPass {
        original_face_index: 104,
        ..pass()
    }
}
fn pass() -> SourceGeometryPass {
    SourceGeometryPass {
        center: [0.; 3],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 1600.,
        original_face_index: 0,
    }
}
fn ordered() -> SourceGeometryOptions {
    SourceGeometryOptions {
        clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
        ..Default::default()
    }
}
fn project(d: &Document) -> Vec<u8> {
    d.to_json().unwrap().into_bytes()
}
fn tree_words(s: &SourceGeometryNormalState) -> serde_json::Value {
    serde_json::to_value(s.ordered_tree_snapshot().unwrap()).unwrap()
}

#[test]
fn o01_independent104_literal_face_and_corner_order() {
    let m = octants();
    assert_eq!(m.positions.len(), 312);
    assert_eq!(m.triangles.len(), 104);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let q = state.ordered_selection(&m, pass()).unwrap();
    let expected_faces = [
        7, 15, 23, 31, 39, 47, 55, 63, 71, 79, 87, 95, 103, 6, 14, 22, 30, 38, 46, 54, 62, 70, 78,
        86, 94, 102, 5, 13, 21, 29, 37, 45, 53, 61, 69, 77, 85, 93, 101, 4, 12, 20, 28, 36, 44, 52,
        60, 68, 76, 84, 92, 100, 3, 11, 19, 27, 35, 43, 51, 59, 67, 75, 83, 91, 99, 2, 10, 18, 26,
        34, 42, 50, 58, 66, 74, 82, 90, 98, 1, 9, 17, 25, 33, 41, 49, 57, 65, 73, 81, 89, 97, 0, 8,
        16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96,
    ];
    assert_eq!(q.candidate_faces, expected_faces);
    let expected_vertices: Vec<usize> = expected_faces
        .iter()
        .flat_map(|&f| [3 * f as usize, 3 * f as usize + 1, 3 * f as usize + 2])
        .collect();
    assert_eq!(q.selected_vertices, expected_vertices);
    let snapshot = state.ordered_tree_snapshot().unwrap();
    assert_eq!(snapshot.cells.len(), 9);
    assert_eq!(
        snapshot
            .cells
            .iter()
            .filter(|c| c.children.is_none())
            .map(|c| c.faces.len())
            .collect::<Vec<_>>(),
        vec![13; 8]
    );
    assert_eq!(q.visited_leaves.len(), 8);
}
#[test]
fn o02_100_unsplit101_split_and_depth8_closed_scope() {
    for count in [100, 101] {
        let text = (0..count)
            .map(|_| "v -1 -1 0\nv 1 -1 0\nv 0 1 0\n")
            .collect::<String>()
            + &(0..count)
                .map(|i| format!("f {} {} {}\n", 3 * i + 1, 3 * i + 2, 3 * i + 3))
                .collect::<String>();
        let mut m = Mesh::from_obj(&text).unwrap();
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        let t = SourceGeometryNormalState::seed(&m)
            .unwrap()
            .ordered_tree_snapshot()
            .unwrap();
        if count == 100 {
            assert_eq!(t.cells.len(), 1);
            assert_eq!(t.cells[0].faces.len(), 100);
        } else {
            assert_eq!(t.cells.len(), 65);
            assert_eq!(t.cells.iter().map(|c| c.depth).max(), Some(8));
            assert_eq!(
                t.cells
                    .iter()
                    .find(|c| !c.faces.is_empty())
                    .unwrap()
                    .faces
                    .len(),
                101
            );
        }
    }
}
#[test]
fn o03_float32_centres_and_midpoint_ties_are_lower_children() {
    let mut m = octants();
    for v in &mut m.positions[0..3] {
        v.z = 0.;
    }
    let t = SourceGeometryNormalState::seed(&m)
        .unwrap()
        .ordered_tree_snapshot()
        .unwrap();
    let root = &t.cells[0];
    let child0 = root.children.unwrap()[0];
    assert_eq!(
        t.face_centers[0].map(f32::to_bits),
        [(-4f32).to_bits(), (-4f32).to_bits(), 0]
    );
    assert_eq!(t.face_leaves[0], child0);
    let c = t.cells.iter().find(|c| c.handle == child0).unwrap();
    assert_eq!(c.split[5], 0.);
}
#[test]
fn o04_loose_sphere_tangency_is_candidate_but_vertex_boundary_is_strict() {
    let mut m = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let q = state
        .ordered_selection(
            &m,
            SourceGeometryPass {
                center: [2., 0., 0.],
                local_radius_squared: 1.,
                ..pass()
            },
        )
        .unwrap();
    assert_eq!(q.candidate_faces, [0]);
    assert_eq!(q.visited_leaves.len(), 1);
    assert!(q.selected_vertices.is_empty());
    let q = state
        .ordered_selection(
            &m,
            SourceGeometryPass {
                center: [0.; 3],
                local_radius_squared: 1.,
                ..pass()
            },
        )
        .unwrap();
    assert_eq!(q.selected_vertices, [0]); // vertices 1 and 2 lie exactly at radius.
}
#[test]
fn o05_original_quad_fourth_corner_first_encounter_not_canonical_duplicates() {
    let mut m = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\nf 1 3 4\n")
        .unwrap()
        .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let q = state.ordered_selection(&m, pass()).unwrap();
    assert_eq!(q.candidate_faces, [0, 1]);
    assert_eq!(q.selected_vertices, [0, 1, 2, 3]);
    assert_eq!(state.ordered_affected_faces(&[0, 2]).unwrap(), [1, 0]);
}
#[test]
fn o07_selected_and_descending_incident_ring_order_before_dedup() {
    let mut m = Mesh::from_obj(
        "v 0 0 0\nv 2 0 0\nv 0 2 0\nv -2 0 0\nv 0 -2 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\n",
    )
    .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(state.ordered_affected_faces(&[1, 0]).unwrap(), [0, 2, 1]);
    assert_eq!(state.ordered_affected_faces(&[0, 1]).unwrap(), [2, 1, 0]);
    assert_eq!(
        state.ordered_affected_faces(&[usize::MAX]).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
}
#[test]
fn o12_mutable_mesh_position_original_face_and_invalid_pass_stale_reject() {
    let mut m = octants();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let before = tree_words(&state);
    m.positions[0].x += 0.125;
    assert_eq!(
        state.ordered_selection(&m, pass()).unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    m.positions[0].x -= 0.125;
    let mut invalid = pass();
    invalid.center[0] = f64::NAN;
    assert_eq!(
        state.ordered_selection(&m, invalid).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    m.triangles.swap(0, 1);
    assert_eq!(
        state.ordered_selection(&m, pass()).unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(tree_words(&state), before);
}
#[test]
fn o13_useful_general_clay_warm_release_newpress_exact_cache_and_project() {
    let mut d = Document::new(clay_mesh());
    let before = project(&d);
    let mut stroke = d.begin_source_geometry_stroke(ordered()).unwrap();
    let r = d
        .apply_source_geometry_dab(&mut stroke, clay_pass(), 0.5, true)
        .unwrap();
    assert_eq!(r.selected_vertices, 315);
    assert_eq!(r.changed_vertices, 156);
    assert!(r.ordered_static_selection);
    assert!(!r.clay_single_leaf);
    assert_eq!(r.displacement_scale, 1.);
    assert!(stroke.proxy_positions().is_empty());
    // Independent double scalar oracle: equal U counts at Z=-4/+4 give
    // plane Z=4. Upper vertices already lie on it; lower delta=4*polynomial.
    // One final f32 conversion is the declared exact gate.
    let original = octants();
    for (old, new) in original
        .positions
        .iter()
        .zip(&d.require_active_mesh().unwrap().positions)
    {
        let p = old.to_array().map(f64::from);
        let dist = ((p[0] * p[0] + p[1] * p[1]) + p[2] * p[2]).sqrt() / 40.;
        let q = dist * dist;
        let expected = if old.z == 4. {
            4f32
        } else {
            (-4. + 4. * (3. * q * q - 4. * q * dist + 1.)) as f32
        };
        assert_eq!(new.z.to_bits(), expected.to_bits());
        assert_eq!(new.x.to_bits(), old.x.to_bits());
        assert_eq!(new.y.to_bits(), old.y.to_bits());
    }
    assert_eq!(
        &d.require_active_mesh().unwrap().positions[312..],
        &clay_mesh().positions[312..]
    );
    d.end_source_geometry_stroke(&stroke).unwrap();
    let warm = d.source_geometry_state(1).unwrap();
    let warm_tree = tree_words(&warm);
    let next = d.begin_source_geometry_stroke(ordered()).unwrap();
    assert!(Arc::ptr_eq(&warm, &d.source_geometry_state(1).unwrap()));
    assert_eq!(tree_words(&d.source_geometry_state(1).unwrap()), warm_tree);
    d.cancel_source_geometry_stroke(&next).unwrap();
    assert!(d.undo());
    assert_eq!(project(&d), before);
    assert!(d.redo());
    assert!(Arc::ptr_eq(&warm, &d.source_geometry_state(1).unwrap()));
    let loaded = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert!(loaded.source_geometry_state(1).is_none());
}
#[test]
fn o14_cancel_and_real_redo_with_initial_noop_retains_complete_state() {
    let mut d = Document::new(clay_mesh());
    let before = project(&d);
    let mut stroke = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut stroke, clay_pass(), 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&stroke).unwrap();
    let edited = project(&d);
    assert!(d.undo());
    assert!(d.source_geometry_state(1).is_none());
    let mut zero = ordered();
    zero.intensity = 0.;
    let mut noop = d.begin_source_geometry_stroke(zero).unwrap();
    assert_eq!(
        d.apply_source_geometry_dab(&mut noop, clay_pass(), 0.5, true)
            .unwrap()
            .changed_vertices,
        0
    );
    d.end_source_geometry_stroke(&noop).unwrap();
    assert_eq!(project(&d), before);
    assert!(d.source_geometry_state(1).is_none());
    assert!(d.redo());
    assert_eq!(project(&d), edited);
    let cache = d.source_geometry_state(1).unwrap();
    let mut next = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut next, clay_pass(), 0.5, true)
        .unwrap();
    d.cancel_source_geometry_stroke(&next).unwrap();
    assert_eq!(project(&d), edited);
    assert!(Arc::ptr_eq(&cache, &d.source_geometry_state(1).unwrap()));
}
#[test]
fn p01_accumulated_brush_has_no_proxy_but_nonaccumulated_preserves_first_touch() {
    let mut d = Document::new(clay_mesh());
    let mut a = d.begin_source_geometry_stroke(ordered()).unwrap();
    let r = d
        .apply_source_geometry_dab(&mut a, clay_pass(), 0.5, true)
        .unwrap();
    assert_eq!(r.proxy_captured_vertices, 0);
    assert!(a.proxy_positions().is_empty());
    d.end_source_geometry_stroke(&a).unwrap();
    let opts = SourceGeometryOptions {
        tool: SourceGeometryTool::Brush {
            clay: true,
            accumulate: false,
        },
        ..ordered()
    };
    let before = d.require_active_mesh().unwrap().positions.clone();
    let mut b = d.begin_source_geometry_stroke(opts).unwrap();
    d.apply_source_geometry_dab(&mut b, clay_pass(), 0.5, true)
        .unwrap();
    assert_eq!(b.proxy_positions(), before);
    d.end_source_geometry_stroke(&b).unwrap();
}
#[test]
fn default_compatibility_blocker_is_not_silently_retired() {
    let d = Document::new(octants());
    assert_eq!(
        d.source_geometry_blocker(SourceGeometryOptions::default())
            .unwrap_err(),
        SourceGeometryError::UnsupportedClayOrder
    );
    assert!(d.source_geometry_blocker(ordered()).is_ok());
}

#[test]
fn o06_actual_general_clay_uses_uncompensated_tree_area_order() {
    let mut text = String::new();
    let mut faces = Vec::new();
    let mut masks = Vec::new();
    let l = 262144f32;
    let t = 0.0009765625f32;
    let groups = [
        [[0., 0., 0.5], [0., l, 0.5], [-l, 0., 0.5]],
        [[0., 0., -0.5], [0., l, -0.5], [l, 0., -0.5]],
        [[0., 0., 0.5], [t, 0., 0.5], [0., t, 0.5]],
    ];
    for points in groups {
        let base = masks.len();
        for [x, y, z] in points {
            text.push_str(&format!("v {x} {y} {z}\n"));
        }
        faces.push([base + 1, base + 2, base + 3]);
        masks.extend([1., 0., 0.]);
    }
    for _ in 0..98 {
        let base = masks.len();
        text.push_str("v -4 -4 -0.5\nv -3.5 -4 -0.5\nv -4 -3.5 -0.5\n");
        faces.push([base + 1, base + 2, base + 3]);
        masks.extend([0.; 3]);
    }
    let contact = faces.len();
    let base = masks.len();
    text.push_str("v -0.25 -0.25 0\nv 0.25 -0.25 0\nv 0 0.25 0\n");
    faces.push([base + 1, base + 2, base + 3]);
    masks.extend([0.; 3]);
    // Unused bbox witness balances Y about0; it has raw unused normal0.
    text.push_str("v 0 -262144 0\n");
    masks.push(0.);
    for [a, b, c] in faces {
        text.push_str(&format!("f {a} {b} {c}\n"));
    }
    let mut m = Mesh::from_obj(&text).unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks = masks;
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let p = SourceGeometryPass {
        center: [0.; 3],
        eye_direction: [1., 0., 0.],
        local_radius_squared: 4.,
        original_face_index: contact,
    };
    assert_eq!(
        state.vertex_normals()[0].z.to_bits(),
        (68719476736f32).to_bits()
    );
    assert_eq!(
        state.vertex_normals()[3].z.to_bits(),
        (-68719476736f32).to_bits()
    );
    assert_eq!(
        state.vertex_normals()[6].z.to_bits(),
        f32::from_bits(0x3580_0000).to_bits()
    );
    let q = state.ordered_selection(&m, p).unwrap();
    assert_eq!(
        q.selected_vertices
            .iter()
            .copied()
            .filter(|&i| m.masks[i] == 1.)
            .collect::<Vec<_>>(),
        [0, 6, 3]
    );
    let ascending = (68719476736f64 - 68719476736f64) + 0.00000095367431640625;
    assert_eq!(ascending, 0.00000095367431640625);
    let mut d = Document::new(m);
    let before = project(&d);
    let mut stroke = d.begin_source_geometry_stroke(ordered()).unwrap();
    let r = d
        .apply_source_geometry_dab(&mut stroke, p, 0.5, true)
        .unwrap();
    assert_eq!(r.noop, Some(SourceGeometryNoop::NoAreaDirection));
    assert_eq!(r.changed_vertices, 0);
    assert!(stroke.proxy_positions().is_empty());
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert_eq!(project(&d), before);
    assert!(d.source_geometry_state(1).is_none());
}

#[test]
fn p03_repeated_warm_zero_dabs_bound_unique_visits_without_project_history_change() {
    let mut d = Document::new(clay_mesh());
    let mut useful = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut useful, clay_pass(), 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&useful).unwrap();
    let edited = project(&d);
    let mut opts = ordered();
    opts.intensity = 0.;
    let mut stroke = d.begin_source_geometry_stroke(opts).unwrap();
    for _ in 0..64 {
        assert_eq!(
            d.apply_source_geometry_dab(&mut stroke, clay_pass(), 0.5, true)
                .unwrap()
                .changed_vertices,
            0
        );
    }
    assert_eq!(stroke.visited_leaf_count(), 8);
    assert!(stroke.visited_leaf_storage_bytes() <= 16 * std::mem::size_of::<SourceLeafHandle>());
    assert_eq!(project(&d), edited);
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert_eq!(project(&d), edited);
    assert!(d.undo()); // one useful stroke, no extra zero-dab history.
    assert!(d.source_geometry_state(1).is_none());
    assert!(!d.undo());
    assert!(d.redo());
    assert_eq!(project(&d), edited);
}

#[test]
fn release_stale_position_cancels_useful_work_and_owned_token() {
    let mut d = Document::new(clay_mesh());
    let before = project(&d);
    let mut stroke = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut stroke, clay_pass(), 0.5, true)
        .unwrap();
    d.require_active_mesh_mut().unwrap().positions[0].x += 0.125;
    assert_eq!(
        d.end_source_geometry_stroke(&stroke).unwrap_err(),
        SourceGeometryError::StaleNormalState
    );
    assert_eq!(project(&d), before);
    assert!(d.source_geometry_state(1).is_none());
    assert!(!d.undo());
    let next = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.cancel_source_geometry_stroke(&next).unwrap();
}
#[test]
fn release_no_dab_external_domain_change_cancels_and_retains_genuine_redo() {
    let mut d = Document::new(clay_mesh());
    let before = project(&d);
    let mut useful = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut useful, clay_pass(), 0.5, true)
        .unwrap();
    d.end_source_geometry_stroke(&useful).unwrap();
    let edited = project(&d);
    assert!(d.undo());
    let stroke = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.require_active_mesh_mut().unwrap().masks[0] = 0.5;
    assert_eq!(
        d.end_source_geometry_stroke(&stroke).unwrap_err(),
        SourceGeometryError::StrokeBindingChanged
    );
    assert_eq!(project(&d), before);
    assert!(d.redo());
    assert_eq!(project(&d), edited);
}
#[test]
fn foreign_release_token_does_not_cancel_rightful_owned_gesture() {
    let mut d = Document::new(clay_mesh());
    let foreign = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.cancel_source_geometry_stroke(&foreign).unwrap();
    let mut owned = d.begin_source_geometry_stroke(ordered()).unwrap();
    d.apply_source_geometry_dab(&mut owned, clay_pass(), 0.5, true)
        .unwrap();
    let mid = project(&d);
    assert_eq!(
        d.end_source_geometry_stroke(&foreign).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(project(&d), mid);
    d.end_source_geometry_stroke(&owned).unwrap();
    assert!(d.undo());
}
