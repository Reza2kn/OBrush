// Paired scoped viewport-resolution check in an owned headless Chrome process.
// Usage: node scripts/check-sculptgl-resolution-c247.mjs WEB_DIST PINNED_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-resolution-browser', randomUUID());
const sphere = process.argv.includes('--sphere');
const fixtureName = sphere ? 'independent-reference-sphere.obj' : 'independent-affine-two-mesh.sgl';
const fixture = await readFile(`crates/obrush-core/tests/fixtures/${fixtureName}`);
const fixturePath = path.join(out, fixtureName);
await mkdir(out, { recursive: true });
await writeFile(fixturePath, fixture);
const sha = b => createHash('sha256').update(b).digest('hex');
const pause = ms => new Promise(r => setTimeout(r, ms));
const assert = (v, why) => { if (!v) throw Error(why); };
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
assert(sha(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle changed');
const result = {
  scope: '0.5/1/2 viewport resolution; actual backing/target pixels, GL allocation, visible controls, project and history',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { name:fixtureName, bytes: fixture.length, sha256: sha(fixture) },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, nextId = 0;
const downloads = path.join(out, 'downloads');
await mkdir(downloads, { recursive: true });
const pending = new Map(), choosers = [], downloadsSeen = [];
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
async function click(x, y) {
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
  await pause(300);
}
const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__resolutionCheck={allocations:[],gl:null};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.texImage2D!=='function'||gl.__resolutionWrapped)return gl;
    gl.__resolutionWrapped=true;window.__resolutionCheck.gl=gl;
    const texture=gl.texImage2D,storage=gl.renderbufferStorage;
    gl.texImage2D=function(...a){if(a.length>=9&&a[0]===gl.TEXTURE_2D){const rows=window.__resolutionCheck.allocations;rows.push({kind:'texture',width:a[3],height:a[4],at:performance.now()});if(rows.length>60)rows.shift();}return texture.apply(this,a);};
    gl.renderbufferStorage=function(...a){const rows=window.__resolutionCheck.allocations;rows.push({kind:'depth',width:a[2],height:a[3],at:performance.now()});if(rows.length>60)rows.shift();return storage.apply(this,a);};
    return gl;
  };
})();`;
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const v=a.getViewport(),c=a.getCanvas();return {ratio:a._pixelRatio,css:[v.clientWidth,v.clientHeight],canvas:[c.width,c.height],history:a.getStateManager()._undos.length,meshes:a.getMeshes().length,gl_error:window.__resolutionCheck.gl?.getError()??null}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();const c=document.getElementById('obrush_canvas');return m?{ratio:m.viewport_resolution_scale,render:m.viewport_resolution,canvas:[c.width,c.height],history:m.history,selected:m.selected_subtool_ids,subtools:m.subtools.length,controls:m.layer_controls,scene_error:m.scene_error,gl_error:window.__resolutionCheck.gl?.getError()??null,allocations:window.__resolutionCheck.allocations.slice(-20)}:null})()`;
async function sample(name, info) {
  await pause(350);
  const record = { name, state: await evaluate(info) };
  const png = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), png);
  result.screenshots.push({ name, sha256: sha(png), size: [png.readUInt32BE(16), png.readUInt32BE(20)] });
  result.cases.push(record);
  return record.state;
}
async function ownResolutionRect() {
  for (let i = 0; i < 20; i++) {
    const rect = (await evaluate(ownInfo))?.controls?.['viewport.resolution'];
    if (rect && rect[1] > 80 && rect[3] < 990) return rect;
    await send('Input.dispatchMouseEvent', { type:'mouseMoved', x:1450, y:750, button:'none' });
    await send('Input.dispatchMouseEvent', { type:'mouseWheel', x:1450, y:750, deltaX:0, deltaY:rect?.[3]>990?350:-350 });
    await pause(100);
  }
  throw Error(`Resolution control offscreen or absent: ${JSON.stringify((await evaluate(ownInfo))?.controls?.['viewport.resolution'])}`);
}
async function save(name) {
  const before = downloadsSeen.length;
  await click(345, 26);
  const event = await until(async () => downloadsSeen.slice(before).find(x => x.state === 'completed'), `project save ${name}`);
  const bytes = await readFile(path.join(downloads, event.guid));
  const record = { name, bytes: bytes.length, sha256: sha(bytes) };
  result.cases.push({ save: record });
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
      response.setHeader('Content-Type', ({ '.wasm':'application/wasm', '.js':'text/javascript', '.html':'text/html', '.css':'text/css' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank'], { stdio:['ignore','pipe','pipe'] });
  let stderr = ''; chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once:true }); socket.addEventListener('error', reject, { once:true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.method === 'Page.fileChooserOpened') choosers.push(message.params);
    if (message.method === 'Browser.downloadWillBegin') downloadsSeen.push({ ...message.params, state:'begin' });
    if (message.method === 'Browser.downloadProgress') { const row=downloadsSeen.find(x=>x.guid===message.params.guid);if(row)Object.assign(row,message.params); }
    if (!message.id) return;
    const row=pending.get(message.id);if(!row)return;pending.delete(message.id);clearTimeout(row.timer);
    message.error?row.reject(Error(JSON.stringify(message.error))):row.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url:'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId:target.targetId, flatten:true }, null)).sessionId;
  for (const domain of ['Page','Runtime','DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled:true });
  await send('Browser.setDownloadBehavior', { behavior:'allowAndName', downloadPath:downloads, eventsEnabled:true }, null);
  await send('Emulation.setDeviceMetricsOverride', { width:1600, height:1000, deviceScaleFactor:1, mobile:false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source:instrument });
  const base=`http://127.0.0.1:${server.address().port}`;
  await send('Page.navigate', { url:`${base}/reference/` });
  await until(async () => (await evaluate(refInfo))?.meshes, 'reference startup');
  await fileInput('#fileopen');
  await until(async () => (await evaluate(refInfo))?.meshes===(sphere?2:3), 'reference fixture import');
  const referenceDefault = await sample('reference-1', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui().onPixelRatio(0.5)');
  const referenceHalf = await sample('reference-0.5', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui().onPixelRatio(2)');
  const referenceDouble = await sample('reference-2', refInfo);
  for (const [state, ratio] of [[referenceDefault,1],[referenceHalf,.5],[referenceDouble,2]]) {
    assert(state.ratio===ratio, `Reference ratio ${state.ratio} != ${ratio}`);
    assert(state.canvas[0]===state.css[0]*ratio&&state.canvas[1]===state.css[1]*ratio, 'Reference canvas backing mismatch');
    assert(state.history===referenceDefault.history, 'Reference resolution changed history');
    assert(state.gl_error===0, `Reference GL error at ${ratio}`);
  }
  await send('Page.navigate', { url:`${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools===1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount=choosers.length;await key('o',2);await until(async()=>choosers.length>chooserCount,'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  await until(async () => (await evaluate(ownInfo))?.subtools===(sphere?2:3), 'OBRush fixture import');
  const ownDefault = await sample('obrush-1', ownInfo);
  const projectDefault = await save('default');
  const rect = await ownResolutionRect();
  await click(rect[0]+4,(rect[1]+rect[3])/2);
  const ownHalf = await sample('obrush-0.5', ownInfo);
  const projectHalf = await save('half');
  const secondRect = await ownResolutionRect();
  await click(secondRect[2]-4,(secondRect[1]+secondRect[3])/2);
  const ownDouble = await sample('obrush-2', ownInfo);
  const projectDouble = await save('double');
  result.control_rect = rect;
  result.control_rect_after_save = secondRect;
  assert(ownDefault.ratio===1, 'OBRush default ratio differs');
  assert(ownHalf.ratio>=.5&&ownHalf.ratio<=.54, `OBRush half control selected ${ownHalf.ratio}`);
  assert(ownDouble.ratio>=1.96&&ownDouble.ratio<=2, `OBRush double control selected ${ownDouble.ratio}`);
  assert(ownDefault.canvas.join(',')===ownHalf.canvas.join(',')&&ownDefault.canvas.join(',')===ownDouble.canvas.join(','), 'OBRush UI canvas changed backing instead of only viewport scene');
  for (const state of [ownHalf,ownDouble]) {
    const expected=ownDefault.render.pixels.map(x=>Math.round(x*state.ratio));
    assert(state.render.pixels.every((x,i)=>Math.abs(x-expected[i])<=1), `OBRush render target mismatch: ${state.render.pixels} vs ${expected}`);
    assert(!state.render.error&&!state.scene_error&&state.gl_error===0, `OBRush scaled render error: ${JSON.stringify(state)}`);
    assert(state.allocations.some(x=>x.kind==='texture'&&x.width===state.render.pixels[0]&&x.height===state.render.pixels[1]), `No actual ${state.ratio} GL texture allocation`);
    assert(JSON.stringify(state.history)===JSON.stringify(ownDefault.history), 'OBRush resolution changed document history');
    assert(JSON.stringify(state.selected)===JSON.stringify(ownDefault.selected), 'OBRush resolution changed selection');
  }
  assert(projectDefault.sha256===projectHalf.sha256&&projectHalf.sha256===projectDouble.sha256, 'OBRush resolution changed public project bytes');
  assert(new Set(result.screenshots.filter(x=>x.name.startsWith('obrush')).map(x=>x.sha256)).size===3, 'OBRush screenshots do not respond to resolution');
  result.status='passed scoped paired resolution dimensions and OBrush file/history checks';
} catch (error) {
  result.status='failed';result.errors.push(String(error.stack||error));
} finally {
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  try { if(socket?.readyState===WebSocket.OPEN) await send('Browser.close',{},null); } catch {}
  socket?.close();if(chrome?.exitCode===null)chrome.kill();server?.close();
}
console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),errors:result.errors}));
if(result.status==='failed')process.exitCode=1;
