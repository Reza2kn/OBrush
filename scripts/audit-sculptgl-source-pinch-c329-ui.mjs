// C329 contact-calibrated whole-public-save and active-renderer Pinch audit.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const local = path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? local);
const { decodeIndependent } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const { decodeRootF32Model } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);

const [sourcePath, rootPath, sourcePressPath, rootPressPath] = process.argv.slice(2);
if (!sourcePath || !rootPath || !sourcePressPath || !rootPressPath)
  throw Error('Pass source/root press-move and source/root press-only result.json paths');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const f32 = word => {const bytes=Buffer.allocUnsafe(4);bytes.writeUInt32LE(word);return bytes.readFloatLE();};
const source = JSON.parse(await readFile(sourcePath));
const root = JSON.parse(await readFile(rootPath));
const sourcePress = JSON.parse(await readFile(sourcePressPath));
const rootPress = JSON.parse(await readFile(rootPressPath));
if (source.side !== 'source' || root.side !== 'root' || source.fixture.sha256 !== root.fixture.sha256 ||
    source.negative !== root.negative || source.protocol_commit !== root.protocol_commit)
  throw Error('Paired fixture, polarity or protocol mismatch');
for(const [side,run,full] of [['source',sourcePress,source],['root',rootPress,root]]){
  if(run.side!==side||run.negative!==full.negative||run.fixture.sha256!==full.fixture.sha256||
     run.protocol_commit!==full.protocol_commit||run.stroke_mode!=='press-only'||
     run.browser_build!==full.browser_build||run.status.startsWith('FAILED'))
    throw Error(`C329 ${side} press-only control identity/status mismatch`);
}
if(source.stroke_mode!=='press-move'||root.stroke_mode!=='press-move'||
   source.status.startsWith('FAILED')||root.status.startsWith('FAILED'))
  throw Error('C329 paired press-move identity/status mismatch');
const validCenter = p => Array.isArray(p) && p.length === 3 && p.every(Number.isFinite);
if (source.press_pick?.picked_vertices !== 0 || root.press_pick !== null)
  throw Error('C329 requires a zero-vertex source press and null OBrush press pass');
if (!validCenter(source.move_pick?.point) || !validCenter(root.move_pick?.center))
  throw Error('C329 requires both measured effective move centers');
const loads = {};
for (const [side,result] of [['source',source],['root',root]]) {
  for (const stage of ['before','after','undo','redo']) {
    const bytes = await readFile(result[stage].path);
    if (sha(bytes) !== result[stage].sha256) throw Error(`${side} ${stage} public-save hash changed`);
    loads[`${side}_${stage}`] = bytes;
  }
}
const decode = (side,bytes) => {
  if (side === 'root') return decodeRootF32Model(bytes);
  const m = decodeIndependent(bytes).meshes[0];
  return {positions:m.positions.map(row=>row.map(f32)),colors:m.colors.map(row=>row.map(f32)),
    materials:m.materials.map(row=>row.map(f32)),unprotected:m.materials.map(row=>f32(row[2])),faces:m.faces};
};
const canonical = data => {
  const positions=[],attributes=[],faces=[],seen=new Set();
  const ids=data.colors.map(c=>{const id=Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16);if(seen.has(id))throw Error('Duplicate RGB vertex ID');seen.add(id);return id;});
  ids.forEach((id,i)=>{positions[id]=data.positions[i];attributes[id]=[...data.colors[i],...data.materials[i].slice(0,2),data.unprotected[i]];});
  for (const face of data.faces) {
    const row=face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]);
    const k=row.indexOf(Math.min(...row));faces.push([...row.slice(k),...row.slice(0,k)].join(','));
  }
  faces.sort();
  return {positions,attributes,faces,ids};
};
const meshes={};
for(const side of ['source','root'])for(const stage of ['before','after'])meshes[`${side}_${stage}`]=canonical(decode(side,loads[`${side}_${stage}`]));
const gpuStages={};
for(const side of ['source','root'])for(const stage of ['before','after','undo','redo']){
  const receipt=side==='source'?source:root, observed=receipt[`${stage}_gpu`];
  const mesh=decode(side,loads[`${side}_${stage}`]);
  if(!observed || observed.active_vbo_candidates!==1 || observed.gl_error!==0)
    throw Error(`${side} ${stage}: missing unique, error-free active GPU words`);
  const packed=Buffer.alloc(mesh.positions.length*12);
  mesh.positions.forEach((row,i)=>row.forEach((v,c)=>packed.writeFloatLE(v,i*12+c*4)));
  const expected=sha(packed), actual=observed.position_words_sha256;
  const normal=Buffer.from(observed.normal_words_base64??'', 'base64');
  if(normal.length!==packed.length || sha(normal)!==observed.normal_words_sha256)
    throw Error(`${side} ${stage}: invalid actual display-normal word stream`);
  if(side==='source'){
    const actualPosition=Buffer.from(observed.position_words_base64??'', 'base64');
    if(actualPosition.length!==packed.length || sha(actualPosition)!==actual)
      throw Error(`${side} ${stage}: invalid forwarded active position stream`);
  }
  const identity=side==='root'?receipt[`${stage}_object_identity`]:null;
  const objectIdentityExact=side==='source'?true:!!identity&&
    identity.active_subtool_id===1&&identity.vertices===receipt.fixture.vertices&&
    JSON.stringify(identity.selected_subtool_ids)==='[1]'&&
    identity.subtools?.length===1&&identity.subtools[0].id===1&&
    identity.subtools[0].vertices===receipt.fixture.vertices&&identity.subtools[0].visible===true&&
    Number.isInteger(observed.buffer_token)&&observed.buffer_token>0&&
    Number.isInteger(observed.last_draw_sequence)&&observed.last_draw_sequence>0;
  gpuStages[`${side}_${stage}`]={expected_position_sha256:expected,actual_position_sha256:actual,
    position_words_exact:expected===actual,normal_words_sha256:observed.normal_words_sha256,
    object_identity_exact:objectIdentityExact,object_identity:identity,
    buffer_token:observed.buffer_token??null,observation:observed.observation??null,
    last_draw_sequence:observed.last_draw_sequence??null,
    source_kind:side==='source'?'WebGL1 active-draw forwarded uploads':'WebGL2 active-draw readback'};
}
const displayNormalComparison={};
for(const stage of ['before','after','undo','redo']){
  const rows={};
  for(const side of ['source','root']){
    const receipt=side==='source'?source:root;
    const mesh=decode(side,loads[`${side}_${stage}`]);
    const bytes=Buffer.from(receipt[`${stage}_gpu`].normal_words_base64,'base64');
    const ids=mesh.colors.map(c=>Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16));
    const mapped=Array(ids.length);for(let i=0;i<ids.length;i++){
      if(mapped[ids[i]])throw Error(`${side} ${stage}: duplicate GPU normal ID`);
      mapped[ids[i]]=bytes.subarray(i*12,i*12+12);
    }
    if(mapped.some(x=>!x))throw Error(`${side} ${stage}: missing GPU normal ID`);
    rows[side]=mapped;
  }
  let differing=0,components=0,max=0,sum=0;
  for(let i=0;i<rows.source.length;i++){
    let changed=false;
    for(let c=0;c<3;c++){
      const a=rows.source[i].readFloatLE(c*4),b=rows.root[i].readFloatLE(c*4),d=a-b;
      if(!Number.isFinite(a)||!Number.isFinite(b))throw Error(`${stage}: non-finite GPU normal`);
      if(!Object.is(a,b)){changed=true;components++;}max=Math.max(max,Math.abs(d));sum+=d*d;
    }
    differing+=Number(changed);
  }
  displayNormalComparison[stage]={different_vertex_ids:differing,different_components:components,
    max_abs_component:max,rms_component:Math.sqrt(sum/(rows.source.length*3)),
    oracle:'Both active GPU display-normal Float32 streams; never public Save normals'};
}
const compareAttributes=(a,b)=>{
  if(a.attributes.length!==b.attributes.length)throw Error('Attribute vertex count mismatch');
  let ids=0,components=0,max=0;
  for(let i=0;i<a.attributes.length;i++){
    const x=a.attributes[i],y=b.attributes[i];if(!x||!y||x.length!==y.length)throw Error('Attribute row missing '+i);
    let different=false;for(let c=0;c<x.length;c++){const delta=x[c]-y[c];if(Math.fround(x[c])!==Math.fround(y[c])){different=true;components++;}max=Math.max(max,Math.abs(delta));}
    ids+=Number(different);
  }
  return {different_vertex_ids:ids,different_components:components,max_abs_component:max};
};
const compare = (a,b) => {
  if(a.positions.length!==b.positions.length)throw Error('Paired vertex count mismatch');
  let changed=0,components=0,max=0,sum=0,examples=[],changedIds=[];
  for(let i=0;i<a.positions.length;i++){
    const x=a.positions[i],y=b.positions[i];if(!x||!y)throw Error('Missing mapped RGB vertex ID '+i);
    let different=false;for(let c=0;c<3;c++){const delta=x[c]-y[c],wordDifferent=Math.fround(x[c])!==Math.fround(y[c]);different||=wordDifferent;components+=Number(wordDifferent);max=Math.max(max,Math.abs(delta));sum+=delta*delta;}
    if(different){changed++;changedIds.push(i);if(examples.length<12)examples.push({id:i,left:x,right:y});}
  }
  return {vertices:a.positions.length,different_vertices:changed,different_components:components,
    changed_ids:changedIds,max_abs_component:max,rms_component:Math.sqrt(sum/(3*a.positions.length)),examples};
};
const pressControls={};
for(const [side,run] of [['source',sourcePress],['root',rootPress]]){
  const files={};
  for(const stage of ['before','after']){
    const bytes=await readFile(run[stage].path);
    if(sha(bytes)!==run[stage].sha256)throw Error(`${side} C329 press-only ${stage} public-save hash changed`);
    files[stage]=canonical(decode(side,bytes));
  }
  const beforeAfter=compare(files.before,files.after);
  pressControls[side]={before_after:beforeAfter,
    faces_exact:JSON.stringify(files.before.faces)===JSON.stringify(files.after.faces),
    attributes:compareAttributes(files.before,files.after),
    public_before_sha256:run.before.sha256,public_after_sha256:run.after.sha256,
    gpu_position_exact:run.before_gpu?.position_words_sha256===run.after_gpu?.position_words_sha256,
    history_unchanged:run.before_history?.undo===run.after_history?.undo,
    admissions:run.contact_admissions,protocol_failures:run.protocol_failures??[]};
}
const contactCountsExact=(run,full)=>{
  const expected=full?[['press held',0],['held move',2],['released',2]]:[['press held',0],['released',0]];
  const rows=run.contact_admissions??[];
  if(rows.length!==expected.length||run.protocol_failures?.length)return false;
  return expected.every(([label,count],i)=>rows[i].label===label&&rows[i].count===count&&rows[i].pass===true);
};
const completedContacts={source_full:contactCountsExact(source,true),root_full:contactCountsExact(root,true),
  source_press:contactCountsExact(sourcePress,false),root_press:contactCountsExact(rootPress,false),
  source_trace:source.source_contact_trace?.errors?.length===0&&
    source.source_contact_trace?.rows?.length===2&&
    source.source_contact_trace.rows.every(row=>row.completed===true&&row.phase==='move'&&validCenter(row.center)),
  root_drop_free:[root,rootPress].every(run=>run.final_contact_metrics?.browser_pen_moves_dropped===0&&
    run.final_contact_metrics?.browser_pen_moves_overflowed===false&&
    run.final_contact_metrics?.pending_dabs===0&&run.final_contact_metrics?.browser_pen_moves_queued===0)};
const centerDirectedDisplacement=(before,after,run)=>{
  const move=run.move_pick?.point??run.move_pick?.center;
  const center=move;
  let sum=0,changed=0,exceptions=0;
  const per_id=[];
  for(let i=0;i<before.positions.length;i++){
    const b=before.positions[i],a=after.positions[i];
    const delta=a.map((v,c)=>v-b[c]);
    if(delta.some(v=>v!==0)){
      const toward=center.map((v,c)=>v-b[c]);
      const radius=Math.hypot(...toward);
      if(radius<1e-12)continue;
      const projection=delta.reduce((s,v,c)=>s+v*toward[c]/radius,0);
      sum+=projection;changed++;exceptions+=Number(Math.sign(projection)===(run.negative?-1:1)*-1);
      per_id.push({id:i,projection});
    }
  }
  return {center,move,sum,mean:sum/changed,changed,exceptions,per_id};
};
const audit={source_result:sourcePath,root_result:rootPath,
  source_press_result:sourcePressPath,root_press_result:rootPressPath,
  fixture_sha256:source.fixture.sha256,
  negative:source.negative,source_build_sha256:source.browser_build,obrush_wasm_sha256:root.browser_build,
  requested_paths:{source:source.requested_path,root:root.requested_path},
  press_controls:pressControls,completed_contacts:completedContacts,
  history:{source_before_equals_undo:loads.source_before.equals(loads.source_undo),source_after_equals_redo:loads.source_after.equals(loads.source_redo),root_before_equals_undo:loads.root_before.equals(loads.root_undo),root_after_equals_redo:loads.root_after.equals(loads.root_redo),source_one_step:source.after_history?.undo===source.before_history?.undo+1,root_one_step:root.after_history?.undo===root.before_history?.undo+1},
  faces:{source_stable:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.source_after.faces),root_stable:JSON.stringify(meshes.root_before.faces)===JSON.stringify(meshes.root_after.faces),paired_before:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.root_before.faces),paired_after:JSON.stringify(meshes.source_after.faces)===JSON.stringify(meshes.root_after.faces)},
  attributes:{source_stable:compareAttributes(meshes.source_before,meshes.source_after),root_stable:compareAttributes(meshes.root_before,meshes.root_after),paired_before:compareAttributes(meshes.source_before,meshes.root_before),paired_after:compareAttributes(meshes.source_after,meshes.root_after)},
  before_pair:compare(meshes.source_before,meshes.root_before),after_pair:compare(meshes.source_after,meshes.root_after),
  source_change:compare(meshes.source_before,meshes.source_after),root_change:compare(meshes.root_before,meshes.root_after),
  center_effect:{source:centerDirectedDisplacement(meshes.source_before,meshes.source_after,source),
    root:centerDirectedDisplacement(meshes.root_before,meshes.root_after,root)},
  active_gpu:gpuStages,display_normal_pair:displayNormalComparison,
  limitations:['Independent press-only replay is not an in-stroke public export','Pass-through Source Pinch method wrapper may perturb timing','One ordinary mouse gesture and checkbox state only; radius/pressure/alpha/culling/lock/dynamic topology remain open','Source WebGL1 upload shadow differs from WebGL2 readback provenance','Display-normal differences do not imply saved mesh-normal differences']};
audit.changed_vertex_membership_equal=JSON.stringify(audit.source_change.changed_ids)===JSON.stringify(audit.root_change.changed_ids);
if(root.ray_alignment){
 const sourceProjectionX=source.setup.state.camera.projection[0],rootProjectionX=root.setup.state.camera.projection[0];
 const sourceProjectionY=source.setup.state.camera.projection[5],rootProjectionY=root.setup.state.camera.projection[5];
 const sourceWidth=source.setup.viewport.screen[2]-source.setup.viewport.screen[0],rootWidth=root.setup.viewport.screen[2]-root.setup.viewport.screen[0];
 const sourceHeight=source.setup.viewport.screen[3]-source.setup.viewport.screen[1],rootHeight=root.setup.viewport.screen[3]-root.setup.viewport.screen[1];
 const sourceRadius=source.config.state.tool._radius,rootRadius=root.config.state.source_geometry.per_tool[3].radius;
 const sourceWorldRadius=2*sourceRadius/(sourceWidth*sourceProjectionX),rootWorldRadius=2*rootRadius/(rootWidth*rootProjectionX);
 const sourceWorldRadiusY=2*sourceRadius/(sourceHeight*sourceProjectionY),rootWorldRadiusY=2*rootRadius/(rootHeight*rootProjectionY);
 const sourceOffsets=source.requested_path.map(([x])=>x-(source.setup.viewport.screen[0]+source.setup.viewport.screen[2])/2),rootOffsets=root.requested_path.map(([x])=>x-(root.setup.viewport.screen[0]+root.setup.viewport.screen[2])/2);
 const rayX=sourceOffsets.map((dx,i)=>({source:2*dx/(sourceWidth*sourceProjectionX),root:2*rootOffsets[i]/(rootWidth*rootProjectionX)}));
 const sourceCenters=[source.move_pick.point];
 const rootCenters=[root.move_pick.center];
 const centerDifferences=[];
 for(let i=0;i<1;i++)for(let c=0;c<3;c++)
   centerDifferences.push(Math.abs((sourceCenters[i]?.[c]??Infinity)-(rootCenters[i]?.[c]??-Infinity)));
 audit.ray_alignment={source_world_radius:sourceWorldRadius,root_world_radius:rootWorldRadius,
   world_radius_abs_difference:Math.abs(sourceWorldRadius-rootWorldRadius),initial_and_move_ray_x:rayX,
   source_world_radius_y:sourceWorldRadiusY,root_world_radius_y:rootWorldRadiusY,
   world_radius_y_abs_difference:Math.abs(sourceWorldRadiusY-rootWorldRadiusY),
   max_ray_x_difference:Math.max(...rayX.map(x=>Math.abs(x.source-x.root))),
   horizontal_projection_ratio:rootProjectionX/sourceProjectionX*rootWidth/sourceWidth,
   vertical_projection_ratio:rootProjectionY/sourceProjectionY*rootHeight/sourceHeight,
   source_pick_centers:sourceCenters,root_pick_centers:rootCenters,
   max_pick_center_abs_difference:Math.max(...centerDifferences)};
}
const allAttributesExact=Object.values(audit.attributes).every(x=>x.different_components===0);
const gpuExact=Object.values(gpuStages).every(x=>x.position_words_exact&&x.object_identity_exact);
const gpuNormalBound=Object.values(displayNormalComparison).every(x=>x.max_abs_component<=1e-6);
const protectedId=1201, halfId=1202;
const protectedExact=['source','root'].every(side=>
  meshes[`${side}_before`].positions[protectedId].every((v,c)=>Object.is(v,meshes[`${side}_after`].positions[protectedId][c])));
const halfMoved=['source','root'].every(side=>
  meshes[`${side}_before`].positions[halfId].some((v,c)=>!Object.is(v,meshes[`${side}_after`].positions[halfId][c])));
const desiredSign=source.negative?-1:1;
audit.gates={
  press_only_public_exact:['source','root'].every(side=>{
    const p=pressControls[side];return p.before_after.different_components===0&&p.faces_exact&&
      p.attributes.different_components===0&&p.gpu_position_exact&&p.history_unchanged;
  }),
  completed_contacts:Object.values(completedContacts).every(Boolean),
  complete_history:Object.values(audit.history).every(Boolean),
  faces_exact:Object.values(audit.faces).every(Boolean),
  attributes_exact:allAttributesExact,
  paired_before_exact:audit.before_pair.different_components===0,
  nonzero_change:audit.source_change.different_vertices>0&&audit.root_change.different_vertices>0,
  changed_ids_exact:audit.changed_vertex_membership_equal,
  protected_1201_exact:protectedExact,
  half_1202_moves:halfMoved,
  center_direction:Math.sign(audit.center_effect.source.mean)===desiredSign&&
    Math.sign(audit.center_effect.root.mean)===desiredSign,
  paired_xyz_bound:audit.after_pair.max_abs_component<=1e-6,
  active_gpu_position_exact:gpuExact,
  active_gpu_normal_bound:gpuNormalBound,
  calibrated_inputs:!!audit.ray_alignment&&
    JSON.stringify(source.setup.viewport.canvas)==='[1024,768]'&&
    JSON.stringify(root.setup.viewport.logical_size)==='[1024,768]'&&
    source.setup.viewport.rect[2]-source.setup.viewport.rect[0]===1024&&
    source.setup.viewport.rect[3]-source.setup.viewport.rect[1]===768&&
    root.ray_alignment?.max_ray_difference<=1e-8&&
    root.ray_alignment?.world_radius_abs_difference<=1e-8&&
    root.ray_alignment?.world_radius_y_abs_difference<=1e-8&&
    audit.ray_alignment.world_radius_abs_difference<=1e-8&&
    audit.ray_alignment.world_radius_y_abs_difference<=1e-8&&
    audit.ray_alignment.max_ray_x_difference<=1e-8&&
    audit.ray_alignment.max_pick_center_abs_difference<=1e-8,
};
audit.failed_gates=Object.entries(audit.gates).filter(([,passed])=>!passed).map(([name])=>name);
audit.status=audit.failed_gates.length?'FAILED C329 paired Pinch gate; output retained':'PASSED scoped C329 paired Pinch polarity';
const out=path.join(path.dirname(sourcePath),'paired-audit-c329-'+path.basename(path.dirname(rootPath))+'.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,failed_gates:audit.failed_gates,before_pair:audit.before_pair,after_pair:audit.after_pair,source_change:audit.source_change,root_change:audit.root_change,faces:audit.faces,history:audit.history}));
if(audit.status.startsWith('FAILED'))process.exitCode=1;
