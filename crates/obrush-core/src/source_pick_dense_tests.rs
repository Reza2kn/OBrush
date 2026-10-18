use super::*;
use crate::{SourceOriginalFaceContact, SourcePickSnapshotAdmission, SurfaceTopology, Vec3};
use std::io::Write;

fn mesh() -> Arc<Mesh> {
    Arc::new(Mesh::new(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        vec![[0, 1, 2]],
    ))
}
fn seal(m: Arc<Mesh>) -> ValidatedSourcePickSnapshot {
    let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
    while !a.step(16384).unwrap().ready {}
    a.finish().unwrap()
}
fn build(m: &Arc<Mesh>) -> Index {
    let mut b = Builder::new(seal(Arc::clone(m)), m).unwrap();
    while !b.step(16384).ready {}
    b.finish().unwrap()
}
fn ray(x: f64, y: f64) -> SceneRay64 {
    SceneRay64 {
        origin: DVec3::new(x, y, 1.0),
        direction: -DVec3::Z,
    }
}
fn id(pose: Option<&ObjectAffineF32V1>) -> Identity {
    Identity::new(7, 3, 11, 19, pose)
}
fn words(hit: &Option<Hit>) -> serde_json::Value {
    let w = |x: f64| {
        let b = x.to_bits();
        [b as u32, (b >> 32) as u32]
    };
    hit.as_ref().map_or(serde_json::Value::Null,|h|serde_json::json!({"triangle":h.prepared.triangle_index,"face":h.prepared.original_face_index,"ordinal":h.ordinal,"t":w(h.t),"local":h.prepared.local_position.map(w),"eye":h.prepared.local_eye_direction.map(w),"world":h.prepared.world_position.map(w)}))
}
fn prepared_words(hit: &Option<PreparedSourcePaintHit>) -> serde_json::Value {
    let w = |x: f64| {
        let b = x.to_bits();
        [b as u32, (b >> 32) as u32]
    };
    hit.as_ref().map_or(serde_json::Value::Null,|h|serde_json::json!({"triangle":h.triangle_index,"face":h.original_face_index,"local":h.local_position.map(w),"eye":h.local_eye_direction.map(w),"world":h.world_position.map(w)}))
}
fn compare(
    index: &Index,
    m: &Arc<Mesh>,
    pose: Option<&ObjectAffineF32V1>,
    ray: SceneRay64,
) -> (Option<Hit>, Counts) {
    let mut b = Budget::default();
    let mut c = Counts::default();
    let got = index
        .query(m, id(pose), id(pose), pose, ray, &mut b, &mut c)
        .expect("complete query within fixed budget");
    let expected = linear_authority(m, pose, ray).unwrap();
    assert_eq!(words(&got), words(&expected));
    let mut full = SourcePickWorkBudget {
        face_tests: MAX_FACES,
        triangle_tests: 2 * MAX_FACES,
        corner_reads: 1024,
    };
    let old = index.sealed.borrow().ray(pose, ray, &mut full).unwrap();
    assert_eq!(
        prepared_words(&got.as_ref().map(|h| h.prepared.clone())),
        prepared_words(&old)
    );
    assert!(c.nodes <= 4096 && c.faces <= 8192 && c.triangles <= 8192);
    (got, c)
}
fn overlap(n: usize) -> Arc<Mesh> {
    let mut m = (*mesh()).clone();
    m.triangles = vec![[0, 1, 2]; n];
    Arc::new(m)
}
fn folded_quad() -> Arc<Mesh> {
    let mut m = Mesh::new(
        vec![
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(1., 1., 0.),
            Vec3::new(1., 0., 0.5),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    );
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3]],
        uv: None,
    });
    Arc::new(m)
}
#[test]
fn p01_only_matching_sealed_publication_is_admitted() {
    let m = mesh();
    let other = Arc::new((*m).clone());
    assert!(matches!(
        Builder::new(seal(m), &other),
        Err(BuildError::Unsealed)
    ));
}
#[test]
fn p02_zero_one_and_clamped_build_steps_are_bounded() {
    let m = overlap(17);
    let mut b = Builder::new(seal(Arc::clone(&m)), &m).unwrap();
    assert_eq!(b.step(0).total, 0);
    assert_eq!(b.step(1).operations, 1);
    while !b.progress.ready {
        let p = b.step(usize::MAX);
        assert!(p.operations <= 16384);
    }
    assert!(b.finish().is_ok());
}
#[test]
fn p03_all_phase_rows_bins_and_merges_are_charged() {
    let m = overlap(17);
    let index = build(&m);
    let p = index.progress;
    assert_eq!(p.phases[0], 3);
    assert_eq!(p.phases[1], 17);
    assert_eq!(p.phases[2], 17);
    assert_eq!(p.phases[3], 4 * 256);
    assert_eq!(p.phases[4], 4 * 17);
    assert_eq!(p.phases[5], 4 * 256);
    assert_eq!(p.phases[6], 4 * 17);
    assert_eq!(p.phases[7], 17);
    assert_eq!(p.phases[8], 3);
    assert_eq!(p.phases[9], 3);
    assert_eq!(p.total, p.phases.into_iter().sum::<usize>());
}
#[test]
fn p04_storage_preflight_and_actual_capacities_are_bounded() {
    assert_eq!(checked_bytes(MAX_FACES + 1, 0), Err(BuildError::Faces));
    assert_eq!(checked_bytes(1, MAX_COMBINED), Err(BuildError::Storage));
    assert_eq!(checked_bytes(usize::MAX, 0), Err(BuildError::Faces));
    let m = overlap(17);
    let i = build(&m);
    let caps = i.progress.capacities;
    assert_eq!(
        [i.faces.capacity(), i.order.capacity(), i.nodes.capacity()],
        [caps[0], caps[1], caps[3]]
    );
    let actual =
        std::mem::size_of::<Builder>() + caps[0] * 16 + caps[1] * 4 + caps[2] * 4 + caps[3] * 256;
    assert_eq!(i.progress.peak_bytes, actual);
    assert!(actual < 64 * 1024 * 1024);
}
#[test]
fn p05_copy_on_write_cannot_change_the_ready_index() {
    let mut m = mesh();
    let i = build(&m);
    let before = serde_json::to_vec(&*m).unwrap();
    Arc::make_mut(&mut m).positions[0].x = 0.25;
    let mut b = Budget::default();
    let mut c = Counts::default();
    assert!(matches!(
        i.query(
            &m,
            id(None),
            id(None),
            None,
            ray(0.25, 0.25),
            &mut b,
            &mut c
        ),
        Err(QueryError::Stale)
    ));
    assert_eq!(c.nodes, 0);
    assert_eq!(serde_json::to_vec(&*i.mesh).unwrap(), before);
}
#[test]
fn p06_every_retained_mesh_field_is_bitwise_unedited() {
    let m = folded_quad();
    let before = serde_json::to_vec(&*m).unwrap();
    let i = build(&m);
    compare(&i, &m, None, ray(0.5, 0.25));
    assert_eq!(serde_json::to_vec(&*m).unwrap(), before);
}
#[test]
fn p07_zero_extent_signedzero_and_fixed_morton_axis_words() {
    assert_eq!(morton([0., -0., 0.], [[0., 0.]; 3]), 0);
    assert_eq!(morton([0., 0., 0.], [[0., 1.]; 3]), 0);
    assert_eq!(morton([1., 0., 0.], [[0., 1.]; 3]), 0x0924_9249);
    assert_eq!(morton([1., 1., 1.], [[0., 1.]; 3]), 0x3fff_ffff);
}
#[test]
fn p08_duplicate_keys_stay_stable_and_odd_frontiers_are_balanced() {
    let m = overlap(17);
    let i = build(&m);
    assert_eq!(i.order, (0_u32..17).collect::<Vec<_>>());
    assert_eq!(i.nodes.len(), 5);
    let mut stack = vec![(i.root, 1)];
    let mut depth = 0;
    while let Some((n, d)) = stack.pop() {
        depth = depth.max(d);
        let n = i.nodes[n as usize];
        if n.links[0] != NONE {
            stack.push((n.links[0], d + 1));
            stack.push((n.links[1], d + 1));
        }
    }
    assert!(depth <= 4);
}
#[test]
fn p09_folded_quad_keeps_first_half_even_when_second_is_nearer() {
    let m = folded_quad();
    let i = build(&m);
    let r = SceneRay64 {
        origin: DVec3::new(0.5, 0.25, 2.),
        direction: -DVec3::Z,
    };
    assert_eq!(intersect(&m, r.origin, r.direction, [0, 1, 2]), Some(2.0));
    assert_eq!(intersect(&m, r.origin, r.direction, [0, 2, 3]), Some(1.875));
    let h = compare(&i, &m, None, r).0.unwrap();
    assert_eq!(h.t.to_bits(), 2_f64.to_bits());
    assert_eq!(h.prepared.triangle_index, 0);
    assert_eq!(h.prepared.original_face_index, Some(0));
}
#[test]
fn p10_equal_distance_selects_original_ordinal_despite_spatial_order() {
    let m = Arc::new(Mesh::new(
        vec![
            Vec3::new(-2., -1., 0.),
            Vec3::new(2., -1., 0.),
            Vec3::new(0., 2., 0.),
            Vec3::new(-10., -1., 0.),
            Vec3::new(1., -1., 0.),
            Vec3::new(1., 2., 0.),
        ],
        vec![[0, 1, 2], [3, 4, 5]],
    ));
    let i = build(&m);
    assert_eq!(i.order, [1, 0]);
    assert_eq!(compare(&i, &m, None, ray(0., 0.)).0.unwrap().ordinal, 0);
}
#[test]
fn p11_literal_eps_outside_geometric_box_is_not_pruned() {
    let m = mesh();
    let i = build(&m);
    let h = compare(&i, &m, None, ray(-5e-16, 0.5)).0.unwrap();
    assert_eq!(h.t.to_bits(), 1_f64.to_bits());
    assert!(h.prepared.local_position[0] < 0.);
}
#[test]
fn p12_parallel_signedzero_axis_membership_matches_authority() {
    let m = mesh();
    let i = build(&m);
    for z in [0., -0.] {
        let r = SceneRay64 {
            origin: DVec3::new(-1., 0.25, z),
            direction: DVec3::new(1., 0., -0.),
        };
        compare(&i, &m, None, r);
    }
}
#[test]
fn p13_determinant_eps_neighbors_keep_exact_result() {
    let m = mesh();
    let i = build(&m);
    let eps = 1e-15_f64;
    for (z, accepted) in [
        (f64::from_bits(eps.to_bits() - 1), false),
        (eps, true),
        (f64::from_bits(eps.to_bits() + 1), true),
    ] {
        // 1+z*z rounds to1, so normalizing [1,0,-z] retains determinant z.
        // At t~1 the point is strictly inside (0.25,0.25,0), not an edge test.
        let r = SceneRay64 {
            origin: DVec3::new(-0.75, 0.25, z),
            direction: DVec3::new(1., 0., -z),
        };
        assert_eq!(compare(&i, &m, None, r).0.is_some(), accepted);
    }
}
#[test]
fn p14_signedzero_distance_and_point_words_match_original_scan() {
    let m = overlap(9);
    let i = build(&m);
    for z in [0., -0.] {
        let r = SceneRay64 {
            origin: DVec3::new(0.25, 0.25, z),
            direction: -DVec3::Z,
        };
        let h = compare(&i, &m, None, r).0.unwrap();
        assert_eq!(h.ordinal, 0);
        assert_eq!(h.t.abs(), 0.);
    }
}
#[test]
fn p15_whole_descriptors_cannot_manufacture_a_miss() {
    let m = mesh();
    let mut i = build(&m);
    for field in &mut i.nodes[0].pairs {
        *field = [[f64::NEG_INFINITY, f64::INFINITY]; 3];
    }
    let (h, c) = compare(&i, &m, None, ray(0.25, 0.25));
    assert!(h.is_some());
    assert_eq!(c.unknown, 1);
}
#[test]
fn p16_compact_edge_gap_bound_retains_independent_cancellation_witness() {
    let v = [[1e6, 0., 0.], [1e-20, 0., 0.], [1e6, 1., 0.]];
    let d = Node::pack(Descriptor::triangle(v), [NONE, NONE, 0, 1]).descriptor();
    let cert = certificate(&d, [5e5, 0.25, 1.], [0., 0., -1.]);
    assert_ne!(cert.verdict, Verdict::ProvedMiss);
    let b = cert.envelope.expect("finite cancellation envelope");
    assert!(b[2][0] <= 0. && b[2][1] >= 0.);
}
#[test]
fn p17_parent_hulls_include_fixed_accepted_residual_points() {
    let m = overlap(9);
    let i = build(&m);
    let c = certificate(
        &i.nodes[i.root as usize].descriptor(),
        [-5e-16, 0.5, 1.],
        [0., 0., -1.],
    );
    assert_ne!(c.verdict, Verdict::ProvedMiss);
    let b = c.envelope.unwrap();
    assert!(b[0][0] <= -5e-16 && b[0][1] >= -5e-16);
}
#[test]
fn p18_nonuniform_negative_and_shear_affine_words_match() {
    let m = mesh();
    let i = build(&m);
    for a in [
        [
            2., 0., 0., 0., 0., 0.5, 0., 0., 0., 0., 3., 0., 1., -2., 4., 1.,
        ],
        [
            -1., 0., 0., 0., 0.5, 1., 0., 0., 0., 0., 2., 0., -1., 1., 0., 1.,
        ],
    ] {
        let p = ObjectAffineF32V1 {
            local_to_world: a,
            interaction_center: [0.; 3],
        };
        let mat = DMat4::from_cols_array(&a.map(f64::from));
        let r = SceneRay64 {
            origin: mat.transform_point3(DVec3::new(0.25, 0.25, 1.)),
            direction: mat.transform_vector3(-DVec3::Z),
        };
        compare(&i, &m, Some(&p), r);
    }
}
#[test]
fn p19_old_admission_rejects_invalid_geometry_before_index() {
    let mut m = (*mesh()).clone();
    m.positions[0].x = f32::NAN;
    let mut a = SourcePickSnapshotAdmission::new(Arc::new(m)).unwrap();
    assert!(a.step(1).is_err());
    let mut m = (*mesh()).clone();
    m.triangles[0][2] = 99;
    let mut a = SourcePickSnapshotAdmission::new(Arc::new(m)).unwrap();
    assert!(a.step(16384).is_err());
}
#[test]
fn p20_node_exhaustion_after_provisional_hit_never_returns_partial() {
    let m = overlap(9);
    let i = build(&m);
    let mut b = Budget {
        nodes: 2,
        ..Budget::default()
    };
    let mut c = Counts::default();
    assert!(matches!(
        i.query(
            &m,
            id(None),
            id(None),
            None,
            ray(0.25, 0.25),
            &mut b,
            &mut c
        ),
        Err(QueryError::WorkBudget)
    ));
    assert_eq!(c.faces, 8);
    assert_eq!(c.nodes, 2);
}
#[test]
fn p21_face_exhaustion_has_no_retained_next_query_continuation() {
    let m = overlap(8);
    let i = build(&m);
    let mut b = Budget {
        work: SourcePickWorkBudget {
            face_tests: 4,
            ..SourcePickWorkBudget::default()
        },
        ..Budget::default()
    };
    let mut c = Counts::default();
    assert!(matches!(
        i.query(
            &m,
            id(None),
            id(None),
            None,
            ray(0.25, 0.25),
            &mut b,
            &mut c
        ),
        Err(QueryError::WorkBudget)
    ));
    assert_eq!(c.faces, 4);
    assert_eq!(compare(&i, &m, None, ray(0.25, 0.25)).0.unwrap().ordinal, 0);
}
#[test]
fn p22_quad_charges_two_triangles_before_first_half_evaluation() {
    let m = folded_quad();
    let i = build(&m);
    let mut b = Budget {
        work: SourcePickWorkBudget {
            triangle_tests: 1,
            ..SourcePickWorkBudget::default()
        },
        ..Budget::default()
    };
    let mut c = Counts::default();
    assert!(matches!(
        i.query(&m, id(None), id(None), None, ray(0.5, 0.25), &mut b, &mut c),
        Err(QueryError::WorkBudget)
    ));
    assert_eq!(c.triangles, 0);
    assert_eq!(b.work.triangle_tests, 1);
}
#[test]
fn p23_corner_sampling_uses_unchanged_borrowed_api_and_budget() {
    let m = mesh();
    let i = build(&m);
    let h = compare(&i, &m, None, ray(0.25, 0.25)).0.unwrap();
    let contact = SourceOriginalFaceContact {
        face_index: h.ordinal,
        local_point: h.prepared.local_position,
    };
    let mut b = SourcePickWorkBudget {
        corner_reads: 2,
        ..SourcePickWorkBudget::default()
    };
    let before = serde_json::to_vec(&*m).unwrap();
    assert!(i
        .sealed
        .borrow()
        .sample_with_budget(contact, &mut b)
        .is_err());
    assert_eq!(b.corner_reads, 2);
    assert_eq!(serde_json::to_vec(&*m).unwrap(), before);
}
#[test]
fn p24_shared_batch_budget_preserves_complete_prefix_then_errors() {
    let m = overlap(8);
    let i = build(&m);
    let mut b = Budget {
        work: SourcePickWorkBudget {
            face_tests: 8,
            ..SourcePickWorkBudget::default()
        },
        ..Budget::default()
    };
    let mut c = Counts::default();
    let first = i
        .query(
            &m,
            id(None),
            id(None),
            None,
            ray(0.25, 0.25),
            &mut b,
            &mut c,
        )
        .unwrap();
    assert!(first.is_some());
    let miss = i
        .query(&m, id(None), id(None), None, ray(2., 2.), &mut b, &mut c)
        .unwrap();
    assert!(miss.is_none());
    let prefix = words(&first);
    assert!(i
        .query(&m, id(None), id(None), None, ray(0.5, 0.1), &mut b, &mut c)
        .is_err());
    assert_eq!(words(&first), prefix);
}
#[test]
fn p25_revision_part_epoch_frame_and_pose_mismatch_reject_before_work() {
    let m = mesh();
    let i = build(&m);
    let captured = id(None);
    for current in [
        Identity {
            revision: 8,
            ..captured
        },
        Identity {
            part: 4,
            ..captured
        },
        Identity {
            epoch: 12,
            ..captured
        },
        Identity {
            frame: 20,
            ..captured
        },
    ] {
        let mut b = Budget::default();
        let mut c = Counts::default();
        assert!(matches!(
            i.query(&m, captured, current, None, ray(0.25, 0.25), &mut b, &mut c),
            Err(QueryError::Stale)
        ));
        assert_eq!(c.nodes, 0);
    }
    let mut pose = ObjectAffineF32V1 {
        local_to_world: DMat4::IDENTITY.to_cols_array().map(|x| x as f32),
        interaction_center: [0.; 3],
    };
    pose.local_to_world[12] = 1.;
    let mut b = Budget::default();
    let mut c = Counts::default();
    assert!(matches!(
        i.query(
            &m,
            captured,
            captured,
            Some(&pose),
            ray(0.25, 0.25),
            &mut b,
            &mut c
        ),
        Err(QueryError::Stale)
    ));
}
#[test]
fn p26_existing_linear_upfront_and_dense_refusal_policy_stay_exact() {
    let m = overlap(9);
    let s = seal(m);
    let mut b = SourcePickWorkBudget {
        face_tests: 8,
        ..SourcePickWorkBudget::default()
    };
    assert!(s.borrow().ray(None, ray(0.25, 0.25), &mut b).is_err());
    assert_eq!(b.face_tests, 8);
    let m = overlap(8193);
    assert!(!seal(m).complete_query_supported());
}
#[test]
fn p27_dense_cluster_and_original_quad_fields_match_complete_authority() {
    let mut points = Vec::new();
    let mut triangles = Vec::new();
    let mut faces = Vec::new();
    let mut uv = Vec::new();
    for q in 0..4097_u32 {
        let base = q * 4;
        let x = q as f32 * 4.;
        points.extend([
            Vec3::new(x, 0., 0.),
            Vec3::new(x + 1., 0., 0.),
            Vec3::new(x + 1., 1., 0.),
            Vec3::new(x, 1., 0.),
        ]);
        triangles.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
        faces.push([base, base + 1, base + 2, base + 3]);
        uv.extend([[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
    }
    let mut m = Mesh::new(points, triangles);
    m.surface = Some(SurfaceTopology {
        faces: faces.clone(),
        uv: Some(crate::CornerUv {
            coordinates: uv,
            face_indices: faces,
        }),
    });
    let m = Arc::new(m);
    let i = build(&m);
    let before = serde_json::to_vec(&*m).unwrap();
    compare(&i, &m, None, ray(0.25, 0.25));
    compare(&i, &m, None, ray(2., 2.));
    assert_eq!(serde_json::to_vec(&*m).unwrap(), before);
}

pub(super) fn default_input() -> (Arc<Mesh>, Vec<SceneRay64>) {
    let bytes = include_bytes!("../tests/fixtures/source-pick-default65.json");
    assert!(bytes.len() <= 32 * 1024 * 1024);
    let v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(v["vertices"], 40962);
    assert_eq!(v["triangles"], 81920);
    let triples = |name: &str| {
        v[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                std::array::from_fn(|i| {
                    f32::from_bits(row[i].as_u64().unwrap().try_into().unwrap())
                })
            })
            .collect::<Vec<[f32; 3]>>()
    };
    let positions = triples("positions_u32")
        .into_iter()
        .map(Vec3::from_array)
        .collect();
    let triangles = v["triangles_u32"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| std::array::from_fn(|i| r[i].as_u64().unwrap().try_into().unwrap()))
        .collect();
    let mut m = Mesh::new(positions, triangles);
    m.normals = triples("normals_u32")
        .into_iter()
        .map(Vec3::from_array)
        .collect();
    // Keep the historical independent geometry/rays/attributes unchanged.
    // Current default Legacy normals use the declared portable angle policy.
    // Every replaced old word is checked before installing its versioned row;
    // the complete fresh-constructor word equality below remains mandatory.
    let portable_bytes =
        include_bytes!("../tests/fixtures/source-pick-default65-portable-normal-words.json");
    assert!(portable_bytes.len() <= 64 * 1024);
    let portable: serde_json::Value = serde_json::from_slice(portable_bytes).unwrap();
    assert_eq!(
        portable["base_sha256"],
        "1ffb55f1a72e505cf4de24179503964f9863c887cfa6e7e9a150b97ae7127094"
    );
    assert_eq!(portable["policy"], "portable_corner_angle_libm_0_2_16_v1");
    let mut seen = std::collections::HashSet::new();
    for row in portable["normal_rows"].as_array().unwrap() {
        let id = row["vertex"].as_u64().unwrap() as usize;
        assert!(seen.insert(id));
        let before: [u32; 3] =
            std::array::from_fn(|i| row["before"][i].as_u64().unwrap().try_into().unwrap());
        let after: [u32; 3] =
            std::array::from_fn(|i| row["after"][i].as_u64().unwrap().try_into().unwrap());
        assert_ne!(before, after);
        assert_eq!(m.normals[id].to_array().map(f32::to_bits), before);
        m.normals[id] = Vec3::from_array(after.map(f32::from_bits));
    }
    m.colors = triples("rgb_u32");
    m.masks = v["mask_u32"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| f32::from_bits(x.as_u64().unwrap().try_into().unwrap()))
        .collect();
    let rays = v["rays_u64"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| SceneRay64 {
            origin: DVec3::from_array(std::array::from_fn(|i| {
                f64::from_bits(r[0][i].as_u64().unwrap())
            })),
            direction: DVec3::from_array(std::array::from_fn(|i| {
                f64::from_bits(r[1][i].as_u64().unwrap())
            })),
        })
        .collect::<Vec<_>>();
    assert_eq!(rays.len(), 65);
    assert_eq!(m.positions.len(), 40962);
    assert_eq!(m.triangles.len(), 81920);
    // Use actual default-constructor capacities after exact input-word agreement.
    // This fixture preparation never runs inside admission or any query.
    let fresh = Mesh::sphere(6);
    assert_eq!(fresh.positions.len(), 40962);
    assert_eq!(fresh.triangles, m.triangles);
    assert!(fresh
        .positions
        .iter()
        .zip(&m.positions)
        .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits)));
    assert!(fresh
        .normals
        .iter()
        .zip(&m.normals)
        .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits)));
    assert!(fresh
        .colors
        .iter()
        .zip(&m.colors)
        .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits)));
    assert!(fresh
        .masks
        .iter()
        .zip(&m.masks)
        .all(|(a, b)| a.to_bits() == b.to_bits()));
    assert_eq!(fresh.normals.len(), m.normals.len());
    assert_eq!(fresh.colors.len(), m.colors.len());
    assert_eq!(fresh.masks.len(), m.masks.len());
    (Arc::new(fresh), rays)
}
fn save_record(name: &str, value: serde_json::Value) {
    let Some(dir) = std::env::var_os("OBRUSH_DENSE_QUERY_EVIDENCE_DIR") else {
        assert!(serde_json::to_vec(&value).unwrap().len() <= 64 * 1024);
        return; // Only optional raw tracing; all fixed comparisons below remain mandatory.
    };
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(std::path::Path::new(&dir).join(name))
        .unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(bytes.len() <= 64 * 1024);
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
}
#[test]
fn p28_actual_default65_all_complete_exact_words_within_caps() {
    let (m, rays) = default_input();
    let before = serde_json::to_vec(&*m).unwrap();
    let i = build(&m);
    assert_eq!(i.nodes.len(), 20479);
    assert!(i.progress.peak_bytes < 13 * 1024 * 1024);
    let mut failed = Vec::with_capacity(65);
    for (n, r) in rays.into_iter().enumerate() {
        let expected = linear_authority(&m, None, r).unwrap();
        let mut b = Budget::default();
        let mut c = Counts::default();
        let got = i.query(&m, id(None), id(None), None, r, &mut b, &mut c);
        let mut full = SourcePickWorkBudget {
            face_tests: MAX_FACES,
            triangle_tests: 2 * MAX_FACES,
            corner_reads: 1024,
        };
        let actual_old = i.sealed.borrow().ray(None, r, &mut full).unwrap();
        save_record(
            &format!("default65-{n:02}.json"),
            serde_json::json!({"case":n,"actual":got.as_ref().ok().map(words),"error":got.as_ref().err().map(|e|format!("{e:?}")),"expected":words(&expected),"actual_old":prepared_words(&actual_old),"nodes":c.nodes,"faces":c.faces,"triangles":c.triangles,"proved_miss":c.proved_miss,"possible":c.possible,"unknown":c.unknown,"distance_pruned":c.distance_pruned,"build_operations":i.progress.total,"build_phase_counts":i.progress.phases,"peak_bytes":i.progress.peak_bytes,"actual_capacities":i.progress.capacities,"actual_mesh_storage_bytes":i.sealed.progress().mesh_storage_bytes}),
        );
        let passed = got.as_ref().is_ok_and(|h| {
            words(h) == words(&expected)
                && prepared_words(&h.as_ref().map(|h| h.prepared.clone()))
                    == prepared_words(&actual_old)
        }) && c.nodes <= 4096
            && c.faces <= 8192
            && c.triangles <= 8192;
        if !passed {
            failed.push(n);
        }
    }
    assert_eq!(serde_json::to_vec(&*m).unwrap(), before);
    assert!(
        failed.is_empty(),
        "mandatory65 completion/word gates failed: {failed:?}; WorkBudget never passes"
    );
}
#[test]
fn p29_default_dense_off_sphere_miss_also_completes() {
    let (m, _) = default_input();
    let i = build(&m);
    let r = SceneRay64 {
        origin: DVec3::new(2., 2., 3.),
        direction: -DVec3::Z,
    };
    let mut b = Budget::default();
    let mut c = Counts::default();
    let hit = i.query(&m, id(None), id(None), None, r, &mut b, &mut c);
    let expected = linear_authority(&m, None, r).unwrap();
    save_record(
        "default-off-sphere.json",
        serde_json::json!({"actual":hit.as_ref().ok().map(words),"error":hit.as_ref().err().map(|e|format!("{e:?}")),"expected":words(&expected),"nodes":c.nodes,"faces":c.faces,"triangles":c.triangles}),
    );
    let hit = hit.expect("default miss must complete");
    assert_eq!(words(&hit), words(&expected));
    assert!(hit.is_none());
    assert!(c.nodes <= 4096 && c.faces <= 8192 && c.triangles <= 8192);
}
#[test]
fn p30_unprunable_dense_overlap_is_explicit_refusal() {
    let m = overlap(100_000);
    let i = build(&m);
    let mut b = Budget::default();
    let mut c = Counts::default();
    assert!(matches!(
        i.query(
            &m,
            id(None),
            id(None),
            None,
            ray(0.25, 0.25),
            &mut b,
            &mut c
        ),
        Err(QueryError::WorkBudget)
    ));
    assert!(c.nodes <= 4096 && c.faces <= 8192 && c.triangles <= 8192);
}
#[test]
fn p31_fixed_holders_stack_nodes_and_query_allocations_are_bounded() {
    assert_eq!(std::mem::size_of::<Node>(), 256);
    assert_eq!(std::mem::size_of::<Face>(), 16);
    assert!(std::mem::size_of::<Builder>() <= 4096);
    assert!(std::mem::size_of::<Index>() <= 4096);
    assert_eq!(std::mem::size_of::<[u32; 64]>(), 256);
    let m = mesh();
    let i = build(&m);
    let caps = (i.faces.capacity(), i.order.capacity(), i.nodes.capacity());
    compare(&i, &m, None, ray(0.25, 0.25));
    assert_eq!(
        caps,
        (i.faces.capacity(), i.order.capacity(), i.nodes.capacity())
    );
}
#[test]
fn p32_default_legacy_kernel_and_old_scope_are_not_replaced() {
    let m = mesh();
    let before = serde_json::to_vec(&*m).unwrap();
    let i = build(&m);
    assert!(i.sealed.complete_query_supported());
    assert!(i.sealed.matches(&m));
    compare(&i, &m, None, ray(0.25, 0.25));
    assert_eq!(serde_json::to_vec(&*m).unwrap(), before);
}

#[test]
fn p33_expanded_face_ceiling_retains_actual_storage_limits() {
    let (_, _, peak) = checked_bytes(MAX_FACES, 0).unwrap();
    assert!(peak + QUERY_OWNER_CHARGE_BYTES <= MAX_BYTES);
    assert!(checked_bytes(MAX_FACES, MAX_COMBINED - peak + 1).is_err());
    assert_eq!(checked_bytes(MAX_FACES + 1, 0), Err(BuildError::Faces));
    assert_eq!(MAX_BYTES, 64 * 1024 * 1024);
    assert_eq!(MAX_COMBINED, 128 * 1024 * 1024);
}

#[test]
fn p34_browser_oblique_large_ray_completes_exact_scalar_with_fixed_budget() {
    let m = Arc::new(Mesh::sphere(7));
    let index = build(&m);
    let projection = DMat4::from_cols_array(&[
        2.1979820728302,
        0.,
        0.,
        0.,
        0.,
        2.7474775314331055,
        0.,
        0.,
        0.,
        0.,
        -1.0000200271606445,
        -1.,
        0.,
        0.,
        -0.016717787832021713,
        0.,
    ]);
    let view = DMat4::from_translation(DVec3::new(-1.125, 0., -8.358809471130371));
    let capture = crate::PaintViewCapture {
        world_to_clip: (projection * view).to_cols_array(),
        viewport: [225., 57., 1140., 912.],
    };
    let r = capture.ray([627., 513.]).unwrap();
    let expected = linear_authority(&m, None, r).unwrap();
    assert!(expected.is_some(), "Fixed actual browser contact is useful");
    let mut budget = Budget::default();
    let mut counts = Counts::default();
    let actual = index.query(&m, id(None), id(None), None, r, &mut budget, &mut counts);
    save_record(
        "large-oblique-browser.json",
        serde_json::json!({"ray_origin":r.origin.to_array(),"ray_direction":r.direction.to_array(),"nodes":counts.nodes,"faces":counts.faces,"unknown":counts.unknown,"proved_miss":counts.proved_miss,"distance_pruned":counts.distance_pruned,"error":actual.as_ref().err().map(|e|format!("{e:?}")),"expected":words(&expected),"actual":actual.as_ref().ok().map(words)}),
    );
    assert_eq!(
        words(&actual.expect("Actual oblique browser ray must complete within original work cap")),
        words(&expected)
    );
}

#[test]
fn p35_fixed_oblique_nine_ray_grid_completes_exact_scalar_with_fixed_budget() {
    let m = Arc::new(Mesh::sphere(7));
    let index = build(&m);
    let projection = DMat4::from_cols_array(&[
        2.1979820728302,
        0.,
        0.,
        0.,
        0.,
        2.7474775314331055,
        0.,
        0.,
        0.,
        0.,
        -1.0000200271606445,
        -1.,
        0.,
        0.,
        -0.016717787832021713,
        0.,
    ]);
    let view = DMat4::from_translation(DVec3::new(-1.125, 0., -8.358809471130371));
    let capture = crate::PaintViewCapture {
        world_to_clip: (projection * view).to_cols_array(),
        viewport: [225., 57., 1140., 912.],
    };
    let mut failures = Vec::new();
    for x in [566., 626., 686.] {
        for y in [453., 513., 573.] {
            let r = capture.ray([x, y]).unwrap();
            let expected = linear_authority(&m, None, r).unwrap();
            assert!(
                expected.is_some(),
                "Independent fixed grid stays within sphere"
            );
            let mut budget = Budget::default();
            let mut counts = Counts::default();
            let actual = index.query(&m, id(None), id(None), None, r, &mut budget, &mut counts);
            save_record(
                &format!("large-grid-{}-{}.json", x as u32, y as u32),
                serde_json::json!({"screen":[x,y],"nodes":counts.nodes,"faces":counts.faces,"unknown":counts.unknown,"error":actual.as_ref().err().map(|e|format!("{e:?}")),"actual":actual.as_ref().ok().map(words),"expected":words(&expected)}),
            );
            if !actual.as_ref().is_ok_and(|h| words(h) == words(&expected)) {
                failures.push((x, y, counts.nodes, counts.faces));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "All fixed grid rays must complete: {failures:?}"
    );
}
