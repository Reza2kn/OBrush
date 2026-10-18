// Actual browser Source File Add smoke: keyboard, file input, Save and history.
// This scoped check is not complete SculptGL importer parity.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const web = path.resolve(process.argv[2] || 'web/dist');
const referenceWeb = path.resolve('.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/source-file-add-browser/' + randomUUID());
const profile = path.join(out, 'chrome-profile');
const downloads = path.join(out, 'downloads');
await mkdir(profile, { recursive: true });
await mkdir(downloads, { recursive: true });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const sha = data => createHash('sha256').update(data).digest('hex');
const requireTrue = (condition, message) => { if (!condition) throw Error(message); };
const result = { scope: 'Source profile normal browser Add via Ctrl+O, four file types, saved project and Undo/Redo', cases: [], files: [], errors: [] };
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
  await key('z');
  const undoneMulti = await save('undo-multi-obj');
  requireTrue(undoneMulti.hash === afterSgl.hash, 'Multi-object OBJ was not one undoable transaction');
  await key('y');
  const redoneMulti = await save('redo-multi-obj');
  requireTrue(redoneMulti.hash === afterMulti.hash, 'Multi-object OBJ redo changed saved project');
  const malformed = path.join(out, 'broken-second-object.obj');
  await writeFile(malformed, Buffer.from(multiObj.toString('utf8').replace('f 4 5 6', 'f 4 5 5')));
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const malformedChooserCount = choosers.length;
  await key('o');
  await until(async () => choosers.length > malformedChooserCount, 'malformed request chooser');
  const dom = (await send('DOM.getDocument')).root;
  const node = (await send('DOM.querySelector', { nodeId: dom.nodeId, selector: '#source-sgl-file' })).nodeId;
  await send('DOM.setFileInputFiles', { nodeId: node, files: [malformed] });
  await until(async () => !(await state()).source_sgl_import?.pending, 'malformed rejection');
  const afterBad = await save('after-malformed');
  requireTrue(afterBad.hash === afterMulti.hash, 'Malformed Source Add mutated project');
  // One real multiple-selection event, with a separate reference-style history
  // step per recognized file. The picker supplies the same independently
  // authored OBJ/PLY/STL bytes used in the single-file checks above.
  requireTrue(await evaluate("document.getElementById('source-sgl-file').multiple"), 'Source Add picker does not permit multiple files');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const batchChooserCount = choosers.length;
  await key('o');
  await until(async () => choosers.length > batchChooserCount, 'multi-file request chooser');
  await send('DOM.setFileInputFiles', { nodeId: node, files: [path.join(out, 'independent-tetra.obj'), path.join(out, 'independent-triangle.ply'), path.join(out, 'independent-triangle.stl')] });
  await until(async () => { const current=await state(); return !current.source_sgl_import?.pending && current.subtools?.length===9 ? current : null; }, 'multi-file Source Add');
  const afterBatch = await save('after-batch');
  requireTrue(afterBatch.document.scene?.objects?.length === 9, 'Source batch did not retain all three files');
  for(let i=0;i<3;i++)await key('z');
  const undoneBatch = await save('undo-batch-three');
  requireTrue(undoneBatch.hash === afterMulti.hash, 'Three Source batch Undo actions did not restore exact pre-batch project');
  for(let i=0;i<3;i++)await key('y');
  const redoneBatch = await save('redo-batch-three');
  requireTrue(redoneBatch.hash === afterBatch.hash, 'Three Source batch Redo actions changed saved project');
  result.cases.push({multi_file_picker:true,files:['independent-tetra.obj','independent-triangle.ply','independent-triangle.stl'],before_parts:6,after_parts:9,undo_steps:3,redo_steps:3,before_sha256:afterMulti.hash,after_sha256:afterBatch.hash});
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const mixedChooserCount=choosers.length;
  await key('o');
  await until(async()=>choosers.length>mixedChooserCount,'mixed-validity request chooser');
  await send('DOM.setFileInputFiles',{nodeId:node,files:[path.join(out,'independent-triangle.ply'),malformed,path.join(out,'independent-triangle.stl')]});
  await until(async()=>{const current=await state();return !current.source_sgl_import?.pending&&current.subtools?.length===11?current:null;},'mixed-validity Source Add');
  requireTrue((await evaluate("document.getElementById('notice').textContent")).includes('file 2'), 'Malformed batch member was not reported to the user');
  const afterMixed=await save('after-mixed-batch');
  requireTrue(afterMixed.document.scene?.objects?.length===11,'Valid files after a malformed batch member were not appended');
  for(let i=0;i<2;i++)await key('z');
  const undoneMixed=await save('undo-mixed-batch');
  requireTrue(undoneMixed.hash===afterBatch.hash,'Malformed batch member added history or changed pre-batch data');
  result.cases.push({mixed_validity_batch:true,files:['valid PLY','malformed OBJ','valid STL'],added_parts:2,undo_steps:2,after_sha256:afterMixed.hash,restored_sha256:undoneMixed.hash});
  const dropped=await dropFiles([['ignored.txt',Buffer.from('not a mesh')],['dropped-tetra.obj',obj]]);
  requireTrue(dropped.over_prevented&&dropped.drop_prevented&&dropped.file_count===2,'Source viewport did not accept dropped files');
  await until(async()=>{const current=await state();return current?.subtools?.length===10&&!current.source_sgl_import?.pending?current:null;},'dropped Source OBJ');
  const afterDrop=await save('after-drop');
  requireTrue(afterDrop.document.scene?.objects?.length===10,'Dropped OBJ did not append exactly one mesh');
  await key('z');
  const undoneDrop=await save('undo-drop');
  requireTrue(undoneDrop.hash===afterBatch.hash,'Dropped file Undo changed the previous project');
  result.cases.push({drop_workflow:true,ignored_extension:'ignored.txt',accepted:'dropped-tetra.obj',added_parts:1,undo_exact:true,after_sha256:afterDrop.hash});
  const modelUrl = `http://127.0.0.1:${server.address().port}/source-tetra.obj`;
  // The pinned source query parser intentionally keeps modelurl literal.
  const startupUrl = `http://127.0.0.1:${server.address().port}/?obrushprofile=sculptgl&scalecenter=1&modelurl=${modelUrl}`;
  await send('Page.navigate', { url: startupUrl });
  const startup = await until(async () => {
    const loadedUrl = await evaluate('location.href');
    const current = await state();
    return loadedUrl.includes('modelurl=') && current?.subtools?.length === 1 && current.import_report?.format === 'obj' ? current : null;
  }, 'Source profile startup OBJ URL');
  requireTrue(startup.source_sgl_import.options_report?.normalization?.diagonal > 7.48, 'Startup URL did not use shared source normalization');
  result.cases.push({ startup_url: startupUrl, part_count: startup.subtools.length, report: startup.import_report, options: startup.source_sgl_import.options_report });
  const ordinaryUrl=`http://127.0.0.1:${server.address().port}/`;
  await send('Page.navigate',{url:ordinaryUrl});
  await until(async()=>{const current=await state();return current?.subtools?.length===1?current:null;},'ordinary OBrush startup');
  const ordinaryDrop=await dropFiles([['dropped-tetra.obj',obj]]);
  requireTrue(ordinaryDrop.over_prevented&&ordinaryDrop.drop_prevented,'Ordinary-profile file drop could navigate away');
  await pause(150);
  requireTrue((await evaluate('location.href'))===ordinaryUrl&&(await state()).subtools?.length===1,'Ordinary-profile file drop changed the page or scene');
  result.cases.push({ordinary_profile_drop_preserved_page:true,drop_prevented:true,scene_parts:1});
  const referenceUrl = `http://127.0.0.1:${server.address().port}/reference/?scalecenter=1`;
  await send('Page.navigate', { url: referenceUrl });
  const beforeReference = await until(async () => evaluate('window.__sculptglReferenceInstance?.getMeshes()?.length ?? null'), 'pinned SculptGL startup');
  const referenceDom = (await send('DOM.getDocument')).root;
  const referenceNode = (await send('DOM.querySelector', { nodeId: referenceDom.nodeId, selector: '#fileopen' })).nodeId;
  await send('DOM.setFileInputFiles', { nodeId: referenceNode, files: [path.join(out, 'independent-tetra.obj')] });
  const reference = await until(async () => evaluate(`(()=>{const meshes=window.__sculptglReferenceInstance?.getMeshes();if(!meshes||meshes.length<=${beforeReference})return null;const m=meshes[meshes.length-1];return {points:Array.from(m.getVertices()),matrix:Array.from(m.getMatrix()),colors:Array.from(m.getColors()),mesh_count:meshes.length}})()`), 'pinned SculptGL OBJ import');
  const world = (flat, matrix) => {
    const points = [];
    for (let i = 0; i < flat.length; i += 3) {
      const x = flat[i], y = flat[i+1], z = flat[i+2];
      points.push([matrix[0]*x+matrix[4]*y+matrix[8]*z+matrix[12],matrix[1]*x+matrix[5]*y+matrix[9]*z+matrix[13],matrix[2]*x+matrix[6]*y+matrix[10]*z+matrix[14]]);
    }
    return points;
  };
  const sourceWorld = world(reference.points, reference.matrix);
  const obrushWorld = world(afterObj.document.mesh.positions.flat(), objPose.local_to_world);
  const differences = obrushWorld.map(p => Math.min(...sourceWorld.map(q => Math.max(...p.map((v,i)=>Math.abs(v-q[i]))))));
  const maxWorldDifference = Math.max(...differences);
  const sameMatrixWords = reference.matrix.length === objPose.local_to_world.length && reference.matrix.every((v,i) => Object.is(Math.fround(v), Math.fround(objPose.local_to_world[i])));
  const sameColorWords = reference.colors.length === afterObj.document.mesh.colors.length*3 && reference.colors.every((v,i) => Object.is(Math.fround(v), Math.fround(afterObj.document.mesh.colors[Math.floor(i/3)][i%3])));
  requireTrue(sameMatrixWords, 'Pinned SculptGL and OBrush OBJ affine words differ');
  requireTrue(sameColorWords, 'Pinned SculptGL and OBrush OBJ RGB words differ');
  requireTrue(Number.isFinite(maxWorldDifference) && maxWorldDifference < 1e-4, 'Pinned SculptGL and OBrush OBJ world positions differ');
  result.cases.push({ pinned_reference: referenceUrl, source_mesh_count: reference.mesh_count, source_vertices: reference.points.length/3, obrush_vertices: obrushWorld.length, same_f32_matrix_words: sameMatrixWords, same_f32_color_words: sameColorWords, max_nearest_world_component_difference: maxWorldDifference, source_matrix: reference.matrix, obrush_matrix: objPose.local_to_world });
  const beforeReferenceGroup = reference.mesh_count;
  await send('DOM.setFileInputFiles', { nodeId: referenceNode, files: [path.join(out, 'two-objects.obj')] });
  const sourceGroup = await until(async () => evaluate(`(()=>{const m=window.__sculptglReferenceInstance?.getMeshes();if(!m||m.length!==${beforeReferenceGroup}+2)return null;return m.slice(-2).map(x=>({points:Array.from(x.getVertices()),matrix:Array.from(x.getMatrix()),colors:Array.from(x.getColors())}))})()`), 'pinned SculptGL multi-object OBJ import');
  const obrushGroup = [
    { points: afterMulti.document.scene.objects[4].payload.mesh.positions.flat(), colors: afterMulti.document.scene.objects[4].payload.mesh.colors.flat(), matrix: multiPoses[0].affine.local_to_world },
    { points: afterMulti.document.mesh.positions.flat(), colors: afterMulti.document.mesh.colors.flat(), matrix: multiPoses[1].affine.local_to_world },
  ];
  const groupComparison = sourceGroup.map((mesh, i) => ({
    part: i + 1,
    same_f32_matrix_words: mesh.matrix.every((v, j) => Object.is(Math.fround(v), Math.fround(obrushGroup[i].matrix[j]))),
    same_f32_color_words: mesh.colors.every((v, j) => Object.is(Math.fround(v), Math.fround(obrushGroup[i].colors[j]))),
    max_nearest_world_component_difference: Math.max(...world(obrushGroup[i].points, obrushGroup[i].matrix).map(p => Math.min(...world(mesh.points, mesh.matrix).map(q => Math.max(...p.map((v,j) => Math.abs(v-q[j]))))))),
    source_matrix: mesh.matrix,
    obrush_matrix: obrushGroup[i].matrix,
  }));
  result.cases.push({ pinned_multi_object_reference: referenceUrl, group_comparison: groupComparison });
  requireTrue(groupComparison.every(x => x.same_f32_matrix_words && x.same_f32_color_words && x.max_nearest_world_component_difference < 1e-4), 'Pinned SculptGL multi-object affine/RGB/world geometry differs');
  const beforeReferenceBatch=beforeReferenceGroup+2;
  await send('DOM.setFileInputFiles', { nodeId: referenceNode, files: [path.join(out, 'independent-tetra.obj'), path.join(out, 'independent-triangle.ply'), path.join(out, 'independent-triangle.stl')] });
  const referenceBatch=await until(async()=>evaluate(`(()=>{const meshes=window.__sculptglReferenceInstance?.getMeshes();if(meshes?.length!==${beforeReferenceBatch}+3)return null;const appended=meshes.slice(-3).map(mesh=>({vertices:mesh.getVertices().length/3,faces:mesh.getNbFaces()}));return JSON.stringify(appended.map(mesh=>mesh.vertices).sort((a,b)=>a-b))===JSON.stringify([3,3,4])?appended:null})()`),'pinned SculptGL multi-file Add with completed geometry');
  requireTrue(JSON.stringify(referenceBatch.map(mesh=>mesh.vertices).sort((a,b)=>a-b))===JSON.stringify([3,3,4]),'Pinned SculptGL did not import all three selected source meshes');
  result.cases.push({pinned_reference_multi_file:true,before_parts:beforeReferenceBatch,after_parts:beforeReferenceBatch+3,appended:referenceBatch});
  const referenceDrop=await dropFiles([['ignored.txt',Buffer.from('not a mesh')],['dropped-tetra.obj',obj]]);
  requireTrue(referenceDrop.over_prevented&&referenceDrop.drop_prevented,'Pinned SculptGL did not intercept viewport file drop');
  const referenceDropped=await until(async()=>evaluate(`(()=>{const meshes=window.__sculptglReferenceInstance?.getMeshes();if(meshes?.length!==${beforeReferenceBatch}+4)return null;const mesh=meshes[meshes.length-1];return {vertices:mesh.getVertices().length/3,faces:mesh.getNbFaces()};})()`),'pinned SculptGL dropped OBJ');
  requireTrue(referenceDropped.vertices===4&&referenceDropped.faces===4,'Pinned SculptGL dropped OBJ geometry was not retained');
  result.cases.push({pinned_reference_drop:true,accepted:'dropped-tetra.obj',ignored:'ignored.txt',appended:referenceDropped});
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
