//! Explicit source appearance resolution. Unencoded constructor defaults are a
//! policy argument, not newly invented persistent/raw SGL fields.
use crate::{SceneObjectMetadata, SglRenderSettings};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceAppearanceOrigin {
    EncodedOrEditable,
    UnencodedConstructorPolicy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceShaderRole {
    Pbr,
    Flat,
    Normal,
    Wireframe,
    Uv,
    Matcap,
    Auxiliary(u32),
}
impl SourceShaderRole {
    pub fn from_source_code(code: u32) -> Result<Self, String> {
        Ok(match code {
            0 => Self::Pbr,
            1 => Self::Flat,
            2 => Self::Normal,
            3 => Self::Wireframe,
            4 => Self::Uv,
            5 => Self::Matcap,
            6..=12 => Self::Auxiliary(code),
            _ => return Err("Unknown pinned source shader code".into()),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SourceAppearance {
    pub settings: SglRenderSettings,
    pub role: SourceShaderRole,
    pub origin: SourceAppearanceOrigin,
}
/// Pinned constructor baseline with no URL shader/Matcap overrides. The source
/// clamps Matcap index to its available asset list; an actual host must resolve
/// that asset rather than claim an embedded texture exists in SGL.
pub fn baseline_source_render_defaults() -> SglRenderSettings {
    SglRenderSettings {
        shader: 5,
        matcap: 3,
        show_wireframe: false,
        flat_shading: false,
        opacity: 1.0,
    }
}
/// None denotes ordinary OBrush viewport policy. Only a legacy source origin
/// resolves absent settings through the supplied constructor/URL policy. Modern
/// source versions encode render data and cannot masquerade as absent legacy data.
pub fn resolve_source_appearance(
    metadata: Option<&SceneObjectMetadata>,
    constructor_defaults: &SglRenderSettings,
) -> Result<Option<SourceAppearance>, String> {
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    let (settings, origin) = if let Some(settings) = &metadata.render {
        (settings.clone(), SourceAppearanceOrigin::EncodedOrEditable)
    } else if let Some(source) = &metadata.origin {
        if source.object.version >= 2 {
            return Err("Modern SGL origin requires an explicit editable appearance; it has no absent render-block default".into());
        }
        crate::sgl::validate_render(constructor_defaults)?;
        (
            constructor_defaults.clone(),
            SourceAppearanceOrigin::UnencodedConstructorPolicy,
        )
    } else {
        return Ok(None);
    };
    crate::sgl::validate_render(&settings)?;
    let role = SourceShaderRole::from_source_code(settings.shader)?;
    Ok(Some(SourceAppearance {
        settings,
        role,
        origin,
    }))
}
