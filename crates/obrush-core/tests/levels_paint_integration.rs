//! Cross-cohort history gates for accepted pending material Paint and level
//! deletion. Independent fixtures; reference equivalence remains a separate gate.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, StaticSubdivisionSettings};

fn levels(mode: usize) -> Document {
    let mut document = Document::new(Mesh::sphere(0));
    for _ in 0..2 {
        match mode {
            0 => document.add_subdivision_level().unwrap(),
            _ => document
                .add_static_subdivision_level(&StaticSubdivisionSettings {
                    linear_edges: mode == 2,
                })
                .unwrap(),
        }
    }
    document
}

fn paint() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Paint,
        paint_rgb: false,
        paint_roughness: true,
        paint_metallic: true,
        roughness: 0.8,
        metallic: 0.6,
        ..Default::default()
    }
}

#[test]
fn useful_delete_commits_pending_first_material_paint_as_a_separate_exact_undo_step() {
    for mode in 0..3 {
        for lower in [false, true] {
            let mut document = levels(mode);
            document.select_level(1).unwrap();
            let original = document.to_json().unwrap();
            let mut expected = Document::from_json(&original).unwrap();
            expected.begin_material_paint_stroke();
            expected
                .require_active_mesh_mut()
                .unwrap()
                .fill_paint_channels(&paint())
                .unwrap();
            expected.end_stroke();
            assert!(expected.take_layer_error().is_none());
            let painted = expected.to_json().unwrap();

            document.begin_material_paint_stroke();
            assert!(
                document
                    .require_active_mesh_mut()
                    .unwrap()
                    .fill_paint_channels(&paint())
                    .unwrap()
                    .materialized
            );
            let report = if lower {
                document.delete_lower_subdivision_levels().unwrap()
            } else {
                document.delete_higher_subdivision_levels().unwrap()
            };
            assert_eq!(report.retained_levels, 2);
            assert!(document.take_layer_error().is_none());
            let deleted = document.to_json().unwrap();
            assert_eq!(
                Document::from_json(&deleted).unwrap().to_json().unwrap(),
                deleted
            );
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), painted);
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), original);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), painted);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), deleted);
        }
    }
}

#[test]
fn no_op_delete_does_not_split_a_pending_material_paint_stroke() {
    for mode in 0..3 {
        let mut document = levels(mode);
        let before = document.to_json().unwrap();
        document.begin_material_paint_stroke();
        document
            .require_active_mesh_mut()
            .unwrap()
            .fill_paint_channels(&paint())
            .unwrap();
        let no_op = document.delete_higher_subdivision_levels().unwrap();
        assert_eq!(no_op.previous_levels, no_op.retained_levels);
        // The second edit is still part of the same pending Paint group.
        document
            .require_active_mesh_mut()
            .unwrap()
            .materials
            .as_mut()
            .unwrap()[0][0] = 0.25;
        document.end_stroke();
        assert!(document.take_layer_error().is_none());
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }
}
