// V04 single-mesh resolution image-quality gate in an owned headless Chrome.
// Usage: node scripts/check-sculptgl-resolution-single-mesh-v04.mjs WEB_DIST PINNED_MIRROR --dpr=2 --product-commit=SHA
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-resolution-single-mesh-v04', randomUUID());
const dpr = Number(process.argv.find(x => x.startsWith('--dpr='))?.split('=')[1] || 2);
if (![1, 2].includes(dpr)) throw Error(`Unsupported frozen DPR ${dpr}`);
const productCommit = process.argv.find(x => x.startsWith('--product-commit='))?.split('=')[1];
if(!process.argv.includes('--self-check')&&!/^[a-f0-9]{40}$/.test(productCommit||''))throw Error('Pass the exact product source commit via --product-commit=SHA');
const fixtureName = 'independent-resolution-dense-r03.obj';
const fixture = await readFile(`crates/obrush-core/tests/fixtures/${fixtureName}`);
const fixturePath = path.join(out, fixtureName);
await mkdir(out, { recursive: true });
await writeFile(fixturePath, fixture);
const sha = b => createHash('sha256').update(b).digest('hex');
if (sha(fixture)!=='492dd0cfa56544bab0e816efa68a4b48b522ddaf5c70cbb2f95b6c92d167ec2e') throw Error('V04 fixture changed');
const pause = ms => new Promise(r => setTimeout(r, ms));
const assert = (v, why) => { if (!v) throw Error(why); };
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
assert(sha(bundle) === manifest.original_bundle.sha256, 'Pinned SculptGL bundle changed');
const result = {
  scope: 'V04 single-mesh pixel and geometry gate at 0.5/1/2 resolution, independent DPR1/2 runs; frozen preregistration',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { product_commit:productCommit||null, harness_commit:execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), harness_sha256:sha(await readFile(new URL(import.meta.url))), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { name:fixtureName, bytes: fixture.length, sha256: sha(fixture) },
  dpr, cases: [], screenshots: [], errors: [], status: 'running',
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
async function clickVisibleText(label) {
  const matches=await evaluate(`(()=>Array.from(document.querySelectorAll('*')).filter(e=>{const r=e.getBoundingClientRect();const exact=e.textContent?.trim()===${JSON.stringify(label)};const sceneMenu=${JSON.stringify(label)}==='Scene'&&e.tagName==='LI'&&e.textContent?.trim().startsWith('SceneClear scene');return (exact||sceneMenu)&&r.width>4&&r.height>4&&r.x>=0&&r.y>=0&&r.right<=1600&&r.bottom<=1000}).map(e=>{const r=e.getBoundingClientRect();return {tag:e.tagName,x:r.x+r.width/2,y:r.y+r.height/2,area:r.width*r.height}}).sort((a,b)=>a.area-b.area).slice(0,6))()`);
  if(!matches?.length){
    const diagnostic=await evaluate(`(()=>({ready:document.readyState,body:document.body.innerText.slice(0,500),candidates:Array.from(document.querySelectorAll('*')).filter(e=>e.textContent?.includes(${JSON.stringify(label)})).map(e=>{const r=e.getBoundingClientRect();return {tag:e.tagName,text:e.textContent?.trim().slice(0,60),rect:[r.x,r.y,r.right,r.bottom]}}).slice(0,20)}))()`);
    result.cases.push({ui_text_failure:{label,diagnostic}});
    throw Error(`Missing visible normal-UI text ${label}: ${JSON.stringify(diagnostic)}`);
  }
  const target=matches[0];result.cases.push({ui_click:{label,target,matches}});
  await click(target.x,target.y);
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
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const v=a.getViewport(),c=a.getCanvas(),r=c.getBoundingClientRect(),mesh=a.getMesh(),tool=a.getSculptManager().getCurrentTool(),meshes=a.getMeshes(),camera=a.getCamera();return {dpr:devicePixelRatio,ratio:a._pixelRatio,css:[v.clientWidth,v.clientHeight],canvas:[c.width,c.height],rect:[r.left,r.top,r.width,r.height],history:a.getStateManager()._undos.length,meshes:meshes.length,visible:meshes.filter(m=>m.isVisible()).length,selected_count:a.getSelectedMeshes().length,selected_id:mesh?.getID(),vertices:mesh?.getNbVertices(),triangles:mesh?.getNbTriangles(),symmetry:a.getSculptManager()._symmetry,brush:{radius:tool?._radius,intensity:tool?._intensity},visual:{shader:mesh?.getShaderType(),flat:mesh?.getFlatShading(),wire:mesh?.getShowWireframe(),background:a.getBackground()?._color,grid:a._showGrid},camera_pose:{quat:camera._quatRot,trans:camera._trans,center:camera._center},gl_error:window.__resolutionCheck.gl?.getError()??null}})()`;
const refVertices = `(()=>{const mesh=window.__sculptglReferenceInstance?.getMesh();return mesh?Array.from(mesh.getVertices()):null})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.(),c=document.getElementById('obrush_canvas');return m?{dpr:devicePixelRatio,ratio:m.viewport_resolution_scale,render:m.viewport_resolution,canvas:[c.width,c.height],canvas_css:[c.clientWidth,c.clientHeight],camera:m.camera,history:m.history,selected:m.selected_subtool_ids,active_id:m.active_subtool_id,kind:m.kind,radius:m.radius,strength:m.strength,symmetry:m.symmetry_x,subtools:m.subtools.length,parts:m.subtools,vertices:m.vertices,triangles:m.triangles,visual:{shading:m.viewport_shading,flat:m.flat_shading,matcap:m.matcap?.selected,environment:m.environment?.preset,background:m.environment?.background_mode,grid:m.display_flags?.show_grid},controls:m.layer_controls,morph_controls:m.morph_controls,sculpt:window.obrushSculptMetrics?.(),background_cursor:m.background_cursor,scene_error:m.scene_error,gl_error:window.__resolutionCheck.gl?.getError()??null,allocations:window.__resolutionCheck.allocations.slice(-20)}:null})()`;
async function sample(name, info, settleMs=350) {
  await pause(settleMs);
  const record = { name, state: await evaluate(info) };
  const png = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), png);
  result.screenshots.push({ name, sha256: sha(png), size: [png.readUInt32BE(16), png.readUInt32BE(20)] });
  result.cases.push(record);
  return record.state;
}
function displacement(row) {
  assert(row.before.length%3===0&&row.after.length%3===0&&row.after.length>=row.before.length,'V04 invalid or shrinking position arrays');
  const changed=[];let total=0,max=0;
  for(let i=0;i<row.before.length;i+=3){
    const d=Math.hypot(row.after[i]-row.before[i],row.after[i+1]-row.before[i+1],row.after[i+2]-row.before[i+2]);
    if(d>1e-10){changed.push(i/3);total+=d;max=Math.max(max,d)}
  }
  return {vertex_count_before:row.before.length/3,vertex_count_after:row.after.length/3,added_vertices:(row.after.length-row.before.length)/3,changed_original_vertices:changed.length,changed_ids_sha256:sha(Buffer.from(JSON.stringify(changed))),total_original_displacement:total,max_original_displacement:max,ids:changed};
}
function comparePositions(label, baseline, candidate) {
  assert(baseline.before.length===candidate.before.length&&baseline.after.length===candidate.after.length,`${label} vertex count differs`);
  const a=displacement(baseline).ids, b=displacement(candidate).ids;
  let maxBeforeDelta=0,maxDelta=0;
  for(let i=0;i<baseline.before.length;i++){
    maxBeforeDelta=Math.max(maxBeforeDelta,Math.abs(baseline.before[i]-candidate.before[i]));
  }
  for(let i=0;i<baseline.after.length;i++){
    maxDelta=Math.max(maxDelta,Math.abs(baseline.after[i]-candidate.after[i]));
  }
  const record={label,changed_baseline:a.length,changed_candidate:b.length,changed_ids_equal:JSON.stringify(a)===JSON.stringify(b),max_before_coordinate_delta:maxBeforeDelta,max_after_coordinate_delta:maxDelta};
  result.cases.push({geometry_comparison:record});
  assert(a.length>=24&&b.length>=24&&record.changed_ids_equal&&maxBeforeDelta<=1e-5&&maxDelta<=1e-5,`${label} geometry differs ${JSON.stringify(record)}`);
}
function maxNestedNumericDelta(a,b){
  if(typeof a==='number'&&typeof b==='number')return Math.abs(a-b);
  if(Array.isArray(a)&&Array.isArray(b)&&a.length===b.length)return Math.max(0,...a.map((v,i)=>maxNestedNumericDelta(v,b[i])));
  if(a&&b&&typeof a==='object'&&typeof b==='object'&&JSON.stringify(Object.keys(a))===JSON.stringify(Object.keys(b)))return Math.max(0,...Object.keys(a).map(k=>maxNestedNumericDelta(a[k],b[k])));
  return a===b?0:Infinity;
}
{const sparse={before:Array(72).fill(0),after:Array(72).fill(0)};
 sparse.after[0]=1;assert(displacement(sparse).changed_original_vertices===1,'V04 sparse self-check failed');
 const dense={before:Array(72).fill(0),after:Array(72).fill(0)};
 for(let i=0;i<24;i++)dense.after[i*3]=1;
 assert(displacement(dense).changed_original_vertices===24,'V04 dense self-check failed');
 const grown={before:Array(72).fill(0),after:Array(75).fill(0)};
 assert(displacement(grown).added_vertices===1,'V04 topology-growth self-check failed');
 assert(maxNestedNumericDelta({x:[1,2]},{x:[1,2+1e-14]})<1e-10,'V04 camera tolerance self-check failed');}
if(process.argv.includes('--self-check')){console.log('V04 static preflight passed');process.exit(0)}
async function contact(pointer) {
  await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:pointer[0],y:pointer[1],button:'none',pointerType:'mouse',force:.5});
  await pause(100);
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x:pointer[0],y:pointer[1],button:'left',buttons:1,clickCount:1,pointerType:'mouse',force:.5});
  await pause(60);
  await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:pointer[0]+12,y:pointer[1],button:'left',buttons:1,pointerType:'mouse',force:.5});
  await pause(120);
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x:pointer[0]+12,y:pointer[1],button:'left',buttons:0,clickCount:1,pointerType:'mouse',force:0});
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
async function ownGridRect() {
  for(let i=0;i<35;i++){
    const rect=(await evaluate(ownInfo))?.controls?.['display.grid'];
    if(rect&&rect[1]>80&&rect[3]<990)return rect;
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:1450,y:750,button:'none'});
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:1450,y:750,deltaX:0,deltaY:rect?.[3]>990?350:-350});
    await pause(100);
  }
  throw Error('OBRush visible Show grid control unavailable');
}
async function ownSymmetryRect() {
  for(let i=0;i<35;i++){
    const rect=(await evaluate(ownInfo))?.morph_controls?.['settings.symmetry'];
    if(rect&&rect[1]>80&&rect[3]<990)return rect;
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:120,y:750,button:'none'});
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:120,y:750,deltaX:0,deltaY:rect?.[3]>990?350:-350});
    await pause(100);
  }
  throw Error('OBRush visible Symmetry checkbox unavailable');
}
async function save(name) {
  const before = downloadsSeen.length;
  await click(345, 26);
  const event = await until(async () => downloadsSeen.slice(before).find(x => x.state === 'completed'), `project save ${name}`);
  const bytes = await readFile(path.join(downloads, event.guid));
  const record = { name, bytes: bytes.length, sha256: sha(bytes), guid:event.guid };
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
    if (message.method === 'Page.javascriptDialogOpening') {
      result.cases.push({normal_ui_dialog:message.params});
      void send('Page.handleJavaScriptDialog',{accept:true}).catch(error=>result.errors.push(`Scene Reset dialog: ${error}`));
    }
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
  const rows={reference:[],obrush:[]};
  for(const ratio of [1,.5,2]){
    await send('Page.navigate', { url:`${base}/reference/` });
    await until(async () => (await evaluate(refInfo))?.meshes===1, `reference ${ratio} startup`);
    await clickVisibleText('Scene');
    await clickVisibleText('Clear scene');
    await until(async () => (await evaluate(refInfo))?.meshes===0, `reference ${ratio} normal Scene Reset`);
    await fileInput('#fileopen');
    await until(async () => (await evaluate(refInfo))?.meshes===1, `reference ${ratio} import`);
    const sourceBeforeGrid=await evaluate(refInfo);
    assert(sourceBeforeGrid.visual?.grid===true,`reference ${ratio} initial Show grid was not on`);
    await clickVisibleText('Show grid');
    const sourceAfterGrid=await evaluate(refInfo);
    assert(sourceAfterGrid.visual?.grid===false,`reference ${ratio} visible Show grid off failed`);
    assert(sourceBeforeGrid.selected_id===sourceAfterGrid.selected_id&&sourceBeforeGrid.vertices===sourceAfterGrid.vertices&&sourceBeforeGrid.history===sourceAfterGrid.history&&JSON.stringify(sourceBeforeGrid.camera_pose)===JSON.stringify(sourceAfterGrid.camera_pose)&&JSON.stringify(sourceBeforeGrid.brush)===JSON.stringify(sourceAfterGrid.brush)&&sourceBeforeGrid.ratio===sourceAfterGrid.ratio,`reference ${ratio} grid UI changed model/camera/brush/ratio`);
    result.cases.push({normal_ui_reference_grid_off:{ratio,selected_id:sourceAfterGrid.selected_id,history:sourceAfterGrid.history,grid:sourceAfterGrid.visual.grid}});
    const sourceSymmetry=await evaluate("(()=>{const gui=window.__sculptglReferenceInstance.getGui()._ctrlSculpting;gui.onSymmetryChange(false);gui._ctrlSymmetry.setValue(false,true);return {controller:gui._ctrlSymmetry.getValue(),sculpt:window.__sculptglReferenceInstance.getSculptManager()._symmetry}})()");
    result.cases.push({nonvisible_source_symmetry_controller_setup:{ratio,sourceSymmetry}});
    assert(sourceSymmetry?.controller===false&&sourceSymmetry?.sculpt===false,`reference ${ratio} controller Symmetry setup`);
    assert((await evaluate(refInfo))?.symmetry===false,`reference ${ratio} Symmetry off`);
    if(ratio!==1)await evaluate(`window.__sculptglReferenceInstance.getGui().onPixelRatio(${ratio})`);
    const sourceBeforeMenus=await evaluate(refInfo);
    await click(1500,20);
    const readMenus="(()=>Array.from(document.querySelectorAll('.gui-topbar > ul > li > ul')).map(e=>({opacity:Number(getComputedStyle(e).opacity),pointerEvents:getComputedStyle(e).pointerEvents,rect:[e.getBoundingClientRect().x,e.getBoundingClientRect().y,e.getBoundingClientRect().width,e.getBoundingClientRect().height]})))()";
    const hidden=items=>items?.length>0&&items.every(e=>e.opacity<=0.001&&e.pointerEvents==='none');
    const menuFirst=await until(async()=>{const items=await evaluate(readMenus);return hidden(items)?items:null},`reference ${ratio} all dropdowns hidden`);
    await pause(350);
    const menuStable=await evaluate(readMenus);
    assert(hidden(menuStable),`reference ${ratio} dropdown appeared after 350ms: ${JSON.stringify(menuStable)}`);
    const sourceAfterMenus=await evaluate(refInfo);
    assert(sourceBeforeMenus.selected_id===sourceAfterMenus.selected_id&&sourceBeforeMenus.vertices===sourceAfterMenus.vertices&&sourceBeforeMenus.history===sourceAfterMenus.history&&JSON.stringify(sourceBeforeMenus.camera_pose)===JSON.stringify(sourceAfterMenus.camera_pose)&&JSON.stringify(sourceBeforeMenus.brush)===JSON.stringify(sourceAfterMenus.brush)&&sourceBeforeMenus.ratio===sourceAfterMenus.ratio,`reference ${ratio} menu UI changed model/camera/brush/ratio`);
    result.cases.push({normal_ui_all_dropdowns_hidden:{ratio,click_css:[1500,20],first:menuFirst,stable:menuStable,wait_ms:350,before:{selected_id:sourceBeforeMenus.selected_id,vertices:sourceBeforeMenus.vertices,history:sourceBeforeMenus.history,camera_pose:sourceBeforeMenus.camera_pose,brush:sourceBeforeMenus.brush,ratio:sourceBeforeMenus.ratio},after:{selected_id:sourceAfterMenus.selected_id,vertices:sourceAfterMenus.vertices,history:sourceAfterMenus.history,camera_pose:sourceAfterMenus.camera_pose,brush:sourceAfterMenus.brush,ratio:sourceAfterMenus.ratio}}});
    const before=await sample(`reference-${ratio}-before`,refInfo);
    const pointer=[Math.round(before.rect[0]+before.rect[2]/2),Math.round(before.rect[1]+before.rect[3]/2)];
    assert(before.dpr===dpr&&before.ratio===ratio&&before.canvas.every((x,i)=>Math.abs(x-Math.round(before.css[i]*ratio))<=1),`reference ${ratio} dimensions`);
    assert(before.meshes===1&&before.visible===1&&before.selected_count===1&&before.selected_id!==undefined&&before.vertices===18242&&before.triangles===36480&&before.symmetry===false&&before.visual?.wire===false&&before.visual?.grid===false&&before.gl_error===0,`reference ${ratio} single-mesh preflight`);
    assert(before.brush.radius===50&&Math.abs(before.brush.intensity-.5)<=1e-6,`reference ${ratio} brush differs`);
    assert(pointer[0]+12<before.rect[0]+before.rect[2]&&pointer[1]<before.rect[1]+before.rect[3],`reference ${ratio} stroke exits canvas`);
    const positionsBefore=await evaluate(refVertices);
    assert(positionsBefore?.length>0,`reference ${ratio} vertices missing`);
    await contact(pointer);
    const after=await sample(`reference-${ratio}-after`,refInfo);
    await pause(500);
    await sample(`reference-${ratio}-stable`,refInfo,0);
    const positionsAfter=await evaluate(refVertices);
    const row={app:'reference',ratio,dpr,pointer,history_before:before.history,history_after:after.history,selected_before:before.selected_id,selected_after:after.selected_id,brush:before.brush,visual_before:before.visual,camera_before:before.camera_pose,before:positionsBefore,after:positionsAfter};
    const change=displacement(row);const {ids,...summary}=change;
    result.cases.push({contact:{...row,before:undefined,after:undefined,geometry:summary}});
    if(rows.reference.length)assert(JSON.stringify(pointer)===JSON.stringify(rows.reference[0].pointer),'Reference CSS contact changed between ratios');
    rows.reference.push(row);
    assert(after.history===before.history+1&&after.selected_id===before.selected_id&&after.meshes===1&&after.visible===1&&change.changed_original_vertices>=24,`reference ${ratio} footprint below 24 vertices: ${JSON.stringify(summary)}`);
  }
  for(const ratio of [1,.5,2]){
    await send('Page.navigate', { url:`${base}/?obrushprofile=sculptgl` });
    await until(async () => (await evaluate(ownInfo))?.subtools===1, `OBRush ${ratio} startup`);
    await sample(`obrush-${ratio}-startup-open-target`,ownInfo);
    const count=choosers.length;await click(272,26);await until(async()=>choosers.length>count,`OBRush ${ratio} visible Open chooser`);
    await fileInput('#import-file');
    await until(async () => (await evaluate(ownInfo))?.subtools===1&&(await evaluate(ownInfo))?.vertices===18242, `OBRush ${ratio} whole-document OBJ open`);
    const symmetryRect=await ownSymmetryRect();
    await click((symmetryRect[0]+symmetryRect[2])/2,(symmetryRect[1]+symmetryRect[3])/2);
    await until(async () => (await evaluate(ownInfo))?.symmetry===false, `OBRush ${ratio} Symmetry off`);
    if(ratio!==1){
      const rect=await ownResolutionRect();
      await click(ratio===.5?rect[0]+4:rect[2]-4,(rect[1]+rect[3])/2);
    }
    assert((await evaluate(ownInfo))?.visual?.grid===true,`OBRush ${ratio} initial Show grid was not on`);
    const gridRect=await ownGridRect();
    await click((gridRect[0]+gridRect[2])/2,(gridRect[1]+gridRect[3])/2);
    assert((await evaluate(ownInfo))?.visual?.grid===false,`OBRush ${ratio} visible Show grid off failed`);
    result.cases.push({normal_ui_obrush_grid_off:{ratio,control_rect:gridRect,grid:false}});
    await evaluate("document.getElementById('obrush_canvas').focus()");
    const before=await sample(`obrush-${ratio}-before`,ownInfo);
    const vp=before.camera?.viewport;
    assert(vp?.length===4,`OBRush ${ratio} viewport missing`);
    const pointer=[Math.round((vp[0]+vp[2])/2),Math.round((vp[1]+vp[3])/2)];
    assert(before.dpr===dpr&&Math.abs(before.ratio-ratio)<=.000001&&before.selected?.length===1&&before.subtools===1&&before.parts?.[0]?.visible&&before.vertices===18242&&before.triangles===36480&&before.symmetry===false&&before.visual?.grid===false&&before.kind==='Draw'&&Math.abs(before.radius-.18)<1e-6&&Math.abs(before.strength-.35)<1e-6,`OBRush ${ratio} single-mesh preflight`);
    assert(!before.scene_error&&before.gl_error===0,`OBRush ${ratio} render error`);
    const scene=before.camera?.source_framing?.user_points;
    assert(scene?.length===2&&before.render.pixels.every((x,i)=>Math.abs(x-Math.round(scene[i]*before.ratio))<=1),`OBRush ${ratio} scene pixels`);
    assert(before.allocations.some(x=>x.kind==='texture'&&x.width===before.render.pixels[0]&&x.height===before.render.pixels[1]),`OBRush ${ratio} actual GL target absent`);
    assert(pointer[0]+12<vp[2]&&pointer[1]<vp[3],`OBRush ${ratio} stroke exits viewport`);
    const prior=await save(`obrush-${ratio}-before`);
    const projectBefore=JSON.parse(await readFile(path.join(downloads,prior.guid),'utf8'));
    await evaluate("document.getElementById('obrush_canvas').focus()");
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:pointer[0],y:pointer[1],button:'none',pointerType:'mouse',force:.5});
    const hover=await sample(`obrush-${ratio}-hover`,ownInfo);
    const local=hover.background_cursor?.pointer?.pixels;
    const expected=[(pointer[0]-vp[0])*dpr,(pointer[1]-vp[1])*dpr];
    const pointerError=local?.map((v,i)=>Math.abs(v-expected[i]));
    result.cases.push({cursor:{ratio,pointer_css:pointer,local_pixels:local,expected_local_pixels:expected,error_pixels:pointerError}});
    assert(pointerError?.every(e=>e<=2*dpr),`OBRush ${ratio} cursor placement error ${pointerError}`);
    await contact(pointer);
    const after=await sample(`obrush-${ratio}-after`,ownInfo);
    await pause(500);
    await sample(`obrush-${ratio}-stable`,ownInfo,0);
    const saved=await save(`obrush-${ratio}-after`);
    const projectAfter=JSON.parse(await readFile(path.join(downloads,saved.guid),'utf8'));
    const flat=positions=>positions.flat();
    const otherBefore=(projectBefore.scene?.objects||[]).filter(x=>x.id!==before.active_id),otherAfter=(projectAfter.scene?.objects||[]).filter(x=>x.id!==before.active_id);
    const row={app:'obrush',ratio,dpr,pointer,history_before:before.history,history_after:after.history,selected_before:before.selected,selected_after:after.selected,visual_before:before.visual,camera_before:{position64:before.camera.position64,target:before.camera.precise_target,view64:before.camera.view64,fov:before.camera.fov_degrees},before:flat(projectBefore.mesh.positions),after:flat(projectAfter.mesh.positions),project_before_sha256:prior.sha256,project_after_sha256:saved.sha256,sculpt_before:before.sculpt,sculpt_after:after.sculpt,other_objects_equal:JSON.stringify(otherBefore)===JSON.stringify(otherAfter)};
    const change=displacement(row);const {ids,...summary}=change;
    result.cases.push({contact:{...row,before:undefined,after:undefined,geometry:summary}});
    if(rows.obrush.length)assert(JSON.stringify(pointer)===JSON.stringify(rows.obrush[0].pointer),'OBRush CSS contact changed between ratios');
    rows.obrush.push(row);
    assert(after.history.undo===before.history.undo+1&&JSON.stringify(after.selected)===JSON.stringify(before.selected)&&after.subtools===1&&after.parts?.[0]?.visible&&row.other_objects_equal&&change.changed_original_vertices>=24,`OBRush ${ratio} footprint below 24 vertices: ${JSON.stringify(summary)}`);
    assert(after.sculpt?.released_strokes===before.sculpt?.released_strokes+1,`OBRush ${ratio} release count differs`);
  }
  for(const app of ['reference','obrush'])for(const row of rows[app].slice(1))comparePositions(`${app} ${row.ratio} vs 1 DPR${dpr}`,rows[app][0],row);
  for(const app of ['reference','obrush'])for(const row of rows[app].slice(1)){
    assert(JSON.stringify(row.visual_before)===JSON.stringify(rows[app][0].visual_before),`${app} visual settings changed with ratio`);
    const camera_delta=maxNestedNumericDelta(row.camera_before,rows[app][0].camera_before);
    result.cases.push({camera_comparison:{app,ratio:row.ratio,max_coordinate_delta:camera_delta}});
    assert(camera_delta<=1e-10,`${app} camera pose changed with ratio by ${camera_delta}`);
  }
  assert(rows.reference.every(x=>x.history_before===rows.reference[0].history_before),'Reference pre-contact history changed by resolution');
  assert(rows.obrush.every(x=>JSON.stringify(x.history_before)===JSON.stringify(rows.obrush[0].history_before)),'OBRush pre-contact history changed by resolution');
  assert(new Set(rows.obrush.map(x=>x.project_before_sha256)).size===1,'OBRush resolution changed before-contact project bytes');
  result.status='passed V04 geometry/history bounds; pixel analysis pending';
} catch (error) {
  result.status='failed';result.errors.push(String(error.stack||error));
} finally {
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  try { if(socket?.readyState===WebSocket.OPEN) await send('Browser.close',{},null); } catch {}
  socket?.close();if(chrome?.exitCode===null)chrome.kill();server?.closeAllConnections?.();server?.close();
}
if(result.status!=='failed'){
  try{
    const script=path.resolve('scripts/analyze-sculptgl-resolution-single-mesh-v04.py');
    execFileSync('python3',[script,path.join(out,'result.json')],{encoding:'utf8',stdio:['ignore','pipe','pipe']});
    result.status='passed V04 single-mesh pixel and geometry gates';
    result.pixel_analysis={path:'pixel-analysis.json',sha256:sha(await readFile(path.join(out,'pixel-analysis.json')))};
  }catch(error){result.status='failed';result.errors.push(`V04 pixel analysis failed: ${error.stderr?.toString()||error}`)}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
}
console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),errors:result.errors}));
process.exit(result.status==='failed'?1:0);
