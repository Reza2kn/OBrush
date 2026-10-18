#[cfg(not(target_arch = "wasm32"))]
mod native {
    use obrush_gpu::*;
    use std::path::Path;
    fn save(out: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(out.join(name), bytes).unwrap();
    }
    pub fn run() {
        pollster::block_on(async {
            let args = std::env::args().collect::<Vec<_>>();
            let out = Path::new(&args[2]);
            std::fs::create_dir(out).unwrap();
            let bytes = std::fs::read(&args[1]).unwrap();
            let input = capture_sgl(&bytes).unwrap();
            drop(bytes);
            let input_files: [(&str, &[u8]); 8] = [
                (
                    "input-positions.bin",
                    bytemuck::cast_slice(input.positions()),
                ),
                ("input-normal-u.bin", bytemuck::cast_slice(input.normal_u())),
                ("input-colors.bin", bytemuck::cast_slice(input.colors())),
                (
                    "input-triangles.bin",
                    bytemuck::cast_slice(input.triangles()),
                ),
                (
                    "input-face-headers.bin",
                    bytemuck::cast_slice(input.normal_topology().headers()),
                ),
                (
                    "input-ring-offsets.bin",
                    bytemuck::cast_slice(input.normal_topology().offsets()),
                ),
                (
                    "input-rings.bin",
                    bytemuck::cast_slice(input.normal_topology().rings()),
                ),
                (
                    "input-face-cache.bin",
                    bytemuck::cast_slice(input.normal_topology().face_normals()),
                ),
            ];
            for (name, data) in input_files {
                save(out, name, data);
            }
            let mut reference = ResidentNormalReference::new(&input).unwrap();
            for _ in 0..32 {
                reference.point_pass(parameters()).unwrap();
            }
            for (name, data) in [
                (
                    "reference-positions.bin",
                    bytemuck::cast_slice(reference.positions()),
                ),
                (
                    "reference-normal-u.bin",
                    bytemuck::cast_slice(reference.normal_u()),
                ),
                (
                    "reference-face-cache.bin",
                    bytemuck::cast_slice(reference.face_normals()),
                ),
                (
                    "reference-visited.bin",
                    bytemuck::cast_slice(reference.visited()),
                ),
            ] {
                save(out, name, data);
            }
            drop(reference);
            let mut gpu = ResidentViewport::new(&input).await.unwrap();
            gpu.checkpoint().unwrap();
            gpu.draw().unwrap();
            let before = gpu.pixels().await.unwrap();
            save(out, "before.png", &png(&before).unwrap());
            gpu.inflate(parameters(), 32).unwrap();
            gpu.draw().unwrap();
            let after = gpu.pixels().await.unwrap();
            save(out, "after.png", &png(&after).unwrap());
            let before_readback = gpu.report();
            assert_eq!(before_readback["position_readbacks"], 0);
            assert_eq!(before_readback["normal_state_readbacks"], 0);
            assert_ne!(before, after);
            assert_eq!(gpu.failure_vertex().await.unwrap(), None);
            save(out, "after-positions.bin", &gpu.positions().await.unwrap());
            for (channel, name) in [
                (0, "after-normal-u.bin"),
                (1, "after-face-cache.bin"),
                (2, "after-visited.bin"),
                (3, "after-current-selection.bin"),
            ] {
                save(out, name, &gpu.normal_state(channel).await.unwrap());
            }
            gpu.cancel().unwrap();
            gpu.draw().unwrap();
            let cancel = gpu.pixels().await.unwrap();
            save(out, "cancel.png", &png(&cancel).unwrap());
            assert_eq!(before, cancel);
            let positions = gpu.positions().await.unwrap();
            assert_eq!(
                positions.as_slice(),
                bytemuck::cast_slice::<[f32; 4], u8>(input.positions())
            );
            save(out, "cancel-positions.bin", &positions);
            let normals = gpu.normal_state(0).await.unwrap();
            let faces = gpu.normal_state(1).await.unwrap();
            let visited = gpu.normal_state(2).await.unwrap();
            assert_eq!(
                normals.as_slice(),
                bytemuck::cast_slice::<[f32; 4], u8>(input.normal_u())
            );
            assert_eq!(
                faces.as_slice(),
                bytemuck::cast_slice::<[f32; 3], u8>(input.normal_topology().face_normals())
            );
            assert!(visited.iter().all(|v| *v == 0));
            save(out, "cancel-normal-u.bin", &normals);
            save(out, "cancel-face-cache.bin", &faces);
            save(out, "cancel-visited.bin", &visited);
            let prior = gpu.report()["point_passes_enqueued"].clone();
            assert!(
                gpu.inflate(
                    obrush_core::resident_point_program::PointInflateParameters {
                        radius: f32::NAN,
                        ..parameters()
                    },
                    1
                )
                .is_err()
            );
            assert_eq!(prior, gpu.report()["point_passes_enqueued"]);
            let before_destroy = gpu.report();
            gpu.destroy();
            assert!(gpu.draw().is_err());
            drop(gpu);
            let mut recreated = ResidentViewport::new(&input).await.unwrap();
            recreated.draw().unwrap();
            assert_eq!(recreated.pixels().await.unwrap(), before);
            recreated.destroy();
            let invalid = invalid_candidate_trial().await.unwrap();
            let cases = normal_case_trials().await.unwrap();
            let result = serde_json::json!({"status":"PASS scoped nativeMetal selected-normal point program on original1M; all retained checkpoint fields restored","before_retained_readback":before_readback,"before_destroy":before_destroy,"invalid_candidate":invalid,"normal_cases":cases,"limits":"No Source proxy/contact/culling/alpha/mirror/guards/document/history/frontend/performance qualification; GPU f32 cache is not assumed equal to f64"});
            save(out, "result.json", result.to_string().as_bytes());
            println!("Native retained-normal test completed");
        });
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    native::run();
}
#[cfg(target_arch = "wasm32")]
fn main() {}
