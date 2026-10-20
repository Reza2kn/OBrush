// C334 bounded ordinary OBrush browser gate; this does not substitute for a paired reference run.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { projectWorld, integerPixelRayHit } from './camera-picked-pivot-c334-ray.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const { OwnedCdp } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const assert = (value, message) => { if (!value) throw Error(message); };
const fixture = path.join(root, 'docs/fixtures/camera-picked-pivot-c334.obj');
const fixtureBytes = await readFile(fixture);
assert(hash(fixtureBytes) === 'a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b', 'Frozen fixture changed');
const out = path.join(root, '.artifacts/camera-picked-pivot-c334-root-ui', randomUUID());
await mkdir(out, { recursive: true });
const result = { status: 'running', fixture_hash: hash(fixtureBytes), wasm_hash: hash(await readFile(path.join(root, 'web/dist/pkg/obrush_app_bg.wasm'))),
  cdp_trace: [], requests: [], ui_actions: [], input_trace: [], download_events: [], downloads: [], screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [], page_observations: [], cleanup_errors: [] };
const owner = new OwnedCdp(out, result);
let page;
const point = [0.4, -0.2, 0.052941176];
const project=(camera,screen,p=point)=>projectWorld(camera.view64,camera.projection,p,screen);
const maxDelta = (a,b) => Math.max(...a.map((v,i)=>Math.abs(v-b[i])));
async function snapshot() { return (await page.live()).state; }
async function caseRun(name, pivot, picked=point) {
  await page.rootCheckbox('camera.pivot', s=>s.camera.use_pivot, pivot);
  await page.rootControl('camera.reset');
  await page.rootControl('camera.center');
  await page.wait(()=>snapshot(),s=>!s.camera.transition_active&&s.camera.source_scale_pending==null,'Source camera Fit/Reset transition settled');
  for (let i=0;i<16;i++) await page.raf();
  const before = await snapshot(), viewport = await page.viewport();
  assert(viewport.logical_size?.join(',') === '1024,768', 'Measured 1024x768 setup failed');
  const projected=project(before.camera,viewport.screen,picked),[x,y]=projected.map(Math.round);
  assert(x>viewport.screen[0]&&x<viewport.screen[2]&&y>viewport.screen[1]&&y<viewport.screen[3], 'Projected pick outside viewport');
  const ray=integerPixelRayHit(before.camera.view64,before.camera.projection,[x,y],viewport.screen);
  await page.pointer('mouseMoved',x,y);
  await page.pointer('mousePressed',x,y,'right',2);
  await page.raf(); await page.raf();
  const held = await snapshot();
  await page.pointer('mouseReleased',x,y,'right',0);
  for(let i=0;i<4;i++) await page.raf();
  const after = await snapshot();
  const row = {name,pivot,point:picked,projected,screen:[x,y],ray,before:before.camera,held:held.camera,after:after.camera,
    history:[before.history,held.history,after.history],vertices:[before.vertices,held.vertices,after.vertices],
    held_view_delta:maxDelta(before.camera.view64,held.camera.view64),after_view_delta:maxDelta(before.camera.view64,after.camera.view64)};
  result.cases.push(row);
  if (before.camera.orthographic) {
    const xy=maxDelta(project(before.camera,viewport.screen),project(held.camera,viewport.screen));
    row.held_projected_xy_delta=xy;
    assert(xy<=0.25,name+' shifted orthographic projected point before drag');
  } else {
    assert(row.held_view_delta <= 2e-5, name+' moved view before drag');
    assert(row.after_view_delta <= 2e-5, name+' moved view after stationary release');
  }
  if (pivot) { assert(maxDelta(before.camera.precise_target,held.camera.precise_target)>1e-3, name+' did not retarget');
    assert(maxDelta(ray.point,held.camera.precise_target)<=.005,name+' missed independent integer-pixel ray hit'); }
  else assert(maxDelta(before.camera.precise_target,held.camera.precise_target)===0, name+' retargeted while disabled');
  assert(row.vertices.every(v=>v===before.vertices), name+' changed geometry count');
  await page.pointer('mouseMoved',x,y);
  await page.pointer('mousePressed',x,y,'right',2);
  await page.pointer('mouseMoved',x+40,y,'right',2);
  await page.pointer('mouseReleased',x+40,y,'right',0);
  for(let i=0;i<4;i++) await page.raf();
  const dragged=await snapshot();
  row.dragged=dragged.camera;
  row.history.push(dragged.history);
  row.drag_view_delta=maxDelta(after.camera.view64,dragged.camera.view64);
  assert(row.drag_view_delta>=1e-4,name+' did not orbit on 40px drag');
  assert(maxDelta(held.camera.precise_target,dragged.camera.precise_target)<1e-5,name+' lost picked orbit center');
  assert(dragged.vertices===before.vertices,name+' drag changed geometry count');
  assert(row.history.every(h=>h.undo===before.history.undo&&h.redo===before.history.redo),name+' changed Undo/Redo counts');
  return row;
}
try {
  const url = await owner.serve(path.join(root,'web/dist'),'root');
  await owner.start();
  page = await owner.page('root',url,'');
  Object.setPrototypeOf(page, PerfDriver.prototype);
  await page.send('Page.bringToFront');
  await page.alignViewport();
  await page.clear();
  const initial = await snapshot();
  await page.file('#import-file',fixture,()=>page.chord('o'));
  await page.wait(()=>snapshot(),s=>s.document_revision>initial.document_revision&&s.vertices===4,'Independent four-vertex OBJ imported');
  await page.rootControl('camera.framing');
  await page.rootControl('camera.framing.source');
  await page.rootCheckbox('camera.orthographic',s=>s.camera.orthographic,false);
  result.cases=[];
  await caseRun('perspective-on',true);
  await caseRun('perspective-off',false);
  await page.rootCheckbox('camera.orthographic',s=>s.camera.orthographic,true);
  await caseRun('orthographic-on',true);
  await page.rootCheckbox('camera.orthographic',s=>s.camera.orthographic,false);
  await caseRun('perspective-second-point',true,[.65,.25,.15*(.25+.8)/1.7]);
  const beforeFit=await snapshot();
  assert(Math.max(...beforeFit.camera.source_framing.pivot_offset.map(Math.abs))>1e-3,'Second pivot did not leave an offset');
  await page.rootControl('camera.center');
  await page.wait(()=>snapshot(),s=>s.camera.source_framing.pivot_offset.every(v=>v===0),'Normal Fit clears picked pivot offset');
  result.fit_clear=(await snapshot()).camera.source_framing.pivot_offset;
  await page.rootControl('camera.reset');
  await page.wait(()=>snapshot(),s=>s.camera.source_framing.pivot_offset.every(v=>v===0),'Normal Reset clears picked pivot offset');
  result.reset_clear=(await snapshot()).camera.source_framing.pivot_offset;
  result.status='pass bounded OBrush picked pivot, second point, orbit, Fit/Reset';
} catch(error) { result.status='failed'; result.failure=String(error.stack??error); process.exitCode=1; }
finally { await owner.close(); await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)); console.log(JSON.stringify({out,status:result.status,failure:result.failure,cases:result.cases?.map(c=>({name:c.name,held_view_delta:c.held_view_delta,after_view_delta:c.after_view_delta,held_projected_xy_delta:c.held_projected_xy_delta,drag_view_delta:c.drag_view_delta,before_target:c.before.precise_target,held_target:c.held.precise_target}))})); }
