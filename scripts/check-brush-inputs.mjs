// CLI-only brush input checks. Owns a temporary loopback server, headless
// Chrome process/profile, and test tab; never attaches to user browser state.
// Usage: node scripts/check-brush-inputs.mjs [web/dist] [run-label]
//        [--pressure-only | --factors-only | --ramp-only | --burst-only]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { deflateSync } from 'node:zlib';

const web = path.resolve(process.argv[2] || 'web/dist');
const label = process.argv[3] || 'brush-inputs';
const pressureOnly = process.argv.includes('--pressure-only');
const factorOnly = process.argv.includes('--factors-only');
const rampOnly = process.argv.includes('--ramp-only');
const burstOnly = process.argv.includes('--burst-only');
if ([pressureOnly, factorOnly, rampOnly, burstOnly].filter(Boolean).length > 1) throw new Error('Select one brush input mode');
const viewportHeight = factorOnly || rampOnly || burstOnly ? 720 : 1000;
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
 const p=window.__brushCheck={events:[],keys:[],contexts:[],allocations:[],viewports:{},gl_errors:[],js_errors:[],raf_count:0,hold_raf:false,deferred_raf:[]};
 const raf=window.requestAnimationFrame;
 window.requestAnimationFrame=function(callback){return raf.call(this,time=>{if(p.hold_raf){p.deferred_raf.push(callback);return;}p.raf_count++;callback(time);});};
 p.releaseHeldRaf=()=>{p.hold_raf=false;const callbacks=p.deferred_raf.splice(0);for(const callback of callbacks){p.raf_count++;callback(performance.now());}};
 for(const type of ['keydown','keyup'])window.addEventListener(type,event=>p.keys.push({type,key:event.key,meta:event.metaKey,ctrl:event.ctrlKey,shift:event.shiftKey,trusted:event.isTrusted,target:event.target.id}),true);
 for(const type of ['pointermove','pointerdown','pointerup','pointercancel'])window.addEventListener(type,event=>p.events.push({type,pointer_type:event.pointerType,pressure:event.pressure,x:event.clientX,y:event.clientY,buttons:event.buttons,trusted:event.isTrusted,raf_count:p.raf_count,at:performance.now()}),true);
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
  await pointer('mouseMoved', 110, 430);
  await send('Input.synthesizeScrollGesture', { x: 110, y: 430, xDistance: 0, yDistance: -1450, gestureSourceType: 'mouse', speed: 500 });
  await pause(500);
  await pointer('mouseMoved', 60, 382);
  await pointer('mousePressed', 60, 382, 'mouse', 0.5, true);
  await pointer('mouseReleased', 60, 382);
  await pause(100);
  const request = await evaluate('window.obrushBrushMetrics().preferences.alpha_import_request');
  if (!request?.id) throw new Error(`Visible Import alpha did not open a request: ${JSON.stringify(request)}`);
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
const setFactorControls = async (intensity, radius) => {
  // This path deliberately uses the real egui inspector in the ordinary
  // browser, then confirms the host captured the requested values.
  await pointer('mouseMoved', 110, 430);
  await pause(100);
  await send('Input.synthesizeScrollGesture', { x: 110, y: 430, xDistance: 0, yDistance: -850, gestureSourceType: 'mouse', speed: 400 });
  await pause(500);
  const before = await screenshot('factor-controls-before');
  await pointer('mouseMoved', 21, 473);
  await pointer('mousePressed', 21, 473, 'mouse', 0.5, true);
  await pointer('mouseReleased', 21, 473);
  await pause(150);
  let actual = await evaluate('window.obrushBrushMetrics().pressure_profile');
  if (actual.kind !== 'sculptgl_factors') throw new Error(`Factor toggle did not switch profile: ${JSON.stringify({before,actual})}`);
  for (const [factor, y] of [[intensity, 523], [radius, 573]]) {
    const x = 20 + 160 * factor;
    await pointer('mouseMoved', x, y);
    await pointer('mousePressed', x, y, 'mouse', 0.5, true);
    await pointer('mouseReleased', x, y);
    await pause(90);
  }
  actual = await evaluate('window.obrushBrushMetrics().pressure_profile');
  if (Math.abs(actual.intensity - intensity) > 0.03 || Math.abs(actual.radius - radius) > 0.03) {
    const after = await screenshot('factor-controls-mismatch');
    throw new Error(`Factor sliders did not capture requested values: ${JSON.stringify({intensity,radius,actual,before,after})}`);
  }
  return actual;
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
  await send('Emulation.setDeviceMetricsOverride', { width: factorOnly || rampOnly || burstOnly ? 1280 : 1600, height: viewportHeight, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Target.activateTarget', { targetId: target.targetId }, null);
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const wasm = await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'));
  const served = Buffer.from(await (await fetch(url + 'pkg/obrush_app_bg.wasm')).arrayBuffer());
  result.wasm = { sha256: hash(wasm), served_sha256: hash(served), bytes: wasm.length };
  if (hash(wasm) !== hash(served)) throw new Error('Served WASM differs from local build');

  const documents = {};
  const cases = burstOnly ? [
    ['burst-low-high', 'pen', 0.25, null, null, [1, 0], 0.75],
    ['burst-near-high', 'pen', 0.25, null, null, [1, 0], 0.75],
  ] : rampOnly ? [
    ['ramp-constant-quarter', 'pen', 0.25, null, null, [1, 0], 0.25],
    ['ramp-quarter-to-three-quarter', 'pen', 0.25, null, null, [1, 0], 0.75],
    ['ramp-neutral-constant', 'pen', 0.25, null, null, [0, 0], 0.25],
    ['ramp-neutral-varying', 'pen', 0.25, null, null, [0, 0], 0.75],
  ] : factorOnly ? [
    ['factor-base-quarter', 'pen', 0.25, null, null, [0, 0]],
    ['factor-radius-quarter', 'pen', 0.25, null, null, [0, 1]],
    ['factor-intensity-quarter', 'pen', 0.25, null, null, [1, 0]],
    ['factor-mouse-neutral', 'mouse', 0.5, null, null, [1, 1]],
  ] : [
    ['pen-zero', 'pen', 0, null], ['pen-quarter', 'pen', 0.25, null], ['pen-full', 'pen', 1, null],
    ['mouse-default', 'mouse', 0.5, null], ['alpha-black', 'pen', 1, 'black'], ['alpha-white', 'pen', 1, 'white'], ['alpha-asymmetric', 'pen', 1, 'asymmetric'],
    ['pen-radius-half', 'pen', 0.5, null, 'pressure-radius'], ['alpha-rotated', 'pen', 1, 'asymmetric', 'rotation'],
  ];
  for (const [name, pointerType, pressure, alpha, uiControl, factors, movePressure] of pressureOnly ? cases.slice(0, 4) : cases) {
    await fresh();
    const trial = { name, pointer_type: pointerType, force: pressure, alpha, snapshots: [] }; result.cases.push(trial);
    trial.initial_settings = await evaluate('window.obrushBrushMetrics()');
    const initial = await save(`${name}-initial`); trial.snapshots.push(metadata(initial));
    if (factors) trial.factor_control = await setFactorControls(...factors);
    if (alpha) trial.alpha_import_settings = await loadAlpha(alpha);
    if (uiControl) {
      // The inspector is scrollable; use the visible control in the same
      // viewport as the owned Chrome screenshot instead of a stale position.
      if (uiControl === 'pressure-radius') {
        await pointer('mouseMoved', 110, 430);
        await send('Input.synthesizeScrollGesture', { x: 110, y: 430, xDistance: 0, yDistance: -850, gestureSourceType: 'mouse', speed: 400 });
        await pause(500);
      }
      const [controlX, controlY] = uiControl === 'pressure-radius' ? [22, 523] : [106, 470];
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
    const x = left + width / 2 - 85, y = viewportHeight - bottom - height / 2 - 30;
    trial.viewport = { left, bottom, width, height, pointer: [x, y] };
    await pointer('mouseMoved', x, y, pointerType, pressure, false); await pause(60);
    await pointer('mousePressed', x, y, pointerType, pressure, true); await pause(120);
    trial.settings_during_contact = await evaluate('window.obrushBrushMetrics()');
    if (burstOnly) {
      trial.canvas_css = await evaluate(`(()=>{const c=document.getElementById('obrush_canvas'),r=c.getBoundingClientRect();return {left:r.left,top:r.top,width:r.width,height:r.height,client_width:c.clientWidth,client_height:c.clientHeight}})()`);
      const words=trial.settings_during_contact.background_cursor?.pointer?.last_input?.logical_words;
      trial.egui_press_position = words?.map(word=>{const buffer=new ArrayBuffer(4),view=new DataView(buffer);view.setUint32(0,word,true);return view.getFloat32(0,true);});
      if (Math.abs(trial.canvas_css.width-trial.canvas_css.client_width)>=1 || Math.abs(trial.canvas_css.height-trial.canvas_css.client_height)>=1 || Math.abs(trial.egui_press_position?.[0]-x)>0.5 || Math.abs(trial.egui_press_position?.[1]-y)>0.5) throw new Error(`DOM-to-egui pen position mapping was not exact: ${name}`);
    }
    if (rampOnly || burstOnly) {
      trial.gpu_after_first_dab = await gpuDigest();
      trial.metrics_after_first_dab = await evaluate('window.obrushSculptMetrics()');
      if (burstOnly) {
        const firstPressure = name === 'burst-low-high' ? 0.25 : 0.70;
        trial.burst_dom_offset = await evaluate('window.__brushCheck.events.length');
        const requested = [[x + 50, firstPressure], [x + 100, 0.75]].map(([moveX, force]) => ({type:'mouseMoved',x:moveX,y,pointerType:'pen',force,button:'none',buttons:1}));
        result.input_trace.push(...requested.map(params=>({...params,at_epoch_ms:Date.now()})));
        trial.burst = requested;
        await evaluate('window.__brushCheck.hold_raf=true');
        try {
          await send('Input.dispatchMouseEvent',requested[0]);
          await pause(3);
          await send('Input.dispatchMouseEvent',requested[1]);
          trial.burst_dom_events = await evaluate(`window.__brushCheck.events.slice(${trial.burst_dom_offset}).filter(event=>event.pointer_type==='pen'&&event.type==='pointermove'&&event.y===${y}&&(event.x===${x + 50}||event.x===${x + 100}))`);
          if (trial.burst_dom_events.length !== 2 || trial.burst_dom_events.some(event=>!event.trusted)) throw new Error(`CDP burst was not delivered as two trusted DOM events: ${name}`);
          if (trial.burst_dom_events[0].raf_count !== trial.burst_dom_events[1].raf_count) throw new Error(`CDP burst crossed a browser animation frame: ${name}`);
        } finally {
          await evaluate('window.__brushCheck.releaseHeldRaf()');
        }
        await pause(200);
      } else {
        await pointer('mouseMoved', x + 100, y, pointerType, movePressure, true); await pause(200);
      }
      trial.settings_after_move = await evaluate('window.obrushBrushMetrics()');
      trial.metrics_after_move = await evaluate('window.obrushSculptMetrics()');
      trial.move_dom_pen_events = await evaluate(`window.__brushCheck.events.filter(event=>event.pointer_type==='pen'&&event.type==='pointermove'&&event.x===${x + 100}&&event.y===${y})`);
      if (!trial.move_dom_pen_events.some(event => (burstOnly || event.trusted) && Math.abs(event.pressure - movePressure) < 1e-6)) throw new Error(`Missing moving pressure event: ${name}`);
      if (Math.abs(trial.settings_after_move.pressure - movePressure) > 1e-6) throw new Error(`Moving contact pressure was not sampled: ${name}`);
      if (trial.metrics_after_move.processed_dabs <= trial.metrics_after_first_dab.processed_dabs) throw new Error(`Moving pressure did not produce another dab: ${name}`);
    }
    await pointer('mouseReleased', rampOnly || burstOnly ? x + 100 : x, y, pointerType, 0, false); await pause(350);
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
    if (name === 'ramp-quarter-to-three-quarter' || name === 'burst-low-high') {
      await chord('z'); const undo = await save(`${name}-undo`); trial.snapshots.push(metadata(undo));
      await chord('z', true); const redo = await save(`${name}-redo`); trial.snapshots.push(metadata(redo));
      trial.history = { undo_exact_initial: meshHash(undo.document) === meshHash(initial.document), redo_exact_final: meshHash(redo.document) === meshHash(final.document) };
      if (!Object.values(trial.history).every(Boolean)) throw new Error('Pressure stroke did not form an exact one-step history edit');
    }
    if (name === 'alpha-asymmetric' || name === 'alpha-rotated' || name === 'pen-radius-half') trial.screenshot = await screenshot(name);
    if (name === 'alpha-asymmetric') {
      await chord('z'); const undo = await save(`${name}-undo`); trial.snapshots.push(metadata(undo));
      await chord('z', true); const redo = await save(`${name}-redo`); trial.snapshots.push(metadata(redo));
      trial.history = { undo_exact_initial: meshHash(undo.document) === meshHash(initial.document), redo_exact_final: meshHash(redo.document) === meshHash(final.document) };
      if (!Object.values(trial.history).every(Boolean)) throw new Error('Alpha undo/redo does not restore every mesh field');
      await pointer('mouseMoved', 60, 382);
      await pointer('mousePressed', 60, 382, 'mouse', 0.5, true);
      await pointer('mouseReleased', 60, 382);
      await pause(100);
      if (!(await evaluate('window.obrushBrushMetrics().preferences.alpha_import_request?.id'))) throw new Error('Oversized alpha test did not start a new public import request');
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
  if (burstOnly) {
    const first = result.cases.find(trial => trial.name === 'burst-low-high');
    const second = result.cases.find(trial => trial.name === 'burst-near-high');
    result.burst_response = {
      same_initial_mesh: meshHash(documents[first.name].initial) === meshHash(documents[second.name].initial),
      same_first_dab_gpu: first.gpu_after_first_dab.sha256 === second.gpu_after_first_dab.sha256,
      first_case_final_mesh: meshHash(documents[first.name].final),
      second_case_final_mesh: meshHash(documents[second.name].final),
      same_final_mesh: meshHash(documents[first.name].final) === meshHash(documents[second.name].final),
      first_case_dabs: first.metrics_after_move.processed_dabs,
      second_case_dabs: second.metrics_after_move.processed_dabs,
      samples: [first.burst, second.burst],
    };
    if (!result.burst_response.same_initial_mesh || !result.burst_response.same_first_dab_gpu) throw new Error('Burst cases did not share the exact first-dab baseline');
    if (first.metrics_after_move.processed_dabs <= first.metrics_after_first_dab.processed_dabs || second.metrics_after_move.processed_dabs <= second.metrics_after_first_dab.processed_dabs) throw new Error('Synchronous pointer burst produced no sculpt dabs');
    result.comparisons = {
      exact_initial_and_press: result.burst_response.same_initial_mesh && result.burst_response.same_first_dab_gpu,
      retained_first_pressure_changes_shape: !result.burst_response.same_final_mesh,
      all_queued_pen_samples_drained: result.cases.every(trial=>trial.metrics_after_move.browser_pen_moves_queued===0 && trial.metrics_after_move.browser_pen_moves_dropped===0),
      exact_one_stroke_history: Object.values(first.history).every(Boolean),
      exact_gpu_public_save: result.cases.every(trial=>trial.gpu_matches_saved_mesh),
    };
    if (!Object.values(result.comparisons).every(Boolean)) throw new Error('Same-frame pen pressure burst gate failed');
    result.status = 'passed same-frame pen pressure burst';
    result.limitations = ['The two CDP pen events are trusted browser inputs, but their arrival before the same RAF must be checked per case; physical tablets and native GUI behavior remain unverified.'];
  } else if (rampOnly) {
    const same = (a, b) => meshHash(a) === meshHash(b);
    const constant = result.cases.find(trial => trial.name === 'ramp-constant-quarter');
    const rising = result.cases.find(trial => trial.name === 'ramp-quarter-to-three-quarter');
    const neutralConstant = result.cases.find(trial => trial.name === 'ramp-neutral-constant');
    const neutralVarying = result.cases.find(trial => trial.name === 'ramp-neutral-varying');
    const constantPositions = documents[constant.name].final.mesh.positions;
    const risingPositions = documents[rising.name].final.mesh.positions;
    const originalVertices = documents[constant.name].initial.mesh.positions.length;
    let divergentOriginalVertices = 0;
    let maxOriginalPositionDifference = 0;
    for (let index = 0; index < originalVertices; index++) {
      let differs = false;
      for (let channel = 0; channel < 3; channel++) {
        const difference = Math.abs(constantPositions[index][channel] - risingPositions[index][channel]);
        maxOriginalPositionDifference = Math.max(maxOriginalPositionDifference, difference);
        differs ||= difference > 0;
      }
      if (differs) divergentOriginalVertices++;
    }
    result.ramp_response = {
      constant_first_dab_gpu: constant.gpu_after_first_dab.sha256,
      rising_first_dab_gpu: rising.gpu_after_first_dab.sha256,
      constant_final_mesh: meshHash(documents[constant.name].final),
      rising_final_mesh: meshHash(documents[rising.name].final),
      constant_processed_dabs: constant.metrics_after_move.processed_dabs,
      rising_processed_dabs: rising.metrics_after_move.processed_dabs,
      divergent_original_vertices: divergentOriginalVertices,
      max_original_position_difference: maxOriginalPositionDifference,
      constant_final_vertices: constantPositions.length,
      rising_final_vertices: risingPositions.length,
    };
    result.comparisons = {
      same_initial_mesh: Object.values(documents).every(pair => same(pair.initial, documents[constant.name].initial)),
      same_first_dab_gpu_before_pressure_change: constant.gpu_after_first_dab.sha256 === rising.gpu_after_first_dab.sha256,
      changed_pressure_changes_factor_one_geometry: !same(documents[constant.name].final, documents[rising.name].final),
      changed_pressure_moves_original_vertices: divergentOriginalVertices > 0 && maxOriginalPositionDifference > 1e-4,
      changing_pressure_with_zero_factor_preserves_geometry: same(documents[neutralConstant.name].final, documents[neutralVarying.name].final),
      neutral_pair_first_dabs_identical: neutralConstant.gpu_after_first_dab.sha256 === neutralVarying.gpu_after_first_dab.sha256,
      one_released_stroke_and_exact_history: rising.metrics_after_release.released_strokes === 1 && Object.values(rising.history).every(Boolean),
    };
    if (!Object.values(result.comparisons).every(Boolean) || result.exceptions.length || result.console_errors.length) throw new Error('Pressure ramp browser comparison gate failed');
    result.status = 'passed pressure ramp browser checks';
    result.limitations = ['Two-contact synthetic pen drags at 0.25 then 0.25 or 0.75 on fresh spheres; physical tablet, other brush families, reference-paired geometry and arbitrary event rates remain open.'];
  } else if (factorOnly) {
    const initial = documents['factor-base-quarter'].initial.mesh.positions;
    const base = documents['factor-base-quarter'].final.mesh.positions;
    const narrow = documents['factor-radius-quarter'].final.mesh.positions;
    const half = documents['factor-intensity-quarter'].final.mesh.positions;
    const changed = positions => initial.flatMap((point, index) => point.some((value, channel) => value !== positions[index][channel]) ? [index] : []);
    const baseIds = new Set(changed(base));
    const narrowIds = changed(narrow);
    const halfIds = changed(half);
    let maxIntensityError = 0;
    for (let index = 0; index < initial.length; index++) {
      for (let channel = 0; channel < 3; channel++) {
        const expected = (base[index][channel] - initial[index][channel]) * 0.5;
        const measured = half[index][channel] - initial[index][channel];
        maxIntensityError = Math.max(maxIntensityError, Math.abs(measured - expected));
      }
    }
    result.factor_response = {
      base_changed_original_vertices: baseIds.size,
      narrowed_changed_original_vertices: narrowIds.length,
      half_intensity_changed_original_vertices: halfIds.length,
      max_half_intensity_position_error: maxIntensityError,
      allowed_position_error: 3e-7,
    };
    result.comparisons = {
      default_factors_ignore_quarter_pressure: meshHash(documents['factor-base-quarter'].final) === meshHash(documents['factor-mouse-neutral'].final),
      radius_factor_one_halves_footprint: narrowIds.length > 0 && narrowIds.length < baseIds.size / 2 && narrowIds.every(index => baseIds.has(index)),
      intensity_factor_one_halves_displacement: halfIds.length === baseIds.size && halfIds.every(index => baseIds.has(index)) && maxIntensityError <= 3e-7,
      all_cases_start_from_same_mesh: Object.values(documents).every(pair => meshHash(pair.initial) === meshHash(documents['factor-base-quarter'].initial)),
    };
    result.factor_cases = Object.fromEntries(Object.entries(documents).map(([name, pair]) => [name, {
      initial: meshHash(pair.initial), final: meshHash(pair.final),
    }]));
    if (!Object.values(result.comparisons).every(Boolean) || result.exceptions.length || result.console_errors.length) throw new Error('Factor browser comparison gate failed');
    result.status = 'passed factor browser checks';
    result.limitations = ['Four stationary synthetic CDP contacts on fresh spheres verify the visible factor controls, saved geometry, and neutral mouse response; physical tablets, pressure ramps, broad brush and mesh coverage, and a pinned SculptGL UI pair remain open.'];
  } else if (pressureOnly) {
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
  process.exit(process.exitCode ?? 0);
}
