// C278 frozen four-run normal-UI Brush Clay audit. Saved mesh normals are not a GPU oracle.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const {decodeIndependent}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const [sourceOnPath,rootOnPath,sourceOffPath,rootOffPath]=process.argv.slice(2);
if(![sourceOnPath,rootOnPath,sourceOffPath,rootOffPath].every(Boolean))
  throw Error('Pass source-on, root-on, source-off and root-off result.json paths');
const paths={source_on:sourceOnPath,root_on:rootOnPath,source_off:sourceOffPath,root_off:rootOffPath};
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const runs=Object.fromEntries(await Promise.all(Object.entries(paths).map(async([key,file])=>[key,JSON.parse(await readFile(file))])));
const failures=[];
const check=(condition,detail)=>{if(!condition)failures.push(detail);};
const canonical=(side,bytes)=>{
  const m=side==='source'?decodeIndependent(bytes).meshes[0]:decodeRootF32Model(bytes);
  const raw=side==='source'?{
    positions:m.positions.map(row=>row.map(w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();})),
    colors:m.colors.map(row=>row.map(w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();})),
    materials:m.materials.map(row=>row.map(w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();})),faces:m.faces}:m;
  const ids=raw.colors.map(c=>Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16));
  const positions=Array(ids.length),attributes=Array(ids.length),faces=[];
  ids.forEach((id,i)=>{
    if(positions[id])throw Error(`Duplicate RGB vertex ID ${id}`);
    positions[id]=raw.positions[i];
    attributes[id]=[...raw.colors[i],...raw.materials[i].slice(0,2),
      side==='source'?raw.materials[i][2]:raw.unprotected[i]];
  });
  if(positions.some(x=>!x))throw Error('Missing RGB vertex ID');
  for(const face of raw.faces){
    const row=face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]);
    const k=row.indexOf(Math.min(...row));faces.push([...row.slice(k),...row.slice(0,k)].join(','));
  }
  faces.sort();
  return {positions,attributes,faces,ids};
};
const mesh={},files={};
for(const [key,run] of Object.entries(runs))for(const stage of ['before','after','undo','redo']){
  const file=run[stage]?.path;
  if(!file)throw Error(`${key} ${stage} public Save missing`);
  const bytes=await readFile(file);
  if(sha(bytes)!==run[stage].sha256)throw Error(`${key} ${stage} public Save hash changed`);
  files[`${key}_${stage}`]=bytes;
  mesh[`${key}_${stage}`]=canonical(run.side,bytes);
}
const cmp=(a,b)=>{
  if(a.length!==b.length)throw Error('Mapped vertex count mismatch');
  const changed_ids=[];let components=0,max=0,sum=0;
  for(let id=0;id<a.length;id++){
    if(!a[id]||!b[id])throw Error(`Mapped row ${id} missing`);
    let changed=false;
    for(let c=0;c<3;c++){
      const d=a[id][c]-b[id][c];
      if(!Number.isFinite(d))throw Error(`Non-finite mapped row ${id}`);
      if(Math.fround(a[id][c])!==Math.fround(b[id][c])){changed=true;components++;}
      max=Math.max(max,Math.abs(d));sum+=d*d;
    }
    if(changed)changed_ids.push(id);
  }
  return {changed_ids,changed_vertices:changed_ids.length,different_components:components,
    max_abs_component:max,rms_component:Math.sqrt(sum/(a.length*3))};
};
const exactRows=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const normalMap=(run,stage)=>{
  const observed=run[`${stage}_gpu`];
  const bytes=Buffer.from(observed?.normal_words_base64??'', 'base64');
  const ids=mesh[`${run.side}_${run.clay?'on':'off'}_before`].ids;
  if(bytes.length!==ids.length*12||sha(bytes)!==observed.normal_words_sha256)
    throw Error(`${run.side} ${stage} GPU normal words missing or invalid`);
  const mapped=Array(ids.length);
  ids.forEach((id,i)=>{if(mapped[id])throw Error('Duplicate GPU normal ID');mapped[id]=[0,1,2].map(c=>bytes.readFloatLE(i*12+c*4));});
  return mapped;
};
const gpu={};
for(const [key,run] of Object.entries(runs))for(const stage of ['before','after','undo','redo']){
  const observed=run[`${stage}_gpu`];
  const m=mesh[`${key}_${stage}`];
  const packed=Buffer.alloc(m.positions.length*12);
  m.ids.forEach((id,i)=>m.positions[id].forEach((v,c)=>packed.writeFloatLE(v,i*12+c*4)));
  const observedPositions=Buffer.from(observed?.position_words_base64??'', 'base64');
  const identity=run.side==='root'?(run[`${stage}_object_identity`]??run.before_object_identity):null;
  gpu[`${key}_${stage}`]={unique:observed?.active_vbo_candidates===1&&observed?.gl_error===0,
    position_exact:observed?.position_words_sha256===sha(packed)&&observedPositions.length===packed.length&&sha(observedPositions)===sha(packed),
    identity:run.side==='source'||(identity?.active_subtool_id===1&&identity?.vertices===run.fixture.vertices&&
      JSON.stringify(identity.selected_subtool_ids)==='[1]'&&identity.subtools?.length===1&&
      identity.subtools[0].visible===true&&Number.isInteger(observed?.buffer_token)&&observed.buffer_token>0),
    position_sha256:observed?.position_words_sha256,normal_sha256:observed?.normal_words_sha256};
  check(Object.values(gpu[`${key}_${stage}`]).slice(0,3).every(Boolean),`${key} ${stage}: active GPU identity/position gate`);
}
const paired={},changes={},effects={},history={},input={},preview={};
for(const state of ['on','off']){
  const s=runs[`source_${state}`],r=runs[`root_${state}`];
  check(s.side==='source'&&r.side==='root'&&s.clay===(state==='on')&&r.clay===(state==='on'),`${state}: side/Clay identity`);
  check(s.protocol_commit==='c278-clay-integer-viewport808'&&r.protocol_commit===s.protocol_commit&&
    s.harness_sha256===r.harness_sha256&&s.fixture.sha256===r.fixture.sha256&&
    JSON.stringify(s.requested_brush)===JSON.stringify(r.requested_brush),`${state}: frozen protocol/build input identity`);
  const aligned=r.ray_alignment;
  const sw=s.setup.viewport.screen[2]-s.setup.viewport.screen[0],rw=r.setup.viewport.screen[2]-r.setup.viewport.screen[0];
  const wrs=2*s.config.state.tool._radius/(sw*s.setup.state.camera.projection[0]);
  const wrr=2*r.config.state.source_geometry.per_tool[0].radius/(rw*r.setup.state.camera.projection[0]);
  input[state]={projection_x:aligned?.offset_scale,projection_y:aligned?.offset_scale_y,
    world_radius_difference:Math.abs(wrs-wrr)};
  check(Math.abs(aligned?.offset_scale-1)<=1e-6&&Math.abs(aligned?.offset_scale_y-1)<=1e-6&&
    input[state].world_radius_difference<=1e-6,`${state}: projection/radius`);
  for(const side of ['source','root']){
    const run=runs[`${side}_${state}`],viewport=run.setup.viewport.screen;
    const center=[(viewport[0]+viewport[2])/2,(viewport[1]+viewport[3])/2];
    const expected=[80,92,80,92,80,92,80].map(x=>[center[0]+x,center[1]]);
    const delivered=run.dom_stroke_events?.filter(e=>e.type==='pointermove'&&e.buttons===1).map(e=>[e.x,e.y]);
    const stateTool=side==='source'?run.config.state.tool:run.config.state.source_geometry.per_tool[0];
    const downs=run.dom_stroke_events?.filter(e=>e.type==='pointerdown'),
      ups=run.dom_stroke_events?.filter(e=>e.type==='pointerup');
    check(JSON.stringify(run.setup.viewport.logical_size)==='[1024,808]'&&center.every(Number.isInteger)&&
      JSON.stringify(run.requested_path)===JSON.stringify(expected)&&
      JSON.stringify(delivered)===JSON.stringify(expected.slice(1))&&
      downs?.length===1&&ups?.length===1&&
      downs[0].x===expected[0][0]&&downs[0].y===expected[0][1]&&
      ups[0].x===expected.at(-1)[0]&&ups[0].y===expected.at(-1)[1]&&
      run.dom_stroke_events?.every(e=>e.trusted),`${side}_${state}: trusted integer input`);
    check(side==='source'?stateTool._clay===(state==='on')&&stateTool._accumulate===true&&stateTool._lockPosition===false:
      stateTool.options.tool.Brush.clay===(state==='on')&&stateTool.options.tool.Brush.accumulate===true&&run.config.state.lock_position===false,
      `${side}_${state}: published tool controls`);
    history[`${side}_${state}`]={before_undo:files[`${side}_${state}_before`].equals(files[`${side}_${state}_undo`]),
      after_redo:files[`${side}_${state}_after`].equals(files[`${side}_${state}_redo`]),
      one_sculpt_step:run.after_history?.undo===run.before_history?.undo+1};
    check(Object.values(history[`${side}_${state}`]).every(Boolean),`${side}_${state}: public history`);
    check(run.held_previews?.length===6,`${side}_${state}: six held previews`);
    preview[`${side}_${state}`]={first_to_final_gpu_distinct:
      run.held_previews?.[0]?.gpu?.position_words_sha256!==run.held_previews?.[5]?.gpu?.position_words_sha256,
      quiet:run.quiet_gpu?.position_words_sha256===run.after_gpu?.position_words_sha256&&
        run.quiet_gpu?.normal_words_sha256===run.after_gpu?.normal_words_sha256};
    check(preview[`${side}_${state}`].first_to_final_gpu_distinct&&preview[`${side}_${state}`].quiet,
      `${side}_${state}: repeated response/quiet release`);
    const faces0=mesh[`${side}_${state}_before`].faces;
    for(const stage of ['after','undo','redo'])check(exactRows(faces0,mesh[`${side}_${state}_${stage}`].faces),
      `${side}_${state} ${stage}: face stability`);
    for(const stage of ['after'])check(exactRows(mesh[`${side}_${state}_before`].attributes,
      mesh[`${side}_${state}_${stage}`].attributes),`${side}_${state} ${stage}: attribute stability`);
  }
  for(const stage of ['before','after','undo','redo']){
    paired[`${state}_${stage}`]=cmp(mesh[`source_${state}_${stage}`].positions,mesh[`root_${state}_${stage}`].positions);
    check(paired[`${state}_${stage}`].max_abs_component<=1e-6,`${state} ${stage}: paired XYZ >1e-6`);
    if(stage==='before')check(paired[`${state}_${stage}`].different_components===0,
      `${state}: initial mapped positions are not exact`);
    check(exactRows(mesh[`source_${state}_${stage}`].attributes,mesh[`root_${state}_${stage}`].attributes)&&
      exactRows(mesh[`source_${state}_${stage}`].faces,mesh[`root_${state}_${stage}`].faces),`${state} ${stage}: paired attributes/faces`);
    const normals=cmp(normalMap(s,stage),normalMap(r,stage));
    paired[`${state}_${stage}_gpu_normals`]=normals;
    check(normals.max_abs_component<=1e-6,`${state} ${stage}: paired GPU display normals >1e-6`);
  }
  for(const stage of ['after']){
    const source=cmp(mesh[`source_${state}_before`].positions,mesh[`source_${state}_${stage}`].positions);
    const root=cmp(mesh[`root_${state}_before`].positions,mesh[`root_${state}_${stage}`].positions);
    changes[`${state}_${stage}`]={source,root};
    check(source.changed_vertices>0&&exactRows(source.changed_ids,root.changed_ids),`${state} ${stage}: changed RGB ID membership`);
    for(const side of ['source','root']){
      const before=mesh[`${side}_${state}_before`].positions,after=mesh[`${side}_${state}_${stage}`].positions;
      const changed=new Set(changes[`${state}_${stage}`][side].changed_ids);
      check(before.every((row,id)=>changed.has(id)||exactRows(row,after[id])),
        `${side}_${state} ${stage}: untouched vertex changed`);
    }
  }
  for(let held=0;held<6;held++){
    const streams={};
    for(const side of ['source','root']){
      const run=runs[`${side}_${state}`],observed=run.held_previews?.[held]?.gpu;
      check(observed?.active_vbo_candidates===1&&observed?.gl_error===0&&
        (side==='source'||(Number.isInteger(observed?.buffer_token)&&observed.buffer_token>0&&
          run.held_previews[held].live?.state?.active_subtool_id===1)),
        `${side}_${state} held ${held}: active GPU/object`);
      const bytes=Buffer.from(observed?.position_words_base64??'', 'base64');
      const normals=Buffer.from(observed?.normal_words_base64??'', 'base64');
      const ids=mesh[`${side}_${state}_before`].ids;
      if(bytes.length!==ids.length*12||normals.length!==ids.length*12||
        sha(bytes)!==observed?.position_words_sha256||sha(normals)!==observed?.normal_words_sha256)
        throw Error(`${side}_${state} held ${held}: GPU words missing or invalid`);
      const positions=Array(ids.length),normalRows=Array(ids.length);
      ids.forEach((id,i)=>{positions[id]=[0,1,2].map(c=>bytes.readFloatLE(i*12+c*4));
        normalRows[id]=[0,1,2].map(c=>normals.readFloatLE(i*12+c*4));});
      streams[side]={positions,normals:normalRows};
    }
    const position=cmp(streams.source.positions,streams.root.positions),normals=cmp(streams.source.normals,streams.root.normals);
    paired[`${state}_held_${held}`]={position,normals};
    check(position.max_abs_component<=1e-6&&normals.max_abs_component<=1e-6,`${state} held ${held}: paired active GPU words >1e-6`);
  }
}
for(const side of ['source','root']){
  check(exactRows(mesh[`${side}_on_before`].positions,mesh[`${side}_off_before`].positions),`${side}: on/off initial geometry`);
  const on=mesh[`${side}_on_after`].positions,off=mesh[`${side}_off_after`].positions;
  effects[side]=cmp(on,off);
  check(effects[side].changed_vertices>0,`${side}: Clay on/off effect absent`);
}
check(exactRows(effects.source.changed_ids,effects.root.changed_ids),'Clay on/off effect RGB membership');
const sourceEffect=mesh.source_on_after.positions.map((row,id)=>row.map((v,c)=>v-mesh.source_off_after.positions[id][c]));
const rootEffect=mesh.root_on_after.positions.map((row,id)=>row.map((v,c)=>v-mesh.root_off_after.positions[id][c]));
effects.paired=cmp(sourceEffect,rootEffect);
check(effects.paired.max_abs_component<=1e-6,'Clay on/off paired effect >1e-6');
const audit={status:failures.length?'FAILED frozen C278 gate; evidence retained':'COMPLETE scoped C278 Clay pair',
  failures,paths,reference_build_sha256:runs.source_on.browser_build,obrush_wasm_sha256:runs.root_on.browser_build,
  fixture_sha256:runs.source_on.fixture.sha256,harness_sha256:runs.source_on.harness_sha256,
  input,paired,changes,effects,history,preview,gpu,
  limitations:['One 10k static sphere/front orthographic integer trace; not the full Clay row',
    'Source WebGL1 upload shadow versus OBrush WebGL2 readback',
    'Public saved normals are not used as the active display-normal oracle']};
const out=path.join(path.dirname(sourceOnPath),'paired-audit-'+path.basename(path.dirname(rootOnPath))+'-with-off.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,failures,paired_after_on:paired.on_after,paired_after_off:paired.off_after,effects}));
if(failures.length)process.exitCode=1;
