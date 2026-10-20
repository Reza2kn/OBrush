// C338: normal OBrush File controls -> downloaded Sketchfab-axis PLY payload.
// No account, upload, OAuth, or network publishing is exercised.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const fixture = path.resolve('crates/obrush-core/tests/fixtures/independent-sketchfab-axis.obj');
const out = path.resolve('.artifacts/sculptgl-sketchfab-ply-c338/' + randomUUID());
const profile = path.join(out, 'chrome-profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads, { recursive: true });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const requireTrue = (condition, message) => { if (!condition) throw Error(message); };
const report = { scope: 'C338 normal File Sketchfab-ready PLY download', fixture: { path: fixture }, checks: [], errors: [] };
let server, chrome, socket, session, next = 0;
const waiting = new Map();
const seen = [];
const choosers = [];
async function send(method, params = {}, selectedSession = session) {
  const id = ++next;
  const reply = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error('CDP timeout: ' + method)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selectedSession ? { sessionId: selectedSession } : {}) }));
  return reply;
}
async function evaluate(expression) {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
  return reply.result.value;
}
async function state() { return evaluate('window.obrushBrushMetrics?.() ?? null'); }
async function until(predicate, description, attempts = 150) {
  for (let i = 0; i < attempts; i++) {
    const value = await predicate();
    if (value) return value;
    await pause(100);
  }
  throw Error('Timed out: ' + description);
}
async function clickControl(key) {
  for (let attempt = 0; attempt < 40; attempt++) {
    const metrics = await state();
    const rect = metrics?.layer_controls?.[key] || metrics?.scene_controls?.[key];
    requireTrue(rect, 'Missing rendered control ' + key);
    const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
    if (y >= 80 && y <= 945) {
      requireTrue(metrics.layer_controls_enabled?.[key] !== false && metrics.scene_controls_enabled?.[key] !== false, 'Disabled control ' + key);
      await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
      await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
      await pause(100);
      return;
    }
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 1550, y: 700, deltaX: 0, deltaY: y > 945 ? 350 : -350 });
    await pause(120);
  }
  throw Error('Control stayed outside viewport: ' + key);
}
async function scrollInspectorToBottom() {
  const x = 1582;
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y: 150, button: 'none', buttons: 0 });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y: 150, button: 'left', buttons: 1, clickCount: 1 });
  for (const y of [300, 500, 700, 875]) {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 1 });
    await pause(60);
  }
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y: 875, button: 'left', buttons: 0, clickCount: 1 });
  await pause(150);
}
async function key(letter, modifiers = 2) {
  const upper = letter.toUpperCase();
  const params = { key: letter.toLowerCase(), code: 'Key' + upper, windowsVirtualKeyCode: upper.charCodeAt(0), modifiers };
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...params });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', ...params });
}
async function addFixture(copy, expectedCount) {
  const file = path.join(out, `axis-${copy}.obj`);
  const bytes = await readFile(fixture);
  await writeFile(file, bytes);
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const before = choosers.length;
  await key('o');
  await until(() => choosers.length > before, 'normal Source Add chooser');
  const dom = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: dom.nodeId, selector: '#source-sgl-file' })).nodeId;
  requireTrue(node, 'Source Add input missing');
  await send('DOM.setFileInputFiles', { nodeId: node, files: [file] });
  const after = await until(async () => {
    const current = await state();
    return !current?.source_sgl_import?.pending && current?.subtools?.length === expectedCount ? current : null;
  }, 'Source Add installation');
  report.checks.push({ add_fixture_copy: copy, request_complete: true, parts: after.subtools.length, revision: after.document_revision });
}
async function downloadAfter(action, description) {
  const before = seen.length;
  await action();
  const event = await until(() => seen.slice(before).find(item => item.state === 'completed'), description);
  return { event, bytes: await readFile(path.join(downloads, event.guid)) };
}
async function save(label) {
  const { event, bytes } = await downloadAfter(async () => {
    // Visible top-bar Save project button at this frozen viewport size.
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: 345, y: 26, button: 'left', clickCount: 1 });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 345, y: 26, button: 'left', clickCount: 1 });
  }, `${label} project Save`);
  requireTrue(event.suggestedFilename.endsWith('.obrush'), 'Save project extension changed');
  const project = JSON.parse(bytes.toString('utf8'));
  const hash = sha(bytes);
  report.checks.push({ save: label, bytes: bytes.length, sha256: hash, parts: project.scene?.objects?.length ?? 0 });
  return { project, hash };
}
function decodePly(bytes) {
  const marker = Buffer.from('end_header\n');
  const end = bytes.indexOf(marker);
  requireTrue(end >= 0, 'PLY header terminator missing');
  const atBody = end + marker.length;
  const header = bytes.toString('ascii', 0, atBody);
  requireTrue(header.includes('format binary_little_endian 1.0'), 'PLY is not little-endian binary');
  requireTrue(header.includes('property uchar red\nproperty uchar green\nproperty uchar blue\n'), 'PLY RGB schema missing');
  const vertexCount = Number(header.match(/element vertex (\d+)/)?.[1]);
  const faceCount = Number(header.match(/element face (\d+)/)?.[1]);
  requireTrue(vertexCount === 6 && faceCount === 2, 'Sketchfab PLY did not include both imported parts');
  let at = atBody;
  const vertices = [];
  for (let i = 0; i < vertexCount; i++) {
    vertices.push({
      xyz: [bytes.readFloatLE(at), bytes.readFloatLE(at + 4), bytes.readFloatLE(at + 8)],
      rgb: [...bytes.subarray(at + 12, at + 15)],
    });
    at += 15;
  }
  const faces = [];
  for (let i = 0; i < faceCount; i++) {
    const count = bytes[at++];
    requireTrue(count === 3, 'Independent fixture triangle changed');
    faces.push([bytes.readUInt32LE(at), bytes.readUInt32LE(at + 4), bytes.readUInt32LE(at + 8)]);
    at += 12;
  }
  requireTrue(bytes.length - at === 26 && bytes.subarray(at).every(value => value === 0), 'Pinned PLY face padding differs');
  return { header, vertices, faces, paddingBytes: bytes.length - at };
}
try {
  const fixtureBytes = await readFile(fixture);
  report.fixture.bytes = fixtureBytes.length;
  report.fixture.sha256 = sha(fixtureBytes);
  requireTrue(report.fixture.sha256 === '40c2e631c25f2014f4ab7f2aee74a0c836e91432af682682ec86c0eae7d8097b', 'Independent fixture hash drifted');
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html');
      const file = path.resolve(web, '.' + pathname);
      requireTrue(file.startsWith(web + path.sep), 'Outside web root');
      const bytes = await readFile(file);
      response.setHeader('Cache-Control', 'no-store');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const url = `http://127.0.0.1:${server.address().port}/?obrushprofile=sculptgl`;
  report.url = url;
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', chunk => { stderr += chunk; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome CDP endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', message => {
    const item = JSON.parse(String(message.data));
    if (item.id) {
      const pending = waiting.get(item.id);
      if (!pending) return;
      waiting.delete(item.id); clearTimeout(pending.timer);
      item.error ? pending.reject(Error(JSON.stringify(item.error))) : pending.resolve(item.result);
    } else if (item.method === 'Browser.downloadWillBegin') seen.push({ ...item.params, state: 'begin' });
    else if (item.method === 'Browser.downloadProgress') {
      const target = seen.find(entry => entry.guid === item.params.guid);
      if (target) Object.assign(target, item.params);
    } else if (item.method === 'Runtime.exceptionThrown') report.errors.push(item.params.exceptionDetails.text);
    else if (item.method === 'Runtime.consoleAPICalled' && item.params.type === 'error') report.errors.push(item.params.args.map(arg => arg.value || arg.description).join(' '));
    else if (item.method === 'Page.fileChooserOpened') choosers.push(item.params);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(domain + '.enable');
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloads, eventsEnabled: true }, null);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url });
  await until(async () => (await state())?.subtools?.length === 1, 'OBRush startup sphere');
  await clickControl('scene.clear');
  await until(async () => (await state())?.subtools?.length === 0, 'clear scene');
  await addFixture(1, 1);
  await addFixture(2, 2);
  const before = await save('before-export');
  requireTrue(before.project.scene?.objects?.length === 2, 'Before Save does not contain two imported parts');
  const initialState = await state();
  requireTrue(initialState.source_sgl_import?.options?.scale_center === false, 'Scale-center option was not at pinned default');
  await scrollInspectorToBottom();
  await clickControl('file.allow_attribute_loss');
  await clickControl('file.export_sgl.selected_only');
  const settings = await state();
  requireTrue(settings?.sgl_export?.selected_only === true, 'Selected-only setting did not toggle on');
  const beforeExportState = {
    revision: settings.document_revision,
    history: settings.history,
    selection: settings.selected_subtool_ids,
  };
  const { event, bytes } = await downloadAfter(() => clickControl('file.export_sketchfab_ply'), 'Sketchfab-ready PLY download');
  requireTrue(event.suggestedFilename.endsWith('.ply'), 'Downloaded file extension is not PLY');
  const decoded = decodePly(bytes);
  const expectedXYZ = [[1, -3, 2], [-4, 6, 5], [7, -9, -8]];
  const expectedRGB = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];
  for (let i = 0; i < 6; i++) {
    requireTrue(JSON.stringify(decoded.vertices[i].xyz) === JSON.stringify(expectedXYZ[i % 3]), `Axis conversion differs at vertex ${i}`);
    requireTrue(JSON.stringify(decoded.vertices[i].rgb) === JSON.stringify(expectedRGB[i % 3]), `RGB differs at vertex ${i}`);
  }
  requireTrue(JSON.stringify(decoded.faces) === JSON.stringify([[0, 1, 2], [3, 4, 5]]), 'PLY face payload differs');
  report.checks.push({ export: event.suggestedFilename, bytes: bytes.length, sha256: sha(bytes), vertices: decoded.vertices, faces: decoded.faces, padding_bytes: decoded.paddingBytes });
  const afterExportState = await state();
  requireTrue(afterExportState.document_revision === beforeExportState.revision, 'Export advanced document revision');
  requireTrue(JSON.stringify(afterExportState.history) === JSON.stringify(beforeExportState.history), 'Export changed Undo/Redo history');
  requireTrue(JSON.stringify(afterExportState.selected_subtool_ids) === JSON.stringify(beforeExportState.selection), 'Export changed selection');
  const after = await save('after-export');
  requireTrue(after.hash === before.hash, 'Sketchfab PLY export mutated project Save');
  requireTrue(report.errors.length === 0, 'Browser runtime errors occurred');
  report.pass = true;
} catch (error) {
  report.pass = false;
  report.failure = String(error.stack || error);
} finally {
  if (socket) socket.close();
  if (chrome) chrome.kill('SIGTERM');
  if (server) await new Promise(resolve => server.close(resolve));
  await writeFile(path.join(out, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  process.stdout.write(JSON.stringify({ pass: report.pass, report: path.join(out, 'report.json'), failure: report.failure || null }) + '\n');
}
if (!report.pass) process.exitCode = 1;
