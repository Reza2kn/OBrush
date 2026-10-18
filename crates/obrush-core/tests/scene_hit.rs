//! Source-only adapter gates; no browser or rendering proof.
use glam::{DMat4, DVec3};
use obrush_core::{Document, Mesh, ObjectRenderTransform, Ray, SceneRay64, SglPickingPolicy, Vec3};
const AFFINE: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
fn imported() -> Document {
    let mut d = Document::empty();
    d.replace_sgl_bytes(AFFINE, &Default::default()).unwrap();
    d
}
fn ray() -> SceneRay64 {
    SceneRay64 {
        origin: DVec3::new(2.5, -0.5, 3.0),
        direction: -DVec3::Z,
    }
}
fn plane() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()
}
#[test]
fn world_metric_picks_reflected_quad_at_exact_world_point() {
    let d = imported();
    let hit = d
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    assert_eq!(hit.id, 1);
    assert_eq!(hit.triangle_index, 0);
    assert_eq!(hit.local_position, DVec3::new(1.0, 0.5, 0.0));
    assert_eq!(hit.world_position, DVec3::new(2.5, -0.5, 1.0));
    assert_eq!(hit.world_distance, 2.0);
    assert_eq!(hit.local_distance, 4.0);
    assert_eq!(hit.barycentric, [0.5, 0.0, 0.5]);
}
#[test]
fn source_scale_row_metric_selects_different_farther_object() {
    let d = imported();
    let hit = d
        .raycast_scene(ray(), SglPickingPolicy::SourceScaleRowV1)
        .unwrap()
        .unwrap();
    assert_eq!(hit.id, 2);
    assert_eq!(hit.world_distance, 2.5);
    assert_eq!(hit.source_selection_distance, 2.5);
}
#[test]
fn source_scale_metric_does_not_use_raw_redundant_scale() {
    let mut d = imported();
    d.set_subtool_visible(2, false).unwrap();
    let h = d
        .raycast_scene(ray(), SglPickingPolicy::SourceScaleRowV1)
        .unwrap()
        .unwrap();
    assert_eq!(h.source_selection_distance, 4.0 * 5.0_f64.sqrt());
    assert_ne!(h.source_selection_distance, 4.0 * 9.0);
}
#[test]
fn reflection_separates_shading_and_oriented_face_normals() {
    let h = imported()
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    assert_eq!(h.local_shading_normal, DVec3::Z);
    assert_eq!(h.world_shading_normal, DVec3::Z);
    assert_eq!(h.world_oriented_face_normal, -DVec3::Z);
}
#[test]
fn local_brush_hit_does_not_substitute_world_coordinates() {
    let h = imported()
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    let (local, report) = h.local_brush_hit().unwrap();
    assert_eq!(local.position, Vec3::new(1.0, 0.5, 0.0));
    assert_eq!(local.distance, 4.0);
    assert_eq!(local.triangle, [0, 1, 2]);
    assert_eq!(report.maximum_local_position_error, 0.0);
    assert_eq!(report.maximum_barycentric_error, 0.0);
}
#[test]
fn true_empty_scene_has_no_pick_or_geometry() {
    let d = Document::empty();
    assert!(d
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .is_none());
    assert!(d.subtool_render_snapshots().is_empty());
}
#[test]
fn legacy_indexed_picker_retains_equal_face_row_and_bitwise_hit() {
    let mut mesh = plane();
    mesh.triangles.push([0, 1, 2]);
    let r = Ray {
        origin: Vec3::new(0.25, 0.25, 2.0),
        direction: -Vec3::Z * 3.7,
    };
    let (index, expected) = mesh.raycast_indexed(r).unwrap();
    let d = Document::new(mesh);
    let h = d
        .raycast_scene(r.into(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    assert_eq!(h.triangle_index, index);
    assert_eq!(index, 0);
    let (actual, report) = h.local_brush_hit().unwrap();
    assert_eq!(
        actual.position.to_array().map(f32::to_bits),
        expected.position.to_array().map(f32::to_bits)
    );
    assert_eq!(
        actual.normal.to_array().map(f32::to_bits),
        expected.normal.to_array().map(f32::to_bits)
    );
    assert_eq!(
        actual.barycentric.map(f32::to_bits),
        expected.barycentric.map(f32::to_bits)
    );
    assert_eq!(actual.distance.to_bits(), expected.distance.to_bits());
    assert_eq!(report.maximum_local_position_error, 0.0);
}
#[test]
fn hidden_and_solo_parts_follow_existing_scene_policy() {
    let mut d = imported();
    d.set_subtool_visible(1, false).unwrap();
    assert_eq!(
        d.raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap()
            .id,
        2
    );
    d.set_subtool_visible(1, true).unwrap();
    d.set_solo(true).unwrap();
    assert_eq!(
        d.raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap()
            .id,
        2
    );
    d.set_subtool_visible(2, false).unwrap();
    assert!(d
        .raycast_scene(ray(), SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .is_none());
}
#[test]
fn plain_overlapping_tie_prefers_active_duplicate() {
    let mut d = Document::new(plane());
    let second = d.duplicate_subtool(1).unwrap();
    let r = SceneRay64 {
        origin: DVec3::new(0.25, 0.25, 1.0),
        direction: -DVec3::Z,
    };
    assert_eq!(
        d.raycast_scene(r, SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap()
            .id,
        second
    );
    d.select_subtool(1).unwrap();
    assert_eq!(
        d.raycast_scene(r, SglPickingPolicy::WorldDistanceV1)
            .unwrap()
            .unwrap()
            .id,
        1
    );
}
#[test]
fn invalid_world_ray_fields_and_zero_direction_are_checked() {
    let d = imported();
    for r in [
        SceneRay64 {
            origin: DVec3::splat(f64::NAN),
            ..ray()
        },
        SceneRay64 {
            origin: DVec3::splat(1_000_001.0),
            ..ray()
        },
        SceneRay64 {
            direction: DVec3::ZERO,
            ..ray()
        },
        SceneRay64 {
            direction: DVec3::splat(f64::INFINITY),
            ..ray()
        },
    ] {
        assert!(d
            .raycast_scene(r, SglPickingPolicy::WorldDistanceV1)
            .is_err());
    }
}
#[test]
fn source_render_descriptor_keeps_pose_flags_and_reflection() {
    let mut d = imported();
    d.select_subtool(1).unwrap();
    let m = d.active_object_metadata().unwrap();
    let r = ObjectRenderTransform::for_object(Some(m)).unwrap();
    assert_eq!(r.model, m.affine.local_to_world);
    assert!(r.reflected);
    assert_eq!(r.render.as_ref().unwrap().matcap, 3);
    assert_eq!(r.render, m.render);
    let normal = glam::Mat3::from_cols_array(&r.inverse_transpose_normal) * Vec3::Z;
    assert_eq!(normal.normalize(), Vec3::Z);
}
#[test]
fn identity_render_descriptor_keeps_old_local_math() {
    let r = ObjectRenderTransform::for_object(None).unwrap();
    assert_eq!(r.model, glam::Mat4::IDENTITY.to_cols_array());
    assert_eq!(
        r.inverse_transpose_normal,
        glam::Mat3::IDENTITY.to_cols_array()
    );
    assert!(!r.reflected);
    assert!(r.render.is_none());
}
#[test]
fn model_view_removes_large_world_offset_before_f32_gpu_boundary() {
    let d = Document::empty();
    assert!(d.is_empty());
    let mut transform = ObjectRenderTransform::for_object(None).unwrap();
    transform.model[12] = 100_000.0;
    let view = DMat4::from_translation(DVec3::new(-100_000.0 - 0.00390625, 0.0, 0.0));
    let (mv, _) = transform.model_view(view.to_cols_array()).unwrap();
    let local = Vec3::new(0.00390625, 0.0, 0.0);
    let direct = glam::Mat4::from_cols_array(&transform.model).transform_point3(local);
    assert_eq!(direct.x, 100_000.0);
    let rebased = glam::Mat4::from_cols_array(&mv).transform_point3(local);
    assert_eq!(rebased, Vec3::ZERO);
    assert_eq!(mv[12], -0.00390625);
}
#[test]
fn singular_nonfinite_or_projective_view_is_rejected() {
    let r = ObjectRenderTransform::for_object(None).unwrap();
    let mut singular = DMat4::IDENTITY.to_cols_array();
    singular[0] = 0.0;
    assert!(r.model_view(singular).is_err());
    let mut invalid = DMat4::IDENTITY.to_cols_array();
    invalid[12] = f64::NAN;
    assert!(r.model_view(invalid).is_err());
    let mut projective = DMat4::IDENTITY.to_cols_array();
    projective[3] = 0.5;
    assert!(r.model_view(projective).is_err());
}

#[test]
fn encoded_modern_shader_zero_is_pbr_not_matcap() {
    let mut d = imported();
    d.select_subtool(1).unwrap();
    let appearance = obrush_core::resolve_source_appearance(
        d.active_object_metadata(),
        &obrush_core::baseline_source_render_defaults(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(appearance.role, obrush_core::SourceShaderRole::Pbr);
    assert_eq!(appearance.settings.shader, 0);
    assert_eq!(
        appearance.origin,
        obrush_core::SourceAppearanceOrigin::EncodedOrEditable
    );
}
#[test]
fn legacy_absence_uses_constructor_matcap_five_without_persisting_invented_fields() {
    let mut d = Document::empty();
    d.replace_sgl_bytes(
        include_bytes!("fixtures/independent-legacy1-absent-fields.sgl"),
        &Default::default(),
    )
    .unwrap();
    let before = d.to_json().unwrap();
    assert!(d.active_object_metadata().unwrap().render.is_none());
    let appearance = obrush_core::resolve_source_appearance(
        d.active_object_metadata(),
        &obrush_core::baseline_source_render_defaults(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(appearance.settings.shader, 5);
    assert_eq!(appearance.settings.matcap, 3);
    assert_eq!(appearance.settings.opacity, 1.0);
    assert_eq!(appearance.role, obrush_core::SourceShaderRole::Matcap);
    assert_eq!(
        appearance.origin,
        obrush_core::SourceAppearanceOrigin::UnencodedConstructorPolicy
    );
    assert!(d.active_object_metadata().unwrap().render.is_none());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn legacy_constructor_url_override_is_an_explicit_policy_argument() {
    let mut d = Document::empty();
    d.replace_sgl_bytes(
        include_bytes!("fixtures/independent-legacy1-absent-fields.sgl"),
        &Default::default(),
    )
    .unwrap();
    let mut defaults = obrush_core::baseline_source_render_defaults();
    defaults.shader = 0;
    defaults.matcap = 7;
    let a = obrush_core::resolve_source_appearance(d.active_object_metadata(), &defaults)
        .unwrap()
        .unwrap();
    assert_eq!(a.role, obrush_core::SourceShaderRole::Pbr);
    assert_eq!(a.settings.matcap, 7);
}
#[test]
fn ordinary_appearance_none_does_not_force_source_defaults() {
    assert!(obrush_core::resolve_source_appearance(
        None,
        &obrush_core::baseline_source_render_defaults()
    )
    .unwrap()
    .is_none());
}
#[test]
fn modern_appearance_omission_and_unknown_codes_are_explicit_errors() {
    let mut d = imported();
    d.select_subtool(1).unwrap();
    let mut m = d.active_object_metadata().unwrap().clone();
    m.render = None;
    assert!(obrush_core::resolve_source_appearance(
        Some(&m),
        &obrush_core::baseline_source_render_defaults()
    )
    .is_err());
    assert!(obrush_core::SourceShaderRole::from_source_code(13).is_err());
}
