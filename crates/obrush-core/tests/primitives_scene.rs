//! Independent primitive topology/volume and scene transaction contracts.
use obrush_core::{
    CylinderSettings, Document, Mesh, ObjectTransform, PlyEncoding, SubtoolMergePolicy,
    SubtoolMergeSettings, TorusSettings, Vec3,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn topology(mesh: &Mesh) -> (usize, i64, usize) {
    let mut edges = HashMap::<(u32, u32), (usize, i32)>::new();
    for face in &mesh.triangles {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let key = (a.min(b), a.max(b));
            let entry = edges.entry(key).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(
        edges
            .values()
            .all(|&(count, orientation)| count == 1 || (count == 2 && orientation == 0)),
        "nonmanifold or inconsistent winding"
    );
    let boundary = edges.values().filter(|&&(count, _)| count == 1).count();
    let chi = mesh.positions.len() as i64 - edges.len() as i64 + mesh.triangles.len() as i64;
    let mut seen = HashSet::new();
    let mut components = 0;
    for seed in 0..mesh.positions.len() as u32 {
        if seen.insert(seed) {
            components += 1;
            let mut stack = vec![seed];
            while let Some(vertex) = stack.pop() {
                for &(a, b) in edges.keys() {
                    let neighbor = if a == vertex {
                        Some(b)
                    } else if b == vertex {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(other) = neighbor {
                        if seen.insert(other) {
                            stack.push(other);
                        }
                    }
                }
            }
        }
    }
    (boundary, chi, components)
}
fn volume(mesh: &Mesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|t| {
            let a = mesh.positions[t[0] as usize].as_dvec3();
            let b = mesh.positions[t[1] as usize].as_dvec3();
            let c = mesh.positions[t[2] as usize].as_dvec3();
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 2e-6 + 1e-12,
        "actual={actual}, expected={expected}"
    );
}
fn small_cylinder() -> CylinderSettings {
    CylinderSettings {
        radial_segments: 12,
        height_segments: 3,
        ..Default::default()
    }
}
fn small_torus() -> TorusSettings {
    TorusSettings {
        radial_segments: 12,
        tubular_segments: 24,
        ..Default::default()
    }
}
fn triangle(offset: f32) -> Mesh {
    Mesh::from_obj(&format!(
        "v {offset} 0 0\nv {} 0 0\nv {offset} 1 0\nf 1 2 3\n",
        offset + 1.0
    ))
    .unwrap()
}
fn merge(ids: Vec<u32>, policy: SubtoolMergePolicy) -> SubtoolMergeSettings {
    SubtoolMergeSettings {
        ids,
        name: "Merged".into(),
        policy,
        allow_source_pose_loss: false,
        allow_mask_precision_loss: false,
        allow_uv_loss: false,
    }
}
fn bits(mesh: &Mesh) -> Vec<u8> {
    serde_json::to_vec(mesh).unwrap()
}
fn rich() -> Document {
    let mut d = Document::new(Mesh::sphere(0));
    d.active_mesh_mut().unwrap().masks[2] = 0.73;
    d.active_mesh_mut().unwrap().colors[2] = [0.1, 0.2, 0.9];
    d.add_subdivision_level().unwrap();
    d.select_level(0).unwrap();
    d.store_morph_target().unwrap();
    d.add_layer("Visible").unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[0].x += 0.012;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    d.add_layer("Hidden").unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[1].y += 0.013;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    d.set_layer_visibility(1, false).unwrap();
    d
}
#[test]
fn cylinder_raw_reference_defaults_have_expected_counts_and_bounds() {
    let s = CylinderSettings::default();
    assert_eq!(
        (
            s.radius_top,
            s.radius_bottom,
            s.height,
            s.radial_segments,
            s.height_segments,
            s.cap_top,
            s.cap_bottom
        ),
        (0.5, 0.5, 2.0, 64, 64, true, true)
    );
    let m = Mesh::cylinder(&s).unwrap();
    assert_eq!(m.positions.len(), 4162);
    assert_eq!(m.triangles.len(), 8320);
    let min = m
        .positions
        .iter()
        .fold(Vec3::splat(f32::INFINITY), |a, &b| a.min(b));
    let max = m
        .positions
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |a, &b| a.max(b));
    assert_eq!(min, Vec3::new(-0.5, -1.0, -0.5));
    assert_eq!(max, Vec3::new(0.5, 1.0, 0.5));
    assert!(volume(&m) > 0.0);
}
#[test]
fn cylinder_closed_has_genus_zero_and_analytic_polygon_volume() {
    let s = small_cylinder();
    let m = Mesh::cylinder(&s).unwrap();
    assert_eq!(topology(&m), (0, 2, 1));
    let n = f64::from(s.radial_segments);
    close(
        volume(&m),
        n * (std::f64::consts::TAU / n).sin()
            * f64::from(s.radius_top).powi(2)
            * f64::from(s.height)
            * 0.5,
    );
}
#[test]
fn cylinder_frustum_volume_is_exact_for_polygon_sections() {
    let s = CylinderSettings {
        radius_top: 0.3,
        radius_bottom: 0.7,
        height: 1.5,
        ..small_cylinder()
    };
    let m = Mesh::cylinder(&s).unwrap();
    assert_eq!(topology(&m), (0, 2, 1));
    let n = f64::from(s.radial_segments);
    let a = f64::from(s.radius_top);
    let b = f64::from(s.radius_bottom);
    close(
        volume(&m),
        n * (std::f64::consts::TAU / n).sin() * f64::from(s.height) * (a * a + a * b + b * b) / 6.0,
    );
}
#[test]
fn cylinder_cones_have_one_apex_and_no_duplicate_caps() {
    for top in [true, false] {
        let s = CylinderSettings {
            radius_top: if top { 0.0 } else { 0.5 },
            radius_bottom: if top { 0.5 } else { 0.0 },
            ..small_cylinder()
        };
        let m = Mesh::cylinder(&s).unwrap();
        assert_eq!(m.positions.len(), 38);
        assert_eq!(m.triangles.len(), 72);
        assert_eq!(topology(&m), (0, 2, 1));
        let n = f64::from(s.radial_segments);
        close(
            volume(&m),
            n * (std::f64::consts::TAU / n).sin() * 0.25 * f64::from(s.height) / 6.0,
        );
    }
}
#[test]
fn cylinder_open_caps_have_explicit_boundary_edge_counts() {
    for (top, bottom, edges) in [(false, false, 24), (true, false, 12), (false, true, 12)] {
        let s = CylinderSettings {
            cap_top: top,
            cap_bottom: bottom,
            ..small_cylinder()
        };
        let m = Mesh::cylinder(&s).unwrap();
        assert_eq!(topology(&m).0, edges);
    }
}
#[test]
fn cylinder_minimum_resolution_retains_closed_winding() {
    let m = Mesh::cylinder(&CylinderSettings {
        radial_segments: 3,
        height_segments: 1,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(topology(&m), (0, 2, 1));
    assert!(volume(&m) > 0.0);
}
#[test]
fn cylinder_invalid_dimensions_and_hostile_resolution_fail() {
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0000001, 1_000_001.0] {
        assert!(Mesh::cylinder(&CylinderSettings {
            radius_top: value,
            ..small_cylinder()
        })
        .is_err());
        assert!(Mesh::cylinder(&CylinderSettings {
            height: value,
            ..small_cylinder()
        })
        .is_err());
    }
    for s in [
        CylinderSettings {
            radius_top: 0.0,
            radius_bottom: 0.0,
            ..small_cylinder()
        },
        CylinderSettings {
            height: 0.0,
            ..small_cylinder()
        },
        CylinderSettings {
            radial_segments: 2,
            ..small_cylinder()
        },
        CylinderSettings {
            height_segments: 0,
            ..small_cylinder()
        },
        CylinderSettings {
            radial_segments: u32::MAX,
            ..small_cylinder()
        },
        CylinderSettings {
            height_segments: u32::MAX,
            ..small_cylinder()
        },
    ] {
        assert!(Mesh::cylinder(&s).is_err());
    }
}
#[test]
fn torus_raw_reference_defaults_have_expected_counts() {
    let s = TorusSettings::default();
    assert_eq!(
        (
            s.major_radius,
            s.tube_radius,
            s.arc,
            s.radial_segments,
            s.tubular_segments
        ),
        (0.5, 0.1, std::f64::consts::TAU, 32, 128)
    );
    let m = Mesh::torus(&s).unwrap();
    assert_eq!(m.positions.len(), 4096);
    assert_eq!(m.triangles.len(), 8192);
    assert!(volume(&m) > 0.0);
}
#[test]
fn torus_closed_is_genus_one_and_matches_discrete_volume() {
    let s = small_torus();
    let m = Mesh::torus(&s).unwrap();
    assert_eq!(topology(&m), (0, 0, 1));
    let radial = f64::from(s.radial_segments);
    let tubular = f64::from(s.tubular_segments);
    close(
        volume(&m),
        0.5 * radial
            * tubular
            * f64::from(s.major_radius)
            * f64::from(s.tube_radius).powi(2)
            * (std::f64::consts::TAU / radial).sin()
            * (std::f64::consts::TAU / tubular).sin(),
    );
}
#[test]
fn torus_partial_arc_closed_caps_are_genus_zero() {
    let s = TorusSettings {
        arc: std::f64::consts::PI,
        ..small_torus()
    };
    let m = Mesh::torus(&s).unwrap();
    assert_eq!(topology(&m), (0, 2, 1));
    assert_eq!(m.positions.len(), 290);
    assert_eq!(m.triangles.len(), 576);
    assert!(volume(&m) > 0.0);
}
#[test]
fn torus_partial_arc_without_caps_has_two_boundary_loops() {
    let s = TorusSettings {
        arc: std::f64::consts::PI,
        cap_ends: false,
        ..small_torus()
    };
    let m = Mesh::torus(&s).unwrap();
    assert_eq!(topology(&m), (24, 0, 1));
    assert_eq!(m.positions.len(), 288);
    assert_eq!(m.triangles.len(), 552);
}
#[test]
fn torus_full_arc_has_no_end_caps_regardless_of_flag() {
    let a = Mesh::torus(&small_torus()).unwrap();
    let b = Mesh::torus(&TorusSettings {
        cap_ends: false,
        ..small_torus()
    })
    .unwrap();
    assert_eq!(bits(&a), bits(&b));
}
#[test]
fn torus_self_intersection_arc_errors_and_hostile_resolution_fail() {
    for s in [
        TorusSettings {
            major_radius: 0.1,
            ..small_torus()
        },
        TorusSettings {
            major_radius: 0.05,
            ..small_torus()
        },
        TorusSettings {
            arc: 0.0,
            ..small_torus()
        },
        TorusSettings {
            arc: -1.0,
            ..small_torus()
        },
        TorusSettings {
            arc: f64::NAN,
            ..small_torus()
        },
        TorusSettings {
            arc: std::f64::consts::TAU + 0.001,
            ..small_torus()
        },
        TorusSettings {
            radial_segments: 2,
            ..small_torus()
        },
        TorusSettings {
            tubular_segments: 2,
            ..small_torus()
        },
        TorusSettings {
            radial_segments: u32::MAX,
            ..small_torus()
        },
        TorusSettings {
            tubular_segments: u32::MAX,
            ..small_torus()
        },
        TorusSettings {
            major_radius: 1_000_000.0,
            tube_radius: 1.0,
            ..small_torus()
        },
    ] {
        assert!(Mesh::torus(&s).is_err());
    }
}
#[test]
fn float32_collapsed_primitive_geometry_is_rejected() {
    assert!(Mesh::torus(&TorusSettings {
        major_radius: 999_999.0,
        tube_radius: 0.000001,
        ..small_torus()
    })
    .is_err());
}
#[test]
fn concatenation_retains_exact_attributes_and_index_offsets() {
    let mut a = triangle(0.0);
    let mut b = triangle(3.0);
    a.normals.fill(Vec3::new(0.1, 0.2, 0.3));
    b.normals.fill(Vec3::new(-0.0, 0.4, 0.5));
    a.masks = [0.1, 0.9, -0.0].to_vec();
    b.masks = [0.4, 1.0, 0.0].to_vec();
    a.colors = [[0.1, 0.2, 0.3]; 3].to_vec();
    b.colors = [[0.8, 0.7, 0.6]; 3].to_vec();
    let before = (bits(&a), bits(&b));
    let m = Mesh::concatenate(&[&a, &b]).unwrap();
    assert_eq!(
        m.positions,
        [a.positions.clone(), b.positions.clone()].concat()
    );
    assert_eq!(m.triangles, [[0, 1, 2], [3, 4, 5]]);
    assert_eq!(
        m.normals
            .iter()
            .flat_map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        a.normals
            .iter()
            .chain(&b.normals)
            .flat_map(|p| p.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        m.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        a.masks
            .iter()
            .chain(&b.masks)
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(m.colors, [a.colors.clone(), b.colors.clone()].concat());
    assert_eq!(before, (bits(&a), bits(&b)));
}
#[test]
fn concatenation_rejects_invalid_or_empty_inputs_before_building() {
    let a = triangle(0.0);
    let mut b = triangle(1.0);
    b.triangles[0][1] = 99;
    assert!(Mesh::concatenate(&[]).is_err());
    assert!(Mesh::concatenate(&[&a, &b]).is_err());
    assert!(Mesh::concatenate(&[&a; 33]).is_err());
}
#[test]
fn preserving_normals_append_survives_selection_and_undo_redo() {
    let mut d = Document::new(triangle(0.0));
    let before = d.to_json().unwrap();
    let mut imported = triangle(3.0);
    imported.normals.fill(Vec3::new(-0.0, 0.25, 0.5));
    let payload = bits(&imported);
    let id = d
        .add_subtool_preserving_normals(imported, "PLY normal")
        .unwrap();
    assert_eq!(bits(d.active_mesh().unwrap()), payload);
    d.select_subtool(1).unwrap();
    d.select_subtool(id).unwrap();
    assert_eq!(bits(d.active_mesh().unwrap()), payload);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(bits(d.active_mesh().unwrap()), payload);
}
#[test]
fn ordinary_append_still_recomputes_geometric_normals() {
    let mut d = Document::new(triangle(0.0));
    let mut m = triangle(2.0);
    m.normals.fill(Vec3::Y);
    d.add_subtool(m, "Generated").unwrap();
    assert_eq!(d.active_mesh().unwrap().normals, [Vec3::Z; 3]);
}
#[test]
fn preserving_normals_append_rejects_invalid_normals_atomically() {
    let mut d = Document::new(triangle(0.0));
    let before = d.to_json().unwrap();
    let mut m = triangle(2.0);
    m.normals[1].x = f32::NAN;
    assert!(d.add_subtool_preserving_normals(m, "Bad").is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
#[test]
fn merging_plain_parts_consumes_ids_with_exact_one_step_scene_history() {
    let mut d = Document::new(triangle(0.0));
    let id = d
        .add_subtool_preserving_normals(triangle(3.0), "Second")
        .unwrap();
    let before = d.to_json().unwrap();
    let r = d
        .merge_subtools(&merge(vec![id, 1], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    assert_eq!(r.consumed_ids, [id, 1]);
    assert_eq!(r.merged_id, id);
    assert_eq!(d.subtool_count(), 1);
    assert_eq!(d.active_mesh().unwrap().triangles, [[0, 1, 2], [3, 4, 5]]);
    assert!(r.flattened_subtools.is_empty());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn merge_preserves_unselected_inactive_arc_id_attributes_and_scene_order() {
    let mut d = Document::new(triangle(0.0));
    let b = d
        .add_subtool_preserving_normals(triangle(3.0), "Second")
        .unwrap();
    let c = d
        .add_subtool_preserving_normals(triangle(6.0), "Untouched")
        .unwrap();
    d.select_subtool(b).unwrap();
    let snapshot = d.subtool_render_snapshots();
    let inactive = snapshot.iter().find(|s| s.id == c).unwrap().mesh.clone();
    let payload = bits(&inactive);
    let r = d
        .merge_subtools(&merge(vec![b, 1], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    let snapshots = d.subtool_render_snapshots();
    let after = snapshots.iter().find(|s| s.id == c).unwrap();
    assert!(Arc::ptr_eq(&inactive, &after.mesh));
    assert_eq!(bits(&after.mesh), payload);
    assert_eq!(
        d.subtool_summaries()
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        [c, r.merged_id]
    );
}
#[test]
fn merging_other_parts_preserves_former_active_rich_state() {
    let mut d = rich();
    let original = 1;
    let a = d.add_subtool(triangle(3.0), "A").unwrap();
    let b = d.add_subtool(triangle(6.0), "B").unwrap();
    d.select_subtool(original).unwrap();
    let payload = d.to_json().unwrap();
    let r = d
        .merge_subtools(&merge(vec![a, b], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    assert_eq!(r.flattened_subtools.len(), 0);
    assert_eq!(d.subtool_count(), 2);
    d.select_subtool(original).unwrap();
    assert_eq!(d.layer_count(), 2);
    assert_eq!(d.level_count(), 2);
    assert!(d.has_morph_target());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), payload);
}
#[test]
fn rich_merge_requires_explicit_flattening_and_reports_each_loss() {
    let mut d = rich();
    let displayed = bits(d.active_mesh().unwrap());
    let id = d.add_subtool(triangle(3.0), "Plain").unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    let r = d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::FlattenDisplayed))
        .unwrap();
    assert_eq!(r.flattened_subtools, [1]);
    assert_eq!(r.discarded_retained_levels, 1);
    assert_eq!(r.baked_layers, 2);
    assert_eq!(r.discarded_morph_targets, 1);
    assert_eq!(d.level_count(), 1);
    assert_eq!(d.layer_count(), 0);
    assert!(!d.has_morph_target());
    let old: Mesh = serde_json::from_slice(&displayed).unwrap();
    assert_eq!(
        d.active_mesh().unwrap().positions[..old.positions.len()],
        old.positions
    );
    assert_eq!(d.active_mesh().unwrap().masks[..old.masks.len()], old.masks);
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn merge_world_geometry_after_transform_is_not_applied_twice() {
    let mut d = Document::new(triangle(0.0));
    d.transform_active_subtool(&ObjectTransform {
        translation: Vec3::new(2.0, 3.0, 4.0),
        ..Default::default()
    })
    .unwrap();
    let transformed = d.active_mesh().unwrap().positions.clone();
    let id = d.add_subtool(triangle(7.0), "Other").unwrap();
    d.merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    assert_eq!(d.active_mesh().unwrap().positions[..3], transformed);
}
#[test]
fn invalid_merge_ids_names_hidden_and_solo_leave_scene_exact() {
    let mut d = Document::new(triangle(0.0));
    let id = d.add_subtool(triangle(3.0), "Other").unwrap();
    for ids in [
        vec![],
        vec![1],
        vec![1, 1],
        vec![1, 0],
        vec![1, 999],
        vec![1; 33],
    ] {
        let before = d.to_json().unwrap();
        assert!(d
            .merge_subtools(&merge(ids, SubtoolMergePolicy::RequirePlain))
            .is_err());
        assert_eq!(d.to_json().unwrap(), before);
    }
    let mut s = merge(vec![1, id], SubtoolMergePolicy::RequirePlain);
    s.name = "bad\nname".into();
    let before = d.to_json().unwrap();
    assert!(d.merge_subtools(&s).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    d.set_subtool_visible(1, false).unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    d.set_subtool_visible(1, true).unwrap();
    d.set_solo(true).unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.solo());
}
#[test]
fn failed_merge_keeps_genuine_redo() {
    let mut d = Document::new(triangle(0.0));
    let id = d.add_subtool(triangle(3.0), "Other").unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[0].z += 0.1;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    let later = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&merge(vec![1, 99], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), later);
    assert_eq!(d.active_subtool_id().unwrap(), id);
}
#[test]
fn merge_new_ids_are_monotonic_after_consuming_originals() {
    let mut d = Document::new(triangle(0.0));
    let id = d.add_subtool(triangle(3.0), "Other").unwrap();
    let r = d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    assert_eq!(
        d.add_subtool(triangle(6.0), "Next").unwrap(),
        r.merged_id + 1
    );
}
#[test]
fn merged_plain_scene_round_trips_through_project() {
    let mut d = Document::new(triangle(0.0));
    let id = d.add_subtool(triangle(3.0), "Other").unwrap();
    d.merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .unwrap();
    let json = d.to_json().unwrap();
    assert_eq!(Document::from_json(&json).unwrap().to_json().unwrap(), json);
}
#[test]
fn ply_provided_normals_append_are_preserved_from_actual_import() {
    let mut m = triangle(3.0);
    m.normals.fill(Vec3::new(0.25, -0.5, 0.75));
    let p = m.to_ply(PlyEncoding::BinaryLittleEndian).unwrap();
    let imported = Mesh::import_ply(&p).unwrap();
    let expected = bits(&imported.mesh);
    let mut d = Document::new(triangle(0.0));
    d.add_subtool_preserving_normals(imported.mesh, "PLY")
        .unwrap();
    assert_eq!(bits(d.active_mesh().unwrap()), expected);
}
#[test]
fn rich_state_rejection_keeps_original_active_selection() {
    let mut d = rich();
    let id = d.add_subtool(triangle(3.0), "Plain").unwrap();
    d.select_subtool(1).unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.active_subtool_id().unwrap(), 1);
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn concatenation_aggregate_point_limit_rejects_two_individually_valid_meshes() {
    let mut a = triangle(0.0);
    let count = 500_001;
    a.positions.resize(count, Vec3::ZERO);
    a.normals.resize(count, Vec3::Z);
    a.masks.resize(count, 0.0);
    a.colors.resize(count, [0.5; 3]);
    a.validate().unwrap();
    let b = a.clone();
    let error = Mesh::concatenate(&[&a, &b]).unwrap_err();
    assert!(error.contains("one-million vertices"));
    assert_eq!(a.positions.len(), count);
    assert_eq!(b.positions.len(), count);
}

#[test]
fn rejected_pending_stroke_cannot_be_merged_or_capture_new_history() {
    let mut d = Document::new(triangle(0.0));
    let id = d.add_subtool(triangle(3.0), "Other").unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[0].z += 0.1;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    let later = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(d
        .merge_subtools(&merge(vec![1, id], SubtoolMergePolicy::RequirePlain))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), later);
}
