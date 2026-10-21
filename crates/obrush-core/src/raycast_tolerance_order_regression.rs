//! Frozen pre-optimization full-f64 picker as an independent oracle.
//! Source: OBrush 647ac51 `Mesh::raycast_indexed`; only `self` was renamed.
use super::*;

fn reference_raycast_indexed(mesh: &Mesh, ray: Ray) -> Option<(usize, Hit)> {
    if mesh.positions.len() > MAX_VERTICES
        || mesh.triangles.len() > MAX_TRIANGLES
        || !ray.origin.is_finite()
        || !ray.direction.is_finite()
    {
        return None;
    }
    let direction = ray.direction.as_dvec3().normalize_or_zero();
    if direction == glam::DVec3::ZERO {
        return None;
    }
    let origin = ray.origin.as_dvec3();
    let mut closest: Option<(usize, [u32; 3], f64, f64, f64)> = None;
    for (row, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        let (a, b, c) = (a as usize, b as usize, c as usize);
        let (Some(&p0), Some(&p1), Some(&p2)) = (
            mesh.positions.get(a),
            mesh.positions.get(b),
            mesh.positions.get(c),
        ) else {
            continue;
        };
        let p0 = p0.as_dvec3();
        let e1 = p1.as_dvec3() - p0;
        let e2 = p2.as_dvec3() - p0;
        let p = direction.cross(e2);
        let determinant = e1.dot(p);
        let tolerance = f64::EPSILON * e1.length() * e2.length();
        if !determinant.is_finite() || determinant.abs() <= tolerance {
            continue;
        }
        let inverse = determinant.recip();
        let t = origin - p0;
        let u = t.dot(p) * inverse;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = t.cross(e1);
        let v = direction.dot(q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let distance = e2.dot(q) * inverse;
        if !distance.is_finite()
            || distance < 0.0
            || closest.is_some_and(|(_, _, d, _, _)| d <= distance)
        {
            continue;
        }
        closest = Some((row, [a as u32, b as u32, c as u32], distance, u, v));
    }
    let (row, triangle, distance, u, v) = closest?;
    let barycentric = [1.0 - u - v, u, v];
    let local_position = origin + direction * distance;
    let mut normal = match triangle.map(|id| mesh.normals.get(id as usize)) {
        [Some(a), Some(b), Some(c)] => (a.as_dvec3() * barycentric[0]
            + b.as_dvec3() * barycentric[1]
            + c.as_dvec3() * barycentric[2])
            .normalize_or_zero(),
        _ => glam::DVec3::ZERO,
    };
    if normal == glam::DVec3::ZERO || !normal.is_finite() {
        let [a, b, c] = triangle.map(|id| mesh.positions[id as usize].as_dvec3());
        normal = (b - a).cross(c - a).normalize_or_zero();
    }
    let hit = Hit {
        position: local_position.as_vec3(),
        normal: normal.as_vec3().normalize_or_zero(),
        distance: distance as f32,
        triangle,
        barycentric: barycentric.map(|weight| weight as f32),
    };
    (hit.position.is_finite()
        && hit.normal.is_finite()
        && hit.normal != Vec3::ZERO
        && hit.distance.is_finite())
    .then_some((row, hit))
}

fn assert_same_pick(mesh: &Mesh, ray: Ray) {
    let old = reference_raycast_indexed(mesh, ray);
    let new = mesh.raycast_indexed(ray);
    match (old, new) {
        (None, None) => {}
        (Some((old_row, old_hit)), Some((new_row, new_hit))) => {
            assert_eq!(old_row, new_row, "row for {ray:?}");
            assert_eq!(old_hit.triangle, new_hit.triangle, "triangle for {ray:?}");
            for (a, b) in old_hit
                .position
                .to_array()
                .into_iter()
                .zip(new_hit.position.to_array())
            {
                assert_eq!(a.to_bits(), b.to_bits(), "position for {ray:?}");
            }
            for (a, b) in old_hit
                .normal
                .to_array()
                .into_iter()
                .zip(new_hit.normal.to_array())
            {
                assert_eq!(a.to_bits(), b.to_bits(), "normal for {ray:?}");
            }
            assert_eq!(
                old_hit.distance.to_bits(),
                new_hit.distance.to_bits(),
                "distance for {ray:?}"
            );
            for (a, b) in old_hit.barycentric.into_iter().zip(new_hit.barycentric) {
                assert_eq!(a.to_bits(), b.to_bits(), "barycentric for {ray:?}");
            }
        }
        (old, new) => panic!("pick presence differs for {ray:?}: {old:?} vs {new:?}"),
    }
}

#[test]
fn deferred_tolerance_preserves_edges_near_parallel_and_scales() {
    // Independently authored front/back triangles with equal-distance duplicate
    // rows. Ray origins include edge/vertex hits and distant near misses.
    for scale in [0.0001_f32, 1.0, 100_000.0] {
        let mesh = Mesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0) * scale,
                Vec3::new(1.0, 0.0, 0.0) * scale,
                Vec3::new(0.0, 1.0, 0.0) * scale,
                Vec3::new(0.0, 0.0, -0.25) * scale,
                Vec3::new(1.0, 0.0, -0.25) * scale,
                Vec3::new(0.0, 1.0, -0.25) * scale,
            ],
            vec![[0, 1, 2], [0, 1, 2], [3, 4, 5]],
        );
        for &(x, y) in &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.0, 0.5),
            (0.5, 0.5),
            (0.2, 0.3),
            (-0.001, 0.2),
        ] {
            for z in [1.0_f32, 100.0] {
                assert_same_pick(
                    &mesh,
                    Ray {
                        origin: Vec3::new(x, y, z) * scale,
                        direction: -Vec3::Z,
                    },
                );
            }
        }
        for epsilon in [0.0_f32, f32::EPSILON, 1e-7, 1e-4, 0.1] {
            assert_same_pick(
                &mesh,
                Ray {
                    origin: Vec3::new(-0.5, 0.25, 0.5) * scale,
                    direction: Vec3::new(1.0, 0.0, -epsilon),
                },
            );
        }
    }
}

#[test]
fn deferred_tolerance_preserves_seeded_skinny_scene_picks() {
    // Stable, authored LCG; no imported mesh or private model is used.
    let mut state = 0xD307_AA54_827B_C19Du64;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((state >> 40) as f32) / ((1u32 << 24) as f32)
    };
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    for i in 0..160 {
        let base = Vec3::new(
            (next() - 0.5) * 12.0,
            (next() - 0.5) * 12.0,
            (next() - 0.5) * 12.0,
        );
        let extent = if i % 4 == 0 { 0.0005 } else { 0.5 };
        let first = positions.len() as u32;
        positions.extend([
            base,
            base + Vec3::new(extent, 0.0, 0.0),
            base + Vec3::new(0.0, extent, extent * 0.03),
        ]);
        triangles.push([first, first + 1, first + 2]);
    }
    let mesh = Mesh::new(positions, triangles);
    for _ in 0..600 {
        let origin = Vec3::new((next() - 0.5) * 20.0, (next() - 0.5) * 20.0, 30.0);
        let direction = Vec3::new((next() - 0.5) * 0.5, (next() - 0.5) * 0.5, -1.0);
        assert_same_pick(&mesh, Ray { origin, direction });
    }
}
