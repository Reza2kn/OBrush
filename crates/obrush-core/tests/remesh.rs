//! Independently authored solid fixtures. These measure OBrush reconstruction,
//! not DynaMesh/ZRemesher or SculptGL parity.
use obrush_core::{
    execute_topology_project, BrushKind, BrushSettings, CornerUv, Document, Mesh, Ray,
    RemeshSettings, SglStageSettings, StaticSubdivisionSettings, SubtoolMergePolicy,
    SurfaceTopology, TopologyOperation, Vec3, TRIANGLE_CORNER,
};
use std::collections::HashMap;

fn settings(resolution: u32) -> RemeshSettings {
    RemeshSettings {
        resolution,
        ..RemeshSettings::default()
    }
}

fn equal(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.masks, b.masks);
    assert_eq!(a.colors, b.colors);
}

fn box_mesh(center: Vec3, scale: Vec3) -> Mesh {
    let mut mesh = Mesh::from_obj("v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nf 1 3 2\nf 1 4 3\nf 5 6 7\nf 5 7 8\nf 1 2 6\nf 1 6 5\nf 4 8 7\nf 4 7 3\nf 1 5 8\nf 1 8 4\nf 2 3 7\nf 2 7 6\n").unwrap();
    for point in &mut mesh.positions {
        *point = *point * scale + center;
    }
    mesh.recompute_normals();
    mesh
}

fn combine(a: &Mesh, b: &Mesh) -> Mesh {
    let mut result = a.clone();
    let offset = result.positions.len() as u32;
    result.positions.extend(&b.positions);
    result.normals.extend(&b.normals);
    result.masks.extend(&b.masks);
    result.colors.extend(&b.colors);
    result
        .triangles
        .extend(b.triangles.iter().map(|ids| ids.map(|i| i + offset)));
    result
}

fn closed_manifold(mesh: &Mesh) {
    mesh.validate().unwrap();
    let mut edges = HashMap::<(u32, u32), (usize, i32)>::new();
    for &[a, b, c] in &mesh.triangles {
        let [pa, pb, pc] = [a, b, c].map(|i| mesh.positions[i as usize].as_dvec3());
        assert!((pb - pa).cross(pc - pa).length_squared() > 0.0);
        for (a, b) in [(a, b), (b, c), (c, a)] {
            let entry = edges.entry((a.min(b), a.max(b))).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges
        .values()
        .all(|&(count, winding)| count == 2 && winding == 0));
    assert!(mesh
        .normals
        .iter()
        .all(|normal| normal.is_finite() && normal.length() > 0.99));
}

#[test]
fn sphere_is_closed_and_shape_volume_are_bounded() {
    let sphere = Mesh::sphere(3);
    let (result, report) = sphere.remeshed(&settings(32)).unwrap();
    closed_manifold(&result);
    assert_eq!(report.input_components, 1);
    assert_eq!(report.output_components, 1);
    assert!(result
        .positions
        .iter()
        .all(|p| (p.length() - 1.0).abs() < 0.04));
    assert!((report.output_volume / report.input_component_volume - 1.0).abs() < 0.02);
    assert!(report.maximum_attribute_projection_distance < report.voxel_size);
    // A closed mesh can still contain folded or locally inverted triangles.
    // Reconstructing a healthy sphere must not introduce those defects.
    for &ids in &result.triangles {
        let [a, b, c] = ids.map(|i| result.positions[i as usize].as_dvec3());
        assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
    }
}

#[test]
fn marching_cubes_cube_edges_form_closed_sculptable_sphere_with_relax_toggle() {
    let source = Mesh::from_obj(include_str!("fixtures/independent-reference-sphere.obj")).unwrap();
    for smoothing_steps in [0, 1] {
        let settings = RemeshSettings {
            resolution: 16,
            marching_cubes_mode: true,
            smoothing_steps,
            ..RemeshSettings::default()
        };
        let (result, report) = source.remeshed(&settings).unwrap();
        closed_manifold(&result);
        assert!(report.marching_cubes_mode);
        assert_eq!(report.output_components, 1);
        assert!(report.output_volume > 0.0);
        assert!(report.output_vertices > 100 && report.output_triangles > 200);
        assert!(result.surface.is_none());
        eprintln!("marching_cubes resolution=16 smoothing_steps={smoothing_steps} vertices={} triangles={} volume={}", report.output_vertices, report.output_triangles, report.output_volume);
    }
}

#[test]
fn marching_cubes_preserves_closed_box_and_rejects_mode_overlap() {
    let source = Mesh::from_obj(include_str!("fixtures/independent-filter-cube.obj")).unwrap();
    let settings = RemeshSettings {
        resolution: 16,
        marching_cubes_mode: true,
        smoothing_steps: 0,
        ..RemeshSettings::default()
    };
    let (cube, report) = source.remeshed(&settings).unwrap();
    closed_manifold(&cube);
    assert_eq!(report.output_components, 1);
    assert!(report.output_volume > 0.0);
    let invalid = RemeshSettings {
        block_mode: true,
        ..settings
    };
    assert!(source.remeshed(&invalid).is_err());
}

#[test]
fn marching_cubes_preserves_torus_hole_at_resolved_grid_scale() {
    const U: usize = 24;
    const V: usize = 12;
    let mut points = Vec::new();
    let mut faces = Vec::new();
    for u in 0..U {
        let a = std::f32::consts::TAU * u as f32 / U as f32;
        for v in 0..V {
            let b = std::f32::consts::TAU * v as f32 / V as f32;
            let radius = 1.0 + 0.32 * b.cos();
            points.push(Vec3::new(
                radius * a.cos(),
                radius * a.sin(),
                0.32 * b.sin(),
            ));
        }
    }
    for u in 0..U {
        for v in 0..V {
            let a = (u * V + v) as u32;
            let b = (((u + 1) % U) * V + v) as u32;
            let c = (((u + 1) % U) * V + (v + 1) % V) as u32;
            let d = (u * V + (v + 1) % V) as u32;
            faces.extend([[a, b, c], [a, c, d]]);
        }
    }
    let mut obj = String::new();
    for p in points {
        obj.push_str(&format!("v {} {} {}\n", p.x, p.y, p.z));
    }
    for [a, b, c] in faces {
        obj.push_str(&format!("f {} {} {}\n", a + 1, b + 1, c + 1));
    }
    let source = Mesh::from_obj(&obj).unwrap();
    let (torus, report) = source
        .remeshed(&RemeshSettings {
            resolution: 32,
            marching_cubes_mode: true,
            smoothing_steps: 0,
            ..RemeshSettings::default()
        })
        .unwrap();
    closed_manifold(&torus);
    assert_eq!(report.output_components, 1);
    let edges = torus.triangles.len() * 3 / 2;
    assert_eq!(
        torus.positions.len() as isize - edges as isize + torus.triangles.len() as isize,
        0
    );
}

#[test]
fn block_mode_produces_closed_stepped_faces_and_keeps_source() {
    let source = Mesh::sphere(2);
    let original = source.clone();
    let settings = RemeshSettings {
        resolution: 20,
        block_mode: true,
        ..RemeshSettings::default()
    };
    let (block, report) = source.remeshed(&settings).unwrap();
    closed_manifold(&block);
    assert!(report.block_mode);
    assert_eq!(report.input_components, 1);
    assert_eq!(report.output_components, 1);
    assert!(report.output_volume > 0.0);
    assert!(block.triangles.iter().all(|&[a, b, c]| {
        let [a, b, c] = [a, b, c].map(|id| block.positions[id as usize]);
        let normal = (b - a).cross(c - a);
        [normal.x, normal.y, normal.z]
            .iter()
            .filter(|component| component.abs() > 1e-8)
            .count()
            == 1
    }));
    equal(&source, &original);
    let (second, _) = source.remeshed(&settings).unwrap();
    equal(&block, &second);
    let (smooth, smooth_report) = source
        .remeshed(&RemeshSettings {
            resolution: 20,
            ..RemeshSettings::default()
        })
        .unwrap();
    assert!(!smooth_report.block_mode);
    assert_ne!(block.positions, smooth.positions);
}

#[test]
fn block_remesh_is_one_undoable_document_replacement() {
    let mut document = Document::new(Mesh::sphere(2));
    let before = document.to_json().unwrap();
    let report = document
        .remesh(&RemeshSettings {
            resolution: 20,
            block_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    assert!(report.block_mode);
    let after = document.to_json().unwrap();
    assert_ne!(before, after);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn block_remesh_matches_independent_cube_reference_counts_and_bounds() {
    let source = Mesh::from_obj(include_str!("fixtures/independent-filter-cube.obj")).unwrap();
    let (block, report) = source
        .remeshed(&RemeshSettings {
            resolution: 64,
            block_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    closed_manifold(&block);
    assert_eq!(block.positions.len(), 24_578);
    assert_eq!(block.triangles.len(), 49_152);
    assert_eq!(report.output_components, 1);
    let min = block.positions.iter().copied().reduce(Vec3::min).unwrap();
    let max = block.positions.iter().copied().reduce(Vec3::max).unwrap();
    assert_eq!(min, Vec3::splat(-1.0));
    assert_eq!(max, Vec3::splat(1.0));
}

#[test]
fn block_remesh_matches_independent_sphere_reference_counts_and_volume() {
    let source = Mesh::from_obj(include_str!("fixtures/independent-reference-sphere.obj")).unwrap();
    let (block, report) = source
        .remeshed(&RemeshSettings {
            resolution: 32,
            block_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    closed_manifold(&block);
    assert_eq!(block.positions.len(), 4_746);
    assert_eq!(block.triangles.len(), 9_488);
    assert_eq!(report.output_components, 1);
    assert!((report.output_volume - 4.0673828125).abs() < 1e-10);
}

#[test]
fn smooth_surface_nets_matches_paired_fixture_topology_and_history() {
    for (source, resolution, vertices, triangles) in [
        (
            include_str!("fixtures/independent-filter-cube.obj"),
            64,
            24_578,
            49_152,
        ),
        (
            include_str!("fixtures/independent-reference-sphere.obj"),
            32,
            4_746,
            9_488,
        ),
    ] {
        let mut document = Document::new(Mesh::from_obj(source).unwrap());
        let before = document.to_json().unwrap();
        let report = document
            .remesh(&RemeshSettings {
                resolution,
                surface_nets_mode: true,
                ..RemeshSettings::default()
            })
            .unwrap();
        closed_manifold(document.mesh.as_ref().unwrap());
        assert!(report.surface_nets_mode);
        assert!(!report.block_mode);
        assert_eq!(report.distance_floor_voxels, 0.0);
        assert_eq!(document.mesh.as_ref().unwrap().positions.len(), vertices);
        assert_eq!(document.mesh.as_ref().unwrap().triangles.len(), triangles);
        let after = document.to_json().unwrap();
        assert_ne!(after, before);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}

#[test]
fn uv_bearing_marching_cubes_requires_explicit_loss_and_preserves_undo() {
    let source = Mesh::import_obj(include_bytes!("fixtures/independent-seamed-sphere.obj"))
        .unwrap()
        .mesh;
    let uv = source.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates.len(), 1_584);
    assert_eq!(uv.face_indices.len(), 528);
    let mut document = Document::new(source);
    let before = document.to_json().unwrap();
    let mut settings = RemeshSettings {
        resolution: 16,
        marching_cubes_mode: true,
        smoothing_steps: 1,
        ..RemeshSettings::default()
    };
    assert!(document.remesh(&settings).unwrap_err().contains("UV loss"));
    assert_eq!(document.to_json().unwrap(), before);
    settings.allow_uv_loss = true;
    let report = document.remesh(&settings).unwrap();
    assert_eq!(report.discarded_uv_coordinates, 1_584);
    assert_eq!(report.discarded_uv_faces, 528);
    assert!(document.mesh.as_ref().unwrap().surface.is_none());
    assert_eq!(document.mesh.as_ref().unwrap().positions.len(), 1_172);
    assert_eq!(document.mesh.as_ref().unwrap().triangles.len(), 2_340);
    // Channel sums are from the independently captured, pinned SculptGL rich
    // sphere output at resolution 16. They cover the whole reconstructed mesh.
    let mesh = document.mesh.as_ref().unwrap();
    let color_sums = [0, 1, 2].map(|channel| {
        mesh.colors
            .iter()
            .map(|color| color[channel] as f64)
            .sum::<f64>()
    });
    let material_sums = [0, 1].map(|channel| {
        mesh.materials
            .as_ref()
            .unwrap()
            .iter()
            .map(|material| material[channel] as f64)
            .sum::<f64>()
    });
    for (actual, expected) in
        color_sums
            .into_iter()
            .zip([518.2033487558365, 516.9602939337492, 577.7126770019531])
    {
        assert!((actual - expected).abs() < 1e-5);
    }
    for (actual, expected) in material_sums
        .into_iter()
        .zip([597.3066894263029, 521.7615093067288])
    {
        assert!((actual - expected).abs() < 1e-5);
    }
    let after = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn selected_overlapping_solids_fuse_in_one_atomic_scene_transaction() {
    let mut left = box_mesh(Vec3::new(-0.3, 0.0, 0.0), Vec3::ONE);
    left.colors.fill([1.0, 0.0, 0.0]);
    let mut right = box_mesh(Vec3::new(0.3, 0.0, 0.0), Vec3::ONE);
    right.colors.fill([0.0, 0.0, 1.0]);
    let untouched = box_mesh(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE);
    let mut document = Document::new(left);
    let right_id = document.add_subtool(right, "Right").unwrap();
    let untouched_id = document.add_subtool(untouched, "Untouched").unwrap();
    document
        .select_subtools(&[1, right_id], Some(right_id))
        .unwrap();
    let before = document.to_json().unwrap();
    let settings = RemeshSettings {
        resolution: 24,
        marching_cubes_mode: true,
        smoothing_steps: 0,
        ..RemeshSettings::default()
    };
    assert!(document
        .remesh_selected_subtools(
            &RemeshSettings {
                max_work: 1,
                ..settings
            },
            SubtoolMergePolicy::RequirePlain,
        )
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    let report = document
        .remesh_selected_subtools(&settings, SubtoolMergePolicy::RequirePlain)
        .unwrap();
    assert_eq!(report.consumed_ids, [1, right_id]);
    assert_eq!(report.created_id, 4);
    assert_eq!(document.selected_subtool_ids(), [4]);
    assert_eq!(
        document
            .subtool_summaries()
            .iter()
            .map(|part| part.id)
            .collect::<Vec<_>>(),
        [4, untouched_id]
    );
    assert_eq!(report.remesh.output_components, 1);
    let fused = document.active_mesh().unwrap();
    closed_manifold(fused);
    assert!(report.remesh.output_volume > 9.0 && report.remesh.output_volume < 12.0);
    assert!(fused.colors.iter().any(|color| color[0] > 0.8));
    assert!(fused.colors.iter().any(|color| color[2] > 0.8));
    let after = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn selected_mixed_uv_fusion_requires_consent_and_restores_sources_on_undo() {
    let mut left = box_mesh(Vec3::new(-0.3, 0.0, 0.0), Vec3::ONE);
    let faces = left
        .triangles
        .iter()
        .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
        .collect::<Vec<_>>();
    let coordinates = left
        .positions
        .iter()
        .map(|point| [(point.x + 1.3) / 2.0, (point.z + 1.0) / 2.0])
        .collect();
    left.surface = Some(SurfaceTopology {
        uv: Some(CornerUv {
            coordinates,
            face_indices: faces.clone(),
        }),
        faces,
    });
    left.validate().unwrap();
    let right = box_mesh(Vec3::new(0.3, 0.0, 0.0), Vec3::ONE);
    for mode in 0..3 {
        let mut document = Document::new(left.snapshot());
        let right_id = document.add_subtool(right.snapshot(), "Right").unwrap();
        document.select_subtools(&[right_id, 1], Some(1)).unwrap();
        let before = document.to_json().unwrap();
        let settings = RemeshSettings {
            resolution: 16,
            marching_cubes_mode: mode == 0,
            surface_nets_mode: mode == 1,
            block_mode: mode == 2,
            ..RemeshSettings::default()
        };
        assert!(document
            .remesh_selected_subtools(&settings, SubtoolMergePolicy::RequirePlain)
            .unwrap_err()
            .contains("explicitly allow UV loss"));
        assert_eq!(document.to_json().unwrap(), before);
        let report = document
            .remesh_selected_subtools(
                &RemeshSettings {
                    allow_uv_loss: true,
                    ..settings
                },
                SubtoolMergePolicy::RequirePlain,
            )
            .unwrap();
        assert_eq!(report.remesh.discarded_uv_coordinates, 8);
        assert_eq!(report.remesh.discarded_uv_faces, 12);
        assert!(document
            .active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .is_none_or(|s| s.uv.is_none()));
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}

#[test]
fn selected_seamed_mixed_uv_fusion_reports_loss_and_keeps_masked_source() {
    let left = Mesh::import_obj(include_bytes!(
        "fixtures/independent-fusion-left-seamed-mask.obj"
    ))
    .unwrap()
    .mesh;
    let right = Mesh::import_obj(include_bytes!("fixtures/independent-fusion-right.obj"))
        .unwrap()
        .mesh;
    assert_eq!(
        left.surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates
            .len(),
        9
    );
    assert!(left.masks.iter().any(|&value| value > 0.0 && value < 1.0));
    for mode in 0..3 {
        let mut document = Document::new(left.snapshot());
        let right_id = document.add_subtool(right.snapshot(), "Right").unwrap();
        document.select_subtools(&[right_id, 1], Some(1)).unwrap();
        let before = document.to_json().unwrap();
        let settings = RemeshSettings {
            resolution: 16,
            allow_uv_loss: true,
            marching_cubes_mode: mode == 0,
            surface_nets_mode: mode == 1,
            block_mode: mode == 2,
            ..RemeshSettings::default()
        };
        let report = document
            .remesh_selected_subtools(&settings, SubtoolMergePolicy::RequirePlain)
            .unwrap();
        assert_eq!(report.remesh.discarded_uv_coordinates, 9);
        assert_eq!(report.remesh.discarded_uv_faces, 12);
        assert!(document
            .active_mesh()
            .unwrap()
            .masks
            .iter()
            .any(|&value| value > 0.0));
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
    }
}

#[test]
fn selected_posed_source_fusion_requires_explicit_provenance_loss() {
    let fixture = include_bytes!("fixtures/independent-posed-fusion.sgl");
    for mode in 0..3 {
        let mut document = Document::empty();
        document
            .replace_sgl_bytes(fixture, &SglStageSettings::default())
            .unwrap();
        document.select_subtools(&[2, 1], Some(1)).unwrap();
        let before = document.to_json().unwrap();
        let settings = RemeshSettings {
            resolution: 16,
            marching_cubes_mode: mode == 0,
            surface_nets_mode: mode == 1,
            block_mode: mode == 2,
            ..RemeshSettings::default()
        };
        assert!(document
            .remesh_selected_subtools(&settings, SubtoolMergePolicy::RequirePlain)
            .unwrap_err()
            .contains("explicit pose/provenance loss consent"));
        assert_eq!(document.to_json().unwrap(), before);
        let report = document
            .remesh_selected_subtools(
                &RemeshSettings {
                    allow_source_pose_loss: true,
                    ..settings
                },
                SubtoolMergePolicy::RequirePlain,
            )
            .unwrap();
        assert_eq!(report.baked_pose_ids, [2, 1]);
        assert_eq!(report.discarded_source_origin_ids, [2, 1]);
        assert_eq!(document.subtool_count(), 1);
        assert!(document.active_object_metadata().is_none());
        let fused = document.active_mesh().unwrap();
        closed_manifold(fused);
        if mode == 0 {
            // The reference grows its voxel box from unrounded affine points,
            // then stores f32 source vertices. Rounding before the box shifts
            // equal-distance red/blue ownership across the overlap.
            assert!((report.remesh.voxel_size - 0.17187500186264515).abs() < 1e-14);
            let red_sum: f64 = fused.colors.iter().map(|color| f64::from(color[0])).sum();
            let blue_sum: f64 = fused.colors.iter().map(|color| f64::from(color[2])).sum();
            assert!((red_sum - 456.60167545080185).abs() < 1e-4);
            assert!((blue_sum - 577.3983218744397).abs() < 1e-4);
        } else {
            assert_eq!(fused.positions.len(), 1036);
            assert_eq!(fused.surface.as_ref().unwrap().faces.len(), 1034);
            assert!((report.remesh.voxel_size - 0.17187500186264515).abs() < 1e-14);
            let red_sum: f64 = fused.colors.iter().map(|color| f64::from(color[0])).sum();
            let blue_sum: f64 = fused.colors.iter().map(|color| f64::from(color[2])).sum();
            assert!((red_sum - 429.35631021112204).abs() < 1e-4);
            assert!((blue_sum - 606.6436871141195).abs() < 1e-4);
        }
        let min_x = fused
            .positions
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = fused
            .positions
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min_x < -1.35 && max_x > 1.25,
            "world-space poses were not baked"
        );
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}

#[test]
fn selected_fusion_project_worker_preserves_scene_contract() {
    let left = Mesh::import_obj(include_bytes!("fixtures/independent-fusion-left.obj"))
        .unwrap()
        .mesh;
    let right = Mesh::import_obj(include_bytes!("fixtures/independent-fusion-right.obj"))
        .unwrap()
        .mesh;
    let mut document = Document::new(left);
    let right_id = document.add_subtool(right, "Right").unwrap();
    document.select_subtools(&[right_id, 1], Some(1)).unwrap();
    let operation = serde_json::to_string(&TopologyOperation::RemeshSelected {
        remesh: RemeshSettings {
            resolution: 16,
            marching_cubes_mode: true,
            smoothing_steps: 1,
            ..RemeshSettings::default()
        },
        policy: SubtoolMergePolicy::RequirePlain,
    })
    .unwrap();
    let response = execute_topology_project(&document.to_json().unwrap(), &operation).unwrap();
    let response: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response["report"]["kind"], "remesh_selected");
    assert_eq!(
        response["report"]["report"]["consumed_ids"],
        serde_json::json!([right_id, 1])
    );
    let project = response["project"].as_str().unwrap();
    let remeshed = Document::from_json(project).unwrap();
    assert_eq!(remeshed.subtool_count(), 1);
    assert_eq!(remeshed.active_mesh().unwrap().positions.len(), 1_056);
    assert_eq!(remeshed.active_mesh().unwrap().triangles.len(), 2_108);
    closed_manifold(remeshed.active_mesh().unwrap());
}

#[test]
fn selected_surface_nets_modes_transport_voxel_channels_and_keep_scene_history() {
    let left = Mesh::import_obj(include_bytes!("fixtures/independent-fusion-left.obj"))
        .unwrap()
        .mesh;
    let right = Mesh::import_obj(include_bytes!("fixtures/independent-fusion-right.obj"))
        .unwrap()
        .mesh;
    for block_mode in [false, true] {
        let mut document = Document::new(left.snapshot());
        let right_id = document.add_subtool(right.snapshot(), "Right").unwrap();
        document.select_subtools(&[right_id, 1], Some(1)).unwrap();
        let before = document.to_json().unwrap();
        let settings = RemeshSettings {
            resolution: 16,
            block_mode,
            surface_nets_mode: !block_mode,
            ..RemeshSettings::default()
        };
        assert!(document
            .remesh_selected_subtools(
                &RemeshSettings {
                    max_work: 1,
                    ..settings
                },
                SubtoolMergePolicy::RequirePlain,
            )
            .is_err());
        assert_eq!(document.to_json().unwrap(), before);
        let report = document
            .remesh_selected_subtools(&settings, SubtoolMergePolicy::RequirePlain)
            .unwrap();
        assert_eq!(report.consumed_ids, [right_id, 1]);
        let fused = document.active_mesh().unwrap();
        assert_eq!(fused.positions.len(), 1_058);
        assert_eq!(fused.surface.as_ref().unwrap().faces.len(), 1_056);
        assert_eq!(fused.triangles.len(), 2_112);
        closed_manifold(fused);
        assert!(fused
            .colors
            .iter()
            .any(|color| color[0] > 0.1 && color[2] > 0.1));
        assert!(fused.materials.as_ref().unwrap().iter().all(|value| {
            value[0].to_bits() == 0.4_f32.to_bits() && value[1].to_bits() == 0.6_f32.to_bits()
        }));
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}

#[test]
fn smooth_surface_nets_preserves_constant_channels_and_rejects_budget_atomically() {
    let mut source = Mesh::from_obj(include_str!("fixtures/independent-filter-cube.obj")).unwrap();
    source.masks.fill(0.25);
    source.colors.fill([0.2, 0.4, 0.8]);
    source.materials = Some(vec![[0.35, 0.7]; source.positions.len()]);
    let original = source.clone();
    let settings = RemeshSettings {
        resolution: 32,
        surface_nets_mode: true,
        ..RemeshSettings::default()
    };
    let (output, _) = source.remeshed(&settings).unwrap();
    closed_manifold(&output);
    assert!(output.masks.iter().all(|&mask| (mask - 0.25).abs() < 1e-6));
    assert!(output.colors.iter().all(|&color| {
        color
            .iter()
            .zip([0.2, 0.4, 0.8])
            .all(|(&value, expected)| (value - expected).abs() < 1e-6)
    }));
    assert!(output.materials.as_ref().unwrap().iter().all(|&material| {
        material
            .iter()
            .zip([0.35, 0.7])
            .all(|(&value, expected)| (value - expected).abs() < 1e-6)
    }));
    assert!(source
        .remeshed(&RemeshSettings {
            max_work: 10,
            ..settings
        })
        .unwrap_err()
        .contains("work budget"));
    assert!(source
        .remeshed(&RemeshSettings {
            block_mode: true,
            ..settings
        })
        .unwrap_err()
        .contains("mutually exclusive"));
    equal(&source, &original);
    assert_eq!(source.materials, original.materials);
}

#[test]
fn surface_nets_quads_roundtrip_and_support_another_remesh() {
    // This check measures quad OBJ interchange. The fixture's explicit #MAT
    // channels require the separate opt-in attribute-loss export path.
    let plain = include_str!("fixtures/independent-filter-cube.obj")
        .lines()
        .filter(|line| !line.starts_with("#MAT"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = Mesh::from_obj(&plain).unwrap();
    let (smooth, _) = source
        .remeshed(&RemeshSettings {
            resolution: 32,
            surface_nets_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    let faces = &smooth.surface.as_ref().unwrap().faces;
    assert!(!faces.is_empty());
    assert!(smooth.surface.as_ref().unwrap().has_quads());
    assert_eq!(faces.len() * 2, smooth.triangles.len());
    let obj = smooth.export_obj_checked().unwrap();
    assert_eq!(
        obj.lines().filter(|line| line.starts_with("f ")).count(),
        faces.len()
    );
    let imported = Mesh::import_obj(obj.as_bytes()).unwrap().mesh;
    assert_eq!(imported.surface.as_ref().unwrap().faces, *faces);
    assert_eq!(imported.positions, smooth.positions);
    let document = Document::new(smooth.clone());
    let json = document.to_json().unwrap();
    let restored = Document::from_json(&json).unwrap();
    assert_eq!(restored.to_json().unwrap(), json);
    assert_eq!(restored.mesh.as_ref().unwrap().surface, smooth.surface);
    let (block, _) = smooth
        .remeshed(&RemeshSettings {
            resolution: 16,
            block_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    closed_manifold(&block);
    assert!(block.surface.as_ref().unwrap().has_quads());
    let mut with_uv = smooth.clone();
    let face_count = with_uv.surface.as_ref().unwrap().faces.len();
    with_uv.surface.as_mut().unwrap().uv = Some(obrush_core::CornerUv {
        coordinates: vec![[0.0, 0.0]],
        face_indices: vec![[0, 0, 0, 0]; face_count],
    });
    with_uv.validate().unwrap();
    assert!(with_uv
        .remeshed(&RemeshSettings {
            resolution: 16,
            block_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap_err()
        .contains("allow UV loss"));
}

#[test]
fn smooth_surface_nets_quads_remain_sculptable_after_retained_subdivision() {
    let source = Mesh::from_obj(include_str!("fixtures/independent-filter-cube.obj")).unwrap();
    let (smooth, _) = source
        .remeshed(&RemeshSettings {
            resolution: 16,
            surface_nets_mode: true,
            ..RemeshSettings::default()
        })
        .unwrap();
    let coarse_faces = smooth.surface.as_ref().unwrap().faces.len();
    let mut document = Document::new(smooth);
    let coarse = document.to_json().unwrap();
    document
        .add_static_subdivision_level(&StaticSubdivisionSettings {
            linear_edges: false,
        })
        .unwrap();
    let fine = document.active_mesh().unwrap();
    fine.validate().unwrap();
    assert_eq!(fine.surface.as_ref().unwrap().faces.len(), coarse_faces * 4);
    assert_eq!(fine.triangles.len(), coarse_faces * 8);
    assert!(fine.surface.as_ref().unwrap().uv.is_none());
    let before_stroke = document.to_json().unwrap();
    let hit = document
        .active_mesh()
        .unwrap()
        .raycast(Ray {
            origin: Vec3::new(0.0, 0.0, 3.0),
            direction: -Vec3::Z,
        })
        .unwrap();
    document.begin_stroke();
    let touched = document.active_mesh_mut().unwrap().apply_brush(
        hit.position,
        hit.normal,
        Vec3::ZERO,
        &BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            ..BrushSettings::default()
        },
    );
    document.end_stroke();
    assert!(touched > 0);
    document.active_mesh().unwrap().validate().unwrap();
    assert_eq!(
        document
            .active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .faces
            .len(),
        coarse_faces * 4
    );
    let sculpted = document.to_json().unwrap();
    assert_ne!(sculpted, before_stroke);
    assert_eq!(
        Document::from_json(&sculpted).unwrap().to_json().unwrap(),
        sculpted
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before_stroke);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), coarse);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), before_stroke);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), sculpted);
}

#[test]
fn sharp_box_reconstruction_has_one_solid_and_bounded_rounding() {
    let source = box_mesh(Vec3::ZERO, Vec3::ONE);
    let (result, report) = source.remeshed(&settings(24)).unwrap();
    closed_manifold(&result);
    assert_eq!(report.output_components, 1);
    assert!((report.output_volume - 8.0).abs() < 0.15);
    assert!(result
        .positions
        .iter()
        .all(|p| p.abs().max_element() <= 1.02));
    assert!(report.maximum_attribute_projection_distance < report.voxel_size * 1.1);
}

#[test]
fn overlapping_and_face_touching_components_become_a_single_union() {
    for separation in [1.0, 2.0] {
        let first = box_mesh(Vec3::new(-separation * 0.5, 0.0, 0.0), Vec3::ONE);
        let second = box_mesh(Vec3::new(separation * 0.5, 0.0, 0.0), Vec3::ONE);
        let source = combine(&first, &second);
        let (result, report) = source.remeshed(&settings(32)).unwrap();
        closed_manifold(&result);
        assert_eq!(report.input_components, 2);
        assert_eq!(report.output_components, 1);
        let expected_union_volume = (2.0 + separation as f64) * 4.0;
        assert!((report.output_volume / expected_union_volume - 1.0).abs() < 0.03);
        // Input component volumes overlap and are not the expected union volume.
        assert!((report.input_component_volume - 16.0).abs() < 1e-10);
    }
}

#[test]
fn disconnected_solids_stay_disconnected_and_inward_components_are_normalized() {
    let a = box_mesh(Vec3::new(-2.0, 0.0, 0.0), Vec3::splat(0.5));
    let mut b = box_mesh(Vec3::new(2.0, 0.0, 0.0), Vec3::splat(0.5));
    for triangle in &mut b.triangles {
        triangle.swap(1, 2);
    }
    b.recompute_normals();
    let (result, report) = combine(&a, &b).remeshed(&settings(40)).unwrap();
    closed_manifold(&result);
    assert_eq!(report.reversed_input_components, 1);
    assert_eq!(report.output_components, 2);
    assert!((report.output_volume / 2.0 - 1.0).abs() < 0.05);
}

#[test]
fn barycentric_attributes_retain_constant_and_spatially_varying_fields() {
    let mut source = box_mesh(Vec3::ZERO, Vec3::ONE);
    for (i, p) in source.positions.iter().enumerate() {
        source.masks[i] = (p.x + 1.0) * 0.5;
        source.colors[i] = [(p.x + 1.0) * 0.5, 0.125, (p.z + 1.0) * 0.5];
    }
    let (result, report) = source.remeshed(&settings(24)).unwrap();
    for ((p, &mask), &color) in result
        .positions
        .iter()
        .zip(&result.masks)
        .zip(&result.colors)
    {
        // Closest-point transfer projects rounded corners to the original box.
        assert!((mask - (p.x + 1.0) * 0.5).abs() < report.voxel_size as f32);
        assert!((color[0] - mask).abs() < 1e-6);
        assert!((color[1] - 0.125).abs() < 1e-6);
        assert!((color[2] - (p.z + 1.0) * 0.5).abs() < report.voxel_size as f32);
    }
}

#[test]
fn deterministic_output_and_source_survive_success_and_errors() {
    let source = Mesh::sphere(2);
    let frozen = source.clone();
    let first = source.remeshed(&settings(20)).unwrap().0;
    let second = source.remeshed(&settings(20)).unwrap().0;
    equal(&first, &second);
    equal(&source, &frozen);
    let too_small = RemeshSettings {
        max_cells: 10,
        ..settings(64)
    };
    assert!(source
        .remeshed(&too_small)
        .unwrap_err()
        .contains("cell budget"));
    let too_little_work = RemeshSettings {
        max_work: 10,
        ..settings(20)
    };
    assert!(source
        .remeshed(&too_little_work)
        .unwrap_err()
        .contains("work budget"));
    assert!(source.remeshed(&settings(u32::MAX)).is_err());
    assert!(source
        .remeshed_with_progress(&settings(20), |_| true)
        .unwrap_err()
        .contains("cancelled"));
    let mut reached_work = false;
    assert!(source
        .remeshed_with_progress(&settings(20), |progress| {
            if progress.completed == 2 {
                reached_work = true;
                true
            } else {
                false
            }
        })
        .unwrap_err()
        .contains("cancelled"));
    assert!(reached_work);
    equal(&source, &frozen);
}

#[test]
fn open_degenerate_and_inconsistently_wound_input_are_rejected_atomically() {
    let source = box_mesh(Vec3::ZERO, Vec3::ONE);
    let mut open = source.clone();
    open.triangles.pop();
    assert!(open
        .remeshed(&settings(20))
        .unwrap_err()
        .contains("closed solid"));
    let mut backwards = source.clone();
    backwards.triangles[0].swap(1, 2);
    assert!(backwards
        .remeshed(&settings(20))
        .unwrap_err()
        .contains("consistent triangle winding"));
    let mut collapsed = source.clone();
    collapsed.positions[0] = collapsed.positions[1];
    assert!(collapsed
        .remeshed(&settings(20))
        .unwrap_err()
        .contains("zero area"));
    let mut invalid = source.clone();
    invalid.triangles[0][0] = u32::MAX;
    assert!(invalid
        .remeshed(&settings(20))
        .unwrap_err()
        .contains("missing vertex"));
    equal(&source, &box_mesh(Vec3::ZERO, Vec3::ONE));
}

#[test]
fn an_edge_manifold_but_pinched_vertex_is_rejected() {
    // Two independently closed cubes meet at a corner through one vertex ID.
    // Every edge has two faces, but the shared vertex has disconnected fans.
    let a = box_mesh(Vec3::ZERO, Vec3::ONE);
    let b = box_mesh(Vec3::splat(2.0), Vec3::ONE);
    let mut source = combine(&a, &b);
    for triangle in source.triangles.iter_mut().skip(a.triangles.len()) {
        for vertex in triangle {
            if *vertex == 8 {
                *vertex = 6;
            }
        }
    }
    assert!(source
        .remeshed(&settings(20))
        .unwrap_err()
        .contains("disconnected surface fans"));
}
