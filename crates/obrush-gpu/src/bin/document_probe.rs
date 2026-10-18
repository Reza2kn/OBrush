#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "../../../obrush-app/src/source_camera_math.rs"]
mod source_camera_math;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use glam::{DMat4, DQuat, DVec2, DVec3};
    use obrush_core::resident_point_program::{ResidentDocumentInput, document_local_ray};
    use obrush_core::{Document, SceneRay64, SglPickingPolicy};
    use obrush_gpu::ResidentViewport;
    use source_camera_math::{
        ClipInput, ProjectionKind, ProjectionZoom, SceneBounds, SourceCameraFrame, SourcePose,
        ViewportDimensions, WorldToView,
    };

    pollster::block_on(async {
        let arguments = std::env::args().collect::<Vec<_>>();
        let bytes = std::fs::read(&arguments[1]).unwrap();
        let document = Document::from_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let saved = document.to_json().unwrap();
        let snapshot = ResidentDocumentInput::from_project_bytes(&bytes).unwrap();
        let pose = DMat4::from_cols_array(&snapshot.local_to_world().map(f64::from));
        let mut minimum = DVec3::splat(f64::INFINITY);
        let mut maximum = DVec3::splat(f64::NEG_INFINITY);
        for point in snapshot.points().positions() {
            let world = pose.transform_point3(DVec3::new(
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            ));
            minimum = minimum.min(world);
            maximum = maximum.max(world);
        }
        let bounds = SceneBounds::checked(minimum, maximum).unwrap();
        let center = bounds.center();
        let span = (maximum - minimum).max_element();
        let camera = SourceCameraFrame::checked(
            SourcePose {
                rotation: WorldToView::from_camera_to_world(DQuat::IDENTITY).unwrap(),
                center,
                plane_translation: DVec2::ZERO,
                offset_z: 0.,
                zoom: ProjectionZoom::checked(2.8 * span).unwrap(),
            },
            ProjectionKind::Orthographic,
            45.,
            ViewportDimensions::checked([512, 512], DVec2::splat(512.)).unwrap(),
            ClipInput::Optimize(bounds),
        )
        .unwrap();
        let world_to_gl_clip = (camera.projection64 * camera.view64).to_cols_array();
        let local_to_gl_clip = snapshot.local_to_gl_clip(world_to_gl_clip).unwrap();
        let input = snapshot.points();
        let mut gpu = ResidentViewport::new(input).await.unwrap();
        let normal_transform_applied = !arguments.iter().any(|arg| arg == "--local-normals");
        if normal_transform_applied {
            gpu.set_document_camera_matrix(
                local_to_gl_clip,
                *snapshot.local_to_world(),
                [512, 512],
            )
            .unwrap();
        } else {
            gpu.set_camera_matrix(local_to_gl_clip, [512, 512]).unwrap();
        }
        gpu.draw().unwrap();
        let output = std::path::Path::new(&arguments[2]);
        std::fs::create_dir_all(output).unwrap();
        std::fs::write(
            output.join("native.png"),
            obrush_gpu::png(&gpu.pixels().await.unwrap()).unwrap(),
        )
        .unwrap();
        let mut samples = Vec::new();
        for y in [96, 160, 224, 288, 352, 416] {
            for x in [96, 160, 224, 288, 352, 416] {
                let (world_origin, world_direction) = camera
                    .ray_device(DVec2::new(f64::from(x) + 0.5, f64::from(y) + 0.5))
                    .unwrap();
                let world_ray = SceneRay64 {
                    origin: world_origin,
                    direction: world_direction,
                };
                let local =
                    document_local_ray(*snapshot.local_to_world(), world_ray, 5000.).unwrap();
                let prepared = gpu
                    .prepare_local_ray(local.ray, local.max_distance)
                    .unwrap();
                let gpu_hit = gpu.pick(prepared).await.unwrap();
                let cpu_hit = document
                    .raycast_scene(world_ray, SglPickingPolicy::WorldDistanceV1)
                    .unwrap();
                if gpu_hit.is_some() || cpu_hit.is_some() {
                    samples.push(serde_json::json!({
                        "pixel":[x,y],
                        "world_ray":{"origin":world_origin.to_array(),"direction":world_direction.to_array()},
                        "local_ray":{"origin":local.ray.origin.to_array(),"direction":local.ray.direction.to_array(),"max_distance":local.max_distance,"world_to_local_distance_scale":local.world_to_local_distance_scale},
                        "gpu":gpu_hit.map(|h|serde_json::json!({"face":h.face,"distance":h.distance,"position":h.position})),
                        "cpu":cpu_hit.map(|h|serde_json::json!({"face":h.triangle_index,"position":h.local_position.to_array(),"world_position":h.world_position.to_array()})),
                    }));
                }
            }
        }
        assert!(!samples.is_empty());
        let result = serde_json::json!({
            "source_project_bytes":bytes.len(),
            "subtool_id":snapshot.subtool_id(),
            "source_mask_convention":format!("{:?}",snapshot.source_mask_convention()),
            "mask_roundtrip_changed_words":snapshot.mask_roundtrip_changed_words(),
            "normal_transform_applied":normal_transform_applied,
            "world_to_gl_clip":world_to_gl_clip,
            "local_to_gl_clip":local_to_gl_clip,
            "local_to_world":snapshot.local_to_world(),
            "world_bounds":{"minimum":minimum.to_array(),"maximum":maximum.to_array()},
            "samples":samples,
            "report":gpu.report(),
            "project_unchanged":document.to_json().unwrap()==saved,
        });
        std::fs::write(
            output.join("native.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        println!(
            "Native document GPU probe complete: {} visible samples",
            samples.len()
        );
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
