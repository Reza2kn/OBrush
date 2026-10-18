//! Portable numerical contract for the pinned octahedral LogLuv environment.
//! Renderer ownership, GPU precision and reference pixels are separate gates.
//! Derived mathematical contracts: SculptGL MIT, see assets/sculptgl/licenses.
use std::sync::Arc;

type V3 = [f64; 3];
type V4 = [f64; 4];

#[derive(Clone, Copy, Debug)]
pub struct PbrInput {
    pub normal_view: V3,
    pub to_eye: V3,
    pub view_to_world: [[f64; 3]; 3],
    pub albedo_srgb: V3,
    pub roughness: f64,
    pub metallic: f64,
    pub exposure: f64,
}

#[derive(Clone, Debug)]
pub struct EnvironmentAtlas {
    width: usize,
    height: usize,
    rgba: Arc<[u8]>,
    sh: [[f64; 3]; 9],
}

impl EnvironmentAtlas {
    /// Encoded bytes, including alpha, must stay raw. No sRGB decode or Y flip.
    pub fn from_raw(
        width: usize,
        height: usize,
        rgba: Vec<u8>,
        sh: [f64; 27],
    ) -> Result<Self, String> {
        if !(32..=1024).contains(&width) || !width.is_power_of_two() || height != 2 * width {
            return Err(
                "Environment requires a power-of-two 32..1024 by double-height atlas".into(),
            );
        }
        let bytes = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(4))
            .ok_or("Environment allocation overflow")?;
        if rgba.len() != bytes || bytes > 8 * 1024 * 1024 {
            return Err("Environment RGBA length or allocation bound differs".into());
        }
        if sh.iter().any(|x| !x.is_finite() || x.abs() > 1_000_000.0) {
            return Err("Environment requires 27 finite bounded SH coefficients".into());
        }
        Ok(Self {
            width,
            height,
            rgba: rgba.into(),
            sh: std::array::from_fn(|i| [sh[i * 3], sh[i * 3 + 1], sh[i * 3 + 2]]),
        })
    }

    pub fn dimensions(&self) -> [usize; 2] {
        [self.width, self.height]
    }
    pub fn raw_rgba(&self) -> &[u8] {
        &self.rgba
    }
    pub fn coefficients(&self) -> [[f64; 3]; 9] {
        self.sh
    }

    /// Octahedral signed-zero axes deliberately retain the pinned sign(0)=0 quirk.
    pub fn direction_uv(direction: V3) -> Result<[f64; 2], String> {
        if direction.iter().any(|x| !x.is_finite()) {
            return Err("Nonfinite environment direction".into());
        }
        let denominator = direction.iter().map(|x| x.abs()).sum::<f64>();
        if denominator == 0.0 {
            return Err("Zero environment direction".into());
        }
        let oct = direction.map(|x| x / denominator);
        let (mut x, mut y) = (oct[0], oct[1]);
        if oct[2] < 0.0 {
            x = glsl_sign(direction[0]) * (1.0 - oct[1].abs());
            y = glsl_sign(direction[1]) * (1.0 - oct[0].abs());
        }
        Ok([x * 0.5 + 0.5, y * 0.5 + 0.5])
    }

    pub fn level_uv(&self, level: usize, uv: [f64; 2]) -> Result<[f64; 2], String> {
        if level > 4
            || uv
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err("Environment level or octahedral UV exceeds bounds".into());
        }
        let width = self.width as f64 / (1_u32 << level) as f64;
        Ok([
            (1.0 + uv[0] * (width - 2.0)) / self.width as f64,
            (1.0 + uv[1] * (width - 2.0) + self.height as f64 - width * 2.0) / self.height as f64,
        ])
    }

    fn sample_repeat_linear(&self, uv: [f64; 2]) -> V4 {
        let xy = [
            uv[0] * self.width as f64 - 0.5,
            uv[1] * self.height as f64 - 0.5,
        ];
        let floor = xy.map(f64::floor);
        let fraction = [xy[0] - floor[0], xy[1] - floor[1]];
        let texel = |dx: isize, dy: isize| {
            let x = (floor[0] as isize + dx).rem_euclid(self.width as isize) as usize;
            let y = (floor[1] as isize + dy).rem_euclid(self.height as isize) as usize;
            let at = (y * self.width + x) * 4;
            std::array::from_fn::<_, 4, _>(|c| f64::from(self.rgba[at + c]) / 255.0)
        };
        let bottom = mix4(texel(0, 0), texel(1, 0), fraction[0]);
        let top = mix4(texel(0, 1), texel(1, 1), fraction[0]);
        mix4(bottom, top, fraction[1])
    }

    /// Mix encoded RGBA levels before decoding LogLuv; decoding each is different.
    pub fn radiance(&self, direction: V3, roughness: f64) -> Result<V3, String> {
        unit_interval(roughness, "Environment roughness")?;
        let uv = Self::direction_uv(direction)?;
        let lod = roughness * 4.0;
        let low = self.sample_repeat_linear(self.level_uv(lod.floor() as usize, uv)?);
        let high = self.sample_repeat_linear(self.level_uv(lod.ceil() as usize, uv)?);
        decode_logluv(mix4(low, high, lod.fract()))
    }

    pub fn diffuse(&self, normal_world: V3) -> Result<V3, String> {
        if normal_world.iter().any(|x| !x.is_finite()) || dot(normal_world, normal_world) == 0.0 {
            return Err("Nonfinite or zero SH direction".into());
        }
        // The source caller supplies its rotated normal. Do not normalize it
        // again: even a near-unit f32 rotation changes the SH basis if normalized.
        let [x, y, z0] = normal_world;
        let z = -z0;
        let basis = [
            1.0,
            y,
            z,
            x,
            y * x,
            y * z,
            3.0 * z * z - 1.0,
            z * x,
            x * x - y * y,
        ];
        Ok(std::array::from_fn(|c| {
            self.sh
                .iter()
                .zip(basis)
                .map(|(row, b)| row[c] * b)
                .sum::<f64>()
                .max(0.0)
        }))
    }

    /// Fixed-Z view ray from the source background quad's interpolated UV.
    /// This intentionally does not inverse-project by the camera's FOV.
    pub fn background_linear(
        &self,
        uv: [f64; 2],
        view_to_world: [[f64; 3]; 3],
        blur: f64,
        ambient: bool,
    ) -> Result<V3, String> {
        for value in uv {
            unit_interval(value, "Background UV")?;
        }
        unit_interval(blur, "Background blur")?;
        if view_to_world.iter().flatten().any(|x| !x.is_finite()) {
            return Err("Nonfinite background camera rotation".into());
        }
        let view = [2.0 * uv[0] - 1.0, 2.0 * uv[1] - 1.0, -1.0];
        let world = normalized(std::array::from_fn(|row| dot(view_to_world[row], view)))?;
        if ambient {
            self.diffuse(world)
        } else {
            self.radiance(world, blur * blur)
        }
    }

    pub fn pbr_linear(&self, input: PbrInput) -> Result<V3, String> {
        let PbrInput {
            normal_view,
            to_eye,
            view_to_world,
            albedo_srgb,
            roughness,
            metallic,
            exposure,
        } = input;
        unit_interval(roughness, "PBR roughness")?;
        unit_interval(metallic, "PBR metallic")?;
        for value in albedo_srgb {
            unit_interval(value, "PBR albedo")?;
        }
        if !exposure.is_finite()
            || !(0.0..=5.0).contains(&exposure)
            || view_to_world.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("Invalid PBR exposure or view rotation".into());
        }
        let normal = normalized(normal_view)?;
        let eye = normalized(to_eye)?;
        let no_v = dot(normal, eye).clamp(0.0, 1.0);
        let r = roughness.max(0.0001);
        let reflection = normalized(std::array::from_fn(|c| 2.0 * no_v * normal[c] - eye[c]))?;
        let smoothness = 1.0 - r;
        let dominant = mix3(normal, reflection, smoothness * (smoothness.sqrt() + r));
        let transform = |v: V3| std::array::from_fn(|row| dot(view_to_world[row], v));
        let diffuse = self.diffuse(transform(normal))?;
        let prefilt = self.radiance(transform(dominant), r)?;
        let linear = albedo_srgb.map(srgb_to_linear);
        let specular = linear.map(|c| 0.04 * (1.0 - metallic) + c * metallic);
        let brdf = integrated_brdf(specular, r, no_v);
        let output = std::array::from_fn(|c| {
            exposure * (linear[c] * (1.0 - metallic) * diffuse[c] + prefilt[c] * brdf[c])
        });
        if output.iter().any(|x: &f64| !x.is_finite()) {
            return Err("Nonfinite environment PBR result".into());
        }
        Ok(output)
    }
}

/// The original image dimensions affect the source quad in every background mode.
/// The environment modes force fill, even when the image fit option is stored.
pub fn background_quad_half_extents(
    viewport: [usize; 2],
    image: [usize; 2],
    fill: bool,
    environment: bool,
) -> Result<[f64; 2], String> {
    if viewport.into_iter().any(|x| x == 0 || x > 16_384)
        || image.into_iter().any(|x| x == 0 || x > 4096)
    {
        return Err("Background dimensions exceed nonzero allocation bounds".into());
    }
    let ratio = (viewport[0] as f64 / viewport[1] as f64) / (image[0] as f64 / image[1] as f64);
    let comparison = if fill || environment {
        1.0 / ratio
    } else {
        ratio
    };
    Ok(if comparison < 1.0 {
        [1.0, ratio]
    } else {
        [1.0 / ratio, 1.0]
    })
}

pub fn decode_logluv(rgba: V4) -> Result<V3, String> {
    for value in rgba {
        unit_interval(value, "Encoded LogLuv channel")?;
    }
    if rgba[1] == 0.0 {
        return Err("Sampled LogLuv chromaticity denominator is zero".into());
    }
    let y = ((rgba[2] * 255.0 + rgba[3] - 127.0) / 2.0).exp2();
    let z = y / rgba[1];
    let x = rgba[0] * z;
    Ok([
        (6.0013 * x - 1.332 * y + 0.3007 * z).max(0.0),
        (-2.700 * x + 3.1029 * y - 1.088 * z).max(0.0),
        (-1.7995 * x - 5.7720 * y + 5.6268 * z).max(0.0),
    ])
}

pub fn integrated_brdf(specular: V3, roughness: f64, no_v: f64) -> V3 {
    let r = [
        -roughness + 1.0,
        -0.0275 * roughness + 0.0425,
        -0.572 * roughness + 1.04,
        0.022 * roughness - 0.04,
    ];
    let a004 = (r[0] * r[0]).min((-9.28 * no_v).exp2()) * r[0] + r[1];
    specular.map(|c| c * (-1.04 * a004 + r[2]) + (1.04 * a004 + r[3]))
}

/// The source merge consumes combined linear light, then one display transfer.
pub fn display_composite(
    opaque_linear: V3,
    premultiplied_transparent_linear: V3,
    transparent_alpha: f64,
    filmic: bool,
) -> Result<V3, String> {
    unit_interval(transparent_alpha, "Transparent alpha")?;
    if opaque_linear
        .iter()
        .chain(&premultiplied_transparent_linear)
        .any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err("Composite expects finite nonnegative linear channels".into());
    }
    Ok(std::array::from_fn(|c| {
        let linear =
            opaque_linear[c] * (1.0 - transparent_alpha) + premultiplied_transparent_linear[c];
        if filmic {
            let x = (linear - 0.004).max(0.0);
            (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06)
        } else {
            linear_to_srgb(linear)
        }
    }))
}

fn glsl_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}
fn unit_interval(x: f64, label: &str) -> Result<(), String> {
    if x.is_finite() && (0.0..=1.0).contains(&x) {
        Ok(())
    } else {
        Err(format!("{label} exceeds finite unit interval"))
    }
}
fn dot(a: V3, b: V3) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn normalized(a: V3) -> Result<V3, String> {
    let squared = dot(a, a);
    if !squared.is_finite() || squared == 0.0 {
        return Err("Nonfinite or zero environment vector".into());
    }
    let len = squared.sqrt();
    Ok(a.map(|x| x / len))
}
fn mix3(a: V3, b: V3, t: f64) -> V3 {
    std::array::from_fn(|c| a[c] * (1.0 - t) + b[c] * t)
}
fn mix4(a: V4, b: V4, t: f64) -> V4 {
    std::array::from_fn(|c| a[c] * (1.0 - t) + b[c] * t)
}
fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
fn linear_to_srgb(c: f64) -> f64 {
    if c < 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn constant(rgba: [u8; 4], sh: [f64; 27]) -> EnvironmentAtlas {
        EnvironmentAtlas::from_raw(32, 64, rgba.repeat(32 * 64), sh).unwrap()
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-12, "{a} != {b}");
    }
    #[test]
    fn independent_axis_octahedron_includes_source_negative_z_and_zero_axis_quirk() {
        for (direction, expected) in [
            ([1., 0., 0.], [1., 0.5]),
            ([-1., 0., 0.], [0., 0.5]),
            ([0., 1., 0.], [0.5, 1.]),
            ([0., -1., 0.], [0.5, 0.]),
            ([0., 0., 1.], [0.5, 0.5]),
            ([0., 0., -1.], [0.5, 0.5]),
            ([1., 1., -1.], [5. / 6., 5. / 6.]),
            ([-0., 1., -1.], [0.5, 1.]),
        ] {
            let actual = EnvironmentAtlas::direction_uv(direction).unwrap();
            for c in 0..2 {
                close(actual[c], expected[c]);
            }
        }
        assert!(EnvironmentAtlas::direction_uv([0.; 3]).is_err());
        assert!(EnvironmentAtlas::direction_uv([f64::NAN, 0., 1.]).is_err());
    }
    #[test]
    fn independent_padded_mip_coordinates_are_known_at_every_level() {
        let atlas = constant([102, 64, 127, 0], [0.; 27]);
        for level in 0..5 {
            let w = 32. / (1_u32 << level) as f64;
            let uv = atlas.level_uv(level, [0.25, 0.75]).unwrap();
            close(uv[0], (1. + 0.25 * (w - 2.)) / 32.);
            close(uv[1], (1. + 0.75 * (w - 2.) + 64. - 2. * w) / 64.);
        }
    }
    #[test]
    fn column_major_logluv_inverse_has_independent_known_values() {
        let rgba = [102. / 255., 64. / 255., 127. / 255., 0.];
        let actual = decode_logluv(rgba).unwrap();
        let xyz = [102. / 64., 1., 255. / 64.];
        let expected: [f64; 3] = [
            6.0013 * xyz[0] - 1.332 + 0.3007 * xyz[2],
            -2.7 * xyz[0] + 3.1029 - 1.088 * xyz[2],
            -1.7995 * xyz[0] - 5.772 + 5.6268 * xyz[2],
        ];
        for c in 0..3 {
            close(actual[c], expected[c].max(0.));
        }
        assert!(decode_logluv([0.; 4]).is_err());
    }
    #[test]
    fn constant_encoded_atlas_is_lod_and_direction_invariant() {
        let atlas = constant([102, 64, 127, 0], [0.; 27]);
        let expected = decode_logluv([102. / 255., 64. / 255., 127. / 255., 0.]).unwrap();
        for r in [0., 0.125, 0.3, 0.5, 0.9, 1.] {
            for direction in [[1., 0., 0.], [0., 0., -1.], [1., 2., 3.]] {
                let actual = atlas.radiance(direction, r).unwrap();
                for c in 0..3 {
                    close(actual[c], expected[c]);
                }
            }
        }
    }
    #[test]
    fn encoded_level_mix_is_decoded_once_and_differs_from_mixing_decoded_light() {
        let mut bytes = [102, 64, 127, 0].repeat(32 * 64);
        for row in 32..64 {
            for col in 0..32 {
                bytes[(row * 32 + col) * 4 + 2] = 131;
            }
        }
        let atlas = EnvironmentAtlas::from_raw(32, 64, bytes, [0.; 27]).unwrap();
        let actual = atlas.radiance([0., 0., 1.], 0.125).unwrap();
        let expected = decode_logluv([102. / 255., 64. / 255., 129. / 255., 0.]).unwrap();
        let low = decode_logluv([102. / 255., 64. / 255., 127. / 255., 0.]).unwrap();
        let high = decode_logluv([102. / 255., 64. / 255., 131. / 255., 0.]).unwrap();
        for c in 0..3 {
            close(actual[c], expected[c]);
        }
        assert!(
            actual
                .iter()
                .zip(mix3(low, high, 0.5))
                .any(|(a, b)| (a - b).abs() > 0.01)
        );
    }
    #[test]
    fn raw_sampler_linear_repeat_has_independent_half_texel_values() {
        let mut bytes = [0, 64, 127, 0].repeat(32 * 64);
        for y in 0..64 {
            bytes[(y * 32 + 31) * 4] = 255;
        }
        let atlas = EnvironmentAtlas::from_raw(32, 64, bytes, [0.; 27]).unwrap();
        close(atlas.sample_repeat_linear([0., 0.3])[0], 0.5);
        close(atlas.sample_repeat_linear([1., 0.3])[0], 0.5);
        close(atlas.sample_repeat_linear([0.5 / 32., 0.3])[0], 0.);
    }
    #[test]
    fn independent_spherical_harmonic_basis_retains_negative_z() {
        let mut sh = [0.; 27];
        sh[0] = 2.;
        sh[7] = 3.;
        sh[10] = 4.;
        sh[20] = 5.;
        let atlas = constant([102, 64, 127, 0], sh);
        assert_eq!(atlas.diffuse([0., 0., 1.]).unwrap(), [2., 0., 10.]);
        assert_eq!(atlas.diffuse([1., 0., 0.]).unwrap(), [2., 4., 0.]);
    }
    #[test]
    fn pbr_extremes_and_zero_exposure_are_known_and_finite() {
        let atlas = constant([102, 64, 127, 0], [0.1; 27]);
        let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        assert_eq!(
            atlas
                .pbr_linear(PbrInput {
                    normal_view: [0., 0., 1.],
                    to_eye: [0., 0., 1.],
                    view_to_world: identity,
                    albedo_srgb: [0.5; 3],
                    roughness: 0.5,
                    metallic: 0.5,
                    exposure: 0.
                })
                .unwrap(),
            [0.; 3]
        );
        for rough in [0., 0.25, 0.5, 1.] {
            for metallic in [0., 0.5, 1.] {
                let out = atlas
                    .pbr_linear(PbrInput {
                        normal_view: [0., 0., 1.],
                        to_eye: [0., 0., 1.],
                        view_to_world: identity,
                        albedo_srgb: [0.5, 0.3, 0.1],
                        roughness: rough,
                        metallic,
                        exposure: 1.,
                    })
                    .unwrap();
                assert!(out.iter().all(|x| x.is_finite() && *x >= 0.));
            }
        }
    }
    #[test]
    fn independent_linear_composition_precedes_display_or_filmic_exactly_once() {
        let ordinary = display_composite([0.2, 0.4, 0.8], [0.1, 0.05, 0.], 0.5, false).unwrap();
        for (actual, linear) in ordinary.into_iter().zip([0.2_f64, 0.25, 0.4]) {
            close(actual, 1.055 * linear.powf(1. / 2.4) - 0.055);
        }
        let filmic = display_composite([0.004, 1., 4.], [0.; 3], 0., true).unwrap();
        assert_eq!(filmic[0], 0.);
        for c in 1..3 {
            let x = [0.004, 1., 4.][c] - 0.004;
            close(
                filmic[c],
                (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06),
            );
        }
    }
    #[test]
    fn malformed_allocations_coefficients_and_parameters_reject_before_replacement() {
        assert!(EnvironmentAtlas::from_raw(31, 62, vec![], [0.; 27]).is_err());
        assert!(EnvironmentAtlas::from_raw(32, 65, vec![], [0.; 27]).is_err());
        assert!(EnvironmentAtlas::from_raw(32, 64, vec![0; 8191], [0.; 27]).is_err());
        let mut sh = [0.; 27];
        sh[2] = f64::INFINITY;
        assert!(EnvironmentAtlas::from_raw(32, 64, vec![0; 8192], sh).is_err());
        let atlas = constant([102, 64, 127, 0], [0.; 27]);
        let original = atlas.raw_rgba().to_vec();
        assert!(atlas.radiance([1., 0., 0.], 1.01).is_err());
        assert!(atlas.level_uv(5, [0.5, 0.5]).is_err());
        assert!(display_composite([1.; 3], [0.; 3], 1.1, false).is_err());
        assert_eq!(atlas.raw_rgba(), original);
    }

    #[test]
    fn supplied_sh_direction_is_not_silently_renormalized() {
        let mut sh = [0.0; 27];
        sh[9] = 1.0;
        sh[24] = 1.0;
        let atlas = constant([102, 64, 127, 0], sh);
        assert_eq!(atlas.diffuse([2.0, 0.0, 0.0]).unwrap(), [6.0, 0.0, 0.0]);
        let near_unit = 0.9999999403953552;
        let actual = atlas.diffuse([near_unit, 0.0, 0.0]).unwrap()[0];
        close(actual, near_unit + near_unit * near_unit);
        assert!(actual < 2.0);
    }

    #[test]
    fn independent_image_fill_fit_and_forced_environment_quad_are_known() {
        assert_eq!(
            background_quad_half_extents([800, 400], [400, 400], true, false).unwrap(),
            [1.0, 2.0]
        );
        assert_eq!(
            background_quad_half_extents([800, 400], [400, 400], false, false).unwrap(),
            [0.5, 1.0]
        );
        assert_eq!(
            background_quad_half_extents([400, 800], [800, 400], true, false).unwrap(),
            [4.0, 1.0]
        );
        assert_eq!(
            background_quad_half_extents([400, 800], [800, 400], false, false).unwrap(),
            [1.0, 0.25]
        );
        assert_eq!(
            background_quad_half_extents([400, 800], [800, 400], false, true).unwrap(),
            [4.0, 1.0]
        );
        assert_eq!(
            background_quad_half_extents([512, 512], [1, 1], false, false).unwrap(),
            [1.0, 1.0]
        );
        assert!(background_quad_half_extents([0, 100], [100, 100], true, false).is_err());
    }

    #[test]
    fn background_fixed_z_camera_ray_and_ambient_sh_have_independent_values() {
        let mut sh = [0.0; 27];
        sh[0] = 1.0;
        sh[9] = 1.0;
        let atlas = constant([102, 64, 127, 0], sh);
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            atlas
                .background_linear([0.5, 0.5], identity, 0.0, true)
                .unwrap(),
            [1.0, 0.0, 0.0]
        );
        let actual = atlas
            .background_linear([1.0, 0.5], identity, 0.0, true)
            .unwrap();
        close(actual[0], 1.0 + 1.0 / 2.0_f64.sqrt());
        let rotation = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(
            atlas
                .background_linear([0.5, 0.5], rotation, 1.0, true)
                .unwrap(),
            [0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn background_blur_is_squared_before_encoded_lod_selection() {
        let mut bytes = [102, 64, 127, 0].repeat(32 * 64);
        for row in 32..64 {
            for col in 0..32 {
                bytes[(row * 32 + col) * 4 + 2] = 131;
            }
        }
        let atlas = EnvironmentAtlas::from_raw(32, 64, bytes, [0.0; 27]).unwrap();
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let blur = 0.125_f64.sqrt();
        let actual = atlas
            .background_linear([0.5, 0.5], identity, blur, false)
            .unwrap();
        let expected = decode_logluv([102.0 / 255.0, 64.0 / 255.0, 129.0 / 255.0, 0.0]).unwrap();
        for c in 0..3 {
            close(actual[c], expected[c]);
        }
    }
}
