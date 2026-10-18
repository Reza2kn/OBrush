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
const subdivideRemeshOnly = process.argv.includes('--subdivide-remesh-only');
const mcRemeshOnly = process.argv.includes('--mc-remesh-only');
const mcRemeshPlainOnly = process.argv.includes('--mc-remesh-plain-only');
const mcRemeshNoUvOnly = process.argv.includes('--mc-remesh-no-uv-only');
const mcRemeshFusionOnly = process.argv.includes('--mc-remesh-fusion-only');
const mcMixedUvFusionOnly = process.argv.includes('--mc-mixed-uv-fusion-only');
const mcSeamedMixedUvFusionOnly = process.argv.includes('--mc-seamed-mixed-uv-fusion-only');
const mcPosedFusionOnly = process.argv.includes('--mc-posed-fusion-only');
const surfaceNetsPosedFusionOnly = process.argv.includes('--surface-nets-posed-fusion-only');
const blockPosedFusionOnly = process.argv.includes('--block-posed-fusion-only');
const posedFusionOnly = mcPosedFusionOnly || surfaceNetsPosedFusionOnly || blockPosedFusionOnly;
const surfaceNetsMixedUvFusionOnly = process.argv.includes('--surface-nets-mixed-uv-fusion-only');
const blockMixedUvFusionOnly = process.argv.includes('--block-mixed-uv-fusion-only');
const surfaceNetsFusionOnly = process.argv.includes('--surface-nets-fusion-only');
const blockFusionOnly = process.argv.includes('--block-fusion-only');
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
const fusionCubeText=(name,shift,color)=>{
  const vertices=[[-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],[-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1]];
  const faces=[[1,3,2],[1,4,3],[5,6,7],[5,7,8],[1,2,6],[1,6,5],[4,8,7],[4,7,3],[1,5,8],[1,8,4],[2,3,7],[2,7,6]];
  return `o ${name}\n`+vertices.map(([x,y,z])=>`v ${x+shift} ${y} ${z}\n`).join('')+`#MRGB ${('ff'+color).repeat(8)}\n`+`#MAT ${'6699ff'.repeat(8)}\n`+faces.map(face=>`f ${face.join(' ')}\n`).join('');
};
const fusionLeft=await fixture('independent-fusion-left.obj',fusionCubeText('Independent_fusion_left',-.3,'ff0000'),{vertices:8,faces:12,triangles:12});
const fusionLeftUv=await fixture('independent-fusion-left-uv.obj',await readFile('crates/obrush-core/tests/fixtures/independent-fusion-left-uv.obj'),{vertices:8,faces:12,triangles:12,has_uv:true});
const fusionLeftSeamedMask=await fixture('independent-fusion-left-seamed-mask.obj',await readFile('crates/obrush-core/tests/fixtures/independent-fusion-left-seamed-mask.obj'),{vertices:8,faces:12,triangles:12,has_uv:true});
const posedFusion=await fixture('independent-posed-fusion.sgl',await readFile('crates/obrush-core/tests/fixtures/independent-posed-fusion.sgl'),{source_objects:2});
const fusionRight=await fixture('independent-fusion-right.obj',fusionCubeText('Independent_fusion_right',.3,'0000ff'),{vertices:8,faces:12,triangles:12});
const materialSphereNoUv=await fixture('independent-material-sphere-no-uv.obj',await readFile('crates/obrush-core/tests/fixtures/independent-reference-sphere.obj'),{vertices:266,faces:528,triangles:528,has_uv:false,has_nonuniform_rgb:true,has_nonuniform_roughness_metalness:true,unmasked_weight_zero_vertices:7});
const plainSphere=await fixture('independent-plain-sphere.obj',independent.text.split('\n').filter(line=>!line.startsWith('#MAT')&&!line.startsWith('#MRGB')).join('\n'),{vertices:266,faces:528,triangles:528,has_uv:true});
const formats=[await fixture('independent-quad-cube-ascii.ply',plyAscii,{vertices:8,faces:6,triangles:12}),await fixture('independent-quad-cube-binary.ply',Buffer.concat([head,plyBody]),{vertices:8,faces:6,triangles:12}),await fixture('independent-cube-ascii.stl',stlAscii,{vertices:8,faces:12,triangles:12}),await fixture('independent-cube-colored.stl',stlBinary,{vertices:8,faces:12,triangles:12})];
const hexByte=x=>Math.round(Math.max(0,Math.min(1,x))*255).toString(16).padStart(2,'0');
let quadText='o Independent_UV_quad_cube\n';cube.positions.forEach((p,i)=>quadText+='v '+p.join(' ')+' '+cube.colors[i].map(x=>x/255).join(' ')+'\n');quadText+='#MAT '+cube.positions.map((_,i)=>[.1+.1*i,.8-.1*i,i%3===0?0:1].map(hexByte).join('')).join('')+'\n';for(let face=0;face<6;face++)for(const uv of [[0,0],[1,0],[1,1],[0,1]])quadText+='vt '+uv.join(' ')+'\n';cube.faces.forEach((f,i)=>quadText+='f '+f.map((v,j)=>`${v+1}/${i*4+j+1}`).join(' ')+'\n');
const quadObj=await fixture('independent-seamed-quad-cube.obj',quadText,{vertices:8,faces:6,triangles:12,has_uv:true});
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
  contract:{scope:'Reference-only behavior/export observations; not OBrush parity',application_mutations:'Real CDP input into production DOM widgets, native file-input route and real file-saver downloads only. JavaScript evaluates read-only diagnostics; no assignment/calls to tool/mesh/camera setters.',
    arithmetic:'Exact saved f32/i32/u32 arrays and export bytes for future comparisons; no response tolerance or reference equivalence inferred.',brushes:['Brush','Pinch','Twist','LocalScale','Drag'],camera:['Orbit','Spherical','Plane','Perspective','Orthographic'],primitives:['Sphere','Cube','Cylinder','Torus'],file_formats:['OBJ','PLY ASCII/LE binary','STL ASCII/color binary','SGL version3 roundtrip'],
    timings:'No performance benchmark. Short waits only ensure input/render/export completion; no latency, polycount-capacity or physical-input claim.'}};
const writeResult=()=>writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
await writeResult();
if(prepareOnly){console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),harness_sha256:result.harness_sha256,fixtures:fixtures.map(f=>({path:f.path,sha256:f.sha256}))}));process.exit(0);}

let child,socket,session,sequence=0,stderr='',stdout='',server;
const pending=new Map(),downloadEvents=[];
const send=(method,params={},target=session)=>new Promise((resolve,reject)=>{
  const id=++sequence,trace={id,method,at:new Date().toISOString(),...(params.expression?{expression_sha256:hash(params.expression),expression_start:params.expression.slice(0,180)}:{})};result.cdp_trace.push(trace);
  // The reference's synchronous Catmull subdivision can occupy its main
  // thread beyond the ordinary diagnostic timeout at this voxel density.
  const timer=setTimeout(()=>{pending.delete(id);trace.outcome='timeout';reject(Error('CDP timeout '+method+' request '+id));},120000);pending.set(id,{resolve,reject,timer,trace});socket.send(JSON.stringify({id,method,params,...(target?{sessionId:target}:{})}));
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
  if (mcRemeshFusionOnly || mcMixedUvFusionOnly || mcSeamedMixedUvFusionOnly || posedFusionOnly || surfaceNetsFusionOnly || surfaceNetsMixedUvFusionOnly || blockFusionOnly || blockMixedUvFusionOnly) {
    const mixedUv = mcMixedUvFusionOnly || mcSeamedMixedUvFusionOnly || surfaceNetsMixedUvFusionOnly || blockMixedUvFusionOnly;
    const fusionMode = mcRemeshFusionOnly || mcMixedUvFusionOnly || mcSeamedMixedUvFusionOnly || mcPosedFusionOnly ? 'mc' : surfaceNetsFusionOnly || surfaceNetsMixedUvFusionOnly || surfaceNetsPosedFusionOnly ? 'nets' : 'block';
    for(const relaxed of fusionMode==='mc'?[false,true]:[false]){
      const left=mcSeamedMixedUvFusionOnly?fusionLeftSeamedMask:mixedUv?fusionLeftUv:fusionLeft;
      await clear();
      if(posedFusionOnly)await importFile(posedFusion.path);
      else {await importFile(left.path,left.expected);await importFile(fusionRight.path,fusionRight.expected);}
      const selected=await evaluate(`(()=>{const m=${app};m.setOrUnsetMesh(m.getMeshes()[0],true);return m.getSelectedMeshes().map(x=>x.getID());})()`);
      assert(selected.length===2,'Reference selected-fusion setup did not select both imported solids');
      await menu('_ctrlTopology');const topology=`${controller('_ctrlTopology')}`;
      const numberInputs=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=number]')).filter(e=>e.min==='8')`;
      assert(await evaluate(`(${numberInputs}).length`)===2,'Expected separate remesh resolution controls');
      await number(`(${numberInputs})[${fusionMode==='mc'?1:0}]`,16);
      if(fusionMode==='mc'){
        const relaxInput=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Relax topology')`;
        await checkbox(relaxInput,relaxed);
      } else {
        const blockInput=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Block')`;
        await checkbox(blockInput,fusionMode==='block');
      }
      const before=await snapshot(fusionMode+(posedFusionOnly?'-posed':mcSeamedMixedUvFusionOnly?'-seamed-mixed-uv':mixedUv?'-mixed-uv':'')+'-fusion-'+relaxed+'-before');
      assert(before.state.selected_ids.length===2&&before.state.meshes.length===2,'Reference pre-remesh fusion state is not two selected meshes');
      const remeshButton=`Array.from(${topology}._menu.domUl.querySelectorAll('button')).filter(e=>e.textContent.trim()==='Remesh')[${fusionMode==='mc'?1:0}]`;
      await click(remeshButton);
      let afterState;for(let i=0;i<300;i++){afterState=await readState();if(afterState.meshes.length===1&&afterState.meshes[0].vertices>8)break;await pause(100);}
      assert(afterState.meshes.length===1&&afterState.meshes[0].vertices>8,'Reference selected fusion did not replace the two inputs');
      const after=await snapshot(fusionMode+(posedFusionOnly?'-posed':mcSeamedMixedUvFusionOnly?'-seamed-mixed-uv':mixedUv?'-mixed-uv':'')+'-fusion-'+relaxed+'-after');const output=after.state.meshes[0];
      result.cases.push({name:'reference-'+fusionMode+(posedFusionOnly?'-posed':mcSeamedMixedUvFusionOnly?'-seamed-mixed-uv':mixedUv?'-mixed-uv':'')+'-fusion-relaxed-'+relaxed,resolution:16,relaxed,before:before.record,after:after.record,output:{vertices:output.vertices,faces:output.faces_count,triangles:output.triangles},gates:{two_selected_sources:before.state.selected_ids.length===2,one_output:after.state.meshes.length===1,valid_faces:output.faces.every((v,i)=>i%4===3?(fusionMode==='mc'?v===4294967295:Number.isInteger(v)&&v!==4294967295):Number.isInteger(v))}});
      await writeResult();
    }
    result.status='passed reference selected-fusion '+fusionMode+' workflows';
  } else if (mcRemeshOnly || mcRemeshPlainOnly || mcRemeshNoUvOnly) {
    for (const relaxed of [false, true]) {
      await clear();
      const source = mcRemeshPlainOnly ? plainSphere : mcRemeshNoUvOnly ? materialSphereNoUv : sphere;
      await importFile(source.path, source.expected);
      await menu('_ctrlTopology');
      const topology = `${controller('_ctrlTopology')}`;
      const numberInputs = `Array.from(${topology}._menu.domUl.querySelectorAll('input[type=number]')).filter(e=>e.min==='8')`;
      assert(await evaluate(`(${numberInputs}).length`)===2, 'Expected separate Surface Nets and Marching Cubes resolution controls');
      await number(`(${numberInputs})[1]`,16);
      const relaxInput = `Array.from(${topology}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Relax topology')`;
      await checkbox(relaxInput,relaxed);
      const before=await snapshot('mc-remesh-'+relaxed+'-before');
      const mcButton=`Array.from(${topology}._menu.domUl.querySelectorAll('button')).filter(e=>e.textContent.trim()==='Remesh')[1]`;
      await click(mcButton);
      let afterState;
      for(let i=0;i<300;i++){afterState=await readState();if(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices)break;await pause(100);}
      assert(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices,'Reference Marching Cubes remesh did not change the selected geometry');
      const after=await snapshot('mc-remesh-'+relaxed+'-after');
      const output=after.state.meshes.find(m=>m.id===after.state.active_id);
      result.cases.push({name:'reference-mc-remesh-relaxed-'+relaxed,resolution:16,relaxed,before:before.record,after:after.record,output:{vertices:output.vertices,faces:output.faces_count,triangles:output.triangles,positions:output.positions,faces_array:output.faces},gates:{active_geometry_changed:output.vertices!==before.state.meshes[0].vertices,nonempty_output:output.vertices>0&&output.triangles>0,all_triangles:output.faces.every((v,i)=>i%4===3?v===4294967295:Number.isInteger(v))}});
      await writeResult();
    }
    result.status='passed reference-only Marching Cubes workflows';
    result.limits=['One independently authored closed sphere at resolution 16, both Relax topology values','No OBrush pairing or general-shape/performance parity in this reference-only run'];
  } else if (blockRemeshOnly || subdivideRemeshOnly) {
    for(const block of subdivideRemeshOnly?[false]:[false,true]){
      await clear();await importFile(sphere.path,sphere.expected);await menu('_ctrlTopology');
      const topology=`${controller('_ctrlTopology')}`;
      const resolution=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=number]')).find(e=>e.min==='8')`;
      const chosenResolution=subdivideRemeshOnly?16:32;
      await number(resolution,chosenResolution);
      const blockInput=`Array.from(${topology}._menu.domUl.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.trim()==='Block')`;
      await checkbox(blockInput,block);
      const before=await snapshot('block-remesh-'+block+'-before');
      await button('_ctrlTopology','Remesh');
      let afterState;for(let i=0;i<300;i++){afterState=await readState();if(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices)break;await pause(100);}
      assert(afterState.meshes.find(m=>m.id===afterState.active_id)?.vertices!==before.state.meshes[0].vertices,'Reference block remesh did not change the selected geometry');
      const after=await snapshot('block-remesh-'+block+'-after');
      const output=after.state.meshes.find(m=>m.id===after.state.active_id);
      result.cases.push({name:'reference-remesh-block-'+block,resolution:chosenResolution,block,before:before.record,after:after.record,output:{vertices:output.vertices,faces:output.faces_count,triangles:output.triangles,positions:output.positions,faces_array:output.faces},gates:{active_geometry_changed:output.vertices!==before.state.meshes[0].vertices,nonempty_output:output.vertices>0&&output.triangles>0}});await writeResult();
      if(subdivideRemeshOnly){
        await button('_ctrlTopology','Subdivide');
        let subdividedState;for(let i=0;i<300;i++){subdividedState=await readState();if(subdividedState.meshes.find(m=>m.id===subdividedState.active_id)?.level_count===2)break;await pause(100);}
        const subdivided=await snapshot('smooth-surface-nets-subdivided');
        const child=subdivided.state.meshes.find(m=>m.id===subdivided.state.active_id);
        result.cases.push({name:'reference-smooth-surface-nets-subdivision',resolution:chosenResolution,before:after.record,after:subdivided.record,output:{vertices:child.vertices,faces:child.faces_count,triangles:child.triangles,level:child.level,level_count:child.level_count},gates:{quad_child_count:child.faces_count===output.faces_count*4,retained_level:child.level_count===2&&child.level===1,all_positions_finite:child.positions.every(Number.isFinite),all_faces_valid:child.faces.every(Number.isInteger)}});await writeResult();
      }
    }
    result.status=subdivideRemeshOnly?'passed reference-only Smooth Surface Nets subdivision workflow':'passed reference-only block remesh workflows';
    result.limits=subdivideRemeshOnly?['One independently authored closed sphere, 16-resolution Smooth Surface Nets followed by one visible Subdivide action','No OBrush pairing or general-shape/performance parity in this reference-only run']:['One independently authored closed sphere, 32 resolution, normal and Block Surface Nets buttons','No OBrush pairing or general-shape/performance parity'];
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
