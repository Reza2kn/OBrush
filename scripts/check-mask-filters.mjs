// CLI-only mask filter scalar, geometry and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-mask-filters.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'mask-filters';
const out = path.resolve(`.artifacts/mask-filter-browser/${label}-${randomUUID()}`);
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
  preregistered_contract:{mask_absolute_tolerance:1e-7,report_max_abs_tolerance:1e-12,scalar_policy:'Unique indexed neighbors plus self; synchronous passes with f64 arithmetic and f32 rounding each pass; Grow=max, Shrink=min, Blur=mean blended by strength; Sharpen=.5+(mask-.5)*(1+strength), clamped; isolated graph vertices unchanged',exact_invariants:'Current/retained-level geometry, normals, RGB, ordered faces; layerf64, target binding, inactive payload, canonical history; retained-level mask transport follows existing policy',real_mask_pipeline:'2separate defaultMask contacts radius.18 strength.35 Xsym60x15px24moves; Grow2/.5 Blur1/.5 Sharpen1/.5; Extract threshold.5 thickness.02; independent sheet zero and inward -.125 plus whole incident faces through visible controls',gpu:'Exact visible packed f32 vertex and solid ordered u32 index buffers; unchanged inactive identities/counts/digests',clip_tolerance:.01,quiet:'100ms mouseup settle then0/200/800 probes; no first-commit/physical latency claim',limits:'Source limits, nativeGUI, reference kernels, large/long-session latency unverified'},
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
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,trusted:event.isTrusted,at:performance.now()}),true);
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
   const draws=[];for(const record of raw)draws.push({...record.meta,vertex_sha256:await digest(record.vertices),index_sha256:await digest(record.indices),vertex_words:record.vertices.length<=4096?Array.from(new Float32Array(record.vertices.buffer)):null});
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
  if(metrics.mask_filter_clip_rect){const c=metrics.mask_filter_clip_rect,e=[Math.max(c[0],canvas[0]),Math.max(c[1],canvas[1]),Math.min(c[2],canvas[2]),Math.min(c[3],canvas[3])];assert(e[0]<e[2]&&e[1]<e[3],'Missing actual left mask filter clip');const visible=Object.entries(metrics.mask_filter_controls||{}).filter(([,r])=>r[3]>e[1]&&r[1]<e[3]),failures=visible.filter(([,r])=>r[0]<e[0]-.01||r[2]>e[2]+.01);result.ui_bounds_checks.push({stage:'mask-left:'+stage,clip:c,effective_clip:e,visible_controls:visible.map(([key])=>key),failures});assert(!failures.length,'Mask controls exceed observed LEFT toolbox clip: '+JSON.stringify(failures));}
  if(metrics.extract_clip_rect){const c=metrics.extract_clip_rect,visible=Object.entries(metrics.extract_controls||{}).filter(([,r])=>r[3]>c[1]&&r[1]<c[3]);const failures=visible.filter(([,r])=>r[0]<Math.max(c[0],canvas[0])-.01||r[2]>Math.min(c[2],canvas[2])+.01);result.ui_bounds_checks.push({stage:'extract:'+stage,clip:c,visible_controls:visible.map(([k])=>k),failures});assert(!failures.length,'Extract fields exceed actual inspector clip: '+JSON.stringify(failures));}
  if(metrics.transform_clip_rect){const tc=metrics.transform_clip_rect,visible=Object.entries(metrics.transform_controls||{}).filter(([,r])=>r[3]>tc[1]&&r[1]<tc[3]);const failures=visible.filter(([,r])=>r[0]<tc[0]-.01||r[2]>tc[2]+.01);result.ui_bounds_checks.push({stage:'transform:'+stage,clip:tc,visible_controls:visible.map(([k])=>k),failures});assert(!failures.length,'Transform controls exceed actual inspector clip: '+JSON.stringify(failures));}
  if(metrics.gizmo_clip_rect){const gc=metrics.gizmo_clip_rect,enabled=Object.entries(metrics.gizmo_controls||{}).filter(([key])=>metrics.gizmo_controls_enabled?.[key]);const failures=enabled.filter(([,r])=>r[0]<Math.max(gc[0],canvas[0])-.01||r[1]<Math.max(gc[1],canvas[1])-.01||r[2]>Math.min(gc[2],canvas[2])+.01||r[3]>Math.min(gc[3],canvas[3])+.01);result.ui_bounds_checks.push({stage:'gizmo:'+stage,clip:gc,visible_controls:enabled.map(([key])=>key),failures});assert(!failures.length,'Enabled gizmos exceed actual viewport clip: '+JSON.stringify(failures));}
};
const control=async(key,allowDisabled=false)=>{
  for(let i=0;i<12;i++){
    const s=await state(),rect=s.mask_filter_controls?.[key]||s.extract_controls?.[key]||s.transform_controls?.[key]||s.gizmo_controls?.[key]||s.scene_controls?.[key]||s.morph_controls?.[key]||s.layer_controls?.[key];
    checkSceneBounds(s,'control:'+key);
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&(s.mask_filter_controls_enabled?.[key]??s.extract_controls_enabled?.[key]??s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key])===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.mask_filter_controls_enabled?.[key]??s.extract_controls_enabled?.[key]??s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key]};
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
      await pause(50);const after=await state(),next=after.mask_filter_controls?.[key]||after.extract_controls?.[key]||after.transform_controls?.[key]||after.gizmo_controls?.[key]||after.scene_controls?.[key]||after.morph_controls?.[key]||after.layer_controls?.[key];
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
  if(scene){assert([5,6,7].includes(document.version)&&scene.scheme==='subtools_v1','Unexpected public scene schema');assert(all.length>0&&all.length<=32&&new Set(all.map(o=>o.id)).size===all.length,'Invalid scene identities');assert(all.every(o=>Number.isInteger(o.id)&&o.id>0&&o.id<=0xffffffff)&&scene.next_id>Math.max(...all.map(o=>o.id)),'Invalid stable IDs/allocator');assert(all.filter(o=>o.id===active).length===1&&all.every(o=>o.id===active?o.payload===null:!!o.payload),'Active payload slot is not external/null or inactive payload missing');}else assert(document.version<5||document.version===6,'Only single-mesh legacy or v6 projects can omit a scene');
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
  const byVao=new Map();
  for(const draw of gpu.draws){if(!byVao.has(draw.vao_key))byVao.set(draw.vao_key,[]);byVao.get(draw.vao_key).push(draw);}
  const translucent=visible.filter(o=>document.scene_v7?.objects.find(meta=>meta.id===o.id)?.render?.opacity<1);
  const repeated=[...byVao.values()].filter(group=>group.length>1);
  assert(repeated.length<=translucent.length&&repeated.every(group=>group.length===2&&group.every(d=>d.active===group[0].active&&d.vertex_sha256===group[0].vertex_sha256&&d.index_sha256===group[0].index_sha256&&d.count===group[0].count)&&translucent.some(o=>group[0].active===(o.id===active)&&group[0].index_sha256===indexHash(o.state.mesh)&&group[0].vertex_bytes===o.state.mesh.positions.length*40)),'Unexpected repeated solid GPU draw');
  const unique=[...byVao.values()].map(group=>group[0]);
  const actual=unique.map(d=>JSON.stringify([d.active,d.vertex_sha256,d.index_sha256,d.count,d.vertex_bytes,d.index_bytes])).sort();
  assert(gpu.draws.every(d=>d.type===5125&&d.offset===0),'Unexpected actual solid index type/offset');
  if(visible.some(o=>o.id===active))assert(unique.filter(d=>d.active).length===1,'Actual u_active solid is not unique');else assert(gpu.draws.every(d=>!d.active),'Hidden active object still rendered as active');
  if(document.version===7){
    const unmatched=[...unique];
    for(const object of visible){
      const mesh=object.state.mesh,slot=unmatched.findIndex(d=>d.active===(object.id===active)&&d.index_sha256===indexHash(mesh)&&d.count===mesh.triangles.length*3&&d.vertex_bytes===mesh.positions.length*40&&d.index_bytes===mesh.triangles.length*12);
      if(slot<0)return false;
      const [draw]=unmatched.splice(slot,1);
      if(!draw.vertex_words||draw.vertex_words.length!==mesh.positions.length*10)return false;
      for(let vertex=0;vertex<mesh.positions.length;vertex++){
        const row=draw.vertex_words.slice(vertex*10,vertex*10+10),raw=Math.fround(mesh.masks[vertex]),mask=mesh.mask_convention==='unprotected_f32_v1'?Math.fround(1-Math.max(0,Math.min(1,raw))):raw;
        if(![...mesh.positions[vertex],mask,...mesh.colors[vertex]].every((value,k)=>Math.fround(value)===row[[0,1,2,6,7,8,9][k]]))return false;
        if(!row.slice(3,6).every(Number.isFinite))return false;
      }
    }
    return unmatched.length===0;
  }
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
const extractCubeMesh=()=>({positions:[[-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],[-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1]],normals:Array.from({length:8},()=>[0,0,1]),masks:[1,0,0,0,0,0,0,0],colors:Array.from({length:8},()=>[.3,.5,.7]),triangles:[[0,2,1],[0,3,2],[4,5,6],[4,6,7],[0,1,5],[0,5,4],[3,7,6],[3,6,2],[0,4,7],[0,7,3],[1,2,6],[1,6,5]]});
const extractQuadCubeMesh=()=>{const faces=[[0,3,2,1],[4,5,6,7],[0,1,5,4],[3,7,6,2],[0,4,7,3],[1,2,6,5]],base=extractCubeMesh();base.masks=[0,1,0,0,0,0,0,0];base.triangles=faces.flatMap(([a,b,c,d])=>[[a,b,c],[a,c,d]]);base.surface={faces,uv:{coordinates:faces.flatMap(()=>[[0,0],[1,0],[1,1],[0,1]]),face_indices:faces.map((_,i)=>[i*4,i*4+1,i*4+2,i*4+3])}};return base;};
const extractMixedCubeMesh=()=>{const base=extractCubeMesh(),none=4294967295,faces=[[0,3,2,1],[4,5,6,7],[0,1,5,none],[0,5,4,none],[3,7,6,2],[0,4,7,3],[1,2,6,5]];base.masks=[0,1,0,0,0,0,0,0];base.triangles=faces.flatMap(([a,b,c,d])=>d===none?[[a,b,c]]:[[a,b,c],[a,c,d]]);base.surface={faces,uv:null};return base;};
const plain=mesh=>({mesh,multiresolution:null,layers:null,morph_target:null});
const plainFixture=async(name,mesh)=>publicFixture(name,{version:2,name,mesh});
const configureExtract=async(threshold=.5,thickness=.125)=>{await numeric('extract.threshold',threshold);await numeric('extract.thickness',thickness);const s=await state();assert(Math.fround(s.extract_settings.threshold)===Math.fround(threshold)&&Math.fround(s.extract_settings.thickness)===Math.fround(thickness),'Actual extract numeric fields differ');return s.extract_settings;};
const createPart=async()=>{const before=await state();await action('extract.create');return waitFor(s=>s.subtools.length===before.subtools.length+1&&s.active_subtool_id!==before.active_subtool_id,'Extraction did not create one new active part');};
const createInactivePart=async()=>{const before=await state();await action('extract.create');return waitFor(s=>s.subtools.length===before.subtools.length+1&&s.active_subtool_id===before.active_subtool_id,'Extraction did not append while keeping the source active');};
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


  const blockRemeshSphereOnly=process.argv.includes('--block-remesh-sphere-only');
  const smoothNetsCubeOnly=process.argv.includes('--smooth-nets-cube-only');
  const smoothNetsSphereSculptOnly=process.argv.includes('--smooth-nets-sphere-sculpt-only');
  const smoothNetsSphereSculptSmallOnly=process.argv.includes('--smooth-nets-sphere-sculpt-small-only');
  const smoothNetsSphereOnly=process.argv.includes('--smooth-nets-sphere-only')||smoothNetsSphereSculptOnly||smoothNetsSphereSculptSmallOnly;
  const smoothNetsOnly=smoothNetsCubeOnly||smoothNetsSphereOnly;
  const sphereOnly=blockRemeshSphereOnly||smoothNetsSphereOnly;
  const blockRemeshOnly=process.argv.includes('--block-remesh-only')||blockRemeshSphereOnly;
  if(blockRemeshOnly||smoothNetsOnly){
    await fresh();
    await importFile(path.resolve(sphereOnly?'crates/obrush-core/tests/fixtures/independent-reference-sphere.obj':'crates/obrush-core/tests/fixtures/independent-filter-cube.obj'),sphereOnly?266:8);
    await action('object.rebuild');
    const ready=await state();assert(ready.remesh_block_mode===false&&ready.remesh_surface_nets_mode===false,'Remesh modes did not default off');
    if(sphereOnly){
      const targetResolution=smoothNetsSphereSculptSmallOnly?16:32;
      const slider=await control('object.remesh.resolution');
      const inset=(slider.rect[3]-slider.rect[1])*.3;
      let x=Math.round(slider.rect[0]+inset+(slider.rect[2]-slider.rect[0]-2*inset)*(targetResolution-16)/(128-16));
      for(let attempt=0;attempt<16;attempt++){
        result.ui_actions.push({key:'object.remesh.resolution',target:targetResolution,pointer:[x,slider.point[1]],at_epoch_ms:Date.now()});
        await click([x,slider.point[1]]);
        const observed=(await state()).remesh_resolution;
        if(observed===targetResolution)break;
        x+=observed<targetResolution?1:-1;
      }
      assert((await state()).remesh_resolution===targetResolution,'Visible remesh resolution did not reach '+targetResolution);
    }
    await action(smoothNetsOnly?'object.remesh.smooth_nets':'object.remesh.block');
    let selected=await state();assert(smoothNetsOnly?(selected.remesh_surface_nets_mode===true&&selected.remesh_block_mode===false):selected.remesh_block_mode===true,'Visible remesh mode checkbox did not commit');
    if(smoothNetsOnly){
      await action('object.remesh.block');
      const switched=await state();assert(switched.remesh_block_mode===true&&switched.remesh_surface_nets_mode===false,'Block did not replace Smooth Surface Nets');
      await action('object.remesh.smooth_nets');
      selected=await state();assert(selected.remesh_surface_nets_mode===true&&selected.remesh_block_mode===false,'Smooth Surface Nets did not replace Block');
    }
    const prefix=smoothNetsOnly?'smooth-nets':'block-remesh';
    const before=await snapshot(prefix+'-before');
    await action('object.remesh');
    await waitFor(s=>!s.topology_busy&&s.document_revision>selected.document_revision,'Remesh did not finish a useful edit');
    const after=await snapshot(prefix+'-after'),mesh=activeState(after.document).mesh,topology=closedTopology(mesh);
    const quadFaces=mesh.surface?.faces;
    assert(Array.isArray(quadFaces)&&quadFaces.length*2===mesh.triangles.length&&mesh.surface.uv==null,'Surface Nets output did not retain UV-free quad faces');
    for(let i=0;i<quadFaces.length;i++){
      const [a,b,c,d]=quadFaces[i];
      assert(JSON.stringify(mesh.triangles[i*2])===JSON.stringify([a,b,c])&&JSON.stringify(mesh.triangles[i*2+1])===JSON.stringify([a,c,d]),'Quad metadata does not cover canonical triangles');
    }
    const axisAligned=mesh.triangles.every(face=>{const [a,b,c]=face.map(i=>mesh.positions[i]),normal=cross(sub(b,a),sub(c,a));return normal.filter(v=>Math.abs(v)>1e-8).length===1;});
    assert(smoothNetsOnly?!axisAligned:axisAligned,'Remesh mode produced the wrong face form');
    // The source fixture has per-vertex material channels. OBJ retains the
    // quads but omits those channels, so the visible export requires the
    // explicit attribute-loss choice before it may produce a download.
    await action('file.allow_attribute_loss');
    const exportIndex=await evaluate('window.__brushCaptures.length');
    await action('file.export_mesh');
    let exportSize=0;
    for(let attempt=0;attempt<60;attempt++){exportSize=await evaluate(`window.__brushCaptures[${exportIndex}]?.contents.length||0`);if(exportSize)break;await pause(100);}
    assert(exportSize>0&&exportSize<128*1024*1024,'Visible OBJ export did not finish');
    let exported='';for(let offset=0;offset<exportSize;offset+=65536)exported+=await evaluate(`window.__brushCaptures[${exportIndex}].contents.slice(${offset},${offset+65536})`);
    const objPath=path.join(out,prefix+'-export.obj');await writeFile(objPath,exported);
    const objFaces=exported.split('\n').filter(line=>line.startsWith('f '));
    assert(objFaces.length===quadFaces.length&&objFaces.every(line=>line.trim().split(/\s+/).length===5),'Visible OBJ export did not retain all quads');
    const h=await history(prefix,before,after);
    await importFile(objPath,mesh.positions.length);
    const imported=await snapshot(prefix+'-obj-import');
    const importedMesh=activeState(imported.document).mesh;
    assert(JSON.stringify(importedMesh.surface?.faces)===JSON.stringify(quadFaces),'Visible OBJ import changed Surface Nets quad faces');
    assert(JSON.stringify(importedMesh.positions)===JSON.stringify(mesh.positions),'Visible OBJ import changed Surface Nets positions');
    await finishCase({name:prefix+(sphereOnly?'-sphere':'')+'-visible-workflow',resolution:selected.remesh_resolution,topology,history:h,obj_roundtrip:{export_path:objPath,export_sha256:hash(exported),quad_faces:quadFaces.length,reimport_snapshot:metadata(imported)},gates:{visible_option:smoothNetsOnly?selected.remesh_surface_nets_mode:selected.remesh_block_mode,closed_solid:Math.abs(topology.signed_volume)>1e-6,expected_face_form:smoothNetsOnly?!axisAligned:axisAligned,quad_metadata:quadFaces.length*2===mesh.triangles.length,visible_obj_quad_roundtrip:true,history_exact:h.undo_exact&&h.redo_exact,visible_gpu_save_exact:true}});
    if(smoothNetsSphereSculptOnly||smoothNetsSphereSculptSmallOnly){
      await action('object.subdivision.mode');
      await action('object.subdivision.static_smooth');
      assert((await state()).subdivision_mode==='SculptGL static smooth','Static quad subdivision mode did not commit');
      await action('object.add_level');
      await waitFor(s=>!s.topology_busy&&s.subdivision_levels===2&&s.subdivision_level===1,'Retained quad subdivision did not finish');
      const fine=await snapshot('smooth-nets-subdivided'),fineMesh=activeState(fine.document).mesh;
      assert(fineMesh.surface?.faces.length===quadFaces.length*4&&fineMesh.triangles.length===quadFaces.length*8&&fineMesh.surface.uv==null,'Retained smooth subdivision did not produce four UV-free child quads per face');
      const subdivisionHistory=await history('smooth-nets-subdivision',imported,fine);
      await sculpt('smooth-nets-subdivided-draw',1,[0,0],[18,0],12);
      const edited=await snapshot('smooth-nets-subdivided-sculpted'),editedMesh=activeState(edited.document).mesh;
      const moved=editedMesh.positions.reduce((sum,p,i)=>sum+(p.some((v,c)=>Math.fround(v)!==Math.fround(fineMesh.positions[i][c]))?1:0),0);
      assert(moved>0&&JSON.stringify(editedMesh.surface?.faces)===JSON.stringify(fineMesh.surface.faces),'Draw did not make a useful retained-quad edit');
      const sculptHistory=await history('smooth-nets-subdivided-draw',fine,edited);
      await action('object.lower');
      const lower=await snapshot('smooth-nets-subdivided-lower');
      await action('object.higher');
      const restored=await snapshot('smooth-nets-subdivided-returned');
      assert(JSON.stringify(activeState(lower.document).mesh)===JSON.stringify(importedMesh),'Coarse remesh changed after fine-level Draw');
      assert(JSON.stringify(activeState(restored.document).mesh)===JSON.stringify(editedMesh),'Fine sculpt changed after level navigation');
      await finishCase({name:'smooth-nets-quad-subdivide-sculpt-workflow',base:metadata(imported),fine:metadata(fine),edited:metadata(edited),lower:metadata(lower),restored:metadata(restored),moved_vertices:moved,subdivision_history:subdivisionHistory,sculpt_history:sculptHistory,gates:{quad_child_count:fineMesh.surface.faces.length===quadFaces.length*4,draw_useful:moved>0,quad_faces_retained:JSON.stringify(editedMesh.surface.faces)===JSON.stringify(fineMesh.surface.faces),coarse_exact:JSON.stringify(activeState(lower.document).mesh)===JSON.stringify(importedMesh),fine_navigation_exact:JSON.stringify(activeState(restored.document).mesh)===JSON.stringify(editedMesh),subdivision_history_exact:subdivisionHistory.undo_exact&&subdivisionHistory.redo_exact,sculpt_history_exact:sculptHistory.undo_exact&&sculptHistory.redo_exact,visible_gpu_save_exact:true}});
    }
  } else {
  const posedOnly=process.argv.includes('--posed-only');
  const sculptglOnly=process.argv.includes('--sculptgl-only');
  let quad;
  if(!posedOnly&&!sculptglOnly){
  await fresh();result.initial_controls=await screenshot('initial-controls');
  quad=await plainFixture('Independent filter quad',quadMask());const quadTrials=[];
  const known={grow:[1,.75,1,1],shrink:[0,0,0,0],blur:[.5,1/3,.5,7/12],sharpen:[0,0,1,1]};
  for(const op of ['grow','shrink','blur','sharpen']){await fresh();await importFile(quad.path,4);const t=await filterTrial('quad-'+op,op);t.known_max_error=Math.max(...t.after.document.mesh.masks.map((v,i)=>Math.abs(Math.fround(v)-Math.fround(known[op][i]))));assert(t.known_max_error<=1e-7,'Authored quad known output differs');t.history=await history('quad-'+op,t.before,t.after);quadTrials.push(compactFilter(t));}
  await finishCase({name:'four-analytic-quad-filters',trials:quadTrials,gates:{all_known_outputs:quadTrials.every(t=>t.known_max_error<=1e-7),all_scalar_masks:quadTrials.every(t=>t.measure.max_mask_error<=1e-7),geometry_attributes_exact:quadTrials.every(t=>t.measure.current_and_retained_geometry_exact),all_useful:quadTrials.every(t=>t.measure.changed_vertices>0),all_reports_bound:quadTrials.every(t=>t.measure.report.operation),all_histories_exact:quadTrials.every(t=>t.history.undo_exact&&t.history.redo_exact)}});

  const irregular=await plainFixture('Independent duplicated irregular graph',irregularMask()),irregularTrials=[];
  for(const op of ['grow','shrink','blur','sharpen']){await fresh();await importFile(irregular.path,8);const t=await filterTrial('irregular-'+op,op,3,.37);t.isolated_expected=op==='sharpen'?scalarMaskOracle(t.before.document.mesh,op,3,.37)[7]:Math.fround(t.before.document.mesh.masks[7]);assert(Math.fround(t.after.document.mesh.masks[7])===t.isolated_expected,'Isolated point policy differs');t.history=await history('irregular-'+op,t.before,t.after);irregularTrials.push(compactFilter(t));}
  await finishCase({name:'irregular-indexed-synchronous-fractional-filters',trials:irregularTrials,gates:{unique_neighbor_oracle:irregularTrials.every(t=>t.measure.max_mask_error<=1e-7),duplicate_faces_preserved:irregularTrials.every(t=>t.measure.current_and_retained_geometry_exact),isolated_graph_policy:irregularTrials.filter(t=>t.operation!=='sharpen').every(t=>t.measure.actual_masks[7]===t.isolated_expected),isolated_sharpen_useful:irregularTrials.find(t=>t.operation==='sharpen').measure.actual_masks[7]!==Math.fround(.37),no_coordinate_welding:irregularTrials.find(t=>t.operation==='blur').measure.actual_masks[0]!==irregularTrials.find(t=>t.operation==='blur').measure.actual_masks[4],all_history_exact:irregularTrials.every(t=>t.history.undo_exact&&t.history.redo_exact)}});

  await fresh();const unmasked=await snapshot('painted-before');const firstMask=await sculpt('filter-mask-first',8,[0,0],[60,15],24),firstPainted=await snapshot('painted-first'),secondMask=await sculpt('filter-mask-second',8,[0,0],[60,15],24),painted=await snapshot('painted-second');assert(painted.document.mesh.masks.some(v=>Math.fround(v)>=.5),'Two default mask contacts did not reach preregistered threshold');
  const paintedTrials=[];for(const [op,n,a] of [['grow',2,.5],['blur',1,.5],['sharpen',1,.5]]){const t=await filterTrial('painted-'+op,op,n,a);t.history=await history('painted-'+op,t.before,t.after);paintedTrials.push(compactFilter(t));}
  const filtered=await snapshot('painted-filtered');await configureExtract(.5,.02);await createPart();const extracted=await snapshot('painted-filter-extracted'),extractContract=extractedContract(filtered.document,extracted.document,await state()),extractHistory=await history('painted-filter-extract',filtered,extracted);await action('scene.solo');await snapshot('filtered-extract-solo');result.filtered_shell=await screenshot('filtered-shell');
  await finishCase({name:'actual-repeated-mask-filter-extract',trials:paintedTrials,mask_preparation:{first_max:Math.max(...firstPainted.document.mesh.masks),second_max:Math.max(...painted.document.mesh.masks),first_dabs:firstMask.probes.at(-1).sculpt.applied_dabs,second_dabs:secondMask.probes.at(-1).sculpt.applied_dabs-firstMask.probes.at(-1).sculpt.applied_dabs},extraction:extractContract,gates:{mask_only_positions:positionBits(unmasked.document.mesh.positions)===positionBits(painted.document.mesh.positions),mask_contacts_quiet:firstMask.quiet&&secondMask.quiet,pipeline_oracle:paintedTrials.every(t=>t.measure.max_mask_error<=1e-7),all_filter_histories:paintedTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),source_preserved:extractContract.source_exact,closed_shell:extractContract.topology.components>=1,extract_history:extractHistory.undo_exact&&extractHistory.redo_exact}});

  const signedFixture=await plainFixture('Independent signed extraction sheet',sheetMesh()),signedTrials=[];
  for(const thickness of [0,-.125]){
    const label=thickness===0?'zero':'inward';await fresh();await importFile(signedFixture.path,4);
    const before=await snapshot(label+'-extract-before');await configureExtract(.5,thickness);await createPart();
    const after=await snapshot(label+'-extract-after'),metrics=await state(),newId=after.document.scene?.active_id,part=byId(after.document,newId).state.mesh;
    assert(newId&&!objects(before.document).some(o=>o.id===newId),'Signed extraction did not add a part');
    for(const prior of objects(before.document))assert(stateHash(prior.state)===stateHash(byId(after.document,prior.id).state),'Signed extraction changed source');
    const report=metrics.extract_report,expected=thickness===0?[5,3]:[10,16];
    assert(report&&Math.fround(report.thickness)===Math.fround(thickness)&&report.output_vertices===expected[0]&&report.output_triangles===expected[1],'Signed extraction report/count mismatch');
    assert(part.positions.length===expected[0]&&part.triangles.length===expected[1]&&part.masks.every(v=>Math.fround(v)===0),'Signed extraction output contract differs');
    const coordinateError=Math.max(...part.positions.map(p=>Math.min(Math.abs(p[2]),Math.abs(p[2]-thickness))));
    assert(coordinateError<=2e-6,'Signed extraction Z coordinate differs');
    const topology=thickness===0?{open_patch:true,triangles:part.triangles.length}:closedTopology(part);
    if(thickness!==0)assert(Math.abs(topology.signed_volume-.0625)<=2e-6,'Inward shell signed volume differs');
    const undoRedo=await history(label+'-extract',before,after);
    signedTrials.push({label,thickness,report,topology,coordinateError,history:undoRedo,before:metadata(before),after:metadata(after)});
  }
  await finishCase({name:'zero-and-inward-mask-extraction',trials:signedTrials,gates:{both_ui_values:signedTrials.every(t=>Math.fround(t.report.thickness)===Math.fround(t.thickness)),open_patch:signedTrials[0].topology.open_patch,closed_inward_shell:signedTrials[1].topology.signed_volume>0,all_coordinates:signedTrials.every(t=>t.coordinateError<=2e-6),all_histories:signedTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  const wholeTrials=[];
  for(const thickness of [0,-.125]){
    const label=thickness===0?'open':'inward';await fresh();await importFile(signedFixture.path,4);
    const before=await snapshot('whole-'+label+'-before');await configureExtract(.5,thickness);
    assert((await state()).extract_settings.selection==='clipped','Default extraction policy changed');
    await action('extract.whole_faces');assert((await state()).extract_settings.selection==='whole_faces','Visible selection toggle did not commit');
    await createPart();const after=await snapshot('whole-'+label+'-after'),metrics=await state();
    const part=byId(after.document,after.document.scene.active_id).state.mesh;
    const expected=thickness===0?[4,2]:[8,12],report=metrics.extract_report;
    assert(report&&report.patch_vertices===4&&report.patch_triangles===2&&report.output_vertices===expected[0]&&report.output_triangles===expected[1],'Whole-face counts differ');
    assert(part.positions.length===expected[0]&&part.triangles.length===expected[1],'Whole-face saved mesh differs');
    assert(part.positions.slice(0,4).every((p,i)=>p.every((v,j)=>Math.fround(v)===Math.fround(activeState(before.document).mesh.positions[i][j]))),'Whole-face source points shifted');
    for(const prior of objects(before.document))assert(stateHash(prior.state)===stateHash(byId(after.document,prior.id).state),'Whole-face extraction changed source');
    const topology=thickness===0?{open_patch:true}:closedTopology(part);
    if(thickness!==0)assert(Math.abs(topology.signed_volume-.125)<=2e-6,'Whole-face shell volume differs');
    const undoRedo=await history('whole-'+label,before,after);
    wholeTrials.push({label,thickness,report,topology,history:undoRedo});
  }
  await finishCase({name:'whole-incident-face-extraction',trials:wholeTrials,gates:{both_outputs:wholeTrials.length===2&&wholeTrials[0].report.output_vertices===4&&wholeTrials[1].report.output_vertices===8,open_and_closed:wholeTrials[0].topology.open_patch&&wholeTrials[1].topology.signed_volume>0,source_exact:true,history_exact:wholeTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  await fresh();await importFile(signedFixture.path,4);const retainedBefore=await snapshot('keep-source-before');
  await configureExtract(.5,-.125);await action('extract.whole_faces');await action('extract.keep_source');
  assert((await state()).extract_settings.keep_source_active===true,'Keep-source checkbox did not commit');
  await createInactivePart();const retainedAfter=await snapshot('keep-source-after'),retainedMetrics=await state();
  const sourceId=retainedBefore.document.scene?.active_id??1,newPart=objects(retainedAfter.document).find(o=>!objects(retainedBefore.document).some(old=>old.id===o.id));
  assert(retainedAfter.document.scene?.active_id===sourceId&&newPart&&newPart.state.mesh.positions.length===8&&newPart.state.mesh.triangles.length===12,'Saved keep-source scene differs');
  assert(stateHash(byId(retainedBefore.document,sourceId).state)===stateHash(byId(retainedAfter.document,sourceId).state),'Keep-source extraction changed the original');
  assert(retainedMetrics.extract_report?.new_subtool_id===newPart.id&&retainedMetrics.active_subtool_id===sourceId,'Keep-source report/active binding differs');
  const retainedTopology=closedTopology(newPart.state.mesh),retainedHistory=await history('keep-source-extract',retainedBefore,retainedAfter);
  await finishCase({name:'extraction-keeps-source-active',source_id:sourceId,new_part_id:newPart.id,topology:retainedTopology,history:retainedHistory,gates:{source_stays_active:true,source_exact:true,independent_shell:Math.abs(retainedTopology.signed_volume-.125)<=2e-6,history_exact:retainedHistory.undo_exact&&retainedHistory.redo_exact,visible_gpu_save_exact:true}});

  await fresh();await importFile(signedFixture.path,4);const offsetBefore=await snapshot('surface-offset-before');
  await configureExtract(.5,0);await action('extract.whole_faces');await action('extract.surface_offset');await action('extract.keep_source');
  const offsetSettings=(await state()).extract_settings;
  assert(offsetSettings.selection==='whole_faces'&&offsetSettings.sculptgl_surface_offset===true&&offsetSettings.keep_source_active===true,'Visible surface-offset options did not commit');
  await createInactivePart();const offsetAfter=await snapshot('surface-offset-after'),offsetSourceId=offsetBefore.document.scene?.active_id??1;
  const offsetPart=objects(offsetAfter.document).find(o=>!objects(offsetBefore.document).some(old=>old.id===o.id));
  assert(offsetPart&&offsetAfter.document.scene.active_id===offsetSourceId,'Surface offset did not retain the active source');
  const offsetMesh=offsetPart.state.mesh;
  assert(offsetMesh.positions.length===4&&offsetMesh.triangles.length===2,'Surface offset topology differs');
  assert(offsetMesh.positions.every(p=>Math.abs(p[2]-.01)<=2e-8),'Surface offset Z differs from pinned zero-thickness reference sheet');
  assert(offsetMesh.colors.every(c=>c.every(v=>v===1)),'Surface offset did not reset colors to white');
  assert(offsetMesh.materials?.every(m=>Math.fround(m[0])===Math.fround(.18)&&Math.fround(m[1])===Math.fround(.08)),'Surface offset did not reset material channels');
  assert(stateHash(byId(offsetBefore.document,offsetSourceId).state)===stateHash(byId(offsetAfter.document,offsetSourceId).state),'Surface offset changed source');
  const offsetHistory=await history('surface-offset-extract',offsetBefore,offsetAfter);
  await finishCase({name:'sculptgl-zero-thickness-surface-offset',source_id:offsetSourceId,new_part_id:offsetPart.id,history:offsetHistory,gates:{surface_offset:offsetMesh.positions.every(p=>Math.abs(p[2]-.01)<=2e-8),fresh_channels:true,source_exact:true,history_exact:offsetHistory.undo_exact&&offsetHistory.redo_exact,visible_gpu_save_exact:true}});

  const cubeFixture=await plainFixture('Independent masked extraction cube',extractCubeMesh()),cubeOffsetTrials=[];
  for(const thickness of [-.125,.125]){
    await fresh();await importFile(cubeFixture.path,8);const before=await snapshot('surface-offset-cube-'+thickness+'-before');
    await configureExtract(.5,thickness);await action('extract.whole_faces');await action('extract.surface_offset');await createPart();
    const after=await snapshot('surface-offset-cube-'+thickness+'-after'),part=activeState(after.document).mesh,report=(await state()).extract_report;
    assert(report?.output_vertices===14&&report?.output_triangles===24&&part.positions.length===14&&part.triangles.length===24,'Closed cube extraction counts differ');
    const expected=thickness<0?[-.43371224,-.43371224,-.67261302]:[-.44128779,-.44128779,-.70238703];
    const maxError=Math.min(...part.positions.map(p=>Math.hypot(...p.map((v,i)=>v-expected[i]))));
    assert(maxError<1e-5,'Closed cube smoothed reference point differs: '+maxError);
    const topology=closedTopology(part);
    assert(Math.abs(topology.signed_volume)>1e-6&&Math.sign(topology.signed_volume)===Math.sign(-thickness),'Smoothed cube shell volume/winding differs');
    assert(stateHash(byId(before.document,before.document.scene?.active_id??1).state)===stateHash(byId(after.document,before.document.scene?.active_id??1).state),'Smoothed cube changed source');
    const undoRedo=await history('surface-offset-cube-'+thickness,before,after);
    cubeOffsetTrials.push({thickness,report,max_reference_point_error:maxError,history:undoRedo});
  }
  await finishCase({name:'sculptgl-closed-cube-border-smoothing',trials:cubeOffsetTrials,gates:{both_signed_shells:cubeOffsetTrials.length===2,reference_point_within_1e5:cubeOffsetTrials.every(t=>t.max_reference_point_error<1e-5),source_exact:true,history_exact:cubeOffsetTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  const quadFixture=await publicFixture('Independent masked quad cube with UV seams',{version:6,name:'Independent masked quad cube with UV seams',mesh:extractQuadCubeMesh()}),quadExtractTrials=[];
  for(const thickness of [0,-.125,.125]){
    await fresh();await importFile(quadFixture.path,8);const before=await snapshot('quad-extract-'+thickness+'-before');
    await configureExtract(.5,thickness);await action('extract.whole_faces');await action('extract.surface_offset');await action('extract.keep_source');await createInactivePart();
    const after=await snapshot('quad-extract-'+thickness+'-after'),sourceId=before.document.scene?.active_id??1;
    const partObject=objects(after.document).find(o=>!objects(before.document).some(prior=>prior.id===o.id));
    assert(partObject&&after.document.scene?.active_id===sourceId,'Quad extraction did not retain active UV source');
    const part=partObject.state.mesh,report=(await state()).extract_report,expected=thickness===0?[7,6]:[14,24];
    assert(report?.patch_vertices===7&&report?.patch_triangles===6&&report?.output_vertices===expected[0]&&report?.output_triangles===expected[1],'Quad extraction report differs');
    assert(part.positions.length===expected[0]&&part.triangles.length===expected[1]&&!part.surface,'Quad extracted part topology/UV policy differs');
    const sourceBefore=byId(before.document,sourceId).state,sourceAfter=byId(after.document,sourceId).state;
    assert(sourceBefore.mesh.surface?.uv?.face_indices.length===6&&stateHash(sourceBefore)===stateHash(sourceAfter),'Quad source faces/UV/history changed');
    const referenceCorner=thickness<0?[.9942265,-.9942265,-.9942265]:[1.0057735,-1.0057735,-1.0057735];
    const maxReferenceCornerError=Math.min(...part.positions.map(p=>Math.hypot(...p.map((v,i)=>v-referenceCorner[i]))));
    assert(maxReferenceCornerError<1e-6,'Quad masked-corner offset differs from pinned reference');
    const topology=thickness===0?{open_patch:true}:closedTopology(part);
    if(thickness!==0)assert(Math.abs(topology.signed_volume)>1e-6,'Quad extracted shell has zero volume');
    const undoRedo=await history('quad-extract-'+thickness,before,after);
    quadExtractTrials.push({thickness,report,max_reference_corner_error:maxReferenceCornerError,topology,history:undoRedo});
  }
  await finishCase({name:'source-quad-uv-extraction',trials:quadExtractTrials,gates:{all_signed_counts:quadExtractTrials.length===3,reference_corner_within_1e_minus_6:quadExtractTrials.every(t=>t.max_reference_corner_error<1e-6),original_face_uv_source_exact:true,closed_nonzero_shells:quadExtractTrials.filter(t=>t.thickness!==0).every(t=>Math.abs(t.topology.signed_volume)>1e-6),history_exact:quadExtractTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  const mixedFixture=await publicFixture('Independent masked mixed-face cube',{version:6,name:'Independent masked mixed-face cube',mesh:extractMixedCubeMesh()}),mixedExtractTrials=[];
  for(const thickness of [0,-.125,.125]){
    await fresh();await importFile(mixedFixture.path,8);const before=await snapshot('mixed-extract-'+thickness+'-before');
    await configureExtract(.5,thickness);await action('extract.whole_faces');await action('extract.surface_offset');await action('extract.keep_source');await createInactivePart();
    const after=await snapshot('mixed-extract-'+thickness+'-after'),sourceId=before.document.scene?.active_id??1;
    const partObject=objects(after.document).find(o=>!objects(before.document).some(prior=>prior.id===o.id));
    assert(partObject&&after.document.scene?.active_id===sourceId,'Mixed-face extraction did not retain active source');
    const part=partObject.state.mesh,report=(await state()).extract_report,expected=thickness===0?[6,5]:[12,20];
    assert(report?.selected_source_triangles===5&&report?.patch_vertices===6&&report?.patch_triangles===5&&report?.output_vertices===expected[0]&&report?.output_triangles===expected[1],'Mixed-face extraction selected the wrong source faces');
    assert(part.positions.length===expected[0]&&part.triangles.length===expected[1]&&!part.surface,'Mixed-face extracted topology differs');
    const sourceBefore=byId(before.document,sourceId).state,sourceAfter=byId(after.document,sourceId).state;
    assert(sourceBefore.mesh.surface?.faces.length===7&&stateHash(sourceBefore)===stateHash(sourceAfter),'Mixed-face source changed');
    const referenceCorner=thickness<0?[.99333334,-.99666667,-.99333334]:[1.00666666,-1.00333333,-1.00666666];
    const referenceCornerError=Math.min(...part.positions.map(p=>Math.hypot(...p.map((v,i)=>v-referenceCorner[i]))));
    if(thickness===0)assert(referenceCornerError<1e-6,'Mixed-face zero-thickness reference corner differs');
    const topology=thickness===0?{open_patch:true}:closedTopology(part);
    if(thickness!==0)assert(Math.abs(topology.signed_volume)>1e-6,'Mixed-face shell has zero volume');
    const undoRedo=await history('mixed-extract-'+thickness,before,after);
    mixedExtractTrials.push({thickness,report,reference_corner_error:referenceCornerError,topology,history:undoRedo});
  }
  await finishCase({name:'source-mixed-face-extraction',trials:mixedExtractTrials,gates:{all_signed_counts:mixedExtractTrials.length===3,zero_reference_corner_within_1e_minus_6:mixedExtractTrials[0].reference_corner_error<1e-6,source_exact:true,closed_nonzero_shells:mixedExtractTrials.slice(1).every(t=>Math.abs(t.topology.signed_volume)>1e-6),history_exact:mixedExtractTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  }
  if(!sculptglOnly){
  const posedFixture=path.resolve('crates/obrush-core/tests/fixtures/independent-affine-extract.obrush'),posedTrials=[];
  for(const [sourceId,thickness] of [[2,0],[2,-.125],[2,.125],[1,.125]]){
    await fresh();await importFile(posedFixture,3);if(sourceId===1)await selected(1);
    const before=await snapshot('posed-extract-'+sourceId+'-'+thickness+'-before');
    assert(before.document.version===7&&before.document.scene?.active_id===sourceId,'Posed fixture lost its active v7 source');
    await configureExtract(.5,thickness);await action('extract.whole_faces');await action('extract.surface_offset');await action('extract.keep_source');await createInactivePart();
    const after=await snapshot('posed-extract-'+sourceId+'-'+thickness+'-after');
    const partObject=objects(after.document).find(o=>!objects(before.document).some(prior=>prior.id===o.id));
    assert(partObject&&after.document.scene.active_id===sourceId,'Posed extraction did not retain active source');
    const originalBefore=byId(before.document,sourceId),originalAfter=byId(after.document,sourceId);
    const metadataBefore=before.document.scene_v7.objects.find(o=>o.id===sourceId),metadataAfter=after.document.scene_v7.objects.find(o=>o.id===sourceId);
    assert(JSON.stringify(originalBefore.state)===JSON.stringify(originalAfter.state)&&JSON.stringify(metadataBefore)===JSON.stringify(metadataAfter),'Posed source geometry or affine metadata changed');
    assert(JSON.stringify(before.document.scene_v7.origins)===JSON.stringify(after.document.scene_v7.origins),'Posed source provenance changed');
    const part=partObject.state.mesh,matrix=metadataBefore.affine.local_to_world;
    const world=originalBefore.state.mesh.positions.map(([x,y,z])=>[
      matrix[0]*x+matrix[4]*y+matrix[8]*z+matrix[12],
      matrix[1]*x+matrix[5]*y+matrix[9]*z+matrix[13],
      matrix[2]*x+matrix[6]*y+matrix[10]*z+matrix[14],
    ]);
    const farthestNearest=Math.max(...part.positions.map(point=>Math.min(...world.map(origin=>Math.hypot(...point.map((v,k)=>v-origin[k]))))));
    assert(farthestNearest<(thickness===0?.03:Math.abs(thickness)+.03),'Posed extracted points are not in world coordinates');
    const topology=thickness===0?{open_patch:true}:closedTopology(part);
    assert(!after.document.scene_v7.objects.some(o=>o.id===partObject.id)&&!part.surface,'Posed extracted part retained source affine/UV metadata');
    const undoRedo=await history('posed-extract-'+sourceId+'-'+thickness,before,after);
    posedTrials.push({source_id:sourceId,thickness,vertices:part.positions.length,triangles:part.triangles.length,farthest_world_source_distance:farthestNearest,topology,history:undoRedo});
  }
  await finishCase({name:'posed-source-world-extraction',trials:posedTrials,gates:{all_four_cases:posedTrials.length===4,source_affine_and_origin_exact:true,new_world_parts:true,history_exact:posedTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});
  }

  const referenceFilterFixtures=[
    {name:'cube',file:'independent-filter-cube.obj',vertices:8,masks:{blur:[.5836601257324219,.5352941155433655,.41372549533843994,.5980392098426819,.6627451181411743,.4156862795352936,.5836601257324219,.4784314036369324],sharpen:[0,0,0,.6019608378410339,.6098039746284485,.8490196466445923,1,1]}},
    {name:'open-quad-grid',file:'independent-filter-quad-grid.obj',vertices:9,masks:{blur:[.250980406999588,.6254901885986328,1,.6254901885986328,1,1,1,1,1],sharpen:[0,1,1,1,0,1,1,1,1]}},
  ],referenceFilterTrials=[];
  for(const fixture of referenceFilterFixtures)for(const operation of ['blur','sharpen']){
    await fresh();await importFile(path.resolve('crates/obrush-core/tests/fixtures',fixture.file),fixture.vertices);
    await action('mask.sculptgl_response');
    const options=await state();assert(options.mask_filter_settings.sculptgl_response&&options.mask_filter_controls_enabled['mask.passes']===false&&options.mask_filter_controls_enabled['mask.amount']===false,'SculptGL response toggle/one-pass controls did not commit');
    const before=await snapshot('sculptgl-mask-'+fixture.name+'-'+operation+'-before');
    await action('mask.'+operation);
    const after=await snapshot('sculptgl-mask-'+fixture.name+'-'+operation+'-after'),report=(await state()).mask_filter_report;
    const mesh=after.document.mesh,diffs=mesh.masks.map((mask,index)=>Math.abs((1-Math.fround(mask))-fixture.masks[operation][index])),maxError=Math.max(...diffs);
    assert(maxError<=(operation==='blur'?4e-8:0),'Pinned SculptGL '+operation+' mask weights differ: '+maxError);
    assert(report?.operation?.toLowerCase()===operation&&report.changed_vertices>0,'SculptGL filter report missing useful edit');
    assert(positionBits(before.document.mesh.positions)===positionBits(mesh.positions)&&JSON.stringify(before.document.mesh.triangles)===JSON.stringify(mesh.triangles)&&JSON.stringify(before.document.mesh.colors)===JSON.stringify(mesh.colors),'SculptGL mask response changed geometry or paint');
    const undoRedo=await history('sculptgl-mask-'+fixture.name+'-'+operation,before,after);
    referenceFilterTrials.push({fixture:fixture.name,operation,max_reference_u_error:maxError,report,history:undoRedo});
  }
  await finishCase({name:'sculptgl-mask-blur-sharpen',trials:referenceFilterTrials,gates:{both_reference_responses_on_both_topologies:referenceFilterTrials.length===4,reference_weights_within_f32_rounding:referenceFilterTrials.every(t=>t.max_reference_u_error<=(t.operation==='blur'?4e-8:0)),history_exact:referenceFilterTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),visible_gpu_save_exact:true}});

  if(!posedOnly&&!sculptglOnly){
  const sphere=await plainFixture('Independent rich filter sphere',meshFixture([0,0,0],1,[.2,.6,.8],'pattern'));await fresh();await importFile(sphere.path,266);await action('object.add_level');await waitFor(s=>s.subdivision_level===1,'High level creation failed');await store();await action('object.lower');await waitFor(s=>s.subdivision_level===0,'Coarse level select failed');await newLayer(1);const empty=await snapshot('rich-layer-empty');await sculptScene('filter-rich-layer-setup',1,empty.document.mesh,10);await intensity(0,.5);await action('layers.visibility.0');await newLayer(2);const rich=await snapshot('rich-filter-source');
  const richScene=await sceneFixture('Independent rich filter scene',[{id:1,name:'Inactive independent sheet',state:plain(sheetMesh(1,-3))},{id:2,name:'Rich active filter sphere',state:activeState(rich.document)}],2);await fresh();await importFile(richScene.path,266);const rt=await filterTrial('rich-blur','blur',2,.37),oldCache=currentBufferFor(rt.before,1),newCache=currentBufferFor(rt.after,1),rh=await history('rich-blur',rt.before,rt.after);result.rich_filtered=await screenshot('rich-filtered');
  await finishCase({name:'rich-layer-target-level-inactive-preservation',trial:compactFilter(rt),history:rh,inactive_cache:{before:oldCache,after:newCache},gates:{layer_geometry_f64_exact:rt.measure.current_and_retained_geometry_exact,high_origin_target_kept:rt.after.document.morph_target.creation_level===1,retained_level_geometry_exact:geometryPayloadHash(activeState(rt.before.document))===geometryPayloadHash(activeState(rt.after.document)),inactive_payload_exact:rt.measure.inactive_payload_exact,inactive_gpu_cache_exact:cacheEqual(oldCache,newCache),history_exact:rh.undo_exact&&rh.redo_exact}});

  const noopTrials=[];for(const kind of ['strength-zero','uniform','rounded']){const m=quadMask();if(kind==='uniform')m.masks.fill(.5);if(kind==='rounded')m.masks=[.5,Math.fround(.5000000596046448),.5,Math.fround(.5000000596046448)];const fixture=await plainFixture('Independent '+kind+' redo',m);await fresh();await importFile(fixture.path,4);const origin=await snapshot(kind+'-origin');if(kind==='uniform')await action('mask.clear');else{await configureFilter(1,1);await action('mask.grow');}const edited=await snapshot(kind+'-edited');await chord('z');await configureFilter(1,kind==='strength-zero'?0:kind==='rounded'?.000001:1);const pre=await state();await action('mask.'+(kind==='uniform'?'grow':'blur'));const post=await state();assert(['dirty','document_revision','snapshot_revision'].every(k=>pre[k]===post[k]),'No-op changed revision/dirty beforeSave: '+kind);const untouched=await snapshot(kind+'-noop');assert(projectHash(untouched)===projectHash(origin),'No-op changed canonical state');await chord('z',true);const redo=await snapshot(kind+'-redo');noopTrials.push({kind,pre,post,canonical_unchanged:projectHash(untouched)===projectHash(origin),genuine_redo_retained:projectHash(redo)===projectHash(edited),report:post.mask_filter_report});}
  await finishCase({name:'noops-preserve-dirty-revision-genuine-redo',trials:noopTrials,gates:{all_pre_save_dirty_revision_exact:noopTrials.every(t=>['dirty','document_revision','snapshot_revision'].every(k=>t.pre[k]===t.post[k])),all_masks_canonical_exact:noopTrials.every(t=>t.canonical_unchanged),all_genuine_redo:noopTrials.every(t=>t.genuine_redo_retained),all_zero_report_changes:noopTrials.every(t=>t.report.changed_vertices===0)}});

  const schemaTrials=[];for(const version of [2,3,4,5]){const mesh=quadMask(),schemaState=plain(mesh);if(version===3)schemaState.layers=zeroLayer(mesh);if(version===4)schemaState.morph_target=targetFor(mesh);const fixture=version===5?await sceneFixture('Independent v5 filter import',[{id:1,name:'Inactive schema sheet',state:plain(sheetMesh(1,-3))},{id:2,name:'Active schema quad',state:schemaState}],2):await publicFixture('Independent v'+version+' filter import',{version,name:'Independent schema filter',...schemaState});await fresh();await importFile(fixture.path,4);const t=await filterTrial('schema-'+version,'blur',2,.37),h=await history('schema-'+version,t.before,t.after);await fresh();await importFile(t.after.path,4);const loaded=await snapshot('schema-'+version+'-loaded');schemaTrials.push({version,fixture,trial:compactFilter(t),history:h,version_kept:t.after.document.version===version,fresh_canonical_exact:projectHash(loaded)===projectHash(t.after),report_cleared:(await state()).mask_filter_report===null});}
  await finishCase({name:'v2-v3-v4-v5-schema-roundtrip',trials:schemaTrials,gates:{versions_unchanged:schemaTrials.every(t=>t.version_kept),fresh_canonical_roundtrip:schemaTrials.every(t=>t.fresh_canonical_exact),all_oracles:schemaTrials.every(t=>t.trial.measure.max_mask_error<=1e-7),all_histories:schemaTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),load_clears_reports:schemaTrials.every(t=>t.report_cleared)}});

  await fresh();await importFile(schemaTrials[3].fixture.path,4);const reportTrial=await filterTrial('report-grow','grow'),boundReport=(await state()).mask_filter_report;await chord('z');const undoReport=(await state()).mask_filter_report;await chord('z',true);await selected(1);const selectionReport=(await state()).mask_filter_report;await selected(2);await action('scene.visibility.2');const hidden=await snapshot('hidden-filter-source'),disabled=await control('mask.grow',true);await action('mask.grow',true);const hiddenAfter=await snapshot('hidden-filter-after');await action('scene.visibility.2');await control('mask.grow');await action('transform.mode');await pointer('mouseMoved',800,450);await pause(150);const gm=await state(),gr=gm.gizmo_controls['gizmo.free'];assert(gr&&gm.gizmo_controls_enabled['gizmo.free'],'Missing live-gizmo fixture');const start=[Math.round((gr[0]+gr[2])/2),Math.round((gr[1]+gr[3])/2)];await pointer('mouseMoved',...start);await pointer('mousePressed',...start,'mouse',.5,true);await pointer('mouseMoved',start[0]+20,start[1]+10,'mouse',.5,true);await pause(150);const held=await state();assert(held.transform_settings.dragging,'Live transform setup failed');result.live_filter_controls=await screenshot('live-filter-controls');const heldDisabled=['grow','shrink','blur','sharpen'].every(op=>held.mask_filter_controls_enabled['mask.'+op]===false);await generalKey('Escape');await pointer('mouseReleased',start[0]+20,start[1]+10);await pause(100);const released=await state();await fresh();await importFile(reportTrial.after.path,4);const loadedReport=(await state()).mask_filter_report;
  await finishCase({name:'reports-hidden-live-controls',report:boundReport,gates:{undo_retains_origin_report:JSON.stringify(undoReport)===JSON.stringify(boundReport),selection_retains_origin_report:JSON.stringify(selectionReport)===JSON.stringify(boundReport),hidden_filter_disabled:disabled.enabled===false,hidden_document_unchanged:projectHash(hidden)===projectHash(hiddenAfter),live_filter_disabled:heldDisabled,cancel_reenables_filter:released.mask_filter_controls_enabled['mask.grow']===true,load_clears_report:loadedReport===null}});

  await fresh();await importFile(quad.path,4);const finalTrial=await filterTrial('reject-import-origin','grow');await chord('z');const rejectBefore=await snapshot('reject-import-before'),preReject=await state(),malformed=await publicFixture('Invalid filtered-era project',{...rejectBefore.document,mesh:{...rejectBefore.document.mesh,masks:[2]}}),notice=await importFile(malformed.path,0,true),postReject=await state(),rejectAfter=await snapshot('reject-import-after');await chord('z',true);const retainedRedo=await snapshot('reject-import-redo');await control('mask.sharpen');result.final_controls=await screenshot('filter-final-controls');
  await finishCase({name:'rejected-import-preserves-state-redo',notice,gates:{canonical_document_exact:projectHash(rejectBefore)===projectHash(rejectAfter),revision_dirty_before_save_exact:['document_revision','dirty'].every(k=>preReject[k]===postReject[k]),genuine_redo_exact:projectHash(retainedRedo)===projectHash(finalTrial.after),public_error_notice:!!notice}});
  }
  }
  await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(p=>p.gl_errors),js:result.page_diagnostics.flatMap(p=>p.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(p=>p.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Browser runtime errors or missing MSAA');result.status=smoothNetsOnly?'passed smooth Surface Nets browser workflow':blockRemeshOnly?'passed block remesh browser workflow':'passed mask filter browser workflows';result.inspector_horizontal_bounds={checked_states:result.ui_bounds_checks.length,visible_control_checks:result.ui_bounds_checks.reduce((n,c)=>n+c.visible_controls.length,0),failures:result.ui_bounds_checks.flatMap(c=>c.failures)};result.limitations=blockRemeshOnly||smoothNetsOnly?['One independently authored remesh fixture per browser run; arbitrary topology and sustained performance unverified','Synthetic headless input and browser runtime; native GUI and physical device unverified']:['Independent OBrush indexed-graph/scalar filter policy; ZBrush kernels/response unverified','Current and retained-level geometry exact; mask transport follows existingf32Loop+delta policy, not hidden-levelmask equality','Short synthetic strokes settle100msbeforequietprobes; no first-commit,input-to-photon,nativeGUI/physicaldevice or long-session/source-cap latency claim','Public input/download route and actual buffers, native file picker/download filesystem persistence unverified'];

} catch(error){result.status='failed mask filter browser workflow';result.failure=String(error);process.exitCode=1;try{result.failure_runtime=await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})');result.failure_screenshot=await screenshot('failure');}catch{}}
finally{await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('Owned headless session closing'));}pending.clear();await pause(400);if(child.exitCode===null)child.kill('SIGTERM');await pause(200);await new Promise(resolve=>server.close(resolve));result.cleanup={owned_chrome_exit_code:child.exitCode,owned_chrome_signal:child.signalCode,owned_server_closed:true};await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),wasm:result.wasm,cases:result.cases.map(c=>({name:c.name,gates:c.gates}))}));}
