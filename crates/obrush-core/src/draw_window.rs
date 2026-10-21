//! Conservative local execution for ordinary high-density Draw dabs.
//! A failed coverage or stitch proof always runs the unchanged full-mesh path.

use crate::{
    refinement::outward_refinement_box, BrushKind, BrushResult, BrushSettings, Mesh, Vec3,
    MAX_TRIANGLES, MAX_VERTICES,
};

const MIN_GLOBAL_VERTICES: usize = 100_000;
const MAX_WINDOW_VERTICES: usize = 25_000;
const MAX_WINDOW_FACES: usize = 50_000;

fn collect_vertices(mesh: &Mesh, faces: &[usize]) -> Option<Vec<usize>> {
    let capacity = faces.len().checked_mul(3)?;
    let mut vertices = Vec::new();
    vertices.try_reserve_exact(capacity).ok()?;
    for &face in faces {
        vertices.extend(mesh.triangles[face].map(|v| v as usize));
    }
    vertices.sort_unstable();
    vertices.dedup();
    Some(vertices)
}

fn aabb_overlaps_box(
    mesh: &Mesh,
    face: usize,
    primary_box: (Vec3, Vec3),
    mirrored_box: (Vec3, Vec3),
    symmetry: bool,
) -> bool {
    let [a, b, c] = mesh.triangles[face].map(|v| mesh.positions[v as usize]);
    let axis_overlaps = |a: f32, b: f32, c: f32, low: f32, high: f32| {
        (a <= high || b <= high || c <= high) && (a >= low || b >= low || c >= low)
    };
    let overlaps = |(low, high): (Vec3, Vec3)| {
        axis_overlaps(a.x, b.x, c.x, low.x, high.x)
            && axis_overlaps(a.y, b.y, c.y, low.y, high.y)
            && axis_overlaps(a.z, b.z, c.z, low.z, high.z)
    };
    // Reflection changes only X. Reject the shared Y/Z slabs once before
    // testing either X interval, instead of visiting two complete boxes.
    if symmetry
        && primary_box.0.y == mirrored_box.0.y
        && primary_box.1.y == mirrored_box.1.y
        && primary_box.0.z == mirrored_box.0.z
        && primary_box.1.z == mirrored_box.1.z
    {
        return axis_overlaps(a.y, b.y, c.y, primary_box.0.y, primary_box.1.y)
            && axis_overlaps(a.z, b.z, c.z, primary_box.0.z, primary_box.1.z)
            && (axis_overlaps(a.x, b.x, c.x, primary_box.0.x, primary_box.1.x)
                || axis_overlaps(a.x, b.x, c.x, mirrored_box.0.x, mirrored_box.1.x));
    }
    overlaps(primary_box) || (symmetry && overlaps(mirrored_box))
}

fn intersect_sorted(left: &mut Vec<usize>, right: &[usize]) {
    left.retain(|face| right.binary_search(face).is_ok());
}

/// The source was fully validated, and the compact result is fully validated
/// before its rows are spliced. Old rows outside the window are copied without
/// alteration; every new row comes from the compact result. The splice maps
/// old local vertices to distinct original IDs and new local vertices to
/// distinct append-only IDs. Every emitted face is either an unchanged valid
/// source face or an injectively remapped valid compact face. Recheck the
/// properties that can change merely from concatenating those valid parts.
fn validate_spliced_counts(mesh: &Mesh) -> bool {
    let vertices = mesh.positions.len();
    vertices > 0
        && vertices <= MAX_VERTICES
        && !mesh.triangles.is_empty()
        && mesh.triangles.len() <= MAX_TRIANGLES
        && mesh.normals.len() == vertices
        && mesh.masks.len() == vertices
        && mesh.colors.len() == vertices
        && mesh.materials.as_ref().is_none_or(|m| m.len() == vertices)
        && mesh.surface.is_none()
        && mesh.approximate_bytes() <= crate::decode_limits::MAX_SCENE_STORAGE_BYTES
}

/// The source and compact result have already passed complete validation.
/// Publishing into the source is atomic only when every append fits existing
/// capacity: reserving one live channel before a later allocation failure
/// could leave the original mesh above its capacity-counted storage budget.
fn can_publish_splice_in_place(
    source: &Mesh,
    local: &Mesh,
    triangles: &Vec<[u32; 3]>,
    new_count: usize,
) -> bool {
    let Some(vertices) = source.positions.len().checked_add(new_count) else {
        return false;
    };
    if vertices == 0
        || vertices > MAX_VERTICES
        || triangles.is_empty()
        || triangles.len() > MAX_TRIANGLES
        || source.surface.is_some()
        || source.materials.is_some() != local.materials.is_some()
        || source.positions.capacity() < vertices
        || source.normals.capacity() < vertices
        || source.masks.capacity() < vertices
        || source.colors.capacity() < vertices
        || source
            .materials
            .as_ref()
            .is_some_and(|m| m.capacity() < vertices)
    {
        return false;
    }
    let capacities = [
        (source.positions.capacity(), std::mem::size_of::<Vec3>()),
        (source.normals.capacity(), std::mem::size_of::<Vec3>()),
        (triangles.capacity(), std::mem::size_of::<[u32; 3]>()),
        (source.masks.capacity(), std::mem::size_of::<f32>()),
        (source.colors.capacity(), std::mem::size_of::<[f32; 3]>()),
        (
            source.materials.as_ref().map_or(0, Vec::capacity),
            std::mem::size_of::<[f32; 2]>(),
        ),
    ];
    capacities
        .into_iter()
        .try_fold(std::mem::size_of::<Mesh>(), |bytes, (capacity, row)| {
            bytes.checked_add(capacity.checked_mul(row)?)
        })
        .is_some_and(|bytes| bytes <= crate::decode_limits::MAX_SCENE_STORAGE_BYTES)
}

/// Scan original faces in row order. This bounded local extraction avoids a
/// full-scene vertex-to-face CSR rebuild after every topology-changing dab.
fn star_faces(
    mesh: &Mesh,
    vertices: &[usize],
    marked: &mut Vec<usize>,
    flags: &mut [bool],
) -> Option<Vec<usize>> {
    for &vertex in marked.iter() {
        flags[vertex] = false;
    }
    marked.clear();
    marked.try_reserve_exact(vertices.len()).ok()?;
    for &vertex in vertices {
        if vertex >= flags.len() {
            return None;
        }
        if !flags[vertex] {
            flags[vertex] = true;
            marked.push(vertex);
        }
    }
    let mut faces = Vec::new();
    faces
        .try_reserve_exact(MAX_WINDOW_FACES.min(mesh.triangles.len()))
        .ok()?;
    for (face, triangle) in mesh.triangles.iter().enumerate() {
        if triangle.iter().any(|&vertex| flags[vertex as usize]) {
            if faces.len() == MAX_WINDOW_FACES {
                return None;
            }
            faces.push(face);
        }
    }
    Some(faces)
}

impl Mesh {
    /// Returns `None` on any uncertainty; the caller then executes the original
    /// whole-mesh dab. It commits only after complete local closure and a valid
    /// global candidate have been proved.
    pub(crate) fn try_local_draw_window(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
    ) -> Option<BrushResult> {
        self.try_local_draw_window_inner(center, normal, delta, settings, false)
    }

    /// The caller holds the exclusive mesh borrow from a validated scene pick.
    pub(crate) fn try_local_draw_window_from_validated_source(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
        _validated_source: crate::validated_draw_contact::ValidatedDrawSource,
    ) -> Option<BrushResult> {
        self.try_local_draw_window_inner(center, normal, delta, settings, true)
    }

    fn try_local_draw_window_inner(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
        source_validated: bool,
    ) -> Option<BrushResult> {
        if self.positions.len() < MIN_GLOBAL_VERTICES
            || settings.kind != BrushKind::Draw
            || !settings.draw_accumulate
            || settings.draw_clay
            || !settings.adaptive_detail
            || settings.invert
            || settings.alpha.is_some()
            || settings.hardness != 0.0
            || delta != Vec3::ZERO
            || !center.is_finite()
            || !normal.is_finite()
            || self.surface.is_some()
            || self.positions.len().checked_add(1_024)? > MAX_VERTICES
            || self.triangles.len().checked_add(4_096)? > MAX_TRIANGLES
        {
            return None;
        }
        let radius = settings.effective_radius()?;
        if !radius.is_finite() || radius <= 0.0 || (!source_validated && self.validate().is_err()) {
            return None;
        }
        if !settings.symmetry_offset.is_finite() || settings.symmetry_offset.abs() > 1_000_000.0 {
            return None;
        }
        let mirrored = Vec3::new(
            2.0 * settings.symmetry_offset - center.x,
            center.y,
            center.z,
        );
        if !mirrored.is_finite() {
            return None;
        }
        let primary_box = outward_refinement_box(center.as_dvec3(), f64::from(radius));
        let mirror_f64 = glam::DVec3::new(
            2.0 * f64::from(settings.symmetry_offset) - f64::from(center.x),
            f64::from(center.y),
            f64::from(center.z),
        );
        let mirror_exact_box = outward_refinement_box(mirror_f64, f64::from(radius));
        let mirror_kernel_box = outward_refinement_box(mirrored.as_dvec3(), f64::from(radius));
        let mirrored_box = (
            mirror_exact_box.0.min(mirror_kernel_box.0),
            mirror_exact_box.1.max(mirror_kernel_box.1),
        );
        let mut seed_faces = Vec::new();
        seed_faces
            .try_reserve_exact(MAX_WINDOW_FACES.min(self.triangles.len()))
            .ok()?;
        for face in 0..self.triangles.len() {
            if aabb_overlaps_box(self, face, primary_box, mirrored_box, settings.symmetry_x) {
                if seed_faces.len() == MAX_WINDOW_FACES {
                    return None;
                }
                seed_faces.push(face);
            }
        }
        if seed_faces.is_empty() {
            return None;
        }
        let mut flags = Vec::new();
        flags.try_reserve_exact(self.positions.len()).ok()?;
        flags.resize(self.positions.len(), false);
        let mut marked = Vec::new();
        let seed_vertices = collect_vertices(self, &seed_faces)?;
        if seed_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let first_faces = star_faces(self, &seed_vertices, &mut marked, &mut flags)?;
        if first_faces.len() > MAX_WINDOW_FACES {
            return None;
        }
        let first_vertices = collect_vertices(self, &first_faces)?;
        if first_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let second_faces = star_faces(self, &first_vertices, &mut marked, &mut flags)?;
        if second_faces.len() > MAX_WINDOW_FACES {
            return None;
        }
        let second_vertices = collect_vertices(self, &second_faces)?;
        if second_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let faces = star_faces(self, &second_vertices, &mut marked, &mut flags)?;
        if faces.len() > MAX_WINDOW_FACES {
            return None;
        }
        let vertices = collect_vertices(self, &faces)?;
        if vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let mut face_selected = Vec::new();
        face_selected.try_reserve_exact(self.triangles.len()).ok()?;
        face_selected.resize(self.triangles.len(), false);
        for &face in &faces {
            face_selected[face] = true;
        }
        let mut global_to_local = Vec::new();
        global_to_local
            .try_reserve_exact(self.positions.len())
            .ok()?;
        global_to_local.resize(self.positions.len(), usize::MAX);
        for (local, &global) in vertices.iter().enumerate() {
            global_to_local[global] = local;
        }
        // An isolated vertex can move under the old kernel despite having no
        // face in the box scan. Such a mesh uses the original route.
        let radius_squared = radius * radius;
        for (global, &point) in self.positions.iter().enumerate() {
            if (point.distance_squared(center) < radius_squared
                || (settings.symmetry_x && point.distance_squared(mirrored) < radius_squared))
                && global_to_local[global] == usize::MAX
            {
                return None;
            }
        }
        let mut local = self
            .draw_window_snapshot(&vertices, &faces, &global_to_local)
            .ok()?;
        let old_local_count = local.positions.len();
        let mut old_triangles = Vec::new();
        old_triangles
            .try_reserve_exact(local.triangles.len())
            .ok()?;
        old_triangles.extend_from_slice(&local.triangles);
        let (result, ancestry) =
            local.apply_brush_dab(center, normal, delta, settings, None, true, true);
        if result.changed_vertices == 0 {
            return None;
        }
        let new_count = local.positions.len().checked_sub(old_local_count)?;
        if new_count != ancestry.len() || new_count != result.refined_vertices {
            return None;
        }
        local.validate().ok()?;
        // Every old position touched by the local kernel needs its complete
        // global guard star. Every recomputed old normal needs its complete
        // incident-face sum in the original ascending face order.
        let mut closure = Vec::new();
        closure.try_reserve_exact(vertices.len()).ok()?;
        for (index, &global) in vertices.iter().enumerate() {
            if local.positions[index] != self.positions[global]
                || local.normals[index] != self.normals[global]
            {
                closure.push(global);
            }
        }
        for &vertex in &marked {
            flags[vertex] = false;
        }
        for global in closure {
            flags[global] = true;
        }
        for (face, triangle) in self.triangles.iter().enumerate() {
            if triangle.iter().any(|&vertex| flags[vertex as usize]) && !face_selected[face] {
                return None;
            }
        }
        if new_count == 0 && local.triangles == old_triangles {
            // The validated local dab changed no topology. All potentially
            // changed positions/normals have their complete source face star,
            // and every copied row came from the validated compact mesh.
            // Preserve the original face buffer and its capacity exactly.
            for (local_vertex, &global) in vertices.iter().enumerate() {
                self.positions[global] = local.positions[local_vertex];
                self.normals[global] = local.normals[local_vertex];
                self.masks[global] = local.masks[local_vertex];
                self.colors[global] = local.colors[local_vertex];
                if let (Some(dst), Some(src)) = (&mut self.materials, &local.materials) {
                    dst[global] = src[local_vertex];
                }
            }
            self.clear_cache();
            debug_assert!(self.validate().is_ok());
            return Some(result);
        }
        // Trace each generated vertex to the original local faces incident to
        // all positive-weight parents. A final child must have exactly one
        // original face. Ambiguous duplicate/nonmanifold topology falls back.
        let mut memberships = Vec::new();
        memberships.try_reserve_exact(old_local_count).ok()?;
        memberships.resize_with(old_local_count, Vec::new);
        for (face, triangle) in old_triangles.iter().enumerate() {
            for &vertex in triangle {
                memberships[vertex as usize].push(face);
            }
        }
        for membership in &mut memberships {
            membership.sort_unstable();
            membership.dedup();
        }
        for parents in ancestry {
            if parents.vertex != memberships.len() {
                return None;
            }
            let mut membership = None;
            for (parent, weight) in parents.parents.into_iter().zip(parents.weights) {
                if weight <= 0.0 {
                    continue;
                }
                let source = memberships.get(parent)?;
                match &mut membership {
                    Some(current) => intersect_sorted(current, source),
                    None => membership = Some(source.clone()),
                }
            }
            let membership = membership?;
            if membership.is_empty() {
                return None;
            }
            memberships.push(membership);
        }
        let mut children = Vec::new();
        children.try_reserve_exact(faces.len()).ok()?;
        children.resize_with(faces.len(), Vec::new);
        for triangle in &local.triangles {
            let mut origin = memberships.get(triangle[0] as usize)?.clone();
            intersect_sorted(&mut origin, memberships.get(triangle[1] as usize)?);
            intersect_sorted(&mut origin, memberships.get(triangle[2] as usize)?);
            if origin.len() != 1 {
                return None;
            }
            let global = triangle.map(|vertex| {
                let vertex = vertex as usize;
                if vertex < old_local_count {
                    vertices[vertex] as u32
                } else {
                    (self.positions.len() + vertex - old_local_count) as u32
                }
            });
            children[origin[0]].push(global);
        }
        if children.iter().any(Vec::is_empty) {
            return None;
        }
        let triangle_count = self
            .triangles
            .len()
            .checked_add(local.triangles.len())?
            .checked_sub(faces.len())?;
        if triangle_count > MAX_TRIANGLES {
            return None;
        }
        let mut triangles = Vec::new();
        triangles.try_reserve_exact(triangle_count).ok()?;
        let mut unchanged_start = 0;
        for (&face, replacement) in faces.iter().zip(&children) {
            // Preserve global face order, copying distant unchanged runs in
            // bulk instead of branching once for every face in a dense mesh.
            triangles.extend_from_slice(self.triangles.get(unchanged_start..face)?);
            triangles.extend_from_slice(replacement);
            unchanged_start = face.checked_add(1)?;
        }
        triangles.extend_from_slice(self.triangles.get(unchanged_start..)?);
        if faces.len() != children.len() || triangles.len() != triangle_count {
            return None;
        }
        if can_publish_splice_in_place(self, &local, &triangles, new_count) {
            // Everything that can fail has finished. Local rows and new face
            // indices are validated, the old-to-global mapping is injective,
            // every append fits a live channel, and the final capacity budget
            // was checked with the staged triangle allocation. No live Vec is
            // reserved or reallocated after the first source write.
            for (local_vertex, &global) in vertices.iter().enumerate() {
                self.positions[global] = local.positions[local_vertex];
                self.normals[global] = local.normals[local_vertex];
                self.masks[global] = local.masks[local_vertex];
                self.colors[global] = local.colors[local_vertex];
                if let (Some(dst), Some(src)) = (&mut self.materials, &local.materials) {
                    dst[global] = src[local_vertex];
                }
            }
            for local_vertex in old_local_count..local.positions.len() {
                self.positions.push(local.positions[local_vertex]);
                self.normals.push(local.normals[local_vertex]);
                self.masks.push(local.masks[local_vertex]);
                self.colors.push(local.colors[local_vertex]);
                if let (Some(dst), Some(src)) = (&mut self.materials, &local.materials) {
                    dst.push(src[local_vertex]);
                }
            }
            self.triangles = triangles;
            self.clear_cache();
            debug_assert!(self.validate().is_ok());
            return Some(result);
        }
        let mut candidate = self
            .draw_window_candidate_with_triangles(triangles, new_count)
            .ok()?;
        for (local_vertex, &global) in vertices.iter().enumerate() {
            candidate.positions[global] = local.positions[local_vertex];
            candidate.normals[global] = local.normals[local_vertex];
            candidate.masks[global] = local.masks[local_vertex];
            candidate.colors[global] = local.colors[local_vertex];
            if let (Some(dst), Some(src)) = (&mut candidate.materials, &local.materials) {
                dst[global] = src[local_vertex];
            }
        }
        for local_vertex in old_local_count..local.positions.len() {
            candidate.positions.push(local.positions[local_vertex]);
            candidate.normals.push(local.normals[local_vertex]);
            candidate.masks.push(local.masks[local_vertex]);
            candidate.colors.push(local.colors[local_vertex]);
            if let (Some(dst), Some(src)) = (&mut candidate.materials, &local.materials) {
                dst.push(src[local_vertex]);
            }
        }
        if !validate_spliced_counts(&candidate) {
            return None;
        }
        #[cfg(debug_assertions)]
        candidate.validate().ok()?;
        *self = candidate;
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_axis_rejection_matches_full_triangle_aabb_for_finite_inputs() {
        let mut mesh = Mesh::new(vec![Vec3::ZERO; 3], vec![[0, 1, 2]]);
        let mut state = 612_129_u32;
        let mut random = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state as f64 / f64::from(u32::MAX) * 20.0 - 10.0) as f32
        };
        for _ in 0..10_000 {
            for point in &mut mesh.positions {
                *point = Vec3::new(random(), random(), random());
            }
            let p = Vec3::new(random(), random(), random());
            let q = Vec3::new(random(), random(), random());
            let first = (p.min(q), p.max(q));
            let p = Vec3::new(random(), random(), random());
            let q = Vec3::new(random(), random(), random());
            let second = (p.min(q), p.max(q));
            let [a, b, c] = [mesh.positions[0], mesh.positions[1], mesh.positions[2]];
            let minimum = a.min(b).min(c);
            let maximum = a.max(b).max(c);
            let old =
                |(low, high): (Vec3, Vec3)| minimum.cmple(high).all() && maximum.cmpge(low).all();
            for mirrored in [false, true] {
                assert_eq!(
                    aabb_overlaps_box(&mesh, 0, first, second, mirrored),
                    old(first) || (mirrored && old(second))
                );
            }
            let shared_slabs = (
                Vec3::new(second.0.x, first.0.y, first.0.z),
                Vec3::new(second.1.x, first.1.y, first.1.z),
            );
            assert_eq!(
                aabb_overlaps_box(&mesh, 0, first, shared_slabs, true),
                old(first) || old(shared_slabs)
            );
        }
        let primary = (Vec3::ZERO, Vec3::ONE);
        let mirrored = (Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));
        for (x, primary_hit, mirror_hit) in [
            (0.0, true, false),
            (1.0, true, false),
            (1.0_f32.next_up(), false, false),
            (2.0, false, true),
            (3.0, false, true),
            (3.0_f32.next_up(), false, false),
        ] {
            mesh.positions = vec![
                Vec3::new(x, 0.2, 0.2),
                Vec3::new(x, 0.8, 0.2),
                Vec3::new(x, 0.2, 0.8),
            ];
            assert_eq!(
                aabb_overlaps_box(&mesh, 0, primary, mirrored, false),
                primary_hit
            );
            assert_eq!(
                aabb_overlaps_box(&mesh, 0, primary, mirrored, true),
                primary_hit || mirror_hit
            );
        }
    }

    #[test]
    fn reflected_f64_refinement_contact_survives_f32_kernel_rounding() {
        // Doubling the offset and subtracting the center in f32 shifts the
        // reflected contact by 0.03125 here. A 0.04-radius face is then inside
        // the refinement query but outside the rounded brush query.
        let center = Vec3::new(f32::from_bits(0xc935_51f7), 0.0, 0.0);
        let offset = f32::from_bits(0x4716_a2ac);
        let radius = 0.04_f32;
        let mirrored_f64 = 2.0 * f64::from(offset) - f64::from(center.x);
        let mirrored_f32 = 2.0 * offset - center.x;
        assert_eq!(mirrored_f64, 819_812.781_25);
        assert_eq!(mirrored_f32, 819_812.75);
        let exact_box =
            outward_refinement_box(glam::DVec3::new(mirrored_f64, 0.0, 0.0), f64::from(radius));
        let rounded_box = outward_refinement_box(
            Vec3::new(mirrored_f32, 0.0, 0.0).as_dvec3(),
            f64::from(radius),
        );
        let contact = Mesh::new(
            vec![
                Vec3::new(f32::from_bits(0x4948_264d), -0.01, 0.0),
                Vec3::new(f32::from_bits(0x4948_264d), 0.01, 0.0),
                Vec3::new(f32::from_bits(0x4948_264d), 0.0, 0.01),
            ],
            vec![[0, 1, 2]],
        );
        assert!(f64::from(contact.positions[0].x) > f64::from(mirrored_f32) + f64::from(radius));
        assert!(f64::from(contact.positions[0].x) <= mirrored_f64 + f64::from(radius));
        assert!(aabb_overlaps_box(&contact, 0, exact_box, rounded_box, true));
    }

    #[test]
    fn authored_high_density_draw_window_matches_full_mesh_words() {
        let mut base = Mesh::sphere(7);
        assert!(base.positions.len() >= MIN_GLOBAL_VERTICES);
        for (index, color) in base.colors.iter_mut().enumerate() {
            if index % 7 == 0 {
                *color = [0.2, 0.4, 0.8];
            }
        }
        for (index, mask) in base.masks.iter_mut().enumerate() {
            if index % 97 == 0 {
                *mask = 0.25;
            }
        }
        base.materials = Some(vec![[0.3, 0.7]; base.positions.len()]);
        base.validate().unwrap();
        for (radius, symmetry_x) in [(0.18, false), (0.18, true)] {
            let settings = BrushSettings {
                radius,
                strength: 0.35,
                symmetry_x,
                ..Default::default()
            };
            let center = Vec3::new(0.0, 0.0, 1.0);
            let normal = Vec3::Z;
            let mut full = base.snapshot();
            let (full_result, _) =
                full.apply_brush_dab(center, normal, Vec3::ZERO, &settings, None, true, true);
            let mut local = base.snapshot();
            let local_result = local.try_local_draw_window(center, normal, Vec3::ZERO, &settings);
            assert!(full_result.changed_vertices > 0);
            assert_eq!(Some(full_result), local_result);
            local.validate().unwrap();
            assert_eq!(
                serde_json::to_vec(&full).unwrap(),
                serde_json::to_vec(&local).unwrap()
            );
        }
    }

    #[test]
    fn window_requires_valid_source_and_splice_lengths() {
        let settings = BrushSettings::default();
        let mut invalid = Mesh::sphere(7);
        let last = invalid.positions.len() - 1;
        invalid.positions[last] = Vec3::splat(f32::NAN);
        assert!(invalid
            .try_local_draw_window(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings)
            .is_none());
        let mut malformed = Mesh::sphere(0);
        malformed.normals.pop();
        assert!(!validate_spliced_counts(&malformed));
        malformed.normals.push(Vec3::Z);
        malformed.materials = Some(Vec::new());
        assert!(!validate_spliced_counts(&malformed));
    }
}

#[cfg(test)]
#[path = "draw_window_independent_tests.rs"]
mod independent_tests;

#[cfg(test)]
#[path = "draw_window_inplace_tests.rs"]
mod inplace_tests;
