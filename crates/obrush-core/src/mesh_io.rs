//! Bounded, portable STL and PLY interchange. Geometry remains in mesh units.
//!
//! PLY preserves complete vertex normal and RGB sets. Integer RGB channels are
//! explicitly 0–255; floating RGB channels are 0–1. Unknown properties/elements
//! are consumed with bounded work and named in the import report. STL facet
//! normals are validated but replaced by geometric vertex normals. Colored STL
//! uses facet RGB: differing colors at coincident positions create vertex seams.
use crate::mesh::{canonical_quad_valid, triangulate_polygon, MAX_INPUT_BYTES};
use crate::{Mesh, SurfaceTopology, Vec3, MAX_TRIANGLES, MAX_VERTICES, TRIANGLE_CORNER};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

pub const MAX_PLY_HEADER_BYTES: usize = 64 * 1024;
pub const MAX_PLY_FACE_VERTICES: usize = 512;
pub const MAX_MESH_IO_SCALARS: usize = 32_000_000;
pub const MAX_PLY_TRIANGULATION_WORK: usize = 32_000_000;
const MAX_PLY_ELEMENTS: usize = 32;
const MAX_PLY_PROPERTIES: usize = 64;
const DEFAULT_COLOR: [f32; 3] = [0.65, 0.69, 0.74];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshFileFormat {
    Obj,
    Stl,
    Ply,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StlEncoding {
    Ascii,
    /// Little endian, VisCAM/SolidView 5-bit facet RGB averaged from vertices.
    Binary,
    /// Standard little-endian STL with zero attribute words and no RGB.
    BinaryUncolored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlyEncoding {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

#[derive(Clone, Debug, Serialize)]
pub struct MeshImportReport {
    pub format: MeshFileFormat,
    pub encoding: String,
    pub source_vertices: usize,
    pub source_faces: usize,
    pub triangulated_faces: usize,
    pub skipped_degenerate_faces: usize,
    pub welded_vertices: usize,
    pub imported_normals: bool,
    pub imported_colors: bool,
    pub skipped_properties: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct MeshImport {
    pub mesh: Mesh,
    pub report: MeshImportReport,
}

impl Mesh {
    /// SculptGL File > Save STL byte layout. This remains separate from the
    /// ordinary STL writer so the ordinary header and double-precision normal
    /// calculation do not silently change.
    pub fn to_stl_sculptgl(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        if self.has_attributes() {
            return Err("This exporter cannot preserve materials/original faces/UV metadata; export a version6 project or explicitly detach unsupported attributes".into());
        }
        let size = self
            .triangles
            .len()
            .checked_mul(50)
            .and_then(|n| n.checked_add(84))
            .ok_or("STL output length overflow")?;
        output_limit(size)?;
        let mut data = bounded_vec(size)?;
        data.resize(80, 0);
        data.extend_from_slice(&(self.triangles.len() as u32).to_le_bytes());
        for &triangle in &self.triangles {
            let p = triangle.map(|index| self.positions[index as usize]);
            // The pinned exporter uses float32 vec3 intermediates: first
            // (p1 - p0), then (p2 - that edge), then their cross product.
            let edge = p[1] - p[0];
            let other = p[2] - edge;
            let cross = edge.cross(other);
            let normal = cross.normalize_or_zero();
            push_vec3(&mut data, normal, false);
            for point in p {
                push_vec3(&mut data, point, false);
            }
            let mut rgb = [0_u16; 3];
            for (channel, value) in rgb.iter_mut().enumerate() {
                let sum: f64 = triangle
                    .iter()
                    .map(|&i| f64::from(self.colors[i as usize][channel]))
                    .sum();
                *value = (sum * (31.0 / 3.0)).round() as u16;
            }
            let attribute = 0x8000 | (rgb[0] << 10) | (rgb[1] << 5) | rgb[2];
            data.extend_from_slice(&attribute.to_le_bytes());
        }
        Ok(data)
    }

    /// Parse an STL entirely before returning a validated, independent mesh.
    pub fn import_stl(source: &[u8]) -> Result<MeshImport, String> {
        input_limit(source, "STL")?;
        if source.len() >= 84 {
            let count = u32::from_le_bytes(source[80..84].try_into().unwrap()) as usize;
            let expected = count.checked_mul(50).and_then(|n| n.checked_add(84));
            if expected == Some(source.len()) {
                return binary_stl(source, count);
            }
        }
        let prefix = source
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(source.len());
        if source[prefix..].starts_with(b"solid") {
            ascii_stl(source)
        } else {
            Err("STL binary length does not match its declared triangle count".into())
        }
    }

    /// Export positions and geometric facet normals. Masks/history are absent
    /// from STL. Binary RGB is an averaged 5-bit facet approximation; ASCII and
    /// BinaryUncolored omit RGB. No coordinate/unit conversion is performed.
    pub fn to_stl(&self, encoding: StlEncoding) -> Result<Vec<u8>, String> {
        self.validate()?;
        if self.has_attributes() {
            return Err("This exporter cannot preserve materials/original faces/UV metadata; export a version6 project or explicitly detach unsupported attributes".into());
        }
        let count = self.triangles.len();
        if encoding == StlEncoding::Ascii {
            let mut data = String::from("solid OBrush\n");
            for &triangle in &self.triangles {
                let normal = facet_normal(self, triangle);
                let mut row = String::new();
                writeln!(
                    row,
                    " facet normal {} {} {}\n  outer loop",
                    normal.x, normal.y, normal.z
                )
                .map_err(|e| e.to_string())?;
                for index in triangle {
                    let p = self.positions[index as usize];
                    writeln!(row, "   vertex {} {} {}", p.x, p.y, p.z)
                        .map_err(|e| e.to_string())?;
                }
                row.push_str("  endloop\n endfacet\n");
                append_text(&mut data, &row)?;
            }
            append_text(&mut data, "endsolid OBrush\n")?;
            return Ok(data.into_bytes());
        }
        let size = count
            .checked_mul(50)
            .and_then(|n| n.checked_add(84))
            .ok_or("STL output length overflow")?;
        output_limit(size)?;
        let mut data = bounded_vec(size)?;
        data.resize(80, 0);
        data[..6].copy_from_slice(b"OBrush");
        data.extend_from_slice(&(count as u32).to_le_bytes());
        for &triangle in &self.triangles {
            push_vec3(&mut data, facet_normal(self, triangle), false);
            for index in triangle {
                push_vec3(&mut data, self.positions[index as usize], false);
            }
            let attribute = if encoding == StlEncoding::Binary {
                let mut rgb = [0_u16; 3];
                for (channel, value) in rgb.iter_mut().enumerate() {
                    let sum: f64 = triangle
                        .iter()
                        .map(|&i| f64::from(self.colors[i as usize][channel]))
                        .sum();
                    *value = (sum * (31.0 / 3.0)).round() as u16;
                }
                0x8000 | (rgb[0] << 10) | (rgb[1] << 5) | rgb[2]
            } else {
                0
            };
            data.extend_from_slice(&attribute.to_le_bytes());
        }
        Ok(data)
    }

    /// Parse ASCII, little-endian or big-endian PLY 1.0. Vertex colors and
    /// normals require complete triples; unsupported fields are reported.
    pub fn import_ply(source: &[u8]) -> Result<MeshImport, String> {
        input_limit(source, "PLY")?;
        let header = PlyHeader::parse(source)?;
        let vertex = header
            .elements
            .iter()
            .find(|e| e.name == "vertex")
            .ok_or("PLY requires a vertex element")?;
        let face = header
            .elements
            .iter()
            .find(|e| e.name == "face")
            .ok_or("PLY requires a face element")?;
        if !(1..=MAX_VERTICES).contains(&vertex.count) || !(1..=MAX_TRIANGLES).contains(&face.count)
        {
            return Err("PLY vertex or face count is outside mesh limits".into());
        }
        require_scalars(vertex, &["x", "y", "z"])?;
        let has_normals = optional_triple(vertex, &["nx", "ny", "nz"])?;
        let has_colors = optional_triple(vertex, &["red", "green", "blue"])?;
        let indices = face
            .properties
            .iter()
            .filter(|p| matches!(p.name.as_str(), "vertex_indices" | "vertex_index"))
            .collect::<Vec<_>>();
        if indices.len() != 1 || !matches!(indices[0].kind, PropertyKind::List(_, _)) {
            return Err("PLY faces require exactly one vertex_indices or vertex_index list".into());
        }
        if let PropertyKind::List(_, kind) = indices[0].kind {
            if !kind.integer() {
                return Err("PLY face index lists require an integer item type".into());
            }
        }
        let min_bytes = header.elements.iter().try_fold(0_usize, |total, element| {
            let row = element
                .properties
                .iter()
                .try_fold(0_usize, |sum, property| {
                    sum.checked_add(match property.kind {
                        PropertyKind::Scalar(t) | PropertyKind::List(t, _) => {
                            if header.encoding == PlyEncoding::Ascii {
                                1
                            } else {
                                t.width()
                            }
                        }
                    })
                    .ok_or("PLY minimum row size overflow")
                })?;
            total
                .checked_add(
                    row.checked_mul(element.count)
                        .ok_or("PLY minimum element size overflow")?,
                )
                .ok_or("PLY minimum body size overflow")
        })?;
        if min_bytes > source.len() - header.body {
            return Err("PLY body is too short for the declared elements".into());
        }
        let mut positions = bounded_vec(vertex.count)?;
        let mut normals = if has_normals {
            bounded_vec(vertex.count)?
        } else {
            Vec::new()
        };
        let mut colors = if has_colors {
            bounded_vec(vertex.count)?
        } else {
            Vec::new()
        };
        // Faces may legally precede vertices. Store bounded polygons, then
        // validate and triangulate after all positions have been decoded.
        let mut polygons = bounded_vec(face.count)?;
        let mut total_indices = 0_usize;
        let mut face_payload_bytes = 0_usize;
        let mut source_quad_faces = 0_usize;
        let mut triangulation_work = 0_usize;
        let mut predicted_triangles = 0_usize;
        let mut reader = ValueReader::new(&source[header.body..], header.encoding)?;
        let mut skipped = Vec::new();
        for element in &header.elements {
            for property in &element.properties {
                if !supported_property(&element.name, &property.name) {
                    skipped.push(format!("{}.{}", element.name, property.name));
                }
            }
            if element.name != "vertex" && element.name != "face" && element.properties.is_empty() {
                skipped.push(format!("{} (empty element)", element.name));
            }
            for row in 0..element.count {
                reader.begin_row()?;
                let mut p = [0_f32; 3];
                let mut n = [0_f32; 3];
                let mut c = [0_f32; 3];
                let mut polygon = None;
                for property in &element.properties {
                    match property.kind {
                        PropertyKind::Scalar(kind) => {
                            let value = reader.scalar(kind)?;
                            if element.name == "vertex" {
                                if let Some(channel) = channel(&property.name, &["x", "y", "z"]) {
                                    p[channel] = finite_f32(value.as_f64(), "PLY coordinate")?;
                                    if p[channel].abs() > 1_000_000.0 {
                                        return Err(
                                            "PLY coordinate exceeds 1,000,000 mesh units".into()
                                        );
                                    }
                                } else if let Some(channel) =
                                    channel(&property.name, &["nx", "ny", "nz"])
                                {
                                    n[channel] = finite_f32(value.as_f64(), "PLY normal")?;
                                } else if let Some(channel) =
                                    channel(&property.name, &["red", "green", "blue"])
                                {
                                    let color =
                                        value.as_f64() / if kind.integer() { 255.0 } else { 1.0 };
                                    if !(0.0..=1.0).contains(&color) {
                                        return Err(
                                            "PLY RGB must be integer 0–255 or floating 0–1".into(),
                                        );
                                    }
                                    c[channel] = color as f32;
                                }
                            }
                        }
                        PropertyKind::List(count_kind, item_kind) => {
                            let count = reader.scalar(count_kind)?.as_count()?;
                            if count > MAX_PLY_FACE_VERTICES {
                                return Err("PLY list exceeds 512 entries".into());
                            }
                            let is_indices = element.name == "face"
                                && matches!(
                                    property.name.as_str(),
                                    "vertex_indices" | "vertex_index"
                                );
                            if is_indices {
                                source_quad_faces += usize::from(count == 4);
                                face_payload_bytes = face_payload_bytes
                                    .checked_add(count_kind.width() + count * item_kind.width())
                                    .ok_or("PLY face payload size overflow")?;
                                triangulation_work = triangulation_work
                                    .checked_add(if count > 3 { count * count * count } else { 0 })
                                    .ok_or("PLY triangulation work overflow")?;
                                if triangulation_work > MAX_PLY_TRIANGULATION_WORK {
                                    return Err(
                                        "PLY exceeds the 32,000,000 triangulation work limit"
                                            .into(),
                                    );
                                }
                                if count < 3 {
                                    return Err("PLY face needs at least three vertices".into());
                                }
                                total_indices = total_indices
                                    .checked_add(count)
                                    .ok_or("PLY index count overflow")?;
                                if total_indices > MAX_TRIANGLES * 3 {
                                    return Err(
                                        "PLY polygon index allocation limit exceeded".into()
                                    );
                                }
                                predicted_triangles = predicted_triangles
                                    .checked_add(count - 2)
                                    .ok_or("PLY triangle count overflow")?;
                                if predicted_triangles > MAX_TRIANGLES {
                                    return Err(
                                        "PLY triangulated face count exceeds mesh limit".into()
                                    );
                                }
                                let mut values = bounded_vec(count)?;
                                for _ in 0..count {
                                    let index = reader.scalar(item_kind)?.as_count()?;
                                    if index >= vertex.count {
                                        return Err(format!(
                                            "PLY face {row} refers to a missing vertex"
                                        ));
                                    }
                                    values.push(index as u32);
                                }
                                if values.iter().copied().collect::<HashSet<_>>().len()
                                    != values.len()
                                {
                                    return Err(format!("PLY face {row} repeats a vertex"));
                                }
                                polygon = Some(values);
                            } else {
                                for _ in 0..count {
                                    reader.scalar(item_kind)?;
                                }
                            }
                        }
                    }
                }
                reader.end_row()?;
                if element.name == "vertex" {
                    positions.push(Vec3::from_array(p));
                    if has_normals {
                        normals.push(Vec3::from_array(n));
                    }
                    if has_colors {
                        colors.push(c);
                    }
                } else if element.name == "face" {
                    polygons.push(polygon.ok_or("PLY face did not contain vertex indices")?);
                }
            }
        }
        let trailing_padding = reader.finish(if header.sculptgl_padding {
            Some(
                face_payload_bytes
                    .checked_add(
                        source_quad_faces
                            .checked_mul(48)
                            .ok_or("PLY SculptGL quad padding overflow")?,
                    )
                    .ok_or("PLY SculptGL padding overflow")?,
            )
        } else {
            None
        })?;
        let mut triangles = Vec::new();
        let retain_surface = source_quad_faces > 0;
        let mut original_faces = Vec::new();
        if retain_surface {
            original_faces
                .try_reserve_exact(predicted_triangles)
                .map_err(|_| "Cannot allocate bounded PLY original faces")?;
        }
        let mut triangulated_faces = 0;
        let mut converted_quads = 0;
        for (i, polygon) in polygons.iter().enumerate() {
            // SculptGL writes the original quad corners, including quads on
            // curved surfaces. Its own importer accepts their nonplanarity.
            // Keep strict planarity for other PLY files and larger polygons.
            if !(header.sculptgl_padding && polygon.len() == 4) {
                validate_polygon_plane(polygon, &positions)
                    .map_err(|e| format!("PLY face {i}: {e}"))?;
            }
            let count = polygon.len() - 2;
            if triangles
                .len()
                .checked_add(count)
                .is_none_or(|n| n > MAX_TRIANGLES)
            {
                return Err("PLY triangulated face count exceeds mesh limit".into());
            }
            triangles
                .try_reserve(count)
                .map_err(|_| "Cannot allocate bounded PLY triangles")?;
            if polygon.len() > 3 {
                triangulated_faces += 1;
            }
            let generated = triangulate_polygon(polygon, &positions)
                .map_err(|e| format!("PLY face {i}: {e}"))?;
            if polygon.len() == 4 && canonical_quad_valid(polygon, &positions) {
                triangles.extend([
                    [polygon[0], polygon[1], polygon[2]],
                    [polygon[0], polygon[2], polygon[3]],
                ]);
                original_faces.push([polygon[0], polygon[1], polygon[2], polygon[3]]);
            } else {
                converted_quads += usize::from(polygon.len() == 4);
                for triangle in generated {
                    triangles.push(triangle);
                    if retain_surface {
                        original_faces.push([
                            triangle[0],
                            triangle[1],
                            triangle[2],
                            TRIANGLE_CORNER,
                        ]);
                    }
                }
            }
        }
        let mut mesh = Mesh::new(positions, triangles);
        if has_normals {
            mesh.normals = normals;
        }
        if has_colors {
            mesh.colors = colors;
        }
        if retain_surface {
            mesh.surface = Some(SurfaceTopology {
                faces: original_faces,
                uv: None,
            });
        }
        mesh.validate()?;
        let mut warnings = vec!["PLY units are preserved without conversion; masks, layers and scene metadata are not imported".into()];
        if trailing_padding != 0 {
            warnings.push(format!("Accepted {trailing_padding} zero bytes matching legacy SculptGL binary PLY exporter padding"));
        }
        if converted_quads != 0 {
            warnings.push(format!(
                "Triangulated {converted_quads} quads whose 0–2 diagonal is unsuitable."
            ));
        }
        warnings.push("PLY float64 fields are converted to float32; integer coordinates retain their numeric mesh-unit values".into());
        if !skipped.is_empty() {
            warnings.push(
                "Unsupported PLY fields were consumed and are listed in skipped_properties".into(),
            );
        }
        Ok(MeshImport {
            mesh,
            report: MeshImportReport {
                format: MeshFileFormat::Ply,
                encoding: header.encoding.name().into(),
                source_vertices: vertex.count,
                source_faces: face.count,
                triangulated_faces,
                skipped_degenerate_faces: 0,
                welded_vertices: 0,
                imported_normals: has_normals,
                imported_colors: has_colors,
                skipped_properties: skipped,
                warnings,
            },
        })
    }

    /// PLY output uses float32 position/normal/RGB channels and uint32 triangle
    /// indices, retaining their f32 values exactly across an OBrush round trip.
    /// It omits masks, layers, units, UVs, materials and scene structure.
    pub fn to_ply(&self, encoding: PlyEncoding) -> Result<Vec<u8>, String> {
        self.validate()?;
        if self.has_attributes() {
            return Err("This exporter cannot preserve materials/original faces/UV metadata; export a version6 project or explicitly detach unsupported attributes".into());
        }
        let header = format!(
            "ply\nformat {} 1.0\ncomment independently written OBrush; mesh units preserved\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nproperty float nx\nproperty float ny\nproperty float nz\nproperty float red\nproperty float green\nproperty float blue\nelement face {}\nproperty list uchar uint vertex_indices\nend_header\n",
            encoding.name(),
            self.positions.len(),
            self.triangles.len()
        );
        if encoding == PlyEncoding::Ascii {
            let mut data = header;
            for ((p, n), c) in self.positions.iter().zip(&self.normals).zip(&self.colors) {
                let row = format!(
                    "{} {} {} {} {} {} {} {} {}\n",
                    p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2]
                );
                append_text(&mut data, &row)?;
            }
            for t in &self.triangles {
                append_text(&mut data, &format!("3 {} {} {}\n", t[0], t[1], t[2]))?;
            }
            return Ok(data.into_bytes());
        }
        let size = self
            .positions
            .len()
            .checked_mul(36)
            .and_then(|n| {
                self.triangles
                    .len()
                    .checked_mul(13)
                    .and_then(|f| n.checked_add(f))
            })
            .and_then(|n| n.checked_add(header.len()))
            .ok_or("PLY output size overflow")?;
        output_limit(size)?;
        let mut data = bounded_vec(size)?;
        data.extend_from_slice(header.as_bytes());
        let big = encoding == PlyEncoding::BinaryBigEndian;
        for ((p, n), c) in self.positions.iter().zip(&self.normals).zip(&self.colors) {
            push_vec3(&mut data, *p, big);
            push_vec3(&mut data, *n, big);
            for &value in c {
                data.extend_from_slice(&if big {
                    value.to_be_bytes()
                } else {
                    value.to_le_bytes()
                });
            }
        }
        for triangle in &self.triangles {
            data.push(3);
            for &index in triangle {
                data.extend_from_slice(&if big {
                    index.to_be_bytes()
                } else {
                    index.to_le_bytes()
                });
            }
        }
        Ok(data)
    }
}

impl PlyEncoding {
    fn name(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::BinaryLittleEndian => "binary_little_endian",
            Self::BinaryBigEndian => "binary_big_endian",
        }
    }
}

fn input_limit(source: &[u8], format: &str) -> Result<(), String> {
    if source.is_empty() || source.len() > MAX_INPUT_BYTES {
        Err(format!("{format} must contain 1–128 MiB of input"))
    } else {
        Ok(())
    }
}
fn output_limit(size: usize) -> Result<(), String> {
    if size > MAX_INPUT_BYTES {
        Err("Mesh output exceeds the 128 MiB limit".into())
    } else {
        Ok(())
    }
}
fn bounded_vec<T>(capacity: usize) -> Result<Vec<T>, String> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| "Cannot allocate bounded mesh interchange field")?;
    Ok(result)
}
fn append_text(data: &mut String, row: &str) -> Result<(), String> {
    output_limit(
        data.len()
            .checked_add(row.len())
            .ok_or("Mesh output size overflow")?,
    )?;
    data.try_reserve(row.len())
        .map_err(|_| "Cannot allocate bounded mesh output")?;
    data.push_str(row);
    Ok(())
}
fn finite_f32(value: f64, label: &str) -> Result<f32, String> {
    let value32 = value as f32;
    if !value.is_finite() || !value32.is_finite() {
        Err(format!(
            "{label} must be finite and representable as float32"
        ))
    } else {
        Ok(value32)
    }
}
fn push_vec3(data: &mut Vec<u8>, p: Vec3, big: bool) {
    for value in p.to_array() {
        data.extend_from_slice(&if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        });
    }
}
fn facet_normal(mesh: &Mesh, triangle: [u32; 3]) -> Vec3 {
    let a = mesh.positions[triangle[0] as usize].as_dvec3();
    let b = mesh.positions[triangle[1] as usize].as_dvec3();
    let c = mesh.positions[triangle[2] as usize].as_dvec3();
    (b - a).cross(c - a).normalize_or_zero().as_vec3()
}

struct StlBuilder {
    positions: Vec<Vec3>,
    colors: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    vertices: HashMap<[u32; 6], u32>,
    source_facets: usize,
    skipped_degenerate_facets: usize,
}
impl StlBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            triangles: Vec::new(),
            vertices: HashMap::new(),
            source_facets: 0,
            skipped_degenerate_facets: 0,
        }
    }
    fn facet(&mut self, points: [Vec3; 3], color: [f32; 3]) -> Result<(), String> {
        if self.source_facets >= MAX_TRIANGLES {
            return Err("STL triangle limit exceeded".into());
        }
        // Validate every corner before considering a skip: a degenerate facet
        // must not hide a hostile third coordinate.
        for point in points {
            if !point.is_finite() || point.abs().max_element() > 1_000_000.0 {
                return Err("STL vertex is non-finite or exceeds coordinate limits".into());
            }
        }
        self.source_facets += 1;
        // The weld key canonicalizes signed zero. Equal points prove zero area
        // without an epsilon or a potentially underflowing cross product.
        let same_position = |a: Vec3, b: Vec3| {
            a.to_array()
                .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
                == b.to_array()
                    .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
        };
        if same_position(points[0], points[1])
            || same_position(points[1], points[2])
            || same_position(points[2], points[0])
        {
            self.skipped_degenerate_facets += 1;
            return Ok(());
        }
        let mut triangle = [0_u32; 3];
        for (slot, point) in triangle.iter_mut().zip(points) {
            let mut key = [0_u32; 6];
            for (target, value) in key[..3].iter_mut().zip(point.to_array()) {
                *target = if value == 0.0 { 0 } else { value.to_bits() };
            }
            for (target, value) in key[3..].iter_mut().zip(color) {
                *target = value.to_bits();
            }
            if let Some(&index) = self.vertices.get(&key) {
                *slot = index;
            } else {
                if self.positions.len() >= MAX_VERTICES {
                    return Err("STL welded vertex limit exceeded".into());
                }
                self.positions
                    .try_reserve(1)
                    .map_err(|_| "Cannot allocate STL vertices")?;
                self.colors
                    .try_reserve(1)
                    .map_err(|_| "Cannot allocate STL colors")?;
                self.vertices
                    .try_reserve(1)
                    .map_err(|_| "Cannot allocate STL weld table")?;
                *slot = self.positions.len() as u32;
                self.positions.push(point);
                self.colors.push(color);
                self.vertices.insert(key, *slot);
            }
        }
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err("STL facet repeats a vertex after exact welding".into());
        }
        self.triangles
            .try_reserve(1)
            .map_err(|_| "Cannot allocate STL triangles")?;
        self.triangles.push(triangle);
        Ok(())
    }
    fn finish(
        self,
        encoding: &str,
        colored: bool,
        mut warnings: Vec<String>,
    ) -> Result<MeshImport, String> {
        if self.triangles.is_empty() {
            return Err("STL contains no non-degenerate facets".into());
        }
        let source_faces = self.source_facets;
        let retained_faces = self.triangles.len();
        let welded_vertices = retained_faces * 3 - self.positions.len();
        let mut mesh = Mesh::new(self.positions, self.triangles);
        mesh.colors = self.colors;
        mesh.validate()?;
        warnings.push(
            "STL facet normals are validated but recomputed as geometric vertex normals".into(),
        );
        warnings.push(
            "STL does not retain masks, layers, vertex normals, units or scene structure".into(),
        );
        if self.skipped_degenerate_facets != 0 {
            warnings.push(format!(
                "Skipped {} zero-area STL facets with exact repeated vertices; no tolerance-based welding or topology repair was applied",
                self.skipped_degenerate_facets
            ));
        }
        Ok(MeshImport {
            mesh,
            report: MeshImportReport {
                format: MeshFileFormat::Stl,
                encoding: encoding.into(),
                source_vertices: source_faces * 3,
                source_faces,
                triangulated_faces: retained_faces,
                skipped_degenerate_faces: self.skipped_degenerate_facets,
                welded_vertices,
                imported_normals: false,
                imported_colors: colored,
                skipped_properties: Vec::new(),
                warnings,
            },
        })
    }
}
fn binary_stl(source: &[u8], count: usize) -> Result<MeshImport, String> {
    if count == 0 || count > MAX_TRIANGLES {
        return Err("STL declared triangle count exceeds mesh limits".into());
    }
    let header = &source[..80];
    let magic = header.windows(6).position(|w| w == b"COLOR=");
    let default = if let Some(offset) = magic {
        let bytes = header
            .get(offset + 6..offset + 10)
            .ok_or("STL COLOR= requires four header color bytes")?;
        [
            f32::from(bytes[0]) / 255.0,
            f32::from(bytes[1]) / 255.0,
            f32::from(bytes[2]) / 255.0,
        ]
    } else {
        DEFAULT_COLOR
    };
    let mut builder = StlBuilder::new();
    let mut colored = magic.is_some();
    let mut unknown_attributes = 0;
    for facet in source[84..].chunks_exact(50) {
        let scalar = |at: usize| f32::from_le_bytes(facet[at..at + 4].try_into().unwrap());
        let normal = Vec3::new(scalar(0), scalar(4), scalar(8));
        if !normal.is_finite() || !normal.length_squared().is_finite() {
            return Err("STL facet normal is not finite".into());
        }
        let points = std::array::from_fn(|i| {
            Vec3::new(
                scalar(12 + i * 12),
                scalar(16 + i * 12),
                scalar(20 + i * 12),
            )
        });
        let attribute = u16::from_le_bytes(facet[48..50].try_into().unwrap());
        let color = if magic.is_some() && attribute & 0x8000 == 0 {
            colored = true;
            [
                f32::from(attribute & 31) / 31.0,
                f32::from((attribute >> 5) & 31) / 31.0,
                f32::from((attribute >> 10) & 31) / 31.0,
            ]
        } else if magic.is_none() && attribute & 0x8000 != 0 {
            colored = true;
            [
                f32::from((attribute >> 10) & 31) / 31.0,
                f32::from((attribute >> 5) & 31) / 31.0,
                f32::from(attribute & 31) / 31.0,
            ]
        } else {
            if magic.is_none() && attribute != 0 {
                unknown_attributes += 1;
            }
            default
        };
        builder.facet(points, color)?;
    }
    let mut warnings = Vec::new();
    if colored {
        warnings.push("Binary STL facet colors use 5-bit RGB; coincident positions with differing RGB remain separate vertices".into());
    }
    if magic.is_some() {
        warnings.push("Materialise COLOR= header alpha is not retained".into());
    }
    if unknown_attributes != 0 {
        warnings.push(format!(
            "{unknown_attributes} non-color STL attribute words are unsupported and were discarded"
        ));
    }
    builder.finish(
        if magic.is_some() {
            "binary_little_endian_magics"
        } else if colored {
            "binary_little_endian_viscam"
        } else {
            "binary_little_endian"
        },
        colored,
        warnings,
    )
}
fn ascii_stl(source: &[u8]) -> Result<MeshImport, String> {
    let text = std::str::from_utf8(source).map_err(|_| "ASCII STL must be UTF-8")?;
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let header = lines.next().ok_or("ASCII STL has no solid declaration")?;
    let solid_name = header
        .strip_prefix("solid")
        .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        .ok_or("ASCII STL requires a solid declaration")?
        .trim();
    let mut builder = StlBuilder::new();
    let mut ended = false;
    while let Some(line) = lines.next() {
        if let Some(name) = line
            .strip_prefix("endsolid")
            .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            if name.trim() != solid_name {
                return Err("ASCII STL solid and endsolid names differ".into());
            }
            if lines.next().is_some() {
                return Err("ASCII STL has trailing content or multiple solids".into());
            }
            ended = true;
            break;
        }
        parse_ascii_vec3(line, "facet normal")?;
        if lines.next() != Some("outer loop") {
            return Err("ASCII STL facet requires outer loop".into());
        }
        let mut points = [Vec3::ZERO; 3];
        for point in &mut points {
            *point = parse_ascii_vec3(
                lines.next().ok_or("ASCII STL facet is truncated")?,
                "vertex",
            )?;
        }
        if lines.next() != Some("endloop") || lines.next() != Some("endfacet") {
            return Err("ASCII STL facet requires endloop and endfacet".into());
        }
        builder.facet(points, DEFAULT_COLOR)?;
    }
    if !ended {
        return Err("ASCII STL is missing endsolid".into());
    }
    builder.finish("ascii", false, Vec::new())
}
fn parse_ascii_vec3(line: &str, prefix: &str) -> Result<Vec3, String> {
    let mut tokens = line.split_whitespace();
    for expected in prefix.split_whitespace() {
        if tokens.next() != Some(expected) {
            return Err(format!("ASCII STL expected {prefix}"));
        }
    }
    let mut values = [0_f32; 3];
    for value in &mut values {
        *value = tokens
            .next()
            .ok_or("ASCII STL vector is incomplete")?
            .parse()
            .map_err(|_| "ASCII STL vector number is invalid")?;
    }
    let point = Vec3::from_array(values);
    if tokens.next().is_some() || !point.is_finite() || !point.length_squared().is_finite() {
        return Err("ASCII STL vector must have exactly three finite values".into());
    }
    Ok(point)
}

#[derive(Clone, Copy, Debug)]
enum ScalarType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}
impl ScalarType {
    fn parse(token: &str) -> Result<Self, String> {
        match token {
            "char" | "int8" => Ok(Self::I8),
            "uchar" | "uint8" => Ok(Self::U8),
            "short" | "int16" => Ok(Self::I16),
            "ushort" | "uint16" => Ok(Self::U16),
            "int" | "int32" => Ok(Self::I32),
            "uint" | "uint32" => Ok(Self::U32),
            "float" | "float32" => Ok(Self::F32),
            "double" | "float64" => Ok(Self::F64),
            _ => Err(format!("Unsupported PLY scalar type {token}")),
        }
    }
    fn integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }
    fn width(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum PropertyKind {
    Scalar(ScalarType),
    List(ScalarType, ScalarType),
}
struct Property {
    name: String,
    kind: PropertyKind,
}
struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}
struct PlyHeader {
    encoding: PlyEncoding,
    elements: Vec<Element>,
    body: usize,
    sculptgl_padding: bool,
}
impl PlyHeader {
    fn parse(source: &[u8]) -> Result<Self, String> {
        let mut offset = 0;
        let mut encoding = None;
        let mut elements: Vec<Element> = Vec::new();
        let mut rows = 0_usize;
        let mut sculptgl_padding = false;
        loop {
            let end = source[offset..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|n| offset + n)
                .ok_or("PLY header is missing a newline or end_header")?;
            if end + 1 > MAX_PLY_HEADER_BYTES {
                return Err("PLY header exceeds 64 KiB".into());
            }
            let line = std::str::from_utf8(&source[offset..end])
                .map_err(|_| "PLY header must be UTF-8")?
                .trim_end_matches('\r');
            if offset == 0 {
                if line != "ply" {
                    return Err("PLY requires the ply signature on its first line".into());
                }
                offset = end + 1;
                continue;
            }
            offset = end + 1;
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                return Err("PLY header contains an empty directive".into());
            }
            match parts[0] {
                "comment" => {
                    if line.trim() == "comment created by SculptGL" {
                        sculptgl_padding = true;
                    }
                }
                "obj_info" => {}
                "format" if parts.len() == 3 && parts[2] == "1.0" => {
                    if encoding.is_some() || !elements.is_empty() {
                        return Err("PLY format must appear once before elements".into());
                    }
                    encoding = Some(match parts[1] {
                        "ascii" => PlyEncoding::Ascii,
                        "binary_little_endian" => PlyEncoding::BinaryLittleEndian,
                        "binary_big_endian" => PlyEncoding::BinaryBigEndian,
                        _ => return Err("Unsupported PLY encoding".into()),
                    });
                }
                "element" if parts.len() == 3 => {
                    if encoding.is_none() || elements.len() >= MAX_PLY_ELEMENTS {
                        return Err(
                            "PLY element declaration exceeds limits or precedes format".into()
                        );
                    }
                    if parts[1].len() > 128 || elements.iter().any(|e| e.name == parts[1]) {
                        return Err("PLY element names must be unique and at most 128 bytes".into());
                    }
                    let count: usize = parts[2]
                        .parse()
                        .map_err(|_| "PLY element count is invalid")?;
                    let limit = if parts[1] == "vertex" {
                        MAX_VERTICES
                    } else {
                        MAX_TRIANGLES
                    };
                    if count > limit {
                        return Err("PLY declared element count exceeds limits".into());
                    }
                    rows = rows
                        .checked_add(count)
                        .ok_or("PLY element count overflow")?;
                    if rows > MAX_VERTICES + MAX_TRIANGLES {
                        return Err("PLY total declared element count exceeds limits".into());
                    }
                    elements.push(Element {
                        name: parts[1].into(),
                        count,
                        properties: Vec::new(),
                    });
                }
                "property" => {
                    let element = elements
                        .last_mut()
                        .ok_or("PLY property precedes an element")?;
                    if element.properties.len() >= MAX_PLY_PROPERTIES {
                        return Err("PLY element exceeds 64 properties".into());
                    }
                    let (name, kind) = match parts.as_slice() {
                        [_, kind, name] => (*name, PropertyKind::Scalar(ScalarType::parse(kind)?)),
                        [_, "list", count, item, name] => {
                            let count = ScalarType::parse(count)?;
                            if !count.integer() {
                                return Err("PLY list counts require an integer type".into());
                            }
                            (*name, PropertyKind::List(count, ScalarType::parse(item)?))
                        }
                        _ => return Err("PLY property declaration is malformed".into()),
                    };
                    if name.len() > 128 || element.properties.iter().any(|p| p.name == name) {
                        return Err(
                            "PLY property names must be unique and at most 128 bytes".into()
                        );
                    }
                    element.properties.push(Property {
                        name: name.into(),
                        kind,
                    });
                }
                "end_header" if parts.len() == 1 => {
                    return Ok(Self {
                        encoding: encoding.ok_or("PLY has no format declaration")?,
                        elements,
                        body: offset,
                        sculptgl_padding,
                    });
                }
                _ => {
                    return Err(format!(
                        "Unknown or malformed PLY header directive {}",
                        parts[0]
                    ));
                }
            }
        }
    }
}
fn require_scalars(element: &Element, names: &[&str]) -> Result<(), String> {
    for name in names {
        if !element
            .properties
            .iter()
            .any(|p| p.name == *name && matches!(p.kind, PropertyKind::Scalar(_)))
        {
            return Err(format!("PLY vertex requires scalar property {name}"));
        }
    }
    Ok(())
}
fn optional_triple(element: &Element, names: &[&str]) -> Result<bool, String> {
    let count = names
        .iter()
        .filter(|name| element.properties.iter().any(|p| p.name == **name))
        .count();
    if count == 0 {
        return Ok(false);
    }
    if count != 3 {
        return Err(format!(
            "PLY requires a complete {} triple",
            names.join("/")
        ));
    }
    require_scalars(element, names)?;
    Ok(true)
}
fn supported_property(element: &str, property: &str) -> bool {
    (element == "vertex"
        && matches!(
            property,
            "x" | "y" | "z" | "nx" | "ny" | "nz" | "red" | "green" | "blue"
        ))
        || (element == "face" && matches!(property, "vertex_indices" | "vertex_index"))
}
fn channel(name: &str, names: &[&str; 3]) -> Option<usize> {
    names.iter().position(|&entry| entry == name)
}

#[derive(Clone, Copy)]
enum Number {
    Integer(i64),
    Float(f64),
}
impl Number {
    fn as_f64(self) -> f64 {
        match self {
            Self::Integer(n) => n as f64,
            Self::Float(n) => n,
        }
    }
    fn as_count(self) -> Result<usize, String> {
        match self {
            Self::Integer(n) if n >= 0 => {
                usize::try_from(n).map_err(|_| "PLY list count/index is too large".into())
            }
            _ => Err("PLY list count/index must be a nonnegative integer".into()),
        }
    }
}
enum ValueReader<'a> {
    Ascii {
        lines: std::str::Lines<'a>,
        tokens: std::str::SplitWhitespace<'a>,
        read: usize,
    },
    Binary {
        source: &'a [u8],
        offset: usize,
        big: bool,
        read: usize,
    },
}
impl<'a> ValueReader<'a> {
    fn new(source: &'a [u8], encoding: PlyEncoding) -> Result<Self, String> {
        Ok(if encoding == PlyEncoding::Ascii {
            Self::Ascii {
                lines: std::str::from_utf8(source)
                    .map_err(|_| "ASCII PLY body must be UTF-8")?
                    .lines(),
                tokens: "".split_whitespace(),
                read: 0,
            }
        } else {
            Self::Binary {
                source,
                offset: 0,
                big: encoding == PlyEncoding::BinaryBigEndian,
                read: 0,
            }
        })
    }
    fn scalar(&mut self, kind: ScalarType) -> Result<Number, String> {
        let number = match self {
            Self::Ascii { tokens, read, .. } => {
                *read += 1;
                if *read > MAX_MESH_IO_SCALARS {
                    return Err("PLY exceeds the 32,000,000 scalar work limit".into());
                }
                let token = tokens.next().ok_or("ASCII PLY body is truncated")?;
                let error = || format!("Invalid ASCII PLY {:?} scalar", kind);
                match kind {
                    ScalarType::I8 => {
                        Number::Integer(token.parse::<i8>().map_err(|_| error())?.into())
                    }
                    ScalarType::U8 => {
                        Number::Integer(token.parse::<u8>().map_err(|_| error())?.into())
                    }
                    ScalarType::I16 => {
                        Number::Integer(token.parse::<i16>().map_err(|_| error())?.into())
                    }
                    ScalarType::U16 => {
                        Number::Integer(token.parse::<u16>().map_err(|_| error())?.into())
                    }
                    ScalarType::I32 => {
                        Number::Integer(token.parse::<i32>().map_err(|_| error())?.into())
                    }
                    ScalarType::U32 => {
                        Number::Integer(token.parse::<u32>().map_err(|_| error())?.into())
                    }
                    ScalarType::F32 => {
                        Number::Float(f64::from(token.parse::<f32>().map_err(|_| error())?))
                    }
                    ScalarType::F64 => Number::Float(token.parse::<f64>().map_err(|_| error())?),
                }
            }
            Self::Binary {
                source,
                offset,
                big,
                read,
            } => {
                *read += 1;
                if *read > MAX_MESH_IO_SCALARS {
                    return Err("PLY exceeds the 32,000,000 scalar work limit".into());
                }
                let end = offset
                    .checked_add(kind.width())
                    .ok_or("PLY body offset overflow")?;
                let bytes = source
                    .get(*offset..end)
                    .ok_or("Binary PLY body is truncated")?;
                *offset = end;
                macro_rules! decode {
                    ($type:ty) => {
                        if *big {
                            <$type>::from_be_bytes(bytes.try_into().unwrap())
                        } else {
                            <$type>::from_le_bytes(bytes.try_into().unwrap())
                        }
                    };
                }
                match kind {
                    ScalarType::I8 => Number::Integer(i8::from_ne_bytes([bytes[0]]).into()),
                    ScalarType::U8 => Number::Integer(bytes[0].into()),
                    ScalarType::I16 => Number::Integer(decode!(i16).into()),
                    ScalarType::U16 => Number::Integer(decode!(u16).into()),
                    ScalarType::I32 => Number::Integer(decode!(i32).into()),
                    ScalarType::U32 => Number::Integer(decode!(u32).into()),
                    ScalarType::F32 => Number::Float(f64::from(decode!(f32))),
                    ScalarType::F64 => Number::Float(decode!(f64)),
                }
            }
        };
        if !number.as_f64().is_finite() {
            return Err("PLY scalar is non-finite, including unsupported fields".into());
        }
        Ok(number)
    }
    fn begin_row(&mut self) -> Result<(), String> {
        if let Self::Ascii { lines, tokens, .. } = self {
            *tokens = lines
                .next()
                .ok_or("ASCII PLY declared row is missing")?
                .split_whitespace();
        }
        Ok(())
    }
    fn end_row(&mut self) -> Result<(), String> {
        if let Self::Ascii { tokens, .. } = self {
            if tokens.next().is_some() {
                return Err("ASCII PLY row contains undeclared values".into());
            }
        }
        Ok(())
    }
    fn finish(&mut self, compatibility_padding: Option<usize>) -> Result<usize, String> {
        match self {
            Self::Ascii { lines, .. } => {
                if lines.any(|line| !line.trim().is_empty()) {
                    Err("ASCII PLY contains undeclared trailing rows".into())
                } else {
                    Ok(0)
                }
            }
            Self::Binary { source, offset, .. } => {
                let tail = &source[*offset..];
                if tail.is_empty() {
                    Ok(0)
                } else if compatibility_padding == Some(tail.len())
                    && tail.iter().all(|&byte| byte == 0)
                {
                    Ok(tail.len())
                } else {
                    Err("Binary PLY contains undeclared trailing bytes".into())
                }
            }
        }
    }
}

fn validate_polygon_plane(polygon: &[u32], positions: &[Vec3]) -> Result<(), String> {
    if polygon.len() <= 3 {
        return Ok(());
    }
    let anchor = positions[polygon[0] as usize].as_dvec3();
    let mut normal = glam::DVec3::ZERO;
    let mut scale = 0.0_f64;
    for index in 0..polygon.len() {
        let point = positions[polygon[index] as usize].as_dvec3() - anchor;
        let next = positions[polygon[(index + 1) % polygon.len()] as usize].as_dvec3() - anchor;
        normal += point.cross(next);
        scale = scale.max(point.abs().max_element());
    }
    let unit = normal.normalize_or_zero();
    if unit == glam::DVec3::ZERO {
        return Err("polygon is degenerate or self-intersecting".into());
    }
    if polygon.iter().any(|&index| {
        (positions[index as usize].as_dvec3() - anchor)
            .dot(unit)
            .abs()
            > scale * 1e-5
    }) {
        return Err("nonplanar polygon must be triangulated explicitly before import".into());
    }
    Ok(())
}
