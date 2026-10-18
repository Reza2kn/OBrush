// Pinned SculptGL normal-browser companion to OBrush's history-limit check.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';
import { PerfDriver } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs';

const root=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const mirror=path.join(root,'.tools/sculptgl-reference/host-mirror');
const sha=b=>createHash('sha256').update(b).digest('hex');
const bundle=sha(await readFile(path.join(mirror,'sculptgl.js')));
if(bundle!=='79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600')throw Error('Pinned hosted SculptGL bundle changed');
const out=path.join(root,'.artifacts/history-limit','source-'+randomUUID());
await mkdir(out,{recursive:true});
const result={status:'running',reference_bundle_sha256:bundle,requests:[],cdp_trace:[],ui_actions:[],input_trace:[],download_events:[],downloads:[],screenshots:[],dialogs:[],exceptions:[],console_errors:[],blocked_external_requests:[],viewport_measurements:[],notice_acknowledgments:[],acknowledgments:[],failed_acknowledgments:[],page_observations:[],cleanup_errors:[],checks:[]};
const owner=new OwnedCdp(out,result);
try{
  const url=await owner.serve(mirror,'source');
  await owner.start();
  const page=await owner.page('source',url,'');
  Object.setPrototypeOf(page,PerfDriver.prototype);
  await page.send('Page.bringToFront');
  await page.clear();
  const manager='window.__sculptglReferenceInstance.getStateManager()';
  const sourceLimit=()=>page.evaluate(`${manager}.constructor.STACK_LENGTH`);
  const setLimit=async value=>{
    await page.sourceNumber('_ctrlStates',`Array.from(window.__sculptglReferenceInstance.getGui()._ctrlStates._menu.domUl.querySelectorAll('input[type=number]')).find(e=>e.getBoundingClientRect().width>0)`,value);
    const actual=await sourceLimit();
    result.checks.push({name:'visible-limit-'+value,actual});
    if(actual!==value)throw Error(`Visible source limit ${value} did not apply: ${actual}`);
  };
  if(await sourceLimit()!==15)throw Error('Pinned default history length changed');
  await setLimit(3);
  const empty=(await page.save('empty')).record;
  for(let i=1;i<=5;i++){
    await page.sourceButton('_ctrlScene','Add sphere');
    const state=await page.state(false);
    if(state.meshes.length!==i||state.history.current_undo!==Math.min(i,3)-1)throw Error('Pinned Add sphere/history cap mismatch '+i);
    result.checks.push({name:'add-'+i,meshes:state.meshes.length,history:state.history});
  }
  const edited=(await page.save('edited')).record;
  for(let i=1;i<=3;i++){
    await page.sourceButton('_ctrlStates','Undo');
    const state=await page.state(false);
    if(state.meshes.length!==5-i||state.history.current_undo!==2-i)throw Error('Pinned Undo mismatch '+i);
    result.checks.push({name:'undo-'+i,meshes:state.meshes.length,history:state.history});
  }
  const retained=(await page.save('retained')).record;
  await page.sourceButton('_ctrlStates','Undo');
  const exhausted=await page.state(false);
  const exhaustedSave=(await page.save('exhausted')).record;
  if(exhausted.meshes.length!==2||exhaustedSave.sha256!==retained.sha256)throw Error('Pinned fourth Undo changed scene');
  for(let i=0;i<3;i++)await page.sourceButton('_ctrlStates','Redo');
  const redone=await page.state(false);
  const redoneSave=(await page.save('redone')).record;
  if(redone.meshes.length!==5||redoneSave.sha256!==edited.sha256)throw Error('Pinned Redo failed exact SGL recovery');
  result.checks.push({name:'exact-redo',history:redone.history,sha256:redoneSave.sha256});
  await setLimit(50);
  await setLimit(10);
  const after=(await page.save('after-limits')).record;
  if(after.sha256!==edited.sha256)throw Error('Pinned history setting changed exported scene');
  result.checks.push({name:'settings-preserve-scene',before:empty.sha256,edited:edited.sha256,after:after.sha256});
  // Lower a branch containing both Undo and Redo entries. SculptGL retains
  // nearest past actions first and discards the distant redo future.
  for(let i=0;i<6;i++)await page.sourceButton('_ctrlScene','Add cube');
  for(let i=0;i<3;i++)await page.sourceButton('_ctrlStates','Undo');
  const branchBefore=await page.state(false);
  if(branchBefore.history.current_undo!==5||branchBefore.history.redo_length!==3)throw Error('Pinned mixed branch precondition failed');
  await setLimit(3);
  const branchAfter=await page.state(false);
  result.checks.push({name:'lower-with-redo',before:branchBefore.history,after:branchAfter.history,meshes:branchAfter.meshes.length});
  if(branchAfter.history.current_undo!==2||branchAfter.history.redo_length!==0||branchAfter.meshes.length!==8)
    throw Error('Pinned mixed branch trim differs from source manager');
  // One visible Brush contact between ordinary scene commands exercises the
  // history stack across geometry and object membership, without equating
  // different apps' brush displacement magnitudes.
  await page.clear();
  const cleared=await page.state(false);
  if(cleared.meshes.length!==0)throw Error('Pinned mixed history clear failed');
  await page.sourceButton('_ctrlScene','Add sphere');
  await page.closeMenus();
  const beforeBrush=await page.state(false);
  await page.screenshot('mixed-before-brush');
  await page.evaluate(`(()=>{const mesh=window.__sculptglReferenceInstance.getMesh();window.__historyPositionSnapshot=new Float32Array(mesh.getVertices().subarray(0,mesh.getNbVertices()*3));})()`);
  const view=await page.viewport(),x=(view.screen[0]+view.screen[2])/2+80,y=(view.screen[1]+view.screen[3])/2;
  await page.pointer('mouseMoved',x,y);
  await page.pointer('mousePressed',x,y,'left',1);
  await page.raf();
  await page.pointer('mouseMoved',x+12,y,'left',1);
  await page.raf();
  await page.pointer('mouseReleased',x+12,y,'left',0);
  for(let i=0;i<12;i++)await page.raf();
  const brushed=await page.state(false);
  const brushDelta=await page.evaluate(`(()=>{const mesh=window.__sculptglReferenceInstance.getMesh(),before=window.__historyPositionSnapshot,after=mesh.getVertices();if(!before||before.length!==mesh.getNbVertices()*3)return {mismatch:true};let words=0,largest=0;for(let i=0;i<before.length;i++){if(before[i]!==after[i]){words++;largest=Math.max(largest,Math.abs(before[i]-after[i]));}}delete window.__historyPositionSnapshot;return {words,largest};})()`);
  result.checks.push({name:'brush-probe',before:beforeBrush.history,after:brushed.history,delta:brushDelta,view:view.screen,point:[x,y],tool:await page.live()});
  if(!brushDelta.words||brushed.history.current_undo!==beforeBrush.history.current_undo+1)
    throw Error('Pinned visible Brush did not make one history geometry edit');
  await page.sourceButton('_ctrlScene','Add cube');
  const mixedEdited=(await page.save('mixed-edited')).record;
  const mixedHistory=[];
  for(let i=0;i<3;i++){
    await page.sourceButton('_ctrlStates','Undo');
    const state=await page.state(false);
    mixedHistory.push({meshes:state.meshes.length,history:state.history});
  }
  if(mixedHistory[0].meshes!==1||mixedHistory[1].meshes!==1||mixedHistory[2].meshes!==0)
    throw Error('Pinned scene/Brush history order mismatch');
  for(let i=0;i<3;i++)await page.sourceButton('_ctrlStates','Redo');
  const mixedRedone=(await page.save('mixed-redone')).record;
  if(mixedRedone.sha256!==mixedEdited.sha256)throw Error('Pinned mixed scene/Brush Redo did not recover exact SGL');
  result.checks.push({name:'scene-brush-scene-history',brush_delta:brushDelta,undo:mixedHistory,edited:mixedEdited.sha256,redone:mixedRedone.sha256});
  await page.clear();
  await page.sourceButton('_ctrlScene','Add cube');
  const coarse=await page.state(false);
  if(coarse.meshes.length!==1)throw Error('Pinned topology-history cube setup failed');
  await page.sourceMenu('_ctrlTopology');
  for(let i=0;i<6;i++)await page.raf();
  await page.sourceButton('_ctrlTopology','Subdivide');
  for(let i=0;i<6;i++)await page.raf();
  const fine=await page.state(false);
  result.checks.push({name:'topology-probe',coarse:coarse.meshes.map(m=>[m.vertices,m.faces_count]),fine:fine.meshes.map(m=>[m.vertices,m.faces_count]),history:fine.history,mesh:await page.evaluate(`(()=>{const m=window.__sculptglReferenceInstance.getMesh();return {name:m?.constructor?.name,sel:m?._sel,levels:m?._meshes?.length,triangles:m?.getNbTriangles(),dialog:document.querySelector('dialog[open]')?.textContent??null}})()`)});
  await page.screenshot('after-subdivide-probe');
  if(fine.meshes.length!==1||fine.meshes[0].vertices<=coarse.meshes[0].vertices||fine.meshes[0].faces_count<=coarse.meshes[0].faces_count)
    throw Error('Pinned visible subdivision did not grow cube topology');
  await page.sourceButton('_ctrlScene','Add sphere');
  const topoEdited=(await page.save('topology-edited')).record;
  const topologyUndo=[];
  for(let i=0;i<3;i++){
    await page.sourceButton('_ctrlStates','Undo');
    const state=await page.state(false);
    topologyUndo.push({meshes:state.meshes.length,vertices:state.meshes[0]?.vertices??0,faces:state.meshes[0]?.faces_count??0,history:state.history});
  }
  if(topologyUndo[0].meshes!==1||topologyUndo[0].vertices!==fine.meshes[0].vertices||
    topologyUndo[1].meshes!==1||topologyUndo[1].vertices!==coarse.meshes[0].vertices||topologyUndo[2].meshes!==0)
    throw Error('Pinned scene/subdivision history order mismatch');
  for(let i=0;i<3;i++)await page.sourceButton('_ctrlStates','Redo');
  const topoRedone=(await page.save('topology-redone')).record;
  if(topoRedone.sha256!==topoEdited.sha256)throw Error('Pinned topology history did not recover exact SGL');
  result.checks.push({name:'scene-subdivision-scene-history',coarse:[coarse.meshes[0].vertices,coarse.meshes[0].faces_count],fine:[fine.meshes[0].vertices,fine.meshes[0].faces_count],undo:topologyUndo,edited:topoEdited.sha256,redone:topoRedone.sha256});
  result.status='PASS pinned SculptGL visible history-limit workflow';
}catch(error){result.status='FAILED pinned SculptGL history-limit workflow; evidence retained';result.failure=String(error.stack??error);process.exitCode=1;}
finally{
  try{await owner.close();}catch(error){result.cleanup_errors.push(String(error));process.exitCode=1;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,checks:result.checks.length,cleanup_errors:result.cleanup_errors}));
}
