//! Read-only Source Paint cursor contacts over the actual sealed publication.
//! Both queries share the existing work budget. Mirroring reflects a LOCAL ray;
//! it does not reflect a screen coordinate or the first surface intersection.
use glam::{DMat4, DVec3};
use obrush_core::{
    ObjectAffineF32V1, PaintViewCapture, SceneRay64, SourceMirrorPlane, SourcePickQueryBudget,
    SourcePreparedPickQuery,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Dot {
    pub local: [f64; 3],
    pub world: [f64; 3],
    pub screen: [f64; 3],
    pub triangle: usize,
    pub original_face: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub primary: Option<Dot>,
    pub mirrored: Option<Dot>,
    pub plane: Option<SourceMirrorPlane>,
    pub face_tests: usize,
    pub triangle_tests: usize,
}
impl Frame {
    pub(crate) fn diagnostics(self) -> serde_json::Value {
        let dot = |d: Dot| serde_json::json!({"local":d.local,"world":d.world,"screen":d.screen,"triangle":d.triangle,"original_face":d.original_face});
        serde_json::json!({"primary":self.primary.map(dot),"mirrored":self.mirrored.map(dot),"plane":self.plane,"face_tests":self.face_tests,"triangle_tests":self.triangle_tests})
    }
}
pub(crate) struct Capture {
    pub query: std::sync::Arc<SourcePreparedPickQuery>,
    pub pose: Option<ObjectAffineF32V1>,
    pub plane: Option<SourceMirrorPlane>,
}
/// A zero radius never changes eligibility of the independent cursor contact.
/// Cursor rendering has no alpha lease and cannot initialize brush tracks.
pub(crate) fn prepare(
    query: &SourcePreparedPickQuery,
    pose: Option<&ObjectAffineF32V1>,
    view: PaintViewCapture,
    screen: [f64; 2],
    plane: Option<SourceMirrorPlane>,
) -> Result<Frame, String> {
    view.validate()?;
    if let Some(pose) = pose {
        pose.validate()?;
    }
    if let Some(plane) = plane {
        plane.validate().map_err(|e| e.to_string())?;
    }
    let matrix = pose.map_or(DMat4::IDENTITY, |p| {
        DMat4::from_cols_array(&p.local_to_world.map(f64::from))
    });
    let inverse = matrix.inverse();
    let near = view.unproject(screen, 0.)?;
    let far = view.unproject(screen, 0.1)?;
    let raw = view.ray(screen)?;
    let local = SceneRay64 {
        origin: inverse.transform_point3(raw.origin),
        direction: inverse.transform_vector3(raw.direction).normalize(),
    };
    local.validate()?;
    let mut budget = SourcePickQueryBudget::default();
    let dot = |hit: obrush_core::PreparedSourcePaintHit| -> Result<Dot, String> {
        let world = matrix.transform_point3(DVec3::from_array(hit.local_position));
        let projected = view.project(world)?;
        Ok(Dot {
            local: hit.local_position,
            world: world.to_array(),
            screen: projected,
            triangle: hit.triangle_index,
            original_face: hit.original_face_index,
        })
    };
    let primary = query
        .ray(None, local, &mut budget)
        .map_err(|e| format!("Source cursor primary query {e:?}"))?
        .map(dot)
        .transpose()?;
    let mirrored = if primary.is_some() {
        if let Some(plane) = plane {
            let origin = DVec3::from_array(
                plane
                    .reflect(inverse.transform_point3(near).to_array())
                    .map_err(|e| e.to_string())?,
            );
            let end = DVec3::from_array(
                plane
                    .reflect(inverse.transform_point3(far).to_array())
                    .map_err(|e| e.to_string())?,
            );
            let reflected = SceneRay64 {
                origin,
                direction: (end - origin).normalize(),
            };
            reflected.validate()?;
            query
                .ray(None, reflected, &mut budget)
                .map_err(|e| format!("Source cursor mirrored query {e:?}"))?
                .map(dot)
                .transpose()?
        } else {
            None
        }
    } else {
        None
    };
    Ok(Frame {
        primary,
        mirrored,
        plane,
        face_tests: obrush_core::MAX_SOURCE_PICK_FACE_TESTS - budget.attributes.face_tests,
        triangle_tests: obrush_core::MAX_SOURCE_PICK_TRIANGLE_TESTS
            - budget.attributes.triangle_tests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{Document, Mesh, SourcePickQueryAdmission};
    use std::sync::Arc;
    fn mesh() -> Mesh {
        let mut m=Mesh::import_obj(b"v -0.8 -0.8 0\nv -0.2 -0.8 0\nv -0.5 0.8 0\nv 0.2 -0.8 0.5\nv 0.8 -0.8 0.5\nv 0.5 0.8 0.5\nf 1 2 3\nf 4 5 6\n").unwrap().mesh;
        m.surface = Some(obrush_core::SurfaceTopology {
            faces: vec![
                [0, 1, 2, obrush_core::TRIANGLE_CORNER],
                [3, 4, 5, obrush_core::TRIANGLE_CORNER],
            ],
            uv: None,
        });
        m
    }
    fn ready(mesh: Arc<Mesh>) -> SourcePreparedPickQuery {
        let mut a = SourcePickQueryAdmission::new_for_batch(mesh, 2).unwrap();
        while !a.step(16_384).unwrap().ready() {}
        a.finish().unwrap()
    }
    fn view() -> PaintViewCapture {
        PaintViewCapture {
            world_to_clip: DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        }
    }
    fn plane() -> SourceMirrorPlane {
        SourceMirrorPlane {
            origin: [0.; 3],
            normal: [1., 0., 0.],
        }
    }
    #[test]
    fn mirrored_marker_picks_the_second_surface_at_its_own_depth_without_document_changes() {
        let d = Document::new(mesh());
        let before = d.to_json().unwrap();
        let query = ready(Arc::new(d.require_active_mesh().unwrap().snapshot()));
        let frame = prepare(&query, None, view(), [25., 55.], Some(plane())).unwrap();
        let a = frame.primary.unwrap();
        let b = frame.mirrored.unwrap();
        for (actual, expected) in a.local.into_iter().zip([-0.5, -0.1, 0.]) {
            assert!((actual - expected).abs() < 1e-12);
        }
        for (actual, expected) in b.local.into_iter().zip([0.5, -0.1, 0.5]) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(a.original_face, Some(0));
        assert_eq!(b.original_face, Some(1));
        assert!((a.screen[0] - 25.).abs() < 1e-12);
        assert!((b.screen[0] - 75.).abs() < 1e-12);
        assert_ne!(a.local[2], b.local[2]);
        assert_eq!(frame.face_tests, 4);
        assert_eq!(before, d.to_json().unwrap());
    }
    #[test]
    fn actual_mirror_miss_and_primary_miss_never_publish_a_stale_or_reflected_contact() {
        let mut m = mesh();
        m.triangles.truncate(1);
        m.surface = None;
        let query = ready(Arc::new(m));
        let first = prepare(&query, None, view(), [25., 55.], Some(plane())).unwrap();
        assert!(first.primary.is_some());
        assert!(first.mirrored.is_none());
        let missed = prepare(&query, None, view(), [75., 55.], Some(plane())).unwrap();
        assert!(missed.primary.is_none());
        assert!(missed.mirrored.is_none());
        assert_eq!(missed.face_tests, 1);
        let off = prepare(&query, None, view(), [25., 55.], None).unwrap();
        assert!(off.primary.is_some());
        assert!(off.mirrored.is_none());
        assert_eq!(off.face_tests, 1);
    }
    #[test]
    fn reflection_uses_local_space_under_distant_reflected_nonuniform_shear_pose() {
        let query = ready(Arc::new(mesh()));
        let matrix = DMat4::from_cols_array(&[
            1.5, 0., 0., 0., 0.25, -0.75, 0., 0., 0.5, 0.25, 2., 0., 262144., -131072., 4096., 1.,
        ]);
        let pose = ObjectAffineF32V1 {
            local_to_world: matrix.to_cols_array().map(|x| x as f32),
            interaction_center: [0.; 3],
        };
        let view = PaintViewCapture {
            world_to_clip: matrix.inverse().to_cols_array(),
            ..view()
        };
        let frame = prepare(&query, Some(&pose), view, [25., 55.], Some(plane())).unwrap();
        let b = frame.mirrored.unwrap();
        assert!((b.local[0] - 0.5).abs() < 1e-9);
        assert!((b.local[1] + 0.1).abs() < 1e-9);
        assert!((b.local[2] - 0.5).abs() < 1e-9);
        let expected = matrix.transform_point3(DVec3::new(0.5, -0.1, 0.5));
        assert!((DVec3::from_array(b.world) - expected).length() < 1e-8);
        assert!((b.screen[0] - 75.).abs() < 1e-5);
        assert!((b.screen[1] - 55.).abs() < 1e-5);
    }
    #[test]
    fn two_ray_preparation_indexes_a_grid_that_exceeds_the_combined_linear_budget() {
        let side = 64;
        let mut positions = Vec::new();
        let mut triangles = Vec::new();
        for y in 0..=side {
            for x in 0..=side {
                positions.push(glam::Vec3::new(
                    x as f32 / 32. - 1.,
                    y as f32 / 32. - 1.,
                    0.,
                ));
            }
        }
        for y in 0..side {
            for x in 0..side {
                let a = y * (side + 1) + x;
                triangles.push([a, a + 1, a + side + 2]);
                triangles.push([a, a + side + 2, a + side + 1]);
            }
        }
        use std::fmt::Write as _;
        let mut obj = String::new();
        for p in positions {
            writeln!(obj, "v {} {} {}", p.x, p.y, p.z).unwrap();
        }
        for t in triangles {
            writeln!(obj, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
        }
        let query = ready(Arc::new(Mesh::from_obj(&obj).unwrap()));
        assert!(query.progress().indexed);
        let frame = prepare(&query, None, view(), [37.5, 55.], Some(plane())).unwrap();
        assert!(frame.primary.is_some() && frame.mirrored.is_some());
        assert!(frame.face_tests < 8192 && frame.triangle_tests < 8192);
    }
    #[test]
    fn actual_paint_capture_keeps_readonly_geometry_through_attribute_dabs_and_refuses_failure() {
        use crate::source_paint_host::{
            PaintMode, RawPressureSample, SourcePaintInput, SourcePaintPreferences,
            SourcePaintSession, apply_shared,
        };
        use obrush_core::{BrushKind, BrushSettings, MaskConvention};
        let mut m = mesh();
        m.install_mask_channel(vec![1.; 6], MaskConvention::UnprotectedF32V1)
            .unwrap();
        let mut d = Document::new(m);
        let before = d.to_json().unwrap();
        let query = Arc::new(ready(Arc::new(d.require_active_mesh().unwrap().snapshot())));
        let preferences = SourcePaintPreferences {
            mode: PaintMode::SourceOrdinary,
            radius: 100.,
            radius_factor: 0.,
            ..Default::default()
        };
        let settings = BrushSettings {
            kind: BrushKind::Paint,
            symmetry_x: true,
            ..Default::default()
        };
        let input = SourcePaintInput {
            view: view(),
            screen: [25., 55.],
            raw: RawPressureSample::default(),
            in_contact: true,
        };
        let session = SourcePaintSession::prepare_tracked_for_stroke(
            &mut d,
            &preferences,
            &settings,
            false,
            false,
            input,
            Arc::new(std::sync::Mutex::new(Default::default())),
        )
        .unwrap();
        session.lock().unwrap().capture_cursor(query.clone());
        d.begin_material_paint_stroke();
        assert!(
            apply_shared(&mut d, &session, input)
                .unwrap()
                .changed_vertices
                > 0
        );
        let held = session.lock().unwrap().cursor_capture(&d).unwrap();
        assert!(Arc::ptr_eq(&held.query, &query));
        let frame = prepare(
            &held.query,
            held.pose.as_ref(),
            view(),
            [25., 55.],
            held.plane,
        )
        .unwrap();
        assert!(frame.primary.is_some() && frame.mirrored.is_some());
        let after = d.to_json().unwrap();
        assert_ne!(after, before);
        let _ = prepare(
            &held.query,
            held.pose.as_ref(),
            view(),
            [30., 55.],
            held.plane,
        )
        .unwrap();
        assert_eq!(after, d.to_json().unwrap());
        let mut invalid = input;
        invalid.screen[0] = f64::NAN;
        assert!(apply_shared(&mut d, &session, invalid).is_err());
        assert!(session.lock().unwrap().cursor_capture(&d).is_none());
        d.end_stroke();
        assert!(d.undo());
        assert_eq!(before, d.to_json().unwrap());
    }
    #[test]
    fn nonfinite_or_invalid_frames_reject_instead_of_drawing_a_fallback_marker() {
        let query = ready(Arc::new(mesh()));
        assert!(prepare(&query, None, view(), [f64::NAN, 55.], Some(plane())).is_err());
        let mut bad = plane();
        bad.normal = [0.; 3];
        assert!(prepare(&query, None, view(), [25., 55.], Some(bad)).is_err());
        let invalid = PaintViewCapture {
            world_to_clip: [0.; 16],
            ..view()
        };
        assert!(prepare(&query, None, invalid, [25., 55.], None).is_err());
    }
}
