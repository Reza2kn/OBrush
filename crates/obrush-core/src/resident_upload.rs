//! Bounded, versioned geometry handoff between the studio WASM module and the
//! separately owned WebGPU module. This is transient GPU input, not a project
//! or interchange format; the original Document retains every channel/history.
use crate::resident_point_program::{ResidentPointInput, MAX_RESIDENT_INPUT_BYTES};
use crate::{
    Document, Mesh, ObjectAffineF32V1, SurfaceTopology, Vec3, MAX_TRIANGLES, MAX_VERTICES,
};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"OBRGPU01";
const VERSION: u32 = 2;
const HEADER_BYTES: usize = 8 + 4 * 5 + 16 * 4;
const CHECKSUM_BYTES: usize = 32;

pub struct ResidentUpload {
    pub subtool_id: u32,
    pub local_to_world: [f32; 16],
    pub input: ResidentPointInput,
}

fn wire_len(vertices: usize, triangles: usize, faces: usize) -> Result<usize, String> {
    if vertices == 0
        || vertices > MAX_VERTICES
        || triangles == 0
        || triangles > MAX_TRIANGLES
        || faces > triangles
    {
        return Err("Resident upload geometry count exceeds limits".into());
    }
    let body = vertices
        .checked_mul(28)
        .and_then(|n| triangles.checked_mul(12).and_then(|m| n.checked_add(m)))
        .and_then(|n| faces.checked_mul(16).and_then(|m| n.checked_add(m)))
        .ok_or("Resident upload byte count overflow")?;
    let total = HEADER_BYTES
        .checked_add(body)
        .and_then(|n| n.checked_add(CHECKSUM_BYTES))
        .ok_or("Resident upload byte count overflow")?;
    if total > MAX_RESIDENT_INPUT_BYTES {
        return Err("Resident upload exceeds bounded 128 MiB input".into());
    }
    Ok(total)
}

pub fn encode_document(document: &Document) -> Result<Vec<u8>, String> {
    let mesh = document.require_active_mesh()?;
    if !document.active_subtool_visible() {
        return Err("Resident upload requires a visible active subtool".into());
    }
    mesh.validate()?;
    let subtool_id = document
        .active_subtool_id()
        .ok_or("Resident upload has no active subtool ID")?;
    let pose = document.active_object_metadata().map_or_else(
        || glam::Mat4::IDENTITY.to_cols_array(),
        |metadata| metadata.affine.local_to_world,
    );
    ObjectAffineF32V1::from_local_points(pose, &mesh.positions)?;
    let faces = mesh
        .surface
        .as_ref()
        .map_or(0, |surface| surface.faces.len());
    let expected = wire_len(mesh.positions.len(), mesh.triangles.len(), faces)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(expected)
        .map_err(|_| "Resident upload allocation failed")?;
    output.extend_from_slice(MAGIC);
    for value in [
        VERSION,
        subtool_id,
        mesh.positions.len() as u32,
        mesh.triangles.len() as u32,
        faces as u32,
    ] {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for value in pose {
        output.extend_from_slice(&value.to_le_bytes());
    }
    for (index, point) in mesh.positions.iter().enumerate() {
        for value in point.to_array() {
            output.extend_from_slice(&value.to_le_bytes());
        }
        let influence = mesh.mask_convention.influence(mesh.masks[index]);
        output.extend_from_slice(&influence.to_le_bytes());
        for value in mesh.colors[index] {
            output.extend_from_slice(&value.to_le_bytes());
        }
    }
    for triangle in &mesh.triangles {
        for value in triangle {
            output.extend_from_slice(&value.to_le_bytes());
        }
    }
    if let Some(surface) = &mesh.surface {
        for face in &surface.faces {
            for value in face {
                output.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    let digest = checksum(VERSION, &output)?;
    output.extend_from_slice(&digest);
    debug_assert_eq!(output.len(), expected);
    Ok(output)
}

pub fn decode(bytes: &[u8]) -> Result<ResidentUpload, String> {
    if bytes.len() < HEADER_BYTES + CHECKSUM_BYTES
        || bytes.len() > MAX_RESIDENT_INPUT_BYTES
        || &bytes[..8] != MAGIC
    {
        return Err("Resident upload header or size is invalid".into());
    }
    let word = |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let version = word(8);
    if version != 1 && version != VERSION {
        return Err("Unsupported resident upload version".into());
    }
    let subtool_id = word(12);
    let vertices = word(16) as usize;
    let triangles = word(20) as usize;
    let faces = word(24) as usize;
    if bytes.len() != wire_len(vertices, triangles, faces)? {
        return Err("Resident upload byte count is inconsistent".into());
    }
    let expected = checksum(version, &bytes[..bytes.len() - CHECKSUM_BYTES])?;
    if expected.as_slice() != &bytes[bytes.len() - CHECKSUM_BYTES..] {
        return Err("Resident upload checksum mismatch".into());
    }
    let local_to_world = std::array::from_fn(|index| {
        f32::from_le_bytes(bytes[28 + index * 4..32 + index * 4].try_into().unwrap())
    });
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut masks = Vec::new();
    let mut colors = Vec::new();
    let mut triangle_rows = Vec::new();
    let mut face_rows = Vec::new();
    for rows in [
        &mut positions as &mut dyn TryReserve,
        &mut normals,
        &mut masks,
        &mut colors,
    ] {
        rows.reserve(vertices)?;
    }
    triangle_rows
        .try_reserve_exact(triangles)
        .map_err(|_| "Resident triangle allocation failed")?;
    face_rows
        .try_reserve_exact(faces)
        .map_err(|_| "Resident face allocation failed")?;
    let mut offset = HEADER_BYTES;
    for _ in 0..vertices {
        let mut next_f32 = || {
            let value = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            offset += 4;
            value
        };
        positions.push(Vec3::new(next_f32(), next_f32(), next_f32()));
        masks.push(next_f32());
        colors.push([next_f32(), next_f32(), next_f32()]);
        normals.push(Vec3::ZERO);
    }
    let mut next_u32 = || {
        let value = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        value
    };
    for _ in 0..triangles {
        triangle_rows.push([next_u32(), next_u32(), next_u32()]);
    }
    for _ in 0..faces {
        face_rows.push([next_u32(), next_u32(), next_u32(), next_u32()]);
    }
    debug_assert_eq!(offset + CHECKSUM_BYTES, bytes.len());
    let mesh = Mesh::from_resident_upload_parts(
        positions,
        normals,
        triangle_rows,
        masks,
        colors,
        (faces > 0).then_some(SurfaceTopology {
            faces: face_rows,
            uv: None,
        }),
    );
    mesh.validate()?;
    ObjectAffineF32V1::from_local_points(local_to_world, &mesh.positions)?;
    let input = ResidentPointInput::capture(&mesh)?;
    Ok(ResidentUpload {
        subtool_id,
        local_to_world,
        input,
    })
}

fn checksum(version: u32, bytes: &[u8]) -> Result<[u8; CHECKSUM_BYTES], String> {
    match version {
        1 => Ok(Sha256::digest(bytes).into()),
        VERSION => Ok(*blake3::hash(bytes).as_bytes()),
        _ => Err("Unsupported resident upload version".into()),
    }
}

trait TryReserve {
    fn reserve(&mut self, count: usize) -> Result<(), String>;
}
impl<T> TryReserve for Vec<T> {
    fn reserve(&mut self, count: usize) -> Result<(), String> {
        self.try_reserve_exact(count)
            .map_err(|_| "Resident vertex allocation failed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resident_point_program::ResidentDocumentInput;

    fn compare(document: &Document) -> Vec<u8> {
        let ordinary = ResidentDocumentInput::capture(document).unwrap();
        let bytes = encode_document(document).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.subtool_id, ordinary.subtool_id());
        assert_eq!(decoded.local_to_world, *ordinary.local_to_world());
        assert_eq!(decoded.input.positions(), ordinary.points().positions());
        assert_eq!(decoded.input.normal_u(), ordinary.points().normal_u());
        assert_eq!(decoded.input.colors(), ordinary.points().colors());
        assert_eq!(decoded.input.triangles(), ordinary.points().triangles());
        let a = decoded.input.normal_topology();
        let b = ordinary.points().normal_topology();
        assert_eq!(a.headers(), b.headers());
        assert_eq!(a.offsets(), b.offsets());
        assert_eq!(a.rings(), b.rings());
        assert_eq!(a.face_normals(), b.face_normals());
        bytes
    }

    #[test]
    fn compact_upload_matches_actual_document_capture_with_masks_and_original_faces() {
        let mut document = Document::new(Mesh::sphere(2));
        let mesh = document.active_mesh_mut().unwrap();
        mesh.masks[0] = 0.25;
        mesh.colors[1] = [0.1, 0.2, 0.3];
        let bytes = compare(&document);
        let mesh = document.active_mesh().unwrap();
        assert_eq!(
            bytes.len(),
            wire_len(mesh.positions.len(), mesh.triangles.len(), 0).unwrap()
        );

        let imported = Mesh::import_obj(
            b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n\
              vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\n",
        )
        .unwrap()
        .mesh;
        let textured = Document::new(imported);
        let bytes = compare(&textured);
        assert_eq!(word_at(&bytes, 24), 1);
    }

    fn word_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn rewrite_checksum(bytes: &mut [u8]) {
        let at = bytes.len() - CHECKSUM_BYTES;
        let digest = super::checksum(word_at(bytes, 8), &bytes[..at]).unwrap();
        bytes[at..].copy_from_slice(&digest);
    }

    #[test]
    fn compact_upload_rejects_truncation_corruption_counts_and_invalid_mesh() {
        let document = Document::new(Mesh::sphere(1));
        let original = encode_document(&document).unwrap();
        assert!(decode(&original[..original.len() - 1]).is_err());
        let mut changed = original.clone();
        changed[HEADER_BYTES + 3] ^= 1;
        assert!(decode(&changed).err().unwrap().contains("checksum"));
        let mut changed = original.clone();
        changed[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&changed).is_err());
        let mut changed = original.clone();
        // First vertex mask follows its three position words.
        changed[HEADER_BYTES + 12..HEADER_BYTES + 16].copy_from_slice(&2.0f32.to_le_bytes());
        rewrite_checksum(&mut changed);
        assert!(decode(&changed).err().unwrap().contains("Mask"));
        let mut changed = original;
        let triangle_start = HEADER_BYTES + document.active_mesh().unwrap().positions.len() * 28;
        changed[triangle_start..triangle_start + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        rewrite_checksum(&mut changed);
        assert!(decode(&changed).err().unwrap().contains("Triangle"));
    }

    #[test]
    fn legacy_sha256_upload_remains_decodable_after_checksum_upgrade() {
        let document = Document::new(Mesh::sphere(1));
        let mut legacy = encode_document(&document).unwrap();
        assert_eq!(word_at(&legacy, 8), VERSION);
        legacy[8..12].copy_from_slice(&1u32.to_le_bytes());
        rewrite_checksum(&mut legacy);
        let decoded = decode(&legacy).unwrap();
        assert_eq!(
            decoded.input.positions().len(),
            document.active_mesh().unwrap().positions.len()
        );
        legacy[HEADER_BYTES] ^= 1;
        assert!(decode(&legacy).err().unwrap().contains("checksum"));
    }
}
