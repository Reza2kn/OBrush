// Scoped normal-browser comparison. Starts its own loopback server and Chrome.
// Usage: node scripts/check-sculptgl-darken.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-darken-browser/' + randomUUID());
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
  scope: 'SGL-RENDER-DARKEN pinned controller and OBrush visible checkbox, selected/unselected imported parts, fragment uniform and history',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: hash(bundle), wrapper_sha256: hash(await readFile(path.join(reference, 'index.html'))) },
  build: { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: hash(fixture), bytes: fixture.length, parts: 2 },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, nextId = 0;
const downloads = path.join(out, 'downloads');
await mkdir(downloads, { recursive: true });
const downloadsSeen = [];
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
  window.__darkenCheck={draws:[],gl:null};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.drawElements!=='function'||gl.__darkenWrapped)return gl;
    gl.__darkenWrapped=true;window.__darkenCheck.gl=gl;
    const names=new WeakMap(),get=gl.getUniformLocation,one=gl.uniform1i,draw=gl.drawElements;
    gl.getUniformLocation=function(program,name){const location=get.call(this,program,name);if(location)names.set(location,name);return location;};
    gl.uniform1i=function(location,value){const name=location&&names.get(location);if(['uDarken','u_darken_unselected','u_selected','u_shading_mode'].includes(name))window.__darkenCheck[name]=value;return one.call(this,location,value);};
    gl.drawElements=function(mode,count,type,offset){if(mode===gl.TRIANGLES){const rows=window.__darkenCheck.draws;if(rows.length===512)rows.shift();rows.push({count,darken:window.__darkenCheck.uDarken??window.__darkenCheck.u_darken_unselected??null,selected:window.__darkenCheck.u_selected??null,shader:window.__darkenCheck.u_shading_mode??null,at:performance.now()});}return draw.call(this,mode,count,type,offset);};
    return gl;
  };
})();`;
const readGpu = `(()=>{const f=window.__darkenCheck;return {draws:f?.draws.slice(-64)??[],gl_error:f?.gl?.getError()??null}})()`;
const refInfo = `(()=>{const app=window.__sculptglReferenceInstance;return app?{meshes:app.getMeshes().length,selected_indices:app.getSelectedMeshes().map(m=>app.getMeshes().indexOf(m)),faces:app.getMeshes().map(m=>m.getNbFaces()),opacity:app.getMeshes().map(m=>m.getOpacity()),shader:app.getMeshes().map(m=>m.getShaderType()),states:app.getStateManager()._undos.length}:null})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),selected:m.selected_subtool_ids,active:m.active_subtool_id,controls:m.scene_controls,source_controls:m.layer_controls,history:m.history,environment:m.environment,error:m.scene_error}:null})()`;
async function sample(name, info) {
  await pause(260);
  const record = { name, info: await evaluate(info), gpu: await evaluate(readGpu) };
  result.cases.push(record);
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), bytes);
  result.screenshots.push({ name, sha256: hash(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
  await evaluate('window.__darkenCheck.draws.length=0');
  return record;
}
async function ownDarkenRect() {
  for (let i = 0; i < 18; i++) {
    const rect = (await evaluate(ownInfo))?.source_controls?.['environment.darken_unselected'];
    if (rect && rect[1] > 80 && rect[3] < 990) return rect;
    const x = rect ? (rect[0] + rect[2]) / 2 : 1450;
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y: 750, button: 'none' });
    await pause(40);
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y: 750, deltaX: 0, deltaY: rect?.[1] > 990 ? 350 : -350 });
    await pause(100);
  }
  throw Error(`OBRush darken checkbox offscreen or absent: ${JSON.stringify((await evaluate(ownInfo))?.source_controls?.['environment.darken_unselected'])}`);
}
async function save(name) {
  const previous = downloadsSeen.length;
  await send('Input.dispatchMouseEvent', { type:'mousePressed', x:345, y:26, button:'left', clickCount:1 });
  await send('Input.dispatchMouseEvent', { type:'mouseReleased', x:345, y:26, button:'left', clickCount:1 });
  const event = await until(async()=>downloadsSeen.slice(previous).find(x=>x.state==='completed'), `project save ${name}`);
  const bytes = await readFile(path.join(downloads,event.guid));
  const doc = JSON.parse(bytes.toString('utf8'));
  const parts = (doc.scene_v7?.objects??[]).map(x=>({id:x.id,opacity:x.render?.opacity??x.metadata?.render?.opacity??null,shader:x.render?.shader??x.metadata?.render?.shader??null}));
  const record = {name,sha256:hash(bytes),bytes:bytes.length,parts};
  result.cases.push({save:record});
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
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.method === 'Page.fileChooserOpened') choosers.push(message.params);
    if (message.method === 'Browser.downloadWillBegin') downloadsSeen.push({...message.params,state:'begin'});
    if (message.method === 'Browser.downloadProgress') {const event=downloadsSeen.find(x=>x.guid===message.params.guid);if(event)Object.assign(event,message.params);}
    if (!message.id) return;
    const row = pending.get(message.id); if (!row) return;
    pending.delete(message.id); clearTimeout(row.timer);
    message.error ? row.reject(Error(JSON.stringify(message.error))) : row.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Browser.setDownloadBehavior', { behavior:'allowAndName', downloadPath:downloads, eventsEnabled:true }, null);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const base = `http://127.0.0.1:${server.address().port}`;
  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async () => (await evaluate(refInfo))?.meshes, 'reference startup');
  await fileInput('#fileopen');
  await until(async () => (await evaluate(refInfo))?.meshes === 3, 'reference two-part import');
  result.reference_menu_probe=await evaluate(`(()=>[...document.querySelectorAll('label')].map(e=>({text:e.textContent.trim().slice(0,80),rect:(()=>{const r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]})()})).filter(x=>x.rect[2]>x.rect[0]).slice(0,40))()`);
  const refOn = await sample('reference-on', refInfo);
  const refOnDraws = refOn.gpu.draws.filter(d => d.count === 3 || d.count === 6);
  assert(refOn.info.selected_indices.length === 1 && refOn.info.selected_indices[0] === 2, 'Reference selected-part scope differs');
  assert(refOnDraws.some(d => d.count === 6 && d.darken === 1) && refOnDraws.some(d => d.count === 3 && d.darken === 0), 'Reference did not darken only the unselected part');
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onDarkenUnselected(false)');
  const refOff = await sample('reference-off', refInfo);
  const refOffDraws = refOff.gpu.draws.filter(d => d.count === 3 || d.count === 6);
  assert(refOffDraws.some(d => d.count === 3 && d.darken === 0) && refOffDraws.some(d => d.count === 6 && d.darken === 0), 'Reference off did not remove darken');
  assert(refOff.info.states === refOn.info.states, 'Reference darken changed history');
  assert(JSON.stringify(refOff.info.opacity) === JSON.stringify(refOn.info.opacity) && JSON.stringify(refOff.info.shader) === JSON.stringify(refOn.info.shader), 'Reference darken changed opacity or shader');
  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  const loaded = await until(async () => { const x = await evaluate(ownInfo); return x?.subtools === 3 ? x : null; }, 'OBRush two-part import');
  assert(JSON.stringify(loaded.selected) === JSON.stringify([loaded.ids[2]]), 'OBRush import selection differs');
  const ownOn = await sample('obrush-on', ownInfo);
  assert(ownOn.info.environment.darken_unselected === true, 'OBRush default darken differs');
  const ownOnDraws = ownOn.gpu.draws.filter(d => d.count === 3 || d.count === 6);
  assert(ownOnDraws.some(d => d.count === 6 && d.darken === 1 && d.selected === 0) && ownOnDraws.some(d => d.count === 3 && d.darken === 1 && d.selected === 1), 'OBRush selected/unselected shader uniforms differ');
  const ownOnSave = await save('darken-on');
  await clickRect(await ownDarkenRect());
  const ownOff = await sample('obrush-off', ownInfo);
  assert(!ownOff.info.error && ownOff.info.environment.darken_unselected === false, `OBRush darken checkbox or renderer failed: ${ownOff.info.error}`);
  const ownOffDraws = ownOff.gpu.draws.filter(d => d.count === 3 || d.count === 6);
  assert(ownOffDraws.some(d => d.count === 3 && d.darken === 0) && ownOffDraws.some(d => d.count === 6 && d.darken === 0), 'OBRush off did not remove darken');
  assert(JSON.stringify(ownOff.info.history) === JSON.stringify(ownOn.info.history), 'OBRush view checkbox changed history');
  assert(JSON.stringify(ownOff.info.selected) === JSON.stringify(ownOn.info.selected), 'OBRush view checkbox changed selection');
  const ownOffSave = await save('darken-off');
  assert(ownOffSave.sha256 === ownOnSave.sha256, 'OBRush view checkbox changed public project bytes');
  assert([refOn,refOff,ownOn,ownOff].every(x => x.gpu.gl_error === 0), 'GL error during darken toggle');
  assert(result.screenshots[0].sha256 !== result.screenshots[1].sha256 && result.screenshots[2].sha256 !== result.screenshots[3].sha256, 'No visible darken toggle');
  result.status = 'passed scoped paired reference and OBrush UI/GL checks';
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error.stack || error));
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close(); if (chrome?.exitCode === null) chrome.kill(); server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
