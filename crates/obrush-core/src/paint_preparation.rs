//! Precise opt-in ordinary Paint preparation over current editable local geometry.
//! Camera/affine math uses the actual OBrush precise scene policy; it is not
//! labeled as equality to SculptGL's Float32 camera/inverse-matrix preparation.
use crate::{
    Mesh, ObjectAffineF32V1, PaintResponsePass, SceneRay64, MAX_TRIANGLES, TRIANGLE_CORNER,
};
use glam::{DMat4, DVec3};
#[derive(Clone, Copy, Debug)]
pub struct PaintViewCapture {
    pub world_to_clip: [f64; 16],
    pub viewport: [f64; 4],
}
impl PaintViewCapture {
    pub fn validate(&self) -> Result<(), String> {
        let m = DMat4::from_cols_array(&self.world_to_clip);
        if !m.is_finite()
            || m.determinant() == 0.
            || !m.inverse().is_finite()
            || self.viewport.iter().any(|v| !v.is_finite())
            || self.viewport[2] <= 0.
            || self.viewport[3] <= 0.
            || self.viewport[2] > 32768.
            || self.viewport[3] > 32768.
        {
            return Err("Precise Paint camera/viewport is invalid or exceeds bounds".into());
        }
        Ok(())
    }
    pub fn unproject(&self, screen: [f64; 2], depth: f64) -> Result<DVec3, String> {
        self.validate()?;
        if screen.iter().any(|v| !v.is_finite()) || !depth.is_finite() {
            return Err("Paint screen/depth must be finite".into());
        }
        let [x, y, w, h] = self.viewport;
        let clip = DVec3::new(
            2. * (screen[0] - x) / w - 1.,
            1. - 2. * (screen[1] - y) / h,
            2. * depth - 1.,
        );
        let p = DMat4::from_cols_array(&self.world_to_clip)
            .inverse()
            .project_point3(clip);
        if !p.is_finite() {
            return Err("Paint unprojection is nonfinite".into());
        }
        Ok(p)
    }
    pub fn project(&self, point: DVec3) -> Result<[f64; 3], String> {
        self.validate()?;
        let p = DMat4::from_cols_array(&self.world_to_clip).project_point3(point);
        let [x, y, w, h] = self.viewport;
        let screen = [
            x + (p.x + 1.) * w * 0.5,
            y + (1. - p.y) * h * 0.5,
            (p.z + 1.) * 0.5,
        ];
        if screen.iter().any(|v| !v.is_finite()) {
            return Err("Paint projection is nonfinite".into());
        }
        Ok(screen)
    }
    pub fn ray(&self, screen: [f64; 2]) -> Result<SceneRay64, String> {
        let near = self.unproject(screen, 0.)?;
        let far = self.unproject(screen, 0.1)?;
        let ray = SceneRay64 {
            origin: near,
            direction: (far - near).normalize(),
        };
        ray.validate()?;
        Ok(ray)
    }
}
#[derive(Clone, Debug)]
pub struct PreparedSourcePaintHit {
    pub triangle_index: usize,
    pub original_face_index: Option<usize>,
    pub local_position: [f64; 3],
    pub local_eye_direction: [f64; 3],
    pub world_position: [f64; 3],
}
/// A retained ray tree bound to the exact borrowed mesh after a full state
/// comparison. Only the document entry constructs this for ordinary dabs;
/// callers cannot carry it across a mutable mesh borrow.
#[derive(Clone, Copy)]
pub(crate) struct VerifiedRetainedPick<'a> {
    mesh: &'a Mesh,
    state: &'a crate::SourceGeometryNormalState,
    exact_domain: bool,
}
impl<'a> VerifiedRetainedPick<'a> {
    pub(crate) fn for_mesh(
        mesh: &'a Mesh,
        state: &'a crate::SourceGeometryNormalState,
    ) -> Option<Self> {
        state.matches_mesh(mesh).then_some(Self {
            mesh,
            state,
            exact_domain: false,
        })
    }

    /// The stroke's immutable-channel witness is checked during this same
    /// exclusive document borrow. A failed witness falls back to the ordinary
    /// public validation path so its error classification stays unchanged.
    pub(crate) fn for_stroke_mesh(
        mesh: &'a Mesh,
        state: &'a crate::SourceGeometryNormalState,
        stroke: &crate::SourceGeometryStroke,
    ) -> Option<Self> {
        let mut verified = Self::for_mesh(mesh, state)?;
        verified.exact_domain = stroke.has_domain() && stroke.validate_domain(mesh).is_ok();
        Some(verified)
    }

    pub(crate) fn exact_domain(self) -> bool {
        self.exact_domain
    }
}
pub fn prepare_source_paint_hit(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    ray: SceneRay64,
) -> Result<Option<PreparedSourcePaintHit>, String> {
    prepare_source_paint_hit_with_state(mesh, pose, ray, None)
}

pub(crate) fn prepare_source_paint_hit_with_state(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    ray: SceneRay64,
    retained: Option<VerifiedRetainedPick<'_>>,
) -> Result<Option<PreparedSourcePaintHit>, String> {
    if let Some(verified) = retained {
        if !std::ptr::eq(verified.mesh, mesh) {
            return Err("Retained Source pick tree belongs to another mesh".into());
        }
        if verified.exact_domain {
            mesh.validate_retained_exact_domain()?;
        } else {
            mesh.validate_retained_geometry()?;
        }
    } else {
        mesh.validate()?;
    }
    ray.validate()?;
    if mesh.triangles.len() > MAX_TRIANGLES {
        return Err("Source Paint pick exceeds bounded triangle work".into());
    }
    let matrix = if let Some(pose) = pose {
        pose.validate()?;
        DMat4::from_cols_array(&pose.local_to_world.map(f64::from))
    } else {
        DMat4::IDENTITY
    };
    let inverse = matrix.inverse();
    let origin = inverse.transform_point3(ray.origin);
    let direction = inverse.transform_vector3(ray.direction).normalize();
    if !origin.is_finite() || !direction.is_finite() || direction == DVec3::ZERO {
        return Err("Precise local Paint ray is invalid".into());
    }
    let candidates = retained
        .map(|verified| verified.state.ray_candidate_faces(origin, direction))
        .transpose()
        .map_err(|error| error.to_string())?;
    let mut best: Option<(usize, Option<usize>, f64)> = None;
    let mut row = 0;
    if let Some(surface) = &mesh.surface {
        if let (Some(faces), Some(verified)) = (&candidates, retained) {
            for &face in faces {
                let face = face as usize;
                let ids = *surface
                    .faces
                    .get(face)
                    .ok_or("Retained Source pick face is missing")?;
                let row = verified
                    .state
                    .first_triangle_row(face)
                    .ok_or("Retained Source pick face row is missing")?;
                if let Some((triangle, t)) =
                    intersect_original_face(mesh, origin, direction, ids, row)
                {
                    if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                        best = Some((triangle, Some(face), t));
                    }
                }
            }
        } else {
            for (face, &ids) in surface.faces.iter().enumerate() {
                if let Some((triangle, t)) =
                    intersect_original_face(mesh, origin, direction, ids, row)
                {
                    if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                        best = Some((triangle, Some(face), t));
                    }
                }
                row += usize::from(ids[3] != TRIANGLE_CORNER) + 1;
            }
        }
    } else {
        let rows: Box<dyn Iterator<Item = usize> + '_> = match &candidates {
            Some(candidates) => Box::new(candidates.iter().map(|&row| row as usize)),
            None => Box::new(0..mesh.triangles.len()),
        };
        for row in rows {
            let ids = mesh.triangles[row];
            if let Some(t) = intersect(mesh, origin, direction, ids) {
                if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                    best = Some((row, None, t));
                }
            }
        }
    }
    Ok(best.map(|(triangle_index, original_face_index, t)| {
        let point = origin + direction * t;
        PreparedSourcePaintHit {
            triangle_index,
            original_face_index,
            local_position: point.to_array(),
            local_eye_direction: direction.to_array(),
            world_position: matrix.transform_point3(point).to_array(),
        }
    }))
}
fn intersect_original_face(
    mesh: &Mesh,
    origin: DVec3,
    direction: DVec3,
    ids: [u32; 4],
    row: usize,
) -> Option<(usize, f64)> {
    let first = [ids[0], ids[1], ids[2]];
    intersect(mesh, origin, direction, first)
        .map(|t| (row, t))
        .or_else(|| {
            (ids[3] != TRIANGLE_CORNER)
                .then(|| intersect(mesh, origin, direction, [ids[0], ids[2], ids[3]]))
                .flatten()
                .map(|t| (row + 1, t))
        })
}
fn intersect(mesh: &Mesh, origin: DVec3, direction: DVec3, ids: [u32; 3]) -> Option<f64> {
    const EPS: f64 = 1e-15;
    let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
    let e1 = b - a;
    let e2 = c - a;
    let p = direction.cross(e2);
    let determinant = e1.dot(p);
    if determinant > -EPS && determinant < EPS {
        return None;
    }
    let inv = 1. / determinant;
    let offset = origin - a;
    let u = offset.dot(p) * inv;
    if !(-EPS..=1. + EPS).contains(&u) {
        return None;
    }
    let q = offset.cross(e1);
    let v = direction.dot(q) * inv;
    if v < -EPS || u + v > 1. + EPS {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t.is_finite() && t >= 0.).then_some(t)
}
#[derive(Clone, Copy, Debug)]
pub struct PaintPreparationInput {
    pub view: PaintViewCapture,
    pub screen: [f64; 2],
    pub screen_radius: f64,
    pub pressure: f64,
    pub radius_factor: f64,
}
/// Validated positive screen travel, distinct from the saved ordinary5–500 UI radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockTravelRadius(f64);
impl LockTravelRadius {
    pub fn new(pixels: f64) -> Result<Self, String> {
        if !pixels.is_finite() || pixels <= 0.0 || pixels > 1_000_000.0 {
            return Err(
                "Source Paint Lock travel must be finite positive and at most1000000 pixels".into(),
            );
        }
        Ok(Self(pixels))
    }
    pub fn pixels(self) -> f64 {
        self.0
    }
}

pub fn prepare_source_paint_pass(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
) -> Result<Option<(PaintResponsePass, PreparedSourcePaintHit)>, String> {
    if !input.screen_radius.is_finite() || !(5.0..=500.0).contains(&input.screen_radius) {
        return Err("Source Paint radius/Tablet input is invalid".into());
    }
    prepare_paint_radius(mesh, pose, input, None)
}

pub(crate) fn prepare_source_paint_pass_with_state(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    retained: VerifiedRetainedPick<'_>,
) -> Result<Option<(PaintResponsePass, PreparedSourcePaintHit)>, String> {
    if !input.screen_radius.is_finite() || !(5.0..=500.0).contains(&input.screen_radius) {
        return Err("Source Paint radius/Tablet input is invalid".into());
    }
    prepare_paint_radius(mesh, pose, input, Some(retained))
}

/// Explicit Lock-only radius contract; it cannot widen the ordinary public entry.
pub fn prepare_source_paint_locked_pass(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    travel: LockTravelRadius,
) -> Result<Option<(PaintResponsePass, PreparedSourcePaintHit)>, String> {
    if input.screen_radius.to_bits() != travel.pixels().to_bits() {
        return Err("Source Paint Lock typed travel does not match actual preparation".into());
    }
    prepare_paint_radius(mesh, pose, input, None)
}

fn prepare_paint_radius(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    retained: Option<VerifiedRetainedPick<'_>>,
) -> Result<Option<(PaintResponsePass, PreparedSourcePaintHit)>, String> {
    let PaintPreparationInput {
        view,
        screen,
        screen_radius,
        pressure,
        radius_factor,
    } = input;
    if !pressure.is_finite()
        || !(0.0..=1.0).contains(&pressure)
        || !radius_factor.is_finite()
        || !(0.0..=1.0).contains(&radius_factor)
    {
        return Err("Source Paint radius/Tablet input is invalid".into());
    }
    let ray = view.ray(screen)?;
    let Some(hit) = prepare_source_paint_hit_with_state(mesh, pose, ray, retained)? else {
        return Ok(None);
    };
    let center = DVec3::from_array(hit.world_position);
    let projection = view.project(center)?;
    let offset = screen_radius * (1. + radius_factor * (pressure * 2. - 1.));
    let edge = view.unproject([projection[0] + offset, projection[1]], projection[2])?;
    let squared =
        (center - edge).length_squared() / pose.map_or(1., ObjectAffineF32V1::source_scale_squared);
    if !squared.is_finite() || squared <= 0. {
        return Err("Source Paint prepared local radius is invalid".into());
    }
    let pass = PaintResponsePass {
        center: hit.local_position,
        local_radius_squared: squared,
        eye_direction: hit.local_eye_direction,
        mirrored_x: false,
        alpha_frame: None,
    };
    Ok(Some((pass, hit)))
}

/// Actual opt-in source-profile preparation. The same shared raw normal field
/// later supplies culling; no Mesh normal cache or Document data is changed.
pub fn prepare_source_paint_pass_with_normals(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    cache: &mut Option<crate::SourceNormalField>,
) -> Result<
    Option<(
        PaintResponsePass,
        PreparedSourcePaintHit,
        crate::SourceNormalReport,
    )>,
    String,
> {
    let Some((pass, hit)) = prepare_source_paint_pass(mesh, pose, input)? else {
        return Ok(None);
    };
    let normals = crate::SourceNormalField::refresh(
        cache,
        mesh,
        crate::SourceNormalPolicy::StaticOriginalFaces,
    )?;
    Ok(Some((pass, hit, normals.report())))
}

/// New explicit normal producer; prior hit/pass and prescribed-normal APIs keep
/// their signatures and arithmetic. Source alpha tracking remains unavailable.
#[derive(Clone, Debug)]
pub struct PreparedSourcePaintNormalPass {
    pub pass: PaintResponsePass,
    pub hit: PreparedSourcePaintHit,
    pub normal_policy: crate::SourceNormalReport,
    pub picked_normal: crate::SourcePickedNormalReport,
}
pub fn prepare_source_paint_pass_with_picked_normal(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    cache: &mut Option<crate::SourceNormalField>,
) -> Result<Option<PreparedSourcePaintNormalPass>, String> {
    let Some((pass, hit, normal_policy)) =
        prepare_source_paint_pass_with_normals(mesh, pose, input, cache)?
    else {
        return Ok(None);
    };
    let field = cache
        .as_ref()
        .ok_or("Source normal producer has no current field")?;
    let picked_normal = crate::prepare_source_picked_normal(
        mesh,
        field,
        crate::SourceOriginalFaceContact {
            face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
            local_point: hit.local_position,
        },
    )?;
    Ok(Some(PreparedSourcePaintNormalPass {
        pass,
        hit,
        normal_policy,
        picked_normal,
    }))
}

/// Explicit alphaNone preparation. Undefined IDW is report data because the
/// source getAlpha(no texture) returns1 before reading its undefined frame.
/// This bundle cannot establish a usable frame for any future alpha mode.
#[derive(Clone, Debug)]
pub struct PreparedSourcePaintAlphaNonePass {
    pub pass: PaintResponsePass,
    pub hit: PreparedSourcePaintHit,
    pub normal_policy: crate::SourceNormalReport,
    pub picked_normal: crate::SourcePickedNormalDisposition,
}
pub fn prepare_source_paint_pass_alpha_none(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    cache: &mut Option<crate::SourceNormalField>,
) -> Result<Option<PreparedSourcePaintAlphaNonePass>, String> {
    let Some((pass, hit, normal_policy)) =
        prepare_source_paint_pass_with_normals(mesh, pose, input, cache)?
    else {
        return Ok(None);
    };
    let field = cache
        .as_ref()
        .ok_or("Source normal producer has no current field")?;
    let picked_normal = crate::prepare_source_picked_normal_alpha_none(
        mesh,
        field,
        crate::SourceOriginalFaceContact {
            face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
            local_point: hit.local_position,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(Some(PreparedSourcePaintAlphaNonePass {
        pass,
        hit,
        normal_policy,
        picked_normal,
    }))
}

/// Additive local reflected-ray entry; original precise public route stays byte exact.
pub(crate) fn prepare_source_paint_local_hit(
    mesh: &Mesh,
    origin: DVec3,
    direction: DVec3,
) -> Option<(usize, Option<usize>, f64)> {
    let mut best: Option<(usize, Option<usize>, f64)> = None;
    let mut row = 0;
    if let Some(surface) = &mesh.surface {
        for (face, ids) in surface.faces.iter().enumerate() {
            let first = [ids[0], ids[1], ids[2]];
            let mut chosen = intersect(mesh, origin, direction, first).map(|t| (row, t));
            row += 1;
            if ids[3] != TRIANGLE_CORNER {
                if chosen.is_none() {
                    chosen = intersect(mesh, origin, direction, [ids[0], ids[2], ids[3]])
                        .map(|t| (row, t));
                }
                row += 1;
            }
            if let Some((triangle, t)) = chosen {
                if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                    best = Some((triangle, Some(face), t));
                }
            }
        }
    } else {
        for (row, &ids) in mesh.triangles.iter().enumerate() {
            if let Some(t) = intersect(mesh, origin, direction, ids) {
                if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                    best = Some((row, None, t));
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod retained_pick_tests {
    use super::*;
    use crate::{MaskConvention, SourceGeometryNormalState};

    #[test]
    fn retained_geometry_proof_still_rejects_changed_attributes_and_stale_geometry() {
        let mut mesh = Mesh::sphere(2);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let state = SourceGeometryNormalState::seed(&mesh).unwrap();
        let ray = SceneRay64 {
            origin: DVec3::new(0.0, 0.0, 3.0),
            direction: -DVec3::Z,
        };
        let compare = |mesh: &Mesh| {
            let scalar = prepare_source_paint_hit(mesh, None, ray);
            let retained = prepare_source_paint_hit_with_state(
                mesh,
                None,
                ray,
                VerifiedRetainedPick::for_mesh(mesh, &state),
            );
            assert_eq!(scalar.as_ref().err(), retained.as_ref().err());
            if let (Ok(Some(a)), Ok(Some(b))) = (scalar, retained) {
                assert_eq!(a.triangle_index, b.triangle_index);
                assert_eq!(a.local_position, b.local_position);
            }
        };
        compare(&mesh);
        let original = mesh.normals[0];
        mesh.normals[0] = glam::Vec3::splat(f32::NAN);
        assert!(VerifiedRetainedPick::for_mesh(&mesh, &state).is_some());
        compare(&mesh);
        mesh.normals[0] = original;
        let original = mesh.masks[0];
        mesh.masks[0] = 2.0;
        compare(&mesh);
        mesh.masks[0] = original;
        let original = mesh.colors[0];
        mesh.colors[0] = [1.1, 0.0, 0.0];
        compare(&mesh);
        mesh.colors[0] = original;
        let original = mesh.positions[0];
        mesh.positions[0] = glam::Vec3::splat(f32::NAN);
        assert!(VerifiedRetainedPick::for_mesh(&mesh, &state).is_none());
        compare(&mesh);
        mesh.positions[0] = original;
        let original = mesh.triangles[0];
        mesh.triangles[0] = [0, 0, 0];
        assert!(VerifiedRetainedPick::for_mesh(&mesh, &state).is_none());
        compare(&mesh);
        mesh.triangles[0] = original;

        let mut textured =
            Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n")
                .unwrap()
                .mesh;
        textured.mask_convention = MaskConvention::UnprotectedF32V1;
        textured.masks.fill(1.0);
        let textured_state = SourceGeometryNormalState::seed(&textured).unwrap();
        textured
            .surface
            .as_mut()
            .unwrap()
            .uv
            .as_mut()
            .unwrap()
            .coordinates[0][0] = f32::NAN;
        assert!(VerifiedRetainedPick::for_mesh(&textured, &textured_state).is_some());
        assert_eq!(
            prepare_source_paint_hit(&textured, None, ray).unwrap_err(),
            prepare_source_paint_hit_with_state(
                &textured,
                None,
                ray,
                VerifiedRetainedPick::for_mesh(&textured, &textured_state),
            )
            .unwrap_err()
        );
    }

    #[test]
    fn retained_mixed_original_faces_keep_second_quad_triangle_row() {
        let mut mesh = Mesh::import_obj(
            b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n\
              v 2 0 0\nv 3 0 0\nv 2 1 0\n\
              v 4 0 0\nv 5 0 0\nv 5 1 0\nv 4 1 0\n\
              f 1 2 3 4\nf 5 6 7\nf 8 9 10 11\n",
        )
        .unwrap()
        .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let state = SourceGeometryNormalState::seed(&mesh).unwrap();
        assert_eq!(state.first_triangle_row(0), Some(0));
        assert_eq!(state.first_triangle_row(1), Some(2));
        assert_eq!(state.first_triangle_row(2), Some(3));
        let ray = SceneRay64 {
            origin: DVec3::new(4.25, 0.75, 1.0),
            direction: -DVec3::Z,
        };
        let scalar = prepare_source_paint_hit(&mesh, None, ray).unwrap().unwrap();
        let indexed = prepare_source_paint_hit_with_state(
            &mesh,
            None,
            ray,
            VerifiedRetainedPick::for_mesh(&mesh, &state),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            (scalar.triangle_index, scalar.original_face_index),
            (4, Some(2))
        );
        assert_eq!(indexed.triangle_index, scalar.triangle_index);
        assert_eq!(indexed.original_face_index, scalar.original_face_index);
        assert_eq!(indexed.local_position, scalar.local_position);
    }

    #[test]
    fn dense_sphere_indexed_hit_matches_full_scan_for_hits_and_misses() {
        let mut mesh = Mesh::sphere(3);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let retained = SourceGeometryNormalState::seed(&mesh).unwrap();
        assert!(
            retained
                .ray_candidate_faces(DVec3::new(0.0, 0.0, 3.0), -DVec3::Z)
                .unwrap()
                .len()
                < mesh.triangles.len() / 4
        );
        for y in -8..=8 {
            for x in -8..=8 {
                let ray = SceneRay64 {
                    origin: DVec3::new(0.0, 0.0, 3.0),
                    direction: DVec3::new(x as f64 * 0.09, y as f64 * 0.09, -3.0).normalize(),
                };
                let scalar = prepare_source_paint_hit(&mesh, None, ray).unwrap();
                let verified = VerifiedRetainedPick::for_mesh(&mesh, &retained).unwrap();
                let indexed =
                    prepare_source_paint_hit_with_state(&mesh, None, ray, Some(verified)).unwrap();
                assert_eq!(
                    scalar
                        .as_ref()
                        .map(|hit| (hit.triangle_index, hit.original_face_index)),
                    indexed
                        .as_ref()
                        .map(|hit| (hit.triangle_index, hit.original_face_index)),
                    "ray grid ({x}, {y})"
                );
                if let (Some(scalar), Some(indexed)) = (scalar, indexed) {
                    assert_eq!(scalar.local_position, indexed.local_position);
                    assert_eq!(scalar.world_position, indexed.world_position);
                    assert_eq!(scalar.local_eye_direction, indexed.local_eye_direction);
                }
            }
        }
    }
}
