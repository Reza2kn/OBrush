//! Independently authored camera/scene/footprint checks. No display or GL needed.
use crate::{footprint::ScreenFootprint, renderer::Camera, source_camera_math::ViewportDimensions};
use glam::{DVec2, DVec3, Mat4, Quat, Vec2, Vec3};
use obrush_core::{
    Document, Mesh, ObjectAffineF32V1, ObjectRenderTransform, SceneObjectMetadata,
    SglPickingPolicy, SubtoolRenderSnapshot,
};
use std::sync::Arc;
fn viewport() -> ViewportDimensions {
    ViewportDimensions::checked([1000, 500], DVec2::new(500.0, 250.0)).unwrap()
}
fn mesh() -> Arc<Mesh> {
    Arc::new(Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3\nf 1 3 4\n").unwrap())
}
fn objects() -> Vec<SubtoolRenderSnapshot> {
    vec![SubtoolRenderSnapshot {
        id: 7,
        active: true,
        mesh: mesh(),
        metadata: None,
    }]
}
fn source(orthographic: bool) -> Camera {
    let mut c = Camera {
        yaw: 0.0,
        pitch: 0.0,
        distance: 4.0,
        fov_degrees: 45.0,
        orthographic,
        ..Default::default()
    };
    c.set_source_framing(true, viewport()).unwrap();
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    c
}

#[test]
fn default_legacy_projection_view_ray_and_curvature_keep_exact_arithmetic() {
    for orthographic in [false, true] {
        for aspect in [0.3_f32, 1.0, 2.5] {
            let c = Camera {
                yaw: 0.35,
                pitch: 0.18,
                distance: 4.0,
                target: Vec3::new(0.5, -0.25, 0.125),
                orthographic,
                ..Default::default()
            };
            assert!(!c.source_enabled());
            let offset = Vec3::new(
                c.yaw.sin() * c.pitch.cos(),
                c.pitch.sin(),
                c.yaw.cos() * c.pitch.cos(),
            );
            let eye = c.target + offset * 4.0;
            assert_eq!(
                c.position().to_array().map(f32::to_bits),
                eye.to_array().map(f32::to_bits)
            );
            let fov = 2.0 * ((40.0_f32.to_radians() * 0.5).tan() / aspect.clamp(0.001, 1.0)).atan();
            let near = 0.004;
            let far = 400.0;
            let projection = if orthographic {
                let h = 4.0 * (fov * 0.5).tan();
                Mat4::orthographic_rh_gl(-h * aspect, h * aspect, -h, h, near, far)
            } else {
                Mat4::perspective_rh_gl(fov, aspect, near, far)
            };
            let view = Mat4::look_at_rh(eye, c.target, Vec3::Y);
            assert_eq!(
                c.projection(aspect).to_cols_array().map(f32::to_bits),
                projection.to_cols_array().map(f32::to_bits)
            );
            assert_eq!(
                c.view_projection(aspect).to_cols_array().map(f32::to_bits),
                (projection * view).to_cols_array().map(f32::to_bits)
            );
            let forward = (c.target - eye).normalize_or_zero();
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            let ndc = Vec2::new(0.35, -0.2);
            let t = (fov * 0.5).tan();
            let ray = c.ray(ndc, aspect);
            let origin = if orthographic {
                eye + right * ndc.x * t * 4.0 * aspect + up * ndc.y * t * 4.0
            } else {
                eye
            };
            let direction = if orthographic {
                forward
            } else {
                (forward + right * ndc.x * t * aspect + up * ndc.y * t).normalize_or_zero()
            };
            assert_eq!(
                ray.origin.to_array().map(f32::to_bits),
                origin.to_array().map(f32::to_bits)
            );
            assert_eq!(
                ray.direction.to_array().map(f32::to_bits),
                direction.to_array().map(f32::to_bits)
            );
            assert_eq!(
                c.curvature_fov(),
                if orthographic {
                    -100.0
                } else {
                    40.0_f32.to_radians().to_degrees()
                }
            );
        }
    }
}
#[test]
fn orthographic_eye_stays_fixed_when_normal_view_height_changes() {
    let mut c = source(true);
    let eye = c.position64();
    let view = c.view64();
    let old = c.source_view_height().unwrap();
    c.set_source_view_height(old * 2.0).unwrap();
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    assert_eq!(c.position64(), eye);
    assert_eq!(eye, DVec3::new(0.0, 0.0, 1000.0));
    assert_eq!(c.view64(), view);
    assert_eq!(c.source_view_height().unwrap(), old * 2.0);
    assert!(
        (c.projection(2.0).y_axis.y - (2.0 / c.source_view_height().unwrap()) as f32).abs() < 1e-6
    );
    assert!(
        c.source_frame()
            .unwrap()
            .project_device(DVec3::ZERO)
            .unwrap()
            .z
            < 1.0
    );
}
#[test]
fn perspective_raw_fov_and_infinite_far_are_shared_by_frame_and_picking() {
    let mut c = source(false);
    c.fov_degrees = 120.0;
    c.set_source_projection_zoom(12.0).unwrap();
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    assert_eq!(c.position64(), DVec3::new(0.0, 0.0, 4.5));
    assert_eq!(c.vertical_fov(0.3), 120.0_f32.to_radians());
    assert_eq!(c.curvature_fov(), 120.0);
    assert_eq!(c.projection(2.0).z_axis.z, -1.0);
    assert_eq!(c.projection(2.0).z_axis.w, -1.0);
    let ray = c.ray64(Vec2::new(0.5, 0.25), 2.0);
    let expected = DVec3::new(0.5 * 3.0f64.sqrt() * 2.0, 0.25 * 3.0f64.sqrt(), -1.0).normalize();
    assert!((ray.direction - expected).length() < 1e-12);
}
#[test]
fn published_frame_snapshot_stays_identical_for_render_pick_and_footprint() {
    let mut c = source(true);
    let snapshot = c.paint_snapshot();
    let frame = snapshot.source_frame().unwrap();
    let old = frame.projection64.to_cols_array();
    let footprint = ScreenFootprint::capture(&snapshot, 75.0, 250.0, 2.0).unwrap();
    let radius = footprint.radius_at(Vec3::ZERO).unwrap();
    let zoom = frame.projection_zoom.value();
    assert_eq!(radius, (75.0 * zoom * 0.0011 * 2.0) as f32);
    c.set_source_projection_zoom(zoom * 0.5).unwrap();
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    assert_eq!(
        snapshot
            .source_frame()
            .unwrap()
            .projection64
            .to_cols_array(),
        old
    );
    assert_ne!(c.source_frame().unwrap().projection64.to_cols_array(), old);
    let ray = snapshot.ray64(Vec2::new(0.25, -0.5), 2.0);
    let point = ray.origin + ray.direction * 2000.0;
    let screen = frame.project_device(point).unwrap();
    assert!((screen.x - 625.0).abs() < 1e-9);
    assert!((screen.y - 375.0).abs() < 1e-9);
    assert_eq!(footprint.radius_at(Vec3::ZERO).unwrap(), radius);
}
#[test]
fn source_left_and_top_use_camera_to_world_inverse_direction() {
    let mut c = source(true);
    for (axis, expected) in [(0, DVec3::Z), (1, DVec3::X), (2, DVec3::Y)] {
        c.rotation = Some(Quat::from_rotation_y(0.2));
        c.axis_view(axis);
        c.prepare_source_frame(viewport(), &objects()).unwrap();
        let direction = c.position64().normalize();
        assert!((direction - expected).length() < 2e-6);
        let (_, _, forward) = c.axes64();
        assert!((forward + expected).length() < 2e-6);
    }
}
#[test]
fn source_pan_uses_current_zoom_and_measured_user_pixel_scale() {
    let mut c = source(true);
    c.set_source_projection_zoom(4.0).unwrap();
    c.pan(eframe::egui::vec2(10.0, -5.0));
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    let expected = DVec3::new(-10.0 * 4.0 * 0.0011 * 2.0, -5.0 * 4.0 * 0.0011 * 2.0, 0.0);
    assert!((c.precise_target.unwrap() - expected).length() < 1e-15);
    assert!((c.position64() - (expected + DVec3::Z * 1000.0)).length() < 1e-12);
}
#[test]
fn affine_local_render_and_f64_ray_share_one_large_offset_frame() {
    let m = mesh();
    let matrix = [
        2.0, 0.0, 0.0, 0.0, 0.5, 3.0, 0.0, 0.0, 0.0, 0.0, 0.75, 0.0, 100_000.0, 7.0, -4.0, 1.0,
    ];
    let affine = ObjectAffineF32V1::from_local_points(matrix, &m.positions).unwrap();
    let metadata = SceneObjectMetadata {
        source_center_initialized: false,
        symmetry_offset: 0.0,
        affine: affine.clone(),
        render: None,
        picking_policy: SglPickingPolicy::SourceScaleRowV1,
        origin: None,
    };
    let objects = [SubtoolRenderSnapshot {
        id: 7,
        active: true,
        mesh: m,
        metadata: Some(metadata.clone()),
    }];
    let mut c = source(true);
    c.precise_target = Some(DVec3::new(100_000.031_25, 7.0, -4.0));
    c.target = c.precise_target.unwrap().as_vec3();
    c.prepare_source_frame(viewport(), &objects).unwrap();
    let render = ObjectRenderTransform::for_object(Some(&metadata)).unwrap();
    let (mv, _) = render.model_view(c.view64().to_cols_array()).unwrap();
    assert_eq!(mv[12], -0.03125);
    assert_eq!(mv[13], 0.0);
    assert_eq!(mv[14], -1000.0);
    let ray = c.ray64(Vec2::ZERO, 2.0);
    let local_origin = affine.local_point(ray.origin).unwrap();
    let expected_x = 0.03125 / 2.0;
    assert!((local_origin.x - expected_x).abs() < 1e-12);
    assert!((local_origin.y).abs() < 1e-12);
    assert_eq!(ray.direction, -DVec3::Z);
}
#[test]
fn resident_pick_span_reaches_visible_surface_from_signed_orthographic_near_plane() {
    let mut camera = source(true);
    camera.prepare_source_frame(viewport(), &objects()).unwrap();
    let ray = camera.ray64(Vec2::ZERO, 2.0);
    let span = camera.resident_pick_distance(ray).unwrap();
    let center_distance = (DVec3::ZERO - ray.origin).dot(ray.direction);
    assert!(center_distance > 0.0);
    assert!(span > center_distance);
    let frame = camera.source_frame().unwrap();
    let origin_z = frame.view64.transform_point3(ray.origin).z;
    assert!(origin_z > 0.0, "source orthographic near is signed");
    assert!(span >= origin_z + frame.clip.far_distance);
    let mut perspective = source(false);
    perspective
        .prepare_source_frame(viewport(), &objects())
        .unwrap();
    let ray = perspective.ray64(Vec2::new(0.5, 0.25), 2.0);
    let span = perspective.resident_pick_distance(ray).unwrap();
    let center_distance = (DVec3::ZERO - ray.origin).dot(ray.direction);
    assert!(span > center_distance);
}
#[test]
fn source_camera_operations_do_not_edit_project_or_history() {
    let mut document = Document::new((*mesh()).clone());
    let before = document.to_json().unwrap();
    let mut c = source(true);
    c.set_source_view_height(3.25).unwrap();
    c.axis_view(1);
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    c.pan(eframe::egui::vec2(3.0, 4.0));
    c.scroll_zoom(240.0);
    c.prepare_source_frame(viewport(), &objects()).unwrap();
    assert_eq!(document.to_json().unwrap(), before);
    assert!(!document.undo());
    assert!(!document.redo());
}
#[test]
fn invalid_prepared_frame_retains_accepted_frame_and_reports_error() {
    let mut c = source(true);
    let old = c.source_frame().unwrap().view64;
    c.fov_degrees = f32::NAN;
    assert!(c.prepare_source_frame(viewport(), &objects()).is_err());
    assert_eq!(c.source_frame().unwrap().view64, old);
    assert!(c.source_camera.as_ref().unwrap().error.is_some());
}
#[test]
fn ordinary_resize_changes_projection_extent_without_changing_fixed_eye() {
    let mut c = source(true);
    let eye = c.position64();
    let original = c.source_frame().unwrap().projection64.y_axis.y;
    let portrait = ViewportDimensions::checked([500, 1000], DVec2::new(250.0, 500.0)).unwrap();
    c.prepare_source_frame(portrait, &objects()).unwrap();
    assert_eq!(c.position64(), eye);
    assert_eq!(
        c.source_frame().unwrap().projection64.y_axis.y,
        original * 0.5
    );
    assert_eq!(
        c.source_frame().unwrap().viewport.render_pixels,
        [500, 1000]
    );
}

#[test]
fn source_frame_all_matches_recorded_reset_front_tetra_fit() {
    let view = ViewportDimensions::checked([1024, 768], DVec2::new(1024., 768.)).unwrap();
    let mesh = Arc::new(
        Mesh::from_obj("v 0 0 1\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n")
            .unwrap(),
    );
    let objects = vec![SubtoolRenderSnapshot {
        id: 1,
        active: true,
        mesh,
        metadata: None,
    }];
    let mut camera = Camera {
        yaw: 0.,
        pitch: 0.,
        target: Vec3::ZERO,
        precise_target: Some(DVec3::ZERO),
        orthographic: true,
        fov_degrees: 40.,
        source_vertical_fov: true,
        ..Default::default()
    };
    camera.set_source_framing(true, view).unwrap();
    camera.prepare_source_frame(view, &objects).unwrap();
    let before = camera.source_diagnostics();
    camera.frame_scene(&objects);
    eprintln!(
        "Source pre-fit near={:?}, actual zoom={:?}",
        before["near"],
        camera.source_projection_zoom()
    );
    // Actual pinned79b24 Reset(bar)/backgroundFrame observation, fixed tetra,
    // native1024x768/DPR1, orthographic40degrees, settled identityrotation.
    let reference_zoom = 2.43916180378013_f64;
    assert!((camera.source_projection_zoom().unwrap() - reference_zoom).abs() < 1e-6);
    assert_eq!(camera.precise_target, Some(DVec3::new(0.5, 0., 0.5)));
}

fn framing_fixture() -> Vec<SubtoolRenderSnapshot> {
    vec![SubtoolRenderSnapshot {
        id: 1,
        active: true,
        mesh: Arc::new(
            Mesh::from_obj(
                "v 0 0 1\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n",
            )
            .unwrap(),
        ),
        metadata: None,
    }]
}
fn framing_camera(orthographic: bool, dimensions: ViewportDimensions) -> Camera {
    let mut c = Camera {
        yaw: 0.,
        pitch: 0.,
        target: Vec3::ZERO,
        precise_target: Some(DVec3::ZERO),
        orthographic,
        fov_degrees: 40.,
        source_vertical_fov: true,
        ..Default::default()
    };
    c.set_source_framing(true, dimensions).unwrap();
    c.prepare_source_frame(dimensions, &framing_fixture())
        .unwrap();
    c
}
#[test]
fn source_reset_has_reference_endpoint_and_preserves_projection_preferences() {
    for ortho in [false, true] {
        let mut c = framing_camera(ortho, viewport());
        c.target = Vec3::new(2., 3., 4.);
        c.precise_target = Some(c.target.as_dvec3());
        c.rotation = Some(Quat::from_rotation_y(0.7));
        c.speed = 1.75;
        c.use_pivot = true;
        assert!(c.reset_source_view());
        c.prepare_source_frame(viewport(), &framing_fixture())
            .unwrap();
        assert_eq!(c.source_projection_zoom(), Some(80.));
        assert_eq!(c.precise_target, Some(DVec3::ZERO));
        assert_eq!(
            (c.orthographic, c.fov_degrees, c.speed, c.use_pivot),
            (ortho, 40., 1.75, true)
        );
        let z = if ortho { 1000. } else { 90. };
        assert_eq!(c.position64(), DVec3::new(0., 0., z));
        assert_eq!(c.source_frame().unwrap().view64.x_axis.x, 1.);
    }
}
#[test]
fn source_empty_frame_resets_view_while_legacy_empty_frame_is_unchanged() {
    let mut c = framing_camera(true, viewport());
    c.set_source_projection_zoom(2.).unwrap();
    c.frame_scene(&[]);
    assert_eq!(c.source_projection_zoom(), Some(80.));
    assert_eq!(c.precise_target, Some(DVec3::ZERO));
    let mut legacy = Camera {
        target: Vec3::new(1., 2., 3.),
        distance: 7.,
        ..Default::default()
    };
    let old = legacy.clone();
    legacy.frame_scene(&[]);
    assert!(!legacy.reset_source_view());
    assert_eq!(
        (legacy.target, legacy.distance, legacy.rotation),
        (old.target, old.distance, old.rotation)
    );
}
#[test]
fn source_single_and_scene_frames_agree_for_collapsed_axis_loose_bounds() {
    let shapes = objects();
    let mut a = framing_camera(true, viewport());
    let mut b = a.clone();
    a.frame(&shapes[0].mesh);
    b.frame_scene(&shapes);
    assert_eq!(a.source_projection_zoom(), b.source_projection_zoom());
    assert_eq!(a.precise_target, b.precise_target);
    assert_eq!(a.precise_target, Some(DVec3::ZERO));
    assert!(a.source_projection_zoom().unwrap() > 0.);
}
#[test]
fn source_fit_uses_render_pixels_rather_than_css_points() {
    let a = ViewportDimensions::checked([1024, 768], DVec2::new(1024., 768.)).unwrap();
    let b = ViewportDimensions::checked([1024, 768], DVec2::new(512., 384.)).unwrap();
    let mut first = framing_camera(true, a);
    let mut second = framing_camera(true, b);
    first.frame_scene(&framing_fixture());
    second.frame_scene(&framing_fixture());
    assert_eq!(
        first.source_projection_zoom().unwrap().to_bits(),
        second.source_projection_zoom().unwrap().to_bits()
    );
    assert_eq!(first.precise_target, second.precise_target);
}
#[test]
fn source_invalid_fit_retains_accepted_pose_zoom_and_snapshot() {
    let mut c = framing_camera(true, viewport());
    let before = c.clone();
    let mut bad = framing_fixture();
    Arc::make_mut(&mut bad[0].mesh).positions[0].x = f32::NAN;
    c.frame_scene(&bad);
    assert_eq!(c.target, before.target);
    assert_eq!(c.precise_target, before.precise_target);
    assert_eq!(c.source_projection_zoom(), before.source_projection_zoom());
    assert_eq!(
        c.source_frame().unwrap().view64,
        before.source_frame().unwrap().view64
    );
    assert!(c.source_diagnostics()["error"].is_string());
}
#[test]
fn source_fit_uses_reflected_sheared_world_box_and_excludes_grid() {
    let mut transformed = framing_fixture();
    let matrix = [
        -2., 0., 0., 0., 0.5, 3., 0., 0., 0., 0., 0.75, 0., 3., 5., -4., 1.,
    ];
    let affine =
        ObjectAffineF32V1::from_local_points(matrix, &transformed[0].mesh.positions).unwrap();
    transformed[0].metadata = Some(SceneObjectMetadata {
        source_center_initialized: false,
        symmetry_offset: 0.0,
        affine,
        render: None,
        picking_policy: SglPickingPolicy::SourceScaleRowV1,
        origin: None,
    });
    let mut a = framing_camera(true, viewport());
    let mut b = a.clone();
    a.frame_scene(&framing_fixture());
    b.frame_scene(&transformed);
    assert_eq!(b.precise_target, Some(DVec3::new(2., 5., -3.625)));
    // Independently enumerated affine box: [.5,2,-4] .. [3.5,8,-3.25].
    let expected = a.source_projection_zoom().unwrap() * (45.5625_f64 / 6.).sqrt();
    assert!((b.source_projection_zoom().unwrap() - expected).abs() < 1e-12);
}
#[test]
fn source_perspective_fit_matches_independent_float32_frustum_literal() {
    let mut c = framing_camera(false, viewport());
    c.frame_scene(&framing_fixture());
    // Primary Source formula independently evaluated for40deg,1000x500,
    // P00=1.3737387657165527/P11=2.7474775314331055,near.01,tetra box.
    assert!((c.source_projection_zoom().unwrap() - 2.8647316583408924).abs() < 1e-12);
    assert_eq!(c.precise_target, Some(DVec3::new(0.5, 0., 0.5)));
}
