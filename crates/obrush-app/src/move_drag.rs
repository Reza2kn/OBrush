//! A Move gesture uses the camera plane and depth captured at pointer-down.
use eframe::egui::{Event, PointerButton, Pos2, Rect};
use glam::{DVec2, Vec3};

const MAX_SAME_FRAME_MOVE_SAMPLES: usize = 64;

pub(crate) struct SameFrameMovePath {
    pub press: Pos2,
    pub samples: Vec<Pos2>,
}

/// Reconstruct the fast mouse contact that egui's summary reduces to its
/// release position. Ambiguous or unbounded raw streams retain the old path.
pub(crate) fn same_frame_move_path(events: &[Event], viewport: Rect) -> Option<SameFrameMovePath> {
    let mut press = None;
    let mut samples = Vec::new();
    let mut moved = false;
    let mut released = false;
    for event in events {
        match event {
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                ..
            } if press.is_none() && !released && pos.is_finite() && viewport.contains(*pos) => {
                press = Some(*pos);
            }
            Event::PointerMoved(pos) if press.is_some() && !released => {
                if !pos.is_finite() || !viewport.contains(*pos) {
                    return None;
                }
                let previous = samples.last().copied().or(press).unwrap();
                if *pos != previous {
                    moved = true;
                    if samples.len() == MAX_SAME_FRAME_MOVE_SAMPLES {
                        return None;
                    }
                    samples.push(*pos);
                }
            }
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                ..
            } if press.is_some() && !released && pos.is_finite() && viewport.contains(*pos) => {
                released = true;
                if samples.last().copied().or(press) != Some(*pos) {
                    if samples.len() == MAX_SAME_FRAME_MOVE_SAMPLES {
                        return None;
                    }
                    samples.push(*pos);
                }
            }
            Event::PointerButton { .. } | Event::PointerGone => return None,
            Event::PointerMoved(_) if released => return None,
            _ => {}
        }
    }
    if moved && released {
        Some(SameFrameMovePath {
            press: press?,
            samples,
        })
    } else {
        None
    }
}

pub(crate) struct MoveDrag {
    right: Vec3,
    up: Vec3,
    units_per_pixel: f64,
    pixels: DVec2,
}

impl MoveDrag {
    pub(crate) fn new(right: Vec3, up: Vec3, units_per_pixel: f32) -> Self {
        Self {
            right,
            up,
            units_per_pixel: f64::from(units_per_pixel),
            pixels: DVec2::ZERO,
        }
    }

    pub(crate) fn advance(&mut self, from: Pos2, to: Pos2) -> Vec3 {
        if from.x.is_finite() && from.y.is_finite() && to.x.is_finite() && to.y.is_finite() {
            self.pixels += DVec2::new(
                f64::from(to.x) - f64::from(from.x),
                f64::from(to.y) - f64::from(from.y),
            );
        }
        ((self.right.as_dvec3() * self.pixels.x - self.up.as_dvec3() * self.pixels.y)
            * self.units_per_pixel)
            .as_vec3()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Event, Modifiers, PointerButton, Rect, Sense, Vec2};
    use obrush_core::{BrushKind, BrushSettings, Document, Mesh, MoveStroke};

    #[test]
    fn event_chunking_and_round_trip_keep_the_same_camera_plane_target() {
        let mut coarse = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        let target = coarse.advance(Pos2::ZERO, Pos2::new(160.0, -80.0));
        let mut fine = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        for step in 0..160 {
            fine.advance(
                Pos2::new(step as f32, -0.5 * step as f32),
                Pos2::new((step + 1) as f32, -0.5 * (step + 1) as f32),
            );
        }
        assert_eq!(fine.advance(Pos2::ZERO, Pos2::ZERO), target);
        assert_eq!(
            fine.advance(Pos2::new(160.0, -80.0), Pos2::ZERO),
            Vec3::ZERO
        );

        let mut wide = MoveDrag::new(Vec3::X, Vec3::Y, 0.004);
        let start = Pos2::new(1111.1234, 1200.4321);
        let middle = Pos2::new(16.9876, 17.6543);
        let end = Pos2::new(1777.8765, 9.8765);
        wide.advance(start, middle);
        wide.advance(middle, end);
        assert_eq!(wide.advance(end, start), Vec3::ZERO);
    }

    #[test]
    fn invalid_input_does_not_change_the_target() {
        let mut drag = MoveDrag::new(Vec3::X, Vec3::Z, 0.003);
        let target = drag.advance(Pos2::ZERO, Pos2::new(3.0, 2.0));
        assert_eq!(drag.advance(Pos2::ZERO, Pos2::new(f32::NAN, 10.0)), target);
        assert_eq!(
            drag.advance(Pos2::new(1.0, f32::INFINITY), Pos2::ZERO),
            target
        );
    }

    struct NativeMoveFrames {
        ctx: egui::Context,
        document: Document,
        settings: BrushSettings,
        gesture: Option<(MoveStroke, MoveDrag, Pos2)>,
        time: f64,
        accepted_moves: usize,
        last_pointer: Option<(bool, bool, bool, Pos2)>,
        recover_same_frame: bool,
    }

    impl NativeMoveFrames {
        fn new() -> Self {
            let settings = BrushSettings {
                kind: BrushKind::Move,
                radius: 0.037,
                strength: 0.79,
                symmetry_x: true,
                backface_culling: true,
                culling_axis: Some(Vec3::Z),
                adaptive_detail: true,
                ..BrushSettings::default()
            };
            Self {
                ctx: egui::Context::default(),
                document: Document::new(Mesh::sphere(6)),
                settings,
                gesture: None,
                time: 0.0,
                accepted_moves: 0,
                last_pointer: None,
                recover_same_frame: true,
            }
        }

        fn frame(&mut self, events: Vec<Event>, center: Vec3) {
            self.time += 1.0 / 60.0;
            let ctx = self.ctx.clone();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(self.time),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let rect =
                            Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(700.0, 500.0));
                        let response = ui.allocate_rect(rect, Sense::drag());
                        let (pos, pressed, down, released) = ctx.input(|input| {
                            (
                                input.pointer.interact_pos(),
                                input.pointer.primary_pressed(),
                                input.pointer.primary_down(),
                                input.pointer.primary_released(),
                            )
                        });
                        let Some(pos) = pos else { return };
                        self.last_pointer = Some((pressed, down, released, pos));
                        let same_frame = if self.recover_same_frame && pressed && released && !down
                        {
                            ctx.input(|input| same_frame_move_path(&input.events, rect))
                        } else {
                            None
                        };
                        let press_hovered = same_frame.as_ref().map_or_else(
                            || response.hovered(),
                            |path| ctx.layer_id_at(path.press) == Some(response.layer_id),
                        );
                        let pos = same_frame.as_ref().map_or(pos, |path| path.press);
                        if pressed && press_hovered {
                            self.document.begin_stroke();
                            let stroke = self
                                .document
                                .require_active_mesh()
                                .unwrap()
                                .prepare_move_stroke(center, center, &self.settings)
                                .unwrap();
                            self.gesture =
                                Some((stroke, MoveDrag::new(Vec3::X, Vec3::Y, 0.0004), pos));
                            if let Some(path) = same_frame.as_ref() {
                                for &sample in &path.samples {
                                    self.sample_move(sample);
                                }
                            }
                        } else if (down || released) && self.gesture.is_some() {
                            self.sample_move(pos);
                        }
                        if released && self.gesture.take().is_some() {
                            self.document.end_stroke();
                        }
                    });
                },
            );
        }

        fn sample_move(&mut self, pos: Pos2) {
            let (stroke, drag, previous) = self.gesture.as_mut().unwrap();
            let total = drag.advance(*previous, pos);
            let result = stroke.apply(
                self.document.require_active_mesh_mut().unwrap(),
                total,
                &self.settings,
            );
            self.accepted_moves += usize::from(result.changed_vertices > 0);
            *previous = pos;
            self.document
                .require_active_mesh()
                .unwrap()
                .validate()
                .unwrap();
        }
    }

    fn pointer(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn front_off_axis_point(mesh: &Mesh) -> Vec3 {
        let target = Vec3::new(0.25, 0.0, 0.968_245_8);
        *mesh
            .positions
            .iter()
            .min_by(|a, b| {
                a.distance_squared(target)
                    .total_cmp(&b.distance_squared(target))
            })
            .unwrap()
    }

    #[test]
    fn separate_native_egui_frames_move_fresh_sphere_and_undo_without_invalid_faces() {
        let mut host = NativeMoveFrames::new();
        let center = front_off_axis_point(host.document.require_active_mesh().unwrap());
        let before = host.document.to_json().unwrap();
        let original = host.document.require_active_mesh().unwrap().snapshot();
        let start = Pos2::new(450.0, 350.0);
        host.frame(vec![Event::PointerMoved(start)], center);
        host.frame(vec![pointer(start, true)], center);
        assert!(host.gesture.is_some(), "native press must start Move");
        for step in 1..=8 {
            let next = start + Vec2::new(step as f32 * 5.0, -step as f32 * 2.0);
            host.frame(vec![Event::PointerMoved(next)], center);
            host.document
                .require_active_mesh()
                .unwrap()
                .validate()
                .unwrap();
        }
        let end = start + Vec2::new(40.0, -16.0);
        host.frame(vec![pointer(end, false)], center);
        assert!(host.gesture.is_none());
        assert!(host.accepted_moves >= 1, "Move must deform in a held frame");
        let after = host.document.to_json().unwrap();
        assert_ne!(after, before);
        assert_eq!(host.document.history_depths(), (1, 0));
        let mesh = host.document.require_active_mesh().unwrap();
        mesh.validate().unwrap();
        let changed: Vec<_> = original
            .positions
            .iter()
            .zip(&mesh.positions)
            .filter(|(before, after)| before.distance(**after) > 1e-7)
            .collect();
        assert!(
            changed.len() >= 8,
            "a meaningful Move footprint is required"
        );
        assert!(
            changed.iter().any(|(before, _)| before.x > 0.01)
                && changed.iter().any(|(before, _)| before.x < -0.01),
            "X symmetry must affect both sides"
        );
        for triangle in &mesh.triangles {
            let [a, b, c] = triangle.map(|index| mesh.positions[index as usize]);
            let cross = (b - a).cross(c - a);
            assert!(cross.length() > 1e-10, "Move created a degenerate triangle");
            assert!(
                cross.dot((a + b + c) / 3.0) > 0.0,
                "Move inverted a sphere face"
            );
        }
        assert!(host.document.undo());
        assert_eq!(host.document.to_json().unwrap(), before);
        assert!(host.document.redo());
        assert_eq!(host.document.to_json().unwrap(), after);
    }

    #[test]
    fn compressed_native_press_move_release_observes_no_held_move_frame() {
        let mut host = NativeMoveFrames::new();
        host.recover_same_frame = false;
        let center = front_off_axis_point(host.document.require_active_mesh().unwrap());
        let before = host.document.to_json().unwrap();
        let start = Pos2::new(450.0, 350.0);
        let end = start + Vec2::new(40.0, -16.0);
        host.frame(
            vec![
                Event::PointerMoved(start),
                pointer(start, true),
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            center,
        );
        let (_, down, released, final_pos) = host.last_pointer.unwrap();
        assert!(!down && released && final_pos == end);
        assert_eq!(host.accepted_moves, 0);
        assert_eq!(host.document.to_json().unwrap(), before);
        assert_eq!(host.document.history_depths(), (0, 0));
    }

    #[test]
    fn same_frame_move_path_accepts_only_one_bounded_ordered_contact() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(700.0, 500.0));
        let start = Pos2::new(450.0, 350.0);
        let end = Pos2::new(490.0, 334.0);
        let valid = vec![
            pointer(start, true),
            Event::PointerMoved(end),
            pointer(end, false),
        ];
        let path = same_frame_move_path(&valid, rect).unwrap();
        assert_eq!(path.press, start);
        assert_eq!(path.samples, vec![end]);
        let cases = [
            vec![pointer(start, true), pointer(start, false)],
            vec![
                pointer(Pos2::new(99.0, 350.0), true),
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            vec![
                pointer(start, true),
                Event::PointerMoved(end),
                pointer(Pos2::new(801.0, 334.0), false),
            ],
            vec![
                pointer(start, true),
                Event::PointerGone,
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            vec![
                pointer(start, false),
                pointer(start, true),
                Event::PointerMoved(end),
            ],
            vec![
                pointer(start, true),
                pointer(start, true),
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            vec![
                pointer(start, true),
                pointer(start, false),
                Event::PointerMoved(end),
            ],
            vec![
                pointer(start, true),
                Event::PointerButton {
                    pos: start,
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerMoved(end),
                pointer(end, false),
            ],
        ];
        for events in cases {
            assert!(same_frame_move_path(&events, rect).is_none());
        }
        let mut overflowing = vec![pointer(start, true)];
        for i in 1..=65 {
            overflowing.push(Event::PointerMoved(start + Vec2::new(i as f32, 0.0)));
        }
        overflowing.push(pointer(end, false));
        assert!(same_frame_move_path(&overflowing, rect).is_none());
    }

    #[test]
    fn same_frame_native_egui_move_deforms_and_has_one_exact_undo_step() {
        let mut host = NativeMoveFrames::new();
        let center = front_off_axis_point(host.document.require_active_mesh().unwrap());
        let before = host.document.to_json().unwrap();
        let original = host.document.require_active_mesh().unwrap().snapshot();
        let start = Pos2::new(450.0, 350.0);
        let mut events = vec![pointer(start, true)];
        for step in 1..=8 {
            events.push(Event::PointerMoved(
                start + Vec2::new(step as f32 * 5.0, -step as f32 * 2.0),
            ));
        }
        events.push(pointer(start + Vec2::new(40.0, -16.0), false));
        host.frame(events, center);
        assert!(host.accepted_moves >= 1);
        assert!(host.gesture.is_none());
        let mesh = host.document.require_active_mesh().unwrap();
        let changed: Vec<_> = original
            .positions
            .iter()
            .zip(&mesh.positions)
            .filter(|(before, after)| before.distance(**after) > 1e-7)
            .collect();
        assert!(changed.len() >= 8);
        assert!(changed.iter().any(|(before, _)| before.x > 0.01));
        assert!(changed.iter().any(|(before, _)| before.x < -0.01));
        for triangle in &mesh.triangles {
            let [a, b, c] = triangle.map(|index| mesh.positions[index as usize]);
            let cross = (b - a).cross(c - a);
            assert!(cross.length() > 1e-10);
            assert!(cross.dot((a + b + c) / 3.0) > 0.0);
        }
        let after = host.document.to_json().unwrap();
        assert_ne!(after, before);
        assert_eq!(host.document.history_depths(), (1, 0));
        assert!(host.document.undo());
        assert_eq!(host.document.to_json().unwrap(), before);
        assert!(host.document.redo());
        assert_eq!(host.document.to_json().unwrap(), after);
    }

    #[test]
    fn stationary_and_rejected_same_frame_contacts_do_not_edit_or_spend_history() {
        let start = Pos2::new(450.0, 350.0);
        let end = Pos2::new(490.0, 334.0);
        let mut overflowing = vec![pointer(start, true)];
        for i in 1..=65 {
            overflowing.push(Event::PointerMoved(start + Vec2::new(i as f32, 0.0)));
        }
        overflowing.push(pointer(end, false));
        let cases = [
            vec![pointer(start, true), pointer(start, false)],
            vec![
                pointer(Pos2::new(99.0, 350.0), true),
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            vec![
                pointer(start, true),
                Event::PointerMoved(end),
                pointer(Pos2::new(801.0, 334.0), false),
            ],
            vec![
                pointer(start, true),
                Event::PointerGone,
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            vec![
                pointer(start, true),
                Event::PointerButton {
                    pos: start,
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerMoved(end),
                pointer(end, false),
            ],
            overflowing,
        ];
        for events in cases {
            let mut host = NativeMoveFrames::new();
            let center = front_off_axis_point(host.document.require_active_mesh().unwrap());
            let before = host.document.to_json().unwrap();
            host.frame(events, center);
            assert_eq!(host.accepted_moves, 0);
            assert_eq!(host.document.to_json().unwrap(), before);
            assert_eq!(host.document.history_depths(), (0, 0));
        }
    }

    #[test]
    fn repeated_small_radius_move_pulls_record_face_quality_without_folds() {
        let mut document = Document::new(Mesh::sphere(6));
        let before = document.to_json().unwrap();
        let settings = BrushSettings {
            kind: BrushKind::Move,
            radius: 0.037,
            strength: 0.79,
            symmetry_x: true,
            backface_culling: true,
            culling_axis: Some(Vec3::Z),
            adaptive_detail: true,
            ..BrushSettings::default()
        };
        eprintln!("pull,changed_vertices,min_quality,min_radial_cosine");
        for pull in 1..=12 {
            let original = document.require_active_mesh().unwrap().snapshot();
            let center = front_off_axis_point(document.require_active_mesh().unwrap());
            let mut stroke = document
                .require_active_mesh()
                .unwrap()
                .prepare_move_stroke(center, center, &settings)
                .unwrap();
            let mut drag = MoveDrag::new(Vec3::X, Vec3::Y, 0.0004);
            let mut previous = Pos2::new(450.0, 350.0);
            document.begin_stroke();
            for step in 1..=8 {
                let pos = Pos2::new(450.0 + step as f32 * 5.0, 350.0 - step as f32 * 2.0);
                let total = drag.advance(previous, pos);
                stroke.apply(
                    document.require_active_mesh_mut().unwrap(),
                    total,
                    &settings,
                );
                document.require_active_mesh().unwrap().validate().unwrap();
                previous = pos;
            }
            document.end_stroke();
            let mesh = document.require_active_mesh().unwrap();
            let changed = original
                .positions
                .iter()
                .zip(&mesh.positions)
                .filter(|(a, b)| a.distance(**b) > 1e-7)
                .count();
            let mut min_quality = f32::INFINITY;
            let mut min_radial_cosine = f32::INFINITY;
            for triangle in &mesh.triangles {
                let [a, b, c] = triangle.map(|index| mesh.positions[index as usize]);
                let ab = b - a;
                let bc = c - b;
                let ca = a - c;
                let cross = ab.cross(c - a);
                assert!(cross.length() > 1e-10);
                let quality = 2.0 * 3.0_f32.sqrt() * cross.length()
                    / (ab.length_squared() + bc.length_squared() + ca.length_squared());
                let radial_cosine = cross.normalize().dot(((a + b + c) / 3.0).normalize());
                min_quality = min_quality.min(quality);
                min_radial_cosine = min_radial_cosine.min(radial_cosine);
            }
            eprintln!("{pull},{changed},{min_quality:.8},{min_radial_cosine:.8}");
            assert!(min_radial_cosine > 0.0, "pull {pull} folded a face");
            assert_eq!(document.history_depths(), (pull, 0));
        }
        for _ in 0..12 {
            assert!(document.undo());
        }
        assert_eq!(document.to_json().unwrap(), before);
    }
}
