//! Independent long-stroke and authored-sharp-surface checks. These establish
//! OBrush surface quality, useful editing and history; they do not establish
//! commercial brush equivalence or global self-intersection prevention.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use std::collections::HashMap;

fn defaults(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..BrushSettings::default()
    }
}

fn surface_quality(mesh: &Mesh) -> (f64, f64) {
    mesh.validate().unwrap();
    let mut minimum_quality = f64::INFINITY;
    let mut maximum_fold = 0.0_f64;
    let mut edges = HashMap::<(u32, u32), Vec<(bool, glam::DVec3)>>::new();
    for &triangle in &mesh.triangles {
        let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let squared_edges = a.distance_squared(b) + b.distance_squared(c) + c.distance_squared(a);
        let quality = 2.0 * 3.0_f64.sqrt() * cross.length() / squared_edges;
        assert!(quality.is_finite() && cross.length_squared() > 0.0);
        minimum_quality = minimum_quality.min(quality);
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((a < b, cross.normalize()));
        }
    }
    for incident in edges.values() {
        assert_eq!(
            incident.len(),
            2,
            "the independently generated sphere stays closed"
        );
        assert_ne!(incident[0].0, incident[1].0);
        maximum_fold = maximum_fold.max(
            incident[0]
                .1
                .dot(incident[1].1)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees(),
        );
    }
    (minimum_quality, maximum_fold)
}

#[test]
fn long_default_crease_and_draw_then_crease_preserve_useful_high_quality_grooves() {
    for draw_passes in [0, 4] {
        let mut mesh = Mesh::sphere(5);
        let original = mesh.snapshot();
        let mut before_crease = Vec::new();
        let mut refined = 0;
        for (kind, passes) in [
            (BrushKind::Draw, draw_passes),
            (BrushKind::Crease, if draw_passes == 0 { 16 } else { 8 }),
        ] {
            if kind == BrushKind::Crease {
                before_crease = mesh.positions.clone();
            }
            for pass in 0..passes {
                for step in 0..41 {
                    let index = if pass % 2 == 0 { step } else { 40 - step };
                    let hit = mesh
                        .raycast(Ray {
                            origin: Vec3::new(-0.4 + index as f32 * 0.02, 0.09, 3.0),
                            direction: -Vec3::Z,
                        })
                        .expect("a live ray follows the actual sculpt");
                    let result = mesh.apply_brush_with_result(
                        hit.position,
                        hit.normal,
                        Vec3::ZERO,
                        &defaults(kind),
                    );
                    assert!(result.refined_vertices <= 1024);
                    refined += result.refined_vertices;
                }
                let (quality, angle) = surface_quality(&mesh);
                assert!(
                    quality > 0.15,
                    "draw_passes={draw_passes}, pass={pass}, quality={quality}"
                );
                assert!(
                    angle <= 90.001,
                    "draw_passes={draw_passes}, pass={pass}, fold={angle}"
                );
            }
        }
        let groove = mesh
            .positions
            .iter()
            .zip(&before_crease)
            .map(|(after, before)| -(*after - *before).dot(before.normalize_or_zero()))
            .fold(0.0_f32, f32::max);
        assert!(
            groove > 0.04,
            "a safety limit must not turn Crease into a no-op: {groove}"
        );
        assert!(refined > 0);
        assert_eq!(&mesh.masks[..original.masks.len()], original.masks);
        assert_eq!(&mesh.colors[..original.colors.len()], original.colors);
        for (vertex, &position) in original.positions.iter().enumerate() {
            if position.z < 0.5 {
                assert_eq!(mesh.positions[vertex], position);
            }
        }
        let (quality, angle) = surface_quality(&mesh);
        println!(
            "sphere5 app-default Draw{draw_passes}->Crease: groove={groove}, added_vertices={refined}, standard_quality={quality}, max_fold_degrees={angle}"
        );
    }
}

#[test]
fn other_surface_tools_add_shared_detail_and_undo_it_with_the_deformation() {
    for kind in [BrushKind::Clay, BrushKind::Inflate, BrushKind::Flatten] {
        let mut document = Document::new(Mesh::sphere(4));
        let original = document.to_json().unwrap();
        let hit = document
            .active_mesh()
            .unwrap()
            .raycast(Ray {
                origin: Vec3::new(0.23, 0.09, 3.0),
                direction: -Vec3::Z,
            })
            .unwrap();
        document.begin_stroke();
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &defaults(kind),
        );
        document.end_stroke();
        assert!(
            result.changed_vertices > 0 && result.refined_vertices > 0,
            "{kind:?}"
        );
        assert!(result.refined_vertices <= 1024);
        let edited = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), edited);
    }
}

#[test]
fn explicit_smooth_can_improve_an_authored_sharp_hinge_without_moving_protected_vertices() {
    let mut mesh =
        Mesh::from_obj("v -1 0 3.7\nv 1 0 3.7\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 2 1 4\n").unwrap();
    mesh.masks[2..].fill(1.0);
    let original = mesh.snapshot();
    let fold = |mesh: &Mesh| {
        let cross = |triangle: [u32; 3]| {
            let [a, b, c] = triangle.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
            (b - a).cross(c - a).normalize()
        };
        cross(mesh.triangles[0])
            .dot(cross(mesh.triangles[1]))
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees()
    };
    let old_fold = fold(&mesh);
    assert!(old_fold > 140.0);
    let settings = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        symmetry_x: false,
        ..defaults(BrushKind::Smooth)
    };
    let result =
        mesh.apply_brush_with_result(Vec3::new(0.0, 0.0, 3.7), Vec3::Z, Vec3::ZERO, &settings);
    assert_eq!(result.changed_vertices, 2);
    assert_eq!(result.refined_vertices, 0);
    assert!(fold(&mesh) < old_fold - 10.0);
    assert_eq!(mesh.positions[2..], original.positions[2..]);
    assert_eq!(mesh.masks, original.masks);
    assert_eq!(mesh.colors, original.colors);
    mesh.validate().unwrap();
}
