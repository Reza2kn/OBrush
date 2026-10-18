use super::*;
use crate::{MaskConvention, SourceOriginalFaceContact, SourcePickSnapshotAdmission, Vec3};
fn small() -> Arc<Mesh> {
    Arc::new(Mesh::new(
        vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        vec![[0, 1, 2]],
    ))
}
fn ray(x: f64, y: f64) -> SceneRay64 {
    SceneRay64 {
        origin: DVec3::new(x, y, 1.),
        direction: -DVec3::Z,
    }
}
fn prepare(m: &Arc<Mesh>) -> SourcePreparedPickQuery {
    let mut a = SourcePickQueryAdmission::new(Arc::clone(m)).unwrap();
    assert!(a.matches(m));
    while !a.step(16384).unwrap().ready() {}
    a.finish().unwrap()
}
fn seal(m: &Arc<Mesh>) -> ValidatedSourcePickSnapshot {
    let mut a = SourcePickSnapshotAdmission::new(Arc::clone(m)).unwrap();
    while !a.step(16384).unwrap().ready {}
    a.finish().unwrap()
}
fn hit_words(hit: &Option<PreparedSourcePaintHit>) -> serde_json::Value {
    hit.as_ref().map_or(serde_json::Value::Null, |h| serde_json::json!({
        "triangle":h.triangle_index,"face":h.original_face_index,
        "local":h.local_position.map(f64::to_bits),"eye":h.local_eye_direction.map(f64::to_bits),
        "world":h.world_position.map(f64::to_bits)
    }))
}
#[test]
fn q01_small_queries_preserve_old_hits_misses_and_exact_remaining_work() {
    let m = small();
    let old = seal(&m);
    let new = prepare(&m);
    assert!(!new.progress().indexed);
    for r in [ray(0.25, 0.25), ray(2., 2.)] {
        let mut a = SourcePickWorkBudget::default();
        let mut b = SourcePickQueryBudget::default();
        let expected = old.borrow().ray(None, r, &mut a).unwrap();
        let actual = new.ray(None, r, &mut b).unwrap();
        assert_eq!(hit_words(&actual), hit_words(&expected));
        assert_eq!(a, b.attributes);
        assert_eq!(b.remaining_node_certificates(), 4096);
    }
}
#[test]
fn q02_shipped_default65_runs_through_public_dense_adapter_and_exact_old_scalar() {
    let (m, rays) = super::tests::default_input();
    let before = serde_json::to_vec(&*m).unwrap();
    let old = seal(&m);
    let new = prepare(&m);
    assert!(new.progress().indexed);
    assert!(new.progress().index_peak_bytes <= MAX_BYTES);
    if let PreparedKind::Indexed(index) = &new.kind {
        assert_eq!(
            new.progress().index_peak_bytes,
            index.progress.peak_bytes + QUERY_OWNER_CHARGE_BYTES
        );
        assert!(
            new.progress().index_peak_bytes + index.sealed.progress().mesh_storage_bytes
                <= MAX_COMBINED
        );
    } else {
        panic!("default sphere must own a charged dense index");
    }
    assert!(new.progress().index_total_operations > 0);
    for r in rays {
        let mut a = SourcePickWorkBudget {
            face_tests: MAX_FACES,
            triangle_tests: 2 * MAX_FACES,
            corner_reads: 1024,
        };
        let mut b = SourcePickQueryBudget::default();
        let expected = old.borrow().ray(None, r, &mut a).unwrap();
        let actual = new
            .ray(None, r, &mut b)
            .expect("complete65 through public production entry");
        assert_eq!(hit_words(&actual), hit_words(&expected));
        assert!(
            b.remaining_node_certificates() <= 4096
                && b.attributes.face_tests <= 8192
                && b.attributes.triangle_tests <= 8192
        );
    }
    assert_eq!(before, serde_json::to_vec(&*m).unwrap());
}
#[test]
fn q03_layered_validation_and_index_steps_never_exceed_the_call_allowance() {
    let mut m = (*small()).clone();
    m.triangles = vec![[0, 1, 2]; 8193];
    let m = Arc::new(m);
    let mut a = SourcePickQueryAdmission::new(m).unwrap();
    let p = a.step(0).unwrap();
    assert!(!p.ready());
    assert_eq!(p.snapshot.operations, 0);
    let mut indexed = false;
    let mut calls = 0;
    while !a.progress().ready() {
        let p = a.step(17).unwrap();
        assert!(p.snapshot.operations <= 17);
        assert!(p.index_operations <= 17);
        assert!(p.index_peak_bytes <= MAX_BYTES);
        indexed |= p.indexed;
        calls += 1;
        assert!(calls < 20000);
    }
    assert!(indexed);
    assert!(a.finish().unwrap().progress().ready());
}
#[test]
fn q04_dense_local_domain_refusal_is_distinct_from_miss_and_preserves_work() {
    let mut m = (*small()).clone();
    m.triangles = vec![[0, 1, 2]; 8193];
    let dense = prepare(&Arc::new(m));
    let small = prepare(&small());
    let mut pose = ObjectAffineF32V1 {
        local_to_world: DMat4::IDENTITY.to_cols_array().map(|x| x as f32),
        interaction_center: [0.; 3],
    };
    pose.local_to_world[0] = 0.5;
    let outside = ray(500_000.0_f64.next_up(), 0.);
    let mut budget = SourcePickQueryBudget::default();
    let before = budget;
    assert!(matches!(
        dense.ray(Some(&pose), outside, &mut budget),
        Err(SourcePickQueryError::OutsideDenseLocalDomain)
    ));
    assert_eq!(budget.attributes, before.attributes);
    assert_eq!(budget.nodes, before.nodes);
    assert!(small
        .ray(Some(&pose), outside, &mut budget)
        .unwrap()
        .is_none());
    let mut budget = SourcePickQueryBudget::default();
    assert!(dense
        .ray(Some(&pose), ray(500_000., 0.), &mut budget)
        .unwrap()
        .is_none());
}
#[test]
fn q05_dense_exhaustion_after_provisional_work_never_returns_a_partial_hit() {
    let mut m = (*small()).clone();
    m.triangles = vec![[0, 1, 2]; 8193];
    let p = prepare(&Arc::new(m));
    let mut b = SourcePickQueryBudget::default();
    assert!(matches!(
        p.ray(None, ray(0.25, 0.25), &mut b),
        Err(SourcePickQueryError::Ray(
            super::super::SourceSnapshotRayError::WorkBudget
        ))
    ));
    let mut b = SourcePickQueryBudget {
        nodes: 0,
        ..Default::default()
    };
    assert!(matches!(
        p.ray(None, ray(0.25, 0.25), &mut b),
        Err(SourcePickQueryError::Ray(
            super::super::SourceSnapshotRayError::WorkBudget
        ))
    ));
    assert_eq!(b.nodes, 0);
}
#[test]
fn q06_copy_on_write_changes_publication_identity_without_mutating_prepared_channels() {
    let mut m = small();
    let p = prepare(&m);
    let before = serde_json::to_vec(&*m).unwrap();
    Arc::make_mut(&mut m).colors[0] = [0.75, 0.25, 0.5];
    assert!(!p.matches(&m));
    let a = SourcePickQueryAdmission::new(Arc::clone(&m)).unwrap();
    assert!(a.matches(&m));
    let mut other = Arc::clone(&m);
    Arc::make_mut(&mut other).positions[0].x = 0.25;
    assert!(!a.matches(&other));
    let PreparedKind::Linear(old) = &p.kind else {
        panic!("small immutable branch")
    };
    assert_eq!(before, serde_json::to_vec(&*old.mesh).unwrap());
}
#[test]
fn q07_attribute_corner_budget_is_shared_across_samples_and_never_synthesizes_data() {
    let p = prepare(&small());
    let mut b = SourcePickQueryBudget::default();
    b.attributes.corner_reads = 3;
    let c = SourceOriginalFaceContact {
        face_index: 0,
        local_point: [0.25, 0.25, 0.],
    };
    assert!(p.sample_with_budget(c, &mut b).unwrap().is_ok());
    assert_eq!(b.attributes.corner_reads, 0);
    assert!(matches!(
        p.sample_with_budget(c, &mut b),
        Err(SourcePickQueryError::Ray(
            super::super::SourceSnapshotRayError::WorkBudget
        ))
    ));
}
#[test]
fn q08_invalid_vertices_reject_before_index_publication_and_cannot_resume() {
    let mut m = (*small()).clone();
    m.positions[0].x = f32::NAN;
    let mut a = SourcePickQueryAdmission::new(Arc::new(m)).unwrap();
    assert!(matches!(
        a.step(16384),
        Err(SourcePickQueryAdmissionError::Snapshot(
            super::super::SourcePickAdmissionError::Vertex
        ))
    ));
    assert!(matches!(
        a.step(16384),
        Err(SourcePickQueryAdmissionError::NotReady)
    ));
    assert!(matches!(
        a.finish(),
        Err(SourcePickQueryAdmissionError::NotReady)
    ));
}
#[test]
fn q09_sampling_preserves_saved_document_and_existing_redo_exactly() {
    let mut m = (*small()).clone();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let mut d = crate::Document::new(m);
    d.replace_mesh(Mesh::cube()).unwrap();
    let endpoint = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let p = prepare(&Arc::new(d.require_active_mesh().unwrap().snapshot()));
    let mut b = SourcePickQueryBudget::default();
    let h = p.ray(None, ray(0.25, 0.25), &mut b).unwrap().unwrap();
    assert!(p
        .sample_with_budget(
            SourceOriginalFaceContact {
                face_index: h.triangle_index,
                local_point: h.local_position
            },
            &mut b
        )
        .unwrap()
        .is_ok());
    assert_eq!(before, d.to_json().unwrap());
    assert!(d.redo());
    assert_eq!(endpoint, d.to_json().unwrap());
}
#[test]
fn q10_index_face_cap_is_preflighted_and_preparation_cannot_finish_early() {
    let mut m = (*small()).clone();
    m.triangles = vec![[0, 1, 2]; MAX_FACES + 1];
    let m = Arc::new(m);
    let mut a = SourcePickQueryAdmission::new(Arc::clone(&m)).unwrap();
    let error = loop {
        match a.step(16384) {
            Ok(p) => assert!(!p.ready()),
            Err(e) => break e,
        }
    };
    assert_eq!(error, SourcePickQueryAdmissionError::IndexFaces);
    assert!(
        !a.progress().ready(),
        "Rejected preparation must never claim ready"
    );
    assert!(!a.matches(&m));
    assert!(matches!(
        a.finish(),
        Err(SourcePickQueryAdmissionError::NotReady)
    ));
    let a = SourcePickQueryAdmission::new(small()).unwrap();
    assert!(matches!(
        a.finish(),
        Err(SourcePickQueryAdmissionError::NotReady)
    ));
}

#[test]
fn q11_one_level_above_default_completes_fixed65_with_exact_scalar_words() {
    let (_, rays) = super::tests::default_input();
    let m = Arc::new(Mesh::sphere(7));
    assert_eq!(m.positions.len(), 163842);
    assert_eq!(m.triangles.len(), 327680);
    let before = serde_json::to_vec(&*m).unwrap();
    let old = seal(&m);
    let new = prepare(&m);
    assert!(new.progress().indexed);
    assert!(new.progress().index_peak_bytes <= MAX_BYTES);
    assert!(
        new.progress().snapshot.mesh_storage_bytes + new.progress().index_peak_bytes
            <= MAX_COMBINED
    );
    for r in rays {
        let mut authority = SourcePickWorkBudget {
            face_tests: 327680,
            triangle_tests: 655360,
            corner_reads: 1024,
        };
        let expected = old.borrow().ray(None, r, &mut authority).unwrap();
        let mut bounded = SourcePickQueryBudget::default();
        let actual = new
            .ray(None, r, &mut bounded)
            .expect("fixed higher resolution rays must complete within original work caps");
        assert_eq!(hit_words(&actual), hit_words(&expected));
    }
    assert_eq!(before, serde_json::to_vec(&*m).unwrap());
}
