//! Analytic Jacobian adapter for the existing Draw SLSQP constraint rows.
//!
//! The constraint *values* remain in `draw_slsqp_row`; this module only fills
//! the corresponding Jacobian, and the original rounded guard remains the
//! final authority for accepting a candidate.

use super::{DrawContactAnchor, FaceGuard, FoldGuard};
use crate::Vec3;
use glam::DVec3;
use slsqp_rssl::Mat;

#[path = "draw_cage_jacobian.rs"]
mod analytic;

fn quality_gradient(points: [DVec3; 3]) -> Option<[DVec3; 3]> {
    let edges = [
        points[0].distance_squared(points[1]),
        points[1].distance_squared(points[2]),
        points[2].distance_squared(points[0]),
    ];
    let longest = edges.into_iter().fold(0.0_f64, f64::max);
    if longest <= 0.0 || !longest.is_finite() {
        return None;
    }
    let near_longest = edges
        .iter()
        .filter(|&&length| longest - length <= longest * 1e-7)
        .count();
    if near_longest < 2 {
        return analytic::triangle_quality(points).map(|result| result.gradient);
    }
    // A longest-edge tie is nondifferentiable. Match the original bounded
    // three-corner fallback instead of imposing an arbitrary branch slope.
    let metric = |p: [DVec3; 3]| {
        (p[1] - p[0]).cross(p[2] - p[0]).length()
            / p[0]
                .distance_squared(p[1])
                .max(p[1].distance_squared(p[2]))
                .max(p[2].distance_squared(p[0]))
    };
    let step = longest.sqrt() * 1e-6;
    let mut local = points;
    let mut gradient = [DVec3::ZERO; 3];
    for corner in 0..3 {
        for axis in 0..3 {
            local[corner][axis] += step;
            let plus = metric(local);
            local[corner][axis] -= 2.0 * step;
            let minus = metric(local);
            local[corner][axis] += step;
            gradient[corner][axis] = (plus - minus) / (2.0 * step);
        }
    }
    gradient
        .iter()
        .all(|value| value.is_finite())
        .then_some(gradient)
}

pub(super) struct DrawCageJacobianContext<'a> {
    pub old: &'a [Vec3],
    pub baseline: &'a [Vec3],
    pub faces: &'a [&'a FaceGuard],
    pub folds: &'a [&'a FoldGuard],
    pub ids: &'a [usize],
    pub mirrors: Option<&'a [Option<usize>]>,
    pub mirror_ids: &'a [usize],
    pub symmetry_offset: f32,
    pub anchor: DrawContactAnchor,
    pub center: Vec3,
    pub camera_axis: Vec3,
    pub radius: f32,
    pub scale: f64,
}

impl DrawCageJacobianContext<'_> {
    pub fn rows(&self) -> usize {
        self.faces.len() * 2 + self.folds.len() + self.ids.len() + self.mirror_ids.len() + 4
    }

    fn point(&self, id: usize, x: &[f64]) -> DVec3 {
        if let Ok(slot) = self.ids.binary_search(&id) {
            let mut p = self.old[id].as_dvec3()
                + DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * self.scale;
            if self.old[id].x == self.symmetry_offset {
                p.x = f64::from(self.symmetry_offset);
            }
            p
        } else if let Some((slot, _)) = self.mirrors.and_then(|items| {
            items
                .iter()
                .enumerate()
                .find(|(_, other)| **other == Some(id))
        }) {
            let p = self.old[self.ids[slot]].as_dvec3()
                + DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * self.scale;
            DVec3::new(2.0 * f64::from(self.symmetry_offset) - p.x, p.y, p.z)
        } else {
            self.baseline[id].as_dvec3()
        }
    }

    fn add_gradient(&self, jac: &mut Mat, row: usize, id: usize, gradient: DVec3) {
        let (slot, sign) = if let Ok(slot) = self.ids.binary_search(&id) {
            (slot, 1.0)
        } else if let Some((slot, _)) = self.mirrors.and_then(|items| {
            items
                .iter()
                .enumerate()
                .find(|(_, other)| **other == Some(id))
        }) {
            (slot, -1.0)
        } else {
            return;
        };
        let scale = self.scale;
        if self.old[self.ids[slot]].x != self.symmetry_offset {
            jac[(row, 3 * slot)] += gradient.x * scale * sign;
        }
        jac[(row, 3 * slot + 1)] += gradient.y * scale;
        jac[(row, 3 * slot + 2)] += gradient.z * scale;
    }

    pub fn fill(&self, x: &[f64], jac: &mut Mat) {
        if x.len() != self.ids.len() * 3
            || jac.rows() < self.rows()
            || jac.cols() < x.len() + 1
            || x.iter().any(|value| !value.is_finite())
        {
            return;
        }
        for (row, face) in self.faces.iter().enumerate() {
            let points = face.indices.map(|id| self.point(id as usize, x));
            if let Some(gradients) = quality_gradient(points) {
                for (id, gradient) in face.indices.into_iter().zip(gradients) {
                    self.add_gradient(jac, row, id as usize, gradient);
                }
            }
            let orientation_row = self.faces.len() + row;
            if let Some(result) = analytic::orientation_projection(points, face.cross) {
                let norm = face.cross.length_squared();
                for (id, gradient) in face.indices.into_iter().zip(result.gradient) {
                    self.add_gradient(jac, orientation_row, id as usize, gradient / norm);
                }
            }
        }
        let fold_start = self.faces.len() * 2;
        for (index, fold) in self.folds.iter().enumerate() {
            let points = fold
                .indices
                .map(|ids| ids.map(|id| self.point(id as usize, x)));
            let crosses = points.map(|[a, b, c]| (b - a).cross(c - a));
            let lengths = crosses.map(DVec3::length);
            if lengths
                .iter()
                .any(|length| *length == 0.0 || !length.is_finite())
            {
                continue;
            }
            let unit = [crosses[0] / lengths[0], crosses[1] / lengths[1]];
            let cosine = unit[0].dot(unit[1]);
            let weights = [
                (unit[1] - cosine * unit[0]) / lengths[0],
                (unit[0] - cosine * unit[1]) / lengths[1],
            ];
            for side in 0..2 {
                let [a, b, c] = points[side];
                for (id, gradient) in fold.indices[side].into_iter().zip([
                    (b - c).cross(weights[side]),
                    (c - a).cross(weights[side]),
                    (a - b).cross(weights[side]),
                ]) {
                    self.add_gradient(jac, fold_start + index, id as usize, gradient);
                }
            }
        }
        let cap_start = fold_start + self.folds.len();
        for (index, id) in self.ids.iter().chain(self.mirror_ids).copied().enumerate() {
            let delta = self.point(id, x) - self.old[id].as_dvec3();
            let length = delta.length();
            if length > 0.0 && length.is_finite() {
                self.add_gradient(jac, cap_start + index, id, -100.0 * delta / length);
            }
        }
        let ray_start = cap_start + self.ids.len() + self.mirror_ids.len();
        let axis = self.camera_axis.as_dvec3().normalize_or_zero();
        if axis == DVec3::ZERO {
            return;
        }
        let corners = self.anchor.ids.map(|id| self.point(id, x));
        // Match the value row's forward-facing policy before differentiating.
        let cross = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
        if cross.dot(axis) <= cross.length() * 0.05 {
            return;
        }
        let origin = self.center.as_dvec3() + axis * (2.0 * f64::from(self.radius));
        if let Some(hit) = analytic::picked_ray_hit(corners, origin, -axis) {
            for (bary, result) in hit.barycentric.iter().enumerate() {
                for (id, gradient) in self.anchor.ids.into_iter().zip(result.gradient) {
                    self.add_gradient(jac, ray_start + bary, id, gradient);
                }
            }
            for (id, gradient) in self.anchor.ids.into_iter().zip(hit.distance.gradient) {
                self.add_gradient(jac, ray_start + 3, id, -100.0 * gradient);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::draw_slsqp_row;
    use super::*;

    fn face(old: &[Vec3], indices: [u32; 3]) -> FaceGuard {
        let [a, b, c] = indices.map(|id| old[id as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        FaceGuard {
            indices,
            cross,
            minimum_projection: cross.length_squared() * 0.05,
            minimum_quality: 0.1,
        }
    }

    fn compare_full_rows(symmetry: bool, seam: bool, amplitude: f64) {
        let mut old = vec![
            Vec3::new(if seam { 0.0 } else { 0.041 }, 0.001, 0.0),
            Vec3::new(0.081, 0.002, 0.0),
            Vec3::new(0.060, 0.034, 0.003),
            Vec3::new(0.062, -0.032, 0.006),
        ];
        if symmetry {
            let mirrored: Vec<_> = old.iter().map(|p| Vec3::new(-p.x, p.y, p.z)).collect();
            old.extend(mirrored);
        }
        let mut proposed = old.clone();
        for position in &mut proposed {
            position.z += 0.009;
        }
        let mut faces = vec![face(&old, [0, 1, 2]), face(&old, [1, 0, 3])];
        let mut folds = vec![FoldGuard {
            indices: [[0, 1, 2], [1, 0, 3]],
            minimum_cosine: 0.5,
        }];
        if symmetry {
            faces.extend([face(&old, [4, 5, 6]), face(&old, [5, 4, 7])]);
            folds.push(FoldGuard {
                indices: [[4, 5, 6], [5, 4, 7]],
                minimum_cosine: 0.5,
            });
        }
        let face_refs: Vec<_> = faces.iter().collect();
        let fold_refs: Vec<_> = folds.iter().collect();
        let ids = [0, 1, 2, 3];
        let mirror_map = [Some(4), Some(5), Some(6), Some(7)];
        let mirror_ids: Vec<_> = if symmetry { (4..8).collect() } else { vec![] };
        let anchor = DrawContactAnchor {
            ids: [0, 1, 2],
            bary: [0.2, 0.4, 0.4],
        };
        let center = Vec3::new(0.057, 0.012, 0.001);
        let camera_axis = Vec3::Z;
        let radius = 0.18;
        let old_distance = super::super::draw_cage_ray_intersection(
            anchor.ids.map(|id| old[id].as_dvec3()),
            center,
            camera_axis,
            radius,
        )
        .unwrap()
        .0;
        let scale = 0.004;
        let context = DrawCageJacobianContext {
            old: &old,
            baseline: &old,
            faces: &face_refs,
            folds: &fold_refs,
            ids: &ids,
            mirrors: symmetry.then_some(mirror_map.as_slice()),
            mirror_ids: &mirror_ids,
            symmetry_offset: 0.0,
            anchor,
            center,
            camera_axis,
            radius,
            scale,
        };
        let mut x = vec![
            0.032, -0.044, 0.053, 0.073, 0.011, 0.025, -0.014, 0.052, 0.061, -0.027, -0.012, 0.034,
        ];
        for coordinate in &mut x {
            *coordinate *= amplitude;
        }
        if seam {
            x[0] = 0.09; // The exact seam projects this coordinate away.
        }
        let mut jac = Mat::new(context.rows(), x.len() + 1);
        context.fill(&x, &mut jac);
        let value = |row, x: &[f64]| {
            draw_slsqp_row(
                &old,
                &proposed,
                &old,
                &face_refs,
                &fold_refs,
                &ids,
                symmetry.then_some(mirror_map.as_slice()),
                &mirror_ids,
                0.0,
                anchor,
                center,
                camera_axis,
                radius,
                old_distance,
                scale,
                0.00005,
                0.0002,
                0.005,
                0.002,
                row,
                x,
            )
        };
        for row in 0..context.rows() {
            for column in 0..x.len() {
                let step = 1e-6;
                let original = x[column];
                x[column] = original + step;
                let plus = value(row, &x);
                x[column] = original - step;
                let minus = value(row, &x);
                x[column] = original;
                let finite = (plus - minus) / (2.0 * step);
                let analytic = jac[(row, column)];
                let tolerance = 2e-5 * finite.abs().max(1.0);
                assert!(
                    (finite - analytic).abs() < tolerance,
                    "sym={symmetry} seam={seam} row={row} column={column} finite={finite} analytic={analytic}"
                );
            }
            assert_eq!(jac[(row, x.len())], 0.0);
        }
        if seam {
            for row in 0..context.rows() {
                assert_eq!(jac[(row, 0)], 0.0);
            }
        }
    }

    #[test]
    fn exact_rows_match_finite_difference_without_symmetry() {
        compare_full_rows(false, false, 1.0);
        compare_full_rows(false, false, 0.01); // Near-zero cap displacement.
    }

    #[test]
    fn exact_rows_match_finite_difference_with_mirrors() {
        compare_full_rows(true, false, 1.0);
        compare_full_rows(true, false, 0.01);
    }

    #[test]
    fn seam_x_gradient_is_zero_without_relaxing_other_rows() {
        compare_full_rows(false, true, 1.0);
        compare_full_rows(false, true, 0.01);
    }
}
