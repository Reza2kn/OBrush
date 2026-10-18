use obrush_core::{
    AttributeExportPolicy, BrushKind, BrushSettings, CornerUv, Document, Mesh,
    StaticSubdivisionSettings, SurfaceTopology, Vec3, TRIANGLE_CORNER,
};

fn bits(mesh: &Mesh) -> String {
    serde_json::to_string(mesh).unwrap()
}

fn rich_quad() -> Mesh {
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n").unwrap();
    mesh.colors = vec![
        [0.25, 0.5, 0.75],
        [0.75, 0.25, 0.5],
        [0.5, 0.75, 0.25],
        [0.125, 0.25, 0.5],
    ];
    mesh.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3]],
        uv: Some(CornerUv {
            // Include an unused prefix and equal-valued but distinct seam IDs.
            coordinates: vec![
                [9.0, -2.0],
                [0.0, -0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, -0.0],
            ],
            face_indices: vec![[5, 2, 3, 4]],
        }),
    });
    mesh
}

#[test]
fn own_obj_quad_return_preserves_channels_corner_ids_and_canonical_diagonal() {
    let mesh = rich_quad();
    let output = mesh
        .export_obj_with_policy(AttributeExportPolicy::RejectLoss)
        .unwrap();
    let imported = Mesh::import_obj(&output.bytes).unwrap();
    assert_eq!(bits(&imported.mesh), bits(&mesh));
    assert_eq!(imported.report.source_faces, 1);
    assert_eq!(imported.report.triangulated_faces, 2);
    assert!(imported.report.imported_colors);
    assert!(!imported.report.imported_normals);
    assert_eq!(imported.report.warnings.len(), 1);
}

#[test]
fn triangle_uv_return_retains_equal_values_as_distinct_ids() {
    let mut mesh = rich_quad();
    let uv = mesh.surface.as_mut().unwrap().uv.as_mut().unwrap();
    uv.face_indices = vec![[1, 2, 3, TRIANGLE_CORNER], [5, 3, 4, TRIANGLE_CORNER]];
    mesh.surface.as_mut().unwrap().faces =
        vec![[0, 1, 2, TRIANGLE_CORNER], [0, 2, 3, TRIANGLE_CORNER]];
    let imported = Mesh::import_obj(mesh.export_obj_checked().unwrap().as_bytes()).unwrap();
    assert_eq!(bits(&imported.mesh), bits(&mesh));
}

#[test]
fn relative_uv_normal_and_vertex_indices_use_current_declarations() {
    let source = b"v 0 0 0\nv 2 0 0\nv 2 2 0\nv 0 2 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvn 1 2 3\nf -4/-4/-1 -3/-3/-1 -2/-2/-1 -1/-1/-1\n";
    let mesh = Mesh::import_obj(source).unwrap().mesh;
    assert_eq!(mesh.triangles, [[0, 1, 2], [0, 2, 3]]);
    let surface = mesh.surface.unwrap();
    assert_eq!(surface.faces, [[0, 1, 2, 3]]);
    assert_eq!(surface.uv.unwrap().face_indices, [[0, 1, 2, 3]]);
}

#[test]
fn source_obj_without_vertex_colors_uses_sculptgl_white() {
    let source = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n";
    let imported = Mesh::import_obj(source).unwrap();
    assert_eq!(imported.mesh.colors, vec![[1.0, 1.0, 1.0]; 3]);
    assert!(!imported.report.imported_colors);
    assert_eq!(
        Mesh::from_obj(std::str::from_utf8(source).unwrap())
            .unwrap()
            .colors,
        vec![[0.65, 0.69, 0.74]; 3]
    );
}

#[test]
fn plain_triangle_keeps_legacy_editability_and_geometry_color_bits() {
    let source =
        "v 0 0 0 2 0.25 0.5 0.75\nv 2 0 0 2 0.125 0.25 0.5\nv 0 2 0 2 0.5 0.75 1\nf 1 2 3\n";
    let legacy = Mesh::from_obj(source).unwrap();
    let imported = Mesh::import_obj(source.as_bytes()).unwrap().mesh;
    assert_eq!(bits(&imported), bits(&legacy));
    assert!(imported.supports_adaptive_detail());
}

#[test]
fn concave_polygon_ear_clipping_keeps_corner_uv_mapping() {
    let source = b"v 0 0 0\nv 2 0 0\nv 2 2 0\nv 1 1 0\nv 0 2 0\nvt 0 0\nvt 2 0\nvt 2 2\nvt 1 1\nvt 0 2\nf 1/1 2/2 3/3 4/4 5/5\n";
    let imported = Mesh::import_obj(source).unwrap();
    assert_eq!(imported.mesh.triangles.len(), 3);
    let surface = imported.mesh.surface.as_ref().unwrap();
    for (triangle, uv) in imported
        .mesh
        .triangles
        .iter()
        .zip(&surface.uv.as_ref().unwrap().face_indices)
    {
        assert_eq!(triangle, &uv[..3]);
        assert_eq!(uv[3], TRIANGLE_CORNER);
    }
    let area: f32 = imported
        .mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| imported.mesh.positions[i as usize]);
            (b - a).cross(c - a).z * 0.5
        })
        .sum();
    assert_eq!(area, 3.0);
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|s| s.starts_with("Triangulated 1 polygons")));
}

#[test]
fn unsuitable_quad_diagonal_becomes_uv_triangles_with_report() {
    // The reflex corner at index1 makes diagonal0–2 lie outside this polygon.
    let source = b"v 0 0 0\nv 0.4 0.4 0\nv 2 0 0\nv 0 2 0\nvt 0 0\nvt 0.2 0.2\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3 4/4\n";
    let imported = Mesh::import_obj(source).unwrap();
    let surface = imported.mesh.surface.as_ref().unwrap();
    assert!(!surface.has_quads());
    assert_eq!(surface.faces.len(), 2);
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|s| s.contains("1 quads")));
    for (tri, uv) in imported
        .mesh
        .triangles
        .iter()
        .zip(&surface.uv.as_ref().unwrap().face_indices)
    {
        assert_eq!(tri, &uv[..3]);
    }
}

#[test]
fn mrgb_precedence_and_sculptgl_mat_channel_semantics_are_explicit() {
    let source = b"v 0 0 0 1 0 0\nv 1 0 0 1 0 0\nv 0 1 0 1 0 0\n#MRGB ff004080 00010203 7fffeedd\n#MAT 0010ff 807f00 ff4001\nf 1 2 3\n";
    let imported = Mesh::import_obj(source).unwrap();
    assert_eq!(imported.mesh.colors[0], [0.0, 64.0 / 255.0, 128.0 / 255.0]);
    assert_eq!(
        imported.mesh.colors[1],
        [1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0]
    );
    assert_eq!(
        imported.mesh.materials.unwrap(),
        [
            [0.0, 16.0 / 255.0],
            [128.0 / 255.0, 127.0 / 255.0],
            [1.0, 64.0 / 255.0]
        ]
    );
    assert_eq!(imported.mesh.masks, [0.0, 1.0, 1.0 - 1.0 / 255.0]);
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|s| s.contains("mask byte")));
}

#[test]
fn malformed_attributes_reject_without_changing_document_or_redo() {
    let mut document = Document::new(rich_quad());
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].z = 0.1;
    document.end_stroke();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    let prefix = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\n";
    for tail in [
        "f 1/1/1 2/2/1 3/4/1\n",
        "f 1/1/1 2/2/2 3/3/1\n",
        "f 1/1 2 3/3\n",
        "f 1/1/1/1 2/2/1 3/3/1\n",
        "f 0/1 2/2 3/3\n",
        "f 1/1 2/2 2/3\n",
        "f -4/1 2/2 3/3\n",
        "vt NaN 0\nf 1/1 2/2 3/3\n",
        "#MAT 000000\nf 1 2 3\n",
        "#MRGB ffffffgf ffffffff ffffffff\nf 1 2 3\n",
        "#MAT\nf 1 2 3\n",
        "f 1/1 2/2 3/3\nvt 1000001 0\n",
    ] {
        assert!(
            Mesh::import_obj(format!("{prefix}{tail}").as_bytes()).is_err(),
            "accepted {tail}"
        );
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.can_redo());
    }
    assert!(document.redo());
    assert_eq!(document.active_mesh().unwrap().positions[0].z, 0.1);
}

#[test]
fn unsupported_object_material_and_uv_w_data_are_reported() {
    let source = b"o first\nv 0 0 0\nv 1 0 0\nv 0 1 0\no second\nmtllib studio.mtl\nusemtl red\ng face\nvt 0 0 7\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n";
    let imported = Mesh::import_obj(source).unwrap();
    assert_eq!(
        imported.report.skipped_properties,
        ["mtllib", "usemtl", "g"]
    );
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|s| s.contains("object boundaries")));
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|s| s.contains("texture w")));
}

#[test]
fn repeated_big_polygons_hit_preflight_work_limit_before_mesh_allocation() {
    let mut source = "v 0 0 0\n".repeat(512);
    source.push('f');
    for i in 1..=512 {
        source.push_str(&format!(" {i}"));
    }
    let error = Mesh::import_obj(source.as_bytes()).unwrap_err();
    assert!(error.contains("polygon work"), "{error}");
}

#[test]
fn imported_quad_uv_static_hierarchy_accepts_first_material_paint_and_exact_history() {
    for linear_edges in [false, true] {
        let mut document = Document::new(rich_quad());
        document
            .add_static_subdivision_level(&StaticSubdivisionSettings { linear_edges })
            .unwrap();
        assert_eq!(
            document
                .retained_static_subdivision_settings()
                .unwrap()
                .linear_edges,
            linear_edges
        );
        let before = document.to_json().unwrap();
        let geometry = document.active_mesh().unwrap().positions.clone();
        let colors = document.active_mesh().unwrap().colors.clone();
        let surface = document.active_mesh().unwrap().surface.clone();
        document.begin_material_paint_stroke();
        let changed = document.active_mesh_mut().unwrap().apply_brush(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::Z,
            Vec3::ZERO,
            &BrushSettings {
                kind: BrushKind::Paint,
                radius: 0.8,
                strength: 0.6,
                symmetry_x: false,
                adaptive_detail: false,
                paint_rgb: false,
                paint_roughness: true,
                roughness: 0.85,
                ..Default::default()
            },
        );
        assert!(changed > 0);
        document.end_stroke();
        let painted = document.to_json().unwrap();
        assert_eq!(document.active_mesh().unwrap().positions, geometry);
        assert_eq!(document.active_mesh().unwrap().colors, colors);
        assert_eq!(document.active_mesh().unwrap().surface, surface);
        assert_eq!(
            Document::from_json(&painted).unwrap().to_json().unwrap(),
            painted
        );
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), painted);
        let fine = bits(document.active_mesh().unwrap());
        document.select_level(0).unwrap();
        document.select_level(1).unwrap();
        assert_eq!(bits(document.active_mesh().unwrap()), fine);
    }
}
