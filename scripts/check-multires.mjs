// Own loopback server, headless Chrome process/profile and CDP tab only.
// Does not attach to user tabs, accessibility, desktop apps or visible windows.
// Usage: node scripts/check-multires.mjs [web/dist] [label] [optional coordinate JSON] [optional timing fixture]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'multires-workflow';
const out = path.resolve(`.artifacts/multires-browser-0.1.6/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
await mkdir(profile, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const coordinates = process.argv[4] ? JSON.parse(await readFile(process.argv[4], 'utf8')) : {
  add: [1480, 292], lower: [1410, 327], higher: [1500, 327], cube: [1473, 257],
};
const fixture = path.resolve('.artifacts/user-long-stroke-0.1.4/user.obrush');
const openFile = path.join(out, 'independent-open-sheet.obj');
await writeFile(openFile, 'v -0.6 -0.6 0\nv 0.6 -0.6 0\nv 0.6 0.6 0\nv -0.6 0.6 0\nf 1 2 3\nf 1 3 4\n');
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) { response.writeHead(403); response.end(); return; }
    const bytes = await readFile(file);
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    response.end(bytes);
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const child = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0',
  '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--disable-sync',
  '--disable-default-apps', '--disable-breakpad', '--metrics-recording-only', '--safebrowsing-disable-auto-update',
  '--disable-background-timer-throttling', '--disable-renderer-backgrounding', '--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1', '--window-size=1600,1000', 'about:blank',
], { stdio: ['ignore', 'pipe', 'pipe'] });
let stdout = '', stderr = '', socket, sequence = 0, session;
child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
const pending = new Map();
const result = {
  label, status: 'running', method: 'Isolated CLI headless Chrome/CDP, actual canvas controls and public file inputs',
  web, out, url, owned_pid: child.pid, coordinates, harness_sha256: hash(await readFile(new URL(import.meta.url))),
  input_trace: [], cases: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  tolerances: { history: 'Exact serialized project and every mesh field', gpu: 'Exact packed f32 vertex-buffer hash', fine_detail: 'Maximum world-component residual difference2e-6; matching coarse branch required within2e-6', topology: 'Exact vertex count and face indices at every sculpted level' },
};
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 45000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
};
const instrument = `(()=>{
 const p=window.__topologyCheck={raf:[],longtasks:[],uploads:[],draws:[],matrices:[],viewports:{},contexts:[],allocations:[],errors:[],gl_errors:[],jobs:[],cancels:[]};
 new PerformanceObserver(list=>{for(const task of list.getEntries())p.longtasks.push({start:task.startTime,duration:task.duration});}).observe({entryTypes:['longtask']});
 const raf=window.requestAnimationFrame.bind(window);let last;raf(function tick(timestamp){const at=performance.now();p.raf.push({at,gap:last===undefined?0:timestamp-last,busy:typeof window.obrushBrushMetrics==='function'?window.obrushBrushMetrics().topology_busy:null});last=timestamp;raf(tick);});
 window.addEventListener('error',event=>p.errors.push(String(event.error||event.message)));window.addEventListener('unhandledrejection',event=>p.errors.push(String(event.reason)));
 const get=HTMLCanvasElement.prototype.getContext;
 HTMLCanvasElement.prototype.getContext=function(kind,...args){const gl=get.call(this,kind,...args);if(kind==='webgl2'&&gl&&!gl.__topologyObserved){
  gl.__topologyObserved=true;window.__topologyGl=gl;p.contexts.push({kind,version:gl.getParameter(gl.VERSION)});
  const bindVao=gl.bindVertexArray;gl.bindVertexArray=function(vao){this.__currentVao=vao;return bindVao.call(this,vao);};
  const bind=gl.bindBuffer;gl.bindBuffer=function(target,buffer){if(target===this.ARRAY_BUFFER)this.__currentArrayBuffer=buffer;return bind.call(this,target,buffer);};
  const attribute=gl.vertexAttribPointer;gl.vertexAttribPointer=function(index,size,type,normalized,stride,offset){if(stride===40&&offset===0&&size===3){this.__meshBuffer=this.__currentArrayBuffer;this.__meshVao=this.__currentVao;}return attribute.call(this,index,size,type,normalized,stride,offset);};
  const data=gl.bufferData;gl.bufferData=function(target,bytes,...args){const at=performance.now();const value=data.call(this,target,bytes,...args);if(target===this.ARRAY_BUFFER&&this.__currentArrayBuffer===this.__meshBuffer){const count=typeof bytes==='number'?bytes:bytes?.byteLength||0;window.__topologyVbo={buffer:this.__meshBuffer,bytes:count,at};p.uploads.push({at,end:performance.now(),bytes:count});}return value;};
  const draw=gl.drawElements;gl.drawElements=function(mode,count,...args){if(this.__currentVao===this.__meshVao)p.draws.push({at:performance.now(),mode,count});return draw.call(this,mode,count,...args);};
  const matrix=gl.uniformMatrix4fv;gl.uniformMatrix4fv=function(location,transpose,values,...args){if(this.__currentVao===this.__meshVao)p.matrices.push({at:performance.now(),values:Array.from(values).slice(0,16)});return matrix.call(this,location,transpose,values,...args);};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;return viewport.call(this,x,y,width,height);};
  const storage=gl.renderbufferStorageMultisample;gl.renderbufferStorageMultisample=function(target,samples,format,width,height){const value=storage.call(this,target,samples,format,width,height);p.allocations.push({samples,actual_samples:this.getRenderbufferParameter(target,this.RENDERBUFFER_SAMPLES),width,height});return value;};
  const error=gl.getError;gl.getError=function(){const value=error.call(this);if(value!==this.NO_ERROR)p.gl_errors.push(value);return value;};
 }return gl;};
})();`;
const pointer = async (type, x, y, button = 'left', down = false) => {
  const parameters = { type, x, y, pointerType: 'mouse', button: type === 'mouseMoved' ? 'none' : button, buttons: down ? (button === 'right' ? 2 : 1) : 0, ...(type === 'mouseMoved' ? {} : { clickCount: 1 }) };
  result.input_trace.push({ ...parameters, epoch_ms: Date.now() });
  await send('Input.dispatchMouseEvent', parameters);
};
const click = async point => { await pointer('mouseMoved', ...point); await pause(30); await pointer('mousePressed', ...point, 'left', true); await pause(25); await pointer('mouseReleased', ...point); await pause(50); };
const chord = async (key, shift = false) => {
  const modifiers = 4 | (shift ? 8 : 0), code = 'Key' + key.toUpperCase();
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
  await pause(180);
};
const save = async name => {
  const start = await evaluate('performance.now()');
  const index = await evaluate('window.__topologyCaptures.length');
  await chord('s');
  let size;
  for (let attempt = 0; attempt < 80; attempt++) { size = await evaluate(`window.__topologyCaptures[${index}]?.contents.length||0`); if (size) break; await pause(50); }
  if (!size || size > 128 * 1024 * 1024) throw new Error(`Save capture invalid: ${name}, ${size}`);
  let contents = '';
  for (let offset = 0; offset < size; offset += 131072) contents += await evaluate(`window.__topologyCaptures[${index}].contents.slice(${offset},${offset + 131072})`);
  const file = path.join(out, name + '.obrush'); await writeFile(file, contents);
  const emitted = await evaluate(`window.__topologyCaptures[${index}].at`);
  const phase = await phaseMetrics(start, emitted);
  return { path: file, sha256: hash(contents), bytes: Buffer.byteLength(contents), export_timing: { start, emitted, elapsed_ms: emitted-start, longtasks: phase.longtasks }, project: JSON.parse(contents) };
};
const metadata = ({ project, ...rest }) => ({ ...rest, vertices: project.mesh.positions.length, triangles: project.mesh.triangles.length });
const projectHash = saved => hash(JSON.stringify(saved.project));
const packedHash = mesh => {
  const bytes = Buffer.alloc(mesh.positions.length * 40);
  for (let vertex = 0; vertex < mesh.positions.length; vertex++) {
    const row = [...mesh.positions[vertex], ...mesh.normals[vertex], mesh.masks[vertex], ...mesh.colors[vertex]];
    for (let channel = 0; channel < 10; channel++) bytes.writeFloatLE(row[channel], vertex * 40 + channel * 4);
  }
  return hash(bytes);
};
const gpuDigest = () => evaluate(`(async()=>{const gl=window.__topologyGl,vbo=window.__topologyVbo;if(!vbo)throw Error('Missing mesh VBO');const previous=gl.__currentArrayBuffer;const bytes=new Uint8Array(vbo.bytes);gl.bindBuffer(gl.ARRAY_BUFFER,vbo.buffer);gl.getBufferSubData(gl.ARRAY_BUFFER,0,bytes);gl.bindBuffer(gl.ARRAY_BUFFER,previous);const digest=await crypto.subtle.digest('SHA-256',bytes);return {bytes:vbo.bytes,last_upload_at:vbo.at,sha256:Array.from(new Uint8Array(digest)).map(byte=>byte.toString(16).padStart(2,'0')).join('')};})()`);
const screenshot = async name => {
  const response = await send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  const bytes = Buffer.from(response.data, 'base64'), file = path.join(out, name + '.png'); await writeFile(file, bytes); return { path: file, sha256: hash(bytes) };
};
const fresh = async () => {
  await send('Page.navigate', { url });
  for (let attempt = 0; attempt < 150; attempt++) {
    if (await evaluate('!document.getElementById("loading")&&window.__topologyVbo&&typeof window.obrushBrushMetrics==="function"')) {
      await pause(300);
      await evaluate(`window.__topologyCaptures=[];window.obrushDownload=(name,contents)=>window.__topologyCaptures.push({name,contents,at:performance.now()});document.getElementById('obrush_canvas').focus();const start=window.obrushStartTopology;window.obrushStartTopology=(id,project,operation)=>{window.__topologyCheck.jobs.push({id,at:performance.now(),project_bytes:project.length,operation:JSON.parse(operation)});return start(id,project,operation);};const cancel=window.obrushCancelTopology;window.obrushCancelTopology=id=>{window.__topologyCheck.cancels.push({id,at:performance.now()});return cancel(id);};`);
      return;
    }
    await pause(100);
  }
  throw new Error('Topology browser startup timeout');
};
const importFile = async (file, expectedVertices) => {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#import-file' })).nodeId;
  await send('DOM.setFileInputFiles', { nodeId, files: [file] });
  for (let attempt = 0; attempt < 160; attempt++) {
    const state = await evaluate('({metrics:window.obrushBrushMetrics(),notice:document.getElementById("notice").textContent,bytes:window.__topologyVbo.bytes})');
    if (state.notice) throw new Error(state.notice);
    if (state.metrics.vertices === expectedVertices && state.bytes === expectedVertices * 40) { await pause(200); return; }
    await pause(50);
  }
  throw new Error(`File import timeout: ${file}`);
};
const waitForCompletion = async jobCount => {
  for (let attempt = 0; attempt < 900; attempt++) {
    const state = await evaluate('({metrics:window.obrushBrushMetrics(),jobs:window.__topologyCheck.jobs.length,at:performance.now()})');
    if (state.jobs > jobCount && !state.metrics.topology_busy) { await pause(250); return state; }
    await pause(50);
  }
  throw new Error('Background topology completion timeout');
};
const phaseMetrics = (start, end) => evaluate(`(()=>{const p=window.__topologyCheck;const within=(items,key='at')=>items.filter(item=>item[key]>=${start}&&item[key]<=${end});const frames=within(p.raf);return {start:${start},end:${end},elapsed_ms:${end-start},raf:frames,longtasks:within(p.longtasks,'start'),uploads:within(p.uploads),draws:within(p.draws),matrices:within(p.matrices),jobs:p.jobs,cancels:p.cancels,busy_raf_frames:frames.filter(frame=>frame.busy).length};})()`);
const begin = async point => {
  const jobs = await evaluate('window.__topologyCheck.jobs.length');
  const start = await evaluate('performance.now()');
  await click(point);
  for (let attempt = 0; attempt < 40; attempt++) {
    if (await evaluate('window.__topologyCheck.jobs.length') > jobs) return { jobs, start };
    await pause(50);
  }
  throw new Error(`The canvas control did not start a topology job at ${point}`);
};
let fullyMaskedFile;
const orbit = async () => {
  await pointer('mouseMoved', 770, 470, 'right'); await pointer('mousePressed', 770, 470, 'right', true);
  for (let move = 0; move < 12; move++) { await pointer('mouseMoved', 770 + move * 4, 470 + move * 2, 'right', true); await pause(20); }
  await pointer('mouseReleased', 814, 492, 'right');
};

const key = async number => {
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: String(number), code: 'Digit'+number, windowsVirtualKeyCode: 48+number });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: String(number), code: 'Digit'+number, windowsVirtualKeyCode: 48+number });
  await pause(100);
};
const state = () => evaluate('window.obrushBrushMetrics()');
const waitLevel = async (level, count) => {
  for(let attempt=0;attempt<150;attempt++){
    const metrics=await state();
    if(metrics.subdivision_level===level&&metrics.subdivision_levels===count&&await evaluate('window.__topologyVbo.bytes')===metrics.vertices*40){await pause(200);return metrics;}
    await pause(100);
  }
  throw Error('Subdivision control did not reach level '+level+' of '+count);
};
const sculpt = async (name, number, from, to, steps=14) => {
  await key(number);const metrics=await state();
  const start=await evaluate('performance.now()');
  await pointer('mouseMoved',...from);await pause(100);await pointer('mousePressed',...from,'left',true);
  for(let i=1;i<=steps;i++){const t=i/steps;await pointer('mouseMoved',from[0]+(to[0]-from[0])*t,from[1]+(to[1]-from[1])*t,'left',true);await pause(90);}
  await pointer('mouseReleased',...to);await pause(250);
  const end=await evaluate('performance.now()');
  result.cases.push({name,brush:metrics.kind,phase:await phaseMetrics(start,end),after:await state()});
};
const meshDifference = (a,b) => {
  if(a.positions.length!==b.positions.length||JSON.stringify(a.triangles)!==JSON.stringify(b.triangles))throw Error('Unexpected topology difference');
  let changed=0,max=0,squared=0;
  for(let i=0;i<a.positions.length;i++){let d=0;for(let k=0;k<3;k++){const v=b.positions[i][k]-a.positions[i][k];d+=v*v;}if(d>0)changed++;max=Math.max(max,Math.sqrt(d));squared+=d;}
  return {changed_vertices:changed,maximum_world_displacement:max,rms_world_displacement:Math.sqrt(squared/a.positions.length)};
};
const checkGpu = async (saved,name) => {const gpu=await gpuDigest();result.cases.push({name,gpu,packed_saved_sha256:packedHash(saved.project.mesh),gpu_exact:gpu.sha256===packedHash(saved.project.mesh)});if(gpu.sha256!==packedHash(saved.project.mesh))throw Error('GPU mismatch '+name);};

try {
  let endpoint;
  for (let attempt = 0; attempt < 150; attempt++) { try { const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); endpoint = `ws://127.0.0.1:${Number(lines[0])}${lines[1]}`; break; } catch {} await pause(100); }
  if (!endpoint) throw new Error('No owned Chrome debugging endpoint');
  socket = new WebSocket(endpoint); await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) { const request = pending.get(message.id); if (request) { clearTimeout(request.timer); pending.delete(message.id); message.error ? request.reject(new Error(JSON.stringify(message.error))) : request.resolve(message.result); } return; }
    if (message.method === 'Runtime.exceptionThrown') result.exceptions.push(message.params.exceptionDetails);
    if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') result.console_errors.push(message.params.args.map(argument => argument.value || argument.description).join(' '));
    if (message.method === 'Fetch.requestPaused') { const address = message.params.request.url; const allowed = /^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?\//.test(address) || /^(blob|data|about):/.test(address); if (!allowed) result.blocked_external_requests.push(address); send(allowed ? 'Fetch.continueRequest' : 'Fetch.failRequest', { requestId: message.params.requestId, ...(allowed ? {} : { errorReason: 'BlockedByClient' }) }, message.sessionId).catch(error => result.console_errors.push(String(error))); }
  });
  result.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'Network']) await send(domain + '.enable');
  await send('Fetch.enable', { patterns: [{ urlPattern: '*', requestStage: 'Request' }] });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const bytes = await readFile(path.join(web, 'pkg/obrush_app_bg.wasm')), served = Buffer.from(await (await fetch(url + 'pkg/obrush_app_bg.wasm')).arrayBuffer());
  result.wasm = { sha256: hash(bytes), served_sha256: hash(served), bytes: bytes.length }; if (hash(bytes) !== hash(served)) throw new Error('WASM serving mismatch');

  if(process.argv[5]) {
    const fixture=path.resolve(process.argv[5]);const bytes=await readFile(fixture);const input=JSON.parse(bytes);
    result.timing_fixture={path:fixture,sha256:hash(bytes),bytes:bytes.length};
    await fresh();await importFile(fixture,input.mesh.positions.length);const initial=await save('timing-initial');
    const lowerStart=await evaluate('performance.now()');await click(coordinates.lower);await waitLevel(0,2);const lowerEnd=await evaluate('performance.now()');
    result.lower_phase=await phaseMetrics(lowerStart,lowerEnd);const low=await save('timing-low');
    await sculpt('timing-coarse-draw',1,[835,525],[855,545],7);
    const higherStart=await evaluate('performance.now()');await click(coordinates.higher);await waitLevel(1,2);const higherEnd=await evaluate('performance.now()');
    result.higher_phase=await phaseMetrics(higherStart,higherEnd);const high=await save('timing-high');
    await checkGpu(high,'timing-high-gpu');result.exports=[metadata(initial),metadata(low),metadata(high)];
    result.timing_runtime=await evaluate('window.__topologyCheck');result.screenshot=await screenshot('timing-final');
    if(result.timing_runtime.errors.length||result.timing_runtime.gl_errors.length||result.exceptions.length||result.console_errors.length)throw Error('Timing probe runtime errors');
    result.status='passed targeted multires timing probe';
    result.limitations=['Single isolated headless Chrome/M2 measurement; excludes physical input and nativeGUI.','Measured operations synchronously validate/serialize on the main thread; navigation intervals include harness waits.','Export timing ends when the browser emits project bytes; writing/chunk retrieval and physical downloads are outside that timing.'];
  } else {
  await fresh();
  result.initial_screenshot=await screenshot('initial-controls');
  const base=await save('base');
  const addStart=await evaluate('performance.now()');await click(coordinates.add);const added=await waitLevel(1,2);
  result.add_phase=await phaseMetrics(addStart,await evaluate('performance.now()'));
  const initialHigh=await save('initial-high');
  if(initialHigh.project.version!==2||initialHigh.project.multiresolution?.levels.length!==2||initialHigh.project.mesh.triangles.length!==base.project.mesh.triangles.length*4)throw Error('Actual Add level failed');
  result.add={base:metadata(base),high:metadata(initialHigh),metrics:added};
  result.add_screenshot=await screenshot('after-add');
  await checkGpu(initialHigh,'added-high-gpu');
  // Reimport the identical hierarchy for both branches so camera framing is
  // identical; file load is itself part of the real consumer path.
  await importFile(initialHigh.path,initialHigh.project.mesh.positions.length);
  await sculpt('fine-mask',8,[850,430],[866,438],6);
  await sculpt('fine-paint',9,[800,540],[816,547],6);
  await sculpt('fine-draw',1,[735,430],[765,468]);
  const fine=await save('fine-edit');
  result.fine_draw=meshDifference(initialHigh.project.mesh,fine.project.mesh);
  result.fine_mask_changed=fine.project.mesh.masks.filter((m,i)=>m!==initialHigh.project.mesh.masks[i]).length;
  result.fine_rgb_changed=fine.project.mesh.colors.filter((c,i)=>c.some((v,k)=>v!==initialHigh.project.mesh.colors[i][k])).length;
  if(!result.fine_draw.changed_vertices||result.fine_draw.maximum_world_displacement<0.002||!result.fine_mask_changed||!result.fine_rgb_changed)throw Error('Fine geometry/attributes did not change usefully');
  await click(coordinates.lower);await waitLevel(0,2);const beforeCoarse=await save('before-coarse');
  if(hash(JSON.stringify(beforeCoarse.project.mesh))!==hash(JSON.stringify(base.project.mesh)))throw Error('Fine edit leaked into coarse geometry or attributes');
  await click(coordinates.add);await pause(250);const lowerAddMessage=(await state()).message;const rejectedAdd=await save('rejected-lower-add');
  result.lower_add_failure={message:lowerAddMessage,project_exact:projectHash(rejectedAdd)===projectHash(beforeCoarse)};
  if(!result.lower_add_failure.project_exact||!result.lower_add_failure.message.includes('highest'))throw Error('Lower-level Add changed hierarchy or omitted failure');
  await sculpt('coarse-draw',1,[835,525],[865,555]);const coarse=await save('coarse-edit');
  result.coarse_draw=meshDifference(beforeCoarse.project.mesh,coarse.project.mesh);
  if(!result.coarse_draw.changed_vertices||result.coarse_draw.maximum_world_displacement<0.002)throw Error('Coarse edit did not change usefully');
  await click(coordinates.higher);await waitLevel(1,2);const final=await save('fine-after-coarse');
  await checkGpu(final,'fine-after-coarse-gpu');
  result.inactive_attributes_retained=JSON.stringify(final.project.mesh.masks)===JSON.stringify(fine.project.mesh.masks)&&JSON.stringify(final.project.mesh.colors)===JSON.stringify(fine.project.mesh.colors);
  if(!result.inactive_attributes_retained)throw Error('Fine attributes lost during coarse edit');
  await chord('z');const undone=await save('coarse-undo');await chord('z',true);const redone=await save('coarse-redo');
  result.sculpt_history={undo_exact:projectHash(undone)===projectHash(beforeCoarse),redo_exact:projectHash(redone)===projectHash(final)};
  if(!Object.values(result.sculpt_history).every(Boolean))throw Error('Multires sculpt Undo/Redo mismatch');
  await click(coordinates.cube);const cube=await save('cube-replacement');
  if(cube.project.multiresolution)throw Error('Replacement retained incompatible levels');
  await chord('z');const restored=await save('hierarchy-restored');await chord('z',true);const cubeRedo=await save('cube-redo');
  result.replacement_history={undo_exact:projectHash(restored)===projectHash(final),redo_exact:projectHash(cubeRedo)===projectHash(cube)};
  if(!Object.values(result.replacement_history).every(Boolean))throw Error('Topology replacement lost hierarchy history');
  await importFile(final.path,final.project.mesh.positions.length);const roundtrip=await save('loaded-hierarchy');
  result.load_exact=projectHash(roundtrip)===projectHash(final);if(!result.load_exact)throw Error('Version2 load did not restore exact hierarchy');
  const damaged=structuredClone(final.project);damaged.multiresolution.levels[0].triangles[0][0]=4294967295;
  const malformed=path.join(out,'malformed-hierarchy.obrush');await writeFile(malformed,JSON.stringify(damaged));
  const dom=(await send('DOM.getDocument')).root;const nodeId=(await send('DOM.querySelector',{nodeId:dom.nodeId,selector:'#import-file'})).nodeId;
  await send('DOM.setFileInputFiles',{nodeId,files:[malformed]});await pause(750);
  result.malformed_notice=await evaluate('document.getElementById("notice").textContent');
  const unchanged=await save('after-malformed-import');result.malformed_preserves_document=projectHash(unchanged)===projectHash(final);
  if(!result.malformed_notice||!result.malformed_preserves_document)throw Error('Malformed hierarchy replaced current project or lacked notice');
  result.final_screenshot=await screenshot('final-hierarchy');
  // A matched branch without fine edits independently checks that fine
  // residuals survive applying precisely the same coarse-level stroke.
  await fresh();await importFile(initialHigh.path,initialHigh.project.mesh.positions.length);
  await click(coordinates.lower);await waitLevel(0,2);
  await sculpt('control-coarse-draw',1,[835,525],[865,555]);const controlCoarse=await save('control-coarse');
  result.control_coarse_difference=meshDifference(coarse.project.mesh,controlCoarse.project.mesh);
  if(result.control_coarse_difference.maximum_world_displacement>2e-6)throw Error('Control coarse stroke differed; residual comparison invalid');
  await click(coordinates.higher);await waitLevel(1,2);const control=await save('control-high');
  let residualMax=0,residualSquared=0,originalMagnitude=0;
  for(let i=0;i<final.project.mesh.positions.length;i++)for(let k=0;k<3;k++){
    const before=fine.project.mesh.positions[i][k]-initialHigh.project.mesh.positions[i][k];
    const after=final.project.mesh.positions[i][k]-control.project.mesh.positions[i][k];const error=after-before;
    residualMax=Math.max(residualMax,Math.abs(error));residualSquared+=error*error;originalMagnitude+=before*before;
  }
  result.fine_residual={maximum_component_error:residualMax,rms_component_error:Math.sqrt(residualSquared/(final.project.mesh.positions.length*3)),original_rms_magnitude:Math.sqrt(originalMagnitude/(final.project.mesh.positions.length*3))};
  if(residualMax>2e-6||originalMagnitude===0)throw Error('Fine residual was lost or changed');
  const runtime=await evaluate('window.__topologyCheck');
  result.runtime_errors={gl:runtime.gl_errors,js:runtime.errors};result.actual_4x_msaa=runtime.allocations.some(a=>a.actual_samples===4);
  if(runtime.gl_errors.length||runtime.errors.length||result.exceptions.length||result.console_errors.length||!result.actual_4x_msaa)throw Error('Runtime errors or missing4x viewport');
  result.status='passed persistent subdivision workflow';
  result.limitations=['Headless Chrome/M2, synthetic mouse input andDPR1; not physical latency or nativeGUI evidence.','Add/select/Save validate and serialize on the main thread; retained longtasks measure this limitation.','Fine detail transport uses additive world-space residuals; rotations/scales do not rotate/scale those vectors.','Cumulative UI selection is not an Undo step; geometry and hierarchy changes restore whole document state.','Shared-core tests establish interpolation contracts; this browser branch checks useful actual edits, retained detail and public file/history behavior.'];
  }
} catch (error) {
  result.status = 'failed persistent subdivision workflow'; result.failure = String(error); process.exitCode = 1;
  try { result.failure_state = await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__topologyCheck,notice:document.getElementById("notice")?.textContent})'); result.failure_screenshot = await screenshot('failure'); } catch {}
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  await writeFile(path.join(out, 'chrome-stderr.log'), stderr); await writeFile(path.join(out, 'chrome-stdout.log'), stdout);
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {} finally { socket?.close(); }
  for (const request of pending.values()) { clearTimeout(request.timer); request.reject(new Error('Owned browser closing')); } pending.clear();
  await pause(500); if (child.exitCode === null) child.kill('SIGTERM'); await pause(200); server.close();
  result.cleanup = { owned_chrome_exit_code: child.exitCode, owned_chrome_signal: child.signalCode, owned_server_closed: true };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json'), wasm: result.wasm, cases: result.cases.map(trial => ({ name: trial.name, message: trial.message, history: trial.history, gpu_matches_saved_mesh: trial.gpu_matches_saved_mesh, elapsed_ms: trial.phase?.elapsed_ms, busy_raf_frames: trial.phase?.busy_raf_frames })) }));
}
