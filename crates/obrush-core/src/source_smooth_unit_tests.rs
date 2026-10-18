//! Preregistered independent original-ring and actual pre-guard candidate gates.
//! Authoring only; test/core/reference execution remains separate.
use super::*;
use crate::{MaskConvention, SurfaceTopology};

fn triangles(text: &str, faces: &[[u32; 3]]) -> Mesh {
    let mut m = Mesh::import_obj(text.as_bytes()).unwrap().mesh;
    assert_eq!(m.triangles, faces);
    m.surface = Some(SurfaceTopology {
        faces: faces
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect(),
        uv: None,
    });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.validate().unwrap();
    m
}
fn tetra() -> Mesh {
    triangles(
        include_str!("../tests/fixtures/independent-smooth-tetra.obj"),
        &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]],
    )
}
fn stencil(m: &Mesh, rows: &[usize]) -> (SourceGeometryNormalState, SmoothStencil) {
    let state = SourceGeometryNormalState::seed(m).unwrap();
    let s = SmoothStencil::build(m, &state, &[rows], state.approximate_bytes()).unwrap();
    (state, s)
}
fn candidates(m: &Mesh, rows: &[usize], intensity: f64, tangent: bool) -> Vec<(usize, Vec3)> {
    let (state, s) = stencil(m, rows);
    s.candidates(
        m,
        state.vertex_normals(),
        rows,
        intensity,
        tangent,
        state.approximate_bytes(),
        |_, _| Ok(1.0),
    )
    .unwrap()
}
fn words(p: Vec3) -> [u32; 3] {
    p.to_array().map(f32::to_bits)
}

#[test]
fn original_triangle_and_quad_branch_order_has_no_diagonal() {
    let tri = [10, 11, 12, TRIANGLE_CORNER];
    let quad = [10, 11, 12, 13];
    assert_eq!(perimeter(tri, 10).unwrap(), [12, 11]);
    assert_eq!(perimeter(tri, 11).unwrap(), [10, 12]);
    assert_eq!(perimeter(tri, 12).unwrap(), [10, 11]);
    for (v, pair) in [
        (10, [13, 11]),
        (11, [10, 12]),
        (12, [13, 11]),
        (13, [10, 12]),
    ] {
        assert_eq!(perimeter(quad, v).unwrap(), pair);
    }
    assert_eq!(
        perimeter(quad, 14).unwrap_err(),
        SourceGeometryError::InvalidMesh
    );
    let mut m = Mesh::import_obj(
        include_str!("../tests/fixtures/independent-smooth-two-quads.obj").as_bytes(),
    )
    .unwrap()
    .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    assert_eq!(
        m.surface.as_ref().unwrap().faces,
        vec![[0, 1, 2, 3], [1, 0, 4, 5]]
    );
    let (state, s) = stencil(&m, &[0]);
    assert_eq!(state.ordered_tree.incident_faces(0).unwrap(), [1, 0]);
    assert_eq!(s.ring(0).unwrap(), [1, 4, 3]);
    assert_eq!(words(s.mean(&m.positions, 0).unwrap())[0], 0x3eaaaaab);
}

#[test]
fn degree_two_original_quad_copies_self_despite_render_diagonal() {
    let mut m = Mesh::import_obj(b"v 0 0 1\nv 0 -1 0\nv 2 -1 0\nv 0 1 0\nf 1 2 3 4\n")
        .unwrap()
        .mesh;
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    let (_, s) = stencil(&m, &[0]);
    assert_eq!(s.ring(0).unwrap(), [3, 1]);
    assert_eq!(
        words(s.mean(&m.positions, 0).unwrap()),
        words(m.positions[0])
    );
    assert_eq!(
        words(candidates(&m, &[0], 0.625, false)[0].1),
        words(m.positions[0])
    );
}

#[test]
fn float32_mean_precedes_plain_blend_and_tangent_projection() {
    let m = tetra();
    let (state, s) = stencil(&m, &[0]);
    assert_eq!(words(s.mean(&m.positions, 0).unwrap()), [0x3eaaaaab, 0, 0]);
    assert_eq!(words(state.vertex_normals()[0]), [0, 0, 0x3f2aaaab]);
    assert_eq!(
        words(candidates(&m, &[0], 0.625, false)[0].1),
        [0x3e555556, 0, 0x3ec00000]
    );
    assert_eq!(
        words(candidates(&m, &[0], 0.625, true)[0].1),
        [0x3e555556, 0, 0x3f800000]
    );
    assert_ne!(
        words(candidates(&m, &[0], 0.625, false)[0].1)[0],
        (5.0f32 / 24.0).to_bits()
    );
}

#[test]
fn boundary_priority_ignores_neighbor_protection_in_geometry_mean() {
    let mut m = triangles(
        include_str!("../tests/fixtures/independent-smooth-boundary-grid.obj"),
        &[
            [0, 1, 4],
            [0, 4, 3],
            [1, 2, 5],
            [1, 5, 4],
            [3, 4, 7],
            [3, 7, 6],
            [4, 5, 8],
            [4, 8, 7],
        ],
    );
    m.masks[5] = 0.0;
    m.masks[2] = 0.0;
    let (_, s) = stencil(&m, &[1]);
    assert_eq!(s.ring(1).unwrap(), [4, 5, 2, 0]);
    assert_eq!(s.boundary[1], 2);
    assert_eq!(s.boundary[4], 1);
    assert_eq!([s.boundary[5], s.boundary[2], s.boundary[0]], [2, 2, 2]);
    assert_eq!(
        words(candidates(&m, &[1], 0.75, false)[0].1),
        [0x3fa00000, 0x3e800000, 0x3e800000]
    );
}

#[test]
fn one_boundary_neighbor_discards_partial_sum_and_falls_back_to_full_ring() {
    let m = triangles(
        include_str!("../tests/fixtures/independent-smooth-boundary-fallback.obj"),
        &[
            [0, 1, 2],
            [0, 2, 3],
            [1, 2, 3],
            [1, 2, 4],
            [1, 3, 4],
            [2, 3, 4],
        ],
    );
    let (_, s) = stencil(&m, &[0, 4]);
    assert_eq!(s.ring(0).unwrap(), [3, 2, 1]);
    assert_eq!(s.boundary, [2, 1, 2, 1, 1]);
    assert_eq!(s.mean(&m.positions, 0).unwrap(), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(
        words(candidates(&m, &[0], 0.5, false)[0].1),
        [0, 0x3f000000, 0x3f000000]
    );
}

#[test]
fn descending_current_face_order_preserves_f64_cancellation() {
    let m = triangles(
        include_str!("../tests/fixtures/independent-smooth-ring-cancellation.obj"),
        &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]],
    );
    assert_eq!(m.positions[3].x, 2.0f32.powi(-45));
    let (state, s) = stencil(&m, &[0]);
    assert_eq!(state.ordered_tree.incident_faces(0).unwrap(), [2, 1, 0]);
    assert_eq!(s.ring(0).unwrap(), [1, 3, 2]);
    assert_eq!(words(s.mean(&m.positions, 0).unwrap())[0], 0);
    let wrong =
        ((f64::from(m.positions[1].x) + f64::from(m.positions[2].x) + f64::from(m.positions[3].x))
            / 3.0) as f32;
    assert_eq!(wrong.to_bits(), 0x282aaaab);
}

#[test]
fn every_pass_is_jacobi_independent_of_write_order() {
    let m = tetra();
    let a = candidates(&m, &[0, 1], 0.625, false);
    let b = candidates(&m, &[1, 0], 0.625, false);
    assert_eq!(words(a[0].1), [0x3e555556, 0, 0x3ec00000]);
    assert_eq!(words(a[1].1), [0x3ec00000, 0, 0x3e555556]);
    assert_eq!(a[0], b[1]);
    assert_eq!(a[1], b[0]);
}

#[test]
fn second_raw_pass_reaverages_live_primary_not_rest_or_deduplicated_overlap() {
    let mut m = tetra();
    let (state, s) = stencil(&m, &[0, 1]);
    let first = s
        .candidates(
            &m,
            state.vertex_normals(),
            &[0, 1],
            0.625,
            false,
            state.approximate_bytes(),
            |_, _| Ok(1.0),
        )
        .unwrap();
    for (v, p) in first {
        m.positions[v] = p;
    }
    let second = s
        .candidates(
            &m,
            state.vertex_normals(),
            &[0, 1],
            0.625,
            false,
            state.approximate_bytes(),
            |_, _| Ok(1.0),
        )
        .unwrap();
    assert_eq!(words(second[0].1)[0], 0x3e200000);
    assert_eq!(words(second[1].1)[2], 0x3e200000);
    // This is the real candidate helper, not an accepted public stroke bypass.
}

#[test]
fn all_four_high_intensity_raw_literals_are_preserved_before_guard() {
    let m = tetra();
    let got = candidates(&m, &[0, 1, 2, 3], 0.625, false);
    let expected = [
        [0x3e555556, 0, 0x3ec00000],
        [0x3ec00000, 0, 0x3e555556],
        [0x3e555556, 0x3e2aaaaa, 0x3e555556],
        [0x3e555556, 0xbe2aaaaa, 0x3e555556],
    ];
    assert_eq!(
        got.iter().map(|(_, p)| words(*p)).collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn all_four_lower_intensity_raw_literals_form_exact_dyadic_contraction() {
    let m = tetra();
    let got = candidates(&m, &[0, 1, 2, 3], 0.375, false);
    let expected = [
        [0x3e000000, 0, 0x3f200000],
        [0x3f200000, 0, 0x3e000000],
        [0x3e000000, 0x3f000000, 0x3e000000],
        [0x3e000000, 0xbf000000, 0x3e000000],
    ];
    assert_eq!(
        got.iter().map(|(_, p)| words(*p)).collect::<Vec<_>>(),
        expected
    );
    for (v, p) in got {
        assert_eq!(p, Vec3::new(0.125, 0.0, 0.125) + m.positions[v] * 0.5);
    }
}

#[test]
fn zero_raw_tangent_normal_skips_actual_sampling_callback_and_plain_uses_mean() {
    let m = triangles(
        include_str!("../tests/fixtures/independent-smooth-zero-two-fans.obj"),
        &[[0, 1, 2], [0, 4, 3]],
    );
    let (state, s) = stencil(&m, &[0]);
    assert_eq!(s.ring(0).unwrap(), [3, 4, 2, 1]);
    assert_eq!(state.vertex_normals()[0], Vec3::ZERO);
    let got = s
        .candidates(
            &m,
            state.vertex_normals(),
            &[0],
            0.5,
            true,
            state.approximate_bytes(),
            |_, _| Err(SourceGeometryError::UndefinedAlphaLookup),
        )
        .unwrap();
    assert_eq!(got, vec![(0, Vec3::ZERO)]);
    assert_eq!(
        words(candidates(&m, &[0], 0.5, false)[0].1),
        [0xbf800000, 0, 0]
    );
}

#[test]
fn raw_u_and_alpha_scale_blend_without_weighting_neighbors_or_radial_factor() {
    let mut m = tetra();
    m.masks[0] = 0.5;
    m.masks[1] = 0.0;
    let (state, s) = stencil(&m, &[0]);
    let got = s
        .candidates(
            &m,
            state.vertex_normals(),
            &[0],
            0.375,
            false,
            state.approximate_bytes(),
            |_, _| Ok(128.0 / 255.0),
        )
        .unwrap();
    let mean = f64::from(f32::from_bits(0x3eaaaaab));
    let blend = (3.0 / 8.0 * 0.5) * (128.0 / 255.0);
    assert_eq!(
        words(got[0].1),
        [(mean * blend) as f32, 0.0, (1.0 * (1.0 - blend)) as f32].map(f32::to_bits)
    );
    assert_eq!(words(s.mean(&m.positions, 0).unwrap())[0], 0x3eaaaaab);
}

#[test]
fn union_stencil_rows_are_deduplicated_without_sorting_either_pass() {
    let m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let s = SmoothStencil::build(
        &m,
        &state,
        &[&[1, 0], &[0, 3, 2]],
        state.approximate_bytes(),
    )
    .unwrap();
    assert_eq!(s.selected, [1, 0, 3, 2]);
    assert_eq!(s.ring(0).unwrap(), [1, 3, 2]);
    assert_eq!(s.mean(&m.positions, 0).unwrap().x.to_bits(), 0x3eaaaaab);
}

#[test]
fn incident_accessor_and_private_scratch_limit_reject_without_public_mutation() {
    let m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    assert_eq!(
        state.ordered_tree.incident_faces(usize::MAX).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    assert_eq!(
        state.ordered_tree.incident_faces(4).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    assert!(matches!(
        SmoothStencil::build_at_limit(&m, &state, &[&[0]], state.approximate_bytes(), 0),
        Err(SourceGeometryError::StorageLimit)
    ));
    assert!(matches!(
        SmoothStencil::build(&m, &state, &[&[4]], state.approximate_bytes()),
        Err(SourceGeometryError::InvalidInput)
    ));
    assert_eq!(m.positions, tetra().positions);
}

#[test]
fn useful_candidate_with_zero_retained_budget_rejects_before_mesh_or_stroke_publication() {
    let mut m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let before = serde_json::to_value(&m).unwrap();
    let raw = m.positions.iter().map(|p| words(*p)).collect::<Vec<_>>();
    let mut stroke = super::super::SourceGeometryStroke::new(super::super::SourceGeometryOptions {
        intensity: 0.375,
        ..super::super::SourceGeometryOptions::smooth()
    })
    .unwrap();
    let p = super::super::SourceGeometryPass {
        center: [0.0, 0.0, 1.0],
        eye_direction: [0.0, 0.0, -1.0],
        local_radius_squared: 4.0,
        original_face_index: 0,
    };
    assert_eq!(
        stroke
            .apply_with_state_limit(&mut m, &state, p, 0.5, true, 0)
            .unwrap_err(),
        SourceGeometryError::StorageLimit
    );
    assert_eq!(serde_json::to_value(&m).unwrap(), before);
    assert_eq!(
        m.positions.iter().map(|p| words(*p)).collect::<Vec<_>>(),
        raw
    );
    assert!(stroke.proxy_positions().is_empty());
}

// Additive preregistered W01/W02. All prior raw fixture bodies remain unchanged.
#[test]
fn reserved_workspace_reuses_allocations_and_reaverages_live_mirror() {
    let mut m = tetra();
    let state = SourceGeometryNormalState::seed(&m).unwrap();
    let s = SmoothStencil::build(
        &m,
        &state,
        &[&[0, 1], &[0, 1, 2, 3]],
        state.approximate_bytes(),
    )
    .unwrap();
    let mut workspace =
        SmoothCandidateWorkspace::reserve(&m, &s, 4, state.approximate_bytes()).unwrap();
    let before = (
        workspace.means.as_ptr(),
        workspace.means.capacity(),
        workspace.out.as_ptr(),
        workspace.out.capacity(),
        workspace.bytes(),
    );
    let first = s
        .candidates_into(
            &m,
            state.vertex_normals(),
            &[0, 1],
            0.625,
            false,
            state.approximate_bytes(),
            &mut workspace,
            |_, _| Ok(1.0),
        )
        .unwrap();
    assert_eq!(words(first[0].1), [0x3e555556, 0, 0x3ec00000]);
    assert_eq!(words(first[1].1), [0x3ec00000, 0, 0x3e555556]);
    for &(v, p) in first {
        m.positions[v] = p;
    }
    let mirror = s
        .candidates_into(
            &m,
            state.vertex_normals(),
            &[0, 1, 2, 3],
            0.625,
            false,
            state.approximate_bytes(),
            &mut workspace,
            |_, _| Ok(1.0),
        )
        .unwrap();
    assert_eq!(mirror.len(), 4);
    assert_eq!(words(mirror[0].1)[0], 0x3e200000);
    assert_eq!(words(mirror[1].1)[2], 0x3e200000);
    assert_eq!(
        (
            workspace.means.as_ptr(),
            workspace.means.capacity(),
            workspace.out.as_ptr(),
            workspace.out.capacity(),
            workspace.bytes()
        ),
        before,
    );
}

#[test]
fn oversized_mirror_rejects_before_buffer_clear_callback_or_mesh_change() {
    let m = tetra();
    let (state, s) = stencil(&m, &[0, 1, 2, 3]);
    let before = serde_json::to_vec(&m).unwrap();
    let mut workspace =
        SmoothCandidateWorkspace::reserve(&m, &s, 2, state.approximate_bytes()).unwrap();
    s.candidates_into(
        &m,
        state.vertex_normals(),
        &[0, 1],
        0.625,
        false,
        state.approximate_bytes(),
        &mut workspace,
        |_, _| Ok(1.0),
    )
    .unwrap();
    let means_before = workspace.means.clone();
    let out_before = workspace.out.clone();
    let pointers = (workspace.means.as_ptr(), workspace.out.as_ptr());
    let mut callbacks = 0;
    let result = s.candidates_into(
        &m,
        state.vertex_normals(),
        &[0, 1, 2, 3],
        0.625,
        false,
        state.approximate_bytes(),
        &mut workspace,
        |_, _| {
            callbacks += 1;
            Ok(1.0)
        },
    );
    assert_eq!(result.unwrap_err(), SourceGeometryError::StorageLimit);
    assert_eq!(callbacks, 0);
    assert_eq!(workspace.means, means_before);
    assert_eq!(workspace.out, out_before);
    assert_eq!((workspace.means.as_ptr(), workspace.out.as_ptr()), pointers);
    assert_eq!(serde_json::to_vec(&m).unwrap(), before);
}
