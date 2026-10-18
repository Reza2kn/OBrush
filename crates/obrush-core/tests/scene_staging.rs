//! Independent scene-coordinate/attribute contracts. No Document install proof.
use obrush_core::{
    MaskConvention, ObjectAffineF32V1, SglByteOrder, SglPickingPolicy, SglScene, SglStageSettings,
    StagedSglScene, Vec3, MAX_STAGED_SCENE_BYTES, MAX_VERTICES, TRIANGLE_CORNER,
};
use std::sync::Arc;
const AFFINE: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
const V2: &[u8] = include_bytes!("fixtures/independent-v2-negative-markers.sgl");
const EMPTY: &[u8] = include_bytes!("fixtures/independent-empty-scene.sgl");
const LEGACY: &[u8] = include_bytes!("fixtures/independent-legacy1-absent-fields.sgl");
fn stage(bytes: &[u8]) -> StagedSglScene {
    StagedSglScene::from_sgl_bytes(bytes, &SglStageSettings::default()).unwrap()
}
fn identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

#[test]
fn empty_scene_has_no_mesh_active_selection_or_placeholder() {
    let scene = stage(EMPTY);
    assert!(scene.objects.is_empty());
    assert!(scene.active_id.is_none());
    assert!(scene.selected_ids.is_empty());
    assert!(scene.active_mesh().is_none());
    assert_eq!(scene.next_id, 1);
}
#[test]
fn empty_encoded_header_remains_exact_and_unapplied() {
    let scene = stage(EMPTY);
    let camera = scene.origin.camera.unwrap();
    assert_eq!(camera.projection, 1);
    assert_eq!(camera.mode, 2);
    assert_eq!(camera.fov_degrees, 33.0);
    assert!(scene.report.camera_not_applied);
}
#[test]
fn legacy_header_render_channel_absence_is_not_invented() {
    let scene = stage(LEGACY);
    assert_eq!(scene.origin.version, 1);
    assert_eq!(scene.origin.byte_order, SglByteOrder::BigEndian);
    assert!(scene.origin.camera.is_none());
    assert!(scene.origin.display.is_none());
    let object = &scene.objects[0];
    assert!(object.render.is_none());
    assert!(object.origin.raw.colors.is_none());
    assert!(object.origin.raw.materials.is_none());
    assert!(object.origin.raw.uv.is_none());
    assert!(object.mesh.materials.is_none());
    assert_eq!(object.mesh.mask_convention, MaskConvention::ProtectedF32V1);
}
#[test]
fn absent_rgb_uses_source_white_without_creating_raw_color_data() {
    let scene = stage(AFFINE);
    assert!(scene.objects[0].origin.raw.colors.is_none());
    assert!(scene.objects[0]
        .mesh
        .colors
        .iter()
        .all(|rgb| *rgb == [1.0; 3]));
    assert_eq!(scene.report.implicit_rgb_objects, 1);
}
#[test]
fn absent_materials_keep_implicit_defaults_and_clear_legacy_channel() {
    let scene = stage(AFFINE);
    let object = &scene.objects[1];
    assert!(object.origin.raw.materials.is_none());
    assert!(object.mesh.materials.is_none());
    assert!(object.mesh.masks.iter().all(|v| *v == 0.0));
    assert_eq!(object.mesh.mask_convention, MaskConvention::ProtectedF32V1);
    assert_eq!(scene.report.implicit_material_objects, 1);
}
#[test]
fn version0_needs_explicit_byte_order_in_staging() {
    let bytes = include_bytes!("fixtures/independent-legacy0-explicit-order.sgl");
    assert!(StagedSglScene::from_sgl_bytes(bytes, &SglStageSettings::default()).is_err());
    let scene = StagedSglScene::from_sgl_bytes(
        bytes,
        &SglStageSettings {
            byte_order: Some(SglByteOrder::LittleEndian),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(scene.origin.version, 0);
    assert!(scene.origin.camera.is_none());
}
#[test]
fn triangle_markers_normalize_only_in_editable_geometry() {
    let scene = stage(V2);
    let object = &scene.objects[0];
    assert_eq!(object.origin.raw.faces[0][3], 0x80000001);
    assert_eq!(
        object.mesh.surface.as_ref().unwrap().faces[0][3],
        TRIANGLE_CORNER
    );
    assert_eq!(scene.report.normalized_geometry_markers, 1);
}
#[test]
fn uv_marker_normalizes_independently_without_changing_raw_marker() {
    let scene = stage(V2);
    let object = &scene.objects[0];
    assert_eq!(
        object.origin.raw.uv.as_ref().unwrap().face_indices[0][3],
        0xfffffffe
    );
    assert_eq!(
        object
            .mesh
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .face_indices[0][3],
        TRIANGLE_CORNER
    );
    assert_eq!(scene.report.normalized_uv_markers, 1);
}
#[test]
fn source_masks_keep_tiny_f32_words_without_double_complement() {
    let scene = stage(V2);
    let mesh = &scene.objects[0].mesh;
    assert_eq!(mesh.mask_convention, MaskConvention::UnprotectedF32V1);
    assert_eq!(
        mesh.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [1, 0x33000000, 0x33000001]
    );
    assert_eq!(mesh.mask_influence(0).to_bits(), 1);
}
#[test]
fn source_rgb_signed_zero_and_material_words_copy_exactly() {
    let scene = stage(V2);
    let object = &scene.objects[0];
    let raw = object.origin.raw.as_ref();
    for vertex in 0..object.mesh.positions.len() {
        assert_eq!(
            object.mesh.colors[vertex].map(f32::to_bits),
            raw.colors.as_ref().unwrap()[vertex].map(f32::to_bits)
        );
        assert_eq!(
            object.mesh.materials.as_ref().unwrap()[vertex].map(f32::to_bits),
            [
                raw.materials.as_ref().unwrap()[vertex][0].to_bits(),
                raw.materials.as_ref().unwrap()[vertex][1].to_bits()
            ]
        );
    }
}
#[test]
fn quad_render_diagonal_and_original_corner_order_remain_source_order() {
    let scene = stage(AFFINE);
    let object = &scene.objects[0];
    assert_eq!(object.mesh.triangles, [[0, 1, 2], [0, 2, 3]]);
    assert_eq!(object.mesh.surface.as_ref().unwrap().faces, [[0, 1, 2, 3]]);
    assert_eq!(scene.report.original_quads, 1);
}
#[test]
fn corner_uv_ids_unused_coordinates_and_signed_zero_survive() {
    let scene = stage(AFFINE);
    let uv = scene.objects[0]
        .mesh
        .surface
        .as_ref()
        .unwrap()
        .uv
        .as_ref()
        .unwrap();
    assert_eq!(uv.coordinates.len(), 5);
    assert_eq!(uv.face_indices, [[4, 1, 2, 3]]);
    assert_eq!(uv.coordinates[4][0].to_bits(), 0x80000000);
    assert_eq!(uv.coordinates[0], [0.0, 0.0]);
}
#[test]
fn raw_center_and_redundant_scale_remain_separate_from_derived_center() {
    let scene = stage(AFFINE);
    let object = &scene.objects[0];
    assert_eq!(object.origin.raw.center, [123.0, -99.0, 0.25]);
    assert_eq!(object.origin.raw.scale, 9.0);
    assert_eq!(object.affine.interaction_center, [1.0, 0.5, 0.0]);
    assert_eq!(object.affine.source_scale_squared(), 5.0);
    assert_eq!(object.affine.determinant(), -3.0);
}
#[test]
fn independently_authored_affine_world_positions_match_exact_scalars() {
    let scene = stage(AFFINE);
    let object = &scene.objects[0];
    let expected = [
        [4.0, -2.0, 1.0],
        [0.0, -2.0, 1.0],
        [1.0, 1.0, 1.0],
        [5.0, 1.0, 1.0],
    ];
    for (point, world) in object.mesh.positions.iter().zip(expected) {
        assert_eq!(object.affine.world_point(*point).unwrap().to_array(), world);
    }
    assert_eq!(
        object.affine.local_to_world.map(f32::to_bits),
        object.origin.raw.matrix.map(f32::to_bits)
    );
}
#[test]
fn reflection_shading_normal_is_distinct_from_oriented_face_normal() {
    let scene = stage(AFFINE);
    let object = &scene.objects[0];
    assert_eq!(
        object.affine.shading_normal(Vec3::Z).unwrap(),
        glam::DVec3::Z
    );
    let p: Vec<_> = object
        .mesh
        .positions
        .iter()
        .map(|p| object.affine.world_point(*p).unwrap())
        .collect();
    assert_eq!(
        (p[1] - p[0]).cross(p[2] - p[0]).normalize(),
        -glam::DVec3::Z
    );
}
#[test]
fn inverse_local_ray_origin_uses_full_affine_not_serialized_scale() {
    let scene = stage(AFFINE);
    assert_eq!(
        scene.objects[0]
            .affine
            .local_point(glam::DVec3::new(2.5, -0.5, 3.0))
            .unwrap()
            .to_array(),
        [1.0, 0.5, 4.0]
    );
}
#[test]
fn source_and_world_distance_policies_are_explicitly_distinct() {
    let source = stage(AFFINE);
    let world = StagedSglScene::from_sgl_bytes(
        AFFINE,
        &SglStageSettings {
            picking_policy: SglPickingPolicy::WorldDistanceV1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(source.picking_policy, SglPickingPolicy::SourceScaleRowV1);
    assert_eq!(world.picking_policy, SglPickingPolicy::WorldDistanceV1);
    let ray_origin = glam::DVec3::new(2.5, -0.5, 3.0);
    let local_distance = source.objects[0].affine.local_point(ray_origin).unwrap().z;
    let source_metric = local_distance * source.objects[0].affine.source_scale_squared().sqrt();
    let world0 = ray_origin.distance(
        source.objects[0]
            .affine
            .world_point(Vec3::new(1.0, 0.5, 0.0))
            .unwrap(),
    );
    let world1 = ray_origin.distance(
        source.objects[1]
            .affine
            .world_point(Vec3::new(2.5, -0.5, 0.0))
            .unwrap(),
    );
    assert!(source_metric > world1);
    assert!(world0 < world1);
}
#[test]
fn local_positions_are_retained_instead_of_destructive_f32_world_bake() {
    let scene = stage(include_bytes!("fixtures/independent-bake-rounding.sgl"));
    let object = &scene.objects[0];
    let local = object.mesh.positions[1];
    assert_eq!(local.x, 0.00390625);
    let world = object.affine.world_point(local).unwrap();
    assert_eq!(world.x, 100000.00390625);
    assert_eq!(object.affine.local_point(world).unwrap().x, 0.00390625);
    assert_eq!(
        object
            .affine
            .local_point(world.as_vec3().as_dvec3())
            .unwrap()
            .x,
        0.0
    );
}
#[test]
fn raw_valid_tiny_inverse_rejects_unsupported_editable_affine() {
    let bytes = include_bytes!("fixtures/independent-tiny-invertible-affine.sgl");
    let raw = SglScene::from_bytes(bytes).unwrap();
    assert_eq!(raw.to_bytes().unwrap(), bytes);
    assert!(raw
        .into_staged_editable(&SglStageSettings::default())
        .unwrap_err()
        .contains("inverse"));
}
#[test]
fn invalid_second_mesh_rejects_the_complete_staging_result() {
    assert!(StagedSglScene::from_sgl_bytes(
        include_bytes!("fixtures/independent-invalid-second-mesh.sgl"),
        &SglStageSettings::default()
    )
    .is_err());
}
#[test]
fn low_combined_raw_editable_budget_rejects_before_editable_allocation() {
    assert!(StagedSglScene::from_sgl_bytes(
        AFFINE,
        &SglStageSettings {
            storage_limit: 1,
            ..Default::default()
        }
    )
    .unwrap_err()
    .contains("allocations"));
}
#[test]
fn invalid_budget_is_rejected_without_decoding_input() {
    assert!(StagedSglScene::from_sgl_bytes(
        b"not SGL",
        &SglStageSettings {
            storage_limit: 0,
            ..Default::default()
        }
    )
    .unwrap_err()
    .contains("storage limit"));
    assert!(StagedSglScene::from_sgl_bytes(
        b"not SGL",
        &SglStageSettings {
            storage_limit: MAX_STAGED_SCENE_BYTES + 1,
            ..Default::default()
        }
    )
    .unwrap_err()
    .contains("storage limit"));
}
#[test]
fn aggregate_geometry_limit_is_checked_before_editable_copies() {
    let mut raw = SglScene::from_bytes(LEGACY).unwrap();
    let mut mesh = raw.meshes[0].clone();
    mesh.positions.resize(MAX_VERTICES / 2 + 1, [0.0, 0.0, 0.0]);
    raw.meshes = vec![mesh.clone(), mesh];
    assert!(raw
        .into_staged_editable(&SglStageSettings::default())
        .unwrap_err()
        .contains("aggregate"));
}
#[test]
fn collapsed_raw_valid_face_has_explicit_editable_policy_error() {
    let mut raw = SglScene::from_bytes(LEGACY).unwrap();
    raw.meshes[0].positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]].to_vec();
    raw.validate().unwrap();
    assert!(raw
        .into_staged_editable(&SglStageSettings::default())
        .unwrap_err()
        .contains("collapsed"));
}
#[test]
fn stable_ids_and_last_import_selection_are_deterministic() {
    let scene = stage(AFFINE);
    assert_eq!(
        scene.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(scene.active_id, Some(2));
    assert_eq!(scene.selected_ids, [2]);
    assert_eq!(scene.next_id, 3);
    assert_eq!(scene.active_mesh().unwrap().positions.len(), 3);
}
#[test]
fn origin_arcs_share_only_each_objects_own_raw_mesh() {
    let scene = stage(AFFINE);
    let copy = scene.objects[0].clone();
    assert!(Arc::ptr_eq(&copy.origin.raw, &scene.objects[0].origin.raw));
    assert!(!Arc::ptr_eq(&copy.origin.raw, &scene.objects[1].origin.raw));
    assert_eq!(Arc::strong_count(&scene.objects[1].origin.raw), 1);
}
#[test]
fn duplicate_missing_and_invalid_selection_ids_are_rejected() {
    let mut scene = stage(AFFINE);
    scene.objects[1].id = 1;
    assert!(scene.validate().is_err());
    let mut scene = stage(AFFINE);
    scene.selected_ids = vec![2, 2];
    assert!(scene.validate().is_err());
    let mut scene = stage(AFFINE);
    scene.active_id = Some(9);
    assert!(scene.validate().is_err());
}
#[test]
fn empty_and_nonempty_active_invariants_are_checked() {
    let mut scene = stage(EMPTY);
    scene.active_id = Some(1);
    assert!(scene.validate().is_err());
    let mut scene = stage(AFFINE);
    scene.active_id = None;
    assert!(scene.validate().is_err());
    let mut scene = stage(AFFINE);
    scene.selected_ids.clear();
    assert!(scene.validate().is_err());
}
#[test]
fn zero_opacity_retains_the_object_and_render_flags() {
    let mut raw = SglScene::from_bytes(AFFINE).unwrap();
    let r = raw.meshes[0].render.as_mut().unwrap();
    r.opacity = 0.0;
    r.show_wireframe = true;
    r.flat_shading = true;
    r.shader = 12;
    let scene = raw
        .into_staged_editable(&SglStageSettings::default())
        .unwrap();
    assert_eq!(scene.objects.len(), 2);
    let r = scene.objects[0].render.as_ref().unwrap();
    assert_eq!(r.opacity, 0.0);
    assert_eq!(r.shader, 12);
    assert!(r.show_wireframe);
    assert!(r.flat_shading);
    assert_eq!(scene.report.appearance_not_applied_objects, 2);
}
#[test]
fn raw_unknown_matcap_and_extreme_valid_fov_are_retained_without_guessing() {
    let scene = stage(V2);
    assert_eq!(scene.objects[0].render.as_ref().unwrap().matcap, u32::MAX);
    assert_eq!(scene.origin.camera.as_ref().unwrap().fov_degrees, 179.0);
    assert_eq!(
        stage(AFFINE).origin.camera.as_ref().unwrap().fov_degrees,
        1.0
    );
}
#[test]
fn mutable_invalid_header_and_render_metadata_fail_validation() {
    let mut scene = stage(AFFINE);
    scene.origin.camera.as_mut().unwrap().fov_degrees = f32::NAN;
    assert!(scene.validate().is_err());
    let mut scene = stage(AFFINE);
    scene.objects[0].render.as_mut().unwrap().opacity = 1.1;
    assert!(scene.validate().is_err());
}
#[test]
fn singular_projective_and_nonfinite_affines_are_rejected() {
    for (index, value) in [(0, 0.0), (3, 1.0), (0, f32::INFINITY)] {
        let mut matrix = identity();
        matrix[index] = value;
        assert!(
            ObjectAffineF32V1::from_local_points(matrix, &[Vec3::ZERO, Vec3::X, Vec3::Y]).is_err()
        );
    }
}
#[test]
fn public_affine_point_method_checks_mutated_matrix() {
    let mut affine =
        ObjectAffineF32V1::from_local_points(identity(), &[Vec3::ZERO, Vec3::X, Vec3::Y]).unwrap();
    affine.local_to_world[0] = 0.0;
    assert!(affine.world_point(Vec3::ZERO).is_err());
}
#[test]
fn retained_raw_storage_is_counted_alongside_editable_channels() {
    let scene = stage(AFFINE);
    let bytes = scene.storage_bytes().unwrap();
    assert_eq!(bytes, scene.report.raw_editable_storage_bytes);
    assert!(bytes > AFFINE.len());
    assert!(bytes < MAX_STAGED_SCENE_BYTES);
}
#[test]
fn staged_view_requires_v7_and_serializes_all_source_metadata() {
    let scene = stage(AFFINE);
    assert_eq!(scene.required_project_version, 7);
    scene.check_encoded_size().unwrap();
    let dto: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&scene).unwrap()).unwrap();
    assert_eq!(dto["objects"][0]["origin"]["raw"]["scale"], 9.0);
    assert_eq!(
        dto["objects"][0]["affine"]["interaction_center"],
        serde_json::json!([1.0, 0.5, 0.0])
    );
    assert_eq!(
        dto["objects"][0]["mesh"]["mask_convention"],
        "unprotected_f32_v1"
    );
}
#[test]
fn public_invalid_version_cannot_masquerade_as_a_legacy_project() {
    let mut scene = stage(AFFINE);
    scene.required_project_version = 6;
    assert!(scene.validate().unwrap_err().contains("version7"));
}
