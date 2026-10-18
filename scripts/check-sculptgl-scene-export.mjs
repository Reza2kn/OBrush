// Actual browser scoped scene export, building on the Source Add workflow.
// This scoped check is not complete SculptGL interchange parity.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const referenceWeb = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-scene-export-browser/' + randomUUID());
const profile = path.join(out, 'chrome-profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads, { recursive: true });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const sha = data => createHash('sha256').update(data).digest('hex');
const requireTrue = (condition, message) => { if (!condition) throw Error(message); };
const result = { scope: 'Source profile normal browser selected scene OBJ/PLY/STL export after multi-object OBJ Add', cases: [], files: [], errors: [] };
let server, chrome, socket, session, next = 0;
const waiting = new Map();
const downloadsSeen = [];
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
async function state() { return evaluate('typeof window.obrushBrushMetrics === "function" ? window.obrushBrushMetrics() : null'); }
async function until(predicate, description) {
  for (let i = 0; i < 150; i++) {
    const value = await predicate();
    if (value) return value;
    await pause(100);
  }
  throw Error('Timed out: ' + description);
}
async function key(letter, modifiers = 2) {
  const code = 'Key' + letter.toUpperCase();
  const keyValue = letter.toLowerCase();
  const windowsVirtualKeyCode = letter.toUpperCase().charCodeAt(0);
  await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: keyValue, code, windowsVirtualKeyCode, modifiers });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: keyValue, code, windowsVirtualKeyCode, modifiers });
}
async function addFile(filename, content, expectedCount) {
  const file = path.join(out, filename);
  await writeFile(file, content);
  result.files.push({ filename, sha256: sha(content), bytes: content.length });
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o');
  await until(async () => choosers.length > chooserCount, 'Source Add file chooser');
  const request = (await state()).source_sgl_import?.pending?.id;
  requireTrue(request, 'Source Add request was not retained for the chosen file');
  const dom = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: dom.nodeId, selector: '#source-sgl-file' })).nodeId;
  requireTrue(node, 'Source Add file input missing');
  await send('DOM.setFileInputFiles', { nodeId: node, files: [file] });
  const after = await until(async () => {
    const current = await state();
    return !current.source_sgl_import?.pending && current.subtools?.length === expectedCount ? current : null;
  }, 'Source Add installation');
  result.cases.push({ filename, request, part_count: after.subtools.length, active_id: after.active_subtool_id, report: after.import_report, options: after.source_sgl_import.options_report, revision: after.document_revision });
  return after;
}
async function dropFiles(entries) {
  const payload=entries.map(([name,bytes])=>({name,base64:Buffer.from(bytes).toString('base64')}));
  return evaluate(`(()=>{const transfer=new DataTransfer();for(const item of ${JSON.stringify(payload)}){const binary=atob(item.base64);const bytes=Uint8Array.from(binary,ch=>ch.charCodeAt(0));transfer.items.add(new File([bytes],item.name));}const over=new DragEvent('dragover',{dataTransfer:transfer,bubbles:true,cancelable:true});const dropped=new DragEvent('drop',{dataTransfer:transfer,bubbles:true,cancelable:true});window.dispatchEvent(over);window.dispatchEvent(dropped);return {over_prevented:over.defaultPrevented,drop_prevented:dropped.defaultPrevented,file_count:transfer.files.length};})()`);
}
async function save(label) {
  const previous = downloadsSeen.length;
  // Visible top-bar Save project button in the frozen 1600x1000 viewport.
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: 345, y: 26, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 345, y: 26, button: 'left', clickCount: 1 });
  const event = await until(async () => downloadsSeen.slice(previous).find(item => item.state === 'completed'), 'project download');
  const bytes = await readFile(path.join(downloads, event.guid));
  const document = JSON.parse(bytes.toString('utf8'));
  result.cases.push({ saved: label, sha256: sha(bytes), bytes: bytes.length, parts: document.scene?.objects?.length ?? null });
  return { bytes, document, hash: sha(bytes) };
}
async function clickControl(key, modifiers = 0) {
  for (let attempt = 0; attempt < 25; attempt++) {
    const metrics = await state();
    const rect = metrics?.layer_controls?.[key] || metrics?.scene_controls?.[key];
    requireTrue(rect, 'Missing rendered control ' + key);
    const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
    if (y >= 80 && y <= 945) {
      requireTrue(metrics.layer_controls_enabled?.[key] !== false && metrics.scene_controls_enabled?.[key] !== false, 'Disabled rendered control ' + key);
      await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1, modifiers });
      await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1, modifiers });
      await pause(100);
      return;
    }
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: Math.max(25, Math.min(250, x)), y: 700, deltaX: 0, deltaY: y > 945 ? 350 : -350 });
    await pause(120);
  }
  throw Error('Rendered control stayed outside viewport: ' + key);
}
async function exported(label, extension) {
  const previous = downloadsSeen.length;
  await clickControl('file.export_mesh');
  const event = await until(async () => downloadsSeen.slice(previous).find(item => item.state === 'completed'), label + ' download');
  requireTrue(path.extname(event.suggestedFilename) === '.' + extension, label + ' extension');
  const bytes = await readFile(path.join(downloads, event.guid));
  return {label, bytes, hash: sha(bytes), name: event.suggestedFilename};
}
const obj = Buffer.from('v 1 2 3 0.5 0.25 0.75\nv 3 2 3 0.5 0.25 0.75\nv 1 6 3 0.5 0.25 0.75\nv 1 2 9 0.5 0.25 0.75\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n');
const ply = Buffer.from('ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n2 0 0\n0 2 0\n3 0 1 2\n');
const stl = Buffer.from('solid one\nfacet normal 0 0 1\n outer loop\n vertex 0 0 0\n vertex 2 0 0\n vertex 0 2 0\n endloop\nendfacet\nendsolid one\n');
const emptySgl = Buffer.alloc(8); // version 0, zero objects, little-endian.
const multiObj = Buffer.from('o Left\nv 0 0 0 1 0 0\nv 1 0 0 1 0 0\nv 0 1 0 1 0 0\nf 1 2 3\no Right\nv 4 0 0 0 0 1\nv 5 0 0 0 0 1\nv 4 1 0 0 0 1\nf 4 5 6\n');
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname.replace(/\/$/, '/index.html');
      if (pathname === '/source-tetra.obj') {
        response.setHeader('Content-Type', 'text/plain'); response.end(obj); return;
      }
      const reference = pathname.startsWith('/reference/');
      const root = reference ? referenceWeb : web;
      const localPath = reference ? pathname.slice('/reference'.length) : pathname;
      const file = path.resolve(root, '.' + localPath);
      requireTrue(file.startsWith(root + path.sep), 'outside web root');
      const bytes = await readFile(file);
      response.setHeader('Cache-Control', 'no-store');
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const url = `http://127.0.0.1:${server.address().port}/?obrushprofile=sculptgl&scalecenter=1`;
  result.url = url;
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new', `--user-data-dir=${profile}`, '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', chunk => { stderr += chunk; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'Chrome CDP endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', message => {
    const item = JSON.parse(String(message.data));
    if (item.id) {
      const pending = waiting.get(item.id);
      if (!pending) return;
      waiting.delete(item.id); clearTimeout(pending.timer);
      item.error ? pending.reject(Error(JSON.stringify(item.error))) : pending.resolve(item.result);
    } else if (item.method === 'Browser.downloadWillBegin') downloadsSeen.push({ ...item.params, state: 'begin' });
    else if (item.method === 'Browser.downloadProgress') {
      const target = downloadsSeen.find(entry => entry.guid === item.params.guid);
      if (target) Object.assign(target, item.params);
    } else if (item.method === 'Runtime.exceptionThrown') result.errors.push(item.params.exceptionDetails.text);
    else if (item.method === 'Runtime.consoleAPICalled' && item.params.type === 'error') result.errors.push(item.params.args.map(arg => arg.value || arg.description).join(' '));
    else if (item.method === 'Page.fileChooserOpened') choosers.push(item.params);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(domain + '.enable');
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloads, eventsEnabled: true }, null);
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url });
  const initial = await until(async () => { const data = await state(); return data?.subtools?.length === 1 ? data : null; }, 'OBRush startup');
  requireTrue(initial.source_sgl_import?.options?.scale_center === true, 'Source scale-center startup option missing');
  await addFile('independent-tetra.obj', obj, 2);
  const afterObj = await save('after-obj');
  requireTrue(afterObj.document.scene?.objects?.length === 2, 'OBJ Add did not persist two parts');
  const expectedScale = 100 / Math.hypot(2, 4, 6);
  const objPose = afterObj.document.scene_v7?.objects?.find(entry => entry.id === 2)?.affine;
  requireTrue(objPose && Math.abs(objPose.local_to_world[0] - expectedScale) < 1e-6, 'OBJ source normalization scale differs from independent diagonal');
  for (const [axis, center] of [2, 4, 6].entries()) requireTrue(Math.abs(objPose.local_to_world[12 + axis] + expectedScale * center) < 1e-5, 'OBJ source normalization center differs');
  requireTrue(JSON.stringify(afterObj.document.mesh.positions) === JSON.stringify([[1,2,3],[3,2,3],[1,6,3],[1,2,9]]), 'OBJ Add baked or reordered local points');
  requireTrue(afterObj.document.mesh.colors.every(row => JSON.stringify(row) === JSON.stringify([0.5,0.25,0.75])), 'OBJ explicit colors changed with sRGB option enabled');
  await key('z');
  const undone = await save('undo-obj');
  requireTrue((undone.document.scene?.objects?.length ?? 1) === 1 && undone.hash !== afterObj.hash, 'OBJ Add Undo did not remove part');
  await key('y');
  const redone = await save('redo-obj');
  requireTrue(redone.hash === afterObj.hash, 'OBJ Add Redo changed saved project');
  await addFile('independent-triangle.ply', ply, 3);
  await addFile('independent-triangle.stl', stl, 4);
  const afterMeshes = await save('after-three-mesh-formats');
  requireTrue(afterMeshes.document.scene?.objects?.length === 4, 'Source Add did not retain all mesh parts');
  requireTrue(afterMeshes.document.scene_v7?.objects?.length === 3, 'PLY/STL source poses were not saved');
  await addFile('empty.sgl', emptySgl, 4);
  const afterSgl = await save('after-empty-sgl');
  requireTrue(afterSgl.hash === afterMeshes.hash, 'Empty SGL Add changed document');
  await addFile('two-objects.obj', multiObj, 6);
  const afterMulti = await save('after-multi-obj');
  requireTrue(afterMulti.document.scene?.objects?.length === 6, 'Multi-object OBJ did not retain two parts');
  const multiPoses = afterMulti.document.scene_v7?.objects?.filter(entry => entry.id >= 5);
  requireTrue(multiPoses?.length === 2, 'Multi-object OBJ poses missing');
  requireTrue(JSON.stringify(multiPoses[0].affine.local_to_world) === JSON.stringify(multiPoses[1].affine.local_to_world), 'Multi-object OBJ did not share a group normalization matrix');
  const selectedBefore = (await state()).selected_subtool_ids;
  requireTrue(Array.isArray(selectedBefore), 'Selected-part browser metric missing');
  if (JSON.stringify(selectedBefore) !== JSON.stringify([5,6])) {
    await clickControl('scene.select.5');
    await clickControl('scene.select.6', 2);
  }
  requireTrue(JSON.stringify((await state()).selected_subtool_ids) === JSON.stringify([5,6]), 'Could not select both imported source parts through the visible scene controls');
  await clickControl('file.allow_attribute_loss');
  await clickControl('file.export_sgl.selected_only');
  requireTrue((await state()).sgl_export.selected_only, 'Selected-only export control did not activate');
  const beforeSceneExports = await save('before-scene-exports');
  const selections = [
    ['obj','OBJ','obj'], ['stl_ascii','STL text','stl'], ['stl_binary','STL binary + color','stl'],
    ['ply_ascii','PLY text','ply'], ['ply_binary','PLY binary','ply'],
  ];
  const sceneExports = [];
  for (const [key, name, extension] of selections) {
    await clickControl('file.export_format');
    await clickControl('file.export_format.' + key);
    requireTrue((await state()).export_format === name, 'Normal format combo did not select ' + name);
    const file = await exported('selected-' + key, extension);
    const inspection = {format:name, bytes:file.bytes.length, sha256:file.hash};
    if (key === 'obj') {
      const text = file.bytes.toString('utf8');
      const vertices = text.split('\n').filter(line => line.startsWith('v ')).map(line => line.split(/\s+/).slice(1,4).map(Number));
      inspection.objects = text.split('\n').filter(line => line.startsWith('o '));
      inspection.vertices = vertices;
      inspection.faces = text.split('\n').filter(line => line.startsWith('f ')).length;
      requireTrue(inspection.objects.length === 2 && vertices.length === 6 && inspection.faces === 2, 'Selected OBJ scene scope differs');
      const matrix = multiPoses[0].affine.local_to_world;
      for (const [i, local] of [[0,0,0],[1,0,0],[0,1,0],[4,0,0],[5,0,0],[4,1,0]].entries()) {
        const world = [0,1,2].map(axis => matrix[12+axis] + matrix[axis] * local[0] + matrix[4+axis] * local[1] + matrix[8+axis] * local[2]);
        requireTrue(world.every((value,axis) => Math.abs(value-vertices[i][axis]) < 0.00001), 'Selected OBJ did not bake source world pose at vertex ' + i);
      }
    } else if (key === 'stl_binary') {
      inspection.faces = file.bytes.readUInt32LE(80);
      requireTrue(inspection.faces === 2 && file.bytes.length === 184, 'Selected binary STL count/size differs');
    } else if (key === 'stl_ascii') {
      inspection.faces = file.bytes.toString('utf8').split('facet normal').length - 1;
      requireTrue(inspection.faces === 2, 'Selected text STL face count differs');
    } else {
      const header = file.bytes.toString('utf8', 0, Math.min(file.bytes.length, 1024)).split('end_header')[0];
      requireTrue(header.includes('element vertex 6') && header.includes('element face 2'), 'Selected PLY counts differ');
    }
    sceneExports.push(inspection);
  }
  const afterSceneExports = await save('after-scene-exports');
  requireTrue(afterSceneExports.hash === beforeSceneExports.hash, 'Scene mesh exports mutated project or history');
  result.cases.push({selected_scene_exports:sceneExports,selected_ids:[5,6],before_sha256:beforeSceneExports.hash,after_sha256:afterSceneExports.hash,project_exact:true});
  requireTrue(result.errors.length === 0, 'Browser runtime exception');
  result.status = 'passed scoped browser checks';
} catch (error) {
  result.status = 'failed';
  result.failure = String(error.stack || error);
  try { result.failure_state = await state(); result.failure_page = await evaluate('({url:location.href,loading:document.getElementById("loading")?.innerText,body:document.body.innerText.slice(0,500)})'); } catch {}
  try { const shot = await send('Page.captureScreenshot', { format: 'png' }); await writeFile(path.join(out, 'failure.png'), Buffer.from(shot.data, 'base64')); } catch {}
  process.exitCode = 1;
} finally {
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  try { socket?.close(); } catch {}
  if (chrome && chrome.exitCode === null) chrome.kill('SIGTERM');
  if (server) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
  result.finished_at = new Date().toISOString();
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ status: result.status, failure: result.failure, result: path.join(out, 'result.json') }));
}
