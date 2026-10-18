#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::Document;
    use obrush_core::resident_point_program::{PointBrushParameters, ResidentDocumentInput};
    use obrush_gpu::ResidentViewport;

    pollster::block_on(async {
        let output = std::env::args().nth(1).expect("output JSON path");
        let mut document = Document::from_json(include_str!(
            "../../tests/fixtures/resident-sphere-162.obrush"
        ))
        .unwrap();
        let before = document.to_json().unwrap();
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        let parameters = PointBrushParameters {
            center: [0., 0., 1.],
            direction: [0., 0., 1.],
            radius: 0.5,
            intensity: 0.25,
        };
        let expected = captured
            .points()
            .reference_brush_positions(parameters, 1)
            .unwrap();
        let mut gpu = ResidentViewport::new(captured.points()).await.unwrap();
        gpu.checkpoint().unwrap();
        gpu.brush(parameters, 1).unwrap();
        assert_eq!(gpu.failure_vertex().await.unwrap(), None);
        let actual_bytes = gpu.positions().await.unwrap();
        let actual: Vec<[f32; 4]> = actual_bytes
            .chunks_exact(16)
            .map(|row| {
                std::array::from_fn(|axis| {
                    f32::from_le_bytes(row[axis * 4..axis * 4 + 4].try_into().unwrap())
                })
            })
            .collect();
        assert_eq!(actual.len(), expected.len());
        let mut changed = 0;
        let mut maximum_component_difference = 0.0_f32;
        for ((before, reference), gpu) in captured
            .points()
            .positions()
            .iter()
            .zip(&expected)
            .zip(&actual)
        {
            changed += usize::from(
                before[..3]
                    .iter()
                    .zip(&gpu[..3])
                    .any(|(old, next)| old.to_bits() != next.to_bits()),
            );
            for axis in 0..4 {
                maximum_component_difference =
                    maximum_component_difference.max((reference[axis] - gpu[axis]).abs());
            }
        }
        assert!(changed > 0);
        assert!(maximum_component_difference <= 2.0e-6);
        let sparse = gpu.sparse_positions().await.unwrap();
        assert_eq!(sparse.len() / 20, changed);
        assert!(
            captured
                .commit_sparse_position_bytes(&mut document, &sparse)
                .unwrap()
        );
        let accepted = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), accepted);
        gpu.cancel().unwrap();
        assert!(gpu.sparse_positions().await.unwrap().is_empty());
        let result = serde_json::json!({
            "status":"PASS native resident explicit-direction Brush candidate",
            "parameters":{"center":parameters.center,"direction":parameters.direction,"radius":parameters.radius,"intensity":parameters.intensity,"passes":1},
            "vertices":actual.len(),"changed_vertices":changed,
            "maximum_component_difference":maximum_component_difference,
            "position_words_le":actual_bytes.chunks_exact(4).map(|word|u32::from_le_bytes(word.try_into().unwrap())).collect::<Vec<_>>(),
            "sparse_bytes":sparse.len(),"undo_exact":true,"redo_exact":true,
            "cancel_exact":true,"gpu_report":gpu.report(),
            "limits":"Explicit picked direction and accumulate-on/no-alpha only; no UI pointer, Source picked-normal/first-touch policy or sustained performance claim"
        });
        std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
