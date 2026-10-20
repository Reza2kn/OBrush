// C334 pinned SculptGL normal-UI reference observation, independent of OBrush's picked-pivot run.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { projectWorld, integerPixelRayHit } from './camera-picked-pivot-c334-ray.mjs';

const root=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??root);
const {OwnedCdp}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const {PerfDriver}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const hash=b=>createHash('sha256').update(b).digest('hex');
const assert=(ok,message)=>{if(!ok)throw Error(message);};
const fixture=path.join(root,'docs/fixtures/camera-picked-pivot-c334.obj');
assert(hash(await readFile(fixture))==='a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b','Fixture changed');
const mirror=path.join(shared,'.tools/sculptgl-reference/host-mirror');
assert(hash(await readFile(path.join(mirror,'sculptgl.js')))==='79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600','Pinned reference bundle changed');
const out=path.join(root,'.artifacts/camera-picked-pivot-c334-source-ui',randomUUID());
await mkdir(out,{recursive:true});
const result={status:'running',reference_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',fixture_hash:hash(await readFile(fixture)),
  cdp_trace:[],requests:[],ui_actions:[],input_trace:[],download_events:[],downloads:[],screenshots:[],dialogs:[],exceptions:[],console_errors:[],blocked_external_requests:[],
  viewport_measurements:[],notice_acknowledgments:[],acknowledgments:[],failed_acknowledgments:[],page_observations:[],cleanup_errors:[]};
const owner=new OwnedCdp(out,result);
let page;
const world=[0.4,-0.2,0.052941176];
const delta=(a,b)=>Math.max(...a.map((v,i)=>Math.abs(v-b[i])));
async function camera(){return page.evaluate('(()=>{const c=window.__sculptglReferenceInstance.getCamera();return {width:c._width,height:c._height,center:Array.from(c._center),offset:Array.from(c._offset),trans:Array.from(c._trans),view:Array.from(c.getView()),projection:Array.from(c.getProjection()),use_pivot:c.getUsePivot(),projection_type:c.getProjectionType(),fov:c.getFov(),timers:Object.values(c._timers)};})()');}
async function exactViewport(){
  const observations=[];
  for(let i=0;i<6;i++){
    await page.raf();const v=await page.viewport();observations.push({rect:v.rect,logical_size:v.logical_size,window:v.window});
    if(v.logical_size?.join(',')==='1024,768'){result.exact_viewport=observations;return v;}
    const current=(await owner.send('Browser.getWindowBounds',{windowId:page.windowId})).bounds;
    const width=current.width+1024-v.logical_size[0],height=current.height+768-v.logical_size[1];
    assert(width>=900&&width<=2300&&height>=600&&height<=1800,'Source bounded viewport resize limits');
    await owner.send('Browser.setWindowBounds',{windowId:page.windowId,bounds:{width,height,windowState:'normal'}});
  }
  result.exact_viewport=observations;throw Error('Exact Source 1024x768 unavailable after bounded window resizes');
}
async function run(name,pivot){
  result.camera_controls=await page.evaluate(`Array.from(${page.controller('_ctrlCamera')}._menu.domUl.querySelectorAll('input[type=checkbox]')).map(e=>({text:e.parentElement.textContent,checked:e.checked}))`);
  await page.sourceCheckbox('_ctrlCamera','Picking pivot',pivot);
  await page.sourceButton('_ctrlCamera','Reset (bar)');
  await page.closeMenus();
  for(let i=0;i<20;i++)await page.raf();
  await exactViewport();
  await page.sourceButton('_ctrlCamera','Reset (bar)');await page.closeMenus();
  for(let i=0;i<20;i++)await page.raf();
  const viewport=await exactViewport();
  const fit=[viewport.screen[0]+16,viewport.screen[3]-16];
  await page.pointer('mouseMoved',...fit);
  for(let tap=0;tap<2;tap++){await page.pointer('mousePressed',...fit,'left',1,{clickCount:tap+1});await page.pointer('mouseReleased',...fit,'left',0,{clickCount:tap+1});}
  await page.wait(()=>camera(),c=>c.timers.every(v=>v===0||v===-1||v===null),'Source fit timer settled');
  for(let i=0;i<20;i++)await page.raf();
  const before=await camera(),mesh=(await page.state(false)).meshes[0];
  assert(viewport.logical_size?.join(',')==='1024,768','Measured 1024x768 reference setup failed');
  assert(mesh.matrix.every((v,i)=>Math.abs(v-(i%5===0?1:0))<1e-8),'Authored reference mesh matrix changed');
  const projected=projectWorld(before.view,before.projection,world,viewport.screen),[x,y]=projected.map(Math.round);
  assert(x>viewport.screen[0]&&x<viewport.screen[2]&&y>viewport.screen[1]&&y<viewport.screen[3],'Reference projected point outside viewport');
  const ray=integerPixelRayHit(before.view,before.projection,[x,y],viewport.screen);
  await page.pointer('mouseMoved',x,y);await page.pointer('mousePressed',x,y,'right',2);await page.raf();await page.raf();
  const held=await camera();await page.pointer('mouseReleased',x,y,'right',0);for(let i=0;i<4;i++)await page.raf();const after=await camera();
  const row={name,pivot,projected,screen:[x,y],ray,before,held,after,mesh_matrix:mesh.matrix,held_view_delta:delta(before.view,held.view),after_view_delta:delta(before.view,after.view)};
  result.cases.push(row);
  assert(pivot?delta(before.center,held.center)>1e-3:delta(before.center,held.center)===0,name+' center toggle');
  if(pivot)assert(delta(ray.point,held.center)<=.005,name+' missed independent integer-pixel ray hit');
  assert(row.held_view_delta<=2e-5,name+' view jumped before drag');
  await page.pointer('mouseMoved',x,y);await page.pointer('mousePressed',x,y,'right',2);await page.pointer('mouseMoved',x+40,y,'right',2);await page.pointer('mouseReleased',x+40,y,'right',0);for(let i=0;i<4;i++)await page.raf();
  row.dragged=await camera();row.drag_view_delta=delta(after.view,row.dragged.view);assert(row.drag_view_delta>=1e-4,name+' no 40px orbit');
  return row;
}
try{
  const url=await owner.serve(mirror,'source');await owner.start();page=await owner.page('source',url,'');Object.setPrototypeOf(page,PerfDriver.prototype);await page.send('Page.bringToFront');await page.alignViewport();
  await page.sourceCheckbox('_ctrlFiles','Scale and center',false);await page.clear();await page.file('#fileopen',fixture);await page.wait(()=>page.state(false),s=>s.meshes.length===1&&s.meshes[0].vertices===4,'Independent OBJ reference import');
  await page.sourceSelect('_ctrlCamera',page.controller('_ctrlCamera')+'._ctrlProjection.domSelect',0);result.cases=[];await run('perspective-on',true);await run('perspective-off',false);result.status='pass bounded pinned Source stationary pivot/40px orbit';
}catch(error){result.status='failed';result.failure=String(error.stack??error);process.exitCode=1;}
finally{await owner.close();await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({out,status:result.status,failure:result.failure,cases:result.cases?.map(c=>({name:c.name,screen:c.screen,held_view_delta:c.held_view_delta,drag_view_delta:c.drag_view_delta,before_center:c.before.center,held_center:c.held.center}))}));}
