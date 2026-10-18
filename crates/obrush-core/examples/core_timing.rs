//! Run with `cargo run -p obrush-core --release --example core_timing`.
//! These are CPU operation costs, not application frame rates or GPU timings.
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use std::hint::black_box;
use std::time::Instant;

const SAMPLES: usize = 128;

fn report(label: &str, mut elapsed: Vec<f64>, mean_touched: Option<f64>) {
    elapsed.sort_by(f64::total_cmp);
    let median = elapsed[elapsed.len() / 2];
    let p95 = elapsed[(elapsed.len() * 95 / 100).min(elapsed.len() - 1)];
    let mean = elapsed.iter().sum::<f64>() / elapsed.len() as f64;
    print!("{label:>12}: median {median:7.3} ms; p95 {p95:7.3} ms; mean {mean:7.3} ms");
    if let Some(touched) = mean_touched {
        print!("; mean touched vertices {touched:.1}");
    }
    println!();
}

fn main() {
    println!(
        "OBrush CPU core timing, {SAMPLES} samples per operation, {} build",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    println!("Single-threaded wall-clock timings; no renderer, event loop, or frame-rate claim.");
    for level in [5, 6] {
        let original = Mesh::sphere(level);
        println!(
            "\nsphere{level}: {} vertices / {} triangles",
            original.positions.len(),
            original.triangles.len()
        );
        let rays: Vec<_> = (0..SAMPLES)
            .map(|sample| {
                let phase = sample as f32 / (SAMPLES - 1) as f32;
                Ray {
                    origin: Vec3::new(
                        -0.4 + 0.8 * phase,
                        0.12 * (phase * std::f32::consts::TAU).sin(),
                        3.0,
                    ),
                    direction: -Vec3::Z,
                }
            })
            .collect();
        let hits: Vec<_> = rays
            .iter()
            .map(|&ray| original.raycast(ray).unwrap())
            .collect();
        let mut times = Vec::with_capacity(SAMPLES);
        for &ray in &rays {
            let start = Instant::now();
            black_box(original.raycast(black_box(ray)));
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        report("raycast", times, None);

        for kind in [
            BrushKind::Draw,
            BrushKind::Smooth,
            BrushKind::Mask,
            BrushKind::Paint,
        ] {
            let mut mesh = original.clone();
            let mut settings = BrushSettings {
                kind,
                radius: 0.2,
                strength: 0.5,
                symmetry_x: true,
                ..BrushSettings::default()
            };
            // Warm the adjacency cache outside measured smoothing samples.
            mesh.apply_brush(hits[0].position, hits[0].normal, Vec3::ZERO, &settings);
            let mut times = Vec::with_capacity(SAMPLES);
            let mut touched = 0;
            for (sample, hit) in hits.iter().enumerate() {
                settings.invert = sample % 2 != 0; // Avoid cumulative height or mask saturation.
                let start = Instant::now();
                touched += black_box(mesh.apply_brush(
                    black_box(hit.position),
                    black_box(hit.normal),
                    Vec3::ZERO,
                    black_box(&settings),
                ));
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            report(kind.name(), times, Some(touched as f64 / SAMPLES as f64));
        }
    }
}
