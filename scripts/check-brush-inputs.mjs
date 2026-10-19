// CLI-only brush input checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-brush-inputs.mjs [web/dist] [run-label] [--pressure-only]
// --pressure-only runs four independent default-policy pen/mouse cases while
// the wider alpha workflow keeps its separate import-request gate.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { deflateSync } from 'node:zlib';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'brush-inputs';
const pressureOnly = process.argv.includes('--pressure-only');
const out = path.resolve(`.artifacts/brush-inputs-0.1.6/${label}-${randomUUID()}`);
const profile = path.join(out, 'isolated-profile');
await mkdir(profile, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

// Independently authored 8-bit grayscale stamps; no downloaded artwork.
const alphaRows = {
  asymmetric: [[0, 0, 0, 0, 0], [0, 255, 192, 64, 0], [0, 192, 128, 32, 0], [0, 64, 32, 0, 0], [0, 0, 0, 0, 0]],
  white: Array.from({ length: 5 }, () => Array(5).fill(255)),
  black: Array.from({ length: 5 }, () => Array(5).fill(0)),
};
const crc32 = bytes => {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
};
const pngChunk = (kind, data) => {
  const body = Buffer.concat([Buffer.from(kind), data]);
  const length = Buffer.alloc(4); length.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
};
const fixtures = {};
for (const [name, rows] of Object.entries(alphaRows)) {
  const header = Buffer.alloc(13); header.writeUInt32BE(5); header.writeUInt32BE(5, 4); header[8] = 8;
  const raw = Buffer.concat(rows.map(row => Buffer.from([0, ...row])));
  const bytes = Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), pngChunk('IHDR', header), pngChunk('IDAT', deflateSync(raw)), pngChunk('IEND', Buffer.alloc(0))]);
  const file = path.join(out, `${name}-5x5.png`); await writeFile(file, bytes);
  fixtures[name] = { path: file, sha256: hash(bytes), bytes: bytes.length, width: 5, height: 5, rows };
}
{
  // A valid compressed PNG exceeding the declared dimension limit, rather
  // than a corrupt file whose decoder failure could conceal a missing guard.
  const header = Buffer.alloc(13); header.writeUInt32BE(1025); header.writeUInt32BE(1, 4); header[8] = 8;
  const raw = Buffer.concat([Buffer.from([0]), Buffer.alloc(1025, 255)]);
  const bytes = Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), pngChunk('IHDR', header), pngChunk('IDAT', deflateSync(raw)), pngChunk('IEND', Buffer.alloc(0))]);
  const file = path.join(out, 'oversized-1025x1.png'); await writeFile(file, bytes);
  fixtures.oversized = { path: file, sha256: hash(bytes), bytes: bytes.length, width: 1025, height: 1, expected: 'Reject without replacing the current brush alpha or mesh' };
}

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
const chrome = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const args = ['--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-breakpad', '--metrics-recording-only', '--safebrowsing-disable-auto-update', '--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication', '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1', '--window-size=1600,1000', 'about:blank'];
const child = spawn(chrome, args, { stdio: ['ignore', 'pipe', 'pipe'] });
let stderr = '', stdout = ''; child.stderr.on('data', bytes => stderr += bytes); child.stdout.on('data', bytes => stdout += bytes);
let socket, sequence = 0, session;
const pending = new Map();
const result = {
  label, status: 'running', method: 'Isolated CLI headless Chrome/CDP; no desktop, CUA, accessibility, user-tab, or physical device control',
  out, url, web, profile, owned_pid: child.pid, fixtures,
  harness_sha256: hash(await readFile(new URL(import.meta.url))),
  prerecorded_tolerances: { quarter_pressure_max_absolute_position_error: 3e-7, quarter_pressure_median_ratio_error: 0.01, mirrored_original_vertex_max_error: 1e-7 },
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
 const p=window.__brushCheck={events:[],keys:[],contexts:[],allocations:[],viewports:{},gl_errors:[],js_errors:[]};
 for(const type of ['keydown','keyup'])window.addEventListener(type,event=>p.keys.push({type,key:event.key,meta:event.metaKey,ctrl:event.ctrlKey,shift:event.shiftKey,trusted:event.isTrusted,target:event.target.id}),true);
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,trusted:event.isTrusted,at:performance.now()}),true);
 window.addEventListener('error',event=>p.js_errors.push(String(event.error||event.message)));
 window.addEventListener('unhandledrejection',event=>p.js_errors.push(String(event.reason)));
 const get=HTMLCanvasElement.prototype.getContext;
 HTMLCanvasElement.prototype.getContext=function(kind,...args){const gl=get.call(this,kind,...args);if(kind==='webgl2'&&gl&&!gl.__brushObserved){
  gl.__brushObserved=true;window.__brushGl=gl;
  p.contexts.push({kind,version:gl.getParameter(gl.VERSION),max_samples:gl.getParameter(gl.MAX_SAMPLES)});
  const bind=gl.bindBuffer;gl.bindBuffer=function(target,buffer){if(target===this.ARRAY_BUFFER)this.__brushArrayBuffer=buffer;return bind.call(this,target,buffer);};
  const data=gl.bufferData;gl.bufferData=function(target,bytes,...args){const value=data.call(this,target,bytes,...args);const count=typeof bytes==='number'?bytes:bytes?.byteLength||0;if(target===this.ARRAY_BUFFER&&count>1000000)window.__brushMeshVbo={buffer:this.__brushArrayBuffer,bytes:count,at:performance.now()};return value;};
  const viewport=gl.viewport;gl.viewport=function(x,y,width,height){p.viewports[[x,y,width,height].join(',')]=true;return viewport.call(this,x,y,width,height);};
  const storage=gl.renderbufferStorageMultisample;gl.renderbufferStorageMultisample=function(target,samples,format,width,height){const value=storage.call(this,target,samples,format,width,height);p.allocations.push({samples,actual_samples:this.getRenderbufferParameter(target,this.RENDERBUFFER_SAMPLES),width,height});return value;};
  const error=gl.getError;gl.getError=function(){const value=error.call(this);if(value!==this.NO_ERROR)p.gl_errors.push(value);return value;};
 }return gl;};
})();`;

const pointer = async (type, x, y, pointerType = 'mouse', force = 0.5, down = false) => {
  const params = { type, x, y, pointerType, force, button: type === 'mouseMoved' ? 'none' : 'left', buttons: down ? 1 : 0, ...(type === 'mouseMoved' ? {} : { clickCount: 1 }) };
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
const loadAlpha = async name => {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#alpha-file, #import-alpha, input[accept*="png"]' })).nodeId;
  if (!nodeId) throw new Error('No public alpha file input');
  await send('DOM.setFileInputFiles', { nodeId, files: [fixtures[name].path] });
  for (let attempt = 0; attempt < 100; attempt++) {
    const metrics = await evaluate('window.obrushBrushMetrics()');
    if (metrics.alpha?.includes(name)) return metrics;
    const notice = await evaluate('document.getElementById("notice").textContent');
    if (notice) throw new Error(notice);
    await pause(50);
  }
  throw new Error(`Public alpha import timed out: ${name}`);
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

  const documents = {};
  const cases = [
    ['pen-zero', 'pen', 0, null], ['pen-quarter', 'pen', 0.25, null], ['pen-full', 'pen', 1, null],
    ['mouse-default', 'mouse', 0.5, null], ['alpha-black', 'pen', 1, 'black'], ['alpha-white', 'pen', 1, 'white'], ['alpha-asymmetric', 'pen', 1, 'asymmetric'],
    ['pen-radius-half', 'pen', 0.5, null, 'pressure-radius'], ['alpha-rotated', 'pen', 1, 'asymmetric', 'rotation'],
  ];
  for (const [name, pointerType, pressure, alpha, uiControl] of pressureOnly ? cases.slice(0, 4) : cases) {
    await fresh();
    const trial = { name, pointer_type: pointerType, force: pressure, alpha, snapshots: [] }; result.cases.push(trial);
    trial.initial_settings = await evaluate('window.obrushBrushMetrics()');
    const initial = await save(`${name}-initial`); trial.snapshots.push(metadata(initial));
    if (alpha) trial.alpha_import_settings = await loadAlpha(alpha);
    if (uiControl) {
      // These points are the actual controls inspected in the owned 1600x1000
      // screenshot, with the fixed left panel and standalone rotation row.
      const [controlX, controlY] = uiControl === 'pressure-radius' ? [22, 653] : [106, 762];
      await pointer('mouseMoved', controlX, controlY); await pause(40);
      await pointer('mousePressed', controlX, controlY, 'mouse', 0.5, true); await pause(60);
      await pointer('mouseReleased', controlX, controlY); await pause(200);
      trial.ui_control = { name: uiControl, pointer: [controlX, controlY], settings: await evaluate('window.obrushBrushMetrics()') };
      if (uiControl === 'pressure-radius' && !trial.ui_control.settings.pressure_radius) throw new Error('Pressure radius checkbox did not toggle');
      if (uiControl === 'rotation' && !(trial.ui_control.settings.alpha_rotation > 0.05)) throw new Error('Rotation slider did not change');
    }
    const runtime = await evaluate('window.__brushCheck');
    const viewport = Object.keys(runtime.viewports).map(value => value.split(',').map(Number)).filter(([x, y, width, height]) => x > 0 && width > 400 && height > 300).at(-1);
    if (!viewport || !runtime.allocations.some(allocation => allocation.actual_samples === 4)) throw new Error('Missing actual sculpt viewport or 4x MSAA');
    const [left, bottom, width, height] = viewport;
    const x = left + width / 2 - 85, y = 1000 - bottom - height / 2 - 30;
    trial.viewport = { left, bottom, width, height, pointer: [x, y] };
    await pointer('mouseMoved', x, y, pointerType, pressure, false); await pause(60);
    await pointer('mousePressed', x, y, pointerType, pressure, true); await pause(120);
    trial.settings_during_contact = await evaluate('window.obrushBrushMetrics()');
    await pointer('mouseReleased', x, y, pointerType, 0, false); await pause(350);
    trial.gpu_before_save = await gpuDigest();
    trial.metrics_after_release = await evaluate('window.obrushSculptMetrics()');
    const final = await save(`${name}-final`); trial.snapshots.push(metadata(final)); documents[name] = { initial: initial.document, final: final.document };
    trial.gpu_matches_saved_mesh = trial.gpu_before_save.sha256 === packedHash(final.document.mesh);
    const source = initial.document.mesh.positions;
    trial.changed_original_vertices = source.filter((point, index) => point.some((coordinate, channel) => coordinate !== final.document.mesh.positions[index]?.[channel])).length;
    trial.actual_dom_pen_events = await evaluate(`window.__brushCheck.events.filter(event=>event.pointer_type===${JSON.stringify(pointerType)}&&event.x===${x}&&event.y===${y})`);
    trial.runtime_errors = await evaluate('({gl:window.__brushCheck.gl_errors,js:window.__brushCheck.js_errors,notice:document.getElementById("notice").textContent})');
    if (trial.runtime_errors.gl.length || trial.runtime_errors.js.length || trial.runtime_errors.notice || !trial.gpu_matches_saved_mesh) throw new Error(`Runtime error or GPU/save mismatch: ${name}`);
    if (!trial.actual_dom_pen_events.some(event => event.type === 'pointerdown' && event.trusted && Math.abs(event.pressure - pressure) < 1e-6)) throw new Error(`Missing real dispatched DOM pointer pressure: ${name}`);
    const expectedPressure = pointerType === 'mouse' ? 1 : pressure;
    if (typeof trial.settings_during_contact.pressure !== 'number' || Math.abs(trial.settings_during_contact.pressure - expectedPressure) > 1e-6) throw new Error(`Host captured wrong pressure: ${name}`);
    if (trial.metrics_after_release.pending_dabs !== 0) throw new Error(`Release retains pending input: ${name}`);
    if (name === 'alpha-asymmetric' || name === 'alpha-rotated' || name === 'pen-radius-half') trial.screenshot = await screenshot(name);
    if (name === 'alpha-asymmetric') {
      await chord('z'); const undo = await save(`${name}-undo`); trial.snapshots.push(metadata(undo));
      await chord('z', true); const redo = await save(`${name}-redo`); trial.snapshots.push(metadata(redo));
      trial.history = { undo_exact_initial: meshHash(undo.document) === meshHash(initial.document), redo_exact_final: meshHash(redo.document) === meshHash(final.document) };
      if (!Object.values(trial.history).every(Boolean)) throw new Error('Alpha undo/redo does not restore every mesh field');
      const root = (await send('DOM.getDocument')).root;
      const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#alpha-file, #import-alpha, input[accept*="png"]' })).nodeId;
      await send('DOM.setFileInputFiles', { nodeId, files: [fixtures.oversized.path] });
      let rejection;
      for (let attempt = 0; attempt < 100; attempt++) {
        rejection = await evaluate('document.getElementById("notice").textContent');
        if (rejection) break;
        await pause(50);
      }
      const retainedSettings = await evaluate('window.obrushBrushMetrics()');
      const retained = await save(`${name}-after-rejected-alpha`); trial.snapshots.push(metadata(retained));
      trial.invalid_alpha = { notice: rejection, retained_alpha: retainedSettings.alpha, mesh_exact: meshHash(retained.document) === meshHash(final.document) };
      if (!/1024|dimension|limit/i.test(rejection || '') || !retainedSettings.alpha?.includes('asymmetric') || !trial.invalid_alpha.mesh_exact) throw new Error('Oversized alpha rejection lost the current brush or sculpt');
    }
    await writeFile(path.join(out, `${name}.json`), JSON.stringify(trial, null, 2) + '\n');
  }
  if (pressureOnly) {
    const same = (a, b) => meshHash(a) === meshHash(b);
    const base = documents['pen-full'].initial.mesh.positions;
    const full = documents['pen-full'].final.mesh.positions;
    const quarter = documents['pen-quarter'].final.mesh.positions;
    let maxError = 0;
    let changed = 0;
    for (let index = 0; index < base.length; index++) {
      for (let channel = 0; channel < 3; channel++) {
        const delta = full[index][channel] - base[index][channel];
        const actual = quarter[index][channel] - base[index][channel];
        maxError = Math.max(maxError, Math.abs(actual - delta * 0.25));
        if (actual !== 0) changed++;
      }
    }
    result.quarter_pressure_response = { changed_position_components: changed, max_absolute_position_error: maxError, expected_ratio: 0.25 };
    result.comparisons = {
      zero_pressure_exact_noop: same(documents['pen-zero'].initial, documents['pen-zero'].final),
      mouse_default_matches_pen_full_exactly: same(documents['mouse-default'].final, documents['pen-full'].final),
      quarter_pressure_scales_geometry: changed > 0 && maxError <= 3e-7,
    };
    if (!Object.values(result.comparisons).every(Boolean) || result.exceptions.length || result.console_errors.length) throw new Error('Basic pressure browser gate failed');
    result.status = 'passed basic pressure browser checks';
    result.limitations = ['Synthetic trusted CDP input on four fresh spheres; SculptGL factor controls, pressure ramps, physical tablets and alpha import are outside this mode.'];
  } else {
  const same = (a, b) => meshHash(a) === meshHash(b);
  result.comparisons = {
    zero_pressure_exact_noop: same(documents['pen-zero'].initial, documents['pen-zero'].final),
    mouse_default_matches_pen_full_exactly: same(documents['mouse-default'].final, documents['pen-full'].final),
    black_alpha_exact_noop: same(documents['alpha-black'].initial, documents['alpha-black'].final),
    white_alpha_matches_no_alpha_exactly: same(documents['alpha-white'].final, documents['pen-full'].final),
    asymmetric_alpha_has_distinct_nonempty_geometry: !same(documents['alpha-asymmetric'].final, documents['alpha-asymmetric'].initial) && !same(documents['alpha-asymmetric'].final, documents['pen-full'].final),
    rotation_control_changes_alpha_geometry: !same(documents['alpha-rotated'].final, documents['alpha-rotated'].initial) && !same(documents['alpha-rotated'].final, documents['alpha-asymmetric'].final),
  };
  const changedIds = name => {
    const { initial, final } = documents[name];
    return initial.mesh.positions.flatMap((point, index) => point.some((coordinate, channel) => coordinate !== final.mesh.positions[index][channel]) ? [index] : []);
  };
  const fullIds = new Set(changedIds('pen-full')), radiusIds = changedIds('pen-radius-half');
  result.pressure_radius_response = { full_changed_original_vertices: fullIds.size, half_radius_changed_original_vertices: radiusIds.length, changed_vertex_fraction: radiusIds.length / fullIds.size };
  result.comparisons.pressure_radius_reduces_footprint = radiusIds.length > 0 && radiusIds.length < fullIds.size / 2 && radiusIds.every(index => fullIds.has(index));
  const base = documents['pen-full'].initial.mesh.positions, full = documents['pen-full'].final.mesh.positions, quarter = documents['pen-quarter'].final.mesh.positions;
  let maxError = 0; const ratios = [];
  for (let index = 0; index < base.length; index++) {
    const fullDelta = full[index].map((coordinate, channel) => coordinate - base[index][channel]);
    const quarterDelta = quarter[index].map((coordinate, channel) => coordinate - base[index][channel]);
    maxError = Math.max(maxError, ...quarterDelta.map((coordinate, channel) => Math.abs(coordinate - fullDelta[channel] * 0.25)));
    const denominator = Math.hypot(...fullDelta);
    if (denominator > 1e-5) ratios.push(Math.hypot(...quarterDelta) / denominator);
  }
  ratios.sort((a, b) => a - b); const median = ratios[Math.floor(ratios.length / 2)];
  result.quarter_pressure_response = { measured_original_vertices: ratios.length, max_absolute_position_error: maxError, median_displacement_ratio: median, expected_ratio: 0.25 };
  result.comparisons.quarter_pressure_matches_scaled_full_displacement = ratios.length > 0 && maxError <= 3e-7 && Math.abs(median - 0.25) <= 0.01;
  // Match independently generated source vertex pairs. Every old ID survives
  // adaptive local splitting, so new topology cannot conceal an asymmetric dab.
  const symmetricSource = documents['alpha-asymmetric'].initial.mesh.positions;
  const stamped = documents['alpha-asymmetric'].final.mesh.positions;
  const lookup = new Map(symmetricSource.map((point, index) => [JSON.stringify(point), index]));
  let mirrorPairs = 0, maxMirrorError = 0;
  for (let index = 0; index < symmetricSource.length; index++) {
    const [x, y, z] = symmetricSource[index];
    if (x <= 0) continue;
    const other = lookup.get(JSON.stringify([-x, y, z]));
    if (other === undefined) continue;
    mirrorPairs++;
    maxMirrorError = Math.max(maxMirrorError, Math.abs(stamped[index][0] + stamped[other][0]), Math.abs(stamped[index][1] - stamped[other][1]), Math.abs(stamped[index][2] - stamped[other][2]));
  }
  result.asymmetric_alpha_symmetry = { measured_original_vertex_pairs: mirrorPairs, max_mirrored_coordinate_error: maxMirrorError };
  result.comparisons.asymmetric_alpha_preserves_mirrored_world_footprint = mirrorPairs > 0 && maxMirrorError <= 1e-7;
  if (!Object.values(result.comparisons).every(Boolean)) throw new Error('Brush input comparison gate failed');
  if (result.exceptions.length || result.console_errors.length) throw new Error('JavaScript exception or console error');
  result.status = 'passed pressure and raster-alpha browser checks';
  result.limitations = ['Synthetic trusted CDP pen events; physical pen/tablet and native GUI pressure are unverified.', 'Nine stationary fresh-sphere samples exercise pressure capture, pressure-radius/rotation controls, bounded PNG upload, actual rendered geometry, save, and alpha undo/redo; long pressure ramps and arbitrary device event rates remain separate gates.', 'Raster alpha is an 8-bit grayscale scalar applied within the existing radial brush footprint; no UV/textures/vector-displacement compatibility or ZBrush response equivalence.'];
  }
} catch (error) {
  result.status = 'failed pressure and raster-alpha browser checks'; result.failure = String(error); process.exitCode = 1;
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
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json'), wasm: result.wasm, comparisons: result.comparisons, quarter_pressure_response: result.quarter_pressure_response, cases: result.cases.map(trial => ({ name: trial.name, changed_original_vertices: trial.changed_original_vertices, history: trial.history })) }));
}
