use obrush_core::{Document, MaskConvention, MaskFilterOperation, Mesh};

fn imported_cube() -> Mesh {
    Mesh::import_obj(include_bytes!("fixtures/independent-filter-cube.obj"))
        .unwrap()
        .mesh
}

fn imported_quad_grid() -> Mesh {
    Mesh::import_obj(include_bytes!("fixtures/independent-filter-quad-grid.obj"))
        .unwrap()
        .mesh
}

#[test]
fn sharpen_reference_thresholds_repeat_and_mask_free_history() {
    // SculptGL Masking.sharpen uses JS Number arithmetic and stores each
    // result into Float32Array material.z. Exercise both OBrush conventions
    // across the threshold and clamping boundaries, including a second action.
    let source = [0.0_f32, 0.1, 0.49, 0.5, 0.51, 0.9, 1.0, 0.75];
    for convention in [
        MaskConvention::UnprotectedF32V1,
        MaskConvention::ProtectedF32V1,
    ] {
        let mut mesh = imported_cube();
        mesh.mask_convention = convention;
        mesh.masks = source
            .map(|u| match convention {
                MaskConvention::UnprotectedF32V1 => u,
                MaskConvention::ProtectedF32V1 => 1.0 - u,
            })
            .to_vec();
        let mut document = Document::new(mesh);
        let before = document.to_json().unwrap();
        let mut expected = source.map(|u| match convention {
            MaskConvention::UnprotectedF32V1 => u,
            MaskConvention::ProtectedF32V1 => 1.0 - (1.0 - u),
        });
        for action in 0..2 {
            expected = expected.map(|u| {
                if u == 1.0 {
                    u
                } else if u > 0.5 {
                    (f64::from(u) + 0.1).min(1.0) as f32
                } else {
                    (f64::from(u) - 1.0).max(0.0) as f32
                }
            });
            let report = document
                .filter_mask_sculptgl(MaskFilterOperation::Sharpen)
                .unwrap();
            assert!(report.changed_vertices > 0, "action {action}");
            let actual =
                document
                    .active_mesh()
                    .unwrap()
                    .masks
                    .iter()
                    .map(|&raw| match convention {
                        MaskConvention::UnprotectedF32V1 => raw,
                        MaskConvention::ProtectedF32V1 => 1.0 - raw,
                    });
            for (index, (actual, expected)) in actual.zip(expected).enumerate() {
                assert!(
                    (actual - expected).abs() <= f32::EPSILON,
                    "action {action}, vertex {index}, U={actual}, expected={expected}"
                );
            }
        }
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }

    let mut mesh = imported_cube();
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.0);
    let mut document = Document::new(mesh);
    let before = document.to_json().unwrap();
    assert!(!document.can_undo());
    assert_eq!(
        document
            .filter_mask_sculptgl(MaskFilterOperation::Sharpen)
            .unwrap()
            .changed_vertices,
        0
    );
    assert_eq!(document.to_json().unwrap(), before);
    assert!(!document.can_undo());
}

#[test]
fn paired_open_quad_boundary_matches_pinned_mask_blur_and_sharpen() {
    // Pinned hosted SculptGL bundle 79b24bf6. The reference reorders OBJ
    // vertices; these expected U values are mapped back by XY coordinates.
    let mesh = imported_quad_grid();
    assert_eq!(mesh.surface.as_ref().unwrap().faces.len(), 4);
    let before = serde_json::to_vec(&mesh).unwrap();
    for (operation, expected, tolerance) in [
        (
            MaskFilterOperation::Blur,
            [
                0.250980406999588,
                0.6254901885986328,
                1.0,
                0.6254901885986328,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            ],
            4.0e-8,
        ),
        (
            MaskFilterOperation::Sharpen,
            [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            0.0,
        ),
    ] {
        let (masks, report) = mesh.sculptgl_filtered_masks(operation).unwrap();
        assert!(report.changed_vertices > 0);
        for (index, (&mask, &reference)) in masks.iter().zip(expected.iter()).enumerate() {
            let actual_u = 1.0_f64 - f64::from(mask);
            assert!(
                (actual_u - reference).abs() <= tolerance,
                "{operation:?} vertex {index}: U={actual_u}, reference={reference}"
            );
        }
    }
    assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
}

#[test]
fn paired_cube_matches_pinned_mask_blur_and_sharpen() {
    // Pinned hosted SculptGL bundle 79b24bf6; actual visible controls on the
    // independently authored OBJ. Compare by original positions, not the
    // reference's reordered import IDs. U=1-P for OBrush's protected masks.
    let mesh = imported_cube();
    let before = serde_json::to_vec(&mesh).unwrap();
    let expected_blur = [
        0.5836601257324219,
        0.5352941155433655,
        0.41372549533843994,
        0.5980392098426819,
        0.6627451181411743,
        0.4156862795352936,
        0.5836601257324219,
        0.4784314036369324,
    ];
    let expected_sharpen = [
        0.0,
        0.0,
        0.0,
        0.6019608378410339,
        0.6098039746284485,
        0.8490196466445923,
        1.0,
        1.0,
    ];
    for (operation, expected, tolerance) in [
        (MaskFilterOperation::Blur, expected_blur, 4.0e-8),
        (MaskFilterOperation::Sharpen, expected_sharpen, 0.0),
    ] {
        let (masks, report) = mesh.sculptgl_filtered_masks(operation).unwrap();
        assert!(report.changed_vertices > 0);
        for (index, (&mask, &reference)) in masks.iter().zip(expected.iter()).enumerate() {
            let actual_u = 1.0_f64 - f64::from(mask);
            assert!(
                (actual_u - reference).abs() <= tolerance,
                "{operation:?} vertex {index}: U={actual_u}, reference={reference}"
            );
        }
    }
    assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
}

#[test]
fn source_mask_transaction_preserves_shape_history_and_rejects_other_operations() {
    let mut document = Document::new(imported_cube());
    let before = document.to_json().unwrap();
    let report = document
        .filter_mask_sculptgl(MaskFilterOperation::Blur)
        .unwrap();
    assert!(report.changed_vertices > 0);
    let after = document.to_json().unwrap();
    assert_ne!(before, after);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
    assert!(document
        .filter_mask_sculptgl(MaskFilterOperation::Grow)
        .is_err());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn original_quad_boundary_blur_excludes_center_and_diagonals() {
    let obj = b"v 0 0 0\nv 1 0 0\nv 2 0 0\nv 0 1 0\nv 1 1 0\nv 2 1 0\nv 0 2 0\nv 1 2 0\nv 2 2 0\nf 1 2 5 4\nf 2 3 6 5\nf 4 5 8 7\nf 5 6 9 8\n";
    let mut mesh = Mesh::import_obj(obj).unwrap().mesh;
    assert_eq!(mesh.surface.as_ref().unwrap().faces.len(), 4);
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks = vec![0.25, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    let (masks, _) = mesh
        .sculptgl_filtered_masks(MaskFilterOperation::Blur)
        .unwrap();
    assert_eq!(masks, [0.25, 0.625, 1.0, 0.625, 1.0, 1.0, 1.0, 1.0, 1.0]);
}
