//! Explicit local-to-clip camera bridge. The studio camera computes its own
//! f64 GL matrix and local ray; this module only validates and converts the
//! clip depth convention for WebGPU. No camera pose is guessed on the GPU.
use glam::{DMat3, DMat4, DVec4, Mat4};

pub const VIEW_UNIFORM_BYTES: u64 = 160;

#[derive(Clone, Copy, Debug)]
pub struct ResidentViewMatrix {
    pub local_to_gl_clip: [f64; 16],
    pub local_to_gpu_clip: [f32; 16],
    pub gpu_clip_to_local: [f32; 16],
    pub normal_to_world: [f32; 16],
}
impl ResidentViewMatrix {
    pub fn checked(local_to_gl_clip: [f64; 16]) -> Result<Self, String> {
        let original = DMat4::from_cols_array(&local_to_gl_clip);
        if !original.is_finite() || original.determinant() == 0.0 || !original.inverse().is_finite()
        {
            return Err("Resident camera matrix is nonfinite or singular".into());
        }
        let mut converted = local_to_gl_clip;
        // GL clip z in [-w,w] becomes WebGPU clip z in [0,w]. XY and W stay
        // exactly as supplied by the actual OBrush camera.
        for column in 0..4 {
            converted[column * 4 + 2] =
                0.5 * (local_to_gl_clip[column * 4 + 2] + local_to_gl_clip[column * 4 + 3]);
        }
        let local_to_gpu_clip = converted.map(|value| value as f32);
        let gpu = Mat4::from_cols_array(&local_to_gpu_clip);
        if !gpu.is_finite() || gpu.determinant() == 0.0 || !gpu.inverse().is_finite() {
            return Err("Resident camera matrix is not representable by f32 GPU projection".into());
        }
        Ok(Self {
            local_to_gl_clip,
            local_to_gpu_clip,
            gpu_clip_to_local: gpu.inverse().to_cols_array(),
            normal_to_world: Mat4::IDENTITY.to_cols_array(),
        })
    }
    /// The position matrix already includes the model pose; lighting needs
    /// its separate inverse-transpose normal transform. In particular, a
    /// reflected/sheared model must not use the raw local normal unchanged.
    pub fn with_model_pose(mut self, local_to_world: [f32; 16]) -> Result<Self, String> {
        let affine = obrush_core::ObjectAffineF32V1 {
            local_to_world,
            interaction_center: [0.; 3],
        };
        affine.validate()?;
        let model = DMat4::from_cols_array(&local_to_world.map(f64::from));
        let linear = DMat3::from_cols(
            model.x_axis.truncate(),
            model.y_axis.truncate(),
            model.z_axis.truncate(),
        );
        let normal = linear.inverse().transpose();
        let expanded = DMat4::from_cols(
            normal.x_axis.extend(0.),
            normal.y_axis.extend(0.),
            normal.z_axis.extend(0.),
            DVec4::W,
        )
        .to_cols_array()
        .map(|value| value as f32);
        if !Mat4::from_cols_array(&expanded).is_finite()
            || Mat4::from_cols_array(&expanded).determinant() == 0.
        {
            return Err("Model normal transform exceeds f32 GPU domain".into());
        }
        self.normal_to_world = expanded;
        Ok(self)
    }
    pub fn uniform(self) -> [u32; 40] {
        let mut words = [0; 40];
        for (destination, value) in words[4..20].iter_mut().zip(self.local_to_gpu_clip) {
            *destination = value.to_bits();
        }
        words[20] = 1; // camera matrix mode
        for (destination, value) in words[24..40].iter_mut().zip(self.normal_to_world) {
            *destination = value.to_bits();
        }
        words
    }
}

pub fn diagnostic_uniform(aspect: [f32; 4]) -> [u32; 40] {
    let mut words = [0; 40];
    for (destination, value) in words[..4].iter_mut().zip(aspect) {
        *destination = value.to_bits();
    }
    for (destination, value) in words[24..40].iter_mut().zip(Mat4::IDENTITY.to_cols_array()) {
        *destination = value.to_bits();
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gl_to_webgpu_depth_preserves_xy_and_w_and_rejects_bad_matrices() {
        let result = ResidentViewMatrix::checked(DMat4::IDENTITY.to_cols_array()).unwrap();
        let matrix = Mat4::from_cols_array(&result.local_to_gpu_clip);
        for z in [-1.0, 0.0, 1.0] {
            let projected = matrix * glam::Vec4::new(0.25, -0.5, z, 1.0);
            assert_eq!(projected.x, 0.25);
            assert_eq!(projected.y, -0.5);
            assert_eq!(projected.z, (z + 1.0) * 0.5);
            assert_eq!(projected.w, 1.0);
        }
        assert!(ResidentViewMatrix::checked([0.0; 16]).is_err());
        let mut invalid = DMat4::IDENTITY.to_cols_array();
        invalid[12] = f64::NAN;
        assert!(ResidentViewMatrix::checked(invalid).is_err());
        invalid[12] = f64::MAX;
        assert!(ResidentViewMatrix::checked(invalid).is_err());
    }
    #[test]
    fn reflected_shear_normal_uses_inverse_transpose_not_local_direction() {
        let pose = [
            2., 0., 0., 0., 1., -1., 0., 0., 0., 0., 0.5, 0., 262144., -131072., 4096., 1.,
        ];
        let view = ResidentViewMatrix::checked(DMat4::IDENTITY.to_cols_array())
            .unwrap()
            .with_model_pose(pose)
            .unwrap();
        let normal = Mat4::from_cols_array(&view.normal_to_world);
        assert_eq!(
            normal.transform_vector3(glam::Vec3::X),
            glam::Vec3::new(0.5, 0.5, 0.)
        );
        assert_eq!(normal.transform_vector3(glam::Vec3::Y), -glam::Vec3::Y);
        assert_eq!(normal.transform_vector3(glam::Vec3::Z), 2. * glam::Vec3::Z);
        assert_eq!(view.uniform().len() as u64 * 4, VIEW_UNIFORM_BYTES);
        assert!(
            ResidentViewMatrix::checked(DMat4::IDENTITY.to_cols_array())
                .unwrap()
                .with_model_pose([0.; 16])
                .is_err()
        );
        assert_eq!(
            diagnostic_uniform([1., 1., 0., 0.])[24..40],
            Mat4::IDENTITY.to_cols_array().map(f32::to_bits)
        );
    }
}
