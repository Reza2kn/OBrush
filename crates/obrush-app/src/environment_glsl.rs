//! Pinned MIT SculptGL color and IBL contracts, adapted to GLSL140/WebGL2.
//! Exact upstream identities and license are in assets/sculptgl/environment-provenance.json.
//! GPU execution, interpolation and reference pixels are validated separately.

pub(crate) const COLOR: &str = r#"
float displayToLinear(float x) { return x<0.04045?x/12.92:pow((x+0.055)/1.055,2.4); }
vec3 displayToLinear(vec3 x) { return vec3(displayToLinear(x.r),displayToLinear(x.g),displayToLinear(x.b)); }
float linearToDisplay(float x) { return x<0.0031308?x*12.92:1.055*pow(x,1.0/2.4)-0.055; }
vec3 linearToDisplay(vec3 x) { return vec3(linearToDisplay(x.r),linearToDisplay(x.g),linearToDisplay(x.b)); }
vec4 linearToRgbm(vec3 color) {
    vec3 scaled=color/5.0;
    float multiplier=ceil(clamp(max(max(scaled.r,scaled.g),max(scaled.b,1e-6)),0.0,1.0)*255.0)/255.0;
    return vec4(scaled/multiplier,multiplier);
}
vec3 rgbmToLinear(vec4 encoded) { return 5.0*encoded.rgb*encoded.a; }
"#;

pub(crate) const IBL: &str = r#"
uniform sampler2D u_environment;
uniform mat3 u_environment_rotation;
uniform vec3 u_environment_sh[9];
uniform vec2 u_environment_size;

vec3 decodeEnvironment(vec4 encoded) {
    float luminance=exp2((encoded.z*255.0+encoded.w-127.0)/2.0);
    vec3 xyz=vec3(encoded.x*luminance/encoded.y,luminance,luminance/encoded.y);
    return max(mat3(6.0013,-2.700,-1.7995,-1.332,3.1029,-5.7720,0.3007,-1.088,5.6268)*xyz,vec3(0.0));
}
vec2 environmentDirectionUv(vec3 direction) {
    vec3 oct=direction/dot(direction,sign(direction));
    if(oct.z<0.0) oct.xy=sign(direction.xy)*(1.0-abs(oct.yx));
    return oct.xy*0.5+0.5;
}
vec2 environmentLevelUv(float level,vec2 uv) {
    float width=u_environment_size.x/exp2(level);
    vec2 local=vec2(1.0)+uv*(width-2.0);
    local.y+=u_environment_size.y-width*2.0;
    return local/u_environment_size;
}
vec3 environmentRadiance(vec3 direction,float roughness) {
    float level=roughness*4.0;
    vec2 uv=environmentDirectionUv(direction);
    vec4 a=texture(u_environment,environmentLevelUv(floor(level),uv));
    vec4 b=texture(u_environment,environmentLevelUv(ceil(level),uv));
    return decodeEnvironment(mix(a,b,fract(level)));
}
vec3 environmentDiffuse(vec3 n) {
    float x=n.x,y=n.y,z=-n.z;
    return max(u_environment_sh[0]+u_environment_sh[1]*y+u_environment_sh[2]*z+u_environment_sh[3]*x
        +u_environment_sh[4]*y*x+u_environment_sh[5]*y*z+u_environment_sh[6]*(3.0*z*z-1.0)
        +u_environment_sh[7]*z*x+u_environment_sh[8]*(x*x-y*y),vec3(0.0));
}
vec3 environmentPbr(vec3 n,vec3 eye,vec3 displayColor,float roughness,float metallic,float exposure) {
    float r=max(0.0001,roughness),nv=clamp(dot(n,eye),0.0,1.0);
    vec3 reflection=normalize(2.0*nv*n-eye);
    float smoothness=1.0-r;
    reflection=mix(n,reflection,smoothness*(sqrt(smoothness)+r));
    vec3 prefiltered=environmentRadiance(u_environment_rotation*reflection,r);
    vec4 fitted=r*vec4(-1.0,-0.0275,-0.572,0.022)+vec4(1.0,0.0425,1.04,-0.04);
    float a004=min(fitted.x*fitted.x,exp2(-9.28*nv))*fitted.x+fitted.y;
    vec2 ab=vec2(-1.04,1.04)*a004+fitted.zw;
    vec3 linear=displayToLinear(displayColor),specular=mix(vec3(0.04),linear,metallic);
    return exposure*(linear*(1.0-metallic)*environmentDiffuse(u_environment_rotation*n)+prefiltered*(specular*ab.x+ab.y));
}
"#;

pub(crate) const FULLSCREEN_VERTEX: &str = r#"
out vec2 v_uv;
void main() {
    v_uv=vec2(float((gl_VertexID<<1)&2),float(gl_VertexID&2));
    gl_Position=vec4(v_uv*2.0-1.0,0.5,1.0);
}
"#;

pub(crate) const MERGE: &str = r#"
in vec2 v_uv;
uniform sampler2D u_opaque;
uniform sampler2D u_transparent;
uniform bool u_filmic;
out vec4 out_color;
void main() {
    vec4 transparent=texture(u_transparent,v_uv);
    vec3 color=rgbmToLinear(texture(u_opaque,v_uv))*(1.0-transparent.a)+transparent.rgb;
    if(u_filmic) {
        vec3 x=max(vec3(0.0),color-vec3(0.004));
        out_color=vec4((x*(6.2*x+0.5))/(x*(6.2*x+1.7)+0.06),1.0);
    } else out_color=vec4(linearToDisplay(color),1.0);
}
"#;

pub(crate) const BACKGROUND_VERTEX: &str = r#"
uniform vec2 u_background_extents;
out vec2 v_uv;
void main() {
    vec2 corners[4]=vec2[4](vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(1.0,1.0));
    vec2 corner=corners[gl_VertexID];
    v_uv=corner*0.5+0.5;
    gl_Position=vec4(corner*u_background_extents,1.0,1.0);
}
"#;

pub(crate) const BACKGROUND: &str = r#"
in vec2 v_uv;
uniform sampler2D u_background_image;
uniform int u_background_type;
uniform float u_background_blur;
out vec4 out_color;
void main() {
    vec3 color;
    if(u_background_type==0) color=displayToLinear(texture(u_background_image,v_uv).rgb);
    else {
        vec3 direction=normalize(u_environment_rotation*vec3(v_uv*2.0-1.0,-1.0));
        color=u_background_type==1?environmentRadiance(direction,u_background_blur*u_background_blur):environmentDiffuse(direction);
    }
    out_color=linearToRgbm(color);
}
"#;

// The pinned final display pass uses the FXAA luma/span contract after the
// display transfer and RGBA8 merge storage, rather than filtering linear HDR.
pub(crate) const PRESENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_final_texture;
uniform vec2 u_inverse_size;
uniform bool u_antialias;
out vec4 out_color;
void main() {
    vec3 center=texture(u_final_texture,v_uv).rgb;
    if(!u_antialias) {out_color=vec4(center,1.0);return;}
    const vec3 weights=vec3(0.299,0.587,0.114);
    float nw=dot(texture(u_final_texture,v_uv+vec2(-1.0,-1.0)*u_inverse_size).rgb,weights);
    float ne=dot(texture(u_final_texture,v_uv+vec2(1.0,-1.0)*u_inverse_size).rgb,weights);
    float sw=dot(texture(u_final_texture,v_uv+vec2(-1.0,1.0)*u_inverse_size).rgb,weights);
    float se=dot(texture(u_final_texture,v_uv+vec2(1.0,1.0)*u_inverse_size).rgb,weights);
    float middle=dot(center,weights);
    float low=min(middle,min(min(nw,ne),min(sw,se))),high=max(middle,max(max(nw,ne),max(sw,se)));
    vec2 direction=vec2(-((nw+ne)-(sw+se)),(nw+sw)-(ne+se));
    float reduction=max((nw+ne+sw+se)*(0.25/8.0),1.0/128.0);
    direction=clamp(direction/(min(abs(direction.x),abs(direction.y))+reduction),vec2(-8.0),vec2(8.0))*u_inverse_size;
    vec3 a=0.5*(texture(u_final_texture,v_uv+direction*(1.0/3.0-0.5)).rgb+texture(u_final_texture,v_uv+direction*(2.0/3.0-0.5)).rgb);
    vec3 b=0.5*a+0.25*(texture(u_final_texture,v_uv-direction*0.5).rgb+texture(u_final_texture,v_uv+direction*0.5).rgb);
    float estimate=dot(b,weights);
    out_color=vec4(estimate<low||estimate>high?a:b,1.0);
}
"#;
