//! Additive ordinary Source geometry mirror/alpha controls. Picking uses the
//! captured OBrush precise ray policy; V8 camera/hypot equality is unpaired.
use crate::{
    Mesh, ObjectAffineF32V1, PaintPreparationInput, RasterAlpha, SourceGeometryError,
    SourceGeometryPass, SourceGeometryReport, SourcePickedNormalDisposition,
    SourcePickedNormalUndefined,
};
use glam::{DMat4, DVec3};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SourceMirrorPlane {
    pub origin: [f32; 3],
    pub normal: [f64; 3],
}
impl SourceMirrorPlane {
    pub fn validate(self) -> Result<(), SourceGeometryError> {
        if self
            .origin
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
            || self.normal.iter().any(|v| !v.is_finite())
            || (dot(self.normal, self.normal) - 1.0).abs() > 1e-12
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        Ok(())
    }
    pub fn reflect(self, point: [f64; 3]) -> Result<[f64; 3], SourceGeometryError> {
        self.validate()?;
        if point.iter().any(|v| !v.is_finite()) {
            return Err(SourceGeometryError::InvalidInput);
        }
        let offset = std::array::from_fn(|i| point[i] - f64::from(self.origin[i]));
        let twice = dot(offset, self.normal) * 2.0;
        let out = std::array::from_fn(|i| point[i] - self.normal[i] * twice);
        if out.iter().any(|v| !v.is_finite()) {
            return Err(SourceGeometryError::InvalidInput);
        }
        Ok(out)
    }
    pub fn from_relative_offset(
        center: [f32; 3],
        normal: [f64; 3],
        offset: f64,
        loose: [f64; 6],
    ) -> Result<Self, SourceGeometryError> {
        if !offset.is_finite()
            || !(-1.0..=1.0).contains(&offset)
            || loose.iter().any(|v| !v.is_finite())
            || (0..3).any(|i| loose[i] > loose[i + 3])
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        let radius = 0.5 * hypot(std::array::from_fn(|i| loose[i + 3] - loose[i]));
        let distance = offset * radius;
        let plane = Self {
            origin: std::array::from_fn(|i| (f64::from(center[i]) + normal[i] * distance) as f32),
            normal,
        };
        plane.validate()?;
        Ok(plane)
    }
}

/// Picking-like transient state. These two rows persist only across admitted
/// Source Draw/Clay/Inflate gestures; Legacy/other-tool persistence is a gap.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SourceGeometryAlphaTrack {
    pub origin: [f64; 3],
    pub frame: [f32; 16],
    pub side: f64,
    pub undefined_normal: Option<SourcePickedNormalUndefined>,
    /// Source Lock updates the frame while retaining the press-point origin.
    pub keep_origin: bool,
}
impl Default for SourceGeometryAlphaTrack {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            frame: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            side: 0.0,
            undefined_normal: None,
            keep_origin: false,
        }
    }
}
impl SourceGeometryAlphaTrack {
    pub fn validate(self) -> Result<(), SourceGeometryError> {
        if self
            .origin
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
            || self.frame.iter().any(|v| !v.is_finite())
            || !self.side.is_finite()
            || self.side < 0.0
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        Ok(())
    }
    pub fn updated(
        mut self,
        pass: SourceGeometryPass,
        picked: SourcePickedNormalDisposition,
    ) -> Result<Self, SourceGeometryError> {
        self.validate()?;
        if pass
            .center
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
            || !pass.local_radius_squared.is_finite()
            || !(0.0..=1_000_000_000_000.0).contains(&pass.local_radius_squared)
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        let (face, point) = match picked {
            SourcePickedNormalDisposition::Defined { report } => {
                if report.normal64.iter().any(|v| !v.is_finite()) {
                    return Err(SourceGeometryError::InvalidInput);
                }
                (report.original_face_index, report.local_point64)
            }
            SourcePickedNormalDisposition::Undefined {
                original_face_index,
                local_point64,
                ..
            } => (original_face_index, local_point64),
        };
        if face != pass.original_face_index
            || point.map(f64::to_bits) != pass.center.map(f64::to_bits)
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        self.side = pass.local_radius_squared.sqrt() * std::f64::consts::FRAC_1_SQRT_2;
        let travel = std::array::from_fn(|i| pass.center[i] - self.origin[i]);
        // Source updates side before this early return. An unused undefined
        // picked normal does not poison the old frame on exact zero travel.
        if hypot(travel) == 0.0 {
            return Ok(self);
        }
        let normal = match picked {
            SourcePickedNormalDisposition::Defined { report } => report.normal64,
            SourcePickedNormalDisposition::Undefined {
                reason: SourcePickedNormalUndefined::ZeroDirection,
                ..
            } => [0.0; 3],
            SourcePickedNormalDisposition::Undefined { reason, .. } => {
                if !self.keep_origin {
                    self.origin = pass.center;
                }
                self.undefined_normal = Some(reason);
                return Ok(self);
            }
        };
        let mut up = normalized(travel);
        let projection = -dot(up, normal);
        for i in 0..3 {
            up[i] += normal[i] * projection;
        }
        up = normalized(up);
        if !self.keep_origin {
            self.origin = pass.center;
        }
        let radius = pass.local_radius_squared.sqrt();
        let target = std::array::from_fn(|i| self.origin[i] + normal[i] * radius);
        self.frame = look_at(self.origin, target, up)?;
        self.undefined_normal = None;
        Ok(self)
    }
    pub fn sample(
        self,
        alpha: &RasterAlpha,
        _pass: SourceGeometryPass,
        mirrored: bool,
        position: crate::Vec3,
    ) -> Result<f64, SourceGeometryError> {
        self.validate()?;
        if let Some(reason) = self.undefined_normal {
            return Err(SourceGeometryError::UndefinedPickedNormal(reason));
        }
        crate::paint_response::source_alpha_prepared_side(
            alpha, self.frame, self.side, mirrored, position,
        )
        .map_err(|_| SourceGeometryError::UndefinedAlphaLookup)
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SourceGeometryAlphaTracks {
    pub primary: SourceGeometryAlphaTrack,
    pub mirror: SourceGeometryAlphaTrack,
}
impl SourceGeometryAlphaTracks {
    pub fn validate(self) -> Result<(), SourceGeometryError> {
        self.primary.validate()?;
        self.mirror.validate()
    }
}
#[derive(Clone, Debug, Default)]
pub struct SourceGeometryControls {
    pub alpha: Option<RasterAlpha>,
    pub mirror: bool,
}
impl SourceGeometryControls {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>().saturating_add(self.alpha.as_ref().map_or(0, |a| {
            // RasterAlpha stores Arc<[u8]>, not a spare-capacity Vec.
            a.pixels()
                .len()
                .saturating_add(2 * std::mem::size_of::<usize>())
        }))
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceGeometryControlledReport {
    pub combined: SourceGeometryReport,
    pub passes: [Option<SourceGeometryReport>; 2],
    pub contacts: [Option<SourceGeometryPass>; 2],
    pub tracks: SourceGeometryAlphaTracks,
    pub alpha_dimensions: Option<[u32; 2]>,
}

/// Normal host contact preparation: original primary camera/radius route is
/// unchanged. Mirror reflects actual LOCAL near/far points before intersection.
/// Source exact octree ray-candidate order and Float32 camera are separate gaps.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SourceGeometryContacts {
    pub passes: [Option<SourceGeometryPass>; 2],
    pub triangles: [Option<usize>; 2],
}
pub fn prepare_source_geometry_contacts(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    mirror: Option<SourceMirrorPlane>,
) -> Result<SourceGeometryContacts, SourceGeometryError> {
    prepare_contacts_phase(mesh, pose, input, mirror, false, None)
}
pub(crate) fn prepare_source_geometry_contacts_with_state(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    mirror: Option<SourceMirrorPlane>,
    retained: crate::paint_preparation::VerifiedRetainedPick<'_>,
) -> Result<SourceGeometryContacts, SourceGeometryError> {
    prepare_contacts_phase(mesh, pose, input, mirror, false, Some(retained))
}
pub fn prepare_source_geometry_press_contacts(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    mirror: Option<SourceMirrorPlane>,
) -> Result<SourceGeometryContacts, SourceGeometryError> {
    prepare_contacts_phase(mesh, pose, input, mirror, true, None)
}
fn prepare_contacts_phase(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    input: PaintPreparationInput,
    mirror: Option<SourceMirrorPlane>,
    press: bool,
    retained: Option<crate::paint_preparation::VerifiedRetainedPick<'_>>,
) -> Result<SourceGeometryContacts, SourceGeometryError> {
    let primary = match retained {
        Some(state) => {
            crate::paint_preparation::prepare_source_paint_pass_with_state(mesh, pose, input, state)
        }
        None => crate::prepare_source_paint_pass(mesh, pose, input),
    }
    .map_err(|_| SourceGeometryError::InvalidInput)?;
    let first_triangle = primary.as_ref().map(|(_, hit)| hit.triangle_index);
    let first = primary.map(|(pass, hit)| SourceGeometryPass {
        center: pass.center,
        eye_direction: pass.eye_direction,
        local_radius_squared: pass.local_radius_squared,
        original_face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
    });
    let Some(plane) = mirror else {
        return Ok(SourceGeometryContacts {
            passes: [first, None],
            triangles: [first_triangle, None],
        });
    };
    plane.validate()?;
    mesh.validate()
        .map_err(|_| SourceGeometryError::InvalidMesh)?;
    let matrix = pose.map_or(DMat4::IDENTITY, |p| {
        DMat4::from_cols_array(&p.local_to_world.map(f64::from))
    });
    if let Some(p) = pose {
        p.validate()
            .map_err(|_| SourceGeometryError::InvalidInput)?;
    }
    let inverse = matrix.inverse();
    let near = inverse.transform_point3(
        input
            .view
            .unproject(input.screen, 0.0)
            .map_err(|_| SourceGeometryError::InvalidInput)?,
    );
    let far = inverse.transform_point3(
        input
            .view
            .unproject(input.screen, 0.1)
            .map_err(|_| SourceGeometryError::InvalidInput)?,
    );
    let origin = DVec3::from_array(plane.reflect(near.to_array())?);
    let end = DVec3::from_array(plane.reflect(far.to_array())?);
    let direction = normalized((end - origin).to_array());
    if direction.iter().any(|v| !v.is_finite()) || direction == [0.0; 3] {
        return Err(SourceGeometryError::InvalidInput);
    }
    let Some(hit) = crate::paint_preparation::prepare_source_paint_local_hit(
        mesh,
        origin,
        DVec3::from_array(direction),
    ) else {
        return Ok(SourceGeometryContacts {
            passes: [first, None],
            triangles: [first_triangle, None],
        });
    };
    let center = origin + DVec3::from_array(direction) * hit.2;
    let squared = if press {
        // SculptBase.start/initAlpha: each Picking owns its actual hit depth.
        let world = matrix.transform_point3(center);
        let projected = input
            .view
            .project(world)
            .map_err(|_| SourceGeometryError::InvalidInput)?;
        let offset =
            input.screen_radius * (1.0 + input.radius_factor * (input.pressure * 2.0 - 1.0));
        let edge = input
            .view
            .unproject([projected[0] + offset, projected[1]], projected[2])
            .map_err(|_| SourceGeometryError::InvalidInput)?;
        let squared = (world - edge).length_squared()
            / pose.map_or(1.0, ObjectAffineF32V1::source_scale_squared);
        if !squared.is_finite() || squared <= 0.0 {
            return Err(SourceGeometryError::InvalidInput);
        }
        squared
    } else {
        // Pinned static makeStroke copies primary radius even if primary missed.
        first.map_or(0.0, |p| p.local_radius_squared)
    };
    let second = SourceGeometryPass {
        center: center.to_array(),
        eye_direction: direction,
        local_radius_squared: squared,
        original_face_index: hit.1.unwrap_or(hit.0),
    };
    Ok(SourceGeometryContacts {
        passes: [first, Some(second)],
        triangles: [first_triangle, Some(hit.0)],
    })
}

fn normalized(v: [f64; 3]) -> [f64; 3] {
    let mut scale = dot(v, v);
    if scale > 0.0 {
        scale = 1.0 / scale.sqrt();
    }
    v.map(|x| x * scale)
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn hypot(v: [f64; 3]) -> f64 {
    crate::source_picked_normal::scaled_compensated_hypot3(v)
}
fn look_at(
    eye: [f64; 3],
    center: [f64; 3],
    up: [f64; 3],
) -> Result<[f32; 16], SourceGeometryError> {
    if (0..3).all(|i| (eye[i] - center[i]).abs() < 0.000001) {
        return Ok(SourceGeometryAlphaTrack::default().frame);
    }
    let mut z = std::array::from_fn(|i| eye[i] - center[i]);
    let length = 1.0 / hypot(z);
    z = z.map(|v| v * length);
    let mut x = [
        up[1] * z[2] - up[2] * z[1],
        up[2] * z[0] - up[0] * z[2],
        up[0] * z[1] - up[1] * z[0],
    ];
    let length = hypot(x);
    x = if length == 0.0 {
        [0.0; 3]
    } else {
        x.map(|v| v * (1.0 / length))
    };
    let mut y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    let length = hypot(y);
    y = if length == 0.0 {
        [0.0; 3]
    } else {
        y.map(|v| v * (1.0 / length))
    };
    let m = [
        x[0],
        y[0],
        z[0],
        0.0,
        x[1],
        y[1],
        z[1],
        0.0,
        x[2],
        y[2],
        z[2],
        0.0,
        -dot(x, eye),
        -dot(y, eye),
        -dot(z, eye),
        1.0,
    ]
    .map(|v| v as f32);
    if m.iter().any(|v| !v.is_finite()) {
        return Err(SourceGeometryError::InvalidInput);
    }
    Ok(m)
}
