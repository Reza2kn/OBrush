//! Version7 scene semantics. One editable state owns the active mesh; original
//! SGL records are immutable provenance, not a second mutable scene engine.
use crate::{
    Mesh, ObjectAffineF32V1, SglByteOrder, SglObjectOrigin, SglPickingPolicy, SglRenderSettings,
    SglScene, SglSceneOrigin, MAX_SUBTOOLS, MAX_TRIANGLES, MAX_UV_COORDINATES, MAX_VERTICES,
};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, sync::Arc};

#[derive(Clone, Debug)]
pub struct SceneObjectOrigin {
    /// Original header belongs to this origin; a later scene header may differ.
    pub header: SglSceneOrigin,
    pub object: SglObjectOrigin,
}
#[derive(Clone, Debug)]
pub struct SceneObjectMetadata {
    /// Explicit Source import center is current export state; origin stays immutable.
    pub source_center_initialized: bool,
    /// Editable local-to-world pose; raw encoded center/scale remain in origin.
    pub affine: ObjectAffineF32V1,
    pub render: Option<SglRenderSettings>,
    pub picking_policy: SglPickingPolicy,
    pub origin: Option<SceneObjectOrigin>,
}
impl SceneObjectMetadata {
    pub fn validate(&self, mesh: &Mesh) -> Result<(), String> {
        self.affine.validate()?;
        for &p in &mesh.positions {
            self.affine.world_point_validated(p)?;
        }
        self.validate_non_position_fields()
    }

    /// A resident replacement has an exact previously validated source and
    /// an unchanged pose. Only these decoded position rows can change world
    /// bounds; current render and raw-origin metadata are still validated.
    pub(crate) fn validate_resident_sparse_positions(
        &self,
        mesh: &Mesh,
        changed_vertices: &[usize],
    ) -> Result<(), String> {
        self.affine.validate()?;
        for &vertex in changed_vertices {
            let point = *mesh
                .positions
                .get(vertex)
                .ok_or("Resident changed vertex is out of range")?;
            self.affine.world_point_validated(point)?;
        }
        self.validate_non_position_fields()
    }

    fn validate_non_position_fields(&self) -> Result<(), String> {
        if let Some(render) = &self.render {
            crate::sgl::validate_render(render)?;
        }
        if let Some(origin) = &self.origin {
            validate_header(&origin.header)?;
            if origin.header.version != origin.object.version
                || origin.header.byte_order != origin.object.byte_order
            {
                return Err("Object origin version/order differs from its header".into());
            }
            crate::sgl::validate_mesh(&origin.object.raw, origin.object.version)?;
        }
        Ok(())
    }
    pub(crate) fn same_bits(&self, b: &Self) -> bool {
        self.affine.local_to_world.map(f32::to_bits) == b.affine.local_to_world.map(f32::to_bits)
            && self.affine.interaction_center.map(f32::to_bits)
                == b.affine.interaction_center.map(f32::to_bits)
            && render_bits_equal(self.render.as_ref(), b.render.as_ref())
            && self.picking_policy == b.picking_policy
            && self.source_center_initialized == b.source_center_initialized
            && match (&self.origin, &b.origin) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    header_bits_equal(&a.header, &b.header)
                        && a.object.version == b.object.version
                        && a.object.byte_order == b.object.byte_order
                        && (Arc::ptr_eq(&a.object.raw, &b.object.raw)
                            || raw_bits_equal(&a.object.raw, &b.object.raw))
                }
                _ => false,
            }
    }
}
pub(crate) fn optional_metadata_equal(
    a: Option<&SceneObjectMetadata>,
    b: Option<&SceneObjectMetadata>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_bits(b),
        _ => false,
    }
}
pub(crate) fn header_bits_equal(a: &SglSceneOrigin, b: &SglSceneOrigin) -> bool {
    a.version == b.version
        && a.byte_order == b.byte_order
        && a.display == b.display
        && match (&a.camera, &b.camera) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.projection == b.projection
                    && a.mode == b.mode
                    && a.use_pivot == b.use_pivot
                    && a.fov_degrees.to_bits() == b.fov_degrees.to_bits()
            }
            _ => false,
        }
}
pub(crate) fn validate_header(header: &SglSceneOrigin) -> Result<(), String> {
    SglScene {
        version: header.version,
        byte_order: header.byte_order,
        display: header.display.clone(),
        camera: header.camera.clone(),
        meshes: Vec::new(),
    }
    .validate()
}
fn render_bits_equal(a: Option<&SglRenderSettings>, b: Option<&SglRenderSettings>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.shader == b.shader
                && a.matcap == b.matcap
                && a.show_wireframe == b.show_wireframe
                && a.flat_shading == b.flat_shading
                && a.opacity.to_bits() == b.opacity.to_bits()
        }
        _ => false,
    }
}
fn raw_bits_equal(a: &crate::SglMesh, b: &crate::SglMesh) -> bool {
    fn floats<const N: usize>(a: &[[f32; N]], b: &[[f32; N]]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
    }
    fn channel(a: Option<&[[f32; 3]]>, b: Option<&[[f32; 3]]>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => floats(a, b),
            _ => false,
        }
    }
    render_bits_equal(a.render.as_ref(), b.render.as_ref())
        && a.center.map(f32::to_bits) == b.center.map(f32::to_bits)
        && a.matrix.map(f32::to_bits) == b.matrix.map(f32::to_bits)
        && a.scale.to_bits() == b.scale.to_bits()
        && floats(&a.positions, &b.positions)
        && channel(a.colors.as_deref(), b.colors.as_deref())
        && channel(a.materials.as_deref(), b.materials.as_deref())
        && a.faces == b.faces
        && match (&a.uv, &b.uv) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                floats(&a.coordinates, &b.coordinates) && a.face_indices == b.face_indices
            }
            _ => false,
        }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SceneV7Scheme {
    OptionalActiveAffineOriginsV1,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SceneV7Project {
    pub(crate) scheme: SceneV7Scheme,
    #[serde(deserialize_with = "bounded_objects")]
    pub(crate) selected_ids: Vec<u32>,
    pub(crate) header: Option<SglSceneOrigin>,
    #[serde(deserialize_with = "bounded_objects")]
    pub(crate) objects: Vec<ObjectPoseProject>,
    #[serde(deserialize_with = "bounded_objects")]
    pub(crate) origins: Vec<OriginBlob>,
}
fn center_policy_is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectPoseProject {
    #[serde(default, skip_serializing_if = "center_policy_is_false")]
    pub(crate) source_center_initialized: bool,
    pub(crate) id: u32,
    pub(crate) affine: ObjectAffineF32V1,
    pub(crate) render: Option<SglRenderSettings>,
    pub(crate) picking_policy: SglPickingPolicy,
    pub(crate) origin: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OriginBlob {
    pub(crate) byte_order: SglByteOrder,
    #[serde(with = "hex_blob")]
    pub(crate) bytes: Vec<u8>,
}
fn bounded_objects<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Bounded<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("at most32 scene records")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut result = Vec::new();
            loop {
                if result.len() == MAX_SUBTOOLS {
                    if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(A::Error::custom("Scene record count exceeds32"));
                    }
                    return Ok(result);
                }
                let Some(value) = seq.next_element()? else {
                    return Ok(result);
                };
                result.try_reserve_exact(1).map_err(A::Error::custom)?;
                result.push(value);
            }
        }
    }
    deserializer.deserialize_seq(Bounded(std::marker::PhantomData))
}
mod hex_blob {
    use super::*;
    pub(super) fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        struct Hex<'a>(&'a [u8]);
        impl fmt::Display for Hex<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }
        serializer.collect_str(&Hex(bytes))
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        struct HexVisitor;
        impl Visitor<'_> for HexVisitor {
            type Value = Vec<u8>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded lowercase hexadecimal SGL bytes")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if !value.len().is_multiple_of(2)
                    || value.len() > crate::resource_limits::MAX_PROJECT_BYTES
                {
                    return Err(E::custom("Origin hex size exceeds bounded input"));
                }
                fn nibble(v: u8) -> Option<u8> {
                    match v {
                        b'0'..=b'9' => Some(v - b'0'),
                        b'a'..=b'f' => Some(v - b'a' + 10),
                        _ => None,
                    }
                }
                // Check all characters before allocation, and charge the shared
                // JSON decode budget before allocating this temporary blob.
                if value.bytes().any(|b| nibble(b).is_none()) {
                    return Err(E::custom("Invalid origin hexadecimal"));
                }
                let size = value.len() / 2;
                crate::decode_limits::capacity(crate::decode_limits::Channel::Positions, size)
                    .map_err(E::custom)?;
                let mut bytes = Vec::new();
                bytes.try_reserve_exact(size).map_err(E::custom)?;
                for pair in value.as_bytes().chunks_exact(2) {
                    bytes.push(nibble(pair[0]).unwrap() * 16 + nibble(pair[1]).unwrap());
                }
                Ok(bytes)
            }
        }
        deserializer.deserialize_str(HexVisitor)
    }
}
type DecodedSceneMetadata = (
    Vec<u32>,
    Option<SglSceneOrigin>,
    Vec<(u32, SceneObjectMetadata)>,
);
impl SceneV7Project {
    pub(crate) fn from_metadata(
        selected_ids: &[u32],
        header: Option<&SglSceneOrigin>,
        objects: &[(u32, &SceneObjectMetadata)],
    ) -> Result<Self, String> {
        if selected_ids.len() > MAX_SUBTOOLS || objects.len() > MAX_SUBTOOLS {
            return Err("v7 scene metadata exceeds32 objects".into());
        }
        if let Some(header) = header {
            validate_header(header)?;
        }
        let mut origins = Vec::<OriginBlob>::new();
        let mut shared = Vec::<&SceneObjectOrigin>::new();
        let mut poses = Vec::new();
        let mut encoded = 0usize;
        for &(id, metadata) in objects {
            let origin = if let Some(origin) = &metadata.origin {
                if let Some(index) = shared.iter().position(|known| {
                    Arc::ptr_eq(&known.object.raw, &origin.object.raw)
                        && header_bits_equal(&known.header, &origin.header)
                }) {
                    Some(index)
                } else {
                    let bytes = crate::sgl::encode_borrowed_scene(
                        origin.object.version,
                        origin.object.byte_order,
                        origin.header.display.as_ref(),
                        origin.header.camera.as_ref(),
                        &[&origin.object.raw],
                    )?;
                    encoded = encoded
                        .checked_add(bytes.len())
                        .ok_or("Origin pool output overflow")?;
                    if encoded > crate::resource_limits::MAX_PROJECT_BYTES / 2 {
                        return Err(
                            "Origin pool hex output exceeds the encoded project size limit".into(),
                        );
                    }
                    let index = origins.len();
                    origins.push(OriginBlob {
                        byte_order: origin.object.byte_order,
                        bytes,
                    });
                    shared.push(origin);
                    Some(index)
                }
            } else {
                None
            };
            poses.push(ObjectPoseProject {
                source_center_initialized: metadata.source_center_initialized,
                id,
                affine: metadata.affine.clone(),
                render: metadata.render.clone(),
                picking_policy: metadata.picking_policy,
                origin,
            });
        }
        Ok(Self {
            scheme: SceneV7Scheme::OptionalActiveAffineOriginsV1,
            selected_ids: selected_ids.to_vec(),
            header: header.cloned(),
            objects: poses,
            origins,
        })
    }
    /// Decode all raw pool entries before assigning any metadata to a Document.
    /// Every pool entry must be referenced; there is no invisible retained data.
    pub(crate) fn into_metadata(self) -> Result<DecodedSceneMetadata, String> {
        if let Some(header) = &self.header {
            validate_header(header)?;
        }
        let mut decoded = Vec::new();
        let (mut vertices, mut triangles, mut uv, mut storage) = (0usize, 0usize, 0usize, 0usize);
        for blob in self.origins {
            // A raw SGL allocation is no larger than its complete binary input
            // plus the fixed Mesh header. Charge before invoking its decoder.
            crate::decode_limits::capacity(
                crate::decode_limits::Channel::Positions,
                blob.bytes
                    .len()
                    .saturating_add(std::mem::size_of::<crate::SglMesh>()),
            )?;
            let mut raw = SglScene::from_bytes_with_order(&blob.bytes, blob.byte_order)?;
            if raw.meshes.len() != 1 {
                return Err("Each v7 origin pool entry must encode exactly one source mesh".into());
            }
            let mesh = raw.meshes.pop().ok_or("Missing origin mesh")?;
            vertices = vertices
                .checked_add(mesh.positions.len())
                .ok_or("Origin point overflow")?;
            for face in 0..mesh.faces.len() {
                triangles = triangles
                    .checked_add(mesh.face_arity(face, raw.version)? - 2)
                    .ok_or("Origin face overflow")?;
            }
            uv = uv
                .checked_add(mesh.uv.as_ref().map_or(0, |v| v.coordinates.len()))
                .ok_or("Origin UV overflow")?;
            storage = storage
                .checked_add(mesh.storage_bytes()?)
                .ok_or("Origin storage overflow")?;
            if vertices > MAX_VERTICES
                || triangles > MAX_TRIANGLES
                || uv > MAX_UV_COORDINATES
                || storage > crate::mesh::MAX_INPUT_BYTES
            {
                return Err("Aggregate origin pool limits exceeded".into());
            }

            decoded.push(SceneObjectOrigin {
                header: SglSceneOrigin {
                    version: raw.version,
                    byte_order: raw.byte_order,
                    display: raw.display,
                    camera: raw.camera,
                },
                object: SglObjectOrigin {
                    version: raw.version,
                    byte_order: raw.byte_order,
                    raw: Arc::new(mesh),
                },
            });
        }
        let mut used = vec![false; decoded.len()];
        let mut result = Vec::new();
        let mut ids = Vec::new();
        for pose in self.objects {
            if pose.id == 0 || ids.contains(&pose.id) {
                return Err("Duplicate/zero v7 metadata object ID".into());
            }
            ids.push(pose.id);
            let origin = match pose.origin {
                None => None,
                Some(index) => {
                    let origin = decoded.get(index).ok_or("Unknown v7 origin pool index")?;
                    used[index] = true;
                    Some(origin.clone())
                }
            };
            pose.affine.validate()?;
            if let Some(render) = &pose.render {
                crate::sgl::validate_render(render)?;
            }
            result.push((
                pose.id,
                SceneObjectMetadata {
                    source_center_initialized: pose.source_center_initialized,
                    affine: pose.affine,
                    render: pose.render,
                    picking_policy: pose.picking_policy,
                    origin,
                },
            ));
        }
        if used.iter().any(|used| !*used) {
            return Err("Unreferenced origin pool data is forbidden".into());
        }
        Ok((self.selected_ids, self.header, result))
    }
}

#[cfg(test)]
mod center_policy_equality_contract {
    use super::*;
    #[test]
    fn metadata_equality_distinguishes_policy_when_all_numeric_state_is_equal() {
        let staged = crate::StagedSglScene::from_sgl_bytes(
            include_bytes!("../tests/fixtures/independent-center-export-v3-le.sgl"),
            &crate::SglStageSettings::default(),
        )
        .unwrap();
        let mut document = crate::Document::empty();
        document.install_staged_sgl_scene(staged).unwrap();
        let a = document.active_object_metadata().unwrap().clone();
        let mut b = a.clone();
        assert!(!a.source_center_initialized);
        assert!(a.same_bits(&b));
        b.source_center_initialized = true;
        assert_eq!(
            a.affine.local_to_world.map(f32::to_bits),
            b.affine.local_to_world.map(f32::to_bits)
        );
        assert_eq!(
            a.affine.interaction_center.map(f32::to_bits),
            b.affine.interaction_center.map(f32::to_bits)
        );
        assert!(Arc::ptr_eq(
            &a.origin.as_ref().unwrap().object.raw,
            &b.origin.as_ref().unwrap().object.raw
        ));
        assert!(!a.same_bits(&b));
        assert!(!b.same_bits(&a));
    }
}
