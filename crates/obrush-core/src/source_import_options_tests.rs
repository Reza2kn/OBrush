//! Twenty preregistered core gates. Raw input files were independently authored
//! before implementation; expected words/tolerances are not producer snapshots.
use super::*;
use crate::*;
use std::sync::Arc;

const SOURCE_OBJ: &[u8] = b"v 1 2 3 0.5 0.25 0.75\nv 3 2 3 0.5 0.25 0.75\nv 1 6 3 0.5 0.25 0.75\nv 1 2 9 0.5 0.25 0.75\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n";

#[test]
fn source_obj_options_keep_local_mesh_and_commit_one_undoable_affine_part() {
    let imported = Mesh::import_obj(SOURCE_OBJ).unwrap();
    assert!(imported.report.imported_colors);
    let original_positions = imported.mesh.positions.clone();
    let prepared = prepare_source_mesh_import(imported.mesh, true, options(true, false)).unwrap();
    assert_eq!(prepared.mesh.positions, original_positions);
    assert_eq!(prepared.report.converted_explicit_rgb_vertices, 4);
    let bound = prepared.report.normalization.as_ref().unwrap();
    assert_eq!(bound.incoming_world_bound, [1., 2., 3., 3., 6., 9.]);
    assert!((bound.diagonal - 56.0_f64.sqrt()).abs() < 1e-12);
    assert_eq!(prepared.metadata.affine.interaction_center, [2., 4., 6.]);
    assert!((prepared.mesh.colors[0][0] - 0.7353569).abs() < 1e-6);
    assert_eq!(
        prepared.metadata.affine.local_to_world[0],
        bound.scale_binary64 as f32
    );
    let mut document = Document::new(Mesh::sphere(0));
    let before = document.to_json().unwrap();
    let added = document
        .add_subtool_with_metadata(prepared.mesh, "four points", false, prepared.metadata)
        .unwrap();
    assert_eq!(document.active_subtool_id(), Some(added));
    let after = document.to_json().unwrap();
    assert_ne!(after, before);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after);
}

#[test]
fn source_mesh_default_retains_explicit_color_and_rejects_invalid_normalization() {
    let imported = Mesh::import_obj(SOURCE_OBJ).unwrap();
    let original = imported.mesh.colors.clone();
    let prepared = prepare_source_mesh_import(imported.mesh, true, options(false, true)).unwrap();
    assert_eq!(prepared.mesh.colors, original);
    assert_eq!(prepared.metadata.affine.local_to_world[0], 1.0);
    assert!(prepared.report.normalization.is_none());
}
const TETRA: &[u8] = include_bytes!("../tests/fixtures/source-import-options/tetra-13-v3-le.sgl");
const FLAT: &[u8] = include_bytes!("../tests/fixtures/source-import-options/flat-quad-v3-le.sgl");
const TRANSFER: &[u8] =
    include_bytes!("../tests/fixtures/source-import-options/transfer-scalar-v3-le.sgl");
const EMPTY: &[u8] = include_bytes!("../tests/fixtures/source-import-options/empty-v3-le.sgl");
const EXPECTED: [u32; 16] = [
    0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0xc1389d8a, 0xc1762762,
    0xc2389d8a, 0x3f800000,
];
fn stage(bytes: &[u8]) -> StagedSglScene {
    StagedSglScene::from_sgl_bytes(bytes, &SglStageSettings::default()).unwrap()
}
fn options(scale_center: bool, vertex_srgb: bool) -> SourceImportOptions {
    SourceImportOptions {
        scale_center,
        vertex_srgb,
    }
}
fn prepared(bytes: &[u8], opt: SourceImportOptions) -> PreparedSourceImportOptions {
    stage(bytes).prepare_source_import_options(opt).unwrap()
}
fn independent_corners(local: [f64; 6], m: [f32; 16]) -> [f64; 6] {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for vertex in 0..8 {
        let point: [f64; 3] =
            std::array::from_fn(|axis| local[axis + if vertex & (1 << axis) == 0 { 0 } else { 3 }]);
        for row in 0..3 {
            let transformed = ((f64::from(m[row]) * point[0] + f64::from(m[row + 4]) * point[1])
                + f64::from(m[row + 8]) * point[2])
                + f64::from(m[row + 12]);
            if transformed < low[row] {
                low[row] = transformed;
            }
            if transformed > high[row] {
                high[row] = transformed;
            }
        }
    }
    [low[0], low[1], low[2], high[0], high[1], high[2]]
}
#[test]
fn c01_tetra13_staged_float32_matrix() {
    let p = prepared(TETRA, options(true, true));
    let row = &p.staged.objects[0];
    assert_eq!(row.affine.local_to_world.map(f32::to_bits), EXPECTED);
    assert_eq!(
        row.affine.interaction_center.map(f32::to_bits),
        [1.5_f32.to_bits(), 2.0_f32.to_bits(), 6.0_f32.to_bits()]
    );
    let n = p.report.normalization.unwrap();
    assert_eq!(n.incoming_world_bound, [0., 0., 0., 3., 4., 12.]);
    assert_eq!(n.diagonal, 13.);
    assert_eq!(
        row.origin.raw.matrix.map(f32::to_bits),
        stage(TETRA).objects[0].origin.raw.matrix.map(f32::to_bits)
    );
}
#[test]
fn c02_nondiscriminating_identity_translation() {
    let p = prepared(TETRA, options(true, true));
    let fused = (-(100.0_f64 / 13.0) * 2.0) as f32;
    assert_eq!(fused.to_bits(), EXPECTED[13]);
    assert_eq!(
        p.staged.objects[0].affine.local_to_world[13].to_bits(),
        EXPECTED[13]
    );
    assert_eq!(
        p.report.normalization.unwrap().normalization_matrix_words,
        EXPECTED
    );
}
#[test]
fn c03_shear_reflection_matrix_order() {
    let lhs = EXPECTED.map(f32::from_bits);
    let rhs = stage(include_bytes!(
        "../tests/fixtures/source-import-options/shear-group-v3-le.sgl"
    ))
    .objects[0]
        .affine
        .local_to_world;
    let s = f64::from(lhs[0]);
    let [x, y, z] = [lhs[12], lhs[13], lhs[14]].map(f64::from);
    let expected = [
        (1.5 * s) as f32,
        0.,
        0.,
        0.,
        (0.375 * s) as f32,
        (-0.75 * s) as f32,
        0.,
        0.,
        (0.25 * s) as f32,
        (0.5 * s) as f32,
        (2. * s) as f32,
        0.,
        (262144. * s + x) as f32,
        (-131072. * s + y) as f32,
        (4096. * s + z) as f32,
        1.,
    ];
    assert_eq!(
        multiply_f32(lhs, rhs).map(f32::to_bits),
        expected.map(f32::to_bits)
    );
}
#[test]
fn c04_flat_initial_loose_padding() {
    let p = prepared(FLAT, options(true, true));
    let b = p.report.object_reports[0].initial_local_bound;
    assert_eq!(
        b,
        [
            -1.,
            -1.,
            -8.0_f64.sqrt() * 0.2,
            1.,
            1.,
            8.0_f64.sqrt() * 0.2
        ]
    );
    let scale = p.report.normalization.unwrap().scale_binary64;
    assert_eq!(
        (scale as f32).to_bits(),
        ((100.0 / (8.0_f64 + 1.28).sqrt()) as f32).to_bits()
    );
    assert_ne!(
        (100.0 / 8.0_f64.sqrt()) as f32,
        p.staged.objects[0].affine.local_to_world[0]
    );
}
#[test]
fn c05_extent_epsilon_boundary() {
    let below = [Vec3::ZERO, Vec3::new(1., 1., 1e-17)];
    let above = [Vec3::ZERO, Vec3::new(1., 1., 1e-15)];
    let padded = initial_local_bound(&below).unwrap();
    let retained = initial_local_bound(&above).unwrap();
    assert!(padded[2] < 0. && padded[5] > 0.2);
    assert_eq!(retained[2].to_bits(), 0.0_f64.to_bits());
    assert_eq!(retained[5], f64::from(above[1].z));
    assert_eq!(retained[0], 0.);
    assert_eq!(retained[3], 1.);
}
#[test]
fn c06_world_interval_eight_corner_consistency() {
    let incoming = stage(include_bytes!(
        "../tests/fixtures/source-import-options/shear-group-v3-le.sgl"
    ));
    for o in incoming.objects {
        let local = initial_local_bound(&o.mesh.positions).unwrap();
        let interval = world_bound(local, &o.affine.local_to_world).unwrap();
        let corners = independent_corners(local, o.affine.local_to_world);
        for (actual, expected) in interval.into_iter().zip(corners) {
            assert!((actual - expected).abs() <= 1e-12 * expected.abs().max(1.));
        }
    }
}
#[test]
fn c07_signed_zero_center_policy() {
    let input =
        include_bytes!("../tests/fixtures/source-import-options/flat-negative-zero-v3-le.sgl");
    let default = prepared(input, SourceImportOptions::default());
    let normalized = prepared(input, options(true, true));
    let color = prepared(input, options(false, false));
    assert_eq!(
        default.staged.objects[0].affine.interaction_center[2].to_bits(),
        0
    );
    assert_eq!(
        color.staged.objects[0].affine.interaction_center[2].to_bits(),
        0
    );
    assert_eq!(
        normalized.staged.objects[0].affine.interaction_center[2].to_bits(),
        0
    );
    assert_eq!(
        normalized.staged.objects[0].mesh.positions[0].z.to_bits(),
        0x80000000
    );
}
#[test]
fn c08_new_group_receiving_scene_exclusion() {
    let mut receiver = Mesh::sphere(0);
    for point in &mut receiver.positions {
        *point += Vec3::new(500000., -200000., 300000.);
    }
    let mut d = Document::new(receiver);
    let first = d.active_subtool_id().unwrap();
    let hidden = d.add_subtool(Mesh::sphere(0), "Hidden receiver").unwrap();
    d.set_subtool_visible(hidden, false).unwrap();
    d.select_subtool(first).unwrap();
    d.set_solo(true).unwrap();
    let before = d.to_json().unwrap();
    let p = prepared(TETRA, options(true, false));
    let expected = p.staged.objects[0].affine.local_to_world.map(f32::to_bits);
    let report = d.append_staged_sgl_scene(p.staged).unwrap();
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .affine
            .local_to_world
            .map(f32::to_bits),
        expected
    );
    assert_eq!(report.selected_ids, vec![report.active_id.unwrap()]);
    assert!(d.solo());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn c09_source_visibility_scope() {
    let incoming = stage(TETRA);
    let o = &incoming.objects[0];
    let local = initial_local_bound(&o.mesh.positions).unwrap();
    let visible = BoundInput {
        local,
        matrix: o.affine.local_to_world,
        visible: true,
    };
    let mut hidden = visible;
    hidden.visible = false;
    hidden.matrix[12] = 999999.;
    assert_eq!(
        union_visible(&[visible, hidden]).unwrap(),
        [0., 0., 0., 3., 4., 12.]
    );
    assert!(union_visible(&[hidden]).is_err());
    let (normal, _, _) = normalization_matrix(union_visible(&[visible, hidden]).unwrap()).unwrap();
    // Hidden is a typed helper descriptor, never an invented SGL file field.
    assert_ne!(
        multiply_f32(normal, hidden.matrix).map(f32::to_bits),
        hidden.matrix.map(f32::to_bits)
    );
    assert_eq!(prepared(TETRA, options(true, true)).staged.objects.len(), 1);
}
#[test]
fn c10_rounded_source_scale_radius() {
    let p = prepared(TETRA, options(true, true));
    let o = &p.staged.objects[0];
    let s = f64::from(f32::from_bits(EXPECTED[0]));
    let actual = o.affine.source_scale_squared();
    assert_eq!(actual, s * s);
    assert_ne!(actual, (100.0_f64 / 13.0).powi(2));
    assert_ne!(actual, 17.25_f64.powi(2));
    let local_radius2 = 0.18_f64.powi(2) / actual;
    assert_eq!(local_radius2, 0.18_f64.powi(2) / (s * s));
    assert_eq!(
        p.report.object_reports[0].source_scale_squared_after,
        actual
    );
    assert_eq!(o.origin.raw.scale.to_bits(), 17.25_f32.to_bits());
}
#[test]
fn c11_rgb_true_exact_bits() {
    let p = prepared(TRANSFER, SourceImportOptions::default());
    let raw = p.staged.objects[0].origin.raw.colors.as_ref().unwrap();
    let actual = &p.staged.objects[0].mesh.colors;
    assert_eq!(
        actual
            .iter()
            .map(|r| r.map(f32::to_bits))
            .collect::<Vec<_>>(),
        raw.iter().map(|r| r.map(f32::to_bits)).collect::<Vec<_>>()
    );
    assert_eq!(actual[0][0].to_bits(), 0x80000000);
    assert_eq!(actual[0][1].to_bits(), 1);
    assert_eq!(p.report.converted_explicit_rgb_vertices, 0);
}
#[test]
fn c12_rgb_false_linear_known_words() {
    let p = prepared(TRANSFER, options(false, false));
    assert_eq!(
        p.staged.objects[0].mesh.colors[0].map(f32::to_bits),
        [0x80000000, 13, 0]
    );
    assert_eq!(p.report.converted_explicit_rgb_vertices, 4);
    assert!(p.report.normalization.is_none());
    assert_eq!(
        p.staged.objects[0].origin.raw.colors.as_ref().unwrap()[0].map(f32::to_bits),
        [0x80000000, 1, 0]
    );
}
#[test]
fn c13_rgb_false_threshold_pow() {
    // Independently declared analytic values, not the implementation's outputs.
    for (x, expected) in [
        (0.25, (0.5370987304831942_f64 as f32)),
        (0.5, (0.7353569830524495_f64 as f32)),
        (0.75, (0.8808250210902997_f64 as f32)),
        (1.0, 1.0),
    ] {
        assert_eq!(encode_linear_rgb(x).unwrap().to_bits(), expected.to_bits());
    }
    let p = prepared(TRANSFER, options(false, false));
    let raw = p.staged.objects[0].origin.raw.colors.as_ref().unwrap();
    assert!(f64::from(raw[1][0]) < 0.0031308);
    assert!(f64::from(raw[1][2]) > 0.0031308);
    for (k, &component) in raw[1].iter().enumerate() {
        let x = f64::from(component);
        let expected = if x < 0.0031308 {
            x * 12.92
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        assert_eq!(
            p.staged.objects[0].mesh.colors[1][k].to_bits(),
            (expected as f32).to_bits()
        );
    }
}
#[test]
fn c14_rgb_absence_and_other_channels() {
    let input = include_bytes!("../tests/fixtures/source-import-options/absent-rgb-v3-le.sgl");
    let before = stage(input);
    let p = prepared(input, options(false, false));
    let old = &before.objects[0].mesh;
    let new = &p.staged.objects[0].mesh;
    assert!(new
        .colors
        .iter()
        .all(|r| r.map(f32::to_bits) == [1.0_f32.to_bits(); 3]));
    assert!(p.staged.objects[0].origin.raw.colors.is_none());
    assert_eq!(p.report.converted_explicit_rgb_vertices, 0);
    assert_eq!(new.materials, old.materials);
    assert_eq!(
        new.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        old.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(new.surface, old.surface);
    assert_eq!(new.normals, old.normals);
}
#[test]
fn c15_raw_origin_all_options() {
    for scale in [false, true] {
        for color in [false, true] {
            let incoming = stage(TRANSFER);
            let archive = incoming.objects[0].origin.raw.clone();
            let raw = serde_json::to_string(archive.as_ref()).unwrap();
            let before_mesh = incoming.objects[0].mesh.clone();
            let p = incoming
                .prepare_source_import_options(options(scale, color))
                .unwrap();
            let o = &p.staged.objects[0];
            assert!(Arc::ptr_eq(&archive, &o.origin.raw));
            assert_eq!(serde_json::to_string(o.origin.raw.as_ref()).unwrap(), raw);
            assert_eq!(o.mesh.positions, before_mesh.positions);
            assert_eq!(o.mesh.triangles, before_mesh.triangles);
            assert_eq!(o.mesh.normals, before_mesh.normals);
            assert_eq!(o.mesh.surface, before_mesh.surface);
            assert_eq!(o.mesh.materials, before_mesh.materials);
        }
    }
}
#[test]
fn c16_versions_orders_options() {
    let cases: [(&[u8], SglByteOrder); 6] = [
        (TETRA, SglByteOrder::LittleEndian),
        (
            include_bytes!("../tests/fixtures/source-import-options/tetra-13-v3-be.sgl"),
            SglByteOrder::BigEndian,
        ),
        (
            include_bytes!("../tests/fixtures/source-import-options/tetra-13-v0-le.sgl"),
            SglByteOrder::LittleEndian,
        ),
        (
            include_bytes!("../tests/fixtures/source-import-options/tetra-13-v0-be.sgl"),
            SglByteOrder::BigEndian,
        ),
        (
            include_bytes!("../tests/fixtures/source-import-options/tetra-13-v1-le.sgl"),
            SglByteOrder::LittleEndian,
        ),
        (
            include_bytes!("../tests/fixtures/source-import-options/tetra-13-v2-be.sgl"),
            SglByteOrder::BigEndian,
        ),
    ];
    for (bytes, order) in cases {
        let input = StagedSglScene::from_sgl_bytes(
            bytes,
            &SglStageSettings {
                byte_order: Some(order),
                ..Default::default()
            },
        )
        .unwrap();
        let version = input.origin.version;
        let p = input
            .prepare_source_import_options(options(true, false))
            .unwrap();
        assert_eq!(p.staged.origin.byte_order, order);
        assert_eq!(p.staged.origin.version, version);
        assert_eq!(
            p.staged.objects[0].affine.local_to_world.map(f32::to_bits),
            EXPECTED
        );
    }
}
#[test]
fn c17_zero_no_math_doc_history() {
    let mut d = Document::new(Mesh::sphere(0));
    d.add_subtool(Mesh::sphere(0), "Redo").unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let history = (d.can_undo(), d.can_redo());
    let p = prepared(EMPTY, options(true, false));
    assert!(p.report.normalization.is_none());
    assert!(p.report.object_reports.is_empty());
    assert_eq!(p.report.options_storage_bytes_bound, 0);
    let r = d.append_staged_sgl_scene(p.staged).unwrap();
    assert!(!r.document_changed);
    assert!(r.input_header.camera.is_some());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!((d.can_undo(), d.can_redo()), history);
}
#[test]
fn c18_malformed_atomic_input() {
    let d = Document::new(Mesh::sphere(0));
    let before = d.to_json().unwrap();
    for bad in [
        include_bytes!("../tests/fixtures/source-import-options/invalid-second-affine-v3-le.sgl")
            .as_slice(),
        include_bytes!("../tests/fixtures/source-import-options/invalid-rgb-v3-le.sgl").as_slice(),
    ] {
        assert!(
            StagedSglScene::from_sgl_bytes(bad, &SglStageSettings::default())
                .and_then(|s| s.prepare_source_import_options(options(true, false)))
                .is_err()
        );
        assert_eq!(d.to_json().unwrap(), before);
    }
}
#[test]
fn c19_normalized_output_bound_atomic() {
    let input = include_bytes!(
        "../tests/fixtures/source-import-options/tiny-normalized-overlimit-v3-le.sgl"
    );
    let incoming = stage(input);
    assert!(incoming
        .clone()
        .prepare_source_import_options(SourceImportOptions::default())
        .is_ok());
    let d = Document::new(Mesh::sphere(0));
    let before = d.to_json().unwrap();
    assert!(incoming
        .prepare_source_import_options(options(true, true))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(ensure_storage_bound(MAX_STAGED_SCENE_BYTES - 1, 2).is_err());
    assert!(ensure_storage_bound(usize::MAX, 1).is_err());
}
#[test]
fn c20_one_complete_history_and_export() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    let p = prepared(TETRA, options(true, false));
    let current_matrix = p.staged.objects[0].affine.local_to_world;
    let current_colors = p.staged.objects[0].mesh.colors.clone();
    let r = d.append_staged_sgl_scene(p.staged).unwrap();
    let id = r.active_id.unwrap();
    assert_eq!(r.selected_ids, vec![id]);
    let after = d.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    let settings = SglExportSettings {
        scope: SglExportScope::AllScene,
        byte_order: SglByteOrder::LittleEndian,
        display: SglDisplaySettings {
            show_grid: true,
            show_symmetry_line: true,
            show_contour: false,
        },
        camera: SglCameraSettings {
            projection: 1,
            mode: 0,
            fov_degrees: 45.,
            use_pivot: true,
        },
        plain_render: baseline_source_render_defaults(),
        legacy_constructor_render: baseline_source_render_defaults(),
        plain_appearance_conversion: false,
        declared_display_defaults: false,
        loss_policy: SglExportLossPolicy::AllowAndReport,
    };
    let exported = d.export_sgl(&settings).unwrap();
    let raw = SglScene::from_bytes(&exported.bytes).unwrap();
    assert_eq!(
        raw.meshes[0].matrix.map(f32::to_bits),
        current_matrix.map(f32::to_bits)
    );
    assert_eq!(
        raw.meshes[0]
            .colors
            .as_ref()
            .unwrap()
            .iter()
            .map(|r| r.map(f32::to_bits))
            .collect::<Vec<_>>(),
        current_colors
            .iter()
            .map(|r| r.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(d.to_json().unwrap(), after);
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .matrix
            .map(f32::to_bits),
        stage(TETRA).objects[0].origin.raw.matrix.map(f32::to_bits)
    );
}

// Source-only insertion draft for source_import_options_tests.rs.
// No formatter/compiler/oracle has run this supplementary input.
#[test]
fn c21_additive_yoffset3_rounding_witness() {
    let input =
        include_bytes!("../tests/fixtures/source-import-options/tetra-13-yoffset3-v3-le.sgl");
    let original = stage(input);
    let raw = original.objects[0].origin.raw.clone();
    assert_eq!(raw.matrix[13].to_bits(), 0x40400000);
    let raw_serialized = serde_json::to_vec(raw.as_ref()).unwrap();
    let p = original
        .prepare_source_import_options(options(true, true))
        .unwrap();
    let norm = p.report.normalization.as_ref().unwrap();
    assert_eq!(norm.incoming_world_bound, [0., 3., 0., 3., 7., 12.]);
    let expected_normalization = [
        0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0xc1389d8a, 0xc219d89d,
        0xc2389d8a, 0x3f800000,
    ];
    let expected_final = [
        0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0, 0, 0, 0x40f62762, 0, 0xc1389d8a, 0xc1762761,
        0xc2389d8a, 0x3f800000,
    ];
    assert_eq!(norm.normalization_matrix_words, expected_normalization);
    let object = &p.staged.objects[0];
    assert_eq!(
        object.affine.local_to_world.map(f32::to_bits),
        expected_final
    );
    let idealized_fused_y = (-200.0_f64 / 13.0) as f32;
    assert_eq!(idealized_fused_y.to_bits(), 0xc1762762);
    assert_ne!(
        object.affine.local_to_world[13].to_bits(),
        idealized_fused_y.to_bits()
    );
    assert!(Arc::ptr_eq(&raw, &object.origin.raw));
    assert_eq!(
        serde_json::to_vec(object.origin.raw.as_ref()).unwrap(),
        raw_serialized
    );
    assert_eq!(object.origin.raw.matrix[13].to_bits(), 0x40400000);
    assert_eq!(object.affine.interaction_center, [1.5, 2., 6.]);
}
