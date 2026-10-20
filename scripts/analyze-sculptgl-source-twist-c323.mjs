// C323 read-only differential analysis of the frozen C318 normal-UI receipts.
// No product code or browser interaction. Run with OBRUSH_SHARED_ROOT at the
// integration checkout that retains the independent fixture decoders.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const local = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? local);
const { decodeIndependent } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const { decodeRootF32Model } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const sourcePath = process.argv[2] ?? path.join(local, '.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/result.json');
const rootPath = process.argv[3] ?? path.join(local, '.artifacts/sculptgl-source-twist-c318-ui/off-root-cedcecb2-8563-4039-8b9c-55aadba8970b/result.json');
const outputPath = process.argv[4] ?? path.join(local, '.artifacts/sculptgl-source-twist-c323-analysis.json');
const sha = b => createHash('sha256').update(b).digest('hex');
const sBytes = await readFile(sourcePath), rBytes = await readFile(rootPath);
if (sha(sBytes) !== 'e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b' ||
    sha(rBytes) !== '0cec63c177442128f8b1ac6caf3422ed3faaed0fc2cf19cd2c3c6647336809f4')
  throw Error('C318 source/root result identity changed');
const runs = { source: JSON.parse(sBytes), root: JSON.parse(rBytes) };
const word = w => { const b = Buffer.allocUnsafe(4); b.writeUInt32LE(w); return b.readFloatLE(); };
const meshes = {};
for (const side of ['source', 'root']) {
  const entry = runs[side].before;
  const bytes = await readFile(entry.path);
  if (sha(bytes) !== entry.sha256) throw Error(`${side} Before Save changed`);
  const raw = side === 'source' ? decodeIndependent(bytes).meshes[0] : decodeRootF32Model(bytes);
  const positions = side === 'source' ? raw.positions.map(row => row.map(word)) : raw.positions;
  const colors = side === 'source' ? raw.colors.map(row => row.map(word)) : raw.colors;
  const masks = side === 'source' ? raw.materials.map(row => word(row[2])) : raw.unprotected;
  const ids = colors.map(c => Math.round(c[0] * 255) | (Math.round(c[1] * 255) << 8) | (Math.round(c[2] * 255) << 16));
  const byId = Array(ids.length), protection = Array(ids.length);
  ids.forEach((id, i) => { if (byId[id]) throw Error(`${side} duplicate RGB ID ${id}`); byId[id] = positions[i]; protection[id] = masks[i]; });
  if (byId.some(row => !row)) throw Error(`${side} missing RGB ID`);
  meshes[side] = { ids, byId, protection };
}
function gpu(side, held) {
  const observed = runs[side].held_previews[held].gpu;
  const bytes = Buffer.from(observed.position_words_base64, 'base64');
  const ids = meshes[side].ids;
  if (sha(bytes) !== observed.position_words_sha256 || bytes.length !== ids.length * 12)
    throw Error(`${side} held${held} GPU position words changed`);
  const result = Array(ids.length);
  ids.forEach((id, i) => { result[id] = [0, 1, 2].map(c => bytes.readFloatLE(i * 12 + c * 4)); });
  return result;
}
const xyAngle = (p, q, center) => {
  const x = p[0] - center[0], y = p[1] - center[1];
  const u = q[0] - center[0], v = q[1] - center[1];
  return Math.atan2(x * v - y * u, x * u + y * v);
};
const source = runs.source.press_pick;
if (!source || source.eye.join(',') !== '0,0,-1') throw Error('Pinned C318 Source Twist axis not +Z');
const center = source.point, radius = Math.sqrt(source.radius_squared);
const changed = [474,475,476,477,478,506,507,508,509,510,511,539,540,541,542,544,545,572,573,574,575,576,577,578,605,606,607,608,609,610,639,640,641,642,643,673,674];
const models = {};
for (const side of ['source', 'root']) {
  const before = meshes[side].byId;
  const fixed = changed.map(id => {
    const p = before[id], d = Math.hypot(p[0] - center[0], p[1] - center[1], p[2] - center[2]) / radius;
    const sourceKernel = 1 - 4 * d ** 3 + 3 * d ** 4;
    const rootKernel = (1 - d * d) ** 2;
    return { id, d, mask: meshes[side].protection[id], sourceKernel, rootKernel };
  });
  models[side] = {};
  for (const held of [1, 2]) {
    const actual = gpu(side, held);
    const samples = fixed.map(row => ({ ...row, angle: xyAngle(before[row.id], actual[row.id], center) }));
    const alternatives = {};
    for (const kernel of ['sourceKernel', 'rootKernel']) {
      let numerator = 0, denominator = 0;
      for (const row of samples) {
        const weight = row[kernel] * row.mask;
        numerator += row.angle * weight;
        denominator += weight * weight;
      }
      const theta = numerator / denominator;
      let maxAngleResidual = 0, sumAngleResidual = 0, maxPositionResidual = 0, sumPositionResidual = 0;
      const rows = [];
      for (const row of samples) {
        const p = before[row.id], q = actual[row.id], predictedAngle = theta * row[kernel] * row.mask;
        const a = predictedAngle, x = p[0] - center[0], y = p[1] - center[1];
        const predicted = [center[0] + x * Math.cos(a) - y * Math.sin(a), center[1] + x * Math.sin(a) + y * Math.cos(a), p[2]];
        const angleResidual = row.angle - predictedAngle;
        const positionResidual = Math.hypot(...q.map((v, i) => v - predicted[i]));
        maxAngleResidual = Math.max(maxAngleResidual, Math.abs(angleResidual));
        sumAngleResidual += angleResidual ** 2;
        maxPositionResidual = Math.max(maxPositionResidual, positionResidual);
        sumPositionResidual += positionResidual ** 2;
        rows.push({ id: row.id, d: row.d, mask: row.mask, actualAngle: row.angle, predictedAngle, angleResidual, positionResidual });
      }
      alternatives[kernel] = { fittedTotalAngle: theta, maxAngleResidual, rmsAngleResidual: Math.sqrt(sumAngleResidual / samples.length), maxPositionResidual, rmsPositionResidual: Math.sqrt(sumPositionResidual / samples.length), rows };
    }
    models[side][`held${held}`] = alternatives;
  }
}
const output = { protocol: 'C323 read-only C318 exact-residual differential', inputs: { sourcePath, sourceSha256: sha(sBytes), rootPath, rootSha256: sha(rBytes), center, radius, changed }, models };
await writeFile(outputPath, JSON.stringify(output, null, 2) + '\n');
console.log(JSON.stringify({ outputPath, summary: Object.fromEntries(Object.entries(models).map(([side, held]) => [side, Object.fromEntries(Object.entries(held).map(([key, fits]) => [key, Object.fromEntries(Object.entries(fits).map(([kernel, m]) => [kernel, { fittedTotalAngle: m.fittedTotalAngle, maxPositionResidual: m.maxPositionResidual, rmsPositionResidual: m.rmsPositionResidual }]))]))])) }, null, 2));
