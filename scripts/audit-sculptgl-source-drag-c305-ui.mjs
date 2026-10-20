// C305 frozen Source Drag symmetry/mask paired audit. Active GPU normals are the display oracle.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const {decodeIndependent}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const [sourcePath,rootPath]=process.argv.slice(2);
const pathOffsets=[[-100,-20],[-64,-20],[-28,-20],[8,-20]];
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
check(['off','on'].includes(runs.source.case_name)&&runs.source.case_name===runs.root.case_name&&runs.source.symmetry===runs.root.symmetry&&runs.source.symmetry===(runs.source.case_name==='on'),'Frozen symmetry case identity');
check(runs.source.protocol_commit==='c305-source-drag-symmetry-mask-integer-viewport808'&&runs.root.protocol_commit===runs.source.protocol_commit&&runs.source.harness_sha256===runs.root.harness_sha256&&runs.source.fixture.sha256===runs.root.fixture.sha256&&exact(runs.source.requested_brush,runs.root.requested_brush),'Frozen protocol/input identity');
check(runs.source.read_only_preflight_audit_sha256==='da28020451312241c1c4859e0050312f16b3683ff94c9a0b79eaf74157d6a57b'&&runs.root.read_only_preflight_audit_sha256===runs.source.read_only_preflight_audit_sha256,'Frozen passing C293 preflight linked');
for(const side of ['source','root'])for(const label of ['after_reset','after_fit']){
  const stable=runs[side].stability?.[label];
  check(stable?.terminal===true&&stable.frames>=12&&stable.elapsed_ms>=250&&stable.elapsed_ms<=5000&&
    typeof stable.signature==='string'&&stable.signature.length>0,`${side} ${label}: frozen settled camera`);
}
const s=runs.source,r=runs.root,p=s.setup.state.camera.projection,q=r.setup.state.camera.projection;
const sw=s.setup.viewport.screen[2]-s.setup.viewport.screen[0],sh=s.setup.viewport.screen[3]-s.setup.viewport.screen[1],rw=r.setup.viewport.screen[2]-r.setup.viewport.screen[0],rh=r.setup.viewport.screen[3]-r.setup.viewport.screen[1];
const worldRadiusS=2*s.config.state.tool._radius/(sw*p[0]),worldRadiusR=2*r.config.state.source_geometry.per_tool[7].radius/(rw*q[0]);
const rayX=pathOffsets.map(([x])=>x).map(x=>Math.abs(2*x/(sw*p[0])-2*x/(rw*q[0]))),rayY=[-20].map(y=>Math.abs(2*y/(sh*p[5])-2*y/(rh*q[5])));
const input={projection_x:q[0]/p[0]*rw/sw,projection_y:q[5]/p[5]*rh/sh,world_radius_difference:Math.abs(worldRadiusS-worldRadiusR),ray_x_differences:rayX,ray_y_differences:rayY,source_projection:[p[0],p[5]],root_projection:[q[0],q[5]]};
check(Math.abs(input.projection_x-1)<=1e-8&&Math.abs(input.projection_y-1)<=1e-8&&input.world_radius_difference<=1e-8&&Math.max(...rayX,...rayY)<=1e-8,'Projection/world-ray/radius setup');
const gpu={},history={},changes={},paired={},preview={},witnesses={};
for(const side of ['source','root']){
  const run=runs[side],v=run.setup.viewport.screen,center=[(v[0]+v[2])/2,(v[1]+v[3])/2],expected=pathOffsets.map(([x,y])=>[center[0]+x,center[1]+y]);
  const delivered=run.dom_stroke_events?.filter(e=>e.type==='pointermove'&&e.buttons===1).map(e=>[e.x,e.y]),down=run.dom_stroke_events?.filter(e=>e.type==='pointerdown'),up=run.dom_stroke_events?.filter(e=>e.type==='pointerup');
  check(exact(run.setup.viewport.logical_size,[1024,808])&&center.every(Number.isInteger)&&exact(run.requested_path,expected)&&exact(delivered,expected.slice(1))&&down?.length===1&&up?.length===1&&down[0].x===expected[0][0]&&down[0].y===expected[0][1]&&up[0].x===expected[3][0]&&up[0].y===expected[3][1]&&run.dom_stroke_events?.every(e=>e.trusted),`${side}: trusted integer UI stroke`);
  const tool=side==='source'?run.config.state.tool:run.config.state.source_geometry.per_tool[7];
  check(side==='source'?run.config.state.tool_index===7&&tool._lockPosition!==true&&[0,'None'].includes(tool._idAlpha)&&run.config.state.symmetry===run.symmetry&&!run.config.state.continuous:run.config.state.kind==='Drag'&&tool.mode==='SourceOrdinary'&&tool.options.tool==='Drag'&&!tool.options.culling&&!tool.options.prevent_folds&&!tool.options.negative&&run.config.state.symmetry_x===run.symmetry&&!run.config.state.continuous.requested&&!run.config.state.lock_position,`${side}: published Source Drag settings`);
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
  witnesses[side]=Object.fromEntries([541,544,545,548].map(id=>{const a=mesh[`${side}_before`].positions[id],b=mesh[`${side}_after`].positions[id];return [id,{before:a,after:b,displacement:Math.hypot(...a.map((v,i)=>b[i]-v))}]}));
  check(changes[side].changed_vertices>0&&changes[side].changed_ids.includes(541)&&(run.symmetry?changes[side].changed_ids.includes(548):true),`${side}: left RGB ID541 and mirrored ID548 when enabled must change`);
  check(exact(mesh[`${side}_before`].positions[544],mesh[`${side}_after`].positions[544]),`${side}: protected ID544 moved`);
  check(changes[side].changed_ids.includes(545),`${side}: half-unprotected ID545 did not move`);
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
const audit={status:failures.length?'FAILED frozen C305 Source Drag gate; evidence retained':'COMPLETE scoped C305 Source Drag case',failures,paths:{source:sourcePath,root:rootPath},source_build_sha256:s.browser_build,obrush_wasm_sha256:r.browser_build,fixture_sha256:s.fixture.sha256,harness_sha256:s.harness_sha256,input,paired,changes,witnesses,history,preview,gpu,redoNormalMembership,limitations:['One 1089-vertex settled-camera curved-grid Drag symmetry case; full row open','Source WebGL1 upload shadow versus OBrush WebGL2 readback','Public saved normals are not used as display-normal oracle']};
const out=path.join(path.dirname(sourcePath),'paired-audit-'+path.basename(path.dirname(rootPath))+'.json');await writeFile(out,JSON.stringify(audit,null,2)+'\n');console.log(JSON.stringify({out,status:audit.status,failures,paired_after:paired.after}));if(failures.length)process.exitCode=1;
