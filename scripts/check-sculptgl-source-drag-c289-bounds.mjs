// C289 read-only normal-browser camera-fit witness. No sculpt stroke or product edit.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

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
    sha(fixtureBytes) !== expected.sha256 || fixtureBytes.length !== expected.bytes) {
  throw Error('C289 independent fixture bytes changed');
}
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const referenceHash = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (referenceHash !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') {
  throw Error('Pinned SculptGL bundle changed');
}
const web = path.resolve(process.env.OBRUSH_WEB_DIST ?? path.join(root, 'web/dist'));
const webUrl = process.env.OBRUSH_WEB_URL ?? null;
const out = path.join(root, '.artifacts/sculptgl-source-drag-c289-bounds', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {
  protocol: 'c289-read-only-normal-ui-camera-bounds-before-stroke', side,
  harness_sha256: sha(await readFile(new URL(import.meta.url))),
  source_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  reference_commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  reference_sha256: referenceHash, fixture_sha256: sha(fixtureBytes),
  web_dist: side === 'root' ? web : null, web_url: side === 'root' ? webUrl : null,
  browser_build_sha256: side === 'root' ? sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) : referenceHash,
  browser_js_sha256: side === 'root' ? sha(await readFile(path.join(web, 'pkg/obrush_app.js'))) : null,
  requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [], downloads: [],
  screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [],
  page_observations: [], cleanup_errors: [], snapshots: {}, status: 'running'
};
const owner = new OwnedCdp(out, result);
let page;
const sourceWitness = `(()=>{
  const a=window.__sculptglReferenceInstance,m=a?.getMesh(),c=a?.getCamera();
  const meshes=a?.getMeshes()??[];
  const box=meshes.length?a.computeBoundingBoxMeshes(meshes):null;
  const timerEntries=Object.entries(c?._timers??{}).map(([name,value])=>({name,value}));
  return {active_mesh_id:m?.getID?.()??null,mesh_count:meshes.length,vertex_count:m?.getNbVertices(),
    matrix:m?Array.from(m.getMatrix()):null,local:m?Array.from(m.getLocalBound()):null,
    world:m?Array.from(m.computeWorldBound()):null,scene_box:box?Array.from(box):null,
    camera:c?{center:Array.from(c._center),offset:Array.from(c._offset),translation:Array.from(c._trans),
      projection:Array.from(c._proj),view:Array.from(c._view),near:c._near,far:c._far,
      projection_type:c._projectionType,width:c._width,height:c._height,
      timer_entries:timerEntries,timers_settled:timerEntries.every(({value})=>value===0||value===-1||value===null)}:null};
})()`;
try {
  if (side === 'root' && webUrl) {
    result.served_http_hashes = {};
    for (const name of ['pkg/obrush_app_bg.wasm', 'pkg/obrush_app.js']) {
      const response = await fetch(new URL(name, webUrl), { cache: 'no-store' });
      if (!response.ok) throw Error(`Existing OBrush server returned ${response.status} for ${name}`);
      const actual = sha(Buffer.from(await response.arrayBuffer()));
      const expectedHash = sha(await readFile(path.join(web, name)));
      result.served_http_hashes[name] = { actual, expected: expectedHash, exact: actual === expectedHash };
      if (actual !== expectedHash) throw Error(`Existing OBrush server serves stale ${name}`);
    }
  }
  const url = side === 'root' && webUrl ? webUrl : await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  page = await owner.page(side, url, '');
  Object.setPrototypeOf(page, PerfDriver.prototype);
  if (side === 'root') page.alignViewport = async function() {
    const base = await PerfDriver.prototype.alignViewport.call(this);
    if (JSON.stringify(base.logical_size) !== '[1024,768]') throw Error('OBRush base viewport is not 1024x768');
    const { bounds } = await this.owner.send('Browser.getWindowBounds', { windowId: this.windowId });
    await this.owner.send('Browser.setWindowBounds', { windowId: this.windowId, bounds: { ...bounds, height: bounds.height + 40 } });
    for (let i = 0; i < 8; i++) {
      await this.raf();
      const viewport = await this.viewport();
      if (JSON.stringify(viewport.logical_size) === '[1024,808]') return viewport;
    }
    throw Error('Normal OBrush 1024x808 viewport not reached');
  };
  await page.send('Page.bringToFront');
  if (side === 'source') {
    await page.alignViewport();
    for (const [label, value] of [['Scale and center', false], ['Export all', true], ['sRGB vertex color', true]])
      await page.sourceCheckbox('_ctrlFiles', label, value);
    await page.clear();
    await page.file('#fileopen', fixture);
    await page.wait(() => page.state(), state => state.meshes.length === 1 &&
      state.meshes[0].vertices === expected.vertices, 'C289 Source fixture import');
    result.snapshots.after_import = { witness: await page.evaluate(sourceWitness), state: await page.state(), viewport: await page.viewport() };
    await page.sourceSelect('_ctrlSculpting', page.controller('_ctrlSculpting') + '._ctrlSculpt.domSelect', 7);
    const menu = page.controller('_ctrlSculpting') + '._menu.domUl';
    const nums = `Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
    await page.sourceNumber('_ctrlSculpting', nums + '[0]', 96);
    for (const [label, value] of [['Symmetry', false], ['Continuous', false]])
      await page.sourceCheckbox('_ctrlSculpting', label, value);
    const tablet = page.controller('_ctrlTablet') + '._menu.domUl.querySelectorAll("input[type=number]")';
    await page.sourceNumber('_ctrlTablet', tablet + '[0]', 0);
    await page.sourceNumber('_ctrlTablet', tablet + '[1]', 0);
    await page.sourceSelect('_ctrlCamera', page.controller('_ctrlCamera') + '._ctrlProjection.domSelect', 1);
    await page.sourceButton('_ctrlCamera', 'Reset (bar)');
    await page.closeMenus();
    const viewport = await page.viewport();
    const point = [viewport.screen[0] + 16, viewport.screen[1] + viewport.pixels[3] - 16];
    await page.pointer('mouseMoved', ...point);
    if (await page.evaluate('window.__sculptglReferenceInstance._focusGui!==false')) throw Error('Source canvas does not own fit gesture');
    for (let tap = 0; tap < 2; tap++) {
      await page.pointer('mousePressed', ...point, 'left', 1, { clickCount: tap + 1 });
      await page.pointer('mouseReleased', ...point, 'left', 0, { clickCount: tap + 1 });
    }
    await page.wait(() => page.evaluate('Object.values(window.__sculptglReferenceInstance.getCamera()._timers).every(v=>v===0||v===-1||v===null)'),
      settled => settled, 'C289 Source camera timers settle');
    await page.closeMenus();
    result.snapshots.after_fit = { witness: await page.evaluate(sourceWitness), state: await page.state(), viewport: await page.viewport() };
    if (!result.snapshots.after_fit.witness.camera?.timers_settled ||
        result.snapshots.after_fit.witness.vertex_count !== expected.vertices ||
        result.snapshots.after_fit.witness.mesh_count !== 1 ||
        !result.snapshots.after_fit.witness.local?.every(Number.isFinite) ||
        !result.snapshots.after_fit.witness.world?.every(Number.isFinite) ||
        !result.snapshots.after_fit.witness.scene_box?.every(Number.isFinite)) {
      throw Error('C289 Source bounds/timer witness incomplete');
    }
    const tool = (await page.live()).state.tool;
    if (tool?._lockPosition === true || ![0, 'None'].includes(tool?._idAlpha)) throw Error('Source Drag/Alpha None setup changed');
  } else {
    const setup = await page.prepareFixture({ path: fixture, vertices: expected.vertices });
    result.snapshots.after_fit = { state: setup.state, viewport: setup.viewport,
      camera: setup.state.camera, object: await page.evaluate('window.obrushBrushMetrics()') };
    if (setup.state.camera?.source_framing?.policy !== 'sculptgl_static_v1') throw Error('OBRush source camera frame unavailable');
  }
  if (JSON.stringify(result.snapshots.after_fit.viewport.logical_size) !== '[1024,808]')
    throw Error('C289 normal-UI 1024x808 viewport not reached');
  result.public_before = (await page.save('before')).record;
  result.status = 'captured read-only normal-UI bounds and settled camera; no stroke';
} catch (error) {
  result.status = 'FAILED read-only normal-UI camera diagnostic; no stroke';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  try { await owner.close(); } catch (error) { result.cleanup_errors.push(String(error)); process.exitCode = 1; }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ out, status: result.status, failure: result.failure ?? null, cleanup_errors: result.cleanup_errors }));
}
