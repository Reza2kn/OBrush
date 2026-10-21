//! Keep the validated pick and its ordinary Draw edit in one exclusive borrow.

use crate::{
    BrushKind, BrushResult, BrushSettings, Document, Hit, Mesh, SceneRay64, SglPickingPolicy,
};

/// An active-mesh contact validated by the scene picker. The exclusive borrow
/// prevents any mesh field from changing between validation and consumption.
/// There is deliberately no mutable-mesh accessor or public constructor.
pub struct ValidatedDrawContact<'a> {
    mesh: &'a mut Mesh,
    hit: Hit,
}

/// Only this module can construct the capability used by the core fast path.
pub(crate) struct ValidatedDrawSource {
    _private: (),
}

impl ValidatedDrawContact<'_> {
    pub fn hit(&self) -> Hit {
        self.hit
    }

    /// Consume the pick in the same history transaction as an ordinary Draw
    /// dab. Nonaccumulating Draw uses its separate captured-stroke proxy.
    pub fn apply(self, settings: &BrushSettings) -> Result<BrushResult, String> {
        if settings.kind != BrushKind::Draw || !settings.draw_accumulate {
            return Err("Validated Draw contact requires accumulating Draw".into());
        }
        Ok(self.mesh.apply_brush_at_validated_draw_hit(
            self.hit,
            settings,
            ValidatedDrawSource { _private: () },
        ))
    }
}

impl Document {
    /// Pick and fully validate the active visible mesh, then retain an
    /// exclusive borrow until its Draw dab is applied or the contact is dropped.
    pub fn pick_active_draw_contact(
        &mut self,
        ray: SceneRay64,
        policy: SglPickingPolicy,
    ) -> Result<Option<ValidatedDrawContact<'_>>, String> {
        let Some(picked) = self.raycast_scene(ray, policy)? else {
            return Ok(None);
        };
        if Some(picked.id) != self.active_subtool_id() || !self.active_subtool_visible() {
            return Ok(None);
        }
        let (hit, _) = picked.local_brush_hit()?;
        Ok(Some(ValidatedDrawContact {
            mesh: self.require_active_mesh_mut()?,
            hit,
        }))
    }
}
