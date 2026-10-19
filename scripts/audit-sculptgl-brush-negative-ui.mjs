// Whole-public-save and active-renderer audit for a matched Negative polarity.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const local = path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? local);
const { decodeIndependent } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const { decodeRootF32Model } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);

const [sourcePath, rootPath] = process.argv.slice(2);
if (!sourcePath || !rootPath) throw Error('Pass source and root result.json paths');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const f32 = word => {const bytes=Buffer.allocUnsafe(4);bytes.writeUInt32LE(word);return bytes.readFloatLE();};
const source = JSON.parse(await readFile(sourcePath));
const root = JSON.parse(await readFile(rootPath));
if (source.side !== 'source' || root.side !== 'root' || source.fixture.sha256 !== root.fixture.sha256 ||
    source.negative !== root.negative || source.protocol_commit !== root.protocol_commit)
  throw Error('Paired fixture, polarity or protocol mismatch');
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
const signedRadialDisplacement=(before,after)=>{
  let sum=0,changed=0;
  for(let i=0;i<before.positions.length;i++){
    const b=before.positions[i],a=after.positions[i];
    const delta=a.map((v,c)=>v-b[c]);
    if(delta.some(v=>v!==0)){
      const radius=Math.hypot(...b);
      if(radius<1e-9)throw Error('Fixture has center vertex; radial sign undefined');
      sum+=delta.reduce((s,v,c)=>s+v*b[c]/radius,0);changed++;
    }
  }
  return {sum,mean:sum/changed,changed};
};
const audit={source_result:sourcePath,root_result:rootPath,fixture_sha256:source.fixture.sha256,
  negative:source.negative,source_build_sha256:source.browser_build,obrush_wasm_sha256:root.browser_build,
  requested_paths:{source:source.requested_path,root:root.requested_path},
  history:{source_before_equals_undo:loads.source_before.equals(loads.source_undo),source_after_equals_redo:loads.source_after.equals(loads.source_redo),root_before_equals_undo:loads.root_before.equals(loads.root_undo),root_after_equals_redo:loads.root_after.equals(loads.root_redo)},
  faces:{source_stable:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.source_after.faces),root_stable:JSON.stringify(meshes.root_before.faces)===JSON.stringify(meshes.root_after.faces),paired_before:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.root_before.faces),paired_after:JSON.stringify(meshes.source_after.faces)===JSON.stringify(meshes.root_after.faces)},
  attributes:{source_stable:compareAttributes(meshes.source_before,meshes.source_after),root_stable:compareAttributes(meshes.root_before,meshes.root_after),paired_before:compareAttributes(meshes.source_before,meshes.root_before),paired_after:compareAttributes(meshes.source_after,meshes.root_after)},
  before_pair:compare(meshes.source_before,meshes.root_before),after_pair:compare(meshes.source_after,meshes.root_after),
  source_change:compare(meshes.source_before,meshes.source_after),root_change:compare(meshes.root_before,meshes.root_after),
  radial_effect:{source:signedRadialDisplacement(meshes.source_before,meshes.source_after),
    root:signedRadialDisplacement(meshes.root_before,meshes.root_after)},
  active_gpu:gpuStages,display_normal_pair:displayNormalComparison,
  limitations:['Same requested pixel offsets are not proof of same 3D contacts or same delivered dabs','One normal UI gesture and checkbox state only; mid-stroke Alt/N and physical pen are untested','Source WebGL1 upload shadow differs from WebGL2 readback provenance','Display-normal differences do not imply saved mesh-normal differences']};
audit.status=Object.values(audit.history).every(Boolean)&&Object.values(audit.faces).every(Boolean)&&
  Object.values(gpuStages).every(x=>x.position_words_exact&&x.object_identity_exact)&&
  audit.source_change.different_vertices>0&&audit.root_change.different_vertices>0&&
  Math.sign(audit.radial_effect.source.mean)===(source.negative?-1:1)&&
  Math.sign(audit.radial_effect.root.mean)===(source.negative?-1:1)&&
  audit.after_pair.max_abs_component<=1e-7?'COMPLETE scoped paired geometry and active GPU observation':'FAILED paired admission; output retained';
audit.changed_vertex_membership_equal=JSON.stringify(audit.source_change.changed_ids)===JSON.stringify(audit.root_change.changed_ids);
if(!audit.changed_vertex_membership_equal ||
   audit.attributes.source_stable.different_components || audit.attributes.root_stable.different_components ||
   audit.attributes.paired_before.different_components || audit.attributes.paired_after.different_components ||
   audit.before_pair.different_components)
  audit.status='FAILED paired membership, attributes or initial geometry; output retained';
if(root.ray_alignment){
 const sourceProjectionX=source.setup.state.camera.projection[0],rootProjectionX=root.setup.state.camera.projection[0];
 const sourceWidth=source.setup.viewport.screen[2]-source.setup.viewport.screen[0],rootWidth=root.setup.viewport.screen[2]-root.setup.viewport.screen[0];
 const sourceRadius=source.config.state.tool._radius,rootRadius=root.config.state.source_geometry.per_tool[0].radius;
 const sourceWorldRadius=2*sourceRadius/(sourceWidth*sourceProjectionX),rootWorldRadius=2*rootRadius/(rootWidth*rootProjectionX);
 const sourceOffsets=source.requested_path.map(([x])=>x-(source.setup.viewport.screen[0]+source.setup.viewport.screen[2])/2),rootOffsets=root.requested_path.map(([x])=>x-(root.setup.viewport.screen[0]+root.setup.viewport.screen[2])/2);
 const rayX=sourceOffsets.map((dx,i)=>({source:2*dx/(sourceWidth*sourceProjectionX),root:2*rootOffsets[i]/(rootWidth*rootProjectionX)}));
 audit.ray_alignment={source_world_radius:sourceWorldRadius,root_world_radius:rootWorldRadius,
   world_radius_abs_difference:Math.abs(sourceWorldRadius-rootWorldRadius),initial_and_move_ray_x:rayX,
   max_ray_x_difference:Math.max(...rayX.map(x=>Math.abs(x.source-x.root)))};
}
const out=path.join(path.dirname(sourcePath),'paired-audit-'+path.basename(path.dirname(rootPath))+'.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,before_pair:audit.before_pair,after_pair:audit.after_pair,source_change:audit.source_change,root_change:audit.root_change,faces:audit.faces,history:audit.history}));
if(audit.status.startsWith('FAILED'))process.exitCode=1;
