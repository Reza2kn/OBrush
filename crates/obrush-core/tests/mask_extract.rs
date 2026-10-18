use obrush_core::{
    CornerUv, Document, MaskExtractSelection, MaskExtractSettings, Mesh, SurfaceTopology, Vec3,
    TRIANGLE_CORNER,
};
use std::collections::{BTreeMap, BTreeSet};

fn mesh(points: &[[f32; 3]], faces: &[[u32; 3]]) -> Mesh {
    let mut obj = String::new();
    for p in points {
        obj.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
    }
    for f in faces {
        obj.push_str(&format!("f {} {} {}\n", f[0] + 1, f[1] + 1, f[2] + 1));
    }
    Mesh::from_obj(&obj).unwrap()
}

fn square() -> Mesh {
    let mut mesh = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[[0, 1, 2], [0, 2, 3]],
    );
    mesh.masks = vec![0., 1., 1., 0.];
    mesh.colors = mesh.positions.iter().map(|p| [p.x, p.y, 0.25]).collect();
    mesh
}

fn closed_edges(mesh: &Mesh) {
    let mut edges: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    for face in &mesh.triangles {
        let [a, b, c] = face.map(|v| mesh.positions[v as usize].as_dvec3());
        assert!((b - a).cross(c - a).length_squared() > 0.0);
        for i in 0..3 {
            let a = face[i];
            let b = face[(i + 1) % 3];
            edges.entry((a.min(b), a.max(b))).or_default().push((a, b));
        }
    }
    for uses in edges.values() {
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0], (uses[1].1, uses[1].0));
    }
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|v| mesh.positions[v as usize].as_dvec3());
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

#[test]
fn whole_incident_faces_selects_original_square_and_preserves_history() {
    let source = square();
    let source_json = serde_json::to_string(&source).unwrap();
    for thickness in [0.0, -0.125, 0.125] {
        let settings = MaskExtractSettings {
            threshold: 0.5,
            thickness,
        };
        let (part, report) = source
            .extract_mask_with_selection(&settings, MaskExtractSelection::WholeIncidentFaces)
            .unwrap();
        assert_eq!(report.selected_source_triangles, 2);
        assert_eq!((report.patch_vertices, report.patch_triangles), (4, 2));
        assert_eq!(report.boundary_edges, 4);
        assert_eq!(
            (report.output_vertices, report.output_triangles),
            if thickness == 0.0 { (4, 2) } else { (8, 12) }
        );
        assert_eq!(&part.positions[..4], &source.positions);
        assert_eq!(&part.colors[..4], &source.colors);
        assert!(part.masks.iter().all(|&mask| mask == 0.0));
        if thickness != 0.0 {
            closed_edges(&part);
            assert!((volume(&part) - 0.125).abs() <= 2e-6);
            assert!(part.positions[4..].iter().all(|p| p.z == thickness));
        }
        assert_eq!(serde_json::to_string(&source).unwrap(), source_json);
    }

    let mut document = Document::new(source);
    let before = document.to_json().unwrap();
    let (part_id, report) = document
        .extract_mask_as_subtool_with_selection(
            &MaskExtractSettings {
                threshold: 0.5,
                thickness: 0.0,
            },
            MaskExtractSelection::WholeIncidentFaces,
        )
        .unwrap();
    assert_eq!(report.output_triangles, 2);
    assert_eq!(document.active_subtool_id(), Some(part_id));
    let after = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn extracted_part_can_be_appended_without_changing_the_active_source() {
    let mut document = Document::new(square());
    let before = document.to_json().unwrap();
    let (part_id, report) = document
        .extract_mask_as_subtool_with_options(
            &MaskExtractSettings {
                threshold: 0.5,
                thickness: -0.125,
            },
            MaskExtractSelection::WholeIncidentFaces,
            true,
        )
        .unwrap();
    assert_eq!(document.active_subtool_id(), Some(1));
    assert_eq!(document.subtool_count(), 2);
    assert_eq!((report.output_vertices, report.output_triangles), (8, 12));
    let source = document.active_mesh().unwrap();
    assert_eq!(source.masks, vec![0.0, 1.0, 1.0, 0.0]);
    let after = document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().active_subtool_id(),
        Some(1)
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
    document.select_subtool(part_id).unwrap();
    assert_eq!(document.active_mesh().unwrap().positions.len(), 8);
    assert!(document
        .active_mesh()
        .unwrap()
        .masks
        .iter()
        .all(|&m| m == 0.0));
}

#[test]
fn optional_sculptgl_surface_offset_matches_the_zero_thickness_sheet_and_resets_channels() {
    let mut source = square();
    source.materials = Some(vec![[0.31, 0.67]; 4]);
    let original = serde_json::to_string(&source).unwrap();
    for (thickness, near, far) in [(0.0, 0.01, None), (-0.125, -0.01, Some(-0.135))] {
        let (part, report) = source
            .extract_mask_with_surface_offset(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
            )
            .unwrap();
        assert_eq!(report.patch_vertices, 4);
        assert_eq!(report.patch_triangles, 2);
        assert!(part.positions[..4]
            .iter()
            .all(|point| (point.z - near).abs() <= 2e-8));
        if let Some(far) = far {
            assert!(part.positions[4..]
                .iter()
                .all(|point| (point.z - far).abs() <= 2e-8));
            closed_edges(&part);
        }
        assert!(part.colors.iter().all(|&color| color == [1.0; 3]));
        assert!(part
            .materials
            .as_ref()
            .unwrap()
            .iter()
            .all(|&material| material == [0.18, 0.08]));
    }
    assert_eq!(serde_json::to_string(&source).unwrap(), original);
    assert!(source
        .extract_mask_with_surface_offset(
            &MaskExtractSettings::default(),
            MaskExtractSelection::Clipped,
            true,
        )
        .is_err());
}

#[test]
fn closed_cube_surface_offset_keeps_a_manifold_shell() {
    let mut source = mesh(
        &[
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ],
        &[
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ],
    );
    source.masks[0] = 1.0;
    let original = serde_json::to_string(&source).unwrap();
    let (open, open_report) = source
        .extract_mask_with_surface_offset(
            &MaskExtractSettings {
                threshold: 0.5,
                thickness: 0.0,
            },
            MaskExtractSelection::WholeIncidentFaces,
            true,
        )
        .unwrap();
    assert_eq!(
        (open_report.output_vertices, open_report.output_triangles),
        (7, 6)
    );
    let reference_direction = Vec3::new(2.0, -1.0, -1.0).normalize();
    assert!(source.normals[1].distance(reference_direction) > 0.1);
    let expected = source.positions[1] + reference_direction * 0.01;
    assert!(open
        .positions
        .iter()
        .any(|&point| point.distance(expected) <= 1e-7));
    for thickness in [-0.125, 0.125] {
        let (part, report) = source
            .extract_mask_with_surface_offset(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
            )
            .unwrap();
        assert_eq!((report.patch_vertices, report.patch_triangles), (7, 6));
        assert_eq!((report.output_vertices, report.output_triangles), (14, 24));
        closed_edges(&part);
        assert!(part.colors.iter().all(|&color| color == [1.0; 3]));
        assert!(part.triangles.iter().all(|&[a, b, c]| {
            let [a, b, c] = [a, b, c].map(|id| part.positions[id as usize]);
            (b - a).cross(c - a).length() > 0.001
        }));
        // The source-side center is outside the border ring; the adjacent
        // corner is smoothed toward the bridge on both signed shells.
        let center = source.positions[0]
            + Vec3::splat(-1.0).normalize() * if thickness < 0.0 { -0.01 } else { 0.01 };
        assert!(part.positions[0].distance(center) < 1e-7);
        let expected_corner = if thickness < 0.0 {
            Vec3::new(-0.433_712_24, -0.433_712_24, -0.672_613)
        } else {
            Vec3::new(-0.441_287_8, -0.441_287_8, -0.702_387_03)
        };
        assert!(
            part.positions[1].distance(expected_corner) < 1e-6,
            "corner {:?} expected {:?}",
            part.positions[1],
            expected_corner
        );
    }
    assert_eq!(serde_json::to_string(&source).unwrap(), original);
}

#[test]
fn extracting_from_original_quads_selects_whole_faces_and_preserves_source_uv_history() {
    let faces = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [3, 7, 6, 2],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
    ];
    let positions = [
        [-1., -1., -1.],
        [1., -1., -1.],
        [1., 1., -1.],
        [-1., 1., -1.],
        [-1., -1., 1.],
        [1., -1., 1.],
        [1., 1., 1.],
        [-1., 1., 1.],
    ];
    let triangles: Vec<[u32; 3]> = faces
        .iter()
        .flat_map(|&[a, b, c, d]| [[a, b, c], [a, c, d]])
        .collect();
    let mut source = mesh(&positions, &triangles);
    source.masks[1] = 1.0;
    source.surface = Some(SurfaceTopology {
        faces: faces.to_vec(),
        uv: Some(CornerUv {
            coordinates: (0..6)
                .flat_map(|_| [[0., 0.], [1., 0.], [1., 1.], [0., 1.]])
                .collect(),
            face_indices: (0..6)
                .map(|face| [face * 4, face * 4 + 1, face * 4 + 2, face * 4 + 3])
                .collect(),
        }),
    });
    source.validate().unwrap();
    let source_json = serde_json::to_string(&source).unwrap();
    let zero = MaskExtractSettings {
        threshold: 0.5,
        thickness: 0.0,
    };
    let (open, report) = source
        .extract_mask_with_surface_offset(&zero, MaskExtractSelection::WholeIncidentFaces, true)
        .unwrap();
    assert_eq!((report.patch_vertices, report.patch_triangles), (7, 6));
    assert_eq!((report.output_vertices, report.output_triangles), (7, 6));
    assert!(open.surface.is_none());
    for thickness in [-0.125, 0.125] {
        let (part, report) = source
            .extract_mask_with_surface_offset(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
            )
            .unwrap();
        assert_eq!((report.output_vertices, report.output_triangles), (14, 24));
        closed_edges(&part);
        assert!(part.surface.is_none());
    }
    let (clipped, _) = source
        .extract_mask(&MaskExtractSettings::default())
        .unwrap();
    assert!(clipped.surface.is_none());
    assert_eq!(serde_json::to_string(&source).unwrap(), source_json);

    let mut document = Document::new(source);
    let before = document.to_json().unwrap();
    let (part_id, _) = document
        .extract_mask_as_subtool_with_surface_options(
            &zero,
            MaskExtractSelection::WholeIncidentFaces,
            true,
            true,
        )
        .unwrap();
    assert_eq!(document.active_subtool_id(), Some(1));
    assert_eq!(document.subtool_count(), 2);
    let after = document.to_json().unwrap();
    assert!(after.contains("face_indices"));
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
    document.select_subtool(part_id).unwrap();
    assert!(document.active_mesh().unwrap().surface.is_none());
}

#[test]
fn mixed_source_faces_admit_selected_quad_but_only_incident_triangle() {
    let positions = [
        [-1., -1., -1.],
        [1., -1., -1.],
        [1., 1., -1.],
        [-1., 1., -1.],
        [-1., -1., 1.],
        [1., -1., 1.],
        [1., 1., 1.],
        [-1., 1., 1.],
    ];
    // Five original quads and two triangles form a closed cube. Vertex 1
    // belongs to the first triangle of the split bottom face, not the second.
    let original_faces = vec![
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, TRIANGLE_CORNER],
        [0, 5, 4, TRIANGLE_CORNER],
        [3, 7, 6, 2],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
    ];
    let triangles: Vec<[u32; 3]> = original_faces
        .iter()
        .flat_map(|&face| {
            if face[3] == TRIANGLE_CORNER {
                vec![[face[0], face[1], face[2]]]
            } else {
                vec![[face[0], face[1], face[2]], [face[0], face[2], face[3]]]
            }
        })
        .collect();
    let mut source = mesh(&positions, &triangles);
    source.masks[1] = 1.0;
    source.surface = Some(SurfaceTopology {
        faces: original_faces,
        uv: None,
    });
    source.validate().unwrap();
    let before = serde_json::to_string(&source).unwrap();
    for thickness in [0.0, -0.125, 0.125] {
        let (part, report) = source
            .extract_mask_with_surface_offset(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
            )
            .unwrap();
        assert_eq!(report.selected_source_triangles, 5);
        assert_eq!((report.patch_vertices, report.patch_triangles), (6, 5));
        if thickness == 0.0 {
            assert_eq!((report.output_vertices, report.output_triangles), (6, 5));
            assert_eq!(report.boundary_edges, 5);
            let mut actual: Vec<_> = part.positions.iter().map(|p| p.to_array()).collect();
            actual.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut reference = vec![
                [-1.0057735, -1.0057735, -1.0057735],
                [1.0066667, -1.0033333, -1.0066667],
                [1.0057735, -1.0057735, 1.0057735],
                [-1.0057735, 1.0057735, -1.0057735],
                [1.0057735, 1.0057735, -1.0057735],
                [1.0057735, 1.0057735, 1.0057735],
            ];
            reference.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for (actual, reference) in actual.iter().zip(reference) {
                assert!(Vec3::from_array(*actual).distance(Vec3::from_array(reference)) < 1e-6);
            }
        } else {
            assert_eq!((report.output_vertices, report.output_triangles), (12, 20));
            closed_edges(&part);
        }
    }
    assert_eq!(serde_json::to_string(&source).unwrap(), before);
}

#[test]
fn linear_half_square_welds_shared_cuts_with_analytic_volume_and_rgb() {
    let source = square();
    let before = serde_json::to_string(&source).unwrap();
    let settings = MaskExtractSettings {
        threshold: 0.5,
        thickness: 0.125,
    };
    let (shell, report) = source.extract_mask(&settings).unwrap();
    assert_eq!((report.patch_vertices, report.patch_triangles), (5, 3));
    assert_eq!((report.boundary_edges, report.boundary_loops), (5, 1));
    assert_eq!((report.output_vertices, report.output_triangles), (10, 16));
    assert_eq!(report.patch_components, 1);
    assert_eq!(report.selected_source_triangles, 2);
    closed_edges(&shell);
    let volume_error = (volume(&shell) - 0.0625).abs();
    assert!(volume_error <= 2e-6);
    let mut rgb_error: f64 = 0.0;
    for (position, color) in shell.positions.iter().zip(&shell.colors) {
        assert!(position.x >= 0.5 && position.x <= 1.0);
        assert!(position.z == 0.0 || position.z == 0.125);
        for (a, b) in color.iter().zip([position.x, position.y, 0.25]) {
            rgb_error = rgb_error.max((f64::from(*a) - f64::from(b)).abs());
        }
    }
    assert!(rgb_error <= 1e-6);
    assert!(shell.masks.iter().all(|&m| m == 0.0));
    assert_eq!(before, serde_json::to_string(&source).unwrap());
    let (again, _) = source.extract_mask(&settings).unwrap();
    assert_eq!(
        serde_json::to_string(&shell).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
    println!(
        "MASK_EXTRACT_METRIC {}",
        serde_json::json!({"case":"half-square","parameters":{"threshold":0.5,"thickness":0.125},"maximum_rgb_error":rgb_error,"absolute_volume_error":volume_error,"volume":volume(&shell),"expected_volume":0.0625})
    );
}

#[test]
fn zero_and_inward_extraction_preserve_patch_and_signed_shell_orientation() {
    let mut source = square();
    source.materials = Some(source.positions.iter().map(|p| [p.x, p.y]).collect());
    let before = serde_json::to_string(&source).unwrap();
    let surface_settings = MaskExtractSettings {
        threshold: 0.5,
        thickness: 0.0,
    };
    let (surface, surface_report) = source.extract_mask(&surface_settings).unwrap();
    assert_eq!(
        (
            surface_report.output_vertices,
            surface_report.output_triangles
        ),
        (5, 3)
    );
    assert_eq!(surface_report.boundary_edges, 5);
    assert!(surface.positions.iter().all(|p| p.z == 0.0));
    assert!(surface.triangles.iter().all(|ids| {
        let [a, b, c] = ids.map(|v| surface.positions[v as usize]);
        (b - a).cross(c - a).z > 0.0
    }));
    assert_eq!(surface.materials.as_ref().unwrap().len(), 5);
    assert!(surface.masks.iter().all(|&m| m == 0.0));

    let inward_settings = MaskExtractSettings {
        threshold: 0.5,
        thickness: -0.125,
    };
    let (inward, report) = source.extract_mask(&inward_settings).unwrap();
    assert_eq!((report.output_vertices, report.output_triangles), (10, 16));
    assert_eq!(report.boundary_loops, 1);
    closed_edges(&inward);
    assert!((volume(&inward) - 0.0625).abs() <= 2e-6);
    assert!(inward.positions[..5].iter().all(|p| p.z == 0.0));
    assert!(inward.positions[5..].iter().all(|p| p.z == -0.125));
    for (p, rgb) in inward.positions.iter().zip(&inward.colors) {
        assert_eq!(*rgb, [p.x, p.y, 0.25]);
    }
    for thickness in [-5.0, 5.0] {
        let (bounded, report) = source
            .extract_mask(&MaskExtractSettings {
                threshold: 0.5,
                thickness,
            })
            .unwrap();
        assert_eq!(report.thickness, thickness);
        closed_edges(&bounded);
        assert!((volume(&bounded) - 2.5).abs() <= 2e-6);
    }
    assert_eq!(serde_json::to_string(&source).unwrap(), before);

    let mut sphere = Mesh::sphere(2);
    sphere.masks.fill(1.0);
    let (closed_surface, surface_report) = sphere.extract_mask(&surface_settings).unwrap();
    assert_eq!(surface_report.boundary_edges, 0);
    assert_eq!(closed_surface.positions.len(), sphere.positions.len());
    closed_edges(&closed_surface);
    let (inner_shell, inner_report) = sphere.extract_mask(&inward_settings).unwrap();
    assert_eq!(inner_report.boundary_edges, 0);
    closed_edges(&inner_shell);
    assert!(volume(&inner_shell) > 0.0);
}

#[test]
fn closed_full_mask_retains_nested_skins_and_ignores_supplied_normals() {
    let mut source = Mesh::sphere(3);
    source.masks.fill(1.0);
    source.normals.fill(Vec3::NEG_Z);
    let (shell, report) = source
        .extract_mask(&MaskExtractSettings::default())
        .unwrap();
    assert_eq!(report.patch_vertices, source.positions.len());
    assert_eq!(report.patch_triangles, source.triangles.len());
    assert_eq!((report.boundary_edges, report.boundary_loops), (0, 0));
    assert_eq!(report.patch_components, 1);
    closed_edges(&shell);
    assert!(volume(&shell) > 0.0);
    let inner_ids: BTreeMap<_, _> = shell.positions[..source.positions.len()]
        .iter()
        .enumerate()
        .map(|(i, p)| (p.to_array().map(f32::to_bits), i as u32))
        .collect();
    assert_eq!(inner_ids.len(), source.positions.len());
    let source_to_patch: Vec<_> = source
        .positions
        .iter()
        .map(|p| inner_ids[&p.to_array().map(f32::to_bits)])
        .collect();
    for i in 0..source.positions.len() {
        let delta = shell.positions[i + source.positions.len()] - shell.positions[i];
        assert!(delta.dot(shell.positions[i]) > 0.019);
        assert!((delta.length() - 0.02).abs() < 2e-6);
    }
    let shell_faces: BTreeSet<_> = shell.triangles.iter().copied().collect();
    for &[a, b, c] in &source.triangles {
        assert!(shell_faces.contains(&[
            source_to_patch[a as usize],
            source_to_patch[c as usize],
            source_to_patch[b as usize],
        ]));
    }
}

#[test]
fn invalid_fans_winding_and_precision_fail_without_touching_source() {
    let mut touching = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
        ],
        &[[0, 1, 2], [0, 3, 4]],
    );
    touching.masks.fill(1.0);
    let before = serde_json::to_string(&touching).unwrap();
    assert!(touching
        .extract_mask(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(before, serde_json::to_string(&touching).unwrap());
    let mut reversed = square();
    reversed.triangles[1] = [0, 3, 2];
    assert!(reversed
        .extract_mask(&MaskExtractSettings::default())
        .is_err());
    let mut high = square();
    high.masks.fill(1.0);
    for p in &mut high.positions {
        p.z = 100_000.0;
    }
    assert!(high
        .extract_mask(&MaskExtractSettings {
            threshold: 0.5,
            thickness: 0.0001
        })
        .unwrap_err()
        .contains("precision"));
    let mut cut = mesh(
        &[
            [100_000., 0., 0.],
            [f32::from_bits(100_000.0_f32.to_bits() + 1), 0., 0.],
            [100_000., 1., 0.],
        ],
        &[[0, 1, 2]],
    );
    cut.masks = vec![1., 0., 0.];
    assert!(cut
        .extract_mask(&MaskExtractSettings::default())
        .unwrap_err()
        .contains("collapsed"));
}

#[test]
fn rich_original_subtool_survives_append_whole_undo_redo_and_roundtrip() {
    let mut document = Document::new(Mesh::sphere(1));
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.store_morph_target().unwrap();
    document.add_layer("Hidden detail").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().colors[5] = [0.17, 0.37, 0.73];
    let mesh = document.active_mesh_mut().unwrap();
    for (position, mask) in mesh.positions.iter().zip(&mut mesh.masks) {
        *mask = ((position.z + 1.0) * 0.5).clamp(0.0, 1.0);
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.set_subtool_name(1, "Redo name").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    let original: serde_json::Value = serde_json::from_str(&before).unwrap();
    let (id, report) = document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap();
    assert_eq!(id, 2);
    assert_eq!(document.active_subtool_id().unwrap(), id);
    assert_eq!(document.subtool_count(), 2);
    assert_eq!(document.layer_count(), 0);
    assert!(!document.has_morph_target());
    assert_eq!(document.level_count(), 1);
    assert_eq!(
        document.active_mesh().unwrap().positions.len(),
        report.output_vertices
    );
    assert!(!document.redo());
    let after = document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    document.select_subtool(1).unwrap();
    let selected: serde_json::Value = serde_json::from_str(&document.to_json().unwrap()).unwrap();
    for key in ["mesh", "multiresolution", "layers", "morph_target"] {
        assert_eq!(selected[key], original[key], "original {key} changed");
    }
    document.select_subtool(id).unwrap();
    assert_eq!(document.to_json().unwrap(), after);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.active_subtool_id().unwrap(), id);
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn failed_empty_hidden_and_pending_requests_preserve_genuine_redo() {
    let mut document = Document::new(Mesh::sphere(1));
    document.set_subtool_name(1, "Redo").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    assert!(document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    document.active_mesh_mut().unwrap().masks.fill(1.0);
    document.set_subtool_visible(1, false).unwrap();
    let before = document.to_json().unwrap();
    assert!(document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .unwrap_err()
        .contains("Show"));
    assert_eq!(document.to_json().unwrap(), before);
    document.set_subtool_visible(1, true).unwrap();
    document.set_subtool_name(1, "Next redo").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x = f32::NAN;
    assert!(document
        .extract_mask_as_subtool(&MaskExtractSettings::default())
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.take_layer_error().is_some());
    assert!(document.redo());
}

#[test]
fn settings_and_source_caps_reject_before_heavy_topology_work() {
    let source = square();
    for settings in [
        MaskExtractSettings {
            threshold: f32::NAN,
            thickness: 0.02,
        },
        MaskExtractSettings {
            threshold: 0.,
            thickness: 0.02,
        },
        MaskExtractSettings {
            threshold: 0.5,
            thickness: -5.001,
        },
        MaskExtractSettings {
            threshold: 0.5,
            thickness: f32::INFINITY,
        },
    ] {
        assert!(source.extract_mask(&settings).is_err());
    }
    let mut excessive = source.snapshot();
    excessive.positions.resize(250_001, Vec3::ZERO);
    assert!(excessive
        .extract_mask(&MaskExtractSettings::default())
        .unwrap_err()
        .contains("250000"));
    let mut excessive = source.snapshot();
    excessive.triangles.resize(500_001, [0, 1, 2]);
    assert!(excessive
        .extract_mask(&MaskExtractSettings::default())
        .unwrap_err()
        .contains("500000"));
}
