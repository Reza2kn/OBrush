//! Affine scene picking and render descriptors. Dual coordinate spaces remain
//! explicit; this module does not invent shader assets or mutate geometry.
use crate::{
    Hit, Mesh, ObjectAffineF32V1, Ray, SceneObjectMetadata, SglPickingPolicy, SglRenderSettings,
    Vec3, MAX_TRIANGLES,
};
use glam::{DMat3, DMat4, DVec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRay64 {
    pub origin: DVec3,
    pub direction: DVec3,
}
impl From<Ray> for SceneRay64 {
    fn from(ray: Ray) -> Self {
        Self {
            origin: ray.origin.as_dvec3(),
            direction: ray.direction.as_dvec3(),
        }
    }
}
impl SceneRay64 {
    pub fn validate(&self) -> Result<(), String> {
        if !self.origin.is_finite()
            || self.origin.abs().max_element() > 1_000_000.0
            || !self.direction.is_finite()
            || self.direction.abs().max_element() > 1_000_000.0
            || self.direction == DVec3::ZERO
        {
            return Err(
                "Scene ray requires finite bounded world coordinates and a nonzero direction"
                    .into(),
            );
        }
        Ok(())
    }
    fn legacy_exact(self) -> Option<Ray> {
        let origin = self.origin.as_vec3();
        let direction = self.direction.as_vec3();
        (origin.as_dvec3() == self.origin && direction.as_dvec3() == self.direction)
            .then_some(Ray { origin, direction })
    }
}
#[derive(Clone, Debug)]
pub struct SceneRayHit64 {
    pub id: u32,
    pub triangle_index: usize,
    pub triangle: [u32; 3],
    pub barycentric: [f64; 3],
    pub local_position: DVec3,
    pub local_shading_normal: DVec3,
    pub local_distance: f64,
    pub world_position: DVec3,
    pub world_shading_normal: DVec3,
    pub world_oriented_face_normal: DVec3,
    pub world_distance: f64,
    pub source_selection_distance: f64,
    legacy_local_hit: Option<Hit>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SceneHitConversionReport {
    pub maximum_local_position_error: f64,
    pub maximum_barycentric_error: f64,
}
impl SceneRayHit64 {
    pub fn selection_distance(&self, policy: SglPickingPolicy) -> f64 {
        match policy {
            SglPickingPolicy::SourceScaleRowV1 => self.source_selection_distance,
            SglPickingPolicy::WorldDistanceV1 => self.world_distance,
        }
    }
    /// Existing brush kernels accept f32 local samples. Any f64→f32 rounding is
    /// reported; the world-space point is never substituted for the local one.
    pub fn local_brush_hit(&self) -> Result<(Hit, SceneHitConversionReport), String> {
        if let Some(hit) = self.legacy_local_hit {
            return Ok((
                hit,
                SceneHitConversionReport {
                    maximum_local_position_error: 0.0,
                    maximum_barycentric_error: 0.0,
                },
            ));
        }
        let position = self.local_position.as_vec3();
        let normal = self.local_shading_normal.as_vec3().normalize_or_zero();
        let barycentric = self.barycentric.map(|v| v as f32);
        let distance = self.local_distance as f32;
        if !position.is_finite()
            || !normal.is_finite()
            || normal == Vec3::ZERO
            || !distance.is_finite()
            || barycentric.iter().any(|v| !v.is_finite())
        {
            return Err("Picked local sample exceeds supported f32 brush range".into());
        }
        let position_error = (position.as_dvec3() - self.local_position)
            .abs()
            .max_element();
        let barycentric_error = self
            .barycentric
            .iter()
            .zip(barycentric)
            .map(|(a, b)| (*a - f64::from(b)).abs())
            .fold(0.0, f64::max);
        Ok((
            Hit {
                position,
                normal,
                distance,
                triangle: self.triangle,
                barycentric,
            },
            SceneHitConversionReport {
                maximum_local_position_error: position_error,
                maximum_barycentric_error: barycentric_error,
            },
        ))
    }
}
#[derive(Clone, Debug)]
pub struct ObjectRenderTransform {
    pub model: [f32; 16],
    pub inverse_transpose_normal: [f32; 9],
    pub reflected: bool,
    pub render: Option<SglRenderSettings>,
}
impl ObjectRenderTransform {
    pub fn for_object(metadata: Option<&SceneObjectMetadata>) -> Result<Self, String> {
        let Some(metadata) = metadata else {
            return Ok(Self {
                model: glam::Mat4::IDENTITY.to_cols_array(),
                inverse_transpose_normal: glam::Mat3::IDENTITY.to_cols_array(),
                reflected: false,
                render: None,
            });
        };
        metadata.affine.validate()?;
        if let Some(render) = &metadata.render {
            crate::sgl::validate_render(render)?;
        }
        let matrix = matrix(&metadata.affine);
        let normal = linear(matrix).inverse().transpose();
        let normal = normal.to_cols_array().map(|v| v as f32);
        if normal.iter().any(|v| !v.is_finite()) {
            return Err("Render normal matrix exceeds finite f32 range".into());
        }
        Ok(Self {
            model: metadata.affine.local_to_world,
            inverse_transpose_normal: normal,
            reflected: metadata.affine.determinant() < 0.0,
            render: metadata.render.clone(),
        })
    }
    /// Compose view and model in f64 before the GPU f32 boundary. Buffers keep
    /// their original local coordinates; large translations are removed first.
    pub fn model_view(&self, view: [f64; 16]) -> Result<([f32; 16], [f32; 9]), String> {
        if self.model.iter().any(|v| !v.is_finite())
            || self.model[3] != 0.0
            || self.model[7] != 0.0
            || self.model[11] != 0.0
            || self.model[15] != 1.0
        {
            return Err("Render model must be finite and affine".into());
        }
        let view = DMat4::from_cols_array(&view);
        if !view.is_finite()
            || view.x_axis.w != 0.0
            || view.y_axis.w != 0.0
            || view.z_axis.w != 0.0
            || view.w_axis.w != 1.0
        {
            return Err("Render view must be finite and affine; projection is separate".into());
        }
        let combined = view * DMat4::from_cols_array(&self.model.map(f64::from));
        let positions = combined.to_cols_array().map(|v| v as f32);
        let normals = linear(combined)
            .inverse()
            .transpose()
            .to_cols_array()
            .map(|v| v as f32);
        if positions.iter().chain(&normals).any(|v| !v.is_finite()) {
            return Err(
                "Model-view/normal transform is singular or exceeds finite f32 range".into(),
            );
        }
        Ok((positions, normals))
    }
}
fn matrix(affine: &ObjectAffineF32V1) -> DMat4 {
    DMat4::from_cols_array(&affine.local_to_world.map(f64::from))
}
fn linear(matrix: DMat4) -> DMat3 {
    DMat3::from_cols(
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    )
}

pub(crate) fn pick_object(
    id: u32,
    mesh: &Mesh,
    metadata: Option<&SceneObjectMetadata>,
    ray: SceneRay64,
) -> Result<Option<SceneRayHit64>, String> {
    ray.validate()?;
    mesh.validate()?;
    // The original authoritative indexed picker is the ordinary f32 route.
    // No duplicate-face inference or rewritten legacy triangle math is used.
    if metadata.is_none() {
        if let Some(legacy) = ray.legacy_exact() {
            return Ok(mesh.raycast_indexed(legacy).map(|(index, hit)| {
                let face = hit.triangle.map(|i| mesh.positions[i as usize].as_dvec3());
                SceneRayHit64 {
                    id,
                    triangle_index: index,
                    triangle: hit.triangle,
                    barycentric: hit.barycentric.map(f64::from),
                    local_position: hit.position.as_dvec3(),
                    local_shading_normal: hit.normal.as_dvec3(),
                    local_distance: f64::from(hit.distance),
                    world_position: hit.position.as_dvec3(),
                    world_shading_normal: hit.normal.as_dvec3(),
                    world_oriented_face_normal: (face[1] - face[0])
                        .cross(face[2] - face[0])
                        .normalize_or_zero(),
                    world_distance: f64::from(hit.distance),
                    source_selection_distance: f64::from(hit.distance),
                    legacy_local_hit: Some(hit),
                }
            }));
        }
    }
    let transform = match metadata {
        None => DMat4::IDENTITY,
        Some(metadata) => {
            metadata.affine.validate()?;
            matrix(&metadata.affine)
        }
    };
    let inverse = linear(transform).inverse();
    let normal_transform = inverse.transpose();
    let local_origin = inverse * (ray.origin - transform.w_axis.truncate());
    let local_direction = (inverse * ray.direction.normalize()).normalize();
    if !local_origin.is_finite() || !local_direction.is_finite() || local_direction == DVec3::ZERO {
        return Err("Inverse affine ray exceeds finite supported range".into());
    }
    if mesh.triangles.len() > MAX_TRIANGLES {
        return Err("Scene pick exceeds bounded triangle work".into());
    }
    let mut nearest: Option<SceneRayHit64> = None;
    for (index, &ids) in mesh.triangles.iter().enumerate() {
        let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
        let e1 = b - a;
        let e2 = c - a;
        let p = local_direction.cross(e2);
        let det = e1.dot(p);
        let tolerance = f64::EPSILON * e1.length() * e2.length();
        if !det.is_finite() || det.abs() <= tolerance {
            continue;
        }
        let inverse_det = det.recip();
        let offset = local_origin - a;
        let u = offset.dot(p) * inverse_det;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = offset.cross(e1);
        let v = local_direction.dot(q) * inverse_det;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let local_distance = e2.dot(q) * inverse_det;
        if !local_distance.is_finite()
            || local_distance < 0.0
            || nearest
                .as_ref()
                .is_some_and(|h| h.local_distance <= local_distance)
        {
            continue;
        }
        let barycentric = [1.0 - u - v, u, v];
        let local_position = local_origin + local_direction * local_distance;
        let mut local_normal = ids
            .iter()
            .zip(barycentric)
            .map(|(&id, w)| mesh.normals[id as usize].as_dvec3() * w)
            .sum::<DVec3>()
            .normalize_or_zero();
        if local_normal == DVec3::ZERO || !local_normal.is_finite() {
            local_normal = e1.cross(e2).normalize_or_zero();
        }
        let world_position = transform.transform_point3(local_position);
        let world_normal = (normal_transform * local_normal).normalize_or_zero();
        let world_face =
            ids.map(|i| transform.transform_point3(mesh.positions[i as usize].as_dvec3()));
        let world_face_normal = (world_face[1] - world_face[0])
            .cross(world_face[2] - world_face[0])
            .normalize_or_zero();
        let world_distance = (world_position - ray.origin).length();
        let source_scale = metadata.map_or(1.0, |m| m.affine.source_scale_squared().sqrt());
        let source_selection_distance = local_distance * source_scale;
        if !world_position.is_finite()
            || !world_normal.is_finite()
            || world_normal == DVec3::ZERO
            || !world_distance.is_finite()
            || !source_selection_distance.is_finite()
        {
            return Err("Picked affine sample is invalid".into());
        }
        nearest = Some(SceneRayHit64 {
            id,
            triangle_index: index,
            triangle: ids,
            barycentric,
            local_position,
            local_shading_normal: local_normal,
            local_distance,
            world_position,
            world_shading_normal: world_normal,
            world_oriented_face_normal: world_face_normal,
            world_distance,
            source_selection_distance,
            legacy_local_hit: None,
        });
    }
    Ok(nearest)
}
