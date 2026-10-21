//! Bounded, shape-preserving triangle refinement near an active brush.
use crate::{Mesh, Vec3, MAX_TRIANGLES, MAX_VERTICES};
use std::collections::{HashMap, HashSet};

const MAX_EDGES_PER_ROUND: usize = 512;
const MAX_ROUNDS: usize = 2;
const TARGET_EDGE_RADIUS: f64 = 0.12;
const INTERIOR_DRAW_ROUNDS: usize = 8;
const INTERIOR_DRAW_NEW_VERTICES: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PatchRefinement {
    pub vertices_added: usize,
    pub triangles_added: usize,
    /// More qualifying edges remain, or a round's fixed work budget was reached.
    pub limited: bool,
}

/// Ordered ancestry lets stroke-local fields follow shape-preserving splits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefinementParents {
    pub vertex: usize,
    pub parents: [usize; 3],
    pub weights: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum GroupKey {
    Edge(u32, u32),
    Symmetric([u32; 6]),
}

#[derive(Default)]
struct EdgeGroup {
    edges: Vec<(u32, u32)>,
    overflow: bool,
}

struct SelectedEdges {
    edges: Vec<(u32, u32)>,
    limited: bool,
}

#[derive(Clone, Copy)]
struct PatchQuery {
    center: glam::DVec3,
    mirrored_center: glam::DVec3,
    primary_box: (Vec3, Vec3),
    mirrored_box: (Vec3, Vec3),
    radius_squared: f64,
    target_squared: f64,
    symmetry_x: bool,
    symmetry_offset: f32,
    move_quality: bool,
    draw_quality: bool,
}

pub(crate) fn outward_refinement_box(center: glam::DVec3, radius: f64) -> (Vec3, Vec3) {
    // Mesh::validate limits each stored f32 coordinate to 1e6. The exact
    // predicate below admits an AABB up to 64 f64 ulps beyond the radius;
    // include that same worst-case slack before rounding outward to f32.
    let scale = center.abs().max_element().max(radius).max(1_000_000.0);
    let reach = radius + 64.0 * f64::EPSILON * scale;
    let low = center - glam::DVec3::splat(reach);
    let high = center + glam::DVec3::splat(reach);
    (
        Vec3::new(
            (low.x as f32).next_down(),
            (low.y as f32).next_down(),
            (low.z as f32).next_down(),
        ),
        Vec3::new(
            (high.x as f32).next_up(),
            (high.y as f32).next_up(),
            (high.z as f32).next_up(),
        ),
    )
}

impl PatchQuery {
    fn new(
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
    ) -> Result<Self, String> {
        if !center.is_finite()
            || !radius.is_finite()
            || radius <= 0.0
            || !symmetry_offset.is_finite()
            || symmetry_offset.abs() > 1_000_000.0
        {
            return Err(
                "Patch refinement requires a finite center and positive finite radius".into(),
            );
        }
        let center = center.as_dvec3();
        let radius = f64::from(radius);
        let mirrored_center = glam::DVec3::new(
            2.0 * f64::from(symmetry_offset) - center.x,
            center.y,
            center.z,
        );
        Ok(Self {
            center,
            mirrored_center,
            primary_box: outward_refinement_box(center, radius),
            mirrored_box: outward_refinement_box(mirrored_center, radius),
            radius_squared: radius * radius,
            target_squared: (radius * TARGET_EDGE_RADIUS).powi(2),
            symmetry_x,
            symmetry_offset,
            move_quality: false,
            draw_quality: false,
        })
    }

    /// A selected segment is inside every incident triangle's AABB. This
    /// conservative broad phase therefore retains all faces needed to detect
    /// duplicate/non-manifold incidences and produce conforming children.
    fn contains_face(self, positions: &[Vec3], triangle: [u32; 3]) -> bool {
        let [a, b, c] = triangle.map(|vertex| positions[vertex as usize]);
        let minimum = a.min(b).min(c);
        let maximum = a.max(b).max(c);
        let overlaps =
            |(low, high): (Vec3, Vec3)| minimum.cmple(high).all() && maximum.cmpge(low).all();
        if !overlaps(self.primary_box) && (!self.symmetry_x || !overlaps(self.mirrored_box)) {
            return false;
        }
        self.contains_face_exact(positions, triangle)
    }

    fn contains_face_exact(self, positions: &[Vec3], triangle: [u32; 3]) -> bool {
        let [a, b, c] = triangle.map(|vertex| positions[vertex as usize].as_dvec3());
        if a.distance_squared(b) <= self.target_squared
            && b.distance_squared(c) <= self.target_squared
            && c.distance_squared(a) <= self.target_squared
        {
            return false;
        }
        let minimum = a.min(b).min(c);
        let maximum = a.max(b).max(c);
        let near = |center: glam::DVec3| {
            // The narrow-phase projection performs more floating arithmetic
            // than clamping to an AABB. Retain tangent bounds conservatively
            // when their results differ by a few f64 rounding units.
            let scale = minimum
                .abs()
                .max_element()
                .max(maximum.abs().max_element())
                .max(center.abs().max_element())
                .max(self.radius_squared.sqrt());
            let slack = 64.0 * f64::EPSILON * scale;
            center.distance_squared(center.clamp(minimum, maximum))
                <= (self.radius_squared.sqrt() + slack).powi(2)
        };
        near(self.center) || (self.symmetry_x && near(self.mirrored_center))
    }

    fn contains_edge(self, mesh: &Mesh, a: u32, b: u32) -> bool {
        let (a, b) = (a as usize, b as usize);
        let a = mesh.positions[a].as_dvec3();
        let b = mesh.positions[b].as_dvec3();
        let direction = b - a;
        let length_squared = direction.length_squared();
        if length_squared <= self.target_squared {
            return false;
        }
        let near = |center: glam::DVec3| {
            let parameter = ((center - a).dot(direction) / length_squared).clamp(0.0, 1.0);
            (a + direction * parameter).distance_squared(center) <= self.radius_squared
        };
        near(self.center) || (self.symmetry_x && near(self.mirrored_center))
    }
}

impl Mesh {
    /// Split long edges within a brush's world-space radius, in at most two
    /// rounds of 512 added vertices each. Shared edges are split on both sides,
    /// including at the patch boundary; existing positions remain unchanged.
    /// Mask and RGB midpoint interpolation follows the unchanged linear surface.
    /// Fully masked faces do not initiate additional sculpt resolution. A
    /// protected edge on an editable face may still split: its fully protected
    /// midpoint keeps the boundary conforming without generating thin wedges.
    ///
    /// Exact reflected edge groups are admitted together when X symmetry is on,
    /// so the work budget cannot cut a symmetric group in half. This preserves
    /// existing exact symmetry without assuming an imported mesh is symmetric.
    /// A self-mirrored two-edge face uses an interior linear sample instead of
    /// an asymmetric diagonal; symmetry mode reserves half the round budget for
    /// those samples. Near symmetry is not welded or treated as exact symmetry.
    /// Allocation-limit or selected-topology failures leave the mesh untouched.
    /// This adds triangles, not a persistent subdivision hierarchy or remeshing.
    pub fn refine_brush_patch(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
    ) -> Result<PatchRefinement, String> {
        self.refine_brush_patch_impl(
            center, radius, symmetry_x, 0.0, None, MAX_ROUNDS, None, false, false,
        )
        .map(|(report, _)| report)
    }

    /// Draw has already changed these vertices and deferred its normal update.
    /// The public entry point retains its full normal-rebuild semantics; this
    /// internal path combines Draw's dirty patch with the topology transition.
    pub(crate) fn refine_brush_patch_with_dirty_normals(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        dirty_vertices: &[usize],
    ) -> Result<PatchRefinement, String> {
        self.refine_brush_patch_impl(
            center,
            radius,
            symmetry_x,
            0.0,
            Some(dirty_vertices),
            MAX_ROUNDS,
            None,
            false,
            false,
        )
        .map(|(report, _)| report)
    }

    /// Move samples its fixed deformation field on the resting surface, before
    /// dragging. Four bounded rounds resolve a small footprint without adding
    /// topology during the gesture or invalidating captured influence weights.
    pub(crate) fn refine_move_patch(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
    ) -> Result<PatchRefinement, String> {
        // Move's transition to coarse triangles must lie outside its active
        // deformation field. Otherwise a skinny refinement transition can
        // clamp a smooth target before the adequately sampled center moves.
        self.refine_brush_patch_impl(
            center,
            radius * 2.0,
            symmetry_x,
            0.0,
            None,
            4,
            Some(radius),
            false,
            false,
        )
        .map(|(report, _)| report)
    }

    pub(crate) fn refine_brush_patch_with_options(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
        dirty_vertices: &[usize],
        draw_quality: bool,
    ) -> Result<(PatchRefinement, Vec<RefinementParents>), String> {
        self.refine_brush_patch_impl(
            center,
            radius,
            symmetry_x,
            symmetry_offset,
            Some(dirty_vertices),
            MAX_ROUNDS,
            None,
            true,
            draw_quality,
        )
    }

    /// Prepare a contact that lies inside a coarse Draw face but reaches no
    /// vertex. Only the returned candidate may be replayed and committed; any
    /// refusal leaves the caller's mesh byte-for-byte unchanged.
    pub(crate) fn prepare_draw_interior_contact(
        &self,
        center: Vec3,
        contact_normal: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
    ) -> Result<Option<(Mesh, Vec<RefinementParents>)>, String> {
        self.validate()?;
        self.require_plain_topology("Interior Draw refinement")?;
        let query = PatchQuery::new(center, radius, symmetry_x, symmetry_offset)?;
        let mut candidate = self.try_snapshot()?;
        let original_vertices = self.positions.len();
        let mut origins = Vec::new();
        origins
            .try_reserve_exact(self.triangles.len())
            .map_err(|_| "Cannot allocate interior Draw face origins")?;
        origins.extend(0..self.triangles.len());
        let mut touched_origins = HashSet::new();
        let reflected_center = Vec3::new(2.0 * symmetry_offset - center.x, center.y, center.z);
        let reflected_normal = Vec3::new(-contact_normal.x, contact_normal.y, contact_normal.z);
        let targets = if symmetry_x
            && reflected_center != center
            && has_face_aabb_near(self, reflected_center, radius)
        {
            vec![
                (center, contact_normal),
                (reflected_center, reflected_normal),
            ]
        } else {
            vec![(center, contact_normal)]
        };
        // An interior point stays in the descendants of its first containing
        // face: rounds add only linear points and do not move the old surface.
        // Initial full-scene uniqueness remains mandatory for overlapping
        // shells; subsequent rounds inspect only those bounded children.
        let mut target_children: Vec<Option<Vec<usize>>> = vec![None; targets.len()];
        let mut ancestry = Vec::new();
        let mut added_any = false;
        for _ in 0..INTERIOR_DRAW_ROUNDS {
            let prepared = |point: Vec3| {
                candidate.positions[original_vertices..]
                    .iter()
                    .any(|&vertex| vertex.distance_squared(point) < (radius * 0.5).powi(2))
            };
            if targets.iter().all(|&(point, _)| prepared(point)) {
                break;
            }
            let mut contact_faces = Vec::new();
            let mut target_faces = Vec::new();
            for (target, &(point, normal)) in targets.iter().enumerate() {
                if prepared(point) {
                    continue;
                }
                let Some(face) = unique_interior_contact_face(
                    &candidate,
                    point,
                    normal,
                    radius,
                    target_children[target].as_deref(),
                ) else {
                    return Ok(None);
                };
                if candidate.triangles[face]
                    .iter()
                    .all(|&vertex| candidate.fully_protected(vertex as usize))
                {
                    return Ok(None);
                }
                contact_faces.push(face);
                target_faces.push((target, face));
            }
            contact_faces.sort_unstable();
            contact_faces.dedup();
            let selected: Vec<_> = contact_faces
                .iter()
                .flat_map(|&face| triangle_edges(candidate.triangles[face]))
                .map(|(a, b)| (a.min(b), a.max(b)))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            let mut selected = selected;
            selected.sort_unstable();
            if selected.is_empty()
                || candidate.positions.len() + selected.len()
                    > original_vertices + INTERIOR_DRAW_NEW_VERTICES
            {
                return Ok(None);
            }
            let selected_set: HashSet<_> = selected.iter().copied().collect();
            let selected_vertices: Vec<_> = selected
                .iter()
                .flat_map(|&(a, b)| [a as usize, b as usize])
                .collect();
            candidate.ensure_local_incidence();
            let mut candidate_faces = Vec::new();
            let incident_faces = candidate.cached_faces_for_vertices(&selected_vertices);
            let mut selected_children = Vec::new();
            selected_children
                .try_reserve_exact(incident_faces.len())
                .map_err(|_| "Cannot allocate interior Draw child counts")?;
            for face in incident_faces {
                let triangle = candidate.triangles[face];
                let slots = triangle_edges(triangle)
                    .map(|(a, b)| selected_set.contains(&(a.min(b), a.max(b))));
                let split_count = slots.iter().filter(|&&split| split).count();
                if split_count == 0 {
                    continue;
                }
                let extra_center = query.symmetry_x
                    && split_count == 2
                    && symmetric_corner(
                        triangle,
                        slots.map(|split| split.then_some(())),
                        &candidate.positions,
                        symmetry_offset,
                    )
                    .is_some();
                let child_count = 1 + split_count + usize::from(extra_center) * 2;
                candidate_faces.push(face);
                selected_children.push((
                    face,
                    child_count,
                    quality(&candidate.positions, triangle).min(0.1),
                ));
                touched_origins.insert(origins[face]);
            }
            if candidate_faces.len() > 96
                || candidate.triangles.len() + selected.len() * 4 > MAX_TRIANGLES
            {
                return Ok(None);
            }
            let mut dirty = Vec::new();
            refine_round(
                &mut candidate,
                &selected,
                &candidate_faces,
                query,
                &mut dirty,
                Some(&mut ancestry),
            )?;
            let mut next_origins = Vec::new();
            next_origins
                .try_reserve_exact(candidate.triangles.len())
                .map_err(|_| "Cannot allocate interior Draw face origins")?;
            let mut old_cursor: usize = 0;
            let mut new_cursor: usize = 0;
            let mut remapped_children = Vec::new();
            for &(face, children, floor) in &selected_children {
                next_origins.extend_from_slice(&origins[old_cursor..face]);
                new_cursor += face - old_cursor;
                let Some(next_cursor) = new_cursor.checked_add(children) else {
                    return Ok(None);
                };
                let Some(child_faces) = candidate.triangles.get(new_cursor..next_cursor) else {
                    return Ok(None);
                };
                remapped_children.push((face, new_cursor, next_cursor));
                for &child in child_faces {
                    if quality(&candidate.positions, child) < floor {
                        return Ok(None);
                    }
                    next_origins.push(origins[face]);
                }
                old_cursor = face + 1;
                new_cursor = next_cursor;
            }
            next_origins.extend_from_slice(&origins[old_cursor..]);
            if next_origins.len() != candidate.triangles.len() {
                return Ok(None);
            }
            for (target, old_face) in target_faces {
                let Some(&(_, start, end)) = remapped_children
                    .iter()
                    .find(|&&(face, _, _)| face == old_face)
                else {
                    return Ok(None);
                };
                target_children[target] = Some((start..end).collect());
            }
            origins = next_origins;
            added_any = true;
        }
        if !added_any
            || !targets.iter().all(|&(point, _)| {
                candidate.positions[original_vertices..]
                    .iter()
                    .any(|&v| v.distance_squared(point) < radius * radius)
            })
        {
            return Ok(None);
        }
        let affected_faces: Vec<_> = origins
            .iter()
            .enumerate()
            .filter_map(|(face, origin)| touched_origins.contains(origin).then_some(face))
            .collect();
        candidate.ensure_local_incidence();
        let pairs = candidate.local_face_pairs_for_faces(&affected_faces)?;
        for &[a, b] in &pairs {
            let (a, b) = (a as usize, b as usize);
            let (oa, ob) = (origins[a], origins[b]);
            if !touched_origins.contains(&oa) && !touched_origins.contains(&ob) {
                continue;
            }
            let old_cos = if oa == ob {
                1.0
            } else {
                let na = cross(&self.positions, self.triangles[oa]).normalize_or_zero();
                let nb = cross(&self.positions, self.triangles[ob]).normalize_or_zero();
                na.dot(nb)
            };
            let na = cross(&candidate.positions, candidate.triangles[a]).normalize_or_zero();
            let nb = cross(&candidate.positions, candidate.triangles[b]).normalize_or_zero();
            if !na.is_finite() || !nb.is_finite() || na.dot(nb) < old_cos.min(0.5) {
                return Ok(None);
            }
        }
        candidate
            .normals
            .try_reserve_exact(candidate.positions.len() - candidate.normals.len())
            .map_err(|_| "Cannot allocate interior Draw normals")?;
        candidate
            .normals
            .resize(candidate.positions.len(), Vec3::ZERO);
        let mut dirty = Vec::new();
        dirty
            .try_reserve_exact(affected_faces.len().saturating_mul(3))
            .map_err(|_| "Cannot allocate interior Draw normal patch")?;
        for &face in &affected_faces {
            dirty.extend(candidate.triangles[face].map(|v| v as usize));
        }
        candidate.recompute_normals_near_vertices_cached(&dirty);
        candidate.validate()?;
        Ok(Some((candidate, ancestry)))
    }

    pub(crate) fn refine_move_patch_at_offset(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
    ) -> Result<PatchRefinement, String> {
        self.refine_brush_patch_impl(
            center,
            radius * 2.0,
            symmetry_x,
            symmetry_offset,
            None,
            4,
            Some(radius),
            false,
            false,
        )
        .map(|(report, _)| report)
    }

    // This internal entry point combines a spatial query, the caller's dirty
    // normal set, and separate bounded policies for live dabs and frozen Move
    // gestures. Public callers expose only the options relevant to their job.
    #[allow(clippy::too_many_arguments)]
    fn refine_brush_patch_impl(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
        dirty_vertices: Option<&[usize]>,
        max_rounds: usize,
        target_radius: Option<f32>,
        collect_parents: bool,
        draw_quality: bool,
    ) -> Result<(PatchRefinement, Vec<RefinementParents>), String> {
        self.validate()?;
        self.require_plain_topology("Adaptive refinement")?;
        let mut query = PatchQuery::new(center, radius, symmetry_x, symmetry_offset)?;
        query.draw_quality = draw_quality;
        if let Some(target_radius) = target_radius {
            query.target_squared = (f64::from(target_radius) * TARGET_EDGE_RADIUS).powi(2);
            query.move_quality = true;
        }
        let original_vertices = self.positions.len();
        let original_triangles = self.triangles.len();
        let mut candidate_faces: Vec<_> = self
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(face, &triangle)| {
                query
                    .contains_face(&self.positions, triangle)
                    .then_some(face)
            })
            .collect();
        let mut selected = select_edges(self, &candidate_faces, query);
        preserve_child_quality(self, &candidate_faces, query, &mut selected)?;
        if selected.edges.is_empty() {
            return Ok((
                PatchRefinement {
                    limited: selected.limited,
                    ..PatchRefinement::default()
                },
                Vec::new(),
            ));
        }
        // Every selected edge adds a vertex and at least one child face. Reject
        // this inexpensive lower bound before cloning a full document snapshot;
        // each round also preflights exact incidences and interior face samples.
        if original_vertices + selected.edges.len() > MAX_VERTICES
            || original_triangles + selected.edges.len() > MAX_TRIANGLES
        {
            return Err(
                "Patch refinement exceeds the one-million vertex or two-million triangle limit"
                    .into(),
            );
        }
        let mut candidate = self.snapshot();
        let mut dirty = dirty_vertices.unwrap_or_default().to_vec();
        let mut limited = false;
        let mut parents = Vec::new();
        for round in 0..max_rounds {
            if round != 0 {
                selected = select_edges(&candidate, &candidate_faces, query);
                preserve_child_quality(&candidate, &candidate_faces, query, &mut selected)?;
            }
            limited |= selected.limited;
            if selected.edges.is_empty() {
                break;
            }
            candidate_faces = refine_round(
                &mut candidate,
                &selected.edges,
                &candidate_faces,
                query,
                &mut dirty,
                collect_parents.then_some(&mut parents),
            )?;
        }
        limited |= candidate_faces.iter().any(|&face| {
            let triangle = candidate.triangles[face];
            !triangle
                .iter()
                .all(|&vertex| candidate.fully_protected(vertex as usize))
                && triangle_edges(triangle)
                    .into_iter()
                    .any(|(a, b)| query.contains_edge(&candidate, a, b))
        });
        let result = PatchRefinement {
            vertices_added: candidate.positions.len() - original_vertices,
            triangles_added: candidate.triangles.len() - original_triangles,
            limited,
        };
        if result.vertices_added != 0 {
            candidate.clear_cache();
            if dirty_vertices.is_some() {
                candidate
                    .normals
                    .resize(candidate.positions.len(), Vec3::ZERO);
                candidate.recompute_normals_near_vertices(&dirty);
            } else {
                candidate.recompute_normals();
            }
            candidate.validate()?;
            *self = candidate;
        }
        Ok((result, parents))
    }
}

fn has_face_aabb_near(mesh: &Mesh, center: Vec3, radius: f32) -> bool {
    let center = center.as_dvec3();
    let radius_squared = f64::from(radius).powi(2);
    mesh.triangles.iter().any(|&triangle| {
        let [a, b, c] = triangle.map(|v| mesh.positions[v as usize].as_dvec3());
        let minimum = a.min(b).min(c);
        let maximum = a.max(b).max(c);
        center.distance_squared(center.clamp(minimum, maximum)) <= radius_squared
    })
}

fn unique_interior_contact_face(
    mesh: &Mesh,
    center: Vec3,
    contact_normal: Vec3,
    radius: f32,
    candidates: Option<&[usize]>,
) -> Option<usize> {
    let center = center.as_dvec3();
    let contact_normal = contact_normal.as_dvec3().normalize_or_zero();
    let scale = center.abs().max_element().max(f64::from(radius)).max(1.0);
    let tolerance = (64.0 * f64::from(f32::EPSILON) * scale).max(1e-5 * f64::from(radius));
    let mut found = None;
    let count = candidates.map_or(mesh.triangles.len(), <[_]>::len);
    for index in 0..count {
        let face = candidates.map_or(index, |faces| faces[index]);
        let triangle = mesh.triangles[face];
        let [a, b, c] = triangle.map(|v| mesh.positions[v as usize].as_dvec3());
        let minimum = a.min(b).min(c);
        let maximum = a.max(b).max(c);
        if center.distance_squared(center.clamp(minimum, maximum))
            > (f64::from(radius) + tolerance).powi(2)
        {
            continue;
        }
        let cross = (b - a).cross(c - a);
        let area_squared = cross.length_squared();
        if area_squared <= 1e-24 || !cross.is_finite() {
            continue;
        }
        let face_normal = cross / area_squared.sqrt();
        if face_normal.dot(contact_normal) <= 0.0 {
            continue;
        }
        let signed = (center - a).dot(face_normal);
        if signed.abs() > tolerance {
            continue;
        }
        let point = center - face_normal * signed;
        let barycentric = [
            (b - point).cross(c - point).dot(cross) / area_squared,
            (c - point).cross(a - point).dot(cross) / area_squared,
            (a - point).cross(b - point).dot(cross) / area_squared,
        ];
        // Inserting near an edge would birth a needle; keep this path for
        // contacts safely inside one coarse triangle.
        if barycentric.iter().any(|&weight| weight < 0.1) {
            continue;
        }
        if found.replace(face).is_some() {
            // Coincident front-facing sheets cannot be disambiguated from a
            // world-space contact alone.
            return None;
        }
    }
    found
}

fn select_edges(mesh: &Mesh, faces: &[usize], query: PatchQuery) -> SelectedEdges {
    let mut group_indices = HashMap::new();
    let mut groups: Vec<EdgeGroup> = Vec::new();
    // This set contains only retained candidate groups, never every mesh edge.
    // Coincident disconnected geometry can create a large symmetric group, so
    // groups are capped too, instead of growing with the full input topology.
    let mut seen = HashSet::new();
    let mut limited = false;
    for &face in faces {
        let triangle = mesh.triangles[face];
        if triangle
            .iter()
            .all(|&vertex| mesh.fully_protected(vertex as usize))
        {
            continue;
        }
        for (a, b) in triangle_edges(triangle) {
            let edge = (a.min(b), a.max(b));
            if seen.contains(&edge) || !query.contains_edge(mesh, a, b) {
                continue;
            }
            let key = if query.symmetry_x {
                symmetric_key(
                    mesh.positions[a as usize],
                    mesh.positions[b as usize],
                    query.symmetry_offset,
                )
            } else {
                GroupKey::Edge(edge.0, edge.1)
            };
            let group = if let Some(&index) = group_indices.get(&key) {
                index
            } else {
                if groups.len() == MAX_EDGES_PER_ROUND {
                    limited = true;
                    continue;
                }
                let index = groups.len();
                groups.push(EdgeGroup::default());
                group_indices.insert(key, index);
                index
            };
            if groups[group].overflow {
                continue;
            }
            if groups[group].edges.len() == MAX_EDGES_PER_ROUND {
                groups[group].overflow = true;
                limited = true;
                continue;
            }
            seen.insert(edge);
            groups[group].edges.push(edge);
        }
    }
    // A self-mirrored two-edge split needs one interior face sample rather than
    // choosing an asymmetric quad diagonal. Reserve space for those samples:
    // each such face consumes two edge incidences, so their count is at most
    // the admitted edge count on the selected manifold patch.
    let edge_budget = if query.symmetry_x {
        MAX_EDGES_PER_ROUND / 2
    } else {
        MAX_EDGES_PER_ROUND
    };
    let mut edges = Vec::with_capacity(edge_budget);
    for group in groups {
        if group.overflow || edges.len() + group.edges.len() > edge_budget {
            limited = true;
            continue;
        }
        edges.extend(group.edges);
    }
    SelectedEdges { edges, limited }
}

fn symmetric_key(a: Vec3, b: Vec3, offset: f32) -> GroupKey {
    let bits = |p: Vec3| {
        p.to_array()
            .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
    };
    let sorted = |a: Vec3, b: Vec3| {
        let (a, b) = (bits(a), bits(b));
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        [a[0], a[1], a[2], b[0], b[1], b[2]]
    };
    let reflected = |p: Vec3| Vec3::new(2.0 * offset - p.x, p.y, p.z);
    GroupKey::Symmetric(sorted(a, b).min(sorted(reflected(a), reflected(b))))
}

/// A conforming transition must not repeatedly bisect a wedge against a longer
/// unsplit edge. Drop complete reflected groups that would manufacture skinny
/// children. Removing a split can change another transition, so repeat locally
/// with a fixed work bound; every unsuccessful pass removes at least one group.
fn preserve_child_quality(
    mesh: &Mesh,
    faces: &[usize],
    query: PatchQuery,
    selected: &mut SelectedEdges,
) -> Result<(), String> {
    let mut incidences: HashMap<_, _> = selected
        .edges
        .iter()
        .map(|&edge| (edge, (0_usize, false)))
        .collect();
    for &(a, b) in &selected.edges {
        let (a, b) = (mesh.positions[a as usize], mesh.positions[b as usize]);
        let midpoint = (a + b) * 0.5;
        if midpoint == a || midpoint == b {
            return Err(
                "Patch refinement midpoint cannot be represented at this coordinate scale".into(),
            );
        }
    }
    let mut unique_faces = HashSet::new();
    for &face in faces {
        let triangle = mesh.triangles[face];
        let mut touched = false;
        for (a, b) in triangle_edges(triangle) {
            if let Some((count, forward)) = incidences.get_mut(&(a.min(b), a.max(b))) {
                if *count == 2 {
                    return Err("Patch refinement refuses a selected non-manifold edge".into());
                }
                if *count == 1 && *forward == (a < b) {
                    return Err("Patch refinement requires consistent selected-edge winding".into());
                }
                *count += 1;
                *forward = a < b;
                touched = true;
            }
        }
        if touched {
            let mut sorted = triangle;
            sorted.sort_unstable();
            if !unique_faces.insert(sorted) {
                return Err("Patch refinement refuses duplicate selected faces".into());
            }
            if cross(&mesh.positions, triangle).length_squared() == 0.0 {
                return Err("Patch refinement refuses a degenerate selected face".into());
            }
        }
    }
    for _ in 0..8 {
        if selected.edges.is_empty() {
            return Ok(());
        }
        let edges: HashSet<_> = selected.edges.iter().copied().collect();
        let mut rejected = HashSet::new();
        for &face in faces {
            let triangle = mesh.triangles[face];
            let slots = triangle_edges(triangle)
                .map(|(a, b)| edges.contains(&(a.min(b), a.max(b))).then_some(()));
            if slots.iter().all(Option::is_none) {
                continue;
            }
            // Move and Draw guard deformation with cross/longest-edge-squared.
            // Keep their generated children above the same local quality
            // measure; ordinary tools retain the established standard metric.
            let triangle_quality: fn(&[Vec3], [u32; 3]) -> f64 =
                if query.move_quality || query.draw_quality {
                    quality
                } else {
                    standard_quality
                };
            let floor = if query.draw_quality { 0.12 } else { 0.15 };
            let parent_quality = triangle_quality(&mesh.positions, triangle);
            // Malformed selected topology is rejected by the atomic round
            // preflight, rather than hidden by skipping its selected edges.
            if !parent_quality.is_finite() || parent_quality <= 0.0 {
                continue;
            }
            let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize]);
            let mut local = [
                a,
                b,
                c,
                (a + b) * 0.5,
                (b + c) * 0.5,
                (c + a) * 0.5,
                Vec3::ZERO,
            ];
            let center = if query.symmetry_x
                && slots.iter().flatten().count() == 2
                && symmetric_corner(triangle, slots, &mesh.positions, query.symmetry_offset)
                    .is_some()
            {
                let [a, b, c] =
                    symmetric_corner(triangle, slots, &mesh.positions, query.symmetry_offset)
                        .unwrap()
                        .map(|vertex| mesh.positions[vertex as usize]);
                local[6] = (a + c) * 0.375 + b * 0.25;
                Some(6)
            } else {
                None
            };
            let children = split_face(
                [0, 1, 2],
                std::array::from_fn(|slot| slots[slot].map(|_| slot as u32 + 3)),
                center,
                &local,
                query.symmetry_offset,
            );
            if children.iter().any(|&child| {
                triangle_quality(&local, child) < parent_quality.min(floor) * (1.0 - 1e-5)
            }) {
                for (a, b) in triangle_edges(triangle) {
                    if edges.contains(&(a.min(b), a.max(b))) {
                        rejected.insert(if query.symmetry_x {
                            symmetric_key(
                                mesh.positions[a as usize],
                                mesh.positions[b as usize],
                                query.symmetry_offset,
                            )
                        } else {
                            GroupKey::Edge(a.min(b), a.max(b))
                        });
                    }
                }
            }
        }
        if rejected.is_empty() {
            return Ok(());
        }
        selected.limited = true;
        selected.edges.retain(|&(a, b)| {
            let key = if query.symmetry_x {
                symmetric_key(
                    mesh.positions[a as usize],
                    mesh.positions[b as usize],
                    query.symmetry_offset,
                )
            } else {
                GroupKey::Edge(a, b)
            };
            !rejected.contains(&key)
        });
    }
    selected.edges.clear();
    selected.limited = true;
    Ok(())
}

fn triangle_edges([a, b, c]: [u32; 3]) -> [(u32, u32); 3] {
    [(a, b), (b, c), (c, a)]
}

struct FaceSplit {
    face: usize,
    edges: [Option<usize>; 3],
    center: Option<u32>,
    needs_center: bool,
}

fn refine_round(
    mesh: &mut Mesh,
    selected: &[(u32, u32)],
    candidate_faces: &[usize],
    query: PatchQuery,
    dirty: &mut Vec<usize>,
    mut ancestry: Option<&mut Vec<RefinementParents>>,
) -> Result<Vec<usize>, String> {
    let edge_indices: HashMap<_, _> = selected
        .iter()
        .copied()
        .enumerate()
        .map(|(index, edge)| (edge, index))
        .collect();
    let mut incidences = vec![(0_usize, false); selected.len()];
    let mut selected_faces = Vec::new();
    let mut unique_faces = HashSet::new();
    let mut central_count = 0;
    let mut growth = 0;
    for &face in candidate_faces {
        let triangle = mesh.triangles[face];
        let mut edge_slots = [None; 3];
        for (slot, (a, b)) in triangle_edges(triangle).into_iter().enumerate() {
            if let Some(&edge) = edge_indices.get(&(a.min(b), a.max(b))) {
                let (count, forward) = &mut incidences[edge];
                if *count == 2 {
                    return Err("Patch refinement refuses a selected non-manifold edge".into());
                }
                if *count == 1 && *forward == (a < b) {
                    return Err("Patch refinement requires consistent selected-edge winding".into());
                }
                *count += 1;
                *forward = a < b;
                edge_slots[slot] = Some(edge);
            }
        }
        let split_count = edge_slots.iter().flatten().count();
        if split_count == 0 {
            continue;
        }
        let mut sorted = triangle;
        sorted.sort_unstable();
        if !unique_faces.insert(sorted) {
            return Err("Patch refinement refuses duplicate selected faces".into());
        }
        if cross(&mesh.positions, triangle).length_squared() == 0.0 {
            return Err("Patch refinement refuses a degenerate selected face".into());
        }
        let needs_center = query.symmetry_x
            && split_count == 2
            && symmetric_corner(triangle, edge_slots, &mesh.positions, query.symmetry_offset)
                .is_some();
        selected_faces.push(FaceSplit {
            face,
            edges: edge_slots,
            center: None,
            needs_center,
        });
        growth += split_count;
        if needs_center {
            central_count += 1;
            growth += 2;
        }
    }
    let vertex_count = mesh.positions.len() + selected.len() + central_count;
    let triangle_count = mesh.triangles.len() + growth;
    if vertex_count > MAX_VERTICES || triangle_count > MAX_TRIANGLES {
        return Err(
            "Patch refinement exceeds the one-million vertex or two-million triangle limit".into(),
        );
    }
    let old_count = mesh.positions.len();
    let added_count = selected.len() + central_count;
    mesh.positions
        .try_reserve_exact(added_count)
        .map_err(|_| "Cannot allocate refinement positions")?;
    mesh.masks
        .try_reserve_exact(added_count)
        .map_err(|_| "Cannot allocate refinement masks")?;
    mesh.colors
        .try_reserve_exact(added_count)
        .map_err(|_| "Cannot allocate refinement colors")?;
    if let Some(materials) = &mut mesh.materials {
        materials
            .try_reserve_exact(added_count)
            .map_err(|_| "Cannot allocate refinement materials")?;
    }
    for &(a, b) in selected {
        let (a, b) = (a as usize, b as usize);
        let midpoint = (mesh.positions[a] + mesh.positions[b]) * 0.5;
        if midpoint == mesh.positions[a] || midpoint == mesh.positions[b] {
            return Err(
                "Patch refinement midpoint cannot be represented at this coordinate scale".into(),
            );
        }
        if let Some(parents) = ancestry.as_deref_mut() {
            parents.push(RefinementParents {
                vertex: mesh.positions.len(),
                parents: [a, b, b],
                weights: [0.5, 0.5, 0.0],
            });
        }
        mesh.positions.push(midpoint);
        mesh.masks.push((mesh.masks[a] + mesh.masks[b]) * 0.5);
        mesh.colors.push(std::array::from_fn(|channel| {
            (mesh.colors[a][channel] + mesh.colors[b][channel]) * 0.5
        }));
        if let Some(materials) = &mut mesh.materials {
            materials.push(std::array::from_fn(|channel| {
                (materials[a][channel] + materials[b][channel]) * 0.5
            }));
        }
    }
    for split in &mut selected_faces {
        let triangle = mesh.triangles[split.face];
        dirty.extend(triangle.map(|vertex| vertex as usize));
        if !split.needs_center {
            continue;
        }
        let [a, b, c] = symmetric_corner(
            triangle,
            split.edges,
            &mesh.positions,
            query.symmetry_offset,
        )
        .unwrap()
        .map(|vertex| vertex as usize);
        if let Some(parents) = ancestry.as_deref_mut() {
            parents.push(RefinementParents {
                vertex: mesh.positions.len(),
                parents: [a, c, b],
                weights: [0.375, 0.375, 0.25],
            });
        }
        split.center = Some(mesh.positions.len() as u32);
        mesh.positions
            .push((mesh.positions[a] + mesh.positions[c]) * 0.375 + mesh.positions[b] * 0.25);
        mesh.masks
            .push((mesh.masks[a] + mesh.masks[c]) * 0.375 + mesh.masks[b] * 0.25);
        mesh.colors.push(std::array::from_fn(|channel| {
            (mesh.colors[a][channel] + mesh.colors[c][channel]) * 0.375
                + mesh.colors[b][channel] * 0.25
        }));
        if let Some(materials) = &mut mesh.materials {
            materials.push(std::array::from_fn(|channel| {
                (materials[a][channel] + materials[c][channel]) * 0.375
                    + materials[b][channel] * 0.25
            }));
        }
    }
    dirty.extend(old_count..vertex_count);
    let mut triangles = Vec::new();
    triangles
        .try_reserve_exact(triangle_count)
        .map_err(|_| "Cannot allocate refinement triangles")?;
    let mut next_candidates = Vec::with_capacity(candidate_faces.len() + growth);
    let mut old_cursor = 0;
    let mut candidate_cursor = 0;
    for split in selected_faces {
        let output_cursor = triangles.len();
        triangles.extend_from_slice(&mesh.triangles[old_cursor..split.face]);
        while candidate_cursor < candidate_faces.len()
            && candidate_faces[candidate_cursor] < split.face
        {
            next_candidates.push(output_cursor + candidate_faces[candidate_cursor] - old_cursor);
            candidate_cursor += 1;
        }
        // The selected face is represented by its children below. Filtering
        // child bounds is safe because unchanged faces cannot become newly near
        // the brush during shape-preserving topology changes.
        debug_assert_eq!(candidate_faces.get(candidate_cursor), Some(&split.face));
        candidate_cursor += 1;
        let triangle = mesh.triangles[split.face];
        let slots = split
            .edges
            .map(|edge| edge.map(|edge| (old_count + edge) as u32));
        let children = split_face(
            triangle,
            slots,
            split.center,
            &mesh.positions,
            query.symmetry_offset,
        );
        let old_cross = cross(&mesh.positions, triangle);
        for child in children {
            if cross(&mesh.positions, child).dot(old_cross) <= 0.0 {
                return Err(
                    "Patch refinement would create a degenerate or reversed triangle".into(),
                );
            }
            if query.contains_face(&mesh.positions, child) {
                next_candidates.push(triangles.len());
            }
            triangles.push(child);
        }
        old_cursor = split.face + 1;
    }
    let output_cursor = triangles.len();
    triangles.extend_from_slice(&mesh.triangles[old_cursor..]);
    while candidate_cursor < candidate_faces.len() {
        next_candidates.push(output_cursor + candidate_faces[candidate_cursor] - old_cursor);
        candidate_cursor += 1;
    }
    mesh.triangles = triangles;
    mesh.clear_cache();
    Ok(next_candidates)
}

fn symmetric_corner<T>(
    triangle: [u32; 3],
    edges: [Option<T>; 3],
    positions: &[Vec3],
    offset: f32,
) -> Option<[u32; 3]> {
    let missing = edges.iter().position(Option::is_none)?;
    let r = (missing + 1) % 3;
    let corner = std::array::from_fn(|slot| triangle[(r + slot) % 3]);
    let [a, b, c] = corner.map(|vertex| positions[vertex as usize]);
    (b.x == offset && a.x != offset && a.x == 2.0 * offset - c.x && a.y == c.y && a.z == c.z)
        .then_some(corner)
}

fn split_face(
    triangle: [u32; 3],
    edges: [Option<u32>; 3],
    center: Option<u32>,
    positions: &[Vec3],
    offset: f32,
) -> Vec<[u32; 3]> {
    match edges.iter().flatten().count() {
        1 => {
            let r = edges.iter().position(Option::is_some).unwrap();
            let [a, b, c] = std::array::from_fn(|slot| triangle[(r + slot) % 3]);
            let midpoint = edges[r].unwrap();
            vec![[a, midpoint, c], [midpoint, b, c]]
        }
        2 => {
            let r = (0..3).find(|&r| edges[(r + 2) % 3].is_none()).unwrap();
            let [a, b, c] = std::array::from_fn(|slot| triangle[(r + slot) % 3]);
            let ab = edges[r].unwrap();
            let bc = edges[(r + 1) % 3].unwrap();
            if let Some(center) = center {
                return vec![
                    [b, bc, ab],
                    [a, ab, center],
                    [ab, bc, center],
                    [bc, c, center],
                    [c, a, center],
                ];
            }
            let first = vec![[b, bc, ab], [a, ab, bc], [a, bc, c]];
            let second = vec![[b, bc, ab], [a, ab, c], [ab, bc, c]];
            let minimum_quality = |triangles: &[[u32; 3]]| {
                triangles
                    .iter()
                    .map(|&triangle| quality(positions, triangle))
                    .fold(f64::INFINITY, f64::min)
            };
            let first_quality = minimum_quality(&first);
            let second_quality = minimum_quality(&second);
            let mirror_key = |a: u32, b: u32| match symmetric_key(
                positions[a as usize],
                positions[b as usize],
                offset,
            ) {
                GroupKey::Symmetric(key) => key,
                GroupKey::Edge(_, _) => unreachable!(),
            };
            // Equivalent diagonals must make the same geometric choice in a
            // reflected face, independent of the face's cyclic index order.
            let first_preferred = if (first_quality - second_quality).abs() <= 1e-12 {
                mirror_key(a, bc) <= mirror_key(ab, c)
            } else {
                first_quality > second_quality
            };
            if first_preferred {
                first
            } else {
                second
            }
        }
        3 => {
            let [a, b, c] = triangle;
            let [ab, bc, ca] = edges.map(Option::unwrap);
            vec![[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
        }
        _ => vec![triangle],
    }
}

fn cross(positions: &[Vec3], triangle: [u32; 3]) -> glam::DVec3 {
    let [a, b, c] = triangle.map(|vertex| positions[vertex as usize].as_dvec3());
    (b - a).cross(c - a)
}

fn quality(positions: &[Vec3], triangle: [u32; 3]) -> f64 {
    let [a, b, c] = triangle.map(|vertex| positions[vertex as usize].as_dvec3());
    (b - a).cross(c - a).length()
        / a.distance_squared(b)
            .max(b.distance_squared(c))
            .max(c.distance_squared(a))
}

fn standard_quality(positions: &[Vec3], triangle: [u32; 3]) -> f64 {
    let [a, b, c] = triangle.map(|vertex| positions[vertex as usize].as_dvec3());
    2.0 * 3.0_f64.sqrt() * (b - a).cross(c - a).length()
        / (a.distance_squared(b) + b.distance_squared(c) + c.distance_squared(a))
}

#[cfg(test)]
mod normal_regressions {
    use super::*;

    #[test]
    fn face_box_prefilter_keeps_exact_predicate_on_random_and_tangent_faces() {
        let mut state = 0x6ad4_9f31_u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            f64::from(state) / f64::from(u32::MAX)
        };
        for &scale in &[0.001_f32, 1.0, 1_000_000.0] {
            for &radius in &[0.000_001_f32, 0.18, 1.0, 100_000.0] {
                for &symmetry in &[false, true] {
                    let center = Vec3::new(0.19, -0.33, 0.57) * scale;
                    let query = PatchQuery::new(center, radius, symmetry, 0.23 * scale).unwrap();
                    let tangent = [
                        center + Vec3::new(radius, -scale * 0.25, 0.0),
                        center + Vec3::new(radius, scale * 0.25, 0.0),
                        center + Vec3::new(radius, 0.0, scale * 0.25),
                    ];
                    assert_eq!(
                        query.contains_face(&tangent, [0, 1, 2]),
                        query.contains_face_exact(&tangent, [0, 1, 2])
                    );
                    for _ in 0..1000 {
                        let positions: [Vec3; 3] = std::array::from_fn(|_| {
                            Vec3::new(
                                (next() * 2.0 - 1.0) as f32,
                                (next() * 2.0 - 1.0) as f32,
                                (next() * 2.0 - 1.0) as f32,
                            ) * scale
                        });
                        assert_eq!(
                            query.contains_face(&positions, [0, 1, 2]),
                            query.contains_face_exact(&positions, [0, 1, 2]),
                            "scale={scale} radius={radius} symmetry={symmetry} positions={positions:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "requires private OBRUSH_SPARK_STL path; run with --release for timing"]
    fn face_box_prefilter_keeps_spark_candidate_ids_and_measures_scan() {
        let source = std::fs::read(std::env::var("OBRUSH_SPARK_STL").unwrap()).unwrap();
        let mesh = Mesh::import_stl(&source).unwrap().mesh;
        let center = Vec3::new(-0.775_825, 3.085_721, 17.645_23);
        for &(radius, symmetry, offset) in
            &[(1.0, false, 0.0), (0.18, false, 0.0), (1.0, true, 0.23)]
        {
            let query = PatchQuery::new(center, radius, symmetry, offset).unwrap();
            let start = std::time::Instant::now();
            let exact: Vec<_> = mesh
                .triangles
                .iter()
                .enumerate()
                .filter_map(|(i, &t)| query.contains_face_exact(&mesh.positions, t).then_some(i))
                .collect();
            let exact_time = start.elapsed();
            let start = std::time::Instant::now();
            let fast: Vec<_> = mesh
                .triangles
                .iter()
                .enumerate()
                .filter_map(|(i, &t)| query.contains_face(&mesh.positions, t).then_some(i))
                .collect();
            let fast_time = start.elapsed();
            assert_eq!(fast, exact);
            eprintln!("Spark face scan radius={radius} symmetry={symmetry} offset={offset} exact_faces={} exact_ms={:.3} box_ms={:.3}", exact.len(), exact_time.as_secs_f64()*1000.0, fast_time.as_secs_f64()*1000.0);
        }
    }

    #[test]
    fn draw_refinement_refuses_a_partial_split_below_its_guard_quality_floor() {
        let authored = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0.19 0.19 0\nf 1 2 3\n").unwrap();
        let center = Vec3::new(0.095, 0.095, 0.0);
        let radius = 0.04;
        let mut ordinary = authored.snapshot();
        let (ordinary_report, _) = ordinary
            .refine_brush_patch_with_options(center, radius, false, 0.0, &[], false)
            .unwrap();
        assert_eq!(
            ordinary_report.vertices_added, 1,
            "old policy must admit AC"
        );
        let ordinary_child_min = ordinary
            .triangles
            .iter()
            .map(|&face| quality(&ordinary.positions, face))
            .fold(f64::INFINITY, f64::min);
        assert!(
            ordinary_child_min < 0.1,
            "the admitted child crosses the Draw guard floor"
        );
        assert!(quality(&authored.positions, authored.triangles[0]) > 0.12);

        let mut draw = authored.snapshot();
        let (report, _) = draw
            .refine_brush_patch_with_options(center, radius, false, 0.0, &[], true)
            .unwrap();
        assert_eq!(report.vertices_added, 0);
        assert!(report.limited);
        assert_eq!(draw.positions, authored.positions);
        assert_eq!(draw.triangles, authored.triangles);
    }

    fn assert_parent_contract(mesh: &Mesh, original: usize, parents: &[RefinementParents]) {
        assert_eq!(parents.len(), mesh.positions.len() - original);
        for (step, ancestry) in parents.iter().enumerate() {
            assert_eq!(ancestry.vertex, original + step);
            assert!(ancestry
                .parents
                .iter()
                .all(|&parent| parent < ancestry.vertex));
            assert!(ancestry
                .weights
                .iter()
                .all(|&weight| weight >= 0.0 && weight.is_finite()));
            assert_eq!(ancestry.weights.iter().sum::<f32>(), 1.0);
            let expected: glam::DVec3 = ancestry
                .parents
                .iter()
                .zip(ancestry.weights)
                .map(|(&parent, weight)| mesh.positions[parent].as_dvec3() * f64::from(weight))
                .sum();
            assert!(expected.distance(mesh.positions[ancestry.vertex].as_dvec3()) < 1e-7);
        }
    }

    #[test]
    fn ancestry_is_complete_and_ordered_across_rounds_and_interior_samples() {
        let mut mesh = Mesh::from_obj("v -1 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let (report, parents) = mesh
            .refine_brush_patch_with_options(Vec3::Y, 0.7, true, 0.0, &[], false)
            .unwrap();
        assert!(report.vertices_added > 0);
        assert_parent_contract(&mesh, 3, &parents);
        assert!(parents
            .iter()
            .any(|parent| parent.weights == [0.375, 0.375, 0.25]));
        assert!(parents
            .iter()
            .any(|parent| parent.parents.iter().any(|&index| index >= 3)));
    }

    #[test]
    fn shifted_plane_retains_exact_reflection_area_and_ancestry() {
        let mut mesh = Mesh::from_obj("v -0.75 0 0\nv 1.25 0 0\nv 0.25 1 0\nf 1 2 3\n").unwrap();
        let (report, parents) = mesh
            .refine_brush_patch_with_options(Vec3::new(0.25, 1.0, 0.0), 0.7, true, 0.25, &[], false)
            .unwrap();
        assert!(report.vertices_added > 0);
        assert_parent_contract(&mesh, 3, &parents);
        for p in &mesh.positions {
            assert!(mesh.positions.contains(&Vec3::new(0.5 - p.x, p.y, p.z)));
        }
        let area: f64 = mesh
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mesh.positions[i as usize].as_dvec3());
                let cross = (b - a).cross(c - a);
                assert!(cross.z > 0.0 && cross.x == 0.0 && cross.y == 0.0);
                cross.z * 0.5
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-8);
    }

    #[test]
    fn zero_offset_option_path_is_bit_exact_with_established_refinement() {
        let mut established = Mesh::sphere(3);
        let mut options = established.snapshot();
        let expected = established.refine_brush_patch(Vec3::Z, 0.1, true).unwrap();
        let (actual, parents) = options
            .refine_brush_patch_with_options(Vec3::Z, 0.1, true, 0.0, &[], false)
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(options.positions, established.positions);
        assert_eq!(options.triangles, established.triangles);
        assert_eq!(options.normals, established.normals);
        assert_eq!(options.colors, established.colors);
        assert_eq!(options.masks, established.masks);
        assert_parent_contract(&options, 642, &parents);
    }

    #[test]
    fn invalid_shifted_refinement_is_atomic_and_does_not_replace_arrays() {
        let mut mesh = Mesh::cube();
        let before = mesh.snapshot();
        let positions = mesh.positions.as_ptr();
        for offset in [f32::NAN, f32::INFINITY, 1_000_001.0] {
            assert!(mesh
                .refine_brush_patch_with_options(Vec3::ZERO, 1.0, true, offset, &[], false)
                .is_err());
            assert_eq!(mesh.positions.as_ptr(), positions);
            assert_eq!(mesh.positions, before.positions);
            assert_eq!(mesh.normals, before.normals);
            assert_eq!(mesh.triangles, before.triangles);
        }
    }

    #[test]
    fn dirty_normal_refinement_matches_full_rebuild_after_a_deferred_surface_edit() {
        let mut local = Mesh::sphere(5);
        let pole = local
            .positions
            .iter()
            .position(|position| position.distance(Vec3::Z) < 1e-6)
            .unwrap();
        local.positions[pole] += Vec3::Z * 0.04;
        let mut full = local.snapshot();
        let public_result = full.refine_brush_patch(Vec3::Z, 0.06, true).unwrap();
        let local_result = local
            .refine_brush_patch_with_dirty_normals(Vec3::Z, 0.06, true, &[pole])
            .unwrap();
        assert!(local_result.vertices_added > 0);
        assert_eq!(public_result, local_result);
        assert_eq!(local.positions, full.positions);
        assert_eq!(local.triangles, full.triangles);
        assert_eq!(local.masks, full.masks);
        assert_eq!(local.colors, full.colors);
        assert_eq!(local.normals, full.normals);
    }

    #[test]
    fn no_refinement_defers_normal_rebuild_to_the_draw_caller() {
        let mut local = Mesh::sphere(6);
        let pole = local
            .positions
            .iter()
            .position(|position| position.distance(Vec3::Z) < 1e-6)
            .unwrap();
        local.positions[pole] += Vec3::Z * 0.001;
        let before = local.normals.clone();
        let mut full = local.snapshot();
        full.recompute_normals();
        let result = local
            .refine_brush_patch_with_dirty_normals(Vec3::Z, 0.18, true, &[pole])
            .unwrap();
        assert_eq!(result.vertices_added, 0);
        assert_eq!(local.normals, before);
        local.recompute_normals_near_vertices(&[pole]);
        assert_eq!(local.normals, full.normals);
    }
}
