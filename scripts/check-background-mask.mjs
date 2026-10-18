// Owned normal-browser check of SculptGL-style Ctrl gestures on empty canvas.
// Usage: node scripts/check-background-mask.mjs [web/dist] [label] [reference]
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = process.argv[4] === 'reference';
const out = path.resolve(`.artifacts/background-mask/${process.argv[3] || 'browser'}-${randomUUID()}`);
const profile = path.join(out, 'profile');
await mkdir(profile, { recursive: true });
const hash = value => createHash('sha256').update(value).digest('hex');
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition, message) => { if (!condition) throw new Error(message); };
const result = { status: 'running', out, mode: reference ? 'pinned SculptGL host mirror' : 'normal OBrush', protocol: 'Owned Chrome/CDP mouse chords on the normal viewport; no direct document mutation', cases: [], exceptions: [], console_errors: [] };
const server = createServer(async (request, response) => {
  try {
    const file = path.resolve(web, '.' + new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html'));
    if (!file.startsWith(web + path.sep)) { response.writeHead(403); response.end(); return; }
    const bytes = await readFile(file);
    response.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : file.endsWith('.png') ? 'image/png' : 'text/html');
    response.end(bytes);
  } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-address=127.0.0.1',
  '--remote-debugging-port=0', '--no-first-run', '--no-default-browser-check',
  '--disable-background-networking', '--disable-component-update', '--disable-sync',
  '--disable-default-apps', '--disable-breakpad', '--metrics-recording-only',
  '--safebrowsing-disable-auto-update', '--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1',
  '--window-size=1600,1000', 'about:blank',
], { stdio: 'ignore' });
result.owned_pid = chrome.pid;
let socket, session, sequence = 0;
const pending = new Map();
const send = (method, params = {}, target = session) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 30000);
  pending.set(id, { resolve, reject, timer });
  socket.send(JSON.stringify({ id, method, params, ...(target ? { sessionId: target } : {}) }));
});
const evaluate = async expression => {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
};
const mouse = (type, x, y, down = false) => send('Input.dispatchMouseEvent', {
  type, x, y, modifiers: 2, pointerType: 'mouse',
  button: type === 'mouseMoved' ? 'none' : 'left', buttons: down ? 1 : 0,
  ...(type === 'mouseMoved' ? {} : { clickCount: 1 }),
});
const key = async (letter, shift = false) => {
  const modifiers = 4 | (shift ? 8 : 0);
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: letter, code: 'Key' + letter.toUpperCase(), windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: letter, code: 'Key' + letter.toUpperCase(), windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Meta', code: 'MetaLeft', windowsVirtualKeyCode: 91, modifiers: 0 });
};
const save = async name => {
  const index = await evaluate('window.__backgroundMaskSaves.length');
  await key('s');
  for (let attempt = 0; attempt < 50; attempt++) {
    const size = await evaluate(`window.__backgroundMaskSaves[${index}]?.contents.length||0`);
    if (size) {
      assert(size <= 128 * 1024 * 1024, `Save exceeds project limit at ${name}`);
      let source = '';
      for (let offset = 0; offset < size; offset += 65536) {
        source += await evaluate(`window.__backgroundMaskSaves[${index}].contents.slice(${offset},${offset + 65536})`);
      }
      await writeFile(path.join(out, `${name}.obrush`), source);
      const document = JSON.parse(source);
      const masks = document.mesh?.masks ?? document.scene?.objects?.find(object => object.id === document.scene.active_id)?.state?.mesh?.masks;
      assert(Array.isArray(masks), `No active saved mask array at ${name}`);
      return { sha256: hash(source), bytes: Buffer.byteLength(source), masks };
    }
    await wait(100);
  }
  throw new Error(`Save timed out at ${name}`);
};
try {
  let endpoint;
  for (let attempt = 0; attempt < 150; attempt++) {
    try {
      const lines = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n');
      endpoint = `ws://127.0.0.1:${Number(lines[0])}${lines[1]}`;
      break;
    } catch { if (chrome.exitCode !== null) throw new Error(`Chrome exited ${chrome.exitCode}`); }
    await wait(100);
  }
  assert(endpoint, 'No owned Chrome CDP endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  socket.addEventListener('message', event => {
    const packet = JSON.parse(String(event.data));
    if (packet.id) {
      const request = pending.get(packet.id);
      if (request) {
        clearTimeout(request.timer); pending.delete(packet.id);
        packet.error ? request.reject(new Error(JSON.stringify(packet.error))) : request.resolve(packet.result);
      }
    } else if (packet.method === 'Runtime.exceptionThrown') {
      result.exceptions.push(packet.params.exceptionDetails);
    } else if (packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error') {
      result.console_errors.push(packet.params.args.map(argument => argument.value || argument.description).join(' '));
    }
  });
  result.browser = await send('Browser.getVersion', {}, null);
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime']) await send(`${domain}.enable`);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Target.activateTarget', { targetId: target.targetId }, null);
  await send('Page.navigate', { url });
  for (let attempt = 0; attempt < 150; attempt++) {
    const ready = reference
      ? await evaluate('!!window.__sculptglReferenceInstance?.getMesh?.()')
      : await evaluate('!document.getElementById("loading")&&typeof window.obrushBrushMetrics==="function"');
    if (ready) break;
    await wait(100);
  }
  assert(reference ? await evaluate('!!window.__sculptglReferenceInstance?.getMesh?.()') : await evaluate('!document.getElementById("loading")'), 'Studio startup timeout');
  if (!reference) {
    await evaluate('window.__backgroundMaskSaves=[];window.obrushDownload=(name,contents)=>window.__backgroundMaskSaves.push({name,contents});document.getElementById("obrush_canvas").focus()');
    result.wasm_sha256 = hash(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm')));
  } else {
    result.bundle_sha256 = hash(await readFile(path.join(web, 'sculptgl.js')));
  }
  let bounds;
  for (let attempt = 0; attempt < 100; attempt++) {
    bounds = reference
      ? await evaluate('(()=>{let r=document.getElementById("canvas").getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]})()')
      : (await evaluate('window.obrushBrushMetrics()')).camera.viewport;
    if (bounds[2] - bounds[0] > 100 && bounds[3] - bounds[1] > 100) break;
    await wait(100);
  }
  const [left, top, right, bottom] = bounds;
  result.viewport = bounds;
  if (reference) result.viewport_diagnostic = await evaluate('(()=>{let v=document.getElementById("viewport"),c=document.getElementById("canvas");return {inner:[innerWidth,innerHeight],viewport_style:v.getAttribute("style"),viewport_rect:v.getBoundingClientRect().toJSON(),canvas_size:[c.width,c.height],ready:document.readyState}})()');
  const x = Math.floor(right - 35), y = Math.floor(top + 35);
  assert(x > left && y < bottom, 'No background viewport corner');
  result.point = [x, y];
  await mouse('mouseMoved', x, y);
  if (reference) {
    const mask = async () => evaluate('(()=>{const m=window.__sculptglReferenceInstance.getMesh(), a=m.getMaterials(), n=m.getNbVertices();let zero=0,one=0,other=0;for(let i=0;i<n;i++){const v=a[i*3+2];if(v===0)zero++;else if(v===1)one++;else other++;}return {vertices:n,zero,one,other}})()');
    result.initial = await mask();
    assert(result.initial.one === result.initial.vertices, 'Pinned source did not start unmasked');
    await mouse('mousePressed', x, y, true); await wait(80); await mouse('mouseReleased', x, y); await wait(150);
    result.inverted = await mask();
    assert(result.inverted.zero === result.initial.vertices, 'Pinned source Ctrl background click did not invert');
    await mouse('mousePressed', x, y, true); await mouse('mouseMoved', x - 20, y + 12, true); await wait(80); await mouse('mouseReleased', x - 20, y + 12); await wait(150);
    result.cleared = await mask();
    assert(result.cleared.one === result.initial.vertices, 'Pinned source Ctrl background drag did not clear');
    result.cases.push({ name: 'ctrl-background-click', action: 'invert', all_mask_influence_one: true }, { name: 'ctrl-background-drag', action: 'clear', all_mask_influence_zero: true });
  } else {
    result.initial = await save('initial');
    assert(result.initial.masks.every(value => value === 0), 'Fresh mesh is not unmasked');
    await mouse('mousePressed', x, y, true); await wait(80); await mouse('mouseReleased', x, y); await wait(150);
    result.inverted = await save('inverted');
    assert(result.inverted.masks.every(value => value === 1), 'Ctrl background click did not invert every active mask');
    result.cases.push({ name: 'ctrl-background-click', action: 'invert', all_masks_one: true });
    await mouse('mousePressed', x, y, true); await mouse('mouseMoved', x - 20, y + 12, true); await wait(80); await mouse('mouseReleased', x - 20, y + 12); await wait(150);
    result.cleared = await save('cleared');
    assert(result.cleared.masks.every(value => value === 0), 'Ctrl background drag did not clear every active mask');
    result.cases.push({ name: 'ctrl-background-drag', action: 'clear', all_masks_zero: true });
    await key('z'); await wait(150);
    result.undo = await save('undo');
    assert(result.undo.sha256 === result.inverted.sha256, 'Undo did not recover exact inverted project');
    await key('z', true); await wait(150);
    result.redo = await save('redo');
    assert(result.redo.sha256 === result.cleared.sha256, 'Redo did not recover exact cleared project');
    await mouse('mousePressed', x, y, true);
    await wait(50);
    result.saved_during_held_contact = await save('held-contact-save');
    await mouse('mouseReleased', x, y);
    await wait(150);
    result.after_command_release = await save('after-command-release');
    assert(result.saved_during_held_contact.sha256 === result.cleared.sha256 && result.after_command_release.sha256 === result.cleared.sha256, 'A completed Save command did not cancel the held background gesture');
    result.cases.push({ name: 'command-cancels-background-contact', action: 'save while held; release is quiet', project_exact: true });
  }
  assert(result.exceptions.length === 0 && result.console_errors.length === 0, 'Browser error was recorded');
  result.status = 'passed';
} catch (error) {
  result.status = 'failed'; result.error = String(error.stack || error);
  process.exitCode = 1;
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  socket?.close();
  chrome.kill('SIGTERM');
  await new Promise(resolve => server.close(resolve));
  console.log(JSON.stringify({ status: result.status, out, error: result.error, wasm_sha256: result.wasm_sha256, cases: result.cases }));
}
