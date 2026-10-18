struct Counter {
    value: atomic<u32>,
};

@group(0) @binding(0) var<storage, read> live: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> checkpoint: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> counter: Counter;
@group(0) @binding(3) var<storage, read_write> packed: array<u32>;

fn changed(index: u32) -> bool {
    return any(bitcast<vec4<u32>>(live[index]) != bitcast<vec4<u32>>(checkpoint[index]));
}

@compute @workgroup_size(256)
fn count(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index < arrayLength(&live) && changed(index)) {
        atomicAdd(&counter.value, 1u);
    }
}

@compute @workgroup_size(256)
fn collect(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= arrayLength(&live) || !changed(index)) {
        return;
    }
    let slot = atomicAdd(&counter.value, 1u);
    let offset = slot * 5u;
    if (offset + 4u >= arrayLength(&packed)) {
        return;
    }
    let words = bitcast<vec4<u32>>(live[index]);
    packed[offset] = index;
    packed[offset + 1u] = words.x;
    packed[offset + 2u] = words.y;
    packed[offset + 3u] = words.z;
    packed[offset + 4u] = words.w;
}
