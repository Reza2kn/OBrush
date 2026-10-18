use obrush_core::{
    Document, MaskRegion, MaskRegionSettings, Mesh, Vec3, MASK_REGION_BOUNDARY_EPSILON,
};
use std::sync::Arc;

const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

#[test]
fn lasso_endpoint_band_uses_finite_segment_distance() {
    let source = Mesh::from_obj("v 0 0 0\nv 0.5 0 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
    let extent = 2.0 + 1.8 * MASK_REGION_BOUNDARY_EPSILON;
    let settings = MaskRegionSettings {
        region: MaskRegion::Lasso {
            points: vec![[0., 0.], [1., 1.], [1., 0.]],
        },
        view_projection: IDENTITY,
        viewport: [0., 0., extent, extent],
        erase: false,
    };
    let (masks, _) = source.region_masks(&settings).unwrap();
    let projected = extent * 0.5;
    let distance = (projected - 1.0).hypot(projected - 1.0);
    assert!(distance > MASK_REGION_BOUNDARY_EPSILON);
    println!(
        "MASK_REGION_ENDPOINT {}",
        serde_json::json!({"probe_mask":masks[0],"expected_probe_mask":0.0,"finite_segment_distance":distance,"epsilon":MASK_REGION_BOUNDARY_EPSILON,"extent":extent})
    );
    assert_eq!(
        masks[0], 0.0,
        "the diagonal beyond-end probe is outside the finite segment band"
    );
}

#[test]
fn finite_noncollinear_lasso_smaller_than_squared_f64_precision_is_valid() {
    let source = Mesh::from_obj("v 0 0 0\nv 0.5 0 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
    let settings = MaskRegionSettings {
        region: MaskRegion::Lasso {
            points: vec![[0., 0.], [1e-200, 0.], [0., 1e-200]],
        },
        view_projection: IDENTITY,
        viewport: [0., 0., 1e-200, 1e-200],
        erase: false,
    };
    let result = source.region_masks(&settings);
    assert!(
        result.is_ok(),
        "finite noncollinear stored points have no minimum extent: {result:?}"
    );
    assert_eq!(result.unwrap().0, vec![1.; 3]);
    let mut changed = settings.clone();
    changed.region = MaskRegion::Lasso {
        points: vec![[0., 0.], [1e-200, 1e-200], [2e-200, 2e-200]],
    };
    assert!(source.region_masks(&changed).is_err());
    changed.region = MaskRegion::Lasso {
        points: vec![[0., 0.], [1e-200, 1e-200], [1e6, 1e6]],
    };
    assert!(source.region_masks(&changed).is_err());
    changed.region = MaskRegion::Lasso {
        points: vec![[0., 0.], [1e-200, 0.], [0., 1e6]],
    };
    assert!(source.region_masks(&changed).is_ok());
}

fn mesh(points: &[[f32; 3]]) -> Mesh {
    let mut obj = String::new();
    for p in points {
        obj.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
    }
    obj.push_str("f 1 2 3\n");
    Mesh::from_obj(&obj).unwrap()
}

fn source() -> Mesh {
    mesh(&[
        [-0.5, 0.5, 0.],
        [0.5, 0.5, 0.],
        [0.5, -0.5, 0.],
        [-0.5, -0.5, 0.],
        [0., 0., -0.5],
        [0., 0., 0.5],
        [0.75, 0., 0.],
        [0., 0., 1.25],
    ])
}

fn settings(region: MaskRegion) -> MaskRegionSettings {
    MaskRegionSettings {
        region,
        view_projection: IDENTITY,
        viewport: [0., 0., 100., 100.],
        erase: false,
    }
}

fn rectangle(start: [f64; 2], end: [f64; 2]) -> MaskRegionSettings {
    settings(MaskRegion::Rectangle { start, end })
}

fn exact_masks(actual: &[f32], expected: &[f32]) {
    assert_eq!(
        actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

fn unchanged_channels(a: &Mesh, b: &Mesh) {
    for (a, b) in a.positions.iter().zip(&b.positions) {
        assert_eq!(
            a.to_array().map(f32::to_bits),
            b.to_array().map(f32::to_bits)
        );
    }
    for (a, b) in a.normals.iter().zip(&b.normals) {
        assert_eq!(
            a.to_array().map(f32::to_bits),
            b.to_array().map(f32::to_bits)
        );
    }
    for (a, b) in a.colors.iter().zip(&b.colors) {
        assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
    }
    assert_eq!(a.triangles, b.triangles);
}

#[test]
fn rectangle_is_reversible_inclusive_through_depth_and_column_major() {
    let source = source();
    let before = serde_json::to_string(&source).unwrap();
    let (forward, report) = source
        .region_masks(&rectangle([25., 25.], [75., 75.]))
        .unwrap();
    exact_masks(&forward, &[1., 1., 1., 1., 1., 1., 0., 0.]);
    assert_eq!(
        (
            report.vertices,
            report.projected_vertices,
            report.selected_vertices,
            report.changed_vertices
        ),
        (8, 7, 6, 6)
    );
    assert_eq!((report.polygon_points, report.point_edge_work), (4, 0));
    assert_eq!(report.max_abs_change, 1.0);
    let (reversed, _) = source
        .region_masks(&rectangle([75., 75.], [25., 25.]))
        .unwrap();
    exact_masks(&forward, &reversed);
    let (empty, report) = source
        .region_masks(&rectangle([50., 0.], [50., 100.]))
        .unwrap();
    exact_masks(&empty, &source.masks);
    assert_eq!(
        (
            report.projected_vertices,
            report.selected_vertices,
            report.changed_vertices
        ),
        (7, 0, 0)
    );
    let mut translated = rectangle([159., 94.], [161., 96.]);
    translated.viewport = [10., 20., 200., 100.];
    translated.view_projection[12] = 0.5;
    translated.view_projection[13] = -0.5;
    let (masks, _) = source.region_masks(&translated).unwrap();
    exact_masks(&masks, &[0., 0., 0., 0., 1., 1., 0., 0.]);
    assert_eq!(serde_json::to_string(&source).unwrap(), before);
    println!(
        "MASK_REGION_METRIC {}",
        serde_json::json!({"case":"identity-reversed-through-depth","selected_vertices":6,"projected_vertices":7,"maximum_mask_error":0.0,"gate":1e-7})
    );
}

#[test]
fn perspective_homogeneous_six_plane_and_positive_w_rules_are_exact() {
    let source = mesh(&[
        [0., 0., -1.],
        [0., 0., -3.],
        [0., 0., -2.],
        [0., 0., -0.5],
        [0., 0., -3.5],
        [0., 0., 1.],
        [0., 0., 0.],
        [-2., 0., -2.],
        [2., 0., -2.],
        [0., 2., -2.],
        [0., -2., -2.],
        [2.01, 0., -2.],
        [0., 2.01, -2.],
    ]);
    let mut settings = rectangle([0., 0.], [100., 100.]);
    // Independent near=1/far=3, 90-degree/aspect1 OpenGL matrix.
    settings.view_projection = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., -2., -1., 0., 0., -3., 0.,
    ];
    let (masks, report) = source.region_masks(&settings).unwrap();
    exact_masks(
        &masks,
        &[1., 1., 1., 0., 0., 0., 0., 1., 1., 1., 1., 0., 0.],
    );
    assert_eq!(
        (report.projected_vertices, report.selected_vertices),
        (7, 7)
    );
    settings.view_projection = [0.; 16];
    let (no_w, report) = source.region_masks(&settings).unwrap();
    exact_masks(&no_w, &source.masks);
    assert_eq!(report.projected_vertices, 0);
}

#[test]
fn lasso_even_odd_keeps_double_traces_crossings_and_repeated_points() {
    let source = source();
    let outline = vec![[25., 25.], [75., 25.], [75., 75.], [25., 75.]];
    let (square, report) = source
        .region_masks(&settings(MaskRegion::Lasso {
            points: outline.clone(),
        }))
        .unwrap();
    exact_masks(&square, &[1., 1., 1., 1., 1., 1., 0., 0.]);
    assert_eq!((report.polygon_points, report.point_edge_work), (4, 32));
    let mut twice = outline.clone();
    twice.extend_from_slice(&outline);
    let (masks, report) = source
        .region_masks(&settings(MaskRegion::Lasso { points: twice }))
        .unwrap();
    exact_masks(&masks, &[1., 1., 1., 1., 0., 0., 0., 0.]);
    assert_eq!((report.polygon_points, report.point_edge_work), (8, 64));
    let (bowtie, _) = source
        .region_masks(&settings(MaskRegion::Lasso {
            points: vec![[25., 25.], [75., 75.], [25., 75.], [75., 25.]],
        }))
        .unwrap();
    exact_masks(&bowtie, &[1., 1., 1., 1., 1., 1., 0., 0.]);
    let mut repeated = outline;
    repeated.insert(0, repeated[0]);
    repeated.push(repeated[0]);
    let (repeat, _) = source
        .region_masks(&settings(MaskRegion::Lasso { points: repeated }))
        .unwrap();
    exact_masks(&repeat, &square);
}

#[test]
fn true_noops_signed_zero_and_coarse_baselines_preserve_genuine_redo() {
    let mut source = source();
    source.masks.fill(-0.0);
    let mut erase = rectangle([0., 0.], [100., 100.]);
    erase.erase = true;
    let (masks, report) = source.region_masks(&erase).unwrap();
    exact_masks(&masks, &source.masks);
    assert_eq!(report.changed_vertices, 0);
    let mut document = Document::new(Mesh::sphere(1));
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_subtool_name(1, "Genuine redo").unwrap();
    let named = document.to_json().unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    let report = document
        .mask_region(&rectangle([101., 101.], [200., 200.]))
        .unwrap();
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), named);
}

#[test]
fn region_edit_keeps_rich_geometry_state_inactive_arc_and_one_whole_undo() {
    let mut document = Document::new(Mesh::sphere(1));
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.store_morph_target().unwrap();
    document.add_layer("Hidden sculpt").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[5] += Vec3::new(0.001, -0.002, 0.003);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.add_subtool(Mesh::sphere(0), "Inactive").unwrap();
    document.select_subtool(1).unwrap();
    let inactive = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == 2)
        .unwrap()
        .mesh;
    let original = document.active_mesh().unwrap().snapshot();
    let before = document.to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&before).unwrap();
    let report = document
        .mask_region(&rectangle([50., 0.], [100., 100.]))
        .unwrap();
    assert!(report.changed_vertices > 0);
    unchanged_channels(&original, document.active_mesh().unwrap());
    assert_eq!(document.current_level(), 0);
    let after = document.to_json().unwrap();
    let next: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(next["layers"], value["layers"]);
    assert_eq!(next["morph_target"], value["morph_target"]);
    let other = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == 2)
        .unwrap()
        .mesh;
    assert!(Arc::ptr_eq(&inactive, &other));
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
fn settings_work_decoder_hidden_and_invalid_pending_fail_atomically() {
    let source = source();
    let valid = rectangle([0., 0.], [100., 100.]);
    let mut invalid = valid.clone();
    invalid.view_projection[0] = f32::NAN;
    assert!(source.region_masks(&invalid).is_err());
    invalid = valid.clone();
    invalid.viewport[2] = 0.;
    assert!(source.region_masks(&invalid).is_err());
    invalid = valid.clone();
    invalid.viewport = [1e6, 0., 1., 1.];
    assert!(source.region_masks(&invalid).is_err());
    invalid = valid.clone();
    invalid.region = MaskRegion::Lasso {
        points: vec![[0., 0.], [1., 1.], [2., 2.]],
    };
    assert!(source.region_masks(&invalid).is_err());
    invalid.region = MaskRegion::Lasso {
        points: vec![[0., 0.]; 513],
    };
    assert!(source.region_masks(&invalid).is_err());
    assert!(
        serde_json::from_value::<MaskRegionSettings>(serde_json::to_value(&invalid).unwrap())
            .is_err()
    );
    let mut large = source.snapshot();
    large.positions.resize(62_501, Vec3::ZERO);
    large.normals.resize(62_501, Vec3::ZERO);
    large.colors.resize(62_501, [0.5; 3]);
    large.masks.resize(62_501, 0.0);
    invalid = valid.clone();
    invalid.region = MaskRegion::Lasso {
        points: (0..512)
            .map(|i| [[0., 0.], [100., 0.], [100., 100.], [0., 100.]][i % 4])
            .collect(),
    };
    assert!(large.region_masks(&invalid).unwrap_err().contains("work"));
    let mut document = Document::new(source);
    document.set_subtool_name(1, "Redo").unwrap();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::INFINITY;
    assert!(document.mask_region(&valid).is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.take_layer_error().is_some());
    assert!(document.redo());
    document.set_subtool_visible(1, false).unwrap();
    let before = document.to_json().unwrap();
    assert!(document.mask_region(&valid).is_err());
    assert_eq!(document.to_json().unwrap(), before);
}
