import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const web = path.join(root, 'web/dist');
const out = path.join(root, '.artifacts/native-move-m02', 'browser-' + randomUUID());
await mkdir(out, { recursive: true });
const hash = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { status: 'running', out, cases: [], errors: [], limitations: [] };
const server = createServer(async (req, res) => {
  try {
    const file = path.resolve(web, '.' + new URL(req.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) throw Error('outside web');
    const bytes = await readFile(file);
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    res.end(bytes);
  } catch { res.writeHead(404); res.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const profile = path.join(out, 'chrome-profile');
await mkdir(profile);
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1',
  '--remote-debugging-port=0', '--no-first-run', '--no-default-browser-check',
  '--disable-background-networking', '--disable-sync', '--disable-default-apps',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1',
  '--window-size=1600,1000', 'about:blank',
], { stdio: ['ignore', 'pipe', 'pipe'] });
let stderr = '';
chrome.stderr.on('data', chunk => { stderr += chunk; });
let socket, session, seq = 0;
const pending = new Map();
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++seq;
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
};
const pointer = (type, x, y, down = false) => send('Input.dispatchMouseEvent', {
  type, x, y, pointerType: 'mouse', button: type === 'mouseMoved' ? 'none' : 'left',
  buttons: down ? 1 : 0, clickCount: type === 'mouseMoved' ? 0 : 1,
});
const key = async (letter, modifiers = 0) => {
  const code = /^\d$/.test(letter) ? `Digit${letter}` : `Key${letter.toUpperCase()}`;
  const virtual = letter.toUpperCase().charCodeAt(0);
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: letter, code, windowsVirtualKeyCode: virtual, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: letter, code, windowsVirtualKeyCode: virtual, modifiers });
  await pause(200);
};
const command = async (letter, shift = false) => {
  const modifiers = 4 | (shift ? 8 : 0);
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await key(letter, modifiers);
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
  await pause(250);
};
const save = async name => {
  const index = await evaluate('window.__moveCaptures.length');
  await command('s');
  let size = 0;
  for (let i = 0; i < 50; i++) {
    size = await evaluate(`window.__moveCaptures[${index}]?.contents.length || 0`);
    if (size) break;
    await pause(100);
  }
  if (!size || size > 128 * 1024 * 1024) throw Error(`save failed ${name}: ${size}`);
  let value = '';
  for (let offset = 0; offset < size; offset += 65536) {
    value += await evaluate(`window.__moveCaptures[${index}].contents.slice(${offset},${offset + 65536})`);
  }
  const file = path.join(out, `${name}.obrush`);
  await writeFile(file, value);
  return { path: file, sha256: hash(value), bytes: Buffer.byteLength(value), document: JSON.parse(value) };
};
const compact = ({ document, ...meta }) => ({ ...meta, vertices: document.mesh.positions.length, triangles: document.mesh.triangles.length });
const meshHash = document => hash(JSON.stringify(document.mesh));
const screenshot = async name => {
  const response = await send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  const bytes = Buffer.from(response.data, 'base64');
  const file = path.join(out, `${name}.png`);
  await writeFile(file, bytes);
  return { path: file, sha256: hash(bytes) };
};
const fresh = async () => {
  await send('Page.navigate', { url });
  for (let i = 0; i < 180; i++) {
    if (await evaluate('!document.getElementById("loading") && typeof window.obrushBrushMetrics === "function"')) break;
    await pause(100);
  }
  await pause(300);
  await evaluate('window.__moveCaptures=[];window.obrushDownload=(name,contents)=>window.__moveCaptures.push({name,contents});document.getElementById("obrush_canvas").focus()');
  await key('7');
  const settings = await evaluate('window.obrushBrushMetrics()');
  if (settings.kind !== 'Move') throw Error('visible key 7 did not select Move');
  const canvas = await evaluate('(()=>{const r=document.getElementById("obrush_canvas").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}})()');
  return { settings, canvas };
};
const instrument = `(()=>{
  const t=window.__moveTrace={raf:0,events:[],errors:[]};
  const tick=()=>requestAnimationFrame(()=>{t.raf++;tick()});tick();
  for(const type of ['pointerdown','pointermove','pointerup','pointercancel'])window.addEventListener(type,e=>t.events.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons,trusted:e.isTrusted,raf:t.raf,at:performance.now()}),true);
  window.addEventListener('error',e=>t.errors.push(String(e.error||e.message)));
  window.addEventListener('unhandledrejection',e=>t.errors.push(String(e.reason)));
})();`;
const runCase = async burst => {
  const name = burst ? 'burst' : 'held';
  const row = { name, build: result.wasm };
  result.cases.push(row);
  Object.assign(row, await fresh());
  const { canvas } = row;
  const start = [canvas.x + canvas.width * 0.5 - 80, canvas.y + canvas.height * 0.5 - 30];
  const points = Array.from({ length: 8 }, (_, i) => [start[0] + 5 * (i + 1), start[1] - 2 * (i + 1)]);
  row.start = start; row.points = points; row.screenshot_before = await screenshot(name + '-before');
  const before = await save(name + '-before'); row.before = compact(before);
  await pointer('mouseMoved', ...start);
  if (!burst) await pause(30);
  await pointer('mousePressed', ...start, true);
  if (!burst) await pause(30);
  for (const point of points) { await pointer('mouseMoved', ...point, true); if (!burst) await pause(30); }
  await pointer('mouseReleased', ...points.at(-1));
  await pause(800);
  row.events = await evaluate('window.__moveTrace.events');
  row.errors = await evaluate('window.__moveTrace.errors');
  row.screenshot_after = await screenshot(name + '-after');
  const after = await save(name + '-after'); row.after = compact(after);
  const prior = before.document.mesh.positions, final = after.document.mesh.positions;
  row.changed_original = prior.reduce((n, point, i) => n + Number(Math.hypot(...point.map((v, axis) => v - final[i][axis])) > 1e-7), 0);
  row.frame_indices = [...new Set(row.events.filter(e => ['pointerdown','pointermove','pointerup'].includes(e.type)).map(e => e.raf))];
  row.trusted = row.events.filter(e => ['pointerdown','pointermove','pointerup'].includes(e.type)).every(e => e.trusted);
  await command('z');
  const undo = await save(name + '-undo'); row.undo = compact(undo);
  await command('z', true);
  const redo = await save(name + '-redo'); row.redo = compact(redo);
  row.undo_exact = meshHash(undo.document) === meshHash(before.document);
  row.redo_exact = meshHash(redo.document) === meshHash(after.document);
  row.passed = row.changed_original >= 8 && row.trusted && row.undo_exact && row.redo_exact && row.errors.length === 0;
  if (!row.passed) throw Error(`${name} gate failed: ${JSON.stringify({changed:row.changed_original,trusted:row.trusted,undo:row.undo_exact,redo:row.redo_exact,errors:row.errors})}`);
};

try {
  let endpoint;
  for (let i = 0; i < 150; i++) {
    try { const [port, suffix] = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); endpoint = `ws://127.0.0.1:${port}${suffix}`; break; } catch {}
    if (chrome.exitCode !== null) throw Error(`Chrome exit ${chrome.exitCode}`);
    await pause(100);
  }
  if (!endpoint) throw Error('Chrome endpoint missing');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const item = pending.get(message.id);
      if (item) { clearTimeout(item.timer); pending.delete(message.id); message.error ? item.reject(Error(JSON.stringify(message.error))) : item.resolve(message.result); }
    }
  });
  result.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const wasm = await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'));
  const served = Buffer.from(await (await fetch(url + 'pkg/obrush_app_bg.wasm')).arrayBuffer());
  result.wasm = { local_sha256: hash(wasm), served_sha256: hash(served), bytes: wasm.length };
  if (result.wasm.local_sha256 !== result.wasm.served_sha256) throw Error('served WASM mismatch');
  await runCase(false);
  await runCase(true);
  result.status = 'passed';
  result.limitations.push('Owned synthetic CDP mouse; burst is same-frame only if measured RAF indices coincide. Browser RAF does not itself prove egui frame coalescing. Native CUA case remains separate.');
} catch (error) {
  result.status = 'failed'; result.failure = String(error);
  process.exitCode = 1;
  try { result.failure_screen = await screenshot('failure'); } catch {}
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  await writeFile(path.join(out, 'chrome-stderr.log'), stderr);
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  try { socket?.close(); } catch {}
  await pause(300); if (chrome.exitCode === null) chrome.kill('SIGTERM');
  await new Promise(resolve => server.close(resolve));
  result.cleanup = { chrome_exit_code: chrome.exitCode, chrome_signal: chrome.signalCode, server_closed: true };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, failure: result.failure, out, cases: result.cases.map(({name,changed_original,frame_indices,passed,undo_exact,redo_exact})=>({name,changed_original,frame_indices,passed,undo_exact,redo_exact})) }));
}
