// CLI-only Move geometry and interaction regression checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-move.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'move';
const out = path.resolve(`.artifacts/move-regression/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
await mkdir(profile, { recursive: true });
await writeFile(path.join(out,'harness.mjs'),await readFile(new URL(import.meta.url)));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

const fixture = path.resolve(".artifacts/user-long-stroke-0.1.4/user.obrush");
const selectedCases = process.env.OBRUSH_MOVE_CASES?.split(',');
const fixtureBytes = !selectedCases || selectedCases.includes('user-crossing') ? await readFile(fixture) : null;
if(fixtureBytes && hash(fixtureBytes)!=="4adc8aa9333903ab0684c1f2e54a995dc6a4e7a3b85fc6e911df9d7deb0c4786") throw Error("Immutable user fixture changed");
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) { response.writeHead(403); response.end(); return; }
    const bytes = await readFile(file);
    response.setHeader('Cache-Control','no-store');
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    response.end(bytes);
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const chrome = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const args = ['--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-breakpad', '--metrics-recording-only', '--safebrowsing-disable-auto-update', '--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication', '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1', '--window-size=1600,1000', 'about:blank'];
const child = spawn(chrome, args, { stdio: ['ignore', 'pipe', 'pipe'] });
let stderr = '', stdout = ''; child.stderr.on('data', bytes => stderr += bytes); child.stdout.on('data', bytes => stdout += bytes);
let socket, sequence = 0, session;
const pending = new Map();
const result = {
  label, status: 'running', method: 'Isolated CLI headless Chrome/CDP; no desktop, CUA, accessibility, user-tab, or physical device control',
  out, url, web, profile, owned_pid: child.pid, fixture: fixtureBytes ? {path:fixture,sha256:hash(fixtureBytes),bytes:fixtureBytes.length} : null,
  harness_sha256: hash(await readFile(new URL(import.meta.url))),
  preregistered_contract: {radius:0.037,strength:0.79,symmetry_x:true,spacing_ms:30,release_probe_incremental_delays_ms:[0,200,800],outbound_min_useful_original_displacement:.001,geometry:'Measure quality and folds without asserting reference equivalence; baseline failures are retained',history:'Exact mesh fields and topology',gpu:'Exact rendered VBO versus saved mesh float32'},
  protocol_reference: 'https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-dispatchMouseEvent',
  cases: [], comparisons: {}, exceptions: [], console_errors: [], blocked_external_requests: [], input_trace: [],
};
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 30000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw new Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
};
const instrument = `(()=>{
 const p=window.__brushCheck={events:[],keys:[],raf:[],longtasks:[],uploads:[],contexts:[],allocations:[],viewports:{},gl_errors:[],js_errors:[]};
 new PerformanceObserver(list=>{for(const task of list.getEntries())p.longtasks.push({at:task.startTime,duration:task.duration});}).observe({entryTypes:['longtask']});
 const raf=window.requestAnimationFrame.bind(window);let last;raf(function tick(at){p.raf.push({at,gap:last===undefined?0:at-last});last=at;raf(tick);});
 for(const type of ['keydown','keyup'])window.addEventListener(type,event=>p.keys.push({type,key:event.key,meta:event.metaKey,ctrl:event.ctrlKey,shift:event.shiftKey,trusted:event.isTrusted,target:event.target.id}),true);
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,trusted:event.isTrusted,at:performance.now()}),true);
 window.addEventListener('error',event=>p.js_errors.push(String(event.error||event.message)));
 window.addEventListener('unhandledrejection',event=>p.js_errors.push(String(event.reason)));
 const get=HTMLCanvasElement.prototype.getContext;
 HTMLCanvasElement.prototype.getContext=function(kind,...args){const gl=get.call(this,kind,...args);if(kind==='webgl2'&&gl&&!gl.__brushObserved){
  gl.__brushObserved=true;window.__brushGl=gl;
  p.contexts.push({kind,version:gl.getParameter(gl.VERSION),max_samples:gl.getParameter(gl.MAX_SAMPLES)});
  const bind=gl.bindBuffer;gl.bindBuffer=function(target,buffer){if(target===this.ARRAY_BUFFER)this.__brushArrayBuffer=buffer;return bind.call(this,target,buffer);};
  const data=gl.bufferData;gl.bufferData=function(target,bytes,...args){const value=data.call(this,target,bytes,...args);const count=typeof bytes==='number'?bytes:bytes?.byteLength||0;if(target===this.ARRAY_BUFFER&&count>1000000){window.__brushMeshVbo={buffer:this.__brushArrayBuffer,bytes:count,at:performance.now()};p.uploads.push({at:performance.now(),bytes:count});}return value;};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;return viewport.call(this,x,y,width,height);};
  const storage=gl.renderbufferStorageMultisample;gl.renderbufferStorageMultisample=function(target,samples,format,width,height){const value=storage.call(this,target,samples,format,width,height);p.allocations.push({samples,actual_samples:this.getRenderbufferParameter(target,this.RENDERBUFFER_SAMPLES),width,height});return value;};
  const error=gl.getError;gl.getError=function(){const value=error.call(this);if(value!==this.NO_ERROR)p.gl_errors.push(value);return value;};
 }return gl;};
})();`;

const pointer = async (type, x, y, pointerType = 'mouse', force = 0.5, down = false, modifiers = 0) => {
  const params = { type, x, y, pointerType, force, modifiers, button: type === 'mouseMoved' ? 'none' : 'left', buttons: down ? 1 : 0, ...(type === 'mouseMoved' ? {} : { clickCount: 1 }) };
  result.input_trace.push({ ...params, at_epoch_ms: Date.now() });
  await send('Input.dispatchMouseEvent', params);
};
const chord = async (key, shift = false) => {
  // CDP modifier bits: Alt=1, Ctrl=2, Meta/Command=4, Shift=8.
  const modifiers = 4 | (shift ? 8 : 0);
  const code = 'Key' + key.toUpperCase();
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
  await pause(250);
};
const save = async stage => {
  const index = await evaluate('window.__brushCaptures.length');
  await chord('s');
  let size;
  for (let attempt = 0; attempt < 40; attempt++) {
    size = await evaluate(`window.__brushCaptures[${index}]?.contents.length||0`);
    if (size) break;
    await pause(100);
  }
  if (!size || size > 128 * 1024 * 1024) throw new Error(`Invalid save capture for ${stage}: ${size}`);
  let text = '';
  for (let offset = 0; offset < size; offset += 65536) text += await evaluate(`window.__brushCaptures[${index}].contents.slice(${offset},${offset + 65536})`);
  const file = path.join(out, `${stage}.obrush`); await writeFile(file, text);
  return { path: file, sha256: hash(text), bytes: Buffer.byteLength(text), document: JSON.parse(text) };
};
const metadata = ({ document, ...rest }) => ({ ...rest, vertices: document.mesh.positions.length, triangles: document.mesh.triangles.length });
const meshHash = document => hash(JSON.stringify(document.mesh));
const packedHash = mesh => {
  const bytes = Buffer.alloc(mesh.positions.length * 40);
  for (let vertex = 0; vertex < mesh.positions.length; vertex++) {
    const values = [...mesh.positions[vertex], ...mesh.normals[vertex], mesh.masks[vertex], ...mesh.colors[vertex]];
    for (let channel = 0; channel < 10; channel++) bytes.writeFloatLE(values[channel], vertex * 40 + channel * 4);
  }
  return hash(bytes);
};
const gpuDigest = () => evaluate(`(async()=>{const gl=window.__brushGl,vbo=window.__brushMeshVbo;if(!vbo)throw Error('No actual mesh VBO');const previous=gl.__brushArrayBuffer;const bytes=new Uint8Array(vbo.bytes);gl.bindBuffer(gl.ARRAY_BUFFER,vbo.buffer);gl.getBufferSubData(gl.ARRAY_BUFFER,0,bytes);gl.bindBuffer(gl.ARRAY_BUFFER,previous);const digest=await crypto.subtle.digest('SHA-256',bytes);return {bytes:vbo.bytes,sha256:Array.from(new Uint8Array(digest)).map(byte=>byte.toString(16).padStart(2,'0')).join(''),last_upload_at:vbo.at};})()`);
const screenshot = async name => {
  const response = await send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  const bytes = Buffer.from(response.data, 'base64');
  const file = path.join(out, `${name}.png`); await writeFile(file, bytes);
  return { path: file, sha256: hash(bytes), bytes: bytes.length };
};
const fresh = async () => {
  await send('Page.navigate', { url });
  for (let attempt = 0; attempt < 150; attempt++) {
    if (await evaluate('!document.getElementById("loading")&&window.__brushMeshVbo&&typeof window.obrushBrushMetrics==="function"')) {
      await pause(300);
      await evaluate('window.__brushCaptures=[];window.obrushDownload=(name,contents)=>window.__brushCaptures.push({name,contents});document.getElementById("obrush_canvas").focus();');
      return;
    }
    await pause(100);
  }
  throw new Error('Brush input startup timeout');
};

const click = async point => {
  await pointer('mouseMoved', ...point); await pause(35);
  await pointer('mousePressed', ...point, 'mouse', .5, true); await pause(40);
  await pointer('mouseReleased', ...point); await pause(130);
};
const importFile = async (file, expectedVertices) => {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', {nodeId:root.nodeId,selector:'#import-file'})).nodeId;
  await send('DOM.setFileInputFiles',{nodeId,files:[file]});
  for(let attempt=0;attempt<150;attempt++) {
    const state=await evaluate('({m:window.obrushBrushMetrics(),n:document.getElementById("notice").textContent,b:window.__brushMeshVbo.bytes})');
    if(state.n) throw Error(state.n);
    if(state.m.vertices===expectedVertices&&state.b===expectedVertices*40&&/loaded|import/i.test(state.m.message)) {await pause(250);return;}
    await pause(60);
  }
  throw Error('Public import timeout: '+file);
};
const settings = async () => {
  await click([43,291]);
  const radiusTarget=.037,strengthTarget=.79;
  // Geometry of these public controls was inspected at this viewport size in
  // the frozen 0.1.6 layout. Read the accepted app value after every click.
  for(const [key,y,target,tolerance] of [['radius',428,radiusTarget,.00011],['strength',489,strengthTarget,.0011]]) {
    let low=20,high=192;
    for(let attempt=0;attempt<14;attempt++) {
      await click([(low+high)/2,y]);
      const value=(await evaluate('window.obrushBrushMetrics()'))[key];
      if(Math.abs(value-target)<=tolerance) break;
      if(value<target)low=(low+high)/2;else high=(low+high)/2;
    }
  }
  const accepted=await evaluate('window.obrushBrushMetrics()');
  if(accepted.kind!=='Move'||Math.abs(accepted.radius-radiusTarget)>.00011||Math.abs(accepted.strength-strengthTarget)>.0011||!accepted.symmetry_x) throw Error('Actual UI settings differ: '+JSON.stringify(accepted));
  await evaluate('document.getElementById("obrush_canvas").focus()');
  return accepted;
};
const viewport = async () => {
  const p=await evaluate('window.__brushCheck');
  const v=Object.keys(p.viewports).map(value=>value.split(',').map(Number)).filter(([x,y,w,h])=>x>0&&w>400&&h>300).at(-1);
  if(!v||!p.allocations.some(item=>item.actual_samples===4))throw Error('No actual sculpt viewport / MSAA');
  return {raw:v,cx:v[0]+v[2]/2,cy:1000-v[1]-v[3]/2};
};
const sub=(a,b)=>a.map((value,axis)=>value-b[axis]);
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const dot=(a,b)=>a.reduce((sum,value,axis)=>sum+value*b[axis],0);
const norm=a=>Math.hypot(...a);
const percentile=(array,p)=>array.length?array[Math.min(array.length-1,Math.floor((array.length-1)*p))]:0;
const quality = mesh => {
  const edges=new Map(),normals=[],qualities=[],bounds=[[],[]];let zero=0,nonfinite=0,volume=0,minRadialCosine=1;
  for(let axis=0;axis<3;axis++){bounds[0][axis]=Infinity;bounds[1][axis]=-Infinity;}
  for(const point of mesh.positions)for(let axis=0;axis<3;axis++){bounds[0][axis]=Math.min(bounds[0][axis],point[axis]);bounds[1][axis]=Math.max(bounds[1][axis],point[axis]);}
  for(const point of mesh.positions)for(const value of point)if(!Number.isFinite(value))nonfinite++;
  for(let face=0;face<mesh.triangles.length;face++){
    const ids=mesh.triangles[face],p=ids.map(index=>mesh.positions[index]);
    const n=cross(sub(p[1],p[0]),sub(p[2],p[0])),area=norm(n),maxEdge=Math.max(norm(sub(p[1],p[0])),norm(sub(p[2],p[1])),norm(sub(p[0],p[2])));
    if(!area)zero++;normals.push(area?n.map(x=>x/area):[0,0,0]);qualities.push(maxEdge?area/(maxEdge*maxEdge):0);volume+=dot(p[0],cross(p[1],p[2]))/6;
    const centroid=p[0].map((value,axis)=>(value+p[1][axis]+p[2][axis])/3);
    if(area&&norm(centroid))minRadialCosine=Math.min(minRadialCosine,dot(n,centroid)/(area*norm(centroid)));
    for(let k=0;k<3;k++){const a=ids[k],b=ids[(k+1)%3],key=a<b?`${a},${b}`:`${b},${a}`;const edge=edges.get(key)||{a,b,faces:[],orientations:[]};edge.faces.push(face);edge.orientations.push(a<b?1:-1);edges.set(key,edge);}
  }
  let boundary=0,nonmanifold=0,winding=0,maxAngle=0,over60=0,over90=0,length60=0,length90=0;const lengths=[];
  for(const edge of edges.values()){
    const length=norm(sub(mesh.positions[edge.a],mesh.positions[edge.b]));lengths.push(length);
    if(edge.faces.length===1)boundary++;if(edge.faces.length>2)nonmanifold++;
    if(edge.faces.length===2){if(edge.orientations[0]===edge.orientations[1])winding++;
      const angle=Math.acos(Math.max(-1,Math.min(1,dot(normals[edge.faces[0]],normals[edge.faces[1]]))))*180/Math.PI;
      maxAngle=Math.max(maxAngle,angle);if(angle>60){over60++;length60+=length;}if(angle>90){over90++;length90+=length;}
    }
  }
  qualities.sort((a,b)=>a-b);lengths.sort((a,b)=>a-b);
  return {vertices:mesh.positions.length,triangles:mesh.triangles.length,nonfinite_positions:nonfinite,degenerate_triangles:zero,boundary_edges:boundary,nonmanifold_edges:nonmanifold,inconsistent_winding_edges:winding,min_cross_over_max_edge_squared:qualities[0],min_radial_face_cosine:minRadialCosine,quality_p01:percentile(qualities,.01),quality_median:percentile(qualities,.5),edge_p95:percentile(lengths,.95),max_edge: lengths.at(-1),max_adjacent_fold_deg:maxAngle,fold_edges_over_60:over60,fold_edges_over_90:over90,fold_edge_length_over_60:length60,fold_edge_length_over_90:length90,signed_volume:volume,bounds};
};
const differences = (base,final) => {
  let changed=0,max=0,sum=0,maskedMoved=0,maxMasked=0,mirrored=0,maxMirror=0;
  const lookup=new Map(base.positions.map((p,i)=>[JSON.stringify(p),i]));
  for(let i=0;i<base.positions.length;i++){
    const distance=norm(sub(final.positions[i],base.positions[i]));if(distance>0)changed++;max=Math.max(max,distance);sum+=distance*distance;
    if(base.masks[i]===1){if(distance>0)maskedMoved++;maxMasked=Math.max(maxMasked,distance);}
    const [x,y,z]=base.positions[i],other=x>0?lookup.get(JSON.stringify([-x,y,z])):undefined;
    if(other!==undefined){mirrored++;maxMirror=Math.max(maxMirror,Math.abs(final.positions[i][0]+final.positions[other][0]),Math.abs(final.positions[i][1]-final.positions[other][1]),Math.abs(final.positions[i][2]-final.positions[other][2]));}
  }
  return {changed_original_vertices:changed,max_original_displacement:max,rms_original_displacement:Math.sqrt(sum/base.positions.length),fully_masked_original_vertices_moved:maskedMoved,max_fully_masked_displacement:maxMasked,eligible_exact_source_mirror_pairs:mirrored,max_original_mirror_error:maxMirror};
};
const interpolate = (points,steps=20) => {
  const path=[];for(let i=1;i<points.length;i++)for(let k=1;k<=steps;k++)path.push(points[i-1].map((x,axis)=>x+(points[i][axis]-x)*k/steps));return path;
};
const drag = async (origin,pathPoints,kind='plain') => {
  const started=await evaluate('performance.now()');
  const phases=[];
  let boundary;
  const pen=kind.startsWith('pressure-'),pointerType=pen?'pen':'mouse';
  const contactForce=pen?(kind.includes('half')?.5:1):.5;
  const pressForce=kind==='pressure-ramp'||kind.includes('zero')?0:contactForce;
  await pointer('mouseMoved',...origin,pointerType,pressForce);await pause(80);await pointer('mousePressed',...origin,pointerType,pressForce,true);await pause(100);
  if(pen&&pressForce===0){await pointer('mouseMoved',...origin,pointerType,contactForce,true);await pause(100);}
  for(let i=0;i<pathPoints.length;i++){
    let modifier=0;if(kind==='shift-transition'&&i>=20&&i<40)modifier=8;
    if(kind==='shift-transition'&&(i===20||i===40)){
      await pause(180);phases.push({before_index:i,gpu:await gpuDigest(),metrics:await evaluate('window.obrushSculptMetrics()')});
    }
    if(kind==='shift-transition'&&(i===20||i===40))await send('Input.dispatchKeyEvent',{type:i===20?'rawKeyDown':'keyUp',key:'Shift',code:'ShiftLeft',windowsVirtualKeyCode:16,modifiers:modifier});
    await pointer('mouseMoved',...pathPoints[i],pointerType,contactForce,true,modifier);await pause(30);
  }
  let endpoint=pathPoints.at(-1)||origin;
  if(kind==='boundary'){
    await pause(150);const beforeOutside=await gpuDigest();
    await pointer('mouseMoved',2,30,'mouse',.5,true);await pause(100);await pointer('mouseMoved',2,980,'mouse',.5,true);await pause(100);const outside=await gpuDigest();
    await pointer('mouseMoved',...endpoint,'mouse',.5,true);await pause(150);const reentry=await gpuDigest();
    endpoint=[endpoint[0]+5,endpoint[1]-2];await pointer('mouseMoved',...endpoint,'mouse',.5,true);await pause(150);const resumed=await gpuDigest();
    await pointer('mouseMoved',2,980,'mouse',.5,true);await pause(100);const beforeOutsideRelease=await gpuDigest();
    boundary={beforeOutside,outside,reentry,resumed,beforeOutsideRelease,outside_exact:outside.sha256===beforeOutside.sha256,reentry_first_point_exact:reentry.sha256===beforeOutside.sha256,resumed_inside_movement_useful:resumed.sha256!==reentry.sha256};
    endpoint=[2,980];
  }
  if(kind==='ctrl-release'){
    await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'Control',code:'ControlLeft',windowsVirtualKeyCode:17,modifiers:2});
    endpoint=[endpoint[0]+12,endpoint[1]];
  }
  const releaseRequested=Date.now();await pointer('mouseReleased',...endpoint,pointerType,pen?0:.5,false,kind==='ctrl-release'?2:0);
  if(kind==='ctrl-release')await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Control',code:'ControlLeft',windowsVirtualKeyCode:17,modifiers:0});
  const probes=[];
  for(const delay of [0,200,800]) {await pause(delay);probes.push({delay_ms:delay,at:await evaluate('performance.now()'),gpu:await gpuDigest(),metrics:await evaluate('window.obrushSculptMetrics()')});}
  const ended=await evaluate('performance.now()');
  const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${started}),frames:window.__brushCheck.raf.filter(x=>x.at>=${started}&&x.at<=${ended}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${started}&&x.at<=${ended}),uploads:window.__brushCheck.uploads.filter(x=>x.at>=${started}&&x.at<=${ended})})`);
  if(boundary)boundary.outside_release_preserves_displayed_state=boundary.beforeOutsideRelease.sha256===probes[0].gpu.sha256;
  return {started,ended,phases,boundary,release_requested_epoch_ms:releaseRequested,probes,quiet:probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.metrics.pending_dabs===0),runtime};
};
try {
  let port, endpoint;
  for (let attempt = 0; attempt < 150; attempt++) {
    try { const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); port = Number(lines[0]); endpoint = `ws://127.0.0.1:${port}${lines[1]}`; break; } catch { }
    if (child.exitCode !== null) throw new Error(`Owned Chrome exited ${child.exitCode}`);
    await pause(100);
  }
  if (!endpoint) throw new Error('No owned loopback Chrome endpoint');
  // Inspect the running browser's own protocol before trusting pen/force input.
  const protocol = await (await fetch(`http://127.0.0.1:${port}/json/protocol`)).json();
  const command = protocol.domains.find(domain => domain.domain === 'Input').commands.find(command => command.name === 'dispatchMouseEvent');
  result.dispatch_mouse_event_parameters = command.parameters.filter(parameter => ['pointerType', 'force'].includes(parameter.name));
  result.dispatch_key_event_modifiers = protocol.domains.find(domain => domain.domain === 'Input').commands.find(command => command.name === 'dispatchKeyEvent').parameters.find(parameter => parameter.name === 'modifiers');
  if (!result.dispatch_mouse_event_parameters.some(parameter => parameter.name === 'force') || !result.dispatch_mouse_event_parameters.some(parameter => parameter.enum?.includes('pen'))) throw new Error('Running Chrome does not expose pen/force input');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const request = pending.get(message.id);
      if (request) { clearTimeout(request.timer); pending.delete(message.id); message.error ? request.reject(new Error(JSON.stringify(message.error))) : request.resolve(message.result); }
      return;
    }
    if (message.method === 'Runtime.exceptionThrown') result.exceptions.push(message.params.exceptionDetails);
    if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') result.console_errors.push(message.params.args.map(argument => argument.value || argument.description).join(' '));
    if (message.method === 'Fetch.requestPaused') {
      const address = message.params.request.url;
      const allowed = /^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?\//.test(address) || /^(blob|data|about):/.test(address);
      if (!allowed) result.blocked_external_requests.push(address);
      send(allowed ? 'Fetch.continueRequest' : 'Fetch.failRequest', { requestId: message.params.requestId, ...(allowed ? {} : { errorReason: 'BlockedByClient' }) }, message.sessionId).catch(error => result.console_errors.push(String(error)));
    }
  });
  result.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'Network']) await send(`${domain}.enable`);
  await send('Fetch.enable', { patterns: [{ urlPattern: '*', requestStage: 'Request' }] });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Target.activateTarget', { targetId: target.targetId }, null);
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const wasm = await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'));
  const served = Buffer.from(await (await fetch(url + 'pkg/obrush_app_bg.wasm')).arrayBuffer());
  result.wasm = { sha256: hash(wasm), served_sha256: hash(served), bytes: wasm.length };
  if (hash(wasm) !== hash(served)) throw new Error('Served WASM differs from local build');

  result.plan = [
    {name:'sphere-outbound',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[180,-80]],steps:60},
    {name:'sphere-cross-return',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[160,-90],[-100,80],[160,60],[0,0]],steps:20},
    {name:'sphere-seam-cross',source:'fresh',origin:[-139.4,68.4],waypoints:[[0,0],[160,-50],[-160,60],[0,0]],steps:20,seam_projection:'World (0,0,1), default camera yaw=.35 pitch=.18 distance=4, FOV=40deg at recorded viewport'},
    {name:'sphere-partial-mask',source:'partial-mask',origin:[-85,-30],waypoints:[[0,0],[160,-75],[-80,70],[140,0]],steps:20},
    {name:'sphere-full-mask',source:'full-mask',origin:[-85,-30],waypoints:[[0,0],[160,-80]],steps:40},
    {name:'user-crossing',source:'user',origin:[-85,-30],waypoints:[[0,0],[140,-60],[-90,80],[140,40]],steps:20},
    {name:'sphere-shift-transition',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[70,-20],[90,-10],[145,25]],steps:20,kind:'shift-transition'},
    {name:'sphere-zero-pressure-start',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[90,-35]],steps:40,kind:'pressure-ramp'},
    {name:'sphere-ctrl-release',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[90,-35]],steps:40,kind:'ctrl-release'},
    {name:'sphere-zero-half-radius',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[90,-35]],steps:40,kind:'pressure-zero-half-radius',pressureRadius:true},
    {name:'sphere-contact-half-radius',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[90,-35]],steps:40,kind:'pressure-half-radius',pressureRadius:true},
    {name:'sphere-contact-full-radius',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[90,-35]],steps:40,kind:'pressure-full-radius',pressureRadius:true},
    {name:'sphere-eight-repeated-grabs',source:'fresh',contacts:[[-85,-30,30,-10],[-90,-25,25,15],[-80,-35,-30,10],[-75,-25,25,-15],[-95,-20,30,5],[-85,-40,-25,-10],[-75,-40,30,15],[-95,-40,25,-5]],steps:12},
    {name:'sphere-twelve-repeated-q02',source:'fresh',contacts:[[-85,-30,30,-10],[-90,-25,25,15],[-80,-35,-30,10],[-75,-25,25,-15],[-95,-20,30,5],[-85,-40,-25,-10],[-75,-40,30,15],[-95,-40,25,-5],[-85,-30,35,-10],[-90,-28,30,12],[-78,-35,-25,10],[-80,-22,30,-12]],steps:12},
    {name:'sphere-exit-reentry-outside-release',source:'fresh',origin:[-85,-30],waypoints:[[0,0],[3,-2]],steps:3,kind:'boundary'},
  ];
  const chosen=selectedCases;
  const documents={};
  for(const plan of result.plan.filter(plan=>!chosen||chosen.includes(plan.name))){
    await fresh();
    const trial={name:plan.name,plan,snapshots:[]};result.cases.push(trial);
    if(plan.source==='user')await importFile(fixture,61580);
    if(plan.source.endsWith('mask')){
      const source=await save(plan.name+'-unmasked-source');
      const masked=structuredClone(source.document);
      // Authored mask protects upper/lower world-Y bands symmetrically; all
      // existing positions, topology, colors and normals remain unchanged.
      masked.mesh.masks=masked.mesh.positions.map(p=>plan.source==='full-mask'||p[1]>.2305?1:0);
      const file=path.join(out,plan.name+'-authored-fixture.obrush'),text=JSON.stringify(masked);await writeFile(file,text);
      trial.fixture={path:file,sha256:hash(text),rule:plan.source==='full-mask'?'Every vertex mask=1':'World Y > 0.2305 => mask 1; otherwise 0; threshold bisects the independent projected fresh sphere grab',source:metadata(source)};
      await importFile(file,masked.mesh.positions.length);
    }
    trial.settings=await settings();
    if(plan.pressureRadius){await click([22,672]);trial.settings=await evaluate('window.obrushBrushMetrics()');if(!trial.settings.pressure_radius)throw Error('Pressure radius actual checkbox did not toggle');}
    trial.viewport=await viewport();trial.initial_screenshot=await screenshot(plan.name+'-initial');
    const initial=await save(plan.name+'-initial');trial.snapshots.push(metadata(initial));trial.initial_quality=quality(initial.document.mesh);
    let origin;
    if(plan.contacts){
      trial.input={contacts:[],spacing_ms:30};trial.strokes=[];
      if(plan.name==='sphere-twelve-repeated-q02')trial.contact_snapshots=[];
      for(const [index,[x,y,dx,dy]]of plan.contacts.entries()){
        origin=[trial.viewport.cx+x,trial.viewport.cy+y];const points=interpolate([[0,0],[dx,dy]],plan.steps).map(p=>[origin[0]+p[0],origin[1]+p[1]]);
        trial.input.contacts.push({origin,points});trial.strokes.push(await drag(origin,points));
        if(plan.name==='sphere-twelve-repeated-q02'){
          const state=await save(`q02-after-${index+1}`);
          const image=[4,8,12].includes(index+1)?await screenshot(`q02-after-${index+1}`):null;
          trial.contact_snapshots.push({index:index+1,project:metadata(state),mesh_sha256:meshHash(state.document),quality:quality(state.document.mesh),screenshot:image});
          await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
        }
      }
      trial.stroke={quiet:trial.strokes.every(s=>s.quiet),phases:[],runtime:{frames:trial.strokes.flatMap(s=>s.runtime.frames),events:trial.strokes.flatMap(s=>s.runtime.events),uploads:trial.strokes.flatMap(s=>s.runtime.uploads),longtasks:trial.strokes.flatMap(s=>s.runtime.longtasks)}};
    }else{
      origin=[trial.viewport.cx+plan.origin[0],trial.viewport.cy+plan.origin[1]];
      const points=interpolate(plan.waypoints,plan.steps).map(p=>[origin[0]+p[0],origin[1]+p[1]]);
      trial.input={origin,points,spacing_ms:30};trial.stroke=await drag(origin,points,plan.kind);
    }
    trial.release_quiet=trial.stroke.quiet;
    trial.gpu_before_save=await gpuDigest();trial.final_screenshot=await screenshot(plan.name+'-final');
    const final=await save(plan.name+'-final');trial.snapshots.push(metadata(final));
    if(plan.name==='sphere-twelve-repeated-q02'){
      await pause(800);const idle=await save('q02-idle');
      trial.idle_save={...metadata(idle),mesh_sha256:meshHash(idle.document),exact_final:meshHash(idle.document)===meshHash(final.document)};
    }
    documents[plan.name]=final.document;
    trial.gpu_matches_saved_mesh=trial.gpu_before_save.sha256===packedHash(final.document.mesh);
    trial.final_quality=quality(final.document.mesh);trial.delta=differences(initial.document.mesh,final.document.mesh);
    trial.attributes={original_masks_unchanged:initial.document.mesh.masks.every((value,index)=>value===final.document.mesh.masks[index]),original_colors_unchanged:initial.document.mesh.colors.every((color,index)=>color.every((value,axis)=>value===final.document.mesh.colors[index][axis]))};
    if(plan.source!=='full-mask'&&meshHash(initial.document)!==meshHash(final.document)){
      const undoChain=[];let undo;
      for(let step=0;step<(plan.contacts?plan.contacts.length:3);step++){
        await chord('z');undo=await save(plan.name+'-undo-'+step);trial.snapshots.push(metadata(undo));undoChain.push({step,mesh_sha256:meshHash(undo.document),packed_sha256:packedHash(undo.document.mesh)});
        if(meshHash(undo.document)===meshHash(initial.document))break;
      }
      let redo;for(let step=0;step<undoChain.length;step++){await chord('z',true);redo=await save(plan.name+'-redo-'+step);trial.snapshots.push(metadata(redo));}
      trial.history={undo_steps:undoChain.length,undo_chain:undoChain,undo_exact_initial:meshHash(undo.document)===meshHash(initial.document),redo_exact_final:meshHash(redo.document)===meshHash(final.document),modifier_phase_states_in_undo:trial.stroke.phases.map(phase=>({before_index:phase.before_index,found:undoChain.some(state=>state.packed_sha256===phase.gpu.sha256)}))};
      if(plan.name==='sphere-twelve-repeated-q02'){
        const commits=[meshHash(initial.document)];
        for(const snapshot of trial.contact_snapshots)if(snapshot.mesh_sha256!==commits.at(-1))commits.push(snapshot.mesh_sha256);
        trial.history.effective_contacts=commits.length-1;
        trial.history.matches_effective_sequence=undoChain.length===commits.length-1&&undoChain.every((state,index)=>state.mesh_sha256===commits[commits.length-2-index]);
      }
      if(plan.name==='sphere-outbound'){
        await click(origin);const clickResult=await save(plan.name+'-no-motion-click');trial.snapshots.push(metadata(clickResult));
        await chord('z');const clickUndo=await save(plan.name+'-no-motion-undo');trial.snapshots.push(metadata(clickUndo));
        await chord('z',true);const clickRedo=await save(plan.name+'-no-motion-redo');trial.snapshots.push(metadata(clickRedo));
        trial.no_motion_click={mesh_unchanged:meshHash(clickResult.document)===meshHash(final.document),undo_reaches_initial:meshHash(clickUndo.document)===meshHash(initial.document),redo_restores_final:meshHash(clickRedo.document)===meshHash(final.document)};
        await chord('z');await click(origin);await chord('z',true);const redoAfterClick=await save(plan.name+'-redo-after-no-motion');trial.snapshots.push(metadata(redoAfterClick));
        trial.no_motion_click.preserves_existing_redo=meshHash(redoAfterClick.document)===meshHash(final.document);
      }
    }else trial.full_mask_exact_noop=meshHash(initial.document)===meshHash(final.document);
    trial.runtime_errors=await evaluate('({gl:window.__brushCheck.gl_errors,js:window.__brushCheck.js_errors,notice:document.getElementById("notice").textContent})');
    trial.interaction_gates={gpu_saved_exact:trial.gpu_matches_saved_mesh,release_quiet:trial.release_quiet,no_runtime_errors:!trial.runtime_errors.gl.length&&!trial.runtime_errors.js.length&&!trial.runtime_errors.notice,finite:trial.final_quality.nonfinite_positions===0,closed_manifold:trial.final_quality.boundary_edges===0&&trial.final_quality.nonmanifold_edges===0&&trial.final_quality.inconsistent_winding_edges===0,mask_protection:trial.delta.fully_masked_original_vertices_moved===0,partial_mask_nonempty:plan.source!=='partial-mask'||trial.delta.changed_original_vertices>0,full_mask_exact_noop:plan.source!=='full-mask'||trial.full_mask_exact_noop,history_exact:!trial.history||(trial.history.undo_exact_initial&&trial.history.redo_exact_final),no_motion_click_preserves_history:!trial.no_motion_click||Object.values(trial.no_motion_click).every(Boolean),modifier_phases_preserved:!trial.history||trial.history.modifier_phase_states_in_undo.every(phase=>phase.found)};
    trial.interaction_gates.pressure_zero_to_positive_still_moves=!plan.kind?.startsWith('pressure-')||trial.delta.changed_original_vertices>0;
    trial.interaction_gates.move_does_not_paint_mask_or_color=Object.values(trial.attributes).every(Boolean);
    if(plan.name==='sphere-outbound')trial.interaction_gates.outbound_has_useful_displacement=trial.delta.max_original_displacement>=.001;
    if(plan.contacts){trial.interaction_gates.repeated_grabs_have_useful_displacement=trial.delta.max_original_displacement>=.001;trial.interaction_gates.repeated_grabs_no_fold_above_90_deg=trial.final_quality.max_adjacent_fold_deg<=90.001;trial.fold_tolerance_deg=.001;}
    if(plan.name==='sphere-twelve-repeated-q02'){
      const qualities=trial.contact_snapshots.map(snapshot=>snapshot.quality);
      trial.visual_gate='pending_offline_original_pixel_review';
      trial.interaction_gates.q02_fresh_sphere=initial.document.mesh.positions.length===40962&&initial.document.mesh.triangles.length===81920;
      trial.interaction_gates.q02_backface_culling=trial.settings.backface_culling===true;
      trial.interaction_gates.q02_four_effective_contacts=trial.history?.effective_contacts>=4;
      trial.interaction_gates.q02_eight_original_vertices=trial.delta.changed_original_vertices>=8;
      trial.interaction_gates.q02_finite_closed_noninverted=qualities.every(q=>q.nonfinite_positions===0&&q.degenerate_triangles===0&&q.boundary_edges===0&&q.nonmanifold_edges===0&&q.inconsistent_winding_edges===0&&q.min_radial_face_cosine>0);
      trial.interaction_gates.q02_quality_floor=qualities.every(q=>q.min_cross_over_max_edge_squared>=.09);
      trial.interaction_gates.q02_no_fold_over_90=qualities.every(q=>q.fold_edges_over_90===0);
      trial.interaction_gates.q02_exact_one_undo_per_effective_contact=trial.history?.matches_effective_sequence===true;
      trial.interaction_gates.q02_trusted_pointer_events=trial.stroke.runtime.events.filter(event=>event.type.startsWith('pointer')).every(event=>event.trusted);
      trial.interaction_gates.q02_idle_project_exact=trial.idle_save.exact_final;
    }
    if(plan.kind==='boundary'){trial.interaction_gates.outside_drops_travel=trial.stroke.boundary.outside_exact&&trial.stroke.boundary.reentry_first_point_exact;trial.interaction_gates.reentry_resumes_useful_inside_movement=trial.stroke.boundary.resumed_inside_movement_useful;trial.interaction_gates.outside_release_preserves_displayed_state=trial.stroke.boundary.outside_release_preserves_displayed_state;trial.interaction_gates.outside_release_one_history_step=trial.history?.undo_steps===1;}
    await writeFile(path.join(out,plan.name+'.json'),JSON.stringify(trial,null,2)+'\n');
    await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify({completed:plan.name,initial:trial.initial_quality,final:trial.final_quality,delta:trial.delta,interaction_gates:trial.interaction_gates,history:trial.history,no_motion_click:trial.no_motion_click}));
    if(!Object.values(trial.interaction_gates).every(Boolean))throw Error('Move interaction gate failed: '+plan.name);
  }
  if(documents['sphere-zero-half-radius']&&documents['sphere-contact-half-radius']&&documents['sphere-contact-full-radius']){
    const half=result.cases.find(c=>c.name==='sphere-contact-half-radius'),full=result.cases.find(c=>c.name==='sphere-contact-full-radius');
    result.pressure_radius_comparison={zero_down_then_half_matches_positive_half_down_exactly:meshHash(documents['sphere-zero-half-radius'])===meshHash(documents['sphere-contact-half-radius']),half_changed_original_vertices:half.delta.changed_original_vertices,full_changed_original_vertices:full.delta.changed_original_vertices,half_footprint_smaller:half.delta.changed_original_vertices<full.delta.changed_original_vertices};
    if(!result.pressure_radius_comparison.zero_down_then_half_matches_positive_half_down_exactly||!result.pressure_radius_comparison.half_footprint_smaller)throw Error('Move first-positive pressure-radius comparison failed');
  }
  result.status='completed Move baseline/candidate observation';
  result.limitations=['Synthetic CDP mouse traces in owned headless Chrome; physical pen, native GUI and input-to-photon latency remain unverified.','Quality and fold metrics quantify these traces; no ZBrush output comparison or global self-intersection proof.','Existing folds in the unchanged supplied model remain part of the measured baseline. Raw edge counts are accompanied by world-length totals because adaptive splitting can increase counts without increasing fold extent.','Shift transition intentionally may become multiple history steps in the candidate; record exact state and compare separately rather than assume one Undo.'];
} catch (error) {
  result.status = 'failed Move regression harness'; result.failure = String(error); process.exitCode = 1;
  try { result.failure_runtime = await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})'); result.failure_screenshot = await screenshot('failure'); } catch { }
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  await writeFile(path.join(out, 'chrome-stderr.log'), stderr); await writeFile(path.join(out, 'chrome-stdout.log'), stdout);
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch { }
  try { socket?.close(); } catch { }
  for (const request of pending.values()) { clearTimeout(request.timer); request.reject(new Error('Owned headless session closing')); } pending.clear();
  await pause(400); if (child.exitCode === null) child.kill('SIGTERM'); await pause(200);
  await new Promise(resolve => server.close(resolve));
  result.cleanup = { owned_chrome_exit_code: child.exitCode, owned_chrome_signal: child.signalCode, owned_server_closed: true };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json'), wasm: result.wasm, cases: result.cases.map(trial => ({ name: trial.name, quality: trial.final_quality, delta: trial.delta, gpu: trial.gpu_matches_saved_mesh, history: trial.history, release:trial.release_quiet })) }));
}
