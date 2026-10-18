//! Additive workflow contracts using existing independently written legacy dabs.
//! Screen/source preparation and reference brush kernels are separate gates.
use obrush_core::{
    BrushKind, BrushSettings, Document, LockedAccumulationAction, LockedAccumulationSample,
    LockedAccumulator, LockedStamp, LockedStampSample, MaskConvention, Mesh, Vec3,
};
fn sheet() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()
}
fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 2.,
        strength: 1.,
        hardness: 1.,
        pressure_strength: false,
        symmetry_x: false,
        backface_culling: false,
        adaptive_detail: false,
        color: [1.; 3],
        ..Default::default()
    }
}
fn sample(radius: f32) -> LockedAccumulationSample {
    LockedAccumulationSample {
        center: Vec3::ZERO,
        normal: Vec3::Z,
        radius,
        pressure: 1.,
        alpha_rotation: 0.,
        alpha_tangent: Some(Vec3::X),
        culling_axis: Some(Vec3::Z),
    }
}
fn exact(a: &Mesh, b: &Mesh) {
    assert_eq!(
        serde_json::to_string(a).unwrap(),
        serde_json::to_string(b).unwrap()
    );
}
fn seeded_redo() -> Document {
    let mut d = Document::new(sheet());
    d.invert_mask();
    assert!(d.undo());
    assert!(d.can_redo());
    d
}
#[test]
fn independent_dyadic_paint_recurrence_accumulates_identical_ticks() {
    let mut m = sheet();
    m.colors.fill([0.; 3]);
    let s = settings(BrushKind::Paint);
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    // Exact closed form 1-(3/4)^n, not another call to the brush implementation.
    for expected in [0.25_f32, 0.4375, 0.578125, 0.68359375] {
        let r = a.apply(&mut m, sample(2.)).unwrap();
        assert_eq!(r.changed_vertices, 3);
        for c in &m.colors {
            assert_eq!(c.map(f32::to_bits), [expected.to_bits(); 3]);
        }
    }
    assert_eq!(a.sequence(), 4);
    assert!(!a.at_original());
}
#[test]
fn absolute_locked_preview_and_additive_lock_are_distinct() {
    let mut m = sheet();
    m.colors.fill([0.; 3]);
    let s = settings(BrushKind::Paint);
    let mut preview = m.clone();
    let mut p = LockedStamp::prepare(&preview, Vec3::ZERO, Vec3::Z, &s).unwrap();
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    for _ in 0..2 {
        a.apply(&mut m, sample(2.)).unwrap();
        p.apply(
            &mut preview,
            LockedStampSample {
                radius: 2.,
                ..Default::default()
            },
        )
        .unwrap();
    }
    assert_eq!(m.colors[0], [0.4375; 3]);
    assert_eq!(preview.colors[0], [0.25; 3]);
}
#[test]
fn zero_travel_holds_accumulated_surface_instead_of_restoring() {
    let mut m = sheet();
    let s = settings(BrushKind::Paint);
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    a.apply(&mut m, sample(2.)).unwrap();
    let before = m.clone();
    let r = a.apply(&mut m, sample(0.)).unwrap();
    assert_eq!(r.action, LockedAccumulationAction::Hold);
    assert_eq!(r.changed_vertices, 0);
    exact(&m, &before);
    assert!(!a.at_original());
}
#[test]
fn pressure_zero_holds_previous_accumulation_and_attributes() {
    let mut m = sheet();
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    a.apply(&mut m, sample(2.)).unwrap();
    let before = m.clone();
    let mut p = sample(2.);
    p.pressure = 0.;
    assert_eq!(
        a.apply(&mut m, p).unwrap().action,
        LockedAccumulationAction::Hold
    );
    exact(&m, &before);
}
#[test]
fn cancellation_restores_exact_original_including_source_mask_words() {
    let mut m = sheet();
    m.install_mask_channel(
        vec![f32::from_bits(1), -0., 1.],
        MaskConvention::UnprotectedF32V1,
    )
    .unwrap();
    let original = m.clone();
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    a.apply(&mut m, sample(2.)).unwrap();
    assert!(!a.at_original());
    a.cancel(&mut m).unwrap();
    exact(&m, &original);
    assert!(a.at_original());
}
#[test]
fn repeated_cancellation_is_exact_noop() {
    let mut m = sheet();
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    a.apply(&mut m, sample(2.)).unwrap();
    a.cancel(&mut m).unwrap();
    let original = m.clone();
    let r = a.cancel(&mut m).unwrap();
    assert_eq!(r.changed_vertices, 0);
    exact(&m, &original);
}
#[test]
fn actual_fresh_center_can_change_the_affected_material_point() {
    let mut m = sheet();
    m.colors.fill([0.; 3]);
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    a.apply(&mut m, sample(0.25)).unwrap();
    let mut p = sample(0.25);
    p.center = Vec3::X;
    a.apply(&mut m, p).unwrap();
    assert_eq!(m.colors, [[0.25; 3], [0.25; 3], [0.; 3]]);
}
#[test]
fn externally_changed_mask_rgb_or_normal_rejects_and_does_not_consume_sequence() {
    for field in 0..3 {
        let mut m = sheet();
        let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
        match field {
            0 => m.masks[0] = 0.25,
            1 => m.colors[0][0] = 0.25,
            _ => m.normals[0].x = 0.1,
        };
        let before = m.clone();
        assert!(a.apply(&mut m, sample(2.)).is_err());
        exact(&m, &before);
        assert_eq!(a.sequence(), 0);
    }
}
#[test]
fn external_signed_zero_mask_marker_is_stale_even_in_protected_convention() {
    let mut m = sheet();
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    m.masks[0] = -0.;
    assert!(a.apply(&mut m, sample(2.)).is_err());
    assert_eq!(m.masks[0].to_bits(), 0x80000000);
}
#[test]
fn external_material_presence_or_uv_edits_reject_atomically() {
    let mut d = Document::from_json(include_str!(
        "fixtures/independent-material-uv-seam-v6.obrush"
    ))
    .unwrap();
    let mut a = LockedAccumulator::prepare(
        d.require_active_mesh().unwrap(),
        &settings(BrushKind::Paint),
    )
    .unwrap();
    d.require_active_mesh_mut()
        .unwrap()
        .surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][0] = 0.125;
    let before = d.require_active_mesh_mut().unwrap().clone();
    assert!(a
        .apply(d.require_active_mesh_mut().unwrap(), sample(2.))
        .is_err());
    exact(d.require_active_mesh().unwrap(), &before);
    let mut m = sheet();
    let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
    m.materials = Some(vec![[0.18, 0.08]; 3]);
    assert!(a.apply(&mut m, sample(2.)).is_err());
}
#[test]
fn invalid_sample_rejects_before_mutating_data_or_provenance() {
    for field in 0..6 {
        let mut m = sheet();
        let original = m.clone();
        let mut a = LockedAccumulator::prepare(&m, &settings(BrushKind::Paint)).unwrap();
        let mut p = sample(2.);
        match field {
            0 => p.radius = f32::NAN,
            1 => p.radius = -1.,
            2 => p.pressure = f32::INFINITY,
            3 => p.normal = Vec3::ZERO,
            4 => p.center.x = 1_000_001.,
            _ => p.culling_axis = Some(Vec3::ZERO),
        };
        assert!(a.apply(&mut m, p).is_err());
        exact(&m, &original);
        assert_eq!(a.sequence(), 0);
    }
}
#[test]
fn exposed_tools_supported_and_nonordinary_kinds_explicitly_rejected() {
    for kind in [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Smooth,
        BrushKind::Flatten,
        BrushKind::Crease,
        BrushKind::Pinch,
        BrushKind::Mask,
        BrushKind::Paint,
    ] {
        assert!(LockedAccumulator::prepare(&sheet(), &settings(kind)).is_ok());
    }
    for kind in [
        BrushKind::Move,
        BrushKind::Drag,
        BrushKind::Twist,
        BrushKind::LocalScale,
        BrushKind::Morph,
    ] {
        assert!(LockedAccumulator::prepare(&sheet(), &settings(kind)).is_err());
    }
}
#[test]
fn adaptive_detail_is_explicitly_rejected_and_topology_stays_fixed() {
    let mut s = settings(BrushKind::Draw);
    s.adaptive_detail = true;
    assert!(LockedAccumulator::prepare(&sheet(), &s).is_err());
    s.adaptive_detail = false;
    let mut m = sheet();
    let topology = m.triangles.clone();
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    for _ in 0..4 {
        a.apply(&mut m, sample(0.18)).unwrap();
    }
    assert_eq!(m.triangles, topology);
    assert_eq!(m.positions.len(), 3);
}
#[test]
fn captured_settings_do_not_follow_later_host_mutation() {
    let mut s = settings(BrushKind::Paint);
    s.draw_accumulate = false;
    let mut m = sheet();
    m.colors.fill([0.; 3]);
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    s.color = [0.; 3];
    s.strength = 0.;
    a.apply(&mut m, sample(2.)).unwrap();
    assert_eq!(m.colors[0], [0.25; 3]);
    assert_eq!(a.settings().color, [1.; 3]);
    assert!(a.settings().draw_accumulate);
    assert!(!s.draw_accumulate);
}
#[test]
fn many_ticks_form_one_complete_document_undo_redo_transaction() {
    let mut d = Document::new(sheet());
    let before = d.to_json().unwrap();
    let mut a = LockedAccumulator::prepare(
        d.require_active_mesh().unwrap(),
        &settings(BrushKind::Paint),
    )
    .unwrap();
    d.begin_stroke();
    for _ in 0..8 {
        a.apply(d.require_active_mesh_mut().unwrap(), sample(2.))
            .unwrap();
    }
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn no_channels_and_zero_contact_preserve_redo_and_material_absence() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    let mut s = settings(BrushKind::Paint);
    s.paint_rgb = false;
    let mut a = LockedAccumulator::prepare(d.require_active_mesh().unwrap(), &s).unwrap();
    d.begin_stroke();
    for _ in 0..4 {
        assert_eq!(
            a.apply(d.require_active_mesh_mut().unwrap(), sample(2.))
                .unwrap()
                .changed_vertices,
            0
        );
    }
    let mut p = sample(2.);
    p.pressure = 0.;
    a.apply(d.require_active_mesh_mut().unwrap(), p).unwrap();
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(d.require_active_mesh_mut().unwrap().materials.is_none());
}
#[test]
fn cancelled_complete_stroke_restores_redo_without_new_history() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    let mut a = LockedAccumulator::prepare(
        d.require_active_mesh().unwrap(),
        &settings(BrushKind::Paint),
    )
    .unwrap();
    d.begin_stroke();
    for _ in 0..3 {
        a.apply(d.require_active_mesh_mut().unwrap(), sample(2.))
            .unwrap();
    }
    a.cancel(d.require_active_mesh_mut().unwrap()).unwrap();
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(!d.can_undo());
}
#[test]
fn retained_material_ticks_use_explicit_marker_and_lazy_seed_once() {
    let mut d = Document::new(sheet());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    let mut s = settings(BrushKind::Paint);
    s.paint_rgb = false;
    s.paint_roughness = true;
    s.roughness = 0.8;
    let mut a = LockedAccumulator::prepare(d.require_active_mesh().unwrap(), &s).unwrap();
    d.begin_material_paint_stroke();
    for _ in 0..3 {
        a.apply(d.require_active_mesh_mut().unwrap(), sample(2.))
            .unwrap();
    }
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    assert!(d.require_active_mesh_mut().unwrap().materials.is_some());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn fully_protected_geometry_rgb_material_ticks_are_noops() {
    for kind in [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Smooth,
        BrushKind::Flatten,
        BrushKind::Crease,
        BrushKind::Pinch,
        BrushKind::Paint,
    ] {
        let mut m = sheet();
        m.masks.fill(1.);
        let original = m.clone();
        let mut s = settings(kind);
        s.paint_roughness = true;
        s.roughness = 0.8;
        let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
        for _ in 0..3 {
            assert_eq!(a.apply(&mut m, sample(2.)).unwrap().changed_vertices, 0);
        }
        exact(&m, &original);
        assert!(m.materials.is_none());
    }
}
#[test]
fn strong_repeated_crease_keeps_independent_face_orientation_and_fixed_topology() {
    let mut m = Mesh::sphere(2);
    let original = m.clone();
    let mut s = settings(BrushKind::Crease);
    s.strength = 0.35;
    s.hardness = 0.;
    let mut a = LockedAccumulator::prepare(&m, &s).unwrap();
    let mut p = sample(0.3);
    p.center = Vec3::Z;
    for _ in 0..24 {
        a.apply(&mut m, p).unwrap();
    }
    assert_eq!(m.triangles, original.triangles);
    for &tri in &m.triangles {
        let [a, b, c] = tri.map(|i| original.positions[i as usize].as_dvec3());
        let [x, y, z] = tri.map(|i| m.positions[i as usize].as_dvec3());
        assert!((b - a).cross(c - a).dot((y - x).cross(z - x)) > 0.);
    }
}
