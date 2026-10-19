// Cross-realm diagnostic only. It does not exercise ordinary held-pointer UI.
// Usage: node scripts/check-source-worker-realm.mjs [web/dist]
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] ?? 'web/dist');
const out = path.resolve('.artifacts/source-worker-realm', randomUUID());
await mkdir(out, { recursive: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fixture = JSON.stringify({
  version: 7, name: 'Independent Source Worker triangle',
  mesh: {
    positions: [[0, 0, 0], [4, 0, 0], [0, 4, 0]],
    normals: [[0, 0, 1], [0, 0, 1], [0, 0, 1]],
    triangles: [[0, 1, 2]], masks: [1, 1, 1],
    mask_convention: 'unprotected_f32_v1',
    colors: [[1, 1, 1], [1, 1, 1], [1, 1, 1]],
  },
});
const options = JSON.stringify({
  tool: 'Inflate', intensity: .3, intensity_factor: 0, negative: false,
  culling: false, clay_order: 'SingleLeaf', prevent_folds: true,
});
const input = JSON.stringify({
  world_to_clip: [.5, 0, 0, 0, 0, .5, 0, 0, 0, 0, -.10050251256281408, 0,
    0, 0, -.6080402010050252, 1],
  viewport: [0, 0, 512, 512], screen: [320, 192], screen_radius: 220,
  pressure: .5, radius_factor: 1, in_contact: true,
});
const page = `<!doctype html><meta charset="utf-8"><script type="module">
window.runSourceWorkerRealm=async(project,options,input)=>{
  const worker=new Worker('/source-geometry-worker.js',{type:'module'});
  let id=0;
  const call=(kind,data)=>new Promise((resolve,reject)=>{
    const current=++id;
    const timeout=setTimeout(()=>reject(Error('Worker '+kind+' timed out')),30000);
    const receive=event=>{
      if(event.data?.id!==current)return;
      worker.removeEventListener('message',receive);clearTimeout(timeout);
      event.data.success?resolve(event.data):reject(Error(event.data.error));
    };
    worker.addEventListener('message',receive);
    worker.postMessage({id:current,kind,...data});
  });
  try{
    await call('begin',{project,options,strokeEpoch:'91',warmSeed:new Uint8Array()});
    const applied=await call('apply',{strokeEpoch:'91',sequence:'1',input});
    const finished=await call('finish',{strokeEpoch:'91',sequence:'2'});
    const rows=packet=>new DataView(packet.buffer,packet.byteOffset,packet.byteLength).getUint32(16,true);
    return {apply_bytes:applied.delta.byteLength,apply_rows:rows(applied.delta),
      final_bytes:finished.delta.byteLength,final_rows:rows(finished.delta),
      retained_bytes:finished.retained.byteLength,timing:applied.timing,
      worker_type:worker.constructor.name};
  }finally{worker.terminate();}
};
</script>`;
const receipt = { scope: 'Display-free real Worker/WASM realm only; ordinary held-pointer and studio publication remain unverified',
  fixture_sha256: hash(fixture), options_sha256: hash(options), input_sha256: hash(input),
  wasm_sha256: hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))), status: 'running' };
let server, chrome, socket, session, seq = 0;
const waiting = new Map();
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
function send(method, params = {}, target = session) {
  const id = ++seq;
  const promise = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error('CDP timeout: ' + method)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
  return promise;
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn, label) {
  for (let i = 0; i < 150; i++) { const result = await fn(); if (result) return result; await pause(100); }
  throw Error('Timed out: ' + label);
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      if (pathname === '/') { response.setHeader('Content-Type', 'text/html'); response.end(page); return; }
      const file = path.resolve(web, '.' + pathname);
      if (!file.startsWith(web + path.sep)) throw Error('Outside web root');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript' })[path.extname(file)] ?? 'application/octet-stream');
      response.end(await readFile(file));
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    '--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0',
    '--no-first-run', '--disable-background-networking', 'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', data => { stderr += data; });
  const endpoint = await until(() => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const response = JSON.parse(String(event.data));
    if (!response.id) return;
    const pending = waiting.get(response.id);
    if (!pending) return;
    waiting.delete(response.id); clearTimeout(pending.timer);
    response.error ? pending.reject(Error(JSON.stringify(response.error))) : pending.resolve(response.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable');
  await send('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
  await until(() => evaluate('typeof window.runSourceWorkerRealm === "function"'), 'realm probe');
  receipt.result = await evaluate(`window.runSourceWorkerRealm(${JSON.stringify(fixture)},${JSON.stringify(options)},${JSON.stringify(input)})`);
  assert.ok(receipt.result.apply_rows > 0);
  assert.ok(receipt.result.final_rows > 0);
  assert.ok(receipt.result.retained_bytes > 0);
  assert.equal(receipt.result.timing.accepted_dabs, 1);
  receipt.status = 'passed';
} catch (error) { receipt.status = 'failed'; receipt.error = String(error); }
finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(receipt, null, 2) + '\n');
  try { socket?.close(); } catch {}
  try { chrome?.kill('SIGTERM'); } catch {}
  try { server?.close(); } catch {}
}
console.log(JSON.stringify({ receipt: path.join(out, 'result.json'), ...receipt }));
if (receipt.status !== 'passed') process.exitCode = 1;
