// Opt-in WebGPU resident-release check. Usage: node scripts/check-resident-sparse-one-map.mjs web/dist
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve(process.argv[2] || 'web/dist');
const out = path.resolve('.artifacts/resident-sparse-one-map/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const result = {
  status: 'running',
  wasm_sha256: sha(await readFile(path.join(root, 'gpu-pkg/resident_bg.wasm'))),
  app_wasm_sha256: sha(await readFile(path.join(root, 'pkg/obrush_app_bg.wasm'))),
  observations: {}, errors: [],
};
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let server, chrome, socket, session, next = 0;
const waiting = new Map();
async function send(method, params = {}, target = session) {
  const id = ++next;
  const answer = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
  return answer;
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(read, name) {
  for (let i = 0; i < 200; i++) { const value = await read(); if (value) return value; await sleep(100); }
  throw Error(`Timed out waiting for ${name}`);
}
try {
  server = createServer(async (request, response) => {
    try {
      const name = new URL(request.url, 'http://127.0.0.1').pathname;
      const file = path.resolve(root, '.' + (name === '/' ? '/index.html' : name));
      if (!file.startsWith(root + path.sep)) throw Error('outside served tree');
      const bytes = await readFile(file);
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`,
    '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking',
    '--enable-unsafe-webgpu', '--window-size=1600,1000', 'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const pending = waiting.get(message.id);
      if (!pending) return;
      waiting.delete(message.id); clearTimeout(pending.timer);
      message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
    } else if (message.method === 'Runtime.exceptionThrown') result.errors.push(message.params.exceptionDetails.text);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/?webgpu-preview` });
  await until(() => evaluate("document.querySelector('#loading')===null"), 'OBRush startup');
  // Let the first app frames settle before the preview captures its camera.
  await sleep(600);
  result.observations.before = await evaluate('window.obrushBrushMetrics()');
  result.observations.start = await evaluate('window.obrushResidentPointerStart()');
  const rect = await until(() => evaluate("(()=>{const c=document.querySelector('#obrush-resident-pointer-canvas');if(!c)return null;const r=c.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}})()"), 'resident canvas');
  const x = rect.x + rect.width / 2, y = rect.y + rect.height / 2;
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  for (let i = 1; i <= 5; i++) {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + i * 4, y, button: 'left', buttons: 1 });
    await sleep(35);
  }
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + 20, y, button: 'left', buttons: 0, clickCount: 1 });
  result.observations.end = await until(async () => {
    const status = await evaluate('window.obrushResidentPointerStatus()');
    return status.phase === 'idle' && status.last ? status.last : null;
  }, 'resident release');
  if (result.observations.end.error || !(result.observations.end.sparseBytes > 0) ||
      !(result.observations.end.core_timing?.changed_vertices > 0) ||
      result.observations.end.gpuReport?.sparse_one_map_readbacks !== 1 ||
      result.observations.end.gpuReport?.position_readbacks !== 0) {
    throw Error('Resident sparse release did not use one checked map and commit changed vertices');
  }
  result.status = 'passed scoped browser resident sparse release';
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error.stack || error));
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close();
  if (chrome?.exitCode === null) chrome.kill();
  server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
