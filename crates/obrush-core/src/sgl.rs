//! Bounded independent SculptGL SGL binary scene records (legacy0/1 and modern2/3).
//!
//! These records preserve the format's complete encoded fields, including
//! original quads and UV corner indices. They deliberately do not convert to
//! `Mesh`, bake transforms, guess external environment assets or invert masks.
use crate::mesh::MAX_INPUT_BYTES;
use crate::{MAX_SUBTOOLS, MAX_TRIANGLES, MAX_VERTICES};
use serde::{Deserialize, Serialize};

pub const MAX_SGL_UV_COORDS: usize = MAX_TRIANGLES * 3;
const MAX_COORDINATE: f32 = 1_000_000.0;
const TRIANGLE_MARKER: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SglByteOrder {
    LittleEndian,
    BigEndian,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SglDisplaySettings {
    pub show_grid: bool,
    pub show_symmetry_line: bool,
    pub show_contour: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SglCameraSettings {
    /// Pinned SGL enum: 0 perspective, 1 orthographic.
    pub projection: u32,
    /// Pinned SGL enum: 0 orbit, 1 spherical, 2 plane.
    pub mode: u32,
    pub fov_degrees: f32,
    pub use_pivot: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SglRenderSettings {
    /// Raw pinned shader enum (0–12); external shader assets are not embedded.
    pub shader: u32,
    /// Raw external MatCap index, not an embedded image or verified asset.
    pub matcap: u32,
    pub show_wireframe: bool,
    pub flat_shading: bool,
    pub opacity: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SglUvData {
    pub coordinates: Vec<[f32; 2]>,
    /// Original four index words, with triangle marker semantics of scene.version.
    pub face_indices: Vec<[u32; 4]>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SglMesh {
    pub render: Option<SglRenderSettings>,
    pub center: [f32; 3],
    /// Column-major affine matrix. Reflection/shear/nonuniform scales are retained.
    pub matrix: [f32; 16],
    /// Serialized redundant scale, which pinned ImportSGL skips. Retained exactly.
    pub scale: f32,
    pub positions: Vec<[f32; 3]>,
    pub colors: Option<Vec<[f32; 3]>>,
    /// Exact roughness/metallic/SculptGL-masking triples. No mask convention conversion.
    pub materials: Option<Vec<[f32; 3]>>,
    /// Original triangle/quad records; no polygon triangulation is performed.
    pub faces: Vec<[u32; 4]>,
    pub uv: Option<SglUvData>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SglScene {
    pub version: u32,
    pub byte_order: SglByteOrder,
    pub display: Option<SglDisplaySettings>,
    pub camera: Option<SglCameraSettings>,
    pub meshes: Vec<SglMesh>,
}

impl SglScene {
    /// Decode the complete supported version and original byte order. Count,
    /// remaining-byte and aggregate allocation checks precede each reservation.
    pub fn from_bytes(source: &[u8]) -> Result<Self, String> {
        input_size(source)?;
        let first: [u8; 4] = source[..4].try_into().unwrap();
        let little = u32::from_le_bytes(first);
        let big = u32::from_be_bytes(first);
        if little == 0 {
            return Err("SGL version0 is endian-ambiguous; use from_bytes_with_order with an explicit byte order".into());
        }
        let order = if (1..=3).contains(&little) {
            SglByteOrder::LittleEndian
        } else if (1..=3).contains(&big) {
            SglByteOrder::BigEndian
        } else {
            return Err("Unsupported SGL version".into());
        };
        Self::from_bytes_with_order(source, order)
    }
    /// Decode with caller-selected byte order. Version0 has only source-derived
    /// legacy layout evidence and never receives a guessed byte order.
    pub fn from_bytes_with_order(source: &[u8], byte_order: SglByteOrder) -> Result<Self, String> {
        input_size(source)?;
        let mut reader = Reader {
            source,
            offset: 0,
            order: byte_order,
            allocated: 0,
        };
        let version = reader.word()?;
        if !supported_version(version) {
            return Err("Unsupported SGL version for the explicit byte order".into());
        }
        let (display, camera) = if version >= 2 {
            let display = SglDisplaySettings {
                show_grid: reader.flag()?,
                show_symmetry_line: reader.flag()?,
                show_contour: reader.flag()?,
            };
            let camera = SglCameraSettings {
                projection: reader.word()?,
                mode: reader.word()?,
                fov_degrees: reader.float()?,
                use_pivot: reader.flag()?,
            };
            validate_camera(&camera)?;
            (Some(display), Some(camera))
        } else {
            (None, None)
        };
        let count = reader.count(MAX_SUBTOOLS, "mesh count")?;
        reader.require(
            count
                .checked_mul(mesh_header_bytes(version))
                .ok_or("SGL metadata size overflow")?,
        )?;
        let mut meshes = reader.allocate(count)?;
        let mut total_points = 0_usize;
        let mut total_triangles = 0_usize;
        let mut total_uv = 0_usize;
        for index in 0..count {
            let render = if version >= 2 {
                let render = SglRenderSettings {
                    shader: reader.word()?,
                    matcap: reader.word()?,
                    show_wireframe: reader.flag()?,
                    flat_shading: reader.flag()?,
                    opacity: reader.float()?,
                };
                validate_render(&render)?;
                Some(render)
            } else {
                None
            };
            let center = reader.floats::<3>()?;
            let matrix = reader.floats::<16>()?;
            let scale = reader.float()?;
            validate_transform(&center, &matrix, scale)?;
            let points = reader.count(MAX_VERTICES, "vertex count")?;
            if points == 0 {
                return Err(format!("SGL mesh {index} requires vertices"));
            }
            total_points = add_count(total_points, points, MAX_VERTICES, "aggregate vertex")?;
            let positions = reader.float_records::<3>(points)?;
            let colors = read_optional_triples(&mut reader, points, "color")?;
            let materials = read_optional_triples(&mut reader, points, "material")?;
            let face_count = reader.count(MAX_TRIANGLES, "face count")?;
            if face_count == 0 {
                return Err(format!("SGL mesh {index} requires faces"));
            }
            // Every polygon produces at least one triangle. Check before allocating.
            if total_triangles
                .checked_add(face_count)
                .is_none_or(|n| n > MAX_TRIANGLES)
            {
                return Err("SGL aggregate triangle limit exceeded".into());
            }
            let faces = reader.index_records(face_count)?;
            let triangle_count = validate_faces(&faces, points, version, false)?;
            total_triangles = add_count(
                total_triangles,
                triangle_count,
                MAX_TRIANGLES,
                "aggregate triangle",
            )?;
            let uv_count = reader.count(MAX_SGL_UV_COORDS, "UV coordinate count")?;
            total_uv = add_count(
                total_uv,
                uv_count,
                MAX_SGL_UV_COORDS,
                "aggregate UV coordinate",
            )?;
            let coordinates = reader.float_records::<2>(uv_count)?;
            let uv_faces = reader.count(MAX_TRIANGLES, "UV face count")?;
            let face_indices = reader.index_records(uv_faces)?;
            let uv = if uv_count == 0 && uv_faces == 0 {
                None
            } else {
                if uv_count == 0 || uv_faces != face_count {
                    return Err(
                        "SGL UV coordinates require complete per-face corner mappings".into(),
                    );
                }
                validate_faces(&face_indices, uv_count, version, true)?;
                for (face, map) in faces.iter().zip(&face_indices) {
                    if is_triangle(face, version) != is_triangle(map, version) {
                        return Err("SGL face and UV corner arities differ".into());
                    }
                }
                Some(SglUvData {
                    coordinates,
                    face_indices,
                })
            };
            let mesh = SglMesh {
                render,
                center,
                matrix,
                scale,
                positions,
                colors,
                materials,
                faces,
                uv,
            };
            validate_mesh(&mesh, version)?;
            meshes.push(mesh);
        }
        if reader.offset != source.len() {
            return Err("SGL contains undeclared trailing words".into());
        }
        let scene = Self {
            version,
            byte_order,
            display,
            camera,
            meshes,
        };
        scene.validate()?;
        Ok(scene)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !supported_version(self.version) {
            return Err("Only SGL legacy0/1 and modern2/3 layouts are supported".into());
        }
        if self.version >= 2 {
            if self.display.is_none() {
                return Err("Modern SGL requires encoded display metadata".into());
            }
            validate_camera(
                self.camera
                    .as_ref()
                    .ok_or("Modern SGL requires encoded camera metadata")?,
            )?;
        } else if self.display.is_some() || self.camera.is_some() {
            return Err("Legacy SGL has no display/camera metadata; do not invent fields".into());
        }
        if self.meshes.len() > MAX_SUBTOOLS {
            return Err("SGL scene exceeds32 meshes".into());
        }
        let mut points = 0_usize;
        let mut triangles = 0_usize;
        let mut uv = 0_usize;
        let mut bytes =
            std::mem::size_of::<Self>() + self.meshes.capacity() * std::mem::size_of::<SglMesh>();
        if bytes > MAX_INPUT_BYTES {
            return Err("SGL retained scene storage exceeds128 MiB".into());
        }
        let mut encoded = header_bytes(self.version);
        for mesh in &self.meshes {
            validate_mesh(mesh, self.version)?;
            points = add_count(
                points,
                mesh.positions.len(),
                MAX_VERTICES,
                "aggregate vertex",
            )?;
            triangles = add_count(
                triangles,
                validate_faces(&mesh.faces, mesh.positions.len(), self.version, false)?,
                MAX_TRIANGLES,
                "aggregate triangle",
            )?;
            if let Some(data) = &mesh.uv {
                uv = add_count(
                    uv,
                    data.coordinates.len(),
                    MAX_SGL_UV_COORDS,
                    "aggregate UV coordinate",
                )?;
            }
            bytes = bytes
                .checked_add(mesh.storage_bytes()?)
                .ok_or("SGL storage size overflow")?;
            if bytes > MAX_INPUT_BYTES {
                return Err("SGL retained scene storage exceeds128 MiB".into());
            }
            encoded = encoded
                .checked_add(mesh.encoded_bytes(self.version)?)
                .ok_or("SGL encoded size overflow")?;
            if encoded > MAX_INPUT_BYTES {
                return Err("SGL encoded scene exceeds128 MiB".into());
            }
        }
        Ok(())
    }
    /// Encode the same supported version, endian, raw face marker words and
    /// every finite stored float bit. External assets/history are not encoded by SGL.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        encode_borrowed_scene(
            self.version,
            self.byte_order,
            self.display.as_ref(),
            self.camera.as_ref(),
            &self.meshes.iter().collect::<Vec<_>>(),
        )
    }
}
/// Bounded borrowed encoder used by version7 origin pooling. The original raw
/// arrays are never cloned merely to persist provenance.
pub(crate) fn encode_borrowed_scene(
    version: u32,
    order: SglByteOrder,
    display: Option<&SglDisplaySettings>,
    camera: Option<&SglCameraSettings>,
    meshes: &[&SglMesh],
) -> Result<Vec<u8>, String> {
    SglScene {
        version,
        byte_order: order,
        display: display.cloned(),
        camera: camera.cloned(),
        meshes: Vec::new(),
    }
    .validate()?;
    if meshes.len() > MAX_SUBTOOLS {
        return Err("SGL origin mesh count exceeds32".into());
    }
    let mut size = header_bytes(version);
    let (mut points, mut triangles, mut uv) = (0, 0, 0);
    for mesh in meshes {
        validate_mesh(mesh, version)?;
        points = add_count(points, mesh.positions.len(), MAX_VERTICES, "origin vertex")?;
        triangles = add_count(
            triangles,
            validate_faces(&mesh.faces, mesh.positions.len(), version, false)?,
            MAX_TRIANGLES,
            "origin triangle",
        )?;
        uv = add_count(
            uv,
            mesh.uv.as_ref().map_or(0, |v| v.coordinates.len()),
            MAX_SGL_UV_COORDS,
            "origin UV",
        )?;
        size = size
            .checked_add(mesh.encoded_bytes(version)?)
            .ok_or("SGL origin output overflow")?;
        if size > MAX_INPUT_BYTES {
            return Err("SGL origin output exceeds128MiB".into());
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        order,
    };
    writer
        .bytes
        .try_reserve_exact(size)
        .map_err(|_| "Cannot allocate bounded SGL origin")?;
    writer.word(version);
    if let (Some(display), Some(camera)) = (display, camera) {
        writer.flag(display.show_grid);
        writer.flag(display.show_symmetry_line);
        writer.flag(display.show_contour);
        writer.word(camera.projection);
        writer.word(camera.mode);
        writer.float(camera.fov_degrees);
        writer.flag(camera.use_pivot);
    }
    writer.word(meshes.len() as u32);
    for mesh in meshes {
        if let Some(render) = &mesh.render {
            writer.word(render.shader);
            writer.word(render.matcap);
            writer.flag(render.show_wireframe);
            writer.flag(render.flat_shading);
            writer.float(render.opacity);
        }
        writer.floats(&mesh.center);
        writer.floats(&mesh.matrix);
        writer.float(mesh.scale);
        writer.float_records(&mesh.positions);
        writer.optional_triples(mesh.colors.as_deref());
        writer.optional_triples(mesh.materials.as_deref());
        writer.index_records(&mesh.faces);
        if let Some(uv) = &mesh.uv {
            writer.float_records(&uv.coordinates);
            writer.index_records(&uv.face_indices);
        } else {
            writer.word(0);
            writer.word(0);
        }
    }
    debug_assert_eq!(writer.bytes.len(), size);
    Ok(writer.bytes)
}

impl SglMesh {
    /// Original face arity under the scene's marker convention.
    pub fn face_arity(&self, index: usize, version: u32) -> Result<usize, String> {
        if !supported_version(version) {
            return Err("Unsupported SGL face marker version".into());
        }
        let face = self
            .faces
            .get(index)
            .ok_or("SGL face index is out of range")?;
        Ok(if is_triangle(face, version) { 3 } else { 4 })
    }
    pub(crate) fn storage_bytes(&self) -> Result<usize, String> {
        let mut bytes = self
            .positions
            .capacity()
            .checked_mul(12)
            .ok_or("SGL storage overflow")?;
        for data in [&self.colors, &self.materials].into_iter().flatten() {
            bytes = bytes
                .checked_add(
                    data.capacity()
                        .checked_mul(12)
                        .ok_or("SGL storage overflow")?,
                )
                .ok_or("SGL storage overflow")?;
        }
        bytes = bytes
            .checked_add(
                self.faces
                    .capacity()
                    .checked_mul(16)
                    .ok_or("SGL storage overflow")?,
            )
            .ok_or("SGL storage overflow")?;
        if let Some(uv) = &self.uv {
            bytes = bytes
                .checked_add(
                    uv.coordinates
                        .capacity()
                        .checked_mul(8)
                        .ok_or("SGL storage overflow")?,
                )
                .and_then(|n| {
                    uv.face_indices
                        .capacity()
                        .checked_mul(16)
                        .and_then(|m| n.checked_add(m))
                })
                .ok_or("SGL storage overflow")?;
        }
        Ok(bytes)
    }
    fn encoded_bytes(&self, version: u32) -> Result<usize, String> {
        let mut bytes = mesh_header_bytes(version);
        for count in [
            self.positions.len(),
            self.colors.as_ref().map_or(0, Vec::len),
            self.materials.as_ref().map_or(0, Vec::len),
        ] {
            bytes = bytes
                .checked_add(count.checked_mul(12).ok_or("SGL encoded size overflow")?)
                .ok_or("SGL encoded size overflow")?;
        }
        bytes = bytes
            .checked_add(
                self.faces
                    .len()
                    .checked_mul(16)
                    .ok_or("SGL encoded size overflow")?,
            )
            .ok_or("SGL encoded size overflow")?;
        if let Some(uv) = &self.uv {
            bytes = bytes
                .checked_add(
                    uv.coordinates
                        .len()
                        .checked_mul(8)
                        .ok_or("SGL encoded size overflow")?,
                )
                .and_then(|n| {
                    uv.face_indices
                        .len()
                        .checked_mul(16)
                        .and_then(|m| n.checked_add(m))
                })
                .ok_or("SGL encoded size overflow")?;
        }
        Ok(bytes)
    }
}
fn input_size(source: &[u8]) -> Result<(), String> {
    if source.len() < 8 || source.len() > MAX_INPUT_BYTES || !source.len().is_multiple_of(4) {
        Err("SGL requires8 bytes–128 MiB aligned to four-byte words".into())
    } else {
        Ok(())
    }
}
fn header_bytes(version: u32) -> usize {
    if version >= 2 {
        36
    } else {
        8
    }
}
fn mesh_header_bytes(version: u32) -> usize {
    if version >= 2 {
        124
    } else {
        104
    }
}
fn supported_version(version: u32) -> bool {
    version <= 3
}
fn is_triangle(face: &[u32; 4], version: u32) -> bool {
    if version <= 2 {
        face[3] > i32::MAX as u32
    } else {
        face[3] == TRIANGLE_MARKER
    }
}
fn add_count(before: usize, count: usize, limit: usize, label: &str) -> Result<usize, String> {
    let total = before
        .checked_add(count)
        .ok_or_else(|| format!("SGL {label} count overflow"))?;
    if total > limit {
        Err(format!("SGL {label} limit exceeded"))
    } else {
        Ok(total)
    }
}
fn validate_camera(camera: &SglCameraSettings) -> Result<(), String> {
    if camera.projection > 1
        || camera.mode > 2
        || !camera.fov_degrees.is_finite()
        || !(0.0..180.0).contains(&camera.fov_degrees)
        || camera.fov_degrees == 0.0
    {
        Err("SGL camera projection/mode/FOV is invalid".into())
    } else {
        Ok(())
    }
}
pub(crate) fn validate_render(render: &SglRenderSettings) -> Result<(), String> {
    if render.shader > 12 || !render.opacity.is_finite() || !(0.0..=1.0).contains(&render.opacity) {
        Err("SGL shader or opacity is invalid".into())
    } else {
        Ok(())
    }
}
fn coordinate(value: f32, label: &str) -> Result<(), String> {
    if !value.is_finite() || value.abs() > MAX_COORDINATE {
        Err(format!("SGL {label} is non-finite or exceeds1,000,000"))
    } else {
        Ok(())
    }
}
fn validate_transform(center: &[f32; 3], matrix: &[f32; 16], scale: f32) -> Result<(), String> {
    for &value in center {
        coordinate(value, "center")?;
    }
    for &value in matrix {
        coordinate(value, "matrix coefficient")?;
    }
    if matrix[3] != 0.0 || matrix[7] != 0.0 || matrix[11] != 0.0 || matrix[15] != 1.0 {
        return Err("SGL matrix must be affine with last row0,0,0,1".into());
    }
    let m = matrix.map(f64::from);
    let determinant = m[0] * (m[5] * m[10] - m[9] * m[6]) - m[4] * (m[1] * m[10] - m[9] * m[2])
        + m[8] * (m[1] * m[6] - m[5] * m[2]);
    if !determinant.is_finite() || determinant == 0.0 {
        return Err("SGL transform is singular".into());
    }
    if !scale.is_finite() || scale <= 0.0 || scale > MAX_COORDINATE {
        return Err("SGL serialized scale must be finite and positive within limits".into());
    }
    Ok(())
}
fn validate_faces(
    faces: &[[u32; 4]],
    count: usize,
    version: u32,
    uv: bool,
) -> Result<usize, String> {
    let mut triangles = 0_usize;
    for face in faces {
        let arity = if is_triangle(face, version) { 3 } else { 4 };
        if face[..arity].iter().any(|&index| index as usize >= count) {
            return Err("SGL face or UV index refers to a missing element".into());
        }
        if !uv {
            for a in 0..arity {
                for b in a + 1..arity {
                    if face[a] == face[b] {
                        return Err("SGL geometric face repeats a vertex".into());
                    }
                }
            }
        }
        triangles = triangles
            .checked_add(arity - 2)
            .ok_or("SGL triangle count overflow")?;
    }
    Ok(triangles)
}
pub(crate) fn validate_mesh(mesh: &SglMesh, version: u32) -> Result<(), String> {
    if version >= 2 {
        validate_render(
            mesh.render
                .as_ref()
                .ok_or("Modern SGL mesh requires encoded render metadata")?,
        )?;
    } else if mesh.render.is_some() {
        return Err("Legacy SGL mesh has no render metadata; do not invent fields".into());
    }
    validate_transform(&mesh.center, &mesh.matrix, mesh.scale)?;
    if mesh.positions.is_empty()
        || mesh.positions.len() > MAX_VERTICES
        || mesh.faces.is_empty()
        || mesh.faces.len() > MAX_TRIANGLES
    {
        return Err("SGL mesh point/face counts are outside limits".into());
    }
    let m = mesh.matrix.map(f64::from);
    for point in &mesh.positions {
        for &value in point {
            coordinate(value, "position")?;
        }
        for row in 0..3 {
            let world = m[row] * f64::from(point[0])
                + m[4 + row] * f64::from(point[1])
                + m[8 + row] * f64::from(point[2])
                + m[12 + row];
            if !world.is_finite() || world.abs() > f64::from(MAX_COORDINATE) {
                return Err("SGL transformed position exceeds coordinate limits".into());
            }
        }
    }
    for (label, data) in [("RGB", &mesh.colors), ("material", &mesh.materials)] {
        if let Some(data) = data {
            if data.len() != mesh.positions.len() {
                return Err(format!(
                    "SGL {label} count must be zero or exactly the vertex count"
                ));
            }
            for values in data {
                if values
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                {
                    return Err(format!("SGL {label} channels must be finite in0–1"));
                }
            }
        }
    }
    validate_faces(&mesh.faces, mesh.positions.len(), version, false)?;
    if let Some(uv) = &mesh.uv {
        if uv.coordinates.is_empty()
            || uv.coordinates.len() > MAX_SGL_UV_COORDS
            || uv.face_indices.len() != mesh.faces.len()
        {
            return Err("SGL UV data requires bounded coordinates and one mapping per face".into());
        }
        for point in &uv.coordinates {
            for &value in point {
                coordinate(value, "UV coordinate")?;
            }
        }
        validate_faces(&uv.face_indices, uv.coordinates.len(), version, true)?;
        if mesh
            .faces
            .iter()
            .zip(&uv.face_indices)
            .any(|(face, map)| is_triangle(face, version) != is_triangle(map, version))
        {
            return Err("SGL face and UV corner arities differ".into());
        }
    }
    Ok(())
}
fn read_optional_triples(
    reader: &mut Reader<'_>,
    vertices: usize,
    label: &str,
) -> Result<Option<Vec<[f32; 3]>>, String> {
    let count = reader.count(MAX_VERTICES, label)?;
    if count == 0 {
        Ok(None)
    } else if count != vertices {
        Err(format!("SGL {label} count must be zero or vertex count"))
    } else {
        Ok(Some(reader.float_records(count)?))
    }
}
struct Reader<'a> {
    source: &'a [u8],
    offset: usize,
    order: SglByteOrder,
    allocated: usize,
}
impl Reader<'_> {
    fn require(&self, bytes: usize) -> Result<(), String> {
        if self
            .offset
            .checked_add(bytes)
            .is_none_or(|end| end > self.source.len())
        {
            Err("SGL is truncated for its declared counts".into())
        } else {
            Ok(())
        }
    }
    fn word(&mut self) -> Result<u32, String> {
        self.require(4)?;
        let bytes = self.source[self.offset..self.offset + 4]
            .try_into()
            .unwrap();
        self.offset += 4;
        Ok(match self.order {
            SglByteOrder::LittleEndian => u32::from_le_bytes(bytes),
            SglByteOrder::BigEndian => u32::from_be_bytes(bytes),
        })
    }
    fn float(&mut self) -> Result<f32, String> {
        let value = f32::from_bits(self.word()?);
        if !value.is_finite() {
            Err("SGL float field is non-finite".into())
        } else {
            Ok(value)
        }
    }
    fn floats<const N: usize>(&mut self) -> Result<[f32; N], String> {
        let mut values = [0.0; N];
        for value in &mut values {
            *value = self.float()?;
        }
        Ok(values)
    }
    fn flag(&mut self) -> Result<bool, String> {
        match self.word()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("SGL flag must be canonical0 or1".into()),
        }
    }
    fn count(&mut self, max: usize, label: &str) -> Result<usize, String> {
        let count = self.word()? as usize;
        if count > max {
            Err(format!("SGL {label} allocation limit exceeded"))
        } else {
            Ok(count)
        }
    }
    fn allocate<T>(&mut self, count: usize) -> Result<Vec<T>, String> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or("SGL allocation size overflow")?;
        self.allocated = self
            .allocated
            .checked_add(bytes)
            .ok_or("SGL allocation aggregate overflow")?;
        if self.allocated > MAX_INPUT_BYTES {
            return Err("SGL aggregate allocation exceeds128 MiB".into());
        }
        let mut items = Vec::new();
        items
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate bounded SGL record")?;
        Ok(items)
    }
    fn float_records<const N: usize>(&mut self, count: usize) -> Result<Vec<[f32; N]>, String> {
        self.require(
            count
                .checked_mul(N)
                .and_then(|n| n.checked_mul(4))
                .ok_or("SGL float count overflow")?,
        )?;
        let mut records = self.allocate(count)?;
        for _ in 0..count {
            records.push(self.floats()?);
        }
        Ok(records)
    }
    fn index_records(&mut self, count: usize) -> Result<Vec<[u32; 4]>, String> {
        self.require(count.checked_mul(16).ok_or("SGL index count overflow")?)?;
        let mut records = self.allocate(count)?;
        for _ in 0..count {
            records.push([self.word()?, self.word()?, self.word()?, self.word()?]);
        }
        Ok(records)
    }
}
struct Writer {
    bytes: Vec<u8>,
    order: SglByteOrder,
}
impl Writer {
    fn word(&mut self, value: u32) {
        self.bytes.extend_from_slice(&match self.order {
            SglByteOrder::LittleEndian => value.to_le_bytes(),
            SglByteOrder::BigEndian => value.to_be_bytes(),
        });
    }
    fn float(&mut self, value: f32) {
        self.word(value.to_bits());
    }
    fn flag(&mut self, value: bool) {
        self.word(u32::from(value));
    }
    fn floats(&mut self, values: &[f32]) {
        for &value in values {
            self.float(value);
        }
    }
    fn float_records<const N: usize>(&mut self, records: &[[f32; N]]) {
        self.word(records.len() as u32);
        for record in records {
            self.floats(record);
        }
    }
    fn optional_triples(&mut self, records: Option<&[[f32; 3]]>) {
        if let Some(records) = records {
            self.float_records(records);
        } else {
            self.word(0);
        }
    }
    fn index_records(&mut self, records: &[[u32; 4]]) {
        self.word(records.len() as u32);
        for record in records {
            for &value in record {
                self.word(value);
            }
        }
    }
}
