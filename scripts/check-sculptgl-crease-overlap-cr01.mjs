// CR01 preregistered repeated default Crease and pinned reference quality pair.
// Usage: node scripts/check-sculptgl-crease-overlap-cr01.mjs WEB_DIST PINNED_HOST_MIRROR --side=source|obrush [--reference-receipt=PATH]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
const filePath=path;
import { inflateSync } from 'node:zlib';

const web = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.argv[3] || '.tools/sculptgl-reference/host-mirror');
const side=process.argv.find(arg=>arg.startsWith('--side='))?.slice(7);
const referenceReceiptPath=process.argv.find(arg=>arg.startsWith('--reference-receipt='))?.slice('--reference-receipt='.length);
const out = path.resolve('.artifacts/sculptgl-crease-overlap-cr01/' + randomUUID());
await mkdir(out, { recursive: true });
const sha = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const insist = (condition, why) => { if (!condition) throw Error(why); };
const close = (a,b,relative=1e-8) => Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=relative*Math.max(1,Math.abs(a),Math.abs(b));
const chainClose = (a,b,relative=0.01) => Number.isFinite(a)&&Number.isFinite(b)&&Math.abs(a-b)<=Math.max(1e-5,relative*Math.max(Math.abs(a),Math.abs(b)));
const mousePressureEvent = (type,x,y,buttons,force) => ({type,x,y,button:'left',buttons,pointerType:'mouse',force,clickCount:type==='mouseMoved'?0:1});
function fixturePositions(bytes){
  insist(bytes.length>=140&&bytes.readUInt32LE(0)===3,'Frozen SGL v3 header differs');
  const count=bytes.readUInt32LE(136);
  insist(count===18242&&140+count*12<=bytes.length,'Frozen SGL vertex section differs');
  return Array.from({length:count},(_,i)=>[0,1,2].map(k=>bytes.readFloatLE(140+i*12+k*4)));
}
function baselineMap(referenceFlat,authored){
  insist(referenceFlat.length===authored.length*3&&referenceFlat.every(Number.isFinite)&&authored.every(v=>v.length===3&&v.every(Number.isFinite)),'Baseline correspondence shape or finite check failed');
  const key=v=>v.map(x=>Math.round(x*10000)).join(',');
  const bins=new Map();
  for(let i=0;i<authored.length;i++){const k=key(authored[i]),q=bins.get(k)??[];q.push(i);bins.set(k,q);}
  const used=new Set(),map=[];let maxError=0;
  for(let i=0;i<authored.length;i++){
    const v=referenceFlat.slice(3*i,3*i+3);
    const nearby=j=>!used.has(j)&&v.every((x,k)=>Math.abs(x-authored[j][k])<1e-6);
    let candidates=(bins.get(key(v))??[]).filter(nearby);
    if(candidates.length===0)candidates=authored.map((_,j)=>j).filter(nearby);
    insist(candidates.length===1,`Baseline reference vertex ${i} has ${candidates.length} unique coordinate candidates`);
    const j=candidates[0];used.add(j);map.push(j);
    maxError=Math.max(maxError,...v.map((x,k)=>Math.abs(x-authored[j][k])));
  }
  insist(used.size===authored.length&&maxError<1e-6,'Baseline correspondence is incomplete');
  return {map,maxError,unchangedIds:map.filter((j,i)=>j===i).length};
}
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
  const radius=capture.per_tool_radius??120, factor=capture.per_tool_radius_factor??0.75;
  const offset=radius*(1+factor*(pressure*2-1));
  const projected=project64(view,vp,world),edge=unproject64(inverse4(view),vp,[projected[0]+offset,projected[1],projected[2]]);
  const worldRadius=Math.hypot(...edge.map((v,i)=>v-world[i]));
  const poseScale2=pose[0]*pose[0]+pose[1]*pose[1]+pose[2]*pose[2];
  return {ordinal:row.ordinal,pressure,radius,radius_factor:factor,pressure_factor:offset/radius,offset_x:offset,world,projected,edge,world_radius:worldRadius,pose_scale_squared:poseScale2,expected_local_radius_squared:worldRadius*worldRadius/poseScale2,reported_local_radius_squared:pass.local_radius_squared,mirror_local_radius_squared:row.passes[1]?.local_radius_squared??null,pass_center:pass.center,screen:row.screen64,viewport:vp};
}
if(process.argv.includes('--self-check')){
  const fixtureSelf=fixturePositions(await readFile('scripts/fixtures/independent-crease-overlap-sphere.sgl'));
  insist(fixtureSelf.length===18242&&JSON.stringify(fixtureSelf[0])==='[0,1,0]','Frozen SGL vertex decoder self-check failed');
  const triples=[[0,0,0],[1,0.25,0.5],[-1,0.1,0.2]];
  const mapped=baselineMap([...triples[2],...triples[0],...triples[1]],triples);
  insist(JSON.stringify(mapped.map)==='[2,0,1]'&&mapped.maxError===0,'Permuted baseline self-check failed');
  for(const invalid of [[[0,0,0],[0,0,0],[-1,0.1,0.2]],[[0,0,0],[1,0.25,0.5],[8,8,8]]]){
    let rejected=false;try{baselineMap([...triples[2],...triples[0],...triples[1]],invalid);}catch{rejected=true;}
    insist(rejected,'Duplicate/missing-coordinate negative control was accepted');
  }
  const press=mousePressureEvent('mousePressed',10,20,1,0.5),move=mousePressureEvent('mouseMoved',12,20,1,0.5),release=mousePressureEvent('mouseReleased',12,20,0,0);
  insist([press,move].every(e=>e.pointerType==='mouse'&&e.force===0.5&&e.buttons===1)&&release.force===0&&release.buttons===0,'CR01 mouse-event contract self-check failed');
  insist(assertHeldMouseEvents([{type:'pointermove',buttons:1,pressure:0,pointerType:'mouse',trusted:true},{type:'pointerdown',buttons:1,pressure:0.5,pointerType:'mouse',trusted:true},{type:'pointermove',buttons:1,pressure:0.5,pointerType:'mouse',trusted:true}],'self-check').length===2,'CR01 pre-contact hover was counted as a held event');
  const m=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],vp=[0,0,100,100],world=[0.1,-0.2,0.3],p=project64(m,vp,world),again=unproject64(inverse4(m),vp,p);
  insist(world.every((v,i)=>close(v,again[i])),'CR01 project/inverse self-check failed');
  const moved=unproject64(inverse4(m),vp,[p[0]+20,p[1],p[2]]);
  insist(close(moved[0]-world[0],0.4),'CR01 X-offset self-check failed');
  const flat=[[0,0,0],[1,0,0],[0,1,0]],tri=[[0,1,2]];
  const healthy=safety(flat,flat,tri),fold=safety(flat,[[0,0,0],[1,0,0],[0,-1,0]],tri),tiny=safety(flat,[[0,0,0],[1,0,0],[0,0,0]],tri),stretch=safety(flat,[[0,0,0],[5,0,0],[0,1,0]],tri);
  insist(healthy.nonfinite===0&&healthy.flipped_faces===0&&healthy.new_tiny_faces===0&&healthy.max_edge_stretch===1,'CR01 healthy triangle negative control failed');
  insist(fold.flipped_faces===1&&tiny.new_tiny_faces===1&&stretch.max_edge_stretch>4,'CR01 safety alarm controls failed');
  if(process.env.C261_C255_RECEIPT){
    const old=JSON.parse(await readFile(process.env.C261_C255_RECEIPT));
    const capture=old.obrush_held_after?.source_geometry?.last_completed?.capture;
    const tool=old.obrush_held_after?.source_geometry?.per_tool?.[0];
    insist(capture&&tool,'C255 self-check receipt lacks Source pass');
    const chain=rootRadiusChain({...capture,per_tool_radius:tool.radius,per_tool_radius_factor:tool.radius_factor});
    insist(chainClose(chain.expected_local_radius_squared,chain.reported_local_radius_squared),'C255 captured-matrix replay differs from Source pass');
    insist(close(chain.offset_x,250),'C255 captured offset was not 250 CSS px');
    console.log('C261 C255 captured-matrix replay passed');
  }
  console.log('CR01 fixture, mouse, matrix and mesh-safety self-check passed');process.exit(0);
}
insist(['source','obrush'].includes(side),'CR01 requires --side=source or --side=obrush');
insist(side==='source'||referenceReceiptPath,'OBRush CR01 requires --reference-receipt');
const fixture = await readFile('scripts/fixtures/independent-crease-overlap-sphere.sgl');
insist(sha(fixture)==='572bed8539b5fb2606f9362e12e94964aa24b2b232f404197b8ab930346aaa08'&&fixture.length===951784,'Independent curved fixture identity changed');
const authoredPositions=fixturePositions(fixture);
const fixturePath = path.join(out, 'independent-crease-overlap-sphere.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned reference bundle differs');
const result = {
  scope:'CR01 repeated default Crease geometry safety and descriptive pinned SculptGL visual comparison; full rows open',
  side,
  source:{commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',bundle_sha256:sha(bundle),wrapper_sha256:sha(await readFile(path.join(reference,'index.html')))},
  build:{base_commit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),wasm_sha256:sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm')))},
  fixture:{sha256:sha(fixture),bytes:fixture.length,vertices:18242,faces:18432},
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  frozen:{strokes:8,points_per_stroke:13,world_radius:0.18,strength:0.35,x:0.25,y_start:-0.18,y_end:0.18,symmetry_x:true,pressure:0.5,pixel_roi_radius:180,pixel_threshold:4,changed_pixel_min:500,edge_stretch_max:4,triangle_area_min:1e-10,orientation_baseline_area_min:1e-7,camera_span_relative_max:0.01},
  cases:[],screenshots:[],errors:[],status:'running',
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
async function digit(n){
  for(const type of ['rawKeyDown','keyUp'])await send('Input.dispatchKeyEvent',{type,key:String(n),code:`Digit${n}`,windowsVirtualKeyCode:48+n});
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
function safety(before,after,triangles){
  if(Array.isArray(triangles[0]))triangles=triangles.flat();
  insist(before.length===after.length&&triangles.length%3===0,'Mesh safety arrays differ');
  let changed=0,maxDisplacement=0,nonfinite=0,tiny=0,flips=0,maxStretch=0,minAreaRatio=Infinity;
  const worst=[];for(let i=0;i<after.length;i++){
    const a=before[i],b=after[i],d=Math.hypot(b[0]-a[0],b[1]-a[1],b[2]-a[2]);
    if(d>1e-6)changed++;maxDisplacement=Math.max(maxDisplacement,d);
    if(!b.every(x=>Number.isFinite(x)&&Math.abs(x)<=1e6))nonfinite++;
  }
  const cross=(u,v)=>[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
  const sub=(u,v)=>u.map((x,k)=>x-v[k]);
  for(let face=0;face<triangles.length/3;face++){
    const ids=triangles.slice(face*3,face*3+3);insist(ids.every(id=>Number.isInteger(id)&&id>=0&&id<after.length),'Out-of-range triangle index');
    const [a,b,c]=ids.map(id=>before[id]),[d,e,f]=ids.map(id=>after[id]);
    const n=cross(sub(b,a),sub(c,a)),m=cross(sub(e,d),sub(f,d));
    const area=Math.hypot(...n)/2,newArea=Math.hypot(...m)/2;
    if(area>1e-10&&newArea<1e-10)tiny++;
    const moved=ids.some(id=>before[id].some((v,k)=>Math.abs(after[id][k]-v)>1e-6));
    if(area>1e-7&&moved&&n.reduce((q,v,k)=>q+v*m[k],0)<=0)flips++;
    if(area>1e-10)minAreaRatio=Math.min(minAreaRatio,newArea/area);
    for(const [u,v] of [[0,1],[1,2],[2,0]]){
      const old=Math.hypot(...sub(before[ids[u]],before[ids[v]])),now=Math.hypot(...sub(after[ids[u]],after[ids[v]]));
      if(old>1e-12){const stretch=now/old;maxStretch=Math.max(maxStretch,stretch);worst.push({face,edge:[ids[u],ids[v]],stretch});}
    }
  }
  return {changed_vertices:changed,max_displacement:maxDisplacement,nonfinite,new_tiny_faces:tiny,flipped_faces:flips,max_edge_stretch:maxStretch,min_area_ratio:Number.isFinite(minAreaRatio)?minAreaRatio:null,worst_edges:worst.sort((a,b)=>b.stretch-a.stretch).slice(0,10)};
}
async function frame(){await evaluate('new Promise(requestAnimationFrame)');}
async function parkCursor(){await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:1550,y:970,buttons:0});await frame();await pause(120);}
async function heldStroke(points){
  const [first,...rest]=points;await send('Input.dispatchMouseEvent',mousePressureEvent('mouseMoved',first[0],first[1],0,0));
  await send('Input.dispatchMouseEvent',mousePressureEvent('mousePressed',first[0],first[1],1,0.5));
  await frame();await pause(50);
  for(const point of rest){await send('Input.dispatchMouseEvent',mousePressureEvent('mouseMoved',point[0],point[1],1,0.5));await frame();await pause(50);}
  const last=points.at(-1);await send('Input.dispatchMouseEvent',mousePressureEvent('mouseReleased',last[0],last[1],0,0));
  for(let i=0;i<4;i++)await frame();await pause(120);
}
function validatePath(points,rect,label){insist(points.every(([x,y])=>Number.isFinite(x)&&Number.isFinite(y)&&x>rect[0]+2&&x<rect[2]-2&&y>rect[1]+2&&y<rect[3]-2),`${label} projected path outside sculpt canvas ${JSON.stringify({points,rect})}`);}
function assertHeldMouseEvents(events,label){
  const down=events.findIndex(e=>e.type==='pointerdown'&&e.buttons===1);
  insist(down>=0,`${label} missing trusted held mouse down`);
  const held=events.slice(down).filter(e=>(e.type==='pointerdown'||e.type==='pointermove')&&e.buttons===1);
  insist(held.some(e=>e.type==='pointerdown')&&held.some(e=>e.type==='pointermove'),`${label} missing trusted held mouse down/move`);
  insist(held.every(e=>e.trusted&&e.pointerType==='mouse'&&Math.abs(e.pressure-0.5)<=1e-6),`${label} held mouse was not trusted pressure 0.5: ${JSON.stringify(held)}`);
  return held.map(e=>({type:e.type,pointerType:e.pointerType,pressure:e.pressure,x:e.x,y:e.y}));
}
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
async function sourceIntensity(value){
  const expr="(()=>{const e=Array.from(window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)[1],r=e?.getBoundingClientRect();return r?{value:Number(e.value),rect:[r.left,r.top,r.right,r.bottom]}:null})()";
  const q=await evaluate(expr);insist(q?.rect,'Visible reference Crease intensity input absent');await enterNumber(q.rect,value*100);
  const actual=await evaluate(expr);insist(Math.abs(actual.value-value*100)<=0.01,`Reference Crease intensity field differs: ${JSON.stringify(actual)}`);
}
async function sourceSceneControl(name){
  const expression=`(()=>{const c=window.__sculptglReferenceInstance?.getGui()?._ctrlScene?.[${JSON.stringify(name)}];if(!c)return null;return Object.keys(c).filter(k=>k.startsWith('dom')).map(k=>{const e=c[k],r=e?.getBoundingClientRect?.();return {key:k,tag:e?.tagName,type:e?.type,rect:r?[r.left,r.top,r.right,r.bottom]:null}})})()`;
  const controls=await evaluate(expression);
  result.reference_visible_controls??={};result.reference_visible_controls[name]=controls;
  const control=controls?.find(q=>q.rect&&q.rect[2]>q.rect[0]&&q.rect[3]>q.rect[1]&&(name==='_ctrlOffSym'?q.tag==='INPUT'&&q.type==='number':q.tag==='LI'));
  insist(control,`Reference visible ${name} input missing: ${JSON.stringify(controls)}`);
  return control.rect;
}
async function openReferenceScene(){
  const x=197,y=20; // Captured visible Scene menu at 1600x1000 DPR1.
  for(const type of ['mousePressed','mouseReleased'])await send('Input.dispatchMouseEvent',{type,x,y,button:'left',clickCount:1});
  await pause(160);
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
const refInfo = `(()=>{const a=window.__sculptglReferenceInstance;if(!a)return null;const sm=a.getSculptManager(),tool=sm.getCurrentTool(),canvas=a.getCanvas(),r=canvas.getBoundingClientRect();return {canvas_rect:[r.left,r.top,r.right,r.bottom],tool:{index:sm.getToolIndex(),symmetry:sm.getSymmetry(),radius:tool._radius,intensity:tool._intensity,negative:tool._negative,alpha:tool._idAlpha,continuous:sm._continuous,culling:tool._culling,lock:tool._lockPosition},meshes:a.getMeshes().length,active:a.getMeshes().indexOf(a.getMesh()),selected:a.getSelectedMeshes().map(m=>a.getMeshes().indexOf(m)),objects:a.getMeshes().map(m=>({vertices:m.getNbVertices(),faces:m.getNbFaces(),positions:m.getNbVertices()<=20000?Array.from(m.getVertices()):null,matrix:Array.from(m.getMatrix()),center:Array.from(m._transformData._center),normal:Array.from(m.getSymmetryNormal()),offset:m.getSymmetryOffset(),loose:Array.from(m.getLocalBound()),plane:Array.from(m.getSymmetryOrigin()),visible:m.isVisible()})),states:a.getStateManager()._undos.length}})()`;
const ownInfo = `(()=>{const m=window.obrushBrushMetrics?.();return m?{camera:m.camera,source_geometry:m.source_geometry,brush:{kind:m.kind,radius:m.radius,strength:m.strength,invert:m.invert,alpha:m.alpha,continuous:m.continuous,pressure:m.pressure,pressure_radius:m.pressure_radius,pressure_strength:m.pressure_strength,adapt_detail:m.adaptive_detail,symmetry_x:m.symmetry_x},brush_options:m.brush_options,subtools:m.subtools.length,ids:m.subtools.map(x=>x.id),active:m.active_subtool_id,selected:m.selected_subtool_ids,controls:m.source_geometry_controls_hit_rect,scene_controls:m.scene_controls,solo:m.solo,mode:m.source_geometry?.per_tool?.[6]?.mode??null,offset:m.source_geometry?.active_object_symmetry_offset??null,display:m.display_flags,history:m.history,scene_error:m.scene_error,viewport:m.viewport}:null})()`;
const installReferenceRadiusTrace = `(()=>{
  const a=window.__sculptglReferenceInstance,p=a.getPicking(),log=window.__symmetryPair.radiusCalls;
  if(p.__c261RadiusWrapped)return false;
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
  p.__c261RadiusWrapped=true;return true;
})()`;
async function visibleLayerControl(key){
  const tried=[];
  for(let i=0;i<24;i++){
    const rect=await evaluate(`window.obrushBrushMetrics()?.layer_controls?.[${JSON.stringify(key)}]??null`);
    tried.push(rect);
    if(rect&&rect[1]>72&&rect[3]<985)return rect;
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:1500,y:760,buttons:0});
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:1500,y:760,deltaX:0,deltaY:360});await pause(110);
  }
  const m=await evaluate('window.obrushBrushMetrics()');
  result.camera_control_debug={tried,layer_keys:Object.keys(m.layer_controls??{}),source_enabled:!!m.camera?.source_framing,viewport:m.camera?.viewport};
  await screenshot('camera-control-missing');
  throw Error(`Visible Source camera numeric control absent: ${key}; final rect ${JSON.stringify(tried.at(-1))}`);
}
async function saveRootVertices(label){
  const before=await evaluate('window.__symmetryPair.downloads.length');
  result.save_trace??=[];result.save_trace.push({label,before});
  for(const type of ['mousePressed','mouseReleased'])await send('Input.dispatchMouseEvent',{type,x:345,y:26,button:'left',clickCount:1});
  const metadata=await until(async()=>{const d=await evaluate('(()=>{const q=window.__symmetryPair.downloads,d=q.at(-1);return d?{sequence:d.sequence,name:d.name,bytes:d.contents.length}:null})()');return d&&d.sequence>before?d:null;},`OBRush normal ${label} .obrush Save`);
  result.save_trace.at(-1).after=metadata;
  insist(metadata.name.endsWith('.obrush')&&!metadata.name.includes('/')&&!metadata.name.includes('\\'),`OBRush normal Save returned ${metadata.name}`);
  const downloaded=path.join(out,metadata.name);
  const bytes=await until(async()=>{try{return await readFile(downloaded);}catch{return null;}},`OBRush normal ${label} .obrush download`);
  insist(bytes.length===metadata.bytes,`OBRush downloaded ${label} bytes differ from host text`);
  await rename(downloaded,path.join(out,`obrush-${label}.obrush`));
  const doc=JSON.parse(bytes.toString('utf8')),positions=doc.mesh?.positions;
  insist(Array.isArray(positions)&&positions.length===18242,`OBRush ${label} active curved mesh is not 18242 vertices`);
  const triangles=doc.mesh?.triangles,active=doc.scene?.active_id,peer=doc.scene?.objects?.find(x=>x.id!==active);
  insist(Array.isArray(triangles)&&triangles.length===36480&&peer,'OBRush saved triangle/hidden peer contract differs');
  return {name:metadata.name,sha256:sha(bytes),bytes:bytes.length,positions,triangles,peer_id:peer.id,peer_sha256:sha(Buffer.from(JSON.stringify(peer)))};
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
  await send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:out,eventsEnabled:true},null);
  await send('Page.setInterceptFileChooserDialog', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
  const base = `http://127.0.0.1:${server.address().port}`;

  const span=p=>Math.hypot(p.at(-1)[0]-p[0][0],p.at(-1)[1]-p[0][1]);
  const localPath=Array.from({length:13},(_,i)=>{const x=.25,y=-.18+i*.03;return [x,y,Math.sqrt(1-x*x-y*y)];});
  const sourcePositions=()=>evaluate('Array.from(window.__sculptglReferenceInstance.getMesh().getVertices())');
  const metrics=()=>evaluate('window.obrushBrushMetrics()');
  const awaitIdle=async()=>until(async()=>{const m=await metrics();return m&&!(m.pending_dabs>0)&&!(m.browser_pen_moves_queued>0)?m:null;},'OBRush pending sculpt work');
  const projectRoot=(cam)=>{const vp=cam.viewport,q=cam.view_projection;return localPath.map(([x,y,z])=>{const p=[x,y,z,1],c=[0,1,2,3].map(k=>q[k]*p[0]+q[4+k]*p[1]+q[8+k]*p[2]+q[12+k]*p[3]);return [vp[0]+(c[0]/c[3]+1)*(vp[2]-vp[0])/2,vp[1]+(1-c[1]/c[3])*(vp[3]-vp[1])/2]})};
  if(side==='source'){
    await send('Page.navigate',{url:`${base}/reference/`});
    await until(async()=>(await evaluate(refInfo))?.meshes===1,'reference startup');
    await fileInput('#fileopen');await until(async()=>(await evaluate(refInfo))?.meshes===2,'reference sphere import');
    let q=await evaluate(refInfo);result.import={active:q.active,objects:q.objects.map(({positions,...m})=>m)};
    insist(q.objects[q.active].vertices===18242,'Imported sphere vertex count differs');
    await screenshot('source-import');await openReferenceScene();await click(await sourceSceneControl('_ctrlIsolate'));
    q=await evaluate(refInfo);insist(q.objects.filter(m=>m.visible).length===1&&q.objects[q.active].visible,'Source Solo failed');
    await digit(7);await pause(160);q=await evaluate(refInfo);
    insist(q.tool.index===6&&q.tool.symmetry&&q.tool.negative&&!q.tool.culling&&!q.tool.lock&&!q.tool.continuous&&[0,'None'].includes(q.tool.alpha),'Source Crease setup differs');
    await sourceIntensity(.35);
    const path=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),cam=a.getCamera(),r=a.getCanvas().getBoundingClientRect(),p=${JSON.stringify(localPath)};return p.map(v=>{const q=m.getMatrix(),w=[q[0]*v[0]+q[4]*v[1]+q[8]*v[2]+q[12],q[1]*v[0]+q[5]*v[1]+q[9]*v[2]+q[13],q[2]*v[0]+q[6]*v[1]+q[10]*v[2]+q[14]],s=cam.project(w);return [r.left+s[0],r.top+s[1]]})})()`);
    result.path=path;validatePath(path,q.canvas_rect,'Source');insist(span(path)>18,'Source path shorter than 18px');
    const projection=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),cam=a.getCamera(),v=${JSON.stringify(localPath[6])},q=m.getMatrix();const toWorld=p=>[q[0]*p[0]+q[4]*p[1]+q[8]*p[2]+q[12],q[1]*p[0]+q[5]*p[1]+q[9]*p[2]+q[13],q[2]*p[0]+q[6]*p[1]+q[10]*p[2]+q[14]];const u=cam.project(toWorld(v)),w=cam.project(toWorld([v[0]+.18,v[1],v[2]]));return {radius_css:Math.hypot(w[0]-u[0],w[1]-u[1]),middle_css:u}})()`);
    result.radius_projection=projection;const sourcePx=Math.round(projection.radius_css);insist(sourcePx>=5&&sourcePx<=500,'Source projected radius outside numeric UI');await sourceRadius(sourcePx);
    q=await evaluate(refInfo);result.controls=q.tool;insist(q.tool.radius===sourcePx&&Math.abs(q.tool.intensity-.35)<1e-6,'Source visible Crease numeric fields differ');
    result.path_hits=await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,m=a.getMesh(),r=a.getCanvas().getBoundingClientRect(),p=${JSON.stringify(path)};return p.map(([x,y])=>({direct:a.getPicking().intersectionMouseMesh(m,x-r.left,y-r.top),mirror:a.getPickingSymmetry().intersectionMouseMesh(m,x-r.left,y-r.top)}))})()`);
    insist(result.path_hits.every(x=>x.direct&&x.mirror),'Source preflight ray missed front or mirror');
    const tris=await evaluate('Array.from(window.__sculptglReferenceInstance.getMesh().getTriangles())');
    result.baseline={positions:await sourcePositions(),triangles:tris,peer_hash:sha(Buffer.from(JSON.stringify(q.objects.filter((_,i)=>i!==q.active)))) ,history:q.states};
    result.checkpoints=[result.baseline.positions];await parkCursor();let initial=await screenshot('source-baseline');
    await evaluate(`(()=>{const a=window.__sculptglReferenceInstance,t=a.getSculptManager().getCurrentTool(),f=t.makeStroke,c=t.crease;window.__symmetryPair.strokeCalls=[];window.__symmetryPair.creaseCalls=[];t.makeStroke=function(...x){window.__symmetryPair.strokeCalls.push({at:performance.now(),args:x.slice(0,2)});return f.apply(this,x)};t.crease=function(...x){window.__symmetryPair.creaseCalls.push({at:performance.now(),count:x[0]?.length??null});return c.apply(this,x)}})()`);
    for(let i=0;i<8;i++){
      const events=await evaluate('window.__symmetryPair.events.length'),calls=await evaluate('window.__symmetryPair.creaseCalls.length');
      await heldStroke(i%2?[...path].reverse():path);await parkCursor();
      const p=await sourcePositions(),im=await screenshot(`source-stroke-${i+1}`),ev=await evaluate(`window.__symmetryPair.events.slice(${events})`),admitted=await evaluate(`window.__symmetryPair.creaseCalls.slice(${calls})`);
      const mouse=assertHeldMouseEvents(ev,`Source stroke ${i+1}`);
      const s=safety(result.baseline.positions.reduce((a,_,k)=>{if(k%3===0)a.push(result.baseline.positions.slice(k,k+3));return a;},[]),p.reduce((a,_,k)=>{if(k%3===0)a.push(p.slice(k,k+3));return a;},[]),tris);
      result.cases.push({stroke:i+1,events:mouse,admitted:admitted.length,safety:s,pixel_change:changedPixels(initial,im,path[6],180,4),history:(await evaluate(refInfo)).states});result.checkpoints.push(p);
      insist(admitted.length>=2,`Source stroke ${i+1} had fewer than two admitted Crease calls`);
      if(i===3)result.stroke4_image='source-stroke-4.png';
    }
    result.final_change=changedPixels(initial,await readFile(filePath.join(out,'source-stroke-8.png')),path[6],180,4);
    insist(result.final_change.changed_pixels>=500,'Source final ROI appears no-op');
    result.history_steps=[];
    for(let i=7;i>=0;i--){await key('z',2);await pause(170);const p=await sourcePositions();result.history_steps.push({kind:'undo',to:i,exact:JSON.stringify(p)===JSON.stringify(result.checkpoints[i])});insist(result.history_steps.at(-1).exact,`Source Undo ${8-i} differs`);}
    for(let i=1;i<=8;i++){await key('y',2);await pause(170);const p=await sourcePositions();result.history_steps.push({kind:'redo',to:i,exact:JSON.stringify(p)===JSON.stringify(result.checkpoints[i])});insist(result.history_steps.at(-1).exact,`Source Redo ${i} differs`);}
    result.gpu=await evaluate(gpu);insist(result.gpu.gl_error===0,'Source WebGL error');result.status='passed scoped Source Crease observations';
  }else{
    const source=JSON.parse(await readFile(referenceReceiptPath));result.reference_receipt={path:filePath.resolve(referenceReceiptPath),sha256:sha(await readFile(referenceReceiptPath)),status:source.status};
    insist(source.status.startsWith('passed')&&source.fixture.sha256===result.fixture.sha256,'Source receipt not passing or fixture differs');
    await send('Page.navigate',{url:`${base}/?obrushprofile=sculptgl`});await until(async()=>(await evaluate(ownInfo))?.subtools===1,'OBRush startup');
    await evaluate("(()=>{const f=window.obrushDownload;window.__symmetryPair.downloads=[];window.obrushDownload=function(n,c){window.__symmetryPair.downloads.push({sequence:window.__symmetryPair.downloads.length+1,name:n,contents:c});return f(n,c)}})()");
    await evaluate("document.getElementById('obrush_canvas').focus()");const beforeChooser=choosers.length;await key('o',2);await until(async()=>choosers.length>beforeChooser,'OBRush import chooser');await fileInput('#source-sgl-file');
    await until(async()=>(await evaluate(ownInfo))?.subtools===2,'OBRush sphere import');
    let q=await evaluate(ownInfo);await click(q.scene_controls['scene.solo']);q=await evaluate(ownInfo);insist(q.solo,'OBRush Solo failed');await click(q.scene_controls['scene.frame']);await pause(300);
    await screenshot('obrush-before-crease-selection');
    await click([140,250,196,316]);q=await evaluate(ownInfo);result.controls=q.brush;
    insist(q.brush.kind==='Crease'&&Math.abs(q.brush.radius-.18)<1e-6&&Math.abs(q.brush.strength-.35)<1e-6&&q.brush.symmetry_x&&!q.brush.invert&&[null,'None'].includes(q.brush.alpha)&&!q.brush.continuous?.requested,'OBRush default Crease controls differ');
    let m=await metrics(),path=projectRoot(m.camera);result.camera_match={initial_span_css:span(path),events:[]};
    for(let step=0;step<32&&span(path)<30;step++){const vp=m.camera.viewport,x=(vp[0]+vp[2])/2,y=(vp[1]+vp[3])/2;await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y,buttons:0});await send('Input.dispatchMouseEvent',{type:'mouseWheel',x,y,deltaX:0,deltaY:-360});await frame();await pause(30);m=await metrics();path=projectRoot(m.camera);result.camera_match.events.push({step:step+1,span_css:span(path)});}
    insist(span(path)>=30,'OBRush normal wheel could not reach stroke spacing');
    for(let i=0;i<3;i++){if(Math.abs(span(path)/span(source.path)-1)<=.01)break;const z=m.camera.orthographic?2/m.camera.projection[5]:m.camera.source_framing?.projection_zoom;insist(z>0,'OBRush camera zoom unavailable');const next=Number((z*span(path)/span(source.path)).toPrecision(12));await enterNumber(await visibleLayerControl(m.camera.orthographic?'camera.view_height':'camera.projection_zoom'),next);await frame();await pause(150);m=await metrics();path=projectRoot(m.camera);result.camera_match.events.push({typed:next,span_css:span(path)});}
    result.camera_match.final_span_css=span(path);result.camera_match.relative_error=Math.abs(span(path)/span(source.path)-1);insist(result.camera_match.relative_error<=.01,'OBRush camera path span differs from Source');
    result.path=path;validatePath(path,m.camera.viewport,'OBRush');
    await parkCursor();const initial=await screenshot('obrush-baseline');const baseline=await saveRootVertices('baseline');result.baseline={...baseline,positions:undefined,triangles:undefined};
    result.checkpoints=[baseline];result.cases=[];let previous=baseline;
    for(let i=0;i<8;i++){
      const events=await evaluate('window.__symmetryPair.events.length');await heldStroke(i%2?[...path].reverse():path);const done=await awaitIdle();await parkCursor();
      const ev=await evaluate(`window.__symmetryPair.events.slice(${events})`);const mouse=assertHeldMouseEvents(ev,`OBRush stroke ${i+1}`);
      const saved=await saveRootVertices(`stroke-${i+1}`);const im=await screenshot(`obrush-stroke-${i+1}`);
      await pause(200);const t200=await saveRootVertices(`stroke-${i+1}-t200`);await pause(600);const t800=await saveRootVertices(`stroke-${i+1}-t800`);
      const stable=JSON.stringify(saved.positions)===JSON.stringify(t200.positions)&&JSON.stringify(saved.positions)===JSON.stringify(t800.positions);
      const s=safety(baseline.positions,saved.positions,baseline.triangles),step=safety(previous.positions,saved.positions,baseline.triangles);
      const row={stroke:i+1,events:mouse,completed:{pending_dabs:done.pending_dabs,browser_pen_moves_queued:done.browser_pen_moves_queued},stable_after_release:stable,safety:s,step_changed_vertices:step.changed_vertices,pixel_change:changedPixels(initial,im,path[6],180,4),history:(await evaluate(ownInfo)).history,save_sha256:saved.sha256,peer_sha256:saved.peer_sha256};result.cases.push(row);result.checkpoints.push(saved);previous=saved;
      insist(stable,`OBRush geometry moved after release on stroke ${i+1}`);
      insist(s.nonfinite===0&&s.new_tiny_faces===0&&s.flipped_faces===0&&s.max_edge_stretch<=4,`OBRush stroke ${i+1} mesh safety alarm ${JSON.stringify(s)}`);
      insist(saved.peer_sha256===baseline.peer_sha256&&saved.peer_id===baseline.peer_id,`OBRush peer changed on stroke ${i+1}`);
      insist(step.changed_vertices>0,`OBRush stroke ${i+1} had no geometry change`);
    }
    result.final_change=changedPixels(initial,await readFile(filePath.join(out,'obrush-stroke-8.png')),path[6],180,4);insist(result.final_change.changed_pixels>=500,'OBRush final ROI appears no-op');
    result.history_steps=[];for(let i=7;i>=0;i--){await key('z',2);await pause(170);const p=await saveRootVertices(`undo-${8-i}`),exact=JSON.stringify(p.positions)===JSON.stringify(result.checkpoints[i].positions);result.history_steps.push({kind:'undo',to:i,exact,sha256:p.sha256});insist(exact,`OBRush Undo ${8-i} differs`);}
    for(let i=1;i<=8;i++){await key('y',2);await pause(170);const p=await saveRootVertices(`redo-${i}`),exact=JSON.stringify(p.positions)===JSON.stringify(result.checkpoints[i].positions);result.history_steps.push({kind:'redo',to:i,exact,sha256:p.sha256});insist(exact,`OBRush Redo ${i} differs`);}
    result.gpu=await evaluate(gpu);insist(result.gpu.gl_error===0,'OBRush WebGL error');result.status='passed scoped OBrush Crease safety observations';
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
