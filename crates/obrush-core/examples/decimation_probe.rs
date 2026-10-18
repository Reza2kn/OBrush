//! CPU-only decimation evidence. Run in release mode; paths and parameters are
//! explicit and original user files are read without writing to them.
use obrush_core::{DecimateSettings, Mesh, Vec3};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

#[derive(Clone)]
struct Node {
    min: Vec3,
    max: Vec3,
    children: Option<[usize; 2]>,
    faces: Vec<usize>,
}

struct Bvh<'a> {
    mesh: &'a Mesh,
    nodes: Vec<Node>,
}

impl<'a> Bvh<'a> {
    fn new(mesh: &'a Mesh) -> Self {
        let mut bvh = Self {
            mesh,
            nodes: Vec::new(),
        };
        bvh.build((0..mesh.triangles.len()).collect());
        bvh
    }

    fn build(&mut self, mut faces: Vec<usize>) -> usize {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for &face in &faces {
            for vertex in self.mesh.triangles[face] {
                min = min.min(self.mesh.positions[vertex as usize]);
                max = max.max(self.mesh.positions[vertex as usize]);
            }
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            min,
            max,
            children: None,
            faces: Vec::new(),
        });
        if faces.len() <= 8 {
            self.nodes[index].faces = faces;
        } else {
            let extent = max - min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            faces.sort_unstable_by(|&a, &b| {
                let centroid = |face: usize| {
                    self.mesh.triangles[face]
                        .iter()
                        .map(|&vertex| self.mesh.positions[vertex as usize][axis])
                        .sum::<f32>()
                };
                centroid(a).total_cmp(&centroid(b)).then(a.cmp(&b))
            });
            let right = faces.split_off(faces.len() / 2);
            let children = [self.build(faces), self.build(right)];
            self.nodes[index].children = Some(children);
        }
        index
    }

    fn distance(&self, point: Vec3) -> f64 {
        let mut best = f64::INFINITY;
        let mut pending = vec![0];
        while let Some(index) = pending.pop() {
            let node = &self.nodes[index];
            let outside = (node.min - point).max(Vec3::ZERO) + (point - node.max).max(Vec3::ZERO);
            if outside.as_dvec3().length_squared() >= best {
                continue;
            }
            if let Some(children) = node.children {
                pending.extend(children);
            } else {
                for &face in &node.faces {
                    let triangle =
                        self.mesh.triangles[face].map(|v| self.mesh.positions[v as usize]);
                    best = best.min(point_triangle_squared(point, triangle));
                }
            }
        }
        best.sqrt()
    }
}

fn point_triangle_squared(point: Vec3, triangle: [Vec3; 3]) -> f64 {
    // Independent closest-point regions, evaluated in f64.
    let [a, b, c] = triangle.map(|position| position.as_dvec3());
    let p = point.as_dvec3();
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return ap.length_squared();
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return bp.length_squared();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (p - (a + ab * (d1 / (d1 - d3)))).length_squared();
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return cp.length_squared();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (p - (a + ac * (d2 / (d2 - d6)))).length_squared();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return (p - (b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))))).length_squared();
    }
    let denominator = 1.0 / (va + vb + vc);
    (p - (a + ab * (vb * denominator) + ac * (vc * denominator))).length_squared()
}

fn uv_sphere(longitudes: usize, latitudes: usize) -> Mesh {
    let mut obj = String::from("v 0 1 0\nv 0 -1 0\n");
    for latitude in 1..latitudes {
        let theta = std::f64::consts::PI * latitude as f64 / latitudes as f64;
        for longitude in 0..longitudes {
            let phi = std::f64::consts::TAU * longitude as f64 / longitudes as f64;
            obj.push_str(&format!(
                "v {} {} {}\n",
                (theta.sin() * phi.cos()) as f32,
                theta.cos() as f32,
                (theta.sin() * phi.sin()) as f32
            ));
        }
    }
    let ring = |latitude: usize, longitude: usize| {
        3 + (latitude - 1) * longitudes + longitude % longitudes
    };
    for longitude in 0..longitudes {
        obj.push_str(&format!(
            "f 1 {} {}\n",
            ring(1, longitude + 1),
            ring(1, longitude)
        ));
        obj.push_str(&format!(
            "f 2 {} {}\n",
            ring(latitudes - 1, longitude),
            ring(latitudes - 1, longitude + 1)
        ));
        for latitude in 1..latitudes - 1 {
            let a = ring(latitude, longitude);
            let b = ring(latitude, longitude + 1);
            let c = ring(latitude + 1, longitude);
            let d = ring(latitude + 1, longitude + 1);
            obj.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }
    Mesh::from_obj(&obj).expect("independent UV sphere")
}

fn metrics(input: &Mesh, output: &Mesh) -> serde_json::Value {
    let signed_volume = |mesh: &Mesh| {
        mesh.triangles
            .iter()
            .map(|face| {
                let [a, b, c] = face.map(|vertex| mesh.positions[vertex as usize].as_dvec3());
                a.dot(b.cross(c)) / 6.0
            })
            .sum::<f64>()
    };
    let mut edges = BTreeMap::<[u32; 2], Vec<bool>>::new();
    let mut minimum_quality = f64::INFINITY;
    for face in &output.triangles {
        let [a, b, c] = face.map(|vertex| output.positions[vertex as usize].as_dvec3());
        minimum_quality = minimum_quality.min(
            (b - a).cross(c - a).length()
                / a.distance_squared(b)
                    .max(b.distance_squared(c))
                    .max(c.distance_squared(a)),
        );
        for [a, b] in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            edges.entry([a.min(b), a.max(b)]).or_default().push(a < b);
        }
    }
    let sample_distances = |source: &Mesh, target: &Mesh| {
        let bvh = Bvh::new(target);
        let mut distances = Vec::new();
        for sample in 0..512 {
            let face = source.triangles[sample * source.triangles.len() / 512];
            let [a, b, c] = face.map(|vertex| source.positions[vertex as usize]);
            distances.push(bvh.distance((a + b + c) / 3.0));
            distances.push(bvh.distance(a));
        }
        json!({"samples": distances.len(), "max": distances.iter().copied().fold(0.0, f64::max), "rms": (distances.iter().map(|distance| distance * distance).sum::<f64>() / distances.len() as f64).sqrt()})
    };
    json!({
        "signed_volume_input":signed_volume(input),
        "signed_volume_output":signed_volume(output),
        "relative_volume_change":signed_volume(output)/signed_volume(input)-1.0,
        "nonmanifold_edges":edges.values().filter(|faces|faces.len()>2).count(),
        "boundary_edges":edges.values().filter(|faces|faces.len()==1).count(),
        "inconsistent_winding_edges":edges.values().filter(|faces|faces.len()==2&&faces[0]==faces[1]).count(),
        "minimum_cross_over_longest_edge_squared":minimum_quality,
        "sampled_input_to_output_distance":sample_distances(input,output),
        "sampled_output_to_input_distance":sample_distances(output,input),
        "sampling_limitation":"1024 deterministic face-centroid/vertex samples each way, not exact Hausdorff or global intersection proof"
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".artifacts/decimation-final".into());
    fs::create_dir_all(&directory)?;
    let user: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        "/Users/Ajab/Downloads/Untitled_sculpture (1).obrush",
    )?)?;
    let cases = [
        ("icosphere-10k", Mesh::sphere(5)),
        (
            "user-61580",
            serde_json::from_value::<Mesh>(user["mesh"].clone())?,
        ),
        ("uv-sphere-99802", uv_sphere(200, 500)),
    ];
    let mut records = Vec::new();
    for (name, mesh) in cases {
        fs::write(format!("{directory}/{name}-input.obj"), mesh.to_obj())?;
        fs::write(
            format!("{directory}/{name}-input.json"),
            serde_json::to_string(&mesh)?,
        )?;
        let settings = DecimateSettings::default();
        let started = Instant::now();
        let (output, report) = mesh.decimated(&settings)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        fs::write(format!("{directory}/{name}-output.obj"), output.to_obj())?;
        fs::write(
            format!("{directory}/{name}-output.json"),
            serde_json::to_string(&output)?,
        )?;
        let record = json!({"fixture":name,"settings":settings,"report":report,"elapsed_ms":elapsed_ms,"metrics":metrics(&mesh,&output)});
        println!("{}", serde_json::to_string(&record)?);
        records.push(record);
    }
    fs::write(
        format!("{directory}/results.json"),
        serde_json::to_string_pretty(
            &json!({"timing":"Single release-mode native CPU runs, operation includes validation, topology setup and recomputed normals; fixture creation and metrics excluded. Not browser latency.","cases":records}),
        )?,
    )?;
    Ok(())
}
