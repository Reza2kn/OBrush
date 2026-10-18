//! Independently generated deformation regressions. These establish local mesh
//! safety and useful editing, not ZBrush brush equivalence or self-intersection
//! prevention. In particular, valid face winding does not prove an embedding.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};

fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        // Application defaults are intentionally part of these regressions.
        radius: 0.18,
        strength: 0.35,
        ..BrushSettings::default()
    }
}

fn hit(mesh: &Mesh, x: f32, y: f32) -> obrush_core::Hit {
    mesh.raycast(Ray {
        origin: Vec3::new(x, y, 3.0),
        direction: -Vec3::Z,
    })
    .expect("the sculpt remains visible to an independent ray")
}

fn assert_surface_equal(first: &Mesh, second: &Mesh) {
    assert_eq!(first.positions, second.positions);
    assert_eq!(first.triangles, second.triangles);
    assert_eq!(first.normals, second.normals);
    assert_eq!(first.masks, second.masks);
    assert_eq!(first.colors, second.colors);
}

fn face_geometry(mesh: &Mesh, triangle: [u32; 3]) -> (glam::DVec3, f64) {
    let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
    let cross = (b - a).cross(c - a);
    let longest_squared = (b - a)
        .length_squared()
        .max((c - b).length_squared())
        .max((a - c).length_squared());
    (cross, cross.length() / longest_squared)
}

fn assert_no_local_collapse_or_reversal(before: &Mesh, after: &Mesh) -> (f64, f64) {
    // Local conforming refinement keeps all old vertex IDs. Check the moved
    // parent geometry before considering its children; a split must not hide a
    // reversed deformation behind smaller replacement triangles.
    assert!(after.positions.len() >= before.positions.len());
    assert!(after.triangles.len() >= before.triangles.len());
    after.validate().unwrap();
    let mut minimum_quality = f64::INFINITY;
    let mut minimum_projected_area_fraction = f64::INFINITY;
    for &triangle in &before.triangles {
        let (old_cross, old_quality) = face_geometry(before, triangle);
        let (new_cross, new_quality) = face_geometry(after, triangle);
        assert!(new_cross.is_finite());
        assert!(new_quality.is_finite());
        assert!(
            new_cross.dot(old_cross) > 0.0,
            "face {triangle:?} reversed or collapsed"
        );
        // A regular sphere/sheet may become a narrow useful crease, but must
        // retain measurable area. An existing needle must not become thinner.
        let required_quality = old_quality.min(0.015);
        assert!(
            new_quality >= required_quality * 0.999,
            "face {triangle:?} quality {old_quality} became {new_quality}"
        );
        minimum_projected_area_fraction = minimum_projected_area_fraction
            .min(new_cross.dot(old_cross) / old_cross.length_squared());
    }
    for &triangle in &after.triangles {
        let (cross, quality) = face_geometry(after, triangle);
        assert!(cross.is_finite() && cross.length_squared() > 0.0);
        assert!(
            quality.is_finite() && quality >= 0.015 * 0.999,
            "refined face {triangle:?} has quality {quality}"
        );
        minimum_quality = minimum_quality.min(quality);
    }
    (minimum_quality, minimum_projected_area_fraction)
}

fn sheet() -> Mesh {
    Mesh::from_obj(
        "v -1 -1 0\nv 0 -1 0\nv 1 -1 0\n\
         v -1 0 0\nv 0 0 0\nv 1 0 0\n\
         v -1 1 0\nv 0 1 0\nv 1 1 0\n\
         f 1 2 5\nf 1 5 4\nf 2 3 6\nf 2 6 5\n\
         f 4 5 8\nf 4 8 7\nf 5 6 9\nf 5 9 8\n",
    )
    .unwrap()
}

fn shared_edge_fold_angle(mesh: &Mesh) -> f64 {
    let (first, _) = face_geometry(mesh, mesh.triangles[0]);
    let (second, _) = face_geometry(mesh, mesh.triangles[1]);
    first
        .normalize()
        .dot(second.normalize())
        .clamp(-1.0, 1.0)
        .acos()
}

#[test]
fn repeated_default_crease_creates_a_measurable_groove_without_collapsing_faces() {
    let mut mesh = Mesh::sphere(4);
    let original = mesh.clone();
    let pole = mesh
        .positions
        .iter()
        .position(|position| position.distance(Vec3::Z) < 1e-6)
        .unwrap();
    let crease = settings(BrushKind::Crease);
    let mut changed_dabs = 0;
    let mut minimum_quality = f64::INFINITY;
    let mut minimum_projected_area_fraction = f64::INFINITY;
    for _ in 0..80 {
        let visible = hit(&mesh, 0.0, 0.0);
        let before = mesh.clone();
        let changed = mesh.apply_brush(visible.position, visible.normal, Vec3::ZERO, &crease);
        changed_dabs += usize::from(changed != 0);
        let (quality, fraction) = assert_no_local_collapse_or_reversal(&before, &mesh);
        minimum_quality = minimum_quality.min(quality);
        minimum_projected_area_fraction = minimum_projected_area_fraction.min(fraction);
    }
    let groove_depth = original.positions[pole].z - mesh.positions[pole].z;
    assert!(
        groove_depth > 0.025,
        "crease became an ineffective no-op: {groove_depth}"
    );
    assert!(changed_dabs > 20);
    for (vertex, &position) in original.positions.iter().enumerate() {
        if position.distance(Vec3::Z) > 0.5 {
            assert_eq!(mesh.positions[vertex], position);
        }
    }
    assert_eq!(&mesh.masks[..original.masks.len()], original.masks);
    assert_eq!(&mesh.colors[..original.colors.len()], original.colors);
    println!(
        "app-default stationary Crease: 80 dabs, sphere4, depth={groove_depth}, changed_dabs={changed_dabs}, minimum_quality={minimum_quality}, minimum_projected_area_fraction={minimum_projected_area_fraction}"
    );
}

#[test]
fn repeated_visible_crease_strokes_remain_stable_across_a_denser_curved_surface() {
    let mut mesh = Mesh::sphere(6);
    let original = mesh.clone();
    let crease = settings(BrushKind::Crease);
    let mut changed_vertices = 0;
    let mut minimum_quality = f64::INFINITY;
    let mut minimum_projected_area_fraction = f64::INFINITY;
    for pass in 0..4 {
        for step in 0..41 {
            let index = if pass % 2 == 0 { step } else { 40 - step };
            let visible = hit(&mesh, -0.35 + index as f32 * 0.0175, 0.05);
            let before = mesh.clone();
            changed_vertices +=
                mesh.apply_brush(visible.position, visible.normal, Vec3::ZERO, &crease);
            let (quality, fraction) = assert_no_local_collapse_or_reversal(&before, &mesh);
            minimum_quality = minimum_quality.min(quality);
            minimum_projected_area_fraction = minimum_projected_area_fraction.min(fraction);
        }
    }
    assert!(
        changed_vertices > 500,
        "the stroke must edit a useful patch"
    );
    let maximum_displacement = mesh
        .positions
        .iter()
        .zip(&original.positions)
        .map(|(after, before)| after.distance(*before))
        .fold(0.0, f32::max);
    assert!(maximum_displacement > 0.015);
    println!(
        "app-default moving Crease: 4x41 dabs, sphere6, maximum_displacement={maximum_displacement}, changed_vertices_sum={changed_vertices}, minimum_quality={minimum_quality}, minimum_projected_area_fraction={minimum_projected_area_fraction}"
    );
}

#[test]
fn crease_preserves_protected_vertices_attributes_and_mirrored_shape() {
    let mut mesh = Mesh::sphere(4);
    for (vertex, position) in mesh.positions.iter().enumerate() {
        if position.z > 0.85 && position.x.abs() > 0.25 && position.y > 0.0 {
            mesh.masks[vertex] = 1.0;
        } else if position.z > 0.9 && position.x.abs() < 0.2 {
            mesh.masks[vertex] = 0.5;
        }
        mesh.colors[vertex] = [position.x.abs(), position.y.abs(), position.z.abs()];
    }
    let original = mesh.clone();
    assert!(original.masks.iter().filter(|&&mask| mask == 1.0).count() > 8);
    assert!(original.masks.iter().filter(|&&mask| mask == 0.5).count() > 8);
    let mirror_pairs: Vec<_> = mesh
        .positions
        .iter()
        .enumerate()
        .filter(|(_, position)| position.z > 0.7 && position.x >= 0.0)
        .map(|(vertex, position)| {
            let reflected = Vec3::new(-position.x, position.y, position.z);
            let mirror = mesh
                .positions
                .iter()
                .position(|candidate| candidate.distance(reflected) < 1e-6)
                .unwrap();
            (vertex, mirror)
        })
        .collect();
    assert!(mirror_pairs.len() > 100);
    let crease = settings(BrushKind::Crease);
    let mut changed = 0;
    for _ in 0..35 {
        let visible = hit(&mesh, 0.22, 0.04);
        let before = mesh.clone();
        changed += mesh.apply_brush(visible.position, visible.normal, Vec3::ZERO, &crease);
        assert_no_local_collapse_or_reversal(&before, &mesh);
        for &(vertex, mirror) in &mirror_pairs {
            let position = mesh.positions[vertex];
            let reflected = Vec3::new(-position.x, position.y, position.z);
            assert!(reflected.distance(mesh.positions[mirror]) < 2e-5);
        }
        for (vertex, &mask) in original.masks.iter().enumerate() {
            if mask == 1.0 {
                assert_eq!(mesh.positions[vertex], original.positions[vertex]);
            }
        }
    }
    assert!(changed > 0);
    assert_eq!(&mesh.masks[..original.masks.len()], original.masks);
    assert_eq!(&mesh.colors[..original.colors.len()], original.colors);
}

#[test]
fn large_move_reports_the_applied_fraction_and_preserves_undoable_sheet_geometry() {
    let mut document = Document::new(sheet());
    let original = document.active_mesh().unwrap().clone();
    let moving = BrushSettings {
        radius: 0.8,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    let requested = Vec3::new(4.0, 0.0, 0.0);
    document.begin_stroke();
    let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
        Vec3::ZERO,
        Vec3::Z,
        requested,
        &moving,
    );
    document.end_stroke();
    assert_eq!(result.changed_vertices, 1);
    assert!((0.0..1.0).contains(&result.displacement_scale));
    assert!(result.displacement_scale > 0.0);
    let applied = document.active_mesh().unwrap().positions[4] - original.positions[4];
    assert!(applied.distance(requested * result.displacement_scale) < 1e-6);
    for vertex in [0, 1, 2, 3, 5, 6, 7, 8] {
        assert_eq!(
            document.active_mesh().unwrap().positions[vertex],
            original.positions[vertex]
        );
    }
    assert_no_local_collapse_or_reversal(&original, document.active_mesh().unwrap());
    let edited = document.active_mesh().unwrap().clone();
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_surface_equal(document.active_mesh().unwrap(), &edited);
}

#[test]
fn a_needle_that_cannot_accept_a_step_remains_unchanged_and_preserves_redo() {
    let needle = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0.5 0.0001 0\nf 1 2 3\n").unwrap();
    let mut document = Document::new(needle);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[2] = 0.25;
    document.end_stroke();
    assert!(document.undo());
    assert!(document.can_redo());
    let original = document.active_mesh().unwrap().clone();
    let moving = BrushSettings {
        radius: 0.00004,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    document.begin_stroke();
    let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
        original.positions[2],
        Vec3::Z,
        Vec3::new(0.0, -0.1, 0.0),
        &moving,
    );
    document.end_stroke();
    assert_eq!(result.changed_vertices, 0);
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.active_mesh().unwrap().masks[2], 0.25);
}

#[test]
fn a_touched_collapsed_face_rejects_deformation_without_creating_history() {
    let mut collapsed = sheet();
    collapsed.positions[4] = collapsed.positions[1];
    collapsed.recompute_normals();
    collapsed.validate().unwrap();
    let original = collapsed.clone();
    let center = collapsed.positions[4];
    let mut document = Document::new(collapsed);
    let draw = BrushSettings {
        radius: 0.2,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Draw)
    };
    document.begin_stroke();
    let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
        center,
        Vec3::Z,
        Vec3::ZERO,
        &draw,
    );
    document.end_stroke();
    assert_eq!(result.changed_vertices, 0);
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
}

#[test]
fn a_fully_masked_sheet_rejects_a_large_move_without_creating_history() {
    let mut masked = sheet();
    masked.masks.fill(1.0);
    let original = masked.clone();
    let mut document = Document::new(masked);
    document.begin_stroke();
    let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
        Vec3::ZERO,
        Vec3::Z,
        Vec3::new(4.0, 0.0, 2.0),
        &BrushSettings {
            radius: 2.0,
            strength: 1.0,
            ..settings(BrushKind::Move)
        },
    );
    document.end_stroke();
    assert_eq!(result.changed_vertices, 0);
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
}

#[test]
fn lifting_a_mask_bounded_edge_cannot_create_a_back_to_back_fold() {
    let mut mesh =
        Mesh::from_obj("v -1 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 2 1 4\n").unwrap();
    mesh.masks[2..].fill(1.0);
    let original = mesh.clone();
    let moving = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    let result =
        mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 10.0), &moving);
    assert_eq!(result.changed_vertices, 2);
    assert!(result.displacement_scale > 0.0 && result.displacement_scale < 1.0);
    // Merely preserving each face's orientation against its old normal would
    // permit a nearly closed fold: both faces keep positive projected area.
    assert_no_local_collapse_or_reversal(&original, &mesh);
    let fold_angle = shared_edge_fold_angle(&mesh);
    assert!(fold_angle.to_degrees() <= 150.0 + 1e-5);
    assert!(
        mesh.positions[0].z > 1.0,
        "the guard must still permit a useful lift"
    );
    assert_eq!(mesh.positions[0].z, mesh.positions[1].z);
    assert_eq!(mesh.positions[2], original.positions[2]);
    assert_eq!(mesh.positions[3], original.positions[3]);
    assert_eq!(mesh.masks, original.masks);
    assert_eq!(mesh.colors, original.colors);
    println!(
        "mask-bounded edge lift: requested_height=10, applied_height={}, applied_scale={}, fold_angle_degrees={}",
        mesh.positions[0].z,
        result.displacement_scale,
        fold_angle.to_degrees()
    );
}

#[test]
fn same_size_public_topology_edits_refresh_fold_protection() {
    // The initial two faces share just a vertex. A small legal edit builds any
    // topology cache before the caller changes the second face, without changing
    // either the vertex count or the face count.
    let mut cached =
        Mesh::from_obj("v -1 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 2 -1 0\nf 1 2 3\nf 2 4 5\n")
            .unwrap();
    cached.masks[2..].fill(1.0);
    let initial = cached.clone();
    let moving = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    assert!(
        cached
            .apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 0.01), &moving)
            .changed_vertices
            > 0
    );
    cached.positions.clone_from(&initial.positions);
    cached.triangles[1] = [1, 0, 3];
    cached.recompute_normals();
    let edited = cached.clone();
    let mut fresh = Mesh::from_obj(&cached.to_obj()).unwrap();
    fresh.masks.clone_from(&cached.masks);
    let requested = Vec3::new(0.0, 0.0, 10.0);
    let reused_result = cached.apply_brush_with_result(Vec3::ZERO, Vec3::Z, requested, &moving);
    let fresh_result = fresh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, requested, &moving);
    assert!(reused_result.changed_vertices > 0);
    assert!(reused_result.displacement_scale > 0.0 && reused_result.displacement_scale < 1.0);
    assert_eq!(
        reused_result.changed_vertices,
        fresh_result.changed_vertices
    );
    assert_eq!(
        reused_result.displacement_scale,
        fresh_result.displacement_scale
    );
    assert_surface_equal(&cached, &fresh);
    assert_no_local_collapse_or_reversal(&edited, &cached);
    assert!(shared_edge_fold_angle(&cached).to_degrees() <= 150.0 + 1e-5);
}

#[test]
fn a_masked_picked_center_stays_anchored_when_its_unmasked_fringe_moves() {
    let mut mesh = sheet();
    mesh.masks[4] = 1.0;
    let picked = hit(&mesh, 0.0, 0.0);
    let original = mesh.clone();
    let moving = BrushSettings {
        radius: 1.8,
        strength: 0.75,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    let result = mesh.apply_brush_with_result(
        picked.position,
        picked.normal,
        Vec3::new(0.2, 0.0, 0.1),
        &moving,
    );
    assert!(result.changed_vertices > 0);
    assert!(mesh
        .positions
        .iter()
        .zip(&original.positions)
        .any(|(after, before)| after != before));
    assert_eq!(mesh.positions[4], original.positions[4]);
    let (anchor, anchor_normal) = picked.surface_point(&mesh).unwrap();
    assert_eq!(anchor, picked.position);
    assert!(anchor_normal.is_finite());
    assert!((anchor_normal.length() - 1.0).abs() < 1e-6);
    assert_no_local_collapse_or_reversal(&original, &mesh);
}

#[test]
fn a_partially_masked_move_anchor_follows_the_picked_material_point() {
    let mut mesh = sheet();
    mesh.masks[4] = 0.5;
    mesh.masks[5] = 0.75;
    let picked = hit(&mesh, 0.25, 0.125);
    assert_eq!(picked.triangle, [4, 5, 8]);
    assert!(picked.position.distance(Vec3::new(0.25, 0.125, 0.0)) < 1e-6);
    let original = mesh.clone();
    let moving = BrushSettings {
        radius: 1.8,
        strength: 0.7,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    let requested = Vec3::new(0.2, 0.05, 0.1);
    let result = mesh.apply_brush_with_result(picked.position, picked.normal, requested, &moving);
    assert!(result.changed_vertices > 0);
    let (anchor, anchor_normal) = picked.surface_point(&mesh).unwrap();
    // The authored face and picked coordinates establish these weights
    // independently: (0.25,0.125)=.75*(0,0)+.125*(1,0)+.125*(1,1).
    let expected = mesh.positions[4] * 0.75 + mesh.positions[5] * 0.125 + mesh.positions[8] * 0.125;
    let expected_normal =
        (mesh.normals[4] * 0.75 + mesh.normals[5] * 0.125 + mesh.normals[8] * 0.125).normalize();
    assert!(anchor.distance(expected) < 1e-6);
    assert!(anchor_normal.dot(expected_normal) > 0.99999);
    assert!(anchor.distance(picked.position) > 0.01);
    let guessed = picked.position + requested * moving.strength * result.displacement_scale;
    assert!(anchor.distance(guessed) > 0.01);
    assert_no_local_collapse_or_reversal(&original, &mesh);
}

#[test]
fn an_x_symmetry_seam_anchor_stays_on_the_seam_while_the_sides_move() {
    let mut mesh = sheet();
    let picked = hit(&mesh, 0.0, 0.0);
    let original = mesh.clone();
    let moving = BrushSettings {
        radius: 1.8,
        strength: 0.75,
        symmetry_x: true,
        ..settings(BrushKind::Move)
    };
    let result = mesh.apply_brush_with_result(
        picked.position,
        picked.normal,
        Vec3::new(0.25, 0.0, 0.0),
        &moving,
    );
    assert!(result.changed_vertices > 0);
    assert_ne!(mesh.positions[3], original.positions[3]);
    assert_ne!(mesh.positions[5], original.positions[5]);
    for vertex in [1, 4, 7] {
        assert_eq!(mesh.positions[vertex].x, 0.0);
    }
    let (anchor, _) = picked.surface_point(&mesh).unwrap();
    assert_eq!(anchor.x, 0.0);
    assert_eq!(anchor, picked.position);
    assert_no_local_collapse_or_reversal(&original, &mesh);
}

#[test]
fn touching_the_third_face_of_a_non_manifold_edge_rejects_geometry_but_allows_attributes() {
    let mesh = Mesh::from_obj(
        "v -1 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 0 0 1\n\
         f 1 2 3\nf 2 1 4\nf 1 2 5\n",
    )
    .unwrap();
    let mut document = Document::new(mesh);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[0] = 0.25;
    document.end_stroke();
    assert!(document.undo());
    let original = document.active_mesh().unwrap().clone();
    let picked = document
        .active_mesh()
        .unwrap()
        .raycast(Ray {
            origin: Vec3::new(0.0, -3.0, 0.6),
            direction: Vec3::Y,
        })
        .unwrap();
    assert_eq!(picked.triangle, [0, 1, 4]);
    let moving = BrushSettings {
        radius: 0.7,
        strength: 0.75,
        symmetry_x: false,
        ..settings(BrushKind::Move)
    };
    document.begin_stroke();
    let rejected = document.active_mesh_mut().unwrap().apply_brush_with_result(
        picked.position,
        picked.normal,
        Vec3::new(0.0, -0.15, 0.0),
        &moving,
    );
    document.end_stroke();
    assert_eq!(rejected.changed_vertices, 0);
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.can_redo());
    let tip = document.active_mesh().unwrap().positions[4];
    let tip_normal = document.active_mesh().unwrap().normals[4];
    let masking = BrushSettings {
        radius: 0.7,
        strength: 1.0,
        symmetry_x: false,
        ..settings(BrushKind::Mask)
    };
    document.begin_stroke();
    assert!(
        document
            .active_mesh_mut()
            .unwrap()
            .apply_brush(tip, tip_normal, Vec3::ZERO, &masking)
            > 0
    );
    document.end_stroke();
    assert!(document.active_mesh().unwrap().masks[4] > 0.0);
    let painting = BrushSettings {
        kind: BrushKind::Paint,
        color: [0.1, 0.9, 0.2],
        ..masking
    };
    document.begin_stroke();
    assert!(
        document
            .active_mesh_mut()
            .unwrap()
            .apply_brush(tip, tip_normal, Vec3::ZERO, &painting)
            > 0
    );
    document.end_stroke();
    assert_ne!(
        document.active_mesh().unwrap().colors[4],
        original.colors[4]
    );
    assert_eq!(
        document.active_mesh().unwrap().positions,
        original.positions
    );
    assert_eq!(
        document.active_mesh().unwrap().triangles,
        original.triangles
    );
    assert_eq!(document.active_mesh().unwrap().normals, original.normals);
}
