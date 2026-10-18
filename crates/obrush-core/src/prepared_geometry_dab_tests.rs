use super::*;
use crate::*;
use glam::{DMat4, DVec3};

fn fixture() -> Mesh {
    let mut m = Mesh::sphere(3);
    for (i, p) in m.positions.iter_mut().enumerate() {
        if i % 11 == 0 {
            *p *= 0.98;
        }
    }
    m.recompute_normals();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    for (i, u) in m.masks.iter_mut().enumerate() {
        *u = [0., 0.25, 1.][i % 3];
    }
    for (i, rgb) in m.colors.iter_mut().enumerate() {
        *rgb = [(i % 17) as f32 / 16., 0.25, 0.75];
    }
    m.materials = Some(vec![[0.6, 0.125]; m.positions.len()]);
    m.surface = Some(SurfaceTopology {
        faces: m
            .triangles
            .iter()
            .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
            .collect(),
        uv: Some(CornerUv {
            coordinates: vec![[0., 0.], [1., 0.], [0., 1.], [0., 0.]],
            face_indices: m
                .triangles
                .iter()
                .enumerate()
                .map(|(i, _)| [if i % 2 == 0 { 0 } else { 3 }, 1, 2, TRIANGLE_CORNER])
                .collect(),
        }),
    });
    m.validate().unwrap();
    m
}
fn input(step: usize) -> PaintPreparationInput {
    PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: (DMat4::orthographic_rh_gl(-2., 2., -2., 2., 0.1, 20.)
                * DMat4::look_at_rh(DVec3::new(0., 0., 4.), DVec3::ZERO, DVec3::Y))
            .to_cols_array(),
            viewport: [0., 0., 512., 512.],
        },
        screen: [273. + step as f64 * 3., 249.],
        screen_radius: 64.,
        pressure: 0.625,
        radius_factor: 0.25,
    }
}
fn options(tool: SourceGeometryTool, guard: bool) -> SourceGeometryOptions {
    SourceGeometryOptions {
        tool,
        intensity: 0.25,
        intensity_factor: 0.5,
        prevent_folds: guard,
        clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
        ..Default::default()
    }
}
fn plane(mirror: bool) -> Option<SourceMirrorPlane> {
    mirror.then_some(SourceMirrorPlane {
        origin: [0.; 3],
        normal: [1., 0., 0.],
    })
}
fn assert_same(a: &Document, b: &Document) {
    assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    let state = |d: &Document| {
        d.source_geometry_state(d.active_subtool_id().unwrap()).map(|s| {
        serde_json::json!({"positions_match":s.matches_mesh(d.active_mesh().unwrap()),
            "face_normals":s.face_normals().iter().map(|n|n.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
            "vertex_normals":s.vertex_normals().iter().map(|n|n.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
            "tree":s.ordered_tree_snapshot().unwrap(),"generation":s.generation()})
    })
    };
    assert_eq!(state(a), state(b));
}

#[test]
fn prepared_and_split_routes_match_all_tools_mirror_alpha_guard_and_history() {
    let tools = [
        SourceGeometryTool::Brush {
            clay: false,
            accumulate: false,
        },
        SourceGeometryTool::Brush {
            clay: false,
            accumulate: true,
        },
        SourceGeometryTool::Brush {
            clay: true,
            accumulate: false,
        },
        SourceGeometryTool::Inflate,
        SourceGeometryTool::Pinch,
        SourceGeometryTool::Smooth { tangent: false },
        SourceGeometryTool::Smooth { tangent: true },
        SourceGeometryTool::Flatten,
        SourceGeometryTool::Crease,
    ];
    for tool in tools {
        for mirror in [false, true] {
            for alpha in [false, true] {
                for guard in [false, true] {
                    let mut a = Document::new(fixture());
                    let mut b = Document::new(fixture());
                    let mut sa = a
                        .begin_source_geometry_selection_history(options(tool, guard))
                        .unwrap();
                    let mut sb = b
                        .begin_source_geometry_selection_history(options(tool, guard))
                        .unwrap();
                    let controls = SourceGeometryControls {
                        mirror,
                        alpha: alpha
                            .then(|| RasterAlpha::new(2, 2, vec![0, 64, 128, 255]).unwrap()),
                    };
                    let mut ta = SourceGeometryAlphaTracks::default();
                    let mut tb = ta;
                    let mut useful = 0;
                    for step in 0..3 {
                        let contacts = prepare_source_geometry_contacts(
                            a.active_mesh().unwrap(),
                            a.active_object_metadata().map(|m| &m.affine),
                            input(step),
                            plane(mirror),
                        )
                        .unwrap();
                        let prepared = b
                            .prepare_source_geometry_controlled_dab(
                                &mut sb,
                                input(step),
                                plane(mirror),
                            )
                            .unwrap();
                        assert_eq!(
                            serde_json::to_vec(&contacts).unwrap(),
                            serde_json::to_vec(&prepared.contacts()).unwrap()
                        );
                        let ra = a
                            .apply_source_geometry_controlled_dab(
                                &mut sa,
                                contacts.passes,
                                0.625,
                                true,
                                &controls,
                                ta,
                            )
                            .unwrap();
                        let rb = prepared.apply(0.625, true, &controls, tb).unwrap();
                        assert_eq!(
                            serde_json::to_vec(&ra).unwrap(),
                            serde_json::to_vec(&rb).unwrap()
                        );
                        useful += ra.combined.changed_vertices;
                        ta = ra.tracks;
                        tb = rb.tracks;
                        assert_same(&a, &b);
                        assert_eq!(sa.proxy_positions(), sb.proxy_positions());
                    }
                    assert!(
                        useful > 0,
                        "{tool:?} mirror={mirror} alpha={alpha} guard={guard}"
                    );
                    a.end_source_geometry_stroke(&sa).unwrap();
                    b.end_source_geometry_stroke(&sb).unwrap();
                    assert_same(&a, &b);
                    assert!(a.undo() && b.undo());
                    assert_same(&a, &b);
                    assert!(a.redo() && b.redo());
                    assert_same(&a, &b);
                }
            }
        }
    }
}

fn warm() -> (Document, SourceGeometryStroke, SourceGeometryAlphaTracks) {
    let mut d = Document::new(fixture());
    let mut stroke = d
        .begin_source_geometry_selection_history(options(SourceGeometryTool::Inflate, false))
        .unwrap();
    let tracks = d
        .prepare_source_geometry_controlled_dab(&mut stroke, input(0), None)
        .unwrap()
        .apply(
            0.625,
            true,
            &SourceGeometryControls::default(),
            SourceGeometryAlphaTracks::default(),
        )
        .unwrap()
        .tracks;
    (d, stroke, tracks)
}

#[test]
fn warm_prepared_entry_checks_mesh_once_split_route_checks_twice_each_call() {
    let (mut a, mut sa, mut tracks) = warm();
    for step in 1..3 {
        crate::mesh::validation_probe::reset();
        let contacts =
            prepare_source_geometry_contacts(a.active_mesh().unwrap(), None, input(step), None)
                .unwrap();
        let report = a
            .apply_source_geometry_controlled_dab(
                &mut sa,
                contacts.passes,
                0.625,
                true,
                &SourceGeometryControls::default(),
                tracks,
            )
            .unwrap();
        assert_eq!(crate::mesh::validation_probe::count(), 2);
        tracks = report.tracks;
        crate::mesh::validation_probe::reset();
        crate::source_geometry::validation_probe::reset();
        let report = a
            .prepare_source_geometry_controlled_dab(&mut sa, input(step), None)
            .unwrap()
            .apply(0.625, true, &SourceGeometryControls::default(), tracks)
            .unwrap();
        assert_eq!(crate::mesh::validation_probe::count(), 1);
        // The exclusive prepared value carries the exact retained match from
        // contact preparation into apply; no second whole-state match runs.
        assert_eq!(crate::source_geometry::validation_probe::counts()[1], 1);
        tracks = report.tracks;
    }
    a.cancel_source_geometry_stroke(&sa).unwrap();
}

#[test]
fn prepared_route_rejects_invalid_and_stale_mesh_channels_between_calls() {
    for mutation in 0..12 {
        let (mut d, mut stroke, tracks) = warm();
        let mesh = d.mesh.as_mut().unwrap();
        match mutation {
            0 => mesh.positions[0].x = f32::NAN,
            1 => mesh.triangles[0][0] = u32::MAX,
            2 => mesh.normals[0].x = f32::INFINITY,
            3 => mesh.masks[0] = 2.,
            4 => mesh.colors[0][0] = f32::NAN,
            5 => mesh.materials.as_mut().unwrap()[0][0] = -1.,
            6 => mesh.surface.as_mut().unwrap().faces[0][0] = u32::MAX,
            7 => {
                mesh.surface
                    .as_mut()
                    .unwrap()
                    .uv
                    .as_mut()
                    .unwrap()
                    .coordinates[0][0] = f32::NAN
            }
            8 => mesh.positions[0].x += 0.001,
            9 => mesh.colors[0][0] = 0.375,
            10 => {
                mesh.surface
                    .as_mut()
                    .unwrap()
                    .uv
                    .as_mut()
                    .unwrap()
                    .coordinates[0][0] = 0.375
            }
            _ => mesh.mask_convention = MaskConvention::ProtectedF32V1,
        }
        let before = serde_json::to_vec(d.active_mesh().unwrap()).unwrap();
        let result = d
            .prepare_source_geometry_controlled_dab(&mut stroke, input(1), None)
            .and_then(|p| p.apply(0.625, true, &SourceGeometryControls::default(), tracks));
        let expected = match mutation {
            0..=7 => SourceGeometryError::InvalidInput,
            8 => SourceGeometryError::StaleNormalState,
            9 | 10 => SourceGeometryError::StrokeBindingChanged,
            _ => SourceGeometryError::MaskConvention,
        };
        assert_eq!(result.err(), Some(expected), "mutation {mutation}");
        assert_eq!(
            serde_json::to_vec(d.active_mesh().unwrap()).unwrap(),
            before
        );
    }
}

#[test]
fn invalid_tracks_pressure_and_mirror_refusal_are_atomic() {
    for mutation in 0..3 {
        let (mut d, mut stroke, mut tracks) = warm();
        let before = d.to_json().unwrap();
        if mutation == 0 {
            tracks.primary.frame[0] = f32::NAN;
        }
        let controls = SourceGeometryControls::default();
        let prepared = d
            .prepare_source_geometry_controlled_dab(&mut stroke, input(1), plane(mutation == 2))
            .unwrap();
        let error = prepared
            .apply(
                if mutation == 1 { f64::NAN } else { 0.625 },
                true,
                &controls,
                tracks,
            )
            .unwrap_err();
        assert_eq!(error, SourceGeometryError::InvalidInput);
        assert_eq!(d.to_json().unwrap(), before);
        d.cancel_source_geometry_stroke(&stroke).unwrap();
    }
}

#[test]
fn dropping_prepared_value_and_no_contact_preserve_document_and_real_redo() {
    let mut d = Document::new(fixture());
    d.begin_stroke();
    d.name = "real future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let mut stroke = d
        .begin_source_geometry_selection_history(options(SourceGeometryTool::Inflate, false))
        .unwrap();
    {
        let prepared = d
            .prepare_source_geometry_controlled_dab(&mut stroke, input(0), None)
            .unwrap();
        assert!(prepared.contacts().passes[0].is_some());
    }
    assert_eq!(d.to_json().unwrap(), before);
    let report = d
        .prepare_source_geometry_controlled_dab(&mut stroke, input(0), None)
        .unwrap()
        .apply(
            0.625,
            false,
            &SourceGeometryControls::default(),
            SourceGeometryAlphaTracks::default(),
        )
        .unwrap();
    assert_eq!(report.combined.noop, Some(SourceGeometryNoop::NoContact));
    assert_eq!(d.to_json().unwrap(), before);
    d.end_source_geometry_stroke(&stroke).unwrap();
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}

#[test]
fn prepared_value_cannot_apply_another_documents_stroke() {
    let (mut a, sa, _) = warm();
    let (mut b, mut sb, tracks) = warm();
    let before = a.to_json().unwrap();
    let result = a
        .prepare_source_geometry_controlled_dab(&mut sb, input(1), None)
        .unwrap()
        .apply(0.625, true, &SourceGeometryControls::default(), tracks);
    assert_eq!(result.unwrap_err(), SourceGeometryError::NoOpenStroke);
    assert_eq!(a.to_json().unwrap(), before);
    a.cancel_source_geometry_stroke(&sa).unwrap();
    b.cancel_source_geometry_stroke(&sb).unwrap();
}
