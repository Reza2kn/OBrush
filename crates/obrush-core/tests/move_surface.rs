//! Independent Move gesture contracts, not ZBrush parity or collision repair.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, RasterAlpha, Ray, Vec3, MAX_VERTICES};
use std::collections::BTreeMap;

fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Move,
        radius: 0.037,
        strength: 0.79,
        symmetry_x: true,
        ..BrushSettings::default()
    }
}

fn picked(mesh: &Mesh, x: f32) -> obrush_core::Hit {
    mesh.raycast(Ray {
        origin: Vec3::new(x, 0.0, 3.0),
        direction: -Vec3::Z,
    })
    .unwrap()
}

fn exact(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.masks, b.masks);
    assert_eq!(a.colors, b.colors);
}

fn geometry(mesh: &Mesh) -> (f64, f64) {
    mesh.validate().unwrap();
    let mut quality = f64::INFINITY;
    let mut fold: f64 = 0.0;
    let mut volume = 0.0;
    let mut edges = BTreeMap::<(u32, u32), Vec<(bool, glam::DVec3)>>::new();
    for &face in &mesh.triangles {
        let [a, b, c] = face.map(|v| mesh.positions[v as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let longest = a
            .distance_squared(b)
            .max(b.distance_squared(c))
            .max(c.distance_squared(a));
        assert!(cross.is_finite() && cross.length_squared() > 0.0);
        quality = quality.min(cross.length() / longest);
        volume += a.dot(b.cross(c)) / 6.0;
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((a < b, cross.normalize()));
        }
    }
    for faces in edges.values() {
        assert_eq!(faces.len(), 2, "closed, manifold edge incidence");
        assert_ne!(faces[0].0, faces[1].0, "consistent face winding");
        fold = fold.max(
            faces[0]
                .1
                .dot(faces[1].1)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees(),
        );
    }
    assert!(volume > 3.0, "sphere keeps outward signed volume: {volume}");
    (quality, fold)
}

#[test]
fn small_long_drag_has_useful_smooth_closed_geometry_and_one_undo_step() {
    let mut document = Document::new(Mesh::sphere(5));
    let original = document.active_mesh().unwrap().snapshot();
    let hit = picked(document.active_mesh().unwrap(), 0.3);
    let mut moving = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    exact(document.active_mesh().unwrap(), &original);
    document.begin_stroke();
    let mut added = 0;
    for sample in 1..=120 {
        let result = moving.apply(
            document.active_mesh_mut().unwrap(),
            Vec3::Z * (0.6 * sample as f32 / 120.0),
            &settings(),
        );
        added += result.refined_vertices;
    }
    document.end_stroke();
    let displacement = original
        .positions
        .iter()
        .zip(&document.active_mesh().unwrap().positions)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max);
    let (quality, fold) = geometry(document.active_mesh().unwrap());
    assert!(added > 0 && added <= 2048);
    let extension = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .map(|p| p.length() - 1.0)
        .fold(0.0, f32::max);
    assert!(
        extension > settings().radius,
        "retain a useful refined-surface pull: {extension} (original vertices {displacement})"
    );
    assert!(
        quality >= 0.1 * (1.0 - 1e-4),
        "fresh refined surface quality: {quality}"
    );
    assert!(fold <= 90.0 + 1e-4, "fresh adjacent fold: {fold}");
    let edited = document.active_mesh().unwrap().snapshot();
    assert!(document.undo());
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.redo());
    exact(document.active_mesh().unwrap(), &edited);
    println!(
        "small Move: displacement={displacement}, added={added}, quality={quality}, fold={fold}"
    );
}

#[test]
fn absolute_targets_are_event_rate_independent_and_return_exactly_to_origin() {
    let original = Mesh::sphere(5);
    let hit = picked(&original, 0.3);
    let mut one = original.snapshot();
    let mut many = original.snapshot();
    let mut single = one
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let mut sampled = many
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let target = Vec3::new(0.0, 0.1, 0.2);
    single.apply(&mut one, target, &settings());
    for i in 1..=120 {
        sampled.apply(&mut many, target * (i as f32 / 120.0), &settings());
    }
    exact(&one, &many);
    sampled.apply(&mut many, Vec3::ZERO, &settings());
    exact(&many, &original);
    sampled.apply(&mut many, target, &settings());
    exact(&many, &one);
}

#[test]
fn guard_clamping_does_not_make_a_collinear_pull_jump_backwards() {
    let mut mesh = Mesh::sphere(5);
    let hit = picked(&mesh, 0.3);
    let mut moving = mesh
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let mut previous = 0.0;
    let mut limited = false;
    for i in 1..=240 {
        let result = moving.apply(&mut mesh, Vec3::Z * (i as f32 / 240.0), &settings());
        let moved = mesh
            .positions
            .iter()
            .map(|p| p.length() - 1.0)
            .fold(0.0, f32::max);
        assert!(
            moved + 0.000_05 >= previous,
            "guard discontinuity {previous}->{moved}"
        );
        previous = moved;
        limited |= result.displacement_scale > 0.0 && result.displacement_scale < 1.0;
    }
    assert!(limited && previous > settings().radius);
}

#[test]
fn frozen_mask_color_and_mirrored_response_survive_long_targets() {
    let mut mesh = Mesh::sphere(5);
    for (v, &p) in mesh.positions.iter().enumerate() {
        mesh.colors[v] = [(p.x + 1.0) * 0.5, (p.y + 1.0) * 0.5, (p.z + 1.0) * 0.5];
        if p.y < 0.0 {
            mesh.masks[v] = 1.0;
        } else if p.y > 0.1 {
            mesh.masks[v] = 0.5;
        }
    }
    let original = mesh.snapshot();
    let hit = picked(&mesh, 0.01);
    let mut moving = mesh
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let result = moving.apply(&mut mesh, Vec3::new(0.1, 0.02, 0.1), &settings());
    assert!(result.changed_vertices > 0);
    assert_eq!(&mesh.masks[..original.masks.len()], &original.masks);
    assert_eq!(&mesh.colors[..original.colors.len()], &original.colors);
    for (v, &p) in original.positions.iter().enumerate() {
        if original.masks[v] == 1.0 {
            assert_eq!(mesh.positions[v], p);
        }
        if p.x == 0.0 {
            assert_eq!(mesh.positions[v].x, 0.0);
        }
        let reflected = Vec3::new(-p.x, p.y, p.z);
        if let Some(pair) = original.positions.iter().position(|&q| q == reflected) {
            let p = mesh.positions[v];
            assert!(Vec3::new(-p.x, p.y, p.z).distance(mesh.positions[pair]) < 2e-6);
        }
    }
    assert!(mesh.masks.iter().all(|v| (0.0..=1.0).contains(v)));
    assert!(mesh
        .colors
        .iter()
        .flatten()
        .all(|v| (0.0..=1.0).contains(v)));
    geometry(&mesh);
}

#[test]
fn zero_contact_black_alpha_full_mask_and_invalid_or_stale_inputs_are_atomic() {
    for state in 0..3 {
        let mut mesh = Mesh::sphere(3);
        let mut brush = settings();
        if state == 0 {
            brush.pressure = 0.0;
        } else if state == 1 {
            brush.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
        } else {
            mesh.masks.fill(1.0);
        }
        let original = mesh.snapshot();
        let hit = picked(&mesh, 0.0);
        let mut moving = mesh
            .prepare_move_stroke(hit.position, hit.normal, &brush)
            .unwrap();
        assert_eq!(
            moving
                .apply(&mut mesh, Vec3::Z * 0.03, &brush)
                .changed_vertices,
            0
        );
        exact(&mesh, &original);
        if state == 0 {
            brush.pressure = 0.5;
            assert!(
                moving
                    .apply(&mut mesh, Vec3::Z * 0.03, &brush)
                    .changed_vertices
                    > 0
            );
        }
    }
    let mut mesh = Mesh::sphere(3);
    let hit = picked(&mesh, 0.0);
    let original = mesh.snapshot();
    let mut moving = mesh
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    assert_eq!(
        moving
            .apply(&mut mesh, Vec3::splat(f32::NAN), &settings())
            .changed_vertices,
        0
    );
    exact(&mesh, &original);
    mesh.colors[0][0] = 0.12;
    let stale = mesh.snapshot();
    assert_eq!(
        moving
            .apply(&mut mesh, Vec3::Z * 0.03, &settings())
            .changed_vertices,
        0
    );
    exact(&mesh, &stale);
    assert!(mesh
        .prepare_move_stroke(hit.position, Vec3::ZERO, &settings())
        .is_err());
}

#[test]
fn return_to_origin_preserves_redo_and_hierarchy_move_keeps_correspondence() {
    let mut document = Document::new(Mesh::sphere(3));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.3];
    document.end_stroke();
    assert!(document.undo());
    let original = document.active_mesh().unwrap().snapshot();
    let hit = picked(document.active_mesh().unwrap(), 0.0);
    let mut moving = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    document.begin_stroke();
    moving.apply(
        document.active_mesh_mut().unwrap(),
        Vec3::Z * 0.03,
        &settings(),
    );
    moving.apply(document.active_mesh_mut().unwrap(), Vec3::ZERO, &settings());
    document.end_stroke();
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.can_redo());
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    let before = document.to_json().unwrap();
    let topology = document.active_mesh().unwrap().triangles.clone();
    let count = document.active_mesh().unwrap().positions.len();
    let brush = BrushSettings {
        adaptive_detail: false,
        ..settings()
    };
    let hit = picked(document.active_mesh().unwrap(), 0.0);
    let mut moving = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &brush)
        .unwrap();
    document.begin_stroke();
    let result = moving.apply(document.active_mesh_mut().unwrap(), Vec3::Z * 0.01, &brush);
    document.end_stroke();
    assert!(result.changed_vertices > 0);
    assert_eq!(result.refined_vertices, 0);
    assert_eq!(document.active_mesh().unwrap().triangles, topology);
    assert_eq!(document.active_mesh().unwrap().positions.len(), count);
    assert_eq!(document.level_count(), 2);
    let after = document.to_json().unwrap();
    assert_ne!(before, after);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
    document.select_level(1).unwrap();
    document.active_mesh().unwrap().validate().unwrap();
}

#[test]
fn capacity_limited_valid_mesh_can_still_move_without_refining() {
    let mut mesh = Mesh::sphere(2);
    mesh.positions.resize(MAX_VERTICES, Vec3::splat(100.0));
    mesh.normals.resize(MAX_VERTICES, Vec3::Z);
    mesh.colors.resize(MAX_VERTICES, [0.2, 0.3, 0.4]);
    mesh.masks.resize(MAX_VERTICES, 1.0);
    let hit = picked(&mesh, 0.0);
    let before_topology = mesh.triangles.clone();
    let mut moving = mesh
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let result = moving.apply(&mut mesh, Vec3::Z * 0.0001, &settings());
    assert!(result.changed_vertices > 0 && result.detail_limited);
    assert_eq!(result.refined_vertices, 0);
    assert_eq!(mesh.positions.len(), MAX_VERTICES);
    assert_eq!(mesh.triangles, before_topology);
    mesh.validate().unwrap();
}

#[test]
fn cancelled_symmetry_and_unrepresentable_targets_do_not_commit_refinement() {
    let mut document = Document::new(Mesh::sphere(3));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.3];
    document.end_stroke();
    assert!(document.undo());
    let original = document.active_mesh().unwrap().snapshot();
    let hit = picked(document.active_mesh().unwrap(), 0.0);
    let mut moving = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    document.begin_stroke();
    assert_eq!(
        moving
            .apply(
                document.active_mesh_mut().unwrap(),
                Vec3::X * 0.2,
                &settings()
            )
            .changed_vertices,
        0
    );
    assert_eq!(
        moving
            .apply(
                document.active_mesh_mut().unwrap(),
                Vec3::Z * 1e-30,
                &settings()
            )
            .changed_vertices,
        0
    );
    document.end_stroke();
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.can_redo());
    document.begin_stroke();
    assert!(
        moving
            .apply(
                document.active_mesh_mut().unwrap(),
                Vec3::Z * 0.01,
                &settings()
            )
            .changed_vertices
            > 0
    );
    assert!(
        moving
            .apply(
                document.active_mesh_mut().unwrap(),
                Vec3::X * 0.2,
                &settings()
            )
            .changed_vertices
            > 0
    );
    document.end_stroke();
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.can_redo());
}

#[test]
fn proportional_small_pull_and_broad_large_pull_retain_visible_motion() {
    for (radius, target, required_extension) in [(0.037, 0.03, 0.018), (0.18, 0.6, 0.15)] {
        let mut mesh = Mesh::sphere(6);
        let hit = picked(&mesh, 0.3);
        let brush = BrushSettings {
            radius,
            ..settings()
        };
        let mut moving = mesh
            .prepare_move_stroke(hit.position, hit.normal, &brush)
            .unwrap();
        let result = moving.apply(&mut mesh, Vec3::Z * target, &brush);
        let extension = mesh
            .positions
            .iter()
            .map(|p| p.length() - 1.0)
            .fold(0.0, f32::max);
        let (quality, fold) = geometry(&mesh);
        assert!(
            result.changed_vertices > 0 && extension > required_extension,
            "radius {radius} target {target}: extension {extension}, scale {}",
            result.displacement_scale
        );
        assert!(quality >= 0.1 * (1.0 - 1e-4) && fold <= 90.0 + 1e-4);
        println!(
            "useful pull radius={radius}, request={target}, extension={extension}, scale={}",
            result.displacement_scale
        );
    }
}

#[test]
fn pressure_and_raster_alpha_scale_the_frozen_material_response() {
    let source = Mesh::sphere(4);
    let hit = picked(&source, 0.3);
    let brush = BrushSettings {
        radius: 0.18,
        adaptive_detail: false,
        symmetry_x: false,
        ..settings()
    };
    let mut full = source.snapshot();
    let mut white = source.snapshot();
    let mut half = source.snapshot();
    let mut full_stroke = full
        .prepare_move_stroke(hit.position, hit.normal, &brush)
        .unwrap();
    let white_brush = BrushSettings {
        alpha: Some(RasterAlpha::new(1, 1, vec![255]).unwrap()),
        ..brush.clone()
    };
    let mut white_stroke = white
        .prepare_move_stroke(hit.position, hit.normal, &white_brush)
        .unwrap();
    let mut half_stroke = half
        .prepare_move_stroke(hit.position, hit.normal, &brush)
        .unwrap();
    let delta = Vec3::Z * 0.002;
    assert_eq!(
        full_stroke
            .apply(&mut full, delta, &brush)
            .displacement_scale,
        1.0
    );
    white_stroke.apply(&mut white, delta, &white_brush);
    exact(&full, &white);
    half_stroke.apply(
        &mut half,
        delta,
        &BrushSettings {
            pressure: 0.5,
            ..brush.clone()
        },
    );
    for ((a, b), c) in source
        .positions
        .iter()
        .zip(&full.positions)
        .zip(&half.positions)
    {
        assert!((*c - (*a + (*b - *a) * 0.5)).length() < 1e-6);
    }
    let before = half.snapshot();
    half_stroke.apply(
        &mut half,
        Vec3::Z * 0.01,
        &BrushSettings {
            pressure: 0.0,
            ..brush
        },
    );
    exact(&half, &before);
}

/// Replay recorded pixels using f32 camera arithmetic, rather than rounding the
/// independently reconstructed f64 hit into a different material footprint.
/// Candidate-1 source project SHA256:
/// 2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb.
/// This captures the recorded inputs; the runtime's internal hit was not logged.
fn recorded_camera_grab(mesh: &Mesh, press: [f32; 2]) -> (obrush_core::Hit, Vec3) {
    let yaw = 0.35_f32;
    let pitch = 0.18_f32;
    let camera = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ) * 4.0;
    let forward = (-camera).normalize_or_zero();
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    let (left, top, width, height) = (212.0, 57.0, 1153.0, 912.0);
    let aspect: f32 = width / height;
    let fov = 2.0 * ((40.0_f32.to_radians() * 0.5).tan() / aspect.clamp(0.001, 1.0)).atan();
    let tangent = (fov * 0.5).tan();
    let ndc_x = 2.0 * (press[0] - left) / width - 1.0;
    let ndc_y = 1.0 - 2.0 * (press[1] - top) / height;
    let hit = mesh
        .raycast(Ray {
            origin: camera,
            direction: (forward + right * ndc_x * tangent * aspect + up * ndc_y * tangent)
                .normalize_or_zero(),
        })
        .expect("recorded and nearby sphere press rays hit");
    let depth = (hit.position - camera).dot(forward).max(0.0001);
    let units_per_pixel = 2.0 * depth * tangent / height;
    let target =
        ((right.as_dvec3() * 180.0 - up.as_dvec3() * -80.0) * f64::from(units_per_pixel)).as_vec3();
    (hit, target)
}

#[test]
fn recorded_browser_camera_move_remains_useful_with_exact_chunk_return_and_history() {
    let original = Mesh::sphere(6);
    let (hit, target) = recorded_camera_grab(&original, [703.5, 483.0]);
    assert_eq!(hit.triangle, [8701, 34572, 34571]);
    assert!(
        hit.position
            .distance(Vec3::new(0.13209009, 0.2459738, 0.960_187_4))
            < 2e-6
    );
    assert!(target.distance(Vec3::new(0.3962712, 0.18997611, -0.18145129)) < 2e-6);

    let mut one = original.snapshot();
    let mut single = one
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    let single_result = single.apply(&mut one, target, &settings());
    assert!(single_result.changed_vertices > 0);
    let displacement = original
        .positions
        .iter()
        .zip(&one.positions)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max);
    assert!(
        displacement >= 0.001,
        "recorded Move frozen: {displacement}"
    );
    let (quality, fold) = geometry(&one);
    assert!(quality >= 0.1 * (1.0 - 1e-4), "recorded quality {quality}");
    assert!(fold <= 90.0 + 1e-4, "recorded fold {fold}");

    let mut document = Document::new(original.snapshot());
    let mut sampled = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    document.begin_stroke();
    for sample in 1..=12 {
        sampled.apply(
            document.active_mesh_mut().unwrap(),
            target * (sample as f32 / 12.0),
            &settings(),
        );
    }
    exact(document.active_mesh().unwrap(), &one);
    document.end_stroke();
    assert!(document.undo());
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.redo());
    exact(document.active_mesh().unwrap(), &one);

    let mut returning = Document::new(original.snapshot());
    let mut roundtrip = returning
        .active_mesh()
        .unwrap()
        .prepare_move_stroke(hit.position, hit.normal, &settings())
        .unwrap();
    returning.begin_stroke();
    for sample in 1..=12 {
        roundtrip.apply(
            returning.active_mesh_mut().unwrap(),
            target * (sample as f32 / 12.0),
            &settings(),
        );
    }
    roundtrip.apply(
        returning.active_mesh_mut().unwrap(),
        Vec3::ZERO,
        &settings(),
    );
    returning.end_stroke();
    exact(returning.active_mesh().unwrap(), &original);
    assert!(!returning.can_undo());
    println!(
        "recorded browser Move: original displacement={displacement}, quality={quality}, fold={fold}"
    );
}

#[test]
fn nearby_recorded_camera_grabs_do_not_repeat_the_refinement_freeze() {
    let original = Mesh::sphere(6);
    for press in [
        [695.5, 483.0],
        [711.5, 483.0],
        [703.5, 475.0],
        [703.5, 491.0],
    ] {
        let (hit, target) = recorded_camera_grab(&original, press);
        let mut mesh = original.snapshot();
        let mut moving = mesh
            .prepare_move_stroke(hit.position, hit.normal, &settings())
            .unwrap();
        moving.apply(&mut mesh, target, &settings());
        let displacement = original
            .positions
            .iter()
            .zip(&mesh.positions)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f32::max);
        assert!(
            displacement >= 0.001,
            "nearby press {press:?} frozen: {displacement}"
        );
        let (quality, fold) = geometry(&mesh);
        assert!(
            quality >= 0.1 * (1.0 - 1e-4),
            "nearby press {press:?} quality {quality}"
        );
        assert!(fold <= 90.0 + 1e-4, "nearby press {press:?} fold {fold}");
        println!(
            "nearby press {press:?}: original displacement={displacement}, quality={quality}, fold={fold}"
        );
    }
}
