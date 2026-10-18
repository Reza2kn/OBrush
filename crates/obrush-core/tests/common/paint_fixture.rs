use obrush_core::{
    BrushKind, BrushSettings, CornerUv, Mesh, RasterAlpha, SurfaceTopology, Vec3, TRIANGLE_CORNER,
};
use serde_json::{json, Value};

pub fn sheet() -> Mesh {
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    for y in -8..=8 {
        for x in -8..=8 {
            positions.push(Vec3::new(x as f32 / 16.0, y as f32 / 16.0, 0.0));
        }
    }
    for y in 0..16_u32 {
        for x in 0..16_u32 {
            let a = y * 17 + x;
            triangles.extend([[a, a + 1, a + 18], [a, a + 18, a + 17]]);
        }
    }
    let mut m = Mesh::cube();
    m.positions = positions;
    m.triangles = triangles;
    m.masks = vec![0.0; m.positions.len()];
    m.colors = vec![[0.65, 0.69, 0.74]; m.positions.len()];
    m.materials = None;
    m.surface = None;
    m.recompute_normals();
    m.validate().unwrap();
    m
}
pub fn point(x: i32, y: i32) -> usize {
    ((y + 8) * 17 + x + 8) as usize
}
pub fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Paint,
        radius: 0.5,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}
pub fn bits(m: &Mesh) -> Value {
    json!({"positions":m.positions.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"normals":m.normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"masks":m.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"colors":m.colors.iter().map(|v|v.map(f32::to_bits)).collect::<Vec<_>>(),"triangles":m.triangles,"materials":m.materials.as_ref().map(|v|v.iter().map(|a|a.map(f32::to_bits)).collect::<Vec<_>>()),"surface":m.surface.as_ref().map(|s|json!({"faces":s.faces,"uv":s.uv.as_ref().map(|u|json!({"coordinates":u.coordinates.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),"face_indices":u.face_indices}))}))})
}
pub fn surface(m: &Mesh) -> SurfaceTopology {
    SurfaceTopology {
        faces: m
            .triangles
            .iter()
            .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
            .collect(),
        uv: Some(CornerUv {
            coordinates: m.positions.iter().map(|p| [p.x + 0.5, p.y + 0.5]).collect(),
            face_indices: m
                .triangles
                .iter()
                .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
                .collect(),
        }),
    }
}
/// Deterministic independent planar fixtures and parameter variations. This
/// driver uses only pre-Paint-channel APIs so it runs on the frozen base too.
pub fn legacy_cases() -> Value {
    let mut cases = Vec::new();
    for case in 0..32 {
        let mut m = sheet();
        for (i, c) in m.colors.iter_mut().enumerate() {
            *c = [
                (i % 13) as f32 / 16.0,
                (i % 7) as f32 / 8.0,
                (i % 11) as f32 / 16.0,
            ];
        }
        m.colors[point(0, 0)][0] = -0.0;
        for (i, v) in m.masks.iter_mut().enumerate() {
            *v = (i % 5) as f32 / 4.0;
        }
        if case % 2 == 1 {
            m.materials = Some(
                (0..m.positions.len())
                    .map(|i| [(i % 9) as f32 / 16.0, (i % 3) as f32 / 8.0])
                    .collect(),
            );
        }
        if case % 4 == 2 {
            m.surface = Some(surface(&m));
        }
        let mut s = settings();
        s.radius = [0.25, 0.4, 0.5][case % 3];
        s.strength = [0.0, 0.35, 0.9, 1.0][case % 4];
        s.pressure = [0.0, 0.125, 0.5, 1.0][(case / 3) % 4];
        s.hardness = [0.0, 0.5, 1.0][(case / 2) % 3];
        s.invert = case % 2 == 1;
        s.symmetry_x = case % 3 == 1;
        s.backface_culling = case % 5 != 2;
        s.pressure_strength = case % 4 != 2;
        s.pressure_radius = case % 7 == 3;
        s.color = if case % 6 == 0 {
            [-0.2, 1.2, 0.25]
        } else {
            [0.8, 0.2, 0.1]
        };
        if case % 3 == 2 {
            s.alpha = Some(RasterAlpha::new(3, 2, vec![0, 128, 255, 255, 20, 0]).unwrap());
            s.alpha_rotation = 0.37;
        }
        let mut counts = Vec::new();
        for step in 0..13 {
            let c = Vec3::new(
                ((step % 5) as f32 - 2.0) / 32.0,
                ((step % 3) as f32 - 1.0) / 32.0,
                0.0,
            );
            let result = m.apply_brush_with_result(c, Vec3::Z, Vec3::ZERO, &s);
            counts.push(json!({"changed":result.changed_vertices,"scale_bits":result.displacement_scale.to_bits(),"refined":result.refined_vertices,"limited":result.detail_limited}));
        }
        m.validate().unwrap();
        cases.push(json!({"case":case,"dabs":13,"results":counts,"mesh":bits(&m)}));
    }
    Value::Array(cases)
}
