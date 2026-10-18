// CLI-only morph-target geometry and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-morph.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'morph';
const out = path.resolve(`.artifacts/morph-browser-0.1.9/${label}-${randomUUID()}`);
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
  preregistered_contract:{draw_radius:.18,draw_strength:.35,symmetry_x:true,draw_min_displacement:.002,local_morph_min_changed_vertices:5,local_morph_min_squared_error_reduction_fraction:.01,local_morph_max_vertex_squared_error_increase:1e-12,local_morph_viewport_trace:[[-90,-40],[-50,-10]],stored_f64_composition:'For each component, start at stored f64 base; add visible Math.fround(intensity)*offset contributions only when nonzero, in entry order; Math.fround result and compare exact f32 bits including signed zero',history:'Exact canonical captured project bytes and packed mesh bits',gpu:'Exact packed f32 vertex-buffer digest; index buffers are not read back, and ordered triangles are checked separately in public Save data',release_probe_incremental_delays_ms:[0,200,800],release_probe_timestamp:'Probe start before asynchronous vertex-buffer readback and sculpt-metric observation; not finalization completion or input-to-photon latency'},
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
 const p=window.__brushCheck={events:[],wheels:[],keys:[],raf:[],longtasks:[],uploads:[],contexts:[],allocations:[],viewports:{},gl_errors:[],js_errors:[]};
 new PerformanceObserver(list=>{for(const task of list.getEntries())p.longtasks.push({at:task.startTime,duration:task.duration});}).observe({entryTypes:['longtask']});
 const raf=window.requestAnimationFrame.bind(window);let last;raf(function tick(at){p.raf.push({at,gap:last===undefined?0:at-last});last=at;raf(tick);});
 for(const type of ['keydown','keyup'])window.addEventListener(type,event=>p.keys.push({type,key:event.key,meta:event.metaKey,ctrl:event.ctrlKey,shift:event.shiftKey,trusted:event.isTrusted,target:event.target.id}),true);
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,trusted:event.isTrusted,at:performance.now()}),true);
 window.addEventListener('wheel',event=>p.wheels.push({x:event.clientX,y:event.clientY,delta_x:event.deltaX,delta_y:event.deltaY,trusted:event.isTrusted,at:performance.now()}),true);
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
  const page = await evaluate(`(()=>{const p=window.__brushCheck;if(!p)return null;return {url:location.href,at:performance.now(),gl_errors:p.gl_errors,js_errors:p.js_errors,contexts:p.contexts,allocations:p.allocations,wheels:p.wheels,scroll_pointer_events:p.events.filter(event=>event.type==='pointermove'&&p.wheels.some(wheel=>wheel.at>=event.at&&wheel.at-event.at<150))};})()`);
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
    const s=await state(),rect=s.morph_controls?.[key]||s.layer_controls?.[key];
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&(s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key])===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key]};
    }
    const deltaY=point[1]>955?350:-350;
    const scrollAction={key:'scroll-for.'+key,pointer:[point[0],750],before_rect:rect,delta_y:deltaY,at_epoch_ms:Date.now()};result.ui_actions.push(scrollAction);
    // CDP wheel dispatch does not move the pointer. egui routes wheel input
    // using its last pointer position, so establish hover in the actual panel.
    await pointer('mouseMoved',point[0],750);await pause(40);
    const wheel={type:'mouseWheel',x:point[0],y:750,deltaX:0,deltaY};result.input_trace.push({...wheel,at_epoch_ms:Date.now()});
    await send('Input.dispatchMouseEvent',wheel);
    let moved=false;
    for(let attempt=0;attempt<12;attempt++){
      await pause(50);const after=await state(),next=after.morph_controls?.[key]||after.layer_controls?.[key];
      if(next&&(deltaY>0?next[1]<rect[1]:next[1]>rect[1])){scrollAction.after_rect=next;moved=true;break;}
    }
    assert(moved,'Actual panel wheel did not move control toward visibility: '+key);await pause(150);
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
const sculpt=async(name,number,offset,endOffset,steps=12,options={})=>{
  const pointerType=options.pointer_type||'mouse',force=options.force??.5;
  await key(number);await evaluate('document.getElementById("obrush_canvas").focus()');const settings=await state();
  assert(settings.kind===(number===0?'Morph':number===7?'Move':'Draw'),'Actual keyboard brush selection failed for '+name);
  assert(Math.abs(settings.radius-.18)<1e-6&&Math.abs(settings.strength-.35)<1e-6&&settings.symmetry_x,'Unexpected actual brush parameters: '+name);
  const v=await viewport();
  const from=[v.cx+offset[0],v.cy+offset[1]],to=[v.cx+endOffset[0],v.cy+endOffset[1]],start=await evaluate('performance.now()');
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,pointerType,force,true);await pause(70);
  for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,pointerType,force,true);await pause(35);}
  await pointer('mouseReleased',...to,pointerType,0,false);const probes=[];
  for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()')});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);
  assert(quiet,'Geometry or pending input changed after release: '+name);
  const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);
  const trial={name,brush:number,settings,pointer_type:pointerType,force,from,to,steps,spacing_ms:35,viewport:v,start,end,probes,quiet,runtime,after:await state()};result.strokes.push(trial);return trial;
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
  const record={...metadata(saved),metrics,gpu,gpu_matches_saved_mesh:gpu.sha256===packedHash(saved.document.mesh),composition:composition(saved.document),morph:targetContract(saved.document,metrics)};
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
const finishCase=async(record)=>{result.cases.push(record);await writeFile(path.join(out,record.name+'.json'),JSON.stringify(record,null,2)+'\n');await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:record.name,gates:record.gates}));assert(Object.values(record.gates).every(Boolean),'Morph case failed: '+record.name);};
const positionBits = positions => {
  const bytes=Buffer.alloc(positions.length*12);
  for(let i=0;i<positions.length;i++)for(let k=0;k<3;k++)bytes.writeFloatLE(positions[i][k],i*12+k*4);
  return hash(bytes);
};
const targetHash = document => {
  const t=document.morph_target;if(!t)return null;
  return hash(JSON.stringify({scheme:t.scheme,creation_level:t.creation_level,triangles:t.triangles,positions_f32_sha256:positionBits(t.positions)}));
};
const positionsEqual=(a,b)=>a.length===b.length&&positionBits(a)===positionBits(b);
const channelsEqual=(a,b)=>JSON.stringify(a.masks)===JSON.stringify(b.masks)&&JSON.stringify(a.colors)===JSON.stringify(b.colors);
const targetContract=(document,metrics)=>{
  const activeLevel=document.multiresolution?.active_level??0;
  assert(metrics.subdivision_level===activeLevel,'Read-only active level differs from public Save metadata');
  const t=document.morph_target,observed=metrics.morph_target;
  if(!t){assert(document.version!==4,'v4 Save has no target');assert(observed===null,'Target diagnostics remain after target deletion');return {present:false};}
  assert(document.version===4&&t.scheme==='positions_f32_v1','Unexpected public target version/scheme');
  assert(Number.isSafeInteger(t.creation_level)&&t.creation_level>=0,'Invalid target creation level');
  // The established hierarchy format stores its active mesh separately in
  // document.mesh and leaves levels[active_level] null at every level index.
  const source=t.creation_level===activeLevel?document.mesh:document.multiresolution?.levels[t.creation_level];
  assert(source&&source.positions.length===t.positions.length&&JSON.stringify(source.triangles)===JSON.stringify(t.triangles),'Stored target has no matching creation-level correspondence');
  const available=activeLevel===t.creation_level&&document.mesh.positions.length===t.positions.length&&JSON.stringify(document.mesh.triangles)===JSON.stringify(t.triangles);
  assert(observed?.available===available&&observed.creation_level===t.creation_level&&observed.vertices===t.positions.length&&observed.triangles===t.triangles.length,'Read-only target diagnostics differ from independently inferred availability');
  return {present:true,scheme:t.scheme,creation_level:t.creation_level,available,position_bits_sha256:positionBits(t.positions),target_sha256:targetHash(document)};
};
const morphError=(before,after,target)=>{
  assert(before.positions.length===after.positions.length&&after.positions.length===target.positions.length,'Morph changed fixed topology');
  assert(JSON.stringify(before.triangles)===JSON.stringify(after.triangles)&&JSON.stringify(after.triangles)===JSON.stringify(target.triangles),'Morph changed ordered triangles');
  let beforeSquared=0,afterSquared=0,changed=0,masked=0,maskedChanged=0,maxIncrease=0;
  for(let i=0;i<before.positions.length;i++){
    let oldDistance=0,newDistance=0;
    for(let k=0;k<3;k++){const t=Math.fround(target.positions[i][k]);oldDistance+=(Math.fround(before.positions[i][k])-t)**2;newDistance+=(Math.fround(after.positions[i][k])-t)**2;}
    const didChange=!positionsEqual([before.positions[i]],[after.positions[i]]);
    beforeSquared+=oldDistance;afterSquared+=newDistance;changed+=Number(didChange);maxIncrease=Math.max(maxIncrease,newDistance-oldDistance);
    if(before.masks[i]===1){masked++;maskedChanged+=Number(didChange);}
  }
  return {before_squared_error:beforeSquared,after_squared_error:afterSquared,reduction_fraction:beforeSquared?1-afterSquared/beforeSquared:0,changed_vertices:changed,fully_masked_vertices:masked,fully_masked_changed:maskedChanged,max_vertex_squared_error_increase:maxIncrease};
};
const available=async(expected)=>waitFor(s=>s.morph_target?.available===expected,'Target availability did not become '+expected);
const store=async()=>{await action('morph.store');const s=await available(true);assert(s.adaptive_detail===false,'Usable target did not disable adaptive detail');return s;};
const publicFixture=async(name,document)=>{
  const file=path.join(out,name+'.obrush'),text=JSON.stringify(document);await writeFile(file,text);return {path:file,sha256:hash(text),bytes:Buffer.byteLength(text)};
};
const enable=async(key,field,value)=>{if((await state())[field]!==value)await action(key);return waitFor(s=>s[field]===value,'Actual checkbox did not change '+field);};
const legacySemantic=(a,b)=>JSON.stringify(a)===JSON.stringify(b)&&packedHash(a.mesh)===packedHash(b.mesh);
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

  await fresh();result.initial_controls=await screenshot('initial-controls');
  assert((await state()).morph_target===null,'Fresh sphere unexpectedly has a target');
  const initial=await snapshot('fresh-sphere');await store();const stored=await snapshot('stored-sphere');
  assert(positionsEqual(initial.document.mesh.positions,stored.document.morph_target.positions),'Store differs from exact original f32 positions');
  assert(packedHash(initial.document.mesh)===packedHash(stored.document.mesh),'Store changed actual visible mesh');
  await sculpt('useful-draw-with-target',1,[-90,-40],[-50,-10],14);const drawn=await snapshot('drawn-target-sphere');
  const drawDifference=positionDiff(stored.document.mesh,drawn.document.mesh);assert(drawDifference.max_displacement>=.002,'Draw with a usable target is empty');
  assert(targetHash(stored.document)===targetHash(drawn.document),'Draw modified stored target');
  await action('morph.restore');const restored=await snapshot('fully-restored-sphere');
  const restoreHistory=await history('restore-target',drawn,restored);
  await chord('z');await chord('z');const beforeNoop=await snapshot('before-noop-restore-with-redo');
  assert(projectHash(beforeNoop)===projectHash(stored),'Undo Draw did not reach stored sphere');
  await action('morph.restore');const noopRestore=await snapshot('noop-restore-with-redo');await chord('z',true);const redoAfterNoop=await snapshot('redo-after-noop-restore');
  await finishCase({name:'store-draw-restore-history',draw_difference:drawDifference,history:restoreHistory,gates:{store_exact_positions:positionsEqual(initial.document.mesh.positions,stored.document.morph_target.positions),draw_keeps_target_bits:targetHash(stored.document)===targetHash(drawn.document),restore_exact_target_positions:positionsEqual(restored.document.mesh.positions,stored.document.morph_target.positions),restore_keeps_base_channels:channelsEqual(drawn.document.mesh,restored.document.mesh),noop_restore_preserves_project:projectHash(beforeNoop)===projectHash(noopRestore),noop_restore_preserves_redo:projectHash(redoAfterNoop)===projectHash(drawn)}});

  const beforeSwap=await snapshot('before-swap');await action('morph.swap');const swapped=await snapshot('swapped-once');
  const swapHistory=await history('swap-target',beforeSwap,swapped);await action('morph.swap');const swapTwice=await snapshot('swapped-twice');
  await action('morph.delete');const deleted=await snapshot('target-deleted');const deleteHistory=await history('delete-target',swapTwice,deleted);await chord('z');const targetReturned=await snapshot('target-returned');
  await finishCase({name:'swap-delete-history',swap_history:swapHistory,delete_history:deleteHistory,gates:{swap_mesh_is_previous_target:positionsEqual(swapped.document.mesh.positions,beforeSwap.document.morph_target.positions),swap_target_is_previous_mesh:positionsEqual(swapped.document.morph_target.positions,beforeSwap.document.mesh.positions),double_swap_exact_positions:positionsEqual(swapTwice.document.mesh.positions,beforeSwap.document.mesh.positions),double_swap_exact_target:targetHash(swapTwice.document)===targetHash(beforeSwap.document),swap_preserves_base_channels:channelsEqual(swapped.document.mesh,beforeSwap.document.mesh),delete_has_no_target:!deleted.document.morph_target,delete_undo_restores_canonical_project:projectHash(targetReturned)===projectHash(swapTwice)}});

  // The authored mask changes only a base channel on a known displaced surface.
  // Its target is the actual independently saved original sphere, never a core
  // implementation result generated solely for the local-Morph expectation.
  const masked=structuredClone(drawn.document);masked.mesh.masks=masked.mesh.positions.map(p=>Math.fround(p[0])<0?1:0);
  const maskedFixture=await publicFixture('authored-half-masked-target',masked);
  await fresh();await importFile(maskedFixture.path,masked.mesh.positions.length);await action('brush.morph');
  assert((await state()).kind==='Morph','Actual Morph brush control did not select Morph');
  const maskedBefore=await snapshot('masked-before-morph');await sculpt('masked-local-morph',0,[-90,-40],[-50,-10],12);const maskedAfter=await snapshot('masked-after-morph');
  const maskedError=morphError(maskedBefore.document.mesh,maskedAfter.document.mesh,maskedBefore.document.morph_target);
  const localHistory=await history('local-morph',maskedBefore,maskedAfter);
  assert(maskedError.changed_vertices>=5&&maskedError.reduction_fraction>=.01,'Local Morph has no useful decrease towards target');
  const radiusTrials=[];
  for(const force of [1,.5]){
    await fresh();await importFile(maskedFixture.path,masked.mesh.positions.length);await enable('settings.pressure_radius','pressure_radius',true);await enable('settings.pressure_strength','pressure_strength',true);await enable('settings.symmetry','symmetry_x',true);
    const before=await snapshot('pressure-'+force+'-before');const stroke=await sculpt('pressure-radius-'+force,0,[-90,-40],[-50,-10],12,{pointer_type:'pen',force});const after=await snapshot('pressure-'+force+'-after');
    const error=morphError(before.document.mesh,after.document.mesh,before.document.morph_target);
    const contact=stroke.runtime.events.filter(e=>e.buttons===1&&e.pointer_type==='pen');
    assert(contact.length>0&&contact.every(e=>e.trusted&&Math.abs(e.pressure-force)<1e-6),'Actual pen PointerEvents lost captured force');
    radiusTrials.push({force,error,history:await history('pressure-morph-'+force,before,after),target_unchanged:targetHash(before.document)===targetHash(after.document),channels_unchanged:channelsEqual(before.document.mesh,after.document.mesh)});
  }
  result.local_morph_png=await screenshot('local-morph-pressure-half');
  await finishCase({name:'local-morph-masks-pressure-radius',fixture:maskedFixture,error:maskedError,history:localHistory,pressure_trials:radiusTrials,gates:{target_error_decreases:maskedError.reduction_fraction>=.01,useful_local_change:maskedError.changed_vertices>=5,fully_masked_positions_exact:maskedError.fully_masked_vertices>0&&maskedError.fully_masked_changed===0,no_new_target_error:maskedError.max_vertex_squared_error_increase<=1e-12,base_channels_exact:channelsEqual(maskedBefore.document.mesh,maskedAfter.document.mesh),target_bits_exact:targetHash(maskedBefore.document)===targetHash(maskedAfter.document),half_radius_footprint_smaller:radiusTrials[1].error.changed_vertices>0&&radiusTrials[1].error.changed_vertices<radiusTrials[0].error.changed_vertices,half_force_moves_less:radiusTrials[1].error.reduction_fraction<radiusTrials[0].error.reduction_fraction,pressure_protection_exact:radiusTrials.every(t=>t.error.fully_masked_changed===0&&t.target_unchanged&&t.channels_unchanged)}});

  await fresh();await importFile(drawn.path,drawn.document.mesh.positions.length);const loaded=await snapshot('fresh-loaded-v4');
  await sculpt('v4-redo-target-draw',1,[0,-110],[15,-100],8);const redoTarget=await snapshot('v4-redo-target');await chord('z');const beforeInvalid=await snapshot('v4-before-invalid-with-redo');
  const zeroStroke=await sculpt('zero-pressure-morph',0,[65,-25],[90,-15],8,{pointer_type:'pen',force:0});const afterZero=await snapshot('v4-after-zero-pressure-with-redo');
  assert(zeroStroke.runtime.events.some(e=>e.pointer_type==='pen'&&e.buttons===1&&e.pressure===0),'Zero-force pen contact was not actually observed');
  const malformed=structuredClone(beforeInvalid.document);malformed.morph_target.positions.pop();const invalidFixture=await publicFixture('invalid-target-count',malformed);
  const notice=await importFile(invalidFixture.path,0,true),afterInvalid=await snapshot('v4-after-invalid-with-redo');await chord('z',true);const redoAfterInvalid=await snapshot('v4-redo-after-invalid');
  await finishCase({name:'v4-fresh-open-invalid-noop-redo',invalid_fixture:invalidFixture,notice,gates:{fresh_canonical_bytes_exact:projectHash(loaded)===projectHash(drawn),zero_pressure_project_exact:projectHash(afterZero)===projectHash(beforeInvalid),invalid_preserves_project:projectHash(afterInvalid)===projectHash(beforeInvalid),invalid_preserves_redo:projectHash(redoAfterInvalid)===projectHash(redoTarget)}});

  const legacyFixtures=[['v2','.artifacts/move-regression/baseline-2e0bdefc-5c0f-4e09-aedb-c97b8a21e24f/sphere-outbound-initial.obrush'],['v3','.artifacts/layer-browser-0.1.8/verified-6480c153-0de7-43b2-8b27-dd21b0414dd2/persisted-two-layers.obrush']];
  const legacy=[];
  for(const [version,file]of legacyFixtures){const bytes=await readFile(file),project=JSON.parse(bytes);assert(project.version===Number(version.slice(1))&&!project.morph_target,'Wrong historical fixture version');await fresh();await importFile(path.resolve(file),project.mesh.positions.length);const saved=await snapshot('legacy-'+version);legacy.push({version,path:path.resolve(file),sha256:hash(bytes),semantic_and_gpu_mesh_exact:legacySemantic(saved.document,project),no_target:(await state()).morph_target===null});}
  await finishCase({name:'legacy-v2-v3-open',fixtures:legacy,gates:{v2_semantic_fields_exact:legacy[0].semantic_and_gpu_mesh_exact,v3_semantic_fields_exact:legacy[1].semantic_and_gpu_mesh_exact,no_legacy_targets:legacy.every(t=>t.no_target)}});

  const smallFile=path.resolve('.artifacts/layer-browser-0.1.8/verified-6480c153-0de7-43b2-8b27-dd21b0414dd2/independent-small-v2.obrush'),smallBytes=await readFile(smallFile),small=JSON.parse(smallBytes);
  assert(Object.is(small.mesh.positions[0][0],-0),'Independent signed-zero fixture lost its token');
  await fresh();await importFile(smallFile,small.mesh.positions.length);await store();const beforeRefine=await snapshot('small-before-refine');
  await action('object.refine');await waitFor(s=>s.morph_target===null&&s.vertices>small.mesh.positions.length,'Explicit Refine did not invalidate target');const refined=await snapshot('small-refined-target-invalid');
  const refineHistory=await history('explicit-refine-target-invalidation',beforeRefine,refined);await chord('z');const refineUndone=await snapshot('small-refine-undone');
  await finishCase({name:'explicit-topology-invalidates-undo-restores',history:refineHistory,gates:{refine_changed_topology:refined.document.mesh.positions.length>beforeRefine.document.mesh.positions.length,target_dropped_atomically:!refined.document.morph_target,undo_restores_canonical_target_project:projectHash(refineUndone)===projectHash(beforeRefine)}});

  await fresh();await importFile(smallFile,small.mesh.positions.length);const signedImported=await snapshot('signed-zero-import');await store();const coarseStored=await snapshot('signed-zero-coarse-target');
  await action('object.add_level');await waitFor(s=>s.subdivision_levels===2&&s.subdivision_level===1&&s.morph_target?.available===false,'Add level did not preserve unavailable original-level target');const high=await snapshot('high-with-unavailable-coarse-target');
  const disabledRestore=(await state()).morph_controls_enabled?.['morph.restore'];await action('morph.restore',true);const unavailableRestore=await snapshot('unavailable-restore-attempt');
  await action('object.lower');await waitFor(s=>s.subdivision_level===0&&s.morph_target?.available===true,'Lower did not make original target available');const low=await snapshot('low-with-available-target');
  await action('object.higher');await waitFor(s=>s.subdivision_level===1&&s.morph_target?.available===false,'Higher did not return unavailable target');const highReturned=await snapshot('high-returned-target-retained');
  await finishCase({name:'signed-zero-subdivision-target-origin',fixture:{path:smallFile,sha256:hash(smallBytes),provenance:'Independently authored latitude12/longitude24 closed sphere, first pole X=-0.0'},gates:{signed_zero_import:Object.is(signedImported.document.mesh.positions[0][0],-0),signed_zero_stored_target:Object.is(coarseStored.document.morph_target.positions[0][0],-0),creation_level_zero:coarseStored.document.morph_target.creation_level===0,target_bits_retained_all_levels:[high,low,highReturned].every(s=>targetHash(s.document)===targetHash(coarseStored.document)),unavailable_restore_disabled_noop:disabledRestore===false&&projectHash(unavailableRestore)===projectHash(high),lower_positions_exact:positionsEqual(low.document.mesh.positions,coarseStored.document.mesh.positions),higher_canonical_project_exact:projectHash(highReturned)===projectHash(high)}});

  await store();const fineTarget=await snapshot('fine-stored-target');await newLayer(1);const emptyLayer=await snapshot('fine-empty-recording-layer');
  await sculpt('morph-layer-recorded-draw',1,[-60,-20],[-35,-5],10);const recordedLayer=await snapshot('fine-recorded-layer');
  assert(positionDiff(emptyLayer.document.mesh,recordedLayer.document.mesh).max_displacement>=.001,'Coexisting layer Draw was empty');
  await sculpt('morph-layer-local-return',0,[-60,-20],[-35,-5],10);const morphedLayer=await snapshot('fine-morphed-layer');const layerError=morphError(recordedLayer.document.mesh,morphedLayer.document.mesh,fineTarget.document.morph_target);
  const morphLayerHistory=await history('recorded-layer-local-morph',recordedLayer,morphedLayer);
  await action('morph.restore');const restoredLayer=await snapshot('fine-layer-full-restored');const restoreLayerHistory=await history('recorded-layer-full-restore',morphedLayer,restoredLayer);
  await chord('z');const beforeLayerSwap=await snapshot('fine-layer-before-swap');await action('morph.swap');const layerSwapped=await snapshot('fine-layer-swapped');const layerSwapHistory=await history('recorded-layer-swap',beforeLayerSwap,layerSwapped);await action('morph.swap');const layerSwapTwice=await snapshot('fine-layer-swap-twice');
  await action('layers.bake');await layerCount(0);const layerBaked=await snapshot('fine-layer-baked-target-retained');const layerBakeHistory=await history('coexisting-layer-bake',layerSwapTwice,layerBaked);
  result.final_controls=await screenshot('morph-final-controls');
  await finishCase({name:'geometry-layer-coexistence',local_error:layerError,morph_history:morphLayerHistory,restore_history:restoreLayerHistory,swap_history:layerSwapHistory,bake_history:layerBakeHistory,gates:{local_error_decreases:layerError.changed_vertices>0&&layerError.reduction_fraction>=.01,recording_stays_selected:morphedLayer.document.layers.recording_layer===0,full_restore_exact_target_positions:positionsEqual(restoredLayer.document.mesh.positions,fineTarget.document.morph_target.positions),restore_preserves_target_bits:targetHash(restoredLayer.document)===targetHash(fineTarget.document),double_swap_exact_positions:positionsEqual(layerSwapTwice.document.mesh.positions,beforeLayerSwap.document.mesh.positions),double_swap_exact_target:targetHash(layerSwapTwice.document)===targetHash(beforeLayerSwap.document),base_channels_unchanged:channelsEqual(emptyLayer.document.mesh,layerSwapTwice.document.mesh),bake_keeps_target_available:targetHash(layerBaked.document)===targetHash(layerSwapTwice.document)&&(await state()).morph_target.available===true,bake_keeps_visible_mesh:packedHash(layerBaked.document.mesh)===packedHash(layerSwapTwice.document.mesh)}});
  await capturePageDiagnostics();
  result.runtime_errors={gl:result.page_diagnostics.flatMap(page=>page.gl_errors),js:result.page_diagnostics.flatMap(page=>page.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(page=>page.actual_4x_msaa);
  assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Runtime errors or missing actual MSAA');
  result.status='passed eight morph-target browser workflows';
  result.limitations=['One geometry-only f32 position target, bound to exact vertex/ordered-face correspondence at its creation subdivision level; no reference equivalence is claimed.','Owned headless Chrome and trusted synthetic mouse/pen events; native GUI, physical devices and input-to-photon latency remain unverified.','Target and visible position bits, canonical captured Save bytes, GPU buffers and named history operations are checked independently; masks/colors remain base channels.','Fresh public file input and captured download bytes are exercised; physical file-picker/download persistence is outside this harness.','Explicit topology changes invalidate the target atomically; unavailable targets remain stored across subdivision navigation.'];
} catch (error) {
  result.status='failed morph-target browser workflow';result.failure=String(error);process.exitCode=1;
  try {result.failure_runtime=await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})');result.failure_screenshot=await screenshot('failure');}catch{}
} finally {
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);
  try {if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}
  for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('Owned headless session closing'));}pending.clear();
  await pause(400);if(child.exitCode===null)child.kill('SIGTERM');await pause(200);await new Promise(resolve=>server.close(resolve));
  result.cleanup={owned_chrome_exit_code:child.exitCode,owned_chrome_signal:child.signalCode,owned_server_closed:true};await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),wasm:result.wasm,cases:result.cases.map(trial=>({name:trial.name,gates:trial.gates}))}));
}
