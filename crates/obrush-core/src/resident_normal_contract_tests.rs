use super::*;
use crate::resident_normal_program::ResidentNormalReference;
use crate::resident_point_program::ResidentPointInput;
fn fixture(mixed: bool) -> Mesh {
    let mut m = if mixed {
        Mesh::new(
            vec![
                Vec3::new(-1., -1., 0.),
                Vec3::new(1., -1., 0.1),
                Vec3::new(1., 1., 0.3),
                Vec3::new(-1., 1., -0.2),
                Vec3::new(0., 2., 0.4),
            ],
            vec![[0, 1, 2], [0, 2, 3], [3, 2, 4]],
        )
    } else {
        Mesh::sphere(2)
    };
    if mixed {
        m.surface = Some(crate::SurfaceTopology {
            faces: vec![[0, 1, 2, 3], [3, 2, 4, crate::TRIANGLE_CORNER]],
            uv: None,
        });
        m.validate().unwrap();
    }
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    for (i, u) in m.masks.iter_mut().enumerate() {
        *u = [0., 0.25, 1.][i % 3];
    }
    m
}
#[test]
fn resident_scalar_matches_original_selected_refresh_and_retains_stale_rows() {
    for mixed in [false, true] {
        let mut mesh = fixture(mixed);
        let input = ResidentPointInput::capture(&mesh).unwrap();
        let mut actual = ResidentNormalReference::new(&input).unwrap();
        let mut state = SourceGeometryNormalState::seed(&mesh).unwrap();
        for selected in [
            vec![],
            vec![0],
            vec![0, 0, 1],
            vec![mesh.positions.len() as u32 - 1],
            (0..mesh.positions.len() as u32).collect(),
        ] {
            mesh.positions[0].z += 0.0375;
            let last = mesh.positions.len() - 1;
            mesh.positions[last].x -= 0.0125;
            let positions = mesh
                .positions
                .iter()
                .map(|p| [p.x, p.y, p.z, 1.])
                .collect::<Vec<_>>();
            let old = actual.normal_u().to_vec();
            actual.refresh_selected(&positions, &selected).unwrap();
            state = state
                .refreshed(
                    &mesh.positions,
                    &selected.iter().map(|v| *v as usize).collect::<Vec<_>>(),
                )
                .unwrap();
            assert_eq!(
                actual
                    .face_normals()
                    .iter()
                    .map(|n| n.map(f32::to_bits))
                    .collect::<Vec<_>>(),
                state
                    .face_normals()
                    .iter()
                    .map(|n| n.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            );
            for (v, n) in actual.normal_u().iter().enumerate() {
                assert_eq!(
                    n[..3].iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
                    state.vertex_normals()[v].to_array().map(f32::to_bits)
                );
                assert_eq!(n[3].to_bits(), input.normal_u()[v][3].to_bits());
                if !selected.contains(&(v as u32)) {
                    assert_eq!(n.map(f32::to_bits), old[v].map(f32::to_bits));
                }
            }
        }
    }
}
#[test]
fn resident_invalid_refresh_is_atomic_and_compact_headers_rings_preserve_order() {
    let mesh = fixture(true);
    let input = ResidentPointInput::capture(&mesh).unwrap();
    let topology = input.normal_topology();
    assert_eq!(topology.headers(), [1, 4]);
    for v in 0..mesh.positions.len() {
        let row =
            &topology.rings()[topology.offsets()[v] as usize..topology.offsets()[v + 1] as usize];
        assert!(row.windows(2).all(|pair| pair[0] >= pair[1]));
    }
    let mut actual = ResidentNormalReference::new(&input).unwrap();
    let before = actual.normal_u().to_vec();
    let faces = actual.face_normals().to_vec();
    let mut bad = input.positions().to_vec();
    bad[0][0] = f32::NAN;
    assert!(actual.refresh_selected(&bad, &[0]).is_err());
    assert!(actual
        .refresh_selected(input.positions(), &[u32::MAX])
        .is_err());
    assert_eq!(actual.normal_u(), before);
    assert_eq!(actual.face_normals(), faces);
    assert!(actual.visited().iter().all(|v| *v == 0));
    assert_eq!(
        ResidentPointInput::storage_requirement(1_000_000, 2_000_000).unwrap(),
        132_000_004
    );
    const {
        assert!(132_000_004 < crate::resident_point_program::MAX_RESIDENT_INPUT_BYTES);
    }
}
