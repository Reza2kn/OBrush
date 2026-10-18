//! Baked whole-object world transforms, shared by native and WebAssembly hosts.
use crate::{Mesh, Vec3};
use glam::{DMat3, DVec3};

/// Degrees rotate around WORLD X, then WORLD Y, then WORLD Z: R=Rz*Ry*Rx
/// for column vectors. Points become pivot + scale*R*(p-pivot) + translation.
/// Scale is positive and uniform; deltas use scale*R without pivot/translation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectTransform {
    pub translation: Vec3,
    pub rotation_degrees: Vec3,
    pub scale: f32,
    pub pivot: Vec3,
}

impl Default for ObjectTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_degrees: Vec3::ZERO,
            scale: 1.0,
            pivot: Vec3::ZERO,
        }
    }
}

pub(crate) struct Affine {
    rotation: DMat3,
    scale: DVec3,
    pivot: DVec3,
    translation: DVec3,
    pub(crate) identity: bool,
    linear_identity: bool,
}

impl Affine {
    pub(crate) fn new(transform: &ObjectTransform) -> Result<Self, String> {
        if !transform.translation.is_finite()
            || transform.translation.abs().max_element() > 1_000_000.0
            || !transform.pivot.is_finite()
            || transform.pivot.abs().max_element() > 1_000_000.0
            || !transform.rotation_degrees.is_finite()
            || transform.rotation_degrees.abs().max_element() > 36_000.0
            || !transform.scale.is_finite()
            || !(0.001..=1000.0).contains(&transform.scale)
        {
            return Err("Transforms require finite translation/pivot within ±1M, rotation within ±36000 degrees and positive uniform scale 0.001–1000".into());
        }
        let angles = transform
            .rotation_degrees
            .as_dvec3()
            .to_array()
            .map(|angle| angle.rem_euclid(360.0).to_radians());
        let linear_identity = angles.iter().all(|&a| a == 0.0) && transform.scale == 1.0;
        let rotation = if angles.iter().all(|&a| a == 0.0) {
            DMat3::IDENTITY
        } else {
            DMat3::from_rotation_z(angles[2])
                * DMat3::from_rotation_y(angles[1])
                * DMat3::from_rotation_x(angles[0])
        };
        Ok(Self {
            rotation,
            scale: DVec3::splat(f64::from(transform.scale)),
            pivot: transform.pivot.as_dvec3(),
            translation: transform.translation.as_dvec3(),
            identity: linear_identity && transform.translation == Vec3::ZERO,
            linear_identity,
        })
    }
    pub(crate) fn axis_scale(scale: Vec3, pivot: Vec3) -> Result<Self, String> {
        if !scale.is_finite()
            || scale.min_element() < 0.001
            || scale.max_element() > 1000.0
            || !pivot.is_finite()
            || pivot.abs().max_element() > 1_000_000.0
        {
            return Err("Axis scale requires finite positive XYZ factors 0.001–1000 and a finite pivot within ±1M".into());
        }
        let scale = scale.as_dvec3();
        let identity = scale == DVec3::ONE;
        Ok(Self {
            rotation: DMat3::IDENTITY,
            scale,
            pivot: pivot.as_dvec3(),
            translation: DVec3::ZERO,
            identity,
            linear_identity: identity,
        })
    }
    pub(crate) fn point(&self, point: DVec3) -> DVec3 {
        if self.linear_identity {
            let mut result = point;
            for component in 0..3 {
                if self.translation[component] != 0.0 {
                    result[component] += self.translation[component];
                }
            }
            result
        } else {
            self.pivot + self.vector(point - self.pivot) + self.translation
        }
    }
    pub(crate) fn vector(&self, vector: DVec3) -> DVec3 {
        if self.linear_identity {
            vector
        } else {
            self.rotation * (vector * self.scale)
        }
    }
    pub(crate) fn is_translation_only(&self) -> bool {
        self.linear_identity
    }
    pub(crate) fn points(
        &self,
        source: &[Vec3],
        triangles: &[[u32; 3]],
    ) -> Result<Vec<Vec3>, String> {
        let mut points = Vec::new();
        points
            .try_reserve_exact(source.len())
            .map_err(|_| "Cannot allocate transformed points")?;
        for point in source {
            let next = self.point(point.as_dvec3());
            if !next.is_finite() || next.abs().max_element() > 1_000_000.0 {
                return Err("Transformed point exceeds finite world-coordinate bounds".into());
            }
            points.push(next.as_vec3());
        }
        self.validate_faces(source, &points, triangles)?;
        Ok(points)
    }
    pub(crate) fn mesh(&self, source: &Mesh) -> Result<Mesh, String> {
        crate::morph::validate_source(source)?;
        let mut candidate = source.snapshot();
        candidate.positions = self.points(&source.positions, &source.triangles)?;
        if !crate::morph::positions_equal_bits(&candidate.positions, &source.positions) {
            candidate.recompute_normals();
        }
        candidate.validate()?;
        Ok(candidate)
    }
    pub(crate) fn validate_faces(
        &self,
        source: &[Vec3],
        candidate: &[Vec3],
        triangles: &[[u32; 3]],
    ) -> Result<(), String> {
        if source.len() != candidate.len() {
            return Err("Transform changed vertex correspondence".into());
        }
        for (face, &ids) in triangles.iter().enumerate() {
            if ids.iter().any(|&id| id as usize >= source.len()) {
                return Err("Transform source has invalid triangle indices".into());
            }
            let [a, b, c] = ids.map(|i| source[i as usize].as_dvec3());
            let original = (b - a).cross(c - a);
            let [a, b, c] = ids.map(|i| candidate[i as usize].as_dvec3());
            let next = (b - a).cross(c - a);
            // Positive uniform transforms preserve winding. Compare to the
            // transformed expected normal, since ordinary rotations may turn
            // a healthy face by 180 degrees relative to its old world normal.
            let expected = self.rotation * (original / self.scale);
            if !original.is_finite()
                || original.length_squared() == 0.0
                || !next.is_finite()
                || next.length_squared() == 0.0
                || next.dot(expected) <= 0.0
            {
                return Err(format!(
                    "Transform refuses collapsed/flipped geometry at face {face}"
                ));
            }
        }
        Ok(())
    }
}
