// Scoped additive primitive and empty-scene workflow comparison.
// Usage: node scripts/check-sculptgl-primitives-c248.mjs [OBRush web/dist]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const obrush = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-primitives-browser/' + randomUUID());
await mkdir(out, { recursive: true });
const downloads = path.join(out, 'downloads');
await mkdir(downloads, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = Buffer.from('o Independent_mixed\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 2 0 0\nv 2 1 0\nf 1 2 3 4\nf 2 5 6\n');
const fixturePath = path.join(out, 'independent-mixed.obj');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
if (sha(bundle) !== manifest.original_bundle.sha256) throw Error('Pinned reference bundle hash changed');
const result = {
  scope: 'SGL-SCENE-SPHERE/CUBE/CYLINDER/TORUS/CLEAR; imported fixture, additive controls and exact undo state',
  fixture: { sha256: sha(fixture), bytes: fixture.length, expected: { vertices: 6, faces: 2, triangles: 3 } },
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(obrush, 'pkg/obrush_app_bg.wasm'))) },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, next = 0;
const downloadEvents = [];
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
async function chord(key, useMeta = false) {
  // A button click can retain inspector focus. Match the normal sculpt tests:
  // move focus to empty viewport background before sending the shortcut.
  const viewport = (await evalRead(obrushInfo))?.viewport;
  if (!Array.isArray(viewport) || viewport.length !== 4) throw Error('Missing viewport for keyboard focus');
  const x = viewport[2] - 25, y = viewport[3] - 25;
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
  await pause(35);
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  await pause(40);
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
  await pause(150);
  const code = `Key${key.toUpperCase()}`;
  // The selected SculptGL shortcut profile uses Ctrl+Z/Ctrl+Y, as the pinned
  // source does, including on macOS. Meta+Z belongs to OBrush's legacy profile.
  const modifiers = useMeta ? 4 : 2;
  const modifierKey = useMeta ? 'Meta' : 'Control';
  const modifierCode = useMeta ? 'MetaLeft' : 'ControlLeft';
  const modifierVkey = useMeta ? 91 : 17;
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: modifierKey, code: modifierCode, windowsVirtualKeyCode: modifierVkey, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: modifierKey, code: modifierCode, windowsVirtualKeyCode: modifierVkey, modifiers: 0 });
  await pause(150);
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  const file = path.join(out, `${name}.png`);
  await writeFile(file, bytes);
  result.screenshots.push({ name, file, sha256: sha(bytes) });
}
const referenceInfo = `(()=>{const app=window.__sculptglReferenceInstance,mesh=app?.getMesh(),gui=app?.getGui()?._ctrlMesh;return app&&gui?{vertices:mesh?.getNbVertices()??0,faces:mesh?.getNbFaces()??0,triangles:mesh?.getNbTriangles()??0,active_id:mesh?.getID()??null,ui_vertices:gui.domVerts.textContent.trim(),ui_faces:gui.domFaces.textContent.trim(),meshes:app.getMeshes().length,parts:app.getMeshes().map(m=>({id:m.getID(),vertices:m.getNbVertices(),faces:m.getNbFaces(),triangles:m.getNbTriangles(),levels:m._meshes?.length??1})),history:{undo:app.getStateManager()._undos.length,redo:app.getStateManager()._redos.length}}:null})()`;
const obrushInfo = `(()=>{if(typeof window.obrushBrushMetrics!=='function')return null;const m=window.obrushBrushMetrics();return {vertices:m.vertices,faces:m.faces,triangles:m.triangles,active_subtool_id:m.active_subtool_id,scene_controls:m.scene_controls,layer_controls:m.layer_controls,subtools:m.subtools?.length,parts:m.subtools?.map(s=>({id:s.id,name:s.name,vertices:s.vertices,faces:s.faces,triangles:s.triangles})),history:m.history,viewport:m.camera?.viewport,message:m.message}})()`;
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
    else if (message.method === 'Browser.downloadWillBegin' || message.method === 'Browser.downloadProgress') downloadEvents.push({ method: message.method, ...message.params });
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloads, eventsEnabled: true }, null);
  const base = `http://127.0.0.1:${server.address().port}`;

  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async () => (await evalRead(referenceInfo))?.meshes, 'pinned reference startup');
  // Start both applications with only the independently authored imported part.
  // SculptGL keeps its startup sphere on normal File Import; its Scene Reset
  // controller removes that sphere and resets history before the fixture load.
  await evalRead('(()=>{window.confirm=()=>true;window.__sculptglReferenceInstance.getGui()._ctrlScene.clearScene();return true})()');
  await until(async () => (await evalRead(referenceInfo))?.meshes === 0, 'reference initial clear');
  await fileInput('#fileopen');
  const ref = await until(async () => { const info = await evalRead(referenceInfo); return info?.vertices === 6 ? info : null; }, 'reference mixed OBJ import');
  result.cases.push({ name: 'reference-mixed-import', ...ref });
  const referenceActions = [
    ['sphere', 'addSphere'], ['cube', 'addCube'],
    ['cylinder', 'addCylinder'], ['torus', 'addTorus'],
  ];
  for (const [name, method] of referenceActions) {
    await evalRead(`(()=>{window.__sculptglReferenceInstance.${method}();return true})()`);
    const state = await until(async () => {
      const info = await evalRead(referenceInfo);
      return info?.meshes === result.cases.at(-1).meshes + 1 ? info : null;
    }, `reference add ${name}`);
    result.cases.push({ name: `reference-add-${name}`, ...state });
  }
  await screenshot('reference-four-additions');
  await evalRead('(()=>{window.__sculptglReferenceInstance.getGui()._ctrlStates.onUndo();return true})()');
  const referenceUndo = await until(async () => { const info = await evalRead(referenceInfo); return info?.meshes === ref.meshes + 3 ? info : null; }, 'reference primitive undo');
  result.cases.push({ name: 'reference-undo-torus', ...referenceUndo });
  await evalRead('(()=>{window.__sculptglReferenceInstance.getGui()._ctrlStates.onRedo();return true})()');
  const referenceRedo = await until(async () => { const info = await evalRead(referenceInfo); return info?.meshes === ref.meshes + 4 ? info : null; }, 'reference primitive redo');
  result.cases.push({ name: 'reference-redo-torus', ...referenceRedo });
  await evalRead('(()=>{window.confirm=()=>true;window.__sculptglReferenceInstance.getGui()._ctrlScene.clearScene()})()');
  const referenceEmpty = await until(async () => { const info = await evalRead(referenceInfo); return info?.meshes === 0 ? info : null; }, 'reference clear scene');
  result.cases.push({ name: 'reference-clear', ...referenceEmpty });

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evalRead(obrushInfo))?.subtools === 1, 'OBRush startup');
  await fileInput('#import-file');
  const own = await until(async () => { const info = await evalRead(obrushInfo); return info?.vertices === 6 ? info : null; }, 'OBRush mixed OBJ import');
  result.cases.push({ name: 'obrush-mixed-import', ...own });
  for (const name of ['sphere', 'cube', 'cylinder', 'torus']) {
    await action(`scene.add_${name}`);
    const state = await until(async () => {
      const info = await evalRead(obrushInfo);
      return info?.subtools === result.cases.at(-1).subtools + 1 ? info : null;
    }, `OBRush add ${name}`);
    result.cases.push({ name: `obrush-add-${name}`, ...state });
  }
  await screenshot('obrush-four-additions');
  await chord('z');
  const ownUndo = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === own.subtools + 3 ? info : null; }, 'OBRush primitive undo');
  result.cases.push({ name: 'obrush-undo-torus', ...ownUndo });
  await chord('y');
  const ownRedo = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === own.subtools + 4 ? info : null; }, 'OBRush primitive redo');
  result.cases.push({ name: 'obrush-redo-torus', ...ownRedo });
  await action('scene.clear');
  const empty = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 0 ? info : null; }, 'OBRush empty scene');
  result.cases.push({ name: 'obrush-clear', ...empty });
  await screenshot('obrush-empty');
  await action('scene.add_cube');
  const denseCube = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 1 && info.vertices === 98306 ? info : null; }, 'OBRush direct dense cube');
  result.cases.push({ name: 'obrush-direct-cube', ...denseCube });
  await chord('z');
  const cubeUndo = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 0 ? info : null; }, 'OBRush direct dense cube undo');
  result.cases.push({ name: 'obrush-direct-cube-undo', ...cubeUndo });
  await chord('y');
  const cubeRedo = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 1 && info.vertices === 98306 ? info : null; }, 'OBRush direct dense cube redo');
  result.cases.push({ name: 'obrush-direct-cube-redo', ...cubeRedo });
  // Normal top-bar Save project control at the fixed 1600x1000 test viewport.
  // The reference shortcut profile does not bind Cmd+S; use the visible UI.
  await clickRect([303, 13, 387, 39]);
  result.after_save = await evalRead(obrushInfo);
  const savedFile = await until(async () => {
    const completed = downloadEvents.find(e => e.method === 'Browser.downloadProgress' && e.state === 'completed');
    if (!completed) return null;
    const file = path.join(downloads, completed.guid);
    const bytes = await readFile(file);
    if (!bytes.length) return null;
    try { return { file, bytes, project: JSON.parse(bytes.toString('utf8')) }; } catch { return null; }
  }, 'OBRush dense cube project download');
  result.saved_cube = { file: savedFile.file, bytes: savedFile.bytes.length, sha256: sha(savedFile.bytes), vertices: savedFile.project.mesh.positions.length, triangles: savedFile.project.mesh.triangles.length, original_faces: savedFile.project.mesh.surface?.faces?.length ?? null };
  await action('scene.clear');
  const beforeReload = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 0 ? info : null; }, 'OBRush clear before dense cube reload');
  result.cases.push({ name: 'obrush-direct-cube-clear-for-reload', ...beforeReload });
  const input = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: input.nodeId, selector: '#import-file' })).nodeId;
  if (!node) throw Error('Missing OBrush project input');
  await send('DOM.setFileInputFiles', { nodeId: node, files: [savedFile.file] });
  const cubeReload = await until(async () => { const info = await evalRead(obrushInfo); return info?.subtools === 1 && info.vertices === 98306 && info.message?.startsWith('Loaded ') ? info : null; }, 'OBRush dense cube project reload');
  result.cases.push({ name: 'obrush-direct-cube-reload', ...cubeReload });
  await clickRect([303, 13, 387, 39]);
  const reSavedFile = await until(async () => {
    const completed = downloadEvents.find(e => e.method === 'Browser.downloadProgress' && e.state === 'completed' && e.guid !== downloadEvents.find(first => first.method === 'Browser.downloadProgress' && first.state === 'completed')?.guid);
    if (!completed) return null;
    const bytes = await readFile(path.join(downloads, completed.guid));
    try { return { bytes, project: JSON.parse(bytes.toString('utf8')) }; } catch { return null; }
  }, 'OBRush re-saved dense cube project');
  result.resaved_cube = { bytes: reSavedFile.bytes.length, sha256: sha(reSavedFile.bytes), exact_json: JSON.stringify(reSavedFile.project) === JSON.stringify(savedFile.project) };
  const referenceCases = result.cases.filter(c => c.name.startsWith('reference-add-'));
  const ownCases = result.cases.filter(c => c.name.startsWith('obrush-add-'));
  if (ref.faces !== 2 || own.faces !== 2 || referenceCases.length !== 4 || ownCases.length !== 4
    || referenceCases.some((c, index) => c.meshes !== ref.meshes + index + 1 || c.parts.at(-1).faces < 1
      || c.active_id !== c.parts.at(-1).id || c.history.undo !== ref.history.undo + index + 1
      || JSON.stringify(c.parts.slice(0, ref.parts.length)) !== JSON.stringify(ref.parts))
    || ownCases.some((c, index) => c.subtools !== own.subtools + index + 1 || c.faces < 1
      || c.active_subtool_id !== c.parts.at(-1).id || c.history.undo !== own.history.undo + index + 1
      || JSON.stringify(c.parts.slice(0, own.parts.length)) !== JSON.stringify(own.parts))
    || referenceUndo.parts.at(-1).id !== referenceCases.at(-2).parts.at(-1).id
    || referenceRedo.parts.at(-1).id !== referenceCases.at(-1).parts.at(-1).id
    || ownUndo.parts.at(-1).id !== ownCases.at(-2).parts.at(-1).id
    || ownRedo.parts.at(-1).id !== ownCases.at(-1).parts.at(-1).id
    || referenceEmpty.meshes !== 0 || referenceEmpty.history.undo !== 0 || empty.subtools !== 0 || empty.faces !== 0
    || denseCube.faces !== 98304 || denseCube.triangles !== 196608
    || cubeUndo.faces !== 0 || cubeRedo.faces !== denseCube.faces
    || result.saved_cube.vertices !== denseCube.vertices || result.saved_cube.triangles !== denseCube.triangles
    || result.saved_cube.original_faces !== denseCube.faces || cubeReload.faces !== denseCube.faces
    || beforeReload.faces !== 0 || !result.resaved_cube.exact_json)
    throw Error('Additive primitive or clear-scene gate failed');
  result.status = 'passed scoped additive controls, retained imported part, primitive undo/redo, empty scene, and direct dense cube undo/save/reload; reference shape remains open';
} catch (error) {
  result.status = 'failed';
  result.errors.push(String(error.stack || error));
} finally {
  result.download_events = downloadEvents;
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close();
  if (chrome?.exitCode === null) chrome.kill();
  server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), cases: result.cases.map(({ name, vertices, faces, triangles, meshes, subtools }) => ({ name, vertices, faces, triangles, meshes, subtools })), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
