use obrush_core::{Mesh, Vec3};

fn fixture() -> Mesh {
    let source = format!(
        "v 0 0 0\nv 1 0 0\nv 0 1 0\n{}f 1 2 3\n",
        "v 0 0 0\n".repeat(766)
    );
    let mut mesh = Mesh::from_obj(&source).unwrap();
    mesh.normals.fill(Vec3::Z);
    mesh
}

#[test]
fn validation_keeps_first_row_errors_across_block_boundaries() {
    for row in [0, 255, 256, 511, 512, 768] {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1_000_000.1] {
            let mut mesh = fixture();
            mesh.positions[row].y = bad;
            assert_eq!(
                mesh.validate().unwrap_err(),
                format!("Vertex {row} has a non-finite or excessive coordinate")
            );
        }
        for bad in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -f32::MIN_POSITIVE,
            1.0_f32.next_up(),
        ] {
            let mut mesh = fixture();
            mesh.masks[row] = bad;
            mesh.masks[768] = bad;
            assert_eq!(
                mesh.validate().unwrap_err(),
                format!("Mask {row} must be in 0–1")
            );
            for channel in 0..3 {
                let mut mesh = fixture();
                mesh.colors[row][channel] = bad;
                mesh.colors[768][channel] = bad;
                assert_eq!(
                    mesh.validate().unwrap_err(),
                    format!("Color {row} must have finite channels in 0–1")
                );
            }
        }
    }
}

#[test]
fn validation_preserves_inclusive_limits_signed_zero_and_channel_order() {
    let mut mesh = fixture();
    mesh.positions[255] = Vec3::new(-1_000_000.0, 1_000_000.0, -0.0);
    mesh.masks[256] = -0.0;
    mesh.masks[512] = 1.0;
    mesh.colors[768] = [-0.0, f32::MIN_POSITIVE, 1.0];
    mesh.validate().unwrap();
    mesh.normals[768] = Vec3::splat(f32::MAX);
    mesh.masks[0] = f32::NAN;
    mesh.colors[0][0] = f32::NAN;
    assert_eq!(mesh.validate().unwrap_err(), "Normal 768 is not finite");
    mesh.positions[512].z = f32::NAN;
    assert_eq!(
        mesh.validate().unwrap_err(),
        "Vertex 512 has a non-finite or excessive coordinate"
    );
}
