// CLI-only SubTool scene geometry and interaction checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-subtools.mjs [frozen web dir] [run-label]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'subtools';
const out = path.resolve(`.artifacts/subtool-browser/${label}-${randomUUID()}`);
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
  preregistered_contract:{draw_radius:.18,draw_strength:.35,symmetry_x:true,draw_min_original_vertex_displacement:.002,visible_scene_control_horizontal_clip_tolerance:.01,recorded_layer_min_displacement:.001,rich_copy_base_draw_min_displacement:.001,gpu:'Exact packed f32 visible vertex-buffer and solid u32 index-buffer multiset; actual u_active flag and active-last draw order',inactive_uploads:'Stable observed inactive VBO/IBO identities, exact digests and upload counts; renderer per-ID uploads unchanged during active Draw',history:'Exact canonical captured full-scene Save bytes',layer_composition:'Ordered visible f64 offsets with stored f32 intensities; skip zero contributions and compare final f32 bits',target_correspondence:'Stored target vertex bits and ordered faces bound to creation level; active hierarchy mesh is top-level with null retained active slot',release_probe_incremental_delays_ms:[0,200,800],release_probe_timestamp:'Probe start before asynchronous GPU readback/metrics; not finalization completion or input-to-photon latency',long_session_performance:'Excluded; short named gestures only'},
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
  const matrix=gl.uniformMatrix4fv;gl.uniformMatrix4fv=function(loc,transpose,value){const name=this.__sceneLocations.get(loc);if(name==='u_view_projection')p.view_projection=Array.from(value);if(name==='u_view')p.view=Array.from(value);return matrix.call(this,loc,transpose,value);};
  const clear=gl.clear;gl.clear=function(mask){if(mask&this.DEPTH_BUFFER_BIT)p.scene_frame={at:performance.now(),draws:[]};return clear.call(this,mask);};
  const draw=gl.drawElements;gl.drawElements=function(mode,count,type,offset){const vao=this.__sceneVaos.get(this.__sceneVao);if(vao&&mode===this.TRIANGLES&&!this.__sceneWire){const vertices=info(vao.vertices),element=this.__sceneElements.get(this.__sceneVao),indices=info(element);const observed={vao_key:vao.key,vertex_key:vertices.key,index_key:indices.key,count,type,offset,active:!!this.__sceneActive,at:performance.now(),vao:this.__sceneVao,vertices:vao.vertices,indices:element};if(p.scene_frame)p.scene_frame.draws.push(observed);if(observed.active)window.__brushMeshVbo={bytes:vertices.bytes};}return draw.call(this,mode,count,type,offset);};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;if(x>0&&width>400&&height>300)p.last_sculpt_viewport=[x,y,width,height];return viewport.call(this,x,y,width,height);};
  const storage=gl.renderbufferStorageMultisample;gl.renderbufferStorageMultisample=function(target,samples,format,width,height){const value=storage.call(this,target,samples,format,width,height);p.allocations.push({samples,actual_samples:this.getRenderbufferParameter(target,this.RENDERBUFFER_SAMPLES),width,height});return value;};
  const error=gl.getError;gl.getError=function(){const value=error.call(this);if(value!==this.NO_ERROR)p.gl_errors.push(value);return value;};
  window.__readSceneGpu=async()=>{const frame=p.scene_frame;if(!frame)throw Error('No actual scene draw frame');const previousArray=gl.__sceneArrayBuffer,previousVao=gl.__sceneVao;const draws=[];const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(b=>b.toString(16).padStart(2,'0')).join('');for(const draw of frame.draws){const vertexInfo=gl.__sceneBuffers.get(draw.vertices),indexInfo=gl.__sceneBuffers.get(draw.indices),vertices=new Uint8Array(vertexInfo.bytes),indices=new Uint8Array(indexInfo.bytes);gl.bindBuffer(gl.ARRAY_BUFFER,draw.vertices);gl.getBufferSubData(gl.ARRAY_BUFFER,0,vertices);gl.bindVertexArray(draw.vao);const oldElement=gl.__sceneElements.get(draw.vao);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,draw.indices);gl.getBufferSubData(gl.ELEMENT_ARRAY_BUFFER,0,indices);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,oldElement);draws.push({vao_key:draw.vao_key,vertex_key:draw.vertex_key,index_key:draw.index_key,active:draw.active,count:draw.count,type:draw.type,offset:draw.offset,vertex_bytes:vertices.length,index_bytes:indices.length,vertex_sha256:await digest(vertices),index_sha256:await digest(indices),vertex_uploads:vertexInfo.uploads,index_uploads:indexInfo.uploads,last_upload_at:vertexInfo.last_upload_at});}gl.bindVertexArray(previousVao);gl.bindBuffer(gl.ARRAY_BUFFER,previousArray);return {frame_at:frame.at,draws,view_projection:p.view_projection,view:p.view,viewport:p.last_sculpt_viewport};};
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
};
const control=async(key,allowDisabled=false)=>{
  for(let i=0;i<12;i++){
    const s=await state(),rect=s.scene_controls?.[key]||s.morph_controls?.[key]||s.layer_controls?.[key];
    checkSceneBounds(s,'control:'+key);
    if(!rect)throw Error('Missing read-only rendered control rectangle: '+key);
    const point=[(rect[0]+rect[2])/2,(rect[1]+rect[3])/2];
    if(point[1]>66&&point[1]<955){
      if(!allowDisabled&&(s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key])===false)throw Error('Unexpected disabled control: '+key);
      return {point,rect,enabled:s.scene_controls_enabled?.[key]??s.morph_controls_enabled?.[key]??s.layer_controls_enabled?.[key]};
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
      await pause(50);const after=await state(),next=after.scene_controls?.[key]||after.morph_controls?.[key]||after.layer_controls?.[key];
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
const finishCase=async(record)=>{result.cases.push(record);await writeFile(path.join(out,record.name+'.json'),JSON.stringify(record,null,2)+'\n');await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:record.name,gates:record.gates}));assert(Object.values(record.gates).every(Boolean),'SubTool case failed: '+record.name);};
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
  await pointer('mouseMoved',...from);await pause(60);await pointer('mousePressed',...from,'mouse',.5,true);await pause(70);for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,'mouse',.5,true);await pause(35);}await pointer('mouseReleased',...to);
  const probes=[];for(const delay of [0,200,800]){await pause(delay);probes.push({at:await evaluate('performance.now()'),gpu:await gpuDigest(),sculpt:await evaluate('window.obrushSculptMetrics()'),metrics:await state()});}
  const quiet=probes.every(p=>p.gpu.sha256===probes[0].gpu.sha256&&p.sculpt.pending_dabs===0);assert(quiet,'Scene geometry or queued input changed after release');const end=await evaluate('performance.now()');const runtime=await evaluate(`({events:window.__brushCheck.events.filter(x=>x.at>=${start}&&x.at<=${end}),frames:window.__brushCheck.raf.filter(x=>x.at>=${start}&&x.at<=${end}),longtasks:window.__brushCheck.longtasks.filter(x=>x.at>=${start}&&x.at<=${end})})`);const trial={name,brush:number,settings,contact,from,to,steps,start,end,quiet,probes,runtime};result.strokes.push(trial);return trial;
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

  await fresh();result.initial_controls=await screenshot('initial-controls');assert((await state()).subtools?.length===1,'Missing scene diagnostics');
  const initial=await snapshot('initial-single-sphere');await action('scene.add_cube');await sceneCount(2);const cube=await snapshot('added-cube');const cubeHistory=await history('add-cube',initial,cube);
  await action('scene.add_sphere');await sceneCount(3);const sphere=await snapshot('added-sphere');const sphereHistory=await history('add-sphere',cube,sphere);
  const appendedFixture=await objFixture('independent-left-sphere',meshFixture([-3,0,0]));await append(appendedFixture);await sceneCount(4);const appended=await snapshot('appended-obj');const appendHistory=await history('append-obj',sphere,appended),appendId=appended.document.scene.active_id;
  await renameScene('Independent OBJ');const renamed=await snapshot('renamed-obj');const renameHistory=await history('rename-subtool',appended,renamed);
  await action('scene.up');const up=await snapshot('reordered-up');const upHistory=await history('reorder-up',renamed,up);await action('scene.down');const down=await snapshot('reordered-down');
  await action('scene.duplicate');await sceneCount(5);const duplicate=await snapshot('duplicated-obj'),duplicateId=duplicate.document.scene.active_id;const duplicateHistory=await history('duplicate-subtool',down,duplicate);
  await action('scene.delete');await sceneCount(4);const deleted=await snapshot('deleted-duplicate');const deleteHistory=await history('delete-subtool',duplicate,deleted);
  await finishCase({name:'add-append-rename-reorder-duplicate-delete',fixture:{path:appendedFixture.path,sha256:appendedFixture.sha256,bytes:appendedFixture.bytes},histories:{cubeHistory,sphereHistory,appendHistory,renameHistory,upHistory,duplicateHistory,deleteHistory},gates:{scene_is_v5:appended.document.version===5,append_preserves_exact_coordinates:positionsEqual(byId(appended.document,appendId).state.mesh.positions,appendedFixture.mesh.positions),rename_exact:renamed.document.scene.objects.find(o=>o.id===appendId).name==='Independent OBJ',reorder_roundtrip_exact:projectHash(down)===projectHash(renamed),duplicate_new_stable_id:duplicateId!==appendId&&duplicate.document.scene.next_id>duplicateId,duplicate_exact_state:stateHash(byId(duplicate.document,duplicateId).state)===stateHash(byId(duplicate.document,appendId).state),delete_retains_source:!!byId(deleted.document,appendId)&&!byId(deleted.document,duplicateId)}});

  await selected(appendId);const beforeVisibility=await snapshot('before-visibility');await action('scene.visibility.'+appendId);const hidden=await snapshot('active-hidden');const hiddenStroke=await sculptScene('hidden-active-noop',1,byId(hidden.document,appendId).state.mesh);const hiddenAfter=await snapshot('hidden-after-draw');
  await action('scene.visibility.'+appendId);const shown=await snapshot('active-shown');await action('scene.solo');const solo=await snapshot('solo-active');const soloHistory=await history('solo-scene',shown,solo);await action('scene.solo');const allShown=await snapshot('solo-restored');
  await finishCase({name:'visibility-solo-hidden-noop',history:soloHistory,gates:{hidden_has_no_active_draw:hiddenAfter.document.scene.objects.find(o=>o.id===appendId).visible===false,hidden_noop_exact_project:projectHash(hidden)===projectHash(hiddenAfter),show_restores_exact_scene:projectHash(shown)===projectHash(beforeVisibility),solo_one_rendered_object:result.snapshots.find(s=>s.path===solo.path).gpu.draws.length===1,solo_keeps_visibility_flags:JSON.stringify(solo.document.scene.objects.map(o=>o.visible))===JSON.stringify(shown.document.scene.objects.map(o=>o.visible)),solo_off_exact:projectHash(allShown)===projectHash(shown),release_quiet:hiddenStroke.quiet}});

  const left=meshFixture([-1.5,0,0],1,[.65,.2,.2],'pattern'),right=meshFixture([1.5,0,0],1,[.2,.6,.8]);const separatedFixture=await sceneFixture('Independent separated scene',[{id:1,name:'Left inactive',state:{mesh:left}},{id:2,name:'Right active',state:{mesh:right}}],2);
  await fresh();await importFile(separatedFixture.path,right.positions.length);const beforeDraw=await snapshot('separated-before-draw'),inactiveBefore=stateHash(byId(beforeDraw.document,1).state),uploadsBefore=(await state()).subtool_uploads.find(o=>o.id===1);
  const drawStroke=await sculptScene('active-only-draw',1,beforeDraw.document.mesh,14);const afterDraw=await snapshot('separated-after-draw'),uploadsAfter=(await state()).subtool_uploads.find(o=>o.id===1);const drawDifference=originalDisplacement(beforeDraw.document.mesh,afterDraw.document.mesh);assert(drawDifference.max_displacement>=.002,'Active Draw has no useful geometry');
  const beforeGpu=result.snapshots.find(s=>s.path===beforeDraw.path).gpu,afterGpu=result.snapshots.find(s=>s.path===afterDraw.path).gpu;const inactiveGpuBefore=beforeGpu.draws.find(d=>!d.active),inactiveGpuAfter=afterGpu.draws.find(d=>!d.active);const drawHistory=await history('active-only-draw',beforeDraw,afterDraw);
  await finishCase({name:'active-draw-inactive-gpu-isolation',fixture:separatedFixture,difference:drawDifference,history:drawHistory,uploads:{before:uploadsBefore,after:uploadsAfter},gates:{useful_active_draw:drawDifference.max_displacement>=.002,active_id_retained:afterDraw.document.scene.active_id===2,inactive_state_exact:inactiveBefore===stateHash(byId(afterDraw.document,1).state),inactive_vertex_index_bytes_exact:inactiveGpuBefore.vertex_sha256===inactiveGpuAfter.vertex_sha256&&inactiveGpuBefore.index_sha256===inactiveGpuAfter.index_sha256,inactive_buffer_uploads_unchanged:inactiveGpuBefore.vertex_key===inactiveGpuAfter.vertex_key&&inactiveGpuBefore.index_key===inactiveGpuAfter.index_key&&inactiveGpuBefore.vertex_uploads===inactiveGpuAfter.vertex_uploads&&inactiveGpuBefore.index_uploads===inactiveGpuAfter.index_uploads,inactive_renderer_uploads_unchanged:uploadsBefore.uploads===uploadsAfter.uploads,release_quiet:drawStroke.quiet}});

  const beforeMask=await snapshot('before-mask');await sculptScene('active-mask',8,beforeMask.document.mesh,12);const masked=await snapshot('active-masked');assert(masked.document.mesh.masks.some((v,i)=>v!==beforeMask.document.mesh.masks[i]),'Actual Mask was empty');const maskHistory=await history('active-mask',beforeMask,masked);
  await action('mask.clear');const cleared=await snapshot('active-mask-cleared');assert(cleared.document.mesh.masks.every(v=>v===0),'Actual Clear mask failed');await sculptScene('active-paint',9,cleared.document.mesh,12);const painted=await snapshot('active-painted');assert(JSON.stringify(painted.document.mesh.colors)!==JSON.stringify(cleared.document.mesh.colors),'Actual Paint was empty');const paintHistory=await history('active-paint',cleared,painted);
  await store();const ownTarget=await snapshot('right-own-target');await newLayer(1);const ownEmpty=await snapshot('right-own-empty-layer');await sculptScene('right-layer-draw',1,ownEmpty.document.mesh,10);const ownLayer=await snapshot('right-own-recorded-layer');const ownLayerHistory=await history('right-recorded-layer',ownEmpty,ownLayer);
  await action('layers.bake');await layerCount(0);const ownBaked=await snapshot('right-own-baked');await action('object.add_level');await waitFor(s=>s.subdivision_levels===2&&s.subdivision_level===1,'Own Add level failed');const ownHigh=await snapshot('right-own-high-level');
  await selected(1);const selectedLeft=await snapshot('left-selected-independent');await selected(2);const selectedRight=await snapshot('right-reselected-independent');
  await finishCase({name:'separate-mask-color-layers-morph-levels',histories:{maskHistory,paintHistory,ownLayerHistory},gates:{inactive_all_state_exact:inactiveBefore===stateHash(byId(selectedRight.document,1).state),mask_changes_active_only:stateHash(byId(masked.document,1).state)===inactiveBefore,color_changes_active_only:stateHash(byId(painted.document,1).state)===inactiveBefore,own_layer_records_usefully:positionDiff(ownEmpty.document.mesh,ownLayer.document.mesh).max_displacement>=.001,own_target_bits_retained:targetHash(ownTarget.document)===targetHash(ownHigh.document),own_target_origin_unavailable_on_high:ownHigh.document.morph_target.creation_level===0&&result.snapshots.find(s=>s.path===ownHigh.path).metrics.morph_target.available===false,left_has_no_foreign_extras:!selectedLeft.document.layers&&!selectedLeft.document.morph_target&&!selectedLeft.document.multiresolution,right_selection_preserves_exact_state:stateHash(activeState(ownHigh.document))===stateHash(activeState(selectedRight.document)),right_channels_retained:channelsEqual(ownBaked.document.mesh,ownLayer.document.mesh)}});

  await fresh();await importFile(selectedRight.path,selectedRight.document.mesh.positions.length);const loaded=await snapshot('fresh-v5-loaded');await action('scene.visibility.1');const redoTarget=await snapshot('v5-redo-target');await chord('z');const beforeInvalid=await snapshot('v5-before-invalid');
  const malformed=structuredClone(beforeInvalid.document);malformed.scene.objects.find(o=>o.id!==malformed.scene.active_id).payload.mesh.triangles[0][0]=0xffffffff;const invalid=await publicFixture('invalid-inactive-index',malformed);const notice=await importFile(invalid.path,0,true),afterInvalid=await snapshot('v5-after-invalid');await chord('z',true);const afterRedo=await snapshot('v5-redo-after-invalid');
  await finishCase({name:'v5-fresh-open-invalid-inactive-redo',invalid_fixture:invalid,notice,gates:{fresh_canonical_bytes_exact:projectHash(loaded)===projectHash(selectedRight),invalid_preserves_scene:projectHash(afterInvalid)===projectHash(beforeInvalid),invalid_preserves_redo:projectHash(afterRedo)===projectHash(redoTarget)}});

  const legacyFiles=[['v2','.artifacts/move-regression/baseline-2e0bdefc-5c0f-4e09-aedb-c97b8a21e24f/sphere-outbound-initial.obrush'],['v3','.artifacts/layer-browser-0.1.8/verified-6480c153-0de7-43b2-8b27-dd21b0414dd2/persisted-two-layers.obrush'],['v4','.artifacts/morph-browser-0.1.9/scroll-final-b1a9c143-bbe7-4c30-a24a-bf6760eb186f/fine-layer-swap-twice.obrush']];const legacy=[];
  for(const [version,file]of legacyFiles){const bytes=await readFile(file),document=JSON.parse(bytes);await fresh();await importFile(path.resolve(file),document.mesh.positions.length);const saved=await snapshot('legacy-'+version);legacy.push({version,path:path.resolve(file),sha256:hash(bytes),exact_semantic_state:legacySemantic(saved.document,document),one_implicit_subtool:(await state()).subtools.length===1});}
  await finishCase({name:'legacy-v2-v3-v4-open',fixtures:legacy,gates:{all_legacy_semantic_states_exact:legacy.every(x=>x.exact_semantic_state),all_legacy_single_part:legacy.every(x=>x.one_implicit_subtool)}});

  const richBefore=await snapshot('rich-before-duplicate');await action('scene.duplicate');await sceneCount(2);const richDuplicate=await snapshot('rich-duplicated'),richCopyId=richDuplicate.document.scene.active_id,richSourceId=richDuplicate.document.scene.objects.find(o=>o.id!==richCopyId).id;const richDuplicateHistory=await history('rich-duplicate',richBefore,richDuplicate);
  await stop();const beforeRichDraw=await snapshot('rich-copy-before-base-draw');await sculptScene('rich-copy-base-draw',1,beforeRichDraw.document.mesh,10);const richCopyEdited=await snapshot('rich-copy-edited');assert(positionDiff(beforeRichDraw.document.mesh,richCopyEdited.document.mesh).max_displacement>=.001,'Rich duplicate Draw was empty');
  const sourceUnchanged=stateHash(byId(richCopyEdited.document,richSourceId).state)===stateHash(byId(richDuplicate.document,richSourceId).state);await action('scene.delete');const richDeleted=await snapshot('rich-copy-deleted');const richDeleteHistory=await history('rich-copy-delete',richCopyEdited,richDeleted);
  await finishCase({name:'rich-duplicate-independence-history',histories:{richDuplicateHistory,richDeleteHistory},gates:{duplicate_copies_complete_state:stateHash(byId(richDuplicate.document,richCopyId).state)===stateHash(byId(richDuplicate.document,richSourceId).state),source_preserved_after_copy_draw:sourceUnchanged,source_extras_retained:!!byId(richDeleted.document,richSourceId).state.layers&&!!byId(richDeleted.document,richSourceId).state.morph_target&&!!byId(richDeleted.document,richSourceId).state.multiresolution,delete_removes_only_copy:!byId(richDeleted.document,richCopyId)&&richDeleted.document.scene.objects.length===1}});

  const yaw=.35,pitch=.18,unit=[Math.sin(yaw)*Math.cos(pitch),Math.sin(pitch),Math.cos(yaw)*Math.cos(pitch)];const back=meshFixture([0,0,0],1,[.2,.2,.8],true),front=meshFixture(unit.map(v=>v*.6),.55,[.8,.2,.2],true);const overlapFixture=await sceneFixture('Independent overlap scene',[{id:11,name:'Masked back',state:{mesh:back}},{id:22,name:'Masked front',state:{mesh:front}}],11);
  await fresh();await importFile(overlapFixture.path,back.positions.length);const beforePick=await snapshot('overlap-back-active');const frontContact=await projectedContact(front);await key(1);await pointer('mouseMoved',...frontContact.screen);await pause(60);await pointer('mousePressed',...frontContact.screen,'mouse',.5,true);await pause(100);await pointer('mouseReleased',...frontContact.screen);await waitFor(s=>s.active_subtool_id===22,'Nearest visible masked surface did not become active');const afterPick=await snapshot('overlap-front-picked');result.final_controls=await screenshot('subtools-final-controls');
  await finishCase({name:'nearest-visible-overlap-picking',fixture:overlapFixture,contact:frontContact,gates:{nearest_front_selected:afterPick.document.scene.active_id===22,masked_geometry_states_exact:objects(beforePick.document).every(o=>stateHash(o.state)===stateHash(byId(afterPick.document,o.id).state)),active_solid_draw_last:result.snapshots.find(s=>s.path===afterPick.path).gpu.draws.at(-1).active}});
  await capturePageDiagnostics();result.runtime_errors={gl:result.page_diagnostics.flatMap(page=>page.gl_errors),js:result.page_diagnostics.flatMap(page=>page.js_errors)};result.actual_4x_msaa=result.page_diagnostics.every(page=>page.actual_4x_msaa);assert(!result.runtime_errors.gl.length&&!result.runtime_errors.js.length&&!result.exceptions.length&&!result.console_errors.length&&result.actual_4x_msaa,'Scene runtime errors or missing actual MSAA');
  result.inspector_horizontal_bounds={tolerance:.01,checked_states:result.ui_bounds_checks.length,visible_control_checks:result.ui_bounds_checks.reduce((sum,c)=>sum+c.visible_controls.length,0),failures:result.ui_bounds_checks.flatMap(c=>c.failures)};
  assert(result.inspector_horizontal_bounds.checked_states>0&&result.inspector_horizontal_bounds.failures.length===0,'Inspector horizontal bounds gate failed');
  result.status='passed eight SubTool browser workflows';result.limitations=['One independently authored scene foundation; no SubTool folders, transforms, Live Boolean or reference parity is claimed.','Owned headless Chrome with trusted synthetic input; native GUI, physical devices and input-to-photon latency remain unverified.','Visible packed vertex and solid index buffers are compared as a complete multiset with actual active shader flag/order. Renderer IDs are not exposed as shader uniforms; identical inactive copies are distinguished by public state and per-ID upload counters.','Release probes cover short named gestures only; sustained long-session queue performance is excluded.','Public file input and captured download bytes are exercised; native file-picker/download filesystem persistence is outside this harness.'];
} catch(error){result.status='failed SubTool browser workflow';result.failure=String(error);process.exitCode=1;try{result.failure_runtime=await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__brushCheck,notice:document.getElementById("notice")?.textContent})');result.failure_screenshot=await screenshot('failure');}catch{}}
finally{await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'chrome-stderr.log'),stderr);await writeFile(path.join(out,'chrome-stdout.log'),stdout);try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}try{socket?.close();}catch{}for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('Owned headless session closing'));}pending.clear();await pause(400);if(child.exitCode===null)child.kill('SIGTERM');await pause(200);await new Promise(resolve=>server.close(resolve));result.cleanup={owned_chrome_exit_code:child.exitCode,owned_chrome_signal:child.signalCode,owned_server_closed:true};await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({status:result.status,failure:result.failure,result:path.join(out,'result.json'),wasm:result.wasm,cases:result.cases.map(c=>({name:c.name,gates:c.gates}))}));}
