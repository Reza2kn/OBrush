// Own loopback server, headless Chrome process/profile and CDP tab only.
// Does not attach to user tabs, accessibility, desktop apps or visible windows.
// Usage: node scripts/check-topology.mjs [web/dist] [label] [optional coordinate JSON]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'topology-workflow';
const out = path.resolve(`.artifacts/topology-browser-0.1.6/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
await mkdir(profile, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const coordinates = process.argv[4] ? JSON.parse(await readFile(process.argv[4], 'utf8')) : {
  expand: [1480, 392], remesh: [1446, 543], decimate: [1435, 634], cancel: [1450, 692], cube: [1473, 257], resolution_max: [1574, 506],
};
const fixture = path.resolve('.artifacts/user-long-stroke-0.1.4/user.obrush');
const openFile = path.join(out, 'independent-open-sheet.obj');
await writeFile(openFile, 'v -0.6 -0.6 0\nv 0.6 -0.6 0\nv 0.6 0.6 0\nv -0.6 0.6 0\nf 1 2 3\nf 1 3 4\n');
const serverRequests = [];
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) { response.writeHead(403); response.end(); return; }
    const bytes = await readFile(file);
    serverRequests.push({ epoch_ms: Date.now(), pathname: new URL(request.url, 'http://127.0.0.1').pathname, bytes: bytes.length, sha256: hash(bytes), referer: request.headers.referer || null, destination: request.headers['sec-fetch-dest'] || null });
    response.setHeader('Cache-Control', 'no-store');
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
  server_requests: serverRequests,
  tolerances: { history: 'Exact serialized project and every mesh field', gpu: 'Exact packed f32 vertex-buffer hash', boundary: 'Core tests, not independently remeasured in this host harness' },
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
  gl.__topologyObserved=true;window.__topologyGl=gl;const debug=gl.getExtension('WEBGL_debug_renderer_info');p.contexts.push({kind,version:gl.getParameter(gl.VERSION),renderer:debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):null,vendor:debug?gl.getParameter(debug.UNMASKED_VENDOR_WEBGL):null,max_samples:gl.getParameter(gl.MAX_SAMPLES)});
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
  const index = await evaluate('window.__topologyCaptures.length');
  await chord('s');
  let size;
  for (let attempt = 0; attempt < 80; attempt++) { size = await evaluate(`window.__topologyCaptures[${index}]?.contents.length||0`); if (size) break; await pause(50); }
  if (!size || size > 128 * 1024 * 1024) throw new Error(`Save capture invalid: ${name}, ${size}`);
  let contents = '';
  for (let offset = 0; offset < size; offset += 131072) contents += await evaluate(`window.__topologyCaptures[${index}].contents.slice(${offset},${offset + 131072})`);
  const file = path.join(out, name + '.obrush'); await writeFile(file, contents);
  return { path: file, sha256: hash(contents), bytes: Buffer.byteLength(contents), project: JSON.parse(contents) };
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
      await evaluate(`window.__topologyCaptures=[];window.obrushDownload=(name,contents)=>window.__topologyCaptures.push({name,contents});document.getElementById('obrush_canvas').focus();const start=window.obrushStartTopology;window.obrushStartTopology=(id,project,operation)=>{window.__topologyCheck.jobs.push({id,at:performance.now(),project_bytes:project.length,operation:JSON.parse(operation)});return start(id,project,operation);};const cancel=window.obrushCancelTopology;window.obrushCancelTopology=id=>{window.__topologyCheck.cancels.push({id,at:performance.now()});return cancel(id);};`);
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
  result.fixture = { path: fixture, sha256: hash(await readFile(fixture)) };
  result.open_fixture = { path: openFile, sha256: hash(await readFile(openFile)) };
  for (const [name, kind, file] of [['sphere-decimate', 'decimate', null], ['sphere-remesh', 'remesh', null], ['user-decimate', 'decimate', fixture], ['user-remesh', 'remesh', fixture]]) {
    await fresh(); if (file) await importFile(file, 61580);
    const trial = { name, kind, snapshots: [] }; result.cases.push(trial);
    const initial = await save(name + '-initial'); trial.snapshots.push(metadata(initial));
    if (name === 'sphere-decimate') {
      const masked = structuredClone(initial.project);
      masked.name = 'Masked sphere'; masked.mesh.masks.fill(1);
      fullyMaskedFile = path.join(out, 'independent-fully-masked-sphere.obrush');
      const contents = JSON.stringify(masked); await writeFile(fullyMaskedFile, contents);
      result.fully_masked_fixture = { path: fullyMaskedFile, sha256: hash(contents), vertices: masked.mesh.positions.length };
    }
    await click(coordinates.expand); trial.controls = await screenshot(name + '-controls');
    const phase = await begin(coordinates[kind]); await orbit(); const completed = await waitForCompletion(phase.jobs);
    trial.message = completed.metrics.message;
    trial.phase = await phaseMetrics(phase.start, completed.at);
    trial.gpu_before_save = await gpuDigest();
    const final = await save(name + '-final'); trial.snapshots.push(metadata(final));
    trial.gpu_matches_saved_mesh = trial.gpu_before_save.sha256 === packedHash(final.project.mesh);
    trial.changed = projectHash(final) !== projectHash(initial);
    if (!trial.changed || !trial.gpu_matches_saved_mesh || !/updated|Reduced/.test(trial.message || '')) throw new Error(`No accepted actual mesh result: ${name}, ${trial.message}`);
    if (kind === 'decimate' && final.project.mesh.triangles.length > Math.ceil(initial.project.mesh.triangles.length * 0.5)) throw new Error(`Default fixture missed decimation target: ${name}`);
    await chord('z'); const undo = await save(name + '-undo'); trial.snapshots.push(metadata(undo));
    await chord('z', true); const redo = await save(name + '-redo'); trial.snapshots.push(metadata(redo));
    trial.history = { undo_exact_project: projectHash(undo) === projectHash(initial), redo_exact_project: projectHash(redo) === projectHash(final) };
    if (!Object.values(trial.history).every(Boolean)) throw new Error(`History mismatch: ${name}`);
    trial.screenshot = await screenshot(name + '-after-redo');
    await writeFile(path.join(out, name + '.json'), JSON.stringify(trial, null, 2) + '\n');
  }
  await fresh(); await importFile(fixture, 61580);
  {
    const trial = { name: 'cancel-and-brush-lock', snapshots: [] }; result.cases.push(trial);
    const initial = await save(trial.name + '-initial'); trial.snapshots.push(metadata(initial));
    await click(coordinates.expand);
    await click(coordinates.resolution_max);
    const phase = await begin(coordinates.remesh);
    trial.job_settings = await evaluate('window.__topologyCheck.jobs.at(-1).operation.settings');
    if (trial.job_settings.resolution < 112) throw new Error('The longer cancellation fixture did not select a high resolution');
    trial.busy_before_contact = await evaluate('window.obrushBrushMetrics().topology_busy');
    if (!trial.busy_before_contact) throw new Error('Remesh ended before cancel/brush-lock check');
    trial.busy_screenshot = await screenshot(trial.name + '-busy');
    await pointer('mouseMoved', 790, 470); await pointer('mousePressed', 790, 470, 'left', true);
    for (let move = 0; move < 5; move++) { await pointer('mouseMoved', 790 + move * 4, 470 + move * 3, 'left', true); await pause(15); }
    await pointer('mouseReleased', 806, 482);
    trial.busy_at_cancel = await evaluate('window.obrushBrushMetrics().topology_busy');
    if (!trial.busy_at_cancel) throw new Error('The operation completed before the cancellation check');
    await click(coordinates.cancel); await pause(1600);
    trial.final_metrics = await evaluate('window.obrushBrushMetrics()');
    const final = await save(trial.name + '-final'); trial.snapshots.push(metadata(final));
    trial.project_unchanged_after_cancel = projectHash(initial) === projectHash(final);
    trial.phase = await phaseMetrics(phase.start, await evaluate('performance.now()'));
    if (trial.final_metrics.topology_busy || !trial.project_unchanged_after_cancel || !trial.phase.cancels.length) throw new Error('Cancel or brush lock changed the sculpt');
  }
  await fresh(); await importFile(openFile, 4);
  {
    const trial = { name: 'failed-open-remesh-preserves-redo', snapshots: [] }; result.cases.push(trial);
    const open = await save(trial.name + '-open'); trial.snapshots.push(metadata(open));
    await click(coordinates.cube); const cube = await save(trial.name + '-cube'); trial.snapshots.push(metadata(cube));
    if (cube.project.mesh.positions.length === open.project.mesh.positions.length) throw new Error('The Cube control did not create the open-fixture history setup');
    await chord('z'); const before = await save(trial.name + '-before'); trial.snapshots.push(metadata(before));
    if (projectHash(open) !== projectHash(before)) throw new Error('Fixture setup undo failed');
    await click(coordinates.expand); const phase = await begin(coordinates.remesh); const completed = await waitForCompletion(phase.jobs);
    trial.message = completed.metrics.message;
    const rejected = await save(trial.name + '-rejected'); trial.snapshots.push(metadata(rejected));
    trial.failed_mesh_exact = projectHash(before) === projectHash(rejected);
    await chord('z', true); const redo = await save(trial.name + '-redo'); trial.snapshots.push(metadata(redo));
    trial.redo_preserved = projectHash(cube) === projectHash(redo);
    if (!trial.failed_mesh_exact || !trial.redo_preserved || !/failed|preserved/.test(trial.message || '')) throw new Error('Failed remesh lost mesh, redo or visible failure report');
  }
  await fresh(); await importFile(fullyMaskedFile, result.fully_masked_fixture.vertices);
  {
    const trial = { name: 'protected-noop-preserves-redo', snapshots: [] }; result.cases.push(trial);
    const initial = await save(trial.name + '-initial'); trial.snapshots.push(metadata(initial));
    await click(coordinates.cube); const cube = await save(trial.name + '-cube'); trial.snapshots.push(metadata(cube));
    if (cube.project.mesh.positions.length === initial.project.mesh.positions.length) throw new Error('The Cube control did not create the protected-fixture history setup');
    await chord('z'); const before = await save(trial.name + '-before'); trial.snapshots.push(metadata(before));
    if (projectHash(before) !== projectHash(initial)) throw new Error('Protected fixture setup undo failed');
    await click(coordinates.expand); const phase = await begin(coordinates.decimate); const completed = await waitForCompletion(phase.jobs);
    trial.message = completed.metrics.message;
    const final = await save(trial.name + '-final'); trial.snapshots.push(metadata(final));
    trial.noop_exact = projectHash(final) === projectHash(before);
    await chord('z', true); const redo = await save(trial.name + '-redo'); trial.snapshots.push(metadata(redo));
    trial.redo_preserved = projectHash(cube) === projectHash(redo);
    if (!trial.noop_exact || !trial.redo_preserved || !/No safe reduction/.test(trial.message || '')) throw new Error('Protected no-op changed project/history or falsely reported reduction');
  }
  await fresh(); await importFile(fixture, 61580);
  {
    const trial = { name: 'new-primitive-cancels-old-worker', snapshots: [] }; result.cases.push(trial);
    const initial = await save(trial.name + '-initial'); trial.snapshots.push(metadata(initial));
    await click(coordinates.expand); const phase = await begin(coordinates.remesh);
    trial.busy_before_primitive = await evaluate('window.obrushBrushMetrics().topology_busy');
    if (!trial.busy_before_primitive) throw new Error('Remesh ended before primitive cancellation check');
    await click(coordinates.cube); const cube = await save(trial.name + '-cube'); trial.snapshots.push(metadata(cube));
    if (cube.project.mesh.positions.length === initial.project.mesh.positions.length) throw new Error('The Cube control did not replace the running job fixture');
    const completed = await waitForCompletion(phase.jobs); trial.message = completed.metrics.message;
    await pause(1600);
    const final = await save(trial.name + '-final'); trial.snapshots.push(metadata(final));
    trial.late_result_does_not_replace_new_mesh = projectHash(cube) === projectHash(final);
    trial.worker_cancelled = await evaluate('window.__topologyCheck.cancels.length>0');
    if (!trial.late_result_does_not_replace_new_mesh || !trial.worker_cancelled) throw new Error('The old operation replaced the newer mesh or its worker was not cancelled');
    await chord('z'); const undo = await save(trial.name + '-undo'); trial.snapshots.push(metadata(undo));
    await chord('z', true); const redo = await save(trial.name + '-redo'); trial.snapshots.push(metadata(redo));
    trial.history = { undo_exact_project: projectHash(undo) === projectHash(initial), redo_exact_project: projectHash(redo) === projectHash(cube) };
    if (!Object.values(trial.history).every(Boolean)) throw new Error('Stale operation consumed history');
  }
  const runtime = await evaluate('window.__topologyCheck');
  result.runtime_errors = { gl: runtime.gl_errors, js: runtime.errors };
  result.contexts = runtime.contexts;
  result.actual_4x_msaa = runtime.allocations.some(allocation => allocation.actual_samples === 4);
  const wasmRequests = serverRequests.filter(request => request.pathname.endsWith('/obrush_app_bg.wasm'));
  const workerRequests = serverRequests.filter(request => request.pathname.endsWith('/topology-worker.js'));
  const workerWasmRequests = wasmRequests.filter(request => request.referer?.includes('topology-worker.js'));
  result.worker_asset_delivery = {
    cache_policy: 'Owned server sends no-store to require fresh, hash-recorded worker asset delivery',
    wasm_requests: wasmRequests.length, worker_script_requests: workerRequests.length, worker_referrer_wasm_requests: workerWasmRequests.length,
    all_delivered_wasm_sha256_match: wasmRequests.every(request => request.sha256 === result.wasm.sha256),
    worker_script_sha256: hash(await readFile(path.join(web, 'topology-worker.js'))),
    attribution: workerWasmRequests.length ? 'Worker-script referrers identify the delivered WASM bytes directly' : 'Worker script delivery and all WASM response hashes recorded; browser referrer policy did not identify individual worker fetches',
  };
  if (!workerRequests.length || !result.worker_asset_delivery.all_delivered_wasm_sha256_match) throw new Error('Worker asset delivery missing or WASM hash mismatch');
  if (runtime.gl_errors.length || runtime.errors.length || result.exceptions.length || result.console_errors.length || !result.actual_4x_msaa) throw new Error('Runtime errors or missing 4x viewport');
  result.status = 'passed topology workflow';
  result.limitations = ['Synthetic CDP input and headless M2 Chrome, not physical input-to-photon latency or native GUI.', 'rAF and WebGL draw calls establish scheduling and submissions, not GPU timer measurements.', 'Actual topology preparation and acceptance serialize/validate on the main thread; phase long tasks are retained.', 'The user remesh result is tested for working operation/history, not repair quality or reference parity.', 'Each worker receives assets from an owned no-store loopback server; startup timing includes delivery and initialization rather than a warm worker pool.'];
} catch (error) {
  result.status = 'failed topology workflow'; result.failure = String(error); process.exitCode = 1;
  try { result.failure_state = await evaluate('({metrics:window.obrushBrushMetrics?.(),runtime:window.__topologyCheck,notice:document.getElementById("notice")?.textContent})'); result.failure_screenshot = await screenshot('failure'); } catch {}
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  await writeFile(path.join(out, 'chrome-stderr.log'), stderr); await writeFile(path.join(out, 'chrome-stdout.log'), stdout);
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {} finally { socket?.close(); }
  for (const request of pending.values()) { clearTimeout(request.timer); request.reject(new Error('Owned browser closing')); } pending.clear();
  await pause(500); if (child.exitCode === null) child.kill('SIGTERM'); await pause(200);
  child.stdout.destroy(); child.stderr.destroy();
  await new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
  result.cleanup = { owned_chrome_exit_code: child.exitCode, owned_chrome_signal: child.signalCode, owned_server_closed: true };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json'), wasm: result.wasm, cases: result.cases.map(trial => ({ name: trial.name, message: trial.message, history: trial.history, gpu_matches_saved_mesh: trial.gpu_matches_saved_mesh, elapsed_ms: trial.phase?.elapsed_ms, busy_raf_frames: trial.phase?.busy_raf_frames })) }));
}
