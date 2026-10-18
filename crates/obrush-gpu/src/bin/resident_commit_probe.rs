#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::resident_point_program::{PointInflateParameters, ResidentDocumentInput};
    use obrush_core::{Document, Mesh};
    use obrush_gpu::ResidentViewport;

    pollster::block_on(async {
        let mut args = std::env::args().skip(1);
        let output = args.next().expect("output JSON path");
        let project_fixture = args.next();
        let position_dump = args.next();
        let accepted_project_dump = args.next();
        let mut document = Document::new(Mesh::sphere(2));
        let before = document.to_json().unwrap();
        if let Some(path) = project_fixture {
            std::fs::write(path, &before).unwrap();
        }
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        let parameters = PointInflateParameters {
            center: [0., 0., 1.],
            radius: 0.5,
            intensity: 0.1,
        };
        let reference = captured
            .points()
            .reference_positions(parameters, 1)
            .unwrap();
        let mut gpu = ResidentViewport::new(captured.points()).await.unwrap();
        gpu.inflate(parameters, 1).unwrap();
        assert_eq!(gpu.failure_vertex().await.unwrap(), None);
        let bytes = gpu.positions().await.unwrap();
        if let Some(path) = position_dump {
            std::fs::write(path, &bytes).unwrap();
        }
        let max_reference_component_error = bytes
            .chunks_exact(16)
            .zip(reference.iter())
            .flat_map(|(row, expected)| {
                (0..3).map(move |component| {
                    let offset = component * 4;
                    let actual = f32::from_le_bytes(row[offset..offset + 4].try_into().unwrap());
                    (actual - expected[component]).abs()
                })
            })
            .fold(0.0_f32, f32::max);
        let changed = captured
            .points()
            .positions()
            .iter()
            .zip(bytes.chunks_exact(16))
            .filter(|(original, row)| {
                (0..3).any(|component| {
                    let offset = component * 4;
                    original[component].to_bits()
                        != f32::from_le_bytes(row[offset..offset + 4].try_into().unwrap()).to_bits()
                })
            })
            .count();
        assert!(changed > 0, "GPU output must be an actual useful edit");
        let committed = captured
            .commit_position_bytes(&mut document, &bytes)
            .unwrap();
        assert!(committed);
        let after = document.to_json().unwrap();
        if let Some(path) = accepted_project_dump {
            std::fs::write(path, &after).unwrap();
        }
        assert_ne!(before, after);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
        let mut corrupt = bytes.clone();
        corrupt[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(
            captured
                .commit_position_bytes(&mut document, &corrupt)
                .is_err()
        );
        assert_eq!(document.to_json().unwrap(), after);
        let result = serde_json::json!({
            "status":"PASS native Metal resident readback to shared-core document/undo",
            "fixture":"independently constructed Mesh::sphere(2)",
            "vertices":captured.points().positions().len(),
            "triangles":captured.points().triangles().len(),
            "parameters":{"center":parameters.center,"radius":parameters.radius,"intensity":parameters.intensity,"passes":1},
            "changed_vertices":changed,
            "max_gpu_scalar_reference_component_error":max_reference_component_error,
            "undo_restored_exact_json":true,
            "redo_restored_exact_json":true,
            "corrupt_rejected_without_mutation":true,
            "gpu_report":gpu.report(),
        });
        std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
