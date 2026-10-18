//! Regression fixtures exercise the public engine API in the order used by the
//! application: pick a surface, group dabs, edit attributes, change topology, save.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};

fn visible_hit(mesh: &Mesh, x: f32, y: f32) -> obrush_core::Hit {
    mesh.raycast(Ray {
        origin: Vec3::new(x, y, 3.0),
        direction: -Vec3::Z,
    })
    .expect("surface remains visible")
}

fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.22,
        strength: 0.75,
        invert: false,
        symmetry_x: true,
        color: [0.85, 0.12, 0.18],
        ..Default::default()
    }
}

fn assert_surface_equal(first: &Mesh, second: &Mesh) {
    assert_eq!(first.positions, second.positions);
    assert_eq!(first.triangles, second.triangles);
    assert_eq!(first.masks, second.masks);
    assert_eq!(first.colors, second.colors);
    assert_eq!(first.normals, second.normals);
}

#[test]
fn picked_sculpt_stroke_masks_polypaint_subdivision_and_save_form_one_workflow() {
    let mut document = Document::new(Mesh::sphere(3));
    document.name = "Workflow regression sculpture".into();
    let original = document.active_mesh().unwrap().clone();
    let draw = settings(BrushKind::Draw);
    document.begin_stroke();
    let mut touched = 0;
    for step in 0..21 {
        let x = -0.45 + 0.045 * step as f32;
        let hit = visible_hit(document.active_mesh().unwrap(), x, 0.08);
        touched += document.active_mesh_mut().unwrap().apply_brush(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &draw,
        );
    }
    document.end_stroke();
    assert!(touched > 21);
    assert_ne!(
        document.active_mesh().unwrap().positions,
        original.positions
    );
    document.active_mesh().unwrap().validate().unwrap();
    let sculpted = document.active_mesh().unwrap().clone();
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    assert!(document.redo());
    assert_surface_equal(document.active_mesh().unwrap(), &sculpted);

    // The center pole is masked to full opacity through ordinary brush dabs.
    let center = visible_hit(document.active_mesh().unwrap(), 0.0, 0.0);
    let protected = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(center.position)
                .total_cmp(&b.distance_squared(center.position))
        })
        .unwrap()
        .0;
    let mut mask = settings(BrushKind::Mask);
    mask.strength = 1.0;
    document.begin_stroke();
    for _ in 0..12 {
        document.active_mesh_mut().unwrap().apply_brush(
            center.position,
            center.normal,
            Vec3::ZERO,
            &mask,
        );
    }
    document.end_stroke();
    assert_eq!(document.active_mesh().unwrap().masks[protected], 1.0);
    let masked = document.active_mesh().unwrap().clone();

    document.begin_stroke();
    let paint = settings(BrushKind::Paint);
    document.active_mesh_mut().unwrap().apply_brush(
        center.position,
        center.normal,
        Vec3::ZERO,
        &paint,
    );
    let color_hit = visible_hit(document.active_mesh().unwrap(), 0.52, -0.23);
    for _ in 0..8 {
        document.active_mesh_mut().unwrap().apply_brush(
            color_hit.position,
            color_hit.normal,
            Vec3::ZERO,
            &paint,
        );
    }
    document.end_stroke();
    assert_eq!(
        document.active_mesh().unwrap().colors[protected],
        masked.colors[protected]
    );
    assert_eq!(document.active_mesh().unwrap().positions, masked.positions);
    assert!(document
        .active_mesh()
        .unwrap()
        .colors
        .iter()
        .zip(&masked.colors)
        .any(|(a, b)| a != b));
    let painted = document.active_mesh().unwrap().clone();
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &masked);
    assert!(document.redo());
    assert_surface_equal(document.active_mesh().unwrap(), &painted);

    let saved = document.to_json().unwrap();
    let restored = Document::from_json(&saved).unwrap();
    assert_eq!(restored.name, document.name);
    assert_surface_equal(restored.active_mesh().unwrap(), &painted);
    assert!(!restored.can_undo());
    let obj = Mesh::from_obj(&document.active_mesh().unwrap().to_obj()).unwrap();
    assert_eq!(obj.positions, painted.positions);
    assert_eq!(obj.triangles, painted.triangles);
    assert_eq!(obj.colors, painted.colors);
    assert_eq!(obj.normals, painted.normals);
    assert!(obj.masks.iter().all(|&mask| mask == 0.0)); // OBJ has no mask channel.

    document.subdivide().unwrap();
    document.active_mesh().unwrap().validate().unwrap();
    assert_eq!(
        document.active_mesh().unwrap().triangles.len(),
        painted.triangles.len() * 4
    );
    assert_eq!(
        &document.active_mesh().unwrap().positions[..painted.positions.len()],
        &painted.positions
    );
    assert_eq!(
        &document.active_mesh().unwrap().masks[..painted.masks.len()],
        &painted.masks
    );
    assert_eq!(
        &document.active_mesh().unwrap().colors[..painted.colors.len()],
        &painted.colors
    );
    assert!(document.active_mesh().unwrap().masks[painted.masks.len()..]
        .iter()
        .any(|&m| m > 0.0 && m < 1.0));
    let subdivided = document.active_mesh().unwrap().clone();
    let reloaded = Document::from_json(&document.to_json().unwrap()).unwrap();
    assert_surface_equal(reloaded.active_mesh().unwrap(), &subdivided);
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &painted);
    assert!(document.redo());
    assert_surface_equal(document.active_mesh().unwrap(), &subdivided);

    // Import is a separate history operation; undo restores subdivision and masks.
    document.replace_mesh(obj.clone()).unwrap();
    assert_surface_equal(document.active_mesh().unwrap(), &obj);
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &subdivided);
    assert!(document.redo());
    assert_surface_equal(document.active_mesh().unwrap(), &obj);
    assert_eq!(document.name, "Workflow regression sculpture");
}

#[test]
fn project_load_and_mesh_replacement_rebuild_derived_normals() {
    let original = Mesh::sphere(2);
    let mut stale = original.clone();
    stale.normals.fill(-Vec3::Z);
    let serialized = Document::new(stale.clone()).to_json().unwrap();
    let loaded = Document::from_json(&serialized).unwrap();
    assert_surface_equal(loaded.active_mesh().unwrap(), &original);
    let mut document = Document::new(Mesh::cube());
    document.replace_mesh(stale).unwrap();
    assert_surface_equal(document.active_mesh().unwrap(), &original);
    document.begin_stroke();
    let hit = visible_hit(document.active_mesh().unwrap(), 0.0, 0.0);
    assert!(
        document.active_mesh_mut().unwrap().apply_brush(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &settings(BrushKind::Draw)
        ) > 0
    );
    document.end_stroke();
    assert!(document.undo());
    assert_surface_equal(document.active_mesh().unwrap(), &original);
}
