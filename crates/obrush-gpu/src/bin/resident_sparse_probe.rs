#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::resident_point_program::{PointInflateParameters, ResidentDocumentInput};
    use obrush_core::{Document, Mesh};
    use obrush_gpu::ResidentViewport;
    use std::time::Instant;

    pollster::block_on(async {
        let mut args = std::env::args().skip(1);
        let output = args.next().expect("output JSON path");
        let sparse_dump = args.next().expect("sparse result path");
        let accepted_dump = args.next().expect("accepted project path");
        let mut document = Document::new(Mesh::sphere(2));
        let before = document.to_json().unwrap();
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        let parameters = PointInflateParameters {
            center: [0., 0., 1.],
            radius: 0.5,
            intensity: 0.1,
        };
        let mut gpu = ResidentViewport::new(captured.points()).await.unwrap();
        gpu.checkpoint().unwrap();
        assert!(gpu.sparse_positions().await.unwrap().is_empty());
        gpu.inflate(parameters, 1).unwrap();
        assert_eq!(gpu.failure_vertex().await.unwrap(), None);
        let sparse_started = Instant::now();
        let sparse = gpu.sparse_positions().await.unwrap();
        let sparse_ms = sparse_started.elapsed().as_secs_f64() * 1000.;
        assert!(!sparse.is_empty());
        assert_eq!(gpu.report()["sparse_one_map_readbacks"], 1);
        assert!(sparse.len() < captured.points().positions().len() * 16);
        assert_eq!(gpu.report()["position_readbacks"], 0);
        std::fs::write(sparse_dump, &sparse).unwrap();
        assert!(
            captured
                .commit_sparse_position_bytes(&mut document, &sparse)
                .unwrap()
        );
        let accepted = document.to_json().unwrap();
        std::fs::write(accepted_dump, &accepted).unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), accepted);
        gpu.cancel().unwrap();
        assert!(gpu.sparse_positions().await.unwrap().is_empty());
        let result = serde_json::json!({
            "status":"PASS native Metal sparse changed-row readback to shared-core document",
            "parameters":{"center":parameters.center,"radius":parameters.radius,"intensity":parameters.intensity,"passes":1},
            "sparse_bytes":sparse.len(),
            "changed_vertices":sparse.len()/20,
            "sparse_readback_ms":sparse_ms,
            "full_position_bytes_avoided":captured.points().positions().len()*16,
            "undo_exact":true,"redo_exact":true,
            "no_op_and_cancel_return_empty":true,
            "gpu_report":gpu.report(),
        });
        std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
