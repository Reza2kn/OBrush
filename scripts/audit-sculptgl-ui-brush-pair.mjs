// Independent whole-public-save audit for two separately owned normal UI runs.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { decodeIndependent } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs';
import { decodeRootF32Model } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs';

const [sourcePath, rootPath] = process.argv.slice(2);
if (!sourcePath || !rootPath) throw Error('Pass source and root result.json paths');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const f32 = word => {const bytes=Buffer.allocUnsafe(4);bytes.writeUInt32LE(word);return bytes.readFloatLE();};
const source = JSON.parse(await readFile(sourcePath));
const root = JSON.parse(await readFile(rootPath));
if (source.side !== 'source' || root.side !== 'root' || source.fixture.sha256 !== root.fixture.sha256) throw Error('Paired fixture/side mismatch');
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
const audit={source_result:sourcePath,root_result:rootPath,fixture_sha256:source.fixture.sha256,
  requested_paths:{source:source.requested_path,root:root.requested_path},
  history:{source_before_equals_undo:loads.source_before.equals(loads.source_undo),source_after_equals_redo:loads.source_after.equals(loads.source_redo),root_before_equals_undo:loads.root_before.equals(loads.root_undo),root_after_equals_redo:loads.root_after.equals(loads.root_redo)},
  faces:{source_stable:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.source_after.faces),root_stable:JSON.stringify(meshes.root_before.faces)===JSON.stringify(meshes.root_after.faces),paired_before:JSON.stringify(meshes.source_before.faces)===JSON.stringify(meshes.root_before.faces),paired_after:JSON.stringify(meshes.source_after.faces)===JSON.stringify(meshes.root_after.faces)},
  attributes:{source_stable:compareAttributes(meshes.source_before,meshes.source_after),root_stable:compareAttributes(meshes.root_before,meshes.root_after),paired_before:compareAttributes(meshes.source_before,meshes.root_before),paired_after:compareAttributes(meshes.source_after,meshes.root_after)},
  before_pair:compare(meshes.source_before,meshes.root_before),after_pair:compare(meshes.source_after,meshes.root_after),
  source_change:compare(meshes.source_before,meshes.source_after),root_change:compare(meshes.root_before,meshes.root_after),
  limitations:['Same requested pixel offsets are not proof of same 3D contacts or same delivered dabs','Normal browser scene/camera setup is separately performed and must be audited','This audit compares whole public saves; it does not inspect GPU buffers or physical pen latency']};
audit.status=Object.values(audit.history).every(Boolean)&&Object.values(audit.faces).every(Boolean)&&audit.source_change.different_vertices>0&&audit.root_change.different_vertices>0?'COMPLETE paired whole-file comparison with changed geometry; equal response not assumed':'FAILED paired admission; output retained';
audit.changed_vertex_membership_equal=JSON.stringify(audit.source_change.changed_ids)===JSON.stringify(audit.root_change.changed_ids);
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
