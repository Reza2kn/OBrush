// Paired normal-browser Source Shift-click selection on three independent boxes.
// Usage: node scripts/check-sculptgl-scene-shift-c261.mjs [web/dist]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const app = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-scene-shift-c261/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = data => createHash('sha256').update(data).digest('hex');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = path.join(root, 'scripts/fixtures/independent-three-closed-boxes.sgl');
const fixtureBytes = await readFile(fixture);
const sourceBundle = await readFile(path.join(reference, 'sculptgl.js'));
const sourceManifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
if (sha(sourceBundle) !== sourceManifest.original_bundle.sha256) throw Error('Pinned SculptGL bundle changed');
const result = {
  scope: 'Source profile three-part stationary Shift-click membership; no click sculpt; ordinary click and Shift-drag guard',
  fixture: { path: 'scripts/fixtures/independent-three-closed-boxes.sgl', bytes: fixtureBytes.length, sha256: sha(fixtureBytes) },
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(sourceBundle) },
  build: { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(app, 'pkg/obrush_app_bg.wasm'))) },
  source_steps: [], obrush_steps: [], errors: [], gaps: [], status: 'running',
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
async function openFile(selector) {
  const document = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: document.nodeId, selector })).nodeId;
  if (!nodeId) throw Error('Missing file input ' + selector);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixture] });
}
async function pointer(x, y, modifier = 0, dragTo = null) {
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, buttons: 0, modifiers: modifier });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, modifiers: modifier, clickCount: 1 });
  // Let the app render the held frame; this catches an unintended down stamp.
  await sleep(80);
  let draggedSourceStroke = null;
  if (dragTo) {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: dragTo[0], y: dragTo[1], button: 'left', buttons: 1, modifiers: modifier });
    await sleep(100);
    draggedSourceStroke = await evaluate('window.obrushBrushMetrics?.().source_geometry?.active ?? null');
  }
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: dragTo?.[0] ?? x, y: dragTo?.[1] ?? y, button: 'left', buttons: 0, modifiers: modifier, clickCount: 1 });
  await sleep(180);
  return draggedSourceStroke;
}
const sourceInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a?._meshes||!a?._selectMeshes)return null;return {count:a._meshes.length,selected:a._selectMeshes.map(m=>a._meshes.indexOf(m)),active:a._meshes.indexOf(a._mesh)}})()`;
const sourceVertices = `(()=>window.__sculptglReferenceInstance._meshes.map(m=>Array.from(m.getVertices())))()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?.subtools?{count:m.subtools.length,selected:m.selected_subtool_ids,active:m.active_subtool_id,history:m.history,viewport:m.camera?.viewport,view_projection:m.camera?.view_projection,message:m.message}:null})()`;
async function ownProject() {
  return evaluate(`(async()=>{const p=window.__capturedProject;if(!p)return null;const bytes=new TextEncoder().encode(p.contents);return {name:p.name,bytes:bytes.length,sha256:Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(x=>x.toString(16).padStart(2,'0')).join(''),document:JSON.parse(p.contents)}})()`);
}
async function saveProject() {
  await evaluate('window.__capturedProject=null;window.obrushDownload=(name,contents)=>{window.__capturedProject={name,contents}};true');
  await pointer(335, 26);
  return until(ownProject, 'ordinary Save project download');
}
function projected(point, camera) {
  const m = camera.view_projection, p = [point[0], point[1], point[2], 1];
  const c = [0, 1, 2, 3].map(row => m[row] * p[0] + m[row + 4] * p[1] + m[row + 8] * p[2] + m[row + 12]);
  const v = camera.viewport;
  return [Math.round(v[0] + (c[0] / c[3] + 1) * (v[2] - v[0]) / 2), Math.round(v[1] + (1 - c[1] / c[3]) * (v[3] - v[1]) / 2)];
}
function geometryById(document) {
  return Object.fromEntries(document.scene.objects.map(object => [
    object.id,
    object.id === document.scene.active_id ? document.mesh : object.payload?.mesh,
  ]));
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const base = pathname.startsWith('/reference/') ? reference : app;
      const relative = pathname.startsWith('/reference/') ? pathname.slice('/reference'.length) : pathname;
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
    } else if (message.method === 'Runtime.exceptionThrown') result.errors.push(message.params.exceptionDetails.text);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(domain + '.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  const url = `http://127.0.0.1:${server.address().port}`;

  result.stage = 'reference navigation';
  await send('Page.navigate', { url: url + '/reference/' });
  result.stage = 'reference startup';
  await until(async () => (await evaluate(sourceInfo))?.count, 'pinned reference startup');
  await evaluate('window.__sculptglReferenceInstance.clearScene();true');
  await until(async () => (await evaluate(sourceInfo))?.count === 0, 'reference scene reset');
  await openFile('#fileopen');
  await until(async () => (await evaluate(sourceInfo))?.count === 3, 'reference fixture import');
  await sleep(500);
  await evaluate('(()=>{const a=window.__sculptglReferenceInstance;a.getGui()._ctrlSculpting._ctrlSculpt.setValue(0);a.getSculptManager().getCurrentTool()._lockPosition=true;return true})()');
  const referenceInitial = await evaluate(sourceInfo);
  const referencePositions = await evaluate(sourceVertices);
  result.source_steps.push({ name: 'import', selected: referenceInitial.selected, active: referenceInitial.active });
  for (const index of [0, 1, 0]) {
    const probe = await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a._meshes[${index}],v=m.getVertices(),p=a.getPicking(),c=a.getCamera();let mn=[Infinity,Infinity,Infinity],mx=[-Infinity,-Infinity,-Infinity];for(let i=0;i<v.length;i+=3)for(let k=0;k<3;k++){mn[k]=Math.min(mn[k],v[i+k]);mx[k]=Math.max(mx[k],v[i+k])}const screen=c.project(mn.map((q,k)=>(q+mx[k])/2));const x=Math.round(screen[0]),y=Math.round(screen[1]);p.intersectionMouseMeshes(a.getMeshes(),x,y);return {x,y,picked:a._meshes.indexOf(p.getMesh())}})()`);
    if (probe.picked !== index) throw Error('Reference pick mismatch ' + JSON.stringify(probe));
    await pointer(probe.x, probe.y + 40, 8);
    const state = await evaluate(sourceInfo);
    result.source_steps.push({ name: 'shift-' + index, probe, selected: state.selected, active: state.active, unchanged_vertices: JSON.stringify(await evaluate(sourceVertices)) === JSON.stringify(referencePositions) });
  }

  result.stage = 'OBRush navigation';
  await send('Page.navigate', { url: url + '/?obrushprofile=sculptgl' });
  await until(async () => (await evaluate(ownInfo))?.count === 1, 'OBRush startup');
  await openFile('#import-file');
  await until(async () => (await evaluate(ownInfo))?.count === 3, 'OBRush fixture import');
  const initial = await evaluate(ownInfo);
  const savedInitial = await saveProject();
  result.project_shape = { keys: Object.keys(savedInitial.document), scene_keys: Object.keys(savedInitial.document.scene ?? {}), object_ids: savedInitial.document.scene?.objects?.map(o=>o.id) ?? null };
  result.obrush_steps.push({ name: 'import', selected: initial.selected, active: initial.active, viewport: initial.viewport, save_sha256: savedInitial.sha256, history: initial.history });
  const centers = [[-1.5, 0.5, 0.5], [0.5, 0.5, 0.5], [2.5, 0.5, 0.5]];
  for (const index of [0, 1, 0]) {
    const info = await evaluate(ownInfo);
    const point = projected(centers[index], info);
    await pointer(...point, 8);
    const state = await evaluate(ownInfo);
    const saved = await saveProject();
    result.obrush_steps.push({ name: 'shift-' + index, point, selected: state.selected, active: state.active, history: state.history, save_sha256: saved.sha256, geometry_unchanged: JSON.stringify(geometryById(saved.document)) === JSON.stringify(geometryById(savedInitial.document)) });
  }
  // A normal click collapses the group to one picked member.
  const normalPoint = projected(centers[0], await evaluate(ownInfo));
  await pointer(...normalPoint);
  const single = await evaluate(ownInfo);
  result.obrush_steps.push({ name: 'ordinary-click-left', point: normalPoint, selected: single.selected, active: single.active });
  // This coarse box needs a near-corner contact to make legacy Smooth's
  // vertex displacement observable. The same Shift gesture crosses the
  // four-point selection threshold while held.
  const beforeDrag = await saveProject();
  const dragPoint = projected([-1.9, 0.1, 1.0], await evaluate(ownInfo));
  const dragStroke = await pointer(...dragPoint, 8, [dragPoint[0] + 12, dragPoint[1]]);
  const afterDrag = await evaluate(ownInfo);
  const savedDrag = await saveProject();
  const dragChangedGeometry = JSON.stringify(geometryById(beforeDrag.document)) !== JSON.stringify(geometryById(savedDrag.document));
  result.obrush_steps.push({ name: 'shift-drag-left', point: dragPoint, selected: afterDrag.selected, active: afterDrag.active, source_stroke_seen: dragStroke !== null, source_stroke: dragStroke, changed_geometry: dragChangedGeometry, history: afterDrag.history, message: afterDrag.message });

  const expectedReference = [[2], [2, 0], [2, 0, 1], [2, 1]];
  const expectedOwn = [[3], [3, 1], [3, 1, 2], [3, 2]];
  if (!result.source_steps.every((s, i) => JSON.stringify(s.selected) === JSON.stringify(expectedReference[i]) && (i === 0 || s.unchanged_vertices))) throw Error('Reference membership/geometry mismatch');
  if (!result.obrush_steps.slice(0, 4).every((s, i) => JSON.stringify(s.selected) === JSON.stringify(expectedOwn[i]) && (i === 0 || s.geometry_unchanged))) throw Error('OBRush membership/geometry mismatch');
  if (JSON.stringify(single.selected) !== '[1]') throw Error('Ordinary click did not select left alone');
  if (JSON.stringify(afterDrag.selected) !== '[1]') throw Error('Shift drag changed selection');
  if (!dragChangedGeometry) result.gaps.push('The three-box drag did not displace any vertex; Smooth drag interaction remains unproven in this fixture. The failed strict guard run is retained.');
  if (result.errors.length) throw Error('Browser exception: ' + result.errors.join('; '));
  result.status = 'PASS scoped paired three-part Shift membership and ordinary click; drag open';
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
