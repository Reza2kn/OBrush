// Scoped owned-browser selected-part transparency comparison.
// Usage: node scripts/check-sculptgl-transparency.mjs WEB_DIST PINNED_HOST_MIRROR [--reference-only]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const referenceOnly = process.argv.includes('--reference-only');
const out = path.resolve('.artifacts/sculptgl-transparency-browser/' + randomUUID());
await mkdir(out, { recursive: true });
const downloads = path.join(out, 'downloads');
const sha = data => createHash('sha256').update(data).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, message) => { if (!condition) throw Error(message); };
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-overlap-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle hash changed');
const result = {
  scope: 'SGL-RENDER-TRANSPARENCY selected overlapping SGL source parts at 0/50/100 percent; opaque/transparent GL state, wire, project/history',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: referenceOnly ? null : { commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: sha(fixture), bytes: fixture.length, type: 'independently authored SGLv3; two affine overlapping meshes' },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, sequence = 0;
const downloadsSeen=[];
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
const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__transparencyChecks={draws:[],errors:[]};
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);
    if(!gl||typeof gl.drawElements!=='function'||gl.__transparencyWrapped)return gl;
    gl.__transparencyWrapped=true;window.__transparencyChecks.gl=gl;
    const draw=gl.drawElements;
    gl.drawElements=function(mode,count,type,offset){
      const rows=window.__transparencyChecks.draws;
      if(rows.length===512)rows.shift();
      if(mode===gl.TRIANGLES||mode===gl.LINES)rows.push({mode:mode===gl.LINES?'lines':'triangles',count,blend:gl.isEnabled(gl.BLEND),depth_write:gl.getParameter(gl.DEPTH_WRITEMASK),cull:gl.isEnabled(gl.CULL_FACE),cull_face:gl.getParameter(gl.CULL_FACE_MODE),depth_func:gl.getParameter(gl.DEPTH_FUNC),at:performance.now()});
      return draw.call(this,mode,count,type,offset);
    };
    return gl;
  };
})();`;
const readDraws = `(()=>{const t=window.__transparencyChecks;return {draws:t?.draws.slice(-128)??[],gl_error:t?.gl?.getError()??null}})()`;
const refInfo = `(()=>{const app=window.__sculptglReferenceInstance;return app?{meshes:app.getMeshes().length,selected:app.getSelectedMeshes().map(m=>app.getMeshes().indexOf(m)),opacity:app.getMeshes().map(m=>m.getOpacity()),wire:app.getMeshes().map(m=>m.getShowWireframe()),states:app.getStateManager()._undos.length}:null})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),selected:m.selected_subtool_ids,active:m.active_subtool_id,scene_error:m.scene_error,controls:{...m.scene_controls,...m.layer_controls},history:m.history,source_sgl_import:m.source_sgl_import}:null})()`;
const screenshot = async name => {
  const bytes=Buffer.from((await send('Page.captureScreenshot',{format:'png',fromSurface:true})).data,'base64');
  await writeFile(path.join(out,`${name}.png`),bytes);
  result.screenshots.push({name,sha256:sha(bytes),width:bytes.readUInt32BE(16),height:bytes.readUInt32BE(20)});
};
async function sample(name, infoExpression){
  await pause(200);
  const record={name,info:await evaluate(infoExpression),gl:await evaluate(readDraws)};
  result.cases.push(record);await screenshot(name);
  await evaluate('window.__transparencyChecks.draws.length=0');
  return record;
}
async function setOwnPercent(percent){
  for(let attempt=0;attempt<25;attempt++){
    const info=await evaluate(ownInfo),rect=info?.controls?.['source.opacity'];
    insist(rect,'Missing Source transparency slider');
    const y=(rect[1]+rect[3])/2;
    if(y>=80&&y<=940){
      // Egui's response rect includes its value/label; the actual slider track
      // occupies the first 180 physical pixels in this fixed 1600px viewport.
      const x=rect[0]+2.5+180*(percent/100);
      result.cases.push({name:`obrush-slider-hit-${percent}`,rect,x,y});
      await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});
      await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});
      await pause(180);return;
    }
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:180,y:730,deltaX:0,deltaY:y>940?320:-320});
    await pause(140);
  }
  throw Error('Source transparency slider stayed outside viewport');
}
async function save(name){
  const previous=downloadsSeen.length;
  // The pinned 1600x1000 app route exposes Save project in the top bar.
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x:345,y:26,button:'left',clickCount:1});
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x:345,y:26,button:'left',clickCount:1});
  const event=await until(async()=>downloadsSeen.slice(previous).find(x=>x.state==='completed'),'project save '+name);
  const bytes=await readFile(path.join(downloads,event.guid));
  const doc=JSON.parse(bytes.toString('utf8'));
  const opacity=(doc.scene_v7?.objects??[]).map(x=>({id:x.id,opacity:x.render?.opacity??x.metadata?.render?.opacity??null}));
  result.cases.push({saved:name,sha256:sha(bytes),bytes:bytes.length,opacity,history:(await evaluate(ownInfo)).history});
  return {doc,hash:sha(bytes),opacity};
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
    if (message.method === 'Browser.downloadWillBegin') downloadsSeen.push({...message.params,state:'begin'});
    if (message.method === 'Browser.downloadProgress') {const event=downloadsSeen.find(x=>x.guid===message.params.guid);if(event)Object.assign(event,message.params);}
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
  await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloads, eventsEnabled: true }, null);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  await mkdir(path.join(out,'downloads'),{recursive:true});
  const base = `http://127.0.0.1:${server.address().port}`;

  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async()=>(await evaluate(refInfo))?.meshes,'reference startup');
  await fileInput('#fileopen');
  await until(async()=>(await evaluate(refInfo))?.meshes===3,'reference two-part SGL import');
  await evaluate(`(()=>{const a=window.__sculptglReferenceInstance;a.getMeshes()[0].setVisible(false);a.setOrUnsetMesh(a.getMeshes()[1],true);a.render();return a.getSelectedMeshes().length})()`);
  const selectedRef=await evaluate(refInfo);
  insist(selectedRef.selected.length===2,'Reference imported two-part selection failed');
  result.cases.push({name:'reference-selection',info:selectedRef});
  for(const percent of [0,50,100]){
    await evaluate(`(()=>{const a=window.__sculptglReferenceInstance;a.getGui()._ctrlRendering._ctrlTransparency.setValue(${percent});a.render();return a.getSelectedMeshes().map(m=>m.getOpacity())})()`);
    const c=await sample(`reference-${percent}`,refInfo);
    insist(c.info.selected.every(index=>Math.abs(c.info.opacity[index]-(1-percent/100))<1e-6),`Reference selected opacity differs at ${percent}`);
    insist(c.gl.gl_error===0,`Reference GL error at ${percent}`);
  }
  await evaluate(`(()=>{const a=window.__sculptglReferenceInstance;a.getGui()._ctrlRendering._ctrlTransparency.setValue(50);a.getGui()._ctrlRendering._ctrlShowWireframe.setValue(true);a.render()})()`);
  const referenceWire=await sample('reference-50-wire',refInfo);
  insist(referenceWire.gl.draws.some(draw=>draw.mode==='lines'),'Reference transparent wire overlay made no line draw');
  await evaluate(`(()=>{const a=window.__sculptglReferenceInstance;a.getGui()._ctrlRendering._ctrlShowWireframe.setValue(false);a.render()})()`);
  result.reference_history_neutral=(await evaluate(refInfo)).states===selectedRef.states;
  if(referenceOnly){result.status='passed scoped pinned reference transparency and GL check';}
  else {
    await send('Page.navigate',{url:`${base}/?obrushprofile=sculptgl`});
    await until(async()=>(await evaluate(ownInfo))?.subtools===1,'OBRush startup');
    await evaluate("document.getElementById('obrush_canvas').focus()");
    const chooserCount=choosers.length;await key('o',2);
    await until(async()=>choosers.length>chooserCount,'OBRush Add chooser');
    await until(async()=>(await evaluate(ownInfo))?.source_sgl_import?.pending?.id,'OBRush Add request');
    await fileInput('#source-sgl-file');
    const loaded=await until(async()=>{const m=await evaluate(ownInfo);return m?.subtools===3?m:null;},'OBRush two-part SGL import');
    const [first,second]=loaded.ids.slice(-2);
    result.cases.push({name:'obrush-loaded',info:loaded});
    const defaultVisibility=await until(async()=>(await evaluate(ownInfo))?.controls?.[`scene.visibility.${loaded.ids[0]}`],'OBRush default part visibility control');
    await clickControl(defaultVisibility);
    const firstControl=await until(async()=>(await evaluate(ownInfo))?.controls?.[`scene.select.${first}`],'OBRush first visible scene control');
    await clickControl(firstControl);
    const secondControl=await until(async()=>(await evaluate(ownInfo))?.controls?.[`scene.select.${second}`],'OBRush second visible scene control');
    await clickControl(secondControl,2);
    insist((await evaluate(ownInfo)).selected.length===2,'OBRush two-part selection failed');
    for(const percent of [0,50,100]){
      await setOwnPercent(percent);
      const c=await sample(`obrush-${percent}`,ownInfo);
      const saved=await save(`obrush-${percent}`);
      insist(c.gl.gl_error===0,`OBRush GL error at ${percent}`);
      insist(saved.opacity.length===2 && saved.opacity.every(row=>row.opacity===1-percent/100),`OBRush selected public opacity differs at ${percent}`);
      result.cases.push({name:`obrush-${percent}-public`,saved_opacity:saved.opacity});
    }
    const final=await save('before-undo');
    await evaluate("document.getElementById('obrush_canvas').focus()");await key('z',2);
    const undone=await save('undo');
    await key('y',2);const redone=await save('redo');
    insist(final.hash===redone.hash&&final.hash!==undone.hash,'OBRush public Undo/Redo transparency did not roundtrip');
    await setOwnPercent(50);
    await evaluate("document.getElementById('obrush_canvas').focus()");await key('w');
    const ownWire=await sample('obrush-50-wire',ownInfo);
    insist(ownWire.gl.draws.some(draw=>draw.mode==='lines'),'OBRush transparent wire overlay made no line draw');
    insist(ownWire.gl.gl_error===0,'OBRush transparent wire GL error');
    result.status='passed scoped pinned/OBRush transparency and GL checks';
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
