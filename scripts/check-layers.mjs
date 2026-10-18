// CLI-only sculpt-layer geometry and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-layers.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'layers';
const out = path.resolve(`.artifacts/layer-browser-0.1.8/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
await mkdir(profile, { recursive: true });
await writeFile(path.join(out,'harness.mjs'),await readFile(new URL(import.meta.url)));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

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
  out, url, web, profile, owned_pid: child.pid,
  harness_sha256: hash(await readFile(new URL(import.meta.url))),
  preregistered_contract:{draw_radius:.18,draw_strength:.35,symmetry_x:true,half_intensity_max_component_error:3e-7,nonbinary_intensity:.37,stored_f64_composition:'For each component, start at stored f64 base; add visible Math.fround(intensity)*offset contributions only when nonzero, in entry order; Math.fround result and compare exact f32 bits including signed zero',history:'Exact project fields and packed mesh bits',gpu:'Exact packed f32 vertex-buffer digest',release_probe_incremental_delays_ms:[0,200,800]},
  protocol_reference: 'https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-dispatchMouseEvent',
  cases: [], comparisons: {}, exceptions: [], console_errors: [], blocked_external_requests: [], input_trace: [],
  ui_actions: [], strokes: [], snapshots: [], page_diagnostics: [],
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
  const bindVao=gl.bindVertexArray;gl.bindVertexArray=function(vao){this.__brushVao=vao;return bindVao.call(this,vao);};
  const bind=gl.bindBuffer;gl.bindBuffer=function(target,buffer){if(target===this.ARRAY_BUFFER)this.__brushArrayBuffer=buffer;return bind.call(this,target,buffer);};
  const attribute=gl.vertexAttribPointer;gl.vertexAttribPointer=function(index,size,type,normalized,stride,offset){if(stride===40&&offset===0&&size===3){this.__brushMeshBuffer=this.__brushArrayBuffer;}return attribute.call(this,index,size,type,normalized,stride,offset);};
  const data=gl.bufferData;gl.bufferData=function(target,bytes,...args){const value=data.call(this,target,bytes,...args);const count=typeof bytes==='number'?bytes:bytes?.byteLength||0;if(target===this.ARRAY_BUFFER&&this.__brushArrayBuffer===this.__brushMeshBuffer){window.__brushMeshVbo={buffer:this.__brushArrayBuffer,bytes:count,at:performance.now()};p.uploads.push({at:performance.now(),bytes:count});}return value;};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;if(x>0&&width>400&&height>300)p.last_sculpt_viewport=[x,y,width,height];return viewport.call(this,x,y,width,height);};
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
const capturePageDiagnostics = async () => {
  const page = await evaluate(`(()=>{const p=window.__brushCheck;if(!p)return null;return {url:location.href,at:performance.now(),gl_errors:p.gl_errors,js_errors:p.js_errors,contexts:p.contexts,allocations:p.allocations};})()`);
  if (!page) return;
  page.actual_4x_msaa = page.allocations.some(allocation => allocation.actual_samples === 4);
  result.page_diagnostics.push(page);
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  if (page.gl_errors.length || page.js_errors.length || !page.actual_4x_msaa) throw Error('Page has runtime errors or missing actual MSAA before navigation');
};
const fresh = async () => {
  // Preserve and check the outgoing page before navigation replaces its globals.
  await capturePageDiagnostics();
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
const state=()=>evaluate('window.obrushBrushMetrics()');
const assert=(condition,message)=>{if(!condition)throw Error(message);};
// Captured saves use the same canonical encoder. Preserve exact download bytes,
// including signed-zero metadata, for project history and roundtrip checks.
const projectHash=saved=>saved.sha256;
const waitFor=async(predicate,message)=>{for(let i=0;i<140;i++){const s=await state();if(predicate(s)){await pause(150);return s;}await pause(50);}throw Error(message);};
const control=async(key,allowDisabled=false)=>{
  for(let i=0;i<12;i++){
    const s=await state(),rect=s.layer_controls?.[key];
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&s.layer_controls_enabled?.[key]===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.layer_controls_enabled?.[key]};
    }
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:1480,y:750,deltaX:0,deltaY:point[1]>955?350:-350});await pause(150);
  }
  throw Error('Control never entered owned viewport: '+key);
};
const action=async(key,allowDisabled=false)=>{const c=await control(key,allowDisabled);result.ui_actions.push({key,...c,at_epoch_ms:Date.now()});await click(c.point);return state();};
const intensity=async(index,value)=>{
  await action('layers.select.'+index);
  const c=await control('layers.intensity');
  // The actual egui0.33.3 default rectangular handle uses height/2.5 with
  // aspect0.75; account for that inset when clicking a nonbinary weight.
  const inset=(c.rect[3]-c.rect[1])*.3;
  // eframe's MouseEvent client_x binding uses integer CSS-pixel coordinates;
  // choose the nearest pixel before that conversion truncates a fraction.
  const x=Math.round(value===0?c.rect[0]+1:value===1?c.rect[2]-1:c.rect[0]+inset+(c.rect[2]-c.rect[0]-2*inset)*value);
  result.ui_actions.push({key:'layers.intensity',layer:index,value,rect:c.rect,pointer:[x,c.point[1]],at_epoch_ms:Date.now()});
  await click([x,c.point[1]]);
  return waitFor(s=>Math.abs(s.layers?.[index]?.intensity-value)<1e-6,'Intensity commit did not reach '+value);
};
const stop=async()=>{if((await state()).recording_layer!==null)await action('layers.stop');return waitFor(s=>s.recording_layer===null,'Recording did not stop');};
const rename=async(index,name)=>{
  await action('layers.select.'+index);await action('layers.name');
  const modifiers=4;for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers});
  await send('Input.insertText',{text:name});await pause(100);await action('layers.rename');
  return waitFor(s=>s.layers[index]?.name===name,'Actual Rename did not commit '+name);
};
const key=async(number)=>{for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:String(number),code:'Digit'+number,windowsVirtualKeyCode:48+number,modifiers:0});await pause(100);};
const viewport=async()=>{
  const p=await evaluate('window.__brushCheck');
  const raw=p.last_sculpt_viewport||Object.keys(p.viewports).map(v=>v.split(',').map(Number)).filter(([x,y,w,h])=>x>0&&w>400&&h>300).at(-1);
  assert(raw,'Missing actual sculpt viewport');return {raw,cx:raw[0]+raw[2]/2,cy:1000-raw[1]-raw[3]/2};
};
const sculpt=async(name,number,offset,endOffset,steps=12)=>{
  await key(number);await evaluate('document.getElementById("obrush_canvas").focus()');const settings=await state();
  assert(settings.kind===(number===7?'Move':'Draw'),'Actual keyboard brush selection failed for '+name);
  assert(Math.abs(settings.radius-.18)<1e-6&&Math.abs(settings.strength-.35)<1e-6&&settings.symmetry_x,'Unexpected actual brush parameters: '+name);
  const v=await viewport();
  const from=[v.cx+offset[0],v.cy+offset[1]],to=[v.cx+endOffset[0],v.cy+endOffset[1]],start=await evaluate('performance.now()');
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,'mouse',.5,true);await pause(70);
  for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,'mouse',.5,true);await pause(35);}
  await pointer('mouseReleased',...to);const probes=[];
  for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()')});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);
  assert(quiet,'Geometry or pending input changed after release: '+name);
  const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);
  const trial={name,brush:number,settings,from,to,steps,spacing_ms:35,viewport:v,start,end,probes,quiet,runtime,after:await state()};result.strokes.push(trial);return trial;
};
const composition=project=>{
  if(!project.layers)return {present:false};
  const layers=project.layers;assert(layers.scheme==='world_offsets_f64_v1','Unexpected stored layer scheme');
  assert(layers.base_positions.length===project.mesh.positions.length,'Layer base count differs');
  assert(JSON.stringify(layers.triangles)===JSON.stringify(project.mesh.triangles),'Layer face correspondence differs');
  let different=0,max=0;
  for(let i=0;i<layers.base_positions.length;i++)for(let k=0;k<3;k++){
    let value=layers.base_positions[i][k];
    for(const entry of layers.entries)if(entry.visible){const contribution=Math.fround(entry.intensity)*entry.deltas[i][k];if(contribution!==0)value+=contribution;}
    // serde emits the shortest decimal that round-trips as f32. Parsing that
    // decimal as JS f64 is not itself the stored f32 value; round both sides.
    const expected=Math.fround(value),actual=Math.fround(project.mesh.positions[i][k]);if(!Object.is(expected,actual))different++;max=Math.max(max,Math.abs(expected-actual));
  }
  assert(different===0,'Saved visible geometry differs from independent ordered f64 composition');
  return {present:true,vertices:layers.base_positions.length,layers:layers.entries.length,position_components_differing:different,max_component_error:max};
};
const snapshot=async(name)=>{
  await pause(150);const gpu=await gpuDigest(),saved=await save(name),metrics=await state();
  const record={...metadata(saved),metrics,gpu,gpu_matches_saved_mesh:gpu.sha256===packedHash(saved.document.mesh),composition:composition(saved.document)};
  assert(record.gpu_matches_saved_mesh,'Actual GPU VBO differs from Save: '+name);
  result.snapshots.push(record);return saved;
};
const positionDiff=(a,b)=>{
  assert(a.positions.length===b.positions.length&&JSON.stringify(a.triangles)===JSON.stringify(b.triangles),'Unexpected topology change');
  let changed=0,max=0,sum=0;
  for(let i=0;i<a.positions.length;i++){let d=0;for(let k=0;k<3;k++){const v=b.positions[i][k]-a.positions[i][k];d+=v*v;}if(d)changed++;max=Math.max(max,Math.sqrt(d));sum+=d;}
  return {changed_vertices:changed,max_displacement:max,rms_displacement:Math.sqrt(sum/a.positions.length)};
};
const sameGeometry=(a,b)=>JSON.stringify(a.positions)===JSON.stringify(b.positions)&&JSON.stringify(a.triangles)===JSON.stringify(b.triangles)&&JSON.stringify(a.masks)===JSON.stringify(b.masks)&&JSON.stringify(a.colors)===JSON.stringify(b.colors);
const halfResponse=(base,full,half)=>{
  let max=0;for(let i=0;i<base.positions.length;i++)for(let k=0;k<3;k++)max=Math.max(max,Math.abs(half.positions[i][k]-(base.positions[i][k]+.5*(full.positions[i][k]-base.positions[i][k]))));
  return {max_component_error:max,tolerance:3e-7,pass:max<=3e-7};
};
const importFile=async(file,expectedVertices,expectRejection=false)=>{
  const dom=(await send('DOM.getDocument')).root,nodeId=(await send('DOM.querySelector',{nodeId:dom.nodeId,selector:'#import-file'})).nodeId;
  await evaluate('document.getElementById("notice").textContent=""');
  await send('DOM.setFileInputFiles',{nodeId,files:[file]});
  for(let i=0;i<160;i++){
    const s=await evaluate('({m:window.obrushBrushMetrics(),notice:document.getElementById("notice").textContent,vbo:window.__brushMeshVbo.bytes})');
    if(expectRejection&&s.notice){await pause(100);return s.notice;}
    if(!expectRejection&&s.notice)throw Error(s.notice);
    if(!expectRejection&&s.m.vertices===expectedVertices&&s.vbo===expectedVertices*40&&/^Loaded /.test(s.m.message)){await pause(200);return s.m;}
    await pause(60);
  }
  throw Error('Public file import/rejection did not complete: '+file);
};
const layerCount=async(count)=>waitFor(s=>s.layers?.length===count&&s.layers_locked_topology===(count>0),'Unexpected layer count/lock');
const newLayer=async(count)=>{await action('layers.new');const s=await layerCount(count);assert(s.recording_layer===count-1&&s.layers[count-1].visible&&s.layers[count-1].intensity===1,'New layer did not start visible unit recording');return s;};
const history=async(name,before,after)=>{
  await chord('z');const undo=await snapshot(name+'-undo');await chord('z',true);const redo=await snapshot(name+'-redo');
  const record={undo_exact:projectHash(undo)===projectHash(before)&&packedHash(undo.document.mesh)===packedHash(before.document.mesh),redo_exact:projectHash(redo)===projectHash(after)&&packedHash(redo.document.mesh)===packedHash(after.document.mesh)};
  assert(Object.values(record).every(Boolean),'Whole project history mismatch: '+name);return record;
};
const finishCase=async(record)=>{result.cases.push(record);await writeFile(path.join(out,record.name+'.json'),JSON.stringify(record,null,2)+'\n');await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:record.name,gates:record.gates}));assert(Object.values(record.gates).every(Boolean),'Layer case failed: '+record.name);};
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

  await fresh();
  result.initial_controls=await screenshot('initial-controls');
  assert(Array.isArray((await state()).layers),'Missing read-only layer diagnostics');
  const base=await snapshot('initial-base');assert(base.document.version===2&&!base.document.layers,'Fresh non-layer project is not v2');
  await newLayer(1);await rename(0,'Independent Draw ridge');const empty=await snapshot('empty-draw-layer');
  assert(empty.document.version===3&&empty.document.layers?.entries.length===1,'Layer-bearing Save is not v3');
  await sculpt('record-draw',1,[-90,-40],[-50,-10],14);const drawFull=await snapshot('draw-full-recording');
  const drawDifference=positionDiff(base.document.mesh,drawFull.document.mesh);assert(drawDifference.max_displacement>=.002,'Recorded Draw has no useful geometry');
  const drawHistory=await history('recorded-draw',empty,drawFull);
  await intensity(0,0);assert((await state()).recording_layer===null,'Nonunit intensity did not atomically stop recording');
  const drawZero=await snapshot('draw-zero');result.draw_zero_png=await screenshot('draw-zero');
  await intensity(0,.5);const drawHalf=await snapshot('draw-half');result.draw_half_png=await screenshot('draw-half');
  const half=halfResponse(base.document.mesh,drawFull.document.mesh,drawHalf.document.mesh);
  const unavailable=(await state()).layer_controls_enabled?.['layers.record'];await action('layers.record',true);const halfAfterRecord=await snapshot('half-record-attempt');
  await intensity(0,.37);const drawNonbinary=await snapshot('draw-nonbinary-37-percent');
  await intensity(0,1);const drawAgain=await snapshot('draw-full-again');result.draw_full_png=await screenshot('draw-full');
  await action('layers.visibility.0');const drawHidden=await snapshot('draw-hidden');
  await action('layers.visibility.0');const drawShown=await snapshot('draw-shown');
  await finishCase({name:'draw-intensity-visibility',draw_difference:drawDifference,half_response:half,history:drawHistory,gates:{zero_returns_base:sameGeometry(base.document.mesh,drawZero.document.mesh),half_is_scaled:half.pass,nonbinary_weight_is_stored_f32:Math.fround(drawNonbinary.document.layers.entries[0].intensity)===Math.fround(.37),full_returns_exact_mesh:meshHash(drawFull.document)===meshHash(drawAgain.document),hidden_returns_zero_mesh:meshHash(drawHidden.document)===meshHash(drawZero.document),shown_returns_full_mesh:meshHash(drawShown.document)===meshHash(drawAgain.document),nonunit_record_unavailable:unavailable===false&&projectHash(halfAfterRecord)===projectHash(drawHalf)}});

  const firstDeltas=hash(JSON.stringify(drawShown.document.layers.entries[0].deltas));
  await newLayer(2);await rename(1,'Independent Move pull');const moveEmpty=await snapshot('empty-move-layer');
  await sculpt('record-move',7,[95,65],[120,75],12);const twoRecording=await snapshot('two-layers-recording');
  const moveDifference=positionDiff(moveEmpty.document.mesh,twoRecording.document.mesh);assert(moveDifference.max_displacement>=.001,'Recorded Move has no useful geometry');
  const moveHistory=await history('recorded-move',moveEmpty,twoRecording);
  await action('layers.visibility.1');assert((await state()).recording_layer===null,'Hiding recorded layer did not atomically stop recording');
  const onlyDraw=await snapshot('only-draw');
  await action('layers.visibility.1');const bothShown=await snapshot('both-shown');
  await action('layers.visibility.0');const onlyMove=await snapshot('only-move');
  await action('layers.visibility.1');const neither=await snapshot('neither-layer');
  await action('layers.visibility.0');const drawRestored=await snapshot('draw-restored');
  await action('layers.visibility.1');const twoShown=await snapshot('two-layers-shown');
  await finishCase({name:'draw-move-layer-independence',move_difference:moveDifference,history:moveHistory,gates:{first_deltas_unchanged:firstDeltas===hash(JSON.stringify(twoShown.document.layers.entries[0].deltas)),hiding_move_retains_draw:meshHash(onlyDraw.document)===meshHash(drawShown.document),hiding_all_returns_base:sameGeometry(neither.document.mesh,base.document.mesh),restoring_draw_exact:meshHash(drawRestored.document)===meshHash(drawShown.document),restoring_both_exact:meshHash(twoShown.document)===meshHash(bothShown.document),move_only_nonempty:positionDiff(base.document.mesh,onlyMove.document.mesh).max_displacement>=.001}});

  await action('layers.select.0');await action('layers.record');assert((await state()).recording_layer===0,'Selected layer did not record');await stop();
  const beforeBase=await snapshot('before-base-edit'),deltasBefore=hash(JSON.stringify(beforeBase.document.layers.entries.map(e=>e.deltas)));
  await sculpt('base-draw-recording-off',1,[0,120],[10,135],10);const baseEdited=await snapshot('base-edited');
  const baseDifference=positionDiff(beforeBase.document.mesh,baseEdited.document.mesh);assert(baseDifference.max_displacement>=.002,'Recording-off base edit is empty');
  await intensity(0,0);await intensity(1,0);const editedBaseOnly=await snapshot('edited-base-only');
  await intensity(0,1);await intensity(1,1);const restoredAfterBase=await snapshot('base-edit-layers-restored');
  await finishCase({name:'stop-record-base-edit',difference:baseDifference,gates:{deltas_unchanged:deltasBefore===hash(JSON.stringify(restoredAfterBase.document.layers.entries.map(e=>e.deltas))),base_positions_changed:hash(JSON.stringify(beforeBase.document.layers.base_positions))!==hash(JSON.stringify(baseEdited.document.layers.base_positions)),zero_is_edited_base:editedBaseOnly.document.mesh.positions.every((p,i)=>p.every((v,k)=>Math.fround(v)===Math.fround(editedBaseOnly.document.layers.base_positions[i][k]))),full_restored_exact:meshHash(baseEdited.document)===meshHash(restoredAfterBase.document),recording_remains_off:(await state()).recording_layer===null}});

  await action('layers.select.1');const beforeDelete=await snapshot('before-delete');await action('layers.delete');await layerCount(1);const deleted=await snapshot('deleted-move-layer');
  const deleteHistory=await history('delete-layer',beforeDelete,deleted);
  await chord('z');const beforeBake=await snapshot('before-bake');assert(projectHash(beforeBake)===projectHash(beforeDelete),'Delete Undo did not restore both layers');
  await action('layers.bake');await layerCount(0);const baked=await snapshot('baked');
  const bakeHistory=await history('bake-layers',beforeBake,baked);await chord('z');const persisted=await snapshot('persisted-two-layers');
  await finishCase({name:'delete-bake-history',delete_history:deleteHistory,bake_history:bakeHistory,gates:{delete_preserves_remaining_delta:hash(JSON.stringify(deleted.document.layers.entries[0].deltas))===hash(JSON.stringify(beforeDelete.document.layers.entries[0].deltas)),bake_retains_visible_mesh:meshHash(beforeBake.document)===meshHash(baked.document),baked_nonlayer_is_v2:baked.document.version===2&&!baked.document.layers,bake_undo_restores_exact_project:projectHash(persisted)===projectHash(beforeBake)}});

  await fresh();await importFile(persisted.path,persisted.document.mesh.positions.length);const loaded=await snapshot('fresh-loaded-layers');
  assert(projectHash(loaded)===projectHash(persisted),'Fresh public Open changed the layer project');
  await sculpt('redo-target-base-edit',1,[0,-110],[15,-100],8);const redoTarget=await snapshot('redo-target');
  await chord('z');const beforeInvalid=await snapshot('before-invalid-with-redo');assert(projectHash(beforeInvalid)===projectHash(loaded),'Undo did not reach loaded project');
  const malformed=structuredClone(beforeInvalid.document);malformed.layers.entries[0].deltas.pop();
  const invalid=path.join(out,'invalid-layer-vector-count.obrush'),invalidText=JSON.stringify(malformed);await writeFile(invalid,invalidText);
  const notice=await importFile(invalid,0,true),afterInvalid=await snapshot('after-invalid-with-redo');
  await chord('z',true);const redoAfterInvalid=await snapshot('redo-after-invalid');
  await finishCase({name:'fresh-open-invalid-preserves-history',invalid_fixture:{path:invalid,sha256:hash(invalidText),corruption:'First layer displacement vector count reduced by one'},notice,gates:{fresh_roundtrip_exact:projectHash(loaded)===projectHash(persisted),invalid_preserves_project:projectHash(afterInvalid)===projectHash(beforeInvalid),invalid_preserves_redo:projectHash(redoAfterInvalid)===projectHash(redoTarget)}});

  const oldFile=path.resolve('.artifacts/move-regression/baseline-2e0bdefc-5c0f-4e09-aedb-c97b8a21e24f/sphere-outbound-initial.obrush'),oldBytes=await readFile(oldFile),oldProject=JSON.parse(oldBytes);
  assert(oldProject.version===2&&!oldProject.layers,'Historical fixture is not a non-layer v2 project');
  await fresh();await importFile(oldFile,oldProject.mesh.positions.length);const oldLoaded=await snapshot('old-v2-loaded');
  await finishCase({name:'historical-v2-import',fixture:{path:oldFile,sha256:hash(oldBytes),bytes:oldBytes.length,provenance:'Immutable independently generated OBrush0.1.6 browser baseline'},gates:{exact_old_project:JSON.stringify(oldLoaded.document)===JSON.stringify(oldProject),no_layers:(await state()).layers.length===0,recording_off:(await state()).recording_layer===null}});

  // Independently authored closed latitude/longitude sphere. Its small mesh
  // makes the actual Add-level, layer locks and bake/navigation checks cheap.
  const longitude=24,latitude=12,positions=[[0,1,0]],triangles=[];
  for(let ring=1;ring<latitude;ring++)for(let column=0;column<longitude;column++){const phi=Math.PI*ring/latitude,theta=2*Math.PI*column/longitude;positions.push([Math.fround(Math.sin(phi)*Math.sin(theta)),Math.fround(Math.cos(phi)),Math.fround(Math.sin(phi)*Math.cos(theta))]);}
  const bottom=positions.length;positions.push([0,-1,0]);
  const vertex=(ring,column)=>1+(ring-1)*longitude+(column%longitude);
  for(let column=0;column<longitude;column++){triangles.push([0,vertex(1,column),vertex(1,column+1)]);triangles.push([bottom,vertex(latitude-1,column+1),vertex(latitude-1,column)]);}
  for(let ring=1;ring<latitude-1;ring++)for(let column=0;column<longitude;column++){const a=vertex(ring,column),b=vertex(ring,column+1),c=vertex(ring+1,column),d=vertex(ring+1,column+1);triangles.push([a,c,b],[b,c,d]);}
  const authored={version:2,name:'Independent layer and subdivision sphere',mesh:{positions,normals:positions.map(p=>{const n=Math.hypot(...p);return p.map(v=>Math.fround(v/n));}),masks:positions.map(()=>0),colors:positions.map(()=>[Math.fround(.65),Math.fround(.69),Math.fround(.74)]),triangles}};
  // JSON.stringify normalizes -0 to +0. Author the exact public JSON token
  // intentionally for the first pole's X component; geometry is unchanged.
  const smallFile=path.join(out,'independent-small-v2.obrush'),smallText=JSON.stringify(authored).replace('"positions":[[0,1,0]','"positions":[[-0.0,1,0]');await writeFile(smallFile,smallText);
  assert(Object.is(JSON.parse(smallText).mesh.positions[0][0],-0),'Authored signed-zero token missing');
  await fresh();await importFile(smallFile,positions.length);const signedImport=await snapshot('small-signed-zero-import');
  await newLayer(1);const signedLayer=await snapshot('small-signed-zero-empty-layer');
  const signedZero={import_preserves_position:Object.is(signedImport.document.mesh.positions[0][0],-0),create_preserves_position:Object.is(signedLayer.document.mesh.positions[0][0],-0),v3_base_preserves_position:Object.is(signedLayer.document.layers.base_positions[0][0],-0)};
  assert(Object.values(signedZero).every(Boolean),'Signed-zero import/create/Save contract failed');
  await action('layers.bake');await layerCount(0);await action('object.add_level');await waitFor(s=>s.subdivision_levels===2&&s.subdivision_level===1,'Actual Add level failed');
  const highBase=await snapshot('small-high-base');await newLayer(1);await sculpt('subdivision-layer-draw',1,[-60,-20],[-35,-5],10);const highLayer=await snapshot('small-high-layer');await stop();const beforeLocks=await snapshot('before-locked-controls');
  const controlsBefore=await state(),lockedKeys=['object.lower','object.add_level','object.refine'];
  for(const actionKey of lockedKeys){assert(controlsBefore.layer_controls_enabled?.[actionKey]===false,'Layer topology/navigation control remains enabled: '+actionKey);await action(actionKey,true);}
  const afterLocks=await snapshot('after-locked-controls');await action('layers.bake');await layerCount(0);const highBaked=await snapshot('small-high-baked');
  const subdivisionBakeHistory=await history('subdivision-layer-bake',beforeLocks,highBaked);
  await action('object.lower');await waitFor(s=>s.subdivision_level===0&&s.subdivision_levels===2,'Bake did not unlock Lower');const low=await snapshot('small-lower-after-bake');
  await action('object.higher');await waitFor(s=>s.subdivision_level===1&&s.subdivision_levels===2,'Higher after Bake failed');const highReturned=await snapshot('small-higher-after-bake');
  result.final_controls=await screenshot('layers-final-controls');
  await finishCase({name:'layers-subdivision-lock-bake-navigation',fixture:{path:smallFile,sha256:hash(smallText),algorithm:'Latitude12/longitude24 closed sphere, outward winding, first pole X=-0.0; actual UI Loop Add produces fine hierarchy'},signed_zero:signedZero,layer_difference:positionDiff(highBase.document.mesh,highLayer.document.mesh),bake_history:subdivisionBakeHistory,gates:{signed_zero_preserved:Object.values(signedZero).every(Boolean),useful_fine_layer:positionDiff(highBase.document.mesh,highLayer.document.mesh).max_displacement>=.001,locked_controls_preserve_project:projectHash(beforeLocks)===projectHash(afterLocks),bake_preserves_active_mesh:meshHash(highBaked.document)===meshHash(beforeLocks.document),navigation_preserves_baked_high:meshHash(highReturned.document)===meshHash(highBaked.document),lower_is_smaller:low.document.mesh.positions.length<highReturned.document.mesh.positions.length,stack_retained:highReturned.document.multiresolution?.levels.length===2}});
  await capturePageDiagnostics();
  result.runtime_errors={gl:result.page_diagnostics.flatMap(page=>page.gl_errors),js:result.page_diagnostics.flatMap(page=>page.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(page=>page.actual_4x_msaa);
  assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Runtime errors or missing actual MSAA');
  result.status='passed seven sculpt-layer browser workflows';
  result.limitations=['Geometry-only dense world-space offsets; masks/colors remain base channels, and no reference layer equivalence is claimed.','Owned headless Chrome and trusted synthetic mouse events; native GUI, physical devices and input-to-photon latency remain unverified.','Exact stored f64 composition is verified independently; inferred half interpolation allows3e-7 component error from f32 rounding.','Fresh public file input and captured download bytes are exercised; physical file-picker/download persistence is outside this harness.','Layer identity is its ordered stack index. Navigation/topology stay locked while layers exist; existing subdivision levels are retained after Bake.'];
} catch (error) {
  result.status = 'failed sculpt-layer browser workflow'; result.failure = String(error); process.exitCode = 1;
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
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json'), wasm: result.wasm, cases: result.cases.map(trial=>({name:trial.name,gates:trial.gates})) }));
}
