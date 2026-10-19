// Paired default Source Add placement through both normal browser UIs.
// Usage: SGL_REFERENCE_DIR=/absolute/host-mirror node scripts/check-sculptgl-primitive-placement.mjs [web/dist]
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const root = path.resolve('.');
const obrush = path.resolve(process.argv[2] || 'web/dist');
const reference = path.resolve(process.env.SGL_REFERENCE_DIR || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-primitive-placement/' + randomUUID());
const downloads=path.join(out,'downloads');
await mkdir(downloads, { recursive: true });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
if (sha(bundle) !== manifest.original_bundle.sha256) throw Error('Pinned reference bundle hash changed');
const result = {
  scope: 'Default Add Sphere/Cube/Cylinder/Torus first-object Source pose and world bounds; sequential Add preservation and project Save/Undo',
  source: {commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d', bundle_sha256:sha(bundle), wrapper_sha256:sha(await readFile(path.join(reference,'index.html')))},
  build: {base_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(), wasm_sha256:sha(await readFile(path.join(obrush,'pkg/obrush_app_bg.wasm')))},
  cases: [], errors: [], status:'running'
};
let server, chrome, socket, session, next = 0;
const waiting = new Map(), downloadEvents=[];
async function send(method, params = {}, selected = session) {
  const id = ++next;
  const response = new Promise((resolve,reject) => {
    const timer=setTimeout(()=>{waiting.delete(id);reject(Error(`CDP timeout: ${method}`));},45000);
    waiting.set(id,{resolve,reject,timer});
  });
  socket.send(JSON.stringify({id,method,params,...(selected?{sessionId:selected}:{})}));
  return response;
}
async function evaluate(expression) {
  const response=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});
  if(response.exceptionDetails)throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(fn,reason) {
  for(let i=0;i<250;i++){const value=await fn();if(value)return value;await pause(100);}
  throw Error(`Timed out: ${reason}`);
}
async function click(rect) {
  const x=(rect[0]+rect[2])/2,y=(rect[1]+rect[3])/2;
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});
}
const obrushInfo=`(()=>{const m=window.obrushBrushMetrics?.();return m?{vertices:m.vertices,faces:m.faces,subtools:m.subtools?.length,history:m.history,controls:m.scene_controls,message:m.message}:null})()`;
async function control(key){
  for(let i=0;i<25;i++){
    const m=await evaluate(obrushInfo),r=m?.controls?.[key];
    if(!r)throw Error(`Missing UI control ${key}`);
    const y=(r[1]+r[3])/2;
    if(y>72&&y<940)return r;
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:1480,y:740});
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:1480,y:740,deltaX:0,deltaY:y>=940?350:-350});
    await pause(150);
  }
  throw Error(`Cannot reveal ${key}`);
}
async function action(key){await click(await control(key));await pause(180);}
const referenceInfo=`(()=>{const a=window.__sculptglReferenceInstance,m=a?.getMesh();return a?{parts:a.getMeshes().length,vertices:m?.getNbVertices()??0,faces:m?.getNbFaces()??0,matrix:m?Array.from(m.getMatrix()):null,local:m?Array.from(m.getLocalBound()):null,world:m?Array.from(m.computeWorldBound()):null}:null})()`;
function bounds(positions,matrix){
  const low=[Infinity,Infinity,Infinity],high=[-Infinity,-Infinity,-Infinity];
  for(const p of positions){
    for(let k=0;k<3;k++){
      const q=matrix[k]*p[0]+matrix[k+4]*p[1]+matrix[k+8]*p[2]+matrix[k+12];
      low[k]=Math.min(low[k],q);high[k]=Math.max(high[k],q);
    }
  }
  return [...low,...high];
}
function maximumDiff(a,b){return Math.max(...a.map((x,i)=>Math.abs(x-b[i])));}
async function projectSave(){
  const before=downloadEvents.filter(e=>e.method==='Browser.downloadProgress'&&e.state==='completed').length;
  await click([303,13,387,39]);
  return until(async()=>{
    const completed=downloadEvents.filter(e=>e.method==='Browser.downloadProgress'&&e.state==='completed');
    if(completed.length<=before)return null;
    const event=completed.at(-1),file=path.join(downloads,event.guid),bytes=await readFile(file);
    return {name:event.guid,file,sha256:sha(bytes),project:JSON.parse(bytes.toString('utf8'))};
  },'public project Save');
}
let primaryError;
try {
  server=createServer(async(request,response)=>{
    try{
      const pathname=new URL(request.url,'http://127.0.0.1').pathname;
      const isReference=pathname.startsWith('/reference/');
      const base=isReference?reference:obrush;
      const relative=isReference?pathname.slice('/reference'.length):pathname;
      const file=path.resolve(base,'.'+(relative==='/'?'/index.html':relative));
      if(!file.startsWith(base+path.sep))throw Error('outside served tree');
      const bytes=await readFile(file);
      response.setHeader('Content-Type',({'.wasm':'application/wasm','.js':'text/javascript','.html':'text/html','.css':'text/css'})[path.extname(file)]||'application/octet-stream');
      response.end(bytes);
    }catch{response.writeHead(404);response.end();}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new',`--user-data-dir=${path.join(out,'chrome-profile')}`,'--remote-debugging-port=0','--no-first-run','--disable-background-networking','--window-size=1600,1000','about:blank'],{stdio:['ignore','pipe','pipe']});
  let stderr='';chrome.stderr.on('data',bytes=>{stderr+=bytes;});
  const endpoint=await until(async()=>stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1],'Chrome endpoint');
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',event=>{
    const message=JSON.parse(String(event.data));
    if(message.id){const pending=waiting.get(message.id);if(!pending)return;waiting.delete(message.id);clearTimeout(pending.timer);message.error?pending.reject(Error(JSON.stringify(message.error))):pending.resolve(message.result);}
    else if(message.method==='Runtime.exceptionThrown')result.errors.push(message.params.exceptionDetails.text);
    else if(message.method==='Browser.downloadWillBegin'||message.method==='Browser.downloadProgress')downloadEvents.push({method:message.method,...message.params});
  });
  const target=await send('Target.createTarget',{url:'about:blank'},null);
  session=(await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
  for(const domain of ['Page','Runtime','DOM'])await send(`${domain}.enable`);
  await send('Emulation.setDeviceMetricsOverride',{width:1600,height:1000,deviceScaleFactor:1,mobile:false});
  await send('Browser.setDownloadBehavior',{behavior:'allowAndName',downloadPath:downloads,eventsEnabled:true},null);
  const base=`http://127.0.0.1:${server.address().port}`;
  result.stage='reference';
  await send('Page.navigate',{url:`${base}/reference/`});
  await until(async()=>(await evaluate(referenceInfo))?.parts>0,'reference startup');
  const cases=[['sphere','addSphere'],['cube','addCube'],['cylinder','addCylinder'],['torus','addTorus']];
  const referenceRows={};
  for(const [name,method] of cases){
    await evaluate('(()=>{window.confirm=()=>true;window.__sculptglReferenceInstance.getGui()._ctrlScene.clearScene();return true})()');
    await until(async()=>(await evaluate(referenceInfo))?.parts===0,'reference clear');
    await evaluate(`(()=>{window.__sculptglReferenceInstance.${method}();return true})()`);
    referenceRows[name]=await until(async()=>{const m=await evaluate(referenceInfo);return m?.parts===1&&m.vertices>60000?m:null;},`reference ${name}`);
  }
  result.stage='OBRush';
  await send('Page.navigate',{url:`${base}/?obrushprofile=sculptgl`});
  await until(async()=>(await evaluate(obrushInfo))?.subtools===1,'OBRush startup');
  for(const [name] of cases){
    result.stage=`OBRush ${name}`;
    await action('scene.clear');
    await until(async()=>(await evaluate(obrushInfo))?.subtools===0,'OBRush clear');
    await action(`scene.add_${name}`);
    const ui=await until(async()=>{const m=await evaluate(obrushInfo);return m?.subtools===1&&m.vertices>60000?m:null;},`OBRush Add ${name}`);
    const saved=await projectSave();
    const project=saved.project;
    const matrix=project.scene_v7?.objects?.find(o=>o.id===project.scene?.active_id)?.affine?.local_to_world;
    if(!matrix)throw Error(`Missing ${name} active Source pose in public project`);
    const ownBounds=bounds(project.mesh.positions,matrix);
    const referenceBounds=referenceRows[name].world;
    const worldDiff=maximumDiff(referenceBounds,ownBounds);
    const matrixEquivalent=name==='sphere'||name==='cube'?matrix[0]*2:matrix[0];
    const matrixDiff=Math.abs(referenceRows[name].matrix[0]-matrixEquivalent);
    result.cases.push({name,reference:referenceRows[name],obrush:{ui,saved_name:saved.name,saved_sha256:saved.sha256,matrix,world_bounds:ownBounds,retained_levels:project.multiresolution?.levels?.length??null},comparison:{world_bounds_max_abs_diff:worldDiff,matrix_scale_equivalent_abs_diff:matrixDiff}});
    if(worldDiff>0.0002||matrixDiff>0.00002)throw Error(`${name} default placement exceeds bounded matrix/world error`);
  }
  result.stage='sequential Add/Save/Undo';
  await action('scene.clear');
  for(const [name] of cases)await action(`scene.add_${name}`);
  const sequence=await projectSave();
  const scene=sequence.project.scene, poses=sequence.project.scene_v7?.objects??[];
  const actualScales=poses.map(p=>p.affine.local_to_world[0]);
  const expectedScales=cases.map(([name])=>referenceRows[name].matrix[0]/(name==='sphere'||name==='cube'?2:1));
  const scaleErrors=actualScales.map((value,i)=>Math.abs(value-expectedScales[i]));
  result.sequence={parts:scene.objects.length,active:scene.active_id,pose_ids:poses.map(p=>p.id),positions:scene.objects.map(o=>o.payload?.mesh?.positions?.length??null),save_sha256:sequence.sha256,actual_scales:actualScales,reference_equivalent_scales:expectedScales,scale_errors:scaleErrors};
  if(scene.objects.length!==4||poses.length!==4)throw Error('Sequential Add did not retain four posed objects');
  if(scaleErrors.some(value=>value>0.00002)||poses.some(p=>p.affine.local_to_world.slice(12,15).some(value=>value!==0)))throw Error('Sequential Add pose changed with existing objects');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'z',code:'KeyZ',windowsVirtualKeyCode:90,modifiers:2});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'z',code:'KeyZ',windowsVirtualKeyCode:90,modifiers:2});
  const undone=await until(async()=>{const m=await evaluate(obrushInfo);return m?.subtools===3?m:null;},'sequential Undo');
  result.sequence.undo_parts=undone.subtools;
  await action('scene.clear');
  await until(async()=>(await evaluate(obrushInfo))?.subtools===0,'clear before project Open');
  const input=(await send('DOM.getDocument')).root;
  const node=(await send('DOM.querySelector',{nodeId:input.nodeId,selector:'#import-file'})).nodeId;
  if(!node)throw Error('Missing OBrush public project input');
  await send('DOM.setFileInputFiles',{nodeId:node,files:[sequence.file]});
  await until(async()=>{const m=await evaluate(obrushInfo);return m?.subtools===4&&m.message?.startsWith('Loaded ')?m:null;},'sequential project Open');
  const reopened=await projectSave();
  result.sequence.reopened_sha256=reopened.sha256;
  result.sequence.exact_save_open=sequence.sha256===reopened.sha256;
  if(!result.sequence.exact_save_open)throw Error('Sequential Source placement changed on Save/Open');
  result.status='passed scoped first-add pose/world-bounds and sequential Add/Undo/Save/Open gates';
}catch(error){primaryError=error;result.status='failed';result.errors.push(String(error.stack||error));}
finally{
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}
  socket?.close();if(chrome?.exitCode===null)chrome.kill();server?.close();
}
console.log(JSON.stringify({status:result.status,result:path.join(out,'result.json'),comparisons:result.cases.map(c=>[c.name,c.comparison]),sequence:result.sequence,errors:result.errors}));
if(primaryError)process.exitCode=1;
