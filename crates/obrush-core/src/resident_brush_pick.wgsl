// One invocation converts the resident triangle pick to the original-face
// inverse-distance picked normal used by the ordinary Brush branch. The
// original-face header encodes (first triangle << 1) | quad.
@group(0) @binding(0) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> normal_u: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> triangles: array<u32>;
@group(0) @binding(3) var<storage, read> headers: array<u32>;
struct Query {
 origin: vec4<f32>, direction: vec4<f32>,
 face_count: u32, max_distance: f32, pad0: u32, pad1: u32,
}
@group(0) @binding(4) var<uniform> query: Query;
struct PickState {distance_bits: atomic<u32>, face: atomic<u32>}
@group(0) @binding(5) var<storage, read_write> pick: PickState;
@group(0) @binding(6) var<storage, read_write> picked_direction: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read_write> failure: atomic<u32>;

@compute @workgroup_size(1)
fn brush_pick_direction(@builtin(global_invocation_id) id: vec3<u32>) {
 if (id.x != 0u) { return; }
 picked_direction[0] = vec4<f32>(0.0);
 let triangle = atomicLoad(&pick.face);
 let depth_bits = atomicLoad(&pick.distance_bits);
 if (triangle == 0xffffffffu || depth_bits == 0xffffffffu) { return; }
 if (triangle >= query.face_count || arrayLength(&headers) == 0u) {
  atomicMin(&failure, 0u); return;
 }
 // Greatest original-face first triangle no greater than the picked triangle.
 var low = 0u;
 var high = arrayLength(&headers);
 while (low + 1u < high) {
  let middle = low + (high - low) / 2u;
  if ((headers[middle] >> 1u) <= triangle) { low = middle; }
  else { high = middle; }
 }
 let header = headers[low];
 let first = header >> 1u;
 let corners = 3u + (header & 1u);
 if (triangle < first || triangle >= first + corners - 2u) {
  atomicMin(&failure, 0u); return;
 }
 let center = query.origin.xyz + query.direction.xyz * bitcast<f32>(depth_bits);
 var inverse = vec4<f32>(0.0);
 var ids = vec4<u32>(0u);
 ids.x = triangles[first * 3u];
 ids.y = triangles[first * 3u + 1u];
 ids.z = triangles[first * 3u + 2u];
 if (corners == 4u) { ids.w = triangles[(first + 1u) * 3u + 2u]; }
 var sum = 0.0;
 for (var corner = 0u; corner < corners; corner++) {
  let vertex = ids[corner];
  if (vertex >= arrayLength(&positions)) { atomicMin(&failure, 0u); return; }
  let distance = length(positions[vertex].xyz - center);
  if (!(distance > 0.0)) { atomicMin(&failure, vertex); return; }
  let value = 1.0 / distance;
  if (!(value <= 1000000000.0)) { atomicMin(&failure, vertex); return; }
  inverse[corner] = value;
  sum += value;
 }
 if (!(sum > 0.0)) { atomicMin(&failure, 0u); return; }
 var interpolated = vec3<f32>(0.0);
 for (var corner = 0u; corner < corners; corner++) {
  interpolated += normal_u[ids[corner]].xyz * (inverse[corner] / sum);
 }
 let magnitude = length(interpolated);
 if (!(magnitude > 0.0)) { return; }
 let direction = interpolated / magnitude;
 if (!(all(abs(direction) <= vec3<f32>(1.0)))) {
  atomicMin(&failure, 0u); return;
 }
 picked_direction[0] = vec4<f32>(direction, 1.0);
}
