//! Source Drag's moving center, independent of a display host.
//! The prior intersection is projected onto the next sample ray in object
//! space. The returned displacement is applied to live (not stroke-proxy)
//! positions by the shared Source geometry transaction.
use crate::{ObjectAffineF32V1, PaintViewCapture, SourceGeometryError, SourceMirrorPlane};
use glam::{DMat4, DVec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedSourceDragCenter {
    pub center: [f64; 3],
    pub direction: [f64; 3],
    pub eye_direction: [f64; 3],
}

pub fn project_center(
    prior: [f64; 3],
    view: PaintViewCapture,
    screen: [f64; 2],
    pose: Option<&ObjectAffineF32V1>,
) -> Result<ProjectedSourceDragCenter, SourceGeometryError> {
    project_center_with_plane(prior, view, screen, pose, None)
}

/// Source Drag's symmetric picker projects its already-reflected prior
/// intersection onto a ray reflected in object space. It does not repick a
/// second surface intersection after the primary side has moved.
pub fn project_center_mirrored(
    prior: [f64; 3],
    view: PaintViewCapture,
    screen: [f64; 2],
    pose: Option<&ObjectAffineF32V1>,
    plane: SourceMirrorPlane,
) -> Result<ProjectedSourceDragCenter, SourceGeometryError> {
    project_center_with_plane(prior, view, screen, pose, Some(plane))
}

fn project_center_with_plane(
    prior: [f64; 3],
    view: PaintViewCapture,
    screen: [f64; 2],
    pose: Option<&ObjectAffineF32V1>,
    plane: Option<SourceMirrorPlane>,
) -> Result<ProjectedSourceDragCenter, SourceGeometryError> {
    if prior.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.) {
        return Err(SourceGeometryError::InvalidInput);
    }
    let near = view
        .unproject(screen, 0.)
        .map_err(|_| SourceGeometryError::InvalidInput)?;
    let far = view
        .unproject(screen, 0.1)
        .map_err(|_| SourceGeometryError::InvalidInput)?;
    let inverse = if let Some(pose) = pose {
        pose.validate()
            .map_err(|_| SourceGeometryError::InvalidInput)?;
        DMat4::from_cols_array(&pose.local_to_world.map(f64::from)).inverse()
    } else {
        DMat4::IDENTITY
    };
    if !inverse.is_finite() {
        return Err(SourceGeometryError::InvalidInput);
    }
    let mut near = inverse.transform_point3(near);
    let mut far = inverse.transform_point3(far);
    if let Some(plane) = plane {
        near = DVec3::from_array(plane.reflect(near.to_array())?);
        far = DVec3::from_array(plane.reflect(far.to_array())?);
    }
    let line = far - near;
    let denominator = line.length_squared();
    if !near.is_finite() || !far.is_finite() || denominator <= 0. || !denominator.is_finite() {
        return Err(SourceGeometryError::InvalidInput);
    }
    let prior = DVec3::from_array(prior);
    let center = near + line * (prior - near).dot(line) / denominator;
    let direction = center - prior;
    let eye = line.normalize();
    if !center.is_finite()
        || !direction.is_finite()
        || !eye.is_finite()
        || center.abs().max_element() > 1_000_000.
        || direction.abs().max_element() > 1_000_000.
    {
        return Err(SourceGeometryError::InvalidInput);
    }
    Ok(ProjectedSourceDragCenter {
        center: center.to_array(),
        direction: direction.to_array(),
        eye_direction: eye.to_array(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Document, MaskConvention, Mesh, SourceGeometryAlphaTracks, SourceGeometryControls,
        SourceGeometryOptions, SourceGeometryPass, Vec3,
    };

    #[test]
    fn moving_ray_updates_center_without_depth_drift() {
        let view = PaintViewCapture {
            world_to_clip: DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        };
        let projected = project_center([0., 0., 0.2], view, [60., 50.], None).unwrap();
        let center = projected.center;
        let delta = projected.direction;
        let eye = projected.eye_direction;
        assert!((center[0] - 0.2).abs() < 1e-12);
        assert_eq!(center[1], 0.);
        assert!((center[2] - 0.2).abs() < 1e-12);
        assert!((delta[0] - 0.2).abs() < 1e-12);
        assert_eq!(eye, [0., 0., 1.]);
    }

    #[test]
    fn mirrored_moving_ray_reflects_center_direction_and_eye() {
        let view = PaintViewCapture {
            world_to_clip: DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        };
        let plane = SourceMirrorPlane {
            origin: [0., 0., 0.],
            normal: [1., 0., 0.],
        };
        let primary = project_center([-0.2, 0., 0.2], view, [35., 50.], None).unwrap();
        let mirror =
            project_center_mirrored([0.2, 0., 0.2], view, [35., 50.], None, plane).unwrap();
        for i in 0..3 {
            let sign = if i == 0 { -1. } else { 1. };
            assert!((mirror.center[i] - sign * primary.center[i]).abs() < 1e-12);
            assert!((mirror.direction[i] - sign * primary.direction[i]).abs() < 1e-12);
            assert!((mirror.eye_direction[i] - sign * primary.eye_direction[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn symmetric_live_drag_is_one_masked_transaction_and_rejection_cancels() {
        let mut mesh = Mesh::new(
            vec![
                Vec3::new(-0.6, -0.1, 0.),
                Vec3::new(-0.4, -0.1, 0.),
                Vec3::new(-0.5, 0.1, 0.),
                Vec3::new(0.4, -0.1, 0.),
                Vec3::new(0.6, -0.1, 0.),
                Vec3::new(0.5, 0.1, 0.),
            ],
            vec![[0, 1, 2], [3, 4, 5]],
        );
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks = vec![1., 1., 0., 1., 0.5, 1.];
        let mut doc = Document::new(mesh);
        let before = doc.to_json().unwrap();
        let depth = doc.history_depths();
        let mut stroke = doc
            .begin_source_geometry_selection_history(SourceGeometryOptions::drag())
            .unwrap();
        stroke.set_drag_direction([0.1, 0., 0.]).unwrap();
        stroke.set_drag_mirror_direction([-0.1, 0., 0.]).unwrap();
        let left = SourceGeometryPass {
            center: [-0.5, 0., 0.],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 0.09,
            original_face_index: 0,
        };
        let right = SourceGeometryPass {
            center: [0.5, 0., 0.],
            original_face_index: 1,
            ..left
        };
        let report = doc
            .apply_source_geometry_controlled_dab(
                &mut stroke,
                [Some(left), Some(right)],
                1.,
                true,
                &SourceGeometryControls {
                    alpha: None,
                    mirror: true,
                },
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        assert!(report.combined.changed_vertices > 0);
        let positions = &doc.require_active_mesh().unwrap().positions;
        assert!(positions[0].x > -0.6 && positions[3].x < 0.4);
        assert_eq!(positions[2], Vec3::new(-0.5, 0.1, 0.));
        assert!(positions[4].x < 0.6);
        assert!(doc
            .apply_source_geometry_controlled_dab(
                &mut stroke,
                [
                    Some(left),
                    Some(SourceGeometryPass {
                        center: [f64::NAN, 0., 0.],
                        ..right
                    })
                ],
                1.,
                true,
                &SourceGeometryControls {
                    alpha: None,
                    mirror: true
                },
                SourceGeometryAlphaTracks::default(),
            )
            .is_err());
        doc.cancel_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(doc.to_json().unwrap(), before);
        assert_eq!(doc.history_depths(), depth);
        assert!(doc.end_source_geometry_stroke(&stroke).is_err());
    }

    #[test]
    fn invalid_ray_rejects_before_geometry() {
        let view = PaintViewCapture {
            world_to_clip: DMat4::IDENTITY.to_cols_array(),
            viewport: [0., 0., 100., 100.],
        };
        assert!(project_center([f64::NAN, 0., 0.], view, [50., 50.], None).is_err());
        assert!(project_center([0., 0., 0.], view, [f64::NAN, 50.], None).is_err());
    }

    #[test]
    fn live_drag_is_one_transaction_and_failed_next_step_cancels_exactly() {
        let mut mesh = Mesh::new(
            vec![
                Vec3::new(-0.2, -0.2, 0.),
                Vec3::new(0.2, -0.2, 0.),
                Vec3::new(0.2, 0.2, 0.),
                Vec3::new(-0.2, 0.2, 0.),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks = vec![1., 1., 0.5, 0.];
        let mut doc = Document::new(mesh);
        let before = doc.to_json().unwrap();
        let before_depth = doc.history_depths();
        let pass = SourceGeometryPass {
            center: [0., 0., 0.],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 1.,
            original_face_index: 0,
        };
        let mut stroke = doc
            .begin_source_geometry_selection_history(SourceGeometryOptions::drag())
            .unwrap();
        stroke.set_drag_direction([0.1, 0., 0.]).unwrap();
        assert!(
            doc.apply_source_geometry_dab(&mut stroke, pass, 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        let first = doc.require_active_mesh().unwrap().positions.clone();
        assert_eq!(first[3], Vec3::new(-0.2, 0.2, 0.));
        stroke.set_drag_direction([0.1, 0., 0.]).unwrap();
        let second_pass = SourceGeometryPass {
            center: [0.1, 0., 0.],
            ..pass
        };
        assert!(
            doc.apply_source_geometry_dab(&mut stroke, second_pass, 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        let second = doc.require_active_mesh().unwrap().positions.clone();
        assert!(second[0].x > first[0].x && second[1].x > first[1].x);
        assert_eq!(second[3], first[3]);
        assert!(stroke.set_drag_direction([f64::NAN, 0., 0.]).is_err());
        doc.cancel_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(doc.to_json().unwrap(), before);
        assert_eq!(doc.history_depths(), before_depth);
        assert!(doc.end_source_geometry_stroke(&stroke).is_err());
    }

    #[test]
    fn selected_history_zero_travel_does_not_commit_a_stroke() {
        let mut mesh = Mesh::new(
            vec![
                Vec3::new(-0.2, -0.2, 0.),
                Vec3::new(0.2, -0.2, 0.),
                Vec3::new(0.0, 0.2, 0.),
            ],
            vec![[0, 1, 2]],
        );
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let mut doc = Document::new(mesh);
        let before = doc.to_json().unwrap();
        let depths = doc.history_depths();
        let mut stroke = doc
            .begin_source_geometry_selection_history(SourceGeometryOptions::drag())
            .unwrap();
        stroke.set_drag_direction([0., 0., 0.]).unwrap();
        doc.apply_source_geometry_dab(
            &mut stroke,
            SourceGeometryPass {
                center: [0., 0., 0.],
                eye_direction: [0., 0., -1.],
                local_radius_squared: 1.,
                original_face_index: 0,
            },
            0.5,
            true,
        )
        .unwrap();
        doc.end_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(doc.to_json().unwrap(), before);
        assert_eq!(doc.history_depths(), depths);
    }
}
