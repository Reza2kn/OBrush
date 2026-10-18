@group(0) @binding(0) var<storage,read> positions:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> normal_u:array<vec4<f32>>;
@group(0) @binding(2) var<storage,read> colors:array<vec4<f32>>;
struct View {
 aspect:vec4<f32>,
 local_to_clip:mat4x4<f32>,
 mode:vec4<u32>,
 normal_to_world:mat4x4<f32>,
}
@group(0) @binding(3) var<uniform> view:View;
struct Vertex {@builtin(position) position:vec4<f32>,@location(0) normal:vec3<f32>,@location(1) color:vec3<f32>}
@vertex fn vs(@builtin(vertex_index) i:u32)->Vertex {
 let p=positions[i];var out:Vertex;
 // Fixed oblique orthographic diagnostic camera. No native/browser CPU camera drift.
 if(view.mode.x==0u){
  out.position=vec4<f32>((0.5*p.x-0.5*p.z)*view.aspect.x,0.70710677*p.y*view.aspect.y,0.5-0.125*(p.x+p.z),1.);
 }else{
  out.position=view.local_to_clip*vec4<f32>(p.xyz,1.);
 }
 out.normal=(view.normal_to_world*vec4<f32>(normal_u[i].xyz,0.)).xyz;
 out.color=colors[i].xyz;return out;
}
@fragment fn fs(input:Vertex)->@location(0)vec4<f32>{
 let length=sqrt(dot(input.normal,input.normal));var lighting=0.3;
 if(length>0.){lighting+=0.7*max(dot(input.normal/length,normalize(vec3<f32>(0.4,0.5,1.))),0.);}
 return vec4<f32>(input.color*lighting,1.);
}
