//! Independent local-tool acceptance fixtures. Matching labels or these checks
//! do not establish measured SculptGL or ZBrush response equivalence.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, RasterAlpha, Vec3};

const SIDE: usize = 33;
const STEP: f32 = 0.05;
const ERROR: f64 = 3e-7;
const NEW_TOOLS: [BrushKind; 4] = [
    BrushKind::Pinch,
    BrushKind::Twist,
    BrushKind::LocalScale,
    BrushKind::Drag,
];

fn sheet() -> Mesh {
    let mut obj = String::new();
    for row in 0..SIDE {
        for column in 0..SIDE {
            let x = (column as i32 - 16) as f32 * STEP;
            let y = (row as i32 - 16) as f32 * STEP;
            obj.push_str(&format!("v {x} {y} 0 0.2 0.4 0.6\n"));
        }
    }
    for row in 0..SIDE - 1 {
        for column in 0..SIDE - 1 {
            let a = row * SIDE + column + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            obj.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }
    Mesh::from_obj(&obj).unwrap()
}

fn index(x: i32, y: i32) -> usize {
    ((y + 16) as usize) * SIDE + (x + 16) as usize
}

fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.6,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}

fn exact(first: &Mesh, second: &Mesh) {
    let vectors = |values: &[Vec3]| {
        values
            .iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(vectors(&first.positions), vectors(&second.positions));
    assert_eq!(vectors(&first.normals), vectors(&second.normals));
    assert_eq!(first.triangles, second.triangles);
    assert_eq!(
        first.masks.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        second.masks.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(
        first
            .colors
            .iter()
            .map(|x| x.map(f32::to_bits))
            .collect::<Vec<_>>(),
        second
            .colors
            .iter()
            .map(|x| x.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
}

fn unchanged_attributes(first: &Mesh, second: &Mesh) {
    assert_eq!(first.triangles, second.triangles);
    assert_eq!(first.masks, second.masks);
    assert_eq!(first.colors, second.colors);
}

fn close(actual: Vec3, expected: glam::DVec3) {
    let error = (actual.as_dvec3() - expected).abs().max_element();
    assert!(
        error <= ERROR,
        "position error {error}: {actual:?} vs {expected:?}"
    );
}

fn analytic_sheet_error(
    before: &Mesh,
    after: &Mesh,
    expected: impl Fn(Vec3) -> glam::DVec3,
) -> f64 {
    let maximum = before
        .positions
        .iter()
        .zip(&after.positions)
        .map(|(&point, actual)| (actual.as_dvec3() - expected(point)).abs().max_element())
        .fold(0.0, f64::max);
    assert!(
        maximum <= ERROR,
        "complete sheet scalar-oracle error: {maximum}"
    );
    maximum
}

fn weight(point: Vec3, radius: f32) -> f64 {
    let distance_squared = point.as_dvec3().length_squared();
    (1.0 - distance_squared / f64::from(radius).powi(2))
        .max(0.0)
        .powi(2)
}

fn direct(mesh: &mut Mesh, kind: BrushKind, settings: &BrushSettings) -> obrush_core::BrushResult {
    let delta = match kind {
        BrushKind::Twist | BrushKind::LocalScale => Vec3::X * 0.2,
        BrushKind::Drag => Vec3::new(0.04, 0.02, 0.01),
        _ => Vec3::ZERO,
    };
    mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, delta, settings)
}

fn assert_guard(old: &Mesh, new: &Mesh) {
    new.validate().unwrap();
    for &face in &old.triangles {
        let before = face.map(|v| old.positions[v as usize].as_dvec3());
        let after = face.map(|v| new.positions[v as usize].as_dvec3());
        let old_cross = (before[1] - before[0]).cross(before[2] - before[0]);
        let new_cross = (after[1] - after[0]).cross(after[2] - after[0]);
        assert!(new_cross.dot(old_cross) >= old_cross.length_squared() * (0.05 - 1e-6));
        let longest = |p: [glam::DVec3; 3]| {
            p[0].distance_squared(p[1])
                .max(p[1].distance_squared(p[2]))
                .max(p[2].distance_squared(p[0]))
        };
        let floor = (old_cross.length() / longest(before)).min(0.1);
        assert!(new_cross.length() / longest(after) + 1e-6 >= floor);
    }
}

#[test]
fn pinch_gathers_the_patch_and_inverse_spreads_with_known_response() {
    let original = sheet();
    let point = index(4, 2);
    for invert in [false, true] {
        let mut mesh = original.snapshot();
        let mut configured = settings(BrushKind::Pinch);
        configured.invert = invert;
        let result = direct(&mut mesh, configured.kind, &configured);
        assert!(result.changed_vertices > 100);
        assert_eq!(result.displacement_scale, 1.0);
        let sign = if invert { -1.0 } else { 1.0 };
        let contraction = 0.05
            * f64::from(configured.strength)
            * weight(original.positions[point], configured.radius)
            * sign;
        close(
            mesh.positions[point],
            original.positions[point].as_dvec3() * (1.0 - contraction),
        );
        let maximum = analytic_sheet_error(&original, &mesh, |point| {
            point.as_dvec3()
                * (1.0
                    - 0.05
                        * f64::from(configured.strength)
                        * weight(point, configured.radius)
                        * sign)
        });
        println!("Pinch inverse={invert}: max_scalar_oracle_error={maximum}");
        unchanged_attributes(&original, &mesh);
        assert_eq!(
            mesh.positions[index(15, 0)],
            original.positions[index(15, 0)]
        );
        assert_eq!(mesh.positions[index(0, 0)], original.positions[index(0, 0)]);
    }
}

#[test]
fn twist_has_known_angle_and_preserves_distance_to_captured_axis() {
    let original = sheet();
    let mut mesh = original.snapshot();
    let configured = settings(BrushKind::Twist);
    let mut gesture = mesh
        .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
        .unwrap();
    exact(&mesh, &original);
    let result = gesture.apply(&mut mesh, 0.4, &configured);
    assert!(result.changed_vertices > 100);
    assert_eq!(result.displacement_scale, 1.0);
    assert_eq!(gesture.kind(), BrushKind::Twist);
    assert_eq!(gesture.accepted_parameter(), 0.4 * configured.strength);
    let point = index(4, 2);
    let before = original.positions[point].as_dvec3();
    let theta = f64::from(0.4_f32 * configured.strength)
        * weight(original.positions[point], configured.radius);
    close(
        mesh.positions[point],
        glam::DVec3::new(
            before.x * theta.cos() - before.y * theta.sin(),
            before.x * theta.sin() + before.y * theta.cos(),
            0.0,
        ),
    );
    let maximum = analytic_sheet_error(&original, &mesh, |point| {
        let point64 = point.as_dvec3();
        let theta = f64::from(gesture.accepted_parameter()) * weight(point, configured.radius);
        glam::DVec3::new(
            point64.x * theta.cos() - point64.y * theta.sin(),
            point64.x * theta.sin() + point64.y * theta.cos(),
            point64.z,
        )
    });
    println!("Twist: max_scalar_oracle_error={maximum}");
    for (&a, &b) in original.positions.iter().zip(&mesh.positions) {
        assert!((a.as_dvec3().length() - b.as_dvec3().length()).abs() < 5e-7);
    }
    unchanged_attributes(&original, &mesh);
}

#[test]
fn twist_inverse_reverses_rotation_and_accepts_a_distinct_camera_axis() {
    let original = sheet();
    let axis = Vec3::new(0.0, 0.6, 0.8);
    for invert in [false, true] {
        let mut mesh = original.snapshot();
        let mut configured = settings(BrushKind::Twist);
        configured.invert = invert;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, axis, &configured)
            .unwrap();
        assert!(gesture.apply(&mut mesh, 0.2, &configured).changed_vertices > 0);
        let point = index(4, 2);
        let a = original.positions[point].as_dvec3();
        let b = mesh.positions[point].as_dvec3();
        let axis = axis.as_dvec3();
        assert!((a.dot(axis) - b.dot(axis)).abs() < 5e-7);
        assert!((a.cross(axis).length() - b.cross(axis).length()).abs() < 5e-7);
        assert_eq!(b.z.is_sign_positive(), invert);
    }
}

#[test]
fn local_scale_has_known_uniform_radial_expansion_and_reciprocal_inverse() {
    let original = sheet();
    let point = index(4, 2);
    let mut lengths = Vec::new();
    for invert in [false, true] {
        let mut mesh = original.snapshot();
        let mut configured = settings(BrushKind::LocalScale);
        configured.invert = invert;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        assert!(gesture.apply(&mut mesh, 0.4, &configured).changed_vertices > 100);
        let sign = if invert { -1.0 } else { 1.0 };
        let exponent = f64::from(0.4_f32 * configured.strength)
            * weight(original.positions[point], configured.radius)
            * sign;
        close(
            mesh.positions[point],
            original.positions[point].as_dvec3() * exponent.exp(),
        );
        let maximum = analytic_sheet_error(&original, &mesh, |point| {
            point.as_dvec3()
                * (f64::from(gesture.accepted_parameter()) * weight(point, configured.radius)).exp()
        });
        println!("Local Scale inverse={invert}: max_scalar_oracle_error={maximum}");
        lengths.push(mesh.positions[point].as_dvec3().length());
        unchanged_attributes(&original, &mesh);
    }
    assert!(
        (lengths[0] * lengths[1] - original.positions[point].as_dvec3().length_squared()).abs()
            < 5e-7
    );
}

#[test]
fn direct_transform_dabs_match_one_prepared_absolute_target() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let original = sheet();
        let mut one = original.snapshot();
        let mut other = original.snapshot();
        let configured = settings(kind);
        direct(&mut one, kind, &configured);
        let mut gesture = other
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        gesture.apply(&mut other, 0.2, &configured);
        exact(&one, &other);
    }
}

#[test]
fn absolute_transform_targets_are_event_partition_independent() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let original = sheet();
        let configured = settings(kind);
        let mut one = original.snapshot();
        let mut many = original.snapshot();
        let mut first = one
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        let mut second = many
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        first.apply(&mut one, 0.8, &configured);
        for step in 1..=40 {
            second.apply(&mut many, 0.8 * step as f32 / 40.0, &configured);
        }
        exact(&one, &many);
        assert_eq!(
            first.accepted_parameter().to_bits(),
            second.accepted_parameter().to_bits()
        );
    }
}

#[test]
fn absolute_zero_return_restores_exact_attributes_and_keeps_the_stroke_empty() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut document = Document::new(sheet());
        document.active_mesh_mut().unwrap().positions[index(0, 0)].z = -0.0;
        document.active_mesh_mut().unwrap().masks[index(0, 0)] = -0.0;
        let original = document.active_mesh().unwrap().snapshot();
        let configured = settings(kind);
        let mut gesture = document
            .active_mesh()
            .unwrap()
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        document.begin_stroke();
        gesture.apply(document.active_mesh_mut().unwrap(), 0.4, &configured);
        assert!(
            gesture
                .apply(document.active_mesh_mut().unwrap(), 0.0, &configured)
                .changed_vertices
                > 0
        );
        document.end_stroke();
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert_eq!(gesture.accepted_parameter(), 0.0);
        assert!(
            gesture
                .apply(document.active_mesh_mut().unwrap(), 0.4, &configured)
                .changed_vertices
                > 0
        );
    }
}

#[test]
fn all_new_tools_group_a_complete_stroke_as_one_exact_undo_entry() {
    for kind in NEW_TOOLS {
        let mut document = Document::new(sheet());
        let original = document.active_mesh().unwrap().snapshot();
        let configured = settings(kind);
        document.begin_stroke();
        if matches!(kind, BrushKind::Twist | BrushKind::LocalScale) {
            let mut gesture = document
                .active_mesh()
                .unwrap()
                .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
                .unwrap();
            for parameter in [0.1, 0.2, 0.3] {
                gesture.apply(document.active_mesh_mut().unwrap(), parameter, &configured);
            }
        } else {
            for _ in 0..3 {
                direct(document.active_mesh_mut().unwrap(), kind, &configured);
            }
        }
        document.end_stroke();
        let final_mesh = document.active_mesh().unwrap().snapshot();
        assert_ne!(final_mesh.positions, original.positions);
        assert!(document.undo());
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert!(document.redo());
        exact(document.active_mesh().unwrap(), &final_mesh);
    }
}

#[test]
fn fully_masked_points_are_exactly_protected_for_every_new_tool() {
    for kind in NEW_TOOLS {
        let mut mesh = sheet();
        mesh.masks.fill(1.0);
        let original = mesh.snapshot();
        assert_eq!(direct(&mut mesh, kind, &settings(kind)).changed_vertices, 0);
        exact(&mesh, &original);
    }
}

#[test]
fn partial_masks_scale_twist_angle_and_local_scale_exponent() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let original = sheet();
        let mut mesh = original.snapshot();
        let point = index(4, 2);
        mesh.masks[point] = 0.5;
        let configured = settings(kind);
        direct(&mut mesh, kind, &configured);
        let p = original.positions[point].as_dvec3();
        let parameter = f64::from(0.2_f32 * configured.strength)
            * weight(original.positions[point], configured.radius)
            * 0.5;
        let expected = if kind == BrushKind::Twist {
            glam::DVec3::new(
                p.x * parameter.cos() - p.y * parameter.sin(),
                p.x * parameter.sin() + p.y * parameter.cos(),
                0.0,
            )
        } else {
            p * parameter.exp()
        };
        close(mesh.positions[point], expected);
        assert_eq!(mesh.masks[point], 0.5);
    }
}

#[test]
fn pinch_and_drag_keep_protected_points_fixed_inside_a_partially_unmasked_patch() {
    for kind in [BrushKind::Pinch, BrushKind::Drag] {
        let mut mesh = sheet();
        let configured = settings(kind);
        let protected = index(4, 2);
        let partial = index(4, -2);
        mesh.masks[protected] = 1.0;
        mesh.masks[partial] = 0.5;
        let original = mesh.snapshot();
        let result = direct(&mut mesh, kind, &configured);
        assert!(result.changed_vertices > 100);
        assert_eq!(result.displacement_scale, 1.0);
        assert_eq!(mesh.positions[protected], original.positions[protected]);
        let point = original.positions[partial];
        let amount = f64::from(configured.strength) * weight(point, configured.radius) * 0.5;
        let expected = if kind == BrushKind::Pinch {
            point.as_dvec3() * (1.0 - amount * 0.05)
        } else {
            point.as_dvec3() + Vec3::new(0.04, 0.02, 0.01).as_dvec3() * amount
        };
        close(mesh.positions[partial], expected);
        unchanged_attributes(&original, &mesh);
    }
}

#[test]
fn pinch_and_drag_support_bounded_detail_with_whole_stroke_topology_undo() {
    for kind in [BrushKind::Pinch, BrushKind::Drag] {
        let mut document = Document::new(Mesh::sphere(2));
        let original = document.active_mesh().unwrap().snapshot();
        let mut configured = settings(kind);
        configured.radius = 0.4;
        configured.adaptive_detail = true;
        document.begin_stroke();
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            Vec3::Z,
            Vec3::Z,
            Vec3::X * 0.01,
            &configured,
        );
        assert!(result.changed_vertices > 0);
        assert!(result.refined_vertices > 0 && result.refined_vertices <= 512);
        assert_eq!(
            document.active_mesh().unwrap().positions.len(),
            original.positions.len() + result.refined_vertices
        );
        document.active_mesh().unwrap().validate().unwrap();
        document.end_stroke();
        let edited = document.active_mesh().unwrap().snapshot();
        assert!(document.undo());
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert!(document.redo());
        exact(document.active_mesh().unwrap(), &edited);
    }
}

#[test]
fn zero_pressure_or_strength_is_an_exact_no_op_and_preserves_redo() {
    for kind in NEW_TOOLS {
        for pressure in [true, false] {
            let mut document = Document::new(sheet());
            document.begin_stroke();
            document.active_mesh_mut().unwrap().colors[0] = [0.7, 0.8, 0.9];
            document.end_stroke();
            assert!(document.undo());
            let original = document.active_mesh().unwrap().snapshot();
            let mut configured = settings(kind);
            if pressure {
                configured.pressure = 0.0;
                configured.pressure_strength = false;
            } else {
                configured.strength = 0.0;
            }
            document.begin_stroke();
            assert_eq!(
                direct(document.active_mesh_mut().unwrap(), kind, &configured).changed_vertices,
                0
            );
            document.end_stroke();
            exact(document.active_mesh().unwrap(), &original);
            assert!(document.can_redo());
        }
    }
}

#[test]
fn pressure_strength_scales_transform_parameter_and_can_be_disabled() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = sheet();
        let mut configured = settings(kind);
        configured.pressure = 0.25;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        gesture.apply(&mut mesh, 0.4, &configured);
        assert_eq!(
            gesture.accepted_parameter(),
            0.4 * configured.strength * 0.25
        );
        gesture.apply(&mut mesh, 0.0, &configured);
        configured.pressure_strength = false;
        gesture.apply(&mut mesh, 0.4, &configured);
        assert_eq!(gesture.accepted_parameter(), 0.4 * configured.strength);
    }
}

#[test]
fn pressure_radius_is_captured_on_contact_and_remains_frozen() {
    let mut mesh = sheet();
    let mut configured = settings(BrushKind::LocalScale);
    configured.pressure = 0.5;
    configured.pressure_radius = true;
    let original = mesh.snapshot();
    let mut gesture = mesh
        .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
        .unwrap();
    configured.pressure = 1.0;
    assert!(gesture.apply(&mut mesh, 0.4, &configured).changed_vertices > 0);
    assert_eq!(mesh.positions[index(8, 0)], original.positions[index(8, 0)]);
    assert_ne!(mesh.positions[index(4, 0)], original.positions[index(4, 0)]);
}

#[test]
fn zero_contact_preparation_waits_for_later_positive_contact() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = sheet();
        let original = mesh.snapshot();
        let mut configured = settings(kind);
        configured.pressure = 0.0;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        assert_eq!(
            gesture.apply(&mut mesh, 0.4, &configured).changed_vertices,
            0
        );
        exact(&mesh, &original);
        configured.pressure = 1.0;
        assert!(gesture.apply(&mut mesh, 0.4, &configured).changed_vertices > 0);
    }
}

#[test]
fn black_alpha_protects_and_white_alpha_preserves_exact_untextured_response() {
    for kind in NEW_TOOLS {
        let original = sheet();
        let mut direct_mesh = original.snapshot();
        let mut white = original.snapshot();
        let mut black = original.snapshot();
        let mut configured = settings(kind);
        let direct_result = direct(&mut direct_mesh, kind, &configured);
        configured.alpha = Some(RasterAlpha::new(2, 2, vec![255; 4]).unwrap());
        assert_eq!(direct(&mut white, kind, &configured), direct_result);
        exact(&white, &direct_mesh);
        configured.alpha = Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap());
        assert_eq!(direct(&mut black, kind, &configured).changed_vertices, 0);
        exact(&black, &original);
    }
}

#[test]
fn asymmetric_alpha_creates_a_directional_local_transform_footprint() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = sheet();
        let original = mesh.snapshot();
        let mut configured = settings(kind);
        configured.alpha = Some(RasterAlpha::new(2, 1, vec![0, 255]).unwrap());
        direct(&mut mesh, kind, &configured);
        let left = index(-4, 0);
        let right = index(4, 0);
        let left_change = mesh.positions[left].distance(original.positions[left]);
        let right_change = mesh.positions[right].distance(original.positions[right]);
        assert!(right_change > left_change * 1.7);
    }
}

#[test]
fn back_facing_sheet_and_outside_footprint_remain_unchanged() {
    for kind in NEW_TOOLS {
        let mut mesh = sheet();
        for face in &mut mesh.triangles {
            face.swap(1, 2);
        }
        mesh.recompute_normals();
        let original = mesh.snapshot();
        assert_eq!(direct(&mut mesh, kind, &settings(kind)).changed_vertices, 0);
        exact(&mesh, &original);
    }
}

#[test]
fn mirrored_local_tools_keep_exactly_mirrored_pairs_and_a_single_scale_at_seam() {
    for kind in NEW_TOOLS {
        let original = sheet();
        let mut mesh = original.snapshot();
        let mut configured = settings(kind);
        configured.symmetry_x = true;
        mesh.apply_brush_with_result(
            Vec3::new(0.15, 0.0, 0.0),
            Vec3::Z,
            Vec3::new(0.12, 0.04, 0.0),
            &configured,
        );
        for y in -16..=16 {
            for x in 0..=16 {
                let a = mesh.positions[index(x, y)];
                let b = mesh.positions[index(-x, y)];
                assert!(
                    (a - Vec3::new(-b.x, b.y, b.z)).abs().max_element() <= 5e-7,
                    "{kind}: {a:?} vs {b:?}"
                );
            }
        }
        assert_ne!(mesh.positions, original.positions);
    }
    let mut single = sheet();
    let mut double = single.snapshot();
    let mut configured = settings(BrushKind::LocalScale);
    direct(&mut single, configured.kind, &configured);
    configured.symmetry_x = true;
    direct(&mut double, configured.kind, &configured);
    exact(&single, &double);
}

#[test]
fn drag_uses_the_current_advected_patch_instead_of_moves_frozen_material_grab() {
    let original = sheet();
    let mut drag = original.snapshot();
    let mut moving = original.snapshot();
    let mut configured = settings(BrushKind::Drag);
    configured.radius = 0.4;
    configured.strength = 1.0;
    let point = index(10, 0);
    assert!(drag.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::X * 0.1, &configured) > 0);
    assert!(drag.apply_brush(Vec3::X * 0.1, Vec3::Z, Vec3::X * 0.1, &configured) > 0);
    assert!(drag.apply_brush(Vec3::X * 0.2, Vec3::Z, Vec3::X * 0.1, &configured) > 0);
    configured.kind = BrushKind::Move;
    let mut gesture = moving
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &configured)
        .unwrap();
    gesture.apply(&mut moving, Vec3::X * 0.3, &configured);
    assert!(drag.positions[point].x > original.positions[point].x);
    assert_eq!(moving.positions[point], original.positions[point]);
    assert_ne!(drag.positions, moving.positions);
    assert_guard(&original, &drag);
}

#[test]
fn drag_inverse_reverses_world_motion_without_changing_attributes() {
    let original = sheet();
    let point = index(4, 2);
    for invert in [false, true] {
        let mut mesh = original.snapshot();
        let mut configured = settings(BrushKind::Drag);
        configured.invert = invert;
        let result = direct(&mut mesh, configured.kind, &configured);
        assert_eq!(result.displacement_scale, 1.0);
        let sign = if invert { -1.0 } else { 1.0 };
        close(
            mesh.positions[point],
            original.positions[point].as_dvec3()
                + glam::DVec3::new(0.04, 0.02, 0.01)
                    * f64::from(configured.strength)
                    * weight(original.positions[point], configured.radius)
                    * sign,
        );
        let maximum = analytic_sheet_error(&original, &mesh, |point| {
            point.as_dvec3()
                + Vec3::new(0.04, 0.02, 0.01).as_dvec3()
                    * f64::from(configured.strength)
                    * weight(point, configured.radius)
                    * sign
        });
        println!("Drag inverse={invert}: max_scalar_oracle_error={maximum}");
        unchanged_attributes(&original, &mesh);
    }
}

#[test]
fn semantic_or_external_surface_changes_reject_anchored_targets_atomically() {
    for changed_attribute in 0..5 {
        let mut mesh = sheet();
        let configured = settings(BrushKind::Twist);
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        gesture.apply(&mut mesh, 0.2, &configured);
        match changed_attribute {
            0 => mesh.positions[0].x += 0.01,
            1 => mesh.masks[0] = 0.5,
            2 => mesh.colors[0][0] = 0.7,
            3 => mesh.normals[0].x = 0.1,
            _ => mesh.triangles[0].swap(1, 2),
        }
        let changed = mesh.snapshot();
        let previous_parameter = gesture.accepted_parameter();
        assert_eq!(
            gesture.apply(&mut mesh, 0.4, &configured).changed_vertices,
            0
        );
        assert_eq!(gesture.accepted_parameter(), previous_parameter);
        exact(&mesh, &changed);
    }
    let mut mesh = sheet();
    let mut configured = settings(BrushKind::Twist);
    let mut gesture = mesh
        .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
        .unwrap();
    configured.kind = BrushKind::LocalScale;
    let original = mesh.snapshot();
    assert_eq!(
        gesture.apply(&mut mesh, 0.2, &configured).changed_vertices,
        0
    );
    exact(&mesh, &original);
}

#[test]
fn invalid_dab_input_is_an_exact_no_op_for_all_new_tools() {
    for kind in NEW_TOOLS {
        for scenario in 0..7 {
            let mut mesh = sheet();
            let original = mesh.snapshot();
            let mut configured = settings(kind);
            let mut center = Vec3::ZERO;
            let mut normal = Vec3::Z;
            let mut delta = Vec3::X * 0.2;
            match scenario {
                0 => configured.radius = 0.0,
                1 => configured.radius = f32::INFINITY,
                2 => configured.pressure = f32::NAN,
                3 => configured.strength = f32::NAN,
                4 => center.x = f32::INFINITY,
                5 => normal = Vec3::ZERO,
                _ => delta.y = f32::NAN,
            }
            assert_eq!(mesh.apply_brush(center, normal, delta, &configured), 0);
            exact(&mesh, &original);
        }
    }
}

#[test]
fn invalid_absolute_parameters_leave_the_last_accepted_shape_and_report_unchanged() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = sheet();
        let configured = settings(kind);
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        gesture.apply(&mut mesh, 0.2, &configured);
        let accepted = mesh.snapshot();
        let parameter = gesture.accepted_parameter();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                gesture
                    .apply(&mut mesh, invalid, &configured)
                    .changed_vertices,
                0
            );
            exact(&mesh, &accepted);
            assert_eq!(gesture.accepted_parameter(), parameter);
        }
    }
}

#[test]
fn extreme_local_targets_are_guarded_against_rest_and_report_the_accepted_parameter() {
    for (kind, parameter) in [
        (BrushKind::Twist, 4.0),
        (BrushKind::LocalScale, 12.0),
        (BrushKind::LocalScale, -12.0),
    ] {
        let original = Mesh::sphere(2);
        let mut mesh = original.snapshot();
        let mut configured = settings(kind);
        configured.radius = 0.55;
        configured.strength = 1.0;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::Z, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        let result = gesture.apply(&mut mesh, parameter, &configured);
        assert!(
            result.changed_vertices > 0,
            "a useful bounded {kind} deformation"
        );
        assert!(result.displacement_scale > 0.0 && result.displacement_scale < 1.0);
        assert!(
            gesture.accepted_parameter().abs() > 0.25,
            "large requests retain a useful transform range"
        );
        assert_eq!(
            result.displacement_scale,
            gesture.accepted_parameter() / parameter
        );
        assert_guard(&original, &mesh);
        println!(
            "{kind}: request={parameter}, accepted={}, changed={}",
            gesture.accepted_parameter(),
            result.changed_vertices
        );
        let accepted_parameter = gesture.accepted_parameter();
        let accepted = mesh.snapshot();
        let repeated = gesture.apply(&mut mesh, parameter, &configured);
        assert_eq!(repeated.changed_vertices, 0);
        exact(&mesh, &accepted);
        assert_eq!(gesture.accepted_parameter(), accepted_parameter);
    }
}

#[test]
fn guarded_absolute_targets_do_not_weaken_with_event_subdivision() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let original = Mesh::sphere(2);
        let mut one = original.snapshot();
        let mut many = original.snapshot();
        let mut configured = settings(kind);
        configured.radius = 0.55;
        configured.strength = 1.0;
        let parameter = if kind == BrushKind::Twist { 4.0 } else { 12.0 };
        let mut first = one
            .prepare_local_transform_stroke(Vec3::Z, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        let mut second = many
            .prepare_local_transform_stroke(Vec3::Z, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        first.apply(&mut one, parameter, &configured);
        for step in 1..=20 {
            second.apply(&mut many, parameter * step as f32 / 20.0, &configured);
        }
        exact(&one, &many);
        assert_eq!(first.accepted_parameter(), second.accepted_parameter());
    }
}

#[test]
fn repeated_pinch_retains_positive_faces_and_finite_normals() {
    let mut mesh = sheet();
    let configured = settings(BrushKind::Pinch);
    for _ in 0..80 {
        let previous = mesh.snapshot();
        direct(&mut mesh, configured.kind, &configured);
        assert_guard(&previous, &mesh);
    }
    for normal in &mesh.normals {
        assert!(normal.is_finite());
    }
}

#[test]
fn anchored_transform_topology_stays_fixed_even_with_adaptive_detail_enabled() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut mesh = sheet();
        let original = mesh.snapshot();
        let mut configured = settings(kind);
        configured.adaptive_detail = true;
        let mut gesture = mesh
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        let result = gesture.apply(&mut mesh, 0.4, &configured);
        assert!(result.changed_vertices > 0);
        assert_eq!(result.refined_vertices, 0);
        assert_eq!(mesh.positions.len(), original.positions.len());
        unchanged_attributes(&original, &mesh);
    }
}

#[test]
fn new_brush_identifiers_roundtrip_without_changing_existing_names() {
    for (kind, json, name) in [
        (BrushKind::Pinch, "\"pinch\"", "Pinch"),
        (BrushKind::Twist, "\"twist\"", "Twist"),
        (BrushKind::LocalScale, "\"local_scale\"", "Local Scale"),
        (BrushKind::Drag, "\"drag\"", "Drag"),
    ] {
        assert_eq!(serde_json::to_string(&kind).unwrap(), json);
        assert_eq!(serde_json::from_str::<BrushKind>(json).unwrap(), kind);
        assert_eq!(kind.name(), name);
        assert!(BrushKind::ALL.contains(&kind));
    }
    assert_eq!(BrushKind::ALL[6], BrushKind::Move);
    assert_eq!(BrushKind::ALL[9], BrushKind::Morph);
}

#[test]
fn returning_an_anchored_transform_to_zero_preserves_existing_redo() {
    for kind in [BrushKind::Twist, BrushKind::LocalScale] {
        let mut document = Document::new(sheet());
        document.begin_stroke();
        document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.3, 0.5];
        document.end_stroke();
        assert!(document.undo());
        let original = document.active_mesh().unwrap().snapshot();
        let configured = settings(kind);
        let mut gesture = document
            .active_mesh()
            .unwrap()
            .prepare_local_transform_stroke(Vec3::ZERO, Vec3::Z, Vec3::Z, &configured)
            .unwrap();
        document.begin_stroke();
        gesture.apply(document.active_mesh_mut().unwrap(), 0.4, &configured);
        gesture.apply(document.active_mesh_mut().unwrap(), 0.0, &configured);
        document.end_stroke();
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert!(document.can_redo());
        assert!(document.redo());
        assert_eq!(document.active_mesh().unwrap().colors[0], [0.1, 0.3, 0.5]);
    }
}
