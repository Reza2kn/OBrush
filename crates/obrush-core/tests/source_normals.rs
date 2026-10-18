//! Independent rational geometry/scalar witnesses; no reference runtime claim.
use obrush_core::{
    BrushKind, Document, MaskConvention, Mesh, PaintResponseOperation, PaintResponseOptions,
    PaintResponsePass, PaintResponseRequest, SourceNormalFaceOrigin, SourceNormalField,
    SourceNormalPolicy, SurfaceTopology, Vec3, TRIANGLE_CORNER,
};
fn mesh(points: &[[f32; 3]], triangles: &[[u32; 3]]) -> Mesh {
    let mut m = Mesh::cube();
    m.positions = points.iter().copied().map(Vec3::from_array).collect();
    m.triangles = triangles.to_vec();
    m.normals = vec![Vec3::ZERO; points.len()];
    m.masks = vec![0.0; points.len()];
    m.colors = vec![[0.0; 3]; points.len()];
    m.materials = None;
    m.surface = None;
    m.recompute_normals();
    m.validate().unwrap();
    m
}
fn quad() -> Mesh {
    let mut m = mesh(
        &[[0., 0., 0.], [2., 0., 0.], [2., 1., 1.], [0., 1., 0.]],
        &[[0, 1, 2], [0, 2, 3]],
    );
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, 3]],
        uv: None,
    });
    m
}
fn source(m: &Mesh) -> SourceNormalField {
    SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap()
}
fn words(n: Vec3) -> [u32; 3] {
    n.to_array().map(f32::to_bits)
}
fn request() -> PaintResponseRequest {
    PaintResponseRequest {
        kind: BrushKind::Paint,
        operation: PaintResponseOperation::OrdinaryDab,
        in_contact: true,
        options: PaintResponseOptions {
            intensity: 0.25,
            hardness: 1.0,
            culling: true,
            color: [1.; 3],
            write_roughness: false,
            write_metallic: false,
            ..Default::default()
        },
        first: PaintResponsePass {
            center: [2., 0., 0.],
            local_radius_squared: 0.01,
            eye_direction: [-1., 0., 0.],
            mirrored_x: false,
            alpha_frame: None,
        },
        mirrored: None,
    }
}
#[test]
fn independent_nonplanar_original_quad_raw_words_at_every_corner() {
    let m = quad();
    let before = serde_json::to_vec(&m).unwrap();
    let field = source(&m);
    assert_eq!(
        words(field.face_normals()[0]),
        [3212836864, 3221225472, 1082130432]
    );
    assert!(field
        .vertex_normals()
        .iter()
        .all(|&n| words(n) == [3212836864, 3221225472, 1082130432]));
    assert_eq!(field.report().quads, 1);
    assert_eq!(
        field.report().face_origin,
        SourceNormalFaceOrigin::StoredOriginalFaces
    );
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn triangle_keeps_area_magnitude_instead_of_unit_direction() {
    let m = mesh(&[[0., 0., 0.], [2., 0., 0.], [0., 3., 0.]], &[[0, 1, 2]]);
    let f = source(&m);
    assert_eq!(f.face_normals(), [Vec3::new(0., 0., 6.)]);
    assert_eq!(f.vertex_normals(), [Vec3::new(0., 0., 6.); 3]);
    assert_eq!(m.normals, [Vec3::Z; 3]);
    assert_eq!(
        f.report().face_origin,
        SourceNormalFaceOrigin::TriangleFaces
    );
}
#[test]
fn unequal_triangle_areas_are_averaged_by_incident_face_count() {
    let m = mesh(
        &[[0., 0., 0.], [2., 0., 0.], [0., 3., 0.], [0., 0., 1.]],
        &[[0, 1, 2], [0, 3, 1]],
    );
    let f = source(&m);
    assert_eq!(
        f.vertex_normals(),
        [
            Vec3::new(0., 1., 3.),
            Vec3::new(0., 1., 3.),
            Vec3::new(0., 0., 6.),
            Vec3::new(0., 2., 0.)
        ]
    );
}
#[test]
fn original_quad_and_two_triangle_faces_are_measurably_distinct() {
    let original = quad();
    let mut split = original.clone();
    split.surface = None;
    let q = source(&original);
    let t = source(&split);
    assert_eq!(q.vertex_normals()[1], Vec3::new(-1., -2., 4.));
    assert_eq!(t.vertex_normals()[1], Vec3::new(0., -2., 2.));
    assert_eq!(t.vertex_normals()[3], Vec3::new(-1., 0., 2.));
    assert_ne!(q.vertex_normals()[1].normalize(), original.normals[1]);
}
#[test]
fn protected_and_canonical_masks_never_weight_the_normal_cache() {
    let mut m = quad();
    m.masks = [0., 0.25, 0.5, 1.].to_vec();
    let p = source(&m);
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    let u = source(&m);
    assert_eq!(p.vertex_normals(), u.vertex_normals());
    assert!(p.matches_mesh(&m));
}
#[test]
fn color_material_changes_reuse_the_exact_geometry_field() {
    let mut m = quad();
    let f = source(&m);
    m.colors[0] = [0.5; 3];
    m.materials = Some(vec![[0.1, 0.9]; 4]);
    assert!(f.matches_mesh(&m));
    assert_eq!(f.vertex_normals(), source(&m).vertex_normals());
}
#[test]
fn position_signed_zero_changes_invalidate_exact_input_identity() {
    let mut m = quad();
    let f = source(&m);
    m.positions[0].x = -0.0;
    assert!(!f.matches_mesh(&m));
    let new = source(&m);
    assert!(new.matches_mesh(&m));
}
#[test]
fn same_size_valid_original_face_edits_invalidate_cache() {
    let mut m = quad();
    let f = source(&m);
    m.surface.as_mut().unwrap().faces[0] = [1, 2, 3, 0];
    m.triangles = vec![[1, 2, 3], [1, 3, 0]];
    m.validate().unwrap();
    assert!(!f.matches_mesh(&m));
    assert!(source(&m).matches_mesh(&m));
}
#[test]
fn canonical_binding_edit_cannot_reuse_original_face_normals() {
    let mut m = quad();
    let f = source(&m);
    m.triangles[1] = [0, 3, 2];
    assert!(!f.matches_mesh(&m));
    assert!(SourceNormalField::compute(&m, SourceNormalPolicy::StaticOriginalFaces).is_err());
}
#[test]
fn refresh_changes_opposite_quad_corner_and_equals_a_full_rebuild() {
    let mut m = quad();
    let mut cache = Some(source(&m));
    m.positions[1].x = 3.0;
    let full = source(&m);
    let refreshed =
        SourceNormalField::refresh(&mut cache, &m, SourceNormalPolicy::StaticOriginalFaces)
            .unwrap();
    assert_eq!(refreshed.vertex_normals(), [Vec3::new(-1., -3., 5.); 4]);
    assert_eq!(refreshed.vertex_normals(), full.vertex_normals());
}
#[test]
fn descending_static_face_order_retains_the_cancellation_witness() {
    let tiny = 2.0_f32.powi(-14);
    let m = mesh(
        &[
            [0., 0., 0.],
            [0., 8192., 0.],
            [0., 0., 8192.],
            [0., 0., 8192.],
            [0., 8192., 0.],
            [0., tiny, 0.],
            [0., 0., tiny],
        ],
        &[[0, 1, 2], [0, 3, 4], [0, 5, 6]],
    );
    let f = source(&m);
    assert_eq!(words(f.vertex_normals()[0]), [0; 3]);
    // Ascending summation would preserve 2^-28/3; the gate detects reordering.
    assert!((f64::from(tiny) * f64::from(tiny) / 3.0) as f32 > 0.0);
    assert_eq!(f.face_normals()[2].x, tiny * tiny);
}
#[test]
fn face_cache_signed_zero_and_vertex_sum_assignment_are_distinct() {
    let m = mesh(&[[0., 0., 0.], [1., 0., -0.], [0., 1., 0.]], &[[0, 1, 2]]);
    let f = source(&m);
    assert_eq!(words(f.face_normals()[0]), [0, 0x80000000, 0x3f800000]);
    assert_eq!(words(f.vertex_normals()[0]), [0, 0, 0x3f800000]);
}
#[test]
fn unused_vertex_is_preserved_and_reported_as_zero_not_removed_or_filled() {
    let m = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [8., 8., 8.]],
        &[[0, 1, 2]],
    );
    let f = source(&m);
    assert_eq!(f.report().unused_vertices, 1);
    assert_eq!(f.report().zero_vertex_normals, 1);
    assert_eq!(words(f.vertex_normals()[3]), [0; 3]);
    assert_eq!(m.positions.len(), 4);
}
#[test]
fn collinear_face_zero_normal_is_reported_without_legacy_direction() {
    let m = mesh(&[[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]], &[[0, 1, 2]]);
    let f = source(&m);
    assert_eq!(f.report().zero_face_normals, 1);
    assert_eq!(f.report().zero_vertex_normals, 3);
    assert_eq!(f.vertex_normals(), [Vec3::ZERO; 3]);
}
#[test]
fn dynamic_ring_policy_is_an_explicit_error() {
    assert!(SourceNormalField::compute(&quad(), SourceNormalPolicy::DynamicFaces).is_err());
}
#[test]
fn precise_scratch_bound_rejects_before_output_and_never_mutates_mesh() {
    let m = quad();
    let before = serde_json::to_vec(&m).unwrap();
    let bytes = SourceNormalField::storage_requirement(&m)
        .unwrap()
        .peak_bytes;
    assert!(SourceNormalField::compute_with_budget(
        &m,
        SourceNormalPolicy::StaticOriginalFaces,
        bytes - 1
    )
    .is_err());
    assert!(SourceNormalField::compute_with_budget(
        &m,
        SourceNormalPolicy::StaticOriginalFaces,
        bytes
    )
    .is_ok());
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn refresh_rejection_retains_old_cache_but_never_returns_it_as_current() {
    let mut m = quad();
    let mut cache = Some(source(&m));
    let old = cache.as_ref().unwrap().vertex_normals().to_vec();
    m.positions[0].x = f32::INFINITY;
    assert!(
        SourceNormalField::refresh(&mut cache, &m, SourceNormalPolicy::StaticOriginalFaces)
            .is_err()
    );
    assert_eq!(cache.as_ref().unwrap().vertex_normals(), old);
    assert!(!cache.as_ref().unwrap().matches_mesh(&m));
}
#[test]
fn nonfinite_and_excessive_source_positions_reject_without_a_fallback() {
    for value in [f32::NAN, f32::INFINITY, f32::MAX] {
        let mut m = quad();
        m.positions[1].x = value;
        assert!(SourceNormalField::compute(&m, SourceNormalPolicy::StaticOriginalFaces).is_err());
    }
}
#[test]
fn explicit_source_paint_culling_uses_original_face_field_and_keeps_old_api() {
    let mut m = quad();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let f = source(&m);
    let mut old = m.clone();
    let r = request();
    assert_eq!(
        m.apply_paint_response_with_source_normals(&r, &f)
            .unwrap()
            .changed_vertices,
        0
    );
    assert_eq!(old.apply_paint_response(&r).unwrap().changed_vertices, 1);
    assert_eq!(old.colors[1], [0.25; 3]);
    assert_eq!(m.colors[1], [0.; 3]);
}
#[test]
fn stale_source_paint_field_rejects_atomically_before_material_allocation() {
    let mut m = quad();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    let f = source(&m);
    m.positions[1].x = 2.5;
    let before = serde_json::to_vec(&m).unwrap();
    let mut r = request();
    r.options.write_roughness = true;
    assert!(m.apply_paint_response_with_source_normals(&r, &f).is_err());
    assert!(m.materials.is_none());
    assert_eq!(before, serde_json::to_vec(&m).unwrap());
}
#[test]
fn protected_source_material_noop_preserves_redo_and_complete_document() {
    let mut m = quad();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(0.);
    let mut d = Document::new(m);
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().colors[0] = [0.2; 3];
    d.end_stroke();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let f = source(d.require_active_mesh().unwrap());
    let mut r = request();
    r.options.culling = false;
    r.options.write_roughness = true;
    d.begin_material_paint_stroke();
    d.require_active_mesh_mut()
        .unwrap()
        .apply_paint_response_with_source_normals(&r, &f)
        .unwrap();
    d.end_stroke();
    assert_eq!(before, d.to_json().unwrap());
    assert!(d.redo());
    assert!(d.require_active_mesh().unwrap().materials.is_none());
}
#[test]
fn triangle_origin_change_is_not_hidden_as_an_equivalent_cached_source() {
    let mut m = mesh(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[[0, 1, 2]]);
    let f = source(&m);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER]],
        uv: None,
    });
    assert!(!f.matches_mesh(&m));
    assert_eq!(
        source(&m).report().face_origin,
        SourceNormalFaceOrigin::StoredOriginalFaces
    );
}

#[test]
fn face_float32_assignment_precedes_incident_mean_and_cancellation() {
    let u = 2.0_f32.powi(-23);
    let m = mesh(
        &[
            [0., 0., 0.],
            [0., 1. + u, 0.],
            [0., 0., 1. + 2. * u],
            [0., 0., 1.],
            [0., 1., 0.],
        ],
        &[[0, 1, 2], [0, 3, 4]],
    );
    let f = source(&m);
    assert_eq!(words(f.face_normals()[0]), [0x3f800003, 0, 0]);
    assert_eq!(words(f.vertex_normals()[0]), [0x34400000, 0, 0]);
    // Unrounded face cross retains u^2 after cancellation, exactly one output ULP.
    let unrounded = ((1.0 + f64::from(u)) * (1.0 + 2.0 * f64::from(u)) - 1.0) * 0.5;
    assert_ne!((unrounded as f32).to_bits(), 0x34400000);
}
