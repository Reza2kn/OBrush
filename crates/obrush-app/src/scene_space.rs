//! Consumer coordinate adapters. Local editable geometry and world camera space
//! are explicit; legacy unposed objects keep the original f32 operations.
use glam::{DMat3, DMat4, DVec3, Vec3};
#[cfg(test)]
use obrush_core::SceneRay64;
use obrush_core::{Document, ObjectAffineF32V1, SglPickingPolicy};

pub(crate) fn active_pose(document: &Document) -> Option<&ObjectAffineF32V1> {
    document.active_object_metadata().map(|m| &m.affine)
}
pub(crate) fn world_point(document: &Document, point: Vec3) -> Result<DVec3, String> {
    match active_pose(document) {
        Some(pose) => pose.world_point(point),
        None if point.is_finite() => Ok(point.as_dvec3()),
        None => Err("Local point must be finite".into()),
    }
}
pub(crate) fn local_point(document: &Document, point: DVec3) -> Result<Vec3, String> {
    let local = match active_pose(document) {
        Some(pose) => pose.local_point(point)?,
        None => point,
    };
    let local = local.as_vec3();
    if !local.is_finite() {
        return Err("Local point exceeds finite brush range".into());
    }
    Ok(local)
}
pub(crate) fn local_vector(document: &Document, vector: Vec3) -> Result<Vec3, String> {
    let Some(pose) = active_pose(document) else {
        return Ok(vector);
    };
    pose.validate()?;
    let m = DMat4::from_cols_array(&pose.local_to_world.map(f64::from));
    let linear = DMat3::from_cols(
        m.x_axis.truncate(),
        m.y_axis.truncate(),
        m.z_axis.truncate(),
    );
    let local = (linear.inverse() * vector.as_dvec3()).as_vec3();
    if !local.is_finite() {
        return Err("Local brush vector exceeds finite range".into());
    }
    Ok(local)
}
/// The source selection/radius approximation uses its first matrix-row norm.
/// Sheared objects can have different actual world footprints; that distinction
/// remains visible rather than silently baking local geometry.
pub(crate) fn source_scale(document: &Document) -> Result<f64, String> {
    let scale = active_pose(document).map_or(1.0, |p| p.source_scale_squared().sqrt());
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Invalid source object scale".into());
    }
    Ok(scale)
}
pub(crate) fn local_radius(document: &Document, world_radius: f32) -> Result<f32, String> {
    if active_pose(document).is_none() {
        return Ok(world_radius);
    }
    let local = (f64::from(world_radius) / source_scale(document)?) as f32;
    if !local.is_finite() || local <= 1e-6 {
        return Err("Local brush radius exceeds supported range".into());
    }
    Ok(local)
}
#[cfg(test)]
pub(crate) fn local_ray(document: &Document, ray: SceneRay64) -> Result<SceneRay64, String> {
    ray.validate()?;
    let Some(pose) = active_pose(document) else {
        return Ok(ray);
    };
    let origin = pose.local_point(ray.origin)?;
    let m = DMat4::from_cols_array(&pose.local_to_world.map(f64::from));
    let linear = DMat3::from_cols(
        m.x_axis.truncate(),
        m.y_axis.truncate(),
        m.z_axis.truncate(),
    );
    let direction = (linear.inverse() * ray.direction).normalize();
    let local = SceneRay64 { origin, direction };
    local.validate()?;
    Ok(local)
}
pub(crate) fn picking_policy(document: &Document) -> Result<SglPickingPolicy, String> {
    document.preferred_scene_picking_policy()
}
