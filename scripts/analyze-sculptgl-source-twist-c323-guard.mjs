// Read-only approximation of the current shared-core Twist guard on C318's
// pinned public Save geometry. This does not invoke product code.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const local = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? local);
const { decodeIndependent } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const { decodeRootF32Model } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const sourcePath = path.join(local, '.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/result.json');
const rootPath = path.join(local, '.artifacts/sculptgl-source-twist-c318-ui/off-root-cedcecb2-8563-4039-8b9c-55aadba8970b/result.json');
const sha = b => createHash('sha256').update(b).digest('hex');
const expected = { source: 'e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b', root: '0cec63c177442128f8b1ac6caf3422ed3faaed0fc2cf19cd2c3c6647336809f4' };
const word = w => { const b = Buffer.allocUnsafe(4); b.writeUInt32LE(w); return b.readFloatLE(); };
const models = {};
for (const [side, resultPath] of Object.entries({ source: sourcePath, root: rootPath })) {
  const resultBytes = await readFile(resultPath);
  if (sha(resultBytes) !== expected[side]) throw Error(`${side} C318 receipt identity changed`);
  const result = JSON.parse(resultBytes), stages = {};
  for (const stage of ['before', 'after']) {
    const bytes = await readFile(result[stage].path);
    if (sha(bytes) !== result[stage].sha256) throw Error(`${side} ${stage} public Save identity changed`);
    const raw = side === 'source' ? decodeIndependent(bytes).meshes[0] : decodeRootF32Model(bytes);
    const positions = side === 'source' ? raw.positions.map(row => row.map(word)) : raw.positions;
    const colors = side === 'source' ? raw.colors.map(row => row.map(word)) : raw.colors;
    const ids = colors.map(c => Math.round(c[0] * 255) | (Math.round(c[1] * 255) << 8) | (Math.round(c[2] * 255) << 16));
    const mapped = Array(ids.length);
    ids.forEach((id, i) => { if (mapped[id]) throw Error(`Duplicate RGB ID ${id}`); mapped[id] = positions[i]; });
    const faces = raw.faces.map(face => face.slice(0, face[3] === 0xffffffff ? 3 : 4).map(i => ids[i]));
    if (faces.some(face => face.length !== 3)) throw Error('This guard audit expects triangle fixture');
    stages[stage] = { positions: mapped, faces, ids };
  }
  for (const held of [1, 2]) {
    const observed = result.held_previews[held].gpu;
    const bytes = Buffer.from(observed.position_words_base64, 'base64');
    if (sha(bytes) !== observed.position_words_sha256 || bytes.length !== stages.before.ids.length * 12)
      throw Error(`${side} held${held} active GPU position words changed`);
    const mapped = Array(stages.before.ids.length);
    stages.before.ids.forEach((id, i) => { mapped[id] = [0, 1, 2].map(c => bytes.readFloatLE(i * 12 + c * 4)); });
    stages[`held${held}`] = { positions: mapped, faces: stages.before.faces };
  }
  models[side] = stages;
}
const sub = (a, b) => a.map((x, i) => x - b[i]);
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const dot = (a, b) => a.reduce((s, x, i) => s + x * b[i], 0);
const len = a => Math.sqrt(dot(a, a));
const faceCross = (positions, face) => {
  const [a, b, c] = face.map(id => positions[id]);
  return cross(sub(b, a), sub(c, a));
};
const longestEdgeSquared = (positions, face) => {
  const [a, b, c] = face.map(id => positions[id]);
  return Math.max(dot(sub(a, b), sub(a, b)), dot(sub(b, c), sub(b, c)), dot(sub(c, a), sub(c, a)));
};
const changed = new Set([474,475,476,477,478,506,507,508,509,510,511,539,540,541,542,544,545,572,573,574,575,576,577,578,605,606,607,608,609,610,639,640,641,642,643,673,674]);
function guardMetrics(before, after) {
  const faces = before.faces;
  const affected = new Set(faces.map((face, i) => face.some(id => changed.has(id)) ? i : -1).filter(i => i >= 0));
  const failureFaces = [];
  let minimumProjectionRatio = Infinity, minimumQualityRatio = Infinity;
  for (const index of affected) {
    const face = faces[index], oldCross = faceCross(before.positions, face), newCross = faceCross(after.positions, face);
    const oldArea2 = dot(oldCross, oldCross), oldEdge2 = longestEdgeSquared(before.positions, face), newEdge2 = longestEdgeSquared(after.positions, face);
    const requiredProjection = oldArea2 * 0.05;
    const requiredQuality = Math.min(Math.sqrt(oldArea2) / oldEdge2, 0.1);
    const projectionRatio = dot(newCross, oldCross) / oldArea2;
    const qualityRatio = len(newCross) / newEdge2;
    minimumProjectionRatio = Math.min(minimumProjectionRatio, projectionRatio);
    minimumQualityRatio = Math.min(minimumQualityRatio, qualityRatio);
    const projectionFail = dot(newCross, oldCross) < requiredProjection;
    const qualityFail = dot(newCross, newCross) * (1 + 32 * Number.EPSILON) < (requiredQuality * newEdge2) ** 2;
    if (projectionFail || qualityFail) failureFaces.push({ index, rgbIds: face, projectionRatio, qualityRatio, requiredQuality, projectionFail, qualityFail });
  }
  const edgeFaces = new Map();
  faces.forEach((face, index) => {
    for (const [u, v] of [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]) {
      const key = Math.min(u, v) + ',' + Math.max(u, v);
      const rows = edgeFaces.get(key) ?? []; rows.push(index); edgeFaces.set(key, rows);
    }
  });
  const failureFolds = []; let minimumFoldCosine = Infinity, checkedFoldPairs = 0;
  for (const [edge, pair] of edgeFaces) {
    if (pair.length !== 2 || !pair.some(i => affected.has(i))) continue;
    checkedFoldPairs++;
    const oldNormals = pair.map(i => faceCross(before.positions, faces[i]));
    const newNormals = pair.map(i => faceCross(after.positions, faces[i]));
    const oldProduct = len(oldNormals[0]) * len(oldNormals[1]);
    const newProduct = len(newNormals[0]) * len(newNormals[1]);
    const oldCosine = dot(oldNormals[0], oldNormals[1]) / oldProduct;
    const newCosine = dot(newNormals[0], newNormals[1]) / newProduct;
    minimumFoldCosine = Math.min(minimumFoldCosine, newCosine);
    const requiredCosine = Math.min(oldCosine, 0);
    const foldFail = dot(newNormals[0], newNormals[1]) + 32 * Number.EPSILON * newProduct < requiredCosine * newProduct;
    if (foldFail) failureFolds.push({ edge, faces: pair, oldCosine, newCosine, requiredCosine });
  }
  return { affectedFaces: affected.size, checkedFoldPairs, minimumProjectionRatio, minimumQualityRatio, minimumFoldCosine, failureFaces, failureFolds };
}
const out = Object.fromEntries(Object.entries(models).map(([side, { before, held1, held2, after }]) =>
  [side, { held1: guardMetrics(before, held1), held2: guardMetrics(before, held2), after: guardMetrics(before, after) }]));
const output = path.join(local, '.artifacts/sculptgl-source-twist-c323-guard-analysis.json');
await writeFile(output, JSON.stringify(out, null, 2) + '\n');
console.log(JSON.stringify({ output, summary: Object.fromEntries(Object.entries(out).map(([side, stages]) => [side,
  Object.fromEntries(Object.entries(stages).map(([stage, m]) => [stage,
    { affectedFaces: m.affectedFaces, checkedFoldPairs: m.checkedFoldPairs, minimumProjectionRatio: m.minimumProjectionRatio,
      minimumQualityRatio: m.minimumQualityRatio, minimumFoldCosine: m.minimumFoldCosine,
      failureFaces: m.failureFaces.length, failureFolds: m.failureFolds.length,
      firstFaces: m.failureFaces.slice(0, 2), firstFolds: m.failureFolds.slice(0, 2) }]))])) }, null, 2));
