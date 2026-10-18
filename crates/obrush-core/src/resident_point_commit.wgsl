// All candidates complete before this dispatch; any failure blocks every write.
@group(0) @binding(0) var<storage,read_write> committed:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> candidates:array<vec4<f32>>;
@group(0) @binding(2) var<storage,read_write> failure:atomic<u32>;
@compute @workgroup_size(256)
fn commit(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=arrayLength(&committed)||atomicLoad(&failure)!=0xffffffffu){return;}
 committed[i]=candidates[i];
}
