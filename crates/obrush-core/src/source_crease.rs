//! Independent static Crease scalar response. The stroke owns first-touch
//! proxy positions; current picked direction and live alpha are separate inputs.
use super::{SourceGeometryError, SourceGeometryPass};
use crate::Vec3;

pub(super) struct CreaseDab {
    pub pass: SourceGeometryPass,
    pub normal: [f64; 3],
    pub intensity: f64,
    pub negative: bool,
}
impl CreaseDab {
    pub fn candidate(
        self,
        point: Vec3,
        proxy: Vec3,
        unprotected: f64,
        alpha: impl FnOnce(Vec3) -> Result<f64, SourceGeometryError>,
    ) -> Result<Vec3, SourceGeometryError> {
        if unprotected == 0.0 {
            return Ok(point);
        }
        let radius = self.pass.local_radius_squared.sqrt();
        let displacement =
            std::array::from_fn::<_, 3, _>(|k| self.pass.center[k] - f64::from(proxy[k]));
        let [dx, dy, dz] = displacement;
        let distance = ((dx * dx + dy * dy) + dz * dz).sqrt() / radius;
        if distance >= 1.0 {
            return Ok(point);
        }
        let squared = distance * distance;
        let mut falloff = (3.0 * squared * squared - 4.0 * squared * distance) + 1.0;
        // The alpha uses live geometry, while the radius/pinch use the proxy.
        falloff *= unprotected * alpha(point)?;
        let deformation = self.intensity * 0.07;
        let mut brush_factor = deformation * radius;
        if self.negative {
            brush_factor = -brush_factor;
        }
        let brush_modifier = falloff.powf(5.0) * brush_factor;
        falloff *= deformation;
        let mut result = point;
        for k in 0..3 {
            let pinch = displacement[k] * falloff;
            let brush = self.normal[k] * brush_modifier;
            // Preserve the two ordered f64 additions before the Float32 store.
            let value = ((f64::from(point[k]) + pinch) + brush) as f32;
            if !value.is_finite() || value.abs() > 1_000_000.0 {
                return Err(SourceGeometryError::NonfiniteCandidate);
            }
            // Retain the established unchanged-component signed-zero policy.
            if pinch != 0.0 || brush != 0.0 {
                result[k] = value;
            }
        }
        Ok(result)
    }
}
