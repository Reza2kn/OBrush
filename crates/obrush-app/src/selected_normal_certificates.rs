//! Private render-only witnesses for immutable shared mesh/normal pairs.
//! This never authorizes Document edits or changes a public core validator.
use super::{Mesh, RenderObject, UploadSource};
use obrush_core::SourceGeometryNormalState;
use std::sync::{Arc, Weak};

const SLOTS: usize = obrush_core::MAX_SUBTOOLS;
const STALE: &str = "Selected-refresh Source render state is stale; explicitly reset it";

#[derive(Clone, Copy)]
pub(super) struct SelectedNormalSeal<'a> {
    mesh: &'a Mesh,
    state: &'a SourceGeometryNormalState,
}
impl SelectedNormalSeal<'_> {
    pub(super) fn packed_rows(self) -> Result<Vec<[f32; 10]>, String> {
        super::packed_geometry_selected_rows_after_validation(self.mesh, self.state)
    }
    pub(super) fn matches(&self, mesh: &Mesh, state: &SourceGeometryNormalState) -> bool {
        std::ptr::eq(self.mesh, mesh) && std::ptr::eq(self.state, state)
    }
}
struct Pair {
    id: u32,
    mesh: Weak<Mesh>,
    state: Weak<SourceGeometryNormalState>,
}
pub(super) struct SelectedNormalCertificates {
    rows: [Option<Pair>; SLOTS],
}
impl Default for SelectedNormalCertificates {
    fn default() -> Self {
        Self {
            rows: std::array::from_fn(|_| None),
        }
    }
}
impl SelectedNormalCertificates {
    // Weak owners keep control allocations, not a mesh's vectors/tree buffers.
    // Charge a conservative maximum even when GPU caches share those owners.
    pub(super) fn storage_bytes() -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of::<[Option<SelectedNormalSeal<'static>>; SLOTS]>()
            + SLOTS
                * (std::mem::size_of::<Mesh>()
                    + std::mem::size_of::<SourceGeometryNormalState>()
                    + 4 * std::mem::size_of::<usize>())
    }
    pub(super) fn retain_visible(&mut self, objects: &[RenderObject<'_>]) {
        for row in &mut self.rows {
            if row
                .as_ref()
                .is_some_and(|pair| !objects.iter().any(|o| o.id == pair.id))
            {
                *row = None;
            }
        }
    }
    pub(super) fn remove(&mut self, id: u32) {
        for row in &mut self.rows {
            if row.as_ref().is_some_and(|pair| pair.id == id) {
                *row = None;
            }
        }
    }
    pub(super) fn clear(&mut self) {
        self.rows = std::array::from_fn(|_| None);
    }
    pub(super) fn certify<'a>(
        &mut self,
        object: &RenderObject<'a>,
    ) -> Result<Option<SelectedNormalSeal<'a>>, String> {
        let Some(state) = object.selected_normals else {
            self.remove(object.id);
            return Ok(None);
        };
        let mesh = match object.source {
            UploadSource::Revision(_) => {
                self.remove(object.id);
                validate(object.mesh, state)?;
                return Ok(Some(SelectedNormalSeal {
                    mesh: object.mesh,
                    state,
                }));
            }
            UploadSource::Shared(mesh) => mesh,
        };
        if !std::ptr::eq(object.mesh, mesh.as_ref()) {
            self.remove(object.id);
            return Err("Shared render mesh does not identify its actual payload".into());
        }
        // A Weak owner keeps its control allocation alive, preventing address
        // reuse. Safe Arc mutation either fails get_mut or dissociates/clones
        // into a different allocation via make_mut; neither can match here.
        if self.rows.iter().flatten().any(|pair| {
            pair.id == object.id
                && pair.mesh.as_ptr() == Arc::as_ptr(mesh)
                && pair.state.as_ptr() == Arc::as_ptr(state)
        }) {
            return Ok(Some(SelectedNormalSeal {
                mesh: object.mesh,
                state,
            }));
        }
        let slot = self
            .rows
            .iter()
            .position(|row| row.as_ref().is_some_and(|p| p.id == object.id))
            .or_else(|| self.rows.iter().position(Option::is_none))
            .ok_or("Selected-normal certificate object limit exceeded")?;
        self.rows[slot] = None;
        validate(object.mesh, state)?;
        self.rows[slot] = Some(Pair {
            id: object.id,
            mesh: Arc::downgrade(mesh),
            state: Arc::downgrade(state),
        });
        Ok(Some(SelectedNormalSeal {
            mesh: object.mesh,
            state,
        }))
    }
}
fn validate(mesh: &Mesh, state: &SourceGeometryNormalState) -> Result<(), String> {
    if super::selected_render_matches_mesh(mesh, state) {
        Ok(())
    } else {
        Err(STALE.into())
    }
}
#[cfg(test)]
pub(super) fn record_validation() {
    VALIDATIONS.with(|n| n.set(n.get() + 1));
}
#[cfg(test)]
std::thread_local! { static VALIDATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Arc<Mesh>, Arc<SourceGeometryNormalState>) {
        let mut mesh = Mesh::sphere(2);
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        let state = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        (Arc::new(mesh), state)
    }
    fn object<'a>(
        mesh: &'a Arc<Mesh>,
        state: &'a Arc<SourceGeometryNormalState>,
    ) -> RenderObject<'a> {
        RenderObject {
            id: 1,
            active: true,
            mesh,
            metadata: None,
            source: UploadSource::Shared(mesh),
            selected_normals: Some(state),
        }
    }
    #[test]
    fn immutable_pair_is_validated_once_for_128_redraws() {
        let (mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        VALIDATIONS.with(|n| n.set(0));
        for _ in 0..128 {
            let proof = cache.certify(&object(&mesh, &state)).unwrap().unwrap();
            assert!(proof.matches(&mesh, &state));
        }
        assert_eq!(
            VALIDATIONS.with(|n| n.get()),
            1,
            "The exact immutable pair must not be rechecked to redraw it"
        );
    }
    #[test]
    fn one_scene_seal_reuses_match_for_storage_and_repeated_packing() {
        let (mesh, state) = fixture();
        let expected = super::super::packed_geometry_selected_rows(&mesh, &state).unwrap();
        let mut cache = SelectedNormalCertificates::default();
        let pbr = super::super::ViewportStyle {
            shading: super::super::ShadingMode::Pbr,
            ..Default::default()
        };
        VALIDATIONS.with(|n| n.set(0));
        for _ in 0..128 {
            let objects = [object(&mesh, &state)];
            let proofs = super::super::scene_source_normal_storage(
                &objects,
                &Default::default(),
                &pbr,
                &mut cache,
            )
            .unwrap();
            let seal = proofs[0].unwrap();
            for _ in 0..3 {
                assert_eq!(seal.packed_rows().unwrap(), expected);
            }
        }
        assert_eq!(VALIDATIONS.with(|n| n.get()), 1);
    }
    #[test]
    fn revision_path_revalidates_on_every_borrow_even_with_same_revision() {
        let (mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        VALIDATIONS.with(|n| n.set(0));
        for _ in 0..8 {
            let mut obj = object(&mesh, &state);
            obj.source = UploadSource::Revision(7);
            assert!(cache.certify(&obj).unwrap().unwrap().matches(&mesh, &state));
        }
        assert_eq!(VALIDATIONS.with(|n| n.get()), 8);
        assert!(cache.rows.iter().all(Option::is_none));
        let mut changed = mesh.as_ref().clone();
        changed.positions[0].x += 0.1;
        let mut obj = object(&mesh, &state);
        obj.mesh = &changed;
        obj.source = UploadSource::Revision(7);
        assert!(cache.certify(&obj).is_err());
    }
    #[test]
    fn weak_owners_block_get_mut_and_make_mut_invalidates_the_old_allocation() {
        let (mut mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        cache.certify(&object(&mesh, &state)).unwrap();
        assert_eq!(Arc::strong_count(&mesh), 1);
        assert!(Arc::get_mut(&mut mesh).is_none());
        let previous = Arc::as_ptr(&mesh);
        Arc::make_mut(&mut mesh).positions[0].x += 0.1;
        assert_ne!(previous, Arc::as_ptr(&mesh));
        assert!(cache.certify(&object(&mesh, &state)).is_err());
        assert!(cache.rows.iter().all(Option::is_none));
        cache.clear();
        assert!(Arc::get_mut(&mut mesh).is_some());
    }
    #[test]
    fn changed_positions_nan_or_face_words_reject_without_publishing_a_new_pair() {
        for mutation in 0..3 {
            let (mut mesh, state) = fixture();
            let mut cache = SelectedNormalCertificates::default();
            cache.certify(&object(&mesh, &state)).unwrap();
            let m = Arc::make_mut(&mut mesh);
            match mutation {
                0 => m.positions[0].x += 0.125,
                1 => m.positions[0].x = f32::from_bits(0x7fc01234),
                _ => m.triangles.swap(0, 1),
            }
            let position_words: Vec<_> = mesh
                .positions
                .iter()
                .map(|p| p.to_array().map(f32::to_bits))
                .collect();
            let faces = mesh.triangles.clone();
            assert!(
                cache
                    .certify(&object(&mesh, &state))
                    .err()
                    .unwrap()
                    .contains("stale")
            );
            assert_eq!(
                mesh.positions
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>(),
                position_words
            );
            assert_eq!(mesh.triangles, faces);
            assert!(cache.rows.iter().all(Option::is_none));
        }
    }
    #[test]
    fn separate_state_with_same_generation_and_same_values_requires_new_validation() {
        let (mesh, old) = fixture();
        let new = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        assert_eq!(old.generation(), new.generation());
        assert!(!Arc::ptr_eq(&old, &new));
        let mut cache = SelectedNormalCertificates::default();
        VALIDATIONS.with(|n| n.set(0));
        let old_seal = cache.certify(&object(&mesh, &old)).unwrap().unwrap();
        assert!(!old_seal.matches(&mesh, &new));
        cache.certify(&object(&mesh, &new)).unwrap();
        assert_eq!(VALIDATIONS.with(|n| n.get()), 2);
    }
    #[test]
    fn shared_payload_and_borrowed_mesh_must_identify_the_same_value() {
        let (mesh, state) = fixture();
        let alternate = mesh.as_ref().clone();
        let mut cache = SelectedNormalCertificates::default();
        cache.certify(&object(&mesh, &state)).unwrap();
        let mut obj = object(&mesh, &state);
        obj.mesh = &alternate;
        assert!(cache.certify(&obj).err().unwrap().contains("payload"));
        assert!(cache.rows.iter().all(Option::is_none));
    }
    #[test]
    fn paint_snapshot_replacement_is_rechecked_and_keeps_new_rgb_words() {
        let (mut mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        VALIDATIONS.with(|n| n.set(0));
        let before = cache
            .certify(&object(&mesh, &state))
            .unwrap()
            .unwrap()
            .packed_rows()
            .unwrap();
        Arc::make_mut(&mut mesh).colors[0] = [0.1, 0.7, 0.2];
        let after = cache
            .certify(&object(&mesh, &state))
            .unwrap()
            .unwrap()
            .packed_rows()
            .unwrap();
        assert_eq!(VALIDATIONS.with(|n| n.get()), 2);
        assert_eq!(&before[0][..7], &after[0][..7]);
        assert_eq!(&after[0][7..], &[0.1, 0.7, 0.2]);
    }
    #[test]
    fn legacy_style_ignores_unneeded_stale_field_but_source_style_rejects_it() {
        let (mesh, state) = fixture();
        let mut changed = mesh.as_ref().clone();
        changed.positions[0].x += 0.1;
        let changed = Arc::new(changed);
        let mut cache = SelectedNormalCertificates::default();
        cache.certify(&object(&mesh, &state)).unwrap();
        let objects = [object(&changed, &state)];
        let p = super::super::scene_source_normal_storage(
            &objects,
            &Default::default(),
            &Default::default(),
            &mut cache,
        )
        .unwrap();
        assert!(p.iter().all(Option::is_none));
        assert!(cache.rows.iter().all(Option::is_none));
        let source = super::super::ViewportStyle {
            shading: super::super::ShadingMode::Pbr,
            ..Default::default()
        };
        assert!(
            super::super::scene_source_normal_storage(
                &objects,
                &Default::default(),
                &source,
                &mut cache
            )
            .is_err()
        );
    }
    #[test]
    fn fixed_slots_and_visible_retirement_bound_cache_and_release_weak_owners() {
        let (mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        for id in 0..SLOTS {
            let mut obj = object(&mesh, &state);
            obj.id = id as u32;
            cache.certify(&obj).unwrap();
        }
        assert_eq!(cache.rows.iter().flatten().count(), SLOTS);
        let mut extra = object(&mesh, &state);
        extra.id = 1000;
        assert!(cache.certify(&extra).err().unwrap().contains("limit"));
        let mut retained = object(&mesh, &state);
        retained.id = 3;
        cache.retain_visible(&[retained]);
        assert_eq!(cache.rows.iter().flatten().count(), 1);
        cache.certify(&extra).unwrap();
        cache.clear();
        assert_eq!(Arc::weak_count(&mesh), 0);
        assert_eq!(Arc::weak_count(&state), 0);
        assert_eq!(Arc::strong_count(&mesh), 1);
        assert_eq!(Arc::strong_count(&state), 1);
        assert!(
            SelectedNormalCertificates::storage_bytes()
                > std::mem::size_of::<SelectedNormalCertificates>()
        );
        assert!(SelectedNormalCertificates::storage_bytes() < 64 * 1024);
    }
    #[test]
    fn certificates_do_not_keep_mesh_or_normal_buffer_owners_alive() {
        let (mesh, state) = fixture();
        let weak_mesh = Arc::downgrade(&mesh);
        let weak_state = Arc::downgrade(&state);
        let mut cache = SelectedNormalCertificates::default();
        cache.certify(&object(&mesh, &state)).unwrap();
        drop(mesh);
        drop(state);
        assert!(weak_mesh.upgrade().is_none());
        assert!(weak_state.upgrade().is_none());
        assert_eq!(cache.rows.iter().flatten().count(), 1);
        cache.retain_visible(&[]);
        assert!(cache.rows.iter().all(Option::is_none));
    }
    #[test]
    fn removing_normals_invalidates_certificate_and_standalone_packing_keeps_validation() {
        let (mut mesh, state) = fixture();
        let mut cache = SelectedNormalCertificates::default();
        cache.certify(&object(&mesh, &state)).unwrap();
        let mut obj = object(&mesh, &state);
        obj.selected_normals = None;
        assert!(cache.certify(&obj).unwrap().is_none());
        assert!(cache.rows.iter().all(Option::is_none));
        Arc::make_mut(&mut mesh).positions[0].x += 0.2;
        assert!(super::super::packed_geometry_selected_rows(&mesh, &state).is_err());
    }
}
