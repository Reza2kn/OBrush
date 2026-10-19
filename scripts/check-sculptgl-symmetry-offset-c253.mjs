// Paired, owned-Chrome diagnostic for per-object Source symmetry planes.
// Usage: node scripts/check-sculptgl-symmetry-offset-c253.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-symmetry-offset-c253/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, why) => { if (!condition) throw Error(why); };
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-affine-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned reference bundle differs');
const result = {
  scope: 'SGL-INPUT-SYMMETRY-OFFSET and SGL-INPUT-SYMMETRY-LINE: imported asymmetric affine parts, reference active object plane and OBrush active object guide',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: sha(fixture), bytes: fixture.length },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, next = 0;
const waiting = new Map(), choosers = [];
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const reply = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return reply;
}
async function evaluate(expression) {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (reply.exceptionDetails) throw Error(JSON.stringify(reply.exceptionDetails));
  return reply.result.value;
}
async function until(fn, why) {
  let last;
  for (let i = 0; i < 150; i++) {
    try { const value = await fn(); if (value) return value; } catch (error) { last = error; }
    await pause(100);
  }
  throw Error(`Timed out ${why}${last ? `: ${last}` : ''}`);
}
async function fileInput(selector) {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector })).nodeId;
  insist(nodeId, `Missing file input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixturePath] });
}
async function key(letter, modifiers = 0) {
  const code = `Key${letter.toUpperCase()}`;
  for (const type of ['rawKeyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: letter.toLowerCase(), code, windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
}
async function click(rect) {
  insist(rect?.length === 4, 'Missing control rectangle');
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  insist(x > 0 && x < 1600 && y > 72 && y < 990, `Control outside viewport ${rect}`);
  for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  await pause(170);
}
async function visibleControl(key) {
  for (let i = 0; i < 24; i++) {
    const rect = (await evaluate(ownInfo))?.controls?.[key];
    if (rect && rect[1] > 72 && rect[3] < 985) return rect;
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 145, y: 760, deltaX: 0, deltaY: 360 });
    await pause(110);
  }
  throw Error(`Control ${key} did not become visible`);
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), bytes);
  result.screenshots.push({ name, sha256: sha(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
}
const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__symmetryPair={draws:[],gl:null};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.drawElements!=='function'||gl.__symmetryPairWrapped)return gl;
    gl.__symmetryPairWrapped=true;window.__symmetryPair.gl=gl;
    const names=new WeakMap(),get=gl.getUniformLocation,u3=gl.uniform3fv,u3f=gl.uniform3f,u1=gl.uniform1i,draw=gl.drawElements;
    gl.getUniformLocation=function(program,name){const loc=get.call(this,program,name);if(loc)names.set(loc,name);return loc;};
    gl.uniform3fv=function(loc,value){const name=loc&&names.get(loc);if(name==='uPlaneO'||name==='uPlaneN'||name==='u_symmetry_origin'||name==='u_symmetry_normal')window.__symmetryPair[name]=Array.from(value);return u3.call(this,loc,value);};
    gl.uniform3f=function(loc,x,y,z){const name=loc&&names.get(loc);if(name==='u_symmetry_origin'||name==='u_symmetry_normal')window.__symmetryPair[name]=[x,y,z];return u3f.call(this,loc,x,y,z);};
    gl.uniform1i=function(loc,value){const name=loc&&names.get(loc);if(name==='uSym'||name==='u_symmetry_line')window.__symmetryPair[name]=value;return u1.call(this,loc,value);};
    gl.drawElements=function(mode,count,type,offset){if(mode===gl.TRIANGLES){const s=window.__symmetryPair,r=s.draws;if(r.length===256)r.shift();r.push({count,at:performance.now(),source_origin:s.uPlaneO??null,source_normal:s.uPlaneN??null,source_enabled:s.uSym??null,obrush_origin:s.u_symmetry_origin??null,obrush_normal:s.u_symmetry_normal??null,obrush_enabled:s.u_symmetry_line??null});}return draw.call(this,mode,count,type,offset);};
    return gl;
  };
})();`;
const gpu = `(()=>{const s=window.__symmetryPair;return {draws:s?.draws.slice(-64)??[],gl_error:s?.gl?.getError()??null}})()`;
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;return {meshes:a.getMeshes().length,active:a.getMeshes().indexOf(a.getMesh()),selected:a.getSelectedMeshes().map(m=>a.getMeshes().indexOf(m)),objects:a.getMeshes().map(m=>({vertices:m.getNbVertices(),faces:m.getNbFaces(),small_positions:m.getNbVertices()<10?Array.from(m.getVertices()):null,small_faces:m.getNbFaces()<10?Array.from(m.getFaces()):null,matrix:Array.from(m.getMatrix()),center:Array.from(m._transformData._center),normal:Array.from(m.getSymmetryNormal()),offset:m.getSymmetryOffset(),loose:Array.from(m.getLocalBound()),plane:Array.from(m.getSymmetryOrigin())})),states:a.getStateManager()._undos.length,guide:a.getGui()._ctrlScene?._main?._showSymmetryLine??null}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),active:m.active_subtool_id,selected:m.selected_subtool_ids,controls:m.source_geometry_controls_hit_rect,mode:m.source_geometry?.per_tool?.[0]?.mode??null,offset:m.source_geometry?.active_object_symmetry_offset??null,display:m.display_flags,history:m.history,scene_error:m.scene_error,viewport:m.viewport}:null})()`;
async function sample(name, info) {
  await pause(260);
  const row = { name, info: await evaluate(info), gpu: await evaluate(gpu) };
  result.cases.push(row);
  await screenshot(name);
  await evaluate('window.__symmetryPair.draws.length=0');
  return row;
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
    const pending = waiting.get(message.id); if (!pending) return;
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
  await until(async () => (await evaluate(refInfo))?.meshes === 1, 'reference startup');
  await fileInput('#fileopen');
  await until(async () => (await evaluate(refInfo))?.meshes === 3, 'reference imported parts');
  const referenceZero = await sample('reference-zero', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0.5)');
  const referenceHalf = await sample('reference-half', refInfo);
  insist(referenceHalf.info.objects[referenceHalf.info.active].offset === 0.5, 'Reference active offset did not change');
  insist(referenceHalf.info.objects.filter((x,i)=>i!==referenceHalf.info.active).every((x,i)=>x.offset===referenceZero.info.objects.filter((_,j)=>j!==referenceZero.info.active)[i].offset), 'Reference peer offset changed');
  insist(referenceHalf.info.objects[referenceHalf.info.active].plane[0] !== referenceZero.info.objects[referenceZero.info.active].plane[0], 'Reference local mirror plane did not move');
  const mirrorInputX = 2 * referenceHalf.info.objects[referenceHalf.info.active].plane[0] - 2.5;
  const mirrorRay = `(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),p=a.getPickingSymmetry();const x=${mirrorInputX};const hit=p.intersectionRayMesh(m,[x,-2/3,10],[x,-2/3,-10]);return {input_x:x,hit,face:p.getPickedFace(),point:hit?Array.from(p.getIntersectionPoint()):null}})()`;
  result.reference_mirror_pick_half = await evaluate(mirrorRay);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0)');
  result.reference_mirror_pick_zero = await evaluate(mirrorRay);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0.5)');
  insist(result.reference_mirror_pick_half.hit && result.reference_mirror_pick_half.face === 0, 'Reference offset mirror ray missed the active triangle');
  insist(Math.abs(result.reference_mirror_pick_half.point[0]-2.5) < 1e-5, 'Reference reflected hit X differs from authored triangle center');
  insist(!result.reference_mirror_pick_zero.hit, 'Reference zero-offset mirror ray unexpectedly hit at the fixed nonzero ray');

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  await until(async () => (await evaluate(ownInfo))?.subtools === 3, 'OBRush imported parts');
  const ownZero = await sample('obrush-zero', ownInfo);
  insist(ownZero.info.scene_error === null, `OBRush imported Source guide error: ${ownZero.info.scene_error}`);
  insist(ownZero.gpu.draws.some(draw => draw.count === 3 && draw.obrush_enabled === 1 && draw.obrush_origin), 'OBRush imported active part has no guide GPU plane');
  await click(await visibleControl('source_geometry.mask.preview'));
  const preview = await sample('obrush-mask-preview', ownInfo);
  result.mask_preview = { history: preview.info.history, controls: preview.info.controls };
  if (preview.info.controls['source_geometry.mask.convert']) {
    await click(await visibleControl('source_geometry.mask.convert'));
    const converted = await sample('obrush-mask-converted', ownInfo);
    await click(await visibleControl('source_geometry.mode.source'));
    const sourceMode = await sample('obrush-source-mode', ownInfo);
    insist(sourceMode.info.mode === 'SourceOrdinary', 'OBRush Source mode could not be selected after mask conversion');
    const rect = await visibleControl('source_geometry.mirror.offset');
    const x = rect[0] + (rect[2] - rect[0]) * 0.75, y = (rect[1] + rect[3]) / 2;
    for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
    const ownHalf = await sample('obrush-half', ownInfo);
    result.diagnostic = { reference: referenceHalf.info, obrush_before: ownZero.info, obrush_after: ownHalf.info, converted: converted.info };
    insist(ownHalf.info.offset > 0.45 && ownHalf.info.offset < 0.55, `OBRush mirror offset slider target differs: ${ownHalf.info.offset}`);
    insist(ownHalf.info.scene_error === null, `OBRush mirror guide error: ${ownHalf.info.scene_error}`);
    const plane = row => row.gpu.draws.filter(draw => draw.count === 3 && draw.obrush_enabled === 1 && draw.obrush_origin).at(-1)?.obrush_origin;
    const beforePlane = plane(converted), afterPlane = plane(ownHalf);
    insist(beforePlane && afterPlane, 'Missing active OBrush guide-plane GPU uniforms');
    result.guide_plane_eye_delta = afterPlane.map((value, i) => value - beforePlane[i]);
    insist(Math.hypot(...result.guide_plane_eye_delta) > 1e-4, 'OBRush SymOffset changed metadata but not rendered guide plane');
    const refActive = referenceHalf.info.active;
    const sourceOffset = referenceHalf.info.objects[refActive].offset;
    const sourceDelta = referenceHalf.info.objects[refActive].plane[0] - referenceZero.info.objects[refActive].plane[0];
    result.normalized_radius_pair = { sculptgl: sourceDelta / sourceOffset, obrush: result.guide_plane_eye_delta[0] / ownHalf.info.offset };
    result.normalized_radius_pair.abs_difference = Math.abs(result.normalized_radius_pair.sculptgl - result.normalized_radius_pair.obrush);
    insist(result.normalized_radius_pair.abs_difference < 1e-5, `Paired loose-root normalized radius differs: ${JSON.stringify(result.normalized_radius_pair)}`);
    insist(result.cases.every(row => row.gpu.gl_error === 0), 'Nonzero WebGL error');
    result.status = 'passed scoped paired active offset/guide and reference mirror pick; OBrush pick and stroke remain open';
  } else {
    result.status = 'failed';
    result.errors.push('OBRush imported SGL has no available mask conversion action');
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
