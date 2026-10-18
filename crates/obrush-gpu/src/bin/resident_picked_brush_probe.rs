#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use obrush_core::resident_point_program::{PointBrushParameters, ResidentDocumentInput};
    use obrush_core::{Document, SceneRay64};
    use obrush_gpu::ResidentViewport;

    pollster::block_on(async {
        let output = std::env::args().nth(1).expect("output JSON path");
        let mut document = Document::from_json(include_str!(
            "../../tests/fixtures/resident-sphere-162.obrush"
        ))
        .unwrap();
        let before = document.to_json().unwrap();
        let capture = ResidentDocumentInput::capture(&document).unwrap();
        let mut gpu = ResidentViewport::new(capture.points()).await.unwrap();
        let local = SceneRay64 {
            origin: glam::DVec3::new(0.17, 0.11, 3.0),
            direction: -glam::DVec3::Z,
        };
        let ray = gpu.prepare_local_ray(local, 5.0).unwrap();
        let hit = gpu.pick(ray).await.unwrap().expect("authored sphere hit");
        let direction = capture
            .points()
            .reference_picked_brush_direction(hit.face, hit.position)
            .unwrap();
        let parameters = PointBrushParameters {
            center: hit.position,
            direction,
            radius: 0.5,
            intensity: 0.25,
        };
        let expected = capture
            .points()
            .reference_brush_positions(parameters, 1)
            .unwrap();
        gpu.checkpoint().unwrap();
        gpu.brush_picked(ray, 0.5, 0.25).unwrap();
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
        let maximum_component_difference = actual
            .iter()
            .zip(&expected)
            .flat_map(|(gpu, scalar)| {
                gpu.iter()
                    .zip(scalar)
                    .map(|(gpu, scalar)| (gpu - scalar).abs())
            })
            .fold(0.0_f32, f32::max);
        assert!(maximum_component_difference <= 2.0e-6);
        let sparse = gpu.sparse_positions().await.unwrap();
        assert!(!sparse.is_empty());
        assert!(
            capture
                .commit_sparse_position_bytes(&mut document, &sparse)
                .unwrap()
        );
        let after = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
        gpu.cancel().unwrap();
        assert!(gpu.sparse_positions().await.unwrap().is_empty());
        let quad_obj = "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0.5 0.5 0.15\nv -0.5 0.5 0\nf 1 2 3 4\n";
        let mut quad = obrush_core::Mesh::from_obj(quad_obj).unwrap();
        quad.triangles = vec![[0, 1, 2], [0, 2, 3]];
        quad.recompute_normals();
        quad.surface = Some(obrush_core::SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: None,
        });
        let quad_capture = ResidentDocumentInput::capture(&Document::new(quad)).unwrap();
        assert_eq!(quad_capture.points().normal_topology().headers().len(), 1);
        assert_eq!(quad_capture.points().triangles().len(), 2);
        let mut quad_gpu = ResidentViewport::new(quad_capture.points()).await.unwrap();
        let quad_ray = quad_gpu
            .prepare_local_ray(
                SceneRay64 {
                    origin: glam::DVec3::new(0.12, 0.18, 2.0),
                    direction: -glam::DVec3::Z,
                },
                4.0,
            )
            .unwrap();
        let quad_hit = quad_gpu.pick(quad_ray).await.unwrap().unwrap();
        let quad_direction = quad_capture
            .points()
            .reference_picked_brush_direction(quad_hit.face, quad_hit.position)
            .unwrap();
        let quad_expected = quad_capture
            .points()
            .reference_brush_positions(
                PointBrushParameters {
                    center: quad_hit.position,
                    direction: quad_direction,
                    radius: 1.5,
                    intensity: 0.25,
                },
                1,
            )
            .unwrap();
        quad_gpu.brush_picked(quad_ray, 1.5, 0.25).unwrap();
        assert_eq!(quad_gpu.failure_vertex().await.unwrap(), None);
        let quad_actual = quad_gpu.positions().await.unwrap();
        let quad_difference = quad_actual
            .chunks_exact(4)
            .map(|word| f32::from_le_bytes(word.try_into().unwrap()))
            .zip(quad_expected.iter().flatten())
            .map(|(gpu, scalar)| (gpu - scalar).abs())
            .fold(0.0_f32, f32::max);
        assert!(quad_difference <= 2.0e-6);
        let result = serde_json::json!({
            "status":"PASS native GPU-owned picked ordinary Brush",
            "ray":{"origin":local.origin.to_array(),"direction":local.direction.to_array(),"max_distance":5.0},
            "pick":{"face":hit.face,"position":hit.position,"distance":hit.distance},
            "reference_direction":direction,"radius":0.5,"intensity":0.25,
            "maximum_component_difference":maximum_component_difference,
            "position_words_le":actual_bytes.chunks_exact(4).map(|word|u32::from_le_bytes(word.try_into().unwrap())).collect::<Vec<_>>(),
            "sparse_bytes":sparse.len(),"undo_exact":true,"redo_exact":true,"cancel_exact":true,
            "gpu_report":gpu.report(),
            "authored_quad":{"picked_triangle":quad_hit.face,"original_faces":1,"triangles":2,"reference_direction":quad_direction,"maximum_component_difference":quad_difference},
        });
        std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}
