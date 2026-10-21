//! Independent large-mesh fixture for full-vs-window Draw equivalence.
//! The candidate comparison is added only after its API is frozen.

use super::*;
use crate::Document;

fn large_authored_mesh(isolated_inside_brush: bool) -> (Mesh, usize) {
    const SIDE: usize = 321;
    let mut obj = String::with_capacity(12_000_000);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let px = (x as i32 - 160) as f32 * 0.02;
            let py = (y as i32 - 160) as f32 * 0.02;
            let pz = 0.06 * (px * 0.5).sin() * (py * 0.4).cos();
            obj.push_str(&format!("v {px} {py} {pz}\n"));
        }
    }
    for y in 0..SIDE - 1 {
        for x in 0..SIDE - 1 {
            let a = y * SIDE + x + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            obj.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }

    // An overlapping disconnected rear sheet is entirely back-facing.
    let rear_base = SIDE * SIDE;
    for y in 0..21 {
        for x in 0..21 {
            let px = (x - 10) as f32 * 0.025;
            let py = (y - 10) as f32 * 0.025;
            obj.push_str(&format!("v {px} {py} -0.12\n"));
        }
    }
    for y in 0..20 {
        for x in 0..20 {
            let a = rear_base + y * 21 + x + 1;
            let b = a + 1;
            let c = a + 21;
            let d = c + 1;
            obj.push_str(&format!("f {a} {d} {b}\nf {a} {c} {d}\n"));
        }
    }

    // A coarse interior face intersects the brush without vertices in it.
    let coarse_base = rear_base + 21 * 21;
    obj.push_str("v -0.5 -0.4 0.3\nv 0.5 -0.4 0.3\nv 0 0.6 0.3\n");
    obj.push_str(&format!(
        "f {} {} {}\n",
        coarse_base + 1,
        coarse_base + 2,
        coarse_base + 3
    ));

    // Three faces share an edge far outside the brush. A local algorithm must
    // neither repair nor reject unrelated valid retained topology by accident.
    let nonmanifold_base = coarse_base + 3;
    obj.push_str("v 4 4 0\nv 4.1 4 0\nv 4.05 4.1 0\nv 4.05 3.9 0\nv 4.05 4 0.1\n");
    for far in 2..5 {
        obj.push_str(&format!(
            "f {} {} {}\n",
            nonmanifold_base + 1,
            nonmanifold_base + 2,
            nonmanifold_base + far + 1
        ));
    }

    // Mesh::validate permits an unreferenced vertex. Draw's vertex loop can
    // edit it when its retained normal is front-facing; face-only windows miss it.
    let isolated = nonmanifold_base + 5;
    if isolated_inside_brush {
        obj.push_str("v 0.031 0.017 0.006\n");
    } else {
        obj.push_str("v 4.3 4.2 0.006\n");
    }
    let mut mesh = Mesh::from_obj(&obj).expect("independently authored large OBJ");
    assert_eq!(mesh.positions.len(), isolated + 1);
    mesh.normals[isolated] = Vec3::Z;
    mesh.materials = Some(vec![[0.36, 0.14]; mesh.positions.len()]);
    for i in 0..mesh.positions.len() {
        let p = mesh.positions[i];
        mesh.masks[i] = if p.x > 0.7 && p.y > 0.7 {
            1.0
        } else if p.x > 0.0 && p.x < 0.3 && p.y < 0.0 {
            0.4
        } else {
            0.0
        };
        mesh.colors[i] = [
            0.25 + ((i % 7) as f32) * 0.05,
            0.35 + ((i % 11) as f32) * 0.03,
            0.45 + ((i % 13) as f32) * 0.02,
        ];
        mesh.materials.as_mut().unwrap()[i] =
            [0.2 + ((i % 5) as f32) * 0.1, ((i % 3) as f32) * 0.15];
    }
    mesh.validate().unwrap();
    (mesh, isolated)
}

#[test]
fn authored_large_fixture_has_an_editable_isolated_vertex_and_exact_history() {
    let (mesh, isolated) = large_authored_mesh(true);
    assert!(mesh.positions.len() > 100_000);
    let initial_position = mesh.positions[isolated];
    let mut doc = Document::new(mesh);
    let initial = doc.to_json().unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        ..BrushSettings::default()
    };
    doc.begin_stroke();
    let changed =
        doc.mesh
            .as_mut()
            .unwrap()
            .apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    doc.end_stroke();
    assert!(changed > 0);
    let edited = doc.to_json().unwrap();
    assert_ne!(
        doc.mesh.as_ref().unwrap().positions[isolated],
        initial_position
    );
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), initial);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), edited);
}

#[test]
fn authored_large_fixture_default_draw_preserves_outside_and_attributes_with_asymmetric_masks() {
    let (source, _) = large_authored_mesh(false);
    for symmetry_x in [false, true] {
        let mut mesh = source.clone();
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            symmetry_x,
            ..BrushSettings::default()
        };
        let before = mesh.clone();
        let result = mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
        assert!(result.changed_vertices > 0, "symmetry={symmetry_x}");
        mesh.validate().unwrap();
        assert_eq!(&mesh.masks[..before.masks.len()], &before.masks);
        assert_eq!(&mesh.colors[..before.colors.len()], &before.colors);
        assert_eq!(
            &mesh.materials.as_ref().unwrap()[..before.positions.len()],
            before.materials.as_ref().unwrap()
        );
        for (i, (&old, &new)) in before.positions.iter().zip(&mesh.positions).enumerate() {
            if old.distance(Vec3::ZERO) >= settings.radius {
                assert_eq!(
                    old.to_array().map(f32::to_bits),
                    new.to_array().map(f32::to_bits),
                    "outside vertex {i} moved"
                );
            }
        }
    }
}

#[test]
fn authored_large_fixture_full_and_window_are_bit_exact() {
    let (source, isolated) = large_authored_mesh(false);
    assert_eq!(source.positions.len(), 103_491);
    assert_eq!(source.normals[isolated], Vec3::Z);
    for (center, radius, symmetry_x) in [
        (Vec3::ZERO, 0.18, false),
        (Vec3::new(0.12, -0.06, 0.0), 0.18, true),
        (Vec3::new(-0.17, 0.11, 0.02), 0.25, false),
    ] {
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius,
            strength: 0.35,
            symmetry_x,
            ..BrushSettings::default()
        };
        let mut full = source.snapshot();
        let (expected, _) =
            full.apply_brush_dab(center, Vec3::Z, Vec3::ZERO, &settings, None, true, true);
        let mut window = source.snapshot();
        let actual = window.try_local_draw_window(center, Vec3::Z, Vec3::ZERO, &settings);
        assert!(expected.changed_vertices > 0, "fixture contact must draw");
        assert_eq!(
            actual,
            Some(expected),
            "window fell back at {center:?}, r={radius}, symmetry={symmetry_x}"
        );
        assert_eq!(
            serde_json::to_vec(&full).unwrap(),
            serde_json::to_vec(&window).unwrap(),
            "mesh words differ at {center:?}, r={radius}, symmetry={symmetry_x}"
        );

        let mut document = Document::new(source.snapshot());
        let before = document.to_json().unwrap();
        document.begin_stroke();
        let public = document.mesh.as_mut().unwrap().apply_brush_with_result(
            center,
            Vec3::Z,
            Vec3::ZERO,
            &settings,
        );
        document.end_stroke();
        assert_eq!(public, expected);
        assert_eq!(
            serde_json::to_vec(document.mesh.as_ref().unwrap()).unwrap(),
            serde_json::to_vec(&full).unwrap()
        );
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}

#[test]
fn authored_large_fixture_falls_back_for_movable_orphan_without_dropping_it() {
    let (source, isolated) = large_authored_mesh(true);
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: false,
        ..BrushSettings::default()
    };
    let mut full = source.snapshot();
    let (expected, _) =
        full.apply_brush_dab(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings, None, true, true);
    assert_ne!(source.positions[isolated], full.positions[isolated]);
    let mut actual = source.snapshot();
    assert_eq!(
        actual.try_local_draw_window(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings),
        None
    );
    let public = actual.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings);
    assert_eq!(public, expected);
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&full).unwrap()
    );
}

#[test]
fn authored_large_coordinate_mirror_tangent_matches_exact_full_query() {
    let mut source = Mesh::sphere(7);
    let p = source.positions.len() as u32;
    // These exact f32 words represent +999_999.125 and -999_999.125.
    source.positions.extend([
        Vec3::new(999_999.0, -0.04, 0.0),
        Vec3::new(f32::from_bits(0x4974_23f2), -0.04, 0.0),
        Vec3::new(999_999.0, 0.04, 0.0),
        Vec3::new(f32::from_bits(0xc974_23f2), -0.04, 0.0),
        Vec3::new(-999_999.0, -0.04, 0.0),
        Vec3::new(-999_999.0, 0.04, 0.0),
    ]);
    source
        .triangles
        .extend([[p, p + 1, p + 2], [p + 3, p + 4, p + 5]]);
    source.masks.resize(source.positions.len(), 0.0);
    source
        .colors
        .resize(source.positions.len(), [0.5, 0.5, 0.5]);
    source.recompute_normals();
    source.validate().unwrap();
    let center = Vec3::new(999_999.0, 0.0, 0.0);
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.05,
        strength: 0.35,
        symmetry_x: true,
        symmetry_offset: 0.02,
        ..BrushSettings::default()
    };
    let mut full = source.snapshot();
    let (expected, _) =
        full.apply_brush_dab(center, Vec3::Z, Vec3::ZERO, &settings, None, true, true);
    assert!(expected.changed_vertices > 0);
    let mut window = source.snapshot();
    let actual = window.try_local_draw_window(center, Vec3::Z, Vec3::ZERO, &settings);
    eprintln!("full={expected:?} window={actual:?}");
    assert_eq!(actual, Some(expected));
    assert_eq!(
        serde_json::to_vec(&full).unwrap(),
        serde_json::to_vec(&window).unwrap()
    );
}
