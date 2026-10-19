// Owned-Chrome, ordinary viewport check for the query-gated release protocol.
// This is a scoped diagnostic, not a SculptGL/ZBrush parity benchmark.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const out = path.resolve('.artifacts/source-worker-release-endpoint', randomUUID());
await mkdir(out, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
// Independent closed UV sphere, with explicit raw unprotected Source U masks.
const latitudes = 128, longitudes = 320;
const positions = [[0, 1, 0]], triangles = [];
for (let ring = 1; ring < latitudes; ring++) {
  const theta = Math.PI * ring / latitudes;
  for (let longitude = 0; longitude < longitudes; longitude++) {
    const phi = 2 * Math.PI * longitude / longitudes;
    positions.push([Math.sin(theta) * Math.cos(phi), Math.cos(theta), Math.sin(theta) * Math.sin(phi)]);
  }
}
const bottom = positions.push([0, -1, 0]) - 1;
const row = (ring, longitude) => 1 + (ring - 1) * longitudes + (longitude % longitudes);
for (let longitude = 0; longitude < longitudes; longitude++) {
  triangles.push([0, row(1, longitude + 1), row(1, longitude)]);
  for (let ring = 1; ring < latitudes - 1; ring++) {
    const a = row(ring, longitude), b = row(ring, longitude + 1);
    const c = row(ring + 1, longitude), d = row(ring + 1, longitude + 1);
    triangles.push([a, b, c], [b, d, c]);
  }
  triangles.push([bottom, row(latitudes - 1, longitude), row(latitudes - 1, longitude + 1)]);
}
const fixture = { version: 7, name: 'Independent endpoint sphere', mesh: {
  positions, normals: positions, triangles, mask_convention: 'unprotected_f32_v1',
  masks: Array(positions.length).fill(1), colors: Array.from({length:positions.length}, () => [0.7,0.4,0.2]),
} };
const fixtureBytes = Buffer.from(JSON.stringify(fixture));
const fixturePath = path.join(out, 'endpoint-sphere.obrush');
await writeFile(fixturePath, fixtureBytes);
const result = { scope: 'query-gated ordinary Source Worker pointer release only',
  fixture: { path: fixturePath, sha256: sha(fixtureBytes), vertices: positions.length,
    triangles: triangles.length, latitudes, longitudes },
  web_wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))),
  input_js_sha256: sha(await readFile(path.join(web, 'source-worker-held-input.js'))),
  status: 'running', out, checks: {}, strokes: [], errors: [] };
let server, chrome, socket, session, next = 0;
const waiting = new Map();
function send(method, params = {}, target = session) {
  const id = ++next;
  const promise = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
  return promise;
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn, label) {
  for (let i = 0; i < 200; i++) { const value = await fn(); if (value) return value; await pause(100); }
  throw Error(`Timed out: ${label}`);
}
async function pointer(type, x, y, buttons) {
  await send('Input.dispatchMouseEvent', { type, x, y, button: type === 'mouseMoved' ? 'none' : 'left',
    buttons, clickCount: type === 'mouseMoved' ? 0 : 1, pointerType: 'mouse' });
}
async function key(letter, modifiers = 2) {
  const modifierKey = modifiers === 4 ? 'Meta' : 'Control';
  const modifierCode = modifiers === 4 ? 'MetaLeft' : 'ControlLeft';
  const modifierVirtual = modifiers === 4 ? 91 : 17;
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: modifierKey, code: modifierCode,
    windowsVirtualKeyCode: modifierVirtual, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: letter,
    code: `Key${letter.toUpperCase()}`, windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: letter,
    code: `Key${letter.toUpperCase()}`, windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: modifierKey, code: modifierCode,
    windowsVirtualKeyCode: modifierVirtual, modifiers: 0 });
  await pause(120);
}
async function save(name) {
  await evaluate('window.__releaseSavedPromise=null');
  await click(344, 26);
  const saved = await until(() => evaluate('window.__releaseSavedPromise'), `public Save ${name}`);
  return { ...saved, name };
}
async function click(x, y) {
  await pointer('mouseMoved', x, y, 0);
  await pointer('mousePressed', x, y, 1);
  await pause(40);
  await pointer('mouseReleased', x, y, 0);
  await pause(160);
}
async function control(key, panel) {
  for (let attempt = 0; attempt < 25; attempt++) {
    const rect = await evaluate(`window.obrushBrushMetrics().${panel}[${JSON.stringify(key)}]`);
    if (!Array.isArray(rect)) throw Error(`Missing control ${key}`);
    if (rect[1] > 75 && rect[3] < 920) return [(rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2];
    const x = panel === 'layer_controls' ? 1500 : 120;
    await pointer('mouseMoved', x, 700, 0);
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y: 700, deltaX: 0,
      deltaY: rect[1] > 920 ? 350 : -350 });
    await pause(100);
  }
  throw Error(`Could not scroll to ${key}`);
}
async function stroke(start, end) {
  const before = await evaluate('window.obrushSourceWorkerTrialStatus()');
  await pointer('mouseMoved', ...start, 0);
  await pointer('mousePressed', ...start, 1);
  for (let step = 1; step <= 8; step++) {
    await pointer('mouseMoved', start[0] + (end[0] - start[0]) * step / 10,
      start[1] + (end[1] - start[1]) * step / 10, 1);
  }
  const releaseAt = performance.now();
  await pointer('mouseReleased', ...end, 0);
  let after;
  try {
    after = await until(async () => {
      const value = await evaluate('window.obrushSourceWorkerTrialStatus()');
      return value.commits > before.commits || value.rejections > before.rejections ? value : null;
    }, 'Worker stroke completion or rejection');
  } catch (error) {
    result.strokes.push({ start, end, before,
      after: await evaluate('window.obrushSourceWorkerTrialStatus()'),
      trace: await evaluate('window.__releaseTrace'),
      notice: await evaluate("document.getElementById('notice').textContent") });
    throw error;
  }
  const trace = await evaluate('window.__releaseTrace');
  const item = { start, end, before, after, trace, release_to_observed_ack_ms: performance.now() - releaseAt,
    notice: await evaluate("document.getElementById('notice').textContent") };
  result.strokes.push(item);
  return item;
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const file = path.resolve(web, '.' + (pathname === '/' ? '/index.html' : pathname));
      if (!file.startsWith(web + path.sep)) throw Error('Outside web root');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript',
        '.html': 'text/html' })[path.extname(file)] || 'application/octet-stream');
      response.end(await readFile(file));
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`,
    '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking',
    '--window-size=1600,1000', 'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', data => { stderr += data; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (!message.id) return;
    const pending = waiting.get(message.id);
    if (!pending) return;
    waiting.delete(message.id); clearTimeout(pending.timer);
    message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable'); await send('DOM.enable');
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `(() => {
    window.__releaseTrace = [];
    for (const type of ['pointerdown','pointermove','pointerup','pointercancel'])
      window.addEventListener(type, event => window.__releaseTrace.push({type,
        x:event.clientX,y:event.clientY,pressure:event.pressure,
        buttons:event.buttons,trusted:event.isTrusted}), true);
  })();` });
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/?obrushprofile=sculptgl&source-worker-held` });
  await until(() => evaluate("document.querySelector('[data-worker-trial-status]')?.dataset.workerTrialStatus"), 'Worker trial button');
  result.startup = await evaluate('({brush:window.obrushBrushMetrics(),trial:window.obrushSourceWorkerTrialStatus()})');
  const documentNode = await send('DOM.getDocument');
  const input = await send('DOM.querySelector', { nodeId: documentNode.root.nodeId, selector: '#import-file' });
  await send('DOM.setFileInputFiles', { nodeId: input.nodeId, files: [fixturePath] });
  await until(async () => (await evaluate('window.obrushBrushMetrics().vertices')) === positions.length,
    'independent sphere import');
  // Prepare the admitted brush exclusively through normal visible controls.
  await click(...await control('source_geometry.mode.source', 'morph_controls'));
  await click(...await control('settings.symmetry', 'morph_controls'));
  await click(...await control('viewport.shading', 'layer_controls'));
  result.controls_after_dropdown = await evaluate(`(() => { const m=window.obrushBrushMetrics();
    return {mode:m.source_geometry.per_tool[0].mode,symmetry_x:m.symmetry_x,
      shading:m.viewport_shading,controls:Object.fromEntries(Object.entries(m.layer_controls)
        .filter(([key])=>key.startsWith('viewport.shading')))}; })()`);
  await click(...await control('viewport.shading.studio_clay', 'layer_controls'));
  result.prepared = await evaluate('({kind:window.obrushBrushMetrics().kind,symmetry_x:window.obrushBrushMetrics().symmetry_x,mode:window.obrushBrushMetrics().source_geometry.per_tool[0].mode})');
  await evaluate("document.querySelector('[data-worker-trial-status]').click()");
  result.trial_enabled = await evaluate('window.obrushSourceWorkerTrialStatus().enabled');
  assert.equal(result.trial_enabled, true);
  await evaluate(`window.obrushDownload=(filename,contents)=>{
    const bytes=new TextEncoder().encode(contents);
    window.__releaseSavedPromise=crypto.subtle.digest('SHA-256',bytes).then(digest=>({
      filename,bytes:bytes.length,sha256:Array.from(new Uint8Array(digest),
        byte=>byte.toString(16).padStart(2,'0')).join('')
    }));
  }`);
  result.before = await save('before');
  await stroke([820, 450], [850, 460]);
  if (result.strokes[0].after.commits === 1) {
    result.after_first = await save('after-first');
    await stroke([785, 440], [815, 445]);
  }
  if (result.strokes[1]?.after.commits === 2) {
    result.after_second = await save('after-second');
    const count = (await evaluate('window.obrushSourceWorkerTrialStatus()')).sentDabs;
    await pause(500);
    result.quiet_tail = await evaluate('window.obrushSourceWorkerTrialStatus()');
    result.stable = await save('stable');
    await key('z');
    result.undo = await save('undo');
    await key('y');
    result.redo = await save('redo');
    result.checks.no_late_dabs = result.quiet_tail.sentDabs === count &&
      result.stable.sha256 === result.after_second.sha256;
    result.checks.public_history_exact = result.before.sha256 !== result.after_first.sha256 &&
      result.after_first.sha256 !== result.after_second.sha256 &&
      result.undo.sha256 === result.after_first.sha256 &&
      result.redo.sha256 === result.after_second.sha256;
  }
  result.checks.trusted_pointer_releases = result.strokes.every(item =>
    item.trace.filter(event => event.type === 'pointerup').every(event => event.trusted));
  result.checks.endpoint_not_dropped = result.strokes.every(item =>
    item.after.lastEndpointDropped === false &&
    (item.after.lastEndpointApplied || item.after.lastEndpointAlreadyCovered));
  result.checks.final_trusted_screen_in_status = result.strokes.every(item => {
    const release = item.trace.filter(event => event.type === 'pointerup').at(-1);
    return release && JSON.stringify(item.after.lastEndpointScreen) ===
      JSON.stringify([release.x, release.y]) && item.after.lastEndpointPressure === 0.5;
  });
  result.checks.one_commit_per_successful_stroke = result.strokes.every((item, index) =>
    item.after.commits === index + 1);
  result.checks.warm_second_stroke = result.strokes.length === 2 &&
    result.strokes[0].after.lastSeedWarm === false &&
    result.strokes[1].after.lastSeedWarm === true;
  result.checks.no_rejection = result.strokes.every(item => item.after.rejections === 0);
  result.status = Object.values(result.checks).every(Boolean) ? 'passed_scoped' : 'failed_scoped';
} catch (error) {
  result.errors.push(String(error?.stack || error)); result.status = 'failed';
} finally {
  try { socket?.close(); } catch {}
  try { chrome?.kill('SIGTERM'); } catch {}
  try { server?.close(); } catch {}
  const bytes = Buffer.from(JSON.stringify(result, null, 2));
  await writeFile(path.join(out, 'result.json'), bytes);
  console.log(JSON.stringify({ status: result.status, out, result_sha256: sha(bytes),
    checks: result.checks, errors: result.errors, strokes: result.strokes.map(item => ({
      after: item.after, notice: item.notice, release_to_observed_ack_ms: item.release_to_observed_ack_ms,
    })) }, null, 2));
}
if (result.status !== 'passed_scoped') process.exitCode = 1;
