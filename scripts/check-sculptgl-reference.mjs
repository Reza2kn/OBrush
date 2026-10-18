// Reference-only, CLI headless Chrome/CDP. Does not attach to user browsers.
// Usage: node scripts/check-sculptgl-reference.mjs [mirror dir] [label] [--prepare-only]
// Read-only diagnostic instance capture lives in the separately hashed mirror
// HTML. All application operations use real control/input/file/download routes.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const mirror = path.resolve(process.argv[2] || '.tools/sculptgl-reference/host-mirror');
const label = process.argv[3] || 'reference';
const prepareOnly = process.argv.includes('--prepare-only');
const skipMixed = process.argv.includes('--skip-mixed');
const extractOnly = process.argv.includes('--extract-only');
const extractDiagnostic = process.argv.includes('--extract-diagnostic');
const extractQuadOnly = process.argv.includes('--extract-quad-only');
const extractMixedOnly = process.argv.includes('--extract-mixed-only');
const extractAffineOnly = process.argv.includes('--extract-affine-only');
const maskFilterOnly = process.argv.includes('--mask-filter-only');
const blockRemeshOnly = process.argv.includes('--block-remesh-only');
const uvSubdivisionOnly = process.argv.includes('--uv-subdivision-only');
const uvCoarseEditOnly = process.argv.includes('--uv-coarse-edit-only');
const uvTwoLevelOnly = process.argv.includes('--uv-two-level-only');
const uvThreeLevelOnly = process.argv.includes('--uv-three-level-only');
const reverseOnly = process.argv.includes('--reverse-only');
const reverseSeamOnly = process.argv.includes('--reverse-seam-only');
const transformOnly = process.argv.includes('--transform-only');
const gizmoOnly = process.argv.includes('--gizmo-only');
const gizmoPhysical = process.argv.includes('--gizmo-physical');
const startedAt = new Date().toISOString();
const out = path.resolve(`.artifacts/sculptgl-reference-runtime/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads, { recursive: true });
const hash = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition, message) => { if (!condition) throw Error(message); };
const manifest = JSON.parse(await readFile(path.join(mirror,'mirror-manifest.json'),'utf8'));
const source = await readFile(new URL(import.meta.url));
await writeFile(path.join(out,'harness.mjs'),source);
assert(manifest.original_bundle.sha256 === hash(await readFile(path.join(mirror,'sculptgl.js'))),'Reference kernel bundle hash changed');
assert(manifest.mirrored_html.sha256 === hash(await readFile(path.join(mirror,'index.html'))),'Reference mirror wrapper hash changed');

// Independent analytic fixtures. OBJ has per-corner UVs, nonuniform RGB and
// SculptGL #MAT bytes (roughness, metalness, UNMASKED weight). No user model used.
function sphereFixture() {
  const positions=[[0,1,0]], colors=[[.2,.3,.9]], materials=[[.2,.4,1]], faces=[], uv=[];
  const columns=24,rows=12;
  for(let row=1;row<rows;row++) {
    const phi=Math.PI*row/rows;
    for(let col=0;col<columns;col++) {
      const theta=2*Math.PI*col/columns,p=[Math.sin(phi)*Math.cos(theta),Math.cos(phi),Math.sin(phi)*Math.sin(theta)].map(Math.fround);
      positions.push(p);colors.push([.2+.5*(p[0]+1)/2,.15+.6*(p[1]+1)/2,.25+.5*(p[2]+1)/2]);
      materials.push([.1+.8*row/rows,.1+.7*col/columns,positions.length%37===0?0:1]);
    }
  }
  const bottom=positions.length;positions.push([0,-1,0]);colors.push([.8,.2,.4]);materials.push([.8,.3,1]);
  const id=(row,col)=>1+(row-1)*columns+(col%columns);
  for(let col=0;col<columns;col++)faces.push([0,id(1,col+1),id(1,col)]);
  for(let row=1;row<rows-1;row++)for(let col=0;col<columns;col++){
    const a=id(row,col),b=id(row,col+1),c=id(row+1,col+1),d=id(row+1,col);faces.push([a,b,c],[a,c,d]);
  }
  for(let col=0;col<columns;col++)faces.push([bottom,id(rows-1,col),id(rows-1,col+1)]);
  for(const face of faces)for(const i of face){const p=positions[i];uv.push([.5+Math.atan2(p[2],p[0])/(2*Math.PI),Math.acos(p[1])/Math.PI]);}
  let text='o Independent_reference_sphere\n';
  positions.forEach(p=>text+=`v ${p.join(' ')}\n`);
  const hex=x=>Math.round(Math.max(0,Math.min(1,x))*255).toString(16).padStart(2,'0');
  for(let i=0;i<positions.length;i+=64)text+='#MRGB '+colors.slice(i,i+64).map(c=>'ff'+c.map(hex).join('')).join('')+'\n';
  for(let i=0;i<positions.length;i+=46)text+='#MAT '+materials.slice(i,i+46).map(m=>m.map(hex).join('')).join('')+'\n';
  uv.forEach(p=>text+=`vt ${p.join(' ')}\n`);
  faces.forEach((f,i)=>text+='f '+f.map((v,j)=>`${v+1}/${i*3+j+1}`).join(' ')+'\n');
  return {positions,colors,materials,faces,uv,text};
}
function cubeFixture(){return {positions:[[-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],[-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1]],faces:[[0,3,2,1],[4,5,6,7],[0,1,5,4],[3,7,6,2],[0,4,7,3],[1,2,6,5]],colors:[[17,33,201],[91,22,43],[111,139,203],[244,187,37],[53,101,179],[233,45,83],[199,217,29],[11,197,241]]};}
const cube=cubeFixture();
const plyHeader=format=>`ply\nformat ${format} 1.0\ncomment independently authored OBrush reference fixture\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face 6\nproperty list uchar uint vertex_indices\nend_header\n`;
let plyAscii=plyHeader('ascii');cube.positions.forEach((p,i)=>plyAscii+=p.join(' ')+' '+cube.colors[i].join(' ')+'\n');cube.faces.forEach(f=>plyAscii+=`4 ${f.join(' ')}\n`);
const head=Buffer.from(plyHeader('binary_little_endian')),plyBody=Buffer.alloc(8*15+6*17);let offset=0;
cube.positions.forEach((p,i)=>{p.forEach(x=>{plyBody.writeFloatLE(x,offset);offset+=4;});cube.colors[i].forEach(x=>plyBody[offset++]=x);});
cube.faces.forEach(f=>{plyBody[offset++]=4;f.forEach(i=>{plyBody.writeUInt32LE(i,offset);offset+=4;});});
const cubeTriangles=cube.faces.flatMap(f=>[[f[0],f[1],f[2]],[f[0],f[2],f[3]]]);
function normal(face){const [a,b,c]=face.map(i=>cube.positions[i]),u=b.map((x,i)=>x-a[i]),v=c.map((x,i)=>x-a[i]);const n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]],len=Math.hypot(...n);return n.map(x=>x/len);}
let stlAscii='solid independent_fixture\n';const stlBinary=Buffer.alloc(84+50*cubeTriangles.length);stlBinary.write('independently authored color STL');stlBinary.writeUInt32LE(cubeTriangles.length,80);
cubeTriangles.forEach((face,i)=>{const n=normal(face);stlAscii+=` facet normal ${n.join(' ')}\n  outer loop\n`;face.forEach(v=>stlAscii+=`   vertex ${cube.positions[v].join(' ')}\n`);stlAscii+='  endloop\n endfacet\n';let at=84+50*i;[...n,...face.flatMap(v=>cube.positions[v])].forEach(x=>{stlBinary.writeFloatLE(x,at);at+=4;});const rgb=[17,24,7];stlBinary.writeUInt16LE(0x8000|(rgb[0]<<10)|(rgb[1]<<5)|rgb[2],at);});stlAscii+='endsolid independent_fixture\n';
const independent=sphereFixture();
const fixtures=[];
async function fixture(name,bytes,expected){const file=path.join(out,name);await writeFile(file,bytes);const item={name,path:file,bytes:Buffer.byteLength(bytes),sha256:hash(bytes),expected};fixtures.push(item);return item;}
const sphere=await fixture('independent-seamed-sphere.obj',independent.text,{vertices:266,faces:528,triangles:528,has_uv:true,has_nonuniform_rgb:true,has_nonuniform_roughness_metalness:true,unmasked_weight_zero_vertices:7});
const formats=[await fixture('independent-quad-cube-ascii.ply',plyAscii,{vertices:8,faces:6,triangles:12}),await fixture('independent-quad-cube-binary.ply',Buffer.concat([head,plyBody]),{vertices:8,faces:6,triangles:12}),await fixture('independent-cube-ascii.stl',stlAscii,{vertices:8,faces:12,triangles:12}),await fixture('independent-cube-colored.stl',stlBinary,{vertices:8,faces:12,triangles:12})];
const hexByte=x=>Math.round(Math.max(0,Math.min(1,x))*255).toString(16).padStart(2,'0');
let quadText='o Independent_UV_quad_cube\n';cube.positions.forEach((p,i)=>quadText+='v '+p.join(' ')+' '+cube.colors[i].map(x=>x/255).join(' ')+'\n');quadText+='#MAT '+cube.positions.map((_,i)=>[.1+.1*i,.8-.1*i,i%3===0?0:1].map(hexByte).join('')).join('')+'\n';for(let face=0;face<6;face++)for(const uv of [[0,0],[1,0],[1,1],[0,1]])quadText+='vt '+uv.join(' ')+'\n';cube.faces.forEach((f,i)=>quadText+='f '+f.map((v,j)=>`${v+1}/${i*4+j+1}`).join(' ')+'\n');
const quadObj=await fixture('independent-seamed-quad-cube.obj',quadText,{vertices:8,faces:6,triangles:12,has_uv:true});
const reverseSource=JSON.parse(await readFile('crates/obrush-core/tests/fixtures/independent-reverse-quad.json','utf8'));
const reverseText='o Independent_reverse_quad\n'+reverseSource.positions.map(p=>'v '+p.join(' ')+'\n').join('')+reverseSource.faces.map(f=>'f '+f.map(v=>v+1).join(' ')+'\n').join('');
const reverseQuad=await fixture('independent-reverse-quad.obj',reverseText,{vertices:9,faces:4,triangles:8});
let reverseGridText='o Independent_reverse_two_step_grid\n';
for(let y=0;y<5;y++)for(let x=0;x<5;x++)reverseGridText+=`v ${x} ${y} ${((x*x+3*y*y+x*y)%7)/16}\n`;
for(let y=0;y<4;y++)for(let x=0;x<4;x++){const a=y*5+x+1;reverseGridText+=`f ${a} ${a+1} ${a+6} ${a+5}\n`;}
const reverseGrid=await fixture('independent-reverse-two-step-grid.obj',reverseGridText,{vertices:25,faces:16,triangles:32});
let reverseUvGridText='o Independent_reverse_two_step_uv_grid\n';
for(let y=0;y<5;y++)for(let x=0;x<5;x++)reverseUvGridText+=`v ${x} ${y} ${((x*x+3*y*y+x*y)%7)/16}\n`;
for(let y=0;y<5;y++)for(let x=0;x<5;x++)reverseUvGridText+=`vt ${x/4} ${y/4}\n`;
for(let y=0;y<4;y++)for(let x=0;x<4;x++){const a=y*5+x+1;reverseUvGridText+=`f ${a}/${a} ${a+1}/${a+1} ${a+6}/${a+6} ${a+5}/${a+5}\n`;}
const reverseUvGrid=await fixture('independent-reverse-two-step-uv-grid.obj',reverseUvGridText,{vertices:25,faces:16,triangles:32});
let reverseSeamGridText='o Independent_reverse_two_step_seam_grid\n';
for(let y=0;y<5;y++)for(let x=0;x<5;x++)reverseSeamGridText+=`v ${x} ${y} ${((x*x+3*y*y+x*y)%7)/16}\n`;
for(let y=0;y<5;y++)for(let x=0;x<5;x++)reverseSeamGridText+=`vt ${x/4} ${y/4}\n`;
for(let y=0;y<5;y++)reverseSeamGridText+=`vt 0.5 ${y/4}\n`;
const seamUvId=(vx,vy,faceX)=>vx===2&&faceX>=2?26+vy:vy*5+vx+1;
for(let y=0;y<4;y++)for(let x=0;x<4;x++){const a=y*5+x+1;reverseSeamGridText+=`f ${a}/${seamUvId(x,y,x)} ${a+1}/${seamUvId(x+1,y,x)} ${a+6}/${seamUvId(x+1,y+1,x)} ${a+5}/${seamUvId(x,y+1,x)}\n`;}
const reverseSeamGrid=await fixture('independent-reverse-two-step-seam-grid.obj',reverseSeamGridText,{vertices:25,faces:16,triangles:32});
async function seamedSubdivisionFixture(name, positions, faces) {
  let text=`o ${name}\n`;
  for (const position of positions) text+=`v ${position.join(' ')}\n`;
  let corner=0;
  for (let face=0;face<faces.length;face++) for (let i=0;i<faces[face].length;i++) {
    // Separate UV islands expose chart-edge reuse and midpoint rounding.
    text+=`vt ${(.07+face*.121+i*.137).toFixed(6)} ${(.11+face*.093+i*.071).toFixed(6)}\n`;
    corner++;
  }
  let first=1;
  for (const face of faces) {
    text+=`f ${face.map((vertex,i)=>`${vertex+1}/${first+i}`).join(' ')}\n`;
    first+=face.length;
  }
  return fixture(`${name}.obj`,text,{vertices:positions.length,faces:faces.length,triangles:faces.reduce((n,face)=>n+face.length-2,0),has_uv:true,uv_corners:corner});
}
const uvTriangle=await seamedSubdivisionFixture('independent-seamed-tetra',[[1,1,1],[-1,-1,1],[-1,1,-1],[1,-1,-1]],[[0,1,2],[0,3,1],[0,2,3],[1,3,2]]);
const uvMixed=await seamedSubdivisionFixture('independent-seamed-pyramid',[[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0],[0,0,1]],[[3,2,1,0],[0,1,4],[1,2,4],[2,3,4],[3,0,4]]);
let extractText='o Independent_masked_sheet\n';
for (const p of [[0,0,0],[1,0,0],[1,1,0],[0,1,0]]) extractText+=`v ${p.join(' ')}\n`;
extractText+='#MRGB '+['ff204080','ff6080a0','ffa0c0e0','ffe0a060'].join('')+'\n';
extractText+='#MAT '+[1,0,0,1].map(weight=>[.2,.4,weight].map(hexByte).join('')).join('')+'\n';
extractText+='f 1 2 3\nf 1 3 4\n';
const extractSheet=await fixture('independent-masked-sheet.obj',extractText,{vertices:4,faces:2,triangles:2});
let extractCubeText='o Independent_masked_cube\n';
for(const p of cube.positions)extractCubeText+=`v ${p.join(' ')}\n`;
extractCubeText+='#MRGB '+cube.colors.map(c=>'ff'+c.map(x=>x.toString(16).padStart(2,'0')).join('')).join('')+'\n';
extractCubeText+='#MAT '+cube.positions.map((_,i)=>[.2,.4,i===0?0:1].map(hexByte).join('')).join('')+'\n';
for(const f of cubeTriangles)extractCubeText+=`f ${f.map(i=>i+1).join(' ')}\n`;
const extractCube=await fixture('independent-masked-cube.obj',extractCubeText,{vertices:8,faces:12,triangles:12});
const filterWeights=[0,.25,.49,.5,.51,.75,.9,1];
let filterCubeText='o Independent_filter_cube\n';
for(const p of cube.positions)filterCubeText+=`v ${p.join(' ')}\n`;
filterCubeText+='#MAT '+filterWeights.map(weight=>[.2,.4,weight].map(hexByte).join('')).join('')+'\n';
for(const f of cubeTriangles)filterCubeText+=`f ${f.map(i=>i+1).join(' ')}\n`;
const filterCube=await fixture('independent-filter-cube.obj',filterCubeText,{vertices:8,faces:12,triangles:12});
const filterQuadWeights=[.25,1,1,1,0,1,1,1,1];
let filterQuadText='o Independent_filter_quad_grid\n';
for(let row=0;row<3;row++)for(let column=0;column<3;column++)filterQuadText+=`v ${column} ${row} 0\n`;
filterQuadText+='#MAT '+filterQuadWeights.map(weight=>[.2,.4,weight].map(hexByte).join('')).join('')+'\n';
for(const face of [[1,2,5,4],[2,3,6,5],[4,5,8,7],[5,6,9,8]])filterQuadText+=`f ${face.join(' ')}\n`;
const filterQuad=await fixture('independent-filter-quad-grid.obj',filterQuadText,{vertices:9,faces:4,triangles:8});
let extractQuadText='o Independent_masked_quad_cube\n';
for(const p of cube.positions)extractQuadText+=`v ${p.join(' ')}\n`;
extractQuadText+='#MRGB '+cube.colors.map(c=>'ff'+c.map(x=>x.toString(16).padStart(2,'0')).join('')).join('')+'\n';
extractQuadText+='#MAT '+cube.positions.map((_,i)=>[.2,.4,i===1?0:1].map(hexByte).join('')).join('')+'\n';
for(const f of cube.faces)extractQuadText+=`f ${f.map(i=>i+1).join(' ')}\n`;
const extractQuad=await fixture('independent-masked-quad-cube.obj',extractQuadText,{vertices:8,faces:6,triangles:12});
let extractMixedText='o Independent_masked_mixed_cube\n';
for(const p of cube.positions)extractMixedText+=`v ${p.join(' ')}\n`;
extractMixedText+='#MRGB '+cube.colors.map(c=>'ff'+c.map(x=>x.toString(16).padStart(2,'0')).join('')).join('')+'\n';
extractMixedText+='#MAT '+cube.positions.map((_,i)=>[.2,.4,i===1?0:1].map(hexByte).join('')).join('')+'\n';
for(const [i,f] of cube.faces.entries()) {
  const faces=i===2?[[f[0],f[1],f[2]],[f[0],f[2],f[3]]]:[f];
  for(const face of faces)extractMixedText+=`f ${face.map(v=>v+1).join(' ')}\n`;
}
const extractMixed=await fixture('independent-masked-mixed-cube.obj',extractMixedText,{vertices:8,faces:7,triangles:12});
const extractAffine=await fixture('independent-affine-extract.sgl',await readFile('crates/obrush-core/tests/fixtures/independent-affine-extract.sgl'),{vertices:3,faces:1,triangles:1,source_objects:2});

const result={label,out,mirror,started_at:startedAt,skip_mixed:skipMixed,status:prepareOnly?'prepared-only':'running',method:'Owned isolated CLI headless Chrome/CDP; no CUA, native GUI, VoiceOver, user tabs/profile or physical devices',harness_sha256:hash(source),mirror_manifest_sha256:hash(await readFile(path.join(mirror,'mirror-manifest.json'))),mirror_manifest:manifest,fixtures,
  cdp_trace:[],cases:[],snapshots:[],exports:[],dom_inventories:[],input_trace:[],dialogs:[],exceptions:[],console_errors:[],blocked_external_requests:[],requests:[],screenshots:[],
  contract:{scope:'Reference-only behavior/export observations; not OBrush parity',application_mutations:uvCoarseEditOnly||uvTwoLevelOnly||uvThreeLevelOnly?'Import/Subdivide/Lower/Higher use production controls; exactly one deterministic direct coarse vertex mutation plus geometry update isolates synthesis from brush response. No normal edit/history claim.':'Real CDP input into production DOM widgets, native file-input route and real file-saver downloads only. JavaScript evaluates read-only diagnostics; no assignment/calls to tool/mesh/camera setters.',
    arithmetic:'Exact saved f32/i32/u32 arrays and export bytes for future comparisons; no response tolerance or reference equivalence inferred.',brushes:['Brush','Pinch','Twist','LocalScale','Drag'],camera:['Orbit','Spherical','Plane','Perspective','Orthographic'],primitives:['Sphere','Cube','Cylinder','Torus'],file_formats:['OBJ','PLY ASCII/LE binary','STL ASCII/color binary','SGL version3 roundtrip'],
    timings:'No performance benchmark. Short waits only ensure input/render/export completion; no latency, polycount-capacity or physical-input claim.'}};
const writeResult=()=>writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
await writeResult();
if(prepareOnly){console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),harness_sha256:result.harness_sha256,fixtures:fixtures.map(f=>({path:f.path,sha256:f.sha256}))}));process.exit(0);}

let child,socket,session,sequence=0,stderr='',stdout='',server;
const pending=new Map(),downloadEvents=[];
const send=(method,params={},target=session)=>new Promise((resolve,reject)=>{
  const id=++sequence,trace={id,method,at:new Date().toISOString(),...(params.expression?{expression_sha256:hash(params.expression),expression_start:params.expression.slice(0,180)}:{})};result.cdp_trace.push(trace);
  const timer=setTimeout(()=>{pending.delete(id);trace.outcome='timeout';reject(Error('CDP timeout '+method+' request '+id));},30000);pending.set(id,{resolve,reject,timer,trace});socket.send(JSON.stringify({id,method,params,...(target?{sessionId:target}:{})}));
});
const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value;};
const instrument=`(()=>{window.__sculptglReferenceEvents={input:[],errors:[],gl:[],files:[]};const q=window.__sculptglReferenceEvents;
 for(const name of ['pointerdown','pointermove','pointerup','pointercancel','mousedown','mousemove','mouseup','mouseover','mouseout','keydown','keyup','keypress','input','change'])window.addEventListener(name,e=>{const observed={type:name,trusted:e.isTrusted,target:e.target.id||e.target.className||e.target.tagName,key:e.key,x:e.clientX,y:e.clientY,buttons:e.buttons,pressure:e.pressure,pointer_type:e.pointerType,at:performance.now()};q.input.push(observed);queueMicrotask(()=>{observed.default_prevented=e.defaultPrevented;observed.main_focus_gui=window.__sculptglReferenceInstance?._focusGui;});if(q.input.length>5000)q.input.shift();if(name==='change'&&e.target.type==='file')q.files.push({trusted:e.isTrusted,id:e.target.id,names:Array.from(e.target.files||[]).map(f=>f.name)});},true);
 window.addEventListener('error',e=>q.errors.push(String(e.error||e.message)));window.addEventListener('unhandledrejection',e=>q.errors.push(String(e.reason)));
 const get=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){const gl=get.call(this,type,...args);if(gl&&(type==='webgl'||type==='experimental-webgl'||type==='webgl2'))q.gl.push({type,version:gl.getParameter(gl.VERSION),renderer:gl.getParameter(gl.RENDERER)});return gl;};})();`;
const app='window.__sculptglReferenceInstance';
const controller=(name)=>`${app}.getGui().${name}`;
const pointer=async(type,x,y,down=false,modifiers=0)=>{const p={type,x,y,button:type==='mouseMoved'?'none':'left',buttons:down?1:0,modifiers,pointerType:'mouse',force:down?.5:0,...(type==='mouseMoved'?{}:{clickCount:1})};result.input_trace.push(p);await send('Input.dispatchMouseEvent',p);};
const key=async(name,code,vk,modifiers=0)=>{await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:name,code,windowsVirtualKeyCode:vk,modifiers});await send('Input.dispatchKeyEvent',{type:'keyUp',key:name,code,windowsVirtualKeyCode:vk,modifiers});};
const point=async expression=>evaluate(`(()=>{const e=(${expression});if(!e)throw Error('Missing DOM control');const r=e.getBoundingClientRect(),s=getComputedStyle(e);return {x:r.left+r.width/2,y:r.top+r.height/2,rect:[r.left,r.top,r.right,r.bottom],text:e.textContent,type:e.type,value:e.value,disabled:e.disabled,visible:r.width>0&&r.height>0&&s.visibility!=='hidden'&&s.display!=='none'};})()`);
const click=async expression=>{const p=await point(expression);assert(p.visible&&!p.disabled,'Control hidden/disabled: '+expression);assert(p.x>=0&&p.x<1600&&p.y>=0&&p.y<1000,'Control outside owned viewport: '+JSON.stringify(p));await pointer('mouseMoved',p.x,p.y);await pause(35);await pointer('mousePressed',p.x,p.y,true);await pointer('mouseReleased',p.x,p.y);await pause(100);return p;};
async function menu(name){const c=controller(name);const info=await evaluate(`(()=>{const m=${c}._menu;return {folder:m.domUl.hasAttribute('opened'),open:m.domUl.getAttribute('opened'),parent:!!m.domContainer};})()`);if(info.folder){if(info.open!=='true')await click(`${c}._menu.domUl.firstElementChild`);}else{const p=await point(`${c}._menu.domContainer`);await pointer('mouseMoved',p.rect[0]+15,p.rect[1]+12);await pause(100);}}
async function button(name,labelText){await menu(name);const e=`Array.from(${controller(name)}._menu.domUl.querySelectorAll('li,button')).find(e=>(e.tagName==='LI'?Array.from(e.childNodes).filter(n=>n.nodeType===Node.TEXT_NODE).map(n=>n.textContent).join('').trim():e.textContent.trim())===${JSON.stringify(labelText)})`;return click(e);}
async function select(expression,value){
  const opts=await evaluate(`Array.from((${expression}).options).map(o=>({value:o.value,label:o.textContent}))`),index=opts.findIndex(o=>String(o.value)===String(value));assert(index>=0,'Missing select option '+value);
  const header=await evaluate(`(()=>{const e=(${expression}).closest('.gui-topbar > ul > li');if(!e)return null;const r=e.getBoundingClientRect();return [r.left+15,r.top+12];})()`),target=await click(expression);
  await key('Escape','Escape',27);const canvas=await point(`${app}.getCanvas()`);await pointer('mouseMoved',canvas.x,canvas.y);await pause(60);if(header){await pointer('mouseMoved',...header);await pause(100);}await pointer('mouseMoved',target.x,target.y);await pause(900);
  assert(await evaluate(`document.activeElement===(${expression})`),'Native select lost focus');
  // macOS headless Chrome delivers arrow keys but does not select options with
  // them. Trusted native character input invokes the ordinary typeahead route;
  // no DOM value assignment, synthetic event or controller callback is used.
  for(const character of opts[index].label){await send('Input.dispatchKeyEvent',{type:'char',key:character,text:character,unmodifiedText:character,windowsVirtualKeyCode:character.toUpperCase().charCodeAt(0)});await pause(30);}
  await key('Enter','Enter',13);await pause(150);const actual=String(await evaluate(`(${expression}).value`));assert(actual===String(value),'Native select did not select '+value+', actual '+actual);return opts[index];
}
async function number(expression,value){await click(expression);assert(await evaluate(`document.activeElement===(${expression})`),'Number field did not receive native focus');await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4,commands:['selectAll']});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4});await send('Input.insertText',{text:String(value)});await key('Enter','Enter',13);await pause(120);const actual=Number(await evaluate(`(${expression}).value`));assert(actual===value,'Numeric control mismatch: expected '+value+', actual '+actual);}
async function checkbox(expression,checked){const old=await evaluate(`(${expression}).checked`);if(old!==checked)await click(`(${expression}).parentElement`);assert(await evaluate(`(${expression}).checked`)===checked,'Checkbox did not update');}
async function readState(arrays=false){return evaluate(`(()=>{const m=${app},c=m.getCamera(),sel=m.getMesh();const arr=x=>x?Array.from(x):null;return {active_id:sel?.getID(),selected_ids:m.getSelectedMeshes().map(x=>x.getID()),action:m._action,canvas:[m.getCanvas().width,m.getCanvas().height,m.getPixelRatio()],camera:{projection_type:c.getProjectionType(),mode:c.getMode(),fov:c.getFov(),use_pivot:c.getUsePivot(),projection:arr(c.getProjection()),view:arr(c.getView()),quat:arr(c._quatRot),center:arr(c._center),offset:arr(c._offset),translation:arr(c._trans)},tool:{index:m.getSculptManager().getToolIndex(),settings:Object.fromEntries(Object.entries(m.getSculptManager().getCurrentTool()).filter(([k,v])=>['_radius','_intensity','_negative','_clay','_accumulate','_culling','_idAlpha','_lockPosition','_topoCheck','_tangent','_hardness','_thickness','_writeAlbedo','_writeRoughness','_writeMetalness'].includes(k)))},symmetry:m.getSculptManager().getSymmetry(),history:{undo_length:m.getStateManager()._undos.length,current_undo:m.getStateManager()._curUndoIndex,redo_length:m.getStateManager()._redos.length},meshes:m.getMeshes().map(x=>({id:x.getID(),vertices:x.getNbVertices(),faces_count:x.getNbFaces(),triangles:x.getNbTriangles(),texcoord_count:x.getNbTexCoords(),has_uv:x.hasUV(),visible:x.isVisible(),dynamic:!!x.isDynamic,level:x._sel??0,level_count:x._meshes?.length??1,center:arr(x.getCenter()),matrix:arr(x.getMatrix()),render:{shader:x.getShaderType(),matcap:x.getMatcap(),wireframe:x.getShowWireframe(),flat:x.getFlatShading(),opacity:x.getOpacity()},...(${arrays}?{positions:arr(x.getVertices().subarray(0,x.getNbVertices()*3)),normals:arr(x.getNormals().subarray(0,x.getNbVertices()*3)),colors:arr(x.getColors().subarray(0,x.getNbVertices()*3)),materials:arr(x.getMaterials().subarray(0,x.getNbVertices()*3)),faces:arr(x.getFaces().subarray(0,x.getNbFaces()*4)),uvs:arr(x.getTexCoords()?.subarray(0,x.getNbTexCoords()*2)),face_uvs:arr(x.getFacesTexCoord()?.subarray(0,x.getNbFaces()*4))}: {})}))};})()`);}
async function snapshot(name){const state=await readState(true),file=path.join(out,name+'.json');await writeFile(file,JSON.stringify(state,null,2)+'\n');const item={name,path:file,bytes:(await readFile(file)).length,sha256:hash(await readFile(file)),active_id:state.active_id,meshes:state.meshes.map(({positions,normals,colors,materials,faces,uvs,face_uvs,...meta})=>meta)};result.snapshots.push(item);return {state,record:item};}
const meshData=m=>JSON.stringify([m.positions,m.normals,m.colors,m.materials,m.faces,m.uvs,m.face_uvs,m.matrix]);
function compareChannels(a,b){const fields=['positions','normals','colors','materials','faces','uvs','face_uvs','matrix'];return Object.fromEntries(fields.map(field=>{const x=a[field]||[],y=b[field]||[];let different=0,max=0,squared=0;for(let i=0;i<Math.min(x.length,y.length);i++){const d=y[i]-x[i];if(x[i]!==y[i])different++;max=Math.max(max,Math.abs(d));squared+=d*d;}return [field,{exact:x.length===y.length&&different===0,before_length:x.length,after_length:y.length,different_components:different,max_abs_error:max,rms_error:Math.sqrt(squared/Math.max(1,x.length))}];}));}
async function screenshot(name){const r=await send('Page.captureScreenshot',{format:'png',fromSurface:true});const bytes=Buffer.from(r.data,'base64'),file=path.join(out,name+'.png');await writeFile(file,bytes);const item={path:file,bytes:bytes.length,sha256:hash(bytes)};result.screenshots.push(item);return item;}
async function domInventory(name){const data=await evaluate(`(()=>{const d=${app}.getGui();const attrs=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return {tag:e.tagName,classes:e.className,type:e.type,text:e.textContent.trim(),value:e.value,checked:e.checked,disabled:e.disabled,visible:r.width>0&&r.height>0&&s.display!=='none'&&s.visibility!=='hidden',rect:[r.left,r.top,r.right,r.bottom],options:e.tagName==='SELECT'?Array.from(e.options).map(o=>({value:o.value,text:o.textContent})):undefined};};return {controllers:Object.fromEntries(Object.entries(d).filter(([k,v])=>k.startsWith('_ctrl')&&v?._menu).map(([k,v])=>[k,{keys:Object.keys(v),menu_keys:Object.keys(v._menu),html:v._menu.domUl.outerHTML,controls:Array.from(v._menu.domUl.querySelectorAll('input,select,button,label,li')).map(attrs)}])),all_selects:Array.from(document.querySelectorAll('select')).map(attrs),topbar_text:document.querySelector('.gui-topbar').innerText,extra_html:d._topbar.uiExtra?Object.fromEntries(Object.entries(d._topbar.uiExtra).map(([k,v])=>[k,{keys:Object.keys(v),html:v.domContainer?.outerHTML}])):null};})()`);const file=path.join(out,name+'.dom.json');await writeFile(file,JSON.stringify(data,null,2)+'\n');result.dom_inventories.push({name,path:file,sha256:hash(await readFile(file)),controller_count:Object.keys(data.controllers).length,select_count:data.all_selects.length});return data;}
async function clear(){await button('_ctrlScene','Clear scene');for(let i=0;i<50;i++){if((await readState()).meshes.length===0)return;await pause(100);}throw Error('Clear scene did not complete');}
async function importFile(file,expected){const before=(await readState()).meshes.map(m=>m.id),doc=await send('DOM.getDocument',{depth:1}),node=await send('DOM.querySelector',{nodeId:doc.root.nodeId,selector:'#fileopen'});assert(node.nodeId,'Missing actual file input');await send('DOM.setFileInputFiles',{nodeId:node.nodeId,files:[file]});for(let i=0;i<100;i++){const s=await readState();if(s.meshes.length>before.length){await pause(350);const after=await readState();if(expected){const active=after.meshes.find(m=>m.id===after.active_id);assert(active.vertices===expected.vertices&&active.faces_count===expected.faces&&active.triangles===expected.triangles,'Reference import counts differ from independent fixture: '+JSON.stringify(active));}return after;}await pause(100);}throw Error('Actual reference file import timeout: '+file);}
async function exportFile(labelText,name){const index=downloadEvents.length,before=await snapshot(name+'-before-export');await button('_ctrlFiles',labelText);let entry;for(let i=0;i<150;i++){entry=downloadEvents.slice(index).find(x=>x.state==='completed');if(entry)break;await pause(100);}assert(entry,'No actual completed download for '+labelText);const bytes=await readFile(path.join(downloads,entry.guid));assert(bytes.length>0&&bytes.length<128*1024*1024,'Export bytes outside harness cap');const file=path.join(out,name+path.extname(entry.suggestedFilename));await writeFile(file,bytes);const after=await snapshot(name+'-after-export');assert(JSON.stringify(before.state.meshes)===JSON.stringify(after.state.meshes),'Export mutated reference scene');const item={action:labelText,name:entry.suggestedFilename,path:file,bytes:bytes.length,sha256:hash(bytes),guid:entry.guid,input_event_index:result.input_trace.length,before_snapshot:before.record,after_snapshot:after.record};result.exports.push(item);return {item,bytes};}
async function tool(index){await menu('_ctrlSculpting');await select(`${controller('_ctrlSculpting')}._ctrlSculpt.domSelect`,index);assert((await readState()).tool.index===index,'GUI tool index mismatch');}
async function configureTool(index){await tool(index);const g=`${controller('_ctrlSculpting')}`;const radius=`Array.from(${g}._menu.domUl.querySelectorAll('input.gui-input-number')).find(e=>e.getBoundingClientRect().height>0&&e.parentElement.textContent.includes('Radius'))`;await number(radius,110);
  const inputs=await evaluate(`Array.from(${g}._menu.domUl.querySelectorAll('input.gui-input-number')).map((e,i)=>({i,visible:e.getBoundingClientRect().height>0,parent:e.parentElement.textContent,min:e.min,max:e.max}))`);
  const strength=inputs.find(x=>x.visible&&x.parent.includes('Intensity'));if(strength)await number(`${g}._menu.domUl.querySelectorAll('input.gui-input-number')[${strength.i}]`,35);
  await checkbox(`${g}._ctrlSymmetry.domCheckbox`,false);
  if(index===0){const exp=`Array.from(${g}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Clay')`;await checkbox(exp,false);}
  return readState();}
async function stroke(name,index){await clear();await importFile(sphere.path,sphere.expected);await configureTool(index);const before=await snapshot(name+'-before');const canvas=await evaluate(`(()=>{const r=${app}.getCanvas().getBoundingClientRect();return [r.left,r.top,r.width,r.height];})()`);
  const projected=await evaluate(`(()=>{const p=${app}.getCamera().project([.22,.08,.96]);return [p[0]/${app}.getPixelRatio(),p[1]/${app}.getPixelRatio()];})()`);
  const center=[canvas[0]+projected[0],canvas[1]+projected[1]].map(Math.round);let points;
  if(index===2)points=[[0,0],[44,0],[38,24],[24,38],[0,44],[-24,38],[-38,24],[-44,0]].map(p=>[center[0]+p[0],center[1]+p[1]]);
  else if(index===11)points=Array.from({length:8},(_,i)=>[center[0]+i*10,center[1]]);
  else points=Array.from({length:8},(_,i)=>[center[0]+i*20,center[1]+Math.sin(i*.5)*12]);
  await pointer('mouseMoved',...points[0]);await pause(40);await pointer('mousePressed',...points[0],true);for(const p of points.slice(1)){await pointer('mouseMoved',...p,true);await pause(40);}await pointer('mouseReleased',...points.at(-1));await pause(250);const after=await snapshot(name+'-after'),old=before.state.meshes.find(m=>m.id===before.state.active_id),now=after.state.meshes.find(m=>m.id===after.state.active_id);
  let changed=0,maxPositionChange=0,protectedExact=true;for(let i=0;i<old.vertices;i++){const delta=Math.hypot(...old.positions.slice(i*3,i*3+3).map((x,j)=>now.positions[i*3+j]-x));if(delta>0)changed++;maxPositionChange=Math.max(maxPositionChange,delta);if(old.materials[i*3+2]===0&&delta!==0)protectedExact=false;}
  const attributesExact=JSON.stringify([old.colors,old.materials,old.faces,old.uvs,old.face_uvs])===JSON.stringify([now.colors,now.materials,now.faces,now.uvs,now.face_uvs]);
  assert(changed>0,'Reference '+name+' produced no useful geometry change');assert(protectedExact&&attributesExact,'Reference '+name+' unexpectedly changed protected attributes/points');assert(now.positions.every(Number.isFinite)&&now.normals.every(Number.isFinite),'Reference '+name+' nonfinite surface');
  // Real history controls; no direct StateManager calls.
  await menu('_ctrlStates');await button('_ctrlStates','Undo');await pause(180);const undo=await snapshot(name+'-undo');await button('_ctrlStates','Redo');await pause(180);const redo=await snapshot(name+'-redo');
  const u=undo.state.meshes.find(m=>m.id===undo.state.active_id),r=redo.state.meshes.find(m=>m.id===redo.state.active_id);const undoComparison=compareChannels(old,u),redoComparison=compareChannels(now,r),undoExact=meshData(u)===meshData(old),redoExact=meshData(r)===meshData(now);
  if(!undoExact||!redoExact)result.retained_reference_failures.push({name,gate:!undoExact?'undo_all_mesh_channels_exact':'redo_all_mesh_channels_exact',undo:undoComparison,redo:redoComparison,note:'Exact zero-error gate remains failed; subsequent reference-only characterization continues without altering the kernel or tolerance.'});
  const exported=await exportFile('Save .obj',name+'-reference-output');
  const item={name,feature_id:['SGL-BRUSH-BRUSH','SGL-BRUSH-PINCH','SGL-BRUSH-TWIST','SGL-BRUSH-LOCALSCALE','SGL-BRUSH-DRAG'][[0,5,2,11,7].indexOf(index)],input_points:points,tool:before.state.tool,symmetry:before.state.symmetry,camera:before.state.camera,before:before.record,after:after.record,undo:undo.record,redo:redo.record,history_comparison:{undo:undoComparison,redo:redoComparison},export:exported.item,changed_vertices:changed,max_position_change:maxPositionChange,gates:{useful_geometry:true,protected_exact:protectedExact,other_channels_uv_faces_exact:attributesExact,finite_geometry_normals:true,undo_all_mesh_channels_exact:undoExact,redo_all_mesh_channels_exact:redoExact}};result.cases.push(item);await writeResult();return item;}

try{
  result.retained_reference_failures=[];
  server=createServer(async(req,res)=>{const address=new URL(req.url,'http://127.0.0.1');try{const file=path.resolve(mirror,'.'+address.pathname.replace(/\/$/,'/index.html'));if(!file.startsWith(mirror+path.sep)){res.writeHead(403);res.end();return;}const bytes=await readFile(file);const ext=path.extname(file);res.setHeader('Cache-Control','no-store');res.setHeader('Content-Type',({'.js':'text/javascript','.css':'text/css','.png':'image/png','.jpg':'image/jpeg','.html':'text/html','.txt':'text/plain'})[ext]||'application/octet-stream');res.end(bytes);result.requests.push({path:address.pathname,bytes:bytes.length,sha256:hash(bytes),status:200});}catch{res.writeHead(404);res.end();result.requests.push({path:address.pathname,status:404});}});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));result.url=`http://127.0.0.1:${server.address().port}/`;
  const chrome='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';const args=['--headless=new',`--user-data-dir=${profile}`,'--remote-debugging-address=127.0.0.1','--remote-debugging-port=0','--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-component-update','--disable-sync','--disable-default-apps','--disable-breakpad','--metrics-recording-only','--safebrowsing-disable-auto-update','--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication','--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1','--window-size=1600,1000','about:blank'];
  child=spawn(chrome,args,{stdio:['ignore','pipe','pipe']});result.owned_pid=child.pid;child.stderr.on('data',b=>stderr+=b);child.stdout.on('data',b=>stdout+=b);
  let endpoint;for(let i=0;i<100;i++){endpoint=stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1];if(endpoint)break;await pause(100);}assert(endpoint,'Owned Chrome CDP endpoint absent');
  socket=new WebSocket(endpoint);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',event=>{const m=JSON.parse(String(event.data));if(m.id){const p=pending.get(m.id);if(p){clearTimeout(p.timer);pending.delete(m.id);p.trace.outcome=m.error?'protocol-error':'completed';p.trace.completed_at=new Date().toISOString();m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}return;}
    if(m.method==='Runtime.exceptionThrown')result.exceptions.push(m.params.exceptionDetails);
    if(m.method==='Runtime.consoleAPICalled'&&m.params.type==='error')result.console_errors.push(m.params.args.map(x=>x.value||x.description).join(' '));
    if(m.method==='Page.javascriptDialogOpening'){result.dialogs.push(m.params);send('Page.handleJavaScriptDialog',{accept:true},m.sessionId).catch(e=>result.console_errors.push(String(e)));}
    if(m.method==='Fetch.requestPaused'){const request=m.params.request.url,allowed=/^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?\//.test(request)||/^(blob|data|about):/.test(request);if(!allowed)result.blocked_external_requests.push(request);send(allowed?'Fetch.continueRequest':'Fetch.failRequest',{requestId:m.params.requestId,...(allowed?{}:{errorReason:'BlockedByClient'})},m.sessionId).catch(e=>result.console_errors.push(String(e)));}
    if(m.method==='Browser.downloadWillBegin')downloadEvents.push({...m.params,state:'begin'});
    if(m.method==='Browser.downloadProgress'){const entry=downloadEvents.find(x=>x.guid===m.params.guid);if(entry)Object.assign(entry,m.params);}
  });
  result.browser=await send('Browser.getVersion',{},null);const target=await send('Target.createTarget',{url:'about:blank'},null);session=(await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
  for(const domain of ['Page','Runtime','Network','DOM'])await send(domain+'.enable');
  await send('Fetch.enable',{patterns:[{urlPattern:'*',requestStage:'Request'}]});await send('Browser.setDownloadBehavior',{behavior:'allowAndName',downloadPath:downloads,eventsEnabled:true},null);
  await send('Emulation.setDeviceMetricsOverride',{width:1600,height:1000,deviceScaleFactor:1,mobile:false});await send('Emulation.setFocusEmulationEnabled',{enabled:true});await send('Page.addScriptToEvaluateOnNewDocument',{source:instrument});
  await send('Page.navigate',{url:result.url});for(let i=0;i<180;i++){if(await evaluate(`!!(${app}?.getMesh()&&${app}?.getGui()?._ctrlSculpting)`))break;await pause(100);}assert(await evaluate(`!!${app}?.getMesh()`),'Reference startup failed');await pause(500);
  const fetched=Buffer.from(await(await fetch(result.url+'sculptgl.js')).arrayBuffer());assert(hash(fetched)===manifest.original_bundle.sha256,'Served kernel hash differs');
  result.launch_state=await readState();await domInventory('runtime-initial');await screenshot('reference-initial');result.cases.push({name:'reference-launch-and-menus',gates:{captured_normal_instance:true,kernel_hash_exact:true,meshes_present:result.launch_state.meshes.length>0},state:result.launch_state});await writeResult();
  if (gizmoPhysical) {
    const trials=[];
    for(const [axis,type] of [['x',8],['y',16],['z',32]]){
      await clear();await importFile(sphere.path,sphere.expected);await tool(12);
      const before=await snapshot(`physical-gizmo-${axis}-before`);
      const candidate=await evaluate(`(()=>{const main=${app},g=main.getSculptManager().getCurrentTool()._gizmo,c=main.getCamera(),canvas=main.getCanvas(),rect=canvas.getBoundingClientRect(),ratio=main.getPixelRatio(),axis=${JSON.stringify(axis)},target=g['_rot'+axis.toUpperCase()];g._updateMatrices();for(const geo of g._pickables)geo._gizmo.updateMatrix();const v=target._pickGeo.getVertices(),m=target._pickGeo.getMatrix();let checks=0;for(let i=9;i<v.length;i+=9){const x=v[i],y=v[i+1],z=v[i+2],world=[m[0]*x+m[4]*y+m[8]*z+m[12],m[1]*x+m[5]*y+m[9]*z+m[13],m[2]*x+m[6]*y+m[10]*z+m[14]],p=c.project(world);for(const [dx,dy] of [[0,0],[2,0],[-2,0],[0,2],[0,-2],[4,0],[-4,0],[0,4],[0,-4]]){const px=Math.round(rect.left+p[0]/ratio+dx),py=Math.round(rect.top+p[1]/ratio+dy);main._mouseX=ratio*(px-rect.left);main._mouseY=ratio*(py-rect.top);checks++;if(g.onMouseOver()&&g._selected===target){if(px>rect.left+8&&px<rect.right-8&&py>rect.top+8&&py<rect.bottom-8)return {axis,found:true,x:px,y:py,local:[main._mouseX,main._mouseY],checks,rect:[rect.left,rect.top,rect.right,rect.bottom],ratio};}}}return {axis,found:false,checks,rect:[rect.left,rect.top,rect.right,rect.bottom],ratio};})()`);
      assert(candidate.found,`No projected ${axis.toUpperCase()} arc point passed the reference picker`);
      await pointer('mouseMoved',candidate.x,candidate.y);await pause(100);
      const hover=await evaluate(`(()=>{const main=${app},g=main.getSculptManager().getCurrentTool()._gizmo;return {selected_type:g._selected?._type??null,mouse:[main._mouseX,main._mouseY],action:main._action};})()`);
      await pointer('mousePressed',candidate.x,candidate.y,true);await pause(60);
      const pressed=await evaluate(`(()=>{const main=${app},g=main.getSculptManager().getCurrentTool()._gizmo;return {editing:g._isEditing,selected_type:g._selected?._type??null,origin:Array.from(g._editLineOrigin),direction:Array.from(g._editLineDirection),action:main._action};})()`);
      const length=Math.hypot(pressed.direction[0],pressed.direction[1]),dx=pressed.direction[0]/length*30/candidate.ratio,dy=pressed.direction[1]/length*30/candidate.ratio;
      const end=[candidate.x+dx,candidate.y+dy];
      await pointer('mouseMoved',...end,true);await pause(100);
      const drag=await evaluate(`(()=>{const main=${app},g=main.getSculptManager().getCurrentTool()._gizmo;return {editing:g._isEditing,selected_type:g._selected?._type??null,preview:Array.from(main.getMesh().getEditMatrix()),mouse:[main._mouseX,main._mouseY],action:main._action};})()`);
      await pointer('mouseReleased',...end);await pause(180);
      const after=await snapshot(`physical-gizmo-${axis}-after`);
      const old=before.state.meshes[0],next=after.state.meshes[0],m=drag.preview,side=Math.min(candidate.rect[2]-candidate.rect[0],candidate.rect[3]-candidate.rect[1])*candidate.ratio;
      const dist=(drag.mouse[0]-pressed.origin[0])*pressed.direction[0]+(drag.mouse[1]-pressed.origin[1])*pressed.direction[1],angle=-7*dist/side,cos=Math.cos(angle),sin=Math.sin(angle);
      const expected=axis==='x'?[1,0,0,0,0,cos,sin,0,0,-sin,cos,0,0,0,0,1]:axis==='y'?[cos,0,-sin,0,0,1,0,0,sin,0,cos,0,0,0,0,1]:[cos,sin,0,0,-sin,cos,0,0,0,0,1,0,0,0,0,1];
      let matrixError=0,positionError=0,protectedExact=0;
      for(let i=0;i<16;i++)matrixError=Math.max(matrixError,Math.abs(m[i]-expected[i]));
      for(let i=0;i<old.vertices;i++){
        const p=old.positions.slice(3*i,3*i+3),weight=old.materials[3*i+2],full=[m[0]*p[0]+m[4]*p[1]+m[8]*p[2]+m[12],m[1]*p[0]+m[5]*p[1]+m[9]*p[2]+m[13],m[2]*p[0]+m[6]*p[1]+m[10]*p[2]+m[14]];
        for(let k=0;k<3;k++)positionError=Math.max(positionError,Math.abs(next.positions[3*i+k]-(p[k]+weight*(full[k]-p[k]))));
        if(weight===0&&p.every((v,k)=>v===next.positions[3*i+k]))protectedExact++;
      }
      await button('_ctrlStates','Undo');await pause(100);const undo=await snapshot(`physical-gizmo-${axis}-undo`);
      await button('_ctrlStates','Redo');await pause(100);const redo=await snapshot(`physical-gizmo-${axis}-redo`);
      trials.push({axis,type,candidate,hover,pressed,drag,end,before:before.record,after:after.record,undo:undo.record,redo:redo.record,changed:meshData(old)!==meshData(next),statistics:{side,dist,angle,matrix_error:matrixError,position_error:positionError,protected_exact:protectedExact,undo_exact:meshData(undo.state.meshes[0])===meshData(old),redo_exact:meshData(redo.state.meshes[0])===meshData(next)}});
    }
    result.cases.push({name:'pinned-physical-rotation-arcs',fixture:sphere,trials,gates:{all_projected_points_hit:trials.every(t=>t.candidate.found),all_physical_hover:trials.every(t=>t.hover.selected_type===t.type),all_physical_press:trials.every(t=>t.pressed.editing&&t.pressed.selected_type===t.type),all_physical_drag:trials.every(t=>t.drag.editing&&t.drag.preview.some((v,i)=>v!==(i%5===0?1:0))),all_physical_commit:trials.every(t=>t.changed),source_angle_matrix:trials.every(t=>t.statistics.matrix_error<=1e-6),masked_position_oracle:trials.every(t=>t.statistics.position_error<=1e-6&&t.statistics.protected_exact===7),exact_undo_redo:trials.every(t=>t.statistics.undo_exact&&t.statistics.redo_exact)}});
    result.failed_case_gates=result.cases.flatMap(c=>Object.entries(c.gates||{}).filter(([,pass])=>pass===false).map(([gate])=>({case:c.name,gate})));
    result.status=result.failed_case_gates.length?'failed pinned physical gizmo pointer gates':'passed pinned physical gizmo pointer gates';
    if(result.failed_case_gates.length)process.exitCode=1;
    result.limits=['Reference-browser pointer hover, press, drag and release are real CDP mouse events, but candidate positions were located by a diagnostic source picker scan of projected arc geometry; same-fixture OBrush position comparison and physical human-device input remain unverified.'];
  } else if (gizmoOnly) {
    const trials=[];
    for(const [axis,index] of [['x',0],['y',1],['z',2]]){
      await clear();await importFile(sphere.path,sphere.expected);await tool(12);
      const before=await snapshot(`gizmo-${axis}-before`);
      const diagnostic=await evaluate(`(()=>{const main=${app},manager=main.getSculptManager(),tool=manager.getCurrentTool(),gizmo=tool._gizmo,mesh=main.getMesh();if(manager.getToolIndex()!==12||!mesh)throw Error('Pinned Transform tool unavailable');gizmo._updateMatrices();const selected=gizmo._rot${axis.toUpperCase()};gizmo._selected=selected;selected._lastInter[0]=0;selected._lastInter[1]=1.5;selected._lastInter[2]=0;const center=[0,0,0];gizmo._computeCenterGizmo(center);const projected=main.getCamera().project(center);main._mouseX=projected[0];main._mouseY=projected[1];const started=gizmo.onMouseDown();const tangent=gizmo._editLineDirection.slice(0,2),origin=gizmo._editLineOrigin.slice(0,2),side=Math.min(main.getCanvasWidth(),main.getCanvasHeight());main._mouseX=origin[0]+tangent[0]*30;main._mouseY=origin[1]+tangent[1]*30;const updated=gizmo.onMouseOver();const preview=Array.from(mesh.getEditMatrix());const pickable=gizmo._pickables.includes(selected._pickGeo),circle_pickable=gizmo._pickables.includes(gizmo._rotW._pickGeo);gizmo.onMouseUp();tool.end();return {axis:${JSON.stringify(axis)},started,updated,tangent,origin,side,preview,pickable,circle_pickable,pickable_count:gizmo._pickables.length,declared_circle:!!gizmo._rotW._drawGeo};})()`);
      const after=await snapshot(`gizmo-${axis}-after`);
      const old=before.state.meshes[0],next=after.state.meshes[0],m=diagnostic.preview,angle=-7*30/diagnostic.side,cos=Math.cos(angle),sin=Math.sin(angle);
      const expected=axis==='x'?[1,0,0,0,0,cos,sin,0,0,-sin,cos,0,0,0,0,1]:axis==='y'?[cos,0,-sin,0,0,1,0,0,sin,0,cos,0,0,0,0,1]:[cos,sin,0,0,-sin,cos,0,0,0,0,1,0,0,0,0,1];
      let matrixError=0,positionError=0,protectedExact=0,partial=0;
      for(let i=0;i<16;i++)matrixError=Math.max(matrixError,Math.abs(m[i]-expected[i]));
      for(let i=0;i<old.vertices;i++){
        const p=old.positions.slice(3*i,3*i+3),weight=old.materials[3*i+2],full=[m[0]*p[0]+m[4]*p[1]+m[8]*p[2]+m[12],m[1]*p[0]+m[5]*p[1]+m[9]*p[2]+m[13],m[2]*p[0]+m[6]*p[1]+m[10]*p[2]+m[14]];
        for(let k=0;k<3;k++)positionError=Math.max(positionError,Math.abs(next.positions[3*i+k]-(p[k]+weight*(full[k]-p[k]))));
        if(weight===0){protectedExact++;assert(p.every((v,k)=>v===next.positions[3*i+k]),'Pinned rotation moved a protected vertex');}
        else if(weight<1)partial++;
      }
      trials.push({axis,diagnostic,before:before.record,after:after.record,changed:meshData(old)!==meshData(next),statistics:{matrix_error:matrixError,position_error:positionError,protected_exact:protectedExact,partial}});
    }
    await clear();await importFile(sphere.path,sphere.expected);await tool(12);
    const viewCircle=await evaluate(`(()=>{const main=${app},tool=main.getSculptManager().getCurrentTool(),gizmo=tool._gizmo,mesh=main.getMesh();gizmo._updateMatrices();const pickable=gizmo._pickables.includes(gizmo._rotW._pickGeo);gizmo._selected=gizmo._rotW;const center=[0,0,0];gizmo._computeCenterGizmo(center);const p=main.getCamera().project(center);main._mouseX=p[0];main._mouseY=p[1];const before=Array.from(mesh.getEditMatrix());const started=gizmo.onMouseDown();main._mouseX+=40;main._mouseY+=25;const updated=gizmo.onMouseOver();const after=Array.from(mesh.getEditMatrix());gizmo.onMouseUp();return {declared:!!gizmo._rotW._drawGeo,pickable,started,updated,before,after,selected_type:gizmo._rotW._type};})()`);
    result.cases.push({name:'pinned-gizmo-rotation-dispatch',fixture:sphere,axis_trials:trials,view_circle:viewCircle,gates:{all_axes_pickable:trials.every(t=>t.diagnostic.pickable),all_axes_update:trials.every(t=>t.diagnostic.updated&&t.diagnostic.preview.some((v,i)=>v!==(i%5===0?1:0))),all_axes_commit:trials.every(t=>t.changed),source_matrix:trials.every(t=>t.statistics.matrix_error<=1e-6),weighted_positions:trials.every(t=>t.statistics.position_error<=1e-6&&t.statistics.protected_exact>0),view_circle_declared:viewCircle.declared,view_circle_not_pickable:!viewCircle.pickable,view_circle_no_edit:JSON.stringify(viewCircle.before)===JSON.stringify(viewCircle.after)}});
    result.status='completed pinned gizmo direct runtime dispatch probe';
    result.limits=['Source browser runtime and real file import, but gizmo selection and pointer coordinates were injected into the diagnostic methods; physical arc picking, drag event delivery and matched OBrush positions remain unverified.'];
  } else if (transformOnly) {
    await clear();await importFile(filterCube.path,filterCube.expected);await tool(12);
    const before=await snapshot('masked-transform-before');
    const kernel=await evaluate(`(()=>{const main=${app},mesh=main.getMesh(),manager=main.getSculptManager(),tool=manager.getCurrentTool(),matrix=mesh.getEditMatrix();if(manager.getToolIndex()!==12)throw Error('Current source tool index is not Transform');matrix[12]=0.125;const incoming=Array.from(matrix);const selected=main.getSelectedMeshes().map(x=>x.getID());tool.end();return {tool_index:manager.getToolIndex(),incoming_edit_matrix:incoming,selected_ids:selected,final_edit_matrix:Array.from(mesh.getEditMatrix())};})()`);
    const after=await snapshot('masked-transform-after');
    await button('_ctrlStates','Undo');const undo=await snapshot('masked-transform-undo');
    await button('_ctrlStates','Redo');const redo=await snapshot('masked-transform-redo');
    const old=before.state.meshes[0],next=after.state.meshes[0];let maxError=0,protectedExact=0,partial=0;
    for(let i=0;i<old.vertices;i++){
      const weight=old.materials[3*i+2],expected=old.positions[3*i]+.125*weight;
      maxError=Math.max(maxError,Math.abs(next.positions[3*i]-expected),Math.abs(next.positions[3*i+1]-old.positions[3*i+1]),Math.abs(next.positions[3*i+2]-old.positions[3*i+2]));
      if(weight===0){protectedExact++;assert(next.positions.slice(i*3,i*3+3).every((v,k)=>v===old.positions[3*i+k]),'Pinned runtime moved a protected vertex');}
      else if(weight<1)partial++;
    }
    const undoExact=meshData(undo.state.meshes[0])===meshData(old),redoExact=meshData(redo.state.meshes[0])===meshData(next);
    result.cases.push({name:'pinned-transform-end-mask-weighted-translation',fixture:filterCube,kernel,before:before.record,after:after.record,undo:undo.record,redo:redo.record,statistics:{max_error: maxError,protected_exact:protectedExact,partial},gates:{selected_one:kernel.selected_ids.length===1,tool_actual:kernel.tool_index===12,edit_matrix_cleared:kernel.final_edit_matrix.every((v,i)=>v===(i%5===0?1:0)),formula:maxError<=1e-6,protected_exact:protectedExact>0,partial:partial>0,undo_exact:undoExact,redo_exact:redoExact}});
    await clear();await importFile(filterCube.path,filterCube.expected);await tool(12);
    const affineBefore=await snapshot('masked-affine-before');
    const affineKernel=await evaluate(`(()=>{const main=${app},mesh=main.getMesh(),manager=main.getSculptManager(),tool=manager.getCurrentTool(),matrix=mesh.getEditMatrix();if(manager.getToolIndex()!==12)throw Error('Current source tool index is not Transform');const [rx,ry,rz]=[7,-3,5].map(x=>x*Math.PI/180),[cx,sx,cy,sy,cz,sz]=[Math.cos(rx),Math.sin(rx),Math.cos(ry),Math.sin(ry),Math.cos(rz),Math.sin(rz)],scale=1.05;const linear=[cz*cy,sz*cy,-sy,cz*sy*sx-sz*cx,sz*sy*sx+cz*cx,cy*sx,cz*sy*cx+sz*sx,sz*sy*cx-cz*sx,cy*cx].map(x=>x*scale);for(let column=0;column<3;column++)for(let row=0;row<3;row++)matrix[column*4+row]=linear[column*3+row];matrix[12]=.02;matrix[13]=-.015;matrix[14]=.01;const incoming=Array.from(matrix);tool.end();return {tool_index:manager.getToolIndex(),incoming_edit_matrix:incoming,final_edit_matrix:Array.from(mesh.getEditMatrix())};})()`);
    const affineAfter=await snapshot('masked-affine-after');
    await button('_ctrlStates','Undo');const affineUndo=await snapshot('masked-affine-undo');
    await button('_ctrlStates','Redo');const affineRedo=await snapshot('masked-affine-redo');
    const oldAffine=affineBefore.state.meshes[0],newAffine=affineAfter.state.meshes[0],matrix=affineKernel.incoming_edit_matrix;
    let affineMaxError=0,affineProtectedExact=0,affinePartial=0;
    for(let i=0;i<oldAffine.vertices;i++){
      const p=oldAffine.positions.slice(i*3,i*3+3),weight=oldAffine.materials[3*i+2];
      const fullyMoved=[0,1,2].map(k=>p[0]*matrix[k]+p[1]*matrix[4+k]+p[2]*matrix[8+k]+matrix[12+k]);
      for(let k=0;k<3;k++)affineMaxError=Math.max(affineMaxError,Math.abs(newAffine.positions[3*i+k]-(p[k]+(fullyMoved[k]-p[k])*weight)));
      if(weight===0){affineProtectedExact++;assert(newAffine.positions.slice(i*3,i*3+3).every((v,k)=>v===p[k]),'Pinned affine runtime moved a protected vertex');}
      else if(weight<1)affinePartial++;
    }
    const affineUndoExact=meshData(affineUndo.state.meshes[0])===meshData(oldAffine),affineRedoExact=meshData(affineRedo.state.meshes[0])===meshData(newAffine);
    result.cases.push({name:'pinned-transform-end-mask-weighted-affine',fixture:filterCube,kernel:affineKernel,before:affineBefore.record,after:affineAfter.record,undo:affineUndo.record,redo:affineRedo.record,statistics:{max_error:affineMaxError,protected_exact:affineProtectedExact,partial:affinePartial},gates:{tool_actual:affineKernel.tool_index===12,edit_matrix_cleared:affineKernel.final_edit_matrix.every((v,i)=>v===(i%5===0?1:0)),formula:affineMaxError<=1e-6,protected_exact:affineProtectedExact>0,partial:affinePartial>0,undo_exact:affineUndoExact,redo_exact:affineRedoExact}});
    await clear();await importFile(filterCube.path,filterCube.expected);await tool(12);
    const axisBefore=await snapshot('masked-axis-before');
    const axisKernel=await evaluate(`(()=>{const main=${app},mesh=main.getMesh(),manager=main.getSculptManager(),tool=manager.getCurrentTool(),matrix=mesh.getEditMatrix();if(manager.getToolIndex()!==12)throw Error('Current source tool index is not Transform');matrix[0]=1.2;matrix[5]=.8;matrix[10]=1.05;const incoming=Array.from(matrix);tool.end();return {tool_index:manager.getToolIndex(),incoming_edit_matrix:incoming,final_edit_matrix:Array.from(mesh.getEditMatrix())};})()`);
    const axisAfter=await snapshot('masked-axis-after');
    await button('_ctrlStates','Undo');const axisUndo=await snapshot('masked-axis-undo');
    await button('_ctrlStates','Redo');const axisRedo=await snapshot('masked-axis-redo');
    const oldAxis=axisBefore.state.meshes[0],newAxis=axisAfter.state.meshes[0],factors=[1.2,.8,1.05];
    let axisMaxError=0,axisProtectedExact=0,axisPartial=0;
    for(let i=0;i<oldAxis.vertices;i++){
      const weight=oldAxis.materials[3*i+2];
      for(let k=0;k<3;k++)axisMaxError=Math.max(axisMaxError,Math.abs(newAxis.positions[3*i+k]-oldAxis.positions[3*i+k]*(1+(axisKernel.incoming_edit_matrix[k*5]-1)*weight)));
      if(weight===0){axisProtectedExact++;assert(newAxis.positions.slice(i*3,i*3+3).every((v,k)=>v===oldAxis.positions[3*i+k]),'Pinned axis runtime moved a protected vertex');}
      else if(weight<1)axisPartial++;
    }
    const axisUndoExact=meshData(axisUndo.state.meshes[0])===meshData(oldAxis),axisRedoExact=meshData(axisRedo.state.meshes[0])===meshData(newAxis);
    result.cases.push({name:'pinned-transform-end-mask-weighted-axis',fixture:filterCube,kernel:axisKernel,before:axisBefore.record,after:axisAfter.record,undo:axisUndo.record,redo:axisRedo.record,statistics:{max_error:axisMaxError,protected_exact:axisProtectedExact,partial:axisPartial},gates:{tool_actual:axisKernel.tool_index===12,edit_matrix_cleared:axisKernel.final_edit_matrix.every((v,i)=>v===(i%5===0?1:0)),formula:axisMaxError<=1e-6,protected_exact:axisProtectedExact>0,partial:axisPartial>0,undo_exact:axisUndoExact,redo_exact:axisRedoExact,three_distinct_factors:new Set(factors).size===3}});
    result.status='completed pinned Transform.end masked translation, affine and axis-scale runtime fixtures';
    result.limits=['Actual pinned tool kernel and source UI import/Undo/Redo, but the three edit matrices were injected by the diagnostic rather than produced by physical gizmo drags; one selected eight-vertex cube only.'];
  } else if (reverseSeamOnly) {
    await clear();await importFile(reverseSeamGrid.path,reverseSeamGrid.expected);
    const before=await snapshot('reverse-seam-before');
    await button('_ctrlTopology','Reverse');
    const first=await snapshot('reverse-seam-first');
    const firstObj=await exportFile('Save .obj','reverse-seam-first-reference');
    await button('_ctrlTopology','Reverse');
    const second=await snapshot('reverse-seam-second');
    await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,3);
    const returned=await snapshot('reverse-seam-returned');
    result.cases.push({name:'reference-reverse-two-step-seam-grid',fixture:reverseSeamGrid,before:before.record,first:first.record,first_obj:firstObj.item,second:second.record,returned:returned.record,uv_counts:[before,first,second,returned].map(x=>x.state.meshes[0].texcoord_count),gates:{first_reverse:first.state.meshes[0]?.level_count===2,second_reverse:second.state.meshes[0]?.level_count===3,returned_fine:returned.state.meshes[0]?.faces_count===16}});
    result.status='completed reference-only repeated UV seam Reverse fixture';
    result.limits=['One independently authored split seam on a nonplanar quad grid; OBrush comparison and other seam placements remain separate.'];
  } else if (reverseOnly) {
    await clear();await importFile(reverseQuad.path,reverseQuad.expected);
    const before=await snapshot('reverse-before');
    await button('_ctrlTopology','Reverse');
    const coarse=await snapshot('reverse-coarse');
    await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,2);
    const returned=await snapshot('reverse-returned');
    result.cases.push({name:'reference-reverse-independent-quad',fixture:reverseQuad,before:before.record,coarse:coarse.record,returned:returned.record,gates:{coarse_level_added:coarse.state.meshes[0]?.level_count===2,one_coarse_face:coarse.state.meshes[0]?.faces_count===1,fine_returned:returned.state.meshes[0]?.faces_count===4}});
    await clear();await importFile(reverseGrid.path,reverseGrid.expected);
    const gridBefore=await snapshot('reverse-grid-before');
    await button('_ctrlTopology','Reverse');
    const gridFirst=await snapshot('reverse-grid-first');
    await button('_ctrlTopology','Reverse');
    const gridSecond=await snapshot('reverse-grid-second');
    await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,3);
    const gridReturned=await snapshot('reverse-grid-returned');
    result.cases.push({name:'reference-reverse-two-step-grid',fixture:reverseGrid,before:gridBefore.record,first:gridFirst.record,second:gridSecond.record,returned:gridReturned.record,gates:{first_level_added:gridFirst.state.meshes[0]?.level_count===2,second_level_added:gridSecond.state.meshes[0]?.level_count===3,coarse_face_recovered:gridSecond.state.meshes[0]?.faces_count===1,fine_returned:gridReturned.state.meshes[0]?.faces_count===16}});
    await clear();await importFile(reverseUvGrid.path,reverseUvGrid.expected);
    const uvBefore=await snapshot('reverse-uv-grid-before');
    await button('_ctrlTopology','Reverse');
    const uvFirst=await snapshot('reverse-uv-grid-first');
    await button('_ctrlTopology','Reverse');
    const uvSecond=await snapshot('reverse-uv-grid-second');
    await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,3);
    const uvReturned=await snapshot('reverse-uv-grid-returned');
    result.cases.push({name:'reference-reverse-two-step-uv-grid',fixture:reverseUvGrid,before:uvBefore.record,first:uvFirst.record,second:uvSecond.record,returned:uvReturned.record,channel_observations:{before_uv_count:uvBefore.state.meshes[0].texcoord_count,first_uv_count:uvFirst.state.meshes[0].texcoord_count,second_uv_count:uvSecond.state.meshes[0].texcoord_count,returned_uv_count:uvReturned.state.meshes[0].texcoord_count},gates:{first_level_added:uvFirst.state.meshes[0]?.level_count===2,second_level_added:uvSecond.state.meshes[0]?.level_count===3,coarse_face_recovered:uvSecond.state.meshes[0]?.faces_count===1,fine_uv_returned:uvReturned.state.meshes[0]?.faces_count===16&&uvReturned.state.meshes[0]?.texcoord_count===25}});
    result.status='completed reference-only Reverse fixtures';
    result.limits=['One single-step quad and one two-step nonplanar quad grid; matched OBrush geometry is recorded separately.','Other topology, UV, edited-channel and sustained-performance behavior require separate checks.'];
    result.final_diagnostics=await evaluate('window.__sculptglReferenceEvents');
  } else if (uvSubdivisionOnly || uvCoarseEditOnly || uvTwoLevelOnly || uvThreeLevelOnly) {
    for (const referenceFixture of [uvTriangle,quadObj,uvMixed]) {
      const name=path.parse(referenceFixture.name).name;
      await clear();await importFile(referenceFixture.path,referenceFixture.expected);
      const before=await snapshot(name+'-uv-subdivision-before');
      await button('_ctrlTopology','Subdivide');
      let fineState;
      for(let i=0;i<100;i++){fineState=await readState();if(fineState.meshes[0]?.level_count===2)break;await pause(100);}
      assert(fineState.meshes[0]?.level_count===2,'Reference UV subdivision did not add a level: '+name);
      const fine=await snapshot(name+'-uv-subdivision-fine');
      const fineMesh=fine.state.meshes[0];
      assert(fineMesh.has_uv&&fineMesh.faces_count===referenceFixture.expected.faces*4,'Reference UV child count/UV failed: '+name);
      let secondFine,secondSourceFrame;
      if(uvTwoLevelOnly||uvThreeLevelOnly){
        await button('_ctrlTopology','Subdivide');
        for(let i=0;i<100;i++){fineState=await readState();if(fineState.meshes[0]?.level_count===3)break;await pause(100);}
        assert(fineState.meshes[0]?.level_count===3,'Reference UV second subdivision failed: '+name);
        secondFine=await snapshot(name+'-uv-second-fine');
        secondSourceFrame=await evaluate(`(()=>{const fine=${app}.getMesh()._meshes[2],starts=fine.getVerticesRingVertStartCount(),ring=fine.getVerticesRingVert();return {first_neighbor:Array.from({length:fine.getNbVertices()},(_,i)=>ring[starts[i*2]])};})()`);
      }
      let thirdFine,thirdSourceFrame;
      if(uvThreeLevelOnly){
        await button('_ctrlTopology','Subdivide');
        for(let i=0;i<100;i++){fineState=await readState();if(fineState.meshes[0]?.level_count===4)break;await pause(100);}
        assert(fineState.meshes[0]?.level_count===4,'Reference UV third subdivision failed: '+name);
        thirdFine=await snapshot(name+'-uv-third-fine');
        thirdSourceFrame=await evaluate(`(()=>{const fine=${app}.getMesh()._meshes[3],starts=fine.getVerticesRingVertStartCount(),ring=fine.getVerticesRingVert();return {first_neighbor:Array.from({length:fine.getNbVertices()},(_,i)=>ring[starts[i*2]])};})()`);
      }
      await menu('_ctrlTopology');
      await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,1);
      const coarse=await snapshot(name+'-uv-subdivision-coarse');
      if (uvCoarseEditOnly || uvTwoLevelOnly || uvThreeLevelOnly) {
        // Controlled reference diagnostic: change one coarse vertex directly
        // to isolate level synthesis from differing brush implementations.
        // This is deliberately not a normal UI editing or history claim.
        const delta=[0.125,-0.09375,0.0625];
        const sourceFrame=await evaluate(`(()=>{const fine=${app}.getMesh()._meshes[1],starts=fine.getVerticesRingVertStartCount(),ring=fine.getVerticesRingVert();return {detail_xyz:Array.from(fine._detailsXYZ),first_neighbor:Array.from({length:fine.getNbVertices()},(_,i)=>ring[starts[i*2]])};})()`);
        const partialPrediction=()=>evaluate(`(()=>{const m=${app}.getMesh(),low=m.getCurrentMesh(),n=m._meshes[1].getNbVertices(),p=new Float32Array(n*3),c=new Float32Array(n*3),mat=new Float32Array(n*3);low.computePartialSubdivision(p,c,mat,n);return Array.from(p);})()`);
        const oldPrediction=await partialPrediction();
        const edit=await evaluate(`(()=>{const m=${app}.getMesh(),level=m.getCurrentMesh(),v=level.getVertices(),before=Array.from(v.subarray(0,3)),delta=${JSON.stringify(delta)};for(let i=0;i<3;i++)v[i]+=delta[i];level.updateGeometry();m.updateResolution();return {vertex:0,before,after:Array.from(v.subarray(0,3)),delta};})()`);
        const editedCoarse=await snapshot(name+'-uv-coarse-edited');
        const newPrediction=await partialPrediction();
        await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,2);
        const editedFine=await snapshot(name+'-uv-fine-after-coarse-edit');
        let editedSecondFine;
        if(uvTwoLevelOnly||uvThreeLevelOnly){
          await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,3);
          editedSecondFine=await snapshot(name+'-uv-second-fine-after-coarse-edit');
        }
        let editedThirdFine;
        if(uvThreeLevelOnly){
          await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,4);
          editedThirdFine=await snapshot(name+'-uv-third-fine-after-coarse-edit');
        }
        result.cases.push({name:(uvThreeLevelOnly?'reference-uv-three-level-edit-':uvTwoLevelOnly?'reference-uv-two-level-edit-':'reference-uv-coarse-edit-')+name,fixture:referenceFixture,before:before.record,fine:fine.record,second_fine:secondFine?.record,third_fine:thirdFine?.record,second_source_frame:secondSourceFrame,third_source_frame:thirdSourceFrame,coarse:coarse.record,source_frame:sourceFrame,old_prediction:oldPrediction,new_prediction:newPrediction,edit,edited_coarse:editedCoarse.record,edited_fine:editedFine.record,edited_second_fine:editedSecondFine?.record,edited_third_fine:editedThirdFine?.record,method:'diagnostic direct first-vertex coarse edit, then actual Higher GUI control for one, two or three levels; not normal brush/history path'});await writeResult();
        continue;
      }
      await number(`${controller('_ctrlTopology')}._ctrlResolution.domInputText`,2);
      const returned=await snapshot(name+'-uv-subdivision-returned');
      const coarseExact=meshData(coarse.state.meshes[0])===meshData(before.state.meshes[0]);
      const fineExact=meshData(returned.state.meshes[0])===meshData(fineMesh);
      result.cases.push({name:'reference-uv-subdivision-'+name,fixture:referenceFixture,before:before.record,fine:fine.record,coarse:coarse.record,returned:returned.record,counts:{source_uv:before.state.meshes[0].texcoord_count,child_uv:fineMesh.texcoord_count,source_faces:referenceFixture.expected.faces,child_faces:fineMesh.faces_count},gates:{real_subdivision:true,uv_retained:true,coarse_exact:coarseExact,fine_return_exact:fineExact}});await writeResult();
    }
    result.final_diagnostics=await evaluate('window.__sculptglReferenceEvents');
    result.status=uvThreeLevelOnly?'completed reference-only controlled three-level edit fixtures':uvTwoLevelOnly?'completed reference-only controlled two-level edit fixtures':uvCoarseEditOnly?'completed reference-only controlled coarse-edit subdivision fixtures':'completed reference-only UV subdivision comparison fixtures';
    result.limits=['Three independently authored all-seamed triangle, quad and mixed fixtures',uvCoarseEditOnly||uvTwoLevelOnly||uvThreeLevelOnly?'Direct typed-array coarse vertex edit isolates synthesis; no real brush or undo claim':'Actual import, subdivision, lower and higher controls; no OBrush or texture-pixel comparison in this reference run','Source UV coordinate allocation order may differ from equivalent corner mappings'];
  } else if (blockRemeshOnly) {
    for(const block of [false,true]){
      await clear();await importFile(filterCube.path,filterCube.expected);await menu('_ctrlTopology');
      const topology=`${controller('_ctrlTopology')}`;
      const resolution=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=number]')).find(e=>e.min==='8')`;
      await number(resolution,64);
      const blockInput=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Block')`;
      await checkbox(blockInput,block);
      const before=await snapshot('block-remesh-'+block+'-before');
      await button('_ctrlTopology','Remesh');
      let afterState;for(let i=0;i<300;i++){afterState=await readState();if(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices)break;await pause(100);}
      assert(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices,'Reference block remesh did not change the selected geometry');
      const after=await snapshot('block-remesh-'+block+'-after');
      const output=after.state.meshes.find(m=>m.id===after.state.active_id);
      result.cases.push({name:'reference-remesh-block-'+block,resolution:64,block,before:before.record,after:after.record,output:{vertices:output.vertices,faces:output.faces_count,triangles:output.triangles,positions:output.positions,faces_array:output.faces},gates:{active_geometry_changed:output.vertices!==before.state.meshes[0].vertices,nonempty_output:output.vertices>0&&output.triangles>0}});await writeResult();
    }
    result.status='passed reference-only block remesh workflows';
    result.limits=['One independently authored closed triangulated cube, 64 resolution, normal and Block Surface Nets buttons','No OBrush pairing or general-shape/performance parity'];
  } else if (maskFilterOnly) {
    for(const referenceFixture of [filterCube,filterQuad])for(const operation of ['Blur','Sharpen']){
      await clear();await importFile(referenceFixture.path,referenceFixture.expected);await tool(10);
      const label=path.parse(referenceFixture.name).name+'-'+operation.toLowerCase();
      const before=await snapshot('mask-'+label+'-before');
      await button('_ctrlSculpting',operation);
      const after=await snapshot('mask-'+label+'-after');
      const a=before.state.meshes[0],b=after.state.meshes[0];
      const otherExact=['positions','normals','colors','faces','uvs','face_uvs','matrix'].every(key=>JSON.stringify(a[key])===JSON.stringify(b[key]));
      const masks=a.materials.filter((_,i)=>i%3===2),updated=b.materials.filter((_,i)=>i%3===2);
      const changed=masks.reduce((n,value,i)=>n+Number(value!==updated[i]),0);
      result.cases.push({name:'reference-mask-'+label,fixture:referenceFixture.name,before:before.record,after:after.record,masks_before:masks,masks_after:updated,history_before:before.state.history,history_after:after.state.history,gates:{other_channels_exact:otherExact,mask_changed:changed>0,vertex_count_exact:a.vertices===b.vertices}});await writeResult();
    }
    result.final_diagnostics=await evaluate('window.__sculptglReferenceEvents');
    result.status='passed reference-only mask filter workflows';
    result.limits=['One independently authored triangulated cube and one open quad grid, reference-only','Blur and Sharpen each invoked once per fixture through visible controls','No OBrush pairing or long-session result'];
  } else if (extractOnly) {
    if(extractDiagnostic){
      await evaluate(`(()=>{const tool=${app}.getSculptManager().getTool(10),original=tool.smoothBorder;window.__sculptglExtractDiagnostics=[];tool.smoothBorder=function(mesh,iFaces){const start=iFaces.length*2,bridgeFaces=Array.from({length:mesh.getNbFaces()-start},(_,i)=>start+i),initialVertices=Array.from(mesh.getVerticesFromFaces(bridgeFaces)),selected=Array.from(mesh.expandsVertices(new Uint32Array(initialVertices),1)),startCount=mesh.getVerticesRingVertStartCount(),ring=mesh.getVerticesRingVert(),neighbors=selected.map(id=>{const a=startCount[id*2],n=startCount[id*2+1];return [id,Array.from(ring.subarray(a,a+n))];}),before=Array.from(mesh.getVertices().subarray(0,mesh.getNbVertices()*3)),faces=Array.from(mesh.getFaces().subarray(0,mesh.getNbFaces()*4)),onEdge=Array.from(mesh.getVerticesOnEdge());const value=original.apply(this,arguments);window.__sculptglExtractDiagnostics.push({iFaces:Array.from(iFaces),bridgeFaces,initialVertices,selected,neighbors,before,after:Array.from(mesh.getVertices().subarray(0,mesh.getNbVertices()*3)),faces,onEdge});return value;};return true;})()`);
    }
    for (const referenceFixture of (extractAffineOnly?[extractAffine]:extractMixedOnly?[extractMixed]:extractQuadOnly?[extractQuad]:[extractSheet,extractCube])) for (const thickness of (extractDiagnostic?[-.125,.125]:[0,-.125,.125])) {
      await clear();await importFile(referenceFixture.path,referenceFixture.expected);
      const source=await snapshot(`extract-${path.parse(referenceFixture.name).name}-${thickness}-source`);
      await tool(10);
      const sculpt=controller('_ctrlSculpting');
      const thicknessInput=`Array.from(${sculpt}._menu.domUl.querySelectorAll('input.gui-input-number')).find(e=>e.getBoundingClientRect().height>0&&e.parentElement.textContent.includes('Thickness'))`;
      await number(thicknessInput,thickness);
      const actual=await readState();assert(actual.tool.index===10&&actual.tool.settings._thickness===thickness,'Reference mask extraction controls did not commit');
      await button('_ctrlSculpting','Extract !');
      for(let i=0;i<100;i++){if((await readState()).meshes.length===source.state.meshes.length+1)break;await pause(100);}
      const after=await snapshot(`extract-${path.parse(referenceFixture.name).name}-${thickness}-after`);
      const sourceIds=new Set(source.state.meshes.map(m=>m.id)),created=after.state.meshes.find(m=>!sourceIds.has(m.id));
      assert(created&&after.state.meshes.length===source.state.meshes.length+1,'Reference extraction did not retain source meshes and create one part');
      const originalExact=source.state.meshes.every(original=>{const retained=after.state.meshes.find(m=>m.id===original.id);return retained&&meshData(original)===meshData(retained);});
      const sourceActive=after.state.active_id===source.state.active_id;
      const colorsDefault=created.colors.every(x=>x===1);
      const materialsDefault=created.materials.every((x,i)=>x===Math.fround([.18,.08,1][i%3]));
      const item={name:'mask-extract-reference',fixture:referenceFixture.name,thickness,source:source.record,after:after.record,source_active:sourceActive,source_exact:originalExact,created:{id:created.id,vertices:created.vertices,faces:created.faces_count,triangles:created.triangles,positions:created.positions,faces_array:created.faces,colors_default:colorsDefault,materials_default:materialsDefault},gates:{source_retained_exact:originalExact,source_stays_active:sourceActive,created_part:true,default_colors:colorsDefault,default_materials:materialsDefault}};
      if(extractDiagnostic)item.smoothing_diagnostic=await evaluate('window.__sculptglExtractDiagnostics');
      result.cases.push(item);await writeResult();
    }
    result.final_diagnostics=await evaluate('window.__sculptglReferenceEvents');
    result.failed_case_gates=result.cases.flatMap(c=>Object.entries(c.gates||{}).filter(([,pass])=>pass===false).map(([gate])=>({case:c.name,gate})));
    result.status=result.failed_case_gates.length?'completed extraction characterization with failed exact gates':'passed reference-only mask extraction workflows';
    result.limits=['Reference-only behavior, not OBrush parity','Fixture-specific extraction; transformed and curved surfaces remain unverified','No native or physical input and no timing claim'];
  } else {
  await clear();const emptySgl=await exportFile('Save .sgl (SculptGL)','empty-reference-sgl');result.empty_sgl=emptySgl.item;await writeResult();
  for(const [name,index] of [['brush',0],['pinch',5],['twist',2],['local-scale',11],['drag',7]])await stroke(name,index);

  await clear();await importFile(sphere.path,sphere.expected);const original=await snapshot('camera-original');await menu('_ctrlCamera');const cam=`${controller('_ctrlCamera')}`;
  const cameraTrials=[];for(const projection of [1,0]){await select(`${cam}._ctrlProjection.domSelect`,projection);assert((await readState()).camera.projection_type===projection,'Camera projection mismatch');cameraTrials.push(await readState());}
  const modeSelect=`Array.from(${cam}._menu.domUl.querySelectorAll('select')).find(e=>Array.from(e.options).some(o=>o.textContent.includes('Spherical')))`;
  for(const mode of [1,2,0]){await menu('_ctrlCamera');await select(modeSelect,mode);assert((await readState()).camera.mode===mode,'Camera mode mismatch');cameraTrials.push(await readState());}
  await menu('_ctrlCamera');await number(`${cam}._ctrlFov.domInputText`,60);assert((await readState()).camera.fov===60,'FOV mismatch');cameraTrials.push(await readState());
  const cameraAfter=await snapshot('camera-after');assert(JSON.stringify(original.state.meshes)===JSON.stringify(cameraAfter.state.meshes),'Camera controls changed mesh');await domInventory('runtime-camera');await screenshot('reference-camera');result.cases.push({name:'camera-projection-mode-fov',trials:cameraTrials,gates:{projection_controls:true,three_modes:true,fov_control:true,all_mesh_arrays_exact:true}});await writeResult();

  const primitiveTrials=[];for(const name of ['sphere','cube','cylinder','torus']){await clear();await button('_ctrlScene','Add '+name);for(let i=0;i<100;i++){if((await readState()).meshes.length===1)break;await pause(100);}await pause(300);const s=await readState();assert(s.meshes.length===1&&s.meshes[0].vertices>0&&s.meshes[0].faces_count>0,'Primitive failed '+name);primitiveTrials.push({name,state:s});await screenshot('primitive-'+name);}
  result.cases.push({name:'primitive-addition',trials:primitiveTrials,gates:{all_four_create:true,selected_created_object:true}});await writeResult();

  const imports=[];for(const f of formats){await clear();await importFile(f.path,f.expected);const snap=await snapshot(path.parse(f.name).name+'-imported');const mesh=snap.state.meshes[0];assert(mesh.positions.every(Number.isFinite)&&mesh.normals.every(Number.isFinite),'Format nonfinite');imports.push({fixture:f,snapshot:snap.record,counts:{vertices:mesh.vertices,faces:mesh.faces_count,triangles:mesh.triangles},finite:true});}
  result.cases.push({name:'ascii-binary-ply-stl-import',trials:imports,gates:{all_four_load_expected_counts:true,all_finite:true}});await writeResult();
  await clear();await importFile(quadObj.path,quadObj.expected);const quadSource=await snapshot('uv-quad-source'),quadSgl=await exportFile('Save .sgl (SculptGL)','uv-quad-reference-sgl');result.uv_quad_sgl={source:quadSource.record,export:quadSgl.item};await writeResult();

  await clear();await importFile(sphere.path,sphere.expected);const rich=await snapshot('rich-interchange-source');const exports=[];
  for(const [buttonLabel,name] of [['Save .obj','rich-reference-obj'],['Save .ply','rich-reference-ply'],['Save .stl','rich-reference-stl'],['Save .sgl (SculptGL)','rich-reference-sgl']])exports.push((await exportFile(buttonLabel,name)).item);
  const saved=exports.find(e=>e.name.endsWith('.sgl'));await clear();await importFile(saved.path,{vertices:266,faces:528,triangles:528});const loaded=await snapshot('rich-sgl-reloaded');const a=rich.state.meshes[0],b=loaded.state.meshes[0];
  assert(a.vertices===b.vertices&&a.faces_count===b.faces_count&&b.has_uv,'SGL lost expected mesh/UV');
  result.cases.push({name:'obj-ply-stl-sgl-exports-and-sgl-reload',exports,source:rich.record,reloaded:loaded.record,gates:{actual_downloads:exports.length===4,sgl_version3:(await readFile(saved.path)).readUInt32LE(0)===3,rich_uv_retained:b.has_uv,counts_retained:true,no_export_geometry_mutation:true},differences:{positions_exact:JSON.stringify(a.positions)===JSON.stringify(b.positions),colors_exact:JSON.stringify(a.colors)===JSON.stringify(b.colors),materials_exact:JSON.stringify(a.materials)===JSON.stringify(b.materials),faces_exact:JSON.stringify(a.faces)===JSON.stringify(b.faces),uv_exact:JSON.stringify(a.uvs)===JSON.stringify(b.uvs),face_uv_exact:JSON.stringify(a.face_uvs)===JSON.stringify(b.face_uvs),matrix_exact:JSON.stringify(a.matrix)===JSON.stringify(b.matrix)}});
  if(!skipMixed){await clear();await importFile(sphere.path,sphere.expected);await importFile(quadObj.path,quadObj.expected);await button('_ctrlScene','Add cube');await pause(350);const mixed=await snapshot('mixed-nonidentity-source'),mixedSgl=await exportFile('Save .sgl (SculptGL)','mixed-nonidentity-reference-sgl');assert(mixed.state.meshes.length===3&&mixed.state.meshes.some(m=>m.matrix.some((x,i)=>x!==(i%5===0?1:0))),'Mixed reference fixture lacks expected nonidentity primitive matrix');result.mixed_sgl={source:mixed.record,export:mixedSgl.item};await writeResult();}
  result.final_diagnostics=await evaluate('window.__sculptglReferenceEvents');await domInventory('runtime-final');await screenshot('reference-final');
  assert(!result.exceptions.length&&!result.final_diagnostics.errors.length,'Reference runtime JavaScript exception');assert(result.final_diagnostics.files.length>=10&&result.final_diagnostics.files.every(e=>e.trusted),'File imports not trusted production route');assert(result.final_diagnostics.input.filter(e=>e.type==='mousedown'||e.type==='keydown').every(e=>e.trusted),'Inputs not trusted');
  result.download_events=downloadEvents;result.failed_case_gates=result.cases.flatMap(c=>Object.entries(c.gates||{}).filter(([,pass])=>pass===false).map(([gate])=>({case:c.name,gate})));result.status=result.failed_case_gates.length?'completed reference characterization with failed exact gates':'passed reference-only initial workflows';result.limits=['No OBrush matched runtime or numerical/visual/performance parity result.','Only named initial brushes/options, camera selections/FOV, four primitives, four ordinary file imports and real exports/SGLv3 reload tested.','No full masks/extraction/dynamic/multires/UV/PBR/material/language/shortcut/endpoint matrix.','No native/physical tablet/touch verification or timed performance benchmark.','Reference mirror origin/telemetry/capture/font differences remain documented; kernel bytes unchanged.','Exact data snapshots expose source behavior and losses for later comparisons; no tolerance was widened to imply parity.'];
  }
}catch(error){result.status='failed reference runtime';result.failure=String(error.stack||error);try{await screenshot('failure');await snapshot('failure-state');await domInventory('failure-dom');result.failure_diagnostics=await evaluate('window.__sculptglReferenceEvents');}catch{};process.exitCode=1;}
finally{
  await writeResult();await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);
  try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}
  for(const p of pending.values()){clearTimeout(p.timer);p.reject(Error('Owned reference session closing'));}pending.clear();if(child&&child.exitCode===null&&child.signalCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),pause(2000)]);if(child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await Promise.race([new Promise(resolve=>child.once('exit',resolve)),pause(2000)]);}}if(server)await new Promise(resolve=>server.close(resolve));
  result.finished_at=new Date().toISOString();result.cleanup={owned_chrome_exit_code:child?.exitCode,owned_chrome_signal:child?.signalCode,owned_server_closed:!!server};await writeResult();console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),cases:result.cases.map(c=>({name:c.name,gates:c.gates}))}));
}
