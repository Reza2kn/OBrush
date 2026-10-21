//! Private imported-STL full/window equivalence trace. The asset is supplied
//! through OBRUSH_SPARK_STL and is never copied into the repository.
use super::*;
use crate::{BrushKind, Ray};
use sha2::{Digest, Sha256};

fn camera_ray(screen_x: f32, screen_y: f32) -> Ray {
    let origin = Vec3::new(-2.758_769_8, 34.199_276, 20.482_391);
    let target = Vec3::new(-0.273_416_52, -4.797_350_4, 16.926_394);
    let forward = (target - origin).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();
    let ndc_x = 2.0 * (screen_x - 225.0) / 820.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (screen_y - 57.0) / 812.0;
    let tangent = (40.0_f32.to_radians() * 0.5).tan();
    Ray {
        origin,
        direction: (forward + right * ndc_x * tangent * (820.0 / 812.0) + up * ndc_y * tangent)
            .normalize(),
    }
}

fn mesh_words_equal(left: &Mesh, right: &Mesh) {
    assert_eq!(left.mask_convention, right.mask_convention);
    assert_eq!(left.surface.is_some(), right.surface.is_some());
    assert_eq!(left.positions.len(), right.positions.len());
    assert_eq!(left.triangles.len(), right.triangles.len());
    assert_eq!(
        bytemuck::cast_slice::<Vec3, u8>(&left.positions),
        bytemuck::cast_slice::<Vec3, u8>(&right.positions),
        "position words"
    );
    assert_eq!(
        bytemuck::cast_slice::<Vec3, u8>(&left.normals),
        bytemuck::cast_slice::<Vec3, u8>(&right.normals),
        "normal words"
    );
    assert_eq!(
        bytemuck::cast_slice::<[u32; 3], u8>(&left.triangles),
        bytemuck::cast_slice::<[u32; 3], u8>(&right.triangles),
        "triangle words"
    );
    assert_eq!(
        bytemuck::cast_slice::<f32, u8>(&left.masks),
        bytemuck::cast_slice::<f32, u8>(&right.masks),
        "mask words"
    );
    assert_eq!(
        bytemuck::cast_slice::<[f32; 3], u8>(&left.colors),
        bytemuck::cast_slice::<[f32; 3], u8>(&right.colors),
        "color words"
    );
    match (&left.materials, &right.materials) {
        (None, None) => {}
        (Some(a), Some(b)) => assert_eq!(
            bytemuck::cast_slice::<[f32; 2], u8>(a),
            bytemuck::cast_slice::<[f32; 2], u8>(b),
            "material words"
        ),
        _ => panic!("material presence differs"),
    }
}

fn mesh_sha256(mesh: &Mesh) -> String {
    let mut digest = Sha256::new();
    digest.update(bytemuck::cast_slice::<Vec3, u8>(&mesh.positions));
    digest.update(bytemuck::cast_slice::<Vec3, u8>(&mesh.normals));
    digest.update(bytemuck::cast_slice::<[u32; 3], u8>(&mesh.triangles));
    digest.update(bytemuck::cast_slice::<f32, u8>(&mesh.masks));
    digest.update(bytemuck::cast_slice::<[f32; 3], u8>(&mesh.colors));
    if let Some(materials) = &mesh.materials {
        digest.update(bytemuck::cast_slice::<[f32; 2], u8>(materials));
    }
    format!("{:x}", digest.finalize())
}

fn spark_mesh() -> Mesh {
    let path = std::env::var("OBRUSH_SPARK_STL").expect("Set private OBRUSH_SPARK_STL path");
    let source = std::fs::read(path).unwrap();
    let mesh = Mesh::import_stl(&source).unwrap().mesh;
    assert_eq!(mesh.positions.len(), 369_407);
    assert_eq!(mesh.triangles.len(), 738_900);
    assert!(mesh.surface.is_none());
    mesh
}

fn compare_trace(mut base: Mesh, symmetry_x: bool, contacts: usize, attributes: bool) {
    if attributes {
        for index in (0..base.positions.len()).step_by(97) {
            base.masks[index] = 0.2;
            base.colors[index] = [0.3, 0.5, 0.7];
        }
        base.materials = Some(vec![[0.25, 0.75]; base.positions.len()]);
        base.validate().unwrap();
    }
    let mut full = base.snapshot();
    let mut window = base.snapshot();
    let mut accepted = 0;
    let mut fallback = 0;
    let mut near_edge = 0;
    let mut useful = 0;
    for sample in 0..contacts {
        let angle = sample as f32 * std::f32::consts::TAU / 20.0;
        let ray = camera_ray(
            635.0 + 18.0 * angle.sin(),
            463.0 + 18.0 * (1.0 - angle.cos()),
        );
        let hit = full.raycast(ray).expect("Spark contact ray must hit");
        let twin = window.raycast(ray).expect("Window contact ray must hit");
        assert_eq!(hit.triangle, twin.triangle, "pick triangle at {sample}");
        assert_eq!(
            bytemuck::bytes_of(&hit.barycentric),
            bytemuck::bytes_of(&twin.barycentric),
            "pick barycentric words at {sample}"
        );
        if hit.barycentric.into_iter().fold(1.0_f32, f32::min) < 0.01 {
            near_edge += 1;
        }
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            symmetry_x,
            culling_axis: Some(-ray.direction),
            ..Default::default()
        };
        let expected = full
            .apply_brush_dab(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings,
                None,
                true,
                true,
            )
            .0;
        let actual = if let Some(result) =
            window.try_local_draw_window(twin.position, twin.normal, Vec3::ZERO, &settings)
        {
            accepted += 1;
            result
        } else {
            fallback += 1;
            window
                .apply_brush_dab(
                    twin.position,
                    twin.normal,
                    Vec3::ZERO,
                    &settings,
                    None,
                    true,
                    true,
                )
                .0
        };
        assert_eq!(expected, actual, "BrushResult differs at contact {sample}");
        mesh_words_equal(&full, &window);
        if expected.changed_vertices > 0 {
            useful += 1;
        }
        if (sample + 1) % 32 == 0 || sample + 1 == contacts {
            eprintln!(
                "SPARK_EXACT_TRACE,symmetry={symmetry_x},attributes={attributes},contact={},accepted={accepted},fallback={fallback},near_edge={near_edge},useful={useful},vertices={},triangles={},sha256={}",
                sample + 1,
                full.positions.len(),
                full.triangles.len(),
                mesh_sha256(&full)
            );
        }
    }
    assert!(useful > 0);
    assert!(accepted > 0, "window path was never accepted");
    if contacts >= 256 {
        assert!(
            near_edge > 0,
            "fixture did not encounter any near-edge picks"
        );
    }
    full.validate().unwrap();
    window.validate().unwrap();
}

#[test]
#[ignore = "requires private Sparkdancer STL path and release-optimized run"]
fn private_spark_256_x_off_full_window_exact() {
    compare_trace(spark_mesh(), false, 256, false);
}

#[test]
#[ignore = "requires private Sparkdancer STL path and release-optimized run"]
fn private_spark_256_x_on_full_window_exact() {
    compare_trace(spark_mesh(), true, 256, false);
}

#[test]
#[ignore = "requires private Sparkdancer STL path and release-optimized run"]
fn private_spark_attributes_full_window_exact() {
    for symmetry_x in [false, true] {
        compare_trace(spark_mesh(), symmetry_x, 8, true);
    }
}
