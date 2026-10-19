//! Independent mathematical and lifecycle gates for exposed brush options.
//! These fixtures do not establish measured reference response equivalence.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, RasterAlpha, Vec3};

const SIDE: usize = 21;
const STEP: f32 = 0.0625;

fn sheet_at(offset: f32, z: f32, reversed: bool) -> Mesh {
    let mut text = String::new();
    for row in 0..SIDE {
        for column in 0..SIDE {
            let x = offset + (column as i32 - 10) as f32 * STEP;
            let y = (row as i32 - 10) as f32 * STEP;
            text.push_str(&format!("v {x} {y} {z} 0.2 0.4 0.6\n"));
        }
    }
    for row in 0..SIDE - 1 {
        for column in 0..SIDE - 1 {
            let a = row * SIDE + column + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            let faces = if reversed {
                [[a, d, b], [a, c, d]]
            } else {
                [[a, b, d], [a, d, c]]
            };
            for [a, b, c] in faces {
                text.push_str(&format!("f {a} {b} {c}\n"));
            }
        }
    }
    Mesh::from_obj(&text).unwrap()
}
fn sheet() -> Mesh {
    sheet_at(0.0, 0.0, false)
}
fn index(x: i32, y: i32) -> usize {
    (y + 10) as usize * SIDE + (x + 10) as usize
}
fn configured(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.5,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}
fn exact(a: &Mesh, b: &Mesh) {
    let vectors = |a: &[Vec3]| {
        a.iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(a.positions.len(), b.positions.len());
    for (vertex, (a, b)) in vectors(&a.positions)
        .into_iter()
        .zip(vectors(&b.positions))
        .enumerate()
    {
        assert_eq!(a, b, "position bits at vertex {vertex}");
    }
    for (vertex, (a, b)) in vectors(&a.normals)
        .into_iter()
        .zip(vectors(&b.normals))
        .enumerate()
    {
        assert_eq!(a, b, "normal bits at vertex {vertex}");
    }
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(
        a.masks.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.masks.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(
        a.colors
            .iter()
            .map(|x| x.map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.colors
            .iter()
            .map(|x| x.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
}
fn dab(mesh: &mut Mesh, settings: &BrushSettings) -> obrush_core::BrushResult {
    let delta = if matches!(settings.kind, BrushKind::Twist | BrushKind::LocalScale) {
        Vec3::X * 0.2
    } else {
        Vec3::new(0.02, 0.01, 0.01)
    };
    mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, delta, settings)
}
fn joined_sheets(reversed: bool) -> Mesh {
    let a = sheet();
    let mut b = sheet_at(0.0, 0.03125, reversed);
    b.positions[index(3, 2)].x += 0.015625;
    b.recompute_normals();
    let mut text = a.to_obj();
    for (p, c) in b.positions.iter().zip(&b.colors) {
        text.push_str(&format!(
            "v {} {} {} {} {} {}\n",
            p.x, p.y, p.z, c[0], c[1], c[2]
        ));
    }
    for face in &b.triangles {
        let f = face.map(|v| v as usize + a.positions.len() + 1);
        text.push_str(&format!("f {} {} {}\n", f[0], f[1], f[2]));
    }
    Mesh::from_obj(&text).unwrap()
}

#[test]
fn older_settings_deserialize_all_option_defaults_without_changing_existing_response() {
    let legacy = r#"{"kind":"draw","radius":0.5,"strength":0.35,"invert":false,"symmetry_x":false,"color":[0.77,0.34,0.19],"adaptive_detail":false}"#;
    let old: BrushSettings = serde_json::from_str(legacy).unwrap();
    assert!(old.backface_culling && old.draw_accumulate);
    assert!(
        !old.smooth_tangent && !old.draw_clay && !old.move_topology_check && !old.move_along_normal
    );
    assert_eq!(old.hardness, 0.0);
    assert_eq!(old.symmetry_offset, 0.0);
    let mut a = sheet();
    let mut b = a.snapshot();
    dab(&mut a, &old);
    dab(&mut b, &configured(BrushKind::Draw));
    exact(&a, &b);
}

#[test]
fn option_settings_serialize_and_roundtrip_every_active_control() {
    let settings = BrushSettings {
        backface_culling: false,
        hardness: 0.75,
        smooth_tangent: true,
        draw_clay: true,
        draw_accumulate: false,
        move_topology_check: true,
        move_along_normal: true,
        symmetry_offset: 1.25,
        ..configured(BrushKind::Draw)
    };
    let data = serde_json::to_value(&settings).unwrap();
    let round: BrushSettings = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(serde_json::to_value(round).unwrap(), data);
}

#[test]
fn disabling_culling_really_includes_rear_facing_geometry_mask_and_color() {
    let original = joined_sheets(true);
    let back = SIDE * SIDE + index(3, 2);
    for kind in BrushKind::ALL
        .into_iter()
        .filter(|kind| *kind != BrushKind::Morph)
    {
        let mut front_only = original.snapshot();
        let mut both = original.snapshot();
        let mut settings = configured(kind);
        dab(&mut front_only, &settings);
        settings.backface_culling = false;
        dab(&mut both, &settings);
        if kind == BrushKind::Mask {
            assert_eq!(front_only.masks[back], original.masks[back]);
            assert!(both.masks[back] > 0.0);
        } else if kind == BrushKind::Paint {
            assert_eq!(front_only.colors[back], original.colors[back]);
            assert_ne!(both.colors[back], original.colors[back]);
        } else {
            assert_eq!(
                front_only.positions[back], original.positions[back],
                "{kind}"
            );
            assert_ne!(both.positions[back], original.positions[back], "{kind}");
        }
        both.validate().unwrap();
    }
}

#[test]
fn culling_is_frozen_into_rest_space_move_and_anchored_transforms() {
    let original = joined_sheets(true);
    let back = SIDE * SIDE + index(3, 2);
    for kind in [BrushKind::Move, BrushKind::Twist, BrushKind::LocalScale] {
        for culling in [true, false] {
            let mut mesh = original.snapshot();
            let mut settings = configured(kind);
            settings.backface_culling = culling;
            let result = if kind == BrushKind::Move {
                let mut gesture = mesh
                    .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
                    .unwrap();
                gesture.apply(&mut mesh, Vec3::X * 0.05, &settings)
            } else {
                let mut gesture = mesh
                    .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &settings)
                    .unwrap();
                gesture.apply(&mut mesh, 0.2, &settings)
            };
            assert!(result.changed_vertices > 0);
            assert_eq!(
                mesh.positions[back] == original.positions[back],
                culling,
                "{kind}"
            );
        }
    }
}

#[test]
fn known_hardness_weights_change_mask_profiles_without_geometry_changes() {
    let original = sheet();
    let point = index(4, 0);
    let distance = 0.25_f64 / 0.5;
    for hardness in [0.0, 0.5, 1.0] {
        let mut mesh = original.snapshot();
        let mut settings = configured(BrushKind::Mask);
        settings.hardness = hardness;
        dab(&mut mesh, &settings);
        let base = (1.0 - distance * distance).powi(2);
        let expected = 0.18 * f64::from(settings.strength) * base.powf(1.0 - f64::from(hardness));
        let error = (f64::from(mesh.masks[point]) - expected).abs();
        assert!(error < 2e-7);
        println!(
            "Mask hardness={hardness}: expected={expected}, measured={}, error={error}",
            mesh.masks[point]
        );
        assert_eq!(mesh.positions, original.positions);
        assert_eq!(mesh.normals, original.normals);
        assert_eq!(mesh.colors, original.colors);
        assert_eq!(mesh.masks[index(8, 0)], 0.0, "strict footprint boundary");
    }
}

#[test]
fn known_hardness_weights_change_paint_profiles_with_partial_mask_protection() {
    let original = sheet();
    let point = index(4, 0);
    for hardness in [0.0, 0.5, 1.0] {
        let mut mesh = original.snapshot();
        mesh.masks[point] = 0.5;
        let mut settings = configured(BrushKind::Paint);
        settings.hardness = hardness;
        settings.color = [0.8, 0.2, 0.1];
        dab(&mut mesh, &settings);
        let weight =
            0.5625_f64.powf(1.0 - f64::from(hardness)) * f64::from(settings.strength) * 0.5 * 0.25;
        let mut maximum_error = 0.0_f64;
        for channel in 0..3 {
            let expected = f64::from(original.colors[point][channel])
                + (f64::from(settings.color[channel]) - f64::from(original.colors[point][channel]))
                    * weight;
            let error = (f64::from(mesh.colors[point][channel]) - expected).abs();
            maximum_error = maximum_error.max(error);
            assert!(error < 2e-7);
        }
        println!("Paint hardness={hardness}: max_scalar_error={maximum_error}");
        assert_eq!(mesh.positions, original.positions);
        assert_eq!(mesh.normals, original.normals);
        assert_eq!(mesh.masks[point], 0.5);
    }
}

#[test]
fn hardness_changes_sculpting_and_the_frozen_move_radial_field() {
    let original = sheet();
    let point = index(6, 0);
    for kind in [
        BrushKind::Pinch,
        BrushKind::Twist,
        BrushKind::LocalScale,
        BrushKind::Drag,
        BrushKind::Move,
    ] {
        let mut soft = original.snapshot();
        let mut hard = original.snapshot();
        let mut settings = configured(kind);
        if kind == BrushKind::Move {
            let mut gesture = soft
                .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
                .unwrap();
            gesture.apply(&mut soft, Vec3::X * 0.005, &settings);
        } else {
            dab(&mut soft, &settings);
        }
        settings.hardness = 1.0;
        if kind == BrushKind::Move {
            let mut gesture = hard
                .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
                .unwrap();
            gesture.apply(&mut hard, Vec3::X * 0.005, &settings);
        } else {
            dab(&mut hard, &settings);
        }
        let soft_distance = soft.positions[point].distance(original.positions[point]);
        let hard_distance = hard.positions[point].distance(original.positions[point]);
        assert!(
            hard_distance > soft_distance * 1.1,
            "{kind}: {soft_distance} -> {hard_distance}"
        );
    }
}

#[test]
fn out_of_range_finite_hardness_clamps_and_nonfinite_options_are_atomic() {
    for (value, clamped) in [(-1.0, 0.0), (2.0, 1.0)] {
        let mut a = sheet();
        let mut b = a.snapshot();
        let mut settings = configured(BrushKind::Mask);
        settings.hardness = value;
        dab(&mut a, &settings);
        settings.hardness = clamped;
        dab(&mut b, &settings);
        exact(&a, &b);
    }
    for kind in BrushKind::ALL {
        for invalid in 0..3 {
            let mut mesh = sheet();
            let original = mesh.snapshot();
            let mut settings = configured(kind);
            match invalid {
                0 => settings.hardness = f32::NAN,
                1 => settings.symmetry_offset = f32::INFINITY,
                _ => settings.symmetry_offset = 1_000_001.0,
            };
            assert_eq!(dab(&mut mesh, &settings).changed_vertices, 0);
            exact(&mesh, &original);
        }
    }
}

#[test]
fn tangential_smooth_changes_tangent_geometry_and_removes_the_normal_component() {
    let mut original = sheet();
    let point = index(3, 2);
    original.positions[point] += Vec3::new(0.015625, 0.0, 0.02);
    original.recompute_normals();
    let mut regular = original.snapshot();
    let mut tangent = original.snapshot();
    let mut settings = configured(BrushKind::Smooth);
    let ordinary = dab(&mut regular, &settings);
    settings.smooth_tangent = true;
    let projected = dab(&mut tangent, &settings);
    assert_eq!(ordinary.displacement_scale, 1.0);
    assert_eq!(projected.displacement_scale, 1.0);
    let normal = original.normals[point];
    let change = tangent.positions[point] - original.positions[point];
    assert!(change.length() > 1e-5);
    assert!(change.dot(normal).abs() < 2e-7);
    assert!(
        (regular.positions[point] - original.positions[point])
            .dot(normal)
            .abs()
            > 1e-4
    );
    assert_ne!(regular.positions, tangent.positions);
    println!(
        "Smooth tangent: displacement={}, normal_component={}",
        change.length(),
        change.dot(normal)
    );
}

#[test]
fn tangential_smooth_keeps_fully_masked_points_fixed_and_inverse_reverses_it() {
    let mut original = sheet();
    let point = index(3, 2);
    original.positions[point].x += 0.015625;
    original.recompute_normals();
    let mut positive = original.snapshot();
    let mut negative = original.snapshot();
    let mut protected = original.snapshot();
    protected.masks[point] = 1.0;
    let mut settings = configured(BrushKind::Smooth);
    settings.smooth_tangent = true;
    dab(&mut positive, &settings);
    dab(&mut protected, &settings);
    settings.invert = true;
    dab(&mut negative, &settings);
    assert_eq!(protected.positions[point], original.positions[point]);
    let a = positive.positions[point] - original.positions[point];
    let b = negative.positions[point] - original.positions[point];
    assert!((a + b).length() < 4e-7);
}

#[test]
fn draw_clay_option_runs_the_same_defined_planar_clay_operation() {
    let original = sheet();
    let mut clay = original.snapshot();
    let mut option = original.snapshot();
    let mut draw = original.snapshot();
    dab(&mut clay, &configured(BrushKind::Clay));
    let mut settings = configured(BrushKind::Draw);
    dab(&mut draw, &settings);
    settings.draw_clay = true;
    dab(&mut option, &settings);
    exact(&clay, &option);
    assert_ne!(draw.positions, option.positions);
}

#[test]
fn non_accumulating_draw_keeps_its_original_footprint_and_changes_repeated_response() {
    let original = sheet();
    let mut current = original.snapshot();
    let mut proxy = original.snapshot();
    let mut settings = configured(BrushKind::Draw);
    let mut current_stroke = current.prepare_draw_stroke(&settings).unwrap();
    settings.draw_accumulate = false;
    let mut proxy_stroke = proxy.prepare_draw_stroke(&settings).unwrap();
    for _ in 0..25 {
        let mut accumulating = settings.clone();
        accumulating.draw_accumulate = true;
        current_stroke.apply(&mut current, Vec3::ZERO, Vec3::Z, &accumulating);
        proxy_stroke.apply(&mut proxy, Vec3::ZERO, Vec3::Z, &settings);
    }
    assert_eq!(
        proxy_stroke.proxy_positions(),
        original.positions.as_slice()
    );
    assert!(proxy.positions[index(0, 0)].z > current.positions[index(0, 0)].z + 0.01);
    current.validate().unwrap();
    proxy.validate().unwrap();
    println!(
        "Draw accumulation: current_peak={}, proxy_peak={}",
        current.positions[index(0, 0)].z,
        proxy.positions[index(0, 0)].z
    );
}

#[test]
fn non_accumulating_adaptive_draw_interpolates_the_rest_proxy_for_all_new_points() {
    let original = sheet();
    let mut mesh = original.snapshot();
    let mut settings = configured(BrushKind::Draw);
    settings.draw_accumulate = false;
    settings.adaptive_detail = true;
    let mut stroke = mesh.prepare_draw_stroke(&settings).unwrap();
    let mut refined = 0;
    for _ in 0..3 {
        let result = stroke.apply(&mut mesh, Vec3::ZERO, Vec3::Z, &settings);
        assert!(result.changed_vertices > 0);
        refined += result.refined_vertices;
        assert_eq!(stroke.proxy_positions().len(), mesh.positions.len());
        assert!(stroke.proxy_positions().iter().all(|p| p.z.to_bits() == 0));
        assert_eq!(
            &stroke.proxy_positions()[..original.positions.len()],
            original.positions.as_slice()
        );
    }
    assert!(refined > 0);
    assert!(mesh.positions[original.positions.len()..]
        .iter()
        .any(|p| p.z > 0.0));
    mesh.validate().unwrap();
}

#[test]
fn draw_proxy_rejects_external_changes_and_semantic_mode_switches_atomically() {
    for change in 0..4 {
        let mut mesh = sheet();
        let mut settings = configured(BrushKind::Draw);
        settings.draw_accumulate = false;
        let mut stroke = mesh.prepare_draw_stroke(&settings).unwrap();
        stroke.apply(&mut mesh, Vec3::ZERO, Vec3::Z, &settings);
        match change {
            0 => mesh.masks[0] = 0.5,
            1 => mesh.normals[0].x = 0.1,
            2 => settings.draw_clay = true,
            _ => settings.draw_accumulate = true,
        };
        let altered = mesh.snapshot();
        let proxy = stroke.proxy_positions().to_vec();
        assert_eq!(
            stroke
                .apply(&mut mesh, Vec3::ZERO, Vec3::Z, &settings)
                .changed_vertices,
            0
        );
        exact(&mesh, &altered);
        assert_eq!(stroke.proxy_positions(), proxy.as_slice());
    }
}

#[test]
fn draw_proxy_zero_contact_noops_preserve_genuine_redo_and_attributes() {
    let mut document = Document::new(sheet());
    document.begin_stroke();
    document.active_mesh_mut().unwrap().colors[0] = [0.8, 0.3, 0.2];
    document.end_stroke();
    assert!(document.undo());
    let original = document.active_mesh().unwrap().snapshot();
    let mut settings = configured(BrushKind::Draw);
    settings.draw_accumulate = false;
    settings.pressure = 0.0;
    let mut stroke = document
        .active_mesh()
        .unwrap()
        .prepare_draw_stroke(&settings)
        .unwrap();
    document.begin_stroke();
    assert_eq!(
        stroke
            .apply(
                document.active_mesh_mut().unwrap(),
                Vec3::ZERO,
                Vec3::Z,
                &settings
            )
            .changed_vertices,
        0
    );
    document.end_stroke();
    exact(document.active_mesh().unwrap(), &original);
    assert!(document.can_redo());
}

#[test]
fn non_accumulating_detail_is_one_whole_stroke_undo_with_exact_proxy_geometry() {
    let mut document = Document::new(sheet());
    let original = document.active_mesh().unwrap().snapshot();
    let mut settings = configured(BrushKind::Draw);
    settings.draw_accumulate = false;
    settings.adaptive_detail = true;
    let mut stroke = document
        .active_mesh()
        .unwrap()
        .prepare_draw_stroke(&settings)
        .unwrap();
    document.begin_stroke();
    for _ in 0..3 {
        stroke.apply(
            document.active_mesh_mut().unwrap(),
            Vec3::ZERO,
            Vec3::Z,
            &settings,
        );
    }
    document.end_stroke();
    let edited = document.active_mesh().unwrap().snapshot();
    assert!(edited.positions.len() > original.positions.len());
    assert!(document.undo());
    exact(document.active_mesh().unwrap(), &original);
    assert!(!document.can_undo());
    assert!(document.redo());
    exact(document.active_mesh().unwrap(), &edited);
}

#[test]
fn move_topology_check_isolates_a_picked_sheet_among_nearby_surfaces() {
    let original = joined_sheets(false);
    let seed = original
        .triangles
        .iter()
        .copied()
        .find(|face| face.contains(&(index(0, 0) as u32)))
        .unwrap();
    let back = SIDE * SIDE + index(3, 2);
    let mut all = original.snapshot();
    let mut connected = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.backface_culling = false;
    let mut unbounded = all
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some(seed))
        .unwrap();
    unbounded.apply(&mut all, Vec3::X * 0.02, &settings);
    settings.move_topology_check = true;
    let mut bounded = connected
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some(seed))
        .unwrap();
    let result = bounded.apply(&mut connected, Vec3::X * 0.02, &settings);
    assert!(result.changed_vertices > 0);
    assert_ne!(all.positions[back], original.positions[back]);
    assert_eq!(connected.positions[back], original.positions[back]);
    assert_ne!(
        connected.positions[index(3, 2)],
        original.positions[index(3, 2)]
    );
}

#[test]
fn connected_move_round_trips_public_project_and_one_history_step() {
    // Independently generated, nearby disconnected sheets. The front seed is
    // in the picked triangle; the rear sheet lies inside the same 0.5 radius.
    let original = joined_sheets(false);
    let seed = [index(0, 0) as u32, index(1, 0) as u32, index(1, 1) as u32];
    let rear = SIDE * SIDE + index(3, 2);
    let mut document = Document::new(original.snapshot());
    let before = document.to_json().unwrap();
    let mut settings = configured(BrushKind::Move);
    settings.move_topology_check = true;
    settings.backface_culling = false;
    let mut stroke = document
        .active_mesh()
        .unwrap()
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some(seed))
        .unwrap();
    document.begin_stroke();
    let result = stroke.apply(
        document.active_mesh_mut().unwrap(),
        Vec3::X * 0.02,
        &settings,
    );
    document.end_stroke();
    assert!(result.changed_vertices > 0);
    assert_eq!(
        document.active_mesh().unwrap().positions[rear],
        original.positions[rear]
    );
    assert_ne!(
        document.active_mesh().unwrap().positions[index(3, 2)],
        original.positions[index(3, 2)]
    );
    let after = document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn move_topology_paths_cannot_escape_the_radius_and_reenter_a_connected_mesh() {
    let text="v -0.3 -0.1 0\nv -0.1 -0.1 0\nv -0.2 0.1 0\nv 0.1 -0.1 0\nv 0.3 -0.1 0\nv 0.2 0.1 0\nv -0.3 1 0\nv -0.1 1 0\nv 0.1 1 0\nv 0.3 1 0\nv -0.3 1.2 0\nv 0.3 1.2 0\nf 1 2 3\nf 4 5 6\nf 1 3 7\nf 3 8 7\nf 6 5 10\nf 6 10 9\nf 7 8 11\nf 8 12 11\nf 8 9 12\nf 9 10 12\n";
    let original = Mesh::from_obj(text).unwrap();
    let mut all = original.snapshot();
    let mut bounded = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.backface_culling = false;
    let mut first = all
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some([0, 1, 2]))
        .unwrap();
    first.apply(&mut all, Vec3::Z * 0.01, &settings);
    settings.move_topology_check = true;
    let mut second = bounded
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some([0, 1, 2]))
        .unwrap();
    second.apply(&mut bounded, Vec3::Z * 0.01, &settings);
    assert_ne!(all.positions[4], original.positions[4]);
    assert_eq!(bounded.positions[4], original.positions[4]);
    assert_ne!(bounded.positions[1], original.positions[1]);
}

#[test]
fn seeded_move_rejects_invalid_triangle_vertices_without_mutation() {
    let mesh = sheet();
    let original = mesh.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.move_topology_check = true;
    assert!(mesh
        .prepare_move_stroke_with_seed(Vec3::ZERO, Vec3::Z, &settings, Some([0, 1, u32::MAX]))
        .is_err());
    exact(&mesh, &original);
}

#[test]
fn along_normal_move_uses_signed_captured_axis_and_exact_zero_return() {
    let original = sheet();
    let mut mesh = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.move_along_normal = true;
    let mut stroke = mesh
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    let result = stroke.apply_normal_distance(&mut mesh, 0.05, &settings);
    assert!(result.changed_vertices > 0);
    for (&before, &after) in original.positions.iter().zip(&mesh.positions) {
        assert_eq!(before.x.to_bits(), after.x.to_bits());
        assert_eq!(before.y.to_bits(), after.y.to_bits());
        assert!(after.z >= before.z);
    }
    stroke.apply_normal_distance(&mut mesh, 0.0, &settings);
    exact(&mesh, &original);
    stroke.apply_normal_distance(&mut mesh, -0.05, &settings);
    assert!(mesh.positions[index(0, 0)].z < 0.0);
    stroke.apply_normal_distance(&mut mesh, 0.0, &settings);
    exact(&mesh, &original);
}

#[test]
fn along_normal_absolute_targets_keep_event_partition_and_mask_protection() {
    let mut original = sheet();
    original.masks[index(0, 0)] = 1.0;
    let mut one = original.snapshot();
    let mut many = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.move_along_normal = true;
    let mut a = one
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    let mut b = many
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    a.apply_normal_distance(&mut one, 0.03, &settings);
    for step in 1..=20 {
        let target = if step == 20 {
            0.03
        } else {
            0.03 * step as f32 / 20.0
        };
        b.apply_normal_distance(&mut many, target, &settings);
    }
    exact(&one, &many);
    assert_eq!(one.positions[index(0, 0)], original.positions[index(0, 0)]);
}

#[test]
fn along_normal_option_projects_world_motion_and_preserves_ordinary_move_mode() {
    let original = sheet();
    let mut along = original.snapshot();
    let mut free = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    let mut ordinary = free
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    assert_eq!(
        ordinary
            .apply_normal_distance(&mut free, 0.05, &settings)
            .changed_vertices,
        0
    );
    exact(&free, &original);
    ordinary.apply(&mut free, Vec3::new(0.02, 0.01, 0.03), &settings);
    settings.move_along_normal = true;
    let mut captured = along
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    captured.apply(&mut along, Vec3::new(0.02, 0.01, 0.03), &settings);
    assert_eq!(along.positions[index(0, 0)].x, 0.0);
    assert!(free.positions[index(0, 0)].x > 0.0);
    assert_eq!(
        along.positions[index(0, 0)].z,
        free.positions[index(0, 0)].z
    );
}

#[test]
fn offset_symmetry_reflects_every_active_dab_about_the_selected_plane() {
    let original = sheet_at(1.0, 0.0, false);
    for kind in BrushKind::ALL
        .into_iter()
        .filter(|kind| *kind != BrushKind::Morph)
    {
        let mut mesh = original.snapshot();
        let mut settings = configured(kind);
        settings.symmetry_x = true;
        settings.symmetry_offset = 1.0;
        mesh.apply_brush_with_result(
            Vec3::new(1.125, 0.0, 0.0),
            Vec3::Z,
            Vec3::new(0.1, 0.02, 0.01),
            &settings,
        );
        for y in -10..=10 {
            for x in 0..=10 {
                let a = mesh.positions[index(x, y)];
                let b = mesh.positions[index(-x, y)];
                assert!(
                    (a - Vec3::new(2.0 - b.x, b.y, b.z)).abs().max_element() < 2e-6,
                    "{kind}: {a:?} / {b:?}"
                );
                assert_eq!(mesh.masks[index(x, y)], mesh.masks[index(-x, y)]);
                assert_eq!(mesh.colors[index(x, y)], mesh.colors[index(-x, y)]);
            }
        }
        assert!(mesh
            .positions
            .iter()
            .enumerate()
            .all(|(i, p)| original.positions[i].x != 1.0 || p.x == 1.0));
    }
}

#[test]
fn offset_move_refines_both_material_patches_and_restores_the_exact_source() {
    let original = sheet_at(1.0, 0.0, false);
    let mut mesh = original.snapshot();
    let mut settings = configured(BrushKind::Move);
    settings.symmetry_x = true;
    settings.symmetry_offset = 1.0;
    settings.radius = 0.15;
    settings.adaptive_detail = true;
    let mut stroke = mesh
        .prepare_move_stroke(Vec3::new(1.25, 0.0, 0.0), Vec3::Z, &settings)
        .unwrap();
    let result = stroke.apply(&mut mesh, Vec3::Z * 0.01, &settings);
    assert!(result.changed_vertices > 0);
    assert!(result.refined_vertices > 0);
    assert!(mesh.positions.iter().any(|p| p.x < 1.0 && p.z > 0.0));
    assert!(mesh.positions.iter().any(|p| p.x > 1.0 && p.z > 0.0));
    stroke.apply(&mut mesh, Vec3::ZERO, &settings);
    exact(&mesh, &original);
}

#[test]
fn shifted_non_accumulating_draw_keeps_adaptive_proxy_points_on_the_rest_plane() {
    let original = sheet_at(1.0, 0.0, false);
    let mut mesh = original.snapshot();
    let mut settings = configured(BrushKind::Draw);
    settings.symmetry_x = true;
    settings.symmetry_offset = 1.0;
    settings.draw_accumulate = false;
    settings.adaptive_detail = true;
    let mut stroke = mesh.prepare_draw_stroke(&settings).unwrap();
    let result = stroke.apply(&mut mesh, Vec3::new(1.125, 0.0, 0.0), Vec3::Z, &settings);
    assert!(result.refined_vertices > 0);
    assert!(stroke.proxy_positions().iter().all(|p| p.z == 0.0));
    assert_eq!(stroke.proxy_positions().len(), mesh.positions.len());
    mesh.validate().unwrap();
}

#[test]
fn pressure_strength_alpha_and_masks_still_protect_new_option_paths() {
    for mode in 0..4 {
        let mut mesh = sheet();
        let mut settings = configured(BrushKind::Draw);
        settings.draw_accumulate = false;
        settings.draw_clay = true;
        settings.hardness = 0.8;
        settings.backface_culling = false;
        match mode {
            0 => settings.pressure = 0.0,
            1 => settings.strength = 0.0,
            2 => settings.alpha = Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap()),
            _ => mesh.masks.fill(1.0),
        };
        let original = mesh.snapshot();
        let mut stroke = mesh.prepare_draw_stroke(&settings).unwrap();
        assert_eq!(
            stroke
                .apply(&mut mesh, Vec3::ZERO, Vec3::Z, &settings)
                .changed_vertices,
            0
        );
        exact(&mesh, &original);
        assert_eq!(stroke.proxy_positions(), original.positions.as_slice());
    }
}

#[test]
fn camera_culling_axis_is_toward_eye_and_independent_of_surface_normal() {
    for kind in [
        BrushKind::Draw,
        BrushKind::Mask,
        BrushKind::Paint,
        BrushKind::Twist,
        BrushKind::LocalScale,
    ] {
        let original = sheet();
        let mut mesh = original.snapshot();
        let mut settings = configured(kind);
        settings.culling_axis = Some(-Vec3::Z);
        assert_eq!(dab(&mut mesh, &settings).changed_vertices, 0);
        exact(&mesh, &original);
        let mut rear = sheet_at(0.0, 0.0, true);
        assert!(dab(&mut rear, &settings).changed_vertices > 0, "{kind}");
    }
}

#[test]
fn camera_culling_normalizes_finite_extreme_axes_without_changing_response() {
    let original = sheet();
    let mut unit = original.snapshot();
    let mut settings = configured(BrushKind::Mask);
    settings.culling_axis = Some(Vec3::Z);
    dab(&mut unit, &settings);
    for magnitude in [100.0, f32::MAX, f32::from_bits(1)] {
        let mut scaled = original.snapshot();
        settings.culling_axis = Some(Vec3::Z * magnitude);
        dab(&mut scaled, &settings);
        exact(&scaled, &unit);
    }
}

#[test]
fn invalid_supplied_culling_axes_never_fall_back_or_mutate_a_live_stroke() {
    let original = sheet();
    for axis in [
        Vec3::ZERO,
        Vec3::new(f32::NAN, 0.0, 1.0),
        Vec3::new(0.0, f32::INFINITY, 1.0),
    ] {
        for kind in [
            BrushKind::Draw,
            BrushKind::Mask,
            BrushKind::Paint,
            BrushKind::Move,
            BrushKind::Twist,
            BrushKind::LocalScale,
        ] {
            let mut mesh = original.snapshot();
            let mut settings = configured(kind);
            settings.culling_axis = Some(axis);
            assert_eq!(dab(&mut mesh, &settings).changed_vertices, 0);
            exact(&mesh, &original);
            if kind == BrushKind::Move {
                assert!(mesh
                    .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
                    .is_err());
            } else if matches!(kind, BrushKind::Twist | BrushKind::LocalScale) {
                assert!(mesh
                    .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &settings)
                    .is_err());
            } else if kind == BrushKind::Draw {
                assert!(mesh.prepare_draw_stroke(&settings).is_err());
            }
        }
        let mut mesh = original.snapshot();
        let mut settings = configured(BrushKind::Move);
        let mut stroke = mesh
            .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
            .unwrap();
        stroke.apply(&mut mesh, Vec3::Z * 0.01, &settings);
        let accepted = mesh.snapshot();
        settings.culling_axis = Some(axis);
        assert_eq!(
            stroke
                .apply(&mut mesh, Vec3::Z * 0.02, &settings)
                .changed_vertices,
            0
        );
        exact(&mesh, &accepted);
    }
}

#[test]
fn anchored_camera_culling_is_captured_before_the_gesture_moves() {
    let original = joined_sheets(true);
    let back = SIDE * SIDE + index(3, 2);
    for kind in [BrushKind::Move, BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = original.snapshot();
        let mut settings = configured(kind);
        settings.culling_axis = Some(-Vec3::Z);
        if kind == BrushKind::Move {
            let mut stroke = mesh
                .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
                .unwrap();
            settings.culling_axis = Some(Vec3::Z);
            stroke.apply(&mut mesh, Vec3::X * 0.01, &settings);
        } else {
            let mut stroke = mesh
                .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &settings)
                .unwrap();
            settings.culling_axis = Some(Vec3::Z);
            stroke.apply(&mut mesh, 0.1, &settings);
        }
        assert_ne!(mesh.positions[back], original.positions[back]);
        assert_eq!(mesh.positions[index(3, 2)], original.positions[index(3, 2)]);
    }
}
