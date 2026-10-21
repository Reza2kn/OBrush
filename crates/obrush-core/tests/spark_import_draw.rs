//! Private-asset regression: set OBRUSH_SPARK_STL to a locally owned STL.
//! The file is deliberately not bundled with the repository.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};

const SCREEN_POINTS: [(f32, f32); 12] = [
    (635.0, 463.0),
    (639.0, 463.0),
    (643.0, 463.0),
    (647.0, 463.0),
    (651.0, 463.0),
    (647.0, 463.0),
    (643.0, 463.0),
    (639.0, 463.0),
    (635.0, 463.0),
    (643.0, 459.0),
    (643.0, 463.0),
    (643.0, 467.0),
];

fn ray(screen: (f32, f32)) -> Ray {
    // Captured normal-browser perspective camera and viewport, with the same
    // direct camera-ray formula used by the ordinary legacy OBrush host.
    let origin = Vec3::new(-2.758_769_8, 34.199_276, 20.482_391);
    let target = Vec3::new(-0.273_416_52, -4.797_350_4, 16.926_394);
    let forward = (target - origin).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();
    let ndc_x = 2.0 * (screen.0 - 225.0) / 1045.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (screen.1 - 57.0) / 869.0;
    let tangent = (40.0_f32.to_radians() * 0.5).tan();
    let direction =
        (forward + right * ndc_x * tangent * (1045.0 / 869.0) + up * ndc_y * tangent).normalize();
    Ray { origin, direction }
}

fn run_spark_stroke(radius: f32) {
    let path = std::env::var("OBRUSH_SPARK_STL").expect("set OBRUSH_SPARK_STL to private STL");
    let source = std::fs::read(path).unwrap();
    let imported = Mesh::import_stl(&source).unwrap();
    assert_eq!(imported.report.source_faces, 738_904);
    assert_eq!(imported.report.skipped_degenerate_faces, 4);
    let mut document = Document::new(imported.mesh);
    let original = document.to_json().unwrap();
    let rays: Vec<_> = SCREEN_POINTS.into_iter().map(ray).collect();
    let anchor_ray = ray((643.0, 463.0));
    let initial_anchor = document.active_mesh().unwrap().raycast(anchor_ray).unwrap();
    let mut settings = BrushSettings {
        kind: BrushKind::Draw,
        radius,
        strength: 0.35,
        symmetry_x: false,
        ..BrushSettings::default()
    };
    let mut effective = 0;
    let mut records = Vec::new();
    document.begin_stroke();
    for (index, ray) in rays.iter().copied().enumerate() {
        let hit = document.active_mesh().unwrap().raycast(ray).unwrap();
        let before_positions = document.active_mesh().unwrap().positions.clone();
        settings.culling_axis = Some(-ray.direction);
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &settings,
        );
        let max_displacement = before_positions
            .iter()
            .zip(&document.active_mesh().unwrap().positions)
            .map(|(before, after)| before.distance(*after))
            .fold(0.0_f32, f32::max);
        effective += usize::from(result.changed_vertices > 0 && max_displacement >= radius * 0.002);
        records.push((index, result, max_displacement));
    }
    document.end_stroke();
    let final_anchor = document.active_mesh().unwrap().raycast(anchor_ray).unwrap();
    let normal_growth =
        (final_anchor.position - initial_anchor.position).dot(initial_anchor.normal);
    eprintln!("Sparkdancer Draw radius={radius} effective={effective}/12 normal_growth={normal_growth} records={records:?} vertices={} faces={}", document.active_mesh().unwrap().positions.len(), document.active_mesh().unwrap().triangles.len());
    document.active_mesh().unwrap().validate().unwrap();
    assert!(
        effective >= 8,
        "at least eight contacts must move a vertex by 0.002r"
    );
    assert!(
        normal_growth >= radius * 0.02,
        "the stroke must build at least 0.02r along the initial normal"
    );
    let edited = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), original);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), edited);
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_draw_radius_one_crossing_stroke() {
    run_spark_stroke(1.0);
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_draw_small_radius_crossing_stroke() {
    run_spark_stroke(0.18);
}
