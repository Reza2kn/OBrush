//! Transient Source history effects. Public documents retain whole-state data;
//! only explicitly marked Source operations use derived normal transitions.
use super::{Document, DocumentState, HISTORY_BUDGET_BYTES};
use crate::{
    Mesh, SourceGeometryError, SourceGeometryNormalState, MAX_SOURCE_GEOMETRY_STATE_BYTES,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) enum SourceHistoryEvent {
    Geometry(GeometryEvent),
    Attributes { part: u32 },
}
#[derive(Debug, Clone)]
pub(super) struct GeometryEvent {
    part: u32,
    selected: Arc<Vec<bool>>,
}
impl SourceHistoryEvent {
    pub(super) fn geometry(
        part: u32,
        selected: Arc<Vec<bool>>,
    ) -> Result<Self, SourceGeometryError> {
        if selected.is_empty() || selected.len() > crate::MAX_VERTICES {
            return Err(SourceGeometryError::InvalidInput);
        }
        Ok(Self::Geometry(GeometryEvent { part, selected }))
    }
    pub(super) fn selection_bytes(vertices: usize) -> usize {
        std::mem::size_of::<Vec<bool>>() + 2 * std::mem::size_of::<usize>() + vertices
    }
    pub(super) fn bytes(&self) -> usize {
        match self {
            Self::Geometry(event) => Self::selection_bytes(event.selected.capacity()),
            Self::Attributes { .. } => 0,
        }
    }
    fn part(&self) -> u32 {
        match self {
            Self::Geometry(event) => event.part,
            Self::Attributes { part } => *part,
        }
    }
}

type Repository = Vec<(u32, Arc<SourceGeometryNormalState>)>;

impl DocumentState {
    fn source_history_mesh(&self, part: u32) -> Option<&Mesh> {
        let active = self
            .scene
            .as_ref()
            .map_or_else(|| self.mesh.as_ref().map(|_| 1), |s| s.active_id);
        if active == Some(part) {
            self.mesh.as_ref()
        } else {
            self.scene
                .as_ref()?
                .objects
                .iter()
                .find(|o| o.id == part)?
                .payload
                .as_ref()
                .map(|p| p.mesh.as_ref())
        }
    }
}

impl Document {
    pub(super) fn attach_source_selection_history(&mut self, stroke: &crate::SourceGeometryStroke) {
        if let (Some(selected), Some((part, _)), Some(previous)) = (
            &stroke.history_selected,
            stroke.binding,
            self.stroke_start.as_mut(),
        ) {
            if !selected.is_empty() {
                previous.source_history = Some(SourceHistoryEvent::Geometry(GeometryEvent {
                    part,
                    selected: selected.clone(),
                }));
            }
        }
    }
    fn source_history_mesh(&self, part: u32) -> Option<&Mesh> {
        if self.active_subtool_id() == Some(part) {
            self.mesh.as_ref()
        } else {
            self.scene
                .as_ref()?
                .objects
                .iter()
                .find(|o| o.id == part)?
                .payload
                .as_ref()
                .map(|p| p.mesh.as_ref())
        }
    }
    /// Mark an ordinary Source attribute transaction. Its history never replaces
    /// a CURRENT normal field with an older cache snapshot. Geometry must match.
    pub fn begin_source_attribute_history(&mut self, part: u32) -> Result<(), SourceGeometryError> {
        if self
            .source_geometry_active
            .is_some_and(|token| token & super::source_geometry_document::WARM_PAINT_TAG == 0)
        {
            return Err(SourceGeometryError::StrokeAlreadyOpen);
        }
        if self.active_subtool_id() != Some(part) {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        self.begin_stroke();
        let previous = self
            .stroke_start
            .as_ref()
            .ok_or(SourceGeometryError::NoOpenStroke)?;
        let old = previous
            .source_history_mesh(part)
            .ok_or(SourceGeometryError::StrokeBindingChanged)?;
        let current = self
            .source_history_mesh(part)
            .ok_or(SourceGeometryError::StrokeBindingChanged)?;
        if !same_geometry(old, current) || previous.approximate_bytes() > HISTORY_BUDGET_BYTES {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        if previous.source_history.as_ref().is_some_and(
            |event| !matches!(event, SourceHistoryEvent::Attributes { part: p } if *p == part),
        ) {
            return Err(SourceGeometryError::StrokeAlreadyOpen);
        }
        self.stroke_start
            .as_mut()
            .expect("checked history snapshot")
            .source_history = Some(SourceHistoryEvent::Attributes { part });
        Ok(())
    }
    /// All fallible normal/storage work runs while both stacks and the document
    /// still own their original states. The stack exchange follows only after
    /// this preparation succeeds.
    pub(super) fn prepare_source_history(
        &self,
        target: &DocumentState,
        max_cache_bytes: usize,
    ) -> Result<Option<Repository>, SourceGeometryError> {
        let Some(event) = target.source_history.as_ref() else {
            return Ok(None);
        };
        if max_cache_bytes > MAX_SOURCE_GEOMETRY_STATE_BYTES {
            return Err(SourceGeometryError::InvalidInput);
        }
        let part = event.part();
        let current_mesh = self
            .source_history_mesh(part)
            .ok_or(SourceGeometryError::StrokeBindingChanged)?;
        let target_mesh = target
            .source_history_mesh(part)
            .ok_or(SourceGeometryError::StrokeBindingChanged)?;
        target_mesh
            .validate()
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        let current = self.source_geometry_state(part);
        if current
            .as_ref()
            .is_some_and(|state| !state.matches_mesh(current_mesh))
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        let next = match event {
            SourceHistoryEvent::Attributes { .. } => {
                if !same_geometry(current_mesh, target_mesh) {
                    return Err(SourceGeometryError::StrokeBindingChanged);
                }
                current
            }
            SourceHistoryEvent::Geometry(event) => {
                let state = match current {
                    Some(state) => state,
                    None => Arc::new(SourceGeometryNormalState::seed_for_history(current_mesh)?),
                };
                Some(Arc::new(state.refreshed_for_history(
                    target_mesh,
                    &event.selected,
                    max_cache_bytes,
                )?))
            }
        };
        // StateGeometry changes one mesh only. Other parts retain their CURRENT
        // normal fields, which may have been refreshed by earlier Undo steps.
        // Restoring those parts from this event's old repository would silently
        // resurrect stale rows even though their public geometry is unchanged.
        let count = self
            .source_geometry_states
            .iter()
            .filter(|(id, _)| *id != part)
            .count()
            .checked_add(usize::from(next.is_some()))
            .ok_or(SourceGeometryError::StorageLimit)?;
        if count > crate::MAX_SUBTOOLS {
            return Err(SourceGeometryError::StorageLimit);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(count)
            .map_err(|_| SourceGeometryError::Allocation)?;
        for (id, state) in &self.source_geometry_states {
            if *id == part {
                continue;
            }
            let mesh = target
                .source_history_mesh(*id)
                .ok_or(SourceGeometryError::StrokeBindingChanged)?;
            if !state.matches_mesh(mesh) {
                return Err(SourceGeometryError::StaleNormalState);
            }
            rows.push((*id, state.clone()));
        }
        if let Some(next) = next {
            rows.push((part, next));
        }
        let bytes = super::source_geometry_document::retained_bytes(&rows, rows.capacity());
        let old = super::source_geometry_document::retained_bytes(
            &target.source_geometry_states,
            target.source_geometry_states.capacity(),
        );
        let total = target
            .approximate_bytes()
            .saturating_sub(old)
            .checked_add(bytes)
            .ok_or(SourceGeometryError::StorageLimit)?;
        if bytes > max_cache_bytes || total > HISTORY_BUDGET_BYTES {
            return Err(SourceGeometryError::StorageLimit);
        }
        Ok(Some(rows))
    }
}

fn same_geometry(a: &Mesh, b: &Mesh) -> bool {
    a.positions.len() == b.positions.len()
        && a.positions
            .iter()
            .zip(&b.positions)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
        && a.triangles == b.triangles
        && a.surface.as_ref().map(|s| &s.faces) == b.surface.as_ref().map(|s| &s.faces)
}

#[cfg(test)]
#[path = "source_history_tests.rs"]
mod tests;
