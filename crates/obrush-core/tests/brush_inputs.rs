//! Authored raster and pressure fixtures for OBrush's shared CPU engine. These
//! establish the stated local footprint/input contract, not ZBrush equivalence.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, RasterAlpha, Vec3};

fn sheet() -> Mesh {
    let mut obj = String::new();
    const SIDE: usize = 61;
    for row in 0..SIDE {
        for column in 0..SIDE {
            let x = (column as i32 - 30) as f32 * 0.05;
            let y = (row as i32 - 30) as f32 * 0.05;
            obj.push_str(&format!("v {x} {y} 0\n"));
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

fn vertex(mesh: &Mesh, x: f32, y: f32) -> usize {
    mesh.positions
        .iter()
        .position(|point| (point.x - x).abs() < 1e-6 && (point.y - y).abs() < 1e-6)
        .unwrap()
}

fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.6,
        strength: 0.5,
        symmetry_x: false,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}

fn equal_surface(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.masks, b.masks);
    assert_eq!(a.colors, b.colors);
}

#[test]
fn authored_raster_bilinear_weights_and_rgba_opacity_are_known() {
    let alpha = RasterAlpha::new(2, 2, vec![0, 255, 64, 128]).unwrap();
    assert_eq!(alpha.sample(0.0, 0.0), 0.0);
    assert_eq!(alpha.sample(1.0, 0.0), 1.0);
    assert!((alpha.sample(0.5, 0.5) - 111.75 / 255.0).abs() < 1e-7);
    assert!((alpha.sample(0.25, 0.0) - 0.25).abs() < 1e-7);
    for uv in [(-0.01, 0.5), (0.5, 1.01), (f32::NAN, 0.0)] {
        assert_eq!(alpha.sample(uv.0, uv.1), 0.0);
    }
    let rgba = RasterAlpha::from_rgba(
        2,
        2,
        &[
            255, 255, 255, 255, 255, 255, 255, 128, 0, 0, 0, 255, 255, 0, 0, 0,
        ],
    )
    .unwrap();
    assert_eq!(rgba.pixels(), &[255, 128, 0, 0]);
}

#[test]
fn alpha_import_rejects_invalid_bounds_lengths_and_serialized_data() {
    assert!(RasterAlpha::new(0, 1, vec![]).is_err());
    assert!(RasterAlpha::new(1025, 1, vec![]).is_err());
    assert!(RasterAlpha::new(2, 2, vec![255; 3]).is_err());
    assert!(RasterAlpha::from_rgba(2, 2, &[255; 15]).is_err());
    let invalid = r#"{"width":2,"height":2,"pixels":[1,2,3]}"#;
    assert!(serde_json::from_str::<RasterAlpha>(invalid).is_err());
    let alpha = RasterAlpha::new(2, 1, vec![0, 255]).unwrap();
    let restored: RasterAlpha =
        serde_json::from_str(&serde_json::to_string(&alpha).unwrap()).unwrap();
    assert_eq!(alpha, restored);
    // Queue clones share the immutable allocation, rather than copying pixels.
    assert_eq!(alpha.pixels().as_ptr(), alpha.clone().pixels().as_ptr());
}

#[test]
fn backward_settings_defaults_and_white_alpha_keep_geometry_exact() {
    let old = r#"{"kind":"draw","radius":0.6,"strength":0.5,"invert":false,"symmetry_x":false,"color":[0.77,0.34,0.19]}"#;
    let old_settings: BrushSettings = serde_json::from_str(old).unwrap();
    assert_eq!(old_settings.pressure, 1.0);
    assert!(old_settings.pressure_strength);
    assert!(!old_settings.pressure_radius);
    assert!(old_settings.alpha.is_none());
    assert!(old_settings.adaptive_detail);
    let mut direct = sheet();
    let mut white = direct.clone();
    let mut configured = old_settings.clone();
    configured.alpha = Some(RasterAlpha::new(3, 2, vec![255; 6]).unwrap());
    let result = direct.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &old_settings);
    assert!(result.changed_vertices > 100);
    assert_eq!(result.displacement_scale, 1.0);
    assert_eq!(
        white.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &configured),
        result
    );
    equal_surface(&direct, &white);
}

#[test]
fn default_pressure_strength_has_zero_quarter_and_full_measurable_response() {
    let original = sheet();
    let pole = vertex(&original, 0.0, 0.0);
    let mut heights = Vec::new();
    for pressure in [0.0, 0.25, 1.0] {
        let mut mesh = original.clone();
        let configured = BrushSettings {
            pressure,
            ..settings(BrushKind::Draw)
        };
        let result = mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &configured);
        if pressure == 0.0 {
            assert_eq!(result.changed_vertices, 0);
            equal_surface(&mesh, &original);
        } else {
            assert_eq!(result.displacement_scale, 1.0);
        }
        heights.push(mesh.positions[pole].z);
    }
    assert!((heights[2] - 0.0225).abs() < 1e-6);
    assert!((heights[1] / heights[2] - 0.25).abs() < 1e-6);
    assert!(heights[0] < heights[1] && heights[1] < heights[2]);
}

#[test]
fn pressure_radius_changes_footprint_and_strength_can_be_disabled() {
    let original = sheet();
    let outer = vertex(&original, 0.4, 0.0);
    let pole = vertex(&original, 0.0, 0.0);
    let mut small = original.clone();
    let configured = BrushSettings {
        pressure: 0.5,
        pressure_radius: true,
        pressure_strength: false,
        ..settings(BrushKind::Mask)
    };
    assert_eq!(configured.effective_radius(), Some(0.3));
    small.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &configured);
    assert_eq!(small.masks[outer], 0.0);
    assert!((small.masks[pole] - 0.09).abs() < 1e-6);
    let mut large = original;
    large.apply_brush(
        Vec3::ZERO,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            pressure: 1.0,
            ..configured
        },
    );
    assert!(large.masks[outer] > 0.0);
    assert_eq!(small.masks[pole], large.masks[pole]);
}

#[test]
fn asymmetric_gradient_draw_and_quarter_turn_have_authored_world_orientation() {
    let original = sheet();
    let left = vertex(&original, -0.3, 0.0);
    let right = vertex(&original, 0.3, 0.0);
    let below = vertex(&original, 0.0, -0.3);
    let above = vertex(&original, 0.0, 0.3);
    let configured = BrushSettings {
        alpha: Some(RasterAlpha::new(2, 2, vec![0, 255, 0, 255]).unwrap()),
        ..settings(BrushKind::Draw)
    };
    let mut horizontal = original.clone();
    horizontal.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &configured);
    assert!(horizontal.positions[left].z > 0.0);
    assert!((horizontal.positions[right].z / horizontal.positions[left].z - 3.0).abs() < 1e-5);
    let mut vertical = original;
    vertical.apply_brush(
        Vec3::ZERO,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            alpha_rotation: std::f32::consts::FRAC_PI_2,
            ..configured
        },
    );
    assert!((vertical.positions[above].z / vertical.positions[below].z - 3.0).abs() < 1e-5);
    assert!((vertical.positions[right].z - vertical.positions[left].z).abs() < 1e-7);
}

#[test]
fn asymmetric_stamp_reflects_both_surface_axes_and_keeps_seam_strength_bounded() {
    let mut mesh = sheet();
    let outer_right = vertex(&mesh, 0.9, 0.0);
    let inner_right = vertex(&mesh, 0.3, 0.0);
    let outer_left = vertex(&mesh, -0.9, 0.0);
    let inner_left = vertex(&mesh, -0.3, 0.0);
    let configured = BrushSettings {
        symmetry_x: true,
        alpha: Some(RasterAlpha::new(2, 2, vec![0, 255, 0, 255]).unwrap()),
        ..settings(BrushKind::Mask)
    };
    mesh.apply_brush(Vec3::new(0.6, 0.0, 0.0), Vec3::Z, Vec3::ZERO, &configured);
    assert!((mesh.masks[outer_right] / mesh.masks[inner_right] - 3.0).abs() < 1e-5);
    assert!((mesh.masks[outer_left] - mesh.masks[outer_right]).abs() < 1e-7);
    assert!((mesh.masks[inner_left] - mesh.masks[inner_right]).abs() < 1e-7);
    let mut seam = sheet();
    let pole = vertex(&seam, 0.0, 0.0);
    seam.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &configured);
    assert!((seam.masks[pole] - 0.045).abs() < 1e-7);

    // An image varying vertically also catches the handedness error from
    // deriving the mirrored bitangent with a fresh cross product.
    let mut vertical = sheet();
    vertical.apply_brush(
        Vec3::new(0.6, 0.0, 0.0),
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            alpha: Some(RasterAlpha::new(2, 2, vec![255, 128, 0, 64]).unwrap()),
            ..configured
        },
    );
    let mask = |x, y| vertical.masks[vertex(&vertical, x, y)];
    assert!((mask(0.9, 0.3) - mask(-0.9, 0.3)).abs() < 1e-7);
    assert!((mask(0.9, -0.3) - mask(-0.9, -0.3)).abs() < 1e-7);
    assert!(mask(0.9, 0.3) > mask(0.9, -0.3));
}

#[test]
fn paint_alpha_has_known_channel_weights_and_protected_pixels_stay_exact() {
    let mut mesh = sheet();
    let left = vertex(&mesh, -0.3, 0.0);
    let right = vertex(&mesh, 0.3, 0.0);
    let pole = vertex(&mesh, 0.0, 0.0);
    mesh.masks[pole] = 1.0;
    let original = mesh.clone();
    mesh.apply_brush(
        Vec3::ZERO,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            pressure: 0.5,
            color: [1.0, 0.0, 0.0],
            alpha: Some(RasterAlpha::new(2, 2, vec![0, 255, 0, 255]).unwrap()),
            ..settings(BrushKind::Paint)
        },
    );
    // Both probes have radial weight (1 - 0.3² / 0.6²)² = 0.5625.
    for (index, alpha_weight) in [(left, 0.25), (right, 0.75)] {
        let blend = 0.5 * 0.5 * 0.5625 * alpha_weight * 0.25;
        for channel in 0..3 {
            let target = if channel == 0 { 1.0 } else { 0.0 };
            let expected = original.colors[index][channel]
                + (target - original.colors[index][channel]) * blend;
            assert!((mesh.colors[index][channel] - expected).abs() < 1e-7);
        }
    }
    assert_eq!(mesh.colors[pole], original.colors[pole]);
    assert_eq!(mesh.positions, original.positions);
    assert_eq!(mesh.normals, original.normals);
    assert_eq!(mesh.triangles, original.triangles);
    assert_eq!(mesh.masks, original.masks);
}

#[test]
fn invalid_pressure_or_empty_alpha_is_an_exact_noop_for_every_brush_and_history() {
    for kind in BrushKind::ALL {
        for pressure in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut document = Document::new(sheet());
            let original = document.active_mesh().unwrap().clone();
            document.begin_stroke();
            document.active_mesh_mut().unwrap().apply_brush(
                Vec3::ZERO,
                Vec3::Z,
                Vec3::X * 0.1,
                &BrushSettings {
                    pressure,
                    pressure_strength: false,
                    ..settings(kind)
                },
            );
            document.end_stroke();
            equal_surface(document.active_mesh().unwrap(), &original);
            assert!(!document.can_undo());
        }
        let mut document = Document::new(sheet());
        let original = document.active_mesh().unwrap().clone();
        document.begin_stroke();
        document.active_mesh_mut().unwrap().apply_brush(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::X * 0.1,
            &BrushSettings {
                alpha: Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap()),
                ..settings(kind)
            },
        );
        document.end_stroke();
        equal_surface(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
    }
}

#[test]
fn pressure_alpha_stroke_preserves_protection_colors_and_exact_single_undo_redo() {
    let mut mesh = sheet();
    let protected = vertex(&mesh, 0.0, 0.0);
    mesh.masks[protected] = 1.0;
    for (index, color) in mesh.colors.iter_mut().enumerate() {
        *color = [(index % 17) as f32 / 16.0, 0.25, 0.75];
    }
    let mut document = Document::new(mesh);
    let original = document.active_mesh().unwrap().clone();
    document.begin_stroke();
    for (x, pressure) in [(-0.1, 0.2), (0.0, 0.7), (0.1, 1.0)] {
        let changed = document.active_mesh_mut().unwrap().apply_brush(
            Vec3::new(x, 0.0, 0.0),
            Vec3::Z,
            Vec3::ZERO,
            &BrushSettings {
                pressure,
                alpha: Some(RasterAlpha::new(2, 2, vec![64, 255, 0, 128]).unwrap()),
                ..settings(BrushKind::Draw)
            },
        );
        assert!(changed > 0);
    }
    document.end_stroke();
    assert_eq!(
        document.active_mesh().unwrap().positions[protected],
        original.positions[protected]
    );
    // The protected point can require conforming local detail around its
    // unchanged position. Existing attributes stay exact and new vertices
    // receive bounded interpolation; topology is included in undo below.
    assert_eq!(
        &document.active_mesh().unwrap().masks[..original.masks.len()],
        original.masks
    );
    assert_eq!(
        &document.active_mesh().unwrap().colors[..original.colors.len()],
        original.colors
    );
    assert!(document
        .active_mesh()
        .unwrap()
        .masks
        .iter()
        .all(|&mask| (0.0..=1.0).contains(&mask)));
    assert!(document
        .active_mesh()
        .unwrap()
        .colors
        .iter()
        .flatten()
        .all(|&channel| (0.0..=1.0).contains(&channel)));
    let drawn = document.active_mesh().unwrap().clone();
    assert!(document.undo());
    equal_surface(document.active_mesh().unwrap(), &original);
    assert!(!document.undo());
    assert!(document.redo());
    equal_surface(document.active_mesh().unwrap(), &drawn);
}

#[test]
fn fixed_topology_brush_samples_still_sculpt_and_preserve_all_existing_attributes() {
    for kind in [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Flatten,
        BrushKind::Crease,
    ] {
        let mut document = Document::new(Mesh::sphere(3));
        let original = document.active_mesh().unwrap().clone();
        document.begin_stroke();
        let configured = BrushSettings {
            adaptive_detail: false,
            pressure: 0.5,
            alpha: Some(RasterAlpha::new(2, 2, vec![255, 128, 64, 255]).unwrap()),
            ..settings(kind)
        };
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            Vec3::Z,
            Vec3::Z,
            Vec3::ZERO,
            &configured,
        );
        assert!(
            result.changed_vertices > 0,
            "{kind} must remain useful at a persistent level"
        );
        assert_eq!(result.refined_vertices, 0);
        assert!(!result.detail_limited);
        assert_eq!(
            document.active_mesh().unwrap().positions.len(),
            original.positions.len()
        );
        assert_eq!(
            document.active_mesh().unwrap().triangles,
            original.triangles
        );
        assert_eq!(document.active_mesh().unwrap().masks, original.masks);
        assert_eq!(document.active_mesh().unwrap().colors, original.colors);
        document.end_stroke();
        let sculpted = document.active_mesh().unwrap().clone();
        assert!(document.undo());
        equal_surface(document.active_mesh().unwrap(), &original);
        assert!(document.redo());
        equal_surface(document.active_mesh().unwrap(), &sculpted);
    }
}
