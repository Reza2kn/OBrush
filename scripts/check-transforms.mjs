// CLI-only whole-part transform geometry and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-transforms.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'transforms';
const pairedReferenceIndex = process.argv.indexOf('--paired-reference');
const pairedReferencePath = pairedReferenceIndex >= 0 ? path.resolve(process.argv[pairedReferenceIndex+1]) : null;
const pairedGizmoIndex = process.argv.indexOf('--paired-gizmo-reference');
const pairedGizmoPath = pairedGizmoIndex >= 0 ? path.resolve(process.argv[pairedGizmoIndex+1]) : null;
const out = path.resolve(`.artifacts/transform-browser/${label}-${randomUUID()}`);
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
  preregistered_contract:{point_absolute_tolerance:2e-6,normal_absolute_tolerance:2e-5,layer_f64_absolute_tolerance:2e-13,whole_part_affine:'p = pivot + scale*(Rz*Ry*Rx)*(p-pivot) + translation; fixed world X then Y then Z; layer delta has no translation',identity_history_attrs:'Exact canonical saved bytes, f32 bits and immutable inactive state',preview:'Actual shader offset, unchanged VBO/IBO bytes/identities/upload counts until one release commit',visible_control_clip_tolerance:.01,release_to_first_committed_render_max_ms:100,release_boundary:'Atomically observed document/snapshot revisions, cleared drag and shader preview, scene frame at or after actual pointerup, expected one active upload or no-op counts, pending dabs zero; 266-point authored active fixture only',release_probe_incremental_delays_ms_after_boundary:[0,200,800],timing_scope:'Short synthetic gestures only; no input-to-photon, native, large-scene or sustained-session claim'},
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
  if(metrics.transform_clip_rect){const tc=metrics.transform_clip_rect,visible=Object.entries(metrics.transform_controls||{}).filter(([,r])=>r[3]>tc[1]&&r[1]<tc[3]);const failures=visible.filter(([,r])=>r[0]<tc[0]-.01||r[2]>tc[2]+.01);result.ui_bounds_checks.push({stage:'transform:'+stage,clip:tc,visible_controls:visible.map(([k])=>k),failures});assert(!failures.length,'Transform controls exceed actual inspector clip: '+JSON.stringify(failures));}
  if(metrics.gizmo_clip_rect){const gc=metrics.gizmo_clip_rect,enabled=Object.entries(metrics.gizmo_controls||{}).filter(([key])=>metrics.gizmo_controls_enabled?.[key]);const failures=enabled.filter(([,r])=>r[0]<Math.max(gc[0],canvas[0])-.01||r[1]<Math.max(gc[1],canvas[1])-.01||r[2]>Math.min(gc[2],canvas[2])+.01||r[3]>Math.min(gc[3],canvas[3])+.01);result.ui_bounds_checks.push({stage:'gizmo:'+stage,clip:gc,visible_controls:enabled.map(([key])=>key),failures});assert(!failures.length,'Enabled gizmos exceed actual viewport clip: '+JSON.stringify(failures));}
};
const control=async(key,allowDisabled=false)=>{
  for(let i=0;i<12;i++){
    const s=await state(),rect=s.transform_controls?.[key]||s.gizmo_controls?.[key]||s.scene_controls?.[key]||s.morph_controls?.[key]||s.layer_controls?.[key];
    checkSceneBounds(s,'control:'+key);
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&(s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key])===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.transform_controls_enabled?.[key]??s.gizmo_controls_enabled?.[key]??s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key]};
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
      await pause(50);const after=await state(),next=after.transform_controls?.[key]||after.gizmo_controls?.[key]||after.scene_controls?.[key]||after.morph_controls?.[key]||after.layer_controls?.[key];
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
  await pause(150);let gpu,saved;if(save_first){saved=await save(name);await pause(150);gpu=await gpuDigest();}else{gpu=await gpuDigest();saved=await save(name);}const metrics=await state();
  checkSceneBounds(metrics,'snapshot:'+name);
  const record={...metadata(saved),metrics,gpu,gpu_matches_saved_scene:matchSceneGpu(saved.document,gpu),scene:sceneContract(saved.document,metrics),composition:composition(saved.document),morph:targetContract(saved.document,metrics)};
  assert(record.gpu_matches_saved_scene,'Actual visible scene GPU vertex/index buffers differ from Save: '+name);
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
const finishCase=async(record)=>{result.cases.push(record);await writeFile(path.join(out,record.name+'.json'),JSON.stringify(record,null,2)+'\n');await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:record.name,gates:record.gates}));assert(Object.values(record.gates).every(Boolean),'Transform case failed: '+record.name);};
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
  if(scene){assert(document.version>=5&&scene.scheme==='subtools_v1','Unexpected public scene schema');assert(all.length>0&&all.length<=32&&new Set(all.map(o=>o.id)).size===all.length,'Invalid scene identities');assert(all.every(o=>Number.isInteger(o.id)&&o.id>0&&o.id<=0xffffffff)&&scene.next_id>Math.max(...all.map(o=>o.id)),'Invalid stable IDs/allocator');assert(all.filter(o=>o.id===active).length===1&&all.every(o=>o.id===active?o.payload===null:!!o.payload),'Active payload slot is not external/null or inactive payload missing');}else assert(document.version<5||document.version===6,'Unexpected scene-less public document version');
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
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,'mouse',.5,true);await pause(70);for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,'mouse',.5,true);await pause(35);}await pointer('mouseReleased',...to);
  const probes=[];for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()'),metrics:await state()});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);assert(quiet,'Scene geometry or queued input changed after release');const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);const trial={name,brush:number,settings,contact,from,to,steps,start,end,quiet,probes,runtime};result.strokes.push(trial);assert(quiet,'Geometry or pending input changed after release: '+name);return trial;
};
const generalKey=async(key)=>{for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key,code:key==='Escape'?'Escape':'Key'+key.toUpperCase(),windowsVirtualKeyCode:key==='Escape'?27:key.toUpperCase().charCodeAt(0),modifiers:0});await pause(120);};
const numeric=async(key,value)=>{
 const c=await control(key);await pointer('mouseMoved',...c.point);await pause(40);
 for(const count of [1,2]){for(const type of ['mousePressed','mouseReleased']){const event={type,x:c.point[0],y:c.point[1],button:'left',buttons:type==='mousePressed'?1:0,clickCount:count};result.input_trace.push({...event,at_epoch_ms:Date.now()});await send('Input.dispatchMouseEvent',event);await pause(25);}await pause(25);}
 await pause(100);await chord('a');await send('Input.insertText',{text:String(value)});
 for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await pause(160);
 result.ui_actions.push({key,numeric_value:value,point:c.point});return state();
};
const configureTransform=async({translation=[0,0,0],rotation_degrees=[0,0,0],scale=1,axis_scale=[1,1,1],pivot_kind='origin'})=>{
 await action('transform.reset');await action('transform.pivot_'+pivot_kind);
 for(const [key,value] of [...translation.map((v,i)=>['transform.t'+['x','y','z'][i],v]),...rotation_degrees.map((v,i)=>['transform.r'+['x','y','z'][i],v]),['transform.scale',scale]])await numeric(key,value);
 if(axis_scale.some(v=>v!==1))for(const [key,value] of axis_scale.map((v,i)=>['transform.s'+['x','y','z'][i],v]))await numeric(key,value);
 const s=(await state()).transform_settings;assert(s&&s.pivot_kind===pivot_kind,'Transform numeric/pivot diagnostics missing');
 for(const k of ['translation','rotation_degrees'])assert(s[k].every((v,i)=>Math.fround(v)===Math.fround((k==='translation'?translation:rotation_degrees)[i])),'Actual numeric input did not set '+k);
 assert(Math.fround(s.scale)===Math.fround(scale)&&s.axis_scale.every((v,i)=>Math.fround(v)===Math.fround(axis_scale[i])),'Actual uniform/axis scale input mismatch');return s;
};
const scalarRotate=(point,rotation)=>{
 const [ax,ay,az]=rotation.map(v=>((Math.fround(v)%360+360)%360)*Math.PI/180),[cx,sx,cy,sy,cz,sz]=[Math.cos(ax),Math.sin(ax),Math.cos(ay),Math.sin(ay),Math.cos(az),Math.sin(az)];
 const x=point[0],y=cx*point[1]-sx*point[2],z=sx*point[1]+cx*point[2];const xx=cy*x+sy*z,zz=-sy*x+cy*z;return [cz*xx-sz*y,sz*xx+cz*y,zz];
};
const expectedPoint=(point,params,linear=false)=>{
 const pivot=params.pivot.map(Math.fround),scale=Math.fround(params.scale),translation=params.translation.map(Math.fround);
 const q=scalarRotate(point.map((v,i)=>linear?v:v-pivot[i]),params.rotation_degrees).map(v=>v*scale);return q.map((v,i)=>linear?v:v+pivot[i]+translation[i]);
};
const affineContract=(before,after,params)=>{
 const a=byId(before,before.scene?.active_id??1).state,b=byId(after,after.scene?.active_id??1).state;const measure={point_max:0,normal_max:0,layer_max:0,target_max:0,point_components:0,layer_components:0};
 if(params.pivot_kind){const expectedPivot=params.pivot_kind==='origin'?[0,0,0]:[0,1,2].map(k=>{const values=a.mesh.positions.map(p=>Math.fround(p[k]));return Math.fround(Math.fround(Math.min(...values)+Math.max(...values))*.5);});assert(params.pivot.every((v,k)=>Math.abs(Math.fround(v)-expectedPivot[k])<=2e-6),'Numeric pivot differs from independently computed active bounds/origin');measure.pivot_max=Math.max(...params.pivot.map((v,k)=>Math.abs(Math.fround(v)-expectedPivot[k])));}
 const points=(old,next,field,linear=false,f32=true)=>{assert(old.length===next.length,'Affine point count changed');for(let i=0;i<old.length;i++){const expected=expectedPoint(old[i].map(v=>f32?Math.fround(v):v),params,linear);for(let k=0;k<3;k++){measure[field]=Math.max(measure[field],Math.abs((f32?Math.fround(next[i][k]):next[i][k])-expected[k]));measure[f32?'point_components':'layer_components']++;}}};
 const meshes=(old,next)=>{assert(JSON.stringify(old.triangles)===JSON.stringify(next.triangles),'Transform changed ordered faces');assert(positionBits(old.masks.map(v=>[v,0,0]))===positionBits(next.masks.map(v=>[v,0,0]))&&positionBits(old.colors)===positionBits(next.colors),'Transform changed mask/color bits');points(old.positions,next.positions,'point_max');for(let i=0;i<old.normals.length;i++){const q=scalarRotate(old.normals[i].map(Math.fround),params.rotation_degrees),n=Math.hypot(...q),expected=q.map(v=>n?v/n:0);for(let k=0;k<3;k++)measure.normal_max=Math.max(measure.normal_max,Math.abs(Math.fround(next.normals[i][k])-expected[k]));}};
 meshes(a.mesh,b.mesh);assert(!!a.multiresolution===!!b.multiresolution&&!!a.layers===!!b.layers&&!!a.morph_target===!!b.morph_target,'Transform removed editable state');
 if(a.multiresolution){assert(a.multiresolution.active_level===b.multiresolution.active_level&&a.multiresolution.levels.length===b.multiresolution.levels.length,'Transform changed retained level structure');for(let i=0;i<a.multiresolution.levels.length;i++)if(a.multiresolution.levels[i])meshes(a.multiresolution.levels[i],b.multiresolution.levels[i]);}
 if(a.layers){assert(a.layers.recording_layer===b.layers.recording_layer&&JSON.stringify(a.layers.triangles)===JSON.stringify(b.layers.triangles),'Transform altered recording/layer topology');points(a.layers.base_positions,b.layers.base_positions,'layer_max',false,false);assert(a.layers.entries.length===b.layers.entries.length,'Layer count changed');for(let i=0;i<a.layers.entries.length;i++){const x=a.layers.entries[i],y=b.layers.entries[i];assert(x.name===y.name&&x.visible===y.visible&&x.intensity===y.intensity,'Transform changed layer metadata');points(x.deltas,y.deltas,'layer_max',true,false);}}
 if(a.morph_target){assert(a.morph_target.creation_level===b.morph_target.creation_level&&JSON.stringify(a.morph_target.triangles)===JSON.stringify(b.morph_target.triangles),'Target correspondence changed');points(a.morph_target.positions,b.morph_target.positions,'target_max');}
 if(before.scene){assert(JSON.stringify(before.scene.objects.map(o=>[o.id,o.name,o.visible]))===JSON.stringify(after.scene.objects.map(o=>[o.id,o.name,o.visible]))&&before.scene.active_id===after.scene.active_id&&before.scene.solo===after.scene.solo&&before.scene.next_id===after.scene.next_id,'Transform changed scene metadata');for(const o of objects(before).filter(o=>o.id!==before.scene.active_id))assert(stateHash(o.state)===stateHash(byId(after,o.id).state),'Inactive object changed');}
 assert(measure.point_max<=2e-6&&measure.target_max<=2e-6&&measure.normal_max<=2e-5&&measure.layer_max<=2e-13,'Independent scalar affine gates failed: '+JSON.stringify(measure));return measure;
};
const transformMode=async(on=true)=>{if((await state()).transform_settings.mode!==on)await action('transform.mode');return waitFor(s=>s.transform_settings.mode===on,'Transform mode did not change');};
const gizmoStart=async(key)=>{
 await transformMode();await evaluate('document.getElementById("obrush_canvas").focus()');await pointer('mouseMoved',800,450);await pause(150);const s=await state(),r=s.gizmo_controls[key];assert(r&&s.gizmo_controls_enabled[key],'Missing/enabled gizmo '+key);const data=s.gizmo_data.find(g=>g.key===key);assert(data,'Missing gizmo world/screen data');const start=key==='gizmo.scale_uniform'||key.startsWith('gizmo.rotate_')?data.screen_end.map(Math.round):[Math.round((r[0]+r[2])/2),Math.round((r[1]+r[3])/2)];await pointer('mouseMoved',...start);await pause(40);await pointer('mousePressed',...start,'mouse',.5,true);await waitFor(s=>s.transform_settings.dragging,'Gizmo did not capture pointer');return {start,data};
};
const previewMove=async(start,end,steps=1)=>{for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',Math.round(start[0]+(end[0]-start[0])*t),Math.round(start[1]+(end[1]-start[1])*t),'mouse',.5,true);await pause(25);}await pause(120);const s=await state();assert(s.transform_settings.dragging,'Preview unexpectedly committed');const gpu=await gpuDigest(),offset=s.transform_settings.preview_translation,allSelected=s.transform_settings.preview_subtool_ids?.length===gpu.draws.length;assert(gpu.draws.every(d=>d.preview_translation.every((v,k)=>Math.fround(v)===Math.fround(allSelected||d.active?offset[k]:0))),'Actual per-draw shader preview offset mismatch');return {settings:s.transform_settings,gpu,metrics:s};};
const previewModelUnchanged=(before,preview)=>['dirty','document_revision','snapshot_revision','active_subtool_id'].every(key=>before[key]===preview[key]);
const previewBuffersEqual=(before,preview)=>{const old=before.draws.map(d=>[d.vertex_key,d.index_key,d.vertex_sha256,d.index_sha256,d.vertex_uploads,d.index_uploads]);const next=preview.draws.map(d=>[d.vertex_key,d.index_key,d.vertex_sha256,d.index_sha256,d.vertex_uploads,d.index_uploads]);return JSON.stringify(old)===JSON.stringify(next);};
const releaseProbes=async(name,end)=>{
 const baselineGpu=await gpuDigest(),baseline=baselineGpu.metrics;
 const edit=baseline.transform_settings.preview_translation.some(v=>v!==0)||baseline.transform_settings.preview_scale.some(v=>v!==1)||baseline.transform_settings.preview_rotation_degrees?.some(v=>v!==0);
 const commits=baseline.transform_settings.dragging&&edit?1:0;
 const expectedRevision=baseline.document_revision+commits;
 const expectedUploads=gpu=>gpu.draws.length===baselineGpu.draws.length&&gpu.draws.every((d,i)=>{
  const old=baselineGpu.draws[i],previewIds=baseline.transform_settings.preview_subtool_ids??[],increment=(previewIds.length===baselineGpu.draws.length||d.active)?commits:0;
  return d.vertex_key===old.vertex_key&&d.index_key===old.index_key&&d.active===old.active&&d.count===old.count&&d.vertex_uploads===old.vertex_uploads+increment&&d.index_uploads===old.index_uploads+increment&&(!increment?(d.vertex_sha256===old.vertex_sha256&&d.index_sha256===old.index_sha256):true);
 });
 const start=await evaluate('performance.now()');await pointer('mouseReleased',...end);
 const release=await evaluate(`window.__brushCheck.events.filter(e=>e.type==='pointerup'&&e.at>=${start}).at(-1)`);
 assert(release&&release.trusted,'Missing actual trusted release event');
 const polls=[];let boundary=null,reason=null;
 for(let attempt=0;attempt<60;attempt++){
  const gpu=await gpuDigest(),sample={at:gpu.observed_at,gpu,sculpt:gpu.sculpt,metrics:gpu.metrics};polls.push(sample);
  if(sample.sculpt.pending_dabs!==0){reason='Pending sculpt dabs during transform finalization';break;}
  const finalized=sample.metrics.document_revision===expectedRevision&&sample.metrics.snapshot_revision===expectedRevision&&!sample.metrics.transform_settings.dragging&&sample.metrics.transform_settings.preview_translation.every(v=>v===0)&&gpu.frame_at>=release.at&&gpu.draws.every(d=>d.preview_translation.every(v=>v===0))&&expectedUploads(gpu);
  if(finalized){boundary=sample;break;}
  if(sample.at-release.at>100){reason='No expected committed render within 100 ms of release for authored 266-point fixture';break;}
  await pause(2);
 }
 if(!boundary&&!reason)reason='No expected committed render in bounded finalization polls';
 if(boundary&&boundary.at-release.at>100)reason='First observed committed render exceeded 100 ms';
 const probes=[];if(boundary&&!reason)for(const delay of [0,200,800]){await pause(delay);const gpu=await gpuDigest();probes.push({at:gpu.observed_at,gpu,sculpt:gpu.sculpt,metrics:gpu.metrics});}
 const quiet=!!boundary&&!reason&&probes.length===3&&probes.every(p=>p.gpu.sha256===boundary.gpu.sha256&&previewBuffersEqual(boundary.gpu,p.gpu)&&p.sculpt.pending_dabs===0&&p.metrics.document_revision===expectedRevision&&p.metrics.snapshot_revision===expectedRevision&&p.metrics.dirty===boundary.metrics.dirty&&!p.metrics.transform_settings.dragging&&p.metrics.transform_settings.preview_translation.every(v=>v===0)&&p.gpu.draws.every(d=>d.preview_translation.every(v=>v===0)));
 const last=await evaluate('performance.now()'),runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${last}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${last}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${last})})`);
 const record={name,start,release_at:release.at,end:last,baseline:{metrics:baseline,gpu:baselineGpu},expected_commit_count:commits,expected_revision:expectedRevision,finalization_polls:polls,boundary,release_to_first_committed_render_ms:boundary?boundary.at-release.at:null,probes,quiet,reason,runtime};
 result.strokes.push(record);assert(quiet,reason||'Transform geometry/uploads/revisions continued after first committed render');return record;
};
const translationParams=(translation)=>({translation,rotation_degrees:[0,0,0],scale:1,pivot:[0,0,0]});


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

  await fresh();result.initial_controls=await screenshot('initial-controls');assert((await state()).subtools?.length===1,'Missing scene diagnostics');

  if(pairedGizmoPath){
    const reference=JSON.parse(await readFile(pairedGizmoPath,'utf8'));
    const sourceCase=reference.cases.find(c=>c.name==='pinned-physical-rotation-arcs');
    assert(sourceCase&&Object.values(sourceCase.gates).every(Boolean),'Pinned physical gizmo reference missing or failed');
    const mapPositions=(candidate,source)=>{
      assert(candidate.positions.length===source.vertices,'Pinned physical gizmo vertex count differs');
      const mapping=[];
      for(let i=0;i<source.vertices;i++){
        const p=source.positions.slice(3*i,3*i+3),j=candidate.positions.findIndex(q=>q.every((v,k)=>Math.fround(v)===Math.fround(p[k])));
        assert(j>=0&&!mapping.includes(j),'Pinned physical gizmo input vertex mapping is missing or ambiguous');mapping.push(j);
      }
      return mapping;
    };
    for(const sourceTrial of sourceCase.trials){
      const axis=sourceTrial.axis,component={x:0,y:1,z:2}[axis];
      const sourceBefore=JSON.parse(await readFile(sourceTrial.before.path,'utf8')).meshes[0];
      const sourceAfter=JSON.parse(await readFile(sourceTrial.after.path,'utf8')).meshes[0];
      await fresh();await importFile(sourceCase.fixture.path,sourceBefore.vertices);await action('transform.respect_masks');
      const before=await snapshot(`paired-physical-${axis}-gesture-before`),start=await gizmoStart(`gizmo.rotate_${axis}`);
      const tangent=start.data.screen_tangent,unit=tangent.map(v=>v/Math.hypot(...tangent)),end=start.start.map((v,i)=>Math.round(v+unit[i]*30));
      const preview=await previewMove(start.start,end,8),degrees=preview.settings.preview_rotation_degrees,dragTangent=preview.settings.drag_rotation_tangent;
      const side=Math.min((await state()).camera.viewport[2]-(await state()).camera.viewport[0],(await state()).camera.viewport[3]-(await state()).camera.viewport[1]);
      const expectedDegrees=-7*end.reduce((sum,v,i)=>sum+(v-start.start[i])*dragTangent[i],0)/side*180/Math.PI;
      await pointer('mouseReleased',...end);await pause(180);
      const after=await snapshot(`paired-physical-${axis}-gesture-after`),gestureHistory=await history(`paired-physical-${axis}-gesture`,before,after);
      const mapping=mapPositions(before.document.mesh,sourceBefore);let inputError=0,weightError=0,gestureError=0,protectedExact=0;
      for(let i=0;i<sourceBefore.vertices;i++){
        const j=mapping[i],old=before.document.mesh.positions[j],next=after.document.mesh.positions[j],weight=sourceBefore.materials[3*i+2],rotated=scalarRotate(old,degrees);
        for(let k=0;k<3;k++){inputError=Math.max(inputError,Math.abs(old[k]-sourceBefore.positions[3*i+k]));gestureError=Math.max(gestureError,Math.abs(next[k]-(old[k]+weight*(rotated[k]-old[k]))));}
        weightError=Math.max(weightError,Math.abs(1-before.document.mesh.masks[j]-weight));
        if(weight===0&&JSON.stringify(old)===JSON.stringify(next))protectedExact++;
      }
      await finishCase({name:`paired-physical-${axis}-gizmo-gesture`,reference_result:pairedGizmoPath,source_angle:sourceTrial.statistics.angle,obrush_degrees:degrees,expected_degrees:expectedDegrees,statistics:{input_error:inputError,weight_error:weightError,gesture_error:gestureError,protected_exact:protectedExact},history:gestureHistory,gates:{source_rule_angle:Math.abs(degrees[component]-expectedDegrees)<.003&&degrees.every((v,i)=>i===component||v===0),same_input:inputError<=1e-6,source_weights:weightError<=1e-6,masked_positions:gestureError<=2e-6&&protectedExact===7,history_exact:gestureHistory.undo_exact&&gestureHistory.redo_exact}});

      await fresh();await importFile(sourceCase.fixture.path,sourceBefore.vertices);await action('transform.respect_masks');
      const numericBefore=await snapshot(`paired-physical-${axis}-numeric-before`),rotation=[0,0,0];rotation[component]=sourceTrial.statistics.angle*180/Math.PI;
      await configureTransform({rotation_degrees:rotation,pivot_kind:'origin'});await action('transform.apply');
      const numericAfter=await snapshot(`paired-physical-${axis}-numeric-after`),numericHistory=await history(`paired-physical-${axis}-numeric`,numericBefore,numericAfter);
      const numericMapping=mapPositions(numericBefore.document.mesh,sourceBefore);let sourceOutputError=0,sourceWeightError=0;
      for(let i=0;i<sourceBefore.vertices;i++){
        const j=numericMapping[i];sourceWeightError=Math.max(sourceWeightError,Math.abs(1-numericBefore.document.mesh.masks[j]-sourceBefore.materials[3*i+2]));
        for(let k=0;k<3;k++)sourceOutputError=Math.max(sourceOutputError,Math.abs(numericAfter.document.mesh.positions[j][k]-sourceAfter.positions[3*i+k]));
      }
      await finishCase({name:`paired-physical-${axis}-same-angle-positions`,reference_result:pairedGizmoPath,source_angle:sourceTrial.statistics.angle,obrush_numeric_degrees:rotation[component],statistics:{source_output_error:sourceOutputError,source_weight_error:sourceWeightError},history:numericHistory,gates:{same_weights:sourceWeightError<=1e-6,same_positions:sourceOutputError<=2e-6,history_exact:numericHistory.undo_exact&&numericHistory.redo_exact}});
    }
    await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(p=>p.gl_errors),js:result.page_diagnostics.flatMap(p=>p.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(p=>p.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Paired physical gizmo browser runtime errors or missing MSAA');result.status='passed pinned physical rotation paired browser fixtures';result.limitations=['Same independent sphere and source angles, but reference and OBrush gizmo screen positions differ','Synthetic browser mouse events; native GUI, physical human/tablet input, alternate poses and sustained performance remain unverified'];
  } else if(pairedReferencePath){
    const reference=JSON.parse(await readFile(pairedReferencePath,'utf8'));
    for(const [kind,parameters] of [['translation',{translation:[.125,0,0],pivot_kind:'origin'}],['affine',{translation:[.02,-.015,.01],rotation_degrees:[7,-3,5],scale:1.05,pivot_kind:'origin'}],['axis',{axis_scale:[1.2,.8,1.05],pivot_kind:'origin'}]]){
    const referenceCase=reference.cases.find(c=>c.name===`pinned-transform-end-mask-weighted-${kind}`);
    assert(referenceCase&&Object.values(referenceCase.gates).every(Boolean),'Pinned reference masked-transform case missing or failed: '+kind);
    const sourceBefore=JSON.parse(await readFile(referenceCase.before.path,'utf8')).meshes[0];
    const sourceAfter=JSON.parse(await readFile(referenceCase.after.path,'utf8')).meshes[0];
    if(kind!=='translation')await fresh();
    await importFile(referenceCase.fixture.path,sourceBefore.vertices);
    await action('transform.respect_masks');
    assert((await state()).transform_settings.respect_masks,'Normal Transform mask option failed to enable');
    const before=await snapshot('paired-mask-'+kind+'-before');
    await configureTransform(parameters);
    await action('transform.apply');
    const after=await snapshot('paired-mask-'+kind+'-after'),pairedHistory=await history('paired-mask-'+kind,before,after);
    const comparison={before_position_max:0,after_position_max:0,unprotected_weight_max:0,protected_exact:0,partial:0,reference_to_obrush_vertex:[]};
    const candidateBefore=before.document.mesh,candidateAfter=after.document.mesh;
    assert(candidateBefore.positions.length===sourceBefore.vertices&&candidateAfter.positions.length===sourceAfter.vertices,'Reference/OBRush vertex counts differ');
    for(let i=0;i<sourceBefore.vertices;i++){
      const sourcePoint=sourceBefore.positions.slice(3*i,3*i+3),j=candidateBefore.positions.findIndex(p=>p.every((v,k)=>v===sourcePoint[k]));
      assert(j>=0&&!comparison.reference_to_obrush_vertex.includes(j),'Reference/OBRush original point mapping is missing or ambiguous');
      comparison.reference_to_obrush_vertex.push(j);
      const weight=sourceBefore.materials[3*i+2],localWeight=1-candidateBefore.masks[j];
      comparison.unprotected_weight_max=Math.max(comparison.unprotected_weight_max,Math.abs(weight-localWeight));
      for(let k=0;k<3;k++){
        comparison.before_position_max=Math.max(comparison.before_position_max,Math.abs(candidateBefore.positions[j][k]-sourceBefore.positions[3*i+k]));
        comparison.after_position_max=Math.max(comparison.after_position_max,Math.abs(candidateAfter.positions[j][k]-sourceAfter.positions[3*i+k]));
      }
      if(weight===0){comparison.protected_exact++;assert(JSON.stringify(candidateBefore.positions[j])===JSON.stringify(candidateAfter.positions[j]),'OBRush moved pinned-reference protected vertex');}
      else if(weight<1)comparison.partial++;
    }
    await finishCase({name:`paired-pinned-masked-${kind}`,reference_result:pairedReferencePath,reference_fixture:referenceCase.fixture,parameters,comparison,history:pairedHistory,gates:{same_vertex_count:true,source_weights:comparison.unprotected_weight_max<=1e-6,source_input_positions:comparison.before_position_max<=1e-6,source_output_positions:comparison.after_position_max<=1e-6,protected_and_partial:comparison.protected_exact>0&&comparison.partial>0,history_exact:pairedHistory.undo_exact&&pairedHistory.redo_exact}});
    }
    await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(p=>p.gl_errors),js:result.page_diagnostics.flatMap(p=>p.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(p=>p.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Paired transform browser runtime errors or missing MSAA');result.status='passed pinned SculptGL masked-transform paired browser fixtures';result.limitations=['One eight-vertex OBJ and controlled reference edit-matrix injections; source gizmo gestures were not exercised','Translation, one rotation/positive-uniform-scale transform and one XYZ axis scale only; broader masks, posed meshes, selected groups, native GUI and sustained latency remain unpaired'];
  } else {

  const seed=await sceneFixture('Independent transform pair',[{id:1,name:'Left independent',state:{mesh:meshFixture([-2,0,0],1,[.3,.6,.2],'pattern')}},{id:2,name:'Right active',state:{mesh:meshFixture([2,0,0],1,[.7,.2,.4],'pattern')}}],2);
  await fresh();await importFile(seed.path,266);await action('object.add_level');await waitFor(s=>s.subdivision_levels===2&&s.subdivision_level===1,'Retained level setup failed');await store();await action('object.lower');await waitFor(s=>s.subdivision_level===0,'Lower level setup failed');await newLayer(1);const empty0=await snapshot('rich-empty-layer-0');await sculptScene('setup-first-layer',1,empty0.document.mesh,10);await intensity(0,.5);await action('layers.visibility.0');await newLayer(2);const empty1=await snapshot('rich-empty-layer-1');await sculptScene('setup-second-layer',1,empty1.document.mesh,10);const rich=await snapshot('rich-transform-baseline');
  assert(rich.document.layers.entries.length===2&&!rich.document.layers.entries[0].visible&&rich.document.layers.entries[0].intensity===.5&&rich.document.layers.recording_layer===1&&rich.document.morph_target.creation_level===1,'Rich fixture missing faded/hidden layer or level-bound target');
  const fullyMasked=structuredClone(rich.document);fullyMasked.mesh.masks.fill(1);const fullFixture=await publicFixture('Fully masked rich transform',fullyMasked);

  await fresh();await importFile(fullFixture.path,266);const beforeT=await snapshot('numeric-translation-before'),uploadsT=(await state()).subtool_uploads.find(s=>s.id===1);const pT=await configureTransform({translation:[.25,-.5,.75]});await action('transform.apply');const afterT=await snapshot('numeric-translation-after'),affT=affineContract(beforeT.document,afterT.document,pT),historyT=await history('numeric-translation',beforeT,afterT);const afterUploadsT=(await state()).subtool_uploads.find(s=>s.id===1);
  await finishCase({name:'numeric-whole-part-translation',parameters:pT,affinity:affT,history:historyT,gates:{fully_masked_points_moved:positionDiff(beforeT.document.mesh,afterT.document.mesh).changed_vertices===266,complete_metadata_retained:afterT.document.layers.entries.length===2&&afterT.document.morph_target.creation_level===1,inactive_uploads_unchanged:uploadsT.uploads===afterUploadsT.uploads,history_exact:historyT.undo_exact&&historyT.redo_exact}});

  await fresh();await importFile(rich.path,266);const beforeRS=await snapshot('numeric-rotate-scale-before');const pRS=await configureTransform({rotation_degrees:[20,-35,50],scale:1.25,pivot_kind:'center'});await action('transform.apply');const afterRS=await snapshot('numeric-rotate-scale-after'),affRS=affineContract(beforeRS.document,afterRS.document,pRS),historyRS=await history('numeric-rotate-scale',beforeRS,afterRS);const persisted=await publicFixture('Transformed rich reload',afterRS.document);await fresh();await importFile(persisted.path,266);const reloadRS=await snapshot('numeric-rotate-scale-reloaded');
  await finishCase({name:'numeric-rotation-scale-pivot-rich-state',parameters:pRS,affinity:affRS,history:historyRS,gates:{scalar_affinity:affRS.point_max<=2e-6&&affRS.layer_max<=2e-13,hidden_faded_layer_preserved:afterRS.document.layers.entries[0].visible===false&&afterRS.document.layers.entries[0].intensity===.5,recording_layer_retained:afterRS.document.layers.recording_layer===1,origin_level_target_retained:afterRS.document.morph_target.creation_level===1,fresh_canonical_reload:projectHash(reloadRS)===projectHash(afterRS)}});

  const frozenLayers=await snapshot('transformed-layers-before');await action('layers.visibility.0');const shownLayers=await snapshot('transformed-hidden-layer-shown');await history('show-transformed-hidden-layer',frozenLayers,shownLayers);await action('layers.bake');const baked=await snapshot('transformed-baked');await action('object.higher');const high=await snapshot('transformed-high-level');await action('object.lower');const lowerAgain=await snapshot('transformed-low-reselected');
  await finishCase({name:'transformed-layer-composition-level-target-retention',gates:{show_is_useful:positionDiff(frozenLayers.document.mesh,shownLayers.document.mesh).changed_vertices>0,layer_composition_exact:result.snapshots.find(s=>s.path===shownLayers.path).composition.present===true&&result.snapshots.find(s=>s.path===shownLayers.path).composition.position_components_differing===0,bake_retains_visible_shape:positionBits(baked.document.mesh.positions)===positionBits(shownLayers.document.mesh.positions),target_available_on_origin:result.snapshots.find(s=>s.path===high.path).metrics.morph_target.available===true,level_return_exact:projectHash(lowerAgain)===projectHash(baked)}});

  await fresh();await importFile(seed.path,266);const beforeG=await snapshot('gizmo-axis-before'),gpuBeforeG=await gpuDigest(),modelBeforeG=await state(),startG=await gizmoStart('gizmo.x'),vG=startG.data.screen_end.map((v,i)=>v-startG.data.screen_start[i]),lenG=Math.hypot(...vG),endG=startG.start.map((v,i)=>Math.round(v+vG[i]/lenG*55));const previewG=await previewMove(startG.start,endG,15);assert(previewBuffersEqual(gpuBeforeG,previewG.gpu),'Preview modified/uploaded geometry buffers');assert(previewModelUnchanged(modelBeforeG,previewG.metrics),'Preview changed dirty/document/snapshot revisions');assert(Math.abs(previewG.settings.preview_translation[0])>.02&&previewG.settings.preview_translation.slice(1).every(v=>v===0),'World X handle leaked another axis or did not move');result.preview_png=await screenshot('translation-preview');const releaseG=await releaseProbes('axis-gizmo-release',endG),afterG=await snapshot('gizmo-axis-after'),affG=affineContract(beforeG.document,afterG.document,translationParams(previewG.settings.preview_translation)),historyG=await history('axis-gizmo',beforeG,afterG);
  await finishCase({name:'world-axis-gizmo-preview-one-commit',preview:previewG.settings,affinity:affG,history:historyG,gates:{no_preview_uploads:previewBuffersEqual(gpuBeforeG,previewG.gpu),no_preview_revision_or_dirty_change:previewModelUnchanged(modelBeforeG,previewG.metrics),one_commit_revision:releaseG.probes[0].metrics.document_revision===modelBeforeG.document_revision+1,axis_only:previewG.settings.preview_translation.slice(1).every(v=>v===0),one_history_state:historyG.undo_exact&&historyG.redo_exact,quiet_release:releaseG.quiet}});

  for(const [plane,normalAxis] of [['x',0],['y',1],['z',2]]){
    await fresh();await importFile(seed.path,266);
    const planeBefore=await snapshot(`plane-${plane}-before`),planeGpuBefore=await gpuDigest(),planeModelBefore=await state();
    const planeStart=await gizmoStart(`gizmo.plane_${plane}`),planeEnd=[planeStart.start[0]+31,planeStart.start[1]-23];
    const planePreview=await previewMove(planeStart.start,planeEnd,8),planePreviewPng=await screenshot(`plane-${plane}-preview`);
    const planeDelta=planePreview.settings.preview_translation,allowed=planeDelta.filter((_,axis)=>axis!==normalAxis);
    assert(Math.hypot(...allowed)>.02&&planeDelta[normalAxis]===0,`Plane ${plane} leaked its normal axis or did not translate`);
    assert(previewBuffersEqual(planeGpuBefore,planePreview.gpu)&&previewModelUnchanged(planeModelBefore,planePreview.metrics),`Plane ${plane} preview changed buffers or document`);
    const planeRelease=await releaseProbes(`plane-${plane}-release`,planeEnd),planeAfter=await snapshot(`plane-${plane}-after`),planeHistory=await history(`plane-${plane}`,planeBefore,planeAfter),planeAffinity=affineContract(planeBefore.document,planeAfter.document,translationParams(planeDelta));
    await finishCase({name:`plane-${plane}-gizmo-preview-one-commit`,translation:planeDelta,preview_png:planePreviewPng,affinity:planeAffinity,history:planeHistory,gates:{plane_only:planeDelta[normalAxis]===0&&Math.hypot(...allowed)>.02,preview_buffers:previewBuffersEqual(planeGpuBefore,planePreview.gpu),preview_document:previewModelUnchanged(planeModelBefore,planePreview.metrics),one_commit:planeRelease.probes[0].metrics.document_revision===planeModelBefore.document_revision+1,quiet_release:planeRelease.quiet,undo_redo:planeHistory.undo_exact&&planeHistory.redo_exact}});
  }

  for(const [axis,component] of [['x',0],['y',1],['z',2]]){
    await fresh();await importFile(seed.path,266);
    const rotateBefore=await snapshot(`rotate-${axis}-before`),rotateGpuBefore=await gpuDigest(),rotateModelBefore=await state();
    const rotateStart=await gizmoStart(`gizmo.rotate_${axis}`),rotateTangent=rotateStart.data.screen_tangent;
    assert(rotateTangent&&Math.hypot(...rotateTangent)>1,`Rotation ${axis} ring lacks projected tangent`);
    const rotateUnit=rotateTangent.map(v=>v/Math.hypot(...rotateTangent));
    const rotateEnd=rotateStart.start.map((v,i)=>Math.round(v+rotateUnit[i]*30));
    const rotatePreview=await previewMove(rotateStart.start,rotateEnd,8),rotateDegrees=rotatePreview.settings.preview_rotation_degrees,rotateCapturedTangent=rotatePreview.settings.drag_rotation_tangent;
    assert(rotateCapturedTangent&&Math.abs(Math.hypot(...rotateCapturedTangent)-1)<1e-5,`Rotation ${axis} pointer-down tangent missing or invalid`);
    const rotateExpected=-7*rotateEnd.reduce((sum,v,i)=>sum+(v-rotateStart.start[i])*rotateCapturedTangent[i],0)/Math.min(rotateModelBefore.camera.viewport[2]-rotateModelBefore.camera.viewport[0],rotateModelBefore.camera.viewport[3]-rotateModelBefore.camera.viewport[1])*180/Math.PI;
    assert(rotateDegrees.every((v,i)=>i===component?Math.abs(v-rotateExpected)<.003:v===0),`Rotation ${axis} differs from source tangent-distance formula`);
    assert(previewBuffersEqual(rotateGpuBefore,rotatePreview.gpu)&&previewModelUnchanged(rotateModelBefore,rotatePreview.metrics),`Rotation ${axis} preview changed held mesh/document`);
    assert(JSON.stringify(rotateGpuBefore.view_projection)!==JSON.stringify(rotatePreview.gpu.view_projection),`Rotation ${axis} preview did not reach model matrix`);
    const rotateRelease=await releaseProbes(`rotate-${axis}-release`,rotateEnd),rotateAfter=await snapshot(`rotate-${axis}-after`),rotateHistory=await history(`rotate-${axis}`,rotateBefore,rotateAfter);
    const rotateAffinity=affineContract(rotateBefore.document,rotateAfter.document,{translation:[0,0,0],rotation_degrees:rotateDegrees,scale:1,pivot:rotateStart.data.pivot});
    await finishCase({name:`world-${axis}-rotation-arc-preview-one-commit`,rotation_degrees:rotateDegrees,captured_tangent:rotateCapturedTangent,source_formula_degrees:rotateExpected,pivot:rotateStart.data.pivot,affinity:rotateAffinity,history:rotateHistory,gates:{axis_only:rotateDegrees.every((v,i)=>i===component?Math.abs(v)>1:v===0),source_angle:Math.abs(rotateDegrees[component]-rotateExpected)<.003,preview_matrix:JSON.stringify(rotateGpuBefore.view_projection)!==JSON.stringify(rotatePreview.gpu.view_projection),preview_buffers:previewBuffersEqual(rotateGpuBefore,rotatePreview.gpu),preview_document:previewModelUnchanged(rotateModelBefore,rotatePreview.metrics),one_commit:rotateRelease.probes[0].metrics.document_revision===rotateModelBefore.document_revision+1,quiet_release:rotateRelease.quiet,undo_redo:rotateHistory.undo_exact&&rotateHistory.redo_exact}});
  }

  await fresh();await importFile(seed.path,266);const scaleBefore=await snapshot('scale-gizmo-before'),scaleGpuBefore=await gpuDigest(),scaleModelBefore=await state(),scaleStart=await gizmoStart('gizmo.scale_x'),scaleVector=scaleStart.data.screen_end.map((v,i)=>v-scaleStart.data.screen_start[i]),scaleVectorLength=Math.hypot(...scaleVector),scaleEnd=scaleStart.start.map((v,i)=>Math.round(v+scaleVector[i]/scaleVectorLength*35));const scalePreview=await previewMove(scaleStart.start,scaleEnd,8);const axisUnit=scaleVector.map(v=>v/scaleVectorLength),scaleSourceFactor=Math.max(.01,Math.abs(scaleEnd.reduce((sum,v,i)=>sum+(v-scaleStart.data.screen_start[i])*axisUnit[i],0))/Math.hypot(...scaleStart.start.map((v,i)=>v-scaleStart.data.screen_start[i])));assert(Math.abs(scalePreview.settings.preview_scale[0]-scaleSourceFactor)<1e-5,'Axis scale differs from pinned screen-distance formula');const scalePreviewPng=await screenshot('scale-gizmo-preview');assert(previewBuffersEqual(scaleGpuBefore,scalePreview.gpu),'Scale preview changed mesh buffers');assert(previewModelUnchanged(scaleModelBefore,scalePreview.metrics),'Scale preview changed document state');assert(scalePreview.settings.preview_scale[0]>1.05&&scalePreview.settings.preview_scale.slice(1).every(v=>v===1),'Scale handle factor is not X-only');assert(JSON.stringify(scaleGpuBefore.view_projection)!==JSON.stringify(scalePreview.gpu.view_projection),'Scale preview did not reach the renderer matrix');await pointer('mouseReleased',...scaleEnd);await pause(180);const scaleAfter=await snapshot('scale-gizmo-after'),scaleAfterMetrics=await state(),scaleHistory=await history('scale-gizmo',scaleBefore,scaleAfter),scaleFactor=scalePreview.settings.preview_scale[0],scalePivot=scaleStart.data.pivot;let scaleError=0;for(let i=0;i<scaleBefore.document.mesh.positions.length;i++)for(let k=0;k<3;k++){const expected=k===0?scalePivot[0]+scaleFactor*(scaleBefore.document.mesh.positions[i][0]-scalePivot[0]):scaleBefore.document.mesh.positions[i][k];scaleError=Math.max(scaleError,Math.abs(expected-scaleAfter.document.mesh.positions[i][k]));}await finishCase({name:'world-axis-scale-gizmo-preview-one-commit',scale_factor:scaleFactor,source_formula_factor:scaleSourceFactor,pivot:scalePivot,max_position_error:scaleError,preview_png:scalePreviewPng,history:scaleHistory,gates:{preview_uses_matrix:JSON.stringify(scaleGpuBefore.view_projection)!==JSON.stringify(scalePreview.gpu.view_projection),preview_keeps_buffers:previewBuffersEqual(scaleGpuBefore,scalePreview.gpu),preview_keeps_document:previewModelUnchanged(scaleModelBefore,scalePreview.metrics),single_commit:scaleAfterMetrics.document_revision===scaleModelBefore.document_revision+1,positions:scaleError<=2e-6,undo_redo:scaleHistory.undo_exact&&scaleHistory.redo_exact}});

  await fresh();
  await importFile(seed.path,266);
  const uniformBefore=await snapshot('uniform-ring-before');
  const uniformGpuBefore=await gpuDigest(),uniformModelBefore=await state();
  const uniformStart=await gizmoStart('gizmo.scale_uniform');
  const ringVector=uniformStart.start.map((v,i)=>v-uniformStart.data.screen_start[i]);
  const ringDistance=Math.hypot(...ringVector);
  const uniformEnd=uniformStart.start.map((v,i)=>Math.round(v+ringVector[i]/ringDistance*35));
  const uniformPreview=await previewMove(uniformStart.start,uniformEnd,8);
  const uniformPreviewPng=await screenshot('uniform-ring-preview');
  const uniformFactor=uniformPreview.settings.preview_scale[0];const uniformSourceFactor=Math.max(.01,Math.hypot(...uniformEnd.map((v,i)=>v-uniformStart.data.screen_start[i]))/ringDistance);assert(Math.abs(uniformFactor-uniformSourceFactor)<1e-5,'Uniform scale differs from pinned screen-distance formula');
  assert(uniformFactor>1.1&&uniformPreview.settings.preview_scale.every(v=>Math.fround(v)===Math.fround(uniformFactor)),'Uniform ring did not preview an equal XYZ factor');
  assert(previewBuffersEqual(uniformGpuBefore,uniformPreview.gpu)&&previewModelUnchanged(uniformModelBefore,uniformPreview.metrics),'Uniform preview mutated geometry or document');
  await pointer('mouseReleased',...uniformEnd);await pause(180);
  const uniformAfter=await snapshot('uniform-ring-after'),uniformAfterMetrics=await state();
  const uniformHistory=await history('uniform-ring',uniformBefore,uniformAfter);
  let uniformError=0;for(let i=0;i<uniformBefore.document.mesh.positions.length;i++)for(let k=0;k<3;k++)uniformError=Math.max(uniformError,Math.abs(uniformStart.data.pivot[k]+uniformFactor*(uniformBefore.document.mesh.positions[i][k]-uniformStart.data.pivot[k])-uniformAfter.document.mesh.positions[i][k]));
  await finishCase({name:'uniform-ring-scale-preview-one-commit',factor:uniformFactor,source_formula_factor:uniformSourceFactor,pivot:uniformStart.data.pivot,max_position_error:uniformError,preview_png:uniformPreviewPng,history:uniformHistory,gates:{preview_matrix:JSON.stringify(uniformGpuBefore.view_projection)!==JSON.stringify(uniformPreview.gpu.view_projection),preview_buffers:previewBuffersEqual(uniformGpuBefore,uniformPreview.gpu),preview_document:previewModelUnchanged(uniformModelBefore,uniformPreview.metrics),one_commit:uniformAfterMetrics.document_revision===uniformModelBefore.document_revision+1,positions:uniformError<=2e-6,undo_redo:uniformHistory.undo_exact&&uniformHistory.redo_exact}});

  const freeTrials=[];for(const steps of [1,20]){await fresh();await importFile(seed.path,266);const before=await snapshot('free-'+steps+'-before'),gpu=await gpuDigest(),modelBefore=await state(),start=await gizmoStart('gizmo.free'),end=[start.start[0]+48,start.start[1]-34],preview=await previewMove(start.start,end,steps);assert(previewBuffersEqual(gpu,preview.gpu),'Free preview uploads');assert(previewModelUnchanged(modelBefore,preview.metrics),'Free preview changed dirty/revisions');const release=await releaseProbes('free-'+steps+'-release',end),after=await snapshot('free-'+steps+'-after');freeTrials.push({steps,before,after,model_before:modelBefore,preview_model_unchanged:previewModelUnchanged(modelBefore,preview.metrics),one_commit_revision:release.probes[0].metrics.document_revision===modelBefore.document_revision+1,preview:preview.settings,affinity:affineContract(before.document,after.document,translationParams(preview.settings.preview_translation)),release_quiet:release.quiet,history:await history('free-'+steps,before,after)});}
  await finishCase({name:'free-plane-total-displacement-event-chunking',trials:freeTrials.map(({before,after,...rest})=>({...rest,before:metadata(before),after:metadata(after)})),gates:{same_frozen_before:projectHash(freeTrials[0].before)===projectHash(freeTrials[1].before),same_final_project:projectHash(freeTrials[0].after)===projectHash(freeTrials[1].after),both_quiet:freeTrials.every(t=>t.release_quiet),both_preview_models_unchanged:freeTrials.every(t=>t.preview_model_unchanged),both_commit_once:freeTrials.every(t=>t.one_commit_revision),both_histories_exact:freeTrials.every(t=>t.history.undo_exact&&t.history.redo_exact)}});

  await chord('z');const redoAnchor=await snapshot('cancel-redo-anchor');const cancelModel=await state(),cancelStart=await gizmoStart('gizmo.free'),cancelEnd=[cancelStart.start[0]+45,cancelStart.start[1]+20];const cancelPreview=await previewMove(cancelStart.start,cancelEnd,8);assert(previewModelUnchanged(cancelModel,cancelPreview.metrics),'Cancel preview changed dirty/revisions');await generalKey('Escape');await pointer('mouseReleased',...cancelEnd);await pause(80);const afterCancelModel=await state();assert(previewModelUnchanged(cancelModel,afterCancelModel),'Escape changed dirty/revisions');const canceled=await snapshot('escape-canceled');assert(projectHash(canceled)===projectHash(redoAnchor),'Escape changed document');await chord('z',true);const canceledRedo=await snapshot('escape-redo-retained');await chord('z');const returnedBefore=await snapshot('exact-return-before'),returnModel=await state(),returnStart=await gizmoStart('gizmo.free');await previewMove(returnStart.start,[returnStart.start[0]+50,returnStart.start[1]-20],6);const zeroPreview=await previewMove(returnStart.start,returnStart.start,6);const zeroRelease=await releaseProbes('exact-return-release',returnStart.start);const returned=await snapshot('exact-return-after');await chord('z',true);const returnedRedo=await snapshot('exact-return-redo');
  await finishCase({name:'preview-cancel-exact-return-preserve-redo',gates:{escape_canonical_unchanged:projectHash(canceled)===projectHash(redoAnchor),escape_redo_retained:projectHash(canceledRedo)===projectHash(freeTrials[1].after),escape_model_unchanged:previewModelUnchanged(cancelModel,afterCancelModel),exact_return_model_unchanged:previewModelUnchanged(returnModel,zeroRelease.probes[0].metrics),exact_return_zero:zeroPreview.settings.preview_translation.every(v=>v===0),exact_return_canonical:projectHash(returnedBefore)===projectHash(returned),exact_return_redo:projectHash(returnedRedo)===projectHash(freeTrials[1].after),quiet:zeroRelease.quiet}});

  await fresh();await importFile(seed.path,266);const beforeSave=await snapshot('held-save-before'),held=await gizmoStart('gizmo.free'),heldEnd=[held.start[0]+42,held.start[1]+25],heldPreview=await previewMove(held.start,heldEnd,12);const midSaved=await snapshot('held-save-commits-preview',{save_first:true});const midAff=affineContract(beforeSave.document,midSaved.document,translationParams(heldPreview.settings.preview_translation));assert(!(await state()).transform_settings.dragging,'Save failed to commit/clear preview');const lateRelease=await releaseProbes('held-save-late-release',heldEnd),afterLate=await snapshot('held-save-after-late-release'),heldHistory=await history('held-save',beforeSave,midSaved);await chord('z');const invalidBefore=await snapshot('invalid-transform-before');await configureTransform({translation:[999999,0,0]});await action('transform.apply');const rejectedMessage=(await state()).message,rejected=await snapshot('invalid-transform-after');assert(projectHash(invalidBefore)===projectHash(rejected),'Invalid transform changed project');await chord('z',true);const invalidRedo=await snapshot('invalid-transform-redo');
  await finishCase({name:'save-barrier-late-release-invalid-atomic-redo',affinity:midAff,rejected_message:rejectedMessage,gates:{save_commits_visible_preview:midAff.point_max<=2e-6,late_release_no_extra_edit:projectHash(afterLate)===projectHash(midSaved),one_undo:heldHistory.undo_exact&&heldHistory.redo_exact,invalid_preserves_document:projectHash(invalidBefore)===projectHash(rejected),invalid_preserves_redo:projectHash(invalidRedo)===projectHash(midSaved),quiet:lateRelease.quiet}});

  await fresh();await importFile(seed.path,266);
  const selectOne=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...selectOne.point,'mouse',.5,down,2);
  const selectedGroup=await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Ctrl-click did not select both parts');
  const groupBefore=await snapshot('selected-group-gizmo-before'),groupGpuBefore=await gpuDigest(),groupRevision=selectedGroup.document_revision;
  const groupStart=await gizmoStart('gizmo.x'),groupVector=groupStart.data.screen_end.map((v,i)=>v-groupStart.data.screen_start[i]),groupLength=Math.hypot(...groupVector),groupEnd=groupStart.start.map((v,i)=>Math.round(v+groupVector[i]/groupLength*55));
  for(let i=1;i<=15;i++){const t=i/15;await pointer('mouseMoved',Math.round(groupStart.start[0]+(groupEnd[0]-groupStart.start[0])*t),Math.round(groupStart.start[1]+(groupEnd[1]-groupStart.start[1])*t),'mouse',.5,true);await pause(25);}
  await pause(120);const groupPreview=await gpuDigest(),groupPreviewMetrics=await state(),groupTranslation=groupPreviewMetrics.transform_settings.preview_translation;
  const groupPreviewAll=groupPreview.draws.length===2&&groupPreview.draws.every(d=>d.preview_translation.every((v,k)=>Math.fround(v)===Math.fround(groupTranslation[k])));
  const groupBuffersStable=previewBuffersEqual(groupGpuBefore,groupPreview);
  assert(groupPreviewAll&&groupBuffersStable&&groupPreviewMetrics.document_revision===groupRevision,'Group gizmo preview did not move both selected rendered parts without editing buffers');
  await pointer('mouseReleased',...groupEnd);
  await waitFor(s=>s.document_revision===groupRevision+1&&!s.transform_settings.dragging,'Group gizmo did not commit once');
  const groupAfter=await snapshot('selected-group-gizmo-after'),groupAfterGpu=await gpuDigest(),groupAfterMetrics=await state(),groupHistory=await history('selected-group-gizmo',groupBefore,groupAfter);
  const groupErrors=[1,2].map(id=>{const before=byId(groupBefore.document,id).state.mesh.positions,after=byId(groupAfter.document,id).state.mesh.positions;return Math.max(...before.flatMap((p,i)=>p.map((v,k)=>Math.abs(after[i][k]-v-groupTranslation[k]))));});
  await finishCase({name:'selected-group-axis-gizmo-preview-one-commit',selection:selectedGroup.display_flags.selected_ids,preview_translation:groupTranslation,point_max_errors:groupErrors,history:groupHistory,gates:{two_selected:groupAfter.document.scene_v7?.selected_ids?.length===2,group_pivot_center:Math.abs(groupStart.data.pivot[0])<=2e-6,both_previewed:groupPreviewAll,preview_buffers_unchanged:groupBuffersStable,one_commit:groupAfterMetrics.document_revision===groupRevision+1,both_world_translations:groupErrors.every(x=>x<=2e-6),gpu_matches_committed_scene:matchSceneGpu(groupAfter.document,groupAfterGpu),history_exact:groupHistory.undo_exact&&groupHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const maskSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...maskSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Masked fixture did not select both parts');
  await action('transform.respect_masks');
  assert((await state()).transform_settings.respect_masks,'Visible mask-weighted transform option did not enable');
  const maskedBefore=await snapshot('selected-group-mask-before');
  await configureTransform({translation:[.025,0,0]});
  await action('transform.apply');
  const maskedAfter=await snapshot('selected-group-mask-after'),maskedHistory=await history('selected-group-mask',maskedBefore,maskedAfter);
  const maskedStats={max_error:0,fully_protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(maskedBefore.document,id).state.mesh,next=byId(maskedAfter.document,id).state.mesh;
    assert(prior.positions.length===next.positions.length&&JSON.stringify(prior.masks)===JSON.stringify(next.masks),'Masked transform changed topology or masks');
    for(let i=0;i<prior.positions.length;i++){
      const influence=1-prior.masks[i];
      maskedStats.max_error=Math.max(maskedStats.max_error,...prior.positions[i].map((v,k)=>Math.abs(next.positions[i][k]-v-(k===0?.025*influence:0))));
      if(influence===0){maskedStats.fully_protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Fully protected vertex moved');}
      else if(influence<1)maskedStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))maskedStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-numeric-translation',statistics:maskedStats,history:maskedHistory,gates:{visible_option_enabled:true,partial_and_protected:maskedStats.partial>0&&maskedStats.fully_protected>0,weighted_response:maskedStats.max_error<=2e-6,changed_vertices:maskedStats.changed>0,history_exact:maskedHistory.undo_exact&&maskedHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const maskedAffineSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...maskedAffineSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Masked affine fixture did not select both parts');
  await action('transform.respect_masks');
  const maskedAffineBefore=await snapshot('masked-affine-before');
  const maskedAffineParams=await configureTransform({translation:[.02,-.015,.01],rotation_degrees:[7,-3,5],scale:1.05,pivot_kind:'origin'});
  await action('transform.apply');
  const maskedAffineAfter=await snapshot('masked-affine-after'),maskedAffineHistory=await history('masked-affine',maskedAffineBefore,maskedAffineAfter);
  const maskedAffineStats={max_error:0,fully_protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(maskedAffineBefore.document,id).state.mesh,next=byId(maskedAffineAfter.document,id).state.mesh;
    assert(JSON.stringify(prior.masks)===JSON.stringify(next.masks)&&JSON.stringify(prior.triangles)===JSON.stringify(next.triangles),'Masked affine transform changed masks or faces');
    for(let i=0;i<prior.positions.length;i++){
      const influence=1-prior.masks[i],full=expectedPoint(prior.positions[i],maskedAffineParams),expected=prior.positions[i].map((v,k)=>v+(full[k]-v)*influence);
      maskedAffineStats.max_error=Math.max(maskedAffineStats.max_error,...expected.map((v,k)=>Math.abs(next.positions[i][k]-v)));
      if(influence===0){maskedAffineStats.fully_protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Masked affine transform moved protected vertex');}
      else if(influence<1)maskedAffineStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))maskedAffineStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-numeric-rotation-scale',parameters:maskedAffineParams,statistics:maskedAffineStats,history:maskedAffineHistory,gates:{visible_option_enabled:(await state()).transform_settings.respect_masks,partial_and_protected:maskedAffineStats.partial>0&&maskedAffineStats.fully_protected>0,weighted_response:maskedAffineStats.max_error<=2e-6,changed_vertices:maskedAffineStats.changed>0,history_exact:maskedAffineHistory.undo_exact&&maskedAffineHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const axisSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...axisSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Axis-scale fixture did not select both parts');
  await action('transform.respect_masks');
  const axisBefore=await snapshot('masked-axis-scale-before'),axisParameters=await configureTransform({axis_scale:[1.2,.8,1.05],pivot_kind:'origin'});
  await action('transform.apply');
  const axisAfter=await snapshot('masked-axis-scale-after'),axisHistory=await history('masked-axis-scale',axisBefore,axisAfter);
  const axisStats={max_error:0,fully_protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(axisBefore.document,id).state.mesh,next=byId(axisAfter.document,id).state.mesh;
    assert(JSON.stringify(prior.masks)===JSON.stringify(next.masks)&&JSON.stringify(prior.triangles)===JSON.stringify(next.triangles),'Axis scale changed masks or faces');
    for(let i=0;i<prior.positions.length;i++){
      const weight=1-prior.masks[i];
      for(let k=0;k<3;k++)axisStats.max_error=Math.max(axisStats.max_error,Math.abs(next.positions[i][k]-prior.positions[i][k]*(1+(axisParameters.axis_scale[k]-1)*weight)));
      if(weight===0){axisStats.fully_protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Axis scale moved protected vertex');}
      else if(weight<1)axisStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))axisStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-numeric-axis-scale',parameters:axisParameters,statistics:axisStats,history:axisHistory,gates:{visible_option_enabled:(await state()).transform_settings.respect_masks,partial_and_protected:axisStats.partial>0&&axisStats.fully_protected>0,weighted_response:axisStats.max_error<=2e-6,changed_vertices:axisStats.changed>0,history_exact:axisHistory.undo_exact&&axisHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const maskedRingSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...maskedRingSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Uniform-ring fixture did not select both parts');
  await action('transform.respect_masks');
  const maskedRingBefore=await snapshot('masked-ring-before'),maskedRingGpu=await gpuDigest(),maskedRingModel=await state();
  const maskedRingStart=await gizmoStart('gizmo.scale_uniform');
  const maskedRingVector=maskedRingStart.start.map((v,i)=>v-maskedRingStart.data.screen_start[i]),maskedRingRadius=Math.hypot(...maskedRingVector);
  const maskedRingEnd=maskedRingStart.start.map((v,i)=>Math.round(v+maskedRingVector[i]/maskedRingRadius*24));
  const maskedRingPreview=await previewMove(maskedRingStart.start,maskedRingEnd,8),maskedRingFactor=maskedRingPreview.settings.preview_scale[0];
  assert(previewBuffersEqual(maskedRingGpu,maskedRingPreview.gpu)&&previewModelUnchanged(maskedRingModel,maskedRingPreview.metrics),'Selected masked ring changed held buffers or document');
  assert(JSON.stringify(maskedRingPreview.settings.preview_subtool_ids)===JSON.stringify([2,1]),'Selected ring omitted a part');
  await pointer('mouseReleased',...maskedRingEnd);await pause(180);
  const maskedRingAfter=await snapshot('masked-ring-after'),maskedRingAfterMetrics=await state(),maskedRingHistory=await history('masked-ring',maskedRingBefore,maskedRingAfter);
  const maskedRingStats={max_error:0,protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(maskedRingBefore.document,id).state.mesh,next=byId(maskedRingAfter.document,id).state.mesh;
    assert(JSON.stringify(prior.masks)===JSON.stringify(next.masks)&&JSON.stringify(prior.triangles)===JSON.stringify(next.triangles),'Uniform ring changed masks or topology');
    for(let i=0;i<prior.positions.length;i++){
      const weight=1-prior.masks[i];
      for(let k=0;k<3;k++){const p=prior.positions[i][k],full=maskedRingStart.data.pivot[k]+maskedRingFactor*(p-maskedRingStart.data.pivot[k]);maskedRingStats.max_error=Math.max(maskedRingStats.max_error,Math.abs(next.positions[i][k]-(p+weight*(full-p))));}
      if(weight===0){maskedRingStats.protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Uniform ring moved protected vertex');}
      else if(weight<1)maskedRingStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))maskedRingStats.changed++;
    }
  }
  await finishCase({name:'selected-group-masked-uniform-ring',factor:maskedRingFactor,pivot:maskedRingStart.data.pivot,statistics:maskedRingStats,history:maskedRingHistory,gates:{group_preview:JSON.stringify(maskedRingPreview.settings.preview_subtool_ids)===JSON.stringify([2,1]),preview_buffers:previewBuffersEqual(maskedRingGpu,maskedRingPreview.gpu),preview_document:previewModelUnchanged(maskedRingModel,maskedRingPreview.metrics),one_commit:maskedRingAfterMetrics.document_revision===maskedRingModel.document_revision+1,masked_positions:maskedRingStats.max_error<=2e-6,protected_partial_changed:maskedRingStats.protected>0&&maskedRingStats.partial>0&&maskedRingStats.changed>0,undo_redo:maskedRingHistory.undo_exact&&maskedRingHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const maskedGizmoSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...maskedGizmoSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Masked gizmo fixture did not select both parts');
  await action('transform.respect_masks');
  const maskedGizmoBefore=await snapshot('masked-gizmo-before'),maskedGizmoGpuBefore=await gpuDigest(),maskedGizmoModelBefore=await state();
  const maskedGizmoStart=await gizmoStart('gizmo.x'),maskedGizmoVector=maskedGizmoStart.data.screen_end.map((v,i)=>v-maskedGizmoStart.data.screen_start[i]),maskedGizmoLength=Math.hypot(...maskedGizmoVector),maskedGizmoEnd=maskedGizmoStart.start.map((v,i)=>Math.round(v+maskedGizmoVector[i]/maskedGizmoLength*4));
  for(let i=1;i<=4;i++){const t=i/4;await pointer('mouseMoved',Math.round(maskedGizmoStart.start[0]+(maskedGizmoEnd[0]-maskedGizmoStart.start[0])*t),Math.round(maskedGizmoStart.start[1]+(maskedGizmoEnd[1]-maskedGizmoStart.start[1])*t),'mouse',.5,true);await pause(25);}
  await pause(120);
  const maskedGizmoPreview={settings:(await state()).transform_settings,metrics:await state(),gpu:await gpuDigest()},maskedGizmoDelta=maskedGizmoPreview.settings.preview_translation;
  assert(Math.abs(maskedGizmoDelta[0])>0&&maskedGizmoDelta.slice(1).every(v=>v===0),'Masked gizmo preview has no isolated world-X movement');
  const maskedGizmoRigidPreview=maskedGizmoPreview.gpu.draws.length===2&&maskedGizmoPreview.gpu.draws.every(d=>d.preview_translation.every((v,k)=>Math.fround(v)===Math.fround(maskedGizmoDelta[k])));
  assert(maskedGizmoRigidPreview&&previewBuffersEqual(maskedGizmoGpuBefore,maskedGizmoPreview.gpu)&&previewModelUnchanged(maskedGizmoModelBefore,maskedGizmoPreview.metrics),'Masked gizmo preview edited mesh or failed to move both selected parts');
  const maskedGizmoRelease=await releaseProbes('masked-gizmo-release',maskedGizmoEnd),maskedGizmoAfter=await snapshot('masked-gizmo-after'),maskedGizmoHistory=await history('masked-gizmo',maskedGizmoBefore,maskedGizmoAfter);
  const maskedGizmoStats={max_error:0,fully_protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(maskedGizmoBefore.document,id).state.mesh,next=byId(maskedGizmoAfter.document,id).state.mesh;
    for(let i=0;i<prior.positions.length;i++){
      const influence=1-prior.masks[i];
      maskedGizmoStats.max_error=Math.max(maskedGizmoStats.max_error,...prior.positions[i].map((v,k)=>Math.abs(next.positions[i][k]-v-maskedGizmoDelta[k]*influence)));
      if(influence===0){maskedGizmoStats.fully_protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Masked gizmo moved protected vertex');}
      else if(influence<1)maskedGizmoStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))maskedGizmoStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-gizmo-preview-commit',preview_translation:maskedGizmoDelta,statistics:maskedGizmoStats,history:maskedGizmoHistory,gates:{rigid_preview:maskedGizmoRigidPreview,preview_buffers_unchanged:previewBuffersEqual(maskedGizmoGpuBefore,maskedGizmoPreview.gpu),preview_model_unchanged:previewModelUnchanged(maskedGizmoModelBefore,maskedGizmoPreview.metrics),weighted_commit:maskedGizmoStats.max_error<=2e-6,protected_and_partial:maskedGizmoStats.fully_protected>0&&maskedGizmoStats.partial>0,changed_vertices:maskedGizmoStats.changed>0,quiet_release:maskedGizmoRelease.quiet,history_exact:maskedGizmoHistory.undo_exact&&maskedGizmoHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const planeGroupSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...planeGroupSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Plane group fixture did not select both parts');
  await action('transform.respect_masks');
  const planeGroupBefore=await snapshot('plane-group-before'),planeGroupGpuBefore=await gpuDigest(),planeGroupModelBefore=await state();
  const planeGroupStart=await gizmoStart('gizmo.plane_z'),planeGroupEnd=[planeGroupStart.start[0]+4,planeGroupStart.start[1]-3];
  const planeGroupPreview=await previewMove(planeGroupStart.start,planeGroupEnd,8),planeGroupDelta=planeGroupPreview.settings.preview_translation;
  assert(planeGroupDelta[2]===0&&Math.hypot(planeGroupDelta[0],planeGroupDelta[1])>.002,'Masked group plane preview left XY');
  assert(JSON.stringify(planeGroupPreview.settings.preview_subtool_ids)===JSON.stringify([2,1])&&previewBuffersEqual(planeGroupGpuBefore,planeGroupPreview.gpu)&&previewModelUnchanged(planeGroupModelBefore,planeGroupPreview.metrics),'Masked group plane preview changed document/buffers or omitted a part');
  const planeGroupRelease=await releaseProbes('plane-group-release',planeGroupEnd),planeGroupAfter=await snapshot('plane-group-after'),planeGroupHistory=await history('plane-group',planeGroupBefore,planeGroupAfter);
  const planeGroupStats={max_error:0,protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(planeGroupBefore.document,id).state.mesh,next=byId(planeGroupAfter.document,id).state.mesh;
    assert(JSON.stringify(prior.triangles)===JSON.stringify(next.triangles)&&JSON.stringify(prior.masks)===JSON.stringify(next.masks),'Masked group plane changed topology or masks');
    for(let i=0;i<prior.positions.length;i++){
      const weight=1-prior.masks[i];
      for(let axis=0;axis<3;axis++)planeGroupStats.max_error=Math.max(planeGroupStats.max_error,Math.abs(next.positions[i][axis]-prior.positions[i][axis]-planeGroupDelta[axis]*weight));
      if(weight===0){planeGroupStats.protected++;assert(JSON.stringify(prior.positions[i])===JSON.stringify(next.positions[i]),'Masked plane moved protected vertex');}
      else if(weight<1)planeGroupStats.partial++;
      if(JSON.stringify(prior.positions[i])!==JSON.stringify(next.positions[i]))planeGroupStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-plane-translation',translation:planeGroupDelta,statistics:planeGroupStats,history:planeGroupHistory,gates:{plane_only:planeGroupDelta[2]===0,preview_group:JSON.stringify(planeGroupPreview.settings.preview_subtool_ids)===JSON.stringify([2,1]),preview_buffers:previewBuffersEqual(planeGroupGpuBefore,planeGroupPreview.gpu),preview_document:previewModelUnchanged(planeGroupModelBefore,planeGroupPreview.metrics),one_commit:planeGroupRelease.probes[0].metrics.document_revision===planeGroupModelBefore.document_revision+1,quiet_release:planeGroupRelease.quiet,weighted_geometry:planeGroupStats.max_error<=2e-6,protected_partial_changed:planeGroupStats.protected>0&&planeGroupStats.partial>0&&planeGroupStats.changed>0,undo_redo:planeGroupHistory.undo_exact&&planeGroupHistory.redo_exact}});

  await fresh();await importFile(seed.path,266);
  const rotateGroupSelection=await control('scene.select.1');
  for(const [type,down] of [['mouseMoved',false],['mousePressed',true],['mouseReleased',false]])await pointer(type,...rotateGroupSelection.point,'mouse',.5,down,2);
  await waitFor(s=>JSON.stringify(s.display_flags?.selected_ids)===JSON.stringify([2,1]),'Rotation group fixture did not select both parts');
  await action('transform.respect_masks');
  const rotateGroupBefore=await snapshot('rotate-group-before'),rotateGroupGpuBefore=await gpuDigest(),rotateGroupModelBefore=await state();
  const rotateGroupStart=await gizmoStart('gizmo.rotate_z'),rotateGroupUnit=rotateGroupStart.data.screen_tangent.map(v=>v/Math.hypot(...rotateGroupStart.data.screen_tangent));
  const rotateGroupEnd=rotateGroupStart.start.map((v,i)=>Math.round(v+rotateGroupUnit[i]*4));
  const rotateGroupPreview=await previewMove(rotateGroupStart.start,rotateGroupEnd,4),rotateGroupDegrees=rotateGroupPreview.settings.preview_rotation_degrees;
  assert(rotateGroupDegrees[0]===0&&rotateGroupDegrees[1]===0&&Math.abs(rotateGroupDegrees[2])>.2,'Masked group rotation preview did not isolate Z');
  assert(JSON.stringify(rotateGroupPreview.settings.preview_subtool_ids)===JSON.stringify([2,1])&&previewBuffersEqual(rotateGroupGpuBefore,rotateGroupPreview.gpu)&&previewModelUnchanged(rotateGroupModelBefore,rotateGroupPreview.metrics),'Masked group rotation preview changed held geometry or omitted a part');
  const rotateGroupRelease=await releaseProbes('rotate-group-release',rotateGroupEnd),rotateGroupAfter=await snapshot('rotate-group-after'),rotateGroupHistory=await history('rotate-group',rotateGroupBefore,rotateGroupAfter);
  const rotateGroupStats={max_error:0,protected:0,partial:0,changed:0};
  for(const id of [1,2]){
    const prior=byId(rotateGroupBefore.document,id).state.mesh,next=byId(rotateGroupAfter.document,id).state.mesh;
    assert(JSON.stringify(prior.triangles)===JSON.stringify(next.triangles)&&JSON.stringify(prior.masks)===JSON.stringify(next.masks),'Masked group rotation changed topology or masks');
    for(let i=0;i<prior.positions.length;i++){
      const weight=1-prior.masks[i],pivot=rotateGroupStart.data.pivot,p=prior.positions[i],rotated=scalarRotate(p.map((v,k)=>v-pivot[k]),rotateGroupDegrees).map((v,k)=>v+pivot[k]);
      for(let axis=0;axis<3;axis++)rotateGroupStats.max_error=Math.max(rotateGroupStats.max_error,Math.abs(next.positions[i][axis]-p[axis]-weight*(rotated[axis]-p[axis])));
      if(weight===0){rotateGroupStats.protected++;assert(JSON.stringify(p)===JSON.stringify(next.positions[i]),'Masked rotation moved protected vertex');}
      else if(weight<1)rotateGroupStats.partial++;
      if(JSON.stringify(p)!==JSON.stringify(next.positions[i]))rotateGroupStats.changed++;
    }
  }
  await finishCase({name:'selected-group-mask-weighted-rotation-arc',rotation_degrees:rotateGroupDegrees,pivot:rotateGroupStart.data.pivot,statistics:rotateGroupStats,history:rotateGroupHistory,gates:{axis_only:rotateGroupDegrees.slice(0,2).every(v=>v===0),preview_group:JSON.stringify(rotateGroupPreview.settings.preview_subtool_ids)===JSON.stringify([2,1]),preview_buffers:previewBuffersEqual(rotateGroupGpuBefore,rotateGroupPreview.gpu),preview_document:previewModelUnchanged(rotateGroupModelBefore,rotateGroupPreview.metrics),one_commit:rotateGroupRelease.probes[0].metrics.document_revision===rotateGroupModelBefore.document_revision+1,quiet_release:rotateGroupRelease.quiet,weighted_geometry:rotateGroupStats.max_error<=2e-6,protected_partial_changed:rotateGroupStats.protected>0&&rotateGroupStats.partial>0&&rotateGroupStats.changed>0,undo_redo:rotateGroupHistory.undo_exact&&rotateGroupHistory.redo_exact}});

  const legacyTrials=[];for(const [version,file] of [[2,path.resolve('.artifacts/move-regression/baseline-2e0bdefc-5c0f-4e09-aedb-c97b8a21e24f/sphere-outbound-initial.obrush')],[3,path.resolve('.artifacts/layer-browser-0.1.8/verified-6480c153-0de7-43b2-8b27-dd21b0414dd2/persisted-two-layers.obrush')],[4,path.resolve('.artifacts/morph-browser-0.1.9/scroll-final-b1a9c143-bbe7-4c30-a24a-bf6760eb186f/fresh-loaded-v4.obrush')]]){const source=JSON.parse(await readFile(file,'utf8'));await fresh();await importFile(file,source.mesh.positions.length);const before=await snapshot('legacy-'+version+'-before');const parameters=await configureTransform({translation:[.125,0,0]});await action('transform.apply');const after=await snapshot('legacy-'+version+'-after'),measure=affineContract(before.document,after.document,parameters);legacyTrials.push({version,fixture:{path:file,sha256:hash(await readFile(file))},measure,encoded_version:after.document.version,history:await history('legacy-'+version,before,after)});}
  await control('object.add_level',true);result.final_controls=await screenshot('transform-final-controls');await finishCase({name:'legacy-v2-v3-v4-transforms-without-schema-promotion',trials:legacyTrials,gates:{versions_retained:legacyTrials.every(t=>t.encoded_version===t.version),all_histories_exact:legacyTrials.every(t=>t.history.undo_exact&&t.history.redo_exact),all_scalar_gates:legacyTrials.every(t=>t.measure.point_max<=2e-6&&t.measure.layer_max<=2e-13)}});
  await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(p=>p.gl_errors),js:result.page_diagnostics.flatMap(p=>p.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(p=>p.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Transform runtime errors or missing MSAA');result.status='passed twenty-four transform browser workflows';result.inspector_horizontal_bounds={checked_states:result.ui_bounds_checks.length,visible_control_checks:result.ui_bounds_checks.reduce((n,c)=>n+c.visible_controls.length,0),failures:result.ui_bounds_checks.flatMap(c=>c.failures)};result.limitations=['Independent OBrush transform foundation; no reference Gizmo/Transpose or family parity','Selected-group translation, optional mask-weighted translation/rotation/uniform/axis scale, but retained hierarchy mask transport and full source handles remain gaps','Owned synthetic headless mouse events, packed VBO/IBO and actual preview uniforms; native/physical devices and input-to-photon remain unverified','Short gestures and quiet releases; large/long-session performance excluded','Public file input/download bytes; native picker/download filesystem persistence untested'];
  }

} catch(error){result.status='failed transform browser workflow';result.failure=String(error);process.exitCode=1;try{result.failure_runtime=await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})');result.failure_screenshot=await screenshot('failure');}catch{}}
finally{await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('Owned headless session closing'));}pending.clear();await pause(400);if(child.exitCode===null)child.kill('SIGTERM');await pause(200);await new Promise(resolve=>server.close(resolve));result.cleanup={owned_chrome_exit_code:child.exitCode,owned_chrome_signal:child.signalCode,owned_server_closed:true};await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),wasm:result.wasm,cases:result.cases.map(c=>({name:c.name,gates:c.gates}))}));}
