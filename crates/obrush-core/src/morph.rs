//! One exact, geometry-only surface target bound to an ordered subdivision mesh.
use crate::{Mesh, Vec3, MAX_TRIANGLES, MAX_VERTICES};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::sync::Arc;

pub const MAX_MORPH_STORAGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MorphTargetSummary {
    pub creation_level: usize,
    pub available: bool,
    pub vertices: usize,
    pub triangles: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct MorphTarget {
    pub(crate) creation_level: usize,
    pub(crate) positions: Arc<Vec<Vec3>>,
    triangles: Arc<Vec<[u32; 3]>>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum MorphScheme {
    PositionsF32V1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MorphProject {
    scheme: MorphScheme,
    creation_level: usize,
    #[serde(deserialize_with = "deserialize_positions")]
    positions: Vec<Vec3>,
    #[serde(deserialize_with = "deserialize_triangles")]
    triangles: Vec<[u32; 3]>,
}

#[derive(Serialize)]
pub(crate) struct MorphProjectRef<'a> {
    scheme: MorphScheme,
    creation_level: usize,
    positions: &'a [Vec3],
    triangles: &'a [[u32; 3]],
}

impl MorphTarget {
    pub(crate) fn transformed(&self, affine: &crate::transform::Affine) -> Result<Self, String> {
        self.validate()?;
        let candidate = Self {
            creation_level: self.creation_level,
            positions: Arc::new(affine.points(&self.positions, &self.triangles)?),
            triangles: self.triangles.clone(),
        };
        candidate.validate()?;
        Ok(candidate)
    }
    pub(crate) fn new(mesh: &Mesh, creation_level: usize) -> Result<Self, String> {
        validate_source(mesh)?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(mesh.positions.len())
            .map_err(|_| "Cannot allocate Morph Target positions")?;
        positions.extend_from_slice(&mesh.positions);
        let mut triangles = Vec::new();
        triangles
            .try_reserve_exact(mesh.triangles.len())
            .map_err(|_| "Cannot allocate Morph Target triangles")?;
        triangles.extend_from_slice(&mesh.triangles);
        let target = Self {
            creation_level,
            positions: Arc::new(positions),
            triangles: Arc::new(triangles),
        };
        target.validate()?;
        Ok(target)
    }

    pub(crate) fn from_project(project: MorphProject) -> Result<Self, String> {
        let _scheme = project.scheme;
        let target = Self {
            creation_level: project.creation_level,
            positions: Arc::new(project.positions),
            triangles: Arc::new(project.triangles),
        };
        target.validate()?;
        Ok(target)
    }

    pub(crate) fn project_ref(&self) -> MorphProjectRef<'_> {
        MorphProjectRef {
            scheme: MorphScheme::PositionsF32V1,
            creation_level: self.creation_level,
            positions: &self.positions,
            triangles: &self.triangles,
        }
    }

    pub(crate) fn summary(&self, available: bool) -> MorphTargetSummary {
        MorphTargetSummary {
            creation_level: self.creation_level,
            available,
            vertices: self.positions.len(),
            triangles: self.triangles.len(),
        }
    }

    pub(crate) fn topology_matches(&self, mesh: &Mesh) -> bool {
        self.positions.len() == mesh.positions.len() && *self.triangles == mesh.triangles
    }

    pub(crate) fn same_state(&self, other: &Self) -> bool {
        self.creation_level == other.creation_level
            && (Arc::ptr_eq(&self.positions, &other.positions)
                || positions_equal_bits(&self.positions, &other.positions))
            && (Arc::ptr_eq(&self.triangles, &other.triangles) || self.triangles == other.triangles)
    }

    pub(crate) fn matches_surface(&self, mesh: &Mesh, level: usize) -> bool {
        self.creation_level == level
            && self.topology_matches(mesh)
            && positions_equal_bits(&self.positions, &mesh.positions)
    }

    pub(crate) fn with_positions(&self, mesh: &Mesh) -> Result<Self, String> {
        if !self.topology_matches(mesh) {
            return Err("Morph Target vertex/triangle correspondence has changed".into());
        }
        Self::new(mesh, self.creation_level)
    }

    pub(crate) fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.positions.capacity() * std::mem::size_of::<Vec3>()
            + self.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
    }

    pub(crate) fn encoded_upper_bound(&self) -> usize {
        // Three 24-byte f32 scalars, commas/brackets/separator per point;
        // three ten-digit indices and delimiters per face. Metadata has slack.
        self.positions
            .len()
            .saturating_mul(77)
            .saturating_add(self.triangles.len().saturating_mul(35))
            .saturating_add(1024)
    }

    fn validate(&self) -> Result<(), String> {
        if self.creation_level >= crate::multires::MAX_SUBDIVISION_LEVELS {
            return Err("Morph Target creation level is out of range".into());
        }
        if self.positions.is_empty()
            || self.positions.len() > MAX_VERTICES
            || self.triangles.is_empty()
            || self.triangles.len() > MAX_TRIANGLES
            || self.approximate_bytes() > MAX_MORPH_STORAGE_BYTES
        {
            return Err("Morph Target exceeds its point, face or 64 MiB storage limit".into());
        }
        validate_geometry(&self.positions, &self.triangles)
    }
}

pub(crate) fn positions_equal_bits(a: &[Vec3], b: &[Vec3]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
}

pub(crate) fn validate_source(mesh: &Mesh) -> Result<(), String> {
    mesh.validate()?;
    validate_geometry(&mesh.positions, &mesh.triangles)
}

pub(crate) fn validate_geometry(positions: &[Vec3], triangles: &[[u32; 3]]) -> Result<(), String> {
    if positions
        .iter()
        .any(|p| !p.is_finite() || p.abs().max_element() > 1_000_000.0)
    {
        return Err("Morph Target contains nonfinite or excessive world coordinates".into());
    }
    for (face, &triangle) in triangles.iter().enumerate() {
        if triangle.iter().any(|&v| v as usize >= positions.len())
            || triangle[0] == triangle[1]
            || triangle[1] == triangle[2]
            || triangle[2] == triangle[0]
        {
            return Err(format!("Morph Target has invalid indices at face {face}"));
        }
        let [a, b, c] = triangle.map(|v| positions[v as usize].as_dvec3());
        let area = (b - a).cross(c - a).length_squared();
        if !area.is_finite() || area == 0.0 {
            return Err(format!(
                "Morph Target refuses collapsed geometry at face {face}"
            ));
        }
    }
    Ok(())
}

fn deserialize_positions<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec3>, D::Error> {
    deserialize_bounded(
        d,
        MAX_VERTICES,
        crate::decode_limits::Channel::MorphPositions,
    )
}

fn deserialize_triangles<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[u32; 3]>, D::Error> {
    deserialize_bounded(
        d,
        MAX_TRIANGLES,
        crate::decode_limits::Channel::MorphTriangles,
    )
}

/// Maximal array capacities total 36 MiB (1M vec3 points plus 2M face rows),
/// below the aggregate 64 MiB target budget. Limit each reserve during parsing;
/// never decode an unlimited Vec and reject it after allocation.
fn deserialize_bounded<'de, D, T>(
    d: D,
    maximum: usize,
    channel: crate::decode_limits::Channel,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct BoundedVisitor<T> {
        maximum: usize,
        channel: crate::decode_limits::Channel,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>> Visitor<'de> for BoundedVisitor<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "at most {} Morph Target entries", self.maximum)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut values = Vec::new();
            loop {
                if values.len() >= self.maximum {
                    if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(A::Error::custom(
                            "Morph Target array allocation limit exceeded",
                        ));
                    }
                    return Ok(values);
                }
                let Some(value) = seq.next_element::<T>()? else {
                    return Ok(values);
                };
                crate::decode_limits::element(self.channel).map_err(A::Error::custom)?;
                if values.len() == values.capacity() {
                    let desired = values
                        .capacity()
                        .saturating_mul(2)
                        .max(8)
                        .min(self.maximum)
                        .min(values.len() + 1 + crate::decode_limits::remaining(self.channel));
                    crate::decode_limits::capacity(
                        self.channel,
                        (desired - values.len()) * std::mem::size_of::<T>(),
                    )
                    .map_err(A::Error::custom)?;
                    values
                        .try_reserve_exact(desired - values.len())
                        .map_err(|_| A::Error::custom("Cannot allocate Morph Target field"))?;
                }
                values.push(value);
            }
        }
    }
    d.deserialize_seq(BoundedVisitor {
        maximum,
        channel,
        marker: std::marker::PhantomData,
    })
}
