// C333 frozen Source Twist guard-safe path pair. Active GPU normals are the display oracle.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const {decodeIndependent}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const [sourcePath,rootPath]=process.argv.slice(2);
const pathOffsets=[[-48,-12],[-12,-12],[-15,-42],[-24,-43]];
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
  const rows=Array(ids.length);ids.forEach((id,i)=>{rows[id]=[0,1,2].map(c=>bytes.readFloatLE(i*12+c*4));});return rows;
}
check(runs.source.status?.startsWith('normal browser')&&runs.root.status?.startsWith('normal browser'),'Both normal-UI captures completed');
check(runs.source.case_name==='off'&&runs.root.case_name==='off'&&runs.source.symmetry===false&&runs.root.symmetry===false,'Frozen symmetry-off case identity');
check(runs.source.protocol_commit==='c333-source-twist-visible-strength-one-integer-viewport808'&&runs.root.protocol_commit===runs.source.protocol_commit&&runs.source.harness_sha256===runs.root.harness_sha256&&runs.source.fixture.sha256===runs.root.fixture.sha256&&exact(runs.source.requested_brush,runs.root.requested_brush),'Frozen protocol/input identity');
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
  check(exact(run.setup.viewport.logical_size,[1024,808])&&center.every(Number.isInteger)&&exact(run.requested_path,expected)&&exact(delivered,expected.slice(1))&&down?.length===1&&up?.length===1&&down[0].x===expected[0][0]&&down[0].y===expected[0][1]&&up[0].x===expected[3][0]&&up[0].y===expected[3][1]&&run.dom_stroke_events?.every(e=>e.trusted),`${side}: trusted integer UI stroke`);
  const tool=side==='source'?run.config.state.tool:run.config.state.preferences.tools.find(t=>t.kind==='Twist');
  check(side==='source'?run.config.state.tool_index===2&&tool._lockPosition!==true&&[0,'None'].includes(tool._idAlpha)&&run.config.state.symmetry===false&&!run.config.state.continuous:run.config.state.kind==='Twist'&&tool.screen_space_radius&&tool.screen_radius_pixels===75&&tool.strength===1&&!tool.backface_culling&&!tool.lock_position&&run.config.state.symmetry_x===false&&!run.config.state.continuous.requested,`${side}: published Twist settings`);
  history[side]={before_undo:files[`${side}_before`].equals(files[`${side}_undo`]),after_redo:files[`${side}_after`].equals(files[`${side}_redo`]),one_step:run.after_history?.undo===run.before_history?.undo+1};
  check(Object.values(history[side]).every(Boolean),`${side}: public history`);
  check(run.held_previews?.length===3,`${side}: three held snapshots`);
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
  for(let i=0;i<3;i++){const observed=run.held_previews[i].gpu;gpu[`${side}_held${i}`]={unique:observed.active_vbo_candidates===1&&observed.gl_error===0,identity:side==='source'||(Number.isInteger(observed.buffer_token)&&observed.buffer_token>0&&run.held_previews[i].live?.state?.active_subtool_id===1)};check(Object.values(gpu[`${side}_held${i}`]).every(Boolean),`${side} held${i}: active GPU/object`);}
  changes[side]=compare(mesh[`${side}_before`].positions,mesh[`${side}_after`].positions);
  witnesses[side]=Object.fromEntries([541,543,577,578].map(id=>{const a=mesh[`${side}_before`].positions[id],b=mesh[`${side}_after`].positions[id];return [id,{before:a,after:b,displacement:Math.hypot(...a.map((v,i)=>b[i]-v))}]}));
  check(changes[side].changed_vertices>0,`${side}: Twist changed no vertices`);
  check(exact(mesh[`${side}_before`].positions[543],mesh[`${side}_after`].positions[543]),`${side}: protected ID543 moved`);
  check(changes[side].changed_ids.includes(577)&&changes[side].changed_ids.includes(578),`${side}: witness IDs did not move`);
  const held=[0,1,2].map(i=>gpuRows(side,'held','position',i));
  check(compare(held[0],held[1]).changed_vertices>0&&compare(held[1],held[2]).changed_vertices>0,`${side}: held captures do not advance`);
}
for(const stage of ['before','after','undo','redo']){
  paired[stage]={positions:compare(mesh[`source_${stage}`].positions,mesh[`root_${stage}`].positions),normals:compare(gpuRows('source',stage,'normal'),gpuRows('root',stage,'normal'))};
  check(paired[stage].positions.max_abs_component<=1e-6&&paired[stage].normals.max_abs_component<=1e-6,`${stage}: paired XYZ/display normals >1e-6`);
  if(stage==='before')check(paired[stage].positions.different_components===0,'Initial positions differ');
  check(exact(mesh[`source_${stage}`].attributes,mesh[`root_${stage}`].attributes)&&exact(mesh[`source_${stage}`].faces,mesh[`root_${stage}`].faces),`${stage}: attributes/faces differ`);
}
for(let i=0;i<3;i++){paired[`held${i}`]={positions:compare(gpuRows('source','held','position',i),gpuRows('root','held','position',i)),normals:compare(gpuRows('source','held','normal',i),gpuRows('root','held','normal',i))};check(paired[`held${i}`].positions.max_abs_component<=1e-6&&paired[`held${i}`].normals.max_abs_component<=1e-6,`held${i}: paired GPU XYZ/normals >1e-6`);}
check(exact(changes.source.changed_ids,changes.root.changed_ids),'Changed RGB membership differs');
const redoNormalMembership={};
for(const side of ['source','root']){
  const after=gpuRows(side,'after','normal'),redo=gpuRows(side,'redo','normal');
  redoNormalMembership[side]=compare(after,redo).changed_ids;
}
check(exact(redoNormalMembership.source,redoNormalMembership.root),'After-to-Redo active display-normal changed RGB membership differs');

// C333 separate selected-membership, mask-normalized angle and actual mesh guard.
const selected=[],normalMembership={},maskAngle={},guard={};
const picked=s.press_pick;
check(picked?.point?.length===3&&picked.radius_squared>0,'Source press pick unavailable');
const selectedIds=[474,475,476,477,478,506,507,508,509,510,511,539,540,541,542,543,544,545,572,573,574,575,576,577,578,605,606,607,608,609,610,639,640,641,642,643,673,674];
function membership(positions){return positions.flatMap((p,id)=>p.reduce((sum,x,i)=>sum+(x-picked.point[i])**2,0)<picked.radius_squared?[id]:[]);}
const beforePositions=mesh.source_before.positions;
for(const side of ['source','root']){
  const b=gpuRows(side,'before','normal');
  const baseline=gpuRows(side,'before','position');
  for(let i=0;i<3;i++){
    const stage=`held${i}`,pos=gpuRows(side,'held','position',i),norm=gpuRows(side,'held','normal',i);
    const ids=membership(pos),changedNormals=compare(b,norm).changed_ids;
    selected.push({side,stage,ids,changedNormals});
    check(exact(ids,selectedIds),`${side} ${stage}: selected membership differs from frozen 38 IDs`);
    check(changedNormals.every(id=>ids.includes(id)),`${side} ${stage}: normals changed outside selected set`);
    normalMembership[`${side}_${stage}`]=changedNormals;
    check(exact(pos[543],baseline[543]),`${side} ${stage}: protected ID543 moved`);
  }
  const first=gpuRows(side,'held','position',1);
  maskAngle[side]={};
  for(const id of [577,578]){
    const a=baseline[id],b=first[id],cx=picked.point[0],cy=picked.point[1];
    const u=[a[0]-cx,a[1]-cy],v=[b[0]-cx,b[1]-cy];
    const angle=Math.atan2(u[0]*v[1]-u[1]*v[0],u[0]*v[0]+u[1]*v[1]);
    const d=Math.sqrt(a.reduce((sum,x,i)=>sum+(x-picked.point[i])**2,0)/picked.radius_squared);
    const falloff=1-4*d**3+3*d**4;
    const unprotected=mesh[`${side}_before`].attributes[id].at(-1);
    const normalized=angle/(unprotected*falloff);
    maskAngle[side][id]={angle,d,falloff,unprotected,normalized};
  }
  check(Math.abs(maskAngle[side][577].normalized-maskAngle[side][578].normalized)<=5e-6,`${side}: mask-normalized first-held angles diverge`);
}
for(const id of [577,578])check(Math.abs(maskAngle.source[id].normalized-maskAngle.root[id].normalized)<=5e-6,`paired ID${id}: mask-normalized angles diverge`);
function vecSub(a,b){return a.map((x,i)=>x-b[i]);}
function dot(a,b){return a.reduce((s,x,i)=>s+x*b[i],0);}
function cross(a,b){return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];}
function length(a){return Math.sqrt(dot(a,a));}
function faceCross(p,f){const [a,b,c]=f.map(id=>p[id]);return cross(vecSub(b,a),vecSub(c,a));}
function edge2(p,f){const [a,b,c]=f.map(id=>p[id]);return Math.max(dot(vecSub(a,b),vecSub(a,b)),dot(vecSub(b,c),vecSub(b,c)),dot(vecSub(c,a),vecSub(c,a)));}
function guardMetrics(before,after,faces,changedIds){
  const changed=new Set(changedIds),affected=new Set(faces.flatMap((f,i)=>f.some(id=>changed.has(id))?[i]:[]));
  const failureFaces=[];let minimumProjectionRatio=Infinity,minimumQualityRatio=Infinity;
  for(const i of affected){const f=faces[i],oc=faceCross(before,f),nc=faceCross(after,f),oa=dot(oc,oc),oe=edge2(before,f),ne=edge2(after,f);
    const projectionRatio=dot(nc,oc)/oa,qualityRatio=length(nc)/ne,requiredQuality=Math.min(Math.sqrt(oa)/oe,0.1);
    minimumProjectionRatio=Math.min(minimumProjectionRatio,projectionRatio);minimumQualityRatio=Math.min(minimumQualityRatio,qualityRatio);
    if(dot(nc,oc)<oa*0.05||dot(nc,nc)*(1+32*Number.EPSILON)<(requiredQuality*ne)**2)failureFaces.push({index:i,face:f,projectionRatio,qualityRatio});}
  const edgeFaces=new Map();faces.forEach((f,i)=>{for(const [a,b] of [[f[0],f[1]],[f[1],f[2]],[f[2],f[0]]]){const key=Math.min(a,b)+','+Math.max(a,b),rows=edgeFaces.get(key)??[];rows.push(i);edgeFaces.set(key,rows);}});
  const failureFolds=[];let minimumFoldCosine=Infinity,checkedFoldPairs=0;
  for(const [edge,pair] of edgeFaces){if(pair.length!==2||!pair.some(i=>affected.has(i)))continue;checkedFoldPairs++;
    const on=pair.map(i=>faceCross(before,faces[i])),nn=pair.map(i=>faceCross(after,faces[i]));
    const op=length(on[0])*length(on[1]),np=length(nn[0])*length(nn[1]);
    const oldCosine=dot(on[0],on[1])/op,newCosine=dot(nn[0],nn[1])/np;minimumFoldCosine=Math.min(minimumFoldCosine,newCosine);
    if(dot(nn[0],nn[1])+32*Number.EPSILON*np<Math.min(oldCosine,0)*np)failureFolds.push({edge,pair,oldCosine,newCosine});}
  return {affectedFaces:affected.size,checkedFoldPairs,minimumProjectionRatio,minimumQualityRatio,minimumFoldCosine,failureFaces,failureFolds};
}
for(const side of ['source','root']){
  const before=gpuRows(side,'before','position'),faces=mesh[`${side}_before`].faces.map(s=>s.split(',').map(Number));guard[side]={};
  for(const i of [1,2]){const stage=`held${i}`,after=gpuRows(side,'held','position',i),changed=compare(before,after).changed_ids;
    guard[side][stage]=guardMetrics(before,after,faces,changed);
    check(guard[side][stage].failureFaces.length===0&&guard[side][stage].failureFolds.length===0,`${side} ${stage}: actual mesh violates unchanged fold guard`);}
  guard[side].after=guardMetrics(before,mesh[`${side}_after`].positions,faces,changes[side].changed_ids);
  check(guard[side].after.failureFaces.length===0&&guard[side].after.failureFolds.length===0,`${side} after: actual mesh violates unchanged fold guard`);
}
const audit={status:failures.length?'FAILED frozen C333 Source Twist gate; evidence retained':'COMPLETE scoped C333 Source Twist case',failures,paths:{source:sourcePath,root:rootPath},source_build_sha256:s.browser_build,obrush_wasm_sha256:r.browser_build,fixture_sha256:s.fixture.sha256,harness_sha256:s.harness_sha256,input,paired,changes,witnesses,history,preview,gpu,redoNormalMembership,selected,normalMembership,maskAngle,guard,limitations:['One 1089-vertex guard-safe Twist arc with symmetry off; full row open','Source WebGL1 upload shadow versus OBrush WebGL2 readback','Public saved normals are not used as display-normal oracle','C318 folded-target and displacement-order failures remain in prior frozen receipts']};
const out=path.join(path.dirname(sourcePath),'paired-audit-'+path.basename(path.dirname(rootPath))+'.json');await writeFile(out,JSON.stringify(audit,null,2)+'\n');console.log(JSON.stringify({out,status:audit.status,failures,paired_after:paired.after,paired_held1:paired.held1,paired_held2:paired.held2,maskAngle,guard_summary:Object.fromEntries(Object.entries(guard).map(([side,stages])=>[side,Object.fromEntries(Object.entries(stages).map(([stage,m])=>[stage,{affectedFaces:m.affectedFaces,minimumProjectionRatio:m.minimumProjectionRatio,minimumQualityRatio:m.minimumQualityRatio,minimumFoldCosine:m.minimumFoldCosine,failureFaces:m.failureFaces.length,failureFolds:m.failureFolds.length}]))]))}));if(failures.length)process.exitCode=1;
