@group(0) @binding(0) var<storage,read> face_rows:array<f32>;
@group(0) @binding(1) var<storage,read> vertex_offsets:array<u32>;
@group(0) @binding(2) var<storage,read> incident_faces:array<u32>;
@group(0) @binding(3) var<storage,read> selected:array<u32>;
@group(0) @binding(4) var<storage,read_write> failure:atomic<u32>;
@compute @workgroup_size(256)
fn mean_preflight(@builtin(global_invocation_id) id:vec3<u32>){
 let v=id.x;if(v>=arrayLength(&selected)||selected[v]==0u||atomicLoad(&failure)!=0xffffffffu){return;}
 if(!all(abs(selected_mean(v))<=vec3<f32>(3.402823e38))){atomicMin(&failure,0x80000000u|v);}
}
