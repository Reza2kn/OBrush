// Preregistered normal-UI Brush Lock Position pair, one run per side.
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
const radius = 64;
const intensity = 0.25;
const negative = false;
const accumulate = false;
const matchSourcePath = process.argv.find(a=>a.startsWith('--match-source='))?.slice('--match-source='.length);
if (matchSourcePath && side !== 'root') throw Error('Source-ray matching applies to root side');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = path.join(shared, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/fixtures/independent-id-sphere-10k.sgl');
const alphaFixture = path.join(root, 'tests/fixtures/independent-hook-alpha-16.png');
const manifest = JSON.parse(await readFile(path.join(shared, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/FIXTURE_MANIFEST.json')));
const expected = manifest.files.find(item => item.label === '10k');
const fixtureBytes = await readFile(fixture);
if (!expected || fixtureBytes.length !== expected.bytes || sha(fixtureBytes) !== expected.sha256) {
  throw Error('Independent 10K fixture identity changed');
}
if (sha(fixtureBytes) !== '39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b')
  throw Error('Preregistered fixture SHA-256 changed');
const alphaBytes = await readFile(alphaFixture);
if (alphaBytes.length !== 95 || sha(alphaBytes) !== '0486e281eeb2766ecf096ef55fe42023545675d9d9bc123441cd729109ac290a')
  throw Error('Preregistered independent hook alpha changed');
const web = path.resolve(process.env.OBRUSH_WEB_DIST ?? path.join(root, 'web/dist'));
const webUrl = process.env.OBRUSH_WEB_URL ?? null;
const webRepositoryRoot = path.resolve(web, '../..');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinned = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (pinned !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') {
  throw Error('Pinned SculptGL bundle changed');
}
const out = path.join(root, '.artifacts/sculptgl-brush-lock-ui', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {side, lock_position:true, protocol_commit:'e239b1f',
  obrush_source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
  served_web_repository_head:side==='root'?execFileSync('git',['rev-parse','HEAD'],{cwd:webRepositoryRoot,encoding:'utf8'}).trim():null,
  served_web_dist:web,served_web_url:side==='root'?webUrl:null,
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  reference_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  stroke_mode:'locked-three-held-previews', status:'running', fixture:{path:fixture, bytes:fixtureBytes.length, sha256:sha(fixtureBytes), vertices:expected.vertices, triangles:expected.triangles},
  alpha_fixture:{path:alphaFixture,bytes:alphaBytes.length,sha256:sha(alphaBytes),width:16,height:16},
  requested_brush:{radius,intensity,accumulate,negative,lock_position:true},
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
async function prepareSourceBrushFixture(page) {
  await page.alignViewport();
  for (const [label,value] of [['Scale and center',false],['Export all',true],['sRGB vertex color',true]])
    await page.sourceCheckbox('_ctrlFiles',label,value);
  await page.clear();
  await page.file('#fileopen',fixture);
  await page.wait(()=>page.state(),s=>s.meshes.length===1&&s.meshes[0].vertices===expected.vertices,
    'Complete independent Source fixture import');
  await page.sourceSelect('_ctrlSculpting',page.controller('_ctrlSculpting')+'._ctrlSculpt.domSelect',0);
  const menu=page.controller('_ctrlSculpting')+'._menu.domUl';
  const nums=`Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
  await page.sourceNumber('_ctrlSculpting',nums+'[0]',64);
  await page.sourceNumber('_ctrlSculpting',nums+'[1]',25);
  for(const [label,value] of [['Clay',false],['Accumulate',false],['Negative',false],
    ['Thin surface',false],['Symmetry',false],['Continuous',false]])
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
  const viewport=await page.viewport();
  const point=[viewport.screen[0]+16,viewport.screen[1]+viewport.pixels[3]-16];
  await page.pointer('mouseMoved',...point);
  if(await page.evaluate('window.__sculptglReferenceInstance._focusGui!==false'))
    throw Error('Actual Source canvas does not own fit gesture');
  for(let tap=0;tap<2;tap++){
    await page.pointer('mousePressed',...point,'left',1,{clickCount:tap+1});
    await page.pointer('mouseReleased',...point,'left',0,{clickCount:tap+1});
  }
  await page.wait(()=>page.evaluate('Object.values(window.__sculptglReferenceInstance.getCamera()._timers).every(v=>v===0||v===-1||v===null)'),
    v=>v,'Source fit timers settle');
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
  await page.send('Page.bringToFront');
  result.setup = side==='source' ? await prepareSourceBrushFixture(page) :
    await page.prepareFixture({path:fixture, vertices:expected.vertices});
  if(side==='root') {
    await page.rootControl('alpha.import');
    await page.wait(()=>page.state(),s=>Number.isInteger(s.preferences?.alpha_import_request?.id),
      'OBRush normal alpha chooser request armed');
  }
  await page.file(side==='source'?'#alphaopen':'#alpha-file',alphaFixture);
  if(side==='source') {
    await page.wait(()=>page.evaluate('window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._idAlpha'),
      v=>typeof v==='string'&&v.includes('independent-hook-alpha-16'),'Source normal PNG alpha import');
  } else {
    await page.wait(()=>page.state(),s=>typeof s.alpha==='string'&&s.alpha.includes('independent-hook-alpha-16'),
      'OBRush normal PNG alpha import');
  }
  if (side === 'source') {
    const menu = 'window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl';
    const numbers = `Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
    for (const [index,value,min,max] of [[0,radius,5,500],[1,intensity*100,0,100]]) {
      if (value === [64,25][index]) continue;
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
    await page.sourceMenu('_ctrlSculpting');
    const lockControl = `Array.from(${menu}.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.includes('Lock position')&&e.parentElement.getBoundingClientRect().width>0)`;
    const lockLabel = `(${lockControl}).nextElementSibling`;
    const lockBefore = await page.evaluate(`(()=>{const e=${lockControl},r=e?.parentElement.getBoundingClientRect();return {checked:e?.checked,disabled:e?.disabled,rect:r?[r.left,r.top,r.right,r.bottom]:null};})()`);
    if (!lockBefore.rect || lockBefore.disabled) throw Error('Visible Source Lock position checkbox unavailable');
    let hit;
    for(let i=0;i<120;i++){
      hit=await page.evaluate(`(()=>{const e=${lockLabel},r=e?.getBoundingClientRect(),x=r&&(r.left+r.right)/2,y=r&&(r.top+r.bottom)/2,h=r&&document.elementFromPoint(x,y);return {tag:e?.tagName,rect:r?[r.left,r.top,r.right,r.bottom]:null,hit:h?.outerHTML?.slice(0,256),ready:!!r&&(h===e||e.contains(h))};})()`);
      if(hit.ready)break;
      await page.raf();
    }
    result.ui_actions.push({side:'source',checkbox:'Lock position',lockBefore,hit});
    if(!hit?.ready || hit.tag!=='LABEL')throw Error('Visible Source Lock position label not actual hit target');
    if (lockBefore.checked !== true) await page.sourceClick(lockLabel);
    const lockAfter = await page.wait(()=>page.evaluate(`(${lockControl}).checked`),v=>v===true,'Visible Source Lock position published');
    result.ui_actions.push({side:'source',checkbox:'Lock position',lockAfter});
    if (lockAfter !== true) throw Error('Source Lock position did not publish');
  } else if (!matchSourcePath) {
    if (radius !== 64) await page.rootNumber('source_geometry.radius.value',radius);
    if (intensity !== 0.25) await page.rootNumber('source_geometry.intensity.value',intensity);
  }
  if (side === 'root') {
    await page.rootNumber('source_geometry.pressure.radius.value',0);
    await page.rootNumber('source_geometry.pressure.intensity.value',0);
  }
  if (side === 'root') await page.rootCheckbox('settings.lock_position',s=>s.lock_position,true);
  if (matchSourcePath) {
    const sourceRun = JSON.parse(await readFile(matchSourcePath));
    if (sourceRun.side !== 'source' || sourceRun.fixture.sha256 !== result.fixture.sha256 ||
        sourceRun.alpha_fixture?.sha256 !== result.alpha_fixture.sha256 ||
        sourceRun.stroke_mode !== result.stroke_mode ||
        (sourceRun.requested_brush?.radius??64) !== radius ||
        (sourceRun.requested_brush?.intensity??0.25) !== intensity ||
        sourceRun.requested_brush?.lock_position !== true) throw Error('Source ray-binding run mismatch');
    const sourceBrush=sourceRun.config?.state?.tool;
    if (!sourceBrush || !Number.isFinite(sourceBrush._radius) || !Number.isFinite(sourceBrush._intensity)) throw Error('Source published Brush settings missing');
    const sourceProjectionX = sourceRun.setup.state.camera.projection[0];
    const sourceProjectionY = sourceRun.setup.state.camera.projection[5];
    const rootProjectionX = result.setup.state.camera.projection[0];
    const rootProjectionY = result.setup.state.camera.projection[5];
    const sourceWidth = sourceRun.setup.viewport.screen[2]-sourceRun.setup.viewport.screen[0];
    const sourceHeight = sourceRun.setup.viewport.screen[3]-sourceRun.setup.viewport.screen[1];
    const rootWidth = result.setup.viewport.screen[2]-result.setup.viewport.screen[0];
    const rootHeight = result.setup.viewport.screen[3]-result.setup.viewport.screen[1];
    const offsetScale = rootProjectionX/sourceProjectionX*rootWidth/sourceWidth;
    const offsetScaleY = rootProjectionY/sourceProjectionY*rootHeight/sourceHeight;
    if (!Number.isFinite(offsetScale) || offsetScale<0.5 || offsetScale>2 ||
        !Number.isFinite(offsetScaleY) || offsetScaleY<0.5 || offsetScaleY>2)
      throw Error('Invalid source ray-alignment scale');
    result.ray_alignment = {source_result:matchSourcePath,source_projection_x:sourceProjectionX,
      root_projection_x:rootProjectionX,source_width:sourceWidth,root_width:rootWidth,offset_scale:offsetScale,
      source_projection_y:sourceProjectionY,root_projection_y:rootProjectionY,
      source_height:sourceHeight,root_height:rootHeight,offset_scale_y:offsetScaleY,
      source_published_radius:sourceBrush._radius,source_published_intensity:sourceBrush._intensity,
      radius_requested:sourceBrush._radius*offsetScale,scope:'Matched initial horizontal orthographic ray and world brush radius from actual projection matrices; not a proof of delivered contacts'};
    await page.rootNumber('source_geometry.radius.value',sourceBrush._radius*offsetScale);
    if (sourceBrush._intensity !== 0.25) await page.rootNumber('source_geometry.intensity.value',sourceBrush._intensity);
  }
  await page.closeMenus();
  result.config = await page.live();
  const p = side === 'source' ? result.config.state.tool : result.config.state.source_geometry.per_tool[0];
  if (side === 'source') {
    const tablet = page.controller('_ctrlTablet') + '._menu.domUl.querySelectorAll("input[type=number]")';
    result.source_tablet_controls = await page.evaluate(`Array.from(${tablet}).map(e=>Number(e.value))`);
    if (result.source_tablet_controls.slice(0,2).some(v=>v!==0)) throw Error('Source pressure factors are not zero');
  } else if (p.radius_factor!==0 || p.options.intensity_factor!==0) {
    throw Error('OBRush pressure factors are not zero');
  }
  if (side === 'source' ? !(p._accumulate===accumulate && p._negative===negative && !p._clay &&
    !p._culling && p._lockPosition===true && String(p._idAlpha).includes('independent-hook-alpha-16') &&
    !result.config.state.symmetry && !result.config.state.continuous) :
    !(p.options.tool.Brush.accumulate===accumulate && p.options.negative===negative &&
      !p.options.tool.Brush.clay && !p.options.culling && !p.options.prevent_folds &&
      result.config.state.lock_position===true && !result.config.state.symmetry_x &&
      !result.config.state.continuous.requested &&
      String(result.config.state.alpha).includes('independent-hook-alpha-16'))) {
    throw Error('Normal UI Brush settings differ from admitted comparison');
  }
  if (side === 'source' ? Math.abs(p._radius-radius)>3 || Math.abs(p._intensity-intensity)>0.03 :
    Math.abs(p.radius-(result.ray_alignment?.radius_requested??radius))>1e-6 ||
    Math.abs(p.options.intensity-(result.ray_alignment?.source_published_intensity??intensity))>1e-6) throw Error('Requested Brush radius/intensity did not publish');
  result.before = (await page.save('before')).record;
  if(side==='root')result.before_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.before_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  const viewport = await page.viewport();
  const center = [(viewport.screen[0]+viewport.screen[2])/2,(viewport.screen[1]+viewport.screen[3])/2];
  const scaleX=result.ray_alignment?.offset_scale??1;
  const scaleY=result.ray_alignment?.offset_scale_y??1;
  const pathPixels = [center,[center[0]+80*scaleX,center[1]],
    [center[0],center[1]-80*scaleY],[center[0]+80*scaleX,center[1]]];
  result.requested_path = pathPixels;
  const deliveredAt = await page.evaluate('window.__uiBrushDelivered?.length??null');
  await page.pointer('mouseMoved',...pathPixels[0]);
  await page.pointer('mousePressed',...pathPixels[0],'left',1,{force:1}); pressed = true;
  await page.raf();
  result.press_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  result.held_previews=[];
  for (const point of pathPixels.slice(1)) {
    await page.pointer('mouseMoved',...point,'left',1,{force:1});
    for(let j=0;j<3;j++)await page.raf();
    const gpu=await page.evaluate(side === 'source' ?
      `window.__sourceActiveWords.capture(${expected.vertices})` :
      `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
    const live=await page.live();
    result.held_previews.push({point,gpu,live});
  }
  await page.pointer('mouseReleased',...pathPixels.at(-1),'left',0); pressed = false;
  for (let i=0;i<12;i++) { await page.raf(); await pause(25); }
  result.dom_stroke_events = await page.evaluate(`window.__uiBrushDelivered?.slice(${deliveredAt})??null`);
  if (!result.dom_stroke_events?.length || result.dom_stroke_events.some(e=>!e.trusted)) throw Error('Trusted normal DOM stroke delivery missing');
  result.release_state = side === 'root' ? await page.evaluate('window.obrushSculptMetrics()') :
    await page.evaluate('({timer:window.__sculptglReferenceInstance.getSculptManager()._sculptTimer,action:window.__sculptglReferenceInstance._action})');
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
