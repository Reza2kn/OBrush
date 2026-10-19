// Actual browser SculptGL-style OBJ color switches and scoped scene export.
// This checks the normal OBrush UI and retained pinned source encoder policy.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const referenceWeb = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-obj-color-browser/' + randomUUID());
const profile = path.join(out, 'chrome-profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads, { recursive: true });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const sha = data => createHash('sha256').update(data).digest('hex');
const requireTrue = (condition, message) => { if (!condition) throw Error(message); };
const result = { scope: 'Source profile normal browser all four OBJ color-switch combinations after multi-object OBJ Add', cases: [], files: [], errors: [] };
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
async function until(predicate, description, attempts = 150) {
  for (let i = 0; i < attempts; i++) {
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
async function addFile(filename, content, expectedCount, attempts = 150) {
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
  }, 'Source Add installation', attempts);
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
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: Math.max(25, Math.min(1575, x)), y: 700, deltaX: 0, deltaY: y > 945 ? 350 : -350 });
    await pause(120);
  }
  throw Error('Rendered control stayed outside viewport: ' + key);
}
async function scrollInspectorToBottom() {
  const x=1582;
  await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y:150,button:'none',buttons:0});
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y:150,button:'left',buttons:1,clickCount:1});
  for(const y of [300,500,700,875]){
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y,button:'none',buttons:1});
    await pause(60);
  }
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y:875,button:'left',buttons:0,clickCount:1});
  await pause(150);
}
async function exported(label, extension) {
  const previous = downloadsSeen.length;
  await clickControl('file.export_mesh');
  const event = await until(async () => downloadsSeen.slice(previous).find(item => item.state === 'completed'), label + ' download');
  requireTrue(path.extname(event.suggestedFilename) === '.' + extension, label + ' extension');
  const bytes = await readFile(path.join(downloads, event.guid));
  return {label, bytes, hash: sha(bytes), name: event.suggestedFilename};
}
function objParts(text) {
  return text.split(/^o mesh_\d+\n/m).slice(1).map(part => {
    const lines=part.split('\n');
    return {
      positions:lines.filter(line=>line.startsWith('v ')).map(line=>line.split(/\s+/).slice(1,4).map(Number)),
      mrgb:lines.filter(line=>line.startsWith('#MRGB ')).map(line=>line.slice(6)),
      mat:lines.filter(line=>line.startsWith('#MAT ')).map(line=>line.slice(5)),
    };
  });
}
function binaryPly(bytes) {
  const end=bytes.indexOf(Buffer.from('end_header\n'));
  requireTrue(end>=0,'PLY header terminator missing');
  const header=bytes.toString('ascii',0,end+11);
  requireTrue(header.includes('format binary_little_endian 1.0') && header.includes('property uchar red\nproperty uchar green\nproperty uchar blue'), 'Pinned binary PLY schema differs');
  const vertexCount=Number(header.match(/element vertex (\d+)/)?.[1]);
  const faceCount=Number(header.match(/element face (\d+)/)?.[1]);
  requireTrue(Number.isSafeInteger(vertexCount) && Number.isSafeInteger(faceCount),'PLY counts missing');
  let at=end+11;
  const vertices=[];
  for(let i=0;i<vertexCount;i++){
    vertices.push({position:[bytes.readFloatLE(at),bytes.readFloatLE(at+4),bytes.readFloatLE(at+8)],color:[...bytes.subarray(at+12,at+15)]});
    at+=15;
  }
  const faces=[];
  for(let i=0;i<faceCount;i++){
    const count=bytes[at++];
    requireTrue(count===3||count===4,'PLY face is not tri or quad');
    const indices=[];
    for(let j=0;j<count;j++){indices.push(bytes.readUInt32LE(at));at+=4;}
    faces.push(indices);
  }
  const faceBytes=faces.reduce((sum,face)=>sum+1+4*face.length,0);
  const expectedPadding=faceBytes+48*faces.filter(face=>face.length===4).length;
  requireTrue(bytes.length-at===expectedPadding && bytes.subarray(at).every(value=>value===0),'Pinned SculptGL face padding differs');
  return {header,vertices,faces,face_padding_bytes:expectedPadding};
}
const obj = Buffer.from('v 1 2 3 0.5 0.25 0.75\nv 3 2 3 0.5 0.25 0.75\nv 1 6 3 0.5 0.25 0.75\nv 1 2 9 0.5 0.25 0.75\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n');
const ply = Buffer.from('ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n2 0 0\n0 2 0\n3 0 1 2\n');
const stl = Buffer.from('solid one\nfacet normal 0 0 1\n outer loop\n vertex 0 0 0\n vertex 2 0 0\n vertex 0 2 0\n endloop\nendfacet\nendsolid one\n');
const emptySgl = Buffer.alloc(8); // version 0, zero objects, little-endian.
const multiObj = Buffer.from('o Left\nv 0 0 0 1 0 0\nv 1 0 0 1 0 0\nv 0 1 0 1 0 0\n#MAT 40208080c040ff0000\nf 1 2 3\no Right\nv 4 0 0 0 0 1\nv 5 0 0 0 0 1\nv 4 1 0 0 0 1\n#MAT 0033ff7f7f7f20e010\nf 4 5 6\n');
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
  let selectedPlyBinary, selectedStlBinary;
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
      selectedStlBinary = file.bytes;
    } else if (key === 'stl_ascii') {
      inspection.faces = file.bytes.toString('utf8').split('facet normal').length - 1;
      requireTrue(inspection.faces === 2, 'Selected text STL face count differs');
    } else {
      const header = file.bytes.toString('utf8', 0, Math.min(file.bytes.length, 1024)).split('end_header')[0];
      requireTrue(header.includes('element vertex 6') && header.includes('element face 2'), 'Selected PLY counts differ');
      if(key==='ply_binary') selectedPlyBinary=binaryPly(file.bytes);
    }
    sceneExports.push(inspection);
  }
  const afterSceneExports = await save('after-scene-exports');
  requireTrue(afterSceneExports.hash === beforeSceneExports.hash, 'Scene mesh exports mutated project or history');
  result.cases.push({selected_scene_exports:sceneExports,selected_ids:[5,6],before_sha256:beforeSceneExports.hash,after_sha256:afterSceneExports.hash,project_exact:true});
  const colorOptions = [];
  for (const zbrush of [true, false]) {
    for (const appendRgb of [false, true]) {
      const current = (await state()).obj_export_options;
      requireTrue(current && typeof current.zbrush === 'boolean', 'Source OBJ option metrics missing');
      if (current.zbrush !== zbrush) await clickControl('file.export_obj.zbrush');
      if ((await state()).obj_export_options.append_rgb !== appendRgb) await clickControl('file.export_obj.append_rgb');
      await clickControl('file.export_format');
      await clickControl('file.export_format.obj');
      const file = await exported(`color-z${Number(zbrush)}-a${Number(appendRgb)}`, 'obj');
      const exportReport=(await state()).last_scene_geometry_export;
      requireTrue(exportReport?.parts===2 && exportReport.omitted_color_vertices===(zbrush||appendRgb?0:6), 'Visible OBJ export loss report differs');
      const lines = file.bytes.toString('utf8').split('\n');
      const vertices = lines.filter(line => line.startsWith('v '));
      const mrgb = lines.filter(line => line.startsWith('#MRGB '));
      const mat = lines.filter(line => line.startsWith('#MAT '));
      requireTrue(vertices.length === 6, 'OBJ color option changed vertex scope');
      requireTrue(mrgb.length === (zbrush ? 2 : 0) && mat.length === (zbrush ? 2 : 0), 'OBJ color option did not control both source-style streams');
      requireTrue(vertices.every(line => line.split(/\s+/).length === (appendRgb ? 7 : 4)), 'OBJ inline RGB switch differs');
      if (zbrush) {
        requireTrue(mrgb[0].slice(6) === 'ffff0000'.repeat(3) && mrgb[1].slice(6) === 'ff0000ff'.repeat(3), 'OBJ #MRGB source colors differ');
        requireTrue(JSON.stringify(mat.map(line=>line.slice(5)))===JSON.stringify(['40208080c040ff0000','0033ff7f7f7f20e010']), 'OBJ #MAT roughness/metal/unmasked bytes differ');
      }
      colorOptions.push({zbrush,append_rgb:appendRgb,sha256:file.hash,bytes:file.bytes.length,mrgb_lines:mrgb.length,mat_lines:mat.length,inline_components:vertices[0].split(/\s+/).length,report:exportReport,parts:objParts(file.bytes.toString('utf8'))});
    }
  }
  const afterColorOptions = await save('after-obj-color-options');
  requireTrue(afterColorOptions.hash === beforeSceneExports.hash, 'OBJ color choices or downloads mutated the project');
  result.cases.push({obj_color_options:colorOptions,project_exact:true,before_sha256:beforeSceneExports.hash,after_sha256:afterColorOptions.hash});
  const referenceUrl = `http://127.0.0.1:${server.address().port}/reference/?scalecenter=1`;
  await send('Page.navigate', {url:referenceUrl});
  const referenceBefore = await until(async()=>evaluate('window.__sculptglReferenceInstance?.getMeshes()?.length ?? null'),'pinned SculptGL startup');
  const referenceDom = (await send('DOM.getDocument')).root;
  const referenceNode = (await send('DOM.querySelector',{nodeId:referenceDom.nodeId,selector:'#fileopen'})).nodeId;
  requireTrue(referenceNode,'Pinned SculptGL file input missing');
  await send('DOM.setFileInputFiles',{nodeId:referenceNode,files:[path.join(out,'two-objects.obj')]});
  const sourceState = await until(async()=>evaluate(`(()=>{const app=window.__sculptglReferenceInstance;if(!app||app.getMeshes().length!==${referenceBefore}+2)return null;return {total:app.getMeshes().length,selected:app.getSelectedMeshes().length,gui_files:!!app._gui?._ctrlFiles};})()`),'pinned SculptGL two-object import');
  requireTrue(sourceState.gui_files,'Pinned SculptGL GuiFiles controller unavailable');
  const sourceOptions = [];
  const oldPlyCount=downloadsSeen.length;
  await evaluate('(()=>{const files=window.__sculptglReferenceInstance._gui._ctrlFiles;files._exportAll=true;files.saveFileAsPLY();return true;})()');
  const sourcePlyEvent=await until(async()=>downloadsSeen.slice(oldPlyCount).find(item=>item.state==='completed'),'pinned SculptGL PLY download');
  const sourcePlyBytes=await readFile(path.join(downloads,sourcePlyEvent.guid));
  const sourcePly=binaryPly(sourcePlyBytes);
  requireTrue(selectedPlyBinary?.vertices.length===6 && selectedPlyBinary.faces.length===2,'OBRush scoped PLY payload missing');
  const vertexOffset=sourcePly.vertices.length-6;
  requireTrue(vertexOffset>0 && sourcePly.faces.length>=2,'Pinned PLY imported-object tail missing');
  let maximumPlyPositionError=0;
  for(let i=0;i<6;i++){
    const source=sourcePly.vertices[vertexOffset+i], ours=selectedPlyBinary.vertices[i];
    requireTrue(JSON.stringify(source.color)===JSON.stringify(ours.color),'Paired PLY RGB bytes differ at vertex '+i);
    for(let axis=0;axis<3;axis++) maximumPlyPositionError=Math.max(maximumPlyPositionError,Math.abs(source.position[axis]-ours.position[axis]));
  }
  requireTrue(maximumPlyPositionError<1e-4,'Paired PLY world positions differ');
  requireTrue(JSON.stringify(sourcePly.faces.slice(-2).map(face=>face.map(index=>index-vertexOffset)))===JSON.stringify(selectedPlyBinary.faces),'Paired PLY imported-object face records differ');
  result.cases.push({pinned_sculptgl_binary_ply:{reference_sha256:sha(sourcePlyBytes),reference_bytes:sourcePlyBytes.length,obrush_selected_sha256:sceneExports.find(item=>item.format==='PLY binary').sha256,selected_vertices:6,selected_faces:2,matched_rgb_bytes:true,matched_face_records:true,maximum_world_position_component_error:maximumPlyPositionError,obrush_face_padding_bytes:selectedPlyBinary.face_padding_bytes,reference_face_padding_bytes:sourcePly.face_padding_bytes}});
  const oldStlCount=downloadsSeen.length;
  await evaluate('(()=>{window.__sculptglReferenceInstance._gui._ctrlFiles.saveFileAsSTL();return true;})()');
  const sourceStlEvent=await until(async()=>downloadsSeen.slice(oldStlCount).find(item=>item.state==='completed'),'pinned SculptGL STL download');
  const sourceStlBytes=await readFile(path.join(downloads,sourceStlEvent.guid));
  const sourceStlFaces=sourceStlBytes.readUInt32LE(80);
  requireTrue(sourceStlBytes.length===84+sourceStlFaces*50 && sourceStlFaces>2,'Pinned SculptGL STL shape differs');
  const sourceTail=sourceStlBytes.subarray(sourceStlBytes.length-100);
  const oursTail=selectedStlBinary.subarray(84);
  let differingStlBytes=0;
  for(let i=0;i<100;i++) differingStlBytes+=Number(sourceTail[i]!==oursTail[i]);
  result.cases.push({pinned_sculptgl_binary_stl:{reference_sha256:sha(sourceStlBytes),reference_bytes:sourceStlBytes.length,reference_facets:sourceStlFaces,obrush_selected_sha256:sha(selectedStlBinary),selected_facets:2,differing_imported_object_facet_bytes:differingStlBytes,source_header_zero:sourceStlBytes.subarray(0,80).every(byte=>byte===0),obrush_header_zero:selectedStlBinary.subarray(0,80).every(byte=>byte===0)}});
  requireTrue(differingStlBytes===0,'Paired SculptGL STL imported-object facet bytes differ');
  for (const zbrush of [true,false]) {
    for (const appendRgb of [false,true]) {
      const oldCount=downloadsSeen.length;
      await evaluate(`(()=>{const files=window.__sculptglReferenceInstance._gui._ctrlFiles;files._exportAll=true;files._objColorZbrush=${zbrush};files._objColorAppended=${appendRgb};files.saveFileAsOBJ();return true;})()`);
      const event=await until(async()=>downloadsSeen.slice(oldCount).find(item=>item.state==='completed'),'pinned SculptGL OBJ download');
      const bytes=await readFile(path.join(downloads,event.guid));
      const lines=bytes.toString('utf8').split('\n');
      const objects=lines.filter(line=>line.startsWith('o '));
      const mrgb=lines.filter(line=>line.startsWith('#MRGB '));
      const mat=lines.filter(line=>line.startsWith('#MAT '));
      const vertices=lines.filter(line=>line.startsWith('v '));
      requireTrue(objects.length===sourceState.total,'Pinned SculptGL exported unexpected object count');
      requireTrue(mrgb.length>0===zbrush && mat.length>0===zbrush,'Pinned SculptGL source options did not control streams');
      requireTrue(vertices.every(line=>line.split(/\s+/).length===(appendRgb?7:4)),'Pinned SculptGL inline RGB switch differs');
      const match=colorOptions.find(item=>item.zbrush===zbrush&&item.append_rgb===appendRgb);
      requireTrue(match,'OBRush matching OBJ switch result missing');
      const tail=objParts(bytes.toString('utf8')).slice(-2);
      requireTrue(tail.length===2&&match.parts.length===2,'Paired OBJ object scope missing');
      let maximumWorldComponentError=0;
      for(let part=0;part<2;part++){
        requireTrue(JSON.stringify(tail[part].mrgb)===JSON.stringify(match.parts[part].mrgb),'Paired #MRGB object bytes differ');
        requireTrue(JSON.stringify(tail[part].mat)===JSON.stringify(match.parts[part].mat),'Paired #MAT object bytes differ');
        requireTrue(tail[part].positions.length===match.parts[part].positions.length,'Paired object vertex count differs');
        for(const point of tail[part].positions){
          const nearest=Math.min(...match.parts[part].positions.map(other=>Math.max(...point.map((value,axis)=>Math.abs(value-other[axis])))));
          maximumWorldComponentError=Math.max(maximumWorldComponentError,nearest);
        }
      }
      requireTrue(maximumWorldComponentError<1e-4,'Paired OBJ world vertex positions differ');
      sourceOptions.push({zbrush,append_rgb:appendRgb,bytes:bytes.length,sha256:sha(bytes),objects:objects.length,mrgb_lines:mrgb.length,mat_lines:mat.length,inline_components:vertices[0].split(/\s+/).length,obrush_sha256:match.sha256,matched_tail_mrgb_and_mat:true,maximum_nearest_world_component_error:maximumWorldComponentError});
    }
  }
  result.cases.push({pinned_sculptgl_obj_options:sourceOptions,source_state:sourceState,reference_url:referenceUrl,comparison_scope:'same fixture, four option combinations, source all-scene direct GuiFiles runtime download; output scene sets differ because OBrush selected two parts'});
  await send('Page.navigate',{url});
  await until(async()=>{const data=await state();return data?.subtools?.length===1?data:null;},'OBRush restart for reference PLY import');
  await scrollInspectorToBottom();
  await clickControl('file.source_sgl.scale_center');
  requireTrue((await state()).source_sgl_import.options.scale_center===false,'Source PLY return must retain unnormalized coordinates');
  await addFile('pinned-sculptgl-scene.ply',sourcePlyBytes,2,600);
  const importedSourcePly=await save('imported-pinned-ply');
  requireTrue(importedSourcePly.document.scene?.objects?.length===2,'Pinned PLY Add did not append one part');
  if(JSON.stringify((await state()).selected_subtool_ids)!==JSON.stringify([2])) await clickControl('scene.select.2');
  requireTrue(JSON.stringify((await state()).selected_subtool_ids)===JSON.stringify([2]),'Imported PLY selection is not one part');
  await clickControl('file.allow_attribute_loss');
  await clickControl('file.export_sgl.selected_only');
  await clickControl('file.export_format');
  await clickControl('file.export_format.ply_binary');
  const returnedPly=await exported('return-reference-ply','ply');
  requireTrue(returnedPly.hash===sha(sourcePlyBytes),'OBRush browser PLY return differs from pinned SculptGL bytes');
  await key('z');
  const undoneSourcePly=await save('undo-pinned-ply');
  requireTrue(undoneSourcePly.hash!==importedSourcePly.hash,'Pinned PLY Add Undo did not change project');
  await key('y');
  const redoneSourcePly=await save('redo-pinned-ply');
  requireTrue(redoneSourcePly.hash===importedSourcePly.hash,'Pinned PLY Add Redo changed project');
  result.cases.push({reference_ply_return:{input_sha256:sha(sourcePlyBytes),input_bytes:sourcePlyBytes.length,returned_sha256:returnedPly.hash,returned_exact:true,imported_project_sha256:importedSourcePly.hash,redo_exact:true,undo_project_sha256:undoneSourcePly.hash}});
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
