// Scoped normal-browser Source Paint All control and public project check.
// Usage: node scripts/check-sculptgl-paint-all-browser.mjs [web/dist]
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] ?? 'web/dist');
const out = path.resolve('.artifacts/sculptgl-paint-all-browser', randomUUID());
const downloads = path.join(out, 'downloads');
await mkdir(downloads, { recursive: true });
const sha = data => createHash('sha256').update(data).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = {
  version: 7, name: 'Independent masked Paint All tetra',
  mesh: {
    positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
    normals: [[0, 0, -1], [0, 0, -1], [0, 0, -1], [0, 0, -1]],
    triangles: [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    mask_convention: 'unprotected_f32_v1',
    masks: [1, .5, .25, 0],
    colors: [[0, 0, 0], [0, .25, .5], [.5, .5, .5], [.75, .75, .75]],
    materials: [[.25, 0], [.5, .25], [.75, .5], [1, .75]],
  },
};
const fixtureBytes = Buffer.from(JSON.stringify(fixture));
const fixturePath = path.join(out, 'paint-all-fixture.obrush');
await writeFile(fixturePath, fixtureBytes);
const result = { status: 'running', fixture_sha256: sha(fixtureBytes), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))), actions: [], errors: [] };
let server, chrome, socket, session, seq = 0;
const waiting = new Map(), downloadsSeen = [];
function send(method, params = {}, target = session) {
  const id = ++seq;
  const pending = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error('CDP timeout: ' + method)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
  return pending;
}
async function evaluate(expression) {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
}
async function until(fn, label) {
  for (let i = 0; i < 150; i++) { const value = await fn(); if (value) return value; await pause(100); }
  throw Error('Timed out: ' + label);
}
const metrics = () => evaluate('window.obrushBrushMetrics?.()');
async function click(rect) {
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
  await pause(45);
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  await pause(55);
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
  await pause(180);
}
async function control(key) {
  for (let i = 0; i < 30; i++) {
    const state = await metrics();
    const rect = state?.morph_controls?.[key];
    if (!rect) {
      await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 150, y: 720, deltaX: 0, deltaY: 350 });
      await pause(120);
      continue;
    }
    const y = (rect[1] + rect[3]) / 2;
    if (y > 70 && y < 940) return rect;
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 150, y: 720, deltaX: 0, deltaY: y >= 940 ? 350 : -350 });
    await pause(120);
  }
  throw Error('Control outside visible inspector: ' + key);
}
async function shortcut(key, modifiers = 0) {
  const code = 'Key' + key.toUpperCase();
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await pause(160);
}
async function focusViewport() {
  const rect = (await metrics()).camera.viewport;
  await click([rect[2] - 35, rect[3] - 35, rect[2] - 15, rect[3] - 15]);
}
try {
  server = createServer(async (request, response) => {
    try {
      const relative = new URL(request.url, 'http://127.0.0.1').pathname;
      const file = path.resolve(web, '.' + (relative === '/' ? '/index.html' : relative));
      if (!file.startsWith(web + path.sep)) throw Error('outside web');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html', '.css': 'text/css' })[path.extname(file)] ?? 'application/octet-stream');
      response.end(await readFile(file));
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0',
    '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', data => { stderr += data; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const response = JSON.parse(String(event.data));
    if (response.id) {
      const pending = waiting.get(response.id);
      if (!pending) return;
      waiting.delete(response.id); clearTimeout(pending.timer);
      response.error ? pending.reject(Error(JSON.stringify(response.error))) : pending.resolve(response.result);
    } else if (response.method === 'Browser.downloadProgress') downloadsSeen.push(response.params);
    else if (response.method === 'Runtime.exceptionThrown') result.errors.push(response.params.exceptionDetails.text);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(domain + '.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloads, eventsEnabled: true }, null);
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
  await until(async () => (await metrics())?.morph_controls && await evaluate('!document.getElementById("loading")'), 'OBRush startup');
  const root = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#import-file' })).nodeId;
  assert(node, 'Normal import input missing');
  await send('DOM.setFileInputFiles', { nodeId: node, files: [fixturePath] });
  await until(async () => (await metrics())?.vertices === 4, 'fixture import');
  result.actions.push('normal project import');
  await focusViewport();
  await shortcut('9');
  await until(async () => (await metrics())?.kind === 'Paint', 'Paint brush selection');
  const sourceRect = await control('source_paint.mode.source');
  result.source_mode_before = (await metrics()).source_paint;
  result.source_mode_rect = sourceRect;
  await click(sourceRect);
  result.source_mode_after_click = (await metrics()).source_paint;
  await until(async () => (await metrics())?.source_paint?.mode === 'SourceOrdinary', 'Source Paint mode');
  result.actions.push('Paint brush and Source mode controls');
  await click(await control('source_paint.paint_all'));
  await until(async () => (await metrics())?.paint_fill?.changed_vertices === 3, 'Source Paint All completion');
  const report = (await metrics()).paint_fill;
  result.actions.push('normal Paint All control');
  // Save through the top-bar control; inspect the public project rather than
  // mutating or reading private app state.
  await click([303, 13, 387, 39]);
  const saved = await until(async () => {
    const done = downloadsSeen.find(item => item.state === 'completed');
    if (!done) return null;
    try { return JSON.parse(await readFile(path.join(downloads, done.guid), 'utf8')); } catch { return null; }
  }, 'project Save download');
  const mesh = saved.mesh;
  const sourceColor = [1, Math.fround(.766), Math.fround(.336)];
  const original = fixture.mesh;
  const blend = (old, target, weight) => weight === 0 ? old : weight === 1 ? target : Math.fround(old * (1 - weight) + target * weight);
  const weights = [1, .5, .25, 0];
  const expectedColors = original.colors.map((row, i) => row.map((old, c) => blend(old, sourceColor[c], weights[i])));
  const expectedMaterials = original.materials.map((row, i) => [blend(row[0], Math.fround(.3), weights[i]), blend(row[1], Math.fround(.95), weights[i])]);
  const maxError = (actual, expected) => Math.max(...actual.flatMap((row, i) => row.map((value, c) => Math.abs(value - expected[i][c]))));
  result.max_rgb_abs_error = maxError(mesh.colors, expectedColors);
  result.max_material_abs_error = maxError(mesh.materials, expectedMaterials);
  assert(result.max_rgb_abs_error <= 1e-7 && result.max_material_abs_error <= 1e-7, 'Source target or soft-mask response differs');
  assert.deepEqual(mesh.positions, original.positions);
  assert.deepEqual(mesh.triangles, original.triangles);
  assert.deepEqual(mesh.masks, original.masks);
  assert.equal(report.changed_vertices, 3);
  assert.equal(report.changed_rgb_vertices, 3);
  assert.equal(report.changed_material_vertices, 3);
  assert.equal(report.materialized, false);
  assert.equal(report.subtool_id, 1);
  assert.equal(result.errors.length, 0);
  result.status = 'passed scoped normal-browser Source Paint All control, masked public channels and fixed geometry';
  result.report = report;
  result.saved_project_sha256 = sha(Buffer.from(JSON.stringify(saved)));
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error.stack ?? error));
  try { result.failure_metrics = await metrics(); } catch {}
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  socket?.close(); chrome?.kill('SIGTERM'); server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
