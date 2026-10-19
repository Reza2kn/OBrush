// Physical Shift-drag Smooth on an independent dense SGL sphere.
// Usage: node scripts/check-sculptgl-scene-shift-dense.mjs BASELINE_DIST CANDIDATE_DIST
// Both product builds and the pinned reference run in one owned normal Chrome.
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const baseline = path.resolve(process.argv[2] || '.artifacts/scene-shift-dense-builds/c260');
const candidate = path.resolve(process.argv[3] || '.artifacts/scene-shift-dense-builds/c261');
const reference = path.resolve(process.env.SCULPTGL_REFERENCE || '/Users/Ajab/AI/OBrush/.tools/sculptgl-reference/host-mirror');
const fixture = path.join(root, 'scripts/fixtures/independent-dense-shift-smooth.sgl');
const out = path.join(root, '.artifacts/sculptgl-scene-shift-dense', randomUUID());
await mkdir(out, { recursive: true });
const sha = data => createHash('sha256').update(data).digest('hex');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixtureBytes = await readFile(fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
if (sha(bundle) !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') throw Error('Pinned SculptGL bundle changed');
const result = {
  scope: 'physical Shift-drag Smooth on one independently authored 4,514-vertex closed sphere; Source versus C260 and candidate',
  fixture: { path: 'scripts/fixtures/independent-dense-shift-smooth.sgl', bytes: fixtureBytes.length, sha256: sha(fixtureBytes) },
  reference: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle) },
  builds: {
    baseline: { path: baseline, wasm_sha256: sha(await readFile(path.join(baseline, 'pkg/obrush_app_bg.wasm'))) },
    candidate: { path: candidate, wasm_sha256: sha(await readFile(path.join(candidate, 'pkg/obrush_app_bg.wasm'))), harness_commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim() },
  },
  parameters: { viewport: [1600, 1000], dpr: 1, pointer_modifier: 'Shift', modifier_bits: 8, hold_down_ms: 80, move_offsets_px: [6, 12, 18, 24, 30, 36], move_pause_ms: 70, release_pause_ms: 350, contact_world: [0.17, 0.14, 1.15] },
  trace: [], cases: {}, exceptions: [], status: 'running', gaps: [],
};
let server, chrome, socket, session, seq = 0;
const waiting = new Map();
async function send(method, params = {}, selected = session) {
  const id = ++seq;
  const promise = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error('CDP timeout: ' + method)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return promise;
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(get, description) {
  for (let i = 0; i < 180; i++) {
    const value = await get();
    if (value) return value;
    await sleep(100);
  }
  throw Error('Timed out: ' + description);
}
async function importFixture(selector) {
  const document = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: document.nodeId, selector })).nodeId;
  if (!nodeId) throw Error('Missing file input ' + selector);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixture] });
}
async function mouse(label, type, x, y, buttons) {
  const params = { type, x, y, button: 'left', buttons, modifiers: 8, clickCount: 1 };
  result.trace.push({ label, at_epoch_ms: Date.now(), ...params });
  await send('Input.dispatchMouseEvent', params);
}
async function drag(label, x, y) {
  await mouse(label, 'mouseMoved', x, y, 0);
  await mouse(label, 'mousePressed', x, y, 1);
  await sleep(result.parameters.hold_down_ms);
  for (const dx of result.parameters.move_offsets_px) {
    await mouse(label, 'mouseMoved', x + dx, y, 1);
    await sleep(result.parameters.move_pause_ms);
  }
  await mouse(label, 'mouseReleased', x + 36, y, 0);
  await sleep(result.parameters.release_pause_ms);
}
async function chord(label, key) {
  const modifier = { type: 'rawKeyDown', key: 'Control', code: 'ControlLeft', windowsVirtualKeyCode: 17, modifiers: 2 };
  const letter = { type: 'rawKeyDown', key, code: `Key${key.toUpperCase()}`, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers: 2 };
  for (const params of [modifier, letter, { ...letter, type: 'keyUp' }, { ...modifier, type: 'keyUp', modifiers: 0 }]) {
    result.trace.push({ label, at_epoch_ms: Date.now(), ...params });
    await send('Input.dispatchKeyEvent', params);
  }
  await sleep(200);
}
function diff(a, b) {
  if (a.length !== b.length) throw Error('Vertex count changed');
  let components = 0, max_component = 0, sum = 0;
  for (let i = 0; i < a.length; i++) {
    const d = Math.abs(a[i] - b[i]);
    if (d > 0) components++;
    max_component = Math.max(max_component, d);
    sum += d;
  }
  return { changed_components: components, max_component, sum_absolute: sum };
}
function project(point, camera) {
  const m = camera.view_projection, p = [...point, 1];
  const c = [0, 1, 2, 3].map(row => m[row] * p[0] + m[row + 4] * p[1] + m[row + 8] * p[2] + m[row + 12]);
  const v = camera.viewport;
  return [Math.round(v[0] + (c[0] / c[3] + 1) * (v[2] - v[0]) / 2), Math.round(v[1] + (1 - c[1] / c[3]) * (v[3] - v[1]) / 2)];
}
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?.subtools?{count:m.subtools.length,selected:m.selected_subtool_ids,active:m.active_subtool_id,history:m.history,camera:m.camera,brush:m.brush,source_geometry:m.source_geometry,message:m.message}:null})()`;
async function ownSave() {
  await evaluate('window.__capturedProject=null;window.obrushDownload=(name,contents)=>{window.__capturedProject={name,contents}};true');
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 335, y: 26, buttons: 0 });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: 335, y: 26, button: 'left', buttons: 1, clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 335, y: 26, button: 'left', buttons: 0, clickCount: 1 });
  const project = await until(() => evaluate('window.__capturedProject?.contents ?? null'), 'ordinary Save download');
  const doc = JSON.parse(project);
  return { sha256: sha(Buffer.from(project)), positions: doc.mesh.positions.flat(), scene: doc.scene, bytes: Buffer.byteLength(project) };
}
async function navigate(route) {
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}${route}` });
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const prefix = ['/reference', '/baseline', '/candidate'].find(value => pathname.startsWith(value + '/') || pathname === value);
      if (!prefix) throw Error('unknown route');
      const base = ({ '/reference': reference, '/baseline': baseline, '/candidate': candidate })[prefix];
      const relative = pathname.slice(prefix.length) || '/';
      const file = path.resolve(base, '.' + (relative === '/' ? '/index.html' : relative));
      if (!file.startsWith(base + path.sep)) throw Error('outside served tree');
      const bytes = await readFile(file);
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html', '.css': 'text/css' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', data => { stderr += data; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const pending = waiting.get(message.id);
      if (pending) { waiting.delete(message.id); clearTimeout(pending.timer); message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result); }
    } else if (message.method === 'Runtime.exceptionThrown') result.exceptions.push(message.params.exceptionDetails.text);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(domain + '.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });

  await navigate('/reference/');
  await until(() => evaluate('window.__sculptglReferenceInstance?.getMesh() !== undefined'), 'reference startup');
  await evaluate('window.__sculptglReferenceInstance.clearScene();true');
  await importFixture('#fileopen');
  await until(() => evaluate('window.__sculptglReferenceInstance?._meshes?.[0]?.getNbVertices() === 4514'), 'reference import');
  await sleep(400);
  const sourcePoint = await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,c=a.getCamera(),r=document.getElementById('canvas').getBoundingClientRect();const p=c.project(${JSON.stringify(result.parameters.contact_world)});return [Math.round(p[0]+r.left),Math.round(p[1]+r.top)]})()`);
  const sourcePositions = () => evaluate('Array.from(window.__sculptglReferenceInstance._meshes[0].getVertices())');
  const sourceBefore = await sourcePositions();
  const sourceHistoryBefore = await evaluate('window.__sculptglReferenceInstance.getStateManager()._undos.length');
  await drag('source', ...sourcePoint);
  const sourceAfter = await sourcePositions();
  const sourceHistoryAfter = await evaluate('window.__sculptglReferenceInstance.getStateManager()._undos.length');
  await chord('source-undo', 'z');
  const sourceUndo = await sourcePositions();
  await chord('source-redo', 'y');
  const sourceRedo = await sourcePositions();
  result.cases.source = { point: sourcePoint, initial_history: sourceHistoryBefore, final_history: sourceHistoryAfter, drag: diff(sourceBefore, sourceAfter), undo: diff(sourceBefore, sourceUndo), redo: diff(sourceAfter, sourceRedo) };

  for (const side of ['baseline', 'candidate']) {
    await navigate(`/${side}/?obrushprofile=sculptgl`);
    await until(() => evaluate(`${ownInfo}?.count === 1`), `${side} startup`);
    await importFixture('#import-file');
    await until(() => evaluate(`${ownInfo}?.count === 1 && ${ownInfo}?.subtools?.[0]?.vertices === 4514`), `${side} import`);
    await sleep(300);
    const initial = await evaluate(ownInfo);
    const point = project(result.parameters.contact_world, initial.camera);
    const before = await ownSave();
    await drag(side, ...point);
    const afterInfo = await evaluate(ownInfo);
    const after = await ownSave();
    await chord(side + '-undo', 'z');
    const undo = await ownSave();
    await chord(side + '-redo', 'y');
    const redo = await ownSave();
    result.cases[side] = {
      point, initial: { history: initial.history, brush: initial.brush, selected: initial.selected, save_sha256: before.sha256 },
      final: { history: afterInfo.history, brush: afterInfo.brush, selected: afterInfo.selected, message: afterInfo.message, save_sha256: after.sha256 },
      drag: diff(before.positions, after.positions), undo: diff(before.positions, undo.positions), redo: diff(after.positions, redo.positions),
      undo_save_sha256: undo.sha256, redo_save_sha256: redo.sha256,
    };
  }
  if (!result.cases.source.drag.changed_components) result.gaps.push('Pinned SculptGL Shift-drag did not move vertices in this pose.');
  if (!result.cases.baseline.drag.changed_components) result.gaps.push('C260 Shift-drag did not move vertices in this pose.');
  if (!result.cases.candidate.drag.changed_components) result.gaps.push('Candidate Shift-drag did not move vertices in this pose.');
  if (result.cases.candidate.undo.changed_components || result.cases.candidate.redo.changed_components) throw Error('Candidate public Save/Undo/Redo failed exact geometry recovery');
  if (result.cases.baseline.drag.changed_components && !result.cases.candidate.drag.changed_components) throw Error('Candidate regressed working C260 Shift-drag Smooth');
  if (result.exceptions.length) throw Error('Browser exceptions: ' + result.exceptions.join('; '));
  result.status = result.gaps.length ? 'INCONCLUSIVE' : 'PASS scoped physical Shift-drag Smooth';
} catch (error) {
  result.status = 'FAIL';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  try { socket?.close(); } catch {}
  try { chrome?.kill('SIGTERM'); } catch {}
  try { await new Promise(resolve => server?.close(resolve) ?? resolve()); } catch {}
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ status: result.status, failure: result.failure ?? null, out }));
}
