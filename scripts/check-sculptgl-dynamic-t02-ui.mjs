// T02: owned normal-browser OBrush UI gate for adaptive-topology preparation.
// Usage: node scripts/check-sculptgl-dynamic-t02-ui.mjs /absolute/path/to/web/dist
import { spawn, execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const fixture = path.resolve('crates/obrush-core/tests/fixtures/independent-dynamic-quad.obj');
const out = path.resolve('.artifacts/sculptgl-dynamic-t02', randomUUID());
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition, why) => { if (!condition) throw Error(why); };
const fixtureBytes = await readFile(fixture);
assert(sha(fixtureBytes) === 'c18fb0d1445a0c98ebae6f86b0dd256e351b9abbe8779ca9d00a748798dafffd', 'Independent fixture changed');
await mkdir(out, { recursive: true });
const result = {
  scope: 'T02 normal OBrush browser conversion UI; reference comparison remains open',
  build_commit: process.argv.find(arg => arg.startsWith('--build-commit='))?.slice('--build-commit='.length)
    || execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
  build_wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))),
  fixture: { path: fixture, bytes: fixtureBytes.length, sha256: sha(fixtureBytes) },
  status: 'running', controls: [], saves: [], screenshots: [], file_choosers: [], errors: [], exceptions: [],
};
let server, chrome, socket, session, sequence = 0;
const pending = new Map();
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout ${method}`)); }, 45000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
  return reply.result.value;
};
const until = async (read, reason, attempts = 150) => {
  let last;
  for (let i = 0; i < attempts; i++) {
    try { const value = await read(); if (value) return value; } catch (error) { last = error; }
    await pause(100);
  }
  throw Error(`Timed out ${reason}${last ? `: ${last}` : ''}`);
};
async function key(letter, modifiers = 0) {
  const code = `Key${letter.toUpperCase()}`;
  const windowsVirtualKeyCode = letter.toUpperCase().charCodeAt(0);
  for (const type of ['rawKeyDown', 'keyUp'])
    await send('Input.dispatchKeyEvent', { type, key: letter.toLowerCase(), code, windowsVirtualKeyCode, modifiers });
}
async function chord(letter, shift = false) {
  const modifiers = 4 | (shift ? 8 : 0);
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await key(letter, modifiers);
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
  await pause(160);
}
async function click(rect, name) {
  assert(Array.isArray(rect) && rect.length === 4, `Missing rectangle for ${name}`);
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  assert(x >= 0 && x < 1600 && y >= 0 && y < 1000, `${name} offscreen: ${rect}`);
  for (const type of ['mousePressed', 'mouseReleased'])
    await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  result.controls.push({ name, rect, point: [x, y] });
  await pause(180);
}
const metrics = () => evaluate('window.obrushBrushMetrics?.()');
async function visibleControl(name) {
  for (let i = 0; i < 30; i++) {
    const rect = (await metrics())?.layer_controls?.[name];
    if (rect && rect[0] >= 0 && rect[2] <= 1600 && rect[1] >= 60 && rect[3] <= 980) return rect;
    const x = Math.min(1540, Math.max(50, rect ? (rect[0] + rect[2]) / 2 : 1450));
    const deltaY = rect?.[3] > 980 ? 370 : -370;
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y: 740, button: 'none' });
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y: 740, deltaX: 0, deltaY });
    await pause(120);
  }
  throw Error(`UI control missing or offscreen: ${name} ${JSON.stringify((await metrics())?.layer_controls?.[name])}`);
}
async function clickControl(name) { await click(await visibleControl(name), name); }
async function fileInput(file) {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#import-file' })).nodeId;
  assert(nodeId, 'Normal File Open input missing');
  await send('DOM.setFileInputFiles', { nodeId, files: [file] });
}
async function open(file, expectedVertices, requireUv = true) {
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const choosers = result.file_choosers.length;
  await chord('o');
  await until(() => result.file_choosers.length > choosers, 'normal File Open chooser');
  await fileInput(file);
  await until(async () => {
    const state = await metrics();
    return state?.vertices === expectedVertices
      && (requireUv ? state?.surface_attributes?.uv_coordinates > 0 : state?.surface_attributes?.uv_coordinates === 0)
      && state;
  }, `File Open ${path.basename(file)}`);
}
async function save(name) {
  const before = await evaluate('window.__t02Captures.length');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  await chord('s');
  await until(async () => (await evaluate('window.__t02Captures.length')) > before, `Save ${name}`);
  const capture = await evaluate(`window.__t02Captures[${before}]`);
  assert(capture?.contents && capture?.name?.endsWith('.obrush'), `Save ${name} did not produce .obrush`);
  const file = path.join(out, `${name}.obrush`);
  await writeFile(file, capture.contents);
  const row = { name, file, bytes: Buffer.byteLength(capture.contents), sha256: sha(capture.contents), project: JSON.parse(capture.contents) };
  result.saves.push({ name, file, bytes: row.bytes, sha256: row.sha256 });
  return row;
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  const file = path.join(out, `${name}.png`);
  await writeFile(file, bytes);
  result.screenshots.push({ name, file, bytes: bytes.length, sha256: sha(bytes) });
}
function exactVisible(before, after) {
  const fields = ['positions', 'triangles', 'colors', 'masks', 'materials'];
  return Object.fromEntries(fields.map(field => [field, JSON.stringify(before.mesh[field]) === JSON.stringify(after.mesh[field])]));
}

try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const file = path.resolve(web, '.' + (pathname === '/' ? '/index.html' : pathname));
      if (!file.startsWith(web + path.sep)) throw Error('Outside web tree');
      const bytes = await readFile(file);
      response.setHeader('Cache-Control', 'no-store');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const url = `http://127.0.0.1:${server.address().port}/`;
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0',
    '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.method === 'Page.fileChooserOpened') result.file_choosers.push(message.params);
    if (message.method === 'Runtime.exceptionThrown') result.exceptions.push(message.params.exceptionDetails?.text || 'browser exception');
    if (!message.id) return;
    const row = pending.get(message.id);
    if (!row) return;
    pending.delete(message.id); clearTimeout(row.timer);
    message.error ? row.reject(Error(JSON.stringify(message.error))) : row.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url });
  await until(async () => (await metrics())?.vertices, 'OBRush startup');
  await evaluate('window.__t02Captures=[];window.obrushDownload=(name,contents)=>window.__t02Captures.push({name,contents});');
  await open(fixture, 4);
  assert((await metrics()).surface_attributes.original_faces === 1, 'Imported quad face record missing');
  await clickControl('object.subdivision.mode');
  await clickControl('object.subdivision.static_smooth');
  await clickControl('object.add_level');
  await until(async () => (await metrics())?.subdivision_levels === 2, 'visible retained level');
  const before = await save('before');
  assert(before.project.mesh.surface?.uv?.coordinates?.length > 0, 'Before project lacks UVs');
  await clickControl('object.dynamic.header');
  const preAck = await metrics();
  assert(preAck.layer_controls_enabled?.['object.dynamic.prepare'] === false, 'Prepare enabled before acknowledgement');
  await visibleControl('object.dynamic.acknowledge');
  await screenshot('disclosure');
  await clickControl('object.dynamic.acknowledge');
  assert((await metrics()).layer_controls_enabled?.['object.dynamic.prepare'] === true, 'Prepare did not enable after acknowledgement');
  await clickControl('object.dynamic.prepare');
  await until(async () => {
    const state = await metrics();
    return state?.subdivision_levels === 1 && state?.surface_attributes?.original_faces === 0 && state;
  }, 'converted adaptive mesh');
  const convertedMetrics = await metrics();
  assert(convertedMetrics.surface_attributes.uv_coordinates === 0 && convertedMetrics.surface_attributes.adaptive_detail_supported === true, 'Converted mesh metadata/capability mismatch');
  assert(!convertedMetrics.layer_controls?.['object.dynamic.prepare'], 'Repeat conversion remains available after metadata removal');
  const after = await save('converted');
  const exact = exactVisible(before.project, after.project);
  assert(Object.values(exact).every(Boolean), `Visible mesh/channel words changed: ${JSON.stringify(exact)}`);
  assert(!after.project.multiresolution && !after.project.mesh.surface, 'Converted project retained levels/surface metadata');
  await screenshot('converted');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  await chord('z');
  const undo = await save('undo');
  assert(undo.sha256 === before.sha256, 'Undo project differs from before conversion');
  await chord('z', true);
  const redo = await save('redo');
  assert(redo.sha256 === after.sha256, 'Redo project differs from converted project');
  await open(after.file, after.project.mesh.positions.length, false);
  const reloaded = await save('reloaded');
  assert(reloaded.sha256 === after.sha256, 'Reload changed converted project');
  assert(result.exceptions.length === 0, `Browser exceptions: ${result.exceptions.join('; ')}`);
  result.exact_visible = exact;
  result.before = { vertices: before.project.mesh.positions.length, triangles: before.project.mesh.triangles.length, levels: 2,
    original_faces: before.project.mesh.surface.faces.length, uv_coordinates: before.project.mesh.surface.uv.coordinates.length };
  result.converted = { vertices: after.project.mesh.positions.length, triangles: after.project.mesh.triangles.length, levels: 1,
    original_faces: 0, uv_coordinates: 0, adaptive_detail_supported: convertedMetrics.surface_attributes.adaptive_detail_supported };
  result.status = 'passed';
} catch (error) {
  result.status = 'failed';
  result.errors.push(String(error?.stack || error));
  try { if (socket) result.last_metrics = await metrics(); } catch {}
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { socket?.close(); } catch {}
  try { chrome?.kill(); } catch {}
  if (server) await new Promise(resolve => server.close(resolve));
  console.log(JSON.stringify({ status: result.status, out, errors: result.errors, saves: result.saves, exact_visible: result.exact_visible }, null, 2));
  if (result.status !== 'passed') process.exitCode = 1;
}
