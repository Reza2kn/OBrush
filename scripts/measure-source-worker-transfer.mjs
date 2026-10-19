// Diagnostic only: raw frozen SGL byte handoff, not a sculpt or parity test.
// Usage: node scripts/measure-source-worker-transfer.mjs /absolute/fixture/dir
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const out = path.join(root, '.artifacts', 'source-worker-transfer', randomUUID());
await mkdir(out, { recursive: true });
if (!process.argv[2]) throw Error('Pass the frozen SGL fixture directory');
const fixtureDir = path.resolve(process.argv[2]);
const fixtures = {
  '100k': path.join(fixtureDir, 'independent-id-sphere-100k.sgl'),
  '1m': path.join(fixtureDir, 'independent-id-sphere-1m.sgl'),
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const expectedSha = {
  '100k': '8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093',
  '1m': 'dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe',
};
const input = {};
for (const [key, file] of Object.entries(fixtures)) {
  const bytes = await readFile(file);
  if (sha256(bytes) !== expectedSha[key]) throw Error(`Frozen ${key} fixture hash differs`);
  input[key] = { file, bytes: bytes.byteLength, sha256: sha256(bytes) };
}
const result = { scope: 'Raw SGL ArrayBuffer worker transfer/clone round-trip only; no app, WASM, geometry, rendering, or history', inputs: input, cases: [], status: 'running' };
const html = `<!doctype html><meta charset="utf-8"><script type="module">
const workerSource = \`self.onmessage = event => {
  const {id, buffer, transfer} = event.data;
  if (!(buffer instanceof ArrayBuffer)) throw Error('Invalid worker input');
  self.postMessage({id, buffer}, transfer ? [buffer] : []);
};\`;
const worker = new Worker(URL.createObjectURL(new Blob([workerSource], {type:'text/javascript'})));
const ack = new Map();
worker.onmessage = event => { const callback = ack.get(event.data.id); if (callback) { ack.delete(event.data.id); callback(event.data.buffer); } };
window.probe = async (name, transfer, repetitions) => {
  const fetched = await fetch('/fixture/' + name);
  const original = await fetched.arrayBuffer();
  const rows = [];
  for (let i = 0; i < repetitions; i++) {
    const preparationStarted = performance.now();
    const input = original.slice(0); // excluded: caller-owned buffer preparation
    const prepareMs = performance.now() - preparationStarted;
    const id = name + '-' + transfer + '-' + i;
    const finished = new Promise(resolve => ack.set(id, resolve));
    const started = performance.now();
    worker.postMessage({id, buffer: input, transfer}, transfer ? [input] : []);
    const detached = input.byteLength === 0;
    const reply = await finished;
    const ms = performance.now() - started;
    const expected = new Uint8Array(original), actual = new Uint8Array(reply);
    if (actual.length !== expected.length || detached !== transfer) throw Error('Transfer boundary contract differs');
    for (let j = 0; j < expected.length; j++) if (actual[j] !== expected[j]) throw Error('Transfer boundary bytes differ at ' + j);
    rows.push({prepare_ms:prepareMs, round_trip_ms:ms});
  }
  return {name, transfer, bytes:original.byteLength, samples:rows, detached_after_send:transfer};
};
</script>`;
let server, chrome, socket, session, next = 0;
let stderr = '';
const waiting = new Map();
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const response = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error('CDP timeout: ' + method)); }, 120000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return response;
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn, label) {
  for (let i = 0; i < 150; i++) { const value = await fn(); if (value) return value; await pause(100); }
  throw Error('Timed out: ' + label);
}
try {
  server = createServer(async (request, response) => {
    try {
      const name = new URL(request.url, 'http://127.0.0.1').pathname.replace('/fixture/', '');
      if (request.url === '/') {
        response.setHeader('Content-Type', 'text/html'); response.end(html); return;
      }
      if (!(name in fixtures)) throw Error('Unknown fixture');
      response.setHeader('Content-Type', 'application/octet-stream');
      response.end(await readFile(fixtures[name]));
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`,
    '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking',
    'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (!message.id) return;
    const pending = waiting.get(message.id);
    if (!pending) return;
    waiting.delete(message.id); clearTimeout(pending.timer);
    message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable');
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
  await until(async () => await evaluate('typeof window.probe === "function"'), 'probe ready');
  for (const name of ['100k', '1m']) for (const transfer of [false, true]) {
    const row = await evaluate(`window.probe(${JSON.stringify(name)},${transfer},5)`);
    if (row.bytes !== input[name].bytes || row.samples.length !== 5) throw Error('Incomplete browser row');
    result.cases.push(row);
  }
  result.status = 'passed';
} catch (error) { result.status = 'failed'; result.error = String(error); }
finally {
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close();
  for (const pending of waiting.values()) { clearTimeout(pending.timer); pending.reject(Error('Closing browser')); }
  waiting.clear();
  await pause(400);
  if (chrome && chrome.exitCode === null) chrome.kill('SIGTERM');
  if (server) await new Promise(resolve => server.close(resolve));
  result.cleanup = { chrome_exit: chrome?.exitCode ?? null, chrome_signal: chrome?.signalCode ?? null, server_closed: !!server };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  await writeFile(path.join(out, 'chrome-stderr.log'), stderr);
  console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), cases: result.cases, error: result.error }));
}
if (result.status !== 'passed') process.exitCode = 1;
