// Scoped normal-browser comparison. Starts its own loopback servers and Chrome.
// Usage: node scripts/check-sculptgl-flat-shading.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const referenceOnly = process.argv.includes('--reference-only');
const out = path.resolve('.artifacts/sculptgl-flat-browser/' + randomUUID());
await mkdir(out, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition, message) => { if (!condition) throw Error(message); };
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-affine-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
assert(hash(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle changed');
const result = {
  scope: 'SGL-RENDER-FLAT selected imported part, UI checkbox, derivative-normal GPU flag',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: hash(bundle), wrapper_sha256: hash(await readFile(path.join(reference, 'index.html'))) },
  build: referenceOnly ? null : { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: hash(fixture), bytes: fixture.length, parts: 2 },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, nextId = 0;
const pending = new Map();
const choosers = [];
async function send(method, params = {}, selected = session) {
  const id = ++nextId;
  const reply = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
    pending.set(id, { resolve, reject, timer });
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
  let last;
  for (let i = 0; i < 150; i++) {
    try { const value = await read(); if (value) return value; } catch (error) { last = error; }
    await pause(100);
  }
  throw Error(`Timed out ${reason}${last ? `; ${last}` : ''}`);
}
async function clickRect(rect) {
  assert(Array.isArray(rect) && rect.length === 4, 'Missing control rectangle');
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  assert(x > 0 && x < 1600 && y > 0 && y < 1000, `Control offscreen: ${rect}`);
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
  await pause(200);
}
async function fileInput(selector) {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector })).nodeId;
  assert(nodeId, `Missing file input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixturePath] });
}
async function key(letter, modifiers = 0) {
  const code = `Key${letter.toUpperCase()}`, windowsVirtualKeyCode = letter.toUpperCase().charCodeAt(0);
  for (const type of ['rawKeyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: letter.toLowerCase(), code, windowsVirtualKeyCode, modifiers });
}
const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__flatCheck={draws:[],gl:null};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.drawElements!=='function'||gl.__flatWrapped)return gl;
    gl.__flatWrapped=true;window.__flatCheck.gl=gl;
    const names=new WeakMap(),get=gl.getUniformLocation,one=gl.uniform1i,draw=gl.drawElements;
    gl.getUniformLocation=function(program,name){const location=get.call(this,program,name);if(location)names.set(location,name);return location;};
    gl.uniform1i=function(location,value){const name=location&&names.get(location);if(name==='uFlat'||name==='u_flat_shading')window.__flatCheck.current=value;return one.call(this,location,value);};
    gl.drawElements=function(mode,count,type,offset){if(mode===gl.TRIANGLES){const rows=window.__flatCheck.draws;if(rows.length===512)rows.shift();rows.push({count,flat:window.__flatCheck.current??null,at:performance.now()});}return draw.call(this,mode,count,type,offset);};
    return gl;
  };
})();`;
const readGpu = `(()=>{const f=window.__flatCheck;return {draws:f?.draws.slice(-64)??[],gl_error:f?.gl?.getError()??null}})()`;
const refInfo = `(()=>{const app=window.__sculptglReferenceInstance;return app?{meshes:app.getMeshes().length,selected:app.getSelectedMeshes().length,selected_indices:app.getSelectedMeshes().map(m=>app.getMeshes().indexOf(m)),flat:app.getMeshes().map(m=>!!m.getFlatShading()),faces:app.getMeshes().map(m=>m.getNbFaces()),states:app.getStateManager()._undos.length}:null})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),selected:m.selected_subtool_ids,active:m.active_subtool_id,controls:m.scene_controls,source_controls:m.layer_controls,history:m.history,error:m.scene_error}:null})()`;
const refFlatRect = `(()=>{const nodes=[...document.querySelectorAll('*')].filter(e=>e.children.length===0&&e.textContent.trim()==='Flat shading');return nodes.map(e=>{const r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]}).find(r=>r[2]>r[0]&&r[3]>r[1]&&r[1]>=0)||null})()`;
const refMenuRect = `(()=>{const e=[...document.querySelectorAll('ul>label')].find(e=>e.textContent.trim()==='Rendering');if(!e)return null;const r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]})()`;
async function sample(name, info) {
  await pause(260);
  const record = { name, info: await evaluate(info), gpu: await evaluate(readGpu) };
  result.cases.push(record);
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), bytes);
  result.screenshots.push({ name, sha256: hash(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
  await evaluate('window.__flatCheck.draws.length=0');
  return record;
}
async function ownFlatRect() {
  for (let i = 0; i < 18; i++) {
    const rect = (await evaluate(ownInfo))?.source_controls?.['source.flat'];
    if (rect && rect[1] > 80 && rect[3] < 990) return rect;
    const x = rect ? (rect[0] + rect[2]) / 2 : 1450;
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y: 750, button: 'none' });
    await pause(40);
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y: 750, deltaX: 0, deltaY: rect?.[1] > 990 ? 350 : -350 });
    await pause(100);
  }
  const after = await sample('obrush-after-scroll', ownInfo);
  throw Error(`OBRush Source flat checkbox never became visible; source_rect=${after.info?.source_controls?.['source.flat']}; controls=${Object.keys(after.info?.source_controls??{}).slice(-20).join(',')}`);
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
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.method === 'Page.fileChooserOpened') choosers.push(message.params);
    if (!message.id) return;
    const row = pending.get(message.id); if (!row) return;
    pending.delete(message.id); clearTimeout(row.timer);
    message.error ? row.reject(Error(JSON.stringify(message.error))) : row.resolve(message.result);
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
  await until(async () => (await evaluate(refInfo))?.meshes === 3, 'reference two-part import');
  const refBefore = await sample('reference-before', refInfo);
  await clickRect(await until(() => evaluate(refMenuRect), 'reference Rendering menu'));
  await until(() => evaluate(`(()=>[...document.querySelectorAll('ul')].some(e=>e.querySelector(':scope>label')?.textContent.trim()==='Rendering'&&e.getAttribute('opened')==='true'))()`), 'reference Rendering menu open');
  const refRect = await until(() => evaluate(refFlatRect), 'reference flat checkbox');
  result.reference_control_probe = await evaluate(`(()=>{const e=[...document.querySelectorAll('*')].find(e=>e.children.length===0&&e.textContent.trim()==='Flat shading');const r=e?.getBoundingClientRect(),input=e?.parentElement?.querySelector('input'),q=input?.getBoundingClientRect();return {rect:r?[r.left,r.top,r.right,r.bottom]:null,input:q?[q.left,q.top,q.right,q.bottom]:null,parent:e?.parentElement?.outerHTML.slice(0,1600),at:r?document.elementFromPoint((r.left+r.right)/2,(r.top+r.bottom)/2)?.outerHTML.slice(0,500):null,menus:[...document.querySelectorAll('*')].filter(n=>n.children.length===0&&n.textContent.trim()==='Rendering').map(n=>{const a=n.getBoundingClientRect();return [a.left,a.top,a.right,a.bottom,n.parentElement?.outerHTML.slice(0,300)]})}})()`);
  await clickRect(refRect);
  const refOn = await sample('reference-on', refInfo);
  assert(refOn.info.selected_indices.length === 1 && refOn.info.flat.every((value, index) => value === refOn.info.selected_indices.includes(index)), 'Reference checkbox did not affect selected imported part only');
  assert(refOn.gpu.draws.some(draw => Boolean(draw.flat)), 'Reference flat shader uniform was not used by a triangle draw');
  assert(refOn.info.states === refBefore.info.states, 'Reference checkbox changed geometry history');
  await clickRect(refRect);
  const refOff = await sample('reference-off', refInfo);
  assert(refOff.info.flat.every(value => !value), 'Reference checkbox did not turn off');
  assert(refOff.gpu.draws.length > 0 && refOff.gpu.draws.every(draw => !draw.flat), 'Reference flat shader uniform remained enabled');
  assert(refOff.info.states === refBefore.info.states, 'Reference off changed geometry history');
  if (referenceOnly) {
    result.status = 'passed scoped pinned reference UI and GL checks';
  } else {
  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  const loaded = await until(async () => { const x = await evaluate(ownInfo); return x?.subtools === 3 ? x : null; }, 'OBRush two-part import');
  assert(JSON.stringify(loaded.selected) === JSON.stringify([loaded.ids[2]]), 'OBRush import selection differs');
  const ownBefore = await sample('obrush-before', ownInfo);
  await clickRect(await ownFlatRect());
  const ownOn = await sample('obrush-on', ownInfo);
  assert(!ownOn.info.error, `OBRush renderer error: ${ownOn.info.error}`);
  assert(JSON.stringify(ownOn.info.selected) === JSON.stringify([loaded.ids[2]]), 'OBRush checkbox changed selection');
  assert(ownOn.gpu.draws.some(draw => Boolean(draw.flat)), 'OBRush flat shader uniform was not used by a triangle draw');
  await clickRect(await ownFlatRect());
  const ownOff = await sample('obrush-off', ownInfo);
  assert(ownOff.gpu.draws.length > 0 && ownOff.gpu.draws.every(draw => !draw.flat), 'OBRush flat shader uniform remained enabled');
  assert([refBefore, refOn, refOff, ownBefore, ownOn, ownOff].every(x => x.gpu.gl_error === 0), 'GL error during flat toggle');
  assert(result.screenshots[0].sha256 !== result.screenshots[1].sha256 && result.screenshots[3].sha256 !== result.screenshots[4].sha256, 'No visible flat-shading change');
  result.status = 'passed scoped reference and OBrush UI checks';
  }
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error.stack || error));
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close(); if (chrome?.exitCode === null) chrome.kill(); server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
