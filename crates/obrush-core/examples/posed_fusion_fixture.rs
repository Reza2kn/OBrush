//! Independently authored pair of closed boxes with editable Source poses.
//! `cargo run -p obrush-core --example posed_fusion_fixture -- output.sgl`
use obrush_core::{
    SglByteOrder, SglCameraSettings, SglDisplaySettings, SglMesh, SglRenderSettings, SglScene,
};

const TRIANGLE: u32 = u32::MAX;
const FACES: [[u32; 4]; 12] = [
    [0, 2, 1, TRIANGLE],
    [0, 3, 2, TRIANGLE],
    [4, 5, 6, TRIANGLE],
    [4, 6, 7, TRIANGLE],
    [0, 1, 5, TRIANGLE],
    [0, 5, 4, TRIANGLE],
    [3, 7, 6, TRIANGLE],
    [3, 6, 2, TRIANGLE],
    [0, 4, 7, TRIANGLE],
    [0, 7, 3, TRIANGLE],
    [1, 2, 6, TRIANGLE],
    [1, 6, 5, TRIANGLE],
];
const POSITIONS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];

fn box_object(matrix: [f32; 16], color: [f32; 3]) -> SglMesh {
    SglMesh {
        render: Some(SglRenderSettings {
            shader: 0,
            matcap: 3,
            show_wireframe: false,
            flat_shading: false,
            opacity: 1.0,
        }),
        center: [0.0; 3],
        matrix,
        scale: 1.0,
        positions: POSITIONS.to_vec(),
        colors: Some(vec![color; POSITIONS.len()]),
        materials: Some(vec![[0.4, 0.6, 1.0]; POSITIONS.len()]),
        faces: FACES.to_vec(),
        uv: None,
    }
}

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("Pass an output .sgl path")?;
    let scene = SglScene {
        version: 3,
        byte_order: SglByteOrder::LittleEndian,
        display: Some(SglDisplaySettings {
            show_grid: false,
            show_symmetry_line: false,
            show_contour: false,
        }),
        camera: Some(SglCameraSettings {
            projection: 0,
            mode: 0,
            fov_degrees: 40.0,
            use_pivot: true,
        }),
        meshes: vec![
            box_object(
                [
                    1.0, 0.0, 0.0, 0.0, 0.15, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -0.3, 0.0, 0.0,
                    1.0,
                ],
                [1.0, 0.0, 0.0],
            ),
            box_object(
                [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.3, 0.0, 0.0, 1.0,
                ],
                [0.0, 0.0, 1.0],
            ),
        ],
    };
    std::fs::write(&path, scene.to_bytes()?).map_err(|error| format!("Write failed: {error}"))?;
    println!("{path}");
    Ok(())
}
