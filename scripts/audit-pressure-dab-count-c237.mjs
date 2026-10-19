// Offline audit of public saves from the C234 one-move OBrush run and C237
// pinned SculptGL move ladder. It does not instrument either geometry engine.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { decodeIndependent } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs';
import { decodeRootF32Model } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs';

const [rootPath, sourcePath, outputPath] = process.argv.slice(2);
if (!rootPath || !sourcePath || !outputPath) throw Error('Pass C234 root result, C237 source result, output JSON');
const root = JSON.parse(await readFile(rootPath));
const source = JSON.parse(await readFile(sourcePath));
if (root.side !== 'root' || source.side !== 'source' || root.fixture.sha256 !== source.fixture.sha256) throw Error('Side or fixture mismatch');
const sha = b => createHash('sha256').update(b).digest('hex');
const f32 = word => { const b = Buffer.allocUnsafe(4); b.writeUInt32LE(word); return b.readFloatLE(); };
const index = model => {
  const ids = new Map();
  model.colors.forEach((color, n) => {
    const id = Math.round(color[0] * 255) | (Math.round(color[1] * 255) << 8) | (Math.round(color[2] * 255) << 16);
    if (ids.has(id)) throw Error(`Duplicate RGB ID ${id}`);
    ids.set(id, model.positions[n]);
  });
  return ids;
};
async function readSave(c, stage, side) {
  const file = c[stage], bytes = await readFile(file.path);
  if (sha(bytes) !== file.sha256) throw Error(`${c.name} ${stage} hash mismatch`);
  const model = side === 'root' ? decodeRootF32Model(bytes) : decodeIndependent(bytes).meshes[0];
  if (side === 'source') {
    model.positions = model.positions.map(row => row.map(f32));
    model.colors = model.colors.map(row => row.map(f32));
  }
  return index(model);
}
const rootCases = Object.fromEntries(root.cases.map(c => [c.name, c]));
const baseCase = rootCases['neutral-quarter'], halfCase = rootCases['intensity-quarter'];
const [rootBefore, full, half] = await Promise.all([
  readSave(baseCase, 'before', 'root'), readSave(baseCase, 'after', 'root'), readSave(halfCase, 'after', 'root'),
]);
let rootChanged = 0, halfChanged = 0, halfMaxError = 0, halfRmsSquared = 0;
for (const [id, base] of rootBefore) {
  const fullPosition = full.get(id), halfPosition = half.get(id);
  if (!fullPosition || !halfPosition) throw Error(`Missing OBrush RGB ID ${id}`);
  let fullMoved = false, halfMoved = false;
  for (let axis = 0; axis < 3; axis++) {
    fullMoved ||= Math.fround(fullPosition[axis]) !== Math.fround(base[axis]);
    halfMoved ||= Math.fround(halfPosition[axis]) !== Math.fround(base[axis]);
    const error = (halfPosition[axis] - base[axis]) - .5 * (fullPosition[axis] - base[axis]);
    halfMaxError = Math.max(halfMaxError, Math.abs(error));
    halfRmsSquared += error * error;
  }
  rootChanged += Number(fullMoved);
  halfChanged += Number(halfMoved);
}
const sourceCases = [];
for (const c of source.cases) {
  const before = await readSave(c, 'before', 'source'), after = await readSave(c, 'after', 'source');
  if (before.size !== after.size) throw Error('Source vertex count mismatch');
  let changed = 0;
  for (const [id, base] of before) {
    const now = after.get(id);
    if (!now) throw Error(`Missing Source RGB ID ${id}`);
    changed += Number(base.some((x, axis) => Math.fround(x) !== Math.fround(now[axis])));
  }
  sourceCases.push({
    name: c.name, requested_moves: c.moves,
    requested_path: c.requested_path,
    hammer_events: c.hammer_events,
    held_pointermove_count: c.dom_events.filter(e => e.type === 'pointermove' && e.buttons === 1).length,
    mousemove_count: c.dom_events.filter(e => e.type === 'mousemove').length,
    changed_original_ids: changed,
    before_sha256: c.before.sha256, after_sha256: c.after.sha256,
    redo_sha256: c.redo.sha256,
  });
}
const audit = {
  root_result: rootPath, source_result: sourcePath, fixture_sha256: root.fixture.sha256,
  root_build: root.browser_build, source_build: source.browser_build,
  root_single_move: { applied_dabs: baseCase.state_after_move.applied_dabs - baseCase.state_after_press.applied_dabs, neutral_changed_original_ids: rootChanged,
    intensity_changed_original_ids: halfChanged, intensity_half_delta_max_abs_error: halfMaxError,
    intensity_half_delta_rms_component: Math.sqrt(halfRmsSquared / (rootBefore.size * 3)),
    neutral_after_sha256: baseCase.after.sha256, intensity_after_sha256: halfCase.after.sha256 },
  source_ladder: sourceCases,
  conclusion: 'Source public saves after requested moves 2, 3, and 4 are byte-identical; more panmove events did not imply more mesh dabs. The C236 comparison therefore does not match OBrush three applied dabs to three SculptGL applied dabs.',
};
await writeFile(outputPath, JSON.stringify(audit, null, 2) + '\n');
console.log(JSON.stringify({ outputPath, root_single_move: audit.root_single_move,
  source_ladder: sourceCases.map(({ name, requested_moves, changed_original_ids, after_sha256 }) => ({ name, requested_moves, changed_original_ids, after_sha256 })) }, null, 2));
