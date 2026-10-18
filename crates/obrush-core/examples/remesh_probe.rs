//! CLI evidence generator; use a new output directory to retain failed checks.
use obrush_core::{Document, Mesh, RemeshSettings, Vec3};
use serde_json::json;
use std::{fs, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let directory = PathBuf::from(args.get(1).ok_or("output directory required")?);
    fs::create_dir_all(&directory)?;
    let mut fixtures = vec![
        ("sphere-4".to_string(), Mesh::sphere(4)),
        ("sharp-box".to_string(), Mesh::cube()),
    ];
    for (name, separation) in [("overlapping-boxes", 1.0), ("face-touching-boxes", 2.0)] {
        let mut first = Mesh::cube();
        let mut second = Mesh::cube();
        for p in &mut first.positions {
            *p -= Vec3::X * separation * 0.5;
        }
        for p in &mut second.positions {
            *p += Vec3::X * separation * 0.5;
        }
        let offset = first.positions.len() as u32;
        first.positions.extend(&second.positions);
        first.normals.extend(&second.normals);
        first.masks.extend(&second.masks);
        first.colors.extend(&second.colors);
        first
            .triangles
            .extend(second.triangles.iter().map(|ids| ids.map(|v| v + offset)));
        fixtures.push((name.to_string(), first));
    }
    if let Some(input) = args.get(2) {
        fixtures.push((
            "user-sculpt".to_string(),
            Document::from_json(&fs::read_to_string(input)?)?
                .mesh
                .ok_or("Remesh probe requires a nonempty active mesh")?,
        ));
    }
    let mut results = Vec::new();
    for (name, source) in fixtures {
        fs::write(
            directory.join(format!("{name}-source.obrush")),
            Document::new(source.clone()).to_json()?,
        )?;
        for resolution in [32, 64, 96] {
            let settings = RemeshSettings {
                resolution,
                ..RemeshSettings::default()
            };
            let start = Instant::now();
            match source.remeshed(&settings) {
                Ok((mesh, report)) => {
                    let elapsed = start.elapsed().as_secs_f64();
                    let output = directory.join(format!("{name}-{resolution}.obrush"));
                    fs::write(&output, Document::new(mesh.clone()).to_json()?)?;
                    let mut edge_faces =
                        std::collections::HashMap::<(u32, u32), Vec<glam::DVec3>>::new();
                    let mut min_quality: f64 = 1.0;
                    for &[a, b, c] in &mesh.triangles {
                        let [pa, pb, pc] = [a, b, c].map(|i| mesh.positions[i as usize].as_dvec3());
                        let cross = (pb - pa).cross(pc - pa);
                        min_quality = min_quality.min(
                            cross.length()
                                / (pb - pa)
                                    .length_squared()
                                    .max((pc - pb).length_squared())
                                    .max((pa - pc).length_squared()),
                        );
                        for (a, b) in [(a, b), (b, c), (c, a)] {
                            edge_faces
                                .entry((a.min(b), a.max(b)))
                                .or_default()
                                .push(cross.normalize());
                        }
                    }
                    let mut max_fold: f64 = 0.0;
                    let mut folds_over_90 = 0;
                    let mut folds_over_90_length = 0.0;
                    let mut fold_excess_radians_length = 0.0;
                    for (&(a, b), normals) in &edge_faces {
                        if normals.len() == 2 {
                            let angle = normals[0]
                                .dot(normals[1])
                                .clamp(-1.0, 1.0)
                                .acos()
                                .to_degrees();
                            max_fold = max_fold.max(angle);
                            folds_over_90 += usize::from(angle > 90.0);
                            if angle > 90.0 {
                                let length = mesh.positions[a as usize]
                                    .as_dvec3()
                                    .distance(mesh.positions[b as usize].as_dvec3());
                                folds_over_90_length += length;
                                fold_excess_radians_length += (angle - 90.0).to_radians() * length;
                            }
                        }
                    }
                    results.push(json!({"fixture":name,"settings":settings,"seconds":elapsed,"report":report,"minimum_cross_over_max_edge_squared":min_quality,"maximum_adjacent_fold_degrees":max_fold,"adjacent_folds_over_90":folds_over_90,"folds_over_90_total_world_length":folds_over_90_length,"fold_excess_radians_world_length":fold_excess_radians_length,"output":output}));
                }
                Err(error) => {
                    results.push(json!({"fixture":name,"settings":settings,"seconds":start.elapsed().as_secs_f64(),"error":error}));
                }
            }
            fs::write(
                directory.join("results.json"),
                serde_json::to_string_pretty(&results)?,
            )?;
            eprintln!(
                "{name} resolution {resolution}: {}",
                results.last().unwrap()
            );
        }
    }
    Ok(())
}
