//! History-neutral, bounded scene export shared by native and browser hosts.
//! Geometry formats flatten editable world poses; the project format retains
//! the original pose and source metadata.
use crate::{
    AttributeExportPolicy, AttributeExportReport, Document, MaskConvention, Mesh, PlyEncoding,
    SglExportScope, StlEncoding, Vec3, MAX_TRIANGLES, MAX_VERTICES,
};

const OUTPUT_LIMIT: usize = crate::resource_limits::MAX_MESH_INPUT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneGeometryFormat {
    Obj,
    ObjSculptGl(ObjColorOptions),
    Ply(PlyEncoding),
    PlySculptGl,
    Stl(StlEncoding),
    StlSculptGl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjColorOptions {
    pub zbrush: bool,
    pub append_rgb: bool,
}

impl Default for ObjColorOptions {
    fn default() -> Self {
        Self {
            zbrush: true,
            append_rgb: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneGeometryReport {
    pub parts: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub baked_pose_parts: usize,
    pub omitted_source_origin_parts: usize,
    pub attributes: AttributeExportReport,
    pub omitted_color_vertices: usize,
    pub quantized_color_vertices: usize,
    pub quantized_material_vertices: usize,
    pub quantized_mask_vertices: usize,
    pub maximum_color_error: f64,
    pub maximum_material_error: f64,
    pub maximum_mask_error: f64,
}

#[derive(Clone, Debug)]
pub struct SceneGeometryExport {
    pub bytes: Vec<u8>,
    pub report: SceneGeometryReport,
}

fn add_limit(total: usize, amount: usize, limit: usize, label: &str) -> Result<usize, String> {
    let next = total
        .checked_add(amount)
        .ok_or_else(|| format!("{label} overflow"))?;
    if next > limit {
        return Err(format!("Scene export {label} exceeds its limit"));
    }
    Ok(next)
}

fn add_report(
    total: &mut AttributeExportReport,
    part: &AttributeExportReport,
) -> Result<(), String> {
    total.omitted_material_vertices = add_limit(
        total.omitted_material_vertices,
        part.omitted_material_vertices,
        MAX_VERTICES,
        "material rows",
    )?;
    total.omitted_source_mask_vertices = add_limit(
        total.omitted_source_mask_vertices,
        part.omitted_source_mask_vertices,
        MAX_VERTICES,
        "mask rows",
    )?;
    total.omitted_original_faces = add_limit(
        total.omitted_original_faces,
        part.omitted_original_faces,
        MAX_TRIANGLES,
        "original faces",
    )?;
    total.omitted_uv_coordinates = add_limit(
        total.omitted_uv_coordinates,
        part.omitted_uv_coordinates,
        crate::MAX_UV_COORDINATES,
        "UV coordinates",
    )?;
    total.omitted_uv_faces = add_limit(
        total.omitted_uv_faces,
        part.omitted_uv_faces,
        MAX_TRIANGLES,
        "UV faces",
    )?;
    Ok(())
}

fn append(output: &mut String, text: &str) -> Result<(), String> {
    let required = add_limit(output.len(), text.len(), OUTPUT_LIMIT, "OBJ bytes")?;
    output
        .try_reserve(required - output.len())
        .map_err(|_| "Cannot reserve bounded scene OBJ output")?;
    output.push_str(text);
    Ok(())
}

fn obj_with_parts(parts: &[(String, Mesh)]) -> Result<Vec<u8>, String> {
    let mut output = String::from("s 0\n");
    let (mut vertex_offset, mut uv_offset) = (0_usize, 0_usize);
    for (index, (_, mesh)) in parts.iter().enumerate() {
        append(&mut output, &format!("o mesh_{index}\n"))?;
        let source = mesh.export_obj_checked()?;
        for line in source.lines() {
            if let Some(face) = line.strip_prefix("f ") {
                let mut row = String::from("f");
                for corner in face.split_whitespace() {
                    let mut fields = corner.split('/');
                    let vertex = fields
                        .next()
                        .ok_or("Generated OBJ face has no vertex")?
                        .parse::<usize>()
                        .map_err(|_| "Generated OBJ vertex index invalid")?;
                    let uv = fields.next().ok_or("Generated OBJ face has no UV field")?;
                    let normal = fields
                        .next()
                        .ok_or("Generated OBJ face has no normal field")?
                        .parse::<usize>()
                        .map_err(|_| "Generated OBJ normal index invalid")?;
                    let v = vertex
                        .checked_add(vertex_offset)
                        .ok_or("OBJ vertex offset overflow")?;
                    let n = normal
                        .checked_add(vertex_offset)
                        .ok_or("OBJ normal offset overflow")?;
                    if uv.is_empty() {
                        row.push_str(&format!(" {v}//{n}"));
                    } else {
                        let t = uv
                            .parse::<usize>()
                            .map_err(|_| "Generated OBJ UV index invalid")?
                            .checked_add(uv_offset)
                            .ok_or("OBJ UV offset overflow")?;
                        row.push_str(&format!(" {v}/{t}/{n}"));
                    }
                }
                row.push('\n');
                append(&mut output, &row)?;
            } else if line.starts_with("v ") || line.starts_with("vn ") || line.starts_with("vt ") {
                append(&mut output, line)?;
                append(&mut output, "\n")?;
            }
        }
        vertex_offset = add_limit(
            vertex_offset,
            mesh.positions.len(),
            MAX_VERTICES,
            "vertices",
        )?;
        uv_offset = add_limit(
            uv_offset,
            mesh.uv_coordinate_count(),
            crate::MAX_UV_COORDINATES,
            "UV coordinates",
        )?;
    }
    Ok(output.into_bytes())
}

fn byte(value: f32) -> u8 {
    // SculptGL reads f32 channels into JavaScript numbers, then multiplies in
    // binary64 before Math.round. A prior f32 multiply changes half-step cases.
    (f64::from(value.clamp(0.0, 1.0)) * 255.0).round() as u8
}

fn obj_with_source_colors(
    parts: &[(String, Mesh)],
    options: ObjColorOptions,
    report: &mut SceneGeometryReport,
) -> Result<Vec<u8>, String> {
    let mut output = String::from("s 0\n");
    let (mut vertex_offset, mut uv_offset) = (0_usize, 0_usize);
    for (index, (_, mesh)) in parts.iter().enumerate() {
        append(&mut output, &format!("o mesh_{index}\n"))?;
        for (position, color) in mesh.positions.iter().zip(&mesh.colors) {
            let mut row = format!("v {} {} {}", position.x, position.y, position.z);
            if options.append_rgb {
                row.push_str(&format!(" {} {} {}", color[0], color[1], color[2]));
            }
            row.push('\n');
            append(&mut output, &row)?;
        }
        if options.zbrush {
            for chunk in (0..mesh.positions.len()).step_by(64) {
                let mut row = String::from("#MRGB ");
                for color in &mesh.colors[chunk..(chunk + 64).min(mesh.positions.len())] {
                    let rgb = color.map(byte);
                    row.push_str(&format!("ff{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
                    let errors = std::array::from_fn::<_, 3, _>(|i| {
                        (f64::from(rgb[i]) / 255.0 - f64::from(color[i])).abs()
                    });
                    report.maximum_color_error = report
                        .maximum_color_error
                        .max(errors.into_iter().fold(0.0_f64, f64::max));
                    report.quantized_color_vertices +=
                        usize::from(errors.into_iter().any(|error| error > 0.0));
                }
                row.push('\n');
                append(&mut output, &row)?;
            }
            for chunk in (0..mesh.positions.len()).step_by(46) {
                let mut row = String::from("#MAT ");
                for vertex in chunk..(chunk + 46).min(mesh.positions.len()) {
                    let material = mesh
                        .materials
                        .as_ref()
                        .map_or(crate::DEFAULT_VERTEX_MATERIAL, |rows| rows[vertex]);
                    let influence = mesh.mask_influence(vertex);
                    let channels = [material[0], material[1], influence];
                    let encoded = channels.map(byte);
                    row.push_str(&format!(
                        "{:02x}{:02x}{:02x}",
                        encoded[0], encoded[1], encoded[2]
                    ));
                    let errors = std::array::from_fn::<_, 3, _>(|i| {
                        (f64::from(encoded[i]) / 255.0 - f64::from(channels[i])).abs()
                    });
                    report.maximum_material_error =
                        report.maximum_material_error.max(errors[0].max(errors[1]));
                    report.maximum_mask_error = report.maximum_mask_error.max(errors[2]);
                    report.quantized_material_vertices +=
                        usize::from(errors[0] > 0.0 || errors[1] > 0.0);
                    report.quantized_mask_vertices += usize::from(errors[2] > 0.0);
                }
                row.push('\n');
                append(&mut output, &row)?;
            }
        }
        if let Some(uv) = mesh.surface.as_ref().and_then(|s| s.uv.as_ref()) {
            for &[u, v] in &uv.coordinates {
                append(&mut output, &format!("vt {u} {v}\n"))?;
            }
        }
        if let Some(surface) = &mesh.surface {
            for (face_index, face) in surface.faces.iter().enumerate() {
                let count = if face[3] == crate::TRIANGLE_CORNER {
                    3
                } else {
                    4
                };
                let mut row = String::from("f");
                for (corner, &vertex) in face[..count].iter().enumerate() {
                    let v = usize::try_from(vertex)
                        .map_err(|_| "OBJ vertex index overflow")?
                        .checked_add(vertex_offset + 1)
                        .ok_or("OBJ vertex index overflow")?;
                    if let Some(uv) = &surface.uv {
                        let t = usize::try_from(uv.face_indices[face_index][corner])
                            .map_err(|_| "OBJ UV index overflow")?
                            .checked_add(uv_offset + 1)
                            .ok_or("OBJ UV index overflow")?;
                        row.push_str(&format!(" {v}/{t}"));
                    } else {
                        row.push_str(&format!(" {v}"));
                    }
                }
                row.push('\n');
                append(&mut output, &row)?;
            }
        } else {
            for face in &mesh.triangles {
                let mut row = String::from("f");
                for &vertex in face {
                    let v = usize::try_from(vertex)
                        .map_err(|_| "OBJ vertex index overflow")?
                        .checked_add(vertex_offset + 1)
                        .ok_or("OBJ vertex index overflow")?;
                    row.push_str(&format!(" {v}"));
                }
                row.push('\n');
                append(&mut output, &row)?;
            }
        }
        vertex_offset = add_limit(
            vertex_offset,
            mesh.positions.len(),
            MAX_VERTICES,
            "vertices",
        )?;
        uv_offset = add_limit(
            uv_offset,
            mesh.uv_coordinate_count(),
            crate::MAX_UV_COORDINATES,
            "UV coordinates",
        )?;
    }
    Ok(output.into_bytes())
}

fn ply_with_source_colors(
    parts: &[(String, Mesh)],
    report: &mut SceneGeometryReport,
) -> Result<Vec<u8>, String> {
    let mut vertices = 0_usize;
    let mut faces = 0_usize;
    let mut face_bytes = 0_usize;
    let mut quads = 0_usize;
    for (_, mesh) in parts {
        vertices = add_limit(vertices, mesh.positions.len(), MAX_VERTICES, "vertices")?;
        let source_faces = mesh
            .surface
            .as_ref()
            .map_or(mesh.triangles.len(), |s| s.faces.len());
        faces = add_limit(faces, source_faces, MAX_TRIANGLES, "faces")?;
        if let Some(surface) = &mesh.surface {
            for face in &surface.faces {
                quads += usize::from(face[3] != crate::TRIANGLE_CORNER);
                face_bytes = add_limit(
                    face_bytes,
                    if face[3] == crate::TRIANGLE_CORNER {
                        13
                    } else {
                        17
                    },
                    OUTPUT_LIMIT,
                    "PLY face bytes",
                )?;
            }
        } else {
            face_bytes = add_limit(
                face_bytes,
                mesh.triangles
                    .len()
                    .checked_mul(13)
                    .ok_or("PLY face byte overflow")?,
                OUTPUT_LIMIT,
                "PLY face bytes",
            )?;
        }
    }
    let header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment created by SculptGL\nelement vertex {vertices}\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face {faces}\nproperty list uchar uint vertex_indices\nend_header\n"
    );
    let body_bytes = vertices.checked_mul(15).ok_or("PLY vertex byte overflow")?;
    // The pinned exporter allocates twice an indexSize that counts each quad
    // both as four polygon indices and as two triangulated faces. The actual
    // writer emits only the four polygon indices; the remaining bytes are zero.
    let reserved_face_bytes = face_bytes
        .checked_add(quads.checked_mul(24).ok_or("PLY quad byte overflow")?)
        .ok_or("PLY face byte overflow")?;
    let total = body_bytes
        .checked_add(
            reserved_face_bytes
                .checked_mul(2)
                .ok_or("PLY face byte overflow")?,
        )
        .and_then(|size| size.checked_add(header.len()))
        .ok_or("PLY output size overflow")?;
    if total > OUTPUT_LIMIT {
        return Err("Scene geometry output exceeds 128 MiB".into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| "Cannot reserve bounded scene PLY output")?;
    output.extend_from_slice(header.as_bytes());
    for (_, mesh) in parts {
        for (position, color) in mesh.positions.iter().zip(&mesh.colors) {
            for component in position.to_array() {
                output.extend_from_slice(&component.to_le_bytes());
            }
            for &component in color {
                let encoded = byte(component);
                output.push(encoded);
                let error = (f64::from(encoded) / 255.0 - f64::from(component)).abs();
                report.maximum_color_error = report.maximum_color_error.max(error);
            }
            report.quantized_color_vertices += usize::from(color.iter().any(|&component| {
                (f64::from(byte(component)) / 255.0 - f64::from(component)).abs() > 0.0
            }));
        }
    }
    let mut offset = 0_u32;
    for (_, mesh) in parts {
        if let Some(surface) = &mesh.surface {
            for face in &surface.faces {
                let count = if face[3] == crate::TRIANGLE_CORNER {
                    3
                } else {
                    4
                };
                output.push(count as u8);
                for &index in &face[..count] {
                    output.extend_from_slice(
                        &index
                            .checked_add(offset)
                            .ok_or("PLY face index overflow")?
                            .to_le_bytes(),
                    );
                }
            }
        } else {
            for face in &mesh.triangles {
                output.push(3);
                for &index in face {
                    output.extend_from_slice(
                        &index
                            .checked_add(offset)
                            .ok_or("PLY face index overflow")?
                            .to_le_bytes(),
                    );
                }
            }
        }
        offset = offset
            .checked_add(
                u32::try_from(mesh.positions.len()).map_err(|_| "PLY vertex count overflow")?,
            )
            .ok_or("PLY vertex offset overflow")?;
    }
    output.resize(total, 0);
    Ok(output)
}

fn joined_mesh(parts: &[(String, Mesh)]) -> Result<Mesh, String> {
    let vertices = parts.iter().try_fold(0_usize, |n, (_, mesh)| {
        add_limit(n, mesh.positions.len(), MAX_VERTICES, "vertices")
    })?;
    let triangles = parts.iter().try_fold(0_usize, |n, (_, mesh)| {
        add_limit(n, mesh.triangles.len(), MAX_TRIANGLES, "triangles")
    })?;
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    let mut colors = Vec::new();
    positions
        .try_reserve_exact(vertices)
        .map_err(|_| "Cannot reserve scene export vertices")?;
    faces
        .try_reserve_exact(triangles)
        .map_err(|_| "Cannot reserve scene export triangles")?;
    colors
        .try_reserve_exact(vertices)
        .map_err(|_| "Cannot reserve scene export colors")?;
    for (_, mesh) in parts {
        let offset =
            u32::try_from(positions.len()).map_err(|_| "Scene export vertex offset exceeds u32")?;
        positions.extend_from_slice(&mesh.positions);
        colors.extend_from_slice(&mesh.colors);
        for face in &mesh.triangles {
            faces.push(face.map(|index| index.checked_add(offset).expect("preflight bounds")));
        }
    }
    let mut combined = Mesh::new(positions, faces);
    combined.colors = colors;
    combined.validate()?;
    Ok(combined)
}

impl Document {
    /// Exports all scene parts or the explicit selection, including hidden parts.
    /// Source state and history are never changed. PLY/STL flatten part identity;
    /// OBJ retains one object record per selected part.
    pub fn export_scene_geometry(
        &self,
        scope: SglExportScope,
        format: SceneGeometryFormat,
        policy: AttributeExportPolicy,
    ) -> Result<SceneGeometryExport, String> {
        if !matches!(
            scope,
            SglExportScope::AllScene | SglExportScope::SelectedParts
        ) {
            return Err("Geometry export supports All or Selected scope".into());
        }
        let views = self.scoped_sgl_export_views(scope)?;
        if views.is_empty() {
            return Err("Geometry export requires at least one part".into());
        }
        let mut report = SceneGeometryReport::default();
        let mut parts = Vec::new();
        parts
            .try_reserve_exact(views.len())
            .map_err(|_| "Cannot reserve scene export parts")?;
        for view in views {
            view.mesh.validate()?;
            report.parts += 1;
            report.vertices = add_limit(
                report.vertices,
                view.mesh.positions.len(),
                MAX_VERTICES,
                "vertices",
            )?;
            report.triangles = add_limit(
                report.triangles,
                view.mesh.triangles.len(),
                MAX_TRIANGLES,
                "triangles",
            )?;
            let preserve_surface = matches!(
                format,
                SceneGeometryFormat::Obj
                    | SceneGeometryFormat::ObjSculptGl(_)
                    | SceneGeometryFormat::PlySculptGl
            );
            let preserve_uv = matches!(
                format,
                SceneGeometryFormat::Obj | SceneGeometryFormat::ObjSculptGl(_)
            );
            let source_colors = match format {
                SceneGeometryFormat::ObjSculptGl(options) => Some(options),
                _ => None,
            };
            let writes_mat = source_colors.is_some_and(|options| options.zbrush);
            let nonclear_mask =
                view.mesh.masks.iter().any(|value| {
                    value.to_bits() != view.mesh.mask_convention.clear_value().to_bits()
                });
            if source_colors.is_some_and(|options| !options.zbrush && !options.append_rgb) {
                report.omitted_color_vertices = add_limit(
                    report.omitted_color_vertices,
                    view.mesh.colors.len(),
                    MAX_VERTICES,
                    "omitted color rows",
                )?;
            }
            add_report(
                &mut report.attributes,
                &AttributeExportReport {
                    omitted_material_vertices: if writes_mat {
                        0
                    } else {
                        view.mesh.materials.as_ref().map_or(0, Vec::len)
                    },
                    omitted_source_mask_vertices: if writes_mat
                        || (view.mesh.mask_convention.is_default() && !nonclear_mask)
                    {
                        0
                    } else {
                        view.mesh.masks.len()
                    },
                    omitted_original_faces: if preserve_surface {
                        0
                    } else {
                        view.mesh.surface.as_ref().map_or(0, |s| s.faces.len())
                    },
                    omitted_uv_coordinates: if preserve_uv {
                        0
                    } else {
                        view.mesh.uv_coordinate_count()
                    },
                    omitted_uv_faces: if preserve_uv {
                        0
                    } else {
                        view.mesh
                            .surface
                            .as_ref()
                            .and_then(|s| s.uv.as_ref())
                            .map_or(0, |uv| uv.face_indices.len())
                    },
                },
            )?;
            if let Some(meta) = view.metadata {
                meta.validate(view.mesh)?;
                report.baked_pose_parts += 1;
                report.omitted_source_origin_parts += usize::from(meta.origin.is_some());
            }
            let mut mesh = view.mesh.snapshot();
            if let Some(meta) = view.metadata {
                for point in &mut mesh.positions {
                    let world = meta.affine.world_point_validated(*point)?;
                    let world = Vec3::new(world.x as f32, world.y as f32, world.z as f32);
                    if !world.is_finite() || world.abs().max_element() > 1_000_000.0 {
                        return Err("Baked world coordinate exceeds mesh limits".into());
                    }
                    *point = world;
                }
                mesh.recompute_normals();
            }
            // Attributes and provenance omitted by geometry formats are removed
            // only from the temporary writer copy, never the source document.
            if !writes_mat {
                mesh.materials = None;
                mesh.mask_convention = MaskConvention::ProtectedF32V1;
                mesh.masks.fill(0.0);
            }
            if !preserve_surface {
                mesh.surface = None;
            }
            mesh.validate()?;
            parts.push((view.name.to_owned(), mesh));
        }
        if policy == AttributeExportPolicy::RejectLoss
            && (report.attributes != AttributeExportReport::default()
                || report.omitted_color_vertices > 0
                || report.baked_pose_parts > 0)
        {
            return Err("Geometry export omits attributes or source pose/provenance; enable explicit loss reporting".into());
        }
        let bytes = match format {
            SceneGeometryFormat::Obj => obj_with_parts(&parts)?,
            SceneGeometryFormat::ObjSculptGl(options) => {
                obj_with_source_colors(&parts, options, &mut report)?
            }
            SceneGeometryFormat::Ply(encoding) => joined_mesh(&parts)?.to_ply(encoding)?,
            SceneGeometryFormat::PlySculptGl => ply_with_source_colors(&parts, &mut report)?,
            SceneGeometryFormat::Stl(encoding) => joined_mesh(&parts)?.to_stl(encoding)?,
            SceneGeometryFormat::StlSculptGl => joined_mesh(&parts)?.to_stl_sculptgl()?,
        };
        if bytes.len() > OUTPUT_LIMIT {
            return Err("Scene geometry output exceeds 128 MiB".into());
        }
        Ok(SceneGeometryExport { bytes, report })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectAffineF32V1, SceneObjectMetadata, SglPickingPolicy};

    #[test]
    fn source_obj_quantization_uses_binary64_multiply_at_half_step() {
        assert_eq!(byte(f32::from_bits(0x3f01_0101)), 128);
    }

    #[test]
    fn sculptgl_binary_ply_keeps_quads_rgb_and_source_padding_across_parts() {
        let quad = Mesh::import_obj(b"v 0 0 0 0.2 0.4 0.6\nv 1 0 0 0.1 0.3 0.5\nv 1 1 0 0.25 0.5 0.75\nv 0 1 0 1 0 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\n")
            .unwrap().mesh;
        let triangle = triangle(2.0, [0.0, 0.5, 1.0]);
        let mut document = Document::new(quad.clone());
        document.add_subtool(triangle.clone(), "triangle").unwrap();
        let before = document.to_json().unwrap();
        let format = SceneGeometryFormat::PlySculptGl;
        assert!(document
            .export_scene_geometry(
                SglExportScope::AllScene,
                format,
                AttributeExportPolicy::RejectLoss,
            )
            .is_err());
        let export = document
            .export_scene_geometry(
                SglExportScope::AllScene,
                format,
                AttributeExportPolicy::AllowAndReport,
            )
            .unwrap();
        assert_eq!(export.report.attributes.omitted_original_faces, 0);
        assert_eq!(export.report.attributes.omitted_uv_coordinates, 4);
        assert_eq!(export.report.attributes.omitted_uv_faces, 1);
        let header = b"ply\nformat binary_little_endian 1.0\ncomment created by SculptGL\nelement vertex 7\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face 2\nproperty list uchar uint vertex_indices\nend_header\n";
        assert!(export.bytes.starts_with(header));
        let body = &export.bytes[header.len()..];
        assert_eq!(body.len(), 7 * 15 + 2 * (17 + 13 + 24));
        assert_eq!(&body[12..15], &[51, 102, 153]);
        assert_eq!(&body[4 * 15 + 12..4 * 15 + 15], &[0, 128, 255]);
        let faces = &body[7 * 15..7 * 15 + 30];
        assert_eq!(faces[0], 4);
        assert_eq!(faces[17], 3);
        for (start, expected) in [
            (1, 0_u32),
            (5, 1),
            (9, 2),
            (13, 3),
            (18, 4),
            (22, 5),
            (26, 6),
        ] {
            assert_eq!(
                u32::from_le_bytes(faces[start..start + 4].try_into().unwrap()),
                expected
            );
        }
        assert!(body[7 * 15 + 30..].iter().all(|&byte| byte == 0));
        let imported = Mesh::import_ply(&export.bytes).unwrap();
        assert_eq!(imported.report.source_faces, 2);
        assert_eq!(imported.report.triangulated_faces, 1);
        assert_eq!(imported.mesh.triangles.len(), 3);
        assert_eq!(imported.mesh.positions.len(), 7);
        assert_eq!(
            imported.mesh.surface.as_ref().unwrap().faces,
            vec![[0, 1, 2, 3], [4, 5, 6, crate::TRIANGLE_CORNER]]
        );
        assert_eq!(document.to_json().unwrap(), before);
    }

    #[test]
    fn protected_default_mask_is_reported_if_the_obj_switches_omit_it() {
        let mut mesh = triangle(0.0, [0.4, 0.5, 0.6]);
        mesh.masks[0] = 0.5;
        let document = Document::new(mesh);
        let format = SceneGeometryFormat::ObjSculptGl(ObjColorOptions {
            zbrush: false,
            append_rgb: true,
        });
        assert!(document
            .export_scene_geometry(
                SglExportScope::AllScene,
                format,
                AttributeExportPolicy::RejectLoss
            )
            .is_err());
        let report = document
            .export_scene_geometry(
                SglExportScope::AllScene,
                format,
                AttributeExportPolicy::AllowAndReport,
            )
            .unwrap()
            .report;
        assert_eq!(report.attributes.omitted_source_mask_vertices, 3);
    }

    #[test]
    fn sculptgl_obj_color_switches_roundtrip_material_mask_and_rgb_with_declared_quantization() {
        let mut mesh = Mesh::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.0)],
            vec![[0, 1, 2], [1, 3, 2]],
        );
        mesh.colors = vec![
            [0.0, 0.1, 0.5],
            [1.0, 0.25, 0.75],
            [0.4, 0.6, 0.8],
            [0.9, 0.3, 0.2],
        ];
        mesh.materials = Some(vec![[0.18, 0.08], [0.25, 0.75], [0.5, 0.125], [1.0, 0.0]]);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks = vec![1.0, 0.75, 0.5, 0.0];
        mesh.validate().unwrap();
        let document = Document::new(mesh.snapshot());
        let before = document.to_json().unwrap();
        for zbrush in [false, true] {
            for append_rgb in [false, true] {
                let options = ObjColorOptions { zbrush, append_rgb };
                let policy = if zbrush {
                    AttributeExportPolicy::RejectLoss
                } else {
                    AttributeExportPolicy::AllowAndReport
                };
                let export = document
                    .export_scene_geometry(
                        SglExportScope::AllScene,
                        SceneGeometryFormat::ObjSculptGl(options),
                        policy,
                    )
                    .unwrap();
                let text = String::from_utf8(export.bytes.clone()).unwrap();
                assert_eq!(
                    text.lines().filter(|line| line.starts_with("v ")).count(),
                    4
                );
                assert_eq!(
                    text.lines().filter(|line| line.starts_with("f ")).count(),
                    2
                );
                assert_eq!(text.contains("#MRGB "), zbrush);
                assert_eq!(text.contains("#MAT "), zbrush);
                assert_eq!(
                    text.lines()
                        .find(|line| line.starts_with("v "))
                        .unwrap()
                        .split_whitespace()
                        .count(),
                    if append_rgb { 7 } else { 4 }
                );
                if zbrush {
                    assert_eq!(export.report.attributes.omitted_material_vertices, 0);
                    assert_eq!(export.report.attributes.omitted_source_mask_vertices, 0);
                    let parsed = Mesh::import_obj_with_mask_convention(
                        &export.bytes,
                        MaskConvention::UnprotectedF32V1,
                    )
                    .unwrap()
                    .mesh;
                    let materials = parsed.materials.unwrap();
                    for (vertex, material) in materials.iter().enumerate() {
                        for channel in 0..3 {
                            assert!(
                                (parsed.colors[vertex][channel] - mesh.colors[vertex][channel])
                                    .abs()
                                    <= 1.0 / 255.0
                            );
                        }
                        for (channel, actual) in material.iter().enumerate() {
                            assert!(
                                (*actual - mesh.materials.as_ref().unwrap()[vertex][channel]).abs()
                                    <= 1.0 / 255.0
                            );
                        }
                        assert!((parsed.masks[vertex] - mesh.masks[vertex]).abs() <= 1.0 / 255.0);
                    }
                    assert!(export.report.maximum_color_error <= 1.0 / 255.0);
                    assert!(export.report.maximum_material_error <= 1.0 / 255.0);
                    assert!(export.report.maximum_mask_error <= 1.0 / 255.0);
                } else {
                    assert_eq!(export.report.attributes.omitted_material_vertices, 4);
                    assert_eq!(export.report.attributes.omitted_source_mask_vertices, 4);
                    assert_eq!(
                        export.report.omitted_color_vertices,
                        if append_rgb { 0 } else { 4 }
                    );
                }
                assert_eq!(document.to_json().unwrap(), before);
            }
        }
    }

    #[test]
    fn sculptgl_obj_writer_offsets_corner_uvs_across_objects_and_bounds_color_chunks() {
        let quad = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\n").unwrap().mesh;
        let mut document = Document::new(quad.snapshot());
        document.add_subtool(quad, "second quad").unwrap();
        let output = document
            .export_scene_geometry(
                SglExportScope::AllScene,
                SceneGeometryFormat::ObjSculptGl(ObjColorOptions::default()),
                AttributeExportPolicy::RejectLoss,
            )
            .unwrap();
        let text = String::from_utf8(output.bytes).unwrap();
        assert!(text.contains("f 1/1 2/2 3/3 4/4\n"));
        assert!(text.contains("f 5/5 6/6 7/7 8/8\n"));
        assert_eq!(
            text.lines().filter(|line| line.starts_with("vt ")).count(),
            8
        );
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("#MAT "))
                .count(),
            2
        );

        let positions: Vec<_> = (0..70)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::TAU / 70.0;
                Vec3::new(angle.cos(), angle.sin(), 0.0)
            })
            .collect();
        let triangles: Vec<_> = (1..69).map(|i| [0, i, i + 1]).collect();
        let many = Document::new(Mesh::new(positions, triangles));
        let output = many
            .export_scene_geometry(
                SglExportScope::AllScene,
                SceneGeometryFormat::ObjSculptGl(ObjColorOptions::default()),
                AttributeExportPolicy::RejectLoss,
            )
            .unwrap();
        let text = String::from_utf8(output.bytes).unwrap();
        let mrgb: Vec<_> = text
            .lines()
            .filter_map(|line| line.strip_prefix("#MRGB "))
            .collect();
        let mat: Vec<_> = text
            .lines()
            .filter_map(|line| line.strip_prefix("#MAT "))
            .collect();
        assert_eq!(
            mrgb.iter().map(|line| line.len()).collect::<Vec<_>>(),
            [64 * 8, 6 * 8]
        );
        assert_eq!(
            mat.iter().map(|line| line.len()).collect::<Vec<_>>(),
            [46 * 6, 24 * 6]
        );
    }

    fn triangle(x: f32, color: [f32; 3]) -> Mesh {
        let mut mesh = Mesh::new(
            vec![
                Vec3::new(x, 0.0, 0.0),
                Vec3::new(x + 1.0, 0.0, 0.0),
                Vec3::new(x, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        mesh.colors.fill(color);
        mesh
    }

    #[test]
    fn selected_hidden_world_pose_exports_all_mesh_formats_without_history_changes() {
        let mut document = Document::new(triangle(0.0, [1.0, 0.0, 0.0]));
        let first = document.active_subtool_id().unwrap();
        let middle = document
            .add_subtool(triangle(3.0, [0.0, 1.0, 0.0]), "middle")
            .unwrap();
        let posed = triangle(1.0, [0.0, 0.0, 1.0]);
        let mut matrix = [0.0_f32; 16];
        matrix[0] = 1.0;
        matrix[5] = 1.0;
        matrix[10] = 1.0;
        matrix[15] = 1.0;
        matrix[12] = 10.0;
        let metadata = SceneObjectMetadata {
            source_center_initialized: true,
            symmetry_offset: 0.0,
            affine: ObjectAffineF32V1::from_local_points(matrix, &posed.positions).unwrap(),
            render: None,
            picking_policy: SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let last = document
            .add_subtool_with_metadata(posed, "posed", false, metadata)
            .unwrap();
        document.set_subtool_visible(last, false).unwrap();
        document
            .select_subtools(&[first, last], Some(first))
            .unwrap();
        let before = document.to_json().unwrap();
        let can_undo = document.can_undo();
        let can_redo = document.can_redo();
        let formats = [
            SceneGeometryFormat::Obj,
            SceneGeometryFormat::Ply(PlyEncoding::Ascii),
            SceneGeometryFormat::Ply(PlyEncoding::BinaryLittleEndian),
            SceneGeometryFormat::Stl(StlEncoding::Ascii),
            SceneGeometryFormat::Stl(StlEncoding::Binary),
            SceneGeometryFormat::StlSculptGl,
        ];
        for format in formats {
            assert!(document
                .export_scene_geometry(
                    SglExportScope::SelectedParts,
                    format,
                    AttributeExportPolicy::RejectLoss
                )
                .is_err());
            let selected = document
                .export_scene_geometry(
                    SglExportScope::SelectedParts,
                    format,
                    AttributeExportPolicy::AllowAndReport,
                )
                .unwrap();
            assert_eq!(selected.report.parts, 2);
            assert_eq!(selected.report.vertices, 6);
            assert_eq!(selected.report.triangles, 2);
            assert_eq!(selected.report.baked_pose_parts, 1);
            assert!(selected.report.attributes == AttributeExportReport::default());
            let imported = match format {
                SceneGeometryFormat::Obj | SceneGeometryFormat::ObjSculptGl(_) => {
                    let text = String::from_utf8(selected.bytes.clone()).unwrap();
                    assert!(text.contains("o mesh_0\n"));
                    assert!(text.contains("o mesh_1\n"));
                    assert!(!text.contains("o mesh_2\n"));
                    Mesh::import_obj(&selected.bytes).unwrap().mesh
                }
                SceneGeometryFormat::Ply(_) | SceneGeometryFormat::PlySculptGl => {
                    Mesh::import_ply(&selected.bytes).unwrap().mesh
                }
                SceneGeometryFormat::Stl(_) | SceneGeometryFormat::StlSculptGl => {
                    Mesh::import_stl(&selected.bytes).unwrap().mesh
                }
            };
            assert_eq!(imported.triangles.len(), 2);
            assert!(imported
                .positions
                .iter()
                .any(|p| (p.x - 11.0).abs() < 0.0001));
            assert!(!imported
                .positions
                .iter()
                .any(|p| (p.x - 3.0).abs() < 0.0001));
            assert_eq!(document.to_json().unwrap(), before);
            assert_eq!(document.can_undo(), can_undo);
            assert_eq!(document.can_redo(), can_redo);
        }
        let all = document
            .export_scene_geometry(
                SglExportScope::AllScene,
                SceneGeometryFormat::Obj,
                AttributeExportPolicy::AllowAndReport,
            )
            .unwrap();
        assert_eq!(all.report.parts, 3);
        assert!(String::from_utf8(all.bytes).unwrap().contains("o mesh_2\n"));
        assert_eq!(document.subtool_mesh(middle).unwrap().positions.len(), 3);
    }
}
