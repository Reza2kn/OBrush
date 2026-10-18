@group(0) @binding(0) var<storage,read_write> face_cache:array<f32>;
@group(0) @binding(1) var<storage,read> face_work:array<f32>;
@group(0) @binding(2) var<storage,read> face_headers:array<u32>;
@group(0) @binding(3) var<storage,read> original_indices:array<u32>;
@group(0) @binding(4) var<storage,read> selected:array<u32>;
@group(0) @binding(5) var<storage,read_write> failure:atomic<u32>;
@compute @workgroup_size(256)
fn face_commit(@builtin(global_invocation_id) id:vec3<u32>){
 let f=id.x;if(f>=arrayLength(&face_headers)||atomicLoad(&failure)!=0xffffffffu){return;}
 if(!face_selected(f)){return;}let at=f*3u;
 face_cache[at]=face_work[at];face_cache[at+1u]=face_work[at+1u];face_cache[at+2u]=face_work[at+2u];
}
