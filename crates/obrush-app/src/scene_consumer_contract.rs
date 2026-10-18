//! Independent consumer fixtures; no GL execution or reference appearance claim.
use super::{
    dab_queue::DabQueue,
    matcap,
    renderer::{
        Camera, PreviewAffine, ShadingMode, ViewportStyle, consumer_matrices,
        consumer_matrices_previewed, consumer_matrices_scaled, consumer_style,
    },
    scene_import, scene_space,
};
use glam::{DMat4, DVec3, Mat4, Vec2, Vec3};
use obrush_core::{
    BrushSettings, Document, Mesh, ObjectRenderTransform, SceneRay64, SglPickingPolicy,
};
fn word(bytes: &mut Vec<u8>, v: u32) {
    bytes.extend_from_slice(&v.to_le_bytes());
}
fn float(bytes: &mut Vec<u8>, v: f32) {
    word(bytes, v.to_bits());
}
fn scene(shader: u32, matcap: u32, matrix: [f32; 16], n: u32) -> Vec<u8> {
    let mut b = Vec::new();
    // Independent sourcev3 layout: version/display/camera/count. No codec encoder.
    for v in [3, 1, 0, 1, 0, 0] {
        word(&mut b, v);
    }
    float(&mut b, 40.0);
    word(&mut b, 1);
    word(&mut b, n);
    for _ in 0..n {
        for v in [shader, matcap, 0, 0] {
            word(&mut b, v);
        }
        float(&mut b, 1.0);
        for v in [9.0, 8.0, 7.0] {
            float(&mut b, v);
        } // Encoded center differs from derived local center.
        for v in matrix {
            float(&mut b, v);
        }
        float(&mut b, 99.0); // Redundant scale retained, not used.
        word(&mut b, 4);
        for p in [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ] {
            for v in p {
                float(&mut b, v);
            }
        }
        word(&mut b, 4);
        for rgb in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ] {
            for v in rgb {
                float(&mut b, v);
            }
        }
        word(&mut b, 4);
        for (i, z) in [1u32, 0x8000_0000, 0x3f00_0000, 0x3f80_0000]
            .into_iter()
            .enumerate()
        {
            float(&mut b, i as f32 * 0.25);
            float(&mut b, 1.0 - i as f32 * 0.25);
            word(&mut b, z);
        }
        word(&mut b, 1);
        for v in [0, 1, 2, 3] {
            word(&mut b, v);
        }
        word(&mut b, 4);
        for uv in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
            for v in uv {
                float(&mut b, v);
            }
        }
        word(&mut b, 1);
        for v in [0, 1, 2, 3] {
            word(&mut b, v);
        }
    }
    b
}
fn imported(matrix: [f32; 16]) -> Document {
    let p = scene_import::prepare(
        "independent.sgl",
        &scene(5, 3, matrix, 1),
        false,
        &ViewportStyle::default(),
    )
    .unwrap();
    let mut d = Document::empty();
    scene_import::install(&mut d, p).unwrap();
    d
}
fn plain() -> Document {
    Document::new(Mesh::sphere(0))
}

#[test]
fn actual_consumer_preflight_and_install_preserve_quad_uv_material_raw_mask_and_origin() {
    let d = imported(Mat4::from_translation(Vec3::new(12.0, 3.0, 0.0)).to_cols_array());
    let m = d.require_active_mesh().unwrap();
    assert_eq!(m.positions[0], Vec3::new(-1.0, -1.0, 0.0));
    assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(m.surface.as_ref().unwrap().faces, vec![[0, 1, 2, 3]]);
    assert_eq!(
        m.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        vec![1, 0x8000_0000, 0x3f00_0000, 0x3f80_0000]
    );
    assert_eq!(m.materials.as_ref().unwrap()[2], [0.5, 0.5]);
    let meta = d.active_object_metadata().unwrap();
    assert_eq!(meta.affine.interaction_center, [0.0, 0.0, 0.0]);
    assert_eq!(
        meta.origin.as_ref().unwrap().object.raw.center,
        [9.0, 8.0, 7.0]
    );
    assert_eq!(meta.origin.as_ref().unwrap().object.raw.scale, 99.0);
    assert_eq!(
        consumer_style(Some(meta), &ViewportStyle::default())
            .unwrap()
            .shading,
        ShadingMode::Matcap
    );
}
#[test]
fn consumer_empty_sgl_is_real_none_and_whole_scene_undo_restores_original() {
    let mut d = plain();
    let before = d.to_json().unwrap();
    let p = scene_import::prepare(
        "empty.sgl",
        &scene(5, 3, Mat4::IDENTITY.to_cols_array(), 0),
        false,
        &ViewportStyle::default(),
    )
    .unwrap();
    scene_import::install(&mut d, p).unwrap();
    assert!(d.is_empty());
    assert_eq!(d.active_subtool_id(), None);
    assert_eq!(d.level_count(), 0);
    assert!(d.subtool_render_snapshots().is_empty());
    let empty = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), empty);
}
#[test]
fn empty_scene_accepts_real_primitive_and_deleting_last_returns_to_none() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    let id = d.add_subtool(Mesh::sphere(0), "Sphere").unwrap();
    assert!(d.active_mesh().is_some());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    d.delete_subtool(id).unwrap();
    assert!(d.is_empty());
    assert!(d.undo());
    assert_eq!(d.active_subtool_id(), Some(id));
}
#[test]
fn malformed_late_sgl_object_preflight_keeps_existing_redo() {
    let mut d = plain();
    d.add_subtool(Mesh::sphere(0), "second").unwrap();
    d.undo();
    let before = d.to_json().unwrap();
    let mut b = scene(5, 3, Mat4::IDENTITY.to_cols_array(), 2);
    b.truncate(b.len() - 8);
    assert!(scene_import::prepare("truncated.sgl", &b, false, &ViewportStyle::default()).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn source_shader_zero_is_pbr_five_is_actual_matcap_and_auxiliary_rejects() {
    for (code, expected) in [
        (0, ShadingMode::Pbr),
        (2, ShadingMode::Normal),
        (4, ShadingMode::Uv),
        (5, ShadingMode::Matcap),
    ] {
        let p = scene_import::prepare(
            "shader.sgl",
            &scene(code, 3, Mat4::IDENTITY.to_cols_array(), 1),
            false,
            &ViewportStyle::default(),
        )
        .unwrap();
        let mut d = Document::empty();
        scene_import::install(&mut d, p).unwrap();
        assert_eq!(
            consumer_style(d.active_object_metadata(), &ViewportStyle::default())
                .unwrap()
                .shading,
            expected
        );
    }
    assert!(
        scene_import::prepare(
            "aux.sgl",
            &scene(9, 3, Mat4::IDENTITY.to_cols_array(), 1),
            false,
            &ViewportStyle::default()
        )
        .unwrap_err_for_test()
        .contains("auxiliary")
    );
}
// Avoid Debug requirements on private prepared scene payloads.
trait ErrorText {
    fn unwrap_err_for_test(self) -> String;
}
impl<T> ErrorText for Result<T, String> {
    fn unwrap_err_for_test(self) -> String {
        match self {
            Err(e) => e,
            Ok(_) => panic!("expected rejection"),
        }
    }
}
#[test]
fn exact_nine_matcap_presets_decode_dimensions_and_keep_power_of_two_policy() {
    let expected = [
        (512, 512),
        (177, 178),
        (749, 749),
        (749, 749),
        (256, 256),
        (256, 256),
        (256, 256),
        (256, 256),
        (256, 256),
    ];
    for (i, size) in expected.into_iter().enumerate() {
        let t = matcap::preset(i as u32).unwrap();
        assert_eq!((t.width, t.height), size);
        assert_eq!(
            t.pixels_bottom_up.len(),
            size.0 as usize * size.1 as usize * 4
        );
        assert!(std::sync::Arc::ptr_eq(
            &t,
            &matcap::preset(i as u32).unwrap()
        ));
    }
    assert!(matcap::preset(9).is_err());
}
#[test]
fn matcap_off_axis_scalar_keeps_deliberately_unnormalized_basis() {
    let normal = DVec3::new(0.0, 1.0, 0.0);
    let uv = matcap::lookup(normal, DVec3::new(3.0, 4.0, -12.0));
    // z=(3,4,-12)/13; x=(12,0,3)/13; y=(-12,153,48)/169.
    assert_eq!(uv.x, 0.5);
    assert!((uv.y - (0.5 + 0.5 * 153.0 / 169.0)).abs() < 1e-15);
    assert!((uv.y - 1.0).abs() > 0.04);
}
#[test]
fn world_camera_ray_hits_source_local_quad_and_row_exactly() {
    let d = imported(
        Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 3.0, 1.0),
            glam::Quat::IDENTITY,
            Vec3::new(100_000.0, 0.0, 0.0),
        )
        .to_cols_array(),
    );
    let camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        distance: 4.0,
        target: Vec3::new(100_000.0, 0.0, 0.0),
        precise_target: Some(DVec3::new(100_000.000_1, 0.0, 0.0)),
        ..Default::default()
    };
    let ray = camera.ray64(Vec2::ZERO, 1.0);
    assert!((ray.origin.x - 100_000.000_1).abs() < 1e-12);
    let hit = d
        .raycast_scene(ray, SglPickingPolicy::WorldDistanceV1)
        .unwrap()
        .unwrap();
    assert!((hit.local_position.x - 0.000_05).abs() < 1e-11);
    assert_eq!(hit.world_position.x, ray.origin.x);
    assert!([0, 1].contains(&hit.triangle_index));
}
#[test]
fn renderer_composes_large_model_and_camera_translation_before_f32() {
    let d = imported(Mat4::from_translation(Vec3::new(100_000.0, 0.0, 0.0)).to_cols_array());
    let pose = ObjectRenderTransform::for_object(d.active_object_metadata()).unwrap();
    let camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        distance: 4.0,
        target: Vec3::new(100_000.0, 0.0, 0.0),
        precise_target: Some(DVec3::new(100_000.000_1, 0.0, 0.0)),
        ..Default::default()
    };
    let (_, mv, _, preview) = consumer_matrices(&camera, 1.0, &pose, true, Vec3::ZERO).unwrap();
    assert!((f64::from(mv[12]) + 0.000_1).abs() < 1e-11);
    assert_eq!(preview, Vec3::ZERO);
}
#[test]
fn ordinary_consumer_matrices_keep_legacy_f32_bits_and_preview() {
    let camera = Camera::default();
    let pose = ObjectRenderTransform::for_object(None).unwrap();
    let preview = Vec3::new(0.1, 0.2, 0.3);
    let (vp, mv, n, delta) = consumer_matrices(&camera, 1.7, &pose, false, preview).unwrap();
    assert_eq!(
        vp.map(f32::to_bits),
        camera
            .view_projection(1.7)
            .to_cols_array()
            .map(f32::to_bits)
    );
    assert_eq!(
        mv.map(f32::to_bits),
        camera.view().to_cols_array().map(f32::to_bits)
    );
    assert_eq!(
        n.map(f32::to_bits),
        glam::Mat3::from_mat4(camera.view())
            .to_cols_array()
            .map(f32::to_bits)
    );
    assert_eq!(delta, preview);
}
#[test]
fn posed_preview_translation_is_world_space_with_inverse_transpose_normal() {
    let d = imported(Mat4::from_scale(Vec3::new(-2.0, 3.0, 1.0)).to_cols_array());
    let pose = ObjectRenderTransform::for_object(d.active_object_metadata()).unwrap();
    assert!(pose.reflected);
    let camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    };
    let (_, mv, n, preview) =
        consumer_matrices(&camera, 1.0, &pose, true, Vec3::new(1.0, 2.0, 0.0)).unwrap();
    assert_eq!(mv[12], 1.0);
    assert_eq!(mv[13], 2.0);
    assert_eq!(n[0], -0.5);
    assert!((n[4] - 1.0 / 3.0).abs() < 1e-7);
    assert_eq!(preview, Vec3::ZERO);
}
#[test]
fn scale_preview_composes_world_pivot_with_imported_pose_and_normal() {
    let d = imported(Mat4::from_scale(Vec3::new(-2.0, 3.0, 1.0)).to_cols_array());
    let pose = ObjectRenderTransform::for_object(d.active_object_metadata()).unwrap();
    let camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    };
    let (_, mv, normal, shader_translation) = consumer_matrices_scaled(
        &camera,
        1.0,
        &pose,
        true,
        Vec3::ZERO,
        Vec3::new(1.5, 1.0, 1.0),
        Vec3::X,
    )
    .unwrap();
    let eye = Mat4::from_cols_array(&mv).transform_point3(Vec3::X);
    assert!((eye.x + 3.5).abs() < 1.0e-6);
    assert!((normal[0] + 1.0 / 3.0).abs() < 1.0e-7);
    assert_eq!(shader_translation, Vec3::ZERO);
    let plain = consumer_matrices(&camera, 1.0, &pose, true, Vec3::ZERO).unwrap();
    let identity =
        consumer_matrices_scaled(&camera, 1.0, &pose, true, Vec3::ZERO, Vec3::ONE, Vec3::X)
            .unwrap();
    assert_eq!(plain, identity);
}
#[test]
fn rotation_preview_moves_world_point_around_pivot_and_rotates_normal() {
    let d = imported(Mat4::IDENTITY.to_cols_array());
    let pose = ObjectRenderTransform::for_object(d.active_object_metadata()).unwrap();
    let camera = Camera::default();
    let (_, mv, normal, shader_translation) = consumer_matrices_previewed(
        &camera,
        1.0,
        &pose,
        true,
        PreviewAffine {
            translation: Vec3::ZERO,
            scale: Vec3::ONE,
            rotation_degrees: Vec3::new(0.0, 0.0, 90.0),
            pivot: Vec3::X,
        },
    )
    .unwrap();
    let actual = Mat4::from_cols_array(&mv).transform_point3(Vec3::new(2.0, 0.0, 0.0));
    let expected = camera.view().transform_point3(Vec3::new(1.0, 1.0, 0.0));
    assert!(actual.abs_diff_eq(expected, 2.0e-6));
    let rotated_normal = glam::Mat3::from_cols_array(&normal) * Vec3::X;
    let expected_normal = camera.view().transform_vector3(Vec3::Y);
    assert!(rotated_normal.abs_diff_eq(expected_normal, 2.0e-6));
    assert_eq!(shader_translation, Vec3::ZERO);
}
#[test]
fn affine_vector_and_source_radius_do_not_mutate_local_channels() {
    let d = imported(Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0)).to_cols_array());
    let before = d.to_json().unwrap();
    assert_eq!(
        scene_space::local_vector(&d, Vec3::new(2.0, 3.0, 4.0)).unwrap(),
        Vec3::ONE
    );
    assert_eq!(scene_space::local_radius(&d, 0.4).unwrap(), 0.2);
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn render_edit_is_one_undo_raw_origin_stays_immutable_and_noop_keeps_redo() {
    let mut d = imported(Mat4::IDENTITY.to_cols_array());
    let before = d.to_json().unwrap();
    let origin = d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw
        .clone();
    let mut r = d.active_object_metadata().unwrap().render.clone().unwrap();
    r.matcap = 8;
    r.opacity = 0.5;
    assert!(d.set_active_source_render(r.clone()).unwrap());
    let after = d.to_json().unwrap();
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .render
            .as_ref()
            .unwrap()
            .matcap,
        3
    );
    assert!(std::sync::Arc::ptr_eq(
        &origin,
        &d.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
    ));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    let old = d.active_object_metadata().unwrap().render.clone().unwrap();
    assert!(!d.set_active_source_render(old).unwrap());
    assert!(d.can_redo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn invalid_appearance_edit_is_atomic_and_does_not_invent_active_for_empty() {
    let mut d = imported(Mat4::IDENTITY.to_cols_array());
    let before = d.to_json().unwrap();
    let mut r = d.active_object_metadata().unwrap().render.clone().unwrap();
    r.opacity = f32::NAN;
    assert!(d.set_active_source_render(r).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    let mut empty = Document::empty();
    assert!(
        empty
            .set_active_source_render(obrush_core::baseline_source_render_defaults())
            .is_err()
    );
    assert!(empty.is_empty());
}
#[test]
fn affine_queue_preserves_release_and_one_complete_scene_undo() {
    let mut d = imported(
        Mat4::from_scale_rotation_translation(
            Vec3::splat(2.0),
            glam::Quat::IDENTITY,
            Vec3::new(12.0, 0.0, 0.0),
        )
        .to_cols_array(),
    );
    let before = d.to_json().unwrap();
    let mut q = DabQueue::default();
    q.begin();
    let settings = BrushSettings {
        radius: 2.0,
        strength: 0.1,
        symmetry_x: false,
        adaptive_detail: false,
        draw_accumulate: true,
        ..Default::default()
    };
    for _ in 0..8 {
        q.dab_scene_with_footprint(
            SceneRay64 {
                origin: DVec3::new(12.0, 0.0, 3.0),
                direction: -DVec3::Z,
            },
            settings.clone(),
            Vec3::ZERO,
            None,
        );
        q.process(&mut d, 1, None);
    }
    let progress = q.release(&mut d);
    assert!(progress.error.is_none());
    assert!(q.is_empty());
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn queue_empty_scene_has_no_hit_no_edit_or_phantom_history() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    let mut q = DabQueue::default();
    q.begin();
    q.dab_scene_with_footprint(
        SceneRay64 {
            origin: DVec3::Z * 3.0,
            direction: -DVec3::Z,
        },
        BrushSettings::default(),
        Vec3::ZERO,
        None,
    );
    q.release(&mut d);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
#[test]
fn camera_frames_transformed_points_and_empty_retains_previous_camera() {
    let d = imported(Mat4::from_translation(Vec3::new(12.0, 3.0, 0.0)).to_cols_array());
    let mut c = Camera::default();
    c.frame_scene(&d.subtool_render_snapshots());
    assert_eq!(c.target, Vec3::new(12.0, 3.0, 0.0));
    let before = (c.target, c.distance, c.precise_target);
    c.frame_scene(&[]);
    assert_eq!((c.target, c.distance, c.precise_target), before);
}
#[test]
fn source_fov_is_vertical_and_unclamped_while_ordinary_policy_is_unchanged() {
    let source = Camera {
        fov_degrees: 120.0,
        source_vertical_fov: true,
        ..Default::default()
    };
    assert!((source.vertical_fov(0.3) - 120f32.to_radians()).abs() < 1e-7);
    let ordinary = Camera {
        fov_degrees: 120.0,
        ..Default::default()
    };
    assert!((ordinary.vertical_fov(1.0) - 90f32.to_radians()).abs() < 1e-7);
}
#[test]
fn affine_normal_ray_adapter_keeps_local_world_distinction_under_shear() {
    let mut m = Mat4::IDENTITY.to_cols_array();
    m[4] = 0.5;
    m[12] = 12.0;
    let d = imported(m);
    let ray = SceneRay64 {
        origin: DVec3::new(12.25, 0.5, 3.0),
        direction: -DVec3::Z,
    };
    let local = scene_space::local_ray(&d, ray).unwrap();
    assert_eq!(local.origin, DVec3::new(0.0, 0.5, 3.0));
    assert_eq!(local.direction, -DVec3::Z);
    assert_eq!(
        DMat4::from_cols_array(&m.map(f64::from)).transform_point3(local.origin),
        ray.origin
    );
}

#[test]
fn encoded_uv_without_uv_resolves_source_matcap_without_rewriting_raw_shader() {
    let mut b = scene(4, 3, Mat4::IDENTITY.to_cols_array(), 1);
    // Final UV block is count+four vec2s+count+four corner indices =56 bytes.
    b.truncate(b.len() - 56);
    word(&mut b, 0);
    word(&mut b, 0);
    let p = scene_import::prepare("no-uv.sgl", &b, false, &ViewportStyle::default()).unwrap();
    let mut d = Document::empty();
    scene_import::install(&mut d, p).unwrap();
    let m = d.require_active_mesh().unwrap();
    let meta = d.active_object_metadata().unwrap();
    assert_eq!(meta.render.as_ref().unwrap().shader, 4);
    assert_eq!(
        super::renderer::consumer_style_for_mesh(Some(meta), m, &ViewportStyle::default())
            .unwrap()
            .shading,
        ShadingMode::Matcap
    );
    assert_eq!(
        meta.origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .render
            .as_ref()
            .unwrap()
            .shader,
        4
    );
}
#[test]
fn legacy_absent_render_uses_constructor_matcap5_index3_and_keeps_none_exact() {
    let modern = scene(0, 0, Mat4::IDENTITY.to_cols_array(), 1);
    let mut legacy = Vec::new();
    word(&mut legacy, 1);
    word(&mut legacy, 1);
    // Independent v1 excludes modern36-byte header and first20-byte render block.
    legacy.extend_from_slice(&modern[56..]);
    let p = scene_import::prepare("legacy.sgl", &legacy, false, &ViewportStyle::default()).unwrap();
    let mut d = Document::empty();
    scene_import::install(&mut d, p).unwrap();
    let meta = d.active_object_metadata().unwrap();
    assert!(meta.render.is_none());
    let effective = consumer_style(Some(meta), &ViewportStyle::default()).unwrap();
    assert_eq!(effective.shading, ShadingMode::Matcap);
    assert_eq!(effective.matcap_index, 3);
    assert!(meta.origin.as_ref().unwrap().object.raw.render.is_none());
    let json = d.to_json().unwrap();
    let again = Document::from_json(&json).unwrap();
    assert!(again.active_object_metadata().unwrap().render.is_none());
    assert_eq!(again.to_json().unwrap(), json);
}
#[test]
fn bounded_jpeg_decoder_rejects_signature_truncation_and_input_cap() {
    for b in [
        &b"not-jpeg"[..],
        &[255, 216, 255, 217][..],
        &[255, 216, 255, 192, 0, 17, 8, 0][..],
    ] {
        assert!(super::uv_view::decode_jpeg("bad", b).is_err());
    }
    let hostile = vec![0u8; 16 * 1024 * 1024 + 1];
    assert!(super::uv_view::decode_jpeg("large", &hostile).is_err());
}

#[test]
fn scene_policy_does_not_change_when_an_unposed_appended_part_becomes_active() {
    let mut d = imported(Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0)).to_cols_array());
    let imported_id = d.active_subtool_id().unwrap();
    assert_eq!(
        scene_space::picking_policy(&d).unwrap(),
        SglPickingPolicy::SourceScaleRowV1
    );
    let added = d.add_subtool(Mesh::sphere(0), "plain").unwrap();
    assert_eq!(d.active_subtool_id(), Some(added));
    assert!(d.active_object_metadata().is_none());
    assert_eq!(
        scene_space::picking_policy(&d).unwrap(),
        SglPickingPolicy::SourceScaleRowV1
    );
    d.select_subtool(imported_id).unwrap();
    assert_eq!(
        scene_space::picking_policy(&d).unwrap(),
        SglPickingPolicy::SourceScaleRowV1
    );
}
#[test]
fn borrowed_member_preflight_includes_hidden_members_without_mutating_geometry() {
    let mut d = imported(Mat4::IDENTITY.to_cols_array());
    let id = d.active_subtool_id().unwrap();
    d.add_subtool(Mesh::sphere(0), "plain").unwrap();
    d.set_subtool_visible(id, false).unwrap();
    let before = d.to_json().unwrap();
    assert_eq!(d.subtool_mesh(id).unwrap().positions.len(), 4);
    assert!(d.subtool_object_metadata(id).unwrap().is_some());
    assert!(d.subtool_render_snapshots().iter().all(|s| s.id != id));
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.subtool_mesh(0).is_err());
}
