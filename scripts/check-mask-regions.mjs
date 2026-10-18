// CLI-only rectangle/lasso region projection and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-mask-regions.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'mask-regions';
const out = path.resolve(`.artifacts/mask-region-browser/${label}-${randomUUID()}`);
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
  preregistered_contract:{mask_policy:'Exactf32bits: selected1orAlt-press0, outsideoldbits; throughdepth active-only; finite frozenf32projection evaluatedf64 withw>0/inclusiveclip before division',lasso:'Finite-segment nearest distance<=1e-7screenunits or evenodd crossings; ordered repeats retained, implicit closure',rectangle:'Exact inclusive bounds; reversed corners; zeroextentempty',geometry:'Current/retained geometry,normals,RGB,orderedfaces,layerf64,target/inactiveexact; retainedmasktransport excluded',preview:'Geometry/masks/bufferidentities/counts/uploads anddirty/revisions unchanged beforeactualrelease',limits:'512rawlassopoints/32Mconservativework; only512overflow browsertested, source32M math coreonly',bounds:'Horizontaltoolbox/inspector, fullenabledgizmocontainment',release:'150mssettle then0/200/800quietprobes; no firstcommit/physical latency bound',pipeline:'One40Kdefaultsphere rectangle→Grow1/.5→Blur1/.5→Extract.5/.02'},
  protocol_reference: 'https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-dispatchMouseEvent',
  cases: [], comparisons: {}, exceptions: [], console_errors: [], blocked_external_requests: [], input_trace: [],
  ui_actions: [], strokes: [], snapshots: [], page_diagnostics: [], ui_bounds_checks: [],
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
 const p=window.__brushCheck={events:[],wheels:[],keys:[],raf:[],longtasks:[],uploads:[],contexts:[],allocations:[],viewports:{},gl_errors:[],js_errors:[],scene_frame:null};
 new PerformanceObserver(list=>{for(const task of list.getEntries())p.longtasks.push({at:task.startTime,duration:task.duration});}).observe({entryTypes:['longtask']});
 const raf=window.requestAnimationFrame.bind(window);let last;raf(function tick(at){p.raf.push({at,gap:last===undefined?0:at-last});last=at;raf(tick);});
 for(const type of ['keydown','keyup'])window.addEventListener(type,event=>p.keys.push({type,key:event.key,meta:event.metaKey,ctrl:event.ctrlKey,shift:event.shiftKey,trusted:event.isTrusted,target:event.target.id}),true);
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,button:event.button,alt:event.altKey,ctrl:event.ctrlKey,shift:event.shiftKey,meta:event.metaKey,trusted:event.isTrusted,at:performance.now()}),true);
 window.addEventListener('wheel',event=>p.wheels.push({x:event.clientX,y:event.clientY,delta_x:event.deltaX,delta_y:event.deltaY,trusted:event.isTrusted,at:performance.now()}),true);
 window.addEventListener('error',event=>p.js_errors.push(String(event.error||event.message)));window.addEventListener('unhandledrejection',event=>p.js_errors.push(String(event.reason)));
 const get=HTMLCanvasElement.prototype.getContext;
 HTMLCanvasElement.prototype.getContext=function(kind,...args){const gl=get.call(this,kind,...args);if(kind==='webgl2'&&gl&&!gl.__sceneObserved){
  gl.__sceneObserved=true;window.__brushGl=gl;gl.__sceneVaos=new Map();gl.__sceneBuffers=new Map();gl.__sceneElements=new Map();gl.__sceneLocations=new Map();let bufferSequence=0,vaoSequence=0;
  p.contexts.push({kind,version:gl.getParameter(gl.VERSION),max_samples:gl.getParameter(gl.MAX_SAMPLES)});
  const info=buffer=>{if(!gl.__sceneBuffers.has(buffer))gl.__sceneBuffers.set(buffer,{key:++bufferSequence,bytes:0,uploads:0,last_upload_at:null});return gl.__sceneBuffers.get(buffer);};
  const bindVao=gl.bindVertexArray;gl.bindVertexArray=function(vao){this.__sceneVao=vao;return bindVao.call(this,vao);};
  const bind=gl.bindBuffer;gl.bindBuffer=function(target,buffer){if(target===this.ARRAY_BUFFER)this.__sceneArrayBuffer=buffer;if(target===this.ELEMENT_ARRAY_BUFFER)this.__sceneElements.set(this.__sceneVao,buffer);return bind.call(this,target,buffer);};
  const attribute=gl.vertexAttribPointer;gl.vertexAttribPointer=function(index,size,type,normalized,stride,offset){if(stride===40&&offset===0&&size===3){this.__sceneVaos.set(this.__sceneVao,{key:++vaoSequence,vertices:this.__sceneArrayBuffer});info(this.__sceneArrayBuffer);}return attribute.call(this,index,size,type,normalized,stride,offset);};
  const data=gl.bufferData;gl.bufferData=function(target,bytes,...args){const value=data.call(this,target,bytes,...args);if(target===this.ARRAY_BUFFER||target===this.ELEMENT_ARRAY_BUFFER){const buffer=target===this.ARRAY_BUFFER?this.__sceneArrayBuffer:this.__sceneElements.get(this.__sceneVao);const record=info(buffer);record.bytes=typeof bytes==='number'?bytes:bytes?.byteLength||0;record.uploads++;record.last_upload_at=performance.now();p.uploads.push({key:record.key,target,bytes:record.bytes,uploads:record.uploads,at:record.last_upload_at});}return value;};
  const location=gl.getUniformLocation;gl.getUniformLocation=function(program,name){const loc=location.call(this,program,name);if(loc)this.__sceneLocations.set(loc,name);return loc;};
  const one=gl.uniform1i;gl.uniform1i=function(loc,value){const name=this.__sceneLocations.get(loc);if(name==='u_active')this.__sceneActive=value;if(name==='u_wire')this.__sceneWire=value;return one.call(this,loc,value);};
  const three=gl.uniform3f;gl.uniform3f=function(loc,x,y,z){if(this.__sceneLocations.get(loc)==='u_preview_translation')this.__scenePreview=[x,y,z];return three.call(this,loc,x,y,z);};
  const threev=gl.uniform3fv;gl.uniform3fv=function(loc,v){if(this.__sceneLocations.get(loc)==='u_preview_translation')this.__scenePreview=Array.from(v);return threev.call(this,loc,v);};
  const matrix=gl.uniformMatrix4fv;gl.uniformMatrix4fv=function(loc,transpose,value){const name=this.__sceneLocations.get(loc);if(name==='u_view_projection')p.view_projection=Array.from(value);if(name==='u_view')p.view=Array.from(value);return matrix.call(this,loc,transpose,value);};
  const clear=gl.clear;gl.clear=function(mask){if(mask&this.DEPTH_BUFFER_BIT)p.scene_frame={at:performance.now(),draws:[]};return clear.call(this,mask);};
  const draw=gl.drawElements;gl.drawElements=function(mode,count,type,offset){const vao=this.__sceneVaos.get(this.__sceneVao);if(vao&&mode===this.TRIANGLES&&!this.__sceneWire){const vertices=info(vao.vertices),element=this.__sceneElements.get(this.__sceneVao),indices=info(element);const observed={vao_key:vao.key,vertex_key:vertices.key,index_key:indices.key,count,type,offset,active:!!this.__sceneActive,preview_translation:this.__scenePreview?.slice()||[0,0,0],at:performance.now(),vao:this.__sceneVao,vertices:vao.vertices,indices:element};if(p.scene_frame)p.scene_frame.draws.push(observed);if(observed.active)window.__brushMeshVbo={bytes:vertices.bytes};}return draw.call(this,mode,count,type,offset);};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;if(x>0&&width>400&&height>300)p.last_sculpt_viewport=[x,y,width,height];return viewport.call(this,x,y,width,height);};
  const storage=gl.renderbufferStorageMultisample;gl.renderbufferStorageMultisample=function(target,samples,format,width,height){const value=storage.call(this,target,samples,format,width,height);p.allocations.push({samples,actual_samples:this.getRenderbufferParameter(target,this.RENDERBUFFER_SAMPLES),width,height});return value;};
  const error=gl.getError;gl.getError=function(){const value=error.call(this);if(value!==this.NO_ERROR)p.gl_errors.push(value);return value;};
  window.__readSceneGpu=async()=>{
   const frame=p.scene_frame;if(!frame)throw Error('No actual scene draw frame');
   const observed_at=performance.now(),metrics=window.obrushBrushMetrics?.(),sculpt=window.obrushSculptMetrics?.();
   const previousArray=gl.__sceneArrayBuffer,previousVao=gl.__sceneVao,raw=[];
   // Copy all visible buffers and metadata synchronously in one event-loop turn.
   // Restore GL state before asynchronous SHA-256, so readback cannot straddle
   // an application commit or leave bindings altered during a rendered frame.
   for(const draw of frame.draws){
    const vi={...gl.__sceneBuffers.get(draw.vertices)},ii={...gl.__sceneBuffers.get(draw.indices)},vertices=new Uint8Array(vi.bytes),indices=new Uint8Array(ii.bytes);
    gl.bindBuffer(gl.ARRAY_BUFFER,draw.vertices);gl.getBufferSubData(gl.ARRAY_BUFFER,0,vertices);
    gl.bindVertexArray(draw.vao);const oldElement=gl.__sceneElements.get(draw.vao);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,draw.indices);gl.getBufferSubData(gl.ELEMENT_ARRAY_BUFFER,0,indices);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,oldElement);
    raw.push({meta:{vao_key:draw.vao_key,vertex_key:draw.vertex_key,index_key:draw.index_key,active:draw.active,preview_translation:draw.preview_translation.slice(),count:draw.count,type:draw.type,offset:draw.offset,vertex_bytes:vertices.length,index_bytes:indices.length,vertex_uploads:vi.uploads,index_uploads:ii.uploads,last_upload_at:vi.last_upload_at},vertices,indices});
   }
   gl.bindVertexArray(previousVao);gl.bindBuffer(gl.ARRAY_BUFFER,previousArray);
   const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(b=>b.toString(16).padStart(2,'0')).join('');
   const draws=[];for(const record of raw)draws.push({...record.meta,vertex_sha256:await digest(record.vertices),index_sha256:await digest(record.indices)});
   return {observed_at,metrics,sculpt,frame_at:frame.at,draws,view_projection:p.view_projection,view:p.view,viewport:p.last_sculpt_viewport};
  };
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
const gpuDigest=async()=>{const gpu=await evaluate('window.__readSceneGpu()');gpu.sha256=hash(JSON.stringify(gpu.draws.map(d=>[d.active,d.vertex_sha256,d.index_sha256,d.count]).sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)))));return gpu;};
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
      result.canvas_rect = await evaluate('(()=>{const r=document.getElementById("obrush_canvas").getBoundingClientRect();return [r.left,r.top,r.right,r.bottom];})()');
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
const checkSceneBounds=(metrics,stage)=>{
  const clip=metrics.scene_clip_rect,canvas=result.canvas_rect;
  assert(Array.isArray(clip)&&clip.length===4&&clip.every(Number.isFinite)&&Array.isArray(canvas),'Missing actual inspector/canvas clip observation');
  const effective=[Math.max(clip[0],canvas[0]),Math.max(clip[1],canvas[1]),Math.min(clip[2],canvas[2]),Math.min(clip[3],canvas[3])];
  assert(effective[0]<effective[2]&&effective[1]<effective[3],'Invalid actual inspector clip');
  const visible=Object.entries(metrics.scene_controls||{}).filter(([,r])=>r[3]>effective[1]&&r[1]<effective[3]);
  const failures=visible.filter(([,r])=>r[0]<effective[0]-.01||r[2]>effective[2]+.01).map(([key,rect])=>({key,rect}));
  result.ui_bounds_checks.push({stage,clip,canvas_rect:canvas,effective_clip:effective,visible_controls:visible.map(([key])=>key),failures});
  assert(failures.length===0,'Visible SubTool controls exceed actual inspector horizontal clip: '+JSON.stringify(failures));
  if(metrics.mask_region_clip_rect){const c=metrics.mask_region_clip_rect,e=[Math.max(c[0],canvas[0]),Math.max(c[1],canvas[1]),Math.min(c[2],canvas[2]),Math.min(c[3],canvas[3])],visible=Object.entries(metrics.mask_region_controls||{}).filter(([,r])=>r[3]>e[1]&&r[1]<e[3]),failures=visible.filter(([,r])=>r[0]<e[0]-.01||r[2]>e[2]+.01);assert(e[0]<e[2]&&e[1]<e[3],'Invalid actualregiontoolboxclip');result.ui_bounds_checks.push({stage:'region-left:'+stage,clip:c,effective_clip:e,visible_controls:visible.map(([key])=>key),failures});assert(!failures.length,'Region controls exceed LEFTclip: '+JSON.stringify(failures));}
  if(metrics.mask_filter_clip_rect){const c=metrics.mask_filter_clip_rect,e=[Math.max(c[0],canvas[0]),Math.max(c[1],canvas[1]),Math.min(c[2],canvas[2]),Math.min(c[3],canvas[3])];assert(e[0]<e[2]&&e[1]<e[3],'Missing actual left mask filter clip');const visible=Object.entries(metrics.mask_filter_controls||{}).filter(([,r])=>r[3]>e[1]&&r[1]<e[3]),failures=visible.filter(([,r])=>r[0]<e[0]-.01||r[2]>e[2]+.01);result.ui_bounds_checks.push({stage:'mask-left:'+stage,clip:c,effective_clip:e,visible_controls:visible.map(([key])=>key),failures});assert(!failures.length,'Mask controls exceed observed LEFT toolbox clip: '+JSON.stringify(failures));}
  if(metrics.extract_clip_rect){const c=metrics.extract_clip_rect,visible=Object.entries(metrics.extract_controls||{}).filter(([,r])=>r[3]>c[1]&&r[1]<c[3]);const failures=visible.filter(([,r])=>r[0]<Math.max(c[0],canvas[0])-.01||r[2]>Math.min(c[2],canvas[2])+.01);result.ui_bounds_checks.push({stage:'extract:'+stage,clip:c,visible_controls:visible.map(([k])=>k),failures});assert(!failures.length,'Extract fields exceed actual inspector clip: '+JSON.stringify(failures));}
  if(metrics.transform_clip_rect){const tc=metrics.transform_clip_rect,visible=Object.entries(metrics.transform_controls||{}).filter(([,r])=>r[3]>tc[1]&&r[1]<tc[3]);const failures=visible.filter(([,r])=>r[0]<tc[0]-.01||r[2]>tc[2]+.01);result.ui_bounds_checks.push({stage:'transform:'+stage,clip:tc,visible_controls:visible.map(([k])=>k),failures});assert(!failures.length,'Transform controls exceed actual inspector clip: '+JSON.stringify(failures));}
  if(metrics.gizmo_clip_rect){const gc=metrics.gizmo_clip_rect,enabled=Object.entries(metrics.gizmo_controls||{}).filter(([key])=>metrics.gizmo_controls_enabled?.[key]);const failures=enabled.filter(([,r])=>r[0]<Math.max(gc[0],canvas[0])-.01||r[1]<Math.max(gc[1],canvas[1])-.01||r[2]>Math.min(gc[2],canvas[2])+.01||r[3]>Math.min(gc[3],canvas[3])+.01);result.ui_bounds_checks.push({stage:'gizmo:'+stage,clip:gc,visible_controls:enabled.map(([key])=>key),failures});assert(!failures.length,'Enabled gizmos exceed actual viewport clip: '+JSON.stringify(failures));}
};
const control=async(key,allowDisabled=false)=>{
  for(let i=0;i<12;i++){
    const s=await state(),rect=s.mask_region_controls?.[key]||s.mask_filter_controls?.[key]||s.extract_controls?.[key]||s.transform_controls?.[key]||s.gizmo_controls?.[key]||s.scene_controls?.[key]||s.morph_controls?.[key]||s.layer_controls?.[key];
    checkSceneBounds(s,'control:'+key);
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&(s.mask_region_controls_enabled?.[key]??s.mask_filter_controls_enabled?.[key]??s.extract_controls_enabled?.[key]??s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key])===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.mask_region_controls_enabled?.[key]??s.mask_filter_controls_enabled?.[key]??s.extract_controls_enabled?.[key]??s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key]};
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
      await pause(50);const after=await state(),next=after.mask_region_controls?.[key]||after.mask_filter_controls?.[key]||after.extract_controls?.[key]||after.transform_controls?.[key]||after.gizmo_controls?.[key]||after.scene_controls?.[key]||after.morph_controls?.[key]||after.layer_controls?.[key];
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
  assert(settings.kind===(number===0?'Morph':number===7?'Move':number===8?'Mask':'Draw'),'Actual keyboard brush selection failed for '+name);
  assert(Math.abs(settings.radius-.18)<1e-6&&Math.abs(settings.strength-.35)<1e-6&&settings.symmetry_x,'Unexpected actual brush parameters: '+name);
  const v=await viewport();
  const from=[v.cx+offset[0],v.cy+offset[1]],to=[v.cx+endOffset[0],v.cy+endOffset[1]],start=await evaluate('performance.now()');
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,pointerType,force,true);await pause(70);
  for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,pointerType,force,true);await pause(35);}
  await pointer('mouseReleased',...to,pointerType,0,false);await pause(100);const probes=[];
  for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()')});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);
  const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);
  const trial={name,brush:number,settings,pointer_type:pointerType,force,from,to,steps,spacing_ms:35,viewport:v,start,end,probes,quiet,runtime,after:await state()};result.strokes.push(trial);assert(quiet,'Geometry or pending input changed after release: '+name);return trial;
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
const snapshot=async(name,{save_first=false}={})=>{
  await pause(150);const saved=await save(name);await pause(150);const gpu=await gpuDigest(),metrics=await state();
  checkSceneBounds(metrics,'snapshot:'+name);
  const record={...metadata(saved),metrics,gpu,gpu_matches_saved_scene:matchSceneGpu(saved.document,gpu),scene:sceneContract(saved.document,metrics),composition:composition(saved.document),morph:targetContract(saved.document,metrics)};
  result.snapshots.push(record);assert(record.gpu_matches_saved_scene,'Actual visible scene GPU vertex/index buffers differ from Save: '+name);return saved;
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
const finishCase=async(record)=>{result.cases.push(record);await writeFile(path.join(out,record.name+'.json'),JSON.stringify(record,null,2)+'\n');await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:record.name,gates:record.gates}));assert(Object.values(record.gates).every(Boolean),'Extraction case failed: '+record.name);};
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
  assert((document.version===4||document.version===5)&&t.scheme==='positions_f32_v1','Unexpected public target version/scheme');
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
const originalDisplacement=(before,after)=>{assert(after.positions.length>=before.positions.length,'Original vertex identities disappeared');let changed=0,max=0;for(let i=0;i<before.positions.length;i++){const d=Math.hypot(...before.positions[i].map((v,k)=>after.positions[i][k]-v));changed+=Number(d>0);max=Math.max(max,d);}return {original_vertices:before.positions.length,after_vertices:after.positions.length,changed_vertices:changed,max_displacement:max};};
const indexHash=mesh=>{const bytes=Buffer.alloc(mesh.triangles.length*12);mesh.triangles.forEach((face,i)=>face.forEach((v,k)=>bytes.writeUInt32LE(v,i*12+k*4)));return hash(bytes);};
const activeState=document=>({mesh:document.mesh,multiresolution:document.multiresolution??null,layers:document.layers??null,morph_target:document.morph_target??null});
const objects=document=>document.scene?document.scene.objects.map(o=>({...o,state:o.id===document.scene.active_id?activeState(document):o.payload})): [{id:1,name:document.name,visible:true,payload:null,state:activeState(document)}];
const stateHash=state=>{
  const bits=[];const doubles=positions=>{const b=Buffer.alloc(positions.length*24);positions.forEach((p,i)=>p.forEach((v,k)=>b.writeDoubleLE(v,i*24+k*8)));return hash(b);};
  bits.push(packedHash(state.mesh),indexHash(state.mesh));
  if(state.multiresolution)for(const level of state.multiresolution.levels)bits.push(level?packedHash(level):null);
  if(state.layers){bits.push(doubles(state.layers.base_positions));for(const e of state.layers.entries)bits.push(doubles(e.deltas));}
  if(state.morph_target)bits.push(positionBits(state.morph_target.positions));
  return hash(JSON.stringify({state:activeState(state),bits}));
};
const byId=(document,id)=>objects(document).find(o=>o.id===id);
const sceneContract=(document,metrics)=>{
  const all=objects(document),scene=document.scene,active=scene?.active_id??1,solo=scene?.solo??false;
  if(scene){assert(document.version===5&&scene.scheme==='subtools_v1','Unexpected public scene schema');assert(all.length>0&&all.length<=32&&new Set(all.map(o=>o.id)).size===all.length,'Invalid scene identities');assert(all.every(o=>Number.isInteger(o.id)&&o.id>0&&o.id<=0xffffffff)&&scene.next_id>Math.max(...all.map(o=>o.id)),'Invalid stable IDs/allocator');assert(all.filter(o=>o.id===active).length===1&&all.every(o=>o.id===active?o.payload===null:!!o.payload),'Active payload slot is not external/null or inactive payload missing');}else assert(document.version<5,'v5 has no scene');
  assert(metrics.active_subtool_id===active&&metrics.solo===solo&&metrics.subtools.length===all.length,'Readonly scene metadata differs from Save');
  const contracts=[];
  for(const o of all){const actual=metrics.subtools.find(s=>s.id===o.id),effective=o.visible&&(!solo||o.id===active);assert(actual&&actual.name===o.name&&actual.visible===o.visible&&actual.effective_visible===effective&&actual.active===(o.id===active)&&actual.vertices===o.state.mesh.positions.length&&actual.triangles===o.state.mesh.triangles.length,'SubTool summary differs from independently interpreted public scene');
    const level=o.state.multiresolution?.active_level??0,t=o.state.morph_target;const synthetic={subdivision_level:level,morph_target:t?{creation_level:t.creation_level,vertices:t.positions.length,triangles:t.triangles.length,available:t.creation_level===level}:null};
    contracts.push({id:o.id,effective_visible:effective,state_sha256:stateHash(o.state),composition:composition(o.state),morph:targetContract({version:t?4:o.state.layers?3:2,...o.state},o.id===active?metrics:synthetic)});
  }
  assert(metrics.active_subtool_visible===all.find(o=>o.id===active).visible,'Active visibility diagnostic differs');assert(!metrics.scene_error,'Renderer reported scene error: '+metrics.scene_error);
  return {active_id:active,solo,objects:contracts};
};
const matchSceneGpu=(document,gpu)=>{
  const active=document.scene?.active_id??1,solo=document.scene?.solo??false;
  const visible=objects(document).filter(o=>o.visible&&(!solo||o.id===active));
  const expected=visible.map(o=>JSON.stringify([o.id===active,packedHash(o.state.mesh),indexHash(o.state.mesh),o.state.mesh.triangles.length*3,o.state.mesh.positions.length*40,o.state.mesh.triangles.length*12])).sort();
  const actual=gpu.draws.map(d=>JSON.stringify([d.active,d.vertex_sha256,d.index_sha256,d.count,d.vertex_bytes,d.index_bytes])).sort();
  assert(gpu.draws.every(d=>d.type===5125&&d.offset===0),'Unexpected actual solid index type/offset');
  assert(new Set(gpu.draws.map(d=>d.vao_key)).size===gpu.draws.length,'Visible solids share an unexpected VAO draw');
  if(visible.some(o=>o.id===active))assert(gpu.draws.filter(d=>d.active).length===1&&gpu.draws.at(-1).active,'Actual u_active solid is not uniquely last');else assert(gpu.draws.every(d=>!d.active),'Hidden active object still rendered as active');
  return JSON.stringify(expected)===JSON.stringify(actual);
};
const selected=async id=>{await action('scene.select.'+id);return waitFor(s=>s.active_subtool_id===id,'Actual part selection failed');};
const sceneCount=async count=>waitFor(s=>s.subtools.length===count,'Unexpected scene object count');
const renameScene=async name=>{await action('scene.name');for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4});await send('Input.insertText',{text:name});await pause(100);await action('scene.rename');return waitFor(s=>s.subtools.find(o=>o.active)?.name===name,'Scene Rename failed');};
const append=async fixture=>{const before=await state();await action('scene.append_obj');const dom=(await send('DOM.getDocument')).root,nodeId=(await send('DOM.querySelector',{nodeId:dom.nodeId,selector:'#import-file'})).nodeId;await evaluate('document.getElementById("notice").textContent=""');await send('DOM.setFileInputFiles',{nodeId,files:[fixture.path]});return waitFor(s=>s.subtools.length===before.subtools.length+1&&s.active_subtool_id!==before.active_subtool_id,'Actual Append OBJ did not add a part');};
const meshFixture=(center=[0,0,0],radius=1,color=[.2,.6,.8],masked=false)=>{
  const longitude=24,latitude=12,positions=[[0,1,0]],triangles=[];
  for(let ring=1;ring<latitude;ring++)for(let column=0;column<longitude;column++){const phi=Math.PI*ring/latitude,theta=2*Math.PI*column/longitude;positions.push([Math.sin(phi)*Math.sin(theta),Math.cos(phi),Math.sin(phi)*Math.cos(theta)]);}
  const bottom=positions.length;positions.push([0,-1,0]);const vertex=(ring,column)=>1+(ring-1)*longitude+(column%longitude);
  for(let c=0;c<longitude;c++){triangles.push([0,vertex(1,c),vertex(1,c+1)],[bottom,vertex(latitude-1,c+1),vertex(latitude-1,c)]);}
  for(let r=1;r<latitude-1;r++)for(let c=0;c<longitude;c++){const a=vertex(r,c),b=vertex(r,c+1),d=vertex(r+1,c),e=vertex(r+1,c+1);triangles.push([a,d,b],[b,d,e]);}
  return {positions:positions.map(p=>p.map((v,k)=>Math.fround(center[k]+radius*v))),normals:positions.map(p=>p.map(Math.fround)),masks:positions.map((_,i)=>masked===true?1:masked==='pattern'?Math.fround((i%7)/6):0),colors:positions.map(()=>color.map(Math.fround)),triangles};
};
const objFixture=async(name,mesh)=>{let text='# Independently authored UV sphere; coordinates and RGB channels\n';mesh.positions.forEach((p,i)=>text+='v '+[...p,...mesh.colors[i]].join(' ')+'\n');mesh.triangles.forEach(t=>text+='f '+t.map(i=>i+1).join(' ')+'\n');const file=path.join(out,name+'.obj');await writeFile(file,text);return {path:file,sha256:hash(text),bytes:Buffer.byteLength(text),mesh};};
const sceneFixture=async(name,states,active=states[0].id)=>publicFixture(name,{version:5,name, ...states.find(o=>o.id===active).state,scene:{scheme:'subtools_v1',active_id:active,next_id:Math.max(...states.map(o=>o.id))+1,solo:false,objects:states.map(o=>({id:o.id,name:o.name,visible:true,payload:o.id===active?null:o.state}))}});
const projectedContact=async mesh=>{
  const gpu=await gpuDigest(),view=gpu.view,matrix=gpu.view_projection,v=gpu.viewport;assert(view?.length===16&&matrix?.length===16&&v,'Missing actual uploaded camera matrices/viewport');
  const direction=[view[2],view[6],view[10]],point=mesh.positions.reduce((best,p)=>p.reduce((s,x,k)=>s+x*direction[k],0)>best.reduce((s,x,k)=>s+x*direction[k],0)?p:best);
  const clip=[0,1,2,3].map(r=>matrix[r]*point[0]+matrix[r+4]*point[1]+matrix[r+8]*point[2]+matrix[r+12]);assert(clip[3]>0,'Authored contact behind camera');
  const screen=[v[0]+v[2]*(clip[0]/clip[3]+1)/2,1000-v[1]-v[3]*(clip[1]/clip[3]+1)/2];assert(screen[0]>v[0]+20&&screen[0]<v[0]+v[2]-20&&screen[1]>70&&screen[1]<940,'Projected contact outside actual canvas');
  return {point,screen,view_projection:matrix,view,viewport:v};
};
const sculptScene=async(name,number,mesh,steps=10)=>{
  await key(number);await evaluate('document.getElementById("obrush_canvas").focus()');const settings=await state();assert(settings.kind===({1:'Draw',8:'Mask',9:'Paint'})[number],'Actual brush shortcut failed');assert(Math.abs(settings.radius-.18)<1e-6&&Math.abs(settings.strength-.35)<1e-6&&settings.symmetry_x,'Unexpected default sculpt parameters');
  const contact=await projectedContact(mesh),from=contact.screen,to=[from[0]+10,from[1]+8],start=await evaluate('performance.now()');
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,'mouse',.5,true);await pause(70);for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,'mouse',.5,true);await pause(35);}await pointer('mouseReleased',...to);await pause(100);
  const probes=[];for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()'),metrics:await state()});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);const trial={name,brush:number,settings,contact,from,to,steps,start,end,quiet,probes,runtime};result.strokes.push(trial);assert(quiet,'Geometry or pending input changed after release: '+name);return trial;
};
const generalKey=async(key)=>{for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key,code:key==='Escape'?'Escape':'Key'+key.toUpperCase(),windowsVirtualKeyCode:key==='Escape'?27:key.toUpperCase().charCodeAt(0),modifiers:0});await pause(120);};
const numeric=async(key,value)=>{
 const c=await control(key);await pointer('mouseMoved',...c.point);await pause(40);
 for(const count of [1,2]){for(const type of ['mousePressed','mouseReleased']){const event={type,x:c.point[0],y:c.point[1],button:'left',buttons:type==='mousePressed'?1:0,clickCount:count};result.input_trace.push({...event,at_epoch_ms:Date.now()});await send('Input.dispatchMouseEvent',event);await pause(25);}await pause(25);}
 await pause(100);await chord('a');await send('Input.insertText',{text:String(value)});
 for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await pause(160);
 result.ui_actions.push({key,numeric_value:value,point:c.point});return state();
};
const configureTransform=async({translation=[0,0,0],rotation_degrees=[0,0,0],scale=1,pivot_kind='origin'})=>{
 await action('transform.reset');await action('transform.pivot_'+pivot_kind);
 for(const [key,value] of [...translation.map((v,i)=>['transform.t'+['x','y','z'][i],v]),...rotation_degrees.map((v,i)=>['transform.r'+['x','y','z'][i],v]),['transform.scale',scale]])await numeric(key,value);
 const s=(await state()).transform_settings;assert(s&&s.pivot_kind===pivot_kind,'Transform numeric/pivot diagnostics missing');
 for(const k of ['translation','rotation_degrees'])assert(s[k].every((v,i)=>Math.fround(v)===Math.fround((k==='translation'?translation:rotation_degrees)[i])),'Actual numeric input did not set '+k);
 assert(Math.fround(s.scale)===Math.fround(scale),'Actual uniform scale input mismatch');return s;
};
const sheetMesh=(maxMask=1,shift=0)=>({positions:[[shift,0,0],[shift+1,0,0],[shift+1,1,0],[shift,1,0]],normals:Array.from({length:4},()=>[0,0,1]),masks:[0,maxMask,maxMask,0].map(Math.fround),colors:[[0,0,.2],[1,0,.2],[1,1,.2],[0,1,.2]].map(c=>c.map(Math.fround)),triangles:[[0,1,2],[0,2,3]]});
const plain=mesh=>({mesh,multiresolution:null,layers:null,morph_target:null});
const plainFixture=async(name,mesh)=>publicFixture(name,{version:2,name,mesh});
const configureExtract=async(threshold=.5,thickness=.125)=>{await numeric('extract.threshold',threshold);await numeric('extract.thickness',thickness);const s=await state();assert(Math.fround(s.extract_settings.threshold)===Math.fround(threshold)&&Math.fround(s.extract_settings.thickness)===Math.fround(thickness),'Actual extract numeric fields differ');return s.extract_settings;};
const createPart=async()=>{const before=await state();await action('extract.create');return waitFor(s=>s.subtools.length===before.subtools.length+1&&s.active_subtool_id!==before.active_subtool_id,'Extraction did not create one new active part');};
const sub=(a,b)=>a.map((v,k)=>v-b[k]);const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],dot=(a,b)=>a.reduce((s,v,k)=>s+v*b[k],0);
const closedTopology=mesh=>{
 const points=mesh.positions.map(p=>p.map(Math.fround)),edges=new Map(),incident=points.map(()=>[]),neighbors=mesh.triangles.map(()=>new Set());let volume=0,minDoubleArea=Infinity;
 for(let i=0;i<mesh.triangles.length;i++){const face=mesh.triangles[i];assert(new Set(face).size===3&&face.every(v=>Number.isInteger(v)&&v>=0&&v<points.length),'Invalid triangle IDs');const [a,b,c]=face.map(v=>points[v]);const area=Math.hypot(...cross(sub(b,a),sub(c,a)));assert(Number.isFinite(area)&&area>0,'Collapsed output face');minDoubleArea=Math.min(minDoubleArea,area);volume+=dot(a,cross(b,c))/6;face.forEach(v=>incident[v].push(i));for(let j=0;j<3;j++){const a=face[j],b=face[(j+1)%3],key=Math.min(a,b)+':'+Math.max(a,b);if(!edges.has(key))edges.set(key,[]);edges.get(key).push({a,b,face:i});}}
 for(const entry of edges.values()){assert(entry.length===2&&entry[0].a===entry[1].b&&entry[0].b===entry[1].a,'Open/nonmanifold/inconsistently wound output edge');neighbors[entry[0].face].add(entry[1].face);neighbors[entry[1].face].add(entry[0].face);}
 for(let v=0;v<points.length;v++){assert(incident[v].length>0,'Unused output vertex');const allowed=new Set(incident[v]),seen=new Set(),queue=[incident[v][0]];while(queue.length){const f=queue.pop();if(seen.has(f))continue;seen.add(f);for(const n of neighbors[f])if(allowed.has(n))queue.push(n);}assert(seen.size===allowed.size,'Disconnected output vertex fan');}
 const seen=new Set();let components=0;for(let f=0;f<neighbors.length;f++)if(!seen.has(f)){components++;const q=[f];while(q.length){const i=q.pop();if(seen.has(i))continue;seen.add(i);q.push(...neighbors[i]);}}
 return {vertices:points.length,faces:mesh.triangles.length,edges:edges.size,components,euler:points.length-edges.size+mesh.triangles.length,signed_volume:volume,min_double_area:minDoubleArea};
};
const extractedContract=(before,after,metrics)=>{const oldIds=objects(before).map(o=>o.id),newId=after.scene?.active_id;assert(after.version===5&&newId&&!oldIds.includes(newId),'New scene/ID not allocated');assert(objects(after).length===oldIds.length+1,'Object count did not grow exactly one');for(const id of oldIds){assert(stateHash(byId(before,id).state)===stateHash(byId(after,id).state),'Source/inactive metadata changed: '+id);assert(byId(before,id).name===byId(after,id).name&&byId(before,id).visible===byId(after,id).visible,'Source name/visibility changed');}const part=byId(after,newId).state;assert(!part.multiresolution&&!part.layers&&!part.morph_target&&part.mesh.masks.every(v=>Math.fround(v)===0),'New part not independent/plain/unmasked');const topology=closedTopology(part.mesh),report=metrics.extract_report;assert(report&&report.source_subtool_id===(before.scene?.active_id??1)&&report.new_subtool_id===newId&&report.output_vertices===part.mesh.positions.length&&report.output_triangles===part.mesh.triangles.length&&report.document_revision===metrics.document_revision&&report.input_vertices===activeState(before).mesh.positions.length&&report.input_triangles===activeState(before).mesh.triangles.length&&Math.fround(report.threshold)===Math.fround(metrics.extract_settings.threshold)&&Math.fround(report.thickness)===Math.fround(metrics.extract_settings.thickness),'Report binding/counts/settings differ');assert(report.output_vertices===2*report.patch_vertices&&report.output_triangles===2*report.patch_triangles+2*report.boundary_edges,'Shell output count law differs from independent two-skin/boundary-quad contract');return {new_id:newId,source_ids:oldIds,source_exact:true,new_plain_unmasked:true,topology,report};};
const planarOracle=(mesh,{left=.5,right=1,area=.5,thickness=.125,shift=0}={})=>{let position_max=0,color_max=0;for(let i=0;i<mesh.positions.length;i++){const p=mesh.positions[i].map(Math.fround),c=mesh.colors[i].map(Math.fround);assert(p[0]>=left+shift-2e-6&&p[0]<=right+shift+2e-6&&p[1]>=-2e-6&&p[1]<=1+2e-6,'Clipped point outside analytic footprint');position_max=Math.max(position_max,Math.min(Math.abs(p[2]),Math.abs(p[2]-thickness)));color_max=Math.max(color_max,Math.abs(c[0]-(p[0]-shift)),Math.abs(c[1]-p[1]),Math.abs(c[2]-Math.fround(.2)));}const t=closedTopology(mesh),volume_error=Math.abs(t.signed_volume-area*thickness);assert(position_max<=2e-6&&color_max<=1e-6&&volume_error<=2e-6&&t.signed_volume>0,'Analytic planar oracle failed');return {position_max,color_max,volume_error,expected_volume:area*thickness,...t};};
const ringMesh=()=>{const positions=[],triangles=[];for(let j=0;j<=4;j++)for(let i=0;i<=4;i++)positions.push([i/4,j/4,0]);for(let j=0;j<4;j++)for(let i=0;i<4;i++){const a=j*5+i,b=a+1,c=a+6,d=a+5;triangles.push([a,b,c],[a,c,d]);}return {positions,triangles,normals:positions.map(()=>[0,0,1]),masks:positions.map((_,i)=>i===12?0:1),colors:positions.map(p=>[p[0],p[1],Math.fround(.2)])};};
const disconnectedMesh=()=>{const a=sheetMesh(),b=sheetMesh(1,2);return {positions:[...a.positions,...b.positions],normals:[...a.normals,...b.normals],colors:[...a.colors,...b.colors],masks:[...a.masks,...b.masks],triangles:[...a.triangles,...b.triangles.map(t=>t.map(v=>v+4))]};};
const bowtieMesh=()=>({positions:[[0,0,0],[1,0,0],[0,1,0],[-1,0,0],[0,-1,0]],normals:Array.from({length:5},()=>[0,0,1]),masks:[1,1,1,1,1],colors:Array.from({length:5},()=>[.2,.5,.8]),triangles:[[0,1,2],[0,3,4]]});
const geometryPayloadHash=state=>{const a=structuredClone(state);a.mesh.masks.fill(0);for(const level of a.multiresolution?.levels||[])if(level)level.masks.fill(0);return stateHash(a);};
const scalarMaskOracle=(mesh,operation,iterations,strength)=>{
 const adjacent=mesh.positions.map(()=>new Set());for(const face of mesh.triangles)for(const a of face)for(const b of face)if(a!==b)adjacent[a].add(b);
 const neighborhoods=adjacent.map((s,i)=>[i,...Array.from(s).sort((a,b)=>a-b)]),amount=Math.fround(strength);let current=mesh.masks.map(Math.fround);
 for(let pass=0;pass<iterations;pass++)current=current.map((old,i)=>{
  if(operation==='sharpen'){if(old===0||old===.5||old===1)return old;const next=Math.fround(Math.max(0,Math.min(1,.5+(old-.5)*(1+amount))));return next===old?old:next;}
  if(neighborhoods[i].length===1)return old;
  const values=neighborhoods[i].map(j=>current[j]);let target;
  if(operation==='grow')target=Math.max(...values);else if(operation==='shrink')target=Math.min(...values);else target=values.reduce((s,v)=>s+v,0)/values.length;
  const next=Math.fround(old+amount*(target-old));return next===old?old:next;
 });return current;
};
const configureFilter=async(iterations=1,strength=1)=>{await numeric('mask.passes',iterations);await numeric('mask.amount',strength);const s=await state();assert(s.mask_filter_settings.iterations===iterations&&Math.fround(s.mask_filter_settings.strength)===Math.fround(strength),'Actual filter numeric fields differ');return s.mask_filter_settings;};
const filterContract=(before,after,operation,settings,beforeMetrics,afterMetrics)=>{
 assert(before.name===after.name&&before.version===after.version,'Filter changed document name/schema');const a=activeState(before),b=activeState(after),expected=scalarMaskOracle(a.mesh,operation,settings.iterations,settings.strength);assert(a.mesh.masks.length===b.mesh.masks.length,'Filter vertex count changed');
 let maxError=0,changed=0,maxChange=0;for(let i=0;i<expected.length;i++){const old=Math.fround(a.mesh.masks[i]),value=Math.fround(b.mesh.masks[i]);maxError=Math.max(maxError,Math.abs(value-expected[i]));if(!Object.is(old,value)){changed++;maxChange=Math.max(maxChange,Math.abs(old-value));}}
 assert(maxError<=1e-7,'Independent scalar mask oracle failed: '+maxError);assert(geometryPayloadHash(a)===geometryPayloadHash(b),'Current/retained geometry, normals, RGB, layerf64 or target changed');
 if(before.scene){assert(JSON.stringify(before.scene.objects.map(o=>[o.id,o.name,o.visible]))===JSON.stringify(after.scene.objects.map(o=>[o.id,o.name,o.visible]))&&before.scene.active_id===after.scene.active_id&&before.scene.next_id===after.scene.next_id&&before.scene.solo===after.scene.solo,'Scene metadata changed');for(const o of objects(before).filter(o=>o.id!==before.scene.active_id))assert(stateHash(o.state)===stateHash(byId(after,o.id).state),'Inactive payload changed');}
 const report=afterMetrics.mask_filter_report;assert(report&&report.subtool_id===beforeMetrics.active_subtool_id&&report.document_revision===afterMetrics.document_revision&&report.operation===operation[0].toUpperCase()+operation.slice(1)&&report.iterations===settings.iterations&&Math.fround(report.strength)===Math.fround(settings.strength)&&report.vertices===expected.length&&report.changed_vertices===changed&&Math.abs(report.max_abs_change-maxChange)<=1e-12,'Filter report binding/settings/counts differ');
 assert(afterMetrics.document_revision===beforeMetrics.document_revision+(changed?1:0),'Filter did not commit exactly one revision or preserve no-op');
 return {max_mask_error:maxError,changed_vertices:changed,max_abs_change:maxChange,current_and_retained_geometry_exact:true,layer_target_bits_exact:true,inactive_payload_exact:true,report,...(expected.length<=16?{expected_masks:expected,actual_masks:b.mesh.masks.map(Math.fround)}:{})};
};
const filterTrial=async(name,operation,iterations=1,strength=1)=>{const before=await snapshot(name+'-before'),settings=await configureFilter(iterations,strength),pre=await state();await action('mask.'+operation);const post=await state(),after=await snapshot(name+'-after'),measure=filterContract(before.document,after.document,operation,settings,pre,post);return {name,operation,settings,before,after,pre,post,measure};};
const compactFilter=({before,after,...rest})=>({...rest,before:metadata(before),after:metadata(after)});
const quadMask=()=>({...sheetMesh(),masks:[0,.25,.75,1]});
const irregularMask=()=>({positions:[[0,0,0],[1,0,0],[1,1,0],[0,1,0],[0,0,0],[2,0,0],[2,1,0],[3,3,0]],normals:Array.from({length:8},()=>[0,0,1]),colors:Array.from({length:8},(_,i)=>[Math.fround(i/8),Math.fround(1-i/8),.2]),masks:[0,.17,.61,1,.8,.23,.4,.37].map(Math.fround),triangles:[[0,1,2],[0,2,3],[0,1,2],[4,5,6]]});
const zeroLayer=mesh=>({scheme:'world_offsets_f64_v1',base_positions:mesh.positions.map(p=>p.slice()),triangles:mesh.triangles.map(t=>t.slice()),recording_layer:0,entries:[{name:'Independent zero offsets',intensity:1,visible:true,deltas:mesh.positions.map(()=>[0,0,0])}]});
const targetFor=mesh=>({scheme:'positions_f32_v1',creation_level:0,positions:mesh.positions.map(p=>p.slice()),triangles:mesh.triangles.map(t=>t.slice())});
const currentBufferFor=(snapshot,id)=>{const mesh=byId(snapshot.document,id).state.mesh,gpu=result.snapshots.find(s=>s.path===snapshot.path).gpu;return gpu.draws.find(d=>d.vertex_sha256===packedHash(mesh)&&d.index_sha256===indexHash(mesh));};
const cacheEqual=(before,after)=>{const fields=['vao_key','vertex_key','index_key','vertex_sha256','index_sha256','vertex_bytes','index_bytes','vertex_uploads','index_uploads','count','type','offset'];return !!before&&!!after&&fields.every(k=>before[k]===after[k]);};

const previewBuffersEqual=(a,b)=>{const keys=['vao_key','vertex_key','index_key','vertex_sha256','index_sha256','vertex_uploads','index_uploads','vertex_bytes','index_bytes','count','type','offset','active'];return a.draws.length===b.draws.length&&a.draws.every((d,i)=>keys.every(k=>d[k]===b.draws[i][k]));};
const maskBits=masks=>{const b=Buffer.alloc(masks.length*4);masks.forEach((v,i)=>b.writeFloatLE(Math.fround(v),i*4));return hash(b);};
const clampedObservedPoint=(event,vp)=>[event.x,event.y].map((v,k)=>Math.max(vp[k],Math.min(vp[k]+vp[k+2],Math.fround(Math.trunc(v)))));
const samePoint=(a,b)=>a.every((v,i)=>v===b[i]);
const projectedPoint=(point,matrix,viewport)=>{const p=point.map(Math.fround),m=matrix.map(Math.fround),q=[0,1,2,3].map(r=>m[r]*p[0]+m[r+4]*p[1]+m[r+8]*p[2]+m[r+12]);if(!q.every(Number.isFinite)||q[3]<=0||q.slice(0,3).some(v=>Math.abs(v)>q[3]))return null;return [viewport[0]+(q[0]/q[3]+1)*viewport[2]/2,viewport[1]+(1-q[1]/q[3])*viewport[3]/2];};
const insideRegion=(p,region)=>{
 if(region.kind==='rectangle'){const lo=region.start.map((v,k)=>Math.min(v,region.end[k])),hi=region.start.map((v,k)=>Math.max(v,region.end[k]));return lo[0]<hi[0]&&lo[1]<hi[1]&&p.every((v,k)=>v>=lo[k]&&v<=hi[k]);}
 const vertices=region.points;let crossings=0;for(let i=0;i<vertices.length;i++){const a=vertices[i],b=vertices[(i+1)%vertices.length],dx=b[0]-a[0],dy=b[1]-a[1],den=dx*dx+dy*dy,t=den?Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/den)):0;if(Math.hypot(p[0]-(a[0]+t*dx),p[1]-(a[1]+t*dy))<=1e-7)return true;if((a[1]>p[1])!==(b[1]>p[1])){const x=a[0]+(p[1]-a[1])*(b[0]-a[0])/(b[1]-a[1]);if(x>p[0])crossings++;}}return crossings%2===1;
};
const regionOracle=(mesh,report)=>{let projected=0,selected=0,changed=0,maxChange=0;const masks=mesh.masks.map(Math.fround);if(!report.skipped_tiny_click)mesh.positions.forEach((p,i)=>{const q=projectedPoint(p,report.view_projection,report.viewport);if(!q)return;projected++;if(!insideRegion(q,report.region))return;selected++;const target=report.erase?0:1;if(masks[i]!==target){changed++;maxChange=Math.max(maxChange,Math.abs(masks[i]-target));masks[i]=target;}});return {masks,projected,selected,changed,maxChange};};
const gridMesh=()=>{const positions=[],triangles=[];for(let y=0;y<7;y++)for(let x=0;x<7;x++)positions.push([Math.fround((x-3)*.3),Math.fround((y-3)*.3),0]);for(let y=0;y<6;y++)for(let x=0;x<6;x++){const a=y*7+x;triangles.push([a,a+1,a+8],[a,a+8,a+7]);}return {positions,triangles,normals:positions.map(()=>[0,0,1]),colors:positions.map((_,i)=>[Math.fround(i/49),Math.fround(1-i/49),.2]),masks:positions.map((_,i)=>Math.fround((i%5)/4))};};
const modeRegion=async mode=>{await action('mask.region.'+mode.toLowerCase());const s=await state();assert(s.mask_region_settings.mode===mode,'Actualregionmodebutton failed');assert(s.mask_region_settings.through_depth&&s.mask_region_settings.point_limit===512&&s.mask_region_settings.point_edge_work_limit===32000000,'Regionlimits/context differ');return s;};
const regionFrame=async mesh=>{const gpu=await gpuDigest(),s=gpu.metrics,v=s.camera.viewport,viewport=[v[0],v[1],v[2]-v[0],v[3]-v[1]],matrix=gpu.view_projection.map(Math.fround),points=mesh.positions.map(p=>projectedPoint(p,matrix,viewport)).filter(Boolean);assert(points.length,'No independentlyprojected fixture points');const lo=[0,1].map(k=>Math.min(...points.map(p=>p[k]))),hi=[0,1].map(k=>Math.max(...points.map(p=>p[k])));return {gpu,metrics:s,matrix,viewport,lo,hi,at:(x,y)=>[Math.round(lo[0]+(hi[0]-lo[0])*x),Math.round(lo[1]+(hi[1]-lo[1])*y)]};};
const rectTrace=f=>[f.at(.2,.2),f.at(.8,.8)];
const wholeTrace=f=>[[Math.ceil(f.viewport[0])+2,Math.ceil(f.viewport[1])+2],[Math.floor(f.viewport[0]+f.viewport[2])-2,Math.floor(f.viewport[1]+f.viewport[3])-2]];
const gestureBegin=async(name,before,mode,trace,options={})=>{
 await modeRegion(mode);if(options.ctrl)await key(1);await evaluate('document.getElementById("obrush_canvas").focus()');const preGpu=await gpuDigest(),pre=preGpu.metrics,start=await evaluate('performance.now()'),mods=(options.erase?1:0)|(options.ctrl?2:0);await pointer('mouseMoved',...trace[0]);await pause(40);await pointer('mousePressed',...trace[0],'mouse',.5,true,mods);await waitFor(s=>s.mask_region_state.active,'Regiondidnotcapture actualpress');
 const first=(await state()).mask_region_gesture;const pressEvent=await evaluate(`window.__brushCheck.events.filter(e=>e.type==='pointerdown'&&e.button===0&&e.at>=${start}).at(-1)`);assert(pressEvent?.trusted&&pressEvent.x===trace[0][0]&&pressEvent.y===trace[0][1],'Missingactualintegerpress binding');assert(samePoint(first.start,clampedObservedPoint(pressEvent,first.viewport))&&first.erase===pressEvent.alt&&first.subtool_id===pre.active_subtool_id&&first.captured_document_revision===pre.document_revision,'Capturedpress/modifiers/identity differ fromactualpress');const pv=pre.camera.viewport,expectedViewport=[pv[0],pv[1],pv[2]-pv[0],pv[3]-pv[1]];assert(JSON.stringify(first.view_projection.map(Math.fround))===JSON.stringify(preGpu.view_projection.map(Math.fround))&&JSON.stringify(first.viewport)===JSON.stringify(expectedViewport),'Capturedprojection/viewport doesnotmatch actualrenderedcamera');for(const point of trace.slice(1)){await pointer('mouseMoved',...point,'mouse',.5,true,options.move_modifiers??0);await pause(35);}await pause(100);const gpu=await gpuDigest(),preview=gpu.metrics,captured=preview.mask_region_gesture;assert(captured&&preview.mask_region_state.active,'Regionpreviewunexpectedlyended');assert(['dirty','document_revision','snapshot_revision','active_subtool_id'].every(k=>pre[k]===preview[k]),'Regionpreviewchanged dirty/revision/activepart');assert(previewBuffersEqual(preGpu,gpu),'Regionpreviewmodified/uploadedgeometry');assert(JSON.stringify(first.view_projection.map(Math.fround))===JSON.stringify(captured.view_projection.map(Math.fround))&&JSON.stringify(first.viewport)===JSON.stringify(captured.viewport),'Capturedcamera/viewportchanged');const observedMoves=await evaluate(`window.__brushCheck.events.filter(e=>e.type==='pointermove'&&e.buttons===1&&e.at>${pressEvent.at}&&e.at<=${gpu.observed_at})`);const expectedMoves=observedMoves.map(e=>clampedObservedPoint(e,captured.viewport)),expectedPoints=[first.start,...expectedMoves];assert(samePoint(captured.end,expectedMoves.at(-1)||first.start),'Capturedend differs fromlastactualobservedmove');if(mode==='Lasso')assert(JSON.stringify(captured.points)===JSON.stringify(expectedPoints),'Observedlassomoves/repeats droppedorreordered');return {name,before,mode,trace,start,pre,preGpu,first,pressEvent,observedMoves,preview,captured,gpu,options};
};
const regionContract=(before,after,pre,post,captured,releaseEvent)=>{
 const report=post.mask_region_report;assert(report,'Missingregionreport');assert(releaseEvent?.trusted&&releaseEvent.button===0,'Missingactualreleasedevent');const expectedEnd=clampedObservedPoint(releaseEvent,captured.viewport);if(report.region.kind==='rectangle')assert(samePoint(report.region.end,expectedEnd),'Rectangleend differs fromindependentlyclamped actualrelease');else{const expectedPoints=captured.points.map(p=>p.slice());if(!samePoint(captured.end,expectedEnd))expectedPoints.push(expectedEnd);assert(JSON.stringify(report.region.points)===JSON.stringify(expectedPoints),'Fullreleasedlassopoints differ fromobservedmoves+changedrelease');}const oracle=regionOracle(activeState(before).mesh,report),actual=activeState(after).mesh;assert(maskBits(actual.masks)===maskBits(oracle.masks),'Independentregionmaskbits differ');assert(geometryPayloadHash(activeState(before))===geometryPayloadHash(activeState(after)),'Regionchangedgeometry/normals/RGB/levels/layerf64/target');assert(before.name===after.name&&before.version===after.version,'Regionchangeddocumentname/schema');if(before.scene){assert(before.scene.active_id===after.scene.active_id&&JSON.stringify(before.scene.objects.map(o=>[o.id,o.name,o.visible]))===JSON.stringify(after.scene.objects.map(o=>[o.id,o.name,o.visible]))&&before.scene.solo===after.scene.solo&&before.scene.next_id===after.scene.next_id,'Regionchangedscene metadata');for(const o of objects(before).filter(o=>o.id!==before.scene.active_id))assert(stateHash(o.state)===stateHash(byId(after,o.id).state),'Regionchangedinactivepayload');}
 assert(report.subtool_id===pre.active_subtool_id&&report.captured_document_revision===pre.document_revision&&report.document_revision===post.document_revision&&report.kind===captured.mode&&report.erase===captured.erase&&report.vertices===actual.positions.length&&report.projected_vertices===oracle.projected&&report.selected_vertices===oracle.selected&&report.changed_vertices===oracle.changed&&Math.abs(report.max_abs_change-oracle.maxChange)<=1e-12,'Regionreportbinding/counts differ');assert(post.document_revision===pre.document_revision+(oracle.changed?1:0),'Regionrevision notonecommit/noop');assert(JSON.stringify(report.view_projection.map(Math.fround))===JSON.stringify(captured.view_projection.map(Math.fround))&&JSON.stringify(report.viewport)===JSON.stringify(captured.viewport),'Reportdoesnotuse capturedcamera');assert(report.polygon_points===(report.region.kind==='rectangle'?4:report.region.points.length)&&report.point_edge_work===(report.skipped_tiny_click||report.region.kind==='rectangle'?0:actual.positions.length*report.region.points.length),'Regionwork/pointcounts differ');if(report.region.kind==='rectangle')assert(JSON.stringify(report.region.start)===JSON.stringify(captured.start),'Rectpressbinding differs');else assert(JSON.stringify(report.region.points.slice(0,captured.points.length))===JSON.stringify(captured.points),'Rawlassopoints were dropped/reordered');return {mask_bits_exact:true,geometry_metadata_exact:true,inactive_exact:true,projected:oracle.projected,selected:oracle.selected,changed:oracle.changed,max_abs_change:oracle.maxChange,report};
};
const gestureRelease=async(run,release=run.trace.at(-1))=>{await pointer('mouseReleased',...release);await pause(150);const probes=[];for(const delay of [0,200,800]){await pause(delay);const gpu=await gpuDigest();probes.push({at:gpu.observed_at,gpu,metrics:gpu.metrics,sculpt:gpu.sculpt});}const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&!p.metrics.mask_region_state.active&&p.sculpt.pending_dabs===0);const end=await evaluate('performance.now()'),runtime=await evaluate(`({events:window.__brushCheck.events.filter(e=>e.at>=${run.start}&&e.at<=${end}),frames:window.__brushCheck.raf.filter(e=>e.at>=${run.start}&&e.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(e=>e.at>=${run.start}&&e.at<=${end})})`),releaseEvent=runtime.events.filter(e=>e.type==='pointerup'&&e.button===0).at(-1),record={name:run.name,start:run.start,end,probes,quiet,runtime,release_event:releaseEvent};result.strokes.push(record);assert(quiet,'Regionnotquietafter150mssettle');return record;};
const compactRegion=run=>({name:run.name,mode:run.mode,trace:run.trace,pre:run.pre,captured:run.captured,preview:run.preview,preview_gpu:run.gpu,first_capture:run.first,actual_press_event:run.pressEvent,actual_observed_moves:run.observedMoves,options:run.options});
const regionTrial=async(name,mode,traceFactory=rectTrace,options={})=>{const before=await snapshot(name+'-before'),frame=await regionFrame(before.document.mesh),trace=traceFactory(frame),run=await gestureBegin(name,before,mode,trace,options),release=options.release_factory?options.release_factory(frame):trace.at(-1),quiet=await gestureRelease(run,release),post=await state(),after=await snapshot(name+'-after'),measure=regionContract(before.document,after.document,run.pre,post,run.captured,quiet.release_event);return {before,after,frame,run:compactRegion(run),quiet,post,measure};};
const unchangedAfterCancel=async(name,run)=>{await pointer('mouseReleased',...run.trace.at(-1));await pause(150);const post=await state(),after=await snapshot(name+'-after');assert(projectHash(after)===projectHash(run.before),'Cancel/command leakedregionmask');return {post,after,canonical_exact:true};};
const navigationCancel=async(run,button='right')=>{const p=run.trace.at(-1),event={type:'mousePressed',x:p[0],y:p[1],button,buttons:button==='right'?3:5,clickCount:1};result.input_trace.push({...event,at_epoch_ms:Date.now()});await send('Input.dispatchMouseEvent',event);await pause(150);const post=await state();const release={type:'mouseReleased',x:p[0],y:p[1],button,buttons:1,clickCount:1};result.input_trace.push({...release,at_epoch_ms:Date.now()});await send('Input.dispatchMouseEvent',release);const actualEvents=await evaluate(`window.__brushCheck.events.filter(e=>e.at>=${run.start})`);result.navigation_chords??=[];result.navigation_chords.push({name:run.name,button,dispatched_press:event,dispatched_release:release,state_while_navigation_held:post.mask_region_state,actual_events:actualEvents});return {post,actualEvents};};

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


  await fresh();result.initial_controls=await screenshot('initial-controls');const grid=await plainFixture('Independent region grid',gridMesh());
  const rectangles=[];for(const kind of ['forward','reverse','erase']){await fresh();await importFile(grid.path,49);const trace=f=>{const t=[f.at(.1,.25),f.at(.45,.8)];return kind==='reverse'?t.reverse():t;};const t=await regionTrial('rectangle-'+kind,'Rectangle',trace,{erase:kind==='erase'});t.history=await history('rectangle-'+kind,t.before,t.after);rectangles.push({kind,run:t.run,measure:t.measure,history:t.history,after:metadata(t.after)});}result.rectangle_controls=await screenshot('rectangle-controls');
  await finishCase({name:'rectangle-forward-reverse-alt',trials:rectangles,gates:{all_bits_exact:rectangles.every(t=>t.measure.mask_bits_exact),all_geometry_exact:rectangles.every(t=>t.measure.geometry_metadata_exact),all_useful:rectangles.every(t=>t.measure.changed>0),reverse_same_project:rectangles[0].after.sha256===rectangles[1].after.sha256,alt_press_erases:rectangles[2].measure.report.erase===true,history_exact:rectangles.every(t=>t.history.undo_exact&&t.history.redo_exact)}});

  const lassos=[];for(const kind of ['closed-repeated','concave','self-cross']){await fresh();await importFile(grid.path,49);const trace=f=>kind==='closed-repeated'?[f.at(.15,.15),f.at(.85,.15),f.at(.85,.15),f.at(.85,.85),f.at(.15,.85),f.at(.15,.15)]:kind==='concave'?[f.at(.15,.15),f.at(.85,.15),f.at(.85,.4),f.at(.5,.4),f.at(.5,.85),f.at(.15,.85)]:[f.at(.2,.2),f.at(.8,.8),f.at(.2,.8),f.at(.8,.2)];const t=await regionTrial('lasso-'+kind,'Lasso',trace);t.history=await history('lasso-'+kind,t.before,t.after);lassos.push({kind,run:t.run,measure:t.measure,history:t.history});}result.lasso_controls=await screenshot('lasso-controls');
  await finishCase({name:'lasso-closed-concave-selfcross',trials:lassos,gates:{finite_segment_evenodd_bits:lassos.every(t=>t.measure.mask_bits_exact),all_geometry_exact:lassos.every(t=>t.measure.geometry_metadata_exact),all_useful:lassos.every(t=>t.measure.changed>0),large_closed_not_tiny:lassos[0].measure.report.skipped_tiny_click===false,ordered_repeats_kept:lassos[0].measure.report.polygon_points===lassos[0].run.captured.point_count&&lassos[0].run.captured.points.some((p,i,a)=>i&&JSON.stringify(p)===JSON.stringify(a[i-1])),history_exact:lassos.every(t=>t.history.undo_exact&&t.history.redo_exact)}});

  await fresh();await importFile(grid.path,49);let pre=await snapshot('cancel-preview-before'),frame=await regionFrame(pre.document.mesh),run=await gestureBegin('escape-preview',pre,'Rectangle',rectTrace(frame));result.region_preview=await screenshot('region-preview');await generalKey('Escape');const esc=await unchangedAfterCancel('escape-preview',run);
  pre=await snapshot('wheel-preview-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('wheel-frozen',pre,'Rectangle',rectTrace(frame));const ep=run.trace.at(-1),wheel={type:'mouseWheel',x:ep[0],y:ep[1],deltaX:0,deltaY:120};result.input_trace.push({...wheel,at_epoch_ms:Date.now()});await send('Input.dispatchMouseEvent',wheel);await pause(180);const frozen=await state();assert(frozen.mask_region_state.active&&JSON.stringify(frozen.camera.view_projection.map(Math.fround))===JSON.stringify(run.pre.camera.view_projection.map(Math.fround)),'Wheelchangedfrozencamera');run.captured=frozen.mask_region_gesture;const wheelRelease=await gestureRelease(run);const wheelPost=await state(),wheelAfter=await snapshot('wheel-frozen-after'),wheelMeasure=regionContract(pre.document,wheelAfter.document,run.pre,wheelPost,run.captured,wheelRelease.release_event);
  pre=await snapshot('navigation-cancel-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('navigation-cancel',pre,'Rectangle',rectTrace(frame));const navChord=await navigationCancel(run),navState=navChord.post,navActualEvents=navChord.actualEvents,nav=await unchangedAfterCancel('navigation-cancel',run);assert(!navState.mask_region_state.active,'Secondarypressdidnotcancelregion');
  pre=await snapshot('middle-navigation-cancel-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('middle-navigation-cancel',pre,'Rectangle',rectTrace(frame));const middleChord=await navigationCancel(run,'middle'),middleState=middleChord.post,middleActualEvents=middleChord.actualEvents,middle=await unchangedAfterCancel('middle-navigation-cancel',run);assert(!middleState.mask_region_state.active,'Middlepressdidnotcancelregion');
  pre=await snapshot('resize-cancel-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('resize-cancel',pre,'Rectangle',rectTrace(frame));await send('Emulation.setDeviceMetricsOverride',{width:1650,height:1000,deviceScaleFactor:1,mobile:false});await pause(250);const resized=await state();await send('Emulation.setDeviceMetricsOverride',{width:1600,height:1000,deviceScaleFactor:1,mobile:false});await pause(250);result.canvas_rect=await evaluate('(()=>{const r=document.getElementById("obrush_canvas").getBoundingClientRect();return [r.left,r.top,r.right,r.bottom];})()');const resizedResult=await unchangedAfterCancel('resize-cancel',run);
  await fresh();await importFile(grid.path,49);const outside=await regionTrial('outside-release','Rectangle',rectTrace,{release_factory:f=>[Math.round(f.viewport[0]+f.viewport[2]+40),Math.round(f.viewport[1]+f.viewport[3]/2)]});const outsideEnd=outside.measure.report.region.end,vp=outside.run.captured.viewport;
  await finishCase({name:'preview-cancel-frozen-camera-outside',escape:esc,navigation:navState.mask_region_state,navigation_actual_events:navActualEvents,middle_navigation:middleState.mask_region_state,middle_navigation_actual_events:middleActualEvents,resize:resized.mask_region_state,wheel:wheelMeasure,outside:outside.measure,gates:{escape_exact:esc.canonical_exact,wheel_camera_frozen:frozen.mask_region_state.active,wheel_report_bits:wheelMeasure.mask_bits_exact,navigation_cancel_exact:nav.canonical_exact&&!navState.mask_region_state.active,middle_navigation_cancel_exact:middle.canonical_exact&&!middleState.mask_region_state.active,resize_cancel_exact:resizedResult.canonical_exact&&!resized.mask_region_state.active,outside_clamped:outsideEnd[0]===vp[0]+vp[2]&&outsideEnd[1]>=vp[1]&&outsideEnd[1]<=vp[1]+vp[3],outside_bits:outside.measure.mask_bits_exact}});

  await fresh();await importFile(grid.path,49);const ctrl=await regionTrial('ctrl-temporary','Rectangle',rectTrace,{ctrl:true,erase:true});const ctrlHistory=await history('ctrl-temporary',ctrl.before,ctrl.after);
  pre=await snapshot('save-cancel-before');const saveAnchor=pre;frame=await regionFrame(pre.document.mesh);run=await gestureBegin('save-cancel',pre,'Rectangle',rectTrace(frame));const duringSave=await save('save-held-cancels'),saveState=await state(),saveLate=await unchangedAfterCancel('save-held',run);assert(projectHash(duringSave)===projectHash(pre),'Saveappliedpendingregion');
  const first=await regionTrial('barrier-edit','Rectangle',wholeTrace);pre=first.after;frame=await regionFrame(pre.document.mesh);run=await gestureBegin('undo-cancel',pre,'Rectangle',rectTrace(frame));await chord('z');await pointer('mouseReleased',...run.trace.at(-1));await pause(150);const undoHeld=await snapshot('undo-held-after');await chord('z',true);const undoRedo=await snapshot('undo-held-redo');
  pre=await snapshot('brush-cancel-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('brush-cancel',pre,'Rectangle',rectTrace(frame));await key(1);const brushState=await state(),brushLate=await unchangedAfterCancel('brush-command',run);
  const pair=await sceneFixture('Independent region barrier pair',[{id:1,name:'Left grid',state:plain({...gridMesh(),positions:gridMesh().positions.map(p=>[p[0]-3,p[1],p[2]])})},{id:2,name:'Right grid',state:plain(gridMesh())}],2);await fresh();await importFile(grid.path,49);const partImportBefore=await snapshot('part-import-history-before');await importFile(pair.path,49);pre=await snapshot('part-cancel-before');frame=await regionFrame(pre.document.mesh);run=await gestureBegin('part-cancel',pre,'Rectangle',rectTrace(frame));await selected(1);const partState=await state();await pointer('mouseReleased',...run.trace.at(-1));await pause(150);const partAfter=await snapshot('part-cancel-after');const partsExact=objects(pre.document).every(o=>stateHash(o.state)===stateHash(byId(partAfter.document,o.id).state));await chord('z');const partUndo=await snapshot('part-cancel-undo');await chord('z',true);const partRedo=await snapshot('part-cancel-redo');
  await finishCase({name:'command-barriers-and-captured-modifiers',ctrl:ctrl.measure,save_state:saveState.mask_region_state,brush_state:brushState.mask_region_state,part_state:partState.mask_region_state,gates:{ctrl_capture_masks_without_geometry:ctrl.measure.mask_bits_exact&&ctrl.measure.geometry_metadata_exact,alt_press_kept_after_modifier_release:ctrl.measure.report.erase===true,ctrl_history_exact:ctrlHistory.undo_exact&&ctrlHistory.redo_exact,save_cancels_without_apply:projectHash(duringSave)===projectHash(saveAnchor)&&saveLate.canonical_exact===true,undo_cancels_before_history:projectHash(undoHeld)===projectHash(first.before)&&projectHash(undoRedo)===projectHash(first.after),brush_cancel_exact:brushLate.canonical_exact&&!brushState.mask_region_state.active,part_switch_no_mask:partsExact&&!partState.mask_region_state.active&&partAfter.document.scene.active_id===1,selection_and_cancel_add_no_history:projectHash(partUndo)===projectHash(partImportBefore),selection_redo_exact:projectHash(partRedo)===projectHash(partAfter)}});

  const sphere=await plainFixture('Independent rich region sphere',meshFixture([0,0,0],1,[.2,.6,.8],'pattern'));await fresh();await importFile(sphere.path,266);await action('object.add_level');await waitFor(s=>s.subdivision_level===1,'Regiontargetlevelsetupfailed');await store();await action('object.lower');await waitFor(s=>s.subdivision_level===0,'Regioncoarselevelsetupfailed');await newLayer(1);const empty=await snapshot('region-layer-empty');await sculptScene('region-rich-layer-setup',1,empty.document.mesh,10);await intensity(0,.5);await action('layers.visibility.0');await newLayer(2);const rich=await snapshot('region-rich-source');const richPair=await sceneFixture('Independent rich region pair',[{id:1,name:'Inactive grid',state:plain({...gridMesh(),positions:gridMesh().positions.map(p=>[p[0]-3,p[1],p[2]])})},{id:2,name:'Rich sphere',state:activeState(rich.document)}],2);await fresh();await importFile(richPair.path,266);const rt=await regionTrial('rich-region','Rectangle',wholeTrace),oldCache=currentBufferFor(rt.before,1),newCache=currentBufferFor(rt.after,1),rh=await history('rich-region',rt.before,rt.after);result.rich_regions=await screenshot('rich-regions');
  await finishCase({name:'rich-active-only-through-depth',run:rt.run,measure:rt.measure,cache:{before:oldCache,after:newCache},history:rh,gates:{all_projected_active_points_selected:rt.measure.selected===rt.measure.projected&&rt.measure.projected===266,all_active_masks_hardone:rt.after.document.mesh.masks.every(v=>v===1),throughdepth_oracle_bits:rt.measure.mask_bits_exact,levels_layer_target_geometry_exact:rt.measure.geometry_metadata_exact&&rt.after.document.morph_target.creation_level===1,inactive_payload_exact:rt.measure.inactive_exact,inactive_gpu_cache_exact:cacheEqual(oldCache,newCache),history_exact:rh.undo_exact&&rh.redo_exact}});

  const noopTrials=[];for(const kind of ['tiny','empty','saturated']){const m=gridMesh();if(kind==='saturated')m.masks.fill(1);const fixture=await plainFixture('Independent region '+kind+' redo',m);await fresh();await importFile(fixture.path,49);const first=await regionTrial(kind+'-redo-edit','Rectangle',wholeTrace,{erase:kind==='saturated'});await chord('z');const frame=await regionFrame(first.before.document.mesh),trace=kind==='tiny'?[frame.at(.5,.5),frame.at(.5,.5)]:kind==='empty'?[[Math.ceil(frame.viewport[0])+3,Math.ceil(frame.viewport[1])+3],[Math.ceil(frame.viewport[0])+13,Math.ceil(frame.viewport[1])+13]]:wholeTrace(frame);run=await gestureBegin(kind+'-noop',first.before,'Rectangle',trace);const noopRelease=await gestureRelease(run);const post=await state();assert(['dirty','document_revision','snapshot_revision'].every(k=>run.pre[k]===post[k]),'Regionnoopchangeddirty/revisionbeforeSave');const after=await snapshot(kind+'-noop-after'),measure=regionContract(first.before.document,after.document,run.pre,post,run.captured,noopRelease.release_event);await chord('z',true);const redo=await snapshot(kind+'-noop-redo');noopTrials.push({kind,run:compactRegion(run),post,measure,canonical_exact:projectHash(after)===projectHash(first.before),genuine_redo:projectHash(redo)===projectHash(first.after)});}
  await fresh();await importFile(grid.path,49);await modeRegion('Rectangle');await action('scene.visibility.1');const hiddenBefore=await snapshot('hidden-region-before'),hiddenControl=await control('mask.region.lasso',true);await action('mask.region.lasso',true);const v=(await state()).camera.viewport;await pointer('mouseMoved',Math.round((v[0]+v[2])/2),Math.round((v[1]+v[3])/2));await pointer('mousePressed',Math.round((v[0]+v[2])/2),Math.round((v[1]+v[3])/2),'mouse',.5,true);await pointer('mouseMoved',Math.round((v[0]+v[2])/2)+30,Math.round((v[1]+v[3])/2)+20,'mouse',.5,true);await pointer('mouseReleased',Math.round((v[0]+v[2])/2)+30,Math.round((v[1]+v[3])/2)+20);const hiddenAfter=await snapshot('hidden-region-after');
  await fresh();await importFile(grid.path,49);await modeRegion('Rectangle');const priorityBefore=await snapshot('transform-region-before');await action('transform.mode');frame=await regionFrame(priorityBefore.document.mesh);const blank=wholeTrace(frame);await pointer('mouseMoved',...blank[0]);await pointer('mousePressed',...blank[0],'mouse',.5,true);await pointer('mouseMoved',...blank[1],'mouse',.5,true);await pause(150);const priority=await state();await pointer('mouseReleased',...blank[1]);const priorityAfter=await snapshot('transform-region-after');
  await fresh();await importFile(grid.path,49);const capBefore=await snapshot('point-cap-before');frame=await regionFrame(capBefore.document.mesh);const center=frame.at(.5,.5);run=await gestureBegin('point-cap',capBefore,'Lasso',[center,[center[0]+10,center[1]]]);for(let i=0;i<514;i++){const p=i%3===0?[center[0]+10,center[1]+10]:i%3===1?[center[0],center[1]+10]:[center[0]+10,center[1]];await pointer('mouseMoved',...p,'mouse',.5,true);}await pause(150);const capped=await state(),capActualEvents=await evaluate(`window.__brushCheck.events.filter(e=>e.at>=${run.start})`),capResult=await unchangedAfterCancel('point-cap',run);
  await finishCase({name:'tiny-empty-saturated-hidden-priority-caps',trials:noopTrials,cap:capped.mask_region_state,cap_actual_events:capActualEvents,gates:{all_noop_bits_exact:noopTrials.every(t=>t.measure.mask_bits_exact&&t.canonical_exact),all_noop_zero_changes:noopTrials.every(t=>t.measure.changed===0),all_dirty_revision_afterUndo_kept:noopTrials.every(t=>['dirty','document_revision','snapshot_revision'].every(k=>t.run.pre[k]===t.post[k])),genuine_redo_kept:noopTrials.every(t=>t.genuine_redo),tiny_click_skipped:noopTrials[0].measure.report.skipped_tiny_click===true,hidden_controls_disabled:hiddenControl.enabled===false,hidden_masks_unchanged:projectHash(hiddenBefore)===projectHash(hiddenAfter),transform_input_priority:!priority.mask_region_state.active&&projectHash(priorityBefore)===projectHash(priorityAfter),raw_point_cap_cancels:!capped.mask_region_state.active&&/too many points/i.test(capped.mask_region_state.cancel_reason||'')&&capResult.canonical_exact}});

  const schemas=[];for(const version of [2,3,4,5]){const mesh=gridMesh(),payload=plain(mesh);if(version===3)payload.layers=zeroLayer(mesh);if(version===4)payload.morph_target=targetFor(mesh);const fixture=version===5?await sceneFixture('Independent region v5',[{id:1,name:'Inactive schema grid',state:plain({...gridMesh(),positions:gridMesh().positions.map(p=>[p[0]-3,p[1],p[2]])})},{id:2,name:'Active schema grid',state:payload}],2):await publicFixture('Independent region v'+version,{version,name:'Independent region schema',...payload});await fresh();await importFile(fixture.path,49);const t=await regionTrial('region-schema-'+version,'Rectangle',rectTrace),h=await history('region-schema-'+version,t.before,t.after),historical=(await state()).mask_region_report;await chord('z');const undoReport=(await state()).mask_region_report;await chord('z',true);if(version===5)await selected(1);const selectionReport=(await state()).mask_region_report;await fresh();await importFile(t.after.path,49);const loaded=await snapshot('region-schema-'+version+'-loaded');schemas.push({version,measure:t.measure,history:h,version_kept:t.after.document.version===version,canonical_reload:projectHash(loaded)===projectHash(t.after),historical_report_after_undo:JSON.stringify(undoReport)===JSON.stringify(historical),historical_report_after_selection:JSON.stringify(selectionReport)===JSON.stringify(historical),report_cleared:(await state()).mask_region_report===null});}
  await finishCase({name:'v2-v5-freshload-historical-reports',trials:schemas,gates:{versions_retained:schemas.every(t=>t.version_kept),reload_exact:schemas.every(t=>t.canonical_reload),history_exact:schemas.every(t=>t.history.undo_exact&&t.history.redo_exact),historical_report_preserved:schemas.every(t=>t.historical_report_after_undo&&t.historical_report_after_selection),load_clears_reports:schemas.every(t=>t.report_cleared),all_maskbits_exact:schemas.every(t=>t.measure.mask_bits_exact)}});

  await fresh();const original=await snapshot('default-region-source');const selectedDefault=await regionTrial('default-region','Rectangle',f=>[f.at(.35,.35),f.at(.58,.6)]),regionHistory=await history('default-region',selectedDefault.before,selectedDefault.after);const filters=[];for(const op of ['grow','blur']){const t=await filterTrial('region-pipeline-'+op,op,1,.5);t.history=await history('region-pipeline-'+op,t.before,t.after);filters.push(compactFilter(t));}const filtered=await snapshot('default-region-filtered');await configureExtract(.5,.02);await createPart();const extracted=await snapshot('default-region-extracted'),extraction=extractedContract(filtered.document,extracted.document,await state()),extractionHistory=await history('default-region-extract',filtered,extracted);await action('scene.solo');await snapshot('default-region-shell-solo');await control('mask.region.rectangle');result.region_shell=await screenshot('region-shell');result.final_controls=await screenshot('region-final-controls');
  await finishCase({name:'region-filter-extract-source-preservation',region:selectedDefault.measure,filters,extraction,gates:{original_geometry_exact:geometryPayloadHash(activeState(original.document))===geometryPayloadHash(activeState(selectedDefault.after.document)),region_bits_exact:selectedDefault.measure.mask_bits_exact,region_history_exact:regionHistory.undo_exact&&regionHistory.redo_exact,filter_oracles:filters.every(t=>t.measure.max_mask_error<=1e-7),filter_history_exact:filters.every(t=>t.history.undo_exact&&t.history.redo_exact),closed_newpart:extraction.topology.components>=1,original_filtered_source_exact:extraction.source_exact,extraction_history_exact:extractionHistory.undo_exact&&extractionHistory.redo_exact}});
  await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(p=>p.gl_errors),js:result.page_diagnostics.flatMap(p=>p.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(p=>p.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Regionruntimeerrors/MSAA');result.status='passed eight mask region browser workflows';result.inspector_horizontal_bounds={checked_states:result.ui_bounds_checks.length,visible_control_checks:result.ui_bounds_checks.reduce((n,c)=>n+c.visible_controls.length,0),failures:result.ui_bounds_checks.flatMap(c=>c.failures)};result.limitations=['Independent active-only through-depth hard region selection; no occlusion/backface test or ZBrushregion response comparison','Exactcurrent/retainedgeometry/layerf64/target/inactive andmaskbits; retainedlevelmasktransport established separately bycoreaudit','Noopdirty/revision trials afterUndo; clean-state/noop trial notseparate','ActualSecondaryandMiddlenavigationchordcancel tested; browserframegrouping/physicalfocus/touch/nativeGUI unverified','512rawpointoverflow tested onsmallgrid;32Mworkpreflight coreonly, sourcecaps/long-session/physical latency unverified','150msrelease settle beforequietprobes; observationstart beforeGPUreadback, notfirstcommit/input-to-photon','DefaultMouse/radius/strength/Xsym fixture; physicalpressure/alpha/device/fullreference parity remain gaps'];

} catch(error){result.status='failed mask region browser workflow';result.failure=String(error);process.exitCode=1;try{result.failure_runtime=await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})');result.failure_screenshot=await screenshot('failure');}catch{}}
finally{await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('Owned headless session closing'));}pending.clear();await pause(400);if(child.exitCode===null)child.kill('SIGTERM');await pause(200);await new Promise(resolve=>server.close(resolve));result.cleanup={owned_chrome_exit_code:child.exitCode,owned_chrome_signal:child.signalCode,owned_server_closed:true};await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),wasm:result.wasm,cases:result.cases.map(c=>({name:c.name,gates:c.gates}))}));}
