// Normal-browser stationary hold/release comparison against the pinned SculptGL.
// Usage: node scripts/check-sculptgl-source-continuous-pair.mjs --side=source|root
// Requires the local ignored .artifacts fixture/driver checkout and web/dist.
// It tests only one Draw contact; it does not claim brush-response parity.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp, PageDriver } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';
import { PerfDriver } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const sideArg = process.argv.find(arg => arg.startsWith('--side='))?.slice(7);
if (!['source', 'root'].includes(sideArg)) throw Error('Use --side=source or --side=root');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const requireTrue = (condition, reason) => { if (!condition) throw Error(reason); };
const fixture = path.join(root, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/fixtures/independent-id-sphere-10k.sgl');
const fixtureBytes = await readFile(fixture);
const mirror = path.join(root, '.tools/sculptgl-reference/host-mirror');
const web = path.join(root, 'web/dist');
const pinned = sha(await readFile(path.join(mirror, 'sculptgl.js')));
requireTrue(pinned === '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600', 'Pinned SculptGL bundle changed');
const out = path.join(root, '.artifacts/sculptgl-source-continuous-pair', randomUUID());
await mkdir(out, { recursive: true });
const result = {
  status: 'running', side: sideArg, fixture: { path: fixture, bytes: fixtureBytes.length, sha256: sha(fixtureBytes), vertices: 10114 },
  source_bundle_sha256: pinned, obrush_wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))),
  parameters: { brush: 'Draw', radius_pixels: 64, intensity: 0.25, pressure_factors: [0, 0],
    symmetry: false, orthographic: true, pointer_offset_x: 80, requested_stationary_hold_ms: 400,
    release_quiet_ms: [0, 200, 800] },
  cases: [], requests: [], cdp_trace: [], ui_actions: [], input_trace: [], download_events: [], downloads: [],
  screenshots: [], dialogs: [], exceptions: [], console_errors: [], blocked_external_requests: [],
  viewport_measurements: [], notice_acknowledgments: [], acknowledgments: [], failed_acknowledgments: [],
  page_observations: [], probes: [], cleanup_errors: []
};
const owner = new OwnedCdp(out, result);
let pressed = null;
const rootPositions = bytes => JSON.parse(bytes.toString()).mesh.positions.flat();
const sourcePositions = state => state.meshes[0].positions;
function delta(a, b) {
  requireTrue(a.length === b.length, 'Changed vertex domain');
  let changed = 0, max = 0;
  for (let i = 0; i < a.length; i++) {
    const difference = Math.abs(b[i] - a[i]);
    if (difference > 0) changed++;
    max = Math.max(max, difference);
  }
  return { changed_components: changed, max_component: max };
}
async function runCase(side, enabled, url) {
  const stage = path.join(root, '.artifacts/sculptgl-large-sustained-c58-stage');
  const observer = side === 'source'
    ? (await readFile(path.join(stage, 'observer/focused-alpha-input-observer.js'), 'utf8')) + '\n' +
      (await readFile(path.join(stage, 'observer/focused-alpha-hit-observer.js'), 'utf8'))
    : '';
  const page = await owner.page(side, url, observer);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  await page.send('Page.bringToFront');
  const setup = await page.prepareFixture({ path: fixture, vertices: result.fixture.vertices });
  if (side === 'root') await page.rootCheckbox('source_geometry.continuous', s => s.continuous.requested, enabled);
  else {
    await page.sourceMenu('_ctrlSculpting');
    const expression = page.checkboxExpression('_ctrlSculpting', 'Continuous');
    const inspect = () => page.evaluate(`(()=>{const e=(${expression}),r=e.parentElement.getBoundingClientRect(),x=(r.left+r.right)/2,y=(r.top+r.bottom)/2,h=document.elementFromPoint(x,y);return {checked:e.checked,rect:[r.left,r.top,r.right,r.bottom],hit_tag:h?.tagName,hit_class:h?.className,hit_owns:h===e.parentElement||e.parentElement.contains(h)}})()`);
    let before = await inspect();
    for (let i = 0; i < 10 && !before.hit_owns; i++) {
      await page.pointer('mouseMoved', 1180, 500);
      await page.send('Input.dispatchMouseEvent', {type:'mouseWheel', x:1180, y:500, deltaX:0, deltaY:100});
      await page.raf();
      before = await inspect();
    }
    result.probes.push({side,enabled,stage:'continuous-control-before',before});
    if (before.checked !== enabled) {
      requireTrue(before.hit_owns, 'Visible Source Continuous checkbox center is occluded');
      await page.sourceClick('(' + expression + ').parentElement');
    }
    await page.wait(() => page.evaluate(`(${expression}).checked`), value => value === enabled, 'normal Source checkbox Continuous input');
  }
  await page.closeMenus();
  const published = await page.live();
  requireTrue((side === 'root' ? published.state.continuous.requested : published.state.continuous) === enabled, 'Visible Continuous control failed to publish');
  const before = await page.save(`${side}-${enabled}-before`);
  await page.closeMenus();
  const viewport = await page.viewport();
  const contact = [(viewport.screen[0] + viewport.screen[2]) / 2 + 80, (viewport.screen[1] + viewport.screen[3]) / 2];
  const preState = side === 'source' ? await PageDriver.prototype.state.call(page, true) : await page.state();
  const positionsBefore = side === 'source' ? sourcePositions(preState) : rootPositions(before.bytes);
  result.probes.push({ side, enabled, stage: 'before-press', live: await page.live(), source_focus_gui: side === 'source' ? await page.evaluate('window.__sculptglReferenceInstance._focusGui') : null });
  await page.pointer('mouseMoved', ...contact);
  await page.pointer('mousePressed', ...contact, 'left', 1);
  pressed = { page, contact };
  await pause(400);
  const held = await page.live();
  const heldState = side === 'source' ? await PageDriver.prototype.state.call(page, true) : await page.state();
  const positionsHeld = side === 'source' ? sourcePositions(heldState) : null;
  result.probes.push({ side, enabled, stage: 'held', live: held, delta: positionsHeld ? delta(positionsBefore, positionsHeld) : null });
  await page.pointer('mouseReleased', ...contact, 'left', 0);
  pressed = null;
  for (let i = 0; i < 15; i++) await page.raf();
  const release = await page.live();
  const after = await page.save(`${side}-${enabled}-after`);
  const positionsAfter = side === 'source' ? sourcePositions(await PageDriver.prototype.state.call(page, true)) : rootPositions(after.bytes);
  const quiet = [];
  for (const delay of [0, 200, 800]) {
    await pause(delay);
    const saved = await page.save(`${side}-${enabled}-quiet-${delay}`);
    quiet.push({ delay_ms: delay, sha256: saved.record.sha256, metrics: await page.live() });
  }
  await page.chord('z');
  const undo = await page.save(`${side}-${enabled}-undo`);
  await page.chord('z', true);
  const redo = await page.save(`${side}-${enabled}-redo`);
  const row = {
    side, enabled, setup: { viewport: setup.viewport, fixture_sha256: result.fixture.sha256 },
    published, contact, hold: held, release, before: before.record, after: after.record,
    quiet, undo: undo.record, redo: redo.record,
    positions: { press_to_release: delta(positionsBefore, positionsAfter),
      held_to_release: positionsHeld ? delta(positionsHeld, positionsAfter) : null },
    gates: {
      changed: !enabled
        ? delta(positionsBefore, positionsAfter).changed_components === 0
        : delta(positionsBefore, positionsAfter).changed_components > 0,
      quiet: quiet.every(item => item.sha256 === after.record.sha256),
      one_stroke_undo: !enabled
        ? undo.record.sha256 !== before.record.sha256
        : undo.record.sha256 === before.record.sha256,
      redo_exact: redo.record.sha256 === after.record.sha256,
      timer_stopped: side === 'source' ? release.state.timer === -1 : release.state.continuous.active === false,
      timer_behavior: side === 'source' ? (enabled ? held.state.timer !== -1 : held.state.timer === -1)
        : (enabled ? held.state.continuous.consumed_tick_permissions > 0 : held.state.continuous.consumed_tick_permissions === 0)
    }
  };
  result.cases.push(row);
  requireTrue(Object.values(row.gates).every(Boolean), `Failed ${side} Continuous=${enabled}: ${JSON.stringify(row.gates)}`);
  return row;
}
try {
  const url = await owner.serve(sideArg === 'source' ? mirror : web, sideArg);
  await owner.start();
  for (const enabled of [false, true]) await runCase(sideArg, enabled, url);
  const [off, on] = result.cases;
  requireTrue(off.after.sha256 !== on.after.sha256, `${sideArg}: stationary Continuous did not differ from ordinary contact`);
  requireTrue(!result.exceptions.length && !result.console_errors.length, 'Browser runtime errors');
  result.status = 'passed scoped stationary hold/release/undo comparison; response parity unverified';
} catch (error) {
  result.status = 'FAILED scoped comparison; raw observations retained';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  if (pressed) try { await pressed.page.pointer('mouseReleased', ...pressed.contact, 'left', 0); } catch (error) { result.cleanup_errors.push(String(error)); }
  try { await owner.close(); } catch (error) { result.cleanup_errors.push(String(error)); process.exitCode = 1; }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ out, status: result.status, failure: result.failure ?? null, cases: result.cases.length, cleanup_errors: result.cleanup_errors }));
}
