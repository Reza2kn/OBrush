//! Independent actual packed-row/UV/upload-role gates, no GL runtime claim.
use super::*;
use obrush_core::{Document, MaskConvention, SourceGeometryOptions, SourceGeometryPass};
fn fixture() -> Mesh {
    let mut m=Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 0 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3\nf 3/4 5/5 4/6\n").unwrap().mesh;
    let uv = m
        .surface
        .as_ref()
        .and_then(|s| s.uv.as_ref())
        .expect("Fixture rich OBJ must retain its supplied UVs");
    assert_eq!(uv.coordinates.len(), 6);
    assert_eq!(uv.face_indices.len(), 2);
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m
}
fn pass(x: f64) -> SourceGeometryPass {
    SourceGeometryPass {
        center: [x, 0., 0.],
        eye_direction: [0., 0., -1.],
        local_radius_squared: 4.,
        original_face_index: 0,
    }
}
fn useful(doc: &mut Document, x: f64) {
    let mut o = SourceGeometryOptions::inflate();
    o.intensity = 0.625;
    let mut s = doc.begin_source_geometry_stroke(o).unwrap();
    assert_eq!(
        doc.apply_source_geometry_dab(&mut s, pass(x), 0.5, true)
            .unwrap()
            .changed_vertices,
        1
    );
    doc.end_source_geometry_stroke(&s).unwrap();
}
fn object<'a>(
    mesh: &'a Arc<Mesh>,
    state: Option<&'a Arc<obrush_core::SourceGeometryNormalState>>,
) -> RenderObject<'a> {
    RenderObject {
        id: 1,
        active: true,
        mesh,
        metadata: None,
        selected_normals: state,
        source: UploadSource::Shared(mesh),
    }
}
#[test]
fn typed_role_chooses_legacy_studio_and_warm_source_pbr() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    let mesh = Arc::new(doc.active_mesh().unwrap().clone());
    let state = doc.source_geometry_state(1).unwrap();
    let view = object(&mesh, Some(&state));
    assert_eq!(
        source_upload_normal_role(&view, &ViewportStyle::default()),
        SourceUploadNormalRole::Legacy
    );
    let pbr = ViewportStyle {
        shading: ShadingMode::Pbr,
        ..Default::default()
    };
    assert_eq!(
        source_upload_normal_role(&view, &pbr),
        SourceUploadNormalRole::SelectedRefresh
    );
    assert_eq!(
        source_upload_normal_role(&object(&mesh, None), &pbr),
        SourceUploadNormalRole::FreshStatic
    );
}
#[test]
fn selected_upload_preserves_raw_unselected_neighbor_literal() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    let mesh = doc.active_mesh().unwrap();
    let cache = doc.source_geometry_state(1).unwrap();
    let saved = serde_json::to_value(mesh).unwrap();
    let rows = packed_geometry_selected_rows(mesh, &cache).unwrap();
    assert_eq!(&rows[0][3..6], &[0.5, 0.5, 16.]);
    assert_eq!(&rows[1][3..6], &[0., 0., 16.]);
    assert_eq!(&rows[2][3..6], &[0., 0., 16.]);
    let fresh = obrush_core::SourceNormalField::compute(
        mesh,
        obrush_core::SourceNormalPolicy::StaticOriginalFaces,
    )
    .unwrap();
    assert_eq!(fresh.vertex_normals()[1], Vec3::new(0.5, 0.5, 16.));
    assert_eq!(serde_json::to_value(mesh).unwrap(), saved);
}
#[test]
fn derived_only_refresh_invalidates_upload_with_exact_same_mesh_arc() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    useful(&mut doc, 4.);
    let redo = doc.to_json().unwrap();
    assert!(doc.undo());
    let saved = doc.to_json().unwrap();
    let mesh = Arc::new(doc.active_mesh().unwrap().clone());
    let old = doc.source_geometry_state(1).unwrap();
    let mut identity = UploadIdentity::default();
    identity.update(&object(&mesh, Some(&old)));
    let mut opts = SourceGeometryOptions::inflate();
    opts.intensity = 0.;
    let mut no = doc.begin_source_geometry_stroke(opts).unwrap();
    let report = doc
        .apply_source_geometry_dab(&mut no, pass(4.), 0.5, true)
        .unwrap();
    assert_eq!(report.changed_vertices, 0);
    assert!(report.derived_cache_changed);
    let new = doc.source_geometry_state(1).unwrap();
    assert!(new.generation() > old.generation());
    assert!(!identity.matches(&object(&mesh, Some(&new))));
    assert!(identity.matches(&object(&mesh, Some(&old))));
    identity.update(&object(&mesh, Some(&new)));
    assert!(identity.matches(&object(&mesh, Some(&new))));
    assert_eq!(
        &packed_geometry_selected_rows(&mesh, &new).unwrap()[1][3..6],
        &[0.5, 0.5, 16.]
    );
    doc.end_source_geometry_stroke(&no).unwrap();
    assert_eq!(doc.to_json().unwrap(), saved);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), redo);
}
#[test]
fn warm_uv_seam_rows_duplicate_the_same_welded_raw_normal_words() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    let mesh = doc.active_mesh().unwrap();
    let state = doc.source_geometry_state(1).unwrap();
    let p = PackedUvMesh::build_with_source_geometry_normals(mesh, 65536, &state).unwrap();
    let geometric2 = p
        .correspondence
        .iter()
        .enumerate()
        .filter(|(_, r)| r[0] == 2)
        .collect::<Vec<_>>();
    assert_eq!(geometric2.len(), 2);
    assert_ne!(geometric2[0].1[1], geometric2[1].1[1]);
    for (i, row) in p.correspondence.iter().enumerate() {
        let v = row[0] as usize;
        assert_eq!(&p.vertices[i][3..6], &state.vertex_normals()[v].to_array());
    }
    for (i, _) in geometric2 {
        assert_eq!(&p.vertices[i][3..6], &[0., 0., 16.]);
    }
    assert_eq!(p.triangles.len(), 2);
    assert!(p.materials.is_none());
}
#[test]
fn stale_selected_upload_rejects_without_fresh_or_legacy_fallback() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    let state = doc.source_geometry_state(1).unwrap();
    let mut mesh = doc.active_mesh().unwrap().clone();
    mesh.positions[0].x = 0.25;
    assert!(packed_geometry_selected_rows(&mesh, &state).is_err());
    assert!(PackedUvMesh::build_with_source_geometry_normals(&mesh, 65536, &state).is_err());
}
#[test]
fn same_arc_source_reset_changes_upload_identity_and_restores_fresh_role() {
    let mut doc = Document::new(fixture());
    useful(&mut doc, 0.);
    let state = doc.source_geometry_state(1).unwrap();
    let mesh = Arc::new(doc.active_mesh().unwrap().clone());
    let mut identity = UploadIdentity::default();
    identity.update(&object(&mesh, Some(&state)));
    let before = doc.to_json().unwrap();
    assert!(doc.reset_source_geometry_normal_state().unwrap());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(!identity.matches(&object(&mesh, None)));
    let pbr = ViewportStyle {
        shading: ShadingMode::Pbr,
        ..Default::default()
    };
    assert_eq!(
        source_upload_normal_role(&object(&mesh, None), &pbr),
        SourceUploadNormalRole::FreshStatic
    );
}
