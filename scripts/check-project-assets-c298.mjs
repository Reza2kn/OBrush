// Owned, normal-browser C298 file-input/keyboard workflow. No user tab access.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const web = path.join(root, 'web/dist');
const out = path.join(root, '.artifacts', 'c298', randomUUID());
const profile = path.join(out, 'chrome-profile');
await mkdir(profile, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const files = {
  mesh: path.join(root, 'crates/obrush-app/tests/fixtures/c294_uv_quad.obj'),
  uv: path.join(root, 'crates/obrush-app/tests/fixtures/c294_uv_quadrants.jpg'),
  matcap: path.join(root, 'crates/obrush-app/tests/fixtures/c291_asymmetric_matcap.png'),
};
const report = { status: 'running', out, fixture_sha256: {}, captures: [], events: [], failures: [] };
for (const [key, file] of Object.entries(files)) report.fixture_sha256[key] = hash(await readFile(file));
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) { response.writeHead(403); response.end(); return; }
    const bytes = await readFile(file);
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    response.end(bytes);
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/?wasm-scalar`;
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0',
  '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update',
  '--disable-sync', '--disable-default-apps', '--disable-breakpad', '--metrics-recording-only',
  '--safebrowsing-disable-auto-update', '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
  '--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1', '--window-size=1600,1000', 'about:blank',
], { stdio: ['ignore', 'pipe', 'pipe'] });
let socket, session, sequence = 0;
let chromeOutput = '';
chrome.stdout.on('data', chunk => chromeOutput += chunk);
chrome.stderr.on('data', chunk => chromeOutput += chunk);
const pending = new Map();
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 45000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
};
const metrics = () => evaluate('window.obrushBrushMetrics?.()');
const wait = async (predicate, label, count = 120) => {
  for (let i = 0; i < count; i++) {
    const value = await predicate();
    if (value) return value;
    await pause(100);
  }
  throw Error(`Timed out: ${label}`);
};
const pointer = async (type, x, y, pressed = false) => send('Input.dispatchMouseEvent', {
  type, x, y, pointerType: 'mouse', button: type === 'mouseMoved' ? 'none' : 'left',
  buttons: pressed ? 1 : 0, ...(type === 'mouseMoved' ? {} : { clickCount: 1 }),
});
const clickPoint = async (x, y) => {
  await pointer('mouseMoved', x, y); await pause(30);
  await pointer('mousePressed', x, y, true); await pause(25);
  await pointer('mouseReleased', x, y); await pause(120);
  report.events.push({ click: [x, y] });
};
const clickControl = async key => {
  for (let i = 0; i < 32; i++) {
    const m = await metrics();
    const rect = m?.layer_controls?.[key];
    if (rect && rect[1] >= 80 && rect[3] <= 950 && rect[0] >= 0 && rect[2] <= 1600) {
      await clickPoint((rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2);
      return rect;
    }
    if (i === 0 && rect && rect[1] > 1800) {
      // egui owns a narrow side-panel scrollbar; dragging it is a normal UI
      // gesture and reliably exposes controls below the long scene panel.
      await pointer('mouseMoved', 1582, 150);
      await pointer('mousePressed', 1582, 150, true);
      for (let step = 1; step <= 10; step++) {
        await pointer('mouseMoved', 1582, 150 + step * 55, true);
        await pause(12);
      }
      await pointer('mouseReleased', 1582, 700);
      await pause(150);
      report.events.push({ scrollbar_drag: [1582, 150, 1582, 700], target: key });
      continue;
    }
    const deltaY = rect && rect[1] < 80 ? -320 : 320;
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 1570, y: 550, deltaX: 0, deltaY });
    await pause(100);
  }
  throw Error(`Cannot reach UI control ${key}`);
};
const chooseShading = async key => {
  await clickControl('viewport.shading');
  await clickControl(`viewport.shading.${key}`);
  await wait(async () => (await metrics())?.viewport_shading === (key === 'uv_texture' ? 'UV texture' : 'Matcap'), `shading ${key}`);
};
const fileInput = async (selector, file) => {
  const before = { pending: (await metrics())?.uv_texture_pending_request,
    matcap_pending: (await metrics())?.matcap?.pending_request,
    files: await evaluate(`document.querySelector(${JSON.stringify(selector)})?.files?.length`) };
  const rootNode = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: rootNode.nodeId, selector })).nodeId;
  if (!nodeId) throw Error(`Missing input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId, files: [file] });
  await pause(200);
  const after = { pending: (await metrics())?.uv_texture_pending_request,
    matcap_pending: (await metrics())?.matcap?.pending_request,
    files: await evaluate(`Array.from(document.querySelector(${JSON.stringify(selector)})?.files||[]).map(f=>f.name)`),
    notice: await evaluate('document.getElementById("notice")?.textContent') };
  report.events.push({ input: selector, file: path.basename(file), before, after });
};
const chord = async (key, modifiers = 4) => {
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code: `Key${key.toUpperCase()}`, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code: `Key${key.toUpperCase()}`, windowsVirtualKeyCode: key.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
};
const save = async label => {
  const before = await evaluate('window.__c298Captures?.length || 0');
  await chord('s');
  const capture = await wait(async () => {
    if ((await evaluate('window.__c298Captures?.length || 0')) <= before) return null;
    const value = await evaluate('window.__c298Captures?.at(-1)');
    return value?.text?.startsWith('{') ? value : null;
  }, `Save ${label}`);
  const file = path.join(out, `${label}.obrush`);
  await writeFile(file, capture.text);
  const parsed = JSON.parse(capture.text);
  report.captures.push({ label, file, sha256: hash(capture.text), bytes: capture.text.length,
    version: parsed.version, assets: parsed.assets && { uv: parsed.assets.uv?.name, matcaps: parsed.assets.matcaps?.map(m => m.name), shading: parsed.assets.shading, matcap_index: parsed.assets.matcap_index },
    metrics: await metrics() });
  return { file, text: capture.text, parsed };
};
const screenshot = async label => {
  await pause(250);
  const image = Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).data, 'base64');
  const file = path.join(out, `${label}.png`);
  await writeFile(file, image);
  report.events.push({ screenshot: label, file, sha256: hash(image) });
};
const fresh = async () => {
  await send('Page.navigate', { url });
  await wait(async () => (await metrics())?.vertices ? true : false, 'OBRush startup', 180);
  await evaluate('window.__c298Captures=[];{const original=window.obrushDownload;window.obrushDownload=(name,text)=>{window.__c298Captures.push({name,text});original(name,text)}}');
};

try {
  let endpoint;
  for (let i = 0; i < 150; i++) {
    try { const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); endpoint = `ws://127.0.0.1:${Number(lines[0])}${lines[1]}`; break; }
    catch { await pause(100); }
  }
  if (!endpoint) throw Error('Owned Chrome CDP endpoint unavailable');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) { const item = pending.get(message.id); if (item) { clearTimeout(item.timer); pending.delete(message.id); message.error ? item.reject(Error(JSON.stringify(message.error))) : item.resolve(message.result); } }
  });
  report.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  report.wasm_sha256 = hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm')));
  report.served_wasm_sha256 = hash(Buffer.from(await (await fetch(new URL('pkg/obrush_app_bg.wasm', url))).arrayBuffer()));
  if (report.wasm_sha256 !== report.served_wasm_sha256) throw Error('Served WASM differs from built WASM');
  await fresh();
  await fileInput('#import-file', files.mesh);
  await wait(async () => (await metrics())?.vertices === 4, 'UV quad import');
  await chooseShading('uv_texture');
  await clickControl('viewport.uv_import');
  await wait(async () => Number.isInteger((await metrics())?.uv_texture_pending_request), 'UV chooser ownership');
  await fileInput('#uv-texture-file', files.uv);
  await wait(async () => (await metrics())?.uv_texture?.name === path.basename(files.uv), 'JPEG UV import');
  const a = await save('uv-original');
  await screenshot('uv-original');
  await chooseShading('matcap');
  await clickControl('viewport.matcap.import');
  await wait(async () => Number.isInteger((await metrics())?.matcap?.pending_request), 'MatCap chooser ownership');
  await fileInput('#matcap-file', files.matcap);
  await wait(async () => (await metrics())?.matcap?.selected === 9, 'MatCap import');
  const b = await save('matcap-original');
  await screenshot('matcap-original');
  await fileInput('#import-file', a.file);
  await wait(async () => (await metrics())?.viewport_shading === 'UV texture', 'Open UV project');
  const reopened = await save('uv-reopened');
  await screenshot('uv-reopened');
  if (reopened.text !== a.text) throw Error('UV Save→Open changed project bytes');
  await chord('z');
  await wait(async () => (await metrics())?.matcap?.selected === 9, 'Undo Open to MatCap');
  const undo = await save('matcap-undo');
  await screenshot('matcap-undo');
  if (undo.text !== b.text) throw Error('Undo Open did not restore MatCap project');
  await chord('z', 4 | 8);
  await wait(async () => (await metrics())?.viewport_shading === 'UV texture', 'Redo Open to UV');
  const redo = await save('uv-redo');
  if (redo.text !== a.text) throw Error('Redo Open did not restore UV project');
  const invalid = structuredClone(a.parsed); invalid.assets.uv.width = 0;
  const invalidFile = path.join(out, 'invalid-width.obrush');
  await writeFile(invalidFile, JSON.stringify(invalid));
  await fileInput('#import-file', invalidFile); await pause(200);
  const afterInvalid = await save('uv-after-invalid');
  await screenshot('uv-after-invalid');
  if (afterInvalid.text !== a.text) throw Error('Invalid image mutated the document');
  report.invalid_notice = await evaluate('document.getElementById("notice")?.textContent');
  await fresh();
  await fileInput('#import-file', b.file);
  await wait(async () => (await metrics())?.matcap?.selected === 9, 'Fresh-page MatCap Open');
  const freshB = await save('matcap-fresh-open');
  await screenshot('matcap-fresh-open');
  if (freshB.text !== b.text) throw Error('Fresh-page MatCap Open changed project bytes');
  report.status = 'passed public Save/Open/Undo/Redo and malformed-project gates';
} catch (error) {
  report.status = 'failed'; report.failures.push(String(error)); process.exitCode = 1;
  try { report.failure_metrics = await metrics(); await screenshot('failure'); }
  catch (secondary) { report.failures.push(`failure receipt: ${secondary}`); }
} finally {
  report.chrome_output_tail = chromeOutput.slice(-3000);
  await writeFile(path.join(out, 'result.json'), JSON.stringify(report, null, 2));
  for (const item of pending.values()) { clearTimeout(item.timer); item.reject(Error('Browser closing')); }
  socket?.close(); chrome.kill(); server.close();
  console.log(JSON.stringify({ status: report.status, out, failures: report.failures, captures: report.captures.map(c => [c.label, c.sha256]) }));
}
