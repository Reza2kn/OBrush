//! Independent, preregistered arithmetic and public-history witness for the
//! pinned SculptGL Inflate method. Reference-browser words are recorded
//! separately; this test alone does not establish paired parity.
use obrush_core::{Document, MaskConvention, Mesh, SourceGeometryOptions, SourceGeometryPass};

fn diamond() -> Mesh {
    let mut mesh = Mesh::from_obj(include_str!("fixtures/independent-inflate-diamond.obj"))
        .expect("independent diamond OBJ");
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks.copy_from_slice(&[1.0, 0.5, 0.0, 1.0]);
    mesh
}

fn dab(negative: bool, intensity: f64) -> (Document, String, String) {
    let mut document = Document::new(diamond());
    let before = document.to_json().unwrap();
    let options = SourceGeometryOptions {
        intensity,
        negative,
        prevent_folds: false,
        ..SourceGeometryOptions::inflate()
    };
    let mut stroke = document.begin_source_geometry_stroke(options).unwrap();
    let report = document
        .apply_source_geometry_dab(
            &mut stroke,
            SourceGeometryPass {
                center: [0.0, 0.0, 0.0],
                eye_direction: [0.0, 0.0, -1.0],
                local_radius_squared: 1.0,
                original_face_index: 0,
            },
            1.0,
            true,
        )
        .unwrap();
    assert_eq!(
        report.changed_vertices,
        if intensity == 0.0 { 0 } else { 3 }
    );
    document.end_source_geometry_stroke(&stroke).unwrap();
    let after = document.to_json().unwrap();
    (document, before, after)
}

#[test]
fn preregistered_positive_negative_and_zero_inflate_have_exact_public_history() {
    // Measured by the pinned direct Inflate method after the prospective
    // tolerance was frozen in SCULPTGL_INFLATE_PAIR_PREREG.md. The complete
    // source receipt is docs/evidence/sculptgl-inflate-kernel-c269.json.
    for (negative, sign, reference_z_words) in [
        (false, 1.0, [0x3ca8f5c3, 0x3c28f5c3, 0, 0x3ca8f5c3]),
        (true, -1.0, [0xbca8f5c3, 0xbc28f5c3, 0, 0xbca8f5c3]),
    ] {
        let (mut document, before, after) = dab(negative, 0.3);
        let mesh = document.require_active_mesh().unwrap();
        let original_mesh = diamond();
        assert_eq!(mesh.triangles, original_mesh.triangles);
        assert_eq!(mesh.masks, original_mesh.masks);
        let expected = [0.020625, 0.0103125, 0.0, 0.020625];
        for (id, position) in mesh.positions.iter().enumerate() {
            let xyz = position.to_array();
            let original = original_mesh.positions[id].to_array();
            assert_eq!(xyz[0].to_bits(), original[0].to_bits());
            assert_eq!(xyz[1].to_bits(), original[1].to_bits());
            assert!(
                ((xyz[2] as f64) - sign * expected[id]).abs() <= 3.0e-8,
                "vertex {id} Z differs from independent Inflate arithmetic"
            );
            let paired_difference =
                (xyz[2] as f64 - f32::from_bits(reference_z_words[id]) as f64).abs();
            assert!(
                paired_difference <= 3.0e-8,
                "vertex {id} exceeds preregistered pinned-reference tolerance: {paired_difference}"
            );
            if id == 2 {
                assert_eq!(xyz[2].to_bits(), original[2].to_bits());
            }
        }
        assert_ne!(after, before);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }

    let (mut zero, before, after) = dab(false, 0.0);
    assert_eq!(after, before);
    assert!(!zero.undo());
}
