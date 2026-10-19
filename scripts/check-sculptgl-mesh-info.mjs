// Scoped ordinary-browser counter comparison with the pinned SculptGL mirror.
// Usage: node scripts/check-sculptgl-mesh-info.mjs [OBRush web/dist]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const obrush = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-mesh-info-browser/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = Buffer.from('o Independent_mixed\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 2 0 0\nv 2 1 0\nf 1 2 3 4\nf 2 5 6\n');
const fixturePath = path.join(out, 'independent-mixed.obj');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
if (sha(bundle) !== manifest.original_bundle.sha256) throw Error('Pinned reference bundle hash changed');
const result = {
  scope: 'SGL-APP-MESH-INFO selected-mesh counters; ordinary browser file inputs and visible UI',
  fixture: { sha256: sha(fixture), bytes: fixture.length, expected: { vertices: 6, faces: 2, triangles: 3 } },
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(obrush, 'pkg/obrush_app_bg.wasm'))) },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, next = 0;
const waiting = new Map();
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const response = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return response;
}
async function evalRead(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn, reason) {
  for (let i = 0; i < 150; i++) { const found = await fn(); if (found) return found; await pause(100); }
  throw Error(`Timed out: ${reason}`);
}
async function fileInput(selector) {
  const document = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: document.nodeId, selector })).nodeId;
  if (!node) throw Error(`Missing file input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId: node, files: [fixturePath] });
}
async function clickRect(rect) {
  // App diagnostics expose egui min/max bounds, not x/y/width/height.
  const x = (rect[0] + rect[2]) / 2;
  const y = (rect[1] + rect[3]) / 2;
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}
async function wheel(point, deltaY) {
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: point[0], y: point[1] });
  await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: point[0], y: point[1], deltaX: 0, deltaY });
  await pause(150);
}
async function control(key) {
  for (let i = 0; i < 20; i++) {
    const info = await evalRead(obrushInfo);
    const rect = info.scene_controls?.[key] || info.layer_controls?.[key];
    if (!rect) throw Error(`Missing ordinary control ${key}`);
    const x = (rect[0] + rect[2]) / 2;
    const y = (rect[1] + rect[3]) / 2;
    if (y > 72 && y < 940) return rect;
    await wheel([Math.max(1390, Math.min(x, 1550)), 740], y >= 940 ? 350 : -350);
  }
  throw Error(`Could not scroll to ${key}`);
}
async function action(key) {
  await clickRect(await control(key));
  await pause(150);
  return evalRead(obrushInfo);
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  const file = path.join(out, `${name}.png`);
  await writeFile(file, bytes);
  result.screenshots.push({ name, file, sha256: sha(bytes) });
}
const referenceInfo = `(()=>{const app=window.__sculptglReferenceInstance,mesh=app?.getMesh(),gui=app?.getGui()?._ctrlMesh;return app&&gui?{vertices:mesh?.getNbVertices()??0,faces:mesh?.getNbFaces()??0,triangles:mesh?.getNbTriangles()??0,ui_vertices:gui.domVerts.textContent.trim(),ui_faces:gui.domFaces.textContent.trim(),meshes:app.getMeshes().length}:null})()`;
const obrushInfo = `(()=>{if(typeof window.obrushBrushMetrics!=='function')return null;const m=window.obrushBrushMetrics();return {vertices:m.vertices,faces:m.faces,triangles:m.triangles,active_subtool_id:m.active_subtool_id,scene_controls:m.scene_controls,layer_controls:m.layer_controls,subtools:m.subtools?.length,level:m.subdivision_level,levels:m.subdivision_levels,mode:m.subdivision_mode}})()`;
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const isReference = pathname.startsWith('/reference/');
      const base = isReference ? reference : obrush;
      const relative = isReference ? pathname.slice('/reference'.length) : pathname;
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
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const pending = waiting.get(message.id);
      if (!pending) return;
      waiting.delete(message.id); clearTimeout(pending.timer);
      message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
    } else if (message.method === 'Runtime.exceptionThrown') result.errors.push(message.params.exceptionDetails.text);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  const base = `http://127.0.0.1:${server.address().port}`;

  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async () => (await evalRead(referenceInfo))?.meshes, 'pinned reference startup');
  await fileInput('#fileopen');
  const ref = await until(async () => { const info = await evalRead(referenceInfo); return info?.vertices === 6 ? info : null; }, 'reference mixed OBJ import');
  result.cases.push({ name: 'reference-mixed-import', ...ref });
  await screenshot('reference-mixed');

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evalRead(obrushInfo))?.subtools === 1, 'OBRush startup');
  await fileInput('#import-file');
  const own = await until(async () => { const info = await evalRead(obrushInfo); return info?.vertices === 6 ? info : null; }, 'OBRush mixed OBJ import');
  result.cases.push({ name: 'obrush-mixed-import', ...own });
  await screenshot('obrush-mixed');
  await action('object.subdivision.mode');
  await action('object.subdivision.static_smooth');
  await action('object.add_level');
  const fine = await until(async () => { const info = await evalRead(obrushInfo); return info?.level === 1 && info?.levels === 2 ? info : null; }, 'OBRush static high level');
  result.cases.push({ name: 'obrush-static-high', ...fine });
  await action('object.lower');
  const coarse = await until(async () => { const info = await evalRead(obrushInfo); return info?.level === 0 ? info : null; }, 'OBRush static coarse level');
  result.cases.push({ name: 'obrush-static-coarse', ...coarse });
  await action('object.higher');
  const highAgain = await until(async () => { const info = await evalRead(obrushInfo); return info?.level === 1 ? info : null; }, 'OBRush static high return');
  result.cases.push({ name: 'obrush-static-high-return', ...highAgain });
  await screenshot('obrush-static-high');
  await action('scene.clear');
  const empty = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 0 ? info : null; }, 'OBRush empty scene');
  result.cases.push({ name: 'obrush-empty-scene', ...empty });
  await screenshot('obrush-empty');
  if (ref.faces !== 2 || ref.ui_faces !== 'Faces : 2' || own.faces !== 2 || fine.faces !== 8 || coarse.faces !== 2 || highAgain.faces !== 8 || empty.faces !== 0) throw Error('Counter gate failed');
  result.status = 'passed scoped imported-quad, static-level and empty OBrush counter checks';
} catch (error) {
  result.status = 'failed';
  result.errors.push(String(error.stack || error));
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close();
  if (chrome?.exitCode === null) chrome.kill();
  server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), cases: result.cases.map(({ name, vertices, faces, triangles }) => ({ name, vertices, faces, triangles })), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
