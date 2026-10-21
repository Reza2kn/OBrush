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
    let minimum = a.min(b).min(c);
    let maximum = a.max(b).max(c);
    let overlaps =
        |(low, high): (Vec3, Vec3)| minimum.cmple(high).all() && maximum.cmpge(low).all();
    overlaps(primary_box) || (symmetry && overlaps(mirrored_box))
}

fn intersect_sorted(left: &mut Vec<usize>, right: &[usize]) {
    left.retain(|face| right.binary_search(face).is_ok());
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
        if !radius.is_finite() || radius <= 0.0 || self.validate().is_err() {
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
        self.ensure_local_incidence();
        let seed_vertices = collect_vertices(self, &seed_faces)?;
        if seed_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let first_faces = self.cached_faces_for_vertices(&seed_vertices);
        if first_faces.len() > MAX_WINDOW_FACES {
            return None;
        }
        let first_vertices = collect_vertices(self, &first_faces)?;
        if first_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let second_faces = self.cached_faces_for_vertices(&first_vertices);
        if second_faces.len() > MAX_WINDOW_FACES {
            return None;
        }
        let second_vertices = collect_vertices(self, &second_faces)?;
        if second_vertices.len() > MAX_WINDOW_VERTICES {
            return None;
        }
        let faces = self.cached_faces_for_vertices(&second_vertices);
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
        for global in closure {
            if self
                .cached_faces_for_vertices(&[global])
                .iter()
                .any(|&face| !face_selected[face])
            {
                return None;
            }
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
        let mut candidate = self.try_snapshot().ok()?;
        candidate.positions.try_reserve_exact(new_count).ok()?;
        candidate.normals.try_reserve_exact(new_count).ok()?;
        candidate.masks.try_reserve_exact(new_count).ok()?;
        candidate.colors.try_reserve_exact(new_count).ok()?;
        if let Some(materials) = &mut candidate.materials {
            materials.try_reserve_exact(new_count).ok()?;
        }
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
        let mut selected = 0;
        for face in 0..self.triangles.len() {
            if faces.get(selected) == Some(&face) {
                triangles.extend_from_slice(&children[selected]);
                selected += 1;
            } else {
                triangles.push(self.triangles[face]);
            }
        }
        if selected != faces.len() || triangles.len() != triangle_count {
            return None;
        }
        candidate.triangles = triangles;
        candidate.clear_cache();
        candidate.validate().ok()?;
        *self = candidate;
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflected_f64_refinement_contact_survives_f32_kernel_rounding() {
        // Doubling the offset and subtracting the center in f32 shifts the
        // reflected contact by 0.03125 here. A 0.04-radius face is then inside
        // the refinement query but outside the rounded brush query.
        let center = Vec3::new(-742_687.437_5, 0.0, 0.0);
        let offset = 38_562.671_875_f32;
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
                Vec3::new(819_812.812_5, -0.01, 0.0),
                Vec3::new(819_812.812_5, 0.01, 0.0),
                Vec3::new(819_812.812_5, 0.0, 0.01),
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
            assert_eq!(
                serde_json::to_vec(&full).unwrap(),
                serde_json::to_vec(&local).unwrap()
            );
        }
    }
}

#[cfg(test)]
#[path = "draw_window_independent_tests.rs"]
mod independent_tests;
