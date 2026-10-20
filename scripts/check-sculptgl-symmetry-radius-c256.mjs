// C256 preregistered, read-only radius-chain diagnostic; see the matching protocol.
// Usage: node scripts/check-sculptgl-symmetry-radius-c256.mjs WEB_DIST PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { inflateSync } from 'node:zlib';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-symmetry-radius-c256/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, why) => { if (!condition) throw Error(why); };
const close = (a,b,relative=1e-8) => Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=relative*Math.max(1,Math.abs(a),Math.abs(b));
const chainClose = (a,b,relative=0.01) => Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=Math.max(1e-5,relative*Math.max(Math.abs(a),Math.abs(b)));
function mul4(m,p){return [0,1,2,3].map(i=>m[i]*p[0]+m[i+4]*p[1]+m[i+8]*p[2]+m[i+12]*p[3]);}
function inverse4(m){
  const rows=Array.from({length:4},(_,r)=>Array.from({length:8},(_,c)=>c<4?m[c*4+r]:Number(c-4===r)));
  for(let c=0;c<4;c++){
    let pivot=c;for(let r=c+1;r<4;r++)if(Math.abs(rows[r][c])>Math.abs(rows[pivot][c]))pivot=r;
    insist(Math.abs(rows[pivot][c])>1e-20,'Captured Source view matrix is singular');
    [rows[c],rows[pivot]]=[rows[pivot],rows[c]];
    const d=rows[c][c];for(let j=0;j<8;j++)rows[c][j]/=d;
    for(let r=0;r<4;r++)if(r!==c){const f=rows[r][c];for(let j=0;j<8;j++)rows[r][j]-=f*rows[c][j];}
  }
  return Array.from({length:16},(_,i)=>rows[i%4][4+Math.floor(i/4)]);
}
function project64(m,vp,world){
  const c=mul4(m,[...world,1]);insist(Math.abs(c[3])>1e-20,'Zero Source projection W');
  return [vp[0]+(c[0]/c[3]+1)*vp[2]/2,vp[1]+(1-c[1]/c[3])*vp[3]/2,(c[2]/c[3]+1)/2];
}
function unproject64(inv,vp,screen){
  const clip=[2*(screen[0]-vp[0])/vp[2]-1,1-2*(screen[1]-vp[1])/vp[3],2*screen[2]-1,1],p=mul4(inv,clip);
  insist(Math.abs(p[3])>1e-20,'Zero Source inverse projection W');return p.slice(0,3).map(v=>v/p[3]);
}
function rootRadiusChain(capture){
  const row=capture.dab_observations?.rows?.find(r=>r.passes?.[0]&&r.in_contact);
  insist(row,'No admitted primary Source dab observation');
  const pass=row.passes[0],pose=capture.pose32,world=mul4(pose,[...pass.center,1]).slice(0,3);
  const vp=row.viewport64,view=row.view64,pressure=row.pressure64;
  const radius=capture.per_tool_radius??250, factor=capture.per_tool_radius_factor??0.75;
  const offset=radius*(1+factor*(pressure*2-1));
  const projected=project64(view,vp,world),edge=unproject64(inverse4(view),vp,[projected[0]+offset,projected[1],projected[2]]);
  const worldRadius=Math.hypot(...edge.map((v,i)=>v-world[i]));
  const poseScale2=pose[0]*pose[0]+pose[1]*pose[1]+pose[2]*pose[2];
  return {ordinal:row.ordinal,pressure,radius,radius_factor:factor,pressure_factor:offset/radius,offset_x:offset,world,projected,edge,world_radius:worldRadius,pose_scale_squared:poseScale2,expected_local_radius_squared:worldRadius*worldRadius/poseScale2,reported_local_radius_squared:pass.local_radius_squared,mirror_local_radius_squared:row.passes[1]?.local_radius_squared??null,pass_center:pass.center,screen:row.screen64,viewport:vp};
}
if(process.argv.includes('--self-check')){
  const m=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],vp=[0,0,100,100],world=[0.1,-0.2,0.3],p=project64(m,vp,world),again=unproject64(inverse4(m),vp,p);
  insist(world.every((v,i)=>close(v,again[i])),'C256 project/inverse self-check failed');
  const moved=unproject64(inverse4(m),vp,[p[0]+20,p[1],p[2]]);
  insist(close(moved[0]-world[0],0.4),'C256 X-offset self-check failed');
  if(process.env.C256_C255_RECEIPT){
    const old=JSON.parse(await readFile(process.env.C256_C255_RECEIPT));
    const capture=old.obrush_held_after?.source_geometry?.last_completed?.capture;
    const tool=old.obrush_held_after?.source_geometry?.per_tool?.[0];
    insist(capture&&tool,'C255 self-check receipt lacks Source pass');
    const chain=rootRadiusChain({...capture,per_tool_radius:tool.radius,per_tool_radius_factor:tool.radius_factor});
    insist(chainClose(chain.expected_local_radius_squared,chain.reported_local_radius_squared),'C255 captured-matrix replay differs from Source pass');
    insist(close(chain.offset_x,250),'C255 captured offset was not 250 CSS px');
    console.log('C256 C255 captured-matrix replay passed');
  }
  console.log('C256 matrix and offset self-check passed');process.exit(0);
}
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-affine-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned reference bundle differs');
const result = {
  scope: 'C256 preregistered normal-browser held Draw original-call radius-chain diagnostic on the same imported affine triangle; full rows remain open',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { base_commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), wasm_sha256: sha(await readFile(path.join(web, 'pkg/obrush_app_bg.wasm'))) },
  fixture: { sha256: sha(fixture), bytes: fixture.length },
  harness_sha256: sha(await readFile(new URL(import.meta.url))),
  frozen: { offset: 0.5, local_path: [[3.70,-1.99,0],[3.845,-1.99,0],[3.99,-1.99,0]], radius: 250, intensity: 0.5, pixel_channel_threshold: 4, pixel_roi_radius: 90, min_changed_pixels: 20, camera_span_relative_tolerance: 0.01, chain_relative_tolerance: 0.01, physical_radius_ratio_expected: 4, ratio_relative_tolerance: 0.02 },
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
async function enterNumber(rect,value){
  await click(rect);
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4,commands:['selectAll']});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:4});
  await send('Input.insertText',{text:String(value)});
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await pause(200);
}


async function exactNumber(key,value){await enterNumber(await visibleControl(key),value);}
async function sourceRadius(value){
  const expr="(()=>{const menu=window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl;const e=Array.from(menu.querySelectorAll('input[type=number]')).find(e=>e.getBoundingClientRect().width>0);const r=e?.getBoundingClientRect();return r?{value:Number(e.value),rect:[r.left,r.top,r.right,r.bottom]}:null})()";
  const q=await evaluate(expr);insist(q?.rect,'Visible reference Draw radius input absent');await enterNumber(q.rect,value);
  const actual=await evaluate(expr);insist(actual.value===value,`Reference Draw radius field differs: ${JSON.stringify(actual)}`);
}

const instrument = `(()=>{
  const original=HTMLCanvasElement.prototype.getContext;
  window.__symmetryPair={draws:[],gl:null,events:[],radiusCalls:[]};
  for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])document.addEventListener(type,e=>{const q=window.__symmetryPair.events;if(q.length===4096)q.shift();q.push({type,trusted:e.isTrusted,x:e.clientX,y:e.clientY,buttons:e.buttons,pressure:e.pressure??null,pointerType:e.pointerType??null,target:e.target?.id??null,at:performance.now()});},true);
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
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const sm=a.getSculptManager(),tool=sm.getCurrentTool(),canvas=a.getCanvas(),r=canvas.getBoundingClientRect();return {path:window.__symmetryPairPath??null,canvas_rect:[r.left,r.top,r.right,r.bottom],tool:{index:sm.getToolIndex(),symmetry:sm.getSymmetry(),radius:tool._radius,intensity:tool._intensity,alpha:tool._idAlpha,continuous:sm._continuous,clay:tool._clay,accumulate:tool._accumulate,culling:tool._culling,negative:tool._negative,lock:tool._lockPosition},meshes:a.getMeshes().length,active:a.getMeshes().indexOf(a.getMesh()),selected:a.getSelectedMeshes().map(m=>a.getMeshes().indexOf(m)),objects:a.getMeshes().map(m=>({vertices:m.getNbVertices(),faces:m.getNbFaces(),small_positions:m.getNbVertices()<10?Array.from(m.getVertices()):null,small_materials:m.getNbVertices()<10?Array.from(m.getMaterials()):null,small_faces:m.getNbFaces()<10?Array.from(m.getFaces()):null,matrix:Array.from(m.getMatrix()),center:Array.from(m._transformData._center),normal:Array.from(m.getSymmetryNormal()),offset:m.getSymmetryOffset(),loose:Array.from(m.getLocalBound()),plane:Array.from(m.getSymmetryOrigin()),visible:m.isVisible()})),states:a.getStateManager()._undos.length,guide:a.getGui()._ctrlScene?._main?._showSymmetryLine??null}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{camera:m.camera,source_geometry:m.source_geometry,brush:{kind:m.kind,alpha:m.alpha,continuous:m.continuous,pressure:m.pressure,pressure_radius:m.pressure_radius,pressure_strength:m.pressure_strength,adapt_detail:m.adaptive_detail,symmetry_x:m.symmetry_x},brush_options:m.brush_options,subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),active:m.active_subtool_id,selected:m.selected_subtool_ids,controls:m.source_geometry_controls_hit_rect,scene_controls:m.scene_controls,solo:m.solo,mode:m.source_geometry?.per_tool?.[0]?.mode??null,offset:m.source_geometry?.active_object_symmetry_offset??null,display:m.display_flags,history:m.history,scene_error:m.scene_error,viewport:m.viewport}:null})()`;
const installReferenceRadiusTrace = `(()=>{
  const a=window.__sculptglReferenceInstance,p=a.getPicking(),log=window.__symmetryPair.radiusCalls;
  if(p.__c256RadiusWrapped)return false;
  const original=p.computeWorldRadius2;
  p.computeWorldRadius2=function(ignorePressure){
    const tool=a.getSculptManager().getCurrentTool(),canvas=a.getCanvas(),cam=a.getCamera();
    const pointer=window.__symmetryPair.events.filter(e=>e.type==='pointerdown'||e.type==='pointermove').at(-1)??null;
    const trace={at:performance.now(),pointer,ignore_pressure:!!ignorePressure,nominal_radius:tool._radius,pixel_ratio:a.getPixelRatio(),screen_radius:tool.getScreenRadius(),canvas:{backing:[canvas.width,canvas.height],client:[canvas.clientWidth,canvas.clientHeight]},camera:[cam._width,cam._height],local_hit:Array.from(this.getIntersectionPoint()),mesh_scale_squared:this.getMesh()?.getScale2()??null};
    const wasProject=this.project,wasUnproject=this.unproject;
    this.project=function(v){trace.world=Array.from(v);const q=wasProject.call(this,v);trace.projected=Array.from(q);return q;};
    this.unproject=function(x,y,z){trace.unproject_screen=[x,y,z];const q=wasUnproject.call(this,x,y,z);trace.edge=Array.from(q);return q;};
    try{
      const squared=original.call(this,ignorePressure);
      trace.world_radius_squared=squared;
      trace.local_radius_squared=squared/trace.mesh_scale_squared;
      trace.offset_x=trace.unproject_screen?.[0]-trace.projected?.[0];
      trace.pressure_factor=trace.offset_x/trace.screen_radius;
      if(log.length===256)log.shift();log.push(trace);
      return squared;
    }finally{this.project=wasProject;this.unproject=wasUnproject;}
  };
  p.__c256RadiusWrapped=true;return true;
})()`;
async function visibleLayerControl(key){
  for(let i=0;i<24;i++){
    const rect=await evaluate(`window.obrushBrushMetrics()?.layer_controls?.[${JSON.stringify(key)}]??null`);
    if(rect&&rect[1]>72&&rect[3]<985)return rect;
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:1520,y:760,deltaX:0,deltaY:360});await pause(110);
  }
  throw Error(`Visible Source camera numeric control absent: ${key}`);
}
async function saveRootVertices(label){
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const before=await evaluate('window.__symmetryPair.downloads.length');
  await key('s',2);
  const record=await until(async()=>{const d=await evaluate('window.__symmetryPair.downloads.at(-1)??null');return d&&d.sequence>before?d:null;},`OBRush normal ${label} .obrush Save`);
  insist(record.name.endsWith('.obrush'),`OBRush normal Save returned ${record.name}`);
  const bytes=Buffer.from(record.contents,'utf8');
  await writeFile(path.join(out,`obrush-${label}.obrush`),bytes);
  const doc=JSON.parse(record.contents),positions=doc.mesh?.positions;
  insist(Array.isArray(positions)&&positions.length===3,`OBRush ${label} active triangle is not three vertices`);
  return {name:record.name,sha256:sha(bytes),bytes:bytes.length,positions};
}
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
  await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.showHide(true)');
  const isolatedReference=await evaluate(refInfo);
  insist(isolatedReference.objects.filter(m=>m.visible).length===1 && isolatedReference.objects[isolatedReference.active].visible,'Reference active triangle was not isolated');
  result.reference_isolation={active:isolatedReference.active,visible:isolatedReference.objects.map(m=>m.visible)};
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

  await sourceRadius(result.frozen.radius);
  const localPath=result.frozen.local_path;
  const refPath=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),cam=a.getCamera(),r=a.getCanvas().getBoundingClientRect(),p=${JSON.stringify(localPath)};return p.map(v=>{const q=m.getMatrix(),w=[q[0]*v[0]+q[4]*v[1]+q[8]*v[2]+q[12],q[1]*v[0]+q[5]*v[1]+q[9]*v[2]+q[13],q[2]*v[0]+q[6]*v[1]+q[10]*v[2]+q[14]],s=cam.project(w);return [r.left+s[0],r.top+s[1]]})})()`);
  result.reference_path=refPath;validatePath(refPath,referenceHalf.info.canvas_rect,'Reference');
  insist(Math.hypot(refPath.at(-1)[0]-refPath[0][0],refPath.at(-1)[1]-refPath[0][1])>37.5,'Reference held path remains below pinned 0.15 × 250 spacing gate');
  insist(referenceHalf.info.tool.symmetry && referenceHalf.info.tool.index===0 && (await evaluate(refInfo)).tool.radius===result.frozen.radius && referenceHalf.info.tool.intensity===0.5 && [0,'None'].includes(referenceHalf.info.tool.alpha) && !referenceHalf.info.tool.continuous,'Reference held-stroke controls differ');
  insist(await evaluate(installReferenceRadiusTrace),'Reference primary picker original-call trace was not installed');
  await parkCursor();const refBeforePng=await screenshot('reference-held-before');
  const refEventIndex=await evaluate('window.__symmetryPair.events.length');
  const refRadiusIndex=await evaluate('window.__symmetryPair.radiusCalls.length');
  result.reference_vertices_before=Array.from((await evaluate(refInfo)).objects[referenceHalf.info.active].small_positions);
  await heldStroke(refPath);
  result.reference_events=await evaluate(`window.__symmetryPair.events.slice(${refEventIndex})`);
  result.reference_radius_calls=await evaluate(`window.__symmetryPair.radiusCalls.slice(${refRadiusIndex})`);
  insist(result.reference_events.some(e=>e.type==='pointerdown'&&e.trusted) && result.reference_events.some(e=>e.type==='pointerup'&&e.trusted) && result.reference_events.some(e=>e.type==='pointermove'&&e.buttons===1&&e.trusted),'Reference trusted held pointer lifecycle missing');
  result.reference_held_pick=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,p=a.getPickingSymmetry();return {face:p.getPickedFace(),point:Array.from(p.getIntersectionPoint()),radius_squared:p.getLocalRadius2(),picked_vertices:p.getPickedVertices()?.length??null,mesh_index:a.getMeshes().indexOf(p.getMesh()),undo:a.getStateManager()._undos.length}})()`);
  await parkCursor();const refAfterPng=await screenshot('reference-held-after');
  result.reference_pixel_change=changedPixels(refBeforePng,refAfterPng,refPath[0],result.frozen.pixel_roi_radius,result.frozen.pixel_channel_threshold);
  result.reference_vertices_after=Array.from((await evaluate(refInfo)).objects[referenceHalf.info.active].small_positions);
  insist(result.reference_held_pick.face===0 && result.reference_held_pick.mesh_index===referenceHalf.info.active,'Reference held mirrored pick missed active face');
  insist(result.reference_radius_calls.length>0,'No reference original-call primary radius trace during held stroke');
  insist(result.reference_vertices_after.some((v,i)=>Math.abs(v-result.reference_vertices_before[i])>1e-6),'Reference held stroke did not deform active triangle');
  insist(result.reference_pixel_change.changed_pixels>=result.frozen.min_changed_pixels,'Reference held screenshot ROI did not change');

  await send('Page.navigate', { url: `${base}/?obrushprofile=sculptgl` });
  await until(async () => (await evaluate(ownInfo))?.subtools === 1, 'OBRush startup');
  await evaluate("(()=>{const old=window.obrushDownload;window.__symmetryPair.downloads=[];window.obrushDownload=function(name,contents){const q=window.__symmetryPair.downloads;q.push({sequence:q.length+1,name,contents});return old(name,contents);};return true})()");
  await evaluate("document.getElementById('obrush_canvas').focus()");
  const chooserCount = choosers.length;
  await key('o', 2);
  await until(async () => choosers.length > chooserCount, 'OBRush Add chooser');
  await fileInput('#source-sgl-file');
  await until(async () => (await evaluate(ownInfo))?.subtools === 3, 'OBRush imported parts');
  const scene=await evaluate(ownInfo);
  insist(scene.scene_controls?.['scene.solo'] && scene.scene_controls?.['scene.frame'],'OBRush visible Solo/Frame controls missing');
  await click(scene.scene_controls['scene.solo']);
  const isolatedOwn=await evaluate(ownInfo);insist(isolatedOwn.solo===true,'OBRush active-part solo did not enable');
  await click(isolatedOwn.scene_controls['scene.frame']);
  await pause(350);result.obrush_isolation={solo:(await evaluate(ownInfo)).solo,active:(await evaluate(ownInfo)).active};
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
    await exactNumber('source_geometry.radius.value',result.frozen.radius);
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
    const projectRoot=(cam)=>{const vp=cam.viewport,q=cam.view_projection;return localPath.map(([x,y,z])=>{const p=[x,y,z+0.5,1],clip=[0,1,2,3].map(row=>q[row]*p[0]+q[4+row]*p[1]+q[8+row]*p[2]+q[12+row]*p[3]);return [vp[0]+(clip[0]/clip[3]+1)*(vp[2]-vp[0])/2,vp[1]+(1-clip[1]/clip[3])*(vp[3]-vp[1])/2]})};
    let rootMetrics=await evaluate('window.obrushBrushMetrics()');let rootPath=projectRoot(rootMetrics.camera);
    const span=p=>Math.hypot(p.at(-1)[0]-p[0][0],p.at(-1)[1]-p[0][1]);
    result.obrush_wheel_zoom={before:{span_css:span(rootPath),camera:rootMetrics.camera},events:[]};
    for(let step=0;step<32&&span(rootPath)<50;step++){
      const vp=rootMetrics.camera.viewport,x=(vp[0]+vp[2])/2,y=(vp[1]+vp[3])/2;
      await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y,buttons:0});
      await send('Input.dispatchMouseEvent',{type:'mouseWheel',x,y,deltaX:0,deltaY:-360});
      await frame();await pause(30);
      rootMetrics=await evaluate('window.obrushBrushMetrics()');rootPath=projectRoot(rootMetrics.camera);
      result.obrush_wheel_zoom.events.push({step:step+1,deltaY:-360,span_css:span(rootPath),projection_zoom:rootMetrics.camera.source_framing?.projection_zoom,eye_z:rootMetrics.camera.position64?.[2]});
    }
    result.obrush_wheel_zoom.after={span_css:span(rootPath),camera:rootMetrics.camera};
    insist(span(rootPath)>=50,`OBRush normal-wheel camera never reached held spacing: ${JSON.stringify(result.obrush_wheel_zoom)}`);
    const targetSpan=span(refPath);
    result.camera_match={reference_span_css:targetSpan,typed:[]};
    for(let attempt=0;attempt<3;attempt++){
      const relative=Math.abs(span(rootPath)/targetSpan-1);
      if(relative<=result.frozen.camera_span_relative_tolerance)break;
      const oldZoom=rootMetrics.camera.source_framing?.projection_zoom;
      insist(Number.isFinite(oldZoom)&&oldZoom>0,'Source camera zoom unavailable');
      const nextZoom=Number((oldZoom*span(rootPath)/targetSpan).toPrecision(12));
      const rect=await visibleLayerControl('camera.projection_zoom');
      await enterNumber(rect,nextZoom);await frame();await pause(150);
      rootMetrics=await evaluate('window.obrushBrushMetrics()');rootPath=projectRoot(rootMetrics.camera);
      result.camera_match.typed.push({attempt:attempt+1,old_zoom:oldZoom,typed_zoom:nextZoom,reported_zoom:rootMetrics.camera.source_framing?.projection_zoom,span_css:span(rootPath),camera:rootMetrics.camera});
    }
    result.camera_match.final={span_css:span(rootPath),relative_error:Math.abs(span(rootPath)/targetSpan-1),camera:rootMetrics.camera};
    insist(result.camera_match.final.relative_error<=result.frozen.camera_span_relative_tolerance,`OBRush visible-camera span did not match reference: ${JSON.stringify(result.camera_match)}`);
    result.obrush_path=rootPath;validatePath(rootPath,rootMetrics.camera.viewport,'OBRush');
    insist(span(rootPath)>37.5,'OBRush held path remains below pinned 0.15 × 250 spacing gate');
    insist(rootMetrics.display_flags.brush_symmetry_enabled && rootMetrics.kind==='Draw' && [null,'None'].includes(rootMetrics.alpha) && !rootMetrics.continuous?.requested && ownHalf.info.mode==='SourceOrdinary' && rootMetrics.source_geometry.per_tool[0].radius===result.frozen.radius && rootMetrics.source_geometry.per_tool[0].options.intensity===0.5 && rootMetrics.source_geometry.per_tool[0].radius_factor===0.75,'OBRush held-stroke controls differ');
    await parkCursor();const ownBeforePng=await screenshot('obrush-held-before');
    result.obrush_vertices_before=await saveRootVertices('before');
    const ownEventIndex=await evaluate('window.__symmetryPair.events.length');
    result.obrush_held_before=await evaluate(ownInfo);
    await heldStroke(rootPath);
    result.obrush_held_after=await evaluate(ownInfo);
    result.obrush_events=await evaluate(`window.__symmetryPair.events.slice(${ownEventIndex})`);
    insist(result.obrush_events.some(e=>e.type==='pointerdown'&&e.trusted) && result.obrush_events.some(e=>e.type==='pointerup'&&e.trusted) && result.obrush_events.some(e=>e.type==='pointermove'&&e.buttons===1&&e.trusted),'OBRush trusted held pointer lifecycle missing');
    await parkCursor();const ownAfterPng=await screenshot('obrush-held-after');
    result.obrush_vertices_after=await saveRootVertices('after');
    result.obrush_pixel_change=changedPixels(ownBeforePng,ownAfterPng,rootPath[0],result.frozen.pixel_roi_radius,result.frozen.pixel_channel_threshold);
    result.pixel_changed_ratio=result.obrush_pixel_change.changed_pixels/result.reference_pixel_change.changed_pixels;
    const completed=result.obrush_held_after.source_geometry.last_completed?.capture;
    insist(completed?.mirror_triangle===0 && completed?.mirror_plane,`OBRush held mirror report lacks active triangle/plane: ${JSON.stringify(completed)}`);
    const perTool=result.obrush_held_after.source_geometry.per_tool[0];
    const rootCapture={...completed,per_tool_radius:perTool.radius,per_tool_radius_factor:perTool.radius_factor};
    result.obrush_radius_chain=rootRadiusChain(rootCapture);
    const refCalls=result.reference_radius_calls.filter(c=>c.pointer?.trusted&&c.pointer.buttons===1&&!c.ignore_pressure&&Number.isFinite(c.local_radius_squared));
    insist(refCalls.length>0,'Reference had no trusted held original-call radius trace');
    result.reference_radius_chain=refCalls.at(-1);
    result.reference_radius_chain_candidates=refCalls.length;
    const s=result.reference_radius_chain,o=result.obrush_radius_chain;
    const sourceDistance=Math.hypot(...s.edge.map((v,i)=>v-s.world[i]));
    insist(chainClose(sourceDistance*sourceDistance,s.world_radius_squared),'Reference actual project/unproject distance disagrees with original picker radius');
    insist(chainClose(s.world_radius_squared/s.mesh_scale_squared,s.local_radius_squared),'Reference mesh-scale/local radius is inconsistent');
    insist(chainClose(s.local_radius_squared,result.reference_held_pick.radius_squared),'Reference primary and held mirrored radius disagree');
    insist(chainClose(s.unproject_screen[0]-s.projected[0],s.offset_x),'Reference original-call X offset is inconsistent');
    insist(chainClose(s.offset_x,s.screen_radius*(1+0.75*(s.pointer.pressure*2-1))),'Reference raw pointer pressure does not explain original-call X offset');
    insist(chainClose(o.expected_local_radius_squared,o.reported_local_radius_squared),'OBRush independent matrix radius disagrees with completed Source pass');
    insist(chainClose(o.reported_local_radius_squared,o.mirror_local_radius_squared),'OBRush primary/mirror radius differs');
    const refPhysical=s.offset_x/(s.canvas.backing[0]/s.canvas.client[0]);
    const ownPhysical=o.offset_x;
    result.radius_chain_comparison={reference_css_radius:refPhysical,obrush_css_radius:ownPhysical,physical_ratio:ownPhysical/refPhysical,reference_local_radius:Math.sqrt(s.local_radius_squared),obrush_local_radius:Math.sqrt(o.reported_local_radius_squared),reference_pressure:s.pointer.pressure,obrush_pressure:o.pressure,reference_pressure_factor:s.pressure_factor,obrush_pressure_factor:o.pressure_factor,reference_screen_radius:s.screen_radius,obrush_screen_radius:o.radius,reference_pixel_ratio:s.pixel_ratio,reference_mirror_local_radius_squared:result.reference_held_pick.radius_squared};
    result.radius_chain_comparison.fourfold_reproduced=Math.abs(result.radius_chain_comparison.physical_ratio/4-1)<=result.frozen.ratio_relative_tolerance;
    result.radius_chain_comparison.pressure_input_explains_fourfold=result.radius_chain_comparison.fourfold_reproduced&&close(s.pointer.pressure,0,1e-6)&&close(o.pressure,0.5,1e-6)&&close(s.pressure_factor,0.25,result.frozen.chain_relative_tolerance)&&close(o.pressure_factor,1,result.frozen.chain_relative_tolerance);
    insist(result.obrush_pixel_change.changed_pixels>=result.frozen.min_changed_pixels,'OBRush held screenshot ROI did not change');
    insist(JSON.stringify(result.obrush_held_before.ids)===JSON.stringify(result.obrush_held_after.ids),'OBRush peer object IDs changed');
    result.status = 'passed scoped C256 original-call radius-chain diagnostic; full rows remain open';
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
