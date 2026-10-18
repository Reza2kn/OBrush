#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::resident_point_program::{PointInflateParameters, ResidentPointInput};
    use obrush_gpu::{PickRay, ResidentViewport};
    pollster::block_on(async {
        let mut mesh = obrush_core::Mesh::sphere(1);
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let input =
            ResidentPointInput::capture(&mesh).expect("Independent sphere input must be valid");
        let mut readback = ResidentViewport::new(&input)
            .await
            .expect("Readback device");
        let mut owned = ResidentViewport::new(&input)
            .await
            .expect("GPU-owned device");
        let ray = PickRay::diagnostic_pixel(256, 256, 512, 512).expect("Center ray");
        let hit = readback
            .pick(ray)
            .await
            .expect("Reference pick")
            .expect("Center hit");
        let radius = 0.18;
        let intensity = 0.35;
        readback
            .inflate(
                PointInflateParameters {
                    center: hit.position,
                    radius,
                    intensity,
                },
                1,
            )
            .expect("Reference dab");
        owned
            .inflate_picked(ray, radius, intensity)
            .expect("GPU-owned dab");
        let reference_positions = readback.positions().await.expect("Reference positions");
        let owned_positions = owned.positions().await.expect("Owned positions");
        let reference_normals = readback.normal_state(0).await.expect("Reference normals");
        let owned_normals = owned.normal_state(0).await.expect("Owned normals");
        let matched_positions = reference_positions == owned_positions;
        let matched_normals = reference_normals == owned_normals;
        let miss = PickRay::diagnostic_pixel(0, 0, 512, 512).expect("Background ray");
        assert!(owned.pick(miss).await.expect("Background pick").is_none());
        owned
            .inflate_picked(miss, radius, intensity)
            .expect("GPU-owned miss");
        let miss_is_noop = owned.positions().await.expect("Post-miss positions") == owned_positions;
        let invalid_rejected = owned.inflate_picked(ray, f32::NAN, intensity).is_err();
        let result = serde_json::json!({
            "vertices": input.positions().len(),
            "triangles": input.triangles().len(),
            "center_face": hit.face,
            "radius": radius,
            "intensity": intensity,
            "matched_positions": matched_positions,
            "matched_normals": matched_normals,
            "background_miss_is_noop": miss_is_noop,
            "invalid_radius_rejected": invalid_rejected,
            "readback": readback.report(),
            "gpu_owned": owned.report(),
        });
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        assert!(matched_positions && matched_normals && miss_is_noop && invalid_rejected);
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
