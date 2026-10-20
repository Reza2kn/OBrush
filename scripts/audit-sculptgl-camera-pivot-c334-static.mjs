// C334 read-only static source/fixture and camera-identity preflight.
// It does not execute the OBrush candidate or either browser.
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const source = await readFile(path.join(shared, '.tools/sculptgl-reference/upstream/src/math3d/Camera.js'));
const fixture = await readFile(path.join(root, 'docs/fixtures/camera-picked-pivot-c334.obj'));
if (sha(source) !== 'da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe')
  throw Error('Pinned Camera.js hash changed');
if (sha(fixture) !== 'a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b')
  throw Error('Authored C334 OBJ hash changed');
const text = source.toString('utf8');
for (const token of ['setPivot(pivot)', 'this._center, pivot', 'this._offset[2] += this.getTransZ() - oldZoom', 'this._offset[2] = 0.0'])
  if (!text.includes(token)) throw Error(`Pinned Source pivot route changed: ${token}`);

const add = (a, b) => a.map((x, i) => x + b[i]);
const sub = (a, b) => a.map((x, i) => x - b[i]);
const mul = (a, scale) => a.map(x => x * scale);
const norm = a => Math.hypot(...a);
const rotateY = (v, radians) => {
  const c = Math.cos(radians), s = Math.sin(radians);
  return [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]];
};
const cameraCoord = (world, center, localOffset, zoom, fov, angle, orthographic) => {
  const local = rotateY(sub(world, center), -angle);
  const z = orthographic ? 1000 : zoom * 45 / fov;
  return add(local, sub(localOffset, [0, 0, z]));
};
const checks = [];
for (const orthographic of [false, true]) {
  for (const angle of [0, Math.PI / 5]) {
    let center = [0.1, -0.3, 0.2], offset = [0, 0, 0], zoom = 4.5;
    const fov = 45;
    const points = [[-1.2, -0.8, 0], [1, 0.9, 0.15], [0.4, -0.2, 0.052941176470588235]];
    for (const pivot of [points[2], [0.2, 0.4, 0.2]]) {
      const before = points.map(point => cameraCoord(point, center, offset, zoom, fov, angle, orthographic));
      const oldDepth = zoom * 45 / fov;
      const oldLocalEye = [-offset[0], -offset[1], (orthographic ? 1000 : oldDepth) - offset[2]];
      const eye = add(center, rotateY(oldLocalEye, angle));
      offset = add(offset, rotateY(sub(pivot, center), -angle));
      if (orthographic) offset[2] = 0;
      else {
        const newDepth = norm(sub(eye, pivot));
        zoom = newDepth * fov / 45;
        offset[2] += newDepth - oldDepth;
      }
      center = pivot;
      const after = points.map(point => cameraCoord(point, center, offset, zoom, fov, angle, orthographic));
      const dimensions = orthographic ? 2 : 3;
      const maximum = Math.max(...before.flatMap((row, i) => row.slice(0, dimensions).map((x, k) => Math.abs(x - after[i][k]))));
      if (maximum > 1e-12) throw Error(`Static pivot identity changed: ${orthographic}/${angle}/${maximum}`);
      checks.push({ orthographic, angle, pivot, maximum });
    }
  }
}
process.stdout.write(`${JSON.stringify({ protocol: 'c334-static-preflight', source_sha256: sha(source), fixture_sha256: sha(fixture), checks })}\n`);
