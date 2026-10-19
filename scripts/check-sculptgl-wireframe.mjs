// Scoped owned-browser comparison of the pinned SculptGL W action and OBrush.
// Usage: node scripts/check-sculptgl-wireframe.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const referenceOnly = process.argv.includes('--reference-only');
const out = path.resolve('.artifacts/sculptgl-wireframe-browser/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = data => createHash('sha256').update(data).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, message) => { if (!condition) throw Error(message); };
const lineCounts = record => [...new Set(record.wire.draws.map(draw => draw.count))].sort((a, b) => a - b);
const fixture = Buffer.from('o Quad\nv -2 -1 0\nv -0.5 -1 0\nv -0.5 1 0\nv -2 1 0\nf 1 2 3 4\no Triangle\nv 0.5 -1 0\nv 2 -1 0\nv 1.25 1 0\nf 5 6 7\n');
const fixturePath = path.join(out, 'independent-quad-triangle.obj');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle hash changed');
const result = {
  scope: 'SGL-RENDER-WIREFRAME selected imported quad and triangle; visible W shortcut',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: referenceOnly ? null : { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: sha(fixture), bytes: fixture.length, original_vertices: 7, original_faces: 2, render_triangles: 3, expected_wire_indices: [8, 6] },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, sequence = 0;
const waiting = new Map();
const choosers = [];
async function send(method, params = {}, selected = session) {
  const id = ++sequence;
  const reply = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return reply;
}
async function evaluate(expression) {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
  return reply.result.value;
}
async function until(read, reason) {
  let lastError;
  for (let i = 0; i < 150; i++) {
    try { const value = await read(); if (value) return value; }
    catch (error) { lastError = error; }
    await pause(100);
  }
  throw Error(`Timed out ${reason}${lastError ? `; last read: ${lastError}` : ''}`);
}
async function fileInput(selector) {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector })).nodeId;
  insist(nodeId, `Missing file input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixturePath] });
}
async function key(letter, modifiers = 0) {
  const code = `Key${letter.toUpperCase()}`;
  const windowsVirtualKeyCode = letter.toUpperCase().charCodeAt(0);
  for (const type of ['rawKeyDown', 'keyUp'])
    await send('Input.dispatchKeyEvent', { type, key: letter.toLowerCase(), code, windowsVirtualKeyCode, modifiers });
  await pause(180);
}
async function clickControl(rect, modifiers = 0) {
  insist(Array.isArray(rect) && rect.length === 4, 'Missing visible scene selection control');
  const x = (rect[0] + rect[2]) / 2;
  const y = (rect[1] + rect[3]) / 2;
  insist(x >= 0 && x <= 1600 && y >= 72 && y <= 990, 'Scene selection control outside viewport');
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1, modifiers });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1, modifiers });
  await pause(150);
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), bytes);
  result.screenshots.push({ name, sha256: sha(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
}
const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__wireChecks={draws:[],errors:[]};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);
    if(!gl||typeof gl.drawElements!=='function'||gl.__wireWrapped)return gl;
    gl.__wireWrapped=true;window.__wireChecks.gl=gl;
    const draw=gl.drawElements;
    gl.drawElements=function(mode,count,type,offset){
      if(mode===gl.LINES){const rows=window.__wireChecks.draws;if(rows.length===1024)rows.shift();rows.push({count,type,offset,at:performance.now()});}
      return draw.call(this,mode,count,type,offset);
    };
    return gl;
  };
})();`;
const readWire = `(()=>{const w=window.__wireChecks;const gl=w?.gl;return {draws:w?.draws.slice(-64)??[],gl_error:gl?.getError()??null}})()`;
const refInfo = `(()=>{const app=window.__sculptglReferenceInstance;return app?{meshes:app.getMeshes().length,selected:app.getSelectedMeshes().length,wire:app.getMeshes().map(m=>m.getShowWireframe()),faces:app.getMeshes().map(m=>m.getNbFaces()),states:app.getStateManager()._undos.length}:null})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{subtools:m.subtools.length,subtool_ids:m.subtools.map(x=>x.id),selected:m.selected_subtool_ids,active:m.active_subtool_id,scene_error:m.scene_error,controls:m.scene_controls,history:m.history,source_sgl_import:m.source_sgl_import}:null})()`;
async function sample(name, infoExpression) {
  await pause(180);
  const record = { name, info: await evaluate(infoExpression), wire: await evaluate(readWire) };
  result.cases.push(record);
  await screenshot(name);
  await evaluate('window.__wireChecks.draws.length=0');
  return record;
}
async function freshWire(name) {
  await evaluate('window.__wireChecks.draws.length=0');
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1300, y: 800, button: 'none' });
  await pause(250);
  const record = await evaluate(readWire);
  result.cases.push({ name, wire: record, reason: 'new pointer repaint after the W transition and draw-log reset' });
  return record;
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const source = pathname.startsWith('/reference/') ? reference : web;
      const relative = pathname.startsWith('/reference/') ? pathname.slice('/reference'.length) : pathname;
      const file = path.resolve(source, '.' + (relative === '/' ? '/index.html' : relative));
      if (!file.startsWith(source + path.sep)) throw Error('outside served tree');
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
    if (message.method === 'Page.fileChooserOpened') choosers.push(message.params);
    if (!message.id) return;
    const pending = waiting.get(message.id);
    if (!pending) return;
    waiting.delete(message.id); clearTimeout(pending.timer);
    message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const base = `http://127.0.0.1:${server.address().port}`;

  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async () => (await evaluate(refInfo))?.meshes, 'reference startup');
  await fileInput('#fileopen');
  const referenceLoaded = await until(async () => { const x = await evaluate(refInfo); return x?.meshes >= 2 && x.faces.reduce((a, b) => a + b, 0) >= 2 && x.selected ? x : null; }, 'reference import');
  const referenceBefore = await sample('reference-before', refInfo);
  await key('w');
  const referenceOn = await sample('reference-on', refInfo);
  insist(referenceOn.info.wire.filter(Boolean).length >= referenceLoaded.selected, 'Reference selected meshes did not enable wireframe');
  insist(referenceLoaded.selected === 1 && JSON.stringify(referenceOn.info.wire) === JSON.stringify([false, false, true]), 'Reference imported triangle was not the sole selected wire part');
  insist(JSON.stringify(lineCounts(referenceOn)) === JSON.stringify([6]), 'Reference selected triangle wire count differs');
  await key('w');
  const referenceOff = await sample('reference-off', refInfo);
  insist(referenceOff.info.wire.every(v => !v), 'Reference W did not disable imported wireframe');
  insist(referenceOn.info.states === referenceBefore.info.states && referenceOff.info.states === referenceBefore.info.states, 'Reference wireframe unexpectedly changed state history');

  if (referenceOnly) {
    result.status = 'passed scoped pinned reference W and GL check';
  } else {

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await until(async () => (await evaluate(ownInfo))?.source_sgl_import?.pending?.id || null, 'OBRush Add request');
  await fileInput('#source-sgl-file');
  const ownLoaded = await until(async () => { const x = await evaluate(ownInfo); return x?.subtools === 3 ? x : null; }, 'OBRush two-part import');
  const quadId = ownLoaded.subtool_ids[1], triangleId = ownLoaded.subtool_ids[2];
  if (JSON.stringify(ownLoaded.selected) !== JSON.stringify([triangleId])) {
    await clickControl(ownLoaded.controls[`scene.select.${triangleId}`]);
    await until(async () => JSON.stringify((await evaluate(ownInfo))?.selected) === JSON.stringify([triangleId]), 'OBRush triangle selection');
  }
  await sample('obrush-before', ownInfo);
  await evaluate("document.getElementById('obrush_canvas').focus()");
  await key('w');
  const ownOn = await sample('obrush-on', ownInfo);
  insist(!ownOn.info.scene_error, `OBRush renderer error: ${ownOn.info.scene_error}`);
  insist(JSON.stringify(ownOn.info.selected) === JSON.stringify([triangleId]) && JSON.stringify(lineCounts(ownOn)) === JSON.stringify([6]), 'OBRush triangle W differs from reference one-part scope');
  const ownOnFresh = await freshWire('obrush-on-fresh-frame');
  if (ownOnFresh.draws.length) insist(ownOnFresh.draws.some(draw => draw.count === 6), 'OBRush triangle fresh repaint used an unexpected wire count');
  await key('w');
  const ownOff = await sample('obrush-off', ownInfo);
  const ownOffFresh = await freshWire('obrush-off-fresh-frame');
  insist(ownOffFresh.draws.length === 0, 'OBRush W left wireframe draws enabled after a fresh frame');
  await clickControl(ownOff.info.controls[`scene.select.${quadId}`], 2);
  await until(async () => (await evaluate(ownInfo))?.selected?.length === 2, 'OBRush two-part selection');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  await key('w');
  const pairOn = await sample('obrush-pair-on', ownInfo);
  insist(JSON.stringify(lineCounts(pairOn)) === JSON.stringify([6, 8]), 'OBRush W did not draw both selected quad/triangle wires');
  await key('w');
  const pairOff = await sample('obrush-pair-off', ownInfo);
  const pairOffFresh = await freshWire('obrush-pair-off-fresh-frame');
  insist(pairOffFresh.draws.length === 0, 'OBRush W left selected pair wires enabled after a fresh frame');
  await clickControl(pairOff.info.controls[`scene.select.${quadId}`]);
  await until(async () => (await evaluate(ownInfo))?.selected?.length === 1, 'OBRush one-part selection');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  await key('w');
  const onePart = await sample('obrush-one-part-on', ownInfo);
  insist(JSON.stringify(lineCounts(onePart)) === JSON.stringify([8]), 'OBRush W did not limit wireframe draw to selected quad');
  insist(onePart.info.selected[0] === quadId, 'OBRush selected part changed on W');
  insist([referenceOn, referenceOff, ownOn, ownOff, pairOn, pairOff, onePart].every(c => c.wire.gl_error === 0), 'Nonzero GL error');
  result.status = 'passed scoped two-part W and GL check';
  }
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
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), cases: result.cases, errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
