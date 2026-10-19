// Pinned SculptGL direct Inflate method probe for the preregistered diamond.
// This does not exercise a normal UI stroke, Save, Undo, rendering or queues.
// Usage: OBRUSH_SHARED_ROOT=/Users/Ajab/AI/OBrush node scripts/check-sculptgl-inflate-kernel-pair.mjs
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fixturePath = path.join(root, 'crates/obrush-core/tests/fixtures/independent-inflate-diamond.obj');
const fixtureBytes = await readFile(fixturePath);
const fixtureHash = sha(fixtureBytes);
assert.equal(fixtureHash, '9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35');
const positions = fixtureBytes.toString('utf8').split('\n').filter(line => line.startsWith('v '))
  .map(line => line.trim().split(/\s+/).slice(1).map(Number));
const faces = fixtureBytes.toString('utf8').split('\n').filter(line => line.startsWith('f '))
  .map(line => line.trim().split(/\s+/).slice(1).map(x => Number(x) - 1));
assert.deepEqual(positions, [[-0.5, 0, 0], [0, -0.5, 0], [0.5, 0, 0], [0, 0.5, 0]]);
assert.deepEqual(faces, [[0, 1, 2], [0, 2, 3]]);
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinnedBundle = sha(await readFile(path.join(mirror, 'sculptgl.js')));
assert.equal(pinnedBundle, '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600');
const { OwnedCdp } = await import(pathToFileURL(path.join(shared,
  '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const out = path.join(root, '.artifacts/sculptgl-inflate-kernel-pair', randomUUID());
await mkdir(out, { recursive: true });
const result = {
  status: 'running', pinned_bundle_sha256: pinnedBundle, fixture_sha256: fixtureHash,
  fixture_positions: positions, fixture_faces: faces, masks: [1, 0.5, 0, 1],
  cases: [], requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [],
  downloads: [], screenshots: [], dialogs: [], exceptions: [], console_errors: [],
  blocked_external_requests: [], cleanup_errors: [],
};
const owner = new OwnedCdp(out, result);
try {
  const url = await owner.serve(mirror, 'pinned-sculptgl');
  await owner.start();
  const page = await owner.page('pinned-sculptgl', url, '');
  for (let i = 0; i < 150; i++) {
    if (await page.evaluate('!!window.__sculptglReferenceInstance?.getSculptManager?.()')) break;
    if (i === 149) throw Error('Pinned SculptGL instance startup deadline');
    await new Promise(resolve => setTimeout(resolve, 60));
  }
  for (const { name, negative, intensity } of [
    { name: 'positive', negative: false, intensity: 0.3 },
    { name: 'negative', negative: true, intensity: 0.3 },
    { name: 'zero', negative: false, intensity: 0 },
  ]) {
    const measured = await page.evaluate(`(() => {
      const app = window.__sculptglReferenceInstance;
      const tool = app.getSculptManager().getTool(1);
      if (typeof tool?.inflate !== 'function') throw Error('Pinned Inflate tool missing');
      const vertices = Float32Array.from(${JSON.stringify(positions.flat())});
      const proxy = Float32Array.from(vertices);
      const normals = Float32Array.from([0,0,1, 0,0,1, 0,0,1, 0,0,1]);
      const materials = Float32Array.from([0,0,1, 0,0,.5, 0,0,0, 0,0,1]);
      const fake = { getVertices:() => vertices, getVerticesProxy:() => proxy,
        getNormals:() => normals, getMaterials:() => materials };
      const pick = app.getPicking(), oldAlpha = pick._alpha;
      const oldMesh = tool._forceToolMesh, oldNegative = tool._negative;
      try {
        tool.setToolMesh(fake);
        tool._negative = ${negative};
        pick._alpha = null;
        tool.inflate(new Uint32Array([0,1,2,3]), [0,0,0], 1, ${intensity}, pick);
        return {positions:Array.from(vertices),bits:Array.from(new Uint32Array(vertices.buffer)),
          proxy:Array.from(proxy), normals:Array.from(normals), materials:Array.from(materials),
          negative:tool._negative, intensity:${intensity}, alpha_null:pick._alpha===null};
      } finally {
        pick._alpha = oldAlpha;
        tool._negative = oldNegative;
        tool.setToolMesh(oldMesh);
      }
    })()`);
    const expected = [0.020625, 0.0103125, 0, 0.020625];
    const sign = negative ? -1 : 1;
    for (let vertex = 0; vertex < 4; vertex++) {
      assert.equal(measured.positions[vertex * 3], positions[vertex][0]);
      assert.equal(measured.positions[vertex * 3 + 1], positions[vertex][1]);
      const want = intensity === 0 ? 0 : sign * expected[vertex];
      assert.ok(Math.abs(measured.positions[vertex * 3 + 2] - want) <= 3e-8,
        `reference ${name} vertex ${vertex} differs from preregistered arithmetic`);
    }
    assert.equal(measured.positions[8], 0, 'masked ID 2 changed');
    result.cases.push({ name, ...measured });
  }
  assert.equal(result.exceptions.length, 0);
  assert.equal(result.console_errors.length, 0);
  result.status = 'passed scoped pinned direct Inflate arithmetic';
} catch (error) {
  result.status = 'failed';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  try { await owner.close(); } catch (error) {
    result.cleanup_errors.push(String(error.stack ?? error));
    process.exitCode = 1;
  }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ out, status: result.status, failure: result.failure ?? null }));
}
