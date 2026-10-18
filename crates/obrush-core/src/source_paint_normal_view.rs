//! Borrowed actual normal producer for ordinary alphaNone Paint. Cold public
//! preparation remains unchanged; a retained field never silently becomes fresh.
use crate::{
    Mesh, ObjectAffineF32V1, PaintPreparationInput, PaintResponsePass, PreparedSourcePaintHit,
    SourceGeometryNormalState, SourceNormalField, SourceNormalPolicy, SourceNormalReport,
    SourceOriginalFaceContact, SourcePickedNormalDisposition, SourcePickedNormalError,
    SourcePickedNormalRejection, SourcePickedNormalUndefined, Vec3,
};
use serde::Serialize;
use std::sync::Weak;

/// Opaque transient transaction owner. A globally unique tagged claim plus
/// exact snapshot/cache identity excludes foreign documents and later gestures.
#[derive(Clone, Debug)]
pub struct SourceRetainedPaintStroke {
    pub(crate) token: u64,
    pub(crate) part: u32,
    pub(crate) level: usize,
    pub(crate) cache: Weak<SourceGeometryNormalState>,
}
impl SourceRetainedPaintStroke {
    pub fn sequence(&self) -> u64 {
        self.token & !(1u64 << 63)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourcePaintNormalProducerError {
    InvalidMesh,
    MaskConvention,
    StaleField,
    PickedNormal(SourcePickedNormalRejection),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourcePaintNormalProducerKind {
    FreshStatic,
    RetainedGeometry,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourcePaintNormalProducerReport {
    pub kind: SourcePaintNormalProducerKind,
    pub normal_policy: SourceNormalReport,
    pub generation: Option<u64>,
}
#[derive(Clone, Copy)]
pub enum SourcePaintNormalView<'a> {
    FreshStatic(&'a SourceNormalField),
    RetainedGeometry(&'a SourceGeometryNormalState),
}
impl<'a> SourcePaintNormalView<'a> {
    pub fn validate(self, mesh: &Mesh) -> Result<(), SourcePaintNormalProducerError> {
        mesh.validate()
            .map_err(|_| SourcePaintNormalProducerError::InvalidMesh)?;
        if mesh.mask_convention != crate::MaskConvention::UnprotectedF32V1 {
            return Err(SourcePaintNormalProducerError::MaskConvention);
        }
        let current = match self {
            Self::FreshStatic(f) => {
                f.report().policy == SourceNormalPolicy::StaticOriginalFaces && f.matches_mesh(mesh)
            }
            Self::RetainedGeometry(f) => f.matches_mesh(mesh),
        };
        if !current {
            return Err(SourcePaintNormalProducerError::StaleField);
        }
        Ok(())
    }
    pub fn vertex_normals(self) -> &'a [Vec3] {
        match self {
            Self::FreshStatic(f) => f.vertex_normals(),
            Self::RetainedGeometry(f) => f.vertex_normals(),
        }
    }
    pub fn report(self) -> SourcePaintNormalProducerReport {
        match self {
            Self::FreshStatic(f) => SourcePaintNormalProducerReport {
                kind: SourcePaintNormalProducerKind::FreshStatic,
                normal_policy: f.report(),
                generation: None,
            },
            Self::RetainedGeometry(f) => SourcePaintNormalProducerReport {
                kind: SourcePaintNormalProducerKind::RetainedGeometry,
                normal_policy: f.normal_report(),
                generation: Some(f.generation()),
            },
        }
    }
    pub fn picked_normal_alpha_none(
        self,
        mesh: &Mesh,
        contact: SourceOriginalFaceContact,
    ) -> Result<SourcePickedNormalDisposition, SourcePaintNormalProducerError> {
        self.validate(mesh)?;
        if let Self::FreshStatic(f) = self {
            return crate::prepare_source_picked_normal_alpha_none(mesh, f, contact)
                .map_err(SourcePaintNormalProducerError::PickedNormal);
        }
        match crate::source_picked_normal::prepare_source_picked_normal_raw(
            mesh,
            self.vertex_normals(),
            SourceNormalPolicy::StaticSelectedRefresh,
            contact,
        ) {
            Ok(report) if report.direction_defined => {
                Ok(SourcePickedNormalDisposition::Defined { report })
            }
            Ok(report) => Ok(SourcePickedNormalDisposition::Undefined {
                reason: SourcePickedNormalUndefined::ZeroDirection,
                original_face_index: contact.face_index,
                local_point64: contact.local_point,
                raw_report: Some(report),
            }),
            Err(SourcePickedNormalError::Undefined(reason)) => {
                Ok(SourcePickedNormalDisposition::Undefined {
                    reason,
                    original_face_index: contact.face_index,
                    local_point64: contact.local_point,
                    raw_report: None,
                })
            }
            Err(SourcePickedNormalError::Rejected(reason)) => {
                Err(SourcePaintNormalProducerError::PickedNormal(reason))
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct PreparedSourcePaintRetainedPass {
    pub pass: PaintResponsePass,
    pub hit: PreparedSourcePaintHit,
    pub normal_producer: SourcePaintNormalProducerReport,
    pub picked_normal: SourcePickedNormalDisposition,
}
pub fn prepare_source_paint_pass_alpha_none_with_retained(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    state: &SourceGeometryNormalState,
) -> Result<Option<PreparedSourcePaintRetainedPass>, String> {
    let view = SourcePaintNormalView::RetainedGeometry(state);
    view.validate(mesh)
        .map_err(|e| format!("Retained Source Paint normal producer rejected: {e:?}"))?;
    let Some((pass, hit)) = crate::prepare_source_paint_pass(mesh, pose, input)? else {
        return Ok(None);
    };
    let picked_normal = view
        .picked_normal_alpha_none(
            mesh,
            SourceOriginalFaceContact {
                face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
                local_point: hit.local_position,
            },
        )
        .map_err(|e| format!("Retained Source Paint picked normal rejected: {e:?}"))?;
    Ok(Some(PreparedSourcePaintRetainedPass {
        pass,
        hit,
        normal_producer: view.report(),
        picked_normal,
    }))
}
