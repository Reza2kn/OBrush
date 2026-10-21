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
    // Browser viewport [225,57,1045,869] is [left,top,right,bottom], so
    // physical width=820 and height=812. CSS(635,463) is its exact center.
    let ndc_x = 2.0 * (screen.0 - 225.0) / 820.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (screen.1 - 57.0) / 812.0;
    let tangent = (40.0_f32.to_radians() * 0.5).tan();
    let direction =
        (forward + right * ndc_x * tangent * (820.0 / 812.0) + up * ndc_y * tangent).normalize();
    Ray { origin, direction }
}

#[test]
fn captured_viewport_center_and_corners_calibrate() {
    let eye = Vec3::new(-2.758_769_8, 34.199_276, 20.482_391);
    let target = Vec3::new(-0.273_416_52, -4.797_350_4, 16.926_394);
    assert!(
        ray((635.0, 463.0))
            .direction
            .distance((target - eye).normalize())
            < 1e-6
    );
    assert!(
        ray((225.0, 463.0))
            .direction
            .distance(ray((1045.0, 463.0)).direction)
            > 0.5
    );
    assert!(
        ray((635.0, 57.0))
            .direction
            .distance(ray((635.0, 869.0)).direction)
            > 0.5
    );
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
        let started = std::time::Instant::now();
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &settings,
        );
        let contact_micros = started.elapsed().as_micros();
        let max_displacement = before_positions
            .iter()
            .zip(&document.active_mesh().unwrap().positions)
            .map(|(before, after)| before.distance(*after))
            .fold(0.0_f32, f32::max);
        for (vertex, (&before, &after)) in before_positions
            .iter()
            .zip(&document.active_mesh().unwrap().positions)
            .enumerate()
        {
            if before.distance(hit.position) > radius {
                assert_eq!(
                    before.to_array().map(f32::to_bits),
                    after.to_array().map(f32::to_bits),
                    "vertex {vertex} outside Draw support moved"
                );
            }
        }
        effective += usize::from(result.changed_vertices > 0 && max_displacement >= radius * 0.002);
        records.push((index, result, max_displacement, contact_micros));
    }
    document.end_stroke();
    let final_anchor = document.active_mesh().unwrap().raycast(anchor_ray).unwrap();
    let normal_growth =
        (final_anchor.position - initial_anchor.position).dot(initial_anchor.normal);
    if let Ok(output) = std::env::var("OBRUSH_SPARK_EXPORT_DIR") {
        let output = std::path::Path::new(&output);
        std::fs::create_dir_all(output).unwrap();
        std::fs::write(
            output.join(format!("spark-draw-r{radius}.obrush")),
            document.to_json().unwrap(),
        )
        .unwrap();
    }
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

fn repeated_center_contacts(radius: f32, contacts: usize) {
    let path = std::env::var("OBRUSH_SPARK_STL").expect("set OBRUSH_SPARK_STL to private STL");
    let source = std::fs::read(path).unwrap();
    let mut mesh = Mesh::import_stl(&source).unwrap().mesh;
    let ray = ray((635.0, 463.0));
    let start = mesh.raycast(ray).unwrap();
    let mut settings = BrushSettings {
        kind: BrushKind::Draw,
        radius,
        strength: 0.35,
        symmetry_x: false,
        ..BrushSettings::default()
    };
    settings.culling_axis = Some(-ray.direction);
    let mut samples = vec![(0, start.position)];
    let mut effective = 0;
    let mut first_no_op = None;
    let mut first_limited = None;
    let mut late_effective = 0;
    let mut late_start_hit = start.position;
    for contact in 1..=contacts {
        let hit = mesh.raycast(ray).unwrap();
        let before = mesh.positions.clone();
        let result = mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
        let max_original_motion = before
            .iter()
            .zip(&mesh.positions)
            .map(|(&old, &new)| old.distance(new))
            .fold(0.0_f32, f32::max);
        let useful = result.changed_vertices > 0 && max_original_motion >= radius * 0.002;
        effective += usize::from(useful);
        if contact > contacts.saturating_sub(32) {
            late_effective += usize::from(useful);
        }
        if contact == contacts.saturating_sub(32) {
            late_start_hit = mesh.raycast(ray).unwrap().position;
        }
        if std::env::var_os("OBRUSH_SPARK_TRACE_ALL").is_some() {
            let current = mesh.raycast(ray).unwrap();
            eprintln!("trace contact={contact} changed={} refined={} scale={} detail_limited={} max_motion={} old_hit={:?} new_hit={:?}", result.changed_vertices, result.refined_vertices, result.displacement_scale, result.detail_limited, max_original_motion, hit.position, current.position);
        }
        if [128, 129, 130, 214, 215].contains(&contact) {
            if let Ok(output) = std::env::var("OBRUSH_SPARK_CAPTURE_MESH_DIR") {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(
                    std::path::Path::new(&output).join(format!("contact-{contact}-mesh.json")),
                    serde_json::to_vec(&mesh).unwrap(),
                )
                .unwrap();
            }
        }
        if result.changed_vertices == 0 || max_original_motion < radius * 0.002 {
            first_no_op.get_or_insert(contact);
            if contact > 200 {
                eprintln!("contact={contact} changed={} scale={} refined={} detail_limited={} max_motion={} hit={:?}", result.changed_vertices, result.displacement_scale, result.refined_vertices, result.detail_limited, max_original_motion, hit.position);
            }
        }
        if result.displacement_scale < 0.5 {
            first_limited.get_or_insert(contact);
        }
        if [1, 16, 64, 128, 256].contains(&contact) {
            samples.push((contact, mesh.raycast(ray).unwrap().position));
        }
        if radius < 0.2 && [213, 214, 215, 224, 256].contains(&contact) {
            let (face, current_hit) = mesh.raycast_indexed(ray).unwrap();
            let corners = current_hit.triangle.map(|vertex| {
                let index = vertex as usize;
                (
                    index,
                    mesh.positions[index],
                    mesh.positions[index].distance(current_hit.position),
                    mesh.normals[index].dot(-ray.direction),
                    mesh.masks[index],
                    before
                        .get(index)
                        .map(|old| mesh.positions[index].distance(*old)),
                )
            });
            eprintln!("tip contact={contact} face={face} hit={:?} bary={:?} hit_normal={:?} corners={corners:?}", current_hit.position, current_hit.barycentric, current_hit.normal);
        }
    }
    let final_hit = mesh.raycast(ray).unwrap();
    let normal_growth = (final_hit.position - start.position).dot(start.normal);
    let late_tip_growth = (final_hit.position - late_start_hit).dot(start.normal);
    eprintln!("Sparkdancer repeated Draw radius={radius} contacts={contacts} effective={effective} late_effective={late_effective}/32 first_no_op={first_no_op:?} first_limited={first_limited:?} normal_growth={normal_growth} late_tip_growth={late_tip_growth} samples={samples:?} vertices={} faces={}", mesh.positions.len(), mesh.triangles.len());
    mesh.validate().unwrap();
    assert!(effective >= contacts * 4 / 5, "sustained Draw stalls");
    assert!(
        normal_growth >= radius * 0.2,
        "sustained Draw lacks useful growth"
    );
    assert!(
        late_tip_growth >= radius * 0.01,
        "late repeated Draw strokes no longer raise the picked tip"
    );
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_repeated_center_draw_radius_one() {
    repeated_center_contacts(1.0, 128);
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_repeated_center_draw_small_radius() {
    repeated_center_contacts(0.18, 256);
}

// Captured from the same owned normal-browser run as the positive-Y fixture.
// Sparkdancer is Z-up; retain this distinct front camera and viewport geometry.
fn front_ray(screen: (f32, f32)) -> Ray {
    let origin = Vec3::new(1.747_767_4, -43.589_294, 22.463_451);
    let target = Vec3::new(-0.273_416_52, -4.797_350_4, 16.926_394);
    let forward = (target - origin).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let ndc_x = 2.0 * (screen.0 - 225.0) / 820.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (screen.1 - 57.0) / 812.0;
    let tangent = (40.0_f32.to_radians() * 0.5).tan();
    Ray {
        origin,
        direction: (forward + right * ndc_x * tangent * (820.0 / 812.0) + up * ndc_y * tangent)
            .normalize(),
    }
}

fn repeat_captured_front_contacts(screen: (f32, f32), radius: f32) {
    let path = std::env::var("OBRUSH_SPARK_STL").expect("set OBRUSH_SPARK_STL to private STL");
    let imported = Mesh::import_stl(&std::fs::read(path).unwrap()).unwrap();
    let mut document = Document::new(imported.mesh);
    let original = document.to_json().unwrap();
    let ray = front_ray(screen);
    let first = document
        .require_active_mesh()
        .unwrap()
        .raycast(ray)
        .unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius,
        strength: 0.35,
        symmetry_x: false,
        culling_axis: Some(-ray.direction),
        ..Default::default()
    };
    let mut useful = 0;
    let mut stalled = 0;
    let mut longest_stall = 0;
    document.begin_stroke();
    for contact in 1..=12 {
        let mesh = document.require_active_mesh_mut().unwrap();
        let hit = mesh.raycast(ray).unwrap();
        let before = mesh.positions.clone();
        let result = mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
        let maximum = before
            .iter()
            .zip(&mesh.positions)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0_f32, f32::max);
        let edited = result.changed_vertices > 0 && maximum >= radius * 0.002;
        useful += usize::from(edited);
        stalled = if edited { 0 } else { stalled + 1 };
        longest_stall = longest_stall.max(stalled);
        for (&a, &b) in before.iter().zip(&mesh.positions) {
            if a.distance(hit.position) > radius {
                assert_eq!(
                    a.to_array().map(f32::to_bits),
                    b.to_array().map(f32::to_bits)
                );
            }
        }
        let current = mesh.raycast(ray).unwrap();
        eprintln!("front={screen:?} radius={radius} contact={contact} useful={edited} changed={} refined={} max_motion={maximum} growth={}", result.changed_vertices, result.refined_vertices, (current.position-first.position).dot(first.normal));
    }
    document.end_stroke();
    let mesh = document.require_active_mesh().unwrap();
    mesh.validate().unwrap();
    let last = mesh.raycast(ray).unwrap();
    let growth = (last.position - first.position).dot(first.normal);
    assert!(
        useful >= 8,
        "captured front Draw stalls: {useful}/12 useful contacts"
    );
    assert!(
        longest_stall < 3,
        "captured front Draw has {longest_stall} consecutive stalls"
    );
    assert!(
        growth >= radius * 0.02,
        "captured front Draw growth {growth} is too small"
    );
    let edited = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), original);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), edited);
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_upper_leg_draw_radius_one() {
    repeat_captured_front_contacts((645.0, 550.0), 1.0);
}

#[test]
#[ignore = "requires private Sparkdancer STL path"]
fn imported_spark_lower_leg_draw_radius_one() {
    repeat_captured_front_contacts((750.0, 750.0), 1.0);
}
