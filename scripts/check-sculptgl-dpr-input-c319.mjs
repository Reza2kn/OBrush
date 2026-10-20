// C319 post-selection baseline DPR and cursor probe in an owned headless Chrome.
// Usage: node scripts/check-sculptgl-dpr-input-c319.mjs WEB_DIST PINNED_MIRROR --dpr=2 [--baseline] [--build-commit=SHA]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-dpr-input-c319', randomUUID());
const dpr = Number(process.argv.find(x => x.startsWith('--dpr='))?.split('=')[1] || 2);
const baseline = process.argv.includes('--baseline');
const visibleSave = [345,26];
const buildCommit = process.argv.find(x => x.startsWith('--build-commit='))?.split('=')[1]
  || execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (![1, 2].includes(dpr)) throw Error(`Unsupported frozen DPR ${dpr}`);
const fixtureName = 'independent-resolution-dpr-c307.obj';
const fixture = await readFile(`crates/obrush-core/tests/fixtures/${fixtureName}`);
const fixturePath = path.join(out, fixtureName);
await mkdir(out, { recursive: true });
await writeFile(fixturePath, fixture);
const sha = b => createHash('sha256').update(b).digest('hex');
if (sha(fixture)!=='fab0a005d7c4cb666d4c17f1b6a5d115a4051abfcd0df254182c591307af2300') throw Error('C307 fixture changed');
const pause = ms => new Promise(r => setTimeout(r, ms));
const assert = (v, why) => { if (!v) throw Error(why); };
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
assert(sha(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle changed');
assert(sha(bundle)==='79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600', 'Wrong pinned source bundle');
assert(sha(await readFile(path.join(reference, 'index.html')))==='a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0', 'Wrong pinned mirror wrapper');
const result = {
  scope: 'C319 post-selection sphere baseline, paired DPR, cursor and placement',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { commit: buildCommit, wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { name:fixtureName, bytes: fixture.length, sha256: sha(fixture) },
  dpr, baseline, cases: [], screenshots: [], errors: [], status: 'running',
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
  window.__resolutionCheck={allocations:[],gl:null,inputs:[],downloads:[]};
  for(const type of ['pointerdown','mousedown','click','keydown'])document.addEventListener(type,e=>{const rows=window.__resolutionCheck.inputs;rows.push({type,target:e.target?.id||e.target?.tagName,clientX:e.clientX??null,clientY:e.clientY??null,key:e.key??null,pressure:e.pressure??null,pointerType:e.pointerType??null,at:performance.now()});if(rows.length>80)rows.shift();},true);
  for(const name of ['obrushDownload','obrushDownloadBytes']){let value;Object.defineProperty(window,name,{configurable:true,get(){return value},set(fn){value=(...a)=>{window.__resolutionCheck.downloads.push({name,filename:a[0],at:performance.now()});return fn(...a)}}});}
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.texImage2D!=='function'||gl.__resolutionWrapped)return gl;
    gl.__resolutionWrapped=true;window.__resolutionCheck.gl=gl;
    const texture=gl.texImage2D,storage=gl.renderbufferStorage;
    gl.texImage2D=function(...a){if(a.length>=9&&a[0]===gl.TEXTURE_2D){const rows=window.__resolutionCheck.allocations;rows.push({kind:'texture',width:a[3],height:a[4],at:performance.now()});if(rows.length>60)rows.shift();}return texture.apply(this,a);};
    gl.renderbufferStorage=function(...a){const rows=window.__resolutionCheck.allocations;rows.push({kind:'depth',width:a[2],height:a[3],at:performance.now()});if(rows.length>60)rows.shift();return storage.apply(this,a);};
    return gl;
  };
})();`;
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const v=a.getViewport(),c=a.getCanvas();return {dpr:window.devicePixelRatio,ratio:a._pixelRatio,css:[v.clientWidth,v.clientHeight],canvas:[c.width,c.height],history:a.getStateManager()._undos.length,meshes:a.getMeshes().length,gl_error:window.__resolutionCheck.gl?.getError()??null}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();const c=document.getElementById('obrush_canvas');return m?{dpr:window.devicePixelRatio,ratio:m.viewport_resolution_scale,render:m.viewport_resolution,canvas:[c.width,c.height],canvas_css:[c.clientWidth,c.clientHeight],canvas_rect:(()=>{let r=c.getBoundingClientRect();return [r.left,r.top,r.width,r.height]})(),zoom:window.visualViewport?.scale??1,camera:m.camera,brush:m.brush,background_cursor:m.background_cursor,history:m.history,selected:m.selected_subtool_ids,active_id:m.active_subtool_id,active_visible:m.active_subtool_visible,subtool_rows:m.subtools,message:m.message,dirty:m.dirty,inputs:window.__resolutionCheck.inputs.slice(-20),download_calls:window.__resolutionCheck.downloads.slice(),subtools:m.subtools.length,controls:m.layer_controls,scene_error:m.scene_error,gl_error:window.__resolutionCheck.gl?.getError()??null,allocations:window.__resolutionCheck.allocations.slice(-20)}:null})()`;
async function sample(name, info) {
  await pause(350);
  const record = { name, state: await evaluate(info) };
  const png = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), png);
  result.screenshots.push({ name, sha256: sha(png), size: [png.readUInt32BE(16), png.readUInt32BE(20)] });
  result.cases.push(record);
  return record.state;
}
async function hoverPair(label) {
  const scene = (await evaluate(ownInfo))?.camera?.viewport;
  assert(scene?.length===4, 'Missing Source viewport for hover probe');
  const state=await evaluate(ownInfo);const sx=dpr*state.canvas_css[0]/state.canvas[0],sy=dpr*state.canvas_css[1]/state.canvas[1];
  const pointer = [800,500];
  assert(pointer[0]>=scene[0]*sx&&pointer[0]<=scene[2]*sx&&pointer[1]>=scene[1]*sy&&pointer[1]<=scene[3]*sy,'C316 pointer outside visible scene');
  await send('Input.dispatchMouseEvent', { type:'mouseMoved', x:pointer[0], y:pointer[1], button:'none' });
  const hovered = await sample(`obrush-hover-${label}`, ownInfo);
  await send('Input.dispatchMouseEvent', { type:'mouseMoved', x:5, y:5, button:'none' });
  await sample(`obrush-away-${label}`, ownInfo);
  const observed=hovered.background_cursor?.pointer?.pixels;
  const expected=[(pointer[0]-scene[0]*sx)*dpr,(pointer[1]-scene[1]*sy)*dpr];
  const offset=observed?.map((x,i)=>Math.abs(x-expected[i]));
  result.cases.push({ hover_probe:{ label, pointer_css:pointer, viewport_css:scene,observed_pointer_pixels:observed,expected_pointer_pixels:expected,pointer_error_pixels:offset,
    brush_screen_radius: hovered.brush?.screen_radius_pixels,
    source_cursor: hovered.background_cursor } });
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
  const prior = await evaluate(ownInfo);
  await click(...visibleSave);
  let event = null;
  for(let i=0;i<50;i++){
    event=downloadsSeen.slice(before).find(x=>x.state==='completed');
    const now=await evaluate(ownInfo);
    if(event||now.message!==prior.message||now.dirty!==prior.dirty||now.download_calls.length!==prior.download_calls.length)break;
    await pause(100);
  }
  const after=await evaluate(ownInfo);
  if(after.download_calls.length>prior.download_calls.length&&!event){
    for(let i=0;i<150;i++){event=downloadsSeen.slice(before).find(x=>x.state==='completed');if(event)break;await pause(100);}
  }
  const classification=event?'completed':after.message?.startsWith('Download requested:')?'download-request-without-completion':
    after.message!==prior.message?'application-message-without-download':'visible-input-miss';
  const record={name,visible_css:visibleSave,before:{message:prior.message,dirty:prior.dirty},
    after:{message:after.message,dirty:after.dirty,inputs:after.inputs,download_calls:after.download_calls},
    cdp_downloads:downloadsSeen.slice(before),classification};
  if(event){const bytes=await readFile(path.join(downloads,event.guid));Object.assign(record,{bytes:bytes.length,sha256:sha(bytes)});}
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
  await send('Emulation.setDeviceMetricsOverride', { width:1600, height:1000, deviceScaleFactor:dpr, mobile:false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source:instrument });
  const base=`http://127.0.0.1:${server.address().port}`;
  await send('Page.navigate', { url:`${base}/reference/` });
  await until(async () => (await evaluate(refInfo))?.meshes, 'reference startup');
  await fileInput('#fileopen');
  await until(async () => (await evaluate(refInfo))?.meshes===3, 'reference fixture import');
  const referenceDefault = await sample('reference-1', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui().onPixelRatio(0.5)');
  const referenceHalf = await sample('reference-0.5', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui().onPixelRatio(2)');
  const referenceDouble = await sample('reference-2', refInfo);
  for (const [state, ratio] of [[referenceDefault,1],[referenceHalf,.5],[referenceDouble,2]]) {
    assert(state.ratio===ratio, `Reference ratio ${state.ratio} != ${ratio}`);
    assert(state.dpr===dpr, `Reference DPR ${state.dpr} != ${dpr}`);
    assert(state.canvas[0]===state.css[0]*ratio&&state.canvas[1]===state.css[1]*ratio, 'Reference canvas backing mismatch');
    assert(state.history===referenceDefault.history, 'Reference resolution changed history');
    assert(state.gl_error===0, `Reference GL error at ${ratio}`);
  }
  await send('Page.navigate', { url:`${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools===1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount=choosers.length;await key('o',2);await until(async()=>choosers.length>chooserCount,'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  await until(async () => (await evaluate(ownInfo))?.subtools===3, 'OBRush fixture import');
  const ownDefault = await sample('obrush-1', ownInfo);
  await click(1450,240);
  const selectedSphere=await sample('obrush-active-sphere',ownInfo);
  assert(selectedSphere.active_id===selectedSphere.subtool_rows?.[0]?.id && selectedSphere.active_visible===true,`C316 visible sphere selection failed: ${selectedSphere.active_id} vs ${selectedSphere.subtool_rows?.[0]?.id}`);
  await hoverPair('1');
  const projectDefault = await save('default');
  await sample('obrush-after-visible-save', ownInfo);
  if(projectDefault.classification!=='completed')throw Error(`Visible Save failed: ${projectDefault.classification}`);
  const rect = await ownResolutionRect();
  const halfScale=[dpr*ownDefault.canvas_css[0]/ownDefault.canvas[0],dpr*ownDefault.canvas_css[1]/ownDefault.canvas[1]];
  const halfVisible=[(rect[0]+4)*halfScale[0],(rect[1]+rect[3])/2*halfScale[1]];result.cases.push({slider_visible_half:{egui_rect:rect,pointer_css:halfVisible}});
  await click(...halfVisible);
  const ownHalf = await sample('obrush-0.5', ownInfo);
  const projectHalf = await save('half');
  const secondRect = await ownResolutionRect();
  const doubleVisible=[(secondRect[2]-4)*halfScale[0],(secondRect[1]+secondRect[3])/2*halfScale[1]];result.cases.push({slider_visible_double:{egui_rect:secondRect,pointer_css:doubleVisible}});
  await click(...doubleVisible);
  const ownDouble = await sample('obrush-2', ownInfo);
  await hoverPair('2');
  const projectDouble = await save('double');
  const resetRect = await ownResolutionRect();
  const resetVisible=[(resetRect[0]+(resetRect[2]-resetRect[0])/3)*halfScale[0],(resetRect[1]+resetRect[3])/2*halfScale[1]];result.cases.push({slider_visible_reset:{egui_rect:resetRect,pointer_css:resetVisible}});
  await click(...resetVisible);
  const ownReset = await sample('obrush-1-return', ownInfo);
  const projectReset = await save('return');
  result.control_rect = rect;
  result.control_rect_after_save = secondRect;
  assert(ownDefault.dpr===dpr && ownDefault.ratio===1, 'OBRush DPR or default ratio differs');
  assert(ownHalf.ratio>=.5&&ownHalf.ratio<=.54, `OBRush half control selected ${ownHalf.ratio}`);
  assert(ownDouble.ratio>=1.96&&ownDouble.ratio<=2, `OBRush double control selected ${ownDouble.ratio}`);
  assert(ownReset.ratio===1, `OBRush exact 1x return failed: ${ownReset.ratio}`);
  assert(ownDefault.canvas.join(',')===ownHalf.canvas.join(',')&&ownDefault.canvas.join(',')===ownDouble.canvas.join(','), 'OBRush UI canvas changed backing instead of only viewport scene');
  const cssScene = ownDefault.camera?.source_framing?.user_points;
  const hostScene = ownDefault.camera?.source_framing?.render_pixels;
  assert(cssScene?.length===2 && hostScene?.length===2, 'Missing measured Source viewport');
  result.scene_dimensions = { css:cssScene, host:hostScene };
  const states = [ownDefault, ownHalf, ownDouble];
  const targetErrors = states.map(state => state.render.pixels.map((actual,i) =>
    Math.abs(actual-Math.round(cssScene[i]*state.ratio))));
  result.css_target_errors = targetErrors;
  result.cases.push({canvas_dpr_invariant:{actual:ownDefault.canvas,expected:ownDefault.canvas_css.map(x=>Math.round(x*dpr)),passes:ownDefault.canvas.every((x,i)=>Math.abs(x-Math.round(ownDefault.canvas_css[i]*dpr))<=1)}});
  assert(result.cases.at(-1).canvas_dpr_invariant.passes,'Canvas backing differs from CSS×DPR');
  for (const state of states) {
    const expected=ownDefault.render.pixels.map(x=>Math.round(x*state.ratio));
    assert(state.render.pixels.every((x,i)=>Math.abs(x-expected[i])<=1), `OBRush render target mismatch: ${state.render.pixels} vs ${expected}`);
    assert(!state.render.error&&!state.scene_error&&state.gl_error===0, `OBRush scaled render error: ${JSON.stringify(state)}`);
    if (state.render.pixels.join(',')!==hostScene.join(',')) {
      assert(state.allocations.some(x=>x.kind==='texture'&&x.width===state.render.pixels[0]&&x.height===state.render.pixels[1]), `No actual ${state.ratio} GL texture allocation`);
    }
    assert(JSON.stringify(state.history)===JSON.stringify(ownDefault.history), 'OBRush resolution changed document history');
    assert(JSON.stringify(state.selected)===JSON.stringify(selectedSphere.selected), 'OBRush resolution changed selection');
  }
  assert(projectDefault.sha256===projectHalf.sha256&&projectHalf.sha256===projectDouble.sha256, 'OBRush resolution changed public project bytes');
  assert(projectDefault.sha256===projectReset.sha256, 'OBRush resolution return changed public project bytes');
  assert(JSON.stringify(ownReset.history)===JSON.stringify(ownDefault.history), 'OBRush resolution return changed history');
  assert(JSON.stringify(ownReset.selected)===JSON.stringify(selectedSphere.selected), 'OBRush resolution return changed selection');
  assert(new Set(result.screenshots.filter(x=>['obrush-1','obrush-0.5','obrush-2'].includes(x.name)).map(x=>x.sha256)).size===3, 'OBRush screenshots do not respond to resolution');
  result.status=baseline?'recorded C319 baseline':'passed C319 browser dimensions and file gates; cursor analyzer and mouse/pen pick pending';
} catch (error) {
  result.status='failed';result.errors.push(String(error.stack||error));
  if(result.cases.some(x=>x.save?.classification==='visible-input-miss')){
    try{
      await send('Page.navigate',{url:`http://127.0.0.1:${server.address().port}/?obrushprofile=sculptgl`});
      await until(async()=>(await evaluate(ownInfo))?.subtools===1,'fresh diagnostic startup');
      const count=choosers.length;await key('o',2);await until(async()=>choosers.length>count,'fresh diagnostic chooser');
      await fileInput('#source-sgl-file');await until(async()=>(await evaluate(ownInfo))?.subtools===3,'fresh diagnostic fixture');
      const before=await evaluate(ownInfo);const point=[visibleSave[0]/dpr,visibleSave[1]/dpr];await click(...point);
      const after=await evaluate(ownInfo);result.cases.push({diagnostic_half_coordinate:{point_css:point,before:{message:before.message,dirty:before.dirty},after:{message:after.message,dirty:after.dirty},downloads:after.download_calls}});
    }catch(diagnosticError){result.errors.push(`Diagnostic fresh-page click failed: ${diagnosticError}`);}
  }
} finally {
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  try { if(socket?.readyState===WebSocket.OPEN) await send('Browser.close',{},null); } catch {}
  socket?.close();if(chrome?.exitCode===null)chrome.kill();server?.close();
}
console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),errors:result.errors}));
if(result.status==='failed')process.exitCode=1;
