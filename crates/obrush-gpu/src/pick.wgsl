// Full current resident triangle ray query. Two passes give a deterministic
// nearest positive f32 distance, then the lowest original face ID at that
// exact distance. No full-mesh position readback or face-ID render expansion.
@group(0) @binding(0) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> triangles: array<u32>;
struct Query {
    origin: vec4<f32>,
    direction: vec4<f32>,
    face_count: u32,
    max_distance: f32,
    pad0: u32,
    pad1: u32,
}
@group(0) @binding(2) var<uniform> query: Query;
struct PickState {
    distance_bits: atomic<u32>,
    face: atomic<u32>,
}
@group(0) @binding(3) var<storage, read_write> state: PickState;

fn hit_distance(face: u32) -> f32 {
    let base = face * 3u;
    let a = positions[triangles[base]].xyz;
    let b = positions[triangles[base + 1u]].xyz;
    let c = positions[triangles[base + 2u]].xyz;
    let edge1 = b - a;
    let edge2 = c - a;
    let p = cross(query.direction.xyz, edge2);
    let det = dot(edge1, p);
    if (!(abs(det) > 0.00000001)) { return -1.0; }
    let inverse = 1.0 / det;
    let from_a = query.origin.xyz - a;
    let u = dot(from_a, p) * inverse;
    if (!(u >= 0.0 && u <= 1.0)) { return -1.0; }
    let q = cross(from_a, edge1);
    let v = dot(query.direction.xyz, q) * inverse;
    if (!(v >= 0.0 && u + v <= 1.0)) { return -1.0; }
    let t = dot(edge2, q) * inverse;
    if (!(t >= 0.0 && t <= query.max_distance)) { return -1.0; }
    return t;
}

@compute @workgroup_size(256)
fn nearest(@builtin(global_invocation_id) global: vec3<u32>) {
    let face = global.x;
    if (face >= query.face_count) { return; }
    let t = hit_distance(face);
    if (t >= 0.0) { atomicMin(&state.distance_bits, bitcast<u32>(t)); }
}

@compute @workgroup_size(256)
fn lowest_face(@builtin(global_invocation_id) global: vec3<u32>) {
    let face = global.x;
    if (face >= query.face_count) { return; }
    let shortest = atomicLoad(&state.distance_bits);
    if (shortest == 0xffffffffu) { return; }
    let t = hit_distance(face);
    if (t >= 0.0 && bitcast<u32>(t) == shortest) {
        atomicMin(&state.face, face);
    }
}
