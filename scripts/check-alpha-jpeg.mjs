// Trusted normal-browser JPEG alpha import and brush/history check.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const sourceMode = process.argv.includes('--source');
const hiddenPng = process.argv.includes('--hidden-rgb-png');
const quantizedPng = process.argv.includes('--quantized-png');
if (hiddenPng && quantizedPng) throw Error('Select only one PNG fixture');
const out = path.join(root, '.artifacts/alpha-jpeg', randomUUID());
await mkdir(out, { recursive: true });
const sha = data => createHash('sha256').update(data).digest('hex');
const fixture = path.join(root, 'crates/obrush-app/tests/fixtures',
  quantizedPng ? 'independent-alpha-quantization16x16.png' :
  hiddenPng ? 'independent-hidden-rgb2x2.png' : 'independent-gray-bands16x16.jpg');
const expectedSide = hiddenPng ? 2 : 16;
const result = { status: 'running', fixture_sha256: sha(await readFile(fixture)),
  requests: [], cdp_trace: [], download_events: [], downloads: [], dialogs: [],
  exceptions: [], console_errors: [], blocked_external_requests: [], cleanup_errors: [] };
const owner = new OwnedCdp(out, result);
let sessionId;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
try {
  const url = await owner.serve(path.join(root, 'web/dist'), 'alpha-jpeg');
  await owner.start();
  const target = await owner.send('Target.createTarget', { url: 'about:blank' });
  sessionId = (await owner.send('Target.attachToTarget', { targetId: target.targetId, flatten: true })).sessionId;
  owner.sessions.set(sessionId, 'alpha-jpeg');
  await owner.send('Runtime.enable', {}, sessionId);
  await owner.send('Page.enable', {}, sessionId);
  await owner.send('DOM.enable', {}, sessionId);
  await owner.send('Page.navigate', { url }, sessionId);
  const evalJs = async expression => {
    const response = await owner.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
    if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
    return response.result.value;
  };
  const metrics = () => evalJs('window.obrushBrushMetrics()');
  for (let i = 0; i < 250; i++) {
    if (await evalJs('!!window.obrushBrushMetrics && !document.getElementById("loading")')) break;
    if (i === 249) throw Error('Studio startup deadline');
    await pause(50);
  }
  const pointer = async (type, x, y, down = false) => owner.send('Input.dispatchMouseEvent', {
    type, x, y, button: type === 'mouseMoved' ? 'none' : 'left',
    buttons: down ? 1 : 0, clickCount: type === 'mouseMoved' ? undefined : 1,
  }, sessionId);
  const click = async (x, y) => {
    await pointer('mouseMoved', x, y);
    await pointer('mousePressed', x, y, true);
    await pause(40);
    await pointer('mouseReleased', x, y);
    await pause(170);
  };
  const control = async key => {
    result.control_attempts ??= [];
    const viewportHeight = await evalJs('window.innerHeight');
    for (let i = 0; i < 30; i++) {
      const s = await metrics();
      const rect = s.morph_controls?.[key];
      if (!rect) throw Error(`Missing rendered ${key}`);
      const x = Math.round((rect[0] + rect[2]) / 2);
      const y = Math.round((rect[1] + rect[3]) / 2);
      result.control_attempts.push({ key, rect, enabled: s.morph_controls_enabled?.[key], message: s.message });
      if (y > 80 && y < viewportHeight - 35) return [x, y];
      await pointer('mouseMoved', x, 700);
      await pause(40);
      await owner.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y: 700,
        deltaX: 0, deltaY: y >= viewportHeight - 35 ? 380 : -380 }, sessionId);
      await pause(180);
    }
    throw Error(`Could not expose ${key}`);
  };
  if (sourceMode) {
    await click(...await control('source_geometry.mask.preview'));
    const conversion = await metrics();
    if (conversion.morph_controls_enabled?.['source_geometry.mask.convert'] === true) {
      await click(...await control('source_geometry.mask.convert'));
    }
    const sourcePoint = await control('source_geometry.mode.source');
    if ((await metrics()).morph_controls_enabled?.['source_geometry.mode.source'] !== true) {
      const shot = await owner.send('Page.captureScreenshot', { format: 'png' }, sessionId);
      await writeFile(path.join(out, 'source-preflight.png'), Buffer.from(shot.data, 'base64'));
      throw Error('Source Brush control is disabled after mask preflight');
    }
    await click(...sourcePoint);
    result.source_mode = (await metrics()).source_geometry;
    if (result.source_mode.per_tool[0]?.mode !== 'SourceOrdinary') {
      throw Error('Source Brush mode did not activate');
    }
  }
  if ((await metrics()).preferences.alpha_conversion !== 'mean_rgb') {
    const [mx, my] = await control('alpha.conversion');
    await click(mx, my);
    const [ox, oy] = await control('alpha.convert.mean_rgb');
    await click(ox, oy);
  }
  const before = await metrics();
  const [cx, cy] = await control('alpha.import');
  await click(cx, cy);
  let armed;
  for (let i = 0; i < 100; i++) {
    armed = await metrics();
    if (Number.isInteger(armed.preferences?.alpha_import_request?.id)) break;
    if (i === 99) throw Error('Alpha import request did not arm');
    await pause(50);
  }
  const doc = (await owner.send('DOM.getDocument', {}, sessionId)).root;
  const input = (await owner.send('DOM.querySelector', { nodeId: doc.nodeId, selector: '#alpha-file' }, sessionId)).nodeId;
  if (!input) throw Error('Alpha file input missing');
  await owner.send('DOM.setFileInputFiles', { nodeId: input, files: [fixture] }, sessionId);
  let imported;
  for (let i = 0; i < 120; i++) {
    imported = await metrics();
    if (!imported.preferences?.alpha_import_request && imported.alpha_width === expectedSide && imported.alpha_height === expectedSide) break;
    if (i === 119) throw Error('JPEG import deadline');
    await pause(50);
  }
  result.import = { before_entries: before.preferences.alpha_entries.length,
    armed: armed.preferences.alpha_import_request,
    after_entries: imported.preferences.alpha_entries,
    alpha: imported.alpha, width: imported.alpha_width, height: imported.alpha_height,
    tool: imported.brush, revision: imported.document_revision };
  if (result.import.after_entries.length !== result.import.before_entries + 1 ||
      result.import.after_entries.at(-1)?.conversion !== 'mean_rgb') {
    throw Error('JPEG did not enter the expected Mean RGB alpha library');
  }
  if (sourceMode && imported.source_geometry.per_tool[0]?.mode !== 'SourceOrdinary') {
    throw Error('Source Brush mode changed during JPEG import');
  }
  const viewport = imported.camera.viewport;
  const screenshot = async name => {
    await pointer('mouseMoved', 180, 60);
    await pause(220);
    const clip = { x: viewport[0], y: viewport[1], width: viewport[2] - viewport[0],
      height: viewport[3] - viewport[1], scale: 1 };
    const bytes = Buffer.from((await owner.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip }, sessionId)).data, 'base64');
    await writeFile(path.join(out, `${name}.png`), bytes);
    return sha(bytes);
  };
  result.before_png_sha256 = await screenshot('before');
  const x = Math.round((viewport[0] + viewport[2]) / 2);
  const y = Math.round((viewport[1] + viewport[3]) / 2);
  if (sourceMode) {
    await pointer('mouseMoved', x, y);
    await pointer('mousePressed', x, y, true);
    await pause(80);
    await pointer('mouseMoved', x + 25, y, true);
    await pause(220);
    await pointer('mouseReleased', x + 25, y);
    await pause(170);
  } else {
    await click(x, y);
  }
  await pause(650);
  const sculpted = await metrics();
  result.after_png_sha256 = await screenshot('after');
  result.revisions = [imported.document_revision, sculpted.document_revision];
  result.stroke = { message: sculpted.message, source_geometry: sculpted.source_geometry?.last_completed,
    source_mode: sculpted.source_geometry?.per_tool?.[0]?.mode, brush: sculpted.brush,
    alpha: sculpted.alpha };
  if (sourceMode && (hiddenPng || quantizedPng)) {
    const captured = result.stroke.source_geometry?.capture?.captured_controls?.alpha;
    const expected = quantizedPng ? await readFile(path.join(root,
      'crates/obrush-app/tests/fixtures/pinned-sculptgl-alpha-quantization16x16.bin')) :
      [0, 85, 85, 255];
    let fnv = 0xcbf29ce484222325n;
    for (const byte of expected) fnv = ((fnv ^ BigInt(byte)) * 0x100000001b3n) & 0xffffffffffffffffn;
    result.hidden_png_expected_fnv1a64_hex = fnv.toString(16).padStart(16, '0');
    if (captured?.byte_checksum_fnv1a64_hex !== result.hidden_png_expected_fnv1a64_hex) {
      throw Error('Source captured PNG alpha does not match pinned reference values');
    }
  }
  if (sculpted.document_revision <= imported.document_revision ||
      result.after_png_sha256 === result.before_png_sha256) throw Error('JPEG alpha did not produce a visible undoable sculpt');
  const chord = async shift => {
    const modifiers = 4 | (shift ? 8 : 0);
    await owner.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers }, sessionId);
    await owner.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'z', code: 'KeyZ', windowsVirtualKeyCode: 90, modifiers }, sessionId);
    await owner.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'z', code: 'KeyZ', windowsVirtualKeyCode: 90, modifiers }, sessionId);
    await owner.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 }, sessionId);
    await pause(300);
  };
  await evalJs('document.getElementById("obrush_canvas").focus()');
  await chord(false);
  result.undo_png_sha256 = await screenshot('undo');
  await chord(true);
  result.redo_png_sha256 = await screenshot('redo');
  if (result.undo_png_sha256 !== result.before_png_sha256 ||
      result.redo_png_sha256 !== result.after_png_sha256) throw Error('JPEG sculpt visible history mismatch');
  const chooseAlpha = async option => {
    let point = await control('alpha.library');
    await click(...point);
    point = await control(`alpha.option.${option}`);
    await click(...point);
    return metrics();
  };
  if (!sourceMode) {
  const square = await chooseAlpha('square');
  if (square.preferences.tools.find(tool => tool.kind === 'Draw')?.alpha !== 'Square' ||
      square.alpha_width !== 64 || square.alpha_height !== 64) throw Error('Square preset did not bind Draw');
  await click(...await control('brush.clay'));
  const clay = await metrics();
  if (clay.kind !== 'Clay' || clay.preferences.tools.find(tool => tool.kind === 'Clay')?.alpha !== 'None') {
    throw Error('Alpha selection leaked from Draw into Clay');
  }
  await click(...await control('brush.draw'));
  const drawAgain = await metrics();
  if (drawAgain.alpha_width !== 64 || drawAgain.preferences.tools.find(tool => tool.kind === 'Draw')?.alpha !== 'Square') {
    throw Error('Draw Square preset was not retained per tool');
  }
  const skin = await chooseAlpha('skin');
  const none = await chooseAlpha('none');
  result.presets = { square: square.alpha, clay: clay.alpha, draw_again: drawAgain.alpha,
    skin: skin.alpha, none: none.alpha, tool_rows: none.preferences.tools.filter(tool => ['Draw', 'Clay'].includes(tool.kind)) };
  if (skin.alpha_width !== 64 || skin.alpha_height !== 64 || none.alpha_width != null) {
    throw Error('Skin/None presets did not resolve expected bounded rasters');
  }
  }
  result.status = `PASS normal-browser ${quantizedPng ? 'quantized PNG' : hiddenPng ? 'hidden-RGB PNG' : 'JPEG'} alpha import and visible ${sourceMode ? 'Source Brush' : 'Draw'} sculpt`;
} catch (error) {
  result.status = 'FAIL preserved';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  await owner.close();
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), failure: result.failure ?? null }));
}
