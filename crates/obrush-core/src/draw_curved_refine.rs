//! Curvature-aware, bounded Draw refinement of an already accepted surface.
use crate::{Mesh, Vec3};
use glam::DVec3;
use std::collections::{HashMap, HashSet};

const MAX_CURVED_POINTS: usize = 64;
const MAX_VALENCE: usize = 64;
const MAX_PAIR_CHECKS: usize = 128;
const MAX_OFFSET_RADIUS: f64 = 0.01;
const SMOOTH_FOLD_COS: f64 = 0.8660254037844386; // 30 degrees
const DRAW_FOLD_COS: f64 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CurvedDrawRefinement {
    pub vertices_added: usize,
    pub triangles_added: usize,
    pub vertices_curved: usize,
    pub limited: bool,
}

fn key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
fn cross(p: &[Vec3], t: [u32; 3]) -> DVec3 {
    let [a, b, c] = t.map(|v| p[v as usize].as_dvec3());
    (b - a).cross(c - a)
}
fn quality(p: &[Vec3], t: [u32; 3]) -> f64 {
    let [a, b, c] = t.map(|v| p[v as usize].as_dvec3());
    let longest = a
        .distance_squared(b)
        .max(b.distance_squared(c))
        .max(c.distance_squared(a));
    if longest <= 0.0 {
        return 0.0;
    }
    (b - a).cross(c - a).length() / longest
}
fn normal(p: &[Vec3], t: [u32; 3]) -> Option<DVec3> {
    let c = cross(p, t);
    (c.is_finite() && c.length_squared() > 1e-24).then(|| c.normalize())
}
fn bits(p: Vec3) -> [u32; 3] {
    [p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() })
}
fn reflect(p: Vec3, offset: f32) -> Vec3 {
    Vec3::new(2.0 * offset - p.x, p.y, p.z)
}
fn same_attrs(mesh: &Mesh, v: [usize; 4]) -> bool {
    let x = v[0];
    v.into_iter().all(|i| {
        mesh.mask_influence_f64(i) == 1.0
            && mesh.masks[i] == mesh.masks[x]
            && mesh.colors[i] == mesh.colors[x]
            && mesh.materials.as_ref().is_none_or(|m| m[i] == m[x])
    })
}
fn admit(mesh: &mut Mesh, changes: &[(usize, Vec3)]) -> bool {
    let vertices: Vec<_> = changes.iter().map(|&(v, _)| v).collect();
    let faces = mesh.cached_faces_for_vertices(&vertices);
    if faces.len() > MAX_VALENCE * changes.len() {
        return false;
    }
    let pairs = mesh.cached_pairs_for_faces(&faces);
    if pairs.len() > MAX_PAIR_CHECKS * changes.len() {
        return false;
    }
    let old_faces: Vec<_> = faces
        .iter()
        .map(|&f| {
            let t = mesh.triangles[f];
            (t, cross(&mesh.positions, t), quality(&mesh.positions, t))
        })
        .collect();
    let old_folds: Vec<_> = pairs
        .iter()
        .map(|&pair| {
            let [a, b] = mesh.cached_face_pairs()[pair];
            let n0 = normal(&mesh.positions, mesh.triangles[a as usize]).unwrap_or(DVec3::ZERO);
            let n1 = normal(&mesh.positions, mesh.triangles[b as usize]).unwrap_or(DVec3::ZERO);
            (a, b, n0.dot(n1))
        })
        .collect();
    let originals: Vec<_> = changes.iter().map(|&(v, _)| mesh.positions[v]).collect();
    for &(v, target) in changes {
        mesh.positions[v] = target;
    }
    let safe_faces = old_faces.iter().all(|&(t, old_cross, old_q)| {
        let new_cross = cross(&mesh.positions, t);
        let new_q = quality(&mesh.positions, t);
        new_cross.is_finite()
            && new_cross.dot(old_cross) >= old_cross.length_squared() * 0.05
            && new_q >= old_q.min(0.1)
            && new_q > 0.0
    });
    let safe_folds = old_folds.iter().all(|&(a, b, old_cos)| {
        let n0 = normal(&mesh.positions, mesh.triangles[a as usize]).unwrap_or(DVec3::ZERO);
        let n1 = normal(&mesh.positions, mesh.triangles[b as usize]).unwrap_or(DVec3::ZERO);
        n0 != DVec3::ZERO && n1 != DVec3::ZERO && n0.dot(n1) >= old_cos.min(DRAW_FOLD_COS)
    });
    if safe_faces && safe_folds {
        true
    } else {
        for (&(v, _), old) in changes.iter().zip(originals) {
            mesh.positions[v] = old;
        }
        false
    }
}

impl Mesh {
    /// Split with the existing Draw child-quality policy, then curve only
    /// first-generation interior midpoints. The tangent-plane curvature offset
    /// is capped at 1% of radius; every original vertex stays unchanged.
    /// An exact mirrored midpoint pair moves together. An imported asymmetric
    /// point without an exact counterpart may move independently.
    pub fn refine_curved_draw_patch(
        &mut self,
        center: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
    ) -> Result<CurvedDrawRefinement, String> {
        if !center.is_finite()
            || !radius.is_finite()
            || radius <= 0.0
            || !symmetry_offset.is_finite()
        {
            return Err("Curved Draw refinement needs finite contact and radius".into());
        }
        self.validate()?;
        self.require_plain_topology("Curved Draw refinement")?;
        let original_count = self.positions.len();
        let mut candidate = self.try_snapshot()?;
        let (refined, ancestry) = candidate.refine_brush_patch_with_options(
            center,
            radius,
            symmetry_x,
            symmetry_offset,
            &[],
            true,
        )?;
        if refined.vertices_added == 0 {
            return Ok(CurvedDrawRefinement {
                limited: refined.limited,
                ..Default::default()
            });
        }
        let wanted: HashSet<_> = ancestry
            .iter()
            .filter(|p| {
                p.weights == [0.5, 0.5, 0.0]
                    && p.parents[0] < original_count
                    && p.parents[1] < original_count
            })
            .map(|p| key(p.parents[0], p.parents[1]))
            .collect();
        let mut opposites: HashMap<(usize, usize), Vec<(usize, bool)>> = HashMap::new();
        for &t in &self.triangles {
            for (a, b, c) in [(t[0], t[1], t[2]), (t[1], t[2], t[0]), (t[2], t[0], t[1])] {
                let k = key(a as usize, b as usize);
                if wanted.contains(&k) {
                    opposites
                        .entry(k)
                        .or_default()
                        .push((c as usize, a as usize == k.0));
                }
            }
        }
        let mut targets = HashMap::new();
        let cap = MAX_OFFSET_RADIUS * f64::from(radius);
        for p in ancestry {
            if p.weights != [0.5, 0.5, 0.0]
                || p.parents[0] >= original_count
                || p.parents[1] >= original_count
            {
                continue;
            }
            let [a, b, _] = p.parents;
            let Some(opp) = opposites.get(&key(a, b)) else {
                continue;
            };
            if opp.len() != 2 || opp[0].1 == opp[1].1 {
                continue;
            }
            let [oa, ob] = [opp[0].0, opp[1].0];
            if !same_attrs(self, [a, b, oa, ob]) {
                continue;
            }
            let na = normal(&self.positions, [a as u32, b as u32, oa as u32]);
            let nb = normal(&self.positions, [b as u32, a as u32, ob as u32]);
            if na.zip(nb).is_none_or(|(x, y)| x.dot(y) < SMOOTH_FOLD_COS) {
                continue;
            }
            let old = candidate.positions[p.vertex];
            let mirrored_center = reflect(center, symmetry_offset);
            if old.distance_squared(center) > radius * radius
                && (!symmetry_x || old.distance_squared(mirrored_center) > radius * radius)
            {
                continue;
            }
            // Two endpoint tangent planes estimate signed curvature at the
            // edge midpoint. A plane gives zero; a convex sphere lifts above
            // its linear chord without changing existing vertex positions.
            let pa = self.positions[a].as_dvec3();
            let pb = self.positions[b].as_dvec3();
            let na = self.normals[a].as_dvec3().normalize_or_zero();
            let nb = self.normals[b].as_dvec3().normalize_or_zero();
            let direction = (na + nb).normalize_or_zero();
            if direction == DVec3::ZERO || na.dot(nb) < SMOOTH_FOLD_COS {
                continue;
            }
            let midpoint = (pa + pb) * 0.5;
            let height = -0.5 * ((midpoint - pa).dot(na) + (midpoint - pb).dot(nb));
            let delta = direction * height;
            if !delta.is_finite() || delta.length_squared() < 1e-20 {
                continue;
            }
            let shift = delta.clamp_length_max(cap * 0.999).as_vec3();
            let target = old + shift;
            if target == old
                || !target.is_finite()
                || target.as_dvec3().distance(old.as_dvec3()) > cap
            {
                continue;
            }
            targets.insert(p.vertex, target);
        }
        if targets.is_empty() {
            *self = candidate;
            return Ok(CurvedDrawRefinement {
                vertices_added: refined.vertices_added,
                triangles_added: refined.triangles_added,
                vertices_curved: 0,
                limited: refined.limited,
            });
        }
        candidate.ensure_adjacency();
        let mut mirror_index = HashMap::new();
        if symmetry_x {
            for v in original_count..candidate.positions.len() {
                mirror_index.insert(bits(candidate.positions[v]), v);
            }
        }
        let mut indices: Vec<_> = targets.keys().copied().collect();
        indices.sort_unstable();
        indices.truncate(MAX_CURVED_POINTS);
        let mut done = HashSet::new();
        let mut curved = 0;
        let mut dirty = Vec::new();
        for i in indices {
            if !done.insert(i) {
                continue;
            }
            let mut changes = vec![(i, targets[&i])];
            if symmetry_x {
                let old = candidate.positions[i];
                if old.x == symmetry_offset {
                    changes[0].1.x = symmetry_offset;
                } else if let Some(&j) = mirror_index.get(&bits(reflect(old, symmetry_offset))) {
                    if j != i {
                        done.insert(j);
                        let Some(&other) = targets.get(&j) else {
                            continue;
                        };
                        let average = (targets[&i] + reflect(other, symmetry_offset)) * 0.5;
                        changes = vec![(i, average), (j, reflect(average, symmetry_offset))];
                    }
                }
            }
            if changes
                .iter()
                .any(|&(v, p)| p.as_dvec3().distance(candidate.positions[v].as_dvec3()) > cap)
            {
                continue;
            }
            if admit(&mut candidate, &changes) {
                curved += changes.len();
                dirty.extend(changes.iter().map(|&(vertex, _)| vertex));
            }
        }
        if curved > 0 {
            candidate.recompute_normals_near_vertices_cached(&dirty);
            candidate.validate()?;
        }
        *self = candidate;
        Ok(CurvedDrawRefinement {
            vertices_added: refined.vertices_added,
            triangles_added: refined.triangles_added,
            vertices_curved: curved,
            limited: refined.limited || targets.len() > MAX_CURVED_POINTS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curvature_lifts_new_sphere_points_toward_reference_sphere() {
        let base = Mesh::sphere(2);
        let mut linear = base.snapshot();
        let mut curved = base.snapshot();
        let center = Vec3::Z;
        let (ordinary, _) = linear
            .refine_brush_patch_with_options(center, 0.8, false, 0.0, &[], true)
            .unwrap();
        let report = curved
            .refine_curved_draw_patch(center, 0.8, false, 0.0)
            .unwrap();
        assert!(ordinary.vertices_added > 0);
        assert_eq!(report.vertices_added, ordinary.vertices_added);
        assert_eq!(linear.triangles, curved.triangles);
        assert!(
            report.vertices_curved > 0,
            "expected accepted curvature offsets: {report:?}"
        );
        let start = base.positions.len();
        let error = |mesh: &Mesh| {
            mesh.positions[start..]
                .iter()
                .map(|p| (p.length() - 1.0).abs() as f64)
                .sum::<f64>()
        };
        assert!(
            error(&curved) < error(&linear),
            "linear={}, curved={}",
            error(&linear),
            error(&curved)
        );
        assert_eq!(&curved.positions[..start], base.positions.as_slice());
        let bound = 0.8 * MAX_OFFSET_RADIUS;
        for (&linear, &curved) in linear.positions[start..]
            .iter()
            .zip(&curved.positions[start..])
        {
            assert!(linear.as_dvec3().distance(curved.as_dvec3()) <= bound);
        }
        let mut full_normals = curved.snapshot();
        full_normals.recompute_normals();
        assert_eq!(curved.normals, full_normals.normals);
        curved.validate().unwrap();
    }

    #[test]
    fn flat_patch_keeps_linear_new_points() {
        let base =
            Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3\nf 1 3 4\n").unwrap();
        let mut mesh = base.snapshot();
        let report = mesh
            .refine_curved_draw_patch(Vec3::ZERO, 1.0, false, 0.0)
            .unwrap();
        assert!(report.vertices_added > 0);
        assert_eq!(report.vertices_curved, 0);
        assert!(mesh.positions.iter().all(|p| p.z == 0.0));
    }

    #[test]
    fn exact_reflected_new_points_remain_reflected_after_curving() {
        let base = Mesh::sphere(2);
        let mut linear = base.snapshot();
        let mut curved = base.snapshot();
        let center = Vec3::new(0.4, 0.0, 0.8).normalize();
        linear
            .refine_brush_patch_with_options(center, 0.8, true, 0.0, &[], true)
            .unwrap();
        let report = curved
            .refine_curved_draw_patch(center, 0.8, true, 0.0)
            .unwrap();
        assert_eq!(linear.triangles, curved.triangles);
        let before = base.positions.len();
        let lookup: HashMap<_, _> = linear.positions[before..]
            .iter()
            .enumerate()
            .map(|(i, &p)| (bits(p), before + i))
            .collect();
        let mut pairs = 0;
        for i in before..linear.positions.len() {
            if let Some(&j) = lookup.get(&bits(reflect(linear.positions[i], 0.0))) {
                assert_eq!(curved.positions[j], reflect(curved.positions[i], 0.0));
                pairs += 1;
            }
        }
        assert!(pairs > 0);
        assert!(report.vertices_curved <= report.vertices_added);
    }
}
