//! Independently implemented, bounded solid remeshing. This reconstructs a
//! triangle surface from a sampled solid; it is not quad retopology or a
//! persistent subdivision operation. Features narrower than a voxel can vanish.
use crate::{Mesh, SurfaceTopology, MAX_TRIANGLES, MAX_VERTICES};
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod mc_table;

/// `resolution` is the number of voxels across the longest input dimension.
/// Limits are additional operation budgets, never overrides of mesh limits.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemeshSettings {
    pub resolution: u32,
    pub max_cells: usize,
    pub max_work: u64,
    /// Paired shrink/expand passes for the legacy triangle extractor; one
    /// tangential pass when Marching Cubes is selected with value 1.
    pub smoothing_steps: u32,
    /// Put one vertex at each mixed-sign grid cell and connect crossing edges
    /// with quads. This produces the stepped Surface Nets Block silhouette.
    #[serde(default)]
    pub block_mode: bool,
    /// Place each Surface Nets vertex at the mean of the cell's crossing
    /// edges. The legacy triangle extractor remains the default when both
    /// this and `marching_cubes_mode` are false.
    #[serde(default)]
    pub surface_nets_mode: bool,
    /// Extract only crossings on cube edges, with consistent face decisions.
    /// This is separate from the older tetrahedral triangle extractor.
    #[serde(default)]
    pub marching_cubes_mode: bool,
    /// Explicitly discard source corner UVs when reconstructing a new voxel surface.
    #[serde(default)]
    pub allow_uv_loss: bool,
    /// Explicitly bake selected source poses into world-space geometry and
    /// discard their editable affine/source provenance in the fused result.
    #[serde(default)]
    pub allow_source_pose_loss: bool,
}

impl Default for RemeshSettings {
    fn default() -> Self {
        Self {
            resolution: 64,
            max_cells: 2_500_000,
            max_work: 150_000_000,
            smoothing_steps: 2,
            block_mode: false,
            surface_nets_mode: false,
            marching_cubes_mode: false,
            allow_uv_loss: false,
            allow_source_pose_loss: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemeshReport {
    pub resolution: u32,
    pub block_mode: bool,
    pub surface_nets_mode: bool,
    #[serde(default)]
    pub marching_cubes_mode: bool,
    pub grid_cells: [usize; 3],
    pub sampled_nodes: usize,
    pub voxel_size: f64,
    /// Small magnitude floor used by Block and legacy triangle remeshing;
    /// Smooth Surface Nets and Marching Cubes use the sampled distance.
    pub distance_floor_voxels: f64,
    pub input_vertices: usize,
    pub input_triangles: usize,
    pub output_vertices: usize,
    pub output_triangles: usize,
    pub input_components: usize,
    pub output_components: usize,
    pub reversed_input_components: usize,
    /// Component volumes add before remeshing; overlapping volume is counted
    /// more than once here, so this is not a union-volume reference.
    pub input_component_volume: f64,
    pub output_volume: f64,
    pub maximum_attribute_projection_distance: f64,
    #[serde(default)]
    pub discarded_uv_coordinates: usize,
    #[serde(default)]
    pub discarded_uv_faces: usize,
    pub work: u64,
}

/// One scene transaction that consumes selected solids and installs their
/// reconstructed union as a new active SubTool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectedRemeshReport {
    pub consumed_ids: Vec<u32>,
    pub created_id: u32,
    pub flattened_ids: Vec<u32>,
    #[serde(default)]
    pub baked_pose_ids: Vec<u32>,
    #[serde(default)]
    pub discarded_source_origin_ids: Vec<u32>,
    pub policy: crate::SubtoolMergePolicy,
    pub remesh: RemeshReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemeshPhase {
    Validate,
    Sample,
    Extract,
    Transfer,
}

#[derive(Clone, Copy, Debug)]
pub struct RemeshProgress {
    pub phase: RemeshPhase,
    pub completed: usize,
    pub total: usize,
}

const MAX_CELLS: usize = 2_500_000;
const MAX_WORK: u64 = 500_000_000;
const MAX_SOURCE_TRIANGLES: usize = 500_000;
struct VoxelAttributes {
    distances: Vec<f32>,
    unmasked: Vec<f32>,
    colors: Vec<[f32; 3]>,
    materials: Option<Vec<[f32; 2]>>,
}

#[derive(Default)]
struct VoxelOutputAttributes {
    unmasked: Vec<f32>,
    colors: Vec<[f32; 3]>,
    materials: Vec<[f32; 2]>,
}

const TETS: [[usize; 4]; 6] = [
    [0, 1, 3, 7],
    [0, 3, 2, 7],
    [0, 2, 6, 7],
    [0, 6, 4, 7],
    [0, 4, 5, 7],
    [0, 5, 1, 7],
];

impl Mesh {
    /// Nonmutating and atomic: errors, budget exhaustion and cancellation leave
    /// the original mesh untouched. The caller commits the returned mesh into
    /// document history only after this operation succeeds.
    pub fn remeshed(&self, settings: &RemeshSettings) -> Result<(Self, RemeshReport), String> {
        self.remeshed_with_progress(settings, |_| false)
    }

    /// A `true` callback result cancels at deterministic phase/row boundaries.
    /// The callback never exposes a partially reconstructed mesh.
    pub fn remeshed_with_progress(
        &self,
        settings: &RemeshSettings,
        cancel: impl FnMut(RemeshProgress) -> bool,
    ) -> Result<(Self, RemeshReport), String> {
        self.remeshed_internal(settings, None, None, None, cancel)
    }

    /// The concatenated mesh retains source face IDs, while voxel attributes
    /// follow each selected imported mesh's own optimized face stream in scene
    /// order. This prevents equal-distance overlap samples from swapping owners.
    pub(crate) fn remeshed_from_sources(
        &self,
        settings: &RemeshSettings,
        sources: &[&Mesh],
        source_face_optimization: &[bool],
        pre_rounding_bounds: Option<(DVec3, DVec3)>,
        cancel: impl FnMut(RemeshProgress) -> bool,
    ) -> Result<(Self, RemeshReport), String> {
        if source_face_optimization.len() != sources.len() {
            return Err("Selected remesh source face policies do not match selected meshes".into());
        }
        self.remeshed_internal(
            settings,
            Some(sources),
            Some(source_face_optimization),
            pre_rounding_bounds,
            cancel,
        )
    }

    fn remeshed_internal(
        &self,
        settings: &RemeshSettings,
        sources: Option<&[&Mesh]>,
        source_face_optimization: Option<&[bool]>,
        pre_rounding_bounds: Option<(DVec3, DVec3)>,
        mut cancel: impl FnMut(RemeshProgress) -> bool,
    ) -> Result<(Self, RemeshReport), String> {
        if !(16..=128).contains(&settings.resolution) {
            return Err("Remesh resolution must be in 16–128".into());
        }
        if settings.max_cells == 0 || settings.max_cells > MAX_CELLS {
            return Err(format!("Remesh cell budget must be in 1–{MAX_CELLS}"));
        }
        if settings.max_work == 0 || settings.max_work > MAX_WORK {
            return Err(format!("Remesh work budget must be in 1–{MAX_WORK}"));
        }
        if settings.smoothing_steps > 4 {
            return Err("Remesh smoothing must be in 0–4 paired passes".into());
        }
        if u8::from(settings.block_mode)
            + u8::from(settings.surface_nets_mode)
            + u8::from(settings.marching_cubes_mode)
            > 1
        {
            return Err("Remesh extraction modes are mutually exclusive".into());
        }
        checkpoint(&mut cancel, RemeshPhase::Validate, 0, 1)?;
        self.validate()?;
        let source_uv = self
            .surface
            .as_ref()
            .and_then(|surface| surface.uv.as_ref());
        if source_uv.is_some() && !settings.allow_uv_loss {
            return Err(
                "Voxel remesh would discard corner UVs; explicitly allow UV loss first".into(),
            );
        }
        if self.triangles.len() > MAX_SOURCE_TRIANGLES {
            return Err(format!(
                "Remesh supports at most {MAX_SOURCE_TRIANGLES} source triangles"
            ));
        }
        let topology = closed_topology(self)?;
        let triangles: Vec<_> = self
            .triangles
            .iter()
            .enumerate()
            .map(|(i, ids)| {
                let p = ids.map(|v| self.positions[v as usize].as_dvec3());
                let bounds = Bounds::points(&p);
                Triangle {
                    p,
                    ids: *ids,
                    bounds,
                    component_sign: topology.signs[i],
                }
            })
            .collect();
        let bounds = pre_rounding_bounds.map_or_else(
            || {
                Bounds::points(
                    &self
                        .positions
                        .iter()
                        .map(|p| p.as_dvec3())
                        .collect::<Vec<_>>(),
                )
            },
            |(min, max)| Bounds { min, max },
        );
        let extent = bounds.max - bounds.min;
        let voxel = extent.max_element() / settings.resolution as f64;
        if !voxel.is_finite() || voxel <= 0.0 {
            return Err("Remesh requires a solid with nonzero extent".into());
        }
        if voxel < f32::MIN_POSITIVE as f64 {
            return Err("Remesh voxel size is too small for finite-precision output".into());
        }
        if voxel <= bounds.min.abs().max(bounds.max.abs()).max_element() * f32::EPSILON as f64 * 8.0
        {
            return Err("Remesh voxel size is too small at these coordinates; move the mesh closer to the origin".into());
        }
        let cells = [extent.x, extent.y, extent.z].map(|v| {
            if settings.block_mode || settings.surface_nets_mode || settings.marching_cubes_mode {
                ((v / voxel) + 3.02).ceil() as usize - 1
            } else {
                (v / voxel).ceil() as usize + 4
            }
        });
        let cell_count = checked_product(cells)?;
        if cell_count > settings.max_cells {
            return Err(format!(
                "Remesh needs {cell_count} cells, exceeding the {} cell budget",
                settings.max_cells
            ));
        }
        let dims = cells.map(|v| v + 1);
        let count = checked_product(dims)?;
        let origin = bounds.min
            - DVec3::splat(
                voxel
                    * if settings.block_mode
                        || settings.surface_nets_mode
                        || settings.marching_cubes_mode
                    {
                        1.51
                    } else {
                        2.0
                    },
            );
        let grid = Grid {
            dims,
            origin,
            voxel,
        };
        let mut work = Work {
            used: 0,
            limit: settings.max_work,
        };
        let voxel_attributes =
            if settings.block_mode || settings.surface_nets_mode || settings.marching_cubes_mode {
                let sample_order = if let Some(sources) = sources {
                    let mut order = Vec::new();
                    order
                        .try_reserve_exact(self.triangles.len())
                        .map_err(|_| "Cannot allocate bounded selected remesh order")?;
                    let mut offset = 0;
                    for (index, source) in sources.iter().enumerate() {
                        let optimize = source_face_optimization
                            .and_then(|policies| policies.get(index))
                            .copied()
                            .unwrap_or(false);
                        let local = voxel_triangle_order(source, optimize, &mut work, &mut cancel)?;
                        order.extend(local.into_iter().map(|index| index + offset));
                        offset += source.triangles.len();
                    }
                    if offset != self.triangles.len() {
                        return Err(
                            "Selected remesh source face ranges do not match the combined mesh"
                                .into(),
                        );
                    }
                    order
                } else {
                    voxel_triangle_order(self, false, &mut work, &mut cancel)?
                };
                Some(voxelize_attributes(
                    &triangles,
                    &sample_order,
                    self,
                    &grid,
                    &mut work,
                    &mut cancel,
                )?)
            } else {
                None
            };
        let bvh = Bvh::build(triangles);
        let mut field = vec![0.0_f32; count];
        let mut crossings = Vec::new();
        // A fixed sub-voxel offset avoids rays lying on projected input edges.
        // Magnitudes are sampled at the exact node; only the winding ray moves.
        let jitter = DVec3::new(0.0, voxel * 0.0000137, voxel * 0.0000191);
        for z in 0..dims[2] {
            checkpoint(&mut cancel, RemeshPhase::Sample, z, dims[2])?;
            for y in 0..dims[1] {
                let ray = grid.point([0, y, z]) + jitter;
                crossings.clear();
                bvh.crossings(ray, &mut crossings, &mut work)?;
                crossings.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                let mut crossing = 0;
                let mut winding = 0_i32;
                for x in 0..dims[0] {
                    let p = grid.point([x, y, z]);
                    while crossing < crossings.len() && crossings[crossing].0 < p.x {
                        winding += crossings[crossing].1;
                        crossing += 1;
                    }
                    // Magnitudes farther than two voxels cannot affect an
                    // extracted crossing. Truncating avoids expensive exact
                    // nearest queries throughout a solid's empty/deep interior.
                    let distance_limit_squared = (voxel * 2.0).powi(2);
                    let nearest = bvh.nearest_limited(p, distance_limit_squared, &mut work)?;
                    let magnitude = nearest.distance_squared.sqrt();
                    let magnitude = if (settings.surface_nets_mode || settings.marching_cubes_mode)
                        && !settings.block_mode
                    {
                        magnitude
                    } else {
                        magnitude.max(voxel * 0.02)
                    };
                    let node = grid.index([x, y, z]);
                    field[node] = if winding > 0 {
                        -(magnitude as f32)
                    } else {
                        magnitude as f32
                    };
                }
                // A closed oriented surface must return to empty after all hits.
                if crossings.iter().map(|hit| hit.1).sum::<i32>() != 0 {
                    return Err("Remesh encountered ambiguous scanline winding; input may contain coincident surfaces".into());
                }
            }
        }
        let mut mc_attributes = VoxelOutputAttributes::default();
        let (mut output_positions, output_triangles, output_quads) = if settings.block_mode
            || settings.surface_nets_mode
        {
            extract_surface_nets(
                &grid,
                cells,
                &field,
                settings.block_mode,
                (voxel_attributes.as_ref().unwrap(), &mut mc_attributes),
                &mut work,
                &mut cancel,
            )?
        } else if settings.marching_cubes_mode {
            extract_marching_cubes(
                &grid,
                cells,
                &field,
                voxel_attributes.as_ref().unwrap(),
                &mut mc_attributes,
                &mut work,
                &mut cancel,
            )?
        } else {
            let mut output_positions = Vec::new();
            let mut output_triangles = Vec::new();
            let mut edges = HashMap::<(u32, u32), u32>::new();
            for z in 0..cells[2] {
                checkpoint(&mut cancel, RemeshPhase::Extract, z, cells[2])?;
                for y in 0..cells[1] {
                    for x in 0..cells[0] {
                        let corners = [
                            [x, y, z],
                            [x + 1, y, z],
                            [x, y + 1, z],
                            [x + 1, y + 1, z],
                            [x, y, z + 1],
                            [x + 1, y, z + 1],
                            [x, y + 1, z + 1],
                            [x + 1, y + 1, z + 1],
                        ];
                        let values = corners.map(|p| field[grid.index(p)]);
                        if values.iter().all(|v| *v > 0.0) || values.iter().all(|v| *v < 0.0) {
                            continue;
                        }
                        work.add(6)?;
                        for tet in TETS {
                            let mut inside = Vec::with_capacity(3);
                            let mut outside = Vec::with_capacity(3);
                            for corner in tet {
                                if values[corner] < 0.0 {
                                    inside.push(corner);
                                } else {
                                    outside.push(corner);
                                }
                            }
                            if inside.is_empty() || outside.is_empty() {
                                continue;
                            }
                            let toward_outside = outside
                                .iter()
                                .map(|&i| grid.point(corners[i]))
                                .sum::<DVec3>()
                                / outside.len() as f64
                                - inside
                                    .iter()
                                    .map(|&i| grid.point(corners[i]))
                                    .sum::<DVec3>()
                                    / inside.len() as f64;
                            let mut vertex = |a: usize, b: usize| -> Result<u32, String> {
                                let ia = grid.index(corners[a]) as u32;
                                let ib = grid.index(corners[b]) as u32;
                                let key = (ia.min(ib), ia.max(ib));
                                if let Some(&v) = edges.get(&key) {
                                    return Ok(v);
                                }
                                if output_positions.len() >= MAX_VERTICES {
                                    return Err("Remesh output exceeds the vertex limit".into());
                                }
                                let t = values[a] as f64 / (values[a] as f64 - values[b] as f64);
                                let p = grid
                                    .point(corners[a])
                                    .lerp(grid.point(corners[b]), t)
                                    .as_vec3();
                                let v = output_positions.len() as u32;
                                output_positions.push(p);
                                edges.insert(key, v);
                                Ok(v)
                            };
                            let faces = if inside.len() == 1 {
                                vec![[
                                    vertex(inside[0], outside[0])?,
                                    vertex(inside[0], outside[1])?,
                                    vertex(inside[0], outside[2])?,
                                ]]
                            } else if outside.len() == 1 {
                                vec![[
                                    vertex(inside[0], outside[0])?,
                                    vertex(inside[1], outside[0])?,
                                    vertex(inside[2], outside[0])?,
                                ]]
                            } else {
                                let [a, b, c, d] = [
                                    vertex(inside[0], outside[0])?,
                                    vertex(inside[0], outside[1])?,
                                    vertex(inside[1], outside[0])?,
                                    vertex(inside[1], outside[1])?,
                                ];
                                vec![[a, b, c], [b, d, c]]
                            };
                            for mut face in faces {
                                if output_triangles.len() >= MAX_TRIANGLES {
                                    return Err("Remesh output exceeds the triangle limit".into());
                                }
                                let [a, b, c] =
                                    face.map(|i| output_positions[i as usize].as_dvec3());
                                if (b - a).cross(c - a).dot(toward_outside) < 0.0 {
                                    face.swap(1, 2);
                                }
                                output_triangles.push(face);
                            }
                        }
                    }
                }
            }
            (output_positions, output_triangles, Vec::new())
        };
        if output_triangles.is_empty() {
            return Err(
                "Remesh found no solid at this resolution; increase resolution for thin features"
                    .into(),
            );
        }
        if settings.block_mode || settings.surface_nets_mode || settings.marching_cubes_mode {
            align_block_bounds(&mut output_positions, bounds, &mut work)?;
        }
        drop(field);
        let mut output = Mesh::new(output_positions, output_triangles);
        if !output_quads.is_empty() {
            output.surface = Some(SurfaceTopology {
                faces: output_quads,
                uv: None,
            });
        }
        output.mask_convention = self.mask_convention;
        if voxel_attributes.is_some() {
            output.masks = mc_attributes
                .unmasked
                .iter()
                .map(|&weight| {
                    if self.mask_convention.is_default() {
                        1.0 - weight
                    } else {
                        weight
                    }
                })
                .collect();
        }
        output.materials = self
            .materials
            .as_ref()
            .map(|_| vec![[0.0; 2]; output.positions.len()]);
        if voxel_attributes.is_some() {
            output.colors = mc_attributes.colors;
            if let Some(materials) = &mut output.materials {
                *materials = mc_attributes.materials;
            }
        }
        if settings.marching_cubes_mode {
            for _ in 0..settings.smoothing_steps {
                smooth_tangent(&mut output, &mut work)?;
            }
        } else if !settings.block_mode && !settings.surface_nets_mode {
            smooth(&mut output, settings.smoothing_steps, &mut work)?;
        }
        let mut max_projection_squared: f64 = 0.0;
        for i in 0..output.positions.len() {
            if i % 1024 == 0 {
                checkpoint(
                    &mut cancel,
                    RemeshPhase::Transfer,
                    i,
                    output.positions.len(),
                )?;
            }
            let nearest = bvh.nearest(output.positions[i].as_dvec3(), &mut work)?;
            max_projection_squared = max_projection_squared.max(nearest.distance_squared);
            if voxel_attributes.is_none() {
                let triangle = &bvh.triangles[nearest.triangle];
                let mut mask = 0.0_f64;
                let mut color = [0.0_f64; 3];
                let mut material = [0.0_f64; 2];
                for (corner, &weight) in nearest.barycentric.iter().enumerate() {
                    let source = triangle.ids[corner] as usize;
                    mask += self.masks[source] as f64 * weight;
                    if let Some(materials) = &self.materials {
                        for (channel, value) in material.iter_mut().enumerate() {
                            *value += materials[source][channel] as f64 * weight;
                        }
                    }
                    for (channel, value) in color.iter_mut().enumerate() {
                        *value += self.colors[source][channel] as f64 * weight;
                    }
                }
                output.masks[i] = mask.clamp(0.0, 1.0) as f32;
                output.colors[i] = color.map(|v| v.clamp(0.0, 1.0) as f32);
                if let Some(materials) = &mut output.materials {
                    materials[i] = material.map(|v| v.clamp(0.0, 1.0) as f32);
                }
            }
        }
        output.recompute_normals();
        output.validate()?;
        let output_topology = closed_topology(&output)?;
        if output_topology.reversed != 0 {
            return Err("Remesh generated an inverted solid; result was discarded".into());
        }
        let report = RemeshReport {
            resolution: settings.resolution,
            block_mode: settings.block_mode,
            surface_nets_mode: settings.surface_nets_mode,
            marching_cubes_mode: settings.marching_cubes_mode,
            grid_cells: cells,
            sampled_nodes: count,
            voxel_size: voxel,
            distance_floor_voxels: if (settings.surface_nets_mode || settings.marching_cubes_mode)
                && !settings.block_mode
            {
                0.0
            } else {
                0.02
            },
            input_vertices: self.positions.len(),
            input_triangles: self.triangles.len(),
            output_vertices: output.positions.len(),
            output_triangles: output.triangles.len(),
            input_components: topology.components,
            output_components: output_topology.components,
            reversed_input_components: topology.reversed,
            input_component_volume: topology.volume,
            output_volume: output_topology.volume,
            maximum_attribute_projection_distance: max_projection_squared.sqrt(),
            discarded_uv_coordinates: source_uv.map_or(0, |uv| uv.coordinates.len()),
            discarded_uv_faces: source_uv.map_or(0, |uv| uv.face_indices.len()),
            work: work.used,
        };
        Ok((output, report))
    }
}

fn align_block_bounds(
    positions: &mut [Vec3],
    source: Bounds,
    work: &mut Work,
) -> Result<(), String> {
    let mut output = Bounds {
        min: DVec3::splat(f64::INFINITY),
        max: DVec3::splat(f64::NEG_INFINITY),
    };
    for &point in positions.iter() {
        let point = point.as_dvec3();
        output.min = output.min.min(point);
        output.max = output.max.max(point);
    }
    let source_diagonal = (source.max - source.min).length();
    let output_diagonal = (output.max - output.min).length();
    if !output_diagonal.is_finite() || output_diagonal <= 0.0 {
        return Err("Block remesh output has no finite extent".into());
    }
    let scale = source_diagonal / output_diagonal;
    let source_center = (source.min + source.max) * 0.5;
    let output_center = (output.min + output.max) * 0.5;
    work.add(positions.len() as u64)?;
    for point in positions {
        *point = ((*point).as_dvec3() - output_center)
            .mul_add(DVec3::splat(scale), source_center)
            .as_vec3();
        if !point.is_finite() {
            return Err("Block remesh alignment exceeded finite coordinates".into());
        }
    }
    Ok(())
}

// Each intersection belongs to a physical grid edge, so neighboring cells
// share vertex IDs. The standard cube-case table specifies triangle diagonals.
fn extract_marching_cubes(
    grid: &Grid,
    cells: [usize; 3],
    field: &[f32],
    voxel_attributes: &VoxelAttributes,
    output_attributes: &mut VoxelOutputAttributes,
    work: &mut Work,
    cancel: &mut impl FnMut(RemeshProgress) -> bool,
) -> Result<ExtractedGeometry, String> {
    if voxel_attributes.unmasked.len() != field.len()
        || voxel_attributes.distances.len() != field.len()
        || voxel_attributes.colors.len() != field.len()
        || voxel_attributes
            .materials
            .as_ref()
            .is_some_and(|materials| materials.len() != field.len())
    {
        return Err("Voxel attribute field has the wrong length".into());
    }
    const EDGES: [[usize; 2]; 12] = [
        [0, 1],
        [1, 3],
        [3, 2],
        [2, 0],
        [4, 5],
        [5, 7],
        [7, 6],
        [6, 4],
        [0, 4],
        [1, 5],
        [3, 7],
        [2, 6],
    ];
    const CASE_CORNERS: [usize; 8] = [0, 1, 3, 2, 4, 5, 7, 6];
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    let mut cache = HashMap::<(usize, usize), u32>::new();
    for z in 0..cells[2] {
        checkpoint(cancel, RemeshPhase::Extract, z, cells[2])?;
        for y in 0..cells[1] {
            for x in 0..cells[0] {
                let corners = [
                    [x, y, z],
                    [x + 1, y, z],
                    [x, y + 1, z],
                    [x + 1, y + 1, z],
                    [x, y, z + 1],
                    [x + 1, y, z + 1],
                    [x, y + 1, z + 1],
                    [x + 1, y + 1, z + 1],
                ];
                let values = corners.map(|p| field[grid.index(p)]);
                let mut case = 0usize;
                for (bit, corner) in CASE_CORNERS.into_iter().enumerate() {
                    if values[corner] > 0.0 {
                        case |= 1 << bit;
                    }
                }
                if case == 0 || case == 255 {
                    continue;
                }
                work.add(12)?;
                let (cell_unmasked, cell_color, cell_material) =
                    sample_voxel_cell(grid, corners, voxel_attributes, work)?;
                let mut edge_ids = [u32::MAX; 12];
                for (edge, [a, b]) in EDGES.iter().copied().enumerate() {
                    if (values[a] < 0.0) == (values[b] < 0.0) {
                        continue;
                    }
                    let ia = grid.index(corners[a]);
                    let ib = grid.index(corners[b]);
                    let key = (ia.min(ib), ia.max(ib));
                    edge_ids[edge] = if let Some(&id) = cache.get(&key) {
                        id
                    } else {
                        if positions.len() >= MAX_VERTICES {
                            return Err("Remesh output exceeds the vertex limit".into());
                        }
                        let t = values[a] as f64 / (values[a] as f64 - values[b] as f64);
                        let point = grid
                            .point(corners[a])
                            .lerp(grid.point(corners[b]), t)
                            .as_vec3();
                        let id = positions.len() as u32;
                        positions.push(point);
                        output_attributes.unmasked.push(cell_unmasked);
                        output_attributes.colors.push(cell_color);
                        if voxel_attributes.materials.is_some() {
                            output_attributes.materials.push(cell_material);
                        }
                        cache.insert(key, id);
                        id
                    };
                }
                for face in mc_table::TRI_TABLE[case].chunks_exact(3) {
                    if triangles.len() >= MAX_TRIANGLES {
                        return Err("Remesh output exceeds the triangle limit".into());
                    }
                    let tri = [
                        edge_ids[face[0] as usize],
                        edge_ids[face[1] as usize],
                        edge_ids[face[2] as usize],
                    ];
                    if tri.contains(&u32::MAX) {
                        return Err("Marching Cubes case refers to an uncrossed edge".into());
                    }
                    triangles.push(tri);
                }
            }
        }
    }
    Ok((positions, triangles, Vec::new()))
}

fn smooth_tangent(mesh: &mut Mesh, work: &mut Work) -> Result<(), String> {
    work.add(mesh.triangles.len() as u64 * 3 + mesh.positions.len() as u64)?;
    let mut neighbors = vec![Vec::new(); mesh.positions.len()];
    let mut area_normals = vec![DVec3::ZERO; mesh.positions.len()];
    for &[a, b, c] in &mesh.triangles {
        let [pa, pb, pc] = [a, b, c].map(|id| mesh.positions[id as usize].as_dvec3());
        let area_normal = (pb - pa).cross(pc - pa);
        for id in [a, b, c] {
            area_normals[id as usize] += area_normal;
        }
        for (a, b) in [(a, b), (b, c), (c, a)] {
            neighbors[a as usize].push(b as usize);
            neighbors[b as usize].push(a as usize);
        }
    }
    for row in &mut neighbors {
        row.sort_unstable();
        row.dedup();
    }
    let original = mesh.positions.clone();
    for (i, position) in mesh.positions.iter_mut().enumerate() {
        if neighbors[i].len() <= 2 {
            continue;
        }
        let normal = area_normals[i].normalize_or_zero();
        if normal == DVec3::ZERO {
            continue;
        }
        let mean = neighbors[i]
            .iter()
            .map(|&j| original[j].as_dvec3())
            .sum::<DVec3>()
            / neighbors[i].len() as f64;
        let displacement = mean - original[i].as_dvec3();
        let influence = mesh.mask_convention.influence(mesh.masks[i]) as f64;
        *position = (original[i].as_dvec3()
            + (displacement - normal * displacement.dot(normal)) * influence)
            .as_vec3();
    }
    mesh.recompute_normals();
    Ok(())
}

type ExtractedGeometry = (Vec<Vec3>, Vec<[u32; 3]>, Vec<[u32; 4]>);

fn sample_voxel_cell(
    grid: &Grid,
    corners: [[usize; 3]; 8],
    voxel_attributes: &VoxelAttributes,
    work: &mut Work,
) -> Result<(f32, [f32; 3], [f32; 2]), String> {
    let mut unmasked_sum = 0.0_f64;
    let mut color_sum = [0.0_f64; 3];
    let mut material_sum = [0.0_f64; 2];
    let mut weight_sum = 0.0_f64;
    for &corner in &corners {
        let node = grid.index(corner);
        let magnitude = voxel_attributes.distances[node] as f64;
        if magnitude.is_finite() {
            let weight = (1.0 / magnitude).min(1e15);
            unmasked_sum += voxel_attributes.unmasked[node] as f64 * weight;
            for (channel, sum) in color_sum.iter_mut().enumerate() {
                *sum += voxel_attributes.colors[node][channel] as f64 * weight;
            }
            if let Some(materials) = &voxel_attributes.materials {
                for (channel, sum) in material_sum.iter_mut().enumerate() {
                    *sum += materials[node][channel] as f64 * weight;
                }
            }
            weight_sum += weight;
        }
    }
    work.add(8)?;
    let cell_unmasked = if weight_sum > 0.0 {
        (unmasked_sum / weight_sum).clamp(0.0, 1.0) as f32
    } else {
        1.0
    };
    let cell_color = color_sum.map(|sum| {
        if weight_sum > 0.0 {
            (sum / weight_sum).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    });
    let cell_material = material_sum.map(|sum| {
        if weight_sum > 0.0 {
            (sum / weight_sum).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    });
    Ok((cell_unmasked, cell_color, cell_material))
}

fn extract_surface_nets(
    grid: &Grid,
    cells: [usize; 3],
    field: &[f32],
    block: bool,
    attributes: (&VoxelAttributes, &mut VoxelOutputAttributes),
    work: &mut Work,
    cancel: &mut impl FnMut(RemeshProgress) -> bool,
) -> Result<ExtractedGeometry, String> {
    let (voxel_attributes, output_attributes) = attributes;
    // Each mixed-sign cell gets one vertex at its grid origin. Crossing grid
    // edges connect the four incident cells, preserving the blocky Surface
    // Nets topology rather than snapping marching-tetra vertices afterward.
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    let mut quads = Vec::new();
    let mut cell_ids = HashMap::<[usize; 3], u32>::new();
    for z in 0..cells[2] {
        checkpoint(cancel, RemeshPhase::Extract, z, cells[2])?;
        for y in 0..cells[1] {
            for x in 0..cells[0] {
                work.add(1)?;
                let cell = [x, y, z];
                let mut has_inside = false;
                let mut has_outside = false;
                for dz in 0..=1 {
                    for dy in 0..=1 {
                        for dx in 0..=1 {
                            let inside = field[grid.index([x + dx, y + dy, z + dz])] < 0.0;
                            has_inside |= inside;
                            has_outside |= !inside;
                        }
                    }
                }
                if has_inside && has_outside {
                    if positions.len() >= MAX_VERTICES {
                        return Err("Block remesh output exceeds the vertex limit".into());
                    }
                    let id = positions.len() as u32;
                    // SculptGL stores Surface Nets vertices in integer grid
                    // coordinates, then aligns the output bounds to the
                    // source. Keeping those integers exact also avoids a
                    // world-space rounding error before alignment.
                    positions.push(if block {
                        Vec3::new(x as f32, y as f32, z as f32)
                    } else {
                        smooth_surface_nets_cell(grid, field, cell)?
                    });
                    let corners = [
                        [x, y, z],
                        [x + 1, y, z],
                        [x, y + 1, z],
                        [x + 1, y + 1, z],
                        [x, y, z + 1],
                        [x + 1, y, z + 1],
                        [x, y + 1, z + 1],
                        [x + 1, y + 1, z + 1],
                    ];
                    let (unmasked, color, material) =
                        sample_voxel_cell(grid, corners, voxel_attributes, work)?;
                    output_attributes.unmasked.push(unmasked);
                    output_attributes.colors.push(color);
                    if voxel_attributes.materials.is_some() {
                        output_attributes.materials.push(material);
                    }
                    cell_ids.insert(cell, id);
                }
            }
        }
    }
    for z in 0..cells[2] {
        checkpoint(cancel, RemeshPhase::Extract, z, cells[2])?;
        for y in 0..cells[1] {
            for x in 0..cells[0] {
                let cell = [x, y, z];
                let Some(&current) = cell_ids.get(&cell) else {
                    continue;
                };
                for axis in 0..3 {
                    let u = (axis + 1) % 3;
                    let v = (axis + 2) % 3;
                    if cell[u] == 0 || cell[v] == 0 {
                        continue;
                    }
                    let mut next = cell;
                    next[axis] += 1;
                    let inside = field[grid.index(cell)] < 0.0;
                    if inside == (field[grid.index(next)] < 0.0) {
                        continue;
                    }
                    let mut minus_u = cell;
                    minus_u[u] -= 1;
                    let mut minus_v = cell;
                    minus_v[v] -= 1;
                    let mut minus_uv = minus_u;
                    minus_uv[v] -= 1;
                    let adjacent = |key| {
                        cell_ids
                            .get(&key)
                            .copied()
                            .ok_or("Block remesh crossing has a missing adjacent cell")
                    };
                    let (side_a, diagonal, side_b) =
                        (adjacent(minus_u)?, adjacent(minus_uv)?, adjacent(minus_v)?);
                    let face = if inside {
                        [current, side_a, diagonal, side_b]
                    } else {
                        [current, side_b, diagonal, side_a]
                    };
                    if triangles.len() > MAX_TRIANGLES.saturating_sub(2) {
                        return Err("Block remesh output exceeds the triangle limit".into());
                    }
                    triangles.push([face[0], face[1], face[2]]);
                    triangles.push([face[0], face[2], face[3]]);
                    quads
                        .try_reserve(1)
                        .map_err(|_| "Cannot allocate Surface Nets quad output")?;
                    quads.push(face);
                    work.add(2)?;
                }
            }
        }
    }
    Ok((positions, triangles, quads))
}

fn smooth_surface_nets_cell(grid: &Grid, field: &[f32], cell: [usize; 3]) -> Result<Vec3, String> {
    let mut offset = [0.0_f64; 3];
    let mut crossings = 0_u32;
    for corner in 0..8 {
        let a = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
        for axis in 0..3 {
            if a[axis] != 0 {
                continue;
            }
            let mut b = a;
            b[axis] = 1;
            let at_a = [cell[0] + a[0], cell[1] + a[1], cell[2] + a[2]];
            let at_b = [cell[0] + b[0], cell[1] + b[1], cell[2] + b[2]];
            let value_a = field[grid.index(at_a)];
            let value_b = field[grid.index(at_b)];
            if (value_a < 0.0) == (value_b < 0.0) {
                continue;
            }
            crossings += 1;
            let denominator = value_a as f64 - value_b as f64;
            if denominator.abs() < 1e-7 {
                continue;
            }
            let t = value_a as f64 / denominator;
            for component in 0..3 {
                offset[component] += if component == axis {
                    t
                } else {
                    a[component] as f64
                };
            }
        }
    }
    if crossings == 0 {
        return Err("Surface Nets mixed cell has no crossing edge".into());
    }
    let scale = 1.0 / crossings as f64;
    Ok(Vec3::new(
        (cell[0] as f64 + offset[0] * scale) as f32,
        (cell[1] as f64 + offset[1] * scale) as f32,
        (cell[2] as f64 + offset[2] * scale) as f32,
    ))
}

fn checkpoint(
    cancel: &mut impl FnMut(RemeshProgress) -> bool,
    phase: RemeshPhase,
    completed: usize,
    total: usize,
) -> Result<(), String> {
    if cancel(RemeshProgress {
        phase,
        completed,
        total,
    }) {
        Err("Remesh cancelled; the original mesh is unchanged".into())
    } else {
        Ok(())
    }
}

fn checked_product(dims: [usize; 3]) -> Result<usize, String> {
    dims.into_iter().try_fold(1_usize, |total, dim| {
        total
            .checked_mul(dim)
            .ok_or_else(|| "Remesh grid size overflow".into())
    })
}

struct Work {
    used: u64,
    limit: u64,
}
impl Work {
    fn add(&mut self, count: u64) -> Result<(), String> {
        self.used = self.used.checked_add(count).ok_or("Remesh work overflow")?;
        if self.used > self.limit {
            Err("Remesh work budget exceeded; reduce resolution or simplify the source".into())
        } else {
            Ok(())
        }
    }
}

struct Grid {
    dims: [usize; 3],
    origin: DVec3,
    voxel: f64,
}
impl Grid {
    fn index(&self, [x, y, z]: [usize; 3]) -> usize {
        (z * self.dims[1] + y) * self.dims[0] + x
    }
    fn point(&self, [x, y, z]: [usize; 3]) -> DVec3 {
        self.origin + DVec3::new(x as f64, y as f64, z as f64) * self.voxel
    }
}

// SculptGL-style material weights are sampled from the nearest candidate
// triangle at each covered grid node. Candidate coverage is a triangle's
// snapped voxel bounds, so this is bounded by the operation work budget.
// The reference optimizes the imported face stream before voxelization. The
// order matters only when equidistant faces have different mask values.
fn voxel_triangle_order(
    source: &Mesh,
    force_optimized_order: bool,
    work: &mut Work,
    cancel: &mut impl FnMut(RemeshProgress) -> bool,
) -> Result<Vec<usize>, String> {
    let faces = &source.triangles;
    if !force_optimized_order
        && (source
            .surface
            .as_ref()
            .is_some_and(SurfaceTopology::has_quads)
            || source.masks.iter().all(|&mask| mask == source.masks[0]))
    {
        return Ok((0..faces.len()).collect());
    }
    let mut rings = vec![Vec::<usize>::new(); source.positions.len()];
    // Face-ring construction in the reference fills each vertex's ring from
    // the end, so its scan sees incident faces in reverse source order.
    for face in (0..faces.len()).rev() {
        for &vertex in &faces[face] {
            rings[vertex as usize].push(face);
        }
    }
    let mut extra_owners = Vec::new();
    let virtual_faces = if let Some(uv) = source
        .surface
        .as_ref()
        .and_then(|surface| surface.uv.as_ref())
    {
        let mut first_uv = vec![None; rings.len()];
        let mut duplicate_uv = vec![Vec::<u32>::new(); rings.len()];
        let mut seen_duplicate = HashSet::new();
        for (face, indices) in faces.iter().zip(&uv.face_indices) {
            for (&vertex, &texture) in face.iter().zip(&indices[..3]) {
                let vertex = vertex as usize;
                match first_uv[vertex] {
                    None => first_uv[vertex] = Some(texture),
                    Some(first) if first != texture && seen_duplicate.insert((vertex, texture)) => {
                        duplicate_uv[vertex].push(texture);
                    }
                    _ => {}
                }
                work.add(1)?;
            }
        }
        let mut starts = vec![0_usize; rings.len()];
        let mut duplicate_slots = HashMap::new();
        for (vertex, textures) in duplicate_uv.iter().enumerate() {
            starts[vertex] = rings.len() + extra_owners.len();
            for (offset, &texture) in textures.iter().enumerate() {
                extra_owners.push(vertex);
                duplicate_slots.insert((vertex, texture), starts[vertex] + offset);
            }
        }
        let mut virtual_faces = Vec::with_capacity(faces.len());
        for (face, indices) in faces.iter().zip(&uv.face_indices) {
            let mut slots = [0_usize; 3];
            for corner in 0..3 {
                let vertex = face[corner] as usize;
                slots[corner] = if first_uv[vertex] == Some(indices[corner]) {
                    vertex
                } else {
                    *duplicate_slots
                        .get(&(vertex, indices[corner]))
                        .ok_or("Remesh UV seam mapping is incomplete")?
                };
            }
            virtual_faces.push(slots);
        }
        Some(virtual_faces)
    } else {
        None
    };
    let mut live: Vec<_> = rings.iter().map(Vec::len).collect();
    live.extend(extra_owners.iter().map(|&vertex| rings[vertex].len()));
    let mut stamps = vec![0_usize; live.len()];
    let mut emitted = vec![false; faces.len()];
    let mut stack = Vec::with_capacity(faces.len().saturating_mul(3));
    let mut order = Vec::with_capacity(faces.len());
    let mut timestamp = 33_usize;
    let mut cursor = 0_usize;
    let mut fan = Some(0_usize);
    while let Some(vertex) = fan {
        let mut candidates = Vec::new();
        let owner = if vertex < rings.len() {
            vertex
        } else {
            extra_owners[vertex - rings.len()]
        };
        for &face in &rings[owner] {
            work.add(1)?;
            if emitted[face] {
                continue;
            }
            if order.len() % 64 == 0 {
                checkpoint(cancel, RemeshPhase::Sample, order.len(), faces.len())?;
            }
            emitted[face] = true;
            order.push(face);
            let slots = virtual_faces
                .as_ref()
                .map_or_else(|| faces[face].map(|id| id as usize), |all| all[face]);
            for id in slots {
                stack.push(id);
                candidates.push(id);
                live[id] -= 1;
                if timestamp - stamps[id] > 32 {
                    stamps[id] = timestamp;
                    timestamp += 1;
                }
            }
        }
        fan = None;
        let mut best_priority = None;
        for id in candidates {
            if live[id] == 0 {
                continue;
            }
            let cache_position = timestamp - stamps[id];
            let priority = if cache_position + 2 * live[id] <= 32 {
                cache_position
            } else {
                0
            };
            if best_priority.is_none_or(|best| priority > best) {
                best_priority = Some(priority);
                fan = Some(id);
            }
        }
        if fan.is_some() {
            continue;
        }
        while let Some(id) = stack.pop() {
            if live[id] > 0 {
                fan = Some(id);
                break;
            }
        }
        if fan.is_some() {
            continue;
        }
        while cursor < live.len() {
            if live[cursor] > 0 {
                fan = Some(cursor);
                cursor += 1;
                break;
            }
            cursor += 1;
        }
    }
    if order.len() != faces.len() {
        return Err("Remesh face ordering did not visit every source triangle".into());
    }
    Ok(order)
}

fn voxelize_attributes(
    triangles: &[Triangle],
    order: &[usize],
    source: &Mesh,
    grid: &Grid,
    work: &mut Work,
    cancel: &mut impl FnMut(RemeshProgress) -> bool,
) -> Result<VoxelAttributes, String> {
    let count = checked_product(grid.dims)?;
    let mut distances = vec![f32::INFINITY; count];
    let mut unmasked = vec![1.0_f32; count];
    let mut colors = vec![[0.0_f32; 3]; count];
    let mut materials = source.materials.as_ref().map(|_| vec![[0.0_f32; 2]; count]);
    let last = grid.dims.map(|dim| (dim - 1) as f64);
    for (ordinal, &triangle_index) in order.iter().enumerate() {
        let triangle = &triangles[triangle_index];
        if ordinal % 64 == 0 {
            checkpoint(cancel, RemeshPhase::Sample, ordinal, triangles.len())?;
        }
        let min = ((triangle.bounds.min - grid.origin) / grid.voxel)
            .floor()
            .max(DVec3::ZERO);
        let max = ((triangle.bounds.max - grid.origin) / grid.voxel)
            .ceil()
            .min(DVec3::new(last[0], last[1], last[2]));
        let lower = [min.x as usize, min.y as usize, min.z as usize];
        let upper = [max.x as usize, max.y as usize, max.z as usize];
        let material_weight = triangle
            .ids
            .iter()
            .map(|&id| source.mask_convention.influence(source.masks[id as usize]) as f64)
            .sum::<f64>()
            / 3.0;
        let triangle_color = [0, 1, 2].map(|channel| {
            triangle
                .ids
                .iter()
                .map(|&id| source.colors[id as usize][channel] as f64)
                .sum::<f64>()
                / 3.0
        });
        let triangle_material = source.materials.as_ref().map(|source_materials| {
            [0, 1].map(|channel| {
                triangle
                    .ids
                    .iter()
                    .map(|&id| source_materials[id as usize][channel] as f64)
                    .sum::<f64>()
                    / 3.0
            })
        });
        for z in lower[2]..=upper[2] {
            for y in lower[1]..=upper[1] {
                for x in lower[0]..=upper[0] {
                    work.add(1)?;
                    let node = grid.index([x, y, z]);
                    let point = grid.point([x, y, z]);
                    let (nearest, _) = closest_point(point, triangle.p);
                    let distance = point.distance(nearest);
                    if distance < distances[node] as f64 {
                        distances[node] = distance as f32;
                        unmasked[node] = material_weight as f32;
                        colors[node] = triangle_color.map(|value| value as f32);
                        if let (Some(values), Some(triangle_material)) =
                            (&mut materials, triangle_material)
                        {
                            values[node] = triangle_material.map(|value| value as f32);
                        }
                    }
                }
            }
        }
    }
    Ok(VoxelAttributes {
        distances,
        unmasked,
        colors,
        materials,
    })
}

struct Topology {
    signs: Vec<i32>,
    components: usize,
    reversed: usize,
    volume: f64,
}
fn closed_topology(mesh: &Mesh) -> Result<Topology, String> {
    let mut edges = HashMap::<(u32, u32), (usize, bool, usize)>::new();
    let mut neighbors = vec![Vec::with_capacity(3); mesh.triangles.len()];
    let mut incident = vec![Vec::new(); mesh.positions.len()];
    let mut volume = vec![0.0_f64; mesh.triangles.len()];
    for (face, &ids) in mesh.triangles.iter().enumerate() {
        let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
        let area = (b - a).cross(c - a).length_squared();
        if !area.is_finite() || area == 0.0 {
            return Err(format!("Remesh source triangle {face} has zero area"));
        }
        let longest_edge_squared = (b - a)
            .length_squared()
            .max((c - b).length_squared())
            .max((a - c).length_squared());
        if area < longest_edge_squared.powi(2) * 1e-16 {
            return Err(format!(
                "Remesh source triangle {face} is numerically degenerate"
            ));
        }
        volume[face] = a.dot(b.cross(c)) / 6.0;
        for vertex in ids {
            incident[vertex as usize].push(face);
        }
        for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            let key = (a.min(b), a.max(b));
            let forward = a < b;
            match edges.get_mut(&key) {
                Some((other, direction, count)) => {
                    if *count != 1 {
                        return Err(
                            "Remesh requires manifold input: an edge has more than two faces"
                                .into(),
                        );
                    }
                    if forward == *direction {
                        return Err("Remesh requires consistent triangle winding".into());
                    }
                    neighbors[face].push(*other);
                    neighbors[*other].push(face);
                    *count += 1;
                }
                None => {
                    edges.insert(key, (face, forward, 1));
                }
            }
        }
    }
    if edges.values().any(|(_, _, count)| *count != 2) {
        return Err("Remesh requires a closed solid; open boundary edges are unsupported".into());
    }
    drop(edges);
    // Check each vertex link is one connected fan, not several solids touching
    // through a shared vertex ID. Coincident but distinct component IDs are OK.
    let mut stamps = vec![0_usize; mesh.triangles.len()];
    for (vertex, faces) in incident.iter().enumerate() {
        if faces.is_empty() {
            continue;
        }
        let stamp = vertex + 1;
        let mut stack = vec![faces[0]];
        stamps[faces[0]] = stamp;
        let mut visited = 0;
        while let Some(face) = stack.pop() {
            visited += 1;
            for &other in &neighbors[face] {
                if stamps[other] != stamp && mesh.triangles[other].contains(&(vertex as u32)) {
                    stamps[other] = stamp;
                    stack.push(other);
                }
            }
        }
        if visited != faces.len() {
            return Err(
                "Remesh requires manifold input: a vertex has disconnected surface fans".into(),
            );
        }
    }
    let mut signs = vec![0; mesh.triangles.len()];
    let mut components = 0;
    let mut reversed = 0;
    let mut total_volume = 0.0;
    for first in 0..mesh.triangles.len() {
        if signs[first] != 0 {
            continue;
        }
        components += 1;
        let mut stack = vec![first];
        let mut faces = Vec::new();
        signs[first] = 1;
        while let Some(face) = stack.pop() {
            faces.push(face);
            for &other in &neighbors[face] {
                if signs[other] == 0 {
                    signs[other] = 1;
                    stack.push(other);
                }
            }
        }
        let v: f64 = faces.iter().map(|&face| volume[face]).sum();
        if !v.is_finite() || v == 0.0 {
            return Err("Remesh requires nonzero enclosed component volume".into());
        }
        if v < 0.0 {
            reversed += 1;
            for face in faces {
                signs[face] = -1;
            }
        }
        total_volume += v.abs();
    }
    Ok(Topology {
        signs,
        components,
        reversed,
        volume: total_volume,
    })
}

#[derive(Clone, Copy)]
struct Bounds {
    min: DVec3,
    max: DVec3,
}
impl Bounds {
    fn points(points: &[DVec3]) -> Self {
        points.iter().fold(
            Self {
                min: DVec3::splat(f64::INFINITY),
                max: DVec3::splat(f64::NEG_INFINITY),
            },
            |bounds, &p| Self {
                min: bounds.min.min(p),
                max: bounds.max.max(p),
            },
        )
    }
    fn merged(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
    fn distance_squared(self, p: DVec3) -> f64 {
        (self.min - p)
            .max(DVec3::ZERO)
            .max(p - self.max)
            .length_squared()
    }
}

struct Triangle {
    p: [DVec3; 3],
    ids: [u32; 3],
    bounds: Bounds,
    component_sign: i32,
}
struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
struct Bvh {
    triangles: Vec<Triangle>,
    nodes: Vec<Node>,
}
struct Nearest {
    triangle: usize,
    distance_squared: f64,
    barycentric: [f64; 3],
}
impl Bvh {
    fn build(triangles: Vec<Triangle>) -> Self {
        let mut bvh = Self {
            triangles,
            nodes: Vec::new(),
        };
        bvh.build_node(0, bvh.triangles.len());
        bvh
    }
    fn build_node(&mut self, start: usize, end: usize) -> usize {
        let bounds = self.triangles[start..end]
            .iter()
            .fold(self.triangles[start].bounds, |a, t| a.merged(t.bounds));
        let index = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let size = bounds.max - bounds.min;
            let axis = if size.x >= size.y && size.x >= size.z {
                0
            } else if size.y >= size.z {
                1
            } else {
                2
            };
            let middle = start + (end - start) / 2;
            self.triangles[start..end].select_nth_unstable_by(middle - start, |a, b| {
                (a.bounds.min[axis] + a.bounds.max[axis])
                    .total_cmp(&(b.bounds.min[axis] + b.bounds.max[axis]))
                    .then(a.ids.cmp(&b.ids))
            });
            let left = self.build_node(start, middle);
            let right = self.build_node(middle, end);
            self.nodes[index].children = Some([left, right]);
        }
        index
    }
    fn nearest(&self, p: DVec3, work: &mut Work) -> Result<Nearest, String> {
        self.nearest_limited(p, f64::INFINITY, work)
    }
    fn nearest_limited(
        &self,
        p: DVec3,
        limit_squared: f64,
        work: &mut Work,
    ) -> Result<Nearest, String> {
        let mut best = Nearest {
            triangle: 0,
            distance_squared: limit_squared,
            barycentric: [0.0; 3],
        };
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            work.add(1)?;
            let node = &self.nodes[index];
            if node.bounds.distance_squared(p) > best.distance_squared {
                continue;
            }
            if let Some([left, right]) = node.children {
                if self.nodes[left].bounds.distance_squared(p)
                    <= self.nodes[right].bounds.distance_squared(p)
                {
                    stack.push(right);
                    stack.push(left);
                } else {
                    stack.push(left);
                    stack.push(right);
                }
            } else {
                for triangle in node.start..node.end {
                    work.add(1)?;
                    let (point, barycentric) = closest_point(p, self.triangles[triangle].p);
                    let distance_squared = p.distance_squared(point);
                    if distance_squared < best.distance_squared
                        || (distance_squared == best.distance_squared
                            && self.triangles[triangle].ids < self.triangles[best.triangle].ids)
                    {
                        best = Nearest {
                            triangle,
                            distance_squared,
                            barycentric,
                        };
                    }
                }
            }
        }
        Ok(best)
    }
    fn crossings(
        &self,
        ray: DVec3,
        hits: &mut Vec<(f64, i32)>,
        work: &mut Work,
    ) -> Result<(), String> {
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            work.add(1)?;
            let node = &self.nodes[index];
            if ray.y < node.bounds.min.y
                || ray.y > node.bounds.max.y
                || ray.z < node.bounds.min.z
                || ray.z > node.bounds.max.z
            {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                for triangle in &self.triangles[node.start..node.end] {
                    work.add(1)?;
                    let [a, b, c] = triangle.p;
                    let u = b - a;
                    let v = c - a;
                    let determinant = u.y * v.z - u.z * v.y;
                    if determinant == 0.0 {
                        continue;
                    }
                    let dy = ray.y - a.y;
                    let dz = ray.z - a.z;
                    let s = (dy * v.z - dz * v.y) / determinant;
                    let t = (u.y * dz - u.z * dy) / determinant;
                    // Half-open projected triangles prevent double hits on a
                    // triangulated planar face when a ray lands on its diagonal.
                    if s < 0.0 || t < 0.0 || s + t > 1.0 {
                        continue;
                    }
                    if (s == 0.0 && !projected_edge_inclusive(c, a))
                        || (t == 0.0 && !projected_edge_inclusive(a, b))
                        || (s + t == 1.0 && !projected_edge_inclusive(b, c))
                    {
                        continue;
                    }
                    let sign = if determinant < 0.0 { 1 } else { -1 } * triangle.component_sign;
                    hits.push((a.x + s * u.x + t * v.x, sign));
                }
            }
        }
        Ok(())
    }
}

fn projected_edge_inclusive(a: DVec3, b: DVec3) -> bool {
    a.y < b.y || (a.y == b.y && a.z < b.z)
}

fn closest_point(p: DVec3, [a, b, c]: [DVec3; 3]) -> (DVec3, [f64; 3]) {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let v = d2 / (d2 - d6);
        return (a + ac * v, [1.0 - v, 0.0, v]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let v = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * v, [0.0, 1.0 - v, v]);
    }
    let denominator = 1.0 / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

fn smooth(mesh: &mut Mesh, steps: u32, work: &mut Work) -> Result<(), String> {
    if steps == 0 {
        return Ok(());
    }
    let mut neighbors = vec![Vec::new(); mesh.positions.len()];
    for &[a, b, c] in &mesh.triangles {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            neighbors[a as usize].push(b as usize);
            neighbors[b as usize].push(a as usize);
        }
    }
    for row in &mut neighbors {
        row.sort_unstable();
        row.dedup();
    }
    let mut edges = HashMap::<(u32, u32), usize>::new();
    let mut pairs = Vec::new();
    for (face, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            let key = (a.min(b), a.max(b));
            if let Some(other) = edges.remove(&key) {
                pairs.push([other, face]);
            } else {
                edges.insert(key, face);
            }
        }
    }
    for _ in 0..steps {
        for scale in [0.5, -0.53] {
            let displacement: Vec<DVec3> = mesh
                .positions
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let mean: DVec3 = neighbors[i]
                        .iter()
                        .map(|&j| mesh.positions[j].as_dvec3())
                        .sum::<DVec3>()
                        / neighbors[i].len() as f64;
                    (mean - p.as_dvec3()) * scale
                })
                .collect();
            let original: Vec<_> = mesh
                .triangles
                .iter()
                .map(|&ids| face_geometry(&mesh.positions, ids))
                .collect();
            let mut weights = vec![1.0_f64; mesh.positions.len()];
            let mut next = mesh.positions.clone();
            let mut accepted = false;
            for _ in 0..16 {
                for (i, p) in next.iter_mut().enumerate() {
                    *p = (mesh.positions[i].as_dvec3() + displacement[i] * weights[i]).as_vec3();
                }
                let geometry: Vec<_> = mesh
                    .triangles
                    .iter()
                    .map(|&ids| face_geometry(&next, ids))
                    .collect();
                let mut invalid = Vec::new();
                for (face, ((old, old_quality), (new, new_quality))) in
                    original.iter().zip(&geometry).enumerate()
                {
                    if new.dot(*old) < old.length_squared() * 0.25
                        || *new_quality < old_quality.min(0.05) * 0.999999
                    {
                        invalid.extend(mesh.triangles[face]);
                    }
                }
                for &[a, b] in &pairs {
                    let old_cosine = original[a].0.normalize().dot(original[b].0.normalize());
                    let new_cosine = geometry[a]
                        .0
                        .normalize_or_zero()
                        .dot(geometry[b].0.normalize_or_zero());
                    if new_cosine + 1e-12 < old_cosine.min(0.5) {
                        invalid.extend(mesh.triangles[a]);
                        invalid.extend(mesh.triangles[b]);
                    }
                }
                work.add(mesh.triangles.len() as u64 + pairs.len() as u64)?;
                if invalid.is_empty() {
                    accepted = true;
                    break;
                }
                invalid.sort_unstable();
                invalid.dedup();
                for vertex in invalid {
                    weights[vertex as usize] *= 0.5;
                }
            }
            // An inability to find a safe smoothing step does not compromise
            // reconstruction; retain the already valid unsmoothed surface.
            if accepted {
                mesh.positions = next;
            }
        }
    }
    Ok(())
}

fn face_geometry(positions: &[Vec3], ids: [u32; 3]) -> (DVec3, f64) {
    let [a, b, c] = ids.map(|i| positions[i as usize].as_dvec3());
    let cross = (b - a).cross(c - a);
    let edge_squared = (b - a)
        .length_squared()
        .max((c - b).length_squared())
        .max((a - c).length_squared());
    (cross, cross.length() / edge_squared)
}

#[cfg(test)]
mod reference_order_tests {
    use super::*;

    #[test]
    fn no_uv_voxel_face_order_matches_pinned_sculptgl_import() {
        let mesh = Mesh::from_obj(include_str!(
            "../tests/fixtures/independent-reference-sphere.obj"
        ))
        .unwrap();
        let mut work = Work {
            used: 0,
            limit: 150_000_000,
        };
        let order = voxel_triangle_order(&mesh, false, &mut work, &mut |_| false).unwrap();
        // Face IDs are mapped back to the authored OBJ from the separately
        // captured pinned SculptGL no-UV import, before either remesher ran.
        assert_eq!(
            &order[..28],
            &[
                23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
                0, 70, 25, 24, 71
            ]
        );
        assert_eq!(order.len(), 528);
        assert_eq!(
            order
                .iter()
                .enumerate()
                .map(|(i, &face)| (i + 1) * face)
                .sum::<usize>(),
            48_968_178
        );
    }

    #[test]
    fn uv_seam_voxel_face_order_matches_pinned_sculptgl_import() {
        let mesh = Mesh::import_obj(include_bytes!(
            "../tests/fixtures/independent-seamed-sphere.obj"
        ))
        .unwrap()
        .mesh;
        let mut work = Work {
            used: 0,
            limit: 150_000_000,
        };
        let order = voxel_triangle_order(&mesh, false, &mut work, &mut |_| false).unwrap();
        assert_eq!(
            &order[..32],
            &[
                23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
                0, 41, 40, 38, 89, 88, 86, 39, 87
            ]
        );
        assert_eq!(order.len(), 528);
        assert_eq!(
            order
                .iter()
                .enumerate()
                .map(|(i, &face)| (i + 1) * face)
                .sum::<usize>(),
            48_503_781
        );
    }
}
