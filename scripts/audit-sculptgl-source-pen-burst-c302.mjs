// C302 public-file audit. No product state is queried or changed here.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [sourcePath, rootPath] = process.argv.slice(2);
if (!sourcePath || !rootPath) throw Error('Pass pinned SculptGL and OBrush C302 result.json paths');
const references = path.resolve(process.env.OBRUSH_REFERENCE_ROOT || '.');
const { decodeIndependent } = await import(pathToFileURL(path.join(references, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')));
const { decodeRootF32Model } = await import(pathToFileURL(path.join(references, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const source = JSON.parse(await readFile(sourcePath));
const root = JSON.parse(await readFile(rootPath));
if (source.side !== 'source' || root.side !== 'root' || source.fixture.sha256 !== root.fixture.sha256)
  throw Error('Paired fixture or side mismatch');
const f32 = word => { const b = Buffer.allocUnsafe(4); b.writeUInt32LE(word); return b.readFloatLE(); };
const decode = (side, bytes) => side === 'root'
  ? decodeRootF32Model(bytes)
  : (() => { const m = decodeIndependent(bytes).meshes[0]; return { positions:m.positions.map(row=>row.map(f32)), colors:m.colors.map(row=>row.map(f32)), faces:m.faces }; })();
const byId = mesh => {
  const rows = new Map();
  mesh.colors.forEach((color, index) => {
    const id = Math.round(color[0]*255) | (Math.round(color[1]*255)<<8) | (Math.round(color[2]*255)<<16);
    if (rows.has(id)) throw Error(`Duplicate authored vertex ID ${id}`);
    rows.set(id, mesh.positions[index]);
  });
  return rows;
};
const savedPositionBytes = mesh => {
  const bytes = Buffer.alloc(mesh.positions.length * 12);
  mesh.positions.forEach((xyz,index) => xyz.forEach((value,axis) => bytes.writeFloatLE(value,index*12+axis*4)));
  return bytes;
};
const positionDiff = (a,b) => {
  if (a.size !== b.size) throw Error('Original vertex count changed');
  let changed = 0, components = 0, maxAbs = 0;
  for (const [id, xyz] of a) {
    const other = b.get(id); if (!other) throw Error(`Missing authored vertex ID ${id}`);
    let rowChanged = false;
    for (let axis=0; axis<3; axis++) {
      const delta = Math.abs(xyz[axis] - other[axis]);
      maxAbs = Math.max(maxAbs, delta);
      if (Math.fround(xyz[axis]) !== Math.fround(other[axis])) { components++; rowChanged = true; }
    }
    if (rowChanged) changed++;
  }
  return { changedOriginalVertices:changed, changedComponents:components, maxAbsoluteComponent:maxAbs };
};
const records = {};
for (const [side, run] of [['source',source],['root',root]]) {
  records[side] = {};
  for (const row of run.cases) {
    const files = {};
    for (const stage of ['before','after','undo','redo']) {
      const bytes = await readFile(row[stage].path);
      if (sha(bytes) !== row[stage].sha256) throw Error(`${side}/${row.name}/${stage} hash mismatch`);
      files[stage] = bytes;
    }
    const before = decode(side, files.before), after = decode(side, files.after);
    const gpu = {};
    for (const stage of ['before','after','undo','redo']) {
      const observed = row[`${stage}_gpu`];
      const expected = savedPositionBytes(decode(side,files[stage]));
      const raw = Buffer.from(observed?.position_words_base64 ?? '', 'base64');
      gpu[stage] = {
        active:observed?.active_vbo_candidates === 1 && observed?.gl_error === 0,
        positionExact:raw.length === expected.length && raw.equals(expected) && observed?.position_words_sha256 === sha(expected),
        readbackSha256:observed?.position_words_sha256 ?? null,
      };
    }
    const delivered = row.burst_dom_events.filter(e => e.type === 'pointermove' && e.pointerType === 'pen' && e.trusted && e.buttons === 1);
    records[side][row.name] = {
      saves:Object.fromEntries(['before','after','undo','redo'].map(stage => [stage,row[stage].sha256])),
      exactUndo:files.before.equals(files.undo), exactRedo:files.after.equals(files.redo),
      beforeToAfter:positionDiff(byId(before),byId(after)),
      sameFaceCount:before.faces.length === after.faces.length,
      gpu,
      burstDelivered:delivered.length === 2 && delivered[0].frame === delivered[1].frame &&
        delivered.every((event,index) => Math.abs(event.pressure-row.burstPressures[index]) <= 1e-6),
      usefulPanmoves:side === 'source' ? row.hammer_events.filter(e => e.type === 'panmove' && e.pointerType === 'pen').length : null,
      rootAppliedDabs:side === 'root' ? [row.state_after_first_useful_move?.applied_dabs,row.state_after_burst?.applied_dabs,row.state_after_release?.applied_dabs] : null,
      _before:byId(before), _after:byId(after),
    };
  }
}
const comparisons = {};
for (const side of ['source','root']) {
  const rows = records[side];
  comparisons[side] = {
    beforePair:positionDiff(rows['low-high']._before,rows['near-high']._before),
    pressureTraceEffect:positionDiff(rows['low-high']._after,rows['near-high']._after),
    factorZeroTraceEffect:positionDiff(rows['factor-zero-low-high']._after,rows['factor-zero-near-high']._after),
  };
  for (const row of Object.values(rows)) { delete row._before; delete row._after; }
}
const overflow = {};
if (root.overflow) {
  const before = await readFile(root.overflow.before.path), after = await readFile(root.overflow.after.path);
  overflow.beforeHashExact = sha(before) === root.overflow.before.sha256;
  overflow.afterHashExact = sha(after) === root.overflow.after.sha256;
  overflow.deliveredHeldPenMoves = root.overflow.dom_events.filter(event =>
    event.type === 'pointermove' && event.pointerType === 'pen' && event.trusted && event.buttons === 1).length;
  overflow.reportedDrops = root.overflow.before_release_metrics?.browser_pen_moves_dropped ?? null;
  overflow.cancelledWholeStroke = before.equals(after);
  overflow.historyExact = JSON.stringify(root.overflow.before_history) === JSON.stringify(root.overflow.after_history);
}
const checks = {
  allTrustedSameFrameBurst:Object.values(records).every(side => Object.values(side).every(row => row.burstDelivered)),
  allOwnHistoryExact:Object.values(records).every(side => Object.values(side).every(row => row.exactUndo && row.exactRedo)),
  unchangedFaceCounts:Object.values(records).every(side => Object.values(side).every(row => row.sameFaceCount)),
  activeGpuPositionsExact:Object.values(records).every(side => Object.values(side).every(row =>
    Object.values(row.gpu).every(stage => stage.active && stage.positionExact))),
  sameInitialGeometry:Object.values(comparisons).every(c => c.beforePair.changedOriginalVertices === 0),
  sourcePressureTraceChangesGeometry:comparisons.source.pressureTraceEffect.changedOriginalVertices > 0,
  rootPressureTraceChangesGeometry:comparisons.root.pressureTraceEffect.changedOriginalVertices > 0,
  sourceFactorZeroIgnoresPressure:comparisons.source.factorZeroTraceEffect.changedOriginalVertices === 0,
  rootFactorZeroIgnoresPressure:comparisons.root.factorZeroTraceEffect.changedOriginalVertices === 0,
  sourceAdmittedAtLeastThreeUsefulPanmoves:Object.values(records.source).every(row => row.usefulPanmoves >= 3),
  rootBurstAddedExactlyTwoDabsWithoutReleaseDuplicate:Object.values(records.root).every(row =>
    row.rootAppliedDabs?.[1] === row.rootAppliedDabs?.[0] + 2 && row.rootAppliedDabs?.[2] === row.rootAppliedDabs?.[1]),
  overflowBoundedAndCancelled:overflow.beforeHashExact && overflow.afterHashExact &&
    overflow.deliveredHeldPenMoves >= 257 && overflow.reportedDrops >= 1 &&
    overflow.cancelledWholeStroke && overflow.historyExact,
};
const result = {
  status:Object.values(checks).every(Boolean) ? 'passed scoped C302 paired pressure-burst gates' : 'FAILED scoped C302 paired pressure-burst gates',
  sourcePath,rootPath, fixtureSha256:source.fixture.sha256, sourceBuild:source.browser_build, rootBuild:root.browser_build,
  auditHarnessSha256:sha(await readFile(new URL(import.meta.url))), records,comparisons,overflow,checks,
  limitations:['Synthetic trusted Chrome pen events do not establish physical tablet behavior or native pressure delivery.',
    'This tests the active ordinary Source Brush queue only. Exact matched geometry, Source Paint, Lock, Continuous, Drag, transformed canvas and long-session responsiveness remain open.'],
};
const out = path.join(path.dirname(sourcePath),`audit-${path.basename(path.dirname(rootPath))}.json`);
await writeFile(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({out,status:result.status,checks,comparisons}));
if (result.status.startsWith('FAILED')) process.exitCode = 1;
