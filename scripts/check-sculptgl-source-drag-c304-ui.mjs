// C304 frozen Source Drag symmetry/mask normal-UI pair.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const { OwnedCdp } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const side = process.argv.find(a => a.startsWith('--side='))?.slice(7);
if (!['source', 'root'].includes(side)) throw Error('Use --side=source or --side=root');
const caseName = process.argv.find(a => a.startsWith('--case='))?.slice(7);
if (!['off','on'].includes(caseName)) throw Error('Use --case=off or --case=on');
const symmetry = caseName === 'on';
const pathOffsets = [[-100,-20],[-64,-20],[-28,-20],[8,-20]];
const radius = 96;
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = path.join(root, 'tests/fixtures/independent-source-drag-c286.sgl');
const expected = JSON.parse(await readFile(path.join(root, 'tests/fixtures/independent-source-drag-c286.json')));
const fixtureBytes = await readFile(fixture);
if (!expected || fixtureBytes.length !== expected.bytes || sha(fixtureBytes) !== expected.sha256) {
  throw Error('Independent opposed-sheet fixture identity changed');
}
if (sha(fixtureBytes) !== 'fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e')
  throw Error('Preregistered fixture SHA-256 changed');
const web = path.resolve(process.env.OBRUSH_WEB_DIST ?? path.join(root, 'web/dist'));
const webUrl = process.env.OBRUSH_WEB_URL ?? null;
const webRepositoryRoot = path.resolve(web, '../..');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinned = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (pinned !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') {
  throw Error('Pinned SculptGL bundle changed');
}
const out = path.join(root, '.artifacts/sculptgl-source-drag-c304-ui', `${caseName}-${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {side, case_name:caseName, symmetry, lock_position:false, protocol_commit:'c304-source-drag-symmetry-mask-integer-viewport808',
  read_only_preflight_audit_sha256:'da28020451312241c1c4859e0050312f16b3683ff94c9a0b79eaf74157d6a57b',
  obrush_source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
  served_web_repository_head:side==='root'?execFileSync('git',['rev-parse','HEAD'],{cwd:webRepositoryRoot,encoding:'utf8'}).trim():null,
  served_web_dist:web,served_web_url:side==='root'?webUrl:null,
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  reference_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  stroke_mode:'drag-three-held-symmetry-mask-integer-viewport808', status:'running', stability:{}, fixture:{path:fixture, bytes:fixtureBytes.length, sha256:sha(fixtureBytes), vertices:expected.vertices, triangles:expected.triangles},
  requested_brush:{radius,culling:false,lock_position:false,symmetry,pathOffsets},
  browser_build:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm'))) : pinned,
  browser_js:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app.js'))) : null,
  browser_simd_build:side === 'root' ? sha(await readFile(path.join(web,'pkg-simd/obrush_app_bg.wasm'))) : null,
  browser_simd_js:side === 'root' ? sha(await readFile(path.join(web,'pkg-simd/obrush_app.js'))) : null,
  requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  viewport_measurements:[], notice_acknowledgments:[], acknowledgments:[], failed_acknowledgments:[],
  page_observations:[], cleanup_errors:[]};
const owner = new OwnedCdp(out, result);
let page, pressed = false;
async function stableCamera(label) {
  const deadline = Date.now() + 5000;
  let signature = null, since = 0, frames = 0, first = null, latest = null, samples = 0;
  while (Date.now() < deadline) {
    const camera = side === 'source' ? await page.evaluate(`(()=>{const c=window.__sculptglReferenceInstance.getCamera();return {
      center:Array.from(c._center),translation:Array.from(c._trans),projection:Array.from(c._proj),
      near:c._near,width:c._width,height:c._height,
      timer_entries:Object.entries(c._timers).map(([name,value])=>({name,value}))};})()`) :
      (await page.live()).state.camera;
    const terminal = side === 'source' ? camera?.timer_entries.every(({value})=>value===0||value===-1||value===null) :
      camera?.source_framing?.error == null && camera?.source_scale_pending == null;
    const next = terminal ? JSON.stringify(side === 'source' ?
      [camera.center,camera.translation,camera.projection,camera.near,camera.width,camera.height] :
      [camera.precise_target,camera.position64,camera.projection,
       camera.source_framing?.near,camera.source_framing?.projection_zoom,camera.viewport]) : null;
    samples++;
    if (next !== null && next === signature) {
      frames++; latest = camera;
      if (frames >= 12 && Date.now() - since >= 250) {
        result.stability[label] = {first,latest,samples,frames,elapsed_ms:Date.now()-since,signature,terminal:true};
        return camera;
      }
    } else { signature=next; since=Date.now(); frames=0; first=camera; latest=camera; }
    await page.raf(); await pause(25);
  }
  result.stability[label] = {first,latest,samples,frames,elapsed_ms:Date.now()-since,signature,terminal:false};
  throw Error(`C304 ${label} camera failed 12-frame/250ms stability in 5s`);
}
async function prepareSourceDragFixture(page) {
  await page.alignViewport();
  for (const [label,value] of [['Scale and center',false],['Export all',true],['sRGB vertex color',true]])
    await page.sourceCheckbox('_ctrlFiles',label,value);
  await page.clear();
  await page.file('#fileopen',fixture);
  await page.wait(()=>page.state(),s=>s.meshes.length===1&&s.meshes[0].vertices===expected.vertices,
    'Complete independent Source fixture import');
  await page.sourceSelect('_ctrlSculpting',page.controller('_ctrlSculpting')+'._ctrlSculpt.domSelect',7);
  const menu=page.controller('_ctrlSculpting')+'._menu.domUl';
  const nums=`Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
  await page.sourceNumber('_ctrlSculpting',nums+'[0]',radius);
  for(const [label,value] of [['Symmetry',symmetry],['Continuous',false]])
    await page.sourceCheckbox('_ctrlSculpting',label,value);
  const tablet=page.controller('_ctrlTablet')+'._menu.domUl.querySelectorAll("input[type=number]")';
  await page.sourceNumber('_ctrlTablet',tablet+'[0]',0);
  await page.sourceNumber('_ctrlTablet',tablet+'[1]',0);
  const alpha=await page.evaluate(`(()=>{const menu=${menu},e=Array.from(menu.querySelectorAll('select')).find(e=>Array.from(e.options).some(o=>o.value==='None'));return {value:e?.value,owner:window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._idAlpha};})()`);
  result.source_alpha_initial=alpha;
  if(alpha.value!=='None'||![0,'None'].includes(alpha.owner))throw Error('Source Alpha None was not published');
  await page.sourceSelect('_ctrlCamera',page.controller('_ctrlCamera')+'._ctrlProjection.domSelect',1);
  await page.sourceButton('_ctrlCamera','Reset (bar)');
  await page.closeMenus();
  await stableCamera('after_reset');
  const viewport=await page.viewport();
  const point=[viewport.screen[0]+16,viewport.screen[1]+viewport.pixels[3]-16];
  await page.pointer('mouseMoved',...point);
  if(await page.evaluate('window.__sculptglReferenceInstance._focusGui!==false'))
    throw Error('Actual Source canvas does not own fit gesture');
  for(let tap=0;tap<2;tap++){
    await page.pointer('mousePressed',...point,'left',1,{clickCount:tap+1});
    await page.pointer('mouseReleased',...point,'left',0,{clickCount:tap+1});
  }
  await stableCamera('after_fit');
  await page.closeMenus();
  return {state:await page.state(),viewport:await page.viewport()};
}
try {
  if (side==='root' && webUrl) {
    result.served_http_hashes={};
    for(const [name,localFile] of [['pkg/obrush_app_bg.wasm','pkg/obrush_app_bg.wasm'],
      ['pkg/obrush_app.js','pkg/obrush_app.js']]){
      const response=await fetch(new URL(name,webUrl),{cache:'no-store'});
      if(!response.ok)throw Error(`Existing OBrush web server returned ${response.status} for ${name}`);
      const actual=sha(Buffer.from(await response.arrayBuffer()));
      const expected=sha(await readFile(path.join(web,localFile)));
      result.served_http_hashes[name]={actual,expected,exact:actual===expected};
      if(actual!==expected)throw Error(`Existing OBrush web server serves stale ${name}`);
    }
  }
  const url = side==='root' && webUrl ? webUrl : await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  const stage = path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage');
  const pointerObserver = `(()=>{const rows=[];window.__uiBrushDelivered=rows;for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])window.addEventListener(type,e=>{if(rows.length<2048)rows.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons,trusted:e.isTrusted,at:performance.now()});},true);})();\n`;
  const gpuObserver = side === 'source' ? await readFile(path.join(root,'scripts/source-webgl1-active-words-observer.js'),'utf8') :
    await readFile(path.join(root,'scripts/brush-lock-gpu-readback-observer.js'),'utf8');
  result.gpu_observer_sha256=sha(Buffer.from(gpuObserver));
  const observer = pointerObserver + gpuObserver + '\n' + (side === 'source' ?
    (await readFile(path.join(stage, 'observer/focused-alpha-input-observer.js'),'utf8')) + '\n' +
    (await readFile(path.join(stage, 'observer/focused-alpha-hit-observer.js'),'utf8')) : '');
  page = await owner.page(side, url, observer);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  if (side === 'root') page.alignViewport = async function() {
    const base=await PerfDriver.prototype.alignViewport.call(this);
    if (JSON.stringify(base.logical_size)!=='[1024,768]')
      throw Error('C304 OBrush base logical viewport is not 1024x768');
    const {bounds}=await this.owner.send('Browser.getWindowBounds',{windowId:this.windowId});
    await this.owner.send('Browser.setWindowBounds',{windowId:this.windowId,
      bounds:{...bounds,height:bounds.height+40}});
    for (let i=0; i<8; i++) {
      await this.raf();
      const viewport=await this.viewport();
      if (JSON.stringify(viewport.logical_size)==='[1024,808]') return viewport;
    }
    throw Error('Normal OBrush logical viewport1024x808 not reached');
  };
  await page.send('Page.bringToFront');
  result.setup = side==='source' ? await prepareSourceDragFixture(page) :
    await page.prepareFixture({path:fixture, vertices:expected.vertices});
  if(side==='root'){
    await page.rootControl('brush.drag');
    await page.rootControl('camera.reset');
    await stableCamera('after_reset');
    await page.rootControl('camera.center');
    await stableCamera('after_fit');
    result.setup = {state:(await page.live()).state,viewport:await page.viewport()};
    await page.rootControl('source_geometry.mode.source');
  }
  if (JSON.stringify(result.setup.viewport.logical_size)!=='[1024,808]')
    throw Error('C304 frozen 1024x808 logical sculpt viewport was not reached');
  if (side === 'source') {
    const menu = 'window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl';
    const numbers = `Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
    for (const [index,value,min,max] of [[0,radius,5,500]]) {
      if (value === [96][index]) continue;
      await page.sourceMenu('_ctrlSculpting');
      const control = await page.evaluate(`(()=>{const e=${numbers}[${index}],s=e?.parentElement?.querySelector('.gui-slider'),r=s?.getBoundingClientRect();return {before:Number(e?.value),rect:r?[r.left,r.top,r.right,r.bottom]:null}})()`);
      if (!control.rect || control.rect[2]-control.rect[0]<30) throw Error('Source visible Brush slider unavailable');
      const startX=control.rect[0]+(control.rect[2]-control.rect[0])*(control.before-min)/(max-min);
      const x=control.rect[0]+(control.rect[2]-control.rect[0])*(value-min)/(max-min);
      const y=(control.rect[1]+control.rect[3])/2;
      const hit=()=>page.evaluate(`(()=>{const s=${numbers}[${index}].parentElement.querySelector('.gui-slider'),e=document.elementFromPoint(${startX},${y});return {ready:e===s||s.contains(e),tag:e?.tagName,cl:e?.className,open:${menu}.getAttribute('opened')}})()`);
      let hitBefore;
      for(let i=0;i<120;i++){hitBefore=await hit();if(hitBefore.ready)break;await page.raf();}
      if(!hitBefore?.ready)throw Error('Source visible Brush slider remained covered after 120 frames');
      await page.pointer('mouseMoved',startX,y);
      await page.pointer('mousePressed',startX,y,'left',1);
      await page.pointer('mouseMoved',x,y,'left',1);
      await page.pointer('mouseReleased',x,y,'left',0);
      await page.raf();
      const after=await page.evaluate(`Number(${numbers}[${index}].value)`);
      const hitAfter=await page.evaluate(`(()=>{const e=document.elementFromPoint(${x},${y});return {tag:e?.tagName,cl:e?.className,open:${menu}.getAttribute('opened')}})()`);
      result.ui_actions.push({side:'source',slider:index,before:control.before,requested:value,after,rect:control.rect,drag:[[startX,y],[x,y]],hitBefore,hitAfter});
      if (!Number.isFinite(after) || Math.abs(after-value)>Math.max(3,(max-min)*3/(control.rect[2]-control.rect[0]))) throw Error('Source visible Brush slider did not publish nearby value');
    }
  }
  if (side === 'root') {
    await page.rootNumber('source_geometry.pressure.radius.value',0);

    await page.rootCheckbox('source_geometry.lock_position',s=>s.lock_position,false);
    await page.rootCheckbox('source_geometry.mirror',s=>s.symmetry_x,symmetry);
  }
  if (side === 'root') await page.rootNumber('source_geometry.radius.value',radius);
  await page.closeMenus();
  result.config = await page.live();
  const p = side === 'source' ? result.config.state.tool : result.config.state.source_geometry.per_tool[7];
  if (side === 'source') {
    const tablet = page.controller('_ctrlTablet') + '._menu.domUl.querySelectorAll("input[type=number]")';
    result.source_tablet_controls = await page.evaluate(`Array.from(${tablet}).map(e=>Number(e.value))`);
    if (result.source_tablet_controls.slice(0,2).some(v=>v!==0)) throw Error('Source pressure factors are not zero');
  } else if (p.radius_factor!==0 || p.options.intensity_factor!==0) {
    throw Error('OBRush pressure factors are not zero');
  }
  if (side === 'source' ? !(result.config.state.tool_index===7 &&
    p._lockPosition!==true && [0,'None'].includes(p._idAlpha) &&
    result.config.state.symmetry===symmetry && !result.config.state.continuous) :
    !(result.config.state.kind==='Drag' && p.mode==='SourceOrdinary' &&
      p.options.tool==='Drag' && !p.options.negative &&
      p.options.culling===false && !p.options.prevent_folds &&
      result.config.state.lock_position===false && result.config.state.symmetry_x===symmetry &&
      !result.config.state.continuous.requested &&
      (result.config.state.alpha===null || result.config.state.alpha==='None'))) {
    throw Error('Normal UI Drag settings differ from admitted comparison');
  }
  if (side === 'source' ? Math.abs(p._radius-radius)>3 :
    Math.abs(p.radius-radius)>1e-6) throw Error('Requested Brush radius did not publish');
  result.before = (await page.save('before')).record;
  result.before_history = side==='source' ?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()') :
    (await page.live()).state.history;
  if(side==='root')result.before_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.before_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  const viewport = await page.viewport();
  const center = [(viewport.screen[0]+viewport.screen[2])/2,(viewport.screen[1]+viewport.screen[3])/2];
  if (!center.every(Number.isInteger)) throw Error('C304 viewport center is not integer CSS pixels');
  const pathPixels = pathOffsets.map(([x,y])=>[center[0]+x,center[1]+y]);
  result.requested_path = pathPixels;
  const deliveredAt = await page.evaluate('window.__uiBrushDelivered?.length??null');
  await page.pointer('mouseMoved',...pathPixels[0]);
  await page.pointer('mousePressed',...pathPixels[0],'left',1,{force:1}); pressed = true;
  await page.raf();
  result.press_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  const pickExpression=`(()=>{
    const p=window.__sculptglReferenceInstance.getPicking();
    const m=p.getMesh(),face=p.getPickedFace();
    const f=m&&face>=0?Array.from(m.getFaces().subarray(face*4,face*4+4)):null;
    const colors=m?.getColors();
    const ids=f?.map(i=>i===4294967295?i:(Math.round(colors[i*3]*255)|
      (Math.round(colors[i*3+1]*255)<<8)|(Math.round(colors[i*3+2]*255)<<16)));
    return {face,face_vertices:f,face_rgb_ids:ids,normal:Array.from(p.getPickedNormal()),
      point:Array.from(p.getIntersectionPoint()),eye:Array.from(p.getEyeDirection()),
      radius_squared:p.getLocalRadius2(),picked_vertices:p.getPickedVertices()?.length??null};
  })()`;
  result.press_pick=side==='source'?await page.evaluate(pickExpression):null;
  result.press_live=side==='root'?await page.live():null;
  result.held_previews=[];
  for (const [index,point] of pathPixels.slice(1).entries()) {
    await page.pointer('mouseMoved',...point,'left',1,{force:1});
    for(let j=0;j<3;j++)await page.raf();
    const gpu=await page.evaluate(side === 'source' ?
      `window.__sourceActiveWords.capture(${expected.vertices})` :
      `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
    const pick = side === 'source' ? await page.evaluate(pickExpression) : null;
    const live=await page.live();
    result.held_previews.push({point,gpu,pick,live});
  }
  await page.pointer('mouseReleased',...pathPixels.at(-1),'left',0); pressed = false;
  for (let i=0;i<12;i++) { await page.raf(); await pause(25); }
  result.dom_stroke_events = await page.evaluate(`window.__uiBrushDelivered?.slice(${deliveredAt})??null`);
  if (!result.dom_stroke_events?.length || result.dom_stroke_events.some(e=>!e.trusted)) throw Error('Trusted normal DOM stroke delivery missing');
  result.release_state = side === 'root' ? await page.evaluate('window.obrushSculptMetrics()') :
    await page.evaluate('({timer:window.__sculptglReferenceInstance.getSculptManager()._sculptTimer,action:window.__sculptglReferenceInstance._action})');
  result.after_history = side==='source' ?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()') :
    (await page.live()).state.history;
  result.after = (await page.save('after')).record;
  if(side==='root')result.after_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.after_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  for(let i=0;i<30;i++)await page.raf();
  result.quiet_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  await page.chord('z'); result.undo = (await page.save('undo')).record;
  if(side==='root')result.undo_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.undo_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  await page.chord('z',true); result.redo = (await page.save('redo')).record;
  if(side==='root')result.redo_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.redo_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  result.status = 'normal browser UI stroke and public save/undo/redo captured; paired geometry audit pending';
} catch (error) {
  result.status = 'FAILED normal browser UI stroke; evidence retained';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  if (pressed && page) try { const v=await page.viewport(); await page.pointer('mouseReleased',(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2,'left',0); }
  catch(error) { result.cleanup_errors.push(String(error)); }
  try { await owner.close(); } catch(error) { result.cleanup_errors.push(String(error)); process.exitCode=1; }
  await writeFile(path.join(out,'result.json'), JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,cleanup_errors:result.cleanup_errors}));
}
