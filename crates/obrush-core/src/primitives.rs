//! Independent bounded primitives and attribute-preserving mesh concatenation.
//! Coordinates are mesh/world units. No implicit scene normalization or Loop
//! subdivision is performed. Polygons are triangulated at construction.
use crate::{Mesh, SurfaceTopology, Vec3, MAX_SUBTOOLS, MAX_TRIANGLES, MAX_VERTICES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_PRIMITIVE_DIMENSION: f32 = 0.000_001;
pub const MAX_PRIMITIVE_DIMENSION: f32 = 1_000_000.0;

impl Mesh {
    /// Smooth the six-quad closed cube as an original-quad surface. Seven
    /// four-way levels give the pinned SculptGL Add Sphere face density;
    /// unlike an icosphere, its limit shape remains a rounded cube.
    pub fn sphere_quad_smooth(levels: u32) -> Result<Self, String> {
        if levels > 7 {
            return Err("Quad sphere supports at most seven refinement levels".into());
        }
        let mut mesh = Self::cube_quad_grid(0)?;
        for _ in 0..levels {
            mesh = mesh
                .subdivided_static(&crate::StaticSubdivisionSettings {
                    linear_edges: false,
                })?
                .mesh;
        }
        Ok(mesh)
    }

    /// Cube with a shared, uniformly sampled vertex grid and retained quad
    /// faces. Seven levels yield SculptGL's default linear cube density:
    /// 98,306 vertices, 98,304 quads, and 196,608 render triangles.
    /// Coordinates remain in OBrush's unit-half-extent convention; the host
    /// applies its own object placement and scene scale.
    pub fn cube_quad_grid(levels: u32) -> Result<Self, String> {
        if levels > 8 {
            return Err("Cube grid supports at most eight refinement levels".into());
        }
        let segments = 1_usize << levels;
        let faces_count = 6 * segments * segments;
        let triangles_count = faces_count * 2;
        let vertices_count = 6 * segments * segments + 2;
        if triangles_count > MAX_TRIANGLES || vertices_count > MAX_VERTICES {
            return Err("Cube grid exceeds the mesh allocation limit".into());
        }

        // Integer coordinates weld the twelve shared edges and eight corners
        // without float comparisons. Each (u, v) frame has outward winding.
        let frames: [([i32; 3], [i32; 3], [i32; 3]); 6] = [
            ([segments as i32, 0, 0], [0, 1, 0], [0, 0, 1]),
            ([0, 0, 0], [0, 0, 1], [0, 1, 0]),
            ([0, segments as i32, 0], [0, 0, 1], [1, 0, 0]),
            ([0, 0, 0], [1, 0, 0], [0, 0, 1]),
            ([0, 0, segments as i32], [1, 0, 0], [0, 1, 0]),
            ([0, 0, 0], [0, 1, 0], [1, 0, 0]),
        ];
        let mut ids = HashMap::<[i32; 3], u32>::with_capacity(vertices_count);
        let mut positions = Vec::with_capacity(vertices_count);
        let mut faces = Vec::with_capacity(faces_count);
        let mut triangles = Vec::with_capacity(triangles_count);
        for (origin, u, v) in frames {
            let mut grid = Vec::with_capacity((segments + 1) * (segments + 1));
            for row in 0..=segments {
                for column in 0..=segments {
                    let key = std::array::from_fn(|axis| {
                        origin[axis] + u[axis] * column as i32 + v[axis] * row as i32
                    });
                    let id = *ids.entry(key).or_insert_with(|| {
                        let id = positions.len() as u32;
                        let scale = 2.0 / segments as f32;
                        positions.push(Vec3::new(
                            key[0] as f32 * scale - 1.0,
                            key[1] as f32 * scale - 1.0,
                            key[2] as f32 * scale - 1.0,
                        ));
                        id
                    });
                    grid.push(id);
                }
            }
            for row in 0..segments {
                for column in 0..segments {
                    let first = row * (segments + 1) + column;
                    let a = grid[first];
                    let b = grid[first + 1];
                    let c = grid[first + segments + 2];
                    let d = grid[first + segments + 1];
                    faces.push([a, b, c, d]);
                    triangles.push([a, b, c]);
                    triangles.push([a, c, d]);
                }
            }
        }
        debug_assert_eq!(positions.len(), vertices_count);
        let mut mesh = Self::new(positions, triangles);
        mesh.surface = Some(SurfaceTopology { faces, uv: None });
        mesh.validate()?;
        Ok(mesh)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CylinderSettings {
    pub radius_top: f32,
    pub radius_bottom: f32,
    pub height: f32,
    pub radial_segments: u32,
    pub height_segments: u32,
    pub cap_top: bool,
    pub cap_bottom: bool,
}
impl Default for CylinderSettings {
    fn default() -> Self {
        Self {
            radius_top: 0.5,
            radius_bottom: 0.5,
            height: 2.0,
            radial_segments: 64,
            height_segments: 64,
            cap_top: true,
            cap_bottom: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TorusSettings {
    pub major_radius: f32,
    pub tube_radius: f32,
    pub arc: f64,
    pub radial_segments: u32,
    pub tubular_segments: u32,
    /// Partial arcs have two boundary loops when false. Full tori have no ends.
    pub cap_ends: bool,
}
impl Default for TorusSettings {
    fn default() -> Self {
        Self {
            major_radius: 0.5,
            tube_radius: 0.1,
            arc: std::f64::consts::TAU,
            radial_segments: 32,
            tubular_segments: 128,
            cap_ends: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtoolMergePolicy {
    /// Reject retained levels, layer stacks and Morph Targets before merging.
    RequirePlain,
    /// Bake displayed surfaces; consume selected levels/layers/targets. Undo
    /// restores the exact complete originals. This is not welding or union.
    FlattenDisplayed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtoolMergeSettings {
    pub ids: Vec<u32>,
    pub name: String,
    pub policy: SubtoolMergePolicy,
    /// Explicitly bake editable SGL poses to world coordinates and discard
    /// consumed source-object provenance. Undo retains the complete originals.
    #[serde(default)]
    pub allow_source_pose_loss: bool,
    /// Permit f32 complement rounding when selected masks use different
    /// conventions. The report counts changed source words on roundtrip.
    #[serde(default)]
    pub allow_mask_precision_loss: bool,
    /// Explicitly detach selected corner UVs, as SculptGL's static merge does.
    /// Required when some selected parts have UVs and others do not.
    #[serde(default)]
    pub allow_uv_loss: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubtoolMergeReport {
    pub consumed_ids: Vec<u32>,
    pub merged_id: u32,
    pub vertices: usize,
    pub triangles: usize,
    pub flattened_subtools: Vec<u32>,
    pub discarded_retained_levels: usize,
    pub baked_layers: usize,
    pub discarded_morph_targets: usize,
    pub baked_pose_ids: Vec<u32>,
    pub discarded_source_origin_ids: Vec<u32>,
    pub converted_mask_ids: Vec<u32>,
    pub mask_roundtrip_changed_words: usize,
    pub discarded_uv_ids: Vec<u32>,
    pub policy: SubtoolMergePolicy,
}

impl Mesh {
    /// Closed or explicitly uncapped Y-axis cylinder/frustum. A zero endpoint
    /// radius becomes one shared cone apex, irrespective of that cap flag.
    pub fn cylinder(settings: &CylinderSettings) -> Result<Self, String> {
        dimension(settings.radius_top, true, "top radius")?;
        dimension(settings.radius_bottom, true, "bottom radius")?;
        dimension(settings.height, false, "height")?;
        if settings.radius_top == 0.0 && settings.radius_bottom == 0.0 {
            return Err("Cylinder requires a positive endpoint radius".into());
        }
        let radial = settings.radial_segments as usize;
        let height = settings.height_segments as usize;
        if radial < 3 || height < 1 {
            return Err("Cylinder requires at least three radial and one height segment".into());
        }
        let top_apex = settings.radius_top == 0.0;
        let bottom_apex = settings.radius_bottom == 0.0;
        let caps = usize::from(settings.cap_top && !top_apex)
            + usize::from(settings.cap_bottom && !bottom_apex);
        let rows = height
            .checked_add(1)
            .ok_or("Cylinder segment count overflow")?;
        let vertices = rows
            .checked_mul(radial)
            .and_then(|n| {
                n.checked_sub((usize::from(top_apex) + usize::from(bottom_apex)) * (radial - 1))
            })
            .and_then(|n| n.checked_add(caps))
            .ok_or("Cylinder vertex count overflow")?;
        let side_triangles = height
            .checked_mul(radial)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| {
                n.checked_sub((usize::from(top_apex) + usize::from(bottom_apex)) * radial)
            })
            .ok_or("Cylinder triangle count overflow")?;
        let triangles = side_triangles
            .checked_add(
                caps.checked_mul(radial)
                    .ok_or("Cylinder cap count overflow")?,
            )
            .ok_or("Cylinder triangle count overflow")?;
        counts(vertices, triangles)?;
        let mut positions = allocate(vertices)?;
        let mut faces = allocate(triangles)?;
        let mut rings = allocate(rows)?;
        for row in 0..=height {
            let t = row as f64 / height as f64;
            let radius =
                f64::from(settings.radius_top) * (1.0 - t) + f64::from(settings.radius_bottom) * t;
            let y = f64::from(settings.height) * (0.5 - t);
            let singular = (row == 0 && top_apex) || (row == height && bottom_apex);
            let start = positions.len() as u32;
            rings.push((start, singular));
            if singular {
                positions.push(Vec3::new(0.0, y as f32, 0.0));
            } else {
                for slice in 0..radial {
                    let theta = std::f64::consts::TAU * slice as f64 / radial as f64;
                    positions.push(Vec3::new(
                        (radius * theta.sin()) as f32,
                        y as f32,
                        (radius * theta.cos()) as f32,
                    ));
                }
            }
        }
        for row in 0..height {
            let (upper, upper_apex) = rings[row];
            let (lower, lower_apex) = rings[row + 1];
            for slice in 0..radial {
                let next = (slice + 1) % radial;
                if upper_apex {
                    faces.push([upper, lower + slice as u32, lower + next as u32]);
                } else if lower_apex {
                    faces.push([upper + slice as u32, lower, upper + next as u32]);
                } else {
                    faces.push([
                        upper + slice as u32,
                        lower + slice as u32,
                        lower + next as u32,
                    ]);
                    faces.push([
                        upper + slice as u32,
                        lower + next as u32,
                        upper + next as u32,
                    ]);
                }
            }
        }
        if settings.cap_top && !top_apex {
            let center = positions.len() as u32;
            positions.push(Vec3::Y * settings.height * 0.5);
            for slice in 0..radial {
                faces.push([
                    rings[0].0 + slice as u32,
                    rings[0].0 + ((slice + 1) % radial) as u32,
                    center,
                ]);
            }
        }
        if settings.cap_bottom && !bottom_apex {
            let center = positions.len() as u32;
            positions.push(-Vec3::Y * settings.height * 0.5);
            for slice in 0..radial {
                faces.push([
                    rings[height].0 + ((slice + 1) % radial) as u32,
                    rings[height].0 + slice as u32,
                    center,
                ]);
            }
        }
        finish(positions, faces)
    }

    /// Full genus-one torus, or a partial arc with explicit end-cap policy.
    /// Ring self-intersection/horn tori are rejected (`major_radius > tube_radius`).
    pub fn torus(settings: &TorusSettings) -> Result<Self, String> {
        dimension(settings.major_radius, false, "major radius")?;
        dimension(settings.tube_radius, false, "tube radius")?;
        if settings.major_radius <= settings.tube_radius {
            return Err(
                "Torus major radius must exceed tube radius to avoid self-intersection".into(),
            );
        }
        if f64::from(settings.major_radius) + f64::from(settings.tube_radius)
            > f64::from(MAX_PRIMITIVE_DIMENSION)
        {
            return Err("Torus outer radius exceeds the coordinate limit".into());
        }
        if !settings.arc.is_finite() || settings.arc <= 0.0 {
            return Err("Torus arc must be finite and positive".into());
        }
        if settings.arc > std::f64::consts::TAU {
            return Err("Torus arc must not exceed one full revolution".into());
        }
        let radial = settings.radial_segments as usize;
        let tubular = settings.tubular_segments as usize;
        if radial < 3 || tubular < 3 {
            return Err("Torus requires at least three radial and tubular segments".into());
        }
        let full = settings.arc == std::f64::consts::TAU;
        let capped = !full && settings.cap_ends;
        let vertices = radial
            .checked_mul(tubular)
            .and_then(|n| n.checked_add(if capped { 2 } else { 0 }))
            .ok_or("Torus vertex count overflow")?;
        let spans = if full { tubular } else { tubular - 1 };
        let triangles = radial
            .checked_mul(spans)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| n.checked_add(if capped { radial * 2 } else { 0 }))
            .ok_or("Torus triangle count overflow")?;
        counts(vertices, triangles)?;
        let mut positions = allocate(vertices)?;
        let mut faces = allocate(triangles)?;
        for ring in 0..tubular {
            let theta = settings.arc * ring as f64 / spans as f64;
            for slice in 0..radial {
                let phi = std::f64::consts::TAU * slice as f64 / radial as f64;
                let radius =
                    f64::from(settings.major_radius) + f64::from(settings.tube_radius) * phi.cos();
                positions.push(Vec3::new(
                    (radius * theta.cos()) as f32,
                    (f64::from(settings.tube_radius) * phi.sin()) as f32,
                    (radius * theta.sin()) as f32,
                ));
            }
        }
        // A nonzero triangle area alone cannot detect a tube that float32
        // storage flattens into a doubled strip. Each polygonal cross-section
        // must turn outward around its stored centerline, with positive area.
        for ring in 0..tubular {
            let theta = settings.arc * ring as f64 / spans as f64;
            let center = Vec3::new(
                (f64::from(settings.major_radius) * theta.cos()) as f32,
                0.0,
                (f64::from(settings.major_radius) * theta.sin()) as f32,
            )
            .as_dvec3();
            let direction = glam::DVec3::new(theta.cos(), 0.0, theta.sin());
            for slice in 0..radial {
                let a = positions[ring * radial + slice].as_dvec3() - center;
                let b = positions[ring * radial + (slice + 1) % radial].as_dvec3() - center;
                let cross = a.dot(direction) * b.y - a.y * b.dot(direction);
                if !cross.is_finite() || cross <= 0.0 {
                    return Err(
                        "Torus tube cross-section collapses or folds at float32 precision".into(),
                    );
                }
            }
        }
        for ring in 0..spans {
            let next_ring = (ring + 1) % tubular;
            for slice in 0..radial {
                let next = (slice + 1) % radial;
                let a = (ring * radial + slice) as u32;
                let b = (ring * radial + next) as u32;
                let c = (next_ring * radial + next) as u32;
                let d = (next_ring * radial + slice) as u32;
                faces.push([a, b, c]);
                faces.push([a, c, d]);
            }
        }
        if capped {
            let first = positions.len() as u32;
            positions.push(Vec3::new(settings.major_radius, 0.0, 0.0));
            let end = positions.len() as u32;
            positions.push(Vec3::new(
                (f64::from(settings.major_radius) * settings.arc.cos()) as f32,
                0.0,
                (f64::from(settings.major_radius) * settings.arc.sin()) as f32,
            ));
            let last = (tubular - 1) * radial;
            for slice in 0..radial {
                let next = (slice + 1) % radial;
                faces.push([first, next as u32, slice as u32]);
                faces.push([end, (last + slice) as u32, (last + next) as u32]);
            }
        }
        finish(positions, faces)
    }

    /// Concatenate disconnected surfaces in supplied order without welding or
    /// changing positions, normals, masks or colors. Each input is validated
    /// before exact aggregate counts are allocated. Coordinates are already world-space.
    pub fn concatenate(meshes: &[&Mesh]) -> Result<Self, String> {
        if meshes.is_empty() || meshes.len() > MAX_SUBTOOLS {
            return Err("Concatenation requires 1–32 meshes".into());
        }
        let mask_convention = meshes[0].mask_convention;
        if meshes
            .iter()
            .any(|mesh| mesh.mask_convention != mask_convention)
        {
            return Err("Concatenation requires matching mask conventions; convert explicitly with a loss policy first".into());
        }
        let mut vertex_count = 0_usize;
        let mut triangle_count = 0_usize;
        for mesh in meshes {
            mesh.validate()?;
            vertex_count = vertex_count
                .checked_add(mesh.positions.len())
                .ok_or("Concatenation vertex count overflow")?;
            triangle_count = triangle_count
                .checked_add(mesh.triangles.len())
                .ok_or("Concatenation triangle count overflow")?;
            counts(vertex_count, triangle_count)?;
        }
        let has_materials = meshes.iter().any(|m| m.materials.is_some());
        let has_surface = meshes.iter().any(|m| m.surface.is_some());
        let has_uv = meshes
            .iter()
            .any(|m| m.surface.as_ref().is_some_and(|s| s.uv.is_some()));
        if has_uv
            && meshes
                .iter()
                .any(|m| m.surface.as_ref().is_none_or(|s| s.uv.is_none()))
        {
            return Err("Concatenation cannot invent missing UVs; all selected meshes must provide UVs or explicitly detach them".into());
        }
        let surface_count: usize = if has_surface {
            meshes
                .iter()
                .map(|m| {
                    m.surface
                        .as_ref()
                        .map_or(m.triangles.len(), |s| s.faces.len())
                })
                .sum()
        } else {
            0
        };
        let uv_count: usize = meshes
            .iter()
            .filter_map(|m| m.surface.as_ref()?.uv.as_ref())
            .map(|uv| uv.coordinates.len())
            .sum();
        if uv_count > crate::MAX_UV_COORDINATES {
            return Err("Concatenation exceeds the aggregate UV coordinate limit".into());
        }
        let bytes = vertex_count * (40 + if has_materials { 8 } else { 0 })
            + triangle_count * 12
            + surface_count * 16
            + uv_count * 8
            + if has_uv { surface_count * 16 } else { 0 };
        if bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Concatenation attribute storage exceeds 128 MiB".into());
        }
        let mut materials = if has_materials {
            Some(allocate(vertex_count)?)
        } else {
            None
        };
        let mut surface = if has_surface {
            Some(crate::SurfaceTopology {
                faces: allocate(surface_count)?,
                uv: if has_uv {
                    Some(crate::CornerUv {
                        coordinates: allocate(uv_count)?,
                        face_indices: allocate(surface_count)?,
                    })
                } else {
                    None
                },
            })
        } else {
            None
        };
        let mut positions = allocate(vertex_count)?;
        let mut normals = allocate(vertex_count)?;
        let mut masks = allocate(vertex_count)?;
        let mut colors = allocate(vertex_count)?;
        let mut triangles = allocate(triangle_count)?;
        for mesh in meshes {
            let offset = positions.len() as u32;
            positions.extend_from_slice(&mesh.positions);
            normals.extend_from_slice(&mesh.normals);
            masks.extend_from_slice(&mesh.masks);
            colors.extend_from_slice(&mesh.colors);
            if let Some(output) = &mut materials {
                if let Some(source) = &mesh.materials {
                    output.extend_from_slice(source);
                } else {
                    output.resize(
                        output.len() + mesh.positions.len(),
                        crate::DEFAULT_VERTEX_MATERIAL,
                    );
                }
            }
            if let Some(output) = &mut surface {
                if let Some(source) = &mesh.surface {
                    output.faces.extend(source.faces.iter().map(|face| {
                        face.map(|i| {
                            if i == crate::TRIANGLE_CORNER {
                                i
                            } else {
                                i + offset
                            }
                        })
                    }));
                    if let (Some(target), Some(uv)) = (&mut output.uv, &source.uv) {
                        let uv_offset = target.coordinates.len() as u32;
                        target.coordinates.extend_from_slice(&uv.coordinates);
                        target
                            .face_indices
                            .extend(uv.face_indices.iter().map(|face| {
                                face.map(|i| {
                                    if i == crate::TRIANGLE_CORNER {
                                        i
                                    } else {
                                        i + uv_offset
                                    }
                                })
                            }));
                    }
                } else {
                    output.faces.extend(mesh.triangles.iter().map(|&[a, b, c]| {
                        [a + offset, b + offset, c + offset, crate::TRIANGLE_CORNER]
                    }));
                }
            }
            for triangle in &mesh.triangles {
                triangles.push(triangle.map(|index| index + offset));
            }
        }
        let mut mesh = Mesh::new(positions, triangles);
        mesh.normals = normals;
        mesh.masks = masks;
        mesh.mask_convention = mask_convention;
        mesh.colors = colors;
        mesh.materials = materials;
        mesh.surface = surface;
        mesh.validate()?;
        Ok(mesh)
    }
}
fn dimension(value: f32, allow_zero: bool, label: &str) -> Result<(), String> {
    if !value.is_finite()
        || (value == 0.0 && !allow_zero)
        || (value != 0.0 && !(MIN_PRIMITIVE_DIMENSION..=MAX_PRIMITIVE_DIMENSION).contains(&value))
    {
        Err(format!(
            "Primitive {label} must be finite within 0.000001–1,000,000{}",
            if allow_zero { " or zero" } else { "" }
        ))
    } else {
        Ok(())
    }
}
fn counts(vertices: usize, triangles: usize) -> Result<(), String> {
    if vertices == 0 || vertices > MAX_VERTICES || triangles == 0 || triangles > MAX_TRIANGLES {
        Err("Primitive or concatenation exceeds one-million vertices/two-million triangles".into())
    } else {
        Ok(())
    }
}
fn allocate<T>(count: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded primitive field")?;
    Ok(values)
}
fn finish(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Result<Mesh, String> {
    for triangle in &triangles {
        let a = positions[triangle[0] as usize].as_dvec3();
        let b = positions[triangle[1] as usize].as_dvec3();
        let c = positions[triangle[2] as usize].as_dvec3();
        let area = (b - a).cross(c - a).length_squared();
        if !area.is_finite() || area == 0.0 {
            return Err(
                "Primitive resolution or dimensions collapse a triangle at float32 precision"
                    .into(),
            );
        }
    }
    let mesh = Mesh::new(positions, triangles);
    mesh.validate()?;
    Ok(mesh)
}
