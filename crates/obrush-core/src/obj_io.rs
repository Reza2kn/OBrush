//! Bounded OBJ import with original quads and indexed corner UVs.
//!
//! Imported normal indices are checked, then normals are derived from geometry,
//! as in the pinned SculptGL importer. Ordinary Mesh import combines object
//! declarations; the separate Source File Add path retains their boundaries.
use crate::mesh::{canonical_quad_valid, triangulate_polygon, MAX_INPUT_BYTES};
use crate::{
    CornerUv, Mesh, MeshFileFormat, MeshImport, MeshImportReport, SurfaceTopology, Vec3,
    MAX_MESH_IO_SCALARS, MAX_PLY_FACE_VERTICES, MAX_PLY_TRIANGULATION_WORK, MAX_TRIANGLES,
    MAX_UV_COORDINATES, MAX_VERTICES, TRIANGLE_CORNER,
};

#[path = "obj_parts.rs"]
mod parts;

#[derive(Default)]
struct Bounds {
    vertices: usize,
    normals: usize,
    coordinates: usize,
    input_faces: usize,
    triangles: usize,
    mrgb: usize,
    materials: usize,
    has_uv: bool,
    has_quads: bool,
}

impl Bounds {
    /// Count output and scratch before any potentially large channel allocation.
    fn scan(source: &str) -> Result<Self, String> {
        let mut bounds = Self::default();
        let mut work = 0_usize;
        let mut scalars = 0_usize;
        for (line, raw) in source.lines().enumerate() {
            let error = |message| failure(line, message);
            let mut words = raw.split_whitespace();
            match words.next() {
                Some("#MRGB") => bounds.mrgb += hex_count(words, 8).map_err(error)?,
                Some("#MAT") => bounds.materials += hex_count(words, 6).map_err(error)?,
                _ => {
                    let mut words = raw.split('#').next().unwrap_or("").split_whitespace();
                    match words.next() {
                        Some("v") => {
                            bounds.vertices += 1;
                            scalars += words.count();
                        }
                        Some("vn") => {
                            bounds.normals += 1;
                            scalars += words.count();
                        }
                        Some("vt") => {
                            bounds.coordinates += 1;
                            scalars += words.count();
                        }
                        Some("f") => {
                            let mut arity = 0_usize;
                            for word in words {
                                arity += 1;
                                if arity > MAX_PLY_FACE_VERTICES {
                                    return Err(error("face exceeds 512 corners"));
                                }
                                bounds.has_uv |=
                                    word.split('/').nth(1).is_some_and(|s| !s.is_empty());
                            }
                            if arity < 3 {
                                return Err(error("face needs at least three vertices"));
                            }
                            bounds.input_faces += 1;
                            bounds.triangles += arity - 2;
                            bounds.has_quads |= arity == 4;
                            scalars += arity * 3;
                            if arity > 3 {
                                work += arity * arity * arity;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if bounds.vertices > MAX_VERTICES
                || bounds.normals > MAX_VERTICES
                || bounds.mrgb > MAX_VERTICES
                || bounds.materials > MAX_VERTICES
                || bounds.coordinates > MAX_UV_COORDINATES
                || bounds.triangles > MAX_TRIANGLES
                || scalars > MAX_MESH_IO_SCALARS
                || work > MAX_PLY_TRIANGULATION_WORK
            {
                return Err(error(
                    "channel count or polygon work exceeds the import limit",
                ));
            }
        }
        if bounds.vertices == 0 || bounds.triangles == 0 {
            return Err("OBJ requires vertices and faces".into());
        }
        // Mesh channels plus parsed color/material streams and normal scratch.
        // Faces allow a concave quad to become two triangles without growing.
        let bytes = bounds.vertices * 76
            + bounds.triangles * 12
            + bounds.mrgb * 12
            + bounds.materials * 12
            + if bounds.has_uv || bounds.has_quads {
                bounds.triangles * 16
                    + if bounds.has_uv {
                        bounds.coordinates * 8 + bounds.triangles * 16
                    } else {
                        0
                    }
            } else {
                0
            };
        if bytes > MAX_INPUT_BYTES {
            return Err("OBJ channels and import scratch exceed 128 MiB".into());
        }
        Ok(bounds)
    }
}

impl Mesh {
    /// Import one mesh while retaining corner UV identities and original quads.
    /// Partial UV channels fail explicitly. Concave/noncanonical quads and larger
    /// polygons are triangulated with their corner UVs and reported. Forward
    /// references are rejected; positive and relative indices use declarations
    /// already seen at the face. #MAT is SculptGL's roughness/metallic/unmasked
    /// byte stream; #MRGB's leading mask byte is validated but not interpreted.
    pub fn import_obj(source: &[u8]) -> Result<MeshImport, String> {
        Self::import_obj_with_mask_convention(source, crate::MaskConvention::ProtectedF32V1)
    }

    /// #MAT source-z can be retained exactly. Files without #MAT keep the legacy default.
    pub fn import_obj_with_mask_convention(
        source: &[u8],
        convention: crate::MaskConvention,
    ) -> Result<MeshImport, String> {
        if source.is_empty() || source.len() > MAX_INPUT_BYTES {
            return Err("OBJ requires nonempty input within 128 MiB".into());
        }
        let source = std::str::from_utf8(source).map_err(|_| "OBJ must be UTF-8 text")?;
        let bounds = Bounds::scan(source)?;
        let retain_surface = bounds.has_uv || bounds.has_quads;
        let mut positions = allocate(bounds.vertices)?;
        let mut colors = allocate(bounds.vertices)?;
        let mut triangles = allocate(bounds.triangles)?;
        let mut coordinates = allocate(if bounds.has_uv { bounds.coordinates } else { 0 })?;
        let mut faces = allocate(if retain_surface { bounds.triangles } else { 0 })?;
        let mut uv_faces = allocate(if bounds.has_uv { bounds.triangles } else { 0 })?;
        let mut mrgb = allocate(bounds.mrgb)?;
        let mut materials = allocate(bounds.materials)?;
        let mut seen_uv = 0;
        let mut seen_normals = 0;
        let mut inline_colors = false;
        let mut skipped = Vec::new();
        let mut warnings = Vec::new();
        let mut objects = 0;
        let mut polygon_count = 0;
        let mut converted_quads = 0;
        let mut uv_w = false;
        for (line, raw) in source.lines().enumerate() {
            let error = |message| failure(line, message);
            let mut comments = raw.split_whitespace();
            match comments.next() {
                Some("#MRGB") => {
                    hex_records(comments, 8, |bytes| {
                        mrgb.push([
                            bytes[1] as f32 / 255.0,
                            bytes[2] as f32 / 255.0,
                            bytes[3] as f32 / 255.0,
                        ]);
                    })
                    .map_err(error)?;
                    continue;
                }
                Some("#MAT") => {
                    hex_records(comments, 6, |bytes| {
                        materials.push([
                            bytes[0] as f32 / 255.0,
                            bytes[1] as f32 / 255.0,
                            bytes[2] as f32 / 255.0,
                        ]);
                    })
                    .map_err(error)?;
                    continue;
                }
                _ => {}
            }
            let mut words = raw.split('#').next().unwrap_or("").split_whitespace();
            let Some(kind) = words.next() else { continue };
            match kind {
                "v" => {
                    let values = numbers::<7>(words).map_err(error)?;
                    if !matches!(values.len(), 3 | 4 | 6 | 7) {
                        return Err(error("vertex requires xyz, optional w and optional RGB"));
                    }
                    let mut position = Vec3::new(values[0], values[1], values[2]);
                    let offset = if values.len() == 4 || values.len() == 7 {
                        if values[3].abs() < 1e-12 {
                            return Err(error("invalid homogeneous coordinate"));
                        }
                        position /= values[3];
                        4
                    } else {
                        3
                    };
                    if !position.is_finite() || position.abs().max_element() > 1_000_000.0 {
                        return Err(error("non-finite or excessive coordinate"));
                    }
                    let color = if values.len() >= 6 {
                        inline_colors = true;
                        [values[offset], values[offset + 1], values[offset + 2]]
                    } else {
                        // The pinned SculptGL OBJ importer initializes absent
                        // vertex colors to white. The older from_obj path and
                        // existing project data retain their own color policy.
                        [1.0, 1.0, 1.0]
                    };
                    if color.iter().any(|c| !(0.0..=1.0).contains(c)) {
                        return Err(error("RGB channels must be in 0–1"));
                    }
                    positions.push(position);
                    colors.push(color);
                }
                "vt" => {
                    let values = numbers::<3>(words).map_err(error)?;
                    if values.is_empty() || values.iter().any(|v| v.abs() > 1_000_000.0) {
                        return Err(error("UV requires 1–3 finite coordinates within ±1000000"));
                    }
                    if bounds.has_uv {
                        coordinates.push([values[0], values.get(1).copied().unwrap_or(0.0)]);
                    }
                    uv_w |= values.len() == 3;
                    seen_uv += 1;
                }
                "vn" => {
                    let values = numbers::<3>(words).map_err(error)?;
                    if values.len() != 3
                        || !Vec3::new(values[0], values[1], values[2])
                            .length_squared()
                            .is_finite()
                    {
                        return Err(error("normal requires three finite bounded components"));
                    }
                    seen_normals += 1;
                }
                "f" => {
                    let mut polygon = [0_u32; MAX_PLY_FACE_VERTICES];
                    let mut texture = [0_u32; MAX_PLY_FACE_VERTICES];
                    let mut arity = 0;
                    for word in words {
                        let mut components = word.split('/');
                        let vertex = obj_index(components.next().unwrap_or(""), positions.len())
                            .map_err(error)?;
                        if polygon[..arity].contains(&vertex) {
                            return Err(error("face repeats a vertex"));
                        }
                        polygon[arity] = vertex;
                        let uv = components.next();
                        if bounds.has_uv {
                            texture[arity] =
                                obj_index(uv.unwrap_or(""), seen_uv).map_err(|_| {
                                    error("partial UV channel or missing texture coordinate")
                                })?;
                        } else if uv.is_some_and(|s| !s.is_empty()) {
                            return Err(error("missing texture coordinate"));
                        }
                        if let Some(normal) = components.next() {
                            if !normal.is_empty() {
                                obj_index(normal, seen_normals).map_err(error)?;
                            }
                        }
                        if components.next().is_some() {
                            return Err(error("face has more than three index components"));
                        }
                        arity += 1;
                    }
                    let polygon = &polygon[..arity];
                    let generated =
                        triangulate_polygon(polygon, &positions).map_err(|e| failure(line, &e))?;
                    let canonical_quad = arity == 4 && canonical_quad_valid(polygon, &positions);
                    if canonical_quad {
                        triangles.extend([
                            [polygon[0], polygon[1], polygon[2]],
                            [polygon[0], polygon[2], polygon[3]],
                        ]);
                        faces.push([polygon[0], polygon[1], polygon[2], polygon[3]]);
                        if bounds.has_uv {
                            uv_faces.push([texture[0], texture[1], texture[2], texture[3]]);
                        }
                    } else {
                        if arity == 4 {
                            converted_quads += 1;
                        }
                        if arity > 4 {
                            polygon_count += 1;
                        }
                        for triangle in generated {
                            triangles.push(triangle);
                            if retain_surface {
                                faces.push([
                                    triangle[0],
                                    triangle[1],
                                    triangle[2],
                                    TRIANGLE_CORNER,
                                ]);
                            }
                            if bounds.has_uv {
                                let uv = triangle.map(|vertex| {
                                    texture[polygon
                                        .iter()
                                        .position(|&v| v == vertex)
                                        .expect("generated corner")]
                                });
                                uv_faces.push([uv[0], uv[1], uv[2], TRIANGLE_CORNER]);
                            }
                        }
                    }
                }
                "o" => {
                    objects += 1;
                }
                _ => {
                    if !skipped.iter().any(|s| s == kind) && skipped.len() < 64 {
                        skipped.push(kind.chars().take(64).collect::<String>());
                    }
                }
            }
        }
        if !mrgb.is_empty() {
            if mrgb.len() != positions.len() {
                return Err("OBJ MRGB color count does not match vertices".into());
            }
            colors = mrgb;
            warnings.push(
                "#MRGB RGB was imported; its leading mask byte was validated but not interpreted."
                    .into(),
            );
        }
        if !materials.is_empty() && materials.len() != positions.len() {
            return Err("OBJ #MAT material count does not match vertices".into());
        }
        let imported_colors = inline_colors || bounds.mrgb > 0;
        if objects > 1 {
            warnings.push("OBJ object boundaries were combined into one mesh.".into());
        }
        if seen_normals > 0 {
            warnings.push(
                "OBJ normal indices were validated; vertex normals were recomputed from geometry."
                    .into(),
            );
        }
        if uv_w {
            warnings.push(
                "OBJ texture w components were validated and omitted; UV uses u and v.".into(),
            );
        }
        if seen_uv > 0 && !bounds.has_uv {
            warnings.push("Unreferenced OBJ texture coordinates were omitted.".into());
        }
        if polygon_count > 0 || converted_quads > 0 {
            warnings.push(format!("Triangulated {polygon_count} polygons and {converted_quads} quads whose 0–2 diagonal is unsuitable; corner UVs were retained."));
        }
        let mut mesh = Mesh::new(positions, triangles);
        mesh.colors = colors;
        if !materials.is_empty() {
            mesh.masks = materials
                .iter()
                .map(|m| match convention {
                    crate::MaskConvention::ProtectedF32V1 => 1.0 - m[2],
                    crate::MaskConvention::UnprotectedF32V1 => m[2],
                })
                .collect();
            mesh.mask_convention = convention;
            mesh.materials = Some(materials.into_iter().map(|m| [m[0], m[1]]).collect());
        }
        if retain_surface {
            mesh.surface = Some(SurfaceTopology {
                faces,
                uv: bounds.has_uv.then_some(CornerUv {
                    coordinates,
                    face_indices: uv_faces,
                }),
            });
        }
        mesh.validate()?;
        Ok(MeshImport {
            report: MeshImportReport {
                format: MeshFileFormat::Obj,
                encoding: "utf8".into(),
                source_vertices: mesh.positions.len(),
                source_faces: bounds.input_faces,
                triangulated_faces: mesh.triangles.len(),
                welded_vertices: 0,
                imported_normals: false,
                imported_colors,
                skipped_properties: skipped,
                warnings,
            },
            mesh,
        })
    }
}

fn allocate<T>(count: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded OBJ channel")?;
    Ok(values)
}
fn failure(line: usize, message: &str) -> String {
    format!("OBJ line {}: {message}", line + 1)
}
fn numbers<'a, const MAX: usize>(
    words: impl Iterator<Item = &'a str>,
) -> Result<Vec<f32>, &'static str> {
    let mut values = Vec::with_capacity(MAX);
    for word in words {
        if values.len() >= MAX {
            return Err("too many numeric components");
        }
        let value = word.parse::<f32>().map_err(|_| "invalid number")?;
        if !value.is_finite() {
            return Err("non-finite number");
        }
        values.push(value);
    }
    Ok(values)
}
fn obj_index(word: &str, count: usize) -> Result<u32, &'static str> {
    let raw = word
        .parse::<i64>()
        .map_err(|_| "invalid or missing index")?;
    let index = if raw > 0 {
        raw - 1
    } else if raw < 0 {
        count as i64 + raw
    } else {
        -1
    };
    if index < 0 || index >= count as i64 {
        return Err("index refers to a missing declaration");
    }
    Ok(index as u32)
}
fn hex_count<'a>(
    words: impl Iterator<Item = &'a str>,
    stride: usize,
) -> Result<usize, &'static str> {
    let mut count = 0;
    for word in words {
        if word.len() % stride != 0 || !word.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("incomplete or invalid hexadecimal attribute record");
        }
        count += word.len() / stride;
    }
    if count == 0 {
        return Err("empty hexadecimal attribute stream");
    }
    Ok(count)
}
fn hex_records<'a>(
    words: impl Iterator<Item = &'a str>,
    stride: usize,
    mut accept: impl FnMut([u8; 4]),
) -> Result<(), &'static str> {
    for word in words {
        for record in word.as_bytes().chunks_exact(stride) {
            let mut bytes = [0_u8; 4];
            for (channel, pair) in record.chunks_exact(2).enumerate() {
                let digit = |byte: u8| {
                    (byte as char)
                        .to_digit(16)
                        .map(|v| v as u8)
                        .ok_or("invalid hexadecimal attribute")
                };
                bytes[channel] = digit(pair[0])? * 16 + digit(pair[1])?;
            }
            accept(bytes);
        }
    }
    Ok(())
}
