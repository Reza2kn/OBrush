//! Validated SGL editing records. This is a staging boundary, not a Document
//! installation API or a .obrush decoder. An empty scene has no fabricated Mesh.
use crate::{
    CornerUv, MaskConvention, Mesh, SglByteOrder, SglCameraSettings, SglDisplaySettings, SglMesh,
    SglRenderSettings, SglScene, SurfaceTopology, Vec3, MAX_SUBTOOLS, MAX_TRIANGLES,
    MAX_UV_COORDINATES, MAX_VERTICES, TRIANGLE_CORNER,
};
use glam::{DMat3, DMat4, DVec3};
use serde::{Deserialize, Serialize, Serializer};
use std::sync::Arc;

pub const EDITABLE_SGL_REQUIRED_PROJECT_VERSION: u32 = 7;
pub const MAX_STAGED_SCENE_BYTES: usize = crate::resource_limits::MAX_SCENE_STORAGE_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SglPickingPolicy {
    /// Pinned first matrix-row norm, which can select a farther object under shear.
    SourceScaleRowV1,
    /// True world distance; explicitly distinct from the source approximation.
    WorldDistanceV1,
}

#[derive(Clone, Copy, Debug)]
pub struct SglStageSettings {
    /// None infers only nonzero source versions; version0 needs an explicit order.
    pub byte_order: Option<SglByteOrder>,
    pub picking_policy: SglPickingPolicy,
    /// Limit combines retained raw origin allocations and editable allocations.
    pub storage_limit: usize,
}
impl Default for SglStageSettings {
    fn default() -> Self {
        Self {
            byte_order: None,
            picking_policy: SglPickingPolicy::SourceScaleRowV1,
            storage_limit: MAX_STAGED_SCENE_BYTES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectAffineF32V1 {
    /// Local f32 points are retained; no baking into world-space f32 points.
    pub local_to_world: [f32; 16],
    /// Source init derives this from local bounds; raw encoded center is provenance.
    pub interaction_center: [f32; 3],
}
impl ObjectAffineF32V1 {
    pub fn from_local_points(matrix: [f32; 16], points: &[Vec3]) -> Result<Self, String> {
        if points.is_empty() || points.len() > MAX_VERTICES {
            return Err("Affine bounds require bounded nonempty local points".into());
        }
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for point in points {
            for (axis, value) in point.to_array().into_iter().enumerate() {
                if !value.is_finite() || value.abs() > 1_000_000.0 {
                    return Err("Affine local point exceeds coordinate limits".into());
                }
                // Source comparisons retain the first equal endpoint, rather
                // than implicitly canonicalizing signed-zero bounds.
                let value = f64::from(value);
                if value < low[axis] {
                    low[axis] = value;
                }
                if value > high[axis] {
                    high[axis] = value;
                }
            }
        }
        let affine = Self {
            local_to_world: matrix,
            interaction_center: std::array::from_fn(|i| ((low[i] + high[i]) * 0.5) as f32),
        };
        affine.validate()?;
        for &point in points {
            affine.world_point_validated(point)?;
        }
        Ok(affine)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self
            .local_to_world
            .iter()
            .chain(&self.interaction_center)
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return Err(
                "Editable affine coefficients/center exceed finite coordinate limits".into(),
            );
        }
        let m = &self.local_to_world;
        if m[3] != 0.0 || m[7] != 0.0 || m[11] != 0.0 || m[15] != 1.0 {
            return Err("Editable matrix must be affine".into());
        }
        let matrix = self.matrix_f64();
        let determinant = matrix.determinant();
        if !determinant.is_finite() || determinant == 0.0 {
            return Err("Editable affine is singular".into());
        }
        let inverse = matrix.inverse();
        if !inverse.is_finite()
            || inverse
                .to_cols_array()
                .iter()
                .any(|v| !(*v as f32).is_finite())
        {
            return Err(
                "Raw affine is not supported for editing: inverse exceeds finite f32 range".into(),
            );
        }
        Ok(())
    }
    fn matrix_f64(&self) -> DMat4 {
        DMat4::from_cols_array(&self.local_to_world.map(f64::from))
    }
    pub fn determinant(&self) -> f64 {
        self.matrix_f64().determinant()
    }
    pub fn source_scale_squared(&self) -> f64 {
        let m = self.local_to_world.map(f64::from);
        m[0] * m[0] + m[4] * m[4] + m[8] * m[8]
    }
    pub fn world_point(&self, point: Vec3) -> Result<DVec3, String> {
        self.validate()?;
        self.world_point_validated(point)
    }
    pub(crate) fn world_point_validated(&self, point: Vec3) -> Result<DVec3, String> {
        if !point.is_finite() {
            return Err("Affine point must be finite".into());
        }
        let point = self.matrix_f64().transform_point3(point.as_dvec3());
        if !point.is_finite() || point.abs().max_element() > 1_000_000.0 {
            return Err("Editable transformed point exceeds coordinate limits".into());
        }
        Ok(point)
    }
    /// f64 local-ray conversion; the result is not silently rounded into f32.
    pub fn local_point(&self, point: DVec3) -> Result<DVec3, String> {
        self.validate()?;
        if !point.is_finite() {
            return Err("World point must be finite".into());
        }
        // Remove the affine translation first rather than multiplying by an
        // already-rounded inverse translation column. This preserves exact
        // representable local points and avoids cancelling large offsets.
        let matrix = self.matrix_f64();
        let linear = DMat3::from_cols(
            matrix.x_axis.truncate(),
            matrix.y_axis.truncate(),
            matrix.z_axis.truncate(),
        );
        let point = linear.inverse() * (point - matrix.w_axis.truncate());
        if !point.is_finite() || point.abs().max_element() > 1_000_000.0 {
            return Err("Local inverse point exceeds supported bounds".into());
        }
        Ok(point)
    }
    /// Inverse-transpose shading normal. A reflection's oriented face normal
    /// has opposite sign; callers must not silently reverse original faces.
    pub fn shading_normal(&self, normal: Vec3) -> Result<DVec3, String> {
        self.validate()?;
        if !normal.is_finite() || normal == Vec3::ZERO {
            return Err("Shading normal must be finite and nonzero".into());
        }
        let normal = self
            .matrix_f64()
            .inverse()
            .transpose()
            .transform_vector3(normal.as_dvec3())
            .normalize();
        if !normal.is_finite() {
            return Err("Invalid transformed shading normal".into());
        }
        Ok(normal)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SglObjectOrigin {
    pub version: u32,
    pub byte_order: SglByteOrder,
    /// Each object owns/shares only its own original data, not the whole scene.
    #[serde(serialize_with = "serialize_origin")]
    pub raw: Arc<SglMesh>,
}
fn serialize_origin<S: Serializer>(raw: &Arc<SglMesh>, serializer: S) -> Result<S::Ok, S::Error> {
    raw.as_ref().serialize(serializer)
}

#[derive(Clone, Debug, Serialize)]
pub struct StagedSglObject {
    /// Source options made the current interaction center authoritative for export.
    #[serde(skip)]
    pub source_center_initialized: bool,
    pub id: u32,
    pub mesh: Mesh,
    pub affine: ObjectAffineF32V1,
    /// Optional legacy absence and all encoded appearance flags remain explicit.
    pub render: Option<SglRenderSettings>,
    pub origin: SglObjectOrigin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SglSceneOrigin {
    pub version: u32,
    pub byte_order: SglByteOrder,
    pub display: Option<SglDisplaySettings>,
    /// Raw vertical FOV/projection/mode/pivot; no pose or host clamping is invented.
    pub camera: Option<SglCameraSettings>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SglStageReport {
    pub objects: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub original_quads: usize,
    pub normalized_geometry_markers: usize,
    pub normalized_uv_markers: usize,
    pub implicit_rgb_objects: usize,
    pub implicit_material_objects: usize,
    pub source_mask_objects: usize,
    pub raw_editable_storage_bytes: usize,
    /// Stage retains appearance/header; none has been applied to a renderer/camera.
    pub appearance_not_applied_objects: usize,
    pub camera_not_applied: bool,
}

/// Not a public project DTO: coordinated Document optional-active migration,
/// bounded v7 deserialization and host installation remain separate work.
#[derive(Clone, Debug, Serialize)]
pub struct StagedSglScene {
    pub required_project_version: u32,
    pub objects: Vec<StagedSglObject>,
    pub active_id: Option<u32>,
    pub selected_ids: Vec<u32>,
    pub next_id: u32,
    pub picking_policy: SglPickingPolicy,
    pub origin: SglSceneOrigin,
    pub report: SglStageReport,
}

impl StagedSglScene {
    /// Decode and validate every source object before returning any editable scene.
    /// This API has no Document/camera side effects, including on second-mesh errors.
    pub fn from_sgl_bytes(source: &[u8], settings: &SglStageSettings) -> Result<Self, String> {
        if settings.storage_limit == 0 || settings.storage_limit > MAX_STAGED_SCENE_BYTES {
            return Err(
                "Staging storage limit is zero or exceeds the scene allocation budget".into(),
            );
        }
        let raw = match settings.byte_order {
            Some(order) => SglScene::from_bytes_with_order(source, order)?,
            None => SglScene::from_bytes(source)?,
        };
        raw.into_staged_editable(settings)
    }
    pub fn active_mesh(&self) -> Option<&Mesh> {
        let id = self.active_id?;
        self.objects.iter().find(|o| o.id == id).map(|o| &o.mesh)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.required_project_version != EDITABLE_SGL_REQUIRED_PROJECT_VERSION {
            return Err("Editable SGL semantics require project version7".into());
        }
        if self.objects.len() > MAX_SUBTOOLS {
            return Err("Staged scene exceeds32 objects".into());
        }
        SglScene {
            version: self.origin.version,
            byte_order: self.origin.byte_order,
            display: self.origin.display.clone(),
            camera: self.origin.camera.clone(),
            meshes: Vec::new(),
        }
        .validate()?;
        if self.storage_bytes()? > MAX_STAGED_SCENE_BYTES {
            return Err("Staged scene exceeds combined raw/editable storage limit".into());
        }
        if self.objects.is_empty() {
            if self.active_id.is_some() || !self.selected_ids.is_empty() {
                return Err("Empty scene must have no active/selected objects".into());
            }
        } else if self.active_id.is_none() || self.selected_ids.is_empty() {
            return Err("Nonempty staged scene requires active selection".into());
        }
        let mut ids = Vec::new();
        let mut vertices = 0usize;
        let mut triangles = 0usize;
        let mut uv = 0usize;
        for object in &self.objects {
            if object.id == 0 || ids.contains(&object.id) || object.id >= self.next_id {
                return Err("Invalid staged stable object ID".into());
            }
            ids.push(object.id);
            object.mesh.validate()?;
            object.affine.validate()?;
            if object.render.as_ref().is_some_and(|r| {
                r.shader > 12 || !r.opacity.is_finite() || !(0.0..=1.0).contains(&r.opacity)
            }) {
                return Err("Editable render metadata is invalid".into());
            }
            for &point in &object.mesh.positions {
                object.affine.world_point_validated(point)?;
            }
            for &ids in &object.mesh.triangles {
                let [a, b, c] = ids.map(|i| object.mesh.positions[i as usize].as_dvec3());
                let area = (b - a).cross(c - a).length_squared();
                if !area.is_finite() || area == 0.0 {
                    return Err("Collapsed editable staged geometry".into());
                }
            }
            if object.origin.version > 3 {
                return Err("Unknown staged SGL origin version".into());
            }
            crate::sgl::validate_mesh(&object.origin.raw, object.origin.version)?;
            vertices = add(vertices, object.mesh.positions.len())?;
            triangles = add(triangles, object.mesh.triangles.len())?;
            uv = add(uv, object.mesh.uv_coordinate_count())?;
            if vertices > MAX_VERTICES || triangles > MAX_TRIANGLES || uv > MAX_UV_COORDINATES {
                return Err("Staged scene aggregate geometry/UV limits exceeded".into());
            }
        }
        if self.next_id == 0 || self.active_id.is_some_and(|id| !ids.contains(&id)) {
            return Err("Staged active ID does not exist".into());
        }
        let mut selected = Vec::new();
        for &id in &self.selected_ids {
            if !ids.contains(&id) || selected.contains(&id) {
                return Err("Invalid staged selection".into());
            }
            selected.push(id);
        }
        if self.active_id.is_some_and(|id| !selected.contains(&id)) {
            return Err("Staged active ID must be selected".into());
        }
        if self.storage_bytes()? > MAX_STAGED_SCENE_BYTES {
            return Err("Staged scene exceeds combined raw/editable storage limit".into());
        }
        Ok(())
    }
    pub fn storage_bytes(&self) -> Result<usize, String> {
        let mut bytes = add(
            std::mem::size_of::<Self>(),
            mul(
                self.objects.capacity(),
                std::mem::size_of::<StagedSglObject>(),
            )?,
        )?;
        bytes = add(
            bytes,
            mul(self.selected_ids.capacity(), std::mem::size_of::<u32>())?,
        )?;
        let mut origins = Vec::new();
        for object in &self.objects {
            // Embedded Mesh header is counted in StagedSglObject already.
            bytes = add(
                bytes,
                object
                    .mesh
                    .approximate_bytes()
                    .saturating_sub(std::mem::size_of::<Mesh>()),
            )?;
            let identity = Arc::as_ptr(&object.origin.raw) as usize;
            if !origins.contains(&identity) {
                origins.push(identity);
                bytes = add(bytes, raw_storage(&object.origin.raw)?)?;
            }
        }
        Ok(bytes)
    }
    pub fn check_encoded_size(&self) -> Result<(), String> {
        struct Counter(usize);
        impl std::io::Write for Counter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0 = self
                    .0
                    .checked_add(data.len())
                    .ok_or_else(|| std::io::Error::other("Staged JSON size overflow"))?;
                if self.0 > crate::resource_limits::MAX_PROJECT_BYTES {
                    return Err(std::io::Error::other(
                        "Staged JSON exceeds the encoded project size limit",
                    ));
                }
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(&mut Counter(0), self)
            .map_err(|e| format!("Cannot encode bounded staging view: {e}"))
    }
}

impl SglScene {
    /// Owning conversion avoids a second retained copy of the full raw scene.
    /// On error nothing is installed; the caller still has its input file bytes.
    pub fn into_staged_editable(
        self,
        settings: &SglStageSettings,
    ) -> Result<StagedSglScene, String> {
        if settings.storage_limit == 0 || settings.storage_limit > MAX_STAGED_SCENE_BYTES {
            return Err(
                "Staging storage limit is zero or exceeds the scene allocation budget".into(),
            );
        }
        self.validate()?;
        if settings
            .byte_order
            .is_some_and(|order| order != self.byte_order)
        {
            return Err("Raw scene byte order differs from the explicit decode setting".into());
        }
        let mut estimated = add(
            std::mem::size_of::<StagedSglScene>(),
            mul(self.meshes.len(), std::mem::size_of::<StagedSglObject>())?,
        )?;
        estimated = add(estimated, mul(self.meshes.len(), 4)?)?;
        let mut report = SglStageReport {
            objects: self.meshes.len(),
            vertices: 0,
            triangles: 0,
            original_quads: 0,
            normalized_geometry_markers: 0,
            normalized_uv_markers: 0,
            implicit_rgb_objects: 0,
            implicit_material_objects: 0,
            source_mask_objects: 0,
            raw_editable_storage_bytes: 0,
            appearance_not_applied_objects: 0,
            camera_not_applied: self.camera.is_some(),
        };
        // Validate all usable affines and all allocation estimates before allocating
        // any new editable channels; a hostile later mesh cannot partially install.
        for (index, raw) in self.meshes.iter().enumerate() {
            let count = raw.positions.len();
            let mut triangles = 0usize;
            for (face_id, face) in raw.faces.iter().enumerate() {
                if raw.face_arity(face_id, self.version)? == 4 {
                    triangles = add(triangles, 2)?;
                    report.original_quads += 1;
                } else {
                    triangles = add(triangles, 1)?;
                    if face[3] != TRIANGLE_CORNER {
                        report.normalized_geometry_markers += 1;
                    }
                }
            }
            if let Some(uv) = &raw.uv {
                for face in &uv.face_indices {
                    if self.version < 3 && (face[3] as i32) < 0 && face[3] != TRIANGLE_CORNER {
                        report.normalized_uv_markers += 1;
                    }
                }
            }
            for (face_id, face) in raw.faces.iter().enumerate() {
                let arity = raw.face_arity(face_id, self.version)?;
                for tri in [[face[0], face[1], face[2]], [face[0], face[2], face[3]]]
                    .into_iter()
                    .take(if arity == 4 { 2 } else { 1 })
                {
                    let [a, b, c] =
                        tri.map(|v| DVec3::from_array(raw.positions[v as usize].map(f64::from)));
                    let area = (b - a).cross(c - a).length_squared();
                    if !area.is_finite() || area == 0.0 {
                        return Err(format!(
                            "SGL mesh{index} face{face_id}: collapsed geometry is unsupported for editable staging"
                        ));
                    }
                }
            }
            let affine = ObjectAffineF32V1 {
                local_to_world: raw.matrix,
                interaction_center: [0.0; 3],
            };
            affine
                .validate()
                .map_err(|e| format!("SGL mesh{index}: {e}"))?;
            estimated = add(estimated, raw_storage(raw)?)?;
            let mut editable = add(mul(count, 40)?, mul(triangles, 12)?)?;
            editable = add(editable, std::mem::size_of::<SurfaceTopology>())?;
            editable = add(editable, mul(raw.faces.len(), 16)?)?;
            if raw.materials.is_some() {
                editable = add(editable, mul(count, 8)?)?;
                report.source_mask_objects += 1;
            } else {
                report.implicit_material_objects += 1;
            }
            if raw.colors.is_none() {
                report.implicit_rgb_objects += 1;
            }
            if let Some(uv) = &raw.uv {
                editable = add(editable, std::mem::size_of::<CornerUv>())?;
                editable = add(editable, mul(uv.coordinates.len(), 8)?)?;
                editable = add(editable, mul(uv.face_indices.len(), 16)?)?;
            }
            estimated = add(estimated, editable)?;
            if estimated > settings.storage_limit {
                return Err(
                    "Combined raw/editable staged allocations exceed the selected storage limit"
                        .into(),
                );
            }
            report.vertices = add(report.vertices, count)?;
            report.triangles = add(report.triangles, triangles)?;
            report.appearance_not_applied_objects += usize::from(raw.render.is_some());
        }
        let origin = SglSceneOrigin {
            version: self.version,
            byte_order: self.byte_order,
            display: self.display,
            camera: self.camera,
        };
        let mut objects = reserved(self.meshes.len(), "staged objects")?;
        for (index, raw) in self.meshes.into_iter().enumerate() {
            let mut positions = reserved(raw.positions.len(), "local points")?;
            positions.extend(raw.positions.iter().copied().map(Vec3::from_array));
            let affine = ObjectAffineF32V1::from_local_points(raw.matrix, &positions)
                .map_err(|e| format!("SGL mesh{index}: {e}"))?;
            let triangle_count =
                raw.faces
                    .iter()
                    .enumerate()
                    .try_fold(0usize, |count, (face_id, _)| {
                        add(
                            count,
                            if raw.face_arity(face_id, origin.version)? == 4 {
                                2
                            } else {
                                1
                            },
                        )
                    })?;
            let mut triangles = reserved(triangle_count, "render triangles")?;
            let mut faces = reserved(raw.faces.len(), "original faces")?;
            for (face_id, &face) in raw.faces.iter().enumerate() {
                let arity = raw.face_arity(face_id, origin.version)?;
                let mut editable = face;
                triangles.push([face[0], face[1], face[2]]);
                if arity == 4 {
                    triangles.push([face[0], face[2], face[3]]);
                } else {
                    editable[3] = TRIANGLE_CORNER;
                }
                faces.push(editable);
            }
            let mut mesh = Mesh::new(positions, triangles);
            match &raw.colors {
                Some(colors) => mesh.colors.copy_from_slice(colors),
                None => mesh.colors.fill([1.0; 3]),
            }
            if let Some(materials) = &raw.materials {
                for (mask, material) in mesh.masks.iter_mut().zip(materials) {
                    *mask = material[2];
                }
                mesh.mask_convention = MaskConvention::UnprotectedF32V1;
                mesh.materials = Some(materials.iter().map(|m| [m[0], m[1]]).collect());
            }
            let uv = raw.uv.as_ref().map(|uv| CornerUv {
                coordinates: uv.coordinates.clone(),
                face_indices: uv
                    .face_indices
                    .iter()
                    .map(|&f| {
                        let mut editable = f;
                        if origin.version < 3 && (f[3] as i32) < 0 {
                            editable[3] = TRIANGLE_CORNER;
                        }
                        editable
                    })
                    .collect(),
            });
            mesh.surface = Some(SurfaceTopology { faces, uv });
            mesh.validate()?;
            for (face, &ids) in mesh.triangles.iter().enumerate() {
                let [a, b, c] = ids.map(|v| mesh.positions[v as usize].as_dvec3());
                if (b - a).cross(c - a).length_squared() == 0.0 {
                    return Err(format!(
                        "SGL mesh{index} face{face}: collapsed geometry is unsupported for editable staging"
                    ));
                }
            }
            objects.push(StagedSglObject {
                source_center_initialized: false,
                id: index as u32 + 1,
                mesh,
                affine,
                render: raw.render.clone(),
                origin: SglObjectOrigin {
                    version: origin.version,
                    byte_order: origin.byte_order,
                    raw: Arc::new(raw),
                },
            });
        }
        let active_id = objects.last().map(|o| o.id);
        let selected_ids = active_id.into_iter().collect();
        let mut scene = StagedSglScene {
            required_project_version: EDITABLE_SGL_REQUIRED_PROJECT_VERSION,
            next_id: objects.len() as u32 + 1,
            objects,
            active_id,
            selected_ids,
            picking_policy: settings.picking_policy,
            origin,
            report,
        };
        scene.report.raw_editable_storage_bytes = scene.storage_bytes()?;
        if scene.report.raw_editable_storage_bytes > settings.storage_limit {
            return Err(
                "Actual staged allocation capacities exceed the selected storage limit".into(),
            );
        }
        scene.validate()?;
        scene.check_encoded_size()?;
        Ok(scene)
    }
}
fn raw_storage(raw: &SglMesh) -> Result<usize, String> {
    let mut bytes = add(
        std::mem::size_of::<SglMesh>(),
        2 * std::mem::size_of::<usize>(),
    )?;
    bytes = add(bytes, mul(raw.positions.capacity(), 12)?)?;
    for channel in [&raw.colors, &raw.materials].into_iter().flatten() {
        bytes = add(bytes, mul(channel.capacity(), 12)?)?;
    }
    bytes = add(bytes, mul(raw.faces.capacity(), 16)?)?;
    if let Some(uv) = &raw.uv {
        bytes = add(bytes, mul(uv.coordinates.capacity(), 8)?)?;
        bytes = add(bytes, mul(uv.face_indices.capacity(), 16)?)?;
    }
    Ok(bytes)
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "Staged allocation count overflow".into())
}
fn mul(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "Staged allocation size overflow".into())
}
fn reserved<T>(count: usize, label: &str) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| format!("Cannot allocate {label}"))?;
    Ok(values)
}
