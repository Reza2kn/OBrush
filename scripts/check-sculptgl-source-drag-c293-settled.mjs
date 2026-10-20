// C293 read-only normal-UI settled-camera preflight, before any Drag candidate.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const { OwnedCdp } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const side = process.argv.find(arg => arg.startsWith('--side='))?.slice(7);
if (!['source', 'root'].includes(side)) throw Error('Use --side=source or --side=root');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fixture = path.join(root, 'tests/fixtures/independent-source-drag-c286.sgl');
const expected = JSON.parse(await readFile(path.join(root, 'tests/fixtures/independent-source-drag-c286.json')));
const fixtureBytes = await readFile(fixture);
if (sha(fixtureBytes) !== 'fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e' ||
    sha(fixtureBytes) !== expected.sha256 || fixtureBytes.length !== expected.bytes) throw Error('C293 fixture changed');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const referenceHash = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (referenceHash !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600')
  throw Error('C293 pinned SculptGL bundle changed');
const web = path.resolve(process.env.OBRUSH_WEB_DIST ?? path.join(root, 'web/dist'));
const out = path.join(root, '.artifacts/sculptgl-source-drag-c293-settled', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {
  protocol: 'c293-read-only-settled-camera-preflight', side,
  harness_sha256: sha(await readFile(new URL(import.meta.url))),
  source_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  served_web_repository_head: side === 'root' ? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: path.resolve(web, '../..'), encoding: 'utf8' }).trim() : null,
  reference_commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  reference_sha256: referenceHash, fixture_sha256: sha(fixtureBytes),
  browser_build_sha256: side === 'root' ? sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) : referenceHash,
  browser_js_sha256: side === 'root' ? sha(await readFile(path.join(web, 'pkg/obrush_app.js'))) : null,
  requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [], downloads: [],
  screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [],
  page_observations: [], cleanup_errors: [], snapshots: {}, stability: {}, hover_picks: [], status: 'running'
};
const owner = new OwnedCdp(out, result);
let page;
const sourceWitness = `(()=>{
  const a=window.__sculptglReferenceInstance,m=a?.getMesh(),c=a?.getCamera(),meshes=a?.getMeshes()??[];
  const box=meshes.length?a.computeBoundingBoxMeshes(meshes):null;
  const timer_entries=Object.entries(c?._timers??{}).map(([name,value])=>({name,value}));
  return {mesh_count:meshes.length,vertex_count:m?.getNbVertices(),matrix:m?Array.from(m.getMatrix()):null,
    local:m?Array.from(m.getLocalBound()):null,world:m?Array.from(m.computeWorldBound()):null,
    scene_box:box?Array.from(box):null,
    camera:c?{center:Array.from(c._center),offset:Array.from(c._offset),translation:Array.from(c._trans),
      projection:Array.from(c._proj),view:Array.from(c._view),near:c._near,far:c._far,
      projection_type:c._projectionType,width:c._width,height:c._height,timer_entries,
      timers_settled:timer_entries.every(({value})=>value===0||value===-1||value===null)}:null};
})()`;
const installObserver = `(()=>{
  const a=window.__sculptglReferenceInstance,c=a.getCamera(),trace=[];
  window.__c293Trace=trace;
  const snap=()=>({at:performance.now(),center:Array.from(c._center),offset:Array.from(c._offset),
    translation:Array.from(c._trans),near:c._near,projection:Array.from(c._proj),
    timers:Object.entries(c._timers).map(([name,value])=>({name,value})),
    box:Array.from(a.computeBoundingBoxMeshes(a.getMeshes()))});
  const wrap=(object,name,serialize)=>{
    const original=object[name]; if(typeof original!=='function')throw Error('Missing Source camera hook '+name);
    object[name]=function(...args){const before=snap();const value=original.apply(this,args);const after=snap();
      trace.push({name,args:serialize(args),before,after,returned:Number.isFinite(value)?value:null});return value;};
  };
  wrap(c,'resetView',()=>[]);
  wrap(c,'computeFrustumFit',()=>[]);
  wrap(c,'setAndFocusOnPivot',args=>[Array.from(args[0]),args[1]]);
  wrap(c,'_centerDelta',args=>[Array.from(args[0]),args[1]]);
  wrap(a,'resetCameraMeshes',args=>[args[0]?.length??null]);
  return {installed:true,at:performance.now(),snapshot:snap()};
})()`;
async function stableCamera(label) {
  const deadline = Date.now() + 5000;
  let signature = null, since = 0, frames = 0, first = null, latest = null, samples = 0;
  while (Date.now() < deadline) {
    const camera = side === 'source' ? (await page.evaluate(sourceWitness)).camera :
      (await page.live()).state.camera;
    const terminal = side === 'source' ? camera?.timers_settled === true :
      camera?.source_framing?.error == null && camera?.source_scale_pending == null;
    const next = terminal ? JSON.stringify(side === 'source' ?
      [camera.center,camera.translation,camera.projection,camera.near,camera.width,camera.height] :
      [camera.precise_target,camera.position64,camera.projection,
        camera.source_framing?.near,camera.source_framing?.projection_zoom,camera.viewport]) : null;
    samples++;
    if (next !== null && next === signature) {
      frames++;
      latest = camera;
      if (frames >= 12 && Date.now() - since >= 250) {
        result.stability[label] = { first, latest, samples, frames, elapsed_ms: Date.now() - since,
          signature, terminal: true };
        return camera;
      }
    } else {
      signature = next;
      since = Date.now(); frames = 0; first = camera; latest = camera;
    }
    await page.raf();
    await pause(25);
  }
  result.stability[label] = { first, latest, samples, frames, elapsed_ms: Date.now() - since,
    signature, terminal: false };
  throw Error(`C293 ${label} camera failed 12-frame/250ms stability in 5s`);
}
try {
  const url = await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  const gpuObserver = await readFile(path.join(root, side === 'source' ?
    'scripts/source-webgl1-active-words-observer.js' :
    'scripts/brush-lock-gpu-readback-observer.js'), 'utf8');
  result.gpu_observer_sha256 = sha(Buffer.from(gpuObserver));
  page = await owner.page(side, url, gpuObserver);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  if (side === 'root') page.alignViewport = async function() {
    const base = await PerfDriver.prototype.alignViewport.call(this);
    if (JSON.stringify(base.logical_size) !== '[1024,768]') throw Error('OBRush base viewport is not 1024x768');
    const { bounds } = await this.owner.send('Browser.getWindowBounds', { windowId: this.windowId });
    await this.owner.send('Browser.setWindowBounds', { windowId: this.windowId, bounds: { ...bounds, height: bounds.height + 40 } });
    for (let i=0;i<8;i++) { await this.raf(); const v=await this.viewport(); if(JSON.stringify(v.logical_size)==='[1024,808]')return v; }
    throw Error('OBRush 1024x808 viewport not reached');
  };
  await page.send('Page.bringToFront');
  if (side === 'source') {
    await page.alignViewport();
    for (const [label,value] of [['Scale and center',false],['Export all',true],['sRGB vertex color',true]])
      await page.sourceCheckbox('_ctrlFiles',label,value);
    await page.clear();
    await page.file('#fileopen',fixture);
    await page.wait(()=>page.state(),s=>s.meshes.length===1&&s.meshes[0].vertices===expected.vertices,'C293 Source fixture import');
    result.snapshots.after_import={witness:await page.evaluate(sourceWitness),state:await page.state(),viewport:await page.viewport()};
    await page.sourceSelect('_ctrlSculpting',page.controller('_ctrlSculpting')+'._ctrlSculpt.domSelect',7);
    const menu=page.controller('_ctrlSculpting')+'._menu.domUl';
    const nums=`Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
    await page.sourceNumber('_ctrlSculpting',nums+'[0]',96);
    for(const [label,value] of [['Symmetry',false],['Continuous',false]])await page.sourceCheckbox('_ctrlSculpting',label,value);
    const tablet=page.controller('_ctrlTablet')+'._menu.domUl.querySelectorAll("input[type=number]")';
    await page.sourceNumber('_ctrlTablet',tablet+'[0]',0);
    await page.sourceNumber('_ctrlTablet',tablet+'[1]',0);
    const alpha=await page.evaluate(`(()=>{const menu=${menu},e=Array.from(menu.querySelectorAll('select')).find(e=>Array.from(e.options).some(o=>o.value==='None'));return {value:e?.value,owner:window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._idAlpha};})()`);
    if(alpha.value!=='None'||![0,'None'].includes(alpha.owner))throw Error('Source Alpha None not published');
    await page.sourceSelect('_ctrlCamera',page.controller('_ctrlCamera')+'._ctrlProjection.domSelect',1);
    result.instrumentation=await page.evaluate(installObserver);
    await page.sourceButton('_ctrlCamera','Reset (bar)');
    await page.closeMenus();
    await stableCamera('after_reset');
    result.snapshots.after_reset={witness:await page.evaluate(sourceWitness),viewport:await page.viewport()};
    const viewport=await page.viewport();
    const point=[viewport.screen[0]+16,viewport.screen[1]+viewport.pixels[3]-16];
    await page.pointer('mouseMoved',...point);
    if(await page.evaluate('window.__sculptglReferenceInstance._focusGui!==false'))throw Error('Source canvas does not own fit gesture');
    result.snapshots.before_doubletap={witness:await page.evaluate(sourceWitness),point};
    for(let tap=0;tap<2;tap++){
      await page.pointer('mousePressed',...point,'left',1,{clickCount:tap+1});
      await page.pointer('mouseReleased',...point,'left',0,{clickCount:tap+1});
      result.snapshots[`after_tap_${tap+1}`]={witness:await page.evaluate(sourceWitness)};
    }
    await stableCamera('after_fit');
    await page.closeMenus();
    result.snapshots.final={witness:await page.evaluate(sourceWitness),state:await page.state(),viewport:await page.viewport()};
    result.source_trace=await page.evaluate('window.__c293Trace');
    if(!result.source_trace?.some(e=>e.name==='computeFrustumFit'))throw Error('No observed normal-UI Source computeFrustumFit');
  } else {
    const setup=await page.prepareFixture({path:fixture,vertices:expected.vertices});
    result.snapshots.after_import={state:setup.state,viewport:setup.viewport};
    await page.rootControl('brush.drag');
    await page.rootControl('camera.reset');
    await stableCamera('after_reset');
    result.snapshots.after_reset={state:(await page.live()).state,viewport:await page.viewport()};
    await page.rootControl('camera.center');
    await stableCamera('after_fit');
    result.snapshots.final={state:(await page.live()).state,viewport:await page.viewport()};
  }
  if(JSON.stringify(result.snapshots.final.viewport.logical_size)!=='[1024,808]')throw Error('C293 1024x808 viewport not reached');
  result.public_before=(await page.save('before')).record;
  result.before_history=side==='source'?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()'):
    (await page.live()).state.history;
  const v=await page.viewport();
  const center=[(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2];
  if(!center.every(Number.isInteger))throw Error('C293 viewport center not integer CSS pixels');
  result.before_gpu=await page.evaluate(side==='source'?
    `window.__sourceActiveWords.capture(${expected.vertices})`:
    `window.__sourceGpuReadback.capture(${expected.vertices},true)`);
  for(const offset of [[-64,-20],[44,-20]]){
    const pixel=[center[0]+offset[0],center[1]+offset[1]];
    await page.pointer('mouseMoved',...pixel);
    await page.raf();
    const pick=side==='source'?await page.evaluate(`(()=>{const a=window.__sculptglReferenceInstance,p=a.getPicking(),hit=p.intersectionMouseMeshes(),m=p.getMesh();return {hit:!!hit,face:p.getPickedFace(),point:Array.from(p.getIntersectionPoint()),normal:Array.from(p.getPickedNormal()),radius_squared:p.getLocalRadius2(),mouse:[a._mouseX,a._mouseY],mesh_vertices:m?.getNbVertices()??null};})()`):null;
    result.hover_picks.push({offset,pixel,pick,camera:side==='source'?await page.evaluate(sourceWitness):(await page.live()).state.camera});
  }
  result.public_after_hover=(await page.save('after-hover')).record;
  result.after_hover_gpu=await page.evaluate(side==='source'?
    `window.__sourceActiveWords.capture(${expected.vertices})`:
    `window.__sourceGpuReadback.capture(${expected.vertices},true)`);
  result.after_hover_history=side==='source'?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()'):
    (await page.live()).state.history;
  if(result.public_before.sha256!==result.public_after_hover.sha256||
     JSON.stringify(result.before_history)!==JSON.stringify(result.after_hover_history))
    throw Error('Read-only C293 hover modified public document or history');
  if(result.before_gpu.active_vbo_candidates!==1||result.after_hover_gpu.active_vbo_candidates!==1||
     result.before_gpu.position_words_sha256!==result.after_hover_gpu.position_words_sha256||
     result.before_gpu.normal_words_sha256!==result.after_hover_gpu.normal_words_sha256||
     result.before_gpu.gl_error!==0||result.after_hover_gpu.gl_error!==0)
    throw Error('C293 active GPU geometry changed during read-only hover');
  result.status='captured C293 read-only settled camera and hover picks; paired audit pending';
}catch(error){
  result.status='FAILED C293 read-only settled-camera preflight; raw evidence retained';
  result.failure=String(error.stack??error);process.exitCode=1;
}finally{
  try{await owner.close();}catch(error){result.cleanup_errors.push(String(error));process.exitCode=1;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,cleanup_errors:result.cleanup_errors}));
}
