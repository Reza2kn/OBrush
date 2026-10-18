//! Arc-length sampling shared by all dab-based tools.

use eframe::egui::Pos2;

pub(crate) struct StrokeSampler {
    previous: Option<Pos2>,
    spacing: f32,
    until_next: f32,
}

impl StrokeSampler {
    pub(crate) fn new(start: Pos2, spacing: f32) -> Self {
        let spacing = if spacing.is_finite() {
            spacing.max(1.0)
        } else {
            2.0
        };
        Self {
            previous: Some(start),
            spacing,
            until_next: spacing,
        }
    }

    /// Move tools consume displacement rather than dabs. Discard that path so
    /// switching to a dab tool does not paint the earlier Move drag again.
    pub(crate) fn skip_to(&mut self, end: Pos2) {
        self.previous = Some(end);
        self.until_next = self.spacing;
    }

    /// Leaving the canvas interrupts the path; re-entry starts a new segment.
    pub(crate) fn break_path(&mut self) {
        self.previous = None;
        self.until_next = self.spacing;
    }

    /// The caller places the initial dab; subsequent dabs stay evenly spaced
    /// across input-event boundaries instead of accumulating at each endpoint.
    pub(crate) fn sample_to(&mut self, end: Pos2) -> Vec<Pos2> {
        if !end.x.is_finite() || !end.y.is_finite() {
            return Vec::new();
        }
        let Some(previous) = self.previous else {
            self.skip_to(end);
            return Vec::new();
        };
        let delta = end - previous;
        let distance = delta.length();
        if !distance.is_finite() {
            self.skip_to(end);
            return Vec::new();
        }
        if distance <= f32::EPSILON {
            return Vec::new();
        }
        let direction = delta / distance;
        let mut travelled = self.until_next;
        let mut samples = Vec::new();
        while travelled <= distance {
            samples.push(previous + direction * travelled);
            travelled += self.spacing;
        }
        self.until_next = travelled - distance;
        self.previous = Some(end);
        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chunks(step: f32) -> Vec<Pos2> {
        let mut sampler = StrokeSampler::new(Pos2::ZERO, 5.76);
        let mut points = vec![Pos2::ZERO];
        let mut x = step;
        while x < 160.0 {
            points.extend(sampler.sample_to(Pos2::new(x, 0.0)));
            x += step;
        }
        points.extend(sampler.sample_to(Pos2::new(160.0, 0.0)));
        points
    }

    #[test]
    fn mouse_event_chunking_does_not_change_dabs_or_stroke_geometry() {
        let coarse = sample_chunks(160.0);
        for step in [1.0, 3.0, 10.0, 37.0] {
            let fine = sample_chunks(step);
            assert_eq!(coarse.len(), fine.len());
            for (a, b) in coarse.iter().zip(&fine) {
                assert!(a.distance(*b) < 0.0002);
            }
            let sculpt = |points: &[Pos2]| {
                let mut mesh = obrush_core::Mesh::sphere(4);
                let settings = obrush_core::BrushSettings {
                    radius: 0.18,
                    symmetry_x: false,
                    ..Default::default()
                };
                for point in points {
                    let x = (point.x - 80.0) * 0.003;
                    let center = glam::Vec3::new(x, 0.0, (1.0 - x * x).sqrt());
                    mesh.apply_brush(center, center, glam::Vec3::ZERO, &settings);
                }
                mesh.positions
            };
            for (a, b) in sculpt(&coarse).iter().zip(sculpt(&fine)) {
                assert!(a.distance(b) < 0.00001);
            }
        }
    }

    #[test]
    fn short_segments_and_corners_preserve_remaining_distance() {
        let mut sampler = StrokeSampler::new(Pos2::ZERO, 5.0);
        assert!(sampler.sample_to(Pos2::new(3.0, 0.0)).is_empty());
        assert_eq!(
            sampler.sample_to(Pos2::new(3.0, 4.0)),
            vec![Pos2::new(3.0, 2.0)]
        );
        assert_eq!(
            sampler.sample_to(Pos2::new(3.0, 7.0)),
            vec![Pos2::new(3.0, 7.0)]
        );
        assert!(sampler.sample_to(Pos2::new(f32::NAN, 0.0)).is_empty());
        assert!(sampler.sample_to(Pos2::new(3.0, 7.0)).is_empty());
    }

    #[test]
    fn skipped_moves_and_canvas_reentry_do_not_backfill_dabs() {
        let mut sampler = StrokeSampler::new(Pos2::ZERO, 5.0);
        sampler.skip_to(Pos2::new(80.0, 0.0));
        assert_eq!(
            sampler.sample_to(Pos2::new(86.0, 0.0)),
            vec![Pos2::new(85.0, 0.0)]
        );
        sampler.break_path();
        assert!(sampler.sample_to(Pos2::new(100.0, 0.0)).is_empty());
        assert_eq!(
            sampler.sample_to(Pos2::new(105.0, 0.0)),
            vec![Pos2::new(105.0, 0.0)]
        );
        assert!(sampler.sample_to(Pos2::new(f32::MAX, f32::MAX)).is_empty());
    }
}
