use crate::*;

#[test]
fn selected_mesh_info_counts_original_quads_and_tracks_editable_document() {
    // Independently authored OBJ: one quad and one triangle, both canonical.
    let mixed = Mesh::import_obj(
        b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 2 0 0\nv 2 1 0\nf 1 2 3 4\nf 2 5 6\n",
    )
    .unwrap()
    .mesh;
    assert_eq!(
        (
            mixed.positions.len(),
            mixed.face_count(),
            mixed.triangles.len()
        ),
        (6, 2, 3)
    );

    let mut document = Document::empty();
    assert!(document.active_mesh().is_none());
    let mixed_id = document.add_subtool(mixed, "Mixed faces").unwrap();
    let triangle_id = document
        .add_subtool(Mesh::sphere(0), "Triangle mesh")
        .unwrap();
    let triangle = document.active_mesh().unwrap();
    assert_eq!(triangle.face_count(), triangle.triangles.len());
    document.select_subtool(mixed_id).unwrap();
    assert_eq!(document.active_mesh().unwrap().face_count(), 2);

    document
        .add_source_static_subdivision_level(&StaticSubdivisionSettings::default())
        .unwrap();
    let subdivided = document.active_mesh().unwrap();
    assert_eq!(subdivided.face_count(), 8);
    document.select_level(0).unwrap();
    assert_eq!(document.active_mesh().unwrap().face_count(), 2);
    document.select_level(1).unwrap();
    assert_eq!(document.active_mesh().unwrap().face_count(), 8);
    assert!(document.undo());
    assert_eq!(document.active_mesh().unwrap().face_count(), 2);
    document.select_subtool(triangle_id).unwrap();
    let before = document.active_mesh().unwrap().face_count();
    document.subdivide().unwrap();
    assert_eq!(document.active_mesh().unwrap().face_count(), before * 4);

    // An ordinary scene merge may have to detach mixed face metadata. The
    // displayed count must follow the resulting editable mesh and its save.
    let mut merged = Document::empty();
    let first = merged
        .add_subtool(
            Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n")
                .unwrap()
                .mesh,
            "Quad",
        )
        .unwrap();
    let second = merged.add_subtool(Mesh::sphere(0), "Sphere").unwrap();
    merged
        .merge_subtools(&SubtoolMergeSettings {
            ids: vec![first, second],
            name: "Merged".into(),
            policy: SubtoolMergePolicy::RequirePlain,
            allow_source_pose_loss: false,
            allow_mask_precision_loss: false,
            allow_uv_loss: false,
        })
        .unwrap();
    let current = merged.active_mesh().unwrap();
    let counts = (
        current.positions.len(),
        current.face_count(),
        current.triangles.len(),
    );
    assert_eq!(counts, (16, 21, 22));
    let saved = merged.to_json().unwrap();
    let restored = Document::from_json(&saved).unwrap();
    let after = restored.active_mesh().unwrap();
    assert_eq!(
        counts,
        (
            after.positions.len(),
            after.face_count(),
            after.triangles.len()
        )
    );
    assert!(merged.undo());
    assert_eq!(merged.active_mesh().unwrap().face_count(), 20);
    assert!(merged.redo());
    assert_eq!(merged.active_mesh().unwrap().face_count(), counts.1);

    // A versioned scene can also have no original SurfaceTopology at all.
    let mut plain_scene = Document::empty();
    plain_scene.add_subtool(Mesh::sphere(0), "Plain").unwrap();
    let plain_json = plain_scene.to_json().unwrap();
    let mut plain_project: serde_json::Value = serde_json::from_str(&plain_json).unwrap();
    assert_eq!(plain_project["version"], 5);
    // Version 7 also accepts an unchanged plain scene with no SurfaceTopology;
    // a version field alone cannot be used to infer quad faces.
    plain_project["version"] = 7.into();
    for encoded in [&plain_json, &plain_project.to_string()] {
        let plain_reloaded = Document::from_json(encoded).unwrap();
        let plain_mesh = plain_reloaded.active_mesh().unwrap();
        assert!(plain_mesh.surface.is_none());
        assert_eq!(plain_mesh.face_count(), plain_mesh.triangles.len());
    }
}

fn brush(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.6,
        strength: 0.8,
        invert: false,
        symmetry_x: false,
        color: [0.9, 0.1, 0.2],
        ..Default::default()
    }
}

#[test]
fn icospheres_have_shared_topology_unit_radius_and_outward_normals() {
    for level in 0..=4 {
        let mesh = Mesh::sphere(level);
        assert_eq!(mesh.positions.len(), 10 * 4_usize.pow(level) + 2);
        assert_eq!(mesh.triangles.len(), 20 * 4_usize.pow(level));
        mesh.validate().unwrap();
        for (&position, &normal) in mesh.positions.iter().zip(&mesh.normals) {
            assert!((position.length() - 1.0).abs() < 1e-5);
            assert!((normal.length() - 1.0).abs() < 1e-5);
            assert!(normal.dot(position) > 0.99);
        }
        for &[a, b, c] in &mesh.triangles {
            let (a, b, c) = (
                mesh.positions[a as usize],
                mesh.positions[b as usize],
                mesh.positions[c as usize],
            );
            assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
        }
    }
}

#[test]
fn cube_is_dense_watertight_and_outward() {
    let mesh = Mesh::cube();
    mesh.validate().unwrap();
    assert_eq!(mesh.triangles.len(), 12 * 4_usize.pow(4));
    for position in &mesh.positions {
        assert!((position.abs().max_element() - 1.0).abs() < 1e-6);
    }
    let mut edges = std::collections::HashMap::new();
    for &[a, b, c] in &mesh.triangles {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
        let (a, b, c) = (
            mesh.positions[a as usize],
            mesh.positions[b as usize],
            mesh.positions[c as usize],
        );
        assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
    }
    assert!(edges.values().all(|&count| count == 2));
}

#[test]
fn raycast_returns_nearest_world_distance_and_interpolated_normal() {
    let mesh = Mesh::sphere(3);
    let hit = mesh
        .raycast(Ray {
            origin: Vec3::new(0.0, 0.0, 3.0),
            direction: Vec3::new(0.0, 0.0, -7.0),
        })
        .unwrap();
    assert!((hit.distance - 2.0).abs() < 1e-5);
    assert!(hit.position.distance(Vec3::Z) < 1e-5);
    assert!(hit.normal.dot(Vec3::Z) > 0.999);
    assert!(mesh
        .raycast(Ray {
            origin: Vec3::new(0.0, 0.0, 3.0),
            direction: Vec3::Z
        })
        .is_none());
    assert!(mesh
        .raycast(Ray {
            origin: Vec3::ZERO,
            direction: Vec3::ZERO
        })
        .is_none());
    assert!(mesh
        .raycast(Ray {
            origin: Vec3::splat(f32::NAN),
            direction: Vec3::Z
        })
        .is_none());
}

#[test]
fn obj_triangulation_and_raycast_work_at_small_coordinate_scales() {
    let mesh = Mesh::from_obj(
        "v 0 0 0\nv 0.0000001 0 0\nv 0.0000001 0.0000001 0\nv 0 0.0000001 0\nf 1 2 3 4\n",
    )
    .unwrap();
    let hit = mesh
        .raycast(Ray {
            origin: Vec3::new(0.00000005, 0.00000005, 0.0000001),
            direction: -Vec3::Z,
        })
        .unwrap();
    assert!((hit.distance - 0.0000001).abs() < 1e-12);
    assert_eq!(hit.normal, Vec3::Z);
}

#[test]
fn draw_changes_front_surface_and_respects_mask_and_inversion() {
    let original = Mesh::sphere(3);
    let mut raised = original.clone();
    let mut carved = original.clone();
    let settings = brush(BrushKind::Draw);
    assert!(raised.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings) > 0);
    let mut inverted = settings.clone();
    inverted.invert = true;
    assert!(carved.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &inverted) > 0);
    let pole = original
        .positions
        .iter()
        .position(|p| p.distance(Vec3::Z) < 1e-6)
        .unwrap();
    assert!(raised.positions[pole].z > 1.0 && carved.positions[pole].z < 1.0);
    for (i, p) in original
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p.z < 0.0)
    {
        assert_eq!(raised.positions[i], *p);
    }
    let mut protected = original.clone();
    protected.masks.fill(1.0);
    for kind in BrushKind::ALL
        .into_iter()
        .filter(|&kind| kind != BrushKind::Mask)
    {
        assert_eq!(
            protected.apply_brush(Vec3::Z, Vec3::Z, Vec3::X, &brush(kind)),
            0
        );
    }
    assert_eq!(protected.positions, original.positions);
    assert_eq!(protected.colors, original.colors);
}

#[test]
fn clay_inflate_and_crease_have_opposite_inverted_displacements() {
    let original = Mesh::sphere(2);
    let pole = original
        .positions
        .iter()
        .position(|p| p.distance(Vec3::Z) < 1e-6)
        .unwrap();
    for kind in [BrushKind::Clay, BrushKind::Inflate, BrushKind::Crease] {
        let mut normal_dab = original.clone();
        let mut inverse_dab = original.clone();
        let settings = brush(kind);
        let mut inverse = settings.clone();
        inverse.invert = true;
        assert!(normal_dab.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings) > 0);
        assert!(inverse_dab.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &inverse) > 0);
        let normal_depth = normal_dab.positions[pole].z - 1.0;
        let inverse_depth = inverse_dab.positions[pole].z - 1.0;
        assert!((normal_depth + inverse_depth).abs() < 1e-6);
        assert!(normal_depth.abs() > 0.001);
        assert_eq!(normal_depth.is_sign_negative(), kind == BrushKind::Crease);
        normal_dab.validate().unwrap();
        inverse_dab.validate().unwrap();
    }
}

#[test]
fn symmetry_does_not_double_the_seam_and_moves_mirrored_sides() {
    let mut single = Mesh::sphere(3);
    let mut mirrored = single.clone();
    let settings = brush(BrushKind::Mask);
    let mut symmetric = settings.clone();
    symmetric.symmetry_x = true;
    let count = single.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings);
    assert_eq!(
        mirrored.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &symmetric),
        count
    );
    assert_eq!(mirrored.masks, single.masks);
    let original = Mesh::sphere(3);
    let mut moved = original.clone();
    let mut moving = brush(BrushKind::Move);
    moving.symmetry_x = true;
    moved.apply_brush(Vec3::Z, Vec3::Z, Vec3::new(0.2, 0.1, 0.0), &moving);
    for (i, p) in original
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p.z > 0.85)
    {
        let mirror = Vec3::new(-p.x, p.y, p.z);
        let j = original
            .positions
            .iter()
            .position(|q| q.distance(mirror) < 1e-5)
            .unwrap();
        let transformed = Vec3::new(
            -moved.positions[i].x,
            moved.positions[i].y,
            moved.positions[i].z,
        );
        assert!(transformed.distance(moved.positions[j]) < 1e-5);
    }
}

#[test]
fn paint_and_mask_do_not_move_vertices() {
    let mut mesh = Mesh::sphere(2);
    let original = mesh.positions.clone();
    let pole = mesh
        .positions
        .iter()
        .position(|p| p.distance(Vec3::Z) < 1e-6)
        .unwrap();
    assert!(mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &brush(BrushKind::Paint)) > 0);
    assert!(mesh.colors[pole][0] > 0.65);
    assert!(mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &brush(BrushKind::Mask)) > 0);
    let mask = mesh.masks[pole];
    assert!(mask > 0.0 && mask < 1.0);
    let mut erase = brush(BrushKind::Mask);
    erase.invert = true;
    mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &erase);
    assert!(mesh.masks[pole] < mask);
    assert_eq!(mesh.positions, original);
}

#[test]
fn smooth_reads_the_old_mesh_without_vertex_order_bias() {
    let original =
        Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3\nf 1 3 4\n").unwrap();
    let mut first = original.clone();
    let mut reordered = original.clone();
    reordered.positions.reverse();
    reordered.normals.reverse();
    reordered.colors.reverse();
    reordered.masks.reverse();
    for triangle in &mut reordered.triangles {
        for index in triangle {
            *index = 3 - *index;
        }
    }
    let mut settings = brush(BrushKind::Smooth);
    settings.radius = 3.0;
    first.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    reordered.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    reordered.positions.reverse();
    for (&a, &b) in first.positions.iter().zip(&reordered.positions) {
        assert!(a.distance(b) < 1e-6);
    }
    assert!(first.positions[0].length() < original.positions[0].length());
}

#[test]
fn flatten_reduces_distance_to_the_stamp_plane() {
    let mut mesh = Mesh::sphere(3);
    let original = mesh.positions.clone();
    assert!(mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &brush(BrushKind::Flatten)) > 0);
    for (i, p) in original.iter().enumerate().filter(|(_, p)| p.z > 0.9) {
        assert!((mesh.positions[i].z - 1.0).abs() <= (p.z - 1.0).abs() + 1e-7);
    }
}

#[test]
fn invalid_brush_inputs_are_a_noop() {
    let mut mesh = Mesh::sphere(1);
    let original = mesh.positions.clone();
    let mut settings = brush(BrushKind::Draw);
    settings.radius = f32::NAN;
    assert_eq!(mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings), 0);
    settings.radius = 1.0;
    settings.strength = f32::INFINITY;
    assert_eq!(mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings), 0);
    settings.strength = 0.5;
    assert_eq!(
        mesh.apply_brush(Vec3::Z, Vec3::ZERO, Vec3::ZERO, &settings),
        0
    );
    assert_eq!(mesh.positions, original);
}

#[test]
fn subdivision_preserves_surface_and_interpolates_attributes() {
    let mut mesh = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    mesh.masks = vec![0.0, 1.0, 0.0];
    mesh.colors = vec![[0.0; 3], [1.0; 3], [0.0; 3]];
    mesh.subdivide().unwrap();
    mesh.validate().unwrap();
    assert_eq!(mesh.positions.len(), 6);
    assert_eq!(mesh.triangles.len(), 4);
    let midpoint = mesh
        .positions
        .iter()
        .position(|&p| p == Vec3::new(0.5, 0.0, 0.0))
        .unwrap();
    assert_eq!(mesh.masks[midpoint], 0.5);
    assert_eq!(mesh.colors[midpoint], [0.5; 3]);
    assert!(mesh.normals.iter().all(|&n| n == Vec3::Z));
}

#[test]
fn smooth_subdivision_uses_boundary_rules_and_preserves_a_plane() {
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n").unwrap();
    mesh.subdivide_smooth().unwrap();
    mesh.validate().unwrap();
    assert_eq!(mesh.positions.len(), 9);
    assert_eq!(mesh.triangles.len(), 8);
    assert_eq!(mesh.positions[0], Vec3::new(0.125, 0.125, 0.0));
    assert_eq!(mesh.positions[1], Vec3::new(0.875, 0.125, 0.0));
    assert_eq!(mesh.positions[2], Vec3::new(0.875, 0.875, 0.0));
    assert_eq!(mesh.positions[3], Vec3::new(0.125, 0.875, 0.0));
    assert_eq!(mesh.positions[4], Vec3::new(0.5, 0.0, 0.0));
    assert_eq!(mesh.positions[6], Vec3::new(0.5, 0.5, 0.0));
    assert!(mesh.positions.iter().all(|position| position.z == 0.0));
    assert!(mesh.normals.iter().all(|&normal| normal == Vec3::Z));
}

#[test]
fn smooth_subdivision_uses_both_opposite_vertices_and_valence_weights() {
    let mut mesh = Mesh::sphere(0);
    let original = mesh.clone();
    let mut neighbors = std::collections::HashSet::new();
    for triangle in &original.triangles {
        if triangle.contains(&0) {
            neighbors.extend(triangle.iter().copied().filter(|&vertex| vertex != 0));
        }
    }
    assert_eq!(neighbors.len(), 5);
    let beta = (0.625 - (0.375 + 0.25 * (std::f32::consts::TAU / 5.0).cos()).powi(2)) / 5.0;
    let expected_vertex = original.positions[0] * (1.0 - 5.0 * beta)
        + neighbors
            .iter()
            .map(|&vertex| original.positions[vertex as usize] * beta)
            .sum::<Vec3>();
    let [a, b, c] = original.triangles[0];
    let opposite = original
        .triangles
        .iter()
        .skip(1)
        .find(|triangle| triangle.contains(&a) && triangle.contains(&b))
        .unwrap()
        .iter()
        .copied()
        .find(|&vertex| vertex != a && vertex != b)
        .unwrap();
    let expected_edge = (original.positions[a as usize] + original.positions[b as usize]) * 0.375
        + (original.positions[c as usize] + original.positions[opposite as usize]) * 0.125;
    mesh.subdivide_smooth().unwrap();
    assert!(mesh.positions[0].distance(expected_vertex) < 1e-6);
    assert!(mesh.positions[original.positions.len()].distance(expected_edge) < 1e-6);
    assert!(mesh.positions[0].distance(original.positions[0]) > 0.1);
    let midpoint = (original.positions[a as usize] + original.positions[b as usize]) * 0.5;
    assert!(expected_edge.distance(midpoint) > 0.01);
}

#[test]
fn repeated_smooth_subdivision_reduces_closed_sphere_faceting_with_expected_counts() {
    fn worst_radial_face_error(mesh: &Mesh) -> f32 {
        mesh.triangles
            .iter()
            .map(|triangle| {
                let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize]);
                let face_normal = (b - a).cross(c - a).normalize();
                [a, b, c]
                    .into_iter()
                    .map(|position| 1.0 - face_normal.dot(position.normalize()))
                    .fold(0.0, f32::max)
            })
            .fold(0.0, f32::max)
    }
    let mut mesh = Mesh::sphere(0);
    let mut previous_error = worst_radial_face_error(&mesh);
    for level in 1..=3 {
        mesh.subdivide_smooth().unwrap();
        mesh.validate().unwrap();
        assert_eq!(mesh.positions.len(), 10 * 4_usize.pow(level) + 2);
        assert_eq!(mesh.triangles.len(), 20 * 4_usize.pow(level));
        let error = worst_radial_face_error(&mesh);
        assert!(
            error < previous_error * 0.5,
            "{error} versus {previous_error}"
        );
        previous_error = error;
        assert!(mesh.positions.iter().all(|position| {
            let radius = position.length();
            (0.65..1.0).contains(&radius)
        }));
    }
}

#[test]
fn smooth_subdivision_interpolates_attributes_and_history_restores_every_field() {
    let mut mesh = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    mesh.masks = vec![0.0, 1.0, 0.0];
    mesh.colors = vec![[0.0; 3], [1.0; 3], [0.0; 3]];
    let original = mesh.clone();
    let mut document = Document::new(mesh);
    document.subdivide_smooth().unwrap();
    assert_eq!(
        document.require_active_mesh().unwrap().masks,
        vec![0.125, 0.75, 0.125, 0.5, 0.5, 0.0]
    );
    assert_eq!(
        document.require_active_mesh().unwrap().colors[0],
        [0.125; 3]
    );
    assert_eq!(document.require_active_mesh().unwrap().colors[1], [0.75; 3]);
    assert_eq!(document.require_active_mesh().unwrap().colors[3], [0.5; 3]);
    let refined = document.require_active_mesh().unwrap().clone();
    assert!(document.undo());
    assert!(document
        .require_active_mesh()
        .unwrap()
        .same_surface(&original));
    assert_eq!(
        document.require_active_mesh().unwrap().normals,
        original.normals
    );
    assert!(!document.can_undo());
    assert!(document.redo());
    assert!(document
        .require_active_mesh()
        .unwrap()
        .same_surface(&refined));
    assert_eq!(
        document.require_active_mesh().unwrap().normals,
        refined.normals
    );
    assert!(!document.can_redo());
}

#[test]
fn smooth_subdivision_refuses_bad_topology_atomically() {
    let mut invalid_index = Mesh::sphere(0);
    invalid_index.triangles[0][0] = u32::MAX;
    let mut duplicate = Mesh::sphere(0);
    duplicate.triangles.push(duplicate.triangles[0]);
    let mut inconsistent_winding = Mesh::sphere(0);
    inconsistent_winding.triangles[0].swap(0, 1);
    let mut degenerate = Mesh::sphere(0);
    let [a, b, _] = degenerate.triangles[0];
    degenerate.positions[a as usize] = degenerate.positions[b as usize];
    let non_manifold_edge = Mesh::new(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z],
        vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]],
    );
    let boundary_bow_tie = Mesh::new(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y],
        vec![[0, 1, 2], [0, 3, 4]],
    );
    // Two closed, individually manifold shells sharing only vertex zero have
    // no boundary or non-manifold edges, but the shared vertex has two fans.
    let mut closed_bow_tie = Mesh::sphere(0);
    let second = closed_bow_tie.clone();
    let offset = closed_bow_tie.positions.len() as u32 - 1;
    for position in second.positions.iter().skip(1) {
        closed_bow_tie
            .positions
            .push(*position + Vec3::new(0.0, 0.0, 3.0));
        closed_bow_tie.normals.push(Vec3::ZERO);
        closed_bow_tie.masks.push(0.0);
        closed_bow_tie.colors.push([0.5; 3]);
    }
    closed_bow_tie.triangles.extend(
        second
            .triangles
            .iter()
            .map(|triangle| triangle.map(|vertex| if vertex == 0 { 0 } else { vertex + offset })),
    );
    closed_bow_tie.recompute_normals();
    let mut isolated = Mesh::sphere(0);
    isolated.positions.push(Vec3::ZERO);
    isolated.normals.push(Vec3::ZERO);
    isolated.masks.push(0.0);
    isolated.colors.push([0.5; 3]);
    for mut mesh in [
        invalid_index,
        duplicate,
        inconsistent_winding,
        degenerate,
        non_manifold_edge,
        boundary_bow_tie,
        closed_bow_tie,
        isolated,
    ] {
        let original = mesh.clone();
        assert!(mesh.subdivide_smooth().is_err());
        assert!(mesh.same_surface(&original));
        assert_eq!(mesh.normals, original.normals);
    }
}

#[test]
fn refused_smooth_subdivision_preserves_limits_and_redo() {
    let mut document = Document::new(Mesh::sphere(0));
    document.subdivide_smooth().unwrap();
    assert!(document.undo());
    let original = document.require_active_mesh().unwrap().clone();
    document.require_active_mesh_mut().unwrap().triangles =
        vec![document.require_active_mesh().unwrap().triangles[0]; MAX_TRIANGLES / 4 + 1];
    let oversized = document.require_active_mesh().unwrap().clone();
    assert!(document.subdivide_smooth().is_err());
    assert!(document
        .require_active_mesh()
        .unwrap()
        .same_surface(&oversized));
    // Returning the public mesh to the unchanged history state demonstrates
    // that the refused operation did not discard the available redo entry.
    document.mesh = Some(original);
    assert!(document.can_redo());
    assert!(document.redo());
}

#[test]
fn smooth_subdivision_refuses_a_full_vertex_budget_before_building_topology() {
    let mut mesh = Mesh::sphere(0);
    mesh.positions.resize(MAX_VERTICES, Vec3::ZERO);
    mesh.normals.resize(MAX_VERTICES, Vec3::ZERO);
    mesh.masks.resize(MAX_VERTICES, 0.0);
    mesh.colors.resize(MAX_VERTICES, [0.5; 3]);
    mesh.validate().unwrap();
    let before = [
        mesh.positions.as_ptr() as usize,
        mesh.normals.as_ptr() as usize,
        mesh.triangles.as_ptr() as usize,
        mesh.masks.as_ptr() as usize,
        mesh.colors.as_ptr() as usize,
    ];
    let message = mesh.subdivide_smooth().unwrap_err();
    assert!(message.contains("one-million vertex limit"));
    assert_eq!(mesh.positions.len(), MAX_VERTICES);
    assert_eq!(mesh.triangles.len(), 20);
    assert_eq!(
        before,
        [
            mesh.positions.as_ptr() as usize,
            mesh.normals.as_ptr() as usize,
            mesh.triangles.as_ptr() as usize,
            mesh.masks.as_ptr() as usize,
            mesh.colors.as_ptr() as usize,
        ]
    );
    assert!(mesh.positions[12..]
        .iter()
        .all(|&value| value == Vec3::ZERO));
    assert!(mesh.masks.iter().all(|&value| value == 0.0));
}

#[test]
fn history_groups_whole_strokes_and_preserves_redo_after_a_noop() {
    let mesh = Mesh::sphere(2);
    let original = mesh.positions.clone();
    let mut document = Document::new(mesh);
    document.begin_stroke();
    document.require_active_mesh_mut().unwrap().apply_brush(
        Vec3::Z,
        Vec3::Z,
        Vec3::ZERO,
        &brush(BrushKind::Draw),
    );
    document.begin_stroke();
    document.require_active_mesh_mut().unwrap().apply_brush(
        Vec3::Z,
        Vec3::Z,
        Vec3::ZERO,
        &brush(BrushKind::Draw),
    );
    document.end_stroke();
    let changed = document.require_active_mesh().unwrap().positions.clone();
    assert!(document.can_undo());
    assert!(document.undo());
    assert_eq!(document.require_active_mesh().unwrap().positions, original);
    document.begin_stroke();
    document.end_stroke();
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.require_active_mesh().unwrap().positions, changed);
    assert!(!document.redo());
    assert!(document.undo());
    assert!(!document.undo());
    document.begin_stroke();
    document.require_active_mesh_mut().unwrap().masks[0] = 0.5;
    document.end_stroke();
    assert!(!document.can_redo());
}

#[test]
fn history_tracks_topology_masks_and_replacements() {
    let mut document = Document::new(Mesh::sphere(1));
    let original_count = document.require_active_mesh().unwrap().positions.len();
    document.subdivide().unwrap();
    assert!(document.require_active_mesh().unwrap().positions.len() > original_count);
    assert!(document.undo());
    assert_eq!(
        document.require_active_mesh().unwrap().positions.len(),
        original_count
    );
    assert!(document.redo());
    document.invert_mask();
    assert!(document
        .require_active_mesh_mut()
        .unwrap()
        .masks
        .iter()
        .all(|&m| m == 1.0));
    document.clear_mask();
    assert!(document
        .require_active_mesh_mut()
        .unwrap()
        .masks
        .iter()
        .all(|&m| m == 0.0));
    assert!(document.undo());
    assert!(document
        .require_active_mesh_mut()
        .unwrap()
        .masks
        .iter()
        .all(|&m| m == 1.0));
    document.replace_mesh(Mesh::cube()).unwrap();
    assert!(document.undo());
    assert!(document
        .require_active_mesh_mut()
        .unwrap()
        .masks
        .iter()
        .all(|&m| m == 1.0));
    let mut invalid = Mesh::sphere(0);
    invalid.masks.clear();
    let count = document.require_active_mesh().unwrap().positions.len();
    assert!(document.replace_mesh(invalid).is_err());
    assert_eq!(
        document.require_active_mesh().unwrap().positions.len(),
        count
    );
}

#[test]
fn obj_roundtrip_retains_geometry_winding_and_vertex_colors() {
    let mut mesh = Mesh::sphere(2);
    mesh.colors[0] = [0.1, 0.2, 0.3];
    let restored = Mesh::from_obj(&mesh.to_obj()).unwrap();
    assert_eq!(mesh.positions, restored.positions);
    assert_eq!(mesh.triangles, restored.triangles);
    assert_eq!(mesh.colors, restored.colors);
    for (&a, &b) in mesh.normals.iter().zip(&restored.normals) {
        assert!(a.distance(b) < 1e-6);
    }
}

#[test]
fn obj_supports_negative_slash_indices_and_concave_polygons() {
    let mesh = Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 2 2 0\nv 1 1 0\nv 0 2 0\nf -5/1/1 -4/2/1 -3/3/1 -2/4/1 -1/5/1 # concave\n").unwrap();
    assert_eq!(mesh.triangles.len(), 3);
    let area: f32 = mesh
        .triangles
        .iter()
        .map(|&[a, b, c]| {
            (mesh.positions[b as usize] - mesh.positions[a as usize])
                .cross(mesh.positions[c as usize] - mesh.positions[a as usize])
                .z
                * 0.5
        })
        .sum();
    assert!((area - 3.0).abs() < 1e-6);
    assert!(mesh.normals.iter().all(|&n| n == Vec3::Z));
}

#[test]
fn malformed_obj_and_structural_mesh_inputs_are_rejected() {
    for source in [
        "",
        "v NaN 0 0\n",
        "v 0 0 0\nf 1 2 3\n",
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 2 3\n",
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 1 3\n",
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/garbage 2 3\n",
    ] {
        assert!(
            Mesh::from_obj(source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
    let mut mesh = Mesh::sphere(0);
    mesh.triangles[0][0] = u32::MAX;
    assert!(mesh.validate().is_err());
    assert!(mesh.subdivide().is_err());
    mesh.recompute_normals(); // malformed public topology remains safe to inspect
    assert!(!mesh.to_obj().is_empty());
    let mut mesh = Mesh::sphere(0);
    mesh.positions[0].x = f32::NAN;
    assert!(mesh.validate().is_err());
    let mut mesh = Mesh::sphere(0);
    mesh.masks[0] = 1.1;
    assert!(mesh.validate().is_err());
    let mut mesh = Mesh::sphere(0);
    mesh.colors[0][1] = f32::INFINITY;
    assert!(mesh.validate().is_err());
}

#[test]
fn obj_rejects_self_intersecting_faces() {
    let mesh = "v 0 0 0\nv 3 0 0\nv 0 2 0\nv 2 2 0\nv 1 -1 0\nf 1 2 3 4 5\n";
    assert!(Mesh::from_obj(mesh).is_err());
}

#[test]
fn refused_subdivision_keeps_the_original_mesh() {
    let mut mesh = Mesh::sphere(0);
    mesh.triangles = vec![mesh.triangles[0]; MAX_TRIANGLES / 4 + 1];
    let positions = mesh.positions.clone();
    let triangle_count = mesh.triangles.len();
    assert!(mesh.subdivide().is_err());
    assert_eq!(mesh.positions, positions);
    assert_eq!(mesh.triangles.len(), triangle_count);
}

#[test]
fn history_obeys_default_and_configured_completed_operation_limits() {
    for limit in [15, 50] {
        let mut document = Document::new(Mesh::sphere(0));
        document.set_history_limit(limit).unwrap();
        for operation in 0..70 {
            document.begin_stroke();
            document.require_active_mesh_mut().unwrap().masks[0] = (operation + 1) as f32 / 100.0;
            document.end_stroke();
        }
        let mut undos = 0;
        while document.undo() {
            undos += 1;
        }
        assert_eq!(undos, limit);
    }
}

#[test]
fn versioned_project_roundtrips_and_rejects_bad_data() {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "My sculpture — مجسمه".into();
    document.require_active_mesh_mut().unwrap().masks[0] = 0.7;
    document.require_active_mesh_mut().unwrap().colors[2] = [0.2, 0.4, 0.8];
    let json = document.to_json().unwrap();
    let restored = Document::from_json(&json).unwrap();
    assert_eq!(restored.name, document.name);
    assert_eq!(
        restored.require_active_mesh().unwrap().positions,
        document.require_active_mesh().unwrap().positions
    );
    assert_eq!(
        restored.require_active_mesh().unwrap().masks,
        document.require_active_mesh().unwrap().masks
    );
    assert_eq!(
        restored.require_active_mesh().unwrap().colors,
        document.require_active_mesh().unwrap().colors
    );
    assert!(!restored.can_undo());
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut unsupported = value.clone();
    unsupported["version"] = serde_json::json!(99);
    assert!(Document::from_json(&unsupported.to_string()).is_err());
    value["mesh"]["triangles"][0][0] = serde_json::json!(u32::MAX);
    assert!(Document::from_json(&value.to_string()).is_err());
    value["mesh"]["masks"] = serde_json::json!([]);
    assert!(Document::from_json(&value.to_string()).is_err());
    assert!(Document::from_json("{truncated").is_err());
}

#[test]
fn smoothing_cache_is_invalidated_after_same_size_public_topology_edits() {
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n").unwrap();
    let mut settings = brush(BrushKind::Smooth);
    settings.radius = 10.0;
    mesh.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    mesh.triangles = vec![[0, 1, 3], [1, 2, 3]];
    mesh.recompute_normals();
    let mut without_cache = Mesh::from_obj(&mesh.to_obj()).unwrap();
    mesh.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    without_cache.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    assert_eq!(mesh.positions, without_cache.positions);
}
#[test]
fn local_normal_update_matches_complete_rebuild() {
    let mut mesh = Mesh::sphere(3);
    let dirty = [0, 12, 40];
    for &vertex in &dirty {
        mesh.positions[vertex] += Vec3::new(0.012, -0.023, 0.031);
    }
    let mut complete = mesh.snapshot();
    complete.recompute_normals();
    mesh.recompute_normals_near_vertices(&dirty);
    assert_eq!(mesh.normals, complete.normals);
}

#[test]
fn compressed_adjacency_preserves_sorted_neighbors_and_public_edits() {
    let mut mesh = Mesh::new(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE],
        vec![[3, 0, 1], [0, 4, 1], [2, 0, 4]],
    );
    mesh.ensure_adjacency();
    assert_eq!(mesh.cached_neighbors(0), &[1, 2, 3, 4]);
    assert_eq!(mesh.cached_neighbors(1), &[0, 3, 4]);
    assert_eq!(mesh.cached_neighbors(2), &[0, 4]);
    mesh.triangles[2] = [2, 3, 4];
    mesh.ensure_adjacency();
    assert_eq!(mesh.cached_neighbors(0), &[1, 3, 4]);
    assert_eq!(mesh.cached_neighbors(2), &[3, 4]);
}

#[test]
fn local_normal_update_covers_unchanged_neighbor_faces() {
    let mut mesh = Mesh::cube();
    mesh.positions[0] += Vec3::new(0.03, 0.02, 0.04);
    let mut complete = mesh.snapshot();
    complete.recompute_normals();
    mesh.recompute_normals_near_vertices(&[0]);
    assert_eq!(mesh.normals, complete.normals);
}
