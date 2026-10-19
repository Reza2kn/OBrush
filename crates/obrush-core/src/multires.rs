//! Persistent Loop and source-static levels with versioned detail transport.
//! Fine edits do not reverse-project into coarse levels; source-static v3
//! rotates fine residuals in the pinned normal/tangent frame. HD geometry is
//! a separate target.
use crate::{Mesh, MAX_TRIANGLES, MAX_VERTICES};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::sync::Arc;

pub const MAX_SUBDIVISION_LEVELS: usize = 8;

#[derive(Debug)]
pub(crate) struct Multiresolution {
    pub(crate) active: usize,
    pub(crate) scheme: SubdivisionScheme,
    /// The active slot is its last committed baseline. Keeping that baseline
    /// lets public active-mesh edits propagate without requiring brush hooks.
    pub(crate) levels: Vec<Mesh>,
    /// Derived topology-only maps. Arc sharing avoids rebuilding them for
    /// every dab/history snapshot; they are reconstructed after project load.
    ring_maps: Vec<Arc<crate::source_ring::SourceStaticRingMap>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SubdivisionScheme {
    LoopWorldOffsetsV1,
    SculptglStaticSmoothV1,
    SculptglStaticLinearEdgesV1,
    SculptglStaticSmoothV2,
    SculptglStaticLinearEdgesV2,
    SculptglStaticSmoothV3,
    SculptglStaticLinearEdgesV3,
    /// Reverse-retained per-level UV IDs may refine canonical child aliases.
    SculptglStaticSmoothV4,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MultiresProject {
    pub(crate) scheme: SubdivisionScheme,
    pub(crate) active_level: usize,
    #[serde(default, deserialize_with = "deserialize_source_root_order")]
    pub(crate) source_root_order: Option<Vec<[u32; 2]>>,
    /// Exactly the active slot is null: the top-level mesh supplies that slot.
    #[serde(deserialize_with = "deserialize_levels")]
    pub(crate) levels: Vec<Option<Mesh>>,
}

#[derive(Serialize)]
pub(crate) struct MultiresProjectRef<'a> {
    scheme: SubdivisionScheme,
    active_level: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_root_order: Option<&'a [[u32; 2]]>,
    levels: Vec<Option<&'a Mesh>>,
}

impl Multiresolution {
    /// Construct Source's default primitive hierarchy as one candidate. The
    /// generator keeps the last four levels, including the visible fine mesh,
    /// without publishing any intermediate document or history state.
    pub(crate) fn source_primitive(
        base: &Mesh,
        settings: &crate::StaticSubdivisionSettings,
        subdivisions: usize,
        translation: crate::Vec3,
    ) -> Result<Self, String> {
        if subdivisions == 0 || subdivisions >= MAX_SUBDIVISION_LEVELS {
            return Err("Source primitive subdivision count is out of range".into());
        }
        if !translation.is_finite() {
            return Err("Source primitive placement must be finite".into());
        }
        let mut stack = Self::first_static_source(base, settings)?;
        while stack.active < subdivisions {
            let active = &stack.levels[stack.active];
            stack = stack.added(active)?;
        }
        let start = stack.levels.len().saturating_sub(4);
        if start != 0 {
            stack.retain_range(start, stack.levels.len())?;
        }
        if translation != crate::Vec3::ZERO {
            for level in &mut stack.levels {
                for point in &mut level.positions {
                    *point += translation;
                }
            }
        }
        stack.validate_budget()?;
        Ok(stack)
    }

    /// Transform an already reconciled hierarchy, making the active baseline
    /// exactly match the visible composed surface to avoid later delta drift.
    pub(crate) fn transformed(
        &self,
        affine: &crate::transform::Affine,
        source_active: &Mesh,
        visible: &Mesh,
    ) -> Result<Self, String> {
        self.validate_budget()?;
        let mut levels = Vec::new();
        levels
            .try_reserve_exact(self.levels.len())
            .map_err(|_| "Cannot allocate transformed subdivision levels")?;
        for (index, level) in self.levels.iter().enumerate() {
            if index == self.active {
                affine.validate_faces(
                    &source_active.positions,
                    &visible.positions,
                    &visible.triangles,
                )?;
                levels.push(visible.snapshot());
            } else {
                levels.push(affine.mesh(level)?);
            }
        }
        let candidate = Self {
            active: self.active,
            scheme: self.scheme,
            levels,
            ring_maps: self.ring_maps.clone(),
        };
        candidate.validate_budget()?;
        Ok(candidate)
    }
    pub(crate) fn first(base: &Mesh) -> Result<Self, String> {
        let mut child = base.snapshot();
        child.subdivide_smooth()?;
        let stack = Self {
            active: 1,
            scheme: SubdivisionScheme::LoopWorldOffsetsV1,
            levels: vec![base.snapshot(), child],
            ring_maps: Vec::new(),
        };
        stack.validate_budget()?;
        Ok(stack)
    }

    pub(crate) fn first_static(
        base: &Mesh,
        settings: &crate::StaticSubdivisionSettings,
    ) -> Result<Self, String> {
        let scheme = if settings.linear_edges {
            SubdivisionScheme::SculptglStaticLinearEdgesV1
        } else {
            SubdivisionScheme::SculptglStaticSmoothV1
        };
        let child = base.subdivided_static(settings)?.mesh;
        let stack = Self {
            active: 1,
            scheme,
            levels: vec![base.snapshot(), child],
            ring_maps: Vec::new(),
        };
        stack.validate_budget()?;
        Ok(stack)
    }
    /// New studio hierarchies use source-style even-vertex analysis on Lower.
    /// The old scheme IDs retain their original world-offset navigation so
    /// existing .obrush projects are not silently reinterpreted.
    pub(crate) fn first_static_source(
        base: &Mesh,
        settings: &crate::StaticSubdivisionSettings,
    ) -> Result<Self, String> {
        let mut stack = Self::first_static(base, settings)?;
        stack.scheme = if settings.linear_edges {
            SubdivisionScheme::SculptglStaticLinearEdgesV3
        } else {
            SubdivisionScheme::SculptglStaticSmoothV3
        };
        stack
            .ring_maps
            .push(Arc::new(crate::source_ring::SourceStaticRingMap::build(
                &stack.levels[0],
                &stack.levels[1],
            )?));
        stack.validate_budget()?;
        Ok(stack)
    }

    /// Recover a new lower level from an imported fine mesh. The structural
    /// solver supplies an explicit predicted-to-imported correspondence; the
    /// retained fine slot is reordered into the normal static child layout so
    /// later Lower/Higher operations use the same validated transport path.
    pub(crate) fn reversed_first_static_source(
        fine: &Mesh,
        settings: &crate::ReverseSubdivisionSettings,
    ) -> Result<(Self, crate::ReverseSubdivisionReport, Vec<u32>), String> {
        let outcome = fine.reversed_static(settings)?;
        let coarse = outcome.coarse;
        let mut canonical = coarse
            .subdivided_static(&crate::StaticSubdivisionSettings {
                linear_edges: false,
            })?
            .mesh;
        let map = &outcome.correspondence.predicted_vertex_to_fine;
        if map.len() != canonical.positions.len() || map.len() != fine.positions.len() {
            return Err("Reverse fine-vertex correspondence changed size".into());
        }
        canonical.positions = map.iter().map(|&id| fine.positions[id as usize]).collect();
        canonical.normals = map.iter().map(|&id| fine.normals[id as usize]).collect();
        canonical.colors = map.iter().map(|&id| fine.colors[id as usize]).collect();
        canonical.masks = map.iter().map(|&id| fine.masks[id as usize]).collect();
        canonical.mask_convention = fine.mask_convention;
        canonical.materials = fine
            .materials
            .as_ref()
            .map(|materials| map.iter().map(|&id| materials[id as usize]).collect());
        let mut retained_source_uv_ids = false;
        if let Some(uv) = fine
            .surface
            .as_ref()
            .and_then(|surface| surface.uv.as_ref())
        {
            let faces = &canonical
                .surface
                .as_ref()
                .ok_or("Reverse fine faces are missing")?
                .faces;
            if outcome.correspondence.predicted_face_to_fine.len() != faces.len() {
                return Err("Reverse fine-face correspondence changed size".into());
            }
            let mut face_indices = Vec::new();
            face_indices
                .try_reserve_exact(faces.len())
                .map_err(|_| "Cannot allocate reversed fine UV faces")?;
            for (face_id, face) in faces.iter().enumerate() {
                let original = outcome.correspondence.predicted_face_to_fine[face_id] as usize;
                let rotation = usize::from(outcome.correspondence.predicted_face_rotation[face_id]);
                let source = uv.face_indices[original];
                let count = if face[3] == crate::TRIANGLE_CORNER {
                    3
                } else {
                    4
                };
                let mut row = [crate::TRIANGLE_CORNER; 4];
                for corner in 0..count {
                    row[corner] = source[(rotation + corner) % count];
                }
                face_indices.push(row);
            }
            let expected = canonical
                .surface
                .as_ref()
                .and_then(|surface| surface.uv.as_ref())
                .ok_or("Reverse fine UV prediction is missing")?;
            let retained = crate::CornerUv {
                coordinates: uv.coordinates.clone(),
                face_indices,
            };
            if !crate::attributes::uv_corner_refinement_matches(expected, &retained) {
                return Err("Reverse cannot retain the original fine UV coordinate IDs and corners in a serializable level; source kept unchanged".into());
            }
            retained_source_uv_ids = expected != &retained;
            canonical
                .surface
                .as_mut()
                .ok_or("Reverse fine faces are missing")?
                .uv = Some(retained);
        }
        canonical.validate()?;
        let ring = crate::source_ring::SourceStaticRingMap::build(&coarse, &canonical)?;
        let stack = Self {
            active: 0,
            scheme: if retained_source_uv_ids {
                SubdivisionScheme::SculptglStaticSmoothV4
            } else {
                SubdivisionScheme::SculptglStaticSmoothV3
            },
            levels: vec![coarse, canonical],
            ring_maps: vec![Arc::new(ring)],
        };
        stack.validate_budget()?;
        Ok((stack, outcome.report, map.clone()))
    }

    /// Prepend another recovered level while retaining every existing fine
    /// level. Each child is structurally renumbered from its parent mapping;
    /// edits and channels follow vertex origins rather than spatial proximity.
    pub(crate) fn reversed_again_static_source(
        &self,
        active_mesh: &Mesh,
        settings: &crate::ReverseSubdivisionSettings,
    ) -> Result<(Self, crate::ReverseSubdivisionReport), String> {
        if self.active != 0
            || !matches!(
                self.scheme,
                SubdivisionScheme::SculptglStaticSmoothV3
                    | SubdivisionScheme::SculptglStaticSmoothV4
            )
        {
            return Err("Reverse requires the lowest source-static smooth level".into());
        }
        if self.levels.len() >= MAX_SUBDIVISION_LEVELS {
            return Err(format!(
                "At most {MAX_SUBDIVISION_LEVELS} subdivision levels are supported"
            ));
        }
        let old = self.reconciled(active_mesh)?;
        let (mut stack, report, mut vertex_to_old) =
            Self::reversed_first_static_source(&old.levels[0], settings)?;
        for level in 1..old.levels.len() {
            let (child, next_mapping) = crate::static_subdivision::rebase_static_child(
                &old.levels[level - 1],
                &old.levels[level],
                stack.levels.last().expect("Reverse stack has a parent"),
                &vertex_to_old,
            )?;
            let parent = stack.levels.last().expect("Reverse stack has a parent");
            let ring = match stack.ring_maps.last() {
                Some(previous) => {
                    crate::source_ring::SourceStaticRingMap::build_after(parent, &child, previous)?
                }
                None => crate::source_ring::SourceStaticRingMap::build(parent, &child)?,
            };
            stack.ring_maps.push(Arc::new(ring));
            stack.levels.push(child);
            vertex_to_old = next_mapping;
        }
        if self.scheme == SubdivisionScheme::SculptglStaticSmoothV4
            || stack
                .levels
                .iter()
                .any(|mesh| mesh.uv_coordinate_count() != 0)
        {
            stack.scheme = SubdivisionScheme::SculptglStaticSmoothV4;
        }
        stack.validate_budget()?;
        Ok((stack, report))
    }
    pub(crate) fn static_settings_match(
        &self,
        settings: &crate::StaticSubdivisionSettings,
    ) -> bool {
        matches!(
            (self.scheme, settings.linear_edges),
            (
                SubdivisionScheme::SculptglStaticSmoothV1
                    | SubdivisionScheme::SculptglStaticSmoothV2
                    | SubdivisionScheme::SculptglStaticSmoothV3
                    | SubdivisionScheme::SculptglStaticSmoothV4,
                false
            ) | (
                SubdivisionScheme::SculptglStaticLinearEdgesV1
                    | SubdivisionScheme::SculptglStaticLinearEdgesV2
                    | SubdivisionScheme::SculptglStaticLinearEdgesV3,
                true
            )
        )
    }
    pub(crate) fn added(&self, active_mesh: &Mesh) -> Result<Self, String> {
        if self.active + 1 != self.levels.len() {
            return Err("Select the highest subdivision level before adding another".into());
        }
        if self.levels.len() >= MAX_SUBDIVISION_LEVELS {
            return Err(format!(
                "At most {MAX_SUBDIVISION_LEVELS} subdivision levels are supported"
            ));
        }
        let mut stack = self.reconciled(active_mesh)?;
        let mut child = stack
            .levels
            .last()
            .expect("a nonempty validated level stack")
            .snapshot();
        self.scheme.subdivide(&mut child)?;
        if self.scheme.uses_source_frame() {
            let parent = stack
                .levels
                .last()
                .expect("a nonempty validated level stack");
            let ring = match stack.ring_maps.last() {
                Some(previous) => {
                    crate::source_ring::SourceStaticRingMap::build_after(parent, &child, previous)?
                }
                None => crate::source_ring::SourceStaticRingMap::build(parent, &child)?,
            };
            stack.ring_maps.push(Arc::new(ring));
        }
        stack.levels.push(child);
        stack.active += 1;
        stack.validate_budget()?;
        Ok(stack)
    }

    pub(crate) fn snapshot(&self) -> Self {
        Self {
            active: self.active,
            scheme: self.scheme,
            levels: self.levels.iter().map(Mesh::snapshot).collect(),
            ring_maps: self.ring_maps.clone(),
        }
    }

    /// Keep one contiguous level range and its adjacent derived ring maps.
    /// The caller works on a candidate stack, so a validation error cannot
    /// partially change the installed document or its history.
    pub(crate) fn retain_range(&mut self, start: usize, end: usize) -> Result<(), String> {
        if start >= end || end > self.levels.len() || !(start..end).contains(&self.active) {
            return Err("Subdivision level retention range is invalid".into());
        }
        if self.scheme.uses_source_frame() {
            self.ring_maps.truncate(end - 1);
            self.ring_maps.drain(..start);
        }
        self.levels.truncate(end);
        self.levels.drain(..start);
        self.active -= start;
        self.validate_budget()
    }

    /// SculptGL's first downward analysis copies the current fine level's even
    /// rows to the parent. UV coordinates/topology stay bound to each level.
    /// Fine residuals remain in the stored child and the established bounded
    /// reconciliation carries later coarse edits upward.
    pub(crate) fn lower_source_even_rows_to(&mut self, level: usize) -> Result<(), String> {
        if !self.scheme.uses_source_lower_analysis() {
            return Ok(());
        }
        while self.active > level {
            let upper_index = self.active;
            let (lower_levels, upper_levels) = self.levels.split_at_mut(upper_index);
            let lower = &mut lower_levels[upper_index - 1];
            let upper = &upper_levels[0];
            let count = lower.positions.len();
            if upper.positions.len() < count
                || upper.colors.len() < count
                || upper.masks.len() < count
            {
                return Err("Static child lacks the parent's even rows".into());
            }
            lower.positions.copy_from_slice(&upper.positions[..count]);
            lower.colors.copy_from_slice(&upper.colors[..count]);
            lower.masks.copy_from_slice(&upper.masks[..count]);
            match (&mut lower.materials, &upper.materials) {
                (Some(parent), Some(child)) if child.len() >= count => {
                    parent.copy_from_slice(&child[..count]);
                }
                (None, None) => {}
                _ => return Err("Static child material channels do not match the parent".into()),
            }
            lower.recompute_normals();
            lower.validate()?;
            self.active -= 1;
        }
        self.validate_budget()?;
        Ok(())
    }

    pub(crate) fn active_topology_matches(&self, mesh: &Mesh) -> bool {
        self.levels
            .get(self.active)
            .is_some_and(|baseline| same_topology(baseline, mesh))
    }

    /// Explicit material-paint initialization, separate from generic attribute
    /// reconciliation. Seed a cloned, entirely implicit hierarchy only after
    /// the caller has observed a real material edit. Original topology and UV
    /// correspondence must still match; all aggregate bounds precede allocation.
    pub(crate) fn with_material_paint_defaults(&self, active_mesh: &Mesh) -> Result<Self, String> {
        self.validate_budget()?;
        active_mesh.validate()?;
        if active_mesh.mask_convention != self.levels[self.active].mask_convention
            || !self.active_topology_matches(active_mesh)
            || !self.levels[self.active].surface_matches_bits(active_mesh)
        {
            return Err(
                "Material painting cannot change retained topology or UV correspondence".into(),
            );
        }
        if self.levels.iter().any(|mesh| mesh.materials.is_some())
            || active_mesh.materials.is_none()
        {
            return Err(
                "Material paint defaults require an entirely implicit retained hierarchy".into(),
            );
        }
        let storage = self.levels.iter().try_fold(0_usize, |total, mesh| {
            let channel_bytes = mesh
                .positions
                .len()
                .checked_mul(std::mem::size_of::<[f32; 2]>())
                .ok_or("Material channel storage overflow")?;
            total
                .checked_add(mesh.approximate_bytes())
                .and_then(|total| total.checked_add(channel_bytes))
                .ok_or("Retained material storage overflow")
        })?;
        if storage > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Retained material channels exceed the storage allocation limit".into());
        }
        let mut candidate = self.snapshot();
        for mesh in &mut candidate.levels {
            let mut values = Vec::new();
            values
                .try_reserve_exact(mesh.positions.len())
                .map_err(|_| "Cannot allocate retained material paint defaults")?;
            values.resize(mesh.positions.len(), crate::DEFAULT_VERTEX_MATERIAL);
            mesh.materials = Some(values);
        }
        candidate.validate_budget()?;
        Ok(candidate)
    }

    /// Add a parent's Loop-interpolated delta to every finer level, retaining
    /// the finer level's own sculpt/paint residuals. Unchanged channels are
    /// copied exactly, avoiding numerical drift merely from switching levels.
    pub(crate) fn reconciled(&self, active_mesh: &Mesh) -> Result<Self, String> {
        active_mesh.validate()?;
        if active_mesh.mask_convention != self.levels[self.active].mask_convention {
            return Err(
                "Mask convention changes require an explicit whole-hierarchy transition".into(),
            );
        }
        if !self.active_topology_matches(active_mesh) {
            return Err("Active topology no longer matches its subdivision hierarchy".into());
        }
        let baseline = &self.levels[self.active];
        if baseline.materials.is_some() != active_mesh.materials.is_some() {
            return Err("Changing material channel presence in a retained hierarchy is unsupported; bake levels first".into());
        }
        if !baseline.surface_matches_bits(active_mesh) {
            return Err("Changing UV/original-face metadata inside a retained hierarchy is unsupported; bake levels first".into());
        }
        let mut result = self.snapshot();
        if active_mesh.same_surface(&self.levels[self.active]) {
            return Ok(result);
        }
        let mut old_parent = self.levels[self.active].snapshot();
        let mut new_parent = active_mesh.snapshot();
        result.levels[self.active] = active_mesh.snapshot();
        for level in self.active + 1..self.levels.len() {
            let old_child = &self.levels[level];
            let mut old_prediction = old_parent.snapshot();
            self.scheme.subdivide(&mut old_prediction)?;
            let mut new_prediction = new_parent.snapshot();
            self.scheme.subdivide(&mut new_prediction)?;
            if !same_topology(&old_prediction, old_child)
                || !same_topology(&new_prediction, old_child)
            {
                return Err("A stored level does not match Loop subdivision of its parent".into());
            }
            if old_prediction.materials.is_some() != new_prediction.materials.is_some()
                || old_child.materials.is_some() != new_prediction.materials.is_some()
            {
                return Err("Changing material channel presence in a retained hierarchy is unsupported; bake levels or install consistent channels before subdividing".into());
            }
            if !old_parent.surface_matches_bits(&new_parent) {
                return Err("Changing UV/original-face metadata inside a retained hierarchy is unsupported; preserve the bound surface or bake levels first".into());
            }
            let mut new_child = old_child.snapshot();
            let mut geometry_changed = false;
            let source_positions = if self.scheme.uses_source_frame()
                && old_prediction.positions != new_prediction.positions
            {
                Some(
                    self.ring_maps
                        .get(level - 1)
                        .ok_or("Static ring map is missing for retained level")?
                        .transport_positions(old_child, &old_prediction, &new_prediction)?,
                )
            } else {
                None
            };
            for vertex in 0..new_child.positions.len() {
                if let Some(positions) = &source_positions {
                    if new_child.positions[vertex] != positions[vertex] {
                        new_child.positions[vertex] = positions[vertex];
                        geometry_changed = true;
                    }
                } else if old_prediction.positions[vertex] != new_prediction.positions[vertex] {
                    new_child.positions[vertex] +=
                        new_prediction.positions[vertex] - old_prediction.positions[vertex];
                    geometry_changed = true;
                }
                if old_prediction.masks[vertex] != new_prediction.masks[vertex]
                    || (!new_parent.mask_convention.is_default()
                        && old_prediction.masks[vertex].to_bits()
                            != new_prediction.masks[vertex].to_bits())
                {
                    new_child.masks[vertex] = if !new_parent.mask_convention.is_default()
                        && old_child.masks[vertex].to_bits()
                            == old_prediction.masks[vertex].to_bits()
                    {
                        // Exact zero residual: copying avoids rounding a tiny source
                        // value or its signed-zero marker through a delta expression.
                        new_prediction.masks[vertex]
                    } else {
                        (old_child.masks[vertex] + new_prediction.masks[vertex]
                            - old_prediction.masks[vertex])
                            .clamp(0.0, 1.0)
                    };
                }
                for channel in 0..3 {
                    if old_prediction.colors[vertex][channel]
                        != new_prediction.colors[vertex][channel]
                    {
                        new_child.colors[vertex][channel] = (old_child.colors[vertex][channel]
                            + new_prediction.colors[vertex][channel]
                            - old_prediction.colors[vertex][channel])
                            .clamp(0.0, 1.0);
                    }
                }
            }
            if let (Some(child), Some(old), Some(new)) = (
                &mut new_child.materials,
                &old_prediction.materials,
                &new_prediction.materials,
            ) {
                for (vertex, material) in child.iter_mut().enumerate() {
                    for (channel, value) in material.iter_mut().enumerate() {
                        if old[vertex][channel] != new[vertex][channel] {
                            *value = (*value + new[vertex][channel] - old[vertex][channel])
                                .clamp(0.0, 1.0);
                        }
                    }
                }
            }
            new_child.validate()?;
            if geometry_changed {
                new_child.recompute_normals();
            }
            old_parent = old_child.snapshot();
            new_parent = new_child.snapshot();
            result.levels[level] = new_child;
        }
        result.validate_budget()?;
        Ok(result)
    }

    pub(crate) fn project_ref(&self) -> MultiresProjectRef<'_> {
        MultiresProjectRef {
            scheme: self.scheme,
            active_level: self.active,
            source_root_order: self.ring_maps.first().map(|map| map.source_parent_order()),
            levels: self
                .levels
                .iter()
                .enumerate()
                .map(|(i, mesh)| if i == self.active { None } else { Some(mesh) })
                .collect(),
        }
    }

    pub(crate) fn from_project(
        project: MultiresProject,
        active_mesh: &Mesh,
    ) -> Result<Self, String> {
        let scheme = project.scheme;
        let source_root_order = project.source_root_order;
        if source_root_order.is_some() && !scheme.uses_source_frame() {
            return Err("Only source-frame levels can carry source face order".into());
        }
        if !(1..=MAX_SUBDIVISION_LEVELS).contains(&project.levels.len())
            || project.active_level >= project.levels.len()
        {
            return Err("Invalid subdivision level count or active index".into());
        }
        let mut levels = Vec::with_capacity(project.levels.len());
        for (i, mesh) in project.levels.into_iter().enumerate() {
            match (i == project.active_level, mesh) {
                (true, None) => {
                    crate::decode_limits::capacity(
                        crate::decode_limits::Channel::Positions,
                        active_mesh.approximate_bytes(),
                    )?;
                    levels.push(active_mesh.snapshot());
                }
                (false, Some(mut mesh)) => {
                    mesh.validate()?;
                    mesh.recompute_normals();
                    levels.push(mesh);
                }
                _ => return Err("Exactly the active subdivision slot must be null".into()),
            }
        }
        let mut stack = Self {
            active: project.active_level,
            scheme,
            levels,
            ring_maps: Vec::new(),
        };
        for pair in stack.levels.windows(2) {
            if pair[0].triangles.len().checked_mul(4) != Some(pair[1].triangles.len()) {
                return Err("Subdivision child has the wrong triangle count".into());
            }
            let mut predicted = pair[0].snapshot();
            scheme.subdivide(&mut predicted)?;
            if !same_topology(&predicted, &pair[1]) {
                return Err("Subdivision child topology does not match its parent".into());
            }
            let surface_matches = if scheme == SubdivisionScheme::SculptglStaticSmoothV4 {
                match (&predicted.surface, &pair[1].surface) {
                    (Some(expected), Some(actual)) if expected.faces == actual.faces => {
                        match (&expected.uv, &actual.uv) {
                            (Some(expected_uv), Some(actual_uv)) => {
                                crate::attributes::uv_corner_refinement_matches(
                                    expected_uv,
                                    actual_uv,
                                )
                            }
                            (None, None) => true,
                            _ => false,
                        }
                    }
                    (None, None) => true,
                    _ => false,
                }
            } else {
                predicted.surface_matches_bits(&pair[1])
            };
            if predicted.materials.is_some() != pair[1].materials.is_some() || !surface_matches {
                return Err("Subdivision child attribute presence/UV correspondence does not match its parent".into());
            }
        }
        if scheme.uses_source_frame() {
            for pair in stack.levels.windows(2) {
                let ring = match stack.ring_maps.last() {
                    Some(previous) => crate::source_ring::SourceStaticRingMap::build_after(
                        &pair[0], &pair[1], previous,
                    )?,
                    None => match &source_root_order {
                        Some(order) => crate::source_ring::SourceStaticRingMap::build_seeded(
                            &pair[0], &pair[1], order,
                        )?,
                        None => crate::source_ring::SourceStaticRingMap::build(&pair[0], &pair[1])?,
                    },
                };
                stack.ring_maps.push(Arc::new(ring));
            }
            if stack.ring_maps.is_empty() && source_root_order.is_some() {
                return Err("Singleton source-static levels cannot carry a face order".into());
            }
        }
        stack.validate_budget()?;
        Ok(stack)
    }

    pub(crate) fn validate_budget(&self) -> Result<(), String> {
        if self.levels.len() > MAX_SUBDIVISION_LEVELS
            || self.levels.is_empty()
            || self.active >= self.levels.len()
        {
            return Err("Invalid subdivision level count or active index".into());
        }
        if self.scheme.uses_source_frame() {
            if self.ring_maps.len() + 1 != self.levels.len() {
                return Err("Static ring cache does not cover all levels".into());
            }
        } else if !self.ring_maps.is_empty() {
            return Err("Legacy levels cannot carry source ring maps".into());
        }
        let mut vertices = 0_usize;
        let mut triangles = 0_usize;
        let mut uv_coordinates = 0_usize;
        let mut storage = self.ring_maps.iter().fold(0usize, |bytes, map| {
            bytes.saturating_add(map.approximate_bytes())
        });
        for mesh in &self.levels {
            mesh.validate()?;
            if mesh.mask_convention != self.levels[0].mask_convention {
                return Err("Retained subdivision levels require one mask convention".into());
            }
            for (face, &ids) in mesh.triangles.iter().enumerate() {
                let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
                let area_squared = (b - a).cross(c - a).length_squared();
                if !area_squared.is_finite() || area_squared == 0.0 {
                    return Err(format!(
                        "Subdivision level has a collapsed or nonfinite triangle at face {face}"
                    ));
                }
            }
            uv_coordinates = uv_coordinates.saturating_add(mesh.uv_coordinate_count());
            storage = storage.saturating_add(mesh.approximate_bytes());
            if uv_coordinates > crate::MAX_UV_COORDINATES
                || storage > crate::decode_limits::MAX_SCENE_STORAGE_BYTES
            {
                return Err("Retained level UV/storage allocation limit exceeded".into());
            }
            vertices = vertices
                .checked_add(mesh.positions.len())
                .ok_or("Subdivision vertex-count overflow")?;
            triangles = triangles
                .checked_add(mesh.triangles.len())
                .ok_or("Subdivision triangle-count overflow")?;
            if vertices > MAX_VERTICES || triangles > MAX_TRIANGLES {
                return Err("Subdivision levels exceed the aggregate one-million vertex/two-million triangle limit".into());
            }
        }
        Ok(())
    }

    pub(crate) fn same_surface(&self, other: &Self) -> bool {
        self.active == other.active
            && self.scheme == other.scheme
            && self.levels.len() == other.levels.len()
            && self.ring_maps.first().map(|map| map.source_parent_order())
                == other.ring_maps.first().map(|map| map.source_parent_order())
            && self
                .levels
                .iter()
                .zip(&other.levels)
                .all(|(a, b)| a.same_surface(b))
    }

    pub(crate) fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.levels.capacity() * std::mem::size_of::<Mesh>()
            + self.ring_maps.capacity()
                * std::mem::size_of::<Arc<crate::source_ring::SourceStaticRingMap>>()
            + self
                .ring_maps
                .iter()
                .map(|map| map.approximate_bytes())
                .sum::<usize>()
            + self
                .levels
                .iter()
                .map(Mesh::approximate_bytes)
                .sum::<usize>()
    }
}

impl SubdivisionScheme {
    pub(crate) fn is_static(self) -> bool {
        self != Self::LoopWorldOffsetsV1
    }
    fn uses_source_lower_analysis(self) -> bool {
        matches!(
            self,
            Self::SculptglStaticSmoothV2
                | Self::SculptglStaticLinearEdgesV2
                | Self::SculptglStaticSmoothV3
                | Self::SculptglStaticLinearEdgesV3
                | Self::SculptglStaticSmoothV4
        )
    }
    fn uses_source_frame(self) -> bool {
        matches!(
            self,
            Self::SculptglStaticSmoothV3
                | Self::SculptglStaticLinearEdgesV3
                | Self::SculptglStaticSmoothV4
        )
    }
    fn subdivide(self, mesh: &mut Mesh) -> Result<(), String> {
        match self {
            Self::LoopWorldOffsetsV1 => mesh.subdivide_smooth(),
            Self::SculptglStaticSmoothV1
            | Self::SculptglStaticLinearEdgesV1
            | Self::SculptglStaticSmoothV2
            | Self::SculptglStaticLinearEdgesV2
            | Self::SculptglStaticSmoothV3
            | Self::SculptglStaticLinearEdgesV3
            | Self::SculptglStaticSmoothV4 => {
                let outcome = mesh.subdivided_static(&crate::StaticSubdivisionSettings {
                    linear_edges: matches!(
                        self,
                        Self::SculptglStaticLinearEdgesV1
                            | Self::SculptglStaticLinearEdgesV2
                            | Self::SculptglStaticLinearEdgesV3
                    ),
                })?;
                *mesh = outcome.mesh;
                Ok(())
            }
        }
    }
}

fn same_topology(a: &Mesh, b: &Mesh) -> bool {
    a.positions.len() == b.positions.len() && a.triangles == b.triangles
}

fn deserialize_levels<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Option<Mesh>>, D::Error> {
    struct LevelsVisitor;
    impl<'de> Visitor<'de> for LevelsVisitor {
        type Value = Vec<Option<Mesh>>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded list of subdivision meshes with one active null slot")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut levels = Vec::new();
            let mut vertices = 0_usize;
            let mut triangles = 0_usize;
            loop {
                if levels.len() >= MAX_SUBDIVISION_LEVELS {
                    if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(A::Error::custom("Too many subdivision levels"));
                    }
                    break;
                }
                let Some(mesh) = seq.next_element::<Option<Mesh>>()? else {
                    break;
                };
                if let Some(mesh) = &mesh {
                    vertices = vertices
                        .checked_add(mesh.positions.len())
                        .ok_or_else(|| A::Error::custom("Subdivision vertex-count overflow"))?;
                    triangles = triangles
                        .checked_add(mesh.triangles.len())
                        .ok_or_else(|| A::Error::custom("Subdivision triangle-count overflow"))?;
                    if vertices > MAX_VERTICES || triangles > MAX_TRIANGLES {
                        return Err(A::Error::custom(
                            "Subdivision aggregate allocation limit exceeded",
                        ));
                    }
                }
                levels.push(mesh);
            }
            Ok(levels)
        }
    }
    deserializer.deserialize_seq(LevelsVisitor)
}

fn deserialize_source_root_order<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<[u32; 2]>>, D::Error> {
    struct OptionVisitor;
    struct RowsVisitor;
    impl<'de> Visitor<'de> for RowsVisitor {
        type Value = Vec<[u32; 2]>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded source face-order array")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            loop {
                if rows.len() >= MAX_TRIANGLES {
                    if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(A::Error::custom("Too many source face-order rows"));
                    }
                    break;
                }
                let Some(row) = seq.next_element::<[u32; 2]>()? else {
                    break;
                };
                rows.try_reserve(1)
                    .map_err(|_| A::Error::custom("Cannot allocate source face order"))?;
                rows.push(row);
            }
            Ok(rows)
        }
    }
    impl<'de> Visitor<'de> for OptionVisitor {
        type Value = Option<Vec<[u32; 2]>>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a source face-order array or null")
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_seq(RowsVisitor).map(Some)
        }
    }
    deserializer.deserialize_option(OptionVisitor)
}
