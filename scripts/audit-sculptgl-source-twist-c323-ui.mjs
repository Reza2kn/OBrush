// C323 frozen Source Twist visible-strength factor pair. Active GPU normals are the display oracle.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const {decodeIndependent}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const [sourcePath,rootPath]=process.argv.slice(2);
const pathOffsets=[[-48,-12],[-12,-12],[-42,-44]];
if(!sourcePath||!rootPath)throw Error('Pass Source and OBrush result.json paths');
const sha=b=>createHash('sha256').update(b).digest('hex');
const runs={source:JSON.parse(await readFile(sourcePath)),root:JSON.parse(await readFile(rootPath))};
const failures=[],check=(ok,message)=>{if(!ok)failures.push(message);};
const f32word=w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();};
const exact=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
function canonical(side,bytes){
  const m=side==='source'?decodeIndependent(bytes).meshes[0]:decodeRootF32Model(bytes);
  const raw=side==='source'?{positions:m.positions.map(r=>r.map(f32word)),colors:m.colors.map(r=>r.map(f32word)),materials:m.materials.map(r=>r.map(f32word)),faces:m.faces}:m;
  const ids=raw.colors.map(c=>Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16));
  const positions=Array(ids.length),attributes=Array(ids.length),faces=[];
  ids.forEach((id,i)=>{if(positions[id])throw Error(`Duplicate RGB ID ${id}`);positions[id]=raw.positions[i];attributes[id]=[...raw.colors[i],...raw.materials[i].slice(0,2),side==='source'?raw.materials[i][2]:raw.unprotected[i]];});
  if(positions.some(x=>!x))throw Error('Missing RGB ID');
  for(const face of raw.faces){const row=face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]);const k=row.indexOf(Math.min(...row));faces.push([...row.slice(k),...row.slice(0,k)].join(','));}
  faces.sort();return {positions,attributes,faces,ids};
}
const mesh={},files={};
for(const [key,run] of Object.entries(runs))for(const stage of ['before','after','undo','redo']){
  const p=run[stage]?.path;if(!p)throw Error(`${key} ${stage} public Save missing`);
  const bytes=await readFile(p);if(sha(bytes)!==run[stage].sha256)throw Error(`${key} ${stage} Save hash changed`);
  files[`${key}_${stage}`]=bytes;mesh[`${key}_${stage}`]=canonical(key,bytes);
}
function compare(a,b){
  if(a.length!==b.length)throw Error('Mapped length mismatch');
  let max=0,sum=0,components=0;const changed_ids=[];
  for(let id=0;id<a.length;id++){if(!a[id]||!b[id])throw Error(`Missing row ${id}`);let changed=false;
    for(let c=0;c<3;c++){const delta=a[id][c]-b[id][c];if(!Number.isFinite(delta))throw Error(`Nonfinite row ${id}`);max=Math.max(max,Math.abs(delta));sum+=delta*delta;
      if(Math.fround(a[id][c])!==Math.fround(b[id][c])){components++;changed=true;}}
    if(changed)changed_ids.push(id);}
  return {changed_ids,changed_vertices:changed_ids.length,different_components:components,max_abs_component:max,rms_component:Math.sqrt(sum/(a.length*3))};
}
function gpuRows(side,stage,field,heldIndex=0){
  const run=runs[side],observed=stage==='held'?run.held_previews?.[heldIndex]?.gpu:run[`${stage}_gpu`];
  const bytes=Buffer.from(observed?.[`${field}_words_base64`]??'','base64'),ids=mesh[`${side}_before`].ids;
  if(bytes.length!==ids.length*12||sha(bytes)!==observed?.[`${field}_words_sha256`])throw Error(`${side} ${stage}/${heldIndex} ${field} GPU words missing/invalid`);
  const rows=Array(ids.length);ids.forEach((id,i)=>{rows[id]=[0,1].map(c=>bytes.readFloatLE(i*12+c*4));});return rows;
}
check(runs.source.status?.startsWith('normal browser')&&runs.root.status?.startsWith('normal browser'),'Both normal-UI captures completed');
check(runs.source.case_name==='off'&&runs.root.case_name==='off'&&runs.source.symmetry===false&&runs.root.symmetry===false,'Frozen symmetry-off case identity');
check(runs.source.protocol_commit==='c323-source-twist-direct-endpoint-strength-one-integer-viewport808'&&runs.root.protocol_commit===runs.source.protocol_commit&&runs.source.harness_sha256===runs.root.harness_sha256&&runs.source.fixture.sha256===runs.root.fixture.sha256&&exact(runs.source.requested_brush,runs.root.requested_brush),'Frozen protocol/input identity');
check(runs.source.read_only_camera_method===runs.root.read_only_camera_method,'Frozen settled-camera method identity');
for(const side of ['source','root'])for(const label of ['after_reset','after_fit']){
  const stable=runs[side].stability?.[label];
  check(stable?.terminal===true&&stable.frames>=12&&stable.elapsed_ms>=250&&stable.elapsed_ms<=5000&&
    typeof stable.signature==='string'&&stable.signature.length>0,`${side} ${label}: frozen settled camera`);
}
const s=runs.source,r=runs.root,p=s.setup.state.camera.projection,q=r.setup.state.camera.projection;
const sw=s.setup.viewport.screen[2]-s.setup.viewport.screen[0],sh=s.setup.viewport.screen[3]-s.setup.viewport.screen[1],rw=r.setup.viewport.screen[2]-r.setup.viewport.screen[0],rh=r.setup.viewport.screen[3]-r.setup.viewport.screen[1];
const worldRadiusS=2*s.config.state.tool._radius/(sw*p[0]),worldRadiusR=2*r.config.state.preferences.tools.find(t=>t.kind==='Twist').screen_radius_pixels/(rw*q[0]);
const rayX=pathOffsets.map(([x])=>x).map(x=>Math.abs(2*x/(sw*p[0])-2*x/(rw*q[0]))),rayY=[-20].map(y=>Math.abs(2*y/(sh*p[5])-2*y/(rh*q[5])));
const input={projection_x:q[0]/p[0]*rw/sw,projection_y:q[5]/p[5]*rh/sh,world_radius_difference:Math.abs(worldRadiusS-worldRadiusR),ray_x_differences:rayX,ray_y_differences:rayY,source_projection:[p[0],p[5]],root_projection:[q[0],q[5]]};
check(Math.abs(input.projection_x-1)<=1e-8&&Math.abs(input.projection_y-1)<=1e-8&&input.world_radius_difference<=1e-8&&Math.max(...rayX,...rayY)<=1e-8,'Projection/world-ray/radius setup');
const gpu={},history={},changes={},paired={},preview={},witnesses={};
for(const side of ['source','root']){
  const run=runs[side],v=run.setup.viewport.screen,center=[(v[0]+v[2])/2,(v[1]+v[3])/2],expected=pathOffsets.map(([x,y])=>[center[0]+x,center[1]+y]);
  const delivered=run.dom_stroke_events?.filter(e=>e.type==='pointermove'&&e.buttons===1).map(e=>[e.x,e.y]),down=run.dom_stroke_events?.filter(e=>e.type==='pointerdown'),up=run.dom_stroke_events?.filter(e=>e.type==='pointerup');
  check(exact(run.setup.viewport.logical_size,[1024,808])&&center.every(Number.isInteger)&&exact(run.requested_path,expected)&&exact(delivered,expected.slice(1))&&down?.length===1&&up?.length===1&&down[0].x===expected[0][0]&&down[0].y===expected[0][1]&&up[0].x===expected[2][0]&&up[0].y===expected[2][1]&&run.dom_stroke_events?.every(e=>e.trusted),`${side}: trusted integer UI stroke`);
  const tool=side==='source'?run.config.state.tool:run.config.state.preferences.tools.find(t=>t.kind==='Twist');
  check(side==='source'?run.config.state.tool_index===2&&tool._lockPosition!==true&&[0,'None'].includes(tool._idAlpha)&&run.config.state.symmetry===false&&!run.config.state.continuous:run.config.state.kind==='Twist'&&tool.screen_space_radius&&tool.screen_radius_pixels===75&&tool.strength===1&&!tool.backface_culling&&!tool.lock_position&&run.config.state.symmetry_x===false&&!run.config.state.continuous.requested,`${side}: published Twist settings`);
  history[side]={before_undo:files[`${side}_before`].equals(files[`${side}_undo`]),after_redo:files[`${side}_after`].equals(files[`${side}_redo`]),one_step:run.after_history?.undo===run.before_history?.undo+1};
  check(Object.values(history[side]).every(Boolean),`${side}: public history`);
  check(run.held_previews?.length===2,`${side}: two held snapshots`);
  preview[side]={quiet:run.quiet_gpu?.position_words_sha256===run.after_gpu?.position_words_sha256&&run.quiet_gpu?.normal_words_sha256===run.after_gpu?.normal_words_sha256};
  check(preview[side].quiet,`${side}: quiet GPU`);
  for(const stage of ['after','undo','redo'])check(exact(mesh[`${side}_before`].faces,mesh[`${side}_${stage}`].faces),`${side} ${stage}: topology changed`);
  check(exact(mesh[`${side}_before`].attributes,mesh[`${side}_after`].attributes),`${side}: attributes/protection changed`);
  for(const stage of ['before','after','undo','redo']){
    const observed=run[`${stage}_gpu`],m=mesh[`${side}_${stage}`],packed=Buffer.alloc(m.ids.length*12);
    m.ids.forEach((id,i)=>m.positions[id].forEach((x,c)=>packed.writeFloatLE(x,i*12+c*4)));
    const identity=side==='root'?(run[`${stage}_object_identity`]??run.before_object_identity):null;
    gpu[`${side}_${stage}`]={unique:observed?.active_vbo_candidates===1&&observed?.gl_error===0,position_exact:observed?.position_words_sha256===sha(packed),identity:side==='source'||(identity?.active_subtool_id===1&&identity?.vertices===1089&&exact(identity.selected_subtool_ids,[1])&&identity.subtools?.length===1&&identity.subtools[0].visible&&Number.isInteger(observed?.buffer_token)&&observed.buffer_token>0)};
    check(Object.values(gpu[`${side}_${stage}`]).every(Boolean),`${side} ${stage}: active GPU identity/Save words`);
  }
  for(let i=0;i<2;i++){const observed=run.held_previews[i].gpu;gpu[`${side}_held${i}`]={unique:observed.active_vbo_candidates===1&&observed.gl_error===0,identity:side==='source'||(Number.isInteger(observed.buffer_token)&&observed.buffer_token>0&&run.held_previews[i].live?.state?.active_subtool_id===1)};check(Object.values(gpu[`${side}_held${i}`]).every(Boolean),`${side} held${i}: active GPU/object`);}
  changes[side]=compare(mesh[`${side}_before`].positions,mesh[`${side}_after`].positions);
  witnesses[side]=Object.fromEntries([541,543,577,578].map(id=>{const a=mesh[`${side}_before`].positions[id],b=mesh[`${side}_after`].positions[id];return [id,{before:a,after:b,displacement:Math.hypot(...a.map((v,i)=>b[i]-v))}]}));
  check(changes[side].changed_vertices>0,`${side}: Twist changed no vertices`);
  check(exact(mesh[`${side}_before`].positions[543],mesh[`${side}_after`].positions[543]),`${side}: protected ID543 moved`);
  const held=[0,1].map(i=>gpuRows(side,'held','position',i));
  check(compare(gpuRows(side,'before','position'),held[0]).changed_vertices===0&&compare(held[0],held[1]).changed_vertices>0,`${side}: direct held captures/anchor`);
}
for(const stage of ['before','after','undo','redo']){
  paired[stage]={positions:compare(mesh[`source_${stage}`].positions,mesh[`root_${stage}`].positions),normals:compare(gpuRows('source',stage,'normal'),gpuRows('root',stage,'normal'))};
  if(stage==='before')check(paired[stage].positions.different_components===0,'Initial positions differ');
  check(exact(mesh[`source_${stage}`].attributes,mesh[`root_${stage}`].attributes)&&exact(mesh[`source_${stage}`].faces,mesh[`root_${stage}`].faces),`${stage}: attributes/faces differ`);
}
for(let i=0;i<2;i++){paired[`held${i}`]={positions:compare(gpuRows('source','held','position',i),gpuRows('root','held','position',i)),normals:compare(gpuRows('source','held','normal',i),gpuRows('root','held','normal',i))};}
check(exact(changes.source.changed_ids,changes.root.changed_ids),'Changed RGB membership differs');
const redoNormalMembership={};
for(const side of ['source','root']){
  const after=gpuRows(side,'after','normal'),redo=gpuRows(side,'redo','normal');
  redoNormalMembership[side]=compare(after,redo).changed_ids;
}
check(exact(redoNormalMembership.source,redoNormalMembership.root),'After-to-Redo active display-normal changed RGB membership differs');
const c318Paths={source:path.join(local,'.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/result.json'),root:path.join(local,'.artifacts/sculptgl-source-twist-c318-ui/off-root-cedcecb2-8563-4039-8b9c-55aadba8970b/result.json')};
const c318Expected={source:'e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b',root:'0cec63c177442128f8b1ac6caf3422ed3faaed0fc2cf19cd2c3c6647336809f4'};
const c318={};
for(const side of ['source','root']){
  const bytes=await readFile(c318Paths[side]);
  check(sha(bytes)===c318Expected[side],`${side}: frozen C318 receipt identity`);
  c318[side]=JSON.parse(bytes);
  check(c318[side].browser_build===runs[side].browser_build&&c318[side].fixture.sha256===runs[side].fixture.sha256,`${side}: C318/C323 build/fixture identity`);
  const oldBefore=await readFile(c318[side].before.path),oldAfter=await readFile(c318[side].after.path);
  check(sha(oldBefore)===c318[side].before.sha256&&sha(oldAfter)===c318[side].after.sha256,`${side}: C318 public Save identity`);
  c318[side].beforeMesh=canonical(side,oldBefore);
  c318[side].afterMesh=canonical(side,oldAfter);
  check(compare(mesh[`${side}_before`].positions,c318[side].beforeMesh.positions).max_abs_component===0,`${side}: C318/C323 Before geometry`);
  check(runs[side].held_previews[0].gpu.position_words_sha256===c318[side].held_previews[0].gpu.position_words_sha256,`${side}: C318/C323 fixed anchor`);
}
const sourcePick=s.press_pick,oldPick=c318.source.press_pick;
const samePick=['point','eye','radius_squared'].every(key=>exact(sourcePick?.[key],oldPick?.[key]));
check(samePick,'Source C318/C323 fixed pick center/axis/radius');
const center=sourcePick?.point,radius=Math.sqrt(sourcePick?.radius_squared??NaN);
function fittedRotation(side,kernel){
  const before=mesh[`${side}_before`],after=mesh[`${side}_after`],ids=changes[side].changed_ids;
  let numerator=0,denominator=0;
  const rows=ids.map(id=>{
    const a=before.positions[id],b=after.positions[id];
    const x=a[0]-center[0],y=a[1]-center[1],u=b[0]-center[0],v=b[1]-center[1];
    const angle=Math.atan2(x*v-y*u,x*u+y*v);
    const d=Math.hypot(x,y,a[2]-center[2])/radius;
    const radial=kernel==='source'?(1-4*d**3+3*d**4):(1-d*d)**2;
    const mask=before.attributes[id][5],weight=radial*mask;
    numerator+=angle*weight;denominator+=weight*weight;
    return {id,a,b,x,y,angle,weight};
  });
  const totalAngle=numerator/denominator;
  let maxPositionResidual=0,sumSquared=0;
  for(const row of rows){
    const angle=totalAngle*row.weight;
    const predicted=[center[0]+row.x*Math.cos(angle)-row.y*Math.sin(angle),center[1]+row.x*Math.sin(angle)+row.y*Math.cos(angle),row.a[2]];
    const residual=Math.hypot(...predicted.map((x,c)=>x-row.b[c]));
    maxPositionResidual=Math.max(maxPositionResidual,residual);sumSquared+=residual*residual;
  }
  return {totalAngle,maxPositionResidual,rmsPositionResidual:Math.sqrt(sumSquared/rows.length),vertices:rows.length};
}
const fit={source:{own:fittedRotation('source','source'),opposite:fittedRotation('source','root')},root:{own:fittedRotation('root','root'),opposite:fittedRotation('root','source')}};
check(fit.source.own.maxPositionResidual<=1e-6&&fit.root.own.maxPositionResidual<=1e-6,'C323 fixed-center analytic-kernel model >1e-6');
const pathEffect={};
for(const side of ['source','root']){
  const final=compare(mesh[`${side}_after`].positions,c318[side].afterMesh.positions);
  const max=final.max_abs_component;
  pathEffect[side]={...final,classification:max<=1e-6?'path-independent at 1e-6':max>=1e-5?'measurable path dependence at 1e-5':'indeterminate 1e-6 to 1e-5'};
}
const audit={status:failures.length?'FAILED frozen C323 Twist differential gate; evidence retained':'COMPLETE scoped C323 Twist differential',failures,
  paths:{source:sourcePath,root:rootPath,c318:c318Paths},source_build_sha256:s.browser_build,obrush_wasm_sha256:r.browser_build,
  fixture_sha256:s.fixture.sha256,harness_sha256:s.harness_sha256,input,paired,changes,witnesses,history,preview,gpu,
  redoNormalMembership,samePick,fit,pathEffect,
  limitations:['One 1089-vertex settled-camera direct Twist arc with symmetry off; full row open','Path classification compares endpoints at fixed observed pick center; it does not prove absence of all evolving-state effects','Source WebGL1 upload shadow versus OBrush WebGL2 readback']};
const out=path.join(path.dirname(sourcePath),'paired-audit-'+path.basename(path.dirname(rootPath))+'.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,failures,fit,pathEffect}));
if(failures.length)process.exitCode=1;
