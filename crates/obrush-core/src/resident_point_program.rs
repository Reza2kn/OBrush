//! Experimental, immutable upload/point-program contract. This is not a Source
//! brush implementation: fresh raw normals remain fixed across point passes.
//! Ownership, limits and scalar reference stay usable without a display/GPU.
use crate::{
    Document, MaskConvention, Mesh, SceneRay64, SourceNormalField, SourceNormalPolicy, MAX_VERTICES,
};
use web_time::Instant;

pub const MAX_RESIDENT_INPUT_BYTES: usize = 128 * 1024 * 1024;
pub const RESIDENT_POINT_COMMIT_WGSL: &str = include_str!("resident_point_commit.wgsl");
pub const RESIDENT_POINT_WGSL: &str = include_str!("resident_point_program.wgsl");
pub const RESIDENT_BRUSH_PICK_WGSL: &str = include_str!("resident_brush_pick.wgsl");

/// Wall-time stages for a single checked sparse result. Diagnostics only;
/// neither success nor authorization depends on these measurements.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct ResidentCommitTiming {
    pub source_match_ms: f64,
    pub decode_ms: f64,
    pub snapshot_ms: f64,
    pub fold_guard_ms: f64,
    pub replace_ms: f64,
    pub replace_validate_ms: f64,
    pub replace_normals_ms: f64,
    pub replace_scene_ms: f64,
    pub replace_history_ms: f64,
    pub replace_witness_ms: f64,
    pub sparse_normals: bool,
    pub changed_vertices: usize,
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

/// An immutable, single-active-subtool upload from an ordinary OBrush document.
/// The document remains authoritative for edits, history and serialization.
/// Coordinates in `points` stay local; consumers must apply `local_to_world`
/// for drawing and convert world-space picks back to local space.
pub struct ResidentDocumentCapture {
    subtool_id: u32,
    local_to_world: [f32; 16],
    source_mesh_digest: [u8; 32],
    source_mask_convention: MaskConvention,
}

/// Complete reference input for CPU/native GPU probes. The browser studio's
/// one-use handoff needs only `ResidentDocumentCapture` for checked history;
/// it already sends geometry through the separate bounded upload format.
pub struct ResidentDocumentInput {
    capture: ResidentDocumentCapture,
    mask_roundtrip_changed_words: usize,
    points: ResidentPointInput,
}
impl ResidentDocumentInput {
    /// Parse a versioned project through the ordinary checked document decoder.
    /// This returns an upload snapshot, never a replacement document or history.
    pub fn from_project_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > crate::resource_limits::MAX_PROJECT_BYTES {
            return Err("Project exceeds the encoded project input limit".into());
        }
        let source = std::str::from_utf8(bytes).map_err(|_| "Project is not UTF-8")?;
        let document = Document::from_json(source)?;
        Self::capture(&document)
    }
    pub fn capture(document: &Document) -> Result<Self, String> {
        let capture = ResidentDocumentCapture::capture(document)?;
        let mesh = document
            .active_mesh()
            .ok_or("Document has no active mesh")?;
        let (points, mask_roundtrip_changed_words) =
            ResidentPointInput::capture_document_mesh(mesh)?;
        Ok(Self {
            capture,
            mask_roundtrip_changed_words,
            points,
        })
    }
    pub fn mask_roundtrip_changed_words(&self) -> usize {
        self.mask_roundtrip_changed_words
    }
    pub fn points(&self) -> &ResidentPointInput {
        &self.points
    }
    pub fn into_points(self) -> ResidentPointInput {
        self.points
    }
}

impl std::ops::Deref for ResidentDocumentInput {
    type Target = ResidentDocumentCapture;

    fn deref(&self) -> &Self::Target {
        &self.capture
    }
}

impl ResidentDocumentCapture {
    /// Capture only the exact document identity needed for a later checked
    /// sparse commit. No CPU reference normals or GPU-ready geometry are built.
    pub fn capture(document: &Document) -> Result<Self, String> {
        let mesh = document
            .active_mesh()
            .ok_or("Document has no active mesh")?;
        if !document.active_subtool_visible() {
            return Err("Resident input requires a visible active subtool".into());
        }
        let subtool_id = document
            .active_subtool_id()
            .ok_or("Document has no active subtool")?;
        mesh.validate()?;
        let local_to_world = if let Some(metadata) = document.active_object_metadata() {
            metadata.validate(mesh)?;
            metadata.affine.local_to_world
        } else {
            glam::Mat4::IDENTITY.to_cols_array()
        };
        let source_mesh_digest = mesh_digest(mesh)?;
        Ok(Self {
            subtool_id,
            local_to_world,
            source_mesh_digest,
            source_mask_convention: mesh.mask_convention,
        })
    }
    pub fn subtool_id(&self) -> u32 {
        self.subtool_id
    }
    pub fn local_to_world(&self) -> &[f32; 16] {
        &self.local_to_world
    }
    pub fn source_mask_convention(&self) -> MaskConvention {
        self.source_mask_convention
    }
    /// Accept an explicitly read-back RGBA32F position buffer as one ordinary,
    /// undoable document edit. The capture remains immutable: a stale mesh,
    /// changed selection/pose or an open edit rejects before document mutation.
    /// This is a transaction boundary, not an implementation of a Source brush.
    pub fn commit_position_bytes(
        &self,
        document: &mut Document,
        position_bytes: &[u8],
    ) -> Result<bool, String> {
        self.ensure_current(document)?;
        let mesh = document.require_active_mesh()?;
        let expected_bytes = mesh
            .positions
            .len()
            .checked_mul(16)
            .ok_or("Resident position byte count overflow")?;
        if position_bytes.len() != expected_bytes {
            return Err("Resident position result has the wrong byte count".into());
        }
        let mut proposals = Vec::new();
        for (index, row) in position_bytes.chunks_exact(16).enumerate() {
            let point = decode_position(index, row)?;
            if point.to_array().map(f32::to_bits)
                != mesh.positions[index].to_array().map(f32::to_bits)
            {
                check_unprotected(mesh, index)?;
                proposals
                    .try_reserve(1)
                    .map_err(|_| "Cannot reserve resident position proposals")?;
                proposals.push((index, point));
            }
        }
        self.apply_proposals(document, proposals, &mut ResidentCommitTiming::default())
    }

    /// Accept sorted, unique 20-byte records: little-endian u32 vertex ID
    /// followed by RGBA32F position. This avoids transferring the whole mesh
    /// for a local GPU stroke; the document still validates the complete
    /// source identity and owns the final undoable mesh replacement.
    pub fn commit_sparse_position_bytes(
        &self,
        document: &mut Document,
        sparse_bytes: &[u8],
    ) -> Result<bool, String> {
        self.commit_sparse_position_bytes_timed(document, sparse_bytes)
            .map(|(changed, _)| changed)
    }

    pub fn commit_sparse_position_bytes_timed(
        &self,
        document: &mut Document,
        sparse_bytes: &[u8],
    ) -> Result<(bool, ResidentCommitTiming), String> {
        let mut timing = ResidentCommitTiming::default();
        let started = Instant::now();
        self.ensure_current(document)?;
        timing.source_match_ms = elapsed_ms(started);
        let started = Instant::now();
        let mesh = document.require_active_mesh()?;
        if !sparse_bytes.len().is_multiple_of(20) || sparse_bytes.len() / 20 > mesh.positions.len()
        {
            return Err("Resident sparse position result has an invalid byte count".into());
        }
        let mut proposals = Vec::new();
        proposals
            .try_reserve(sparse_bytes.len() / 20)
            .map_err(|_| "Cannot reserve resident sparse position proposals")?;
        let mut previous = None;
        for row in sparse_bytes.chunks_exact(20) {
            let index = u32::from_le_bytes(row[..4].try_into().unwrap()) as usize;
            if index >= mesh.positions.len() || previous.is_some_and(|last| index <= last) {
                return Err(
                    "Resident sparse vertex IDs must be in range and strictly ordered".into(),
                );
            }
            previous = Some(index);
            let point = decode_position(index, &row[4..])?;
            if point.to_array().map(f32::to_bits)
                != mesh.positions[index].to_array().map(f32::to_bits)
            {
                check_unprotected(mesh, index)?;
                proposals.push((index, point));
            }
        }
        timing.decode_ms = elapsed_ms(started);
        timing.changed_vertices = proposals.len();
        let changed = self.apply_proposals(document, proposals, &mut timing)?;
        Ok((changed, timing))
    }

    fn ensure_current(&self, document: &Document) -> Result<(), String> {
        document.resident_position_commit_ready()?;
        if document.active_subtool_id() != Some(self.subtool_id)
            || !document.active_subtool_visible()
        {
            return Err("Resident position result targets a different active subtool".into());
        }
        let current_pose = document
            .active_object_metadata()
            .map_or(glam::Mat4::IDENTITY.to_cols_array(), |m| {
                m.affine.local_to_world
            });
        if current_pose.map(f32::to_bits) != self.local_to_world.map(f32::to_bits) {
            return Err("Resident position result has a stale object pose".into());
        }
        let mesh = document.require_active_mesh()?;
        if mesh_digest(mesh)? != self.source_mesh_digest {
            return Err("Resident position result has a stale source mesh".into());
        }
        Ok(())
    }

    fn apply_proposals(
        &self,
        document: &mut Document,
        proposals: Vec<(usize, glam::Vec3)>,
        timing: &mut ResidentCommitTiming,
    ) -> Result<bool, String> {
        if proposals.is_empty() {
            return Ok(false);
        }
        if document.can_redo() {
            return Err("Resident position result would replace an active redo branch".into());
        }
        let mesh = document.require_active_mesh()?;
        let sparse_normals = mesh.resident_normals_match_witness();
        let dirty_vertices: Vec<_> = proposals.iter().map(|&(index, _)| index).collect();
        let started = Instant::now();
        let mut candidate = mesh
            .try_resident_candidate()
            .map_err(|_| "Cannot reserve resident candidate mesh")?;
        for &(index, point) in &proposals {
            candidate.positions[index] = point;
        }
        timing.snapshot_ms = elapsed_ms(started);
        let started = Instant::now();
        if candidate.accept_source_geometry_candidate_sparse(&mesh.positions, &proposals) != 1. {
            return Err("Resident position result violates the surface fold guard".into());
        }
        timing.fold_guard_ms = elapsed_ms(started);
        // Document::replace_mesh validates this candidate before installation
        // (and again against any retained scene). Avoid a redundant full scan
        // on the browser's synchronous million-vertex release path.
        let started = Instant::now();
        let stages = document.replace_mesh_timed(candidate, &dirty_vertices, sparse_normals)?;
        timing.replace_ms = elapsed_ms(started);
        timing.replace_validate_ms = stages.validate_ms;
        timing.replace_normals_ms = stages.normals_ms;
        timing.replace_scene_ms = stages.scene_ms;
        timing.replace_history_ms = stages.history_ms;
        timing.replace_witness_ms = stages.witness_ms;
        timing.sparse_normals = stages.sparse_normals;
        Ok(true)
    }
    /// Compose the actual world camera and document pose in f64, before the
    /// viewport's checked f32 WebGPU projection boundary.
    pub fn local_to_gl_clip(&self, world_to_gl_clip: [f64; 16]) -> Result<[f64; 16], String> {
        compose_document_camera(self.local_to_world, world_to_gl_clip)
    }
}

fn decode_position(index: usize, row: &[u8]) -> Result<glam::Vec3, String> {
    let words: [f32; 4] = std::array::from_fn(|component| {
        f32::from_le_bytes(row[component * 4..component * 4 + 4].try_into().unwrap())
    });
    if words[3].to_bits() != 1.0_f32.to_bits() {
        return Err(format!(
            "Resident position {index} has an invalid fourth word"
        ));
    }
    let point = glam::Vec3::from_array([words[0], words[1], words[2]]);
    if !point.is_finite() || point.abs().max_element() > 1_000_000. {
        return Err(format!(
            "Resident position {index} is nonfinite or out of bounds"
        ));
    }
    Ok(point)
}

fn check_unprotected(mesh: &Mesh, index: usize) -> Result<(), String> {
    if mesh.mask_convention.influence(mesh.masks[index]) == 0. {
        return Err(format!(
            "Resident position {index} changed a protected vertex"
        ));
    }
    Ok(())
}

/// Canonically hash every serialized mesh field without decimal JSON formatting.
/// The versioned domain, option tags and vector lengths make channel and
/// shape changes unambiguous; adjacency is derived and intentionally omitted.
fn mesh_digest(mesh: &Mesh) -> Result<[u8; 32], String> {
    struct BinaryHash {
        digest: blake3::Hasher,
        buffer: Vec<u8>,
    }
    impl BinaryHash {
        fn new() -> Result<Self, String> {
            let mut buffer = Vec::new();
            buffer
                .try_reserve_exact(64 * 1024)
                .map_err(|_| "Cannot reserve resident mesh fingerprint buffer")?;
            let mut digest = blake3::Hasher::new();
            digest.update(b"OBR-MESH-FP-1");
            Ok(Self { digest, buffer })
        }
        fn word(&mut self, value: u32) {
            if self.buffer.len() == 64 * 1024 {
                self.digest.update(&self.buffer);
                self.buffer.clear();
            }
            self.buffer.extend_from_slice(&value.to_le_bytes());
        }
        fn length(&mut self, count: usize) {
            let count = count as u64;
            self.word(count as u32);
            self.word((count >> 32) as u32);
        }
        #[cfg(target_endian = "little")]
        fn bytes(&mut self, bytes: &[u8]) {
            self.digest.update(&self.buffer);
            self.buffer.clear();
            self.digest.update(bytes);
        }
        fn finish(mut self) -> [u8; 32] {
            self.digest.update(&self.buffer);
            *self.digest.finalize().as_bytes()
        }
    }
    let mut hash = BinaryHash::new()?;
    hash.length(mesh.positions.len());
    #[cfg(target_endian = "little")]
    hash.bytes(bytemuck::cast_slice(&mesh.positions));
    #[cfg(not(target_endian = "little"))]
    for point in &mesh.positions {
        for value in point.to_array() {
            hash.word(value.to_bits());
        }
    }
    hash.length(mesh.normals.len());
    #[cfg(target_endian = "little")]
    hash.bytes(bytemuck::cast_slice(&mesh.normals));
    #[cfg(not(target_endian = "little"))]
    for normal in &mesh.normals {
        for value in normal.to_array() {
            hash.word(value.to_bits());
        }
    }
    hash.length(mesh.triangles.len());
    #[cfg(target_endian = "little")]
    hash.bytes(bytemuck::cast_slice(&mesh.triangles));
    #[cfg(not(target_endian = "little"))]
    for triangle in &mesh.triangles {
        for &value in triangle {
            hash.word(value);
        }
    }
    hash.length(mesh.masks.len());
    #[cfg(target_endian = "little")]
    hash.bytes(bytemuck::cast_slice(&mesh.masks));
    #[cfg(not(target_endian = "little"))]
    for &mask in &mesh.masks {
        hash.word(mask.to_bits());
    }
    hash.word(match mesh.mask_convention {
        crate::MaskConvention::ProtectedF32V1 => 0,
        crate::MaskConvention::UnprotectedF32V1 => 1,
    });
    hash.length(mesh.colors.len());
    #[cfg(target_endian = "little")]
    hash.bytes(bytemuck::cast_slice(&mesh.colors));
    #[cfg(not(target_endian = "little"))]
    for color in &mesh.colors {
        for &value in color {
            hash.word(value.to_bits());
        }
    }
    if let Some(materials) = &mesh.materials {
        hash.word(1);
        hash.length(materials.len());
        #[cfg(target_endian = "little")]
        hash.bytes(bytemuck::cast_slice(materials));
        #[cfg(not(target_endian = "little"))]
        for material in materials {
            for &value in material {
                hash.word(value.to_bits());
            }
        }
    } else {
        hash.word(0);
    }
    if let Some(surface) = &mesh.surface {
        hash.word(1);
        hash.length(surface.faces.len());
        #[cfg(target_endian = "little")]
        hash.bytes(bytemuck::cast_slice(&surface.faces));
        #[cfg(not(target_endian = "little"))]
        for face in &surface.faces {
            for &value in face {
                hash.word(value);
            }
        }
        if let Some(uv) = &surface.uv {
            hash.word(1);
            hash.length(uv.coordinates.len());
            #[cfg(target_endian = "little")]
            hash.bytes(bytemuck::cast_slice(&uv.coordinates));
            #[cfg(not(target_endian = "little"))]
            for coord in &uv.coordinates {
                for &value in coord {
                    hash.word(value.to_bits());
                }
            }
            hash.length(uv.face_indices.len());
            #[cfg(target_endian = "little")]
            hash.bytes(bytemuck::cast_slice(&uv.face_indices));
            #[cfg(not(target_endian = "little"))]
            for face in &uv.face_indices {
                for &value in face {
                    hash.word(value);
                }
            }
        } else {
            hash.word(0);
        }
    } else {
        hash.word(0);
    }
    Ok(hash.finish())
}

/// Stateless camera adapter for an already captured document pose. This
/// validates both matrices again because callers may retain only the pose.
pub fn compose_document_camera(
    local_to_world: [f32; 16],
    world_to_gl_clip: [f64; 16],
) -> Result<[f64; 16], String> {
    if local_to_world[3] != 0.
        || local_to_world[7] != 0.
        || local_to_world[11] != 0.
        || local_to_world[15] != 1.
    {
        return Err("Document pose must be affine".into());
    }
    let world = glam::DMat4::from_cols_array(&world_to_gl_clip);
    if !world.is_finite() || world.determinant() == 0.0 || !world.inverse().is_finite() {
        return Err("World camera matrix is nonfinite or singular".into());
    }
    let pose = glam::DMat4::from_cols_array(&local_to_world.map(f64::from));
    if !pose.is_finite() || pose.determinant() == 0.0 || !pose.inverse().is_finite() {
        return Err("Document pose is nonfinite or singular".into());
    }
    let combined = world * pose;
    if !combined.is_finite() || combined.determinant() == 0.0 || !combined.inverse().is_finite() {
        return Err("Document camera matrix is nonfinite or singular".into());
    }
    Ok(combined.to_cols_array())
}

#[derive(Clone, Copy, Debug)]
pub struct ResidentLocalRay {
    pub ray: SceneRay64,
    pub max_distance: f64,
    /// Multiply a world-space ray parameter by this value to obtain its
    /// local-space parameter under the object's affine pose.
    pub world_to_local_distance_scale: f64,
}

/// Convert an actual world-space studio ray to the resident local mesh. A
/// nonuniform or reflected affine changes ray distance as well as direction;
/// the f64 conversion is completed before bounded GPU f32 pick preparation.
pub fn document_local_ray(
    local_to_world: [f32; 16],
    world_ray: SceneRay64,
    max_world_distance: f64,
) -> Result<ResidentLocalRay, String> {
    world_ray.validate()?;
    if !max_world_distance.is_finite()
        || max_world_distance <= 0.
        || max_world_distance > 2_000_000.
        || (world_ray.direction.length() - 1.).abs() > 1e-8
    {
        return Err("Resident world ray requires unit direction and bounded distance".into());
    }
    let affine = crate::ObjectAffineF32V1 {
        local_to_world,
        interaction_center: [0.; 3],
    };
    affine.validate()?;
    let local_origin = affine.local_point(world_ray.origin)?;
    let inverse = glam::DMat4::from_cols_array(&local_to_world.map(f64::from)).inverse();
    let direction = inverse.transform_vector3(world_ray.direction);
    let scale = direction.length();
    let max_distance = max_world_distance * scale;
    if !scale.is_finite() || scale <= 0. || !max_distance.is_finite() || max_distance > 2_000_000. {
        return Err("Resident local ray exceeds bounded distance".into());
    }
    let ray = SceneRay64 {
        origin: local_origin,
        direction: direction / scale,
    };
    ray.validate()?;
    Ok(ResidentLocalRay {
        ray,
        max_distance,
        world_to_local_distance_scale: scale,
    })
}

pub struct ResidentPointInput {
    positions: Vec<[f32; 4]>,
    normal_u: Vec<[f32; 4]>,
    colors: Vec<[f32; 4]>,
    triangles: Vec<[u32; 3]>,
    normal_topology: crate::resident_normal_program::ResidentNormalTopology,
}
impl ResidentPointInput {
    /// Fully validate before allocating derived rows. The private owned rows
    /// cannot become stale through later public Mesh mutation. The source Mesh
    /// is never changed; this snapshot carries no authority to replace it.
    pub fn capture(mesh: &Mesh) -> Result<Self, String> {
        if mesh.mask_convention != MaskConvention::UnprotectedF32V1 {
            return Err("Resident point input requires explicit raw unprotected masks".into());
        }
        Self::capture_document_mesh(mesh).map(|(input, _)| input)
    }
    fn capture_document_mesh(mesh: &Mesh) -> Result<(Self, usize), String> {
        mesh.validate()?;
        Self::storage_requirement(mesh.positions.len(), mesh.triangles.len())?;
        let normals = SourceNormalField::compute(mesh, SourceNormalPolicy::StaticOriginalFaces)?;
        fn reserve<T>(n: usize) -> Result<Vec<T>, String> {
            let mut rows = Vec::new();
            rows.try_reserve_exact(n)
                .map_err(|_| "Resident input allocation failed")?;
            Ok(rows)
        }
        let normal_topology =
            crate::resident_normal_program::ResidentNormalTopology::capture(mesh, &normals)?;
        let mut positions = reserve(mesh.positions.len())?;
        let mut normal_u = reserve(mesh.positions.len())?;
        let mut colors = reserve(mesh.positions.len())?;
        let mut triangles = reserve(mesh.triangles.len())?;
        let mut mask_roundtrip_changed_words = 0;
        for (i, position) in mesh.positions.iter().enumerate() {
            positions.push([position.x, position.y, position.z, 1.]);
            let n = normals.vertex_normals()[i];
            let raw = mesh.masks[i];
            let influence = mesh.mask_convention.influence(raw);
            if mesh.mask_convention == MaskConvention::ProtectedF32V1
                && (1.0 - influence).to_bits() != raw.to_bits()
            {
                mask_roundtrip_changed_words += 1;
            }
            normal_u.push([n.x, n.y, n.z, influence]);
            let rgb = mesh.colors[i];
            colors.push([rgb[0], rgb[1], rgb[2], 1.]);
        }
        triangles.extend_from_slice(&mesh.triangles);
        Ok((
            Self {
                positions,
                normal_u,
                colors,
                triangles,
                normal_topology,
            },
            mask_roundtrip_changed_words,
        ))
    }
    pub fn storage_requirement(vertices: usize, triangles: usize) -> Result<usize, String> {
        if vertices == 0
            || vertices > MAX_VERTICES
            || triangles == 0
            || triangles > crate::MAX_TRIANGLES
        {
            return Err("Resident input exceeds geometry count limits".into());
        }
        let bytes = vertices
            .checked_mul(52)
            .and_then(|n| triangles.checked_mul(40).and_then(|t| n.checked_add(t)))
            .and_then(|n| n.checked_add(4))
            .ok_or("Resident input storage overflow")?;
        if bytes > MAX_RESIDENT_INPUT_BYTES {
            return Err("Resident input exceeds allocation limit".into());
        }
        Ok(bytes)
    }
    pub fn normal_topology(&self) -> &crate::resident_normal_program::ResidentNormalTopology {
        &self.normal_topology
    }
    pub fn positions(&self) -> &[[f32; 4]] {
        &self.positions
    }
    pub fn normal_u(&self) -> &[[f32; 4]] {
        &self.normal_u
    }
    pub fn colors(&self) -> &[[f32; 4]] {
        &self.colors
    }
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }
    /// Scalar reference uses f64 arithmetic and f32 storage after each pass,
    /// just as the preceding feasibility experiment. GPU f32 equality is not
    /// assumed. An error returns no partial output and cannot alter the input.
    pub fn reference_positions(
        &self,
        parameters: PointInflateParameters,
        passes: u32,
    ) -> Result<Vec<[f32; 4]>, String> {
        parameters.validate()?;
        if passes > 256 {
            return Err("Resident point pass limit exceeded".into());
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(self.positions.len())
            .map_err(|_| "Reference allocation failed")?;
        rows.extend_from_slice(&self.positions);
        let center = parameters.center.map(f64::from);
        let radius = f64::from(parameters.radius);
        let amplitude = f64::from(parameters.intensity) * radius * 0.1;
        for _ in 0..passes {
            for (p, n) in rows.iter_mut().zip(&self.normal_u) {
                let u = f64::from(n[3]);
                if u == 0. {
                    continue;
                }
                let live = [p[0], p[1], p[2]].map(f64::from);
                let delta = std::array::from_fn::<_, 3, _>(|c| live[c] - center[c]);
                let d = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt()
                    / radius;
                if d >= 1. {
                    continue;
                }
                let normal = [n[0], n[1], n[2]].map(f64::from);
                let length = ((normal[0] * normal[0] + normal[1] * normal[1])
                    + normal[2] * normal[2])
                    .sqrt();
                if length == 0. {
                    return Err("Resident point normal is zero".into());
                }
                let q = d * d;
                let weight = ((amplitude * (3. * q * q - 4. * q * d + 1.)) / length) * u;
                for c in 0..3 {
                    let value = (live[c] + normal[c] * weight) as f32;
                    if !value.is_finite() || value.abs() > 1_000_000. {
                        return Err("Resident point candidate exceeds coordinate bounds".into());
                    }
                    p[c] = value;
                }
            }
        }
        Ok(rows)
    }

    /// Scalar reference for the explicit-direction resident Brush candidate.
    /// Repeated passes read the last accepted f32 positions, matching the
    /// accumulate-on/no-alpha branch of the shared Source displacement kernel.
    pub fn reference_brush_positions(
        &self,
        parameters: PointBrushParameters,
        passes: u32,
    ) -> Result<Vec<[f32; 4]>, String> {
        parameters.validate()?;
        if passes > 256 {
            return Err("Resident point pass limit exceeded".into());
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(self.positions.len())
            .map_err(|_| "Reference allocation failed")?;
        rows.extend_from_slice(&self.positions);
        let center = parameters.center.map(f64::from);
        let direction = parameters.direction.map(f64::from);
        let radius = f64::from(parameters.radius);
        let amplitude = f64::from(parameters.intensity) * radius * 0.1;
        for _ in 0..passes {
            for (position, normal_u) in rows.iter_mut().zip(&self.normal_u) {
                let influence = f64::from(normal_u[3]);
                if influence == 0.0 {
                    continue;
                }
                let live = [position[0], position[1], position[2]].map(f64::from);
                let delta = std::array::from_fn::<_, 3, _>(|axis| live[axis] - center[axis]);
                let d = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt()
                    / radius;
                if d >= 1.0 {
                    continue;
                }
                let q = d * d;
                let weight = (3.0 * q * q - 4.0 * q * d + 1.0) * amplitude * influence;
                for axis in 0..3 {
                    let candidate = (live[axis] + direction[axis] * weight) as f32;
                    if !candidate.is_finite() || candidate.abs() > 1_000_000.0 {
                        return Err(
                            "Resident Brush candidate exceeds finite coordinate bounds".into()
                        );
                    }
                    position[axis] = candidate;
                }
            }
        }
        Ok(rows)
    }
    /// Scalar check for a GPU-picked ordinary Brush direction on the current
    /// immutable capture. The GPU keeps this value resident during live dabs.
    pub fn reference_picked_brush_direction(
        &self,
        picked_triangle: u32,
        center: [f32; 3],
    ) -> Result<[f32; 3], String> {
        if center
            .iter()
            .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
        {
            return Err("Picked Brush center exceeds coordinate bounds".into());
        }
        let headers = self.normal_topology.headers();
        let face = headers
            .partition_point(|header| (header >> 1) <= picked_triangle)
            .checked_sub(1)
            .ok_or("Picked Brush triangle has no original face")?;
        let header = headers[face];
        let first = (header >> 1) as usize;
        let corners = 3 + (header & 1) as usize;
        if picked_triangle as usize >= first + corners - 2 {
            return Err("Picked Brush triangle exceeds original face".into());
        }
        let t = self.triangles[first];
        let ids = [
            t[0],
            t[1],
            t[2],
            if corners == 4 {
                self.triangles[first + 1][2]
            } else {
                t[0]
            },
        ];
        let mut inverse = [0.0_f64; 4];
        for corner in 0..corners {
            let vertex = ids[corner] as usize;
            let position = self.positions[vertex];
            let delta = std::array::from_fn::<_, 3, _>(|axis| {
                f64::from(position[axis]) - f64::from(center[axis])
            });
            let distance =
                ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt();
            if !distance.is_finite() || distance == 0.0 {
                return Err("Picked Brush inverse distance is undefined".into());
            }
            inverse[corner] = 1.0 / distance;
        }
        let sum: f64 = inverse.iter().sum();
        if !sum.is_finite() || sum <= 0.0 {
            return Err("Picked Brush inverse sum is undefined".into());
        }
        let mut direction = [0.0_f64; 3];
        for corner in 0..corners {
            let normal = self.normal_u[ids[corner] as usize];
            for axis in 0..3 {
                direction[axis] += f64::from(normal[axis]) * (inverse[corner] / sum);
            }
        }
        let length = ((direction[0] * direction[0] + direction[1] * direction[1])
            + direction[2] * direction[2])
            .sqrt();
        if !length.is_finite() || length == 0.0 {
            return Err("Picked Brush direction is undefined".into());
        }
        Ok(direction.map(|value| (value / length) as f32))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct PointInflateParameters {
    pub center: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
}

/// Experimental resident ordinary-Brush candidate with an already checked
/// picked direction. It uses live positions, the captured raw unprotected
/// influence, no alpha, and accumulation. Picking/first-touch policy remain
/// separate; this is not a complete Source Brush transaction.
#[derive(Clone, Copy, Debug)]
pub struct PointBrushParameters {
    pub center: [f32; 3],
    pub direction: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
}
impl PointBrushParameters {
    pub fn validate(self) -> Result<(), String> {
        PointInflateParameters {
            center: self.center,
            radius: self.radius,
            intensity: self.intensity,
        }
        .validate()?;
        if self
            .direction
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1.0)
        {
            return Err("Resident Brush direction is outside the finite unit domain".into());
        }
        let length_squared: f64 = self
            .direction
            .iter()
            .map(|&v| f64::from(v) * f64::from(v))
            .sum();
        if !(0.999..=1.001).contains(&length_squared) {
            return Err("Resident Brush direction must be unit length".into());
        }
        Ok(())
    }
    /// Center/radius and intensity/direction occupy the existing two vec4
    /// point-program uniforms. The separate count remains a u32 word.
    pub fn uniform(self) -> Result<[f32; 8], String> {
        self.validate()?;
        Ok([
            self.center[0],
            self.center[1],
            self.center[2],
            self.radius,
            self.intensity,
            self.direction[0],
            self.direction[1],
            self.direction[2],
        ])
    }
}
impl PointInflateParameters {
    pub fn validate(self) -> Result<(), String> {
        if self
            .center
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
            || !self.radius.is_finite()
            || self.radius <= 0.
            || self.radius > 2_000_000.
            || !self.intensity.is_finite()
            || !(-1.0..=1.0).contains(&self.intensity)
        {
            return Err("Resident point parameters exceed finite bounded domain".into());
        }
        Ok(())
    }
    /// Eight f32 words form two vec4 uniforms; vertex count is separate u32.
    pub fn uniform(self) -> Result<[f32; 8], String> {
        self.validate()?;
        Ok([
            self.center[0],
            self.center[1],
            self.center[2],
            self.radius,
            self.intensity,
            0.,
            0.,
            0.,
        ])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CornerUv, SurfaceTopology, TRIANGLE_CORNER};
    const SOURCE_TETRA: &[u8] =
        include_bytes!("../tests/fixtures/source-import-options/tetra-13-v3-le.sgl");
    const PROJECT_TRIANGLE: &[u8] =
        include_bytes!("../tests/fixtures/independent-unprotected-v7.obrush");
    fn mesh() -> Mesh {
        let mut m = Mesh::sphere(2);
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        m
    }
    fn parameters() -> PointInflateParameters {
        PointInflateParameters {
            center: [0., 0., 1.],
            radius: 0.5,
            intensity: 0.25,
        }
    }
    #[test]
    fn resident_brush_requires_bounded_unit_direction_and_keeps_zero_passes_exact() {
        let valid = PointBrushParameters {
            center: [0., 0., 1.],
            direction: [0., 0., 1.],
            radius: 0.5,
            intensity: 0.25,
        };
        assert_eq!(
            valid.uniform().unwrap(),
            [0., 0., 1., 0.5, 0.25, 0., 0., 1.]
        );
        for direction in [[0., 0., 0.], [0., 0., 0.9], [f32::NAN, 0., 1.]] {
            assert!(PointBrushParameters { direction, ..valid }
                .validate()
                .is_err());
        }
        let capture = ResidentPointInput::capture(&mesh()).unwrap();
        assert_eq!(
            capture.reference_brush_positions(valid, 0).unwrap(),
            capture.positions()
        );
        assert!(capture.reference_brush_positions(valid, 257).is_err());
        let moved = capture.reference_brush_positions(valid, 2).unwrap();
        assert!(moved.iter().zip(capture.positions()).any(|(a, b)| a != b));
    }
    fn position_bytes(rows: &[[f32; 4]]) -> Vec<u8> {
        rows.iter()
            .flat_map(|row| row.iter().flat_map(|word| word.to_le_bytes()))
            .collect()
    }
    fn sparse_bytes(rows: &[(u32, [f32; 4])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for &(index, point) in rows {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&position_bytes(&[point]));
        }
        bytes
    }
    #[test]
    fn binary_source_fingerprint_covers_every_serialized_mesh_channel() {
        let mut base = mesh();
        base.materials = Some(vec![[0.18, 0.08]; base.positions.len()]);
        base.surface = Some(SurfaceTopology {
            faces: base
                .triangles
                .iter()
                .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
                .collect(),
            uv: Some(CornerUv {
                coordinates: vec![[0., 0.], [1., 0.], [0., 1.]],
                face_indices: vec![[0, 1, 2, TRIANGLE_CORNER]; base.triangles.len()],
            }),
        });
        let original = mesh_digest(&base).unwrap();
        assert_eq!(
            original,
            [
                54, 97, 61, 75, 191, 97, 191, 75, 71, 154, 171, 89, 103, 187, 125, 31, 134, 48,
                247, 55, 91, 96, 34, 166, 161, 232, 231, 136, 254, 142, 70, 44
            ]
        );
        assert_eq!(mesh_digest(&base.clone()).unwrap(), original);
        let changes: [fn(&mut Mesh); 13] = [
            |m| m.positions[0].x += 0.125,
            |m| m.normals[0].y += 0.125,
            |m| m.triangles[0].swap(1, 2),
            |m| m.masks[0] = 0.25,
            |m| m.mask_convention = MaskConvention::ProtectedF32V1,
            |m| m.colors[0][0] = 0.25,
            |m| m.materials.as_mut().unwrap()[0][0] = 0.25,
            |m| m.materials = None,
            |m| m.surface.as_mut().unwrap().faces[0].swap(0, 1),
            |m| m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[0][0] = 0.25,
            |m| {
                m.surface
                    .as_mut()
                    .unwrap()
                    .uv
                    .as_mut()
                    .unwrap()
                    .face_indices[0]
                    .swap(0, 1)
            },
            |m| m.surface.as_mut().unwrap().uv = None,
            |m| m.surface = None,
        ];
        for change in changes {
            let mut changed = base.clone();
            change(&mut changed);
            assert_ne!(mesh_digest(&changed).unwrap(), original);
        }
    }
    #[test]
    fn sparse_and_full_resident_commits_have_identical_document_history() {
        let mut full = Document::new(mesh());
        let mut sparse = Document::new(mesh());
        let capture = ResidentDocumentInput::capture(&full).unwrap();
        let rows = capture
            .points()
            .reference_positions(parameters(), 1)
            .unwrap();
        let changed: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(index, row)| {
                row.map(f32::to_bits) != capture.points().positions()[*index].map(f32::to_bits)
            })
            .map(|(index, row)| (index as u32, *row))
            .collect();
        assert!(!changed.is_empty());
        assert!(changed.len() < rows.len());
        let compact = sparse_bytes(&changed);
        assert_eq!(compact.len(), changed.len() * 20);
        assert!(capture
            .commit_position_bytes(&mut full, &position_bytes(&rows))
            .unwrap());
        assert!(capture
            .commit_sparse_position_bytes(&mut sparse, &compact)
            .unwrap());
        assert_eq!(full.to_json().unwrap(), sparse.to_json().unwrap());
        assert!(full.undo() && sparse.undo());
        assert_eq!(full.to_json().unwrap(), sparse.to_json().unwrap());
        assert!(full.redo() && sparse.redo());
        assert_eq!(full.to_json().unwrap(), sparse.to_json().unwrap());
    }

    #[test]
    fn commit_only_capture_matches_full_capture_and_rejects_stale_mesh() {
        let mut complete = Document::new(mesh());
        let mut lightweight = Document::new(mesh());
        let full = ResidentDocumentInput::capture(&complete).unwrap();
        let commit = ResidentDocumentCapture::capture(&lightweight).unwrap();
        let rows = full.points().reference_positions(parameters(), 1).unwrap();
        let changed: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(index, row)| {
                row.map(f32::to_bits) != full.points().positions()[*index].map(f32::to_bits)
            })
            .map(|(index, row)| (index as u32, *row))
            .collect();
        let compact = sparse_bytes(&changed);
        assert!(full
            .commit_sparse_position_bytes(&mut complete, &compact)
            .unwrap());
        assert!(commit
            .commit_sparse_position_bytes(&mut lightweight, &compact)
            .unwrap());
        assert_eq!(complete.to_json().unwrap(), lightweight.to_json().unwrap());
        assert!(complete.undo() && lightweight.undo());
        assert_eq!(complete.to_json().unwrap(), lightweight.to_json().unwrap());
        assert!(complete.redo() && lightweight.redo());
        assert_eq!(complete.to_json().unwrap(), lightweight.to_json().unwrap());

        let mut stale = Document::new(mesh());
        let captured = ResidentDocumentCapture::capture(&stale).unwrap();
        stale.active_mesh_mut().unwrap().colors[0][0] += 0.01;
        let before = stale.to_json().unwrap();
        assert!(captured
            .commit_sparse_position_bytes(&mut stale, &compact)
            .unwrap_err()
            .contains("stale source mesh"));
        assert_eq!(stale.to_json().unwrap(), before);
    }

    #[test]
    fn resident_sparse_candidate_rechecks_changed_world_bounds_atomically() {
        let staged = crate::StagedSglScene::from_sgl_bytes(
            SOURCE_TETRA,
            &crate::SglStageSettings::default(),
        )
        .unwrap();
        let mut document = Document::empty();
        document.install_staged_sgl_scene(staged).unwrap();
        document
            .transform_active_subtool(&crate::ObjectTransform {
                translation: glam::Vec3::new(900_000., 0., 0.),
                ..Default::default()
            })
            .unwrap();
        let mesh = document.active_mesh_mut().unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        let capture = ResidentDocumentCapture::capture(&document).unwrap();
        let rows: Vec<_> = document
            .require_active_mesh()
            .unwrap()
            .positions
            .iter()
            .enumerate()
            .map(|(index, point)| (index as u32, [point.x + 200_000., point.y, point.z, 1.]))
            .collect();
        let before = document.to_json().unwrap();
        let error = capture
            .commit_sparse_position_bytes(&mut document, &sparse_bytes(&rows))
            .unwrap_err();
        assert!(error.contains("transformed point"), "{error}");
        assert_eq!(document.to_json().unwrap(), before);
    }
    #[test]
    fn successive_resident_normals_match_fresh_full_rebuild_and_public_edits_fall_back() {
        fn records(capture: &ResidentDocumentInput) -> Vec<u8> {
            let rows = capture
                .points()
                .reference_positions(parameters(), 1)
                .unwrap();
            let changed: Vec<_> = rows
                .iter()
                .enumerate()
                .filter(|(index, row)| {
                    row.map(f32::to_bits) != capture.points().positions()[*index].map(f32::to_bits)
                })
                .map(|(index, row)| (index as u32, *row))
                .collect();
            sparse_bytes(&changed)
        }
        let mut cached = Document::new(mesh());
        let first = ResidentDocumentInput::capture(&cached).unwrap();
        assert!(first
            .commit_sparse_position_bytes(&mut cached, &records(&first))
            .unwrap());
        assert!(cached
            .active_mesh()
            .unwrap()
            .resident_normals_match_witness());
        let mut fresh = Document::from_json(&cached.to_json().unwrap()).unwrap();
        assert!(!fresh
            .active_mesh()
            .unwrap()
            .resident_normals_match_witness());
        let second_cached = ResidentDocumentInput::capture(&cached).unwrap();
        let second_fresh = ResidentDocumentInput::capture(&fresh).unwrap();
        let bytes = records(&second_cached);
        assert_eq!(bytes, records(&second_fresh));
        let (cached_changed, cached_timing) = second_cached
            .commit_sparse_position_bytes_timed(&mut cached, &bytes)
            .unwrap();
        let (fresh_changed, fresh_timing) = second_fresh
            .commit_sparse_position_bytes_timed(&mut fresh, &bytes)
            .unwrap();
        assert!(cached_changed && fresh_changed);
        assert!(cached_timing.sparse_normals);
        assert!(!fresh_timing.sparse_normals);
        assert_eq!(cached.to_json().unwrap(), fresh.to_json().unwrap());

        let mut topology_edited = cached.active_mesh().unwrap().clone();
        topology_edited.triangles[0].swap(1, 2);
        assert!(!topology_edited.resident_normals_match_witness());
        let mut edited = cached;
        let normal = &mut edited.mesh.as_mut().unwrap().normals[0].x;
        *normal = f32::from_bits(normal.to_bits() ^ 0x8000_0000);
        assert!(!edited
            .active_mesh()
            .unwrap()
            .resident_normals_match_witness());
        let edited_capture = ResidentDocumentInput::capture(&edited).unwrap();
        let (_, edited_timing) = edited_capture
            .commit_sparse_position_bytes_timed(&mut edited, &records(&edited_capture))
            .unwrap();
        assert!(!edited_timing.sparse_normals);
    }
    #[test]
    fn sparse_resident_commit_rejects_bad_records_without_document_mutation() {
        let mut document = Document::new(mesh());
        let capture = ResidentDocumentInput::capture(&document).unwrap();
        let before = document.to_json().unwrap();
        let mut point = capture.points().positions()[0];
        point[0] += 0.001;
        let good = sparse_bytes(&[(0, point)]);
        let cases = [
            good[..19].to_vec(),
            sparse_bytes(&[(0, point), (0, point)]),
            sparse_bytes(&[(1, capture.points().positions()[1]), (0, point)]),
            sparse_bytes(&[(u32::MAX, point)]),
            sparse_bytes(&[(0, [f32::NAN, point[1], point[2], 1.])]),
            sparse_bytes(&[(0, [point[0], point[1], point[2], 0.])]),
        ];
        for bad in cases {
            assert!(capture
                .commit_sparse_position_bytes(&mut document, &bad)
                .is_err());
            assert_eq!(document.to_json().unwrap(), before);
        }
        document.active_mesh_mut().unwrap().masks[0] = 0.;
        let protected = ResidentDocumentInput::capture(&document).unwrap();
        let protected_before = document.to_json().unwrap();
        assert!(protected
            .commit_sparse_position_bytes(&mut document, &good)
            .unwrap_err()
            .contains("protected vertex"));
        assert_eq!(document.to_json().unwrap(), protected_before);
        document.active_mesh_mut().unwrap().colors[0][0] = 0.25;
        let stale_before = document.to_json().unwrap();
        assert!(protected
            .commit_sparse_position_bytes(&mut document, &good)
            .unwrap_err()
            .contains("stale source mesh"));
        assert_eq!(document.to_json().unwrap(), stale_before);
    }
    #[test]
    fn resident_result_commits_one_undoable_edit_and_preserves_redo_on_noop() {
        let mut document = Document::new(mesh());
        let before = document.to_json().unwrap();
        let input = ResidentDocumentInput::capture(&document).unwrap();
        let mut rows = input.points().positions().to_vec();
        let normal = input.points().normal_u()[0];
        for component in 0..3 {
            rows[0][component] += normal[component] * 0.001;
        }
        assert!(input
            .commit_position_bytes(&mut document, &position_bytes(&rows))
            .unwrap());
        assert_eq!(
            document.active_mesh().unwrap().positions[0].to_array(),
            [rows[0][0], rows[0][1], rows[0][2]]
        );
        assert!(document.can_undo());
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        let unchanged = ResidentDocumentInput::capture(&document).unwrap();
        assert!(!unchanged
            .commit_position_bytes(
                &mut document,
                &position_bytes(unchanged.points().positions())
            )
            .unwrap());
        assert!(document.can_redo());
        assert!(document.redo());
        assert_eq!(
            document.active_mesh().unwrap().positions[0].to_array(),
            [rows[0][0], rows[0][1], rows[0][2]]
        );
    }
    #[test]
    fn resident_result_rejects_stale_protected_invalid_and_open_stroke_without_mutation() {
        let mut document = Document::new(mesh());
        let input = ResidentDocumentInput::capture(&document).unwrap();
        let mut rows = input.points().positions().to_vec();
        rows[0][0] += 0.001;
        let mut output = position_bytes(&rows);
        let before = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &output[..15])
            .is_err());
        output[12..16].copy_from_slice(&0.0f32.to_le_bytes());
        assert!(input.commit_position_bytes(&mut document, &output).is_err());
        output[12..16].copy_from_slice(&1.0f32.to_le_bytes());
        output[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(input.commit_position_bytes(&mut document, &output).is_err());
        assert_eq!(document.to_json().unwrap(), before);
        document.begin_stroke();
        assert!(input
            .commit_position_bytes(&mut document, &position_bytes(input.points().positions()))
            .is_err());
        document.end_stroke();
        document.active_mesh_mut().unwrap().colors[0][0] = 0.25;
        let changed = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &position_bytes(input.points().positions()))
            .unwrap_err()
            .contains("stale source mesh"));
        assert_eq!(document.to_json().unwrap(), changed);
        document.active_mesh_mut().unwrap().masks[0] = 0.;
        let protected = ResidentDocumentInput::capture(&document).unwrap();
        let mut rows = protected.points().positions().to_vec();
        rows[0][0] += 0.001;
        let changed = document.to_json().unwrap();
        assert!(protected
            .commit_position_bytes(&mut document, &position_bytes(&rows))
            .unwrap_err()
            .contains("protected vertex"));
        assert_eq!(document.to_json().unwrap(), changed);
    }
    #[test]
    fn resident_result_rejects_folded_candidate_atomically() {
        let mut document = Document::new(mesh());
        let input = ResidentDocumentInput::capture(&document).unwrap();
        let mut rows = input.points().positions().to_vec();
        rows[47][0] += 2.;
        rows[47][1] -= 3.5;
        rows[47][2] += 5.;
        let before = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &position_bytes(&rows))
            .unwrap_err()
            .contains("surface fold guard"));
        assert_eq!(document.to_json().unwrap(), before);
    }
    #[test]
    fn resident_result_rejects_changed_selection_pose_and_retained_levels() {
        let mut document = Document::new(mesh());
        let input = ResidentDocumentInput::capture(&document).unwrap();
        let bytes = position_bytes(input.points().positions());
        let other = document.add_subtool(mesh(), "Other").unwrap();
        document.select_subtool(other).unwrap();
        let before = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &bytes)
            .unwrap_err()
            .contains("different active subtool"));
        assert_eq!(document.to_json().unwrap(), before);

        let mut document = Document::new(mesh());
        let input = ResidentDocumentInput::capture(&document).unwrap();
        document.add_subdivision_level().unwrap();
        let before = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &bytes)
            .unwrap_err()
            .contains("baked levels"));
        assert_eq!(document.to_json().unwrap(), before);

        let staged = crate::StagedSglScene::from_sgl_bytes(
            SOURCE_TETRA,
            &crate::SglStageSettings::default(),
        )
        .unwrap();
        let mut document = Document::empty();
        document.install_staged_sgl_scene(staged).unwrap();
        let input = ResidentDocumentInput::capture(&document).unwrap();
        let bytes = position_bytes(input.points().positions());
        document
            .transform_active_subtool(&crate::ObjectTransform {
                translation: glam::Vec3::X,
                ..Default::default()
            })
            .unwrap();
        let before = document.to_json().unwrap();
        assert!(input
            .commit_position_bytes(&mut document, &bytes)
            .unwrap_err()
            .contains("stale object pose"));
        assert_eq!(document.to_json().unwrap(), before);
    }
    #[test]
    fn resident_result_does_not_replace_a_redo_branch_after_undo() {
        let mut document = Document::new(mesh());
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        let mut rows = captured.points().positions().to_vec();
        let normal = captured.points().normal_u()[0];
        for component in 0..3 {
            rows[0][component] += normal[component] * 0.001;
        }
        let mut different = document.active_mesh().unwrap().snapshot();
        different.colors[0][0] = 0.25;
        document.replace_mesh(different).unwrap();
        assert!(document.undo());
        assert!(document.can_redo());
        let before = document.to_json().unwrap();
        assert!(captured
            .commit_position_bytes(&mut document, &position_bytes(&rows))
            .unwrap_err()
            .contains("redo branch"));
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.can_redo());
    }
    #[test]
    fn document_capture_is_immutable_and_reports_mask_conversion() {
        let mut document = Document::new(Mesh::sphere(2));
        let before = document.to_json().unwrap();
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        assert_eq!(captured.subtool_id(), 1);
        assert_eq!(
            captured.local_to_world(),
            &glam::Mat4::IDENTITY.to_cols_array()
        );
        assert_eq!(
            captured.source_mask_convention(),
            MaskConvention::ProtectedF32V1
        );
        assert_eq!(captured.mask_roundtrip_changed_words(), 0);
        assert!(captured.points().normal_u().iter().all(|row| row[3] == 1.));
        assert_eq!(document.to_json().unwrap(), before);
        document.mesh.as_mut().unwrap().positions[0].x += 0.125;
        assert_ne!(
            captured.points().positions()[0][0],
            document.mesh.as_ref().unwrap().positions[0].x
        );
        document.set_subtool_visible(1, false).unwrap();
        assert!(ResidentDocumentInput::capture(&document).is_err());
        assert!(ResidentDocumentInput::capture(&Document::empty()).is_err());
    }
    #[test]
    fn document_capture_preserves_imported_local_pose_and_mask_convention() {
        let staged = crate::StagedSglScene::from_sgl_bytes(
            SOURCE_TETRA,
            &crate::SglStageSettings::default(),
        )
        .unwrap();
        let expected_pose = staged.objects[0].affine.local_to_world;
        let mut document = Document::empty();
        document.install_staged_sgl_scene(staged).unwrap();
        let before = document.to_json().unwrap();
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        assert_eq!(
            captured.local_to_world().map(f32::to_bits),
            expected_pose.map(f32::to_bits)
        );
        assert_eq!(
            captured.points().positions().len(),
            document.active_mesh().unwrap().positions.len()
        );
        assert_eq!(before, document.to_json().unwrap());
        assert_eq!(
            captured.source_mask_convention(),
            MaskConvention::UnprotectedF32V1
        );
        assert!(ResidentPointInput::capture(document.active_mesh().unwrap()).is_ok());
        let mut world = glam::DMat4::IDENTITY;
        world.w_axis.x = 0.375;
        let local = captured.local_to_gl_clip(world.to_cols_array()).unwrap();
        let expected = world * glam::DMat4::from_cols_array(&expected_pose.map(f64::from));
        assert_eq!(
            local.map(f64::to_bits),
            expected.to_cols_array().map(f64::to_bits)
        );
        assert!(captured.local_to_gl_clip([f64::NAN; 16]).is_err());
        assert!(captured.local_to_gl_clip([0.; 16]).is_err());
        let world_ray = SceneRay64 {
            origin: glam::DVec3::new(262_144., -131_072., 4_101.),
            direction: -glam::DVec3::Z,
        };
        let converted = document_local_ray(expected_pose, world_ray, 10.).unwrap();
        let pose = glam::DMat4::from_cols_array(&expected_pose.map(f64::from));
        for distance in [0., 1., 5., 10.] {
            let local = converted.ray.origin
                + converted.ray.direction * distance * converted.world_to_local_distance_scale;
            let world = pose.transform_point3(local);
            let expected = world_ray.origin + world_ray.direction * distance;
            assert!((world - expected).length() < 1e-9);
        }
        assert!(document_local_ray(
            expected_pose,
            SceneRay64 {
                direction: -2. * glam::DVec3::Z,
                ..world_ray
            },
            10.
        )
        .is_err());
        assert!(document_local_ray([0.; 16], world_ray, 10.).is_err());
    }
    #[test]
    fn independently_authored_project_uses_normal_decoder_and_does_not_replace_document() {
        let from_bytes = ResidentDocumentInput::from_project_bytes(PROJECT_TRIANGLE).unwrap();
        let original = Document::from_json(std::str::from_utf8(PROJECT_TRIANGLE).unwrap()).unwrap();
        let from_document = ResidentDocumentInput::capture(&original).unwrap();
        assert_eq!(from_bytes.subtool_id(), from_document.subtool_id());
        assert_eq!(
            from_bytes.points().positions(),
            from_document.points().positions()
        );
        assert_eq!(
            from_bytes.points().triangles(),
            from_document.points().triangles()
        );
        assert_eq!(from_bytes.local_to_world(), from_document.local_to_world());
        assert!(ResidentDocumentInput::from_project_bytes(b"{").is_err());
        assert!(ResidentDocumentInput::from_project_bytes(&[0xff]).is_err());
    }
    #[test]
    fn document_capture_reports_non_roundtripping_protected_mask_words() {
        let mut document = Document::new(Mesh::sphere(2));
        let raw = (1..1000)
            .map(|i| i as f32 / 1000.)
            .find(|raw| (1.0_f32 - (1.0_f32 - raw)).to_bits() != raw.to_bits())
            .unwrap();
        document.mesh.as_mut().unwrap().masks[0] = raw;
        let captured = ResidentDocumentInput::capture(&document).unwrap();
        assert_eq!(captured.mask_roundtrip_changed_words(), 1);
        assert_eq!(
            document.mesh.as_ref().unwrap().masks[0].to_bits(),
            raw.to_bits()
        );
        assert_eq!(
            captured.points().normal_u()[0][3].to_bits(),
            (1.0_f32 - raw).to_bits()
        );
    }
    #[test]
    fn capture_owns_complete_rows_and_never_mutates_source() {
        let mut m = mesh();
        let before = serde_json::to_string(&m).unwrap();
        let input = ResidentPointInput::capture(&m).unwrap();
        assert_eq!(before, serde_json::to_string(&m).unwrap());
        assert_eq!(input.positions().len(), m.positions.len());
        assert_eq!(input.triangles(), m.triangles);
        assert_eq!(input.colors().len(), m.colors.len());
        m.positions[0].x = 123.;
        assert_ne!(input.positions()[0][0], m.positions[0].x);
        let after = input.reference_positions(parameters(), 32).unwrap();
        assert!(after.iter().zip(input.positions()).any(|(a, b)| a != b));
        assert_eq!(
            input.reference_positions(parameters(), 0).unwrap(),
            input.positions()
        );
    }
    #[test]
    fn invalid_public_channels_and_convention_are_rejected() {
        let original = mesh();
        for kind in 0..7 {
            let mut m = original.clone();
            match kind {
                0 => m.positions[0].x = f32::NAN,
                1 => m.normals.pop().map(|_| ()).unwrap(),
                2 => m.masks[0] = f32::INFINITY,
                3 => m.colors[0][0] = -1.,
                4 => m.triangles[0][0] = u32::MAX,
                5 => m.mask_convention = MaskConvention::ProtectedF32V1,
                _ => {
                    m.surface = Some(crate::SurfaceTopology {
                        faces: vec![],
                        uv: None,
                    })
                }
            };
            assert!(ResidentPointInput::capture(&m).is_err());
        }
        assert!(ResidentPointInput::storage_requirement(0, 1).is_err());
        assert!(ResidentPointInput::storage_requirement(MAX_VERTICES + 1, 1).is_err());
        assert!(ResidentPointInput::storage_requirement(1, crate::MAX_TRIANGLES + 1).is_err());
        assert_eq!(
            ResidentPointInput::storage_requirement(999002, 1998000).unwrap(),
            131868108
        );
    }
    #[test]
    fn finite_parameters_masks_and_noops_are_explicit() {
        let mut m = mesh();
        m.masks.fill(0.);
        let input = ResidentPointInput::capture(&m).unwrap();
        assert_eq!(
            input.reference_positions(parameters(), 32).unwrap(),
            input.positions()
        );
        assert!(input.reference_positions(parameters(), 257).is_err());
        for bad in [f32::NAN, f32::INFINITY, -1., 0., 2_000_001.] {
            assert!(PointInflateParameters {
                radius: bad,
                ..parameters()
            }
            .validate()
            .is_err());
        }
        for bad in [f32::NAN, f32::INFINITY, -1.01, 1.01] {
            assert!(PointInflateParameters {
                intensity: bad,
                ..parameters()
            }
            .validate()
            .is_err());
        }
        assert!(PointInflateParameters {
            center: [f32::NAN, 0., 0.],
            ..parameters()
        }
        .validate()
        .is_err());
    }
}
