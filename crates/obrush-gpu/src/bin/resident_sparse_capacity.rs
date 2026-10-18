#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_gpu::{ResidentViewport, capture_sgl, parameters};
    use std::time::Instant;

    pollster::block_on(async {
        let mut args = std::env::args().skip(1);
        let fixture = args.next().expect("SGL fixture path");
        let output = args.next().expect("result JSON path");
        let sparse_dump = args.next().expect("GPU sparse byte path");
        let full_dump = args.next().expect("GPU full byte path");
        let input = capture_sgl(&std::fs::read(fixture).unwrap()).unwrap();
        let mut gpu = ResidentViewport::new(&input).await.unwrap();
        gpu.checkpoint().unwrap();
        gpu.inflate(parameters(), 1).unwrap();
        assert_eq!(gpu.failure_vertex().await.unwrap(), None);
        let sparse_start = Instant::now();
        let sparse = gpu.sparse_positions().await.unwrap();
        let sparse_ms = sparse_start.elapsed().as_secs_f64() * 1000.;
        assert_eq!(gpu.report()["position_readbacks"], 0);
        let full_start = Instant::now();
        let full = gpu.positions().await.unwrap();
        let full_ms = full_start.elapsed().as_secs_f64() * 1000.;
        let mut expected = Vec::new();
        for (index, (original, current)) in input
            .positions()
            .iter()
            .zip(full.chunks_exact(16))
            .enumerate()
        {
            if bytemuck::bytes_of(original) != current {
                expected.extend_from_slice(&(index as u32).to_le_bytes());
                expected.extend_from_slice(current);
            }
        }
        assert_eq!(sparse, expected);
        std::fs::write(sparse_dump, &sparse).unwrap();
        std::fs::write(full_dump, &full).unwrap();
        let result = serde_json::json!({
            "status":"PASS one-pass original million-vertex GPU sparse/full comparison",
            "vertices":input.positions().len(),
            "triangles":input.triangles().len(),
            "parameters":{"center":parameters().center,"radius":parameters().radius,"intensity":parameters().intensity,"passes":1},
            "changed_vertices":sparse.len()/20,
            "sparse_bytes":sparse.len(),
            "full_position_bytes":full.len(),
            "sparse_equal_full_changed_rows":true,
            "native_sparse_observed_ms":sparse_ms,
            "native_full_observed_ms":full_ms,
            "timing_scope":"Sequential one-shot native Metal operations; different order/warm state, not browser or studio speed comparison",
            "gpu_report":gpu.report(),
        });
        std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
