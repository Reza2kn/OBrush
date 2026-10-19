// C270 normal-UI Inflate pair. Run separately with --side=source|root and
// --negative=off|on. Requires OBRUSH_SHARED_ROOT for the pinned reference and
// ignored owned-CDP driver when invoked from an isolated worktree.
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const side = process.argv.find(a => a.startsWith('--side='))?.slice(7);
const polarity = process.argv.find(a => a.startsWith('--negative='))?.slice(11);
assert(['source', 'root'].includes(side) && ['off', 'on'].includes(polarity), 'Use --side=source|root --negative=off|on');
const negative = polarity === 'on';
const longInterior = process.argv.includes('--long-interior');
const interior = longInterior || process.argv.includes('--interior');
const sourcePath = process.argv.find(a => a.startsWith('--source-result='))?.slice(16);
assert(side === 'root' || !sourcePath, 'Only root can bind a source result');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fixture = path.join(root, 'crates/obrush-core/tests/fixtures/independent-inflate-diamond.obj');
const fixtureHash = sha(await readFile(fixture));
assert.equal(fixtureHash, '9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinnedHash = sha(await readFile(path.join(mirror, 'sculptgl.js')));
assert.equal(pinnedHash, '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600');
const web = path.join(root, 'web/dist');
const { OwnedCdp } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const out = path.join(root, '.artifacts/sculptgl-inflate-ui-diamond', `${side}-${polarity}-${longInterior?'long-interior':interior?'interior':'edge'}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {
  status: 'running', side, negative, cohort: longInterior?'C270C-long-interior':interior?'C270B-interior':'C270-edge', fixture: { path: fixture, sha256: fixtureHash, vertices: 4, triangles: 2 },
  pinned_bundle_sha256: pinnedHash,
  obrush_wasm_sha256: side === 'root' ? sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) : null,
  protocol_commit: 'a7d3f5a', source_result: sourcePath ?? null,
  requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [], downloads: [],
  screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [],
  page_observations: [], cleanup_errors: [],
};
// Capture-phase DOM observer and read-only active GL vertex word probe. It
// instruments the browser before navigation and never writes application state.
const observer = `(()=>{
  const events=[]; window.__inflateUiEvents=events;
  for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])
    window.addEventListener(type,e=>{if(events.length<1024)events.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons,pressure:e.pressure,trusted:e.isTrusted,at:performance.now()});},true);
  const original=HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(kind!=='webgl2'||!gl||gl.__inflateProbe)return gl;
    gl.__inflateProbe=true;window.__inflateGl=gl;const drawn=[];window.__inflateDrawn=drawn;
    const draw=gl.drawElements;gl.drawElements=function(...args){
      const buffer=this.getVertexAttrib(0,this.VERTEX_ATTRIB_ARRAY_BUFFER_BINDING);
      if(buffer)drawn.push({buffer,at:performance.now(),count:args[1]});
      if(drawn.length>256)drawn.shift();return draw.apply(this,args);
    };
    return gl;
  };
  window.__readInflateGpu=async()=>{
    const gl=window.__inflateGl,rows=[];if(!gl)return {error:'no WebGL2 context',rows};
    const buffers=[...new Set(window.__inflateDrawn.map(row=>row.buffer))];
    const prior=gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    try{for(const buffer of buffers){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);const bytes=gl.getBufferParameter(gl.ARRAY_BUFFER,gl.BUFFER_SIZE);
      if(bytes!==160)continue;const data=new Uint8Array(bytes);gl.getBufferSubData(gl.ARRAY_BUFFER,0,data);
      const digest=await crypto.subtle.digest('SHA-256',data);
      rows.push({bytes,sha256:Array.from(new Uint8Array(digest)).map(x=>x.toString(16).padStart(2,'0')).join(''),
        position_words:Array.from(new Uint32Array(data.buffer)).filter((_,i)=>i%10<3),
        normal_words:Array.from(new Uint32Array(data.buffer)).filter((_,i)=>i%10>=3&&i%10<6)});
    }}finally{gl.bindBuffer(gl.ARRAY_BUFFER,prior);}return {draws:window.__inflateDrawn.length,active_candidates:rows.length,rows,gl_error:gl.getError()};
  };
})();` + (side === 'source' ? '\n' +
  await readFile(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/observer/focused-alpha-input-observer.js'), 'utf8') + '\n' +
  await readFile(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/observer/focused-alpha-hit-observer.js'), 'utf8') : '');
const owner = new OwnedCdp(out, result);
let page, pressed = false;
async function prepareRootDiamond(page) {
  await page.alignViewport();
  await page.clear();
  const before = await page.state();
  await page.file('#import-file', fixture, () => page.chord('o'));
  await page.wait(() => page.state(), s => s.vertices === 4 && s.document_revision > before.document_revision, 'Complete four-vertex OBJ import');
  await page.rootControl('brush.draw');
  await page.rootCheckbox('settings.symmetry', s => s.symmetry_x, false);
  await page.rootCheckbox('settings.invert', s => s.invert, false);
  await page.rootCheckbox('settings.lock_position', s => s.lock_position, false);
  // The public OBJ imports with protected-mask representation. Source mode
  // explicitly requires raw unprotected U, even when all weights are one.
  await page.rootControl('source_geometry.mask.preview');
  const preview = await page.state();
  const convertControl = page.found(preview, 'source_geometry.mask.convert');
  result.root_mask_preview = { control: convertControl, message: preview.message };
  assert.equal(convertControl?.enabled, true,
    'Exact mask conversion unavailable after visible preview');
  await page.rootControl('source_geometry.mask.convert');
  result.root_mask_conversion = await page.state();
  await page.rootControl('source_geometry.mode.source');
  for (const [key,value] of [['source_geometry.radius.value',64],['source_geometry.intensity.value',.25],
    ['source_geometry.pressure.radius.value',0],['source_geometry.pressure.intensity.value',0]]) await page.rootNumber(key,value);
  const row=s=>s.source_geometry.per_tool[0];
  for (const [key,read,value] of [['source_geometry.clay',s=>row(s).options.tool.Brush.clay,false],
    ['source_geometry.accumulate',s=>row(s).options.tool.Brush.accumulate,false],
    ['source_geometry.negative',s=>row(s).options.negative,false],
    ['source_geometry.culling',s=>row(s).options.culling,false],
    ['source_geometry.prevent_folds',s=>row(s).options.prevent_folds,false],
    ['source_geometry.continuous',s=>s.continuous.requested,false]]) await page.rootCheckbox(key,read,value);
  await page.rootControl('camera.framing'); await page.rootControl('camera.framing.source');
  await page.rootCheckbox('camera.orthographic',s=>s.camera.orthographic,true);
  await page.rootControl('camera.reset');
  const camera=(await page.state()).camera;
  if (![0,5,10].every(i=>Math.abs(camera.view64[i]-1)<1e-6)) await page.rootControl('camera.front');
  await page.rootControl('scene.frame'); await page.closeMenus();
  const state=await page.state(),viewport=await page.viewport();
  return {state,viewport};
}
try {
  const url = await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  page = await owner.page(side, url, observer);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  await page.send('Page.bringToFront');
  result.setup = side === 'source' ? await page.prepareFixture({ path: fixture, vertices: 4 }) : await prepareRootDiamond(page);
  if (side === 'source') {
    await page.sourceSelect('_ctrlSculpting', page.controller('_ctrlSculpting') + '._ctrlSculpt.domSelect', 1);
    const menu = page.controller('_ctrlSculpting') + '._menu.domUl';
    const numbers = `Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
    await page.sourceNumber('_ctrlSculpting', numbers + '[0]', 400);
    await page.sourceNumber('_ctrlSculpting', numbers + '[1]', 30);
    await page.sourceMenu('_ctrlSculpting');
    const negativeControl = `Array.from(${menu}.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.includes('Negative')&&e.parentElement.getBoundingClientRect().width>0)`;
    result.source_negative_control = await page.evaluate(`(()=>{const e=${negativeControl},r=e?.parentElement.getBoundingClientRect();return {count:Array.from(${menu}.querySelectorAll('input[type=checkbox]')).filter(e=>e.parentElement.textContent.includes('Negative')).length,checked:e?.checked,disabled:e?.disabled,rect:r?[r.left,r.top,r.right,r.bottom]:null};})()`);
    assert(result.source_negative_control.rect && !result.source_negative_control.disabled, 'Visible source Negative checkbox unavailable');
    if (result.source_negative_control.checked !== negative) await page.sourceClick(`(${negativeControl}).parentElement`);
    assert.equal(await page.evaluate(`(${negativeControl}).checked`), negative);
  } else {
    await page.rootControl('brush.inflate');
    await page.rootControl('source_geometry.mode.source');
    await page.rootNumber('source_geometry.radius.value', 400);
    await page.rootNumber('source_geometry.intensity.value', .3);
    await page.rootNumber('source_geometry.pressure.radius.value', 0);
    await page.rootNumber('source_geometry.pressure.intensity.value', 0);
    await page.rootCheckbox('source_geometry.negative', s => s.source_geometry.per_tool[2].options.negative, negative);
    await page.rootCheckbox('source_geometry.culling', s => s.source_geometry.per_tool[2].options.culling, false);
    await page.rootCheckbox('source_geometry.prevent_folds', s => s.source_geometry.per_tool[2].options.prevent_folds, false);
  }
  await page.closeMenus();
  result.config = await page.live();
  const p = side === 'source' ? result.config.state.tool : result.config.state.source_geometry.per_tool[2];
  if (side === 'source') {
    assert.equal(result.config.state.tool_index, 1);
    const menu = page.controller('_ctrlSculpting') + '._menu.domUl';
    result.source_alpha_selector = await page.evaluate(`(()=>{const e=Array.from(${menu}.querySelectorAll('select')).find(e=>Array.from(e.options).some(o=>o.value==='None'));return e?{value:e.value,selected_text:e.selectedOptions[0]?.textContent,owner_id:window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._idAlpha}:null;})()`);
    assert.equal(p._negative, negative);
    assert.equal(result.source_alpha_selector?.value, 'None');
    assert.equal(p._culling, false); assert.equal(result.config.state.symmetry, false);
    assert.equal(result.config.state.continuous, false);
    const tablet = page.controller('_ctrlTablet') + '._menu.domUl.querySelectorAll("input[type=number]")';
    result.source_tablet_controls = await page.evaluate(`Array.from(${tablet}).map(e=>Number(e.value))`);
    assert.deepEqual(result.source_tablet_controls.slice(0, 2), [0, 0]);
    assert.ok(Math.abs(p._radius - 400) <= 1 && Math.abs(p._intensity - .3) <= .01);
  } else {
    assert.equal(result.config.state.kind, 'Inflate');
    assert.equal(p.options.negative, negative); assert.equal(p.options.culling, false);
    assert.equal(p.options.prevent_folds, false);
    assert.equal(p.radius, 400); assert.equal(p.options.intensity, .3);
    assert.equal(p.radius_factor, 0); assert.equal(p.options.intensity_factor, 0);
    assert.equal(result.config.state.symmetry_x, false);
  }
  await page.screenshot('before-stroke');
  result.before = (await page.save('before')).record;
  const viewport = await page.viewport();
  const center = [(viewport.screen[0] + viewport.screen[2]) / 2, (viewport.screen[1] + viewport.screen[3]) / 2];
  let scale = 1, scaleY = 1;
  if (side === 'root' && sourcePath) {
    const source = JSON.parse(await readFile(sourcePath));
    assert.equal(source.side, 'source'); assert.equal(source.negative, negative);
    assert.equal(source.cohort, result.cohort);
    assert.equal(source.fixture.sha256, fixtureHash);
    const sourcePx = source.setup.state.camera.projection[0], rootPx = result.setup.state.camera.projection[0];
    const sourceWidth = source.setup.viewport.screen[2] - source.setup.viewport.screen[0];
    const rootWidth = viewport.screen[2] - viewport.screen[0];
    scale = rootPx / sourcePx * rootWidth / sourceWidth;
    assert(Number.isFinite(scale) && scale > .5 && scale < 2);
    const sourcePy = source.setup.state.camera.projection[5], rootPy = result.setup.state.camera.projection[5];
    const sourceHeight = source.setup.viewport.screen[3] - source.setup.viewport.screen[1];
    const rootHeight = viewport.screen[3] - viewport.screen[1];
    scaleY = rootPy / sourcePy * rootHeight / sourceHeight;
    assert(Number.isFinite(scaleY) && scaleY > .5 && scaleY < 2);
    result.ray_alignment = { source_projection_x: sourcePx, root_projection_x: rootPx, source_width: sourceWidth, root_width: rootWidth, scale,
      source_projection_y:sourcePy,root_projection_y:rootPy,source_height:sourceHeight,root_height:rootHeight,scale_y:scaleY,source_result: sourcePath };
    await page.rootNumber('source_geometry.radius.value', source.config.state.tool._radius * scale);
    result.config = await page.live();
  }
  const y=center[1]+(interior?64*scaleY:0);
  const points = [[center[0] + 8 * scale, y], [center[0] + (longInterior?88:24) * scale, y]];
  result.requested_path = points;
  const baseline = await page.evaluate('window.__inflateUiEvents.length');
  await page.pointer('mouseMoved', ...points[0]);
  await page.pointer('mousePressed', ...points[0], 'left', 1, { force: 1 }); pressed = true;
  await page.raf();
  await page.pointer('mouseMoved', ...points[1], 'left', 1, { force: 1 });
  await page.raf();
  await page.pointer('mouseReleased', ...points[1], 'left', 0); pressed = false;
  for (let i = 0; i < 12; i++) await page.raf();
  result.dom_stroke_events = await page.evaluate(`window.__inflateUiEvents.slice(${baseline})`);
  assert(result.dom_stroke_events.length > 0 && result.dom_stroke_events.every(e => e.trusted));
  result.release_state = side === 'root' ? await page.evaluate('window.obrushSculptMetrics()') :
    await page.evaluate('({timer:window.__sculptglReferenceInstance.getSculptManager()._sculptTimer,action:window.__sculptglReferenceInstance._action})');
  await page.screenshot('after-stroke');
  result.after = (await page.save('after')).record;
  if (side === 'root') result.after_gpu = await page.evaluate('window.__readInflateGpu()');
  await page.chord('z'); result.undo = (await page.save('undo')).record;
  await page.chord('z', true); result.redo = (await page.save('redo')).record;
  if (side === 'root') result.redo_gpu = await page.evaluate('window.__readInflateGpu()');
  result.status = 'captured normal UI and public Save/Undo/Redo; independent audit pending';
} catch (error) {
  result.status = 'FAILED normal UI or public save capture; evidence retained';
  result.failure = String(error.stack ?? error); process.exitCode = 1;
} finally {
  if (pressed && page) try { const v = await page.viewport(); await page.pointer('mouseReleased', (v.screen[0]+v.screen[2])/2, (v.screen[1]+v.screen[3])/2, 'left', 0); }
  catch (error) { result.cleanup_errors.push(String(error)); }
  try { await owner.close(); } catch (error) { result.cleanup_errors.push(String(error)); process.exitCode = 1; }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ out, status: result.status, failure: result.failure ?? null, cleanup_errors: result.cleanup_errors }));
}
