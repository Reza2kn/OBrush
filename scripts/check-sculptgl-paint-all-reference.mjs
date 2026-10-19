// Execute only the pinned local SculptGL Paint.paintAll method against an
// independently authored four-point channel fixture. This checks source
// arithmetic, not the reference browser's GUI, rendering, or undo manager.
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const sourcePath = process.argv[2]
  ?? '/Users/Ajab/AI/OBrush/.tools/sculptgl-reference/upstream/src/editing/tools/Paint.js';
const source = await readFile(sourcePath, 'utf8');
const paintAll = source.match(/  paintAll\(\) \{([\s\S]*?)\n  \}\n\}\n\s*export default Paint;/);
assert(paintAll, 'Pinned Paint.paintAll body was not found');
const runPaintAll = new Function(paintAll[1]);
const sourceHash = createHash('sha256').update(source).digest('hex');

const originalColors = Float32Array.from([
  0, 0, 0,
  0, .25, .5,
  .5, .5, .5,
  .75, .75, .75,
]);
const originalMaterials = Float32Array.from([
  .25, 0, 1,
  .5, .25, .5,
  .75, .5, .25,
  1, .75, 0,
]);
const colors = originalColors.slice();
const materials = originalMaterials.slice();
const geometry = Float32Array.from([
  0, 0, 0,
  1, 0, 0,
  1, 1, 0,
  0, 1, 0,
]);
const geometryBefore = geometry.slice();
const otherPart = Float32Array.from([.125, .25, .5]);
const otherPartBefore = otherPart.slice();
const calls = { state: 0, vertices: [], duplicate: 0, drawArrays: 0, render: 0 };
const mesh = {
  getColors: () => colors,
  getMaterials: () => materials,
  updateDuplicateColorsAndMaterials: () => { calls.duplicate++; },
  updateDrawArrays: () => { calls.drawArrays++; },
};
const tool = {
  getMesh: () => mesh,
  getUnmaskedVertices: () => Uint32Array.from([0, 1, 2]),
  pushState: force => { assert.equal(force, true); calls.state++; },
  _main: { getStateManager: () => ({ pushVertices: selected => {
    calls.vertices.push(Array.from(selected));
  } }) },
  _color: Float32Array.from([1, .5, .25]),
  _material: Float32Array.from([.75, 1, 0]),
  _writeAlbedo: true,
  _writeRoughness: true,
  _writeMetalness: false,
  updateRender: () => { calls.render++; },
};
runPaintAll.call(tool);

const expectedColors = [
  1, .5, .25,
  .5, .375, .375,
  .625, .5, .4375,
  .75, .75, .75,
];
const expectedMaterials = [
  .75, 0, 1,
  .625, .25, .5,
  .75, .5, .25,
  1, .75, 0,
];
assert.deepEqual(Array.from(colors), expectedColors);
assert.deepEqual(Array.from(materials), expectedMaterials);
assert.deepEqual(geometry, geometryBefore);
assert.deepEqual(otherPart, otherPartBefore);
assert.deepEqual(calls, {
  state: 1, vertices: [[0, 1, 2]], duplicate: 1, drawArrays: 1, render: 1,
});
console.log(JSON.stringify({
  reference_source_sha256: sourceHash,
  fixture: 'four independently authored points; mask [1, .5, .25, 0]; RGB and roughness on, metallic off',
  colors: Array.from(colors),
  materials: Array.from(materials),
  geometry_unchanged: true,
  other_part_unchanged: true,
  calls,
  scope: 'Pinned source Paint.paintAll method on mock mesh; browser GUI/history are not checked',
}, null, 2));
