// Owns a headless Chrome/profile; never reloads the user's sculpting tab.
// node scripts/check-unsaved-browser.mjs [web/dist]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const out = path.resolve(`.artifacts/alpha-unsaved/${randomUUID()}`);
const profile = path.join(out, 'profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { status: 'running', out, dialogs: [], errors: [], checks: {} };
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://localhost').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) throw Error('Outside web root');
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    response.end(await readFile(file));
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/?wasm-scalar`;
const child = spawn(process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0',
  '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-sync',
  '--window-size=1280,900', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
let chromeErrors = '';
child.stderr.on('data', bytes => chromeErrors += bytes);
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
const key = async name => {
  const modifiers = 4;
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: name, code: 'Key' + name.toUpperCase(), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: name, code: 'Key' + name.toUpperCase(), modifiers });
};
try {
  let endpoint;
  await waitFor(async () => {
    try {
      const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n');
      endpoint = `ws://127.0.0.1:${lines[0]}${lines[1]}`; return true;
    } catch { return false; }
  }, 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const request = pending.get(message.id);
      if (request) { clearTimeout(request.timer); pending.delete(message.id); message.error ? request.reject(Error(JSON.stringify(message.error))) : request.resolve(message.result); }
    } else if (message.method === 'Page.javascriptDialogOpening') {
      result.dialogs.push(message.params);
      send('Page.handleJavaScriptDialog', { accept: false }, message.sessionId).catch(error => result.errors.push(String(error)));
    } else if (message.method === 'Runtime.exceptionThrown') result.errors.push(message.params.exceptionDetails);
  });
  result.browser = await send('Browser.getVersion', {}, null);
  result.wasm_sha256 = hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm')));
  result.index_sha256 = hash(await readFile(path.join(web, 'index.html')));
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: downloads }, null);
  await send('Page.navigate', { url }); await ready(); await pause(400);
  result.checks.initial_clean = !(await evaluate('window.obrushBrushMetrics().dirty'));
  // A trusted toolbar contact establishes activation without editing the mesh.
  for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: 25, y: 25, button: 'left', buttons: type === 'mousePressed' ? 1 : 0, clickCount: 1 });
  await send('Page.reload'); await pause(800); await ready();
  result.checks.clean_reload_no_warning = result.dialogs.length === 0;
  const viewport = await evaluate('window.obrushBrushMetrics().camera.viewport');
  const x = (viewport[0] + viewport[2]) / 2, y = (viewport[1] + viewport[3]) / 2;
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  for (let step = 1; step <= 8; step++) {
    await pause(35);
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + step * 3, y, button: 'none', buttons: 1 });
  }
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + 24, y, button: 'left', buttons: 0, clickCount: 1 });
  await pause(400);
  result.checks.draw_makes_dirty = await evaluate('window.obrushBrushMetrics().dirty');
  await key('s');
  await waitFor(async () => (await readdir(downloads)).some(name => name.endsWith('.obrush')), 'public project download');
  const projectPath = path.join(downloads, (await readdir(downloads)).find(name => name.endsWith('.obrush')));
  const project = await readFile(projectPath);
  result.saved_project = { path: projectPath, sha256: hash(project), bytes: project.length, vertices: JSON.parse(project).mesh.positions.length };
  result.checks.requested_download_keeps_warning = await evaluate('window.obrushBrushMetrics().dirty');
  await send('Browser.setDownloadBehavior', { behavior: 'deny' }, null);
  await key('s'); await pause(300);
  result.checks.blocked_download_keeps_warning = await evaluate('window.obrushBrushMetrics().dirty');
  const before = await evaluate('window.obrushBrushMetrics().document_revision');
  await send('Page.navigate', { url: 'about:blank' });
  await waitFor(async () => result.dialogs.some(dialog => dialog.type === 'beforeunload'), 'leave-page warning');
  await pause(200);
  result.checks.cancel_navigation_preserves_document = before === await evaluate('window.obrushBrushMetrics().document_revision');
  result.checks.cancel_navigation_preserves_url = await evaluate('location.href') === url;
  const screenshot = await send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(out, 'saved-and-stayed.png'), Buffer.from(screenshot.data, 'base64'));
  if (!Object.values(result.checks).every(Boolean) || result.errors.length) throw Error('Unsaved-work gate failed');
  result.status = 'passed';
} catch (error) { result.status = 'failed'; result.failure = String(error); process.exitCode = 1; }
finally {
  if (socket?.readyState === WebSocket.OPEN) {
    try { await send('Browser.close', {}, null); } catch { }
    socket.close();
  }
  if (child.exitCode === null) await Promise.race([new Promise(resolve => child.once('exit', resolve)), pause(3000)]);
  if (child.exitCode === null) child.kill('SIGTERM');
  server.close();
  result.chrome_exit = child.exitCode;
  await writeFile(path.join(out, 'chrome-stderr.log'), chromeErrors);
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
}
