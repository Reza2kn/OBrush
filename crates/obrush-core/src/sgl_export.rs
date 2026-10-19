//! Bounded editable SGLv3 export. Preflight reports conversions before allocating
//! mapped channels. This cannot encode OBrush history/provenance/scene controls.
use crate::{
    MaskConvention, Mesh, SceneObjectMetadata, SglByteOrder, SglCameraSettings, SglDisplaySettings,
    SglMesh, SglRenderSettings, SglUvData, MAX_SGL_UV_COORDS, MAX_TRIANGLES, MAX_VERTICES,
    TRIANGLE_CORNER,
};
use serde::Serialize;
const LIMIT: usize = 128 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SglExportScope {
    AllScene,
    SelectedParts,
    VisibleScene,
    ActivePart,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SglExportLossPolicy {
    RejectLoss,
    AllowAndReport,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SglExportSettings {
    pub scope: SglExportScope,
    pub byte_order: SglByteOrder,
    /// Actual host camera and explicitly declared display state. Their absence
    /// in an original v0/1 record stays absent in that immutable origin.
    pub display: SglDisplaySettings,
    pub camera: SglCameraSettings,
    /// Current ordinary-object view mapping (Studio must be declared as a
    /// constructor-Matcap conversion by the host, not aliased silently).
    pub plain_render: SglRenderSettings,
    pub legacy_constructor_render: SglRenderSettings,
    pub plain_appearance_conversion: bool,
    pub declared_display_defaults: bool,
    pub loss_policy: SglExportLossPolicy,
}
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct SglExportReport {
    pub version: u32,
    pub objects: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub faces: usize,
    pub uv_coordinates: usize,
    pub encoded_bytes: usize,
    pub mapped_scratch_bytes: usize,
    pub borrowed_unchanged_source_objects: usize,
    pub source_version_conversions: usize,
    pub source_byte_order_conversions: usize,
    pub normalized_geometry_markers: usize,
    pub normalized_uv_markers: usize,
    pub defaulted_render_objects: usize,
    pub defaulted_header_fields: usize,
    pub declared_display_default_fields: usize,
    pub changed_header_fields: usize,
    pub changed_source_centers: usize,
    pub changed_source_scales: usize,
    pub absent_rgb_channels_created: usize,
    pub absent_material_channels_created: usize,
    pub protected_mask_words_rounded: usize,
    pub maximum_mask_roundtrip_error: f64,
    pub maximum_derived_scale_rounding_error: f64,
    pub omitted_subtool_names: usize,
    pub omitted_stable_ids: usize,
    pub omitted_selection: bool,
    pub omitted_visibility_objects: usize,
    pub omitted_solo: bool,
    pub omitted_retained_levels: usize,
    pub omitted_retained_hierarchies: usize,
    pub omitted_layers: usize,
    pub omitted_morph_targets: usize,
    pub normals_recomputed_on_import_vertices: usize,
    pub ordinary_appearance_conversions: usize,
    pub immutable_origins_not_embedded: usize,
    pub omitted_picking_policy_objects: usize,
    /// SGL has no per-mesh symmetry-offset word.
    pub omitted_symmetry_offset_objects: usize,
}
impl SglExportReport {
    pub fn requires_allow(&self) -> bool {
        self.protected_mask_words_rounded > 0
            || self.normalized_geometry_markers > 0
            || self.normalized_uv_markers > 0
            || self.defaulted_render_objects > 0
            || self.defaulted_header_fields > 0
            || self.declared_display_default_fields > 0
            || self.changed_header_fields > 0
            || self.changed_source_centers > 0
            || self.changed_source_scales > 0
            || self.omitted_subtool_names > 0
            || self.omitted_stable_ids > 0
            || self.omitted_selection
            || self.omitted_visibility_objects > 0
            || self.omitted_solo
            || self.omitted_retained_levels > 0
            || self.omitted_retained_hierarchies > 0
            || self.omitted_layers > 0
            || self.omitted_morph_targets > 0
            || self.ordinary_appearance_conversions > 0
            || self.omitted_picking_policy_objects > 0
            || self.omitted_symmetry_offset_objects > 0
            || self.maximum_derived_scale_rounding_error > 0.0
    }
}
#[derive(Debug)]
pub struct SglExport {
    pub bytes: Vec<u8>,
    pub report: SglExportReport,
}
pub(crate) struct ObjectView<'a> {
    pub id: u32,
    pub name: &'a str,
    pub visible: bool,
    pub mesh: &'a Mesh,
    pub metadata: Option<&'a SceneObjectMetadata>,
    pub levels: usize,
    pub retained: bool,
    pub layers: usize,
    pub morph: bool,
}
fn equal(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}
fn triples(a: &[[f32; 3]], b: &[[f32; 3]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
}
/// Read-only field comparison. Normal arrays are derived and have no SGL field.
fn source_geometry_fields_unchanged(view: &ObjectView<'_>, ignore_render: bool) -> bool {
    let Some(meta) = view.metadata else {
        return false;
    };
    let Some(origin) = &meta.origin else {
        return false;
    };
    let raw = &origin.object.raw;
    let m = view.mesh;
    if meta.affine.local_to_world.map(f32::to_bits) != raw.matrix.map(f32::to_bits)
        || !(m.positions.len() == raw.positions.len()
            && m.positions
                .iter()
                .zip(&raw.positions)
                .all(|(a, b)| a.to_array().map(f32::to_bits) == b.map(f32::to_bits)))
    {
        return false;
    }
    match &raw.colors {
        Some(c) if triples(&m.colors, c) => {}
        None if m
            .colors
            .iter()
            .all(|r| r.map(f32::to_bits) == [1.0f32.to_bits(); 3]) => {}
        _ => return false,
    }
    match &raw.materials {
        Some(rows)
            if m.mask_convention == MaskConvention::UnprotectedF32V1
                && m.materials.as_ref().is_some_and(|rm| {
                    rm.len() == rows.len()
                        && rm.iter().zip(rows).zip(&m.masks).all(|((rm, raw), z)| {
                            equal(rm[0], raw[0]) && equal(rm[1], raw[1]) && equal(*z, raw[2])
                        })
                }) => {}
        None if m.materials.is_none()
            && m.mask_convention == MaskConvention::ProtectedF32V1
            && m.masks.iter().all(|z| z.to_bits() == 0) => {}
        _ => return false,
    }
    let Some(surface) = &m.surface else {
        return false;
    };
    if surface.faces.len() != raw.faces.len()
        || !surface.faces.iter().zip(&raw.faces).all(|(a, b)| {
            let mut normalized = *b;
            if origin.object.version < 3 && (b[3] as i32) < 0 {
                normalized[3] = TRIANGLE_CORNER;
            }
            *a == normalized
        })
    {
        return false;
    }
    match (&surface.uv, &raw.uv) {
        (None, None) => {}
        (Some(a), Some(b))
            if a.coordinates.len() == b.coordinates.len()
                && a.coordinates
                    .iter()
                    .zip(&b.coordinates)
                    .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
                && a.face_indices.iter().zip(&b.face_indices).all(|(x, y)| {
                    x[..3] == y[..3]
                        && (x[3] == y[3]
                            || (origin.object.version < 3
                                && x[3] == TRIANGLE_CORNER
                                && (y[3] as i32) < 0))
                })
                && a.face_indices.len() == b.face_indices.len() => {}
        _ => return false,
    }
    if ignore_render {
        return true;
    }
    match (&meta.render, &raw.render) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.shader == b.shader
                && a.matcap == b.matcap
                && a.show_wireframe == b.show_wireframe
                && a.flat_shading == b.flat_shading
                && equal(a.opacity, b.opacity)
        }
        _ => false,
    }
}
/// Geometry/render comparison stays independent from the center export policy.
fn source_fields_unchanged(view: &ObjectView<'_>, ignore_render: bool) -> bool {
    if !source_geometry_fields_unchanged(view, ignore_render) {
        return false;
    }
    let metadata = view.metadata.unwrap();
    !metadata.source_center_initialized
        || metadata.affine.interaction_center.map(f32::to_bits)
            == metadata
                .origin
                .as_ref()
                .unwrap()
                .object
                .raw
                .center
                .map(f32::to_bits)
}
fn source_center(view: &ObjectView<'_>, metadata: &SceneObjectMetadata) -> [f32; 3] {
    if metadata.source_center_initialized || !source_geometry_fields_unchanged(view, true) {
        metadata.affine.interaction_center
    } else {
        metadata.origin.as_ref().unwrap().object.raw.center
    }
}
fn unchanged(view: &ObjectView<'_>) -> bool {
    source_fields_unchanged(view, false)
}
fn keep_absent_rgb(view: &ObjectView<'_>) -> bool {
    view.metadata
        .and_then(|m| m.origin.as_ref())
        .is_some_and(|o| o.object.raw.colors.is_none())
        && view
            .mesh
            .colors
            .iter()
            .all(|r| r.map(f32::to_bits) == [1.0f32.to_bits(); 3])
}
fn keep_absent_material(view: &ObjectView<'_>) -> bool {
    view.metadata
        .and_then(|m| m.origin.as_ref())
        .is_some_and(|o| o.object.raw.materials.is_none())
        && view.mesh.materials.is_none()
        && view.mesh.mask_convention == MaskConvention::ProtectedF32V1
        && view.mesh.masks.iter().all(|v| v.to_bits() == 0)
}
fn checked_add(target: &mut usize, value: usize, cap: usize, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("SGL {label} count overflow"))?;
    if *target > cap {
        return Err(format!("SGL {label} limit exceeded"));
    }
    Ok(())
}
fn raw_z(mesh: &Mesh, index: usize) -> f32 {
    match mesh.mask_convention {
        MaskConvention::UnprotectedF32V1 => mesh.masks[index],
        MaskConvention::ProtectedF32V1 => 1.0 - mesh.masks[index],
    }
}
fn source_scale(meta: Option<&SceneObjectMetadata>) -> (f32, f64) {
    let exact = meta.map_or(1.0, |m| m.affine.source_scale_squared().sqrt());
    let rounded = exact as f32;
    (rounded, (exact - f64::from(rounded)).abs())
}
fn record_counts(view: &ObjectView<'_>, reuse: bool) -> (usize, usize, usize, usize) {
    if reuse {
        let raw = &view.metadata.unwrap().origin.as_ref().unwrap().object.raw;
        (
            raw.colors.as_ref().map_or(0, Vec::len),
            raw.materials.as_ref().map_or(0, Vec::len),
            raw.uv.as_ref().map_or(0, |u| u.coordinates.len()),
            raw.uv.as_ref().map_or(0, |u| u.face_indices.len()),
        )
    } else {
        (
            if keep_absent_rgb(view) {
                0
            } else {
                view.mesh.colors.len()
            },
            if keep_absent_material(view) {
                0
            } else {
                view.mesh.positions.len()
            },
            view.mesh
                .surface
                .as_ref()
                .and_then(|s| s.uv.as_ref())
                .map_or(0, |u| u.coordinates.len()),
            view.mesh
                .surface
                .as_ref()
                .and_then(|s| s.uv.as_ref())
                .map_or(0, |u| u.face_indices.len()),
        )
    }
}
pub(crate) fn plan(
    views: &[ObjectView<'_>],
    settings: &SglExportSettings,
    origin: Option<&crate::SglSceneOrigin>,
    selection: &[u32],
    active: Option<u32>,
    solo: bool,
) -> Result<SglExportReport, String> {
    if !matches!(
        settings.scope,
        SglExportScope::AllScene | SglExportScope::SelectedParts
    ) {
        return Err("Only all-scene and selected-parts SGL export are mapped; visible/active scopes remain unsupported".into());
    }
    crate::sgl::validate_render(&settings.plain_render)?;
    crate::sgl::validate_render(&settings.legacy_constructor_render)?;
    // Reuse the core header validation, including empty scenes and exact FOV.
    crate::SglScene {
        version: 3,
        byte_order: settings.byte_order,
        display: Some(settings.display.clone()),
        camera: Some(settings.camera.clone()),
        meshes: vec![],
    }
    .validate()?;
    let mut report = SglExportReport {
        version: 3,
        encoded_bytes: 36,
        ..Default::default()
    };
    report.objects = views.len();
    if views.len() > crate::MAX_SUBTOOLS {
        return Err("SGL scene exceeds32 objects".into());
    }
    report.declared_display_default_fields = if settings.declared_display_defaults {
        3
    } else {
        0
    };
    match origin {
        Some(origin) => {
            report.defaulted_header_fields = if origin.camera.is_none() { 7 } else { 0 };
            if let Some(c) = &origin.camera {
                report.changed_header_fields +=
                    usize::from(c.projection != settings.camera.projection)
                        + usize::from(c.mode != settings.camera.mode)
                        + usize::from(!equal(c.fov_degrees, settings.camera.fov_degrees))
                        + usize::from(c.use_pivot != settings.camera.use_pivot);
            }
            if let Some(d) = &origin.display {
                report.changed_header_fields +=
                    usize::from(d.show_grid != settings.display.show_grid)
                        + usize::from(d.show_symmetry_line != settings.display.show_symmetry_line)
                        + usize::from(d.show_contour != settings.display.show_contour);
            }
        }
        None => report.defaulted_header_fields = 7,
    }
    for (row, view) in views.iter().enumerate() {
        view.mesh.validate()?;
        if let Some(meta) = view.metadata {
            meta.validate(view.mesh)?;
        }
        checked_add(
            &mut report.vertices,
            view.mesh.positions.len(),
            MAX_VERTICES,
            "vertices",
        )?;
        checked_add(
            &mut report.triangles,
            view.mesh.triangles.len(),
            MAX_TRIANGLES,
            "triangles",
        )?;
        let face_count = view
            .mesh
            .surface
            .as_ref()
            .map_or(view.mesh.triangles.len(), |s| s.faces.len());
        checked_add(&mut report.faces, face_count, MAX_TRIANGLES, "faces")?;
        let same = unchanged(view);
        let raw = view
            .metadata
            .and_then(|m| m.origin.as_ref())
            .map(|o| o.object.raw.as_ref());
        let compatible =
            same && view.metadata.and_then(|m| m.render.as_ref()).is_some()
                && raw.is_some_and(|r| {
                    r.faces.iter().all(|f| {
                        f[3] == TRIANGLE_CORNER || (f[3] as usize) < view.mesh.positions.len()
                    }) && r.uv.as_ref().is_none_or(|u| {
                        u.face_indices.iter().all(|f| {
                            f[3] == TRIANGLE_CORNER || (f[3] as usize) < u.coordinates.len()
                        })
                    })
                });
        report.borrowed_unchanged_source_objects += usize::from(compatible);
        let (c, m, u, uf) = record_counts(view, compatible);
        checked_add(
            &mut report.uv_coordinates,
            u,
            MAX_SGL_UV_COORDS,
            "UV coordinates",
        )?;
        let encoded = 124usize
            .checked_add(
                view.mesh
                    .positions
                    .len()
                    .checked_mul(12)
                    .ok_or("SGL position byte overflow")?,
            )
            .and_then(|x| x.checked_add(c * 12))
            .and_then(|x| x.checked_add(m * 12))
            .and_then(|x| x.checked_add(face_count * 16))
            .and_then(|x| x.checked_add(u * 8))
            .and_then(|x| x.checked_add(uf * 16))
            .ok_or("SGL object byte overflow")?;
        checked_add(&mut report.encoded_bytes, encoded, LIMIT, "output bytes")?;
        if !compatible {
            checked_add(
                &mut report.mapped_scratch_bytes,
                encoded,
                LIMIT,
                "mapped scratch",
            )?;
        }
        report.omitted_subtool_names += usize::from(
            view.name != format!("SGL object {}", row + 1)
                && view.name != format!("Part {}", row + 1),
        );
        report.omitted_stable_ids += usize::from(view.id != row as u32 + 1);
        report.omitted_visibility_objects += usize::from(!view.visible);
        report.omitted_retained_levels += view.levels.saturating_sub(1);
        report.omitted_retained_hierarchies += usize::from(view.retained);
        report.omitted_layers += view.layers;
        report.omitted_morph_targets += usize::from(view.morph);
        report.normals_recomputed_on_import_vertices += view.mesh.positions.len();
        if !compatible && !source_geometry_fields_unchanged(view, true) {
            report.maximum_derived_scale_rounding_error = report
                .maximum_derived_scale_rounding_error
                .max(source_scale(view.metadata).1);
        }
        if let Some(meta) = view.metadata {
            report.omitted_symmetry_offset_objects += usize::from(meta.symmetry_offset != 0.0);
            report.omitted_picking_policy_objects +=
                usize::from(meta.picking_policy != crate::SglPickingPolicy::SourceScaleRowV1);
            report.defaulted_render_objects += usize::from(meta.render.is_none());
            if let Some(o) = &meta.origin {
                report.immutable_origins_not_embedded += 1;
                report.source_version_conversions += usize::from(o.object.version != 3);
                report.source_byte_order_conversions +=
                    usize::from(o.object.byte_order != settings.byte_order);
                if o.object.version < 3 {
                    report.normalized_geometry_markers += o
                        .object
                        .raw
                        .faces
                        .iter()
                        .filter(|f| f[3] != TRIANGLE_CORNER && (f[3] as i32) < 0)
                        .count();
                    report.normalized_uv_markers += o.object.raw.uv.as_ref().map_or(0, |u| {
                        u.face_indices
                            .iter()
                            .filter(|f| f[3] != TRIANGLE_CORNER && (f[3] as i32) < 0)
                            .count()
                    });
                }
                report.changed_source_centers += usize::from(
                    source_center(view, meta).map(f32::to_bits)
                        != o.object.raw.center.map(f32::to_bits),
                );
                if !compatible && !source_geometry_fields_unchanged(view, true) {
                    let (scale, error) = source_scale(Some(meta));
                    report.maximum_derived_scale_rounding_error =
                        report.maximum_derived_scale_rounding_error.max(error);
                    report.changed_source_scales += usize::from(!equal(scale, o.object.raw.scale));
                    report.absent_rgb_channels_created +=
                        usize::from(o.object.raw.colors.is_none() && !keep_absent_rgb(view));
                    report.absent_material_channels_created += usize::from(
                        o.object.raw.materials.is_none() && !keep_absent_material(view),
                    );
                }
            }
        } else {
            report.defaulted_render_objects += 1;
            report.ordinary_appearance_conversions +=
                usize::from(settings.plain_appearance_conversion);
        }
        if !compatible && view.mesh.mask_convention == MaskConvention::ProtectedF32V1 {
            for &p in &view.mesh.masks {
                let roundtrip = 1.0f32 - (1.0f32 - p);
                if p.to_bits() != roundtrip.to_bits() {
                    report.protected_mask_words_rounded += 1;
                    report.maximum_mask_roundtrip_error = report
                        .maximum_mask_roundtrip_error
                        .max((f64::from(p) - f64::from(roundtrip)).abs());
                }
            }
        }
    }
    report.omitted_selection =
        active != views.last().map(|o| o.id) || selection != active.into_iter().collect::<Vec<_>>();
    report.omitted_solo = solo;
    Ok(report)
}
fn reserve<T>(count: usize, label: &str) -> Result<Vec<T>, String> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| format!("Cannot allocate bounded SGL {label}"))?;
    Ok(v)
}
pub(crate) fn export(
    views: &[ObjectView<'_>],
    settings: &SglExportSettings,
    report: SglExportReport,
) -> Result<SglExport, String> {
    if settings.loss_policy == SglExportLossPolicy::RejectLoss && report.requires_allow() {
        return Err(
            "SGL export requires explicit approval of its preflight conversion/loss report".into(),
        );
    }
    enum Record<'a> {
        Raw(&'a SglMesh),
        Mapped(usize),
    }
    // All three header arrays are independently bounded by MAX_SUBTOOLS (32)
    // and allocated fallibly. Indices keep borrowed records small without an
    // infallible Box allocation for each mapped mesh or cloning source arrays.
    let mut records = reserve(views.len(), "object records")?;
    let mut mapped = reserve(views.len(), "mapped object records")?;
    for view in views {
        if unchanged(view) && view.metadata.and_then(|m| m.render.as_ref()).is_some() {
            let raw = view
                .metadata
                .unwrap()
                .origin
                .as_ref()
                .unwrap()
                .object
                .raw
                .as_ref();
            let version = view
                .metadata
                .unwrap()
                .origin
                .as_ref()
                .unwrap()
                .object
                .version;
            if version == 3
                || raw
                    .faces
                    .iter()
                    .all(|f| f[3] == TRIANGLE_CORNER || ((f[3] as i32) >= 0))
                    && raw.uv.as_ref().is_none_or(|u| {
                        u.face_indices
                            .iter()
                            .all(|f| f[3] == TRIANGLE_CORNER || (f[3] as i32) >= 0)
                    })
            {
                records.push(Record::Raw(raw));
                continue;
            }
        }
        let mesh = view.mesh;
        let mut positions = reserve(mesh.positions.len(), "positions")?;
        positions.extend(mesh.positions.iter().map(|p| p.to_array()));
        let mut colors = reserve(
            if keep_absent_rgb(view) {
                0
            } else {
                mesh.colors.len()
            },
            "RGB",
        )?;
        if !keep_absent_rgb(view) {
            colors.extend_from_slice(&mesh.colors);
        }
        let mut materials = reserve(
            if keep_absent_material(view) {
                0
            } else {
                mesh.positions.len()
            },
            "material triples",
        )?;
        for i in 0..if keep_absent_material(view) {
            0
        } else {
            mesh.positions.len()
        } {
            let rm = mesh
                .materials
                .as_ref()
                .map_or(crate::DEFAULT_VERTEX_MATERIAL, |m| m[i]);
            materials.push([rm[0], rm[1], raw_z(mesh, i)]);
        }
        let mut faces = reserve(
            mesh.surface
                .as_ref()
                .map_or(mesh.triangles.len(), |s| s.faces.len()),
            "faces",
        )?;
        match &mesh.surface {
            Some(s) => faces.extend_from_slice(&s.faces),
            None => faces.extend(
                mesh.triangles
                    .iter()
                    .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER]),
            ),
        }
        let uv = mesh
            .surface
            .as_ref()
            .and_then(|s| s.uv.as_ref())
            .map(|u| {
                let mut coordinates = reserve(u.coordinates.len(), "UV coordinates")?;
                coordinates.extend_from_slice(&u.coordinates);
                let mut face_indices = reserve(u.face_indices.len(), "UV corners")?;
                face_indices.extend_from_slice(&u.face_indices);
                Ok::<_, String>(SglUvData {
                    coordinates,
                    face_indices,
                })
            })
            .transpose()?;
        let (center, matrix, scale, render) = match view.metadata {
            Some(m) => (
                source_center(view, m),
                m.affine.local_to_world,
                if source_geometry_fields_unchanged(view, true) {
                    m.origin.as_ref().unwrap().object.raw.scale
                } else {
                    source_scale(Some(m)).0
                },
                m.render
                    .clone()
                    .unwrap_or_else(|| settings.legacy_constructor_render.clone()),
            ),
            None => {
                let low = mesh
                    .positions
                    .iter()
                    .fold(crate::Vec3::splat(f32::INFINITY), |a, &p| a.min(p));
                let high = mesh
                    .positions
                    .iter()
                    .fold(crate::Vec3::splat(f32::NEG_INFINITY), |a, &p| a.max(p));
                (
                    ((low.as_dvec3() + high.as_dvec3()) * 0.5)
                        .as_vec3()
                        .to_array(),
                    glam::Mat4::IDENTITY.to_cols_array(),
                    1.0,
                    settings.plain_render.clone(),
                )
            }
        };
        records.push(Record::Mapped(mapped.len()));
        mapped.push(SglMesh {
            render: Some(render),
            center,
            matrix,
            scale,
            positions,
            colors: if keep_absent_rgb(view) {
                None
            } else {
                Some(colors)
            },
            materials: if keep_absent_material(view) {
                None
            } else {
                Some(materials)
            },
            faces,
            uv,
        });
    }
    let mut refs = reserve(records.len(), "object references")?;
    refs.extend(records.iter().map(|r| match r {
        Record::Raw(r) => *r,
        Record::Mapped(index) => &mapped[*index],
    }));
    let bytes = crate::sgl::encode_borrowed_scene(
        3,
        settings.byte_order,
        Some(&settings.display),
        Some(&settings.camera),
        &refs,
    )?;
    if bytes.len() != report.encoded_bytes {
        return Err("SGL preflight/output byte accounting differs".into());
    }
    Ok(SglExport { bytes, report })
}
