// Preregistered held-stroke and pixel-change diagnostic for C255 per-object Source symmetry.
// Usage: node scripts/check-sculptgl-symmetry-held-c255.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { inflateSync } from 'node:zlib';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-symmetry-held-c255/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, why) => { if (!condition) throw Error(why); };
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-affine-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned reference bundle differs');
const result = {
  scope: 'C255 preregistered normal-browser held Draw stroke and changed-pixel diagnostic on the same imported affine triangle; full rows remain open',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: sha(fixture), bytes: fixture.length },
  harness_sha256: sha(await readFile(new URL(import.meta.url))),
  frozen: { offset: 0.5, local_path: [[3.84,-1.70,0],[3.835,-1.71,0],[3.83,-1.72,0]], radius: 50, intensity: 0.5, pixel_channel_threshold: 4, pixel_roi_radius: 90, min_changed_pixels: 20 },
  cases: [], screenshots: [], errors: [], status: 'running',
};
let server, chrome, socket, session, next = 0;
const waiting = new Map(), choosers = [];
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const reply = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
    waiting.set(id, { resolve, reject, timer });
  });
  socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
  return reply;
}
async function evaluate(expression) {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (reply.exceptionDetails) throw Error(JSON.stringify(reply.exceptionDetails));
  return reply.result.value;
}
async function until(fn, why) {
  let last;
  for (let i = 0; i < 150; i++) {
    try { const value = await fn(); if (value) return value; } catch (error) { last = error; }
    await pause(100);
  }
  throw Error(`Timed out ${why}${last ? `: ${last}` : ''}`);
}
async function fileInput(selector) {
  const root = (await send('DOM.getDocument')).root;
  const nodeId = (await send('DOM.querySelector', { nodeId: root.nodeId, selector })).nodeId;
  insist(nodeId, `Missing file input ${selector}`);
  await send('DOM.setFileInputFiles', { nodeId, files: [fixturePath] });
}
async function key(letter, modifiers = 0) {
  const code = `Key${letter.toUpperCase()}`;
  for (const type of ['rawKeyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: letter.toLowerCase(), code, windowsVirtualKeyCode: letter.toUpperCase().charCodeAt(0), modifiers });
}
async function click(rect) {
  insist(rect?.length === 4, 'Missing control rectangle');
  const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
  insist(x > 0 && x < 1600 && y > 72 && y < 990, `Control outside viewport ${rect}`);
  for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  await pause(170);
}
async function visibleControl(key) {
  for (let i = 0; i < 24; i++) {
    const rect = (await evaluate(ownInfo))?.controls?.[key];
    if (rect && rect[1] > 72 && rect[3] < 985) return rect;
    await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 145, y: 760, deltaX: 0, deltaY: 360 });
    await pause(110);
  }
  throw Error(`Control ${key} did not become visible`);
}
async function screenshot(name) {
  const bytes = Buffer.from((await send('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
  await writeFile(path.join(out, `${name}.png`), bytes);
  result.screenshots.push({ name, sha256: sha(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
  return bytes;
}
function pngRgb(bytes) {
  insist(bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])), 'PNG signature differs');
  let off=8,width,height,channels,depth,comp=[];
  while(off<bytes.length){const n=bytes.readUInt32BE(off),type=bytes.toString('ascii',off+4,off+8),data=bytes.subarray(off+8,off+8+n);off+=12+n;
    if(type==='IHDR'){width=data.readUInt32BE(0);height=data.readUInt32BE(4);depth=data[8];channels=data[9]===6?4:data[9]===2?3:0;}
    if(type==='IDAT')comp.push(data);if(type==='IEND')break;
  }
  insist(depth===8 && [3,4].includes(channels) && width<=4096 && height<=4096, 'Unsupported PNG pixels');
  const raw=inflateSync(Buffer.concat(comp)),stride=width*channels,out=Buffer.alloc(height*stride);let at=0;
  for(let y=0;y<height;y++){const filter=raw[at++];for(let x=0;x<stride;x++){
    const left=x>=channels?out[y*stride+x-channels]:0,up=y?out[(y-1)*stride+x]:0,ul=y&&x>=channels?out[(y-1)*stride+x-channels]:0;
    let predictor=0;if(filter===1)predictor=left;else if(filter===2)predictor=up;else if(filter===3)predictor=(left+up)>>1;
    else if(filter===4){const q=left+up-ul,a=Math.abs(q-left),b=Math.abs(q-up),c=Math.abs(q-ul);predictor=a<=b&&a<=c?left:b<=c?up:ul;}
    else insist(filter===0,'Unsupported PNG filter');
    out[y*stride+x]=(raw[at++]+predictor)&255;
  }}return {width,height,channels,pixels:out};
}
function changedPixels(beforeBytes,afterBytes,center,radius,threshold){
  const a=pngRgb(beforeBytes),b=pngRgb(afterBytes);insist(a.width===b.width&&a.height===b.height&&a.channels===b.channels,'Screenshot dimensions differ');
  const bounds=[Math.max(0,Math.floor(center[0]-radius)),Math.max(0,Math.floor(center[1]-radius)),Math.min(a.width,Math.ceil(center[0]+radius)),Math.min(a.height,Math.ceil(center[1]+radius))];
  let changed=0,box=[Infinity,Infinity,-Infinity,-Infinity];for(let y=bounds[1];y<bounds[3];y++)for(let x=bounds[0];x<bounds[2];x++){
    const i=(y*a.width+x)*a.channels;if([0,1,2].some(c=>Math.abs(a.pixels[i+c]-b.pixels[i+c])>threshold)){changed++;box=[Math.min(box[0],x),Math.min(box[1],y),Math.max(box[2],x),Math.max(box[3],y)];}
  }return {roi:bounds,changed_pixels:changed,bbox:changed?box:null};
}
async function frame(){await evaluate('new Promise(requestAnimationFrame)');}
async function parkCursor(){await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:1550,y:970,buttons:0});await frame();await pause(120);}
async function heldStroke(points){
  const [first,...rest]=points;await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:first[0],y:first[1],buttons:0});
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x:first[0],y:first[1],button:'left',buttons:1,clickCount:1});
  await frame();await pause(50);
  for(const point of rest){await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:point[0],y:point[1],button:'left',buttons:1});await frame();await pause(50);}
  const last=points.at(-1);await send('Input.dispatchMouseEvent',{type:'mouseReleased',x:last[0],y:last[1],button:'left',buttons:0,clickCount:1});
  for(let i=0;i<4;i++)await frame();await pause(120);
}
function validatePath(points,rect,label){insist(points.every(([x,y])=>Number.isFinite(x)&&Number.isFinite(y)&&x>rect[0]+2&&x<rect[2]-2&&y>rect[1]+2&&y<rect[3]-2),`${label} projected path outside sculpt canvas ${JSON.stringify({points,rect})}`);}
async function exactNumber(key,value){
  const rect=await visibleControl(key);await click(rect);
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4,commands:['selectAll']});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4});
  await send('Input.insertText',{text:String(value)});
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await pause(200);
}

const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__symmetryPair={draws:[],gl:null,events:[]};
  for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])document.addEventListener(type,e=>{const q=window.__symmetryPair.events;if(q.length===4096)q.shift();q.push({type,trusted:e.isTrusted,x:e.clientX,y:e.clientY,buttons:e.buttons,target:e.target?.id??null,at:performance.now()});},true);
  HTMLCanvasElement.prototype.getContext=function(kind,...args){
    const gl=original.call(this,kind,...args);if(!gl||typeof gl.drawElements!=='function'||gl.__symmetryPairWrapped)return gl;
    gl.__symmetryPairWrapped=true;window.__symmetryPair.gl=gl;
    const names=new WeakMap(),get=gl.getUniformLocation,u3=gl.uniform3fv,u3f=gl.uniform3f,u1=gl.uniform1i,draw=gl.drawElements;
    gl.getUniformLocation=function(program,name){const loc=get.call(this,program,name);if(loc)names.set(loc,name);return loc;};
    gl.uniform3fv=function(loc,value){const name=loc&&names.get(loc);if(name==='uPlaneO'||name==='uPlaneN'||name==='u_symmetry_origin'||name==='u_symmetry_normal')window.__symmetryPair[name]=Array.from(value);return u3.call(this,loc,value);};
    gl.uniform3f=function(loc,x,y,z){const name=loc&&names.get(loc);if(name==='u_symmetry_origin'||name==='u_symmetry_normal')window.__symmetryPair[name]=[x,y,z];return u3f.call(this,loc,x,y,z);};
    gl.uniform1i=function(loc,value){const name=loc&&names.get(loc);if(name==='uSym'||name==='u_symmetry_line')window.__symmetryPair[name]=value;return u1.call(this,loc,value);};
    gl.drawElements=function(mode,count,type,offset){if(mode===gl.TRIANGLES){const s=window.__symmetryPair,r=s.draws;if(r.length===256)r.shift();r.push({count,at:performance.now(),source_origin:s.uPlaneO??null,source_normal:s.uPlaneN??null,source_enabled:s.uSym??null,obrush_origin:s.u_symmetry_origin??null,obrush_normal:s.u_symmetry_normal??null,obrush_enabled:s.u_symmetry_line??null});}return draw.call(this,mode,count,type,offset);};
    return gl;
  };
})();`;
const gpu = `(()=>{const s=window.__symmetryPair;return {draws:s?.draws.slice(-64)??[],gl_error:s?.gl?.getError()??null}})()`;
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const sm=a.getSculptManager(),tool=sm.getCurrentTool(),canvas=a.getCanvas(),r=canvas.getBoundingClientRect();return {path:window.__symmetryPairPath??null,canvas_rect:[r.left,r.top,r.right,r.bottom],tool:{index:sm.getToolIndex(),symmetry:sm.getSymmetry(),radius:tool._radius,intensity:tool._intensity,alpha:tool._idAlpha,continuous:sm._continuous,clay:tool._clay,accumulate:tool._accumulate,culling:tool._culling,negative:tool._negative,lock:tool._lockPosition},meshes:a.getMeshes().length,active:a.getMeshes().indexOf(a.getMesh()),selected:a.getSelectedMeshes().map(m=>a.getMeshes().indexOf(m)),objects:a.getMeshes().map(m=>({vertices:m.getNbVertices(),faces:m.getNbFaces(),small_positions:m.getNbVertices()<10?Array.from(m.getVertices()):null,small_faces:m.getNbFaces()<10?Array.from(m.getFaces()):null,matrix:Array.from(m.getMatrix()),center:Array.from(m._transformData._center),normal:Array.from(m.getSymmetryNormal()),offset:m.getSymmetryOffset(),loose:Array.from(m.getLocalBound()),plane:Array.from(m.getSymmetryOrigin())})),states:a.getStateManager()._undos.length,guide:a.getGui()._ctrlScene?._main?._showSymmetryLine??null}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{camera:m.camera,source_geometry:m.source_geometry,brush:m.brush,brush_options:m.brush_options,subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),active:m.active_subtool_id,selected:m.selected_subtool_ids,controls:m.source_geometry_controls_hit_rect,mode:m.source_geometry?.per_tool?.[0]?.mode??null,offset:m.source_geometry?.active_object_symmetry_offset??null,display:m.display_flags,history:m.history,scene_error:m.scene_error,viewport:m.viewport}:null})()`;
async function sample(name, info) {
  await pause(260);
  const row = { name, info: await evaluate(info), gpu: await evaluate(gpu) };
  result.cases.push(row);
  await screenshot(name);
  await evaluate('window.__symmetryPair.draws.length=0');
  return row;
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const source = pathname.startsWith('/reference/') ? reference : web;
      const relative = pathname.startsWith('/reference/') ? pathname.slice('/reference'.length) : pathname;
      const file = path.resolve(source, '.' + (relative === '/' ? '/index.html' : relative));
      if (!file.startsWith(source + path.sep)) throw Error('outside served tree');
      const bytes = await readFile(file);
      response.setHeader('Content-Type', ({ '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html', '.css': 'text/css' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--headless=new', `--user-data-dir=${path.join(out, 'chrome-profile')}`, '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking', '--window-size=1600,1000', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1], 'owned Chrome endpoint');
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data));
    if (message.method === 'Page.fileChooserOpened') choosers.push(message.params);
    if (!message.id) return;
    const pending = waiting.get(message.id); if (!pending) return;
    waiting.delete(message.id); clearTimeout(pending.timer);
    message.error ? pending.reject(Error(JSON.stringify(message.error))) : pending.resolve(message.result);
  });
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  for (const domain of ['Page', 'Runtime', 'DOM']) await send(`${domain}.enable`);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const base = `http://127.0.0.1:${server.address().port}`;

  await send('Page.navigate', { url: `${base}/reference/` });
  await until(async () => (await evaluate(refInfo))?.meshes === 1, 'reference startup');
  await fileInput('#fileopen');
  await until(async () => (await evaluate(refInfo))?.meshes === 3, 'reference imported parts');
  const referenceZero = await sample('reference-zero', refInfo);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0.5)');
  const referenceHalf = await sample('reference-half', refInfo);
  insist(referenceHalf.info.objects[referenceHalf.info.active].offset === 0.5, 'Reference active offset did not change');
  insist(referenceHalf.info.objects.filter((x,i)=>i!==referenceHalf.info.active).every((x,i)=>x.offset===referenceZero.info.objects.filter((_,j)=>j!==referenceZero.info.active)[i].offset), 'Reference peer offset changed');
  insist(referenceHalf.info.objects[referenceHalf.info.active].plane[0] !== referenceZero.info.objects[referenceZero.info.active].plane[0], 'Reference local mirror plane did not move');
  const mirrorInputX = 2 * referenceHalf.info.objects[referenceHalf.info.active].plane[0] - 2.5;
  const mirrorRay = `(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),p=a.getPickingSymmetry();const x=${mirrorInputX};const hit=p.intersectionRayMesh(m,[x,-2/3,10],[x,-2/3,-10]);return {input_x:x,hit,face:p.getPickedFace(),point:hit?Array.from(p.getIntersectionPoint()):null}})()`;
  result.reference_mirror_pick_half = await evaluate(mirrorRay);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0)');
  result.reference_mirror_pick_zero = await evaluate(mirrorRay);
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0.5)');
  insist(result.reference_mirror_pick_half.hit && result.reference_mirror_pick_half.face === 0, 'Reference offset mirror ray missed the active triangle');
  insist(Math.abs(result.reference_mirror_pick_half.point[0]-2.5) < 1e-5, 'Reference reflected hit X differs from authored triangle center');
  insist(!result.reference_mirror_pick_zero.hit, 'Reference zero-offset mirror ray unexpectedly hit at the fixed nonzero ray');

  const localPath=result.frozen.local_path;
  const refPath=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),cam=a.getCamera(),r=a.getCanvas().getBoundingClientRect(),p=${JSON.stringify(localPath)};return p.map(v=>{const q=m.getMatrix(),w=[q[0]*v[0]+q[4]*v[1]+q[8]*v[2]+q[12],q[1]*v[0]+q[5]*v[1]+q[9]*v[2]+q[13],q[2]*v[0]+q[6]*v[1]+q[10]*v[2]+q[14]],s=cam.project(w);return [r.left+s[0],r.top+s[1]]})})()`);
  result.reference_path=refPath;validatePath(refPath,referenceHalf.info.canvas_rect,'Reference');
  insist(referenceHalf.info.tool.symmetry && referenceHalf.info.tool.index===0 && referenceHalf.info.tool.radius===50 && referenceHalf.info.tool.intensity===0.5 && [0,'None'].includes(referenceHalf.info.tool.alpha) && !referenceHalf.info.tool.continuous,'Reference held-stroke controls differ');
  await parkCursor();const refBeforePng=await screenshot('reference-held-before');
  const refEventIndex=await evaluate('window.__symmetryPair.events.length');
  result.reference_vertices_before=Array.from((await evaluate(refInfo)).objects[referenceHalf.info.active].small_positions);
  await heldStroke(refPath);
  result.reference_events=await evaluate(`window.__symmetryPair.events.slice(${refEventIndex})`);
  insist(result.reference_events.some(e=>e.type==='pointerdown'&&e.trusted) && result.reference_events.some(e=>e.type==='pointerup'&&e.trusted) && result.reference_events.some(e=>e.type==='pointermove'&&e.buttons===1&&e.trusted),'Reference trusted held pointer lifecycle missing');
  result.reference_held_pick=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,p=a.getPickingSymmetry();return {face:p.getPickedFace(),point:Array.from(p.getIntersectionPoint()),mesh_index:a.getMeshes().indexOf(p.getMesh()),undo:a.getStateManager()._undos.length}})()`);
  await parkCursor();const refAfterPng=await screenshot('reference-held-after');
  result.reference_pixel_change=changedPixels(refBeforePng,refAfterPng,refPath[0],result.frozen.pixel_roi_radius,result.frozen.pixel_channel_threshold);
  result.reference_vertices_after=Array.from((await evaluate(refInfo)).objects[referenceHalf.info.active].small_positions);
  insist(result.reference_held_pick.face===0 && result.reference_held_pick.mesh_index===referenceHalf.info.active,'Reference held mirrored pick missed active face');
  insist(result.reference_vertices_after.some((v,i)=>Math.abs(v-result.reference_vertices_before[i])>1e-6),'Reference held stroke did not deform active triangle');
  insist(result.reference_pixel_change.changed_pixels>=result.frozen.min_changed_pixels,'Reference held screenshot ROI did not change');

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  await until(async () => (await evaluate(ownInfo))?.subtools === 3, 'OBRush imported parts');
  const ownZero = await sample('obrush-zero', ownInfo);
  insist(ownZero.info.scene_error === null, `OBRush imported Source guide error: ${ownZero.info.scene_error}`);
  insist(ownZero.gpu.draws.some(draw => draw.count === 3 && draw.obrush_enabled === 1 && draw.obrush_origin), 'OBRush imported active part has no guide GPU plane');
  await click(await visibleControl('source_geometry.mask.preview'));
  const preview = await sample('obrush-mask-preview', ownInfo);
  result.mask_preview = { history: preview.info.history, controls: preview.info.controls };
  if (preview.info.controls['source_geometry.mask.convert']) {
    await click(await visibleControl('source_geometry.mask.convert'));
    const converted = await sample('obrush-mask-converted', ownInfo);
    await click(await visibleControl('source_geometry.mode.source'));
    const sourceMode = await sample('obrush-source-mode', ownInfo);
    insist(sourceMode.info.mode === 'SourceOrdinary', 'OBRush Source mode could not be selected after mask conversion');
    await exactNumber('source_geometry.mirror.offset.value',0.5);
    await exactNumber('source_geometry.pressure.radius.value',0);
    const ownHalf = await sample('obrush-half', ownInfo);
    result.diagnostic = { reference: referenceHalf.info, obrush_before: ownZero.info, obrush_after: ownHalf.info, converted: converted.info };
    insist(ownHalf.info.offset === 0.5, `OBRush exact normalized offset differs: ${ownHalf.info.offset}`);
    insist(ownHalf.info.scene_error === null, `OBRush mirror guide error: ${ownHalf.info.scene_error}`);
    const plane = row => row.gpu.draws.filter(draw => draw.count === 3 && draw.obrush_enabled === 1 && draw.obrush_origin).at(-1)?.obrush_origin;
    const beforePlane = plane(converted), afterPlane = plane(ownHalf);
    insist(beforePlane && afterPlane, 'Missing active OBrush guide-plane GPU uniforms');
    result.guide_plane_eye_delta = afterPlane.map((value, i) => value - beforePlane[i]);
    insist(Math.hypot(...result.guide_plane_eye_delta) > 1e-4, 'OBRush SymOffset changed metadata but not rendered guide plane');
    const refActive = referenceHalf.info.active;
    const sourceOffset = referenceHalf.info.objects[refActive].offset;
    const sourceDelta = referenceHalf.info.objects[refActive].plane[0] - referenceZero.info.objects[refActive].plane[0];
    result.normalized_radius_pair = { sculptgl: sourceDelta / sourceOffset, obrush: result.guide_plane_eye_delta[0] / ownHalf.info.offset };
    result.normalized_radius_pair.abs_difference = Math.abs(result.normalized_radius_pair.sculptgl - result.normalized_radius_pair.obrush);
    insist(result.normalized_radius_pair.abs_difference < 1e-5, `Paired loose-root normalized radius differs: ${JSON.stringify(result.normalized_radius_pair)}`);
    insist(result.cases.every(row => row.gpu.gl_error === 0), 'Nonzero WebGL error');
    const rootMetrics=await evaluate('window.obrushBrushMetrics()');
    const cam=rootMetrics.camera,vp=cam.viewport;
    const rootPath=localPath.map(([x,y,z])=>{const q=cam.view_projection,p=[x,y,z+0.5,1],clip=[0,1,2,3].map(row=>q[row]*p[0]+q[4+row]*p[1]+q[8+row]*p[2]+q[12+row]*p[3]);return [vp[0]+(clip[0]/clip[3]+1)*(vp[2]-vp[0])/2,vp[1]+(1-clip[1]/clip[3])*(vp[3]-vp[1])/2]});
    result.obrush_path=rootPath;validatePath(rootPath,vp,'OBRush');
    insist(rootMetrics.display_flags.brush_symmetry_enabled && ownHalf.info.mode==='SourceOrdinary' && rootMetrics.source_geometry.per_tool[0].radius===50 && rootMetrics.source_geometry.per_tool[0].options.intensity===0.5 && rootMetrics.source_geometry.per_tool[0].radius_factor===0,'OBRush held-stroke controls differ');
    await parkCursor();const ownBeforePng=await screenshot('obrush-held-before');
    const ownEventIndex=await evaluate('window.__symmetryPair.events.length');
    result.obrush_held_before=await evaluate(ownInfo);
    await heldStroke(rootPath);
    result.obrush_held_after=await evaluate(ownInfo);
    result.obrush_events=await evaluate(`window.__symmetryPair.events.slice(${ownEventIndex})`);
    insist(result.obrush_events.some(e=>e.type==='pointerdown'&&e.trusted) && result.obrush_events.some(e=>e.type==='pointerup'&&e.trusted) && result.obrush_events.some(e=>e.type==='pointermove'&&e.buttons===1&&e.trusted),'OBRush trusted held pointer lifecycle missing');
    await parkCursor();const ownAfterPng=await screenshot('obrush-held-after');
    result.obrush_pixel_change=changedPixels(ownBeforePng,ownAfterPng,rootPath[0],result.frozen.pixel_roi_radius,result.frozen.pixel_channel_threshold);
    result.pixel_changed_ratio=result.obrush_pixel_change.changed_pixels/result.reference_pixel_change.changed_pixels;
    const completed=result.obrush_held_after.source_geometry.last_completed?.capture;
    insist(completed?.mirror_triangle===0 && completed?.mirror_plane,`OBRush held mirror report lacks active triangle/plane: ${JSON.stringify(completed)}`);
    insist(result.obrush_pixel_change.changed_pixels>=result.frozen.min_changed_pixels,'OBRush held screenshot ROI did not change');
    insist(JSON.stringify(result.obrush_held_before.ids)===JSON.stringify(result.obrush_held_after.ids),'OBRush peer object IDs changed');
    result.status = 'passed scoped C255 held-stroke delivery, mirrored contact and changed-pixel gates; full rows remain open';
  } else {
    result.status = 'failed';
    result.errors.push('OBRush imported SGL has no available mask conversion action');
  }
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error.stack || error));
} finally {
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  try { if (socket?.readyState === WebSocket.OPEN) await send('Browser.close', {}, null); } catch {}
  socket?.close(); if (chrome?.exitCode === null) chrome.kill(); server?.close();
}
console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), errors: result.errors }));
if (result.status === 'failed') process.exitCode = 1;
