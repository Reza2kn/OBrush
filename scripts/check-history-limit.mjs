// Normal browser UI acceptance for the session history-length control.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';
import { PerfDriver } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs';

const root=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const out=path.join(root,'.artifacts/history-limit',randomUUID());
await mkdir(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const result={status:'running',build_sha256:sha(await readFile(path.join(root,'web/dist/pkg/obrush_app_bg.wasm'))),
  requests:[],cdp_trace:[],ui_actions:[],input_trace:[],download_events:[],downloads:[],
  screenshots:[],dialogs:[],exceptions:[],console_errors:[],blocked_external_requests:[],
  viewport_measurements:[],notice_acknowledgments:[],acknowledgments:[],failed_acknowledgments:[],
  page_observations:[],cleanup_errors:[],slider_alignment:[],checks:[]};
const owner=new OwnedCdp(out,result);
try{
  const url=await owner.serve(path.join(root,'web/dist'),'root');
  await owner.start();
  const page=await owner.page('root',url,'');
  Object.setPrototypeOf(page,PerfDriver.prototype);
  await owner.send('Browser.setWindowBounds',{windowId:page.windowId,bounds:{width:1900,height:1000}});
  await page.raf();
  page.rootControl=async(key,click=true)=>{
    for(let attempt=0;attempt<30;attempt++){
      const state=await page.state();
      const c=page.found(state,key);
      if(!c){await page.raf();continue;}
      const viewport=await page.viewport();
      const [x0,y0,x1,y1]=c.rect;
      const x=(x0+x1)/2,y=(y0+y1)/2;
      if(x0>=0&&x<viewport.window[0]-5&&y0>=65&&y1<=viewport.window[1]-40){
        for(let frame=0;frame<3;frame++)await page.raf();
        const stable=page.found(await page.state(),key);
        if(JSON.stringify(stable?.rect)!==JSON.stringify(c.rect))continue;
        result.ui_actions.push({key,attempt,rect:c.rect,action:click?'click':'ready'});
        if(click)await page.click(x,y);
        return c;
      }
      const hoverX=x0<viewport.screen[0]?100:Math.min(viewport.window[0]-80,viewport.screen[2]+100);
      const delta=y1<65?-500:500;
      result.ui_actions.push({key,attempt,rect:c.rect,action:'scroll',hoverX,delta});
      await page.pointer('mouseMoved',hoverX,Math.min(500,viewport.window[1]-100));
      await page.send('Input.dispatchMouseEvent',{type:'mouseWheel',x:hoverX,y:Math.min(500,viewport.window[1]-100),deltaX:0,deltaY:delta});
      await page.raf();
    }
    throw Error('Normal control remained offscreen '+key);
  };
  page.rootSlider=async(key,get,value,min,max)=>{
    await page.rootControl(key,false);
    for(let i=0;i<3;i++)await page.raf();
    const c=page.found(await page.state(),key);
    const trackWidth=Math.min(140,c.rect[2]-c.rect[0]-45);
    let x=c.rect[0]+2+(trackWidth-4)*(value-min)/(max-min);
    await page.pointer('mouseMoved',x,(c.rect[1]+c.rect[3])/2);
    await page.raf();
    await page.click(Math.min(x,1890),(c.rect[1]+c.rect[3])/2);
    let actual=get(await page.state());
    if(actual!==value){
      const code=value===max?'End':value===min?'Home':null;
      if(code){await page.key(code,code,value===max?35:36);actual=get(await page.state());}
      else for(let i=0;i<6&&actual!==value;i++){
        x+=Math.sign(value-actual)*Math.max(2,Math.abs(value-actual)*(trackWidth-4)/(max-min));
        await page.pointer('mouseMoved',x,(c.rect[1]+c.rect[3])/2);
        await page.raf();
        await page.click(x,(c.rect[1]+c.rect[3])/2);
        actual=get(await page.state());
      }
    }
    result.slider_alignment.push({key,value,actual,rect:c.rect,x});
    if(actual!==value)throw Error(`Normal slider ${key}: expected ${value}, got ${actual}`);
  };
  await page.send('Page.bringToFront');
  const initial=await page.state();
  if(initial.history?.limit!==15||initial.history.undo!==0||initial.history.redo!==0)throw Error('Default history limit/depth mismatch');
  result.initial={history:initial.history,subtools:initial.subtools.length};
  result.initial.history_control=initial.layer_controls?.['history.limit'];
  const before=(await page.save('before')).record;
  await page.rootSlider('history.limit',s=>s.history.limit,3,3,50);
  const limitOnly=(await page.save('limit-only')).record;
  if(limitOnly.sha256!==before.sha256)throw Error('Changing history limit altered project save');
  result.checks.push({name:'limit3-preserves-project',limit:(await page.state()).history.limit,sha256:before.sha256});
  for(let i=1;i<=5;i++){
    const prior=await page.state();
    await page.rootControl('scene.add_sphere');
    const next=await page.state();
    if(next.subtools.length!==prior.subtools.length+1||next.history.undo!==Math.min(i,3))
      throw Error('Add sphere/history cap failed at edit '+i);
    result.checks.push({name:'add-sphere-'+i,subtools:next.subtools.length,history:next.history});
  }
  const edited=(await page.save('edited')).record;
  for(let i=1;i<=3;i++){
    await page.chord('z');
    const state=await page.state();
    if(state.subtools.length!==initial.subtools.length+5-i||state.history.undo!==3-i||state.history.redo!==i)
      throw Error('Undo depth/order mismatch at '+i);
    result.checks.push({name:'undo-'+i,subtools:state.subtools.length,history:state.history});
  }
  const retained=(await page.save('retained')).record;
  await page.chord('z');
  const exhausted=await page.state();
  if(exhausted.subtools.length!==initial.subtools.length+2||exhausted.history.undo!==0)
    throw Error('History cap allowed fourth Undo');
  const exhaustedSave=(await page.save('exhausted')).record;
  if(exhaustedSave.sha256!==retained.sha256)throw Error('Exhausted Undo changed project');
  for(let i=1;i<=3;i++)await page.chord('z',true);
  const redone=await page.state();
  const redoneSave=(await page.save('redone')).record;
  if(redone.history.undo!==3||redone.history.redo!==0||redoneSave.sha256!==edited.sha256)
    throw Error('Redo did not restore exact edited project');
  result.checks.push({name:'redo-restores-edited-project',history:redone.history,sha256:redoneSave.sha256});
  await page.rootSlider('history.limit',s=>s.history.limit,50,3,50);
  const expanded=await page.state();
  const expandedSave=(await page.save('expanded')).record;
  if(expanded.history.limit!==50||expandedSave.sha256!==edited.sha256)
    throw Error('Increasing limit changed project or failed to publish');
  result.checks.push({name:'limit50-preserves-project',history:expanded.history,sha256:expandedSave.sha256});
  await page.rootSlider('history.limit',s=>s.history.limit,10,3,50);
  const intermediate=await page.state();
  const intermediateSave=(await page.save('intermediate')).record;
  if(intermediate.history.limit!==10||intermediateSave.sha256!==edited.sha256)
    throw Error('Intermediate limit changed project or failed to publish');
  result.checks.push({name:'limit10-preserves-project',history:intermediate.history,sha256:intermediateSave.sha256});
  for(let i=0;i<6;i++)await page.rootControl('scene.add_cube');
  for(let i=0;i<3;i++)await page.chord('z');
  const branchBefore=await page.state();
  if(branchBefore.history.undo!==6||branchBefore.history.redo!==3)throw Error('Mixed Undo/Redo branch precondition failed');
  const branchSave=(await page.save('branch-before-limit')).record;
  await page.rootSlider('history.limit',s=>s.history.limit,3,3,50);
  const branchAfter=await page.state();
  const branchAfterSave=(await page.save('branch-after-limit')).record;
  if(branchAfter.history.undo!==3||branchAfter.history.redo!==0||branchAfterSave.sha256!==branchSave.sha256)
    throw Error('Lowering history limit did not match pinned nearest-Undo/far-Redo policy');
  result.checks.push({name:'lower-with-redo',before:branchBefore.history,after:branchAfter.history,sha256:branchSave.sha256});
  await page.rootControl('scene.clear');
  result.checks.push({name:'clear-probe',state:(await page.state()).subtools.length,control:page.found(await page.state(),'scene.clear')});
  await page.screenshot('after-clear-click');
  await page.wait(()=>page.state(),s=>s.subtools.length===0,'Mixed browser scene clear');
  await page.rootControl('scene.add_sphere');
  const beforeBrush=await page.state();
  const beforeBrushSave=(await page.save('mixed-before-brush')).record;
  const view=await page.viewport(),x=(view.screen[0]+view.screen[2])/2+80,y=(view.screen[1]+view.screen[3])/2;
  await page.pointer('mouseMoved',x,y);
  await page.pointer('mousePressed',x,y,'left',1);
  await page.raf();
  await page.pointer('mouseMoved',x+12,y,'left',1);
  await page.raf();
  await page.pointer('mouseReleased',x+12,y,'left',0);
  for(let i=0;i<12;i++)await page.raf();
  const brushed=await page.state();
  const brushedSave=(await page.save('mixed-brushed')).record;
  result.checks.push({name:'draw-probe',before:beforeBrush.history,after:brushed.history,before_sha:beforeBrushSave.sha256,after_sha:brushedSave.sha256,view:view.screen,point:[x,y],radius:brushed.radius,strength:brushed.strength,tool:brushed.tool});
  await page.screenshot('after-mixed-draw');
  if(brushedSave.sha256===beforeBrushSave.sha256||brushed.history.undo!==Math.min(beforeBrush.history.undo+1,brushed.history.limit))
    throw Error('Normal browser Draw failed one geometry history edit');
  await page.rootControl('scene.add_cube');
  const mixedEdited=(await page.save('mixed-edited')).record;
  const mixedHistory=[];
  for(let i=0;i<3;i++){
    await page.chord('z');
    const state=await page.state();
    mixedHistory.push({subtools:state.subtools.length,history:state.history});
  }
  if(mixedHistory[0].subtools!==1||mixedHistory[1].subtools!==1||mixedHistory[2].subtools!==0)
    throw Error('Normal browser scene/Draw history order mismatch');
  for(let i=0;i<3;i++)await page.chord('z',true);
  const mixedRedone=(await page.save('mixed-redone')).record;
  if(mixedRedone.sha256!==mixedEdited.sha256)throw Error('Normal browser mixed scene/Draw Redo failed exact project recovery');
  result.checks.push({name:'scene-brush-scene-history',before_brush:beforeBrushSave.sha256,brushed:brushedSave.sha256,undo:mixedHistory,edited:mixedEdited.sha256,redone:mixedRedone.sha256});
  await page.rootControl('scene.clear');
  await page.wait(()=>page.state(),s=>s.subtools.length===0,'Topology history scene clear');
  await page.rootControl('scene.add_cube');
  const coarse=await page.state();
  if(coarse.subtools.length!==1)throw Error('Topology history cube setup failed');
  await page.rootControl('object.subdivision.mode');
  await page.rootControl('object.subdivision.static_smooth');
  await page.rootControl('object.add_level');
  const fine=await page.state();
  if(fine.subtools.length!==1||fine.vertices<=coarse.vertices||fine.triangles<=coarse.triangles)
    throw Error('Visible static subdivision did not grow cube topology');
  await page.rootControl('scene.add_sphere');
  const topoEdited=(await page.save('topology-edited')).record;
  const topologyUndo=[];
  for(let i=0;i<3;i++){
    await page.chord('z');
    const state=await page.state();
    topologyUndo.push({subtools:state.subtools.length,vertices:state.vertices,triangles:state.triangles,history:state.history});
  }
  if(topologyUndo[0].subtools!==1||topologyUndo[0].vertices!==fine.vertices||
    topologyUndo[1].subtools!==1||topologyUndo[1].vertices!==coarse.vertices||topologyUndo[2].subtools!==0)
    throw Error('Normal browser scene/subdivision history order mismatch');
  for(let i=0;i<3;i++)await page.chord('z',true);
  const topoRedone=(await page.save('topology-redone')).record;
  if(topoRedone.sha256!==topoEdited.sha256)throw Error('Normal browser topology history did not recover exact project');
  result.checks.push({name:'scene-subdivision-scene-history',coarse:[coarse.vertices,coarse.triangles],fine:[fine.vertices,fine.triangles],undo:topologyUndo,edited:topoEdited.sha256,redone:topoRedone.sha256});
  result.status='PASS normal browser history limit, bounded Undo/Redo, exact public save';
}catch(error){result.status='FAILED normal browser history limit; evidence retained';result.failure=String(error.stack??error);process.exitCode=1;}
finally{
  try{await owner.close();}catch(error){result.cleanup_errors.push(String(error));process.exitCode=1;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,checks:result.checks.length,cleanup_errors:result.cleanup_errors}));
}
