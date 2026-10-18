//! Affine fixtures with an independent scalar oracle; no reference parity claim.
use obrush_core::{Document, Mesh, ObjectTransform, Vec3};
use serde_json::Value;
use std::sync::Arc;

fn vector(value: &Value) -> [f64; 3] {
    std::array::from_fn(|i| value[i].as_f64().unwrap())
}
fn rotated(mut p: [f64; 3], t: &ObjectTransform) -> [f64; 3] {
    let [x, y, z] = t
        .rotation_degrees
        .to_array()
        .map(|a| f64::from(a).to_radians());
    let (s, c) = x.sin_cos();
    p = [p[0], c * p[1] - s * p[2], s * p[1] + c * p[2]];
    let (s, c) = y.sin_cos();
    p = [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]];
    let (s, c) = z.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1], p[2]]
}
fn point(p: [f64; 3], t: &ObjectTransform) -> [f64; 3] {
    let r = rotated(std::array::from_fn(|i| p[i] - f64::from(t.pivot[i])), t);
    std::array::from_fn(|i| {
        f64::from(t.pivot[i]) + r[i] * f64::from(t.scale) + f64::from(t.translation[i])
    })
}
fn difference(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}
fn rich_scene() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.active_mesh_mut().unwrap().masks[3] = 0.73;
    document.active_mesh_mut().unwrap().colors[3] = [0.13, 0.51, 0.91];
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.store_morph_target().unwrap();
    document.add_layer("Visible").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.017, -0.009, 0.023);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.add_layer("Hidden").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[6] += Vec3::new(-0.012, 0.008, 0.014);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(1, false).unwrap();
    document.add_layer("Faded").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[7] += Vec3::new(0.009, 0.011, -0.007);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_intensity(2, 0.37).unwrap();
    let mut other = Mesh::cube();
    for p in &mut other.positions {
        p.x += 4.0;
    }
    other.recompute_normals();
    document.add_subtool(other, "Unchanged").unwrap();
    document.select_subtool(1).unwrap();
    document
}

#[test]
fn rich_affine_matches_independent_geometry_metadata_and_normal_oracles() {
    let mut document = rich_scene();
    let before = document.to_json().unwrap();
    let old: Value = serde_json::from_str(&before).unwrap();
    let t = ObjectTransform {
        translation: Vec3::new(0.27, -0.31, 0.19),
        rotation_degrees: Vec3::new(23.0, -41.0, 67.0),
        scale: 1.25,
        pivot: Vec3::new(0.14, -0.08, 0.21),
    };
    let inactive_before = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|o| !o.active)
        .unwrap();
    assert!(document.transform_active_subtool(&t).unwrap());
    let after = document.to_json().unwrap();
    let new: Value = serde_json::from_str(&after).unwrap();
    let inactive_after = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|o| !o.active)
        .unwrap();
    assert!(Arc::ptr_eq(&inactive_before.mesh, &inactive_after.mesh));
    assert_eq!(old["scene"], new["scene"]);
    for channel in ["triangles", "masks", "colors"] {
        assert_eq!(old["mesh"][channel], new["mesh"][channel]);
    }
    let mut point_error = 0.0_f64;
    let mut normal_error = 0.0_f64;
    let mut base_error = 0.0_f64;
    let mut delta_error = 0.0_f64;
    let mut level_error = 0.0_f64;
    let mut target_error = 0.0_f64;
    for (a, b) in old["mesh"]["positions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["mesh"]["positions"].as_array().unwrap())
    {
        point_error = point_error.max(difference(point(vector(a), &t), vector(b)));
    }
    for (a, b) in old["mesh"]["normals"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["mesh"]["normals"].as_array().unwrap())
    {
        normal_error = normal_error.max(difference(rotated(vector(a), &t), vector(b)));
    }
    for (a, b) in old["layers"]["base_positions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["layers"]["base_positions"].as_array().unwrap())
    {
        base_error = base_error.max(difference(point(vector(a), &t), vector(b)));
    }
    for (a, b) in old["layers"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["layers"]["entries"].as_array().unwrap())
    {
        for key in ["name", "visible", "intensity"] {
            assert_eq!(a[key], b[key]);
        }
        for (a, b) in a["deltas"]
            .as_array()
            .unwrap()
            .iter()
            .zip(b["deltas"].as_array().unwrap())
        {
            let expected = rotated(vector(a), &t).map(|v| v * f64::from(t.scale));
            delta_error = delta_error.max(difference(expected, vector(b)));
        }
    }
    for (a, b) in old["multiresolution"]["levels"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["multiresolution"]["levels"].as_array().unwrap())
    {
        if a.is_null() {
            assert!(b.is_null());
            continue;
        }
        for (a, b) in a["positions"]
            .as_array()
            .unwrap()
            .iter()
            .zip(b["positions"].as_array().unwrap())
        {
            level_error = level_error.max(difference(point(vector(a), &t), vector(b)));
        }
    }
    for (a, b) in old["morph_target"]["positions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["morph_target"]["positions"].as_array().unwrap())
    {
        target_error = target_error.max(difference(point(vector(a), &t), vector(b)));
    }
    let measurements = serde_json::json!({"case":"rich-active-world-xyz","parameters":{"translation":t.translation.to_array(),"rotation_degrees":t.rotation_degrees.to_array(),"scale":t.scale,"pivot":t.pivot.to_array()},"max_component_error":{"visible_points":point_error,"normals":normal_error,"layer_base_f64":base_error,"layer_delta_f64":delta_error,"retained_level_points":level_error,"morph_points":target_error}});
    println!("TRANSFORM_METRICS {measurements}");
    assert!(point_error <= 2e-6 && level_error <= 2e-6 && target_error <= 2e-6);
    assert!(normal_error <= 2e-5);
    assert!(base_error <= 2e-13 && delta_error <= 2e-13);
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
fn identity_full_turns_and_sub_float_motion_preserve_exact_history() {
    let mesh = Mesh::from_obj("v -0.0 0 0\nv 1 0 -0.0\nv 0 1 0\nf 1 2 3\n").unwrap();
    let mut document = Document::new(mesh);
    document.store_morph_target().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].z = 0.01;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    let normal_bits = document
        .active_mesh()
        .unwrap()
        .normals
        .iter()
        .map(|n| n.to_array().map(f32::to_bits))
        .collect::<Vec<_>>();
    for t in [
        ObjectTransform::default(),
        ObjectTransform {
            rotation_degrees: Vec3::new(360.0, -720.0, 36000.0),
            pivot: Vec3::splat(10000.0),
            ..ObjectTransform::default()
        },
    ] {
        assert!(!document.transform_active_subtool(&t).unwrap());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.can_redo());
        assert_eq!(
            document
                .active_mesh()
                .unwrap()
                .normals
                .iter()
                .map(|n| n.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            normal_bits
        );
    }
    let mut document = Document::new(Mesh::sphere(0));
    for p in &mut document.active_mesh_mut().unwrap().positions {
        *p += Vec3::splat(100.0);
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    let before = document.to_json().unwrap();
    assert!(!document
        .transform_active_subtool(&ObjectTransform {
            translation: Vec3::splat(1e-20),
            ..ObjectTransform::default()
        })
        .unwrap());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn precision_collapse_and_bounds_failure_reject_the_complete_scene_atomically() {
    let mesh = Mesh::from_obj("v 0 0 0\nv 0.001 0 0\nv 0 0.001 0\nf 1 2 3\n").unwrap();
    let mut document = Document::new(mesh);
    document.add_subtool(Mesh::sphere(0), "Other").unwrap();
    document.select_subtool(1).unwrap();
    document.set_subtool_name(1, "Renamed").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    assert!(document
        .transform_active_subtool(&ObjectTransform {
            translation: Vec3::splat(1_000_000.0),
            ..ObjectTransform::default()
        })
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
    assert!(document
        .transform_active_subtool(&ObjectTransform {
            scale: -1.0,
            ..ObjectTransform::default()
        })
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}

#[test]
fn aabb_center_and_half_turn_keep_winding_and_material_channels() {
    let mut document = Document::new(Mesh::cube());
    document.active_mesh_mut().unwrap().masks[0] = 1.0;
    document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.3];
    assert_eq!(document.active_subtool_center().unwrap(), Vec3::ZERO);
    let triangles = document.active_mesh().unwrap().triangles.clone();
    let masks = document.active_mesh().unwrap().masks.clone();
    let colors = document.active_mesh().unwrap().colors.clone();
    assert!(document
        .transform_active_subtool(&ObjectTransform {
            rotation_degrees: Vec3::new(180.0, 0.0, 0.0),
            translation: Vec3::new(2.0, 3.0, 4.0),
            ..ObjectTransform::default()
        })
        .unwrap());
    assert_eq!(
        document.active_subtool_center().unwrap(),
        Vec3::new(2.0, 3.0, 4.0)
    );
    assert_eq!(document.active_mesh().unwrap().triangles, triangles);
    assert_eq!(document.active_mesh().unwrap().masks, masks);
    assert_eq!(document.active_mesh().unwrap().colors, colors);
}

#[test]
fn invalid_pending_stroke_is_rejected_before_transform_candidate() {
    let mut document = rich_scene();
    document.set_subtool_name(1, "Changed").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(document
        .transform_active_subtool(&ObjectTransform {
            translation: Vec3::X,
            ..ObjectTransform::default()
        })
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}

#[test]
fn rounded_away_transform_after_coarse_sculpt_preserves_unreconciled_state_and_redo() {
    let mut mesh = Mesh::sphere(1);
    for point in &mut mesh.positions {
        *point += Vec3::splat(100.0);
    }
    mesh.recompute_normals();
    let mut document = Document::new(mesh);
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.011, -0.007, 0.013);
    document.active_mesh_mut().unwrap().masks[5] = 0.73;
    document.active_mesh_mut().unwrap().colors[5] = [0.11, 0.37, 0.83];
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_subtool_name(1, "Genuine redo").unwrap();
    let named = document.to_json().unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    let before = document.to_json().unwrap();
    let positions = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect::<Vec<_>>();
    let normals = document
        .active_mesh()
        .unwrap()
        .normals
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect::<Vec<_>>();
    let changed = document
        .transform_active_subtool(&ObjectTransform {
            translation: Vec3::splat(1e-20),
            ..ObjectTransform::default()
        })
        .unwrap();
    let after = document.to_json().unwrap();
    println!(
        "NOOP_REGRESSION changed={changed} bytes_equal={} genuine_redo={}",
        before == after,
        document.can_redo()
    );
    assert!(
        !changed,
        "reconciling a retained baseline is not a useful transform"
    );
    assert_eq!(after, before);
    assert_eq!(
        document
            .active_mesh()
            .unwrap()
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        positions
    );
    assert_eq!(
        document
            .active_mesh()
            .unwrap()
            .normals
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        normals
    );
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), named);
}

#[test]
fn tiny_transform_retains_real_hidden_f64_changes_when_visible_f32_points_do_not_move() {
    let mut mesh = Mesh::sphere(1);
    for point in &mut mesh.positions {
        *point += Vec3::splat(100.0);
    }
    mesh.recompute_normals();
    let mut document = Document::new(mesh);
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.add_layer("Hidden precise offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.012, -0.019, 0.007);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.set_subtool_name(1, "Prior redo").unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    let before = document.to_json().unwrap();
    let old: Value = serde_json::from_str(&before).unwrap();
    let positions = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect::<Vec<_>>();
    let t = ObjectTransform {
        rotation_degrees: Vec3::new(0.0, 0.0, 1e-12),
        pivot: document.active_subtool_center().unwrap(),
        ..ObjectTransform::default()
    };
    assert!(document.transform_active_subtool(&t).unwrap());
    let after = document.to_json().unwrap();
    let new: Value = serde_json::from_str(&after).unwrap();
    assert_eq!(
        document
            .active_mesh()
            .unwrap()
            .positions
            .iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        positions
    );
    assert_ne!(
        old["layers"]["entries"][0]["deltas"],
        new["layers"]["entries"][0]["deltas"]
    );
    assert!(!document.can_redo());
    let mut maximum = 0.0_f64;
    let mut observed_change = 0.0_f64;
    for (a, b) in old["layers"]["entries"][0]["deltas"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new["layers"]["entries"][0]["deltas"].as_array().unwrap())
    {
        maximum = maximum.max(difference(rotated(vector(a), &t), vector(b)));
        observed_change = observed_change.max(difference(vector(a), vector(b)));
    }
    println!("HIDDEN_PRECISE_REGRESSION visible_bits_unchanged=true offset_oracle_max={maximum} observed_offset_change={observed_change}");
    assert!(observed_change > 0.0);
    assert!(maximum <= 2e-13);
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}
