@group(0) @binding(0) var<storage,read_write> normal_u:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> face_rows:array<f32>;
@group(0) @binding(2) var<storage,read> vertex_offsets:array<u32>;
@group(0) @binding(3) var<storage,read> incident_faces:array<u32>;
@group(0) @binding(4) var<storage,read> selected:array<u32>;
@group(0) @binding(5) var<storage,read_write> failure:atomic<u32>;
@group(0) @binding(6) var<storage,read_write> visited:array<u32>;
@compute @workgroup_size(256)
fn vertex_commit(@builtin(global_invocation_id) id:vec3<u32>){
 let v=id.x;if(v>=arrayLength(&selected)||selected[v]==0u||atomicLoad(&failure)!=0xffffffffu){return;}
 normal_u[v]=vec4<f32>(selected_mean(v),normal_u[v].w);visited[v]=1u;
}
