// Paired normal-browser control and pinned front-vertex predicate probe.
// The opposed-sheet deformation/history fixture lives in source_history_tests.
// Usage: OBRUSH_SHARED_ROOT=/path/to/main node scripts/check-sculptgl-brush-culling-c262.mjs --side=source|root
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const driver = path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs');
const { OwnedCdp } = await import(pathToFileURL(driver).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const web = path.join(root, 'web/dist');
const side = process.argv.find(arg => arg.startsWith('--side='))?.slice(7);
assert(['source', 'root'].includes(side), 'Choose --side=source or --side=root');
const out = path.join(root, '.artifacts/sculptgl-brush-culling-c262', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const bundle = sha(await readFile(path.join(mirror, 'sculptgl.js')));
assert.equal(bundle, '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600');
const result = {
  status: 'running', side, pinned_bundle_sha256: bundle,
  obrush_wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))),
  normal_fixture: [[0, 0, 0], [0, 0, 1], [0, 0, 1], [0, 0, -1], [0, 0, -1]],
  eye_direction: [0, 0, -1], expected_front_ids: [0, 1, 2],
  cases: [], requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [], downloads: [],
  screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [],
  page_observations: [], cleanup_errors: [],
};
const owner = new OwnedCdp(out, result);
try {
  const url = await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  const page = await owner.page(side, url, '');
  if (side === 'source') {
    const sourcePredicate = async () => page.evaluate(`(()=>{
    const tool=window.__sculptglReferenceInstance.getSculptManager().getCurrentTool();
    const normals=new Float32Array(${JSON.stringify(result.normal_fixture.flat())});
    const receiver={getMesh:()=>({getNormals:()=>normals})};
    const all=new Uint32Array([0,1,2,3,4]);
    const front=Array.from(tool.getFrontVertices.call(receiver,all,new Float32Array([0,0,-1])));
    return {culling:tool._culling,front,write:tool._culling?front:Array.from(all)};
    })()`);
    const sourceOff = await sourcePredicate();
    assert.equal(sourceOff.culling, false);
    assert.deepEqual(sourceOff.front, result.expected_front_ids);
    assert.deepEqual(sourceOff.write, [0, 1, 2, 3, 4]);
    await page.sourceMenu('_ctrlSculpting');
    const expression = page.checkboxExpression('_ctrlSculpting', 'Thin surface');
    const before = await page.evaluate(`(()=>{const e=${expression},r=e.parentElement.getBoundingClientRect(),x=(r.left+r.right)/2,y=(r.top+r.bottom)/2,h=document.elementFromPoint(x,y);return {checked:e.checked,rect:[r.left,r.top,r.right,r.bottom],hit:h===e.parentElement||e.parentElement.contains(h)}})()`);
    assert.equal(before.checked, false);
    assert.equal(before.hit, true, 'Source Culling control center is covered');
    await page.sourceClick(`(${expression}).parentElement`);
    const sourceOn = await sourcePredicate();
    assert.equal(sourceOn.culling, true);
    assert.deepEqual(sourceOn.front, result.expected_front_ids);
    assert.deepEqual(sourceOn.write, result.expected_front_ids);
    result.cases.push({source_off:sourceOff,source_control_before:before,source_on:sourceOn,
      scope:'normal Source checkbox and direct pinned getFrontVertices method'});
  } else {
    Object.setPrototypeOf(page, PerfDriver.prototype);
    const sphereFixture = path.join(shared, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/fixtures/independent-id-sphere-10k.sgl');
    result.obrush_scene_fixture_sha256 = sha(await readFile(sphereFixture));
    result.obrush_setup = await page.prepareFixture({path:sphereFixture, vertices:10114});
    const rootOption = async () => (await page.state(false)).source_geometry.per_tool[0].options.culling;
    assert.equal(await rootOption(), false);
    await page.rootCheckbox('source_geometry.culling', s => s.source_geometry.per_tool[0].options.culling, true);
    assert.equal(await rootOption(), true);
    result.cases.push({obrush_options:[false,await rootOption()],
      scope:'normal OBrush Source culling checkbox and stored preference; geometry changes covered by core fixture'});
  }
  assert.equal(result.exceptions.length, 0);
  assert.equal(result.console_errors.length, 0);
  result.status = 'passed scoped '+side+' browser culling control';
} catch (error) {
  result.status = 'failed';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  try { await owner.close(); } catch (error) { result.cleanup_errors.push(String(error)); process.exitCode = 1; }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({out, status:result.status, failure:result.failure ?? null}));
}
