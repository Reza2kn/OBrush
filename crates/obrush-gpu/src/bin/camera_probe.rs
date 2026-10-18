#[cfg(not(target_arch = "wasm32"))]
// This binary imports the production camera module as a fixture. Its other
// host-facing methods are intentionally unused by this isolated GPU probe.
#[allow(dead_code)]
#[path = "../../../obrush-app/src/source_camera_math.rs"]
mod source_camera_math;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use glam::{DQuat, DVec2, DVec3};
    use obrush_gpu::{ResidentViewport, capture_sgl, parameters};
    use source_camera_math::{
        ClipInput, ProjectionKind, ProjectionZoom, SceneBounds, SourceCameraFrame, SourcePose,
        ViewportDimensions, WorldToView,
    };
    use std::path::Path;

    fn frame(kind: ProjectionKind, width: u32, height: u32) -> SourceCameraFrame {
        let pose = SourcePose {
            rotation: WorldToView::from_camera_to_world(
                DQuat::from_rotation_y(0.23) * DQuat::from_rotation_x(-0.11),
            )
            .unwrap(),
            center: DVec3::ZERO,
            plane_translation: DVec2::new(0.06, -0.04),
            offset_z: 0.0,
            zoom: ProjectionZoom::checked(match kind {
                ProjectionKind::Perspective => 3.1,
                ProjectionKind::Orthographic => 4.8,
            })
            .unwrap(),
        };
        SourceCameraFrame::checked(
            pose,
            kind,
            45.0,
            ViewportDimensions::checked([width, height], DVec2::new(width as f64, height as f64))
                .unwrap(),
            ClipInput::Optimize(
                SceneBounds::checked(DVec3::splat(-1.6), DVec3::splat(1.6)).unwrap(),
            ),
        )
        .unwrap()
    }
    pollster::block_on(async {
        let arguments = std::env::args().collect::<Vec<_>>();
        let input = capture_sgl(&std::fs::read(&arguments[1]).unwrap()).unwrap();
        let out = Path::new(&arguments[2]);
        std::fs::create_dir_all(out).unwrap();
        let mut gpu = ResidentViewport::new(&input).await.unwrap();
        gpu.inflate(parameters(), 32).unwrap();
        let mut cases = Vec::new();
        for (name, kind) in [
            ("perspective", ProjectionKind::Perspective),
            ("orthographic", ProjectionKind::Orthographic),
        ] {
            let camera = frame(kind, 512, 512);
            let local_to_gl_clip = (camera.projection64 * camera.view64).to_cols_array();
            gpu.set_camera_matrix(local_to_gl_clip, [512, 512]).unwrap();
            gpu.draw().unwrap();
            std::fs::write(
                out.join(format!("{name}.png")),
                obrush_gpu::png(&gpu.pixels().await.unwrap()).unwrap(),
            )
            .unwrap();
            let mut samples = Vec::new();
            for [x, y] in [[256, 256], [290, 240], [0, 0]] {
                let (origin, direction) = camera
                    .ray_device(DVec2::new(x as f64 + 0.5, y as f64 + 0.5))
                    .unwrap();
                let ray = gpu
                    .prepare_local_ray(obrush_core::SceneRay64 { origin, direction }, 5000.0)
                    .unwrap();
                let hit = gpu.pick(ray).await.unwrap();
                let projection = hit.map(|h| {
                    camera
                        .project_device(DVec3::from_array(h.position.map(f64::from)))
                        .unwrap()
                        .to_array()
                });
                samples.push(serde_json::json!({
                    "pixel":[x,y],
                    "ray_origin":origin.to_array(),
                    "ray_direction":direction.to_array(),
                    "gpu_ray":{"origin":ray.origin,"direction":ray.direction,"max_distance":ray.max_distance,"distance_offset":ray.source_distance_offset()},
                    "hit":hit.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
                    "projected_hit_device":projection,
                }));
            }
            cases.push(serde_json::json!({
                "kind":name,
                "local_to_gl_clip":local_to_gl_clip,
                "view64":camera.view64.to_cols_array(),
                "projection64":camera.projection64.to_cols_array(),
                "clip":{"near":camera.clip.near_distance,"far":camera.clip.far_distance},
                "eye64":camera.eye64.to_array(),
                "samples":samples,
                "report":gpu.report(),
            }));
        }
        let resized = frame(ProjectionKind::Perspective, 960, 540);
        let (resized_origin, resized_direction) =
            resized.ray_device(DVec2::new(480.5, 270.5)).unwrap();
        let resized_case = serde_json::json!({
            "kind":"perspective",
            "extent":[960,540],
            "local_to_gl_clip":(resized.projection64 * resized.view64).to_cols_array(),
            "center_ray_origin":resized_origin.to_array(),
            "center_ray_direction":resized_direction.to_array(),
        });
        let before = gpu.pixels().await.unwrap();
        let rejected = gpu.set_camera_matrix([f64::NAN; 16], [512, 512]).is_err();
        let mismatch_rejected = gpu
            .set_camera_matrix(
                (resized.projection64 * resized.view64).to_cols_array(),
                [960, 540],
            )
            .is_err();
        gpu.draw().unwrap();
        let after = gpu.pixels().await.unwrap();
        assert!(rejected && mismatch_rejected && before == after);
        let result = serde_json::json!({
            "source_camera_file":"crates/obrush-app/src/source_camera_math.rs",
            "fixture_bytes":67932232,
            "cases":cases,
            "resized_case":resized_case,
            "invalid_matrix_rejected_without_changing_frame":rejected,
            "mismatched_extent_rejected_without_changing_frame":mismatch_rejected,
            "last_report":gpu.report(),
        });
        std::fs::write(
            out.join("result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        println!("Actual staged OBrush Source camera GPU probe complete");
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
