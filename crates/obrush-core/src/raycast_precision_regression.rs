//! Independently authored shared-edge precision regression for distant picks.
// The authored decimal coordinates are retained verbatim as fixture provenance;
// conversion to f32 occurs when the Vec3 values are constructed.
#![allow(clippy::excessive_precision)]
use super::*;

#[test]
fn distant_near_edge_front_surface_is_not_lost_to_rear_shell() {
    // Fixed seed 20261021, sample 156 from an authored random paired-triangle
    // generator. No imported or proprietary model contributes coordinates.
    let origin = Vec3::new(2.5, -31.700_000_762_939_453, 6.300_000_190_734_863);
    let direction = Vec3::new(
        -0.137_022_659_182_548_52,
        0.970_160_543_918_609_6,
        0.200_033_098_459_243_77,
    );
    let axis = direction.normalize_or_zero();
    let u = (Vec3::X - axis * Vec3::X.dot(axis)).normalize();
    let v = axis.cross(u).normalize();
    let rear = origin + axis * 35.7;
    let mut mesh = Mesh::new(
        vec![
            Vec3::new(
                -1.704_394_340_515_136_7,
                -1.922_909_736_633_300_8,
                12.395_369_529_724_121,
            ),
            Vec3::new(
                -1.733_661_174_774_17,
                -1.919_737_815_856_933_6,
                12.440_261_840_820_312,
            ),
            Vec3::new(
                -1.679_530_620_574_951_2,
                -1.912_404_298_782_348_6,
                12.441_773_414_611_816,
            ),
            Vec3::new(
                -1.704_394_340_515_136_7,
                -1.940_408_229_827_880_9,
                12.480_237_007_141_113,
            ),
            rear + u * 0.5,
            rear - u * 0.25 + v * 0.433,
            rear - u * 0.25 - v * 0.433,
        ],
        vec![[0, 1, 2], [3, 2, 1], [4, 5, 6]],
    );
    mesh.validate().unwrap();
    let ray = Ray { origin, direction };
    let (row, hit) = mesh
        .raycast_indexed(ray)
        .expect("front surface or rear shell");
    assert_eq!(row, 0, "f32-only bary rejection must not expose rear shell");
    assert_eq!(hit.triangle, [0, 1, 2]);
    assert!((hit.distance - 30.7).abs() < 0.0001);

    // The result must be deterministic and not depend on cached normals.
    mesh.recompute_normals();
    let repeat = mesh.raycast_indexed(ray).unwrap();
    assert_eq!(repeat.0, row);
    assert_eq!(repeat.1.distance.to_bits(), hit.distance.to_bits());
}
