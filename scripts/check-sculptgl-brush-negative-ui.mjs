// Preregistered C272 normal-UI Brush Negative pair, one run per side/polarity.
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
const polarity = process.argv.find(a => a.startsWith('--negative='))?.slice(11);
if (!['off','on'].includes(polarity)) throw Error('Use --negative=off or --negative=on');
const negative = polarity === 'on';
const pressOnly = process.argv.includes('--press-only');
const oneMove = process.argv.includes('--one-move');
const retraceArg=process.argv.find(a=>a.startsWith('--retrace-moves='));
const retraceMoves=retraceArg?Number(retraceArg.slice(16)):0;
if ((pressOnly&&oneMove)||(retraceArg&&(pressOnly||oneMove)) ||
    (retraceArg&&(!Number.isInteger(retraceMoves)||retraceMoves<2||retraceMoves>20))) throw Error('Select one admitted stroke mode');
const radius = Number(process.argv.find(a=>a.startsWith('--radius='))?.slice(9) ?? 64);
const intensity = Number(process.argv.find(a=>a.startsWith('--intensity='))?.slice(12) ?? 0.25);
const accumulate = process.argv.includes('--accumulate');
if (!Number.isFinite(radius) || radius < 5 || radius > 500 ||
    !Number.isFinite(intensity) || intensity < 0 || intensity > 1) {
  throw Error('Use a finite --radius=5..500 and --intensity=0..1');
}
if (!oneMove || pressOnly || retraceArg || radius!==64 || intensity!==0.25 || accumulate)
  throw Error('C272 protocol requires --one-move, radius 64, intensity 0.25, Accumulate off');
const matchSourcePath = process.argv.find(a=>a.startsWith('--match-source='))?.slice('--match-source='.length);
if (matchSourcePath && side !== 'root') throw Error('Source-ray matching applies to root side');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = path.join(shared, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/fixtures/independent-id-sphere-10k.sgl');
const manifest = JSON.parse(await readFile(path.join(shared, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/FIXTURE_MANIFEST.json')));
const expected = manifest.files.find(item => item.label === '10k');
const fixtureBytes = await readFile(fixture);
if (!expected || fixtureBytes.length !== expected.bytes || sha(fixtureBytes) !== expected.sha256) {
  throw Error('Independent 10K fixture identity changed');
}
if (sha(fixtureBytes) !== '39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b')
  throw Error('Preregistered fixture SHA-256 changed');
const web = path.join(root, 'web/dist');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinned = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (pinned !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') {
  throw Error('Pinned SculptGL bundle changed');
}
const out = path.join(root, '.artifacts/sculptgl-brush-negative-ui', `${side}-${polarity}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {side, negative, protocol_commit:'e09510c',
  obrush_source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  reference_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  stroke_mode:pressOnly?'one-press':oneMove?'one-move':retraceMoves?`retrace-${retraceMoves}`:'three-point-drag', status:'running', fixture:{path:fixture, bytes:fixtureBytes.length, sha256:sha(fixtureBytes), vertices:expected.vertices, triangles:expected.triangles},
  requested_brush:{radius,intensity,accumulate,negative},
  browser_build:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm'))) : pinned,
  browser_js:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app.js'))) : null,
  requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  viewport_measurements:[], notice_acknowledgments:[], acknowledgments:[], failed_acknowledgments:[],
  page_observations:[], cleanup_errors:[]};
const owner = new OwnedCdp(out, result);
let page, pressed = false;
try {
  const url = await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  const stage = path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage');
  const pointerObserver = `(()=>{const rows=[];window.__uiBrushDelivered=rows;for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])window.addEventListener(type,e=>{if(rows.length<2048)rows.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons,trusted:e.isTrusted,at:performance.now()});},true);})();\n`;
  const gpuObserver = side === 'source' ? await readFile(path.join(root,'scripts/source-webgl1-active-words-observer.js'),'utf8') :
    await readFile(path.join(root,'scripts/source-gpu-readback-observer.js'),'utf8');
  result.gpu_observer_sha256=sha(Buffer.from(gpuObserver));
  const observer = pointerObserver + gpuObserver + '\n' + (side === 'source' ?
    (await readFile(path.join(stage, 'observer/focused-alpha-input-observer.js'),'utf8')) + '\n' +
    (await readFile(path.join(stage, 'observer/focused-alpha-hit-observer.js'),'utf8')) : '');
  page = await owner.page(side, url, observer);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  await page.send('Page.bringToFront');
  result.setup = await page.prepareFixture({path:fixture, vertices:expected.vertices});
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
    if (accumulate) {
      await page.sourceMenu('_ctrlSculpting');
      const expression=page.checkboxExpression('_ctrlSculpting','Accumulate');
      const checkbox=await page.evaluate(`(()=>{const e=${expression},r=e?.parentElement?.getBoundingClientRect();return {checked:e?.checked,rect:r?[r.left,r.top,r.right,r.bottom]:null}})()`);
      if (!checkbox.rect || checkbox.checked !== false) throw Error('Source Accumulate checkbox initial state unexpected');
      const x=(checkbox.rect[0]+checkbox.rect[2])/2,y=(checkbox.rect[1]+checkbox.rect[3])/2;
      let hit;
      for(let i=0;i<120;i++){
        hit=await page.evaluate(`(()=>{const p=(${expression}).parentElement,e=document.elementFromPoint(${x},${y});return {ready:p===e||p.contains(e),tag:e?.tagName,cl:e?.className}})()`);
        if(hit.ready)break;
        await page.raf();
      }
      if(!hit?.ready)throw Error('Source Accumulate checkbox remained covered after 120 frames');
      await page.click(x,y);
      const published=await page.evaluate(`(()=>({checked:(${expression}).checked,owner:window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._accumulate}))()`);
      result.ui_actions.push({side:'source',checkbox:'Accumulate',initial:checkbox,published,clicked:[x,y],hit});
      if(published.checked!==true||published.owner!==true)throw Error('Source Accumulate checkbox did not publish');
    }
    await page.sourceMenu('_ctrlSculpting');
    const negativeControl = `Array.from(${menu}.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.includes('Negative')&&e.parentElement.getBoundingClientRect().width>0)`;
    const negativeBefore = await page.evaluate(`(()=>{const e=${negativeControl},r=e?.parentElement.getBoundingClientRect();return {checked:e?.checked,disabled:e?.disabled,rect:r?[r.left,r.top,r.right,r.bottom]:null};})()`);
    if (!negativeBefore.rect || negativeBefore.disabled) throw Error('Visible Source Negative checkbox unavailable');
    if (negativeBefore.checked !== negative) await page.sourceClick(`(${negativeControl}).parentElement`);
    const negativeAfter = await page.evaluate(`(${negativeControl}).checked`);
    result.ui_actions.push({side:'source',checkbox:'Negative',negativeBefore,negativeAfter});
    if (negativeAfter !== negative) throw Error('Source Negative did not publish');
  } else if (!matchSourcePath) {
    if (radius !== 64) await page.rootNumber('source_geometry.radius.value',radius);
    if (intensity !== 0.25) await page.rootNumber('source_geometry.intensity.value',intensity);
  }
  if (side === 'root') {
    await page.rootNumber('source_geometry.pressure.radius.value',0);
    await page.rootNumber('source_geometry.pressure.intensity.value',0);
  }
  if (side === 'root' && accumulate) await page.rootCheckbox('source_geometry.accumulate',s=>s.source_geometry.per_tool[0].options.tool.Brush.accumulate,true);
  if (side === 'root') await page.rootCheckbox('source_geometry.negative',s=>s.source_geometry.per_tool[0].options.negative,negative);
  if (matchSourcePath) {
    const sourceRun = JSON.parse(await readFile(matchSourcePath));
    if (sourceRun.side !== 'source' || sourceRun.fixture.sha256 !== result.fixture.sha256 ||
        sourceRun.stroke_mode !== result.stroke_mode ||
        (sourceRun.requested_brush?.radius??64) !== radius ||
        (sourceRun.requested_brush?.intensity??0.25) !== intensity ||
        (sourceRun.requested_brush?.accumulate??false) !== accumulate || sourceRun.negative !== negative) throw Error('Source ray-binding run mismatch');
    const sourceBrush=sourceRun.config?.state?.tool;
    if (!sourceBrush || !Number.isFinite(sourceBrush._radius) || !Number.isFinite(sourceBrush._intensity)) throw Error('Source published Brush settings missing');
    const sourceProjectionX = sourceRun.setup.state.camera.projection[0];
    const rootProjectionX = result.setup.state.camera.projection[0];
    const sourceWidth = sourceRun.setup.viewport.screen[2]-sourceRun.setup.viewport.screen[0];
    const rootWidth = result.setup.viewport.screen[2]-result.setup.viewport.screen[0];
    const offsetScale = rootProjectionX/sourceProjectionX*rootWidth/sourceWidth;
    if (!Number.isFinite(offsetScale) || offsetScale<0.5 || offsetScale>2) throw Error('Invalid source ray-alignment scale');
    result.ray_alignment = {source_result:matchSourcePath,source_projection_x:sourceProjectionX,
      root_projection_x:rootProjectionX,source_width:sourceWidth,root_width:rootWidth,offset_scale:offsetScale,
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
    !p._culling && p._idAlpha === 'None' && !result.config.state.symmetry && !result.config.state.continuous) :
    !(p.options.tool.Brush.accumulate===accumulate && p.options.negative===negative &&
      !p.options.tool.Brush.clay && !p.options.culling && !p.options.prevent_folds &&
      !result.config.state.symmetry_x && !result.config.state.continuous.requested &&
      result.config.state.alpha===null)) {
    throw Error('Normal UI Brush settings differ from admitted comparison');
  }
  if (side === 'source' ? Math.abs(p._radius-radius)>3 || Math.abs(p._intensity-intensity)>0.03 :
    Math.abs(p.radius-(result.ray_alignment?.radius_requested??radius))>1e-6 ||
    Math.abs(p.options.intensity-(result.ray_alignment?.source_published_intensity??intensity))>1e-6) throw Error('Requested Brush radius/intensity did not publish');
  result.before = (await page.save('before')).record;
  result.before_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  const viewport = await page.viewport();
  const center = [(viewport.screen[0]+viewport.screen[2])/2,(viewport.screen[1]+viewport.screen[3])/2];
  const scale=result.ray_alignment?.offset_scale??1;
  const pathPixels = retraceMoves ? Array.from({length:retraceMoves+1},(_,i)=>[center[0]+(i%2?92:80)*scale,center[1]]) :
    pressOnly ? [[center[0]+80*scale,center[1]]] : oneMove ?
    [[center[0]+80*scale,center[1]],[center[0]+92*scale,center[1]]] :
    [[center[0]+80*scale,center[1]],[center[0]+92*scale,center[1]],[center[0]+104*scale,center[1]]];
  result.requested_path = pathPixels;
  const deliveredAt = await page.evaluate('window.__uiBrushDelivered?.length??null');
  await page.pointer('mouseMoved',...pathPixels[0]);
  await page.pointer('mousePressed',...pathPixels[0],'left',1,{force:1}); pressed = true;
  await page.raf();
  for (const point of pathPixels.slice(1)) { await page.pointer('mouseMoved',...point,'left',1,{force:1}); await page.raf(); }
  await page.pointer('mouseReleased',...pathPixels.at(-1),'left',0); pressed = false;
  for (let i=0;i<12;i++) { await page.raf(); await pause(25); }
  result.dom_stroke_events = await page.evaluate(`window.__uiBrushDelivered?.slice(${deliveredAt})??null`);
  if (!result.dom_stroke_events?.length || result.dom_stroke_events.some(e=>!e.trusted)) throw Error('Trusted normal DOM stroke delivery missing');
  result.release_state = side === 'root' ? await page.evaluate('window.obrushSculptMetrics()') :
    await page.evaluate('({timer:window.__sculptglReferenceInstance.getSculptManager()._sculptTimer,action:window.__sculptglReferenceInstance._action})');
  result.after = (await page.save('after')).record;
  result.after_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  await page.chord('z'); result.undo = (await page.save('undo')).record;
  result.undo_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  await page.chord('z',true); result.redo = (await page.save('redo')).record;
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
