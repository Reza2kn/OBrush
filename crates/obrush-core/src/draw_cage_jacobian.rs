//! Analytic local derivatives for a future Draw cage solver.
//!
//! This file is deliberately not wired into the production core yet. The
//! integration test compiles it independently so the solver can adopt a
//! reviewed, measured interface without changing the current brush path.

use glam::DVec3;

#[derive(Clone, Copy, Debug)]
pub struct ScalarJacobian<const V: usize> {
    pub value: f64,
    pub gradient: [DVec3; V],
}

#[derive(Clone, Copy, Debug)]
pub struct RayHitJacobian {
    pub distance: ScalarJacobian<3>,
    /// Barycentric order matches triangle vertex order. Values are the
    /// unconstrained ray/triangle-plane intersection; the caller decides if
    /// the intersection is forward and inside the face.
    pub barycentric: [ScalarJacobian<3>; 3],
}

fn cross_gradient(p: [DVec3; 3], weight: DVec3) -> [DVec3; 3] {
    [
        (p[1] - p[2]).cross(weight),
        (p[2] - p[0]).cross(weight),
        (p[0] - p[1]).cross(weight),
    ]
}

fn area_vector(p: [DVec3; 3]) -> DVec3 {
    (p[1] - p[0]).cross(p[2] - p[0])
}

/// Twice-area magnitude divided by longest squared edge. This is the exact
/// Draw quality guard metric, with a deterministic longest-edge tie order.
/// It is nondifferentiable at a tie; the selected branch's derivative is
/// returned, and callers must re-evaluate the exact guard after a trial.
pub fn triangle_quality(p: [DVec3; 3]) -> Option<ScalarJacobian<3>> {
    if p.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let area = area_vector(p);
    let magnitude = area.length();
    let edges = [(0, 1), (1, 2), (2, 0)];
    let mut longest = 0.0;
    let mut longest_edge = (0, 1);
    for edge in edges {
        let squared = (p[edge.0] - p[edge.1]).length_squared();
        if squared > longest {
            longest = squared;
            longest_edge = edge;
        }
    }
    if !magnitude.is_finite() || !longest.is_finite() || magnitude == 0.0 || longest == 0.0 {
        return None;
    }
    let mut gradient = cross_gradient(p, area / (magnitude * longest));
    let edge_gradient =
        (p[longest_edge.0] - p[longest_edge.1]) * (2.0 * magnitude / (longest * longest));
    gradient[longest_edge.0] -= edge_gradient;
    gradient[longest_edge.1] += edge_gradient;
    let value = magnitude / longest;
    (value.is_finite() && gradient.iter().all(|g| g.is_finite()))
        .then_some(ScalarJacobian { value, gradient })
}

/// Signed twice-area projection onto a fixed reference vector.
pub fn orientation_projection(p: [DVec3; 3], reference: DVec3) -> Option<ScalarJacobian<3>> {
    if !reference.is_finite() || p.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let value = area_vector(p).dot(reference);
    let gradient = cross_gradient(p, reference);
    (value.is_finite() && gradient.iter().all(|g| g.is_finite()))
        .then_some(ScalarJacobian { value, gradient })
}

/// Ray distance and barycentric Jacobians for the intersection with the
/// triangle's plane. `direction` need not be unit length; distance is its ray
/// parameter. Near-parallel systems are rejected using a scale-relative
/// determinant threshold, since their derivatives are ill-conditioned.
pub fn picked_ray_hit(p: [DVec3; 3], origin: DVec3, direction: DVec3) -> Option<RayHitJacobian> {
    if p.iter().any(|v| !v.is_finite()) || !origin.is_finite() || !direction.is_finite() {
        return None;
    }
    let e1 = p[1] - p[0];
    let e2 = p[2] - p[0];
    let normal = e1.cross(e2);
    let determinant = direction.dot(normal);
    let scale = direction.length() * normal.length();
    if !scale.is_finite()
        || scale == 0.0
        || !determinant.is_finite()
        || determinant.abs() <= 64.0 * f64::EPSILON * scale
    {
        return None;
    }
    let distance_row = normal / determinant;
    let u_row = direction.cross(e2) / determinant;
    let v_row = e1.cross(direction) / determinant;
    let displacement = p[0] - origin;
    let t = displacement.dot(distance_row);
    let u = displacement.dot(u_row);
    let v = displacement.dot(v_row);
    let bary = [1.0 - u - v, u, v];
    let distance = ScalarJacobian {
        value: t,
        gradient: bary.map(|weight| weight * distance_row),
    };
    let barycentric = [
        ScalarJacobian {
            value: bary[0],
            gradient: bary.map(|weight| -weight * (u_row + v_row)),
        },
        ScalarJacobian {
            value: bary[1],
            gradient: bary.map(|weight| weight * u_row),
        },
        ScalarJacobian {
            value: bary[2],
            gradient: bary.map(|weight| weight * v_row),
        },
    ];
    if !distance.value.is_finite()
        || distance.gradient.iter().any(|g| !g.is_finite())
        || barycentric
            .iter()
            .any(|item| !item.value.is_finite() || item.gradient.iter().any(|g| !g.is_finite()))
    {
        return None;
    }
    Some(RayHitJacobian {
        distance,
        barycentric,
    })
}
