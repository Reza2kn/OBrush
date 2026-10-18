@group(0) @binding(0) var<storage,read> position_rows:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> face_headers:array<u32>;
@group(0) @binding(2) var<storage,read> original_indices:array<u32>;
@group(0) @binding(3) var<storage,read> selected:array<u32>;
@group(0) @binding(4) var<storage,read_write> face_work:array<f32>;
@group(0) @binding(5) var<storage,read_write> failure:atomic<u32>;
@compute @workgroup_size(256)
fn face_candidate(@builtin(global_invocation_id) id:vec3<u32>){
 let f=id.x;if(f>=arrayLength(&face_headers)||atomicLoad(&failure)!=0xffffffffu){return;}
 if(!face_selected(f)){return;}let normal=raw_face(f);
 if(!all(abs(normal)<=vec3<f32>(3.402823e38))){atomicMin(&failure,0x40000000u|f);return;}
 // This physical f32 storage boundary precedes every incident-ring sum.
 let at=f*3u;face_work[at]=normal.x;face_work[at+1u]=normal.y;face_work[at+2u]=normal.z;
}
