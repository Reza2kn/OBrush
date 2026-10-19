// Paired normal-browser default Add Cylinder/Torus geometry audit.
// Usage: SGL_REFERENCE_DIR=/absolute/host-mirror node scripts/check-sculptgl-cylinder-torus-shape.mjs /absolute/web/dist
// The run retains a JSON receipt and never modifies the reference bundle.
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const obrush = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.env.SGL_REFERENCE_DIR || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-cylinder-torus-shape/' + randomUUID());
const downloads = path.join(out, 'downloads');
await mkdir(downloads, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const gpuHelper = await readFile('scripts/check-subtools.mjs', 'utf8');
const gpuInstrument = gpuHelper.slice(gpuHelper.indexOf('const instrument = `') + 20, gpuHelper.indexOf('\n\nconst pointer =')).slice(0, -2);
if (!gpuInstrument.startsWith('(()=>{') || !gpuInstrument.endsWith('})();')) throw Error('GPU observer source boundary changed');
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
if (sha(bundle) !== manifest.original_bundle.sha256) throw Error('Pinned reference bundle hash changed');
const result = {
  scope: 'SGL-SCENE-CYLINDER and SGL-SCENE-TORUS default normal-browser Add geometry, local/world bounds, normals, manifold and cap/tube orientation',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(obrush, 'pkg/obrush_app_bg.wasm'))) },
  cases: [], errors: [], status: 'running',
};
let server, chrome, socket, session, next = 0;
const waiting = new Map(), downloadEvents = [];
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const response = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 45000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return response;
}
async function evalRead(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn, reason) {
  for (let i = 0; i < 200; i++) { const found = await fn(); if (found) return found; await pause(100); }
  throw Error(`Timed out: ${reason}`);
}
async function clickRect(rect) {
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}
async function control(key) {
  for (let i = 0; i < 25; i++) {
    const info = await evalRead(obrushInfo);
    const rect = info.scene_controls?.[key];
    if (!rect) throw Error(`Missing ordinary control ${key}`);
    const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
    if (y > 72 && y < 940) return rect;
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: Math.max(1390, Math.min(x, 1550)), y: 740 });
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: Math.max(1390, Math.min(x, 1550)), y: 740, deltaX: 0, deltaY: y >= 940 ? 350 : -350 });
    await pause(150);
  }
  throw Error(`Could not scroll to ${key}`);
}
async function action(key) { await clickRect(await control(key)); await pause(180); return evalRead(obrushInfo); }
const referenceInfo = `(()=>{const a=window.__sculptglReferenceInstance,m=a?.getMesh();return m?{vertices:m.getNbVertices(),faces:m.getNbFaces(),triangles:m.getNbTriangles(),matrix:Array.from(m.getMatrix()),levels:m._meshes?.length??1}:null})()`;
async function referenceMesh(name) {
  const meta=await evalRead(referenceInfo), data={...meta,positions:[],normals:[],face_indices:[]};
  const batch=1024;
  for(let start=0;start<meta.vertices;start+=batch){
    result.stage=`reference extract ${name} vertices ${start}/${meta.vertices}`;
    const chunk=await evalRead(`(()=>{const m=window.__sculptglReferenceInstance.getMesh(),a=${start*3},b=${Math.min((start+batch)*3,meta.vertices*3)};return {positions:Array.from(m.getVertices().subarray(a,b)),normals:Array.from(m.getNormals().subarray(a,b))}})()`);
    data.positions.push(...chunk.positions);data.normals.push(...chunk.normals);
  }
  for(let start=0;start<meta.faces;start+=batch){
    result.stage=`reference extract ${name} faces ${start}/${meta.faces}`;
    const chunk=await evalRead(`(()=>{const m=window.__sculptglReferenceInstance.getMesh(),a=${start*4},b=${Math.min((start+batch)*4,meta.faces*4)};return Array.from(m.getFaces().subarray(a,b))})()`);
    data.face_indices.push(...chunk);
  }
  return data;
}
const referenceCount = `(()=>{const a=window.__sculptglReferenceInstance,m=a?.getMesh();return a?{parts:a.getMeshes().length,vertices:m?.getNbVertices()??0,faces:m?.getNbFaces()??0}:null})()`;
const obrushInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{vertices:m.vertices,faces:m.faces,triangles:m.triangles,scene_controls:m.scene_controls,subtools:m.subtools?.length,history:m.history,message:m.message}:null})()`;
function floatHash(positions) { const bytes = Buffer.alloc(positions.length * 4); positions.forEach((v, i) => bytes.writeFloatLE(v, i * 4)); return sha(bytes); }
function quantile(values, fraction) { const a = values.slice().sort((x,y)=>x-y); return a[Math.floor((a.length - 1) * fraction)]; }
function shape(positions, normals, faces, transform) {
  const p = [], n = positions.length / 3, bounds = [[Infinity,Infinity,Infinity],[-Infinity,-Infinity,-Infinity]], sums = [0,0,0], lengths = [], radii = [], absolute = [[],[],[]], axialCapNormals=[];
  for (let i = 0; i < n; i++) {
    const v = positions.slice(i*3,i*3+3);
    const q = transform ? [0,1,2].map(k=>transform[k]*v[0]+transform[k+4]*v[1]+transform[k+8]*v[2]+transform[k+12]) : v;
    p.push(q);
    for (let k = 0; k < 3; k++) { bounds[0][k]=Math.min(bounds[0][k],q[k]); bounds[1][k]=Math.max(bounds[1][k],q[k]); sums[k]+=q[k]; absolute[k].push(Math.abs(q[k])); }
    radii.push(Math.hypot(q[0],q[2]));
    const normal=normals.slice(i*3,i*3+3),length=Math.hypot(...normal);
    lengths.push(length);
    if(Math.abs(v[1])>0.99&&Math.hypot(v[0],v[2])<0.2)axialCapNormals.push(normal[1]*Math.sign(v[1])/length);
  }
  const edges = new Map(); let triangles=0, quads=0, tubeOut=0, tubeIn=0;
  for (const face of faces) {
    const indices=face.filter(x=>x!==4294967295 && x!==null);
    if(indices.length===3)triangles++; else if(indices.length===4)quads++;
    for(let j=0;j<indices.length;j++){const a=indices[j],b=indices[(j+1)%indices.length],key=`${Math.min(a,b)},${Math.max(a,b)}`,old=edges.get(key)||[0,0];old[0]++;old[1]+=a<b?1:-1;edges.set(key,old);}
    const a=p[indices[0]],b=p[indices[1]],c=p[indices[2]];
    const u=b.map((x,k)=>x-a[k]),v=c.map((x,k)=>x-a[k]),cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
    const centroid=[0,1,2].map(k=>indices.reduce((s,id)=>s+p[id][k],0)/indices.length);
    const theta=Math.atan2(centroid[2],centroid[0]),center=[0.5*Math.cos(theta),0,0.5*Math.sin(theta)],radial=centroid.map((x,k)=>x-center[k]);
    const dot=cross.reduce((s,x,k)=>s+x*radial[k],0);
    if(dot>1e-10)tubeOut++;else if(dot < -1e-10)tubeIn++;
  }
  const nonmanifold=[...edges.values()].filter(([count,orientation])=>count!==2||orientation!==0).length;
  return { vertices:n, original_faces:faces.length, triangles, quads, position_float32_sha256:floatHash(positions), normal_float32_sha256:floatHash(normals), bounds, centroid:sums.map(x=>x/n), abs_quantiles:absolute.map(a=>[0.5,0.95,1].map(q=>quantile(a,q))), radial_quantiles:[0,0.05,0.5,0.95,1].map(q=>quantile(radii,q)), normal_length:[Math.min(...lengths),quantile(lengths,0.5),Math.max(...lengths)], unique_edges:edges.size, nonmanifold_edges:nonmanifold, euler:n-edges.size+faces.length, near_axis_cap_normal_y:{points:axialCapNormals.length,min:axialCapNormals.length?Math.min(...axialCapNormals):null,mean:axialCapNormals.length?axialCapNormals.reduce((s,x)=>s+x,0)/axialCapNormals.length:null}, torus_face_radial_winding:{out:tubeOut,in:tubeIn} };
}
function compare(a,b) {
  return {vertex_delta:b.vertices-a.vertices,face_delta:b.original_faces-a.original_faces,bounds_abs_diff:a.bounds.map((arr,i)=>arr.map((x,k)=>Math.abs(x-b.bounds[i][k]))),centroid_abs_diff:a.centroid.map((x,k)=>Math.abs(x-b.centroid[k])),abs_quantile_diff:a.abs_quantiles.map((arr,k)=>arr.map((x,j)=>Math.abs(x-b.abs_quantiles[k][j]))),radial_quantile_diff:a.radial_quantiles.map((x,j)=>Math.abs(x-b.radial_quantiles[j]))};
}
function originalFaceAreaNormals(positions, faces) {
  const normals = new Float64Array(positions.length);
  const point = id => positions.slice(id * 3, id * 3 + 3);
  const sub = (a,b) => a.map((x,i)=>x-b[i]);
  const cross = (a,b) => [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  for(const face of faces){
    const ids=face.filter(x=>x!==4294967295 && x!==null),[a,b,c]=ids.map(point);
    const edge=sub(c,a),normal=cross(sub(b,a),edge);
    if(ids.length===4){const other=cross(sub(c,point(ids[3])),edge);for(let k=0;k<3;k++)normal[k]+=other[k];}
    for(const id of ids)for(let k=0;k<3;k++)normals[id*3+k]+=normal[k];
  }
  return Array.from(normals);
}
function normalAngle(a,b) {
  return Math.acos(Math.max(-1,Math.min(1,a.reduce((s,v,i)=>s+v*b[i],0)/(Math.hypot(...a)*Math.hypot(...b)))))*180/Math.PI;
}
async function gpuNormal(id) {
  return evalRead(`(()=>{const g=window.__brushGl,p=window.__brushCheck,d=p?.scene_frame?.draws?.find(x=>x.active);if(!g||!d)return null;const old=g.__sceneArrayBuffer,b=new Float32Array(3);g.bindBuffer(g.ARRAY_BUFFER,d.vertices);g.getBufferSubData(g.ARRAY_BUFFER,${id}*40+12,b);g.bindBuffer(g.ARRAY_BUFFER,old);return {normal:Array.from(b),vertex_bytes:g.__sceneBuffers.get(d.vertices)?.bytes,draw_count:d.count}})()`);
}
function pairedVertices(aPositions,aNormals,bPositions,bNormals) {
  const cell=1e-5,tolerance=1e-6,key=(x,y,z)=>`${x},${y},${z}`;
  const reference=new Map(),used=new Uint8Array(aPositions.length/3);
  for(let i=0;i<aPositions.length/3;i++){
    const x=Math.floor(aPositions[i*3]/cell),y=Math.floor(aPositions[i*3+1]/cell),z=Math.floor(aPositions[i*3+2]/cell),k=key(x,y,z),bucket=reference.get(k)||[];
    bucket.push(i);reference.set(k,bucket);
  }
  let paired=0,unmatched=0,maximumAngle=0,angleSum=0,maxDistance=0,distanceSum=0,worst=null;
  for(let i=0;i<bPositions.length/3;i++){
    const x=Math.floor(bPositions[i*3]/cell),y=Math.floor(bPositions[i*3+1]/cell),z=Math.floor(bPositions[i*3+2]/cell);
    let best=-1,bestDistance=Infinity;
    for(let dx=-1;dx<=1;dx++)for(let dy=-1;dy<=1;dy++)for(let dz=-1;dz<=1;dz++){
      for(const id of reference.get(key(x+dx,y+dy,z+dz))||[]){
        if(used[id])continue;
        const d=Math.hypot(...[0,1,2].map(k=>aPositions[id*3+k]-bPositions[i*3+k]));
        if(d<bestDistance){best=id;bestDistance=d;}
      }
    }
    if(best<0||bestDistance>tolerance){unmatched++;continue;}
    used[best]=1;paired++;distanceSum+=bestDistance;maxDistance=Math.max(maxDistance,bestDistance);
    const a=aNormals.slice(best*3,best*3+3),b=bNormals.slice(i*3,i*3+3),aLength=Math.hypot(...a),bLength=Math.hypot(...b);
    const dot=a.reduce((s,v,j)=>s+v*b[j],0)/(aLength*bLength);
    const angle=Math.acos(Math.max(-1,Math.min(1,dot)))*180/Math.PI;
    if(angle>maximumAngle){maximumAngle=angle;worst={reference_id:best,obrush_id:i,position:bPositions.slice(i*3,i*3+3),reference_normal:a,obrush_normal:b,angle_degrees:angle};}
    angleSum+=angle;
  }
  return {cell_width:cell,maximum_match_distance:tolerance,paired,unmatched_obrush:unmatched,unused_reference:used.length-paired,position_mean_distance:distanceSum/paired,position_max_distance:maxDistance,normal_direction_mean_degrees:angleSum/paired,normal_direction_max_degrees:maximumAngle,worst};
}
let primaryError;
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const isReference = pathname.startsWith('/reference/');
      const base = isReference ? reference : obrush;
      const relative = isReference ? pathname.slice('/reference'.length) : pathname;
      const file = path.resolve(base, '.' + (relative === '/' ? '/index.html' : relative));
      if (!file.startsWith(base + path.sep)) throw Error('outside served tree');
      const bytes = await readFile(file);
      response.setHeader('Content-Type', ({ '.wasm':'application/wasm', '.js':'text/javascript', '.html':'text/html', '.css':'text/css' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new',`--user-data-dir=${path.join(out,'chrome-profile')}`,'--remote-debugging-port=0','--no-first-run','--disable-background-networking','--window-size=1600,1000','about:blank'], { stdio:['ignore','pipe','pipe'] });
  let stderr=''; chrome.stderr.on('data', bytes=>{stderr+=bytes;});
  const endpoint=await until(async()=>stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1],'Chrome endpoint');
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message', event=>{
    const message=JSON.parse(String(event.data));
    if(message.id){const pending=waiting.get(message.id);if(!pending)return;waiting.delete(message.id);clearTimeout(pending.timer);message.error?pending.reject(Error(JSON.stringify(message.error))):pending.resolve(message.result);}
    else if(message.method==='Runtime.exceptionThrown')result.errors.push(message.params.exceptionDetails.text);
    else if(message.method==='Browser.downloadWillBegin'||message.method==='Browser.downloadProgress')downloadEvents.push({method:message.method,...message.params});
  });
  const target=await send('Target.createTarget',{url:'about:blank'},null);
  session=(await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
  for(const domain of ['Page','Runtime','DOM'])await send(`${domain}.enable`);
  await send('Emulation.setDeviceMetricsOverride',{width:1600,height:1000,deviceScaleFactor:1,mobile:false});
  await send('Browser.setDownloadBehavior',{behavior:'allowAndName',downloadPath:downloads,eventsEnabled:true},null);
  const base=`http://127.0.0.1:${server.address().port}`;
  result.stage='reference navigation';
  await send('Page.navigate',{url:`${base}/reference/`});
  await until(async()=>(await evalRead(referenceCount))?.parts,'pinned reference startup');
  await evalRead('(()=>{window.confirm=()=>true;window.__sculptglReferenceInstance.getGui()._ctrlScene.clearScene();return true})()');
  await until(async()=>(await evalRead(referenceCount))?.parts===0,'reference initial clear');
  const refData={},refRaw={};
  for(const [name,method] of [['cylinder','addCylinder'],['torus','addTorus']]){
    result.stage=`reference Add ${name}`;
    await evalRead(`(()=>{window.__sculptglReferenceInstance.${method}();return true})()`);
    await until(async()=>(await evalRead(referenceCount))?.vertices>60000,`reference Add ${name}`);
    result.stage=`reference extract ${name}`;
    const mesh=await referenceMesh(name);
    refRaw[name]={positions:mesh.positions,normals:mesh.normals};
    result.stage=`reference analyze ${name}`;
    refData[name]={local:shape(mesh.positions,mesh.normals,Array.from({length:mesh.faces},(_,i)=>mesh.face_indices.slice(i*4,i*4+4))),world:shape(mesh.positions,mesh.normals,Array.from({length:mesh.faces},(_,i)=>mesh.face_indices.slice(i*4,i*4+4)),mesh.matrix),matrix:mesh.matrix,levels:mesh.levels,reported:{vertices:mesh.vertices,faces:mesh.faces,triangles:mesh.triangles}};
    await evalRead('(()=>{window.confirm=()=>true;window.__sculptglReferenceInstance.getGui()._ctrlScene.clearScene();return true})()');
    await until(async()=>(await evalRead(referenceCount))?.parts===0,'reference reset');
  }
  result.stage='OBRush navigation';
  await send('Page.addScriptToEvaluateOnNewDocument',{source:gpuInstrument});
  await send('Page.navigate',{url:`${base}/?obrushprofile=sculptgl`});
  await until(async()=>(await evalRead(obrushInfo))?.subtools===1,'OBRush startup');
  for(const name of ['cylinder','torus']){
    result.stage=`OBRush Add ${name}`;
    await action('scene.clear');
    await until(async()=>(await evalRead(obrushInfo))?.subtools===0,'OBRush empty before Add');
    await action(`scene.add_${name}`);
    const ui=await until(async()=>{const m=await evalRead(obrushInfo);return m?.subtools===1&&m.vertices>60000?m:null;},`OBRush Add ${name}`);
    const before=downloadEvents.filter(e=>e.method==='Browser.downloadProgress'&&e.state==='completed').length;
    result.stage=`OBRush Save ${name}`;
    await clickRect([303,13,387,39]);
    const saved=await until(async()=>{
      const completed=downloadEvents.filter(e=>e.method==='Browser.downloadProgress'&&e.state==='completed');
      if(completed.length<=before)return null;
      const file=path.join(downloads,completed.at(-1).guid),bytes=await readFile(file);
      try{return {file,bytes,project:JSON.parse(bytes.toString('utf8'))};}catch{return null;}
    },`OBRush ${name} public Save`);
    const mesh=saved.project.mesh;
    result.stage=`OBRush analyze ${name}`;
    const faces=mesh.surface?.faces ?? mesh.triangles;
    const positions=mesh.positions.flat(),normals=mesh.normals.flat();
    const own=shape(positions,normals,faces);
    const areaNormals=originalFaceAreaNormals(positions,faces);
    const paired=pairedVertices(refRaw[name].positions,refRaw[name].normals,positions,normals);
    const gpu=await until(()=>gpuNormal(paired.worst.obrush_id),`OBRush ${name} GPU normal sample`);
    gpu.reference_direction_degrees=normalAngle(paired.worst.reference_normal,gpu.normal);
    gpu.saved_direction_degrees=normalAngle(paired.worst.obrush_normal,gpu.normal);
    result.cases.push({name,reference:refData[name],obrush:{ui,saved_file:saved.file,saved_sha256:sha(saved.bytes),shape:own,scene:saved.project.scene,gpu_normal_at_worst_saved_vertex:gpu},comparison_local:compare(refData[name].local,own),comparison_world:compare(refData[name].world,own),paired_vertices:paired,diagnostic_original_face_area_normals:pairedVertices(refRaw[name].positions,refRaw[name].normals,positions,areaNormals)});
  }
  const c=result.cases.find(x=>x.name==='cylinder'),t=result.cases.find(x=>x.name==='torus');
  if(!c||!t||result.cases.some(x=>x.reference.reported.vertices!==x.obrush.ui.vertices||x.reference.reported.faces!==x.obrush.ui.faces||x.reference.local.nonmanifold_edges!==0||x.obrush.shape.nonmanifold_edges!==0||x.paired_vertices.unmatched_obrush!==0||x.paired_vertices.unused_reference!==0||x.paired_vertices.position_max_distance>1e-6))throw Error('Geometry count/manifold/local-position gate failed');
  result.status='passed scoped browser extraction and manifold/count gates; shape deltas require interpretation';
} catch(error) { primaryError=error; result.status='failed'; result.errors.push(String(error.stack||error)); }
finally {
  result.download_events=downloadEvents;
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  try {if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}
  socket?.close();if(chrome?.exitCode===null)chrome.kill();server?.close();
}
console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),cases:result.cases.map(x=>({name:x.name,comparison_local:x.comparison_local,comparison_world:x.comparison_world})),errors:result.errors}));
if(primaryError)process.exitCode=1;
