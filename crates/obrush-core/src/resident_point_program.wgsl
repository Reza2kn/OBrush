// Experimental point-only transaction. Fixed normals; no Source brush claim.
struct Parameters {center_radius:vec4<f32>,options:vec4<f32>,counts:vec4<u32>}
@group(0) @binding(0) var<storage,read> positions:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> normal_u:array<vec4<f32>>;
@group(0) @binding(2) var<storage,read_write> candidates:array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> failure:atomic<u32>;
@group(0) @binding(4) var<uniform> parameters:Parameters;
@group(0) @binding(5) var<storage,read_write> selected:array<u32>;
struct PickQuery {
 origin:vec4<f32>,direction:vec4<f32>,face_count:u32,max_distance:f32,pad0:u32,pad1:u32
}
struct PickState {distance_bits:atomic<u32>,face:atomic<u32>}
@group(0) @binding(6) var<uniform> pick_query:PickQuery;
@group(0) @binding(7) var<storage,read_write> pick_state:PickState;
struct DepthQuery {clip_to_local:mat4x4<f32>,pixel:vec2<u32>,extent:vec2<u32>}
@group(0) @binding(8) var depth_texture:texture_depth_2d;
@group(0) @binding(9) var<uniform> depth_query:DepthQuery;
@group(0) @binding(10) var<storage,read> picked_brush_direction:array<vec4<f32>>;
fn propose(i:u32,center:vec3<f32>) {
 selected[i]=0u;let p=positions[i];candidates[i]=p;let n=normal_u[i];
 let delta=p.xyz-center;
 let d=sqrt((delta.x*delta.x+delta.y*delta.y)+delta.z*delta.z)/parameters.center_radius.w;
 if(d>=1.){return;}selected[i]=1u;if(n.w==0.){return;}
 let length=sqrt((n.x*n.x+n.y*n.y)+n.z*n.z);
 if(length==0.){atomicMin(&failure,i);return;}
 let q=d*d;let amplitude=parameters.options.x*parameters.center_radius.w*0.1;
 let weight=((amplitude*(3.*q*q-4.*q*d+1.))/length)*n.w;
 let value=p.xyz+n.xyz*weight;
 if(!all(abs(value)<=vec3<f32>(1000000.))){atomicMin(&failure,i);return;}
 candidates[i]=vec4<f32>(value,p.w);
}
@compute @workgroup_size(256)
fn candidate(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=parameters.counts.x){return;}
 propose(i,parameters.center_radius.xyz);
}
// Explicit picked-direction ordinary Brush, accumulate-on and alphaNone.
// The same selected/failure/normal-refresh transaction handles this pass.
fn propose_brush(i:u32,center:vec3<f32>,direction:vec3<f32>) {
 selected[i]=0u;let p=positions[i];candidates[i]=p;
 if(all(direction==vec3<f32>(0.0))){return;}
 let offset=p.xyz-center;
 let d=length(offset)/parameters.center_radius.w;
 if(d>=1.0){return;}
 selected[i]=1u;let influence=normal_u[i].w;
 if(influence==0.0){return;}
 let q=d*d;
 let weight=(3.0*q*q-4.0*q*d+1.0)*
  parameters.options.x*parameters.center_radius.w*0.1*influence;
 let value=p.xyz+direction*weight;
 if(!(all(abs(value)<=vec3<f32>(1000000.0)))){
  atomicMin(&failure,i);return;
 }
 candidates[i]=vec4<f32>(value,p.w);
}
@compute @workgroup_size(256)
fn candidate_brush(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=parameters.counts.x){return;}
 propose_brush(i,parameters.center_radius.xyz,parameters.options.yzw);
}
@compute @workgroup_size(256)
fn candidate_brush_picked(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=parameters.counts.x){return;}
 let bits=atomicLoad(&pick_state.distance_bits);
 if(bits==0xffffffffu){selected[i]=0u;candidates[i]=positions[i];return;}
 let center=pick_query.origin.xyz+pick_query.direction.xyz*bitcast<f32>(bits);
 propose_brush(i,center,picked_brush_direction[0].xyz);
}
// The ray query and the point pass use the same resident buffers. A miss
// leaves every position unchanged; no per-dab pick result crosses to the CPU.
@compute @workgroup_size(256)
fn candidate_picked(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=parameters.counts.x){return;}
 let bits=atomicLoad(&pick_state.distance_bits);
 if(bits==0xffffffffu){selected[i]=0u;candidates[i]=positions[i];return;}
 let center=pick_query.origin.xyz+pick_query.direction.xyz*bitcast<f32>(bits);
 propose(i,center);
}
@compute @workgroup_size(256)
fn candidate_depth(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=parameters.counts.x){return;}
 let depth=textureLoad(depth_texture,vec2<i32>(depth_query.pixel),0);
 if(depth>=1.0){selected[i]=0u;candidates[i]=positions[i];return;}
 let xy=(vec2<f32>(depth_query.pixel)+vec2<f32>(0.5)) /
  vec2<f32>(depth_query.extent);
 let clip=vec4<f32>(xy.x*2.0-1.0,1.0-xy.y*2.0,depth,1.0);
 let local=depth_query.clip_to_local*clip;
 let center=local.xyz/local.w;
 if(!(all(abs(center)<=vec3<f32>(1000000.0)))){
  selected[i]=0u;candidates[i]=positions[i];atomicMin(&failure,i);return;
 }
 propose(i,center);
}
