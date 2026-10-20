// Q03 frozen public-project, ordinary camera close-up; never attaches to a user tab.
// Usage: node scripts/check-native-move-q03-closeup.mjs /absolute/root/web/dist /absolute/q02/receipt
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const web=path.resolve(process.argv[2]);
const fixture=path.resolve(process.argv[3]);
const out=path.resolve('.artifacts/native-move-q03/'+randomUUID());
await mkdir(out,{recursive:true});
const hash=value=>createHash('sha256').update(value).digest('hex');
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const inputs={
  before:{file:path.join(fixture,'sphere-twelve-repeated-q02-initial.obrush'),sha256:'2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb'},
  after:{file:path.join(fixture,'sphere-twelve-repeated-q02-final.obrush'),sha256:'5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632'},
};
const result={status:'running',out,web,fixture,cases:[],errors:[],limitations:[],protocol:{css:[1600,1000],dpr:1,pan:[[650,470],[800,470]],pan_steps:12,wheel_point:[800,470],wheel_delta_y:-120,wheel_count:5,roi:[550,260,1080,740],rgb_diff_threshold:8,min_diff_pixels:100}};
for(const [name,entry]of Object.entries(inputs)){
  const bytes=await readFile(entry.file);
  if(hash(bytes)!==entry.sha256)throw Error(`Frozen ${name} input hash mismatch`);
  entry.bytes=bytes.length;entry.document=JSON.parse(bytes.toString());
}
await writeFile(path.join(out,'harness.mjs'),await readFile(new URL(import.meta.url)));
const server=createServer(async(req,res)=>{
  try{
    const file=path.resolve(web,'.'+new URL(req.url,'http://127.0.0.1').pathname.replace(/\/$/,'/index.html'));
    if(!file.startsWith(web+path.sep))throw Error('outside web');
    const bytes=await readFile(file);
    res.setHeader('Cache-Control','no-store');
    res.setHeader('Content-Type',file.endsWith('.wasm')?'application/wasm':file.endsWith('.js')?'text/javascript':'text/html');
    res.end(bytes);
  }catch{res.writeHead(404);res.end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const url=`http://127.0.0.1:${server.address().port}/`;
const profile=path.join(out,'owned-chrome-profile');await mkdir(profile);
const chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[
  '--headless=new',`--user-data-dir=${profile}`,'--remote-debugging-address=127.0.0.1','--remote-debugging-port=0',
  '--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-sync','--disable-default-apps',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1','--window-size=1600,1000','about:blank',
],{stdio:['ignore','pipe','pipe']});
let stderr='';chrome.stderr.on('data',chunk=>{stderr+=chunk;});
let socket,session,seq=0;const pending=new Map();
const send=(method,params={},target=session)=>new Promise((resolve,reject)=>{
  const id=++seq,timer=setTimeout(()=>{pending.delete(id);reject(Error(`CDP timeout ${method}`));},30000);
  pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params,...(target?{sessionId:target}:{})}));
});
const evaluate=async expression=>{
  const response=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});
  if(response.exceptionDetails)throw Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
};
const pointer=(type,x,y,button='none',buttons=0)=>send('Input.dispatchMouseEvent',{type,x,y,button,buttons,clickCount:type==='mouseMoved'?0:1,pointerType:'mouse'});
const screenshot=async name=>{
  const response=await send('Page.captureScreenshot',{format:'png',fromSurface:true});
  const bytes=Buffer.from(response.data,'base64');const file=path.join(out,`${name}.png`);
  await writeFile(file,bytes);return {path:file,sha256:hash(bytes),bytes:bytes.length};
};
const meshHash=document=>hash(JSON.stringify(document.mesh));
const command=async letter=>{
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:'Meta',code:'MetaLeft',windowsVirtualKeyCode:91,modifiers:4});
  const code='Key'+letter.toUpperCase(),v=letter.toUpperCase().charCodeAt(0);
  await send('Input.dispatchKeyEvent',{type:'rawKeyDown',key:letter,code,windowsVirtualKeyCode:v,modifiers:4});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:letter,code,windowsVirtualKeyCode:v,modifiers:4});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Meta',code:'MetaLeft',windowsVirtualKeyCode:91,modifiers:0});
};
const save=async name=>{
  const index=await evaluate('window.__q03Captures.length');await command('s');
  let size=0;for(let i=0;i<60;i++){size=await evaluate(`window.__q03Captures[${index}]?.contents.length||0`);if(size)break;await pause(100);}
  if(!size||size>128*1024*1024)throw Error(`Public Save failed ${name}: ${size}`);
  let value='';for(let offset=0;offset<size;offset+=65536)value+=await evaluate(`window.__q03Captures[${index}].contents.slice(${offset},${offset+65536})`);
  const file=path.join(out,`${name}.obrush`);await writeFile(file,value);
  return {path:file,sha256:hash(value),bytes:Buffer.byteLength(value),document:JSON.parse(value)};
};
const fresh=async()=>{
  await send('Page.navigate',{url});
  for(let i=0;i<180;i++){
    if(await evaluate('!document.getElementById("loading")&&typeof window.obrushBrushMetrics==="function"'))break;
    if(i===179)throw Error('App startup timeout');await pause(100);
  }
  await evaluate('window.__q03Captures=[];window.__q03InputChanges=0;window.obrushDownload=(name,contents)=>window.__q03Captures.push({name,contents});document.getElementById("import-file").addEventListener("change",()=>window.__q03InputChanges++);document.getElementById("obrush_canvas").focus()');
};
const load=async file=>{
  const document=(await send('DOM.getDocument')).root;
  const nodeId=(await send('DOM.querySelector',{nodeId:document.nodeId,selector:'#import-file'})).nodeId;
  if(!nodeId)throw Error('Public Open file input absent');
  await send('DOM.setFileInputFiles',{nodeId,files:[file]});
  if(await evaluate('window.__q03InputChanges')!==1)throw Error('Public Open file input did not emit one change');
  for(let i=0;i<150;i++){
    const m=await evaluate('window.obrushBrushMetrics()');
    if(m.scene_error)throw Error(m.scene_error);
    if(m.subtools?.length===1&&m.active_subtool_id===1&&m.vertices===inputs[file===inputs.before.file?'before':'after'].document.mesh.positions.length){await pause(300);return m;}
    await pause(100);
  }
  throw Error('Public project load timeout');
};
const camera=()=>evaluate('window.obrushBrushMetrics().camera');
const finiteCamera=c=>[c.distance,...c.target,...c.view_projection].every(Number.isFinite);
const maxError=(a,b)=>Math.max(...a.map((value,index)=>Math.abs(value-b[index])));
try{
  let endpoint;
  for(let i=0;i<150;i++){
    try{const [port,suffix]=(await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');endpoint=`ws://127.0.0.1:${port}${suffix}`;break;}catch{}
    if(chrome.exitCode!==null)throw Error(`Chrome exited ${chrome.exitCode}`);await pause(100);
  }
  if(!endpoint)throw Error('Chrome endpoint missing');
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',event=>{const message=JSON.parse(String(event.data));if(message.id){const p=pending.get(message.id);if(p){clearTimeout(p.timer);pending.delete(message.id);message.error?p.reject(Error(JSON.stringify(message.error))):p.resolve(message.result);}}});
  result.browser=await send('Browser.getVersion',{},null);
  const target=await send('Target.createTarget',{url:'about:blank'},null);
  session=(await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
  await send('Page.enable');await send('Runtime.enable');await send('DOM.enable');
  await send('Emulation.setDeviceMetricsOverride',{width:1600,height:1000,deviceScaleFactor:1,mobile:false});
  await send('Emulation.setFocusEmulationEnabled',{enabled:true});
  const wasm=await readFile(path.join(web,'pkg/obrush_app_bg.wasm'));
  const served=Buffer.from(await (await fetch(url+'pkg/obrush_app_bg.wasm')).arrayBuffer());
  result.wasm={sha256:hash(wasm),served_sha256:hash(served),bytes:wasm.length};
  if(result.wasm.sha256!==result.wasm.served_sha256)throw Error('Served WASM mismatch');
  for(const [name,input]of Object.entries(inputs)){
    const row={name,input:{path:input.file,sha256:input.sha256,bytes:input.bytes},status:'running'};result.cases.push(row);
    await fresh();row.load_metrics=await load(input.file);
    const opened=await save(name+'-opened');row.opened={path:opened.path,sha256:opened.sha256,mesh_sha256:meshHash(opened.document)};
    if(meshHash(opened.document)!==meshHash(input.document))throw Error(`${name} public Open changed mesh`);
    row.camera_before=await camera();
    await pointer('mouseMoved',650,470);await pause(50);
    await pointer('mousePressed',650,470,'middle',4);await pause(50);
    for(let i=1;i<=12;i++){await pointer('mouseMoved',650+150*i/12,470,'none',4);await pause(20);}
    await pointer('mouseReleased',800,470,'middle',0);await pause(100);
    row.camera_after_pan=await camera();
    for(let i=0;i<5;i++){await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:800,y:470,deltaX:0,deltaY:-120,pointerType:'mouse'});await evaluate('new Promise(resolve=>requestAnimationFrame(()=>resolve(true)))');await pause(70);row[`camera_after_wheel_${i+1}`]=await camera();}
    await pointer('mouseMoved',1000,30);await pause(300);
    row.camera_final=await camera();row.screenshot=await screenshot(name+'-closeup');
    const post=await save(name+'-after-camera');row.after_camera={path:post.path,sha256:post.sha256,mesh_sha256:meshHash(post.document)};
    row.mesh_unchanged=row.after_camera.mesh_sha256===row.opened.mesh_sha256;
    row.camera_finite=finiteCamera(row.camera_final);
    row.distance_in_range=row.camera_final.distance>=1&&row.camera_final.distance<=1.6;
    row.status=row.mesh_unchanged&&row.camera_finite&&row.distance_in_range?'passed camera/file gate':'failed camera/file gate';
    await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
    if(row.status!=='passed camera/file gate')throw Error(`${name}: ${row.status}`);
  }
  const a=result.cases[0].camera_final,b=result.cases[1].camera_final;
  result.camera_match={distance_error:Math.abs(a.distance-b.distance),target_max_error:maxError(a.target,b.target),view_projection_max_error:maxError(a.view_projection,b.view_projection)};
  result.camera_match.passed=Object.values(result.camera_match).every(value=>typeof value==='boolean'?value:value<=1e-5);
  if(!result.camera_match.passed)throw Error('Paired camera mismatch');
  result.status='camera/file pass; pixel difference and visual review pending';
}catch(error){result.status='failed';result.failure=String(error.stack??error);process.exitCode=1;try{result.failure_screenshot=await screenshot('failure');}catch{}}
finally{
  await writeFile(path.join(out,'chrome-stderr.log'),stderr);
  try{if(socket?.readyState===WebSocket.OPEN)await send('Browser.close',{},null);}catch{}
  try{socket?.close();}catch{}
  await pause(300);if(chrome.exitCode===null)chrome.kill('SIGTERM');
  await new Promise(resolve=>server.close(resolve));
  result.cleanup={chrome_exit_code:chrome.exitCode,chrome_signal:chrome.signalCode,server_closed:true};
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({status:result.status,failure:result.failure,out,camera_match:result.camera_match,cases:result.cases.map(c=>({name:c.name,status:c.status,distance:c.camera_final?.distance,screenshot:c.screenshot?.path}))}));
}
