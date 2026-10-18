#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::resident_point_program::PointInflateParameters;
    use obrush_gpu::{ResidentViewport, capture_sgl, parameters};
    use std::path::Path;
    pollster::block_on(async {
        let arguments = std::env::args().collect::<Vec<_>>();
        let bytes = std::fs::read(&arguments[1]).unwrap();
        let input = capture_sgl(&bytes).unwrap();
        let destination = Path::new(&arguments[2]);
        std::fs::create_dir_all(destination).unwrap();
        let mut viewport = ResidentViewport::new(&input).await.unwrap();
        let initial = viewport.pick_pixel(256, 256, 512, 512).await.unwrap();
        let initial_ridge = viewport.pick_pixel(128, 256, 512, 512).await.unwrap();
        viewport.checkpoint().unwrap();
        viewport.inflate(parameters(), 32).unwrap();
        let first = viewport.pick_pixel(256, 256, 512, 512).await.unwrap();
        let second = viewport.pick_pixel(325, 256, 512, 512).await.unwrap();
        let ridge = viewport.pick_pixel(128, 256, 512, 512).await.unwrap();
        let background = viewport.pick_pixel(0, 0, 512, 512).await.unwrap();
        let invalid = viewport.pick_pixel(512, 256, 512, 512).await.is_err();
        let before_second_operation = viewport.report();
        let center = ridge
            .expect("Expected current deformed ridge pick")
            .position;
        viewport
            .inflate(
                PointInflateParameters {
                    center,
                    radius: 0.13,
                    intensity: 0.2,
                },
                1,
            )
            .unwrap();
        let after_centered = viewport.pick_pixel(128, 256, 512, 512).await.unwrap();
        let before_full_readback = viewport.report();
        viewport.draw().unwrap();
        let pixels = viewport.pixels().await.unwrap();
        std::fs::write(
            destination.join("after-picked-dab.png"),
            obrush_gpu::png(&pixels).unwrap(),
        )
        .unwrap();
        std::fs::write(
            destination.join("final-positions.bin"),
            viewport.positions().await.unwrap(),
        )
        .unwrap();
        let summary = serde_json::json!({
            "initial": initial.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "initial_ridge": initial_ridge.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "after_32_center": first.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "after_32_offset": second.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "after_32_ridge": ridge.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "background": background.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "invalid_pixel_rejected": invalid,
            "before_second_operation": before_second_operation,
            "dab_center": center,
            "after_current_surface_ridge_operation": after_centered.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
            "before_full_readback": before_full_readback,
            "after_full_readback": viewport.report(),
        });
        std::fs::write(
            destination.join("result.json"),
            serde_json::to_vec_pretty(&summary).unwrap(),
        )
        .unwrap();
        println!("Native current-surface GPU pick probe complete");
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
