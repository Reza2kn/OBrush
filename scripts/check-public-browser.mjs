// Owns a disposable headless Chrome profile. Never touches a user's browser tab.
// Run after deploying: node scripts/check-public-browser.mjs https://obrush.app/ web/dist [auto|scalar]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const origin = new URL(process.argv[2] || 'https://obrush.app/');
if (origin.protocol !== 'https:' || origin.username || origin.password || origin.search || origin.hash) {
  throw Error('Expected a plain HTTPS site URL');
}
const mode = process.argv[4] || 'auto';
if (!['auto', 'scalar'].includes(mode)) throw Error('Expected auto or scalar backend mode');
const url = new URL(mode === 'scalar' ? './?wasm-scalar' : './', origin).href;
const web = path.resolve(process.argv[3] || 'web/dist');
const out = path.resolve(`.artifacts/public-browser/${randomUUID()}`);
const profile = path.join(out, 'profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { status: 'running', url, mode, web, out, checks: {}, assets: {}, dialogs: [], errors: [] };

async function served(relative) {
  const response = await fetch(new URL(relative, origin), { redirect: 'error', signal: AbortSignal.timeout(30000) });
  if (!response.ok) throw Error(`Public ${relative}: HTTP ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  result.assets[relative] = { bytes: bytes.length, sha256: hash(bytes), content_type: response.headers.get('content-type') };
  return bytes;
}

// Asset integrity is checked before launching Chrome; a stale or partial deploy cannot pass.
try {
const localInfo = await readFile(path.join(web, 'build-info.json'));
const publicInfo = await served('build-info.json');
result.build_info = JSON.parse(publicInfo);
result.checks.build_info_exact = hash(publicInfo) === hash(localInfo);
const assets = [
  ['index.html', null],
  ['pkg/obrush_app_bg.wasm', 'portable_wasm_sha256'],
  ['pkg-simd/obrush_app_bg.wasm', 'simd_wasm_sha256'],
  ['gpu-pkg/resident_bg.wasm', 'gpu_wasm_sha256'],
  ['obrush-icon.svg', null],
  ['favicon-32.png', null],
  ['apple-touch-icon.png', null],
];
for (const [name, infoKey] of assets) {
  const bytes = await served(name);
  const localHash = hash(await readFile(path.join(web, name)));
  result.assets[name].local_sha256 = localHash;
  result.checks[`asset_${name}`] = hash(bytes) === localHash && (!infoKey || result.build_info[infoKey] === localHash);
}
for (const name of ['favicon-32.png', 'apple-touch-icon.png']) {
  result.checks[`png_${name}`] = (await readFile(path.join(web, name))).subarray(0, 8)
    .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
}
if (!Object.values(result.checks).every(Boolean)) throw Error('Public asset integrity gate failed');

} catch (error) {
  result.status = 'failed'; result.failure = String(error);
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  process.exit(1);
}

const child = spawn(process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0',
  '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-sync',
  '--window-size=1280,900', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
let chromeErrors = '';
child.stderr.on('data', bytes => { chromeErrors += bytes; });
let socket, session, sequence = 0;
const pending = new Map();
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 30000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
};
const waitFor = async (predicate, label) => {
  for (let n = 0; n < 200; n++) { if (await predicate()) return; await pause(100); }
  throw Error(`Timeout: ${label}`);
};
const ready = () => waitFor(async () => {
  try { return await evaluate('!document.getElementById("loading") && !!window.obrushBrushMetrics'); }
  catch { return false; }
}, 'engine startup');
const saveKey = async () => {
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 's', code: 'KeyS', modifiers: 4 });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 's', code: 'KeyS', modifiers: 4 });
};

try {
  let endpoint;
  await waitFor(async () => {
    try {
      const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n');
      endpoint = `ws://127.0.0.1:${lines[0]}${lines[1]}`;
      return true;
    } catch { return false; }
  }, 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const request = pending.get(message.id);
      if (request) {
        clearTimeout(request.timer); pending.delete(message.id);
        message.error ? request.reject(Error(JSON.stringify(message.error))) : request.resolve(message.result);
      }
    } else if (message.method === 'Page.javascriptDialogOpening') {
      result.dialogs.push(message.params);
      send('Page.handleJavaScriptDialog', { accept: false }, message.sessionId)
        .catch(error => result.errors.push(String(error)));
    } else if (message.method === 'Runtime.exceptionThrown') {
      result.errors.push(message.params.exceptionDetails);
    }
  });
  result.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: downloads }, null);
  await send('Page.navigate', { url }); await ready(); await pause(400);
  result.selected_backend = await evaluate('window.obrushWasmBackend');
  result.brush = await evaluate('({kind:window.obrushBrushMetrics().kind,radius:window.obrushBrushMetrics().radius,strength:window.obrushBrushMetrics().strength})');
  result.checks.draw_selected = result.brush.kind === 'Draw';
  result.checks.https_loaded = await evaluate('location.protocol === "https:" && location.hostname === ' + JSON.stringify(origin.hostname));
  result.checks.initial_clean = !(await evaluate('window.obrushBrushMetrics().dirty'));
  // Trusted click establishes user activation without modifying the clean document.
  for (const type of ['mousePressed', 'mouseReleased']) {
    await send('Input.dispatchMouseEvent', { type, x: 25, y: 25, button: 'left', buttons: type === 'mousePressed' ? 1 : 0, clickCount: 1 });
  }
  await send('Page.reload'); await pause(800); await ready();
  result.checks.clean_reload_no_warning = result.dialogs.length === 0;
  const viewport = await evaluate('window.obrushBrushMetrics().camera.viewport');
  const x = (viewport[0] + viewport[2]) / 2, y = (viewport[1] + viewport[3]) / 2;
  result.stroke = { tool: 'Draw', viewport, start: [x, y], moves: 8, step_css_pixels: 3 };
  const before = await evaluate('window.obrushBrushMetrics().document_revision');
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  for (let step = 1; step <= 8; step++) {
    await pause(35);
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + step * 3, y, button: 'none', buttons: 1 });
  }
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + 24, y, button: 'left', buttons: 0, clickCount: 1 });
  await pause(400);
  result.checks.draw_changed_revision = (await evaluate('window.obrushBrushMetrics().document_revision')) > before;
  result.checks.draw_makes_dirty = await evaluate('window.obrushBrushMetrics().dirty');
  await saveKey();
  await waitFor(async () => (await readdir(downloads)).some(name => name.endsWith('.obrush')), 'project download');
  const projectPath = path.join(downloads, (await readdir(downloads)).find(name => name.endsWith('.obrush')));
  const projectBytes = await readFile(projectPath);
  const project = JSON.parse(projectBytes);
  result.saved_project = { path: projectPath, sha256: hash(projectBytes), bytes: projectBytes.length,
    vertices: project.mesh?.positions?.length ?? 0 };
  result.checks.project_download_valid = result.saved_project.vertices > 0;
  result.checks.requested_download_keeps_warning = await evaluate('window.obrushBrushMetrics().dirty');
  await send('Browser.setDownloadBehavior', { behavior: 'deny' }, null);
  await saveKey(); await pause(300);
  result.checks.blocked_download_keeps_warning = await evaluate('window.obrushBrushMetrics().dirty');
  const finalRevision = await evaluate('window.obrushBrushMetrics().document_revision');
  await send('Page.navigate', { url: 'about:blank' });
  await waitFor(async () => result.dialogs.some(dialog => dialog.type === 'beforeunload'), 'leave-page warning');
  await pause(200);
  result.checks.cancel_navigation_preserves_document = finalRevision === await evaluate('window.obrushBrushMetrics().document_revision');
  result.checks.cancel_navigation_preserves_url = await evaluate('location.href') === url;
  const screenshot = await send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(out, 'saved-and-stayed.png'), Buffer.from(screenshot.data, 'base64'));
  if (!Object.values(result.checks).every(Boolean) || result.errors.length) throw Error('Public browser gate failed');
  result.status = 'passed';
} catch (error) {
  result.status = 'failed'; result.failure = String(error); process.exitCode = 1;
} finally {
  if (socket?.readyState === WebSocket.OPEN) {
    try { await send('Browser.close', {}, null); } catch { /* Browser may already be closed. */ }
    socket.close();
  }
  if (child.exitCode === null) await Promise.race([new Promise(resolve => child.once('exit', resolve)), pause(3000)]);
  if (child.exitCode === null) child.kill('SIGTERM');
  result.chrome_exit = child.exitCode;
  await writeFile(path.join(out, 'chrome-stderr.log'), chromeErrors);
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
}
