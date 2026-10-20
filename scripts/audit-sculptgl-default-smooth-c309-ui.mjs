// Frozen C309 default Smooth/Relax-only normal-UI four-run audit. GPU normals are the display oracle.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const {decodeIndependent}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs')).href);
const [sourceOnPath,rootOnPath,sourceOffPath,rootOffPath]=process.argv.slice(2);
if(![sourceOnPath,rootOnPath,sourceOffPath,rootOffPath].every(Boolean))
  throw Error('Pass source-on, root-on, source-off, root-off result.json paths');
const paths={source_on:sourceOnPath,root_on:rootOnPath,source_off:sourceOffPath,root_off:rootOffPath};
const sha=b=>createHash('sha256').update(b).digest('hex');
const runs=Object.fromEntries(await Promise.all(Object.entries(paths).map(async([k,p])=>[k,JSON.parse(await readFile(p))])));
const failures=[],check=(ok,message)=>{if(!ok)failures.push(message);};
const f32word=w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();};
function canonical(side,bytes){
  const m=side==='source'?decodeIndependent(bytes).meshes[0]:decodeRootF32Model(bytes);
  const raw=side==='source'?{positions:m.positions.map(r=>r.map(f32word)),
    colors:m.colors.map(r=>r.map(f32word)),materials:m.materials.map(r=>r.map(f32word)),faces:m.faces}:m;
  const ids=raw.colors.map(c=>Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16));
  const positions=Array(ids.length),attributes=Array(ids.length),faces=[];
  ids.forEach((id,i)=>{
    if(positions[id])throw Error(`Duplicate RGB ID ${id}`);
    positions[id]=raw.positions[i];
    attributes[id]=[...raw.colors[i],...raw.materials[i].slice(0,2),
      side==='source'?raw.materials[i][2]:raw.unprotected[i]];
  });
  if(positions.some(x=>!x))throw Error('Missing RGB ID');
  for(const face of raw.faces){
    const row=face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]);
    const k=row.indexOf(Math.min(...row));faces.push([...row.slice(k),...row.slice(0,k)].join(','));
  }
  faces.sort();return {positions,attributes,faces,ids};
}
const mesh={},files={};
for(const [key,run] of Object.entries(runs))for(const stage of ['before','after','undo','redo']){
  const p=run[stage]?.path;if(!p)throw Error(`${key} ${stage} public Save missing`);
  const bytes=await readFile(p);if(sha(bytes)!==run[stage].sha256)throw Error(`${key} ${stage} Save hash changed`);
  files[`${key}_${stage}`]=bytes;mesh[`${key}_${stage}`]=canonical(run.side,bytes);
}
function compare(a,b){
  if(a.length!==b.length)throw Error('Mapped length mismatch');
  let max=0,sum=0,components=0;const changed_ids=[];
  for(let id=0;id<a.length;id++){
    if(!a[id]||!b[id])throw Error(`Missing mapped row ${id}`);
    let changed=false;
    for(let c=0;c<3;c++){
      const delta=a[id][c]-b[id][c];if(!Number.isFinite(delta))throw Error(`Nonfinite row ${id}`);
      max=Math.max(max,Math.abs(delta));sum+=delta*delta;
      if(Math.fround(a[id][c])!==Math.fround(b[id][c])){components++;changed=true;}
    }
    if(changed)changed_ids.push(id);
  }
  return {changed_ids,changed_vertices:changed_ids.length,different_components:components,
    max_abs_component:max,rms_component:Math.sqrt(sum/(a.length*3))};
}
const exact=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
function gpuRows(run,key,stage,field){
  const observed=stage==='held'?run.held_previews?.[0]?.gpu:run[`${stage}_gpu`];
  const bytes=Buffer.from(observed?.[`${field}_words_base64`]??'','base64');
  const ids=mesh[`${key}_before`].ids;
  if(bytes.length!==ids.length*12||sha(bytes)!==observed?.[`${field}_words_sha256`])
    throw Error(`${key} ${stage} ${field} GPU words missing/invalid`);
  const rows=Array(ids.length);
  ids.forEach((id,i)=>{rows[id]=[0,1,2].map(c=>bytes.readFloatLE(i*12+c*4));});
  return rows;
}
const input={},gpu={},paired={},changes={},history={},preview={},effects={};
for(const state of ['on','off']){
  const s=runs[`source_${state}`],r=runs[`root_${state}`];
  check(s.side==='source'&&r.side==='root'&&s.tangent===(state==='on')&&r.tangent===(state==='on'),
    `${state}: side/tangent identity`);
  check(s.protocol_commit==='c309-default-smooth-asymmetric-grid-integer-viewport808'&&
    r.protocol_commit===s.protocol_commit&&s.harness_sha256===r.harness_sha256&&
    s.fixture.sha256===r.fixture.sha256&&exact(s.requested_brush,r.requested_brush),
    `${state}: frozen protocol/input identity`);
  const p=s.setup.state.camera.projection,q=r.setup.state.camera.projection;
  const sw=s.setup.viewport.screen[2]-s.setup.viewport.screen[0],
    sh=s.setup.viewport.screen[3]-s.setup.viewport.screen[1],
    rw=r.setup.viewport.screen[2]-r.setup.viewport.screen[0],
    rh=r.setup.viewport.screen[3]-r.setup.viewport.screen[1];
  const worldRadiusS=2*s.config.state.tool._radius/(sw*p[0]),
    worldRadiusR=2*r.config.state.source_geometry.per_tool[4].radius/(rw*q[0]);
  const rayX=[3,16].map(x=>Math.abs(2*x/(sw*p[0])-2*x/(rw*q[0]))),
    rayY=[-20,-20].map(y=>Math.abs(2*y/(sh*p[5])-2*y/(rh*q[5])));
  input[state]={projection_x:q[0]/p[0]*rw/sw,projection_y:q[5]/p[5]*rh/sh,
    world_radius_difference:Math.abs(worldRadiusS-worldRadiusR),ray_x_differences:rayX,ray_y_differences:rayY,
    source_projection:[p[0],p[5]],root_projection:[q[0],q[5]]};
  check(Math.abs(input[state].projection_x-1)<=5e-6&&Math.abs(input[state].projection_y-1)<=5e-6&&
    input[state].world_radius_difference<=1e-6&&Math.max(...rayX,...rayY)<=1e-6,
    `${state}: projection/world-ray/radius setup`);
  for(const side of ['source','root']){
    const key=`${side}_${state}`,run=runs[key],v=run.setup.viewport.screen,
      center=[(v[0]+v[2])/2,(v[1]+v[3])/2],
      expected=[[center[0]+3,center[1]-20],[center[0]+16,center[1]-20]],
      delivered=run.dom_stroke_events?.filter(e=>e.type==='pointermove'&&e.buttons===1).map(e=>[e.x,e.y]),
      down=run.dom_stroke_events?.filter(e=>e.type==='pointerdown'),
      up=run.dom_stroke_events?.filter(e=>e.type==='pointerup');
    check(exact(run.setup.viewport.logical_size,[1024,808])&&center.every(Number.isInteger)&&
      exact(run.requested_path,expected)&&exact(delivered,[expected[1]])&&
      down?.length===1&&up?.length===1&&down[0].x===expected[0][0]&&down[0].y===expected[0][1]&&
      up[0].x===expected[1][0]&&up[0].y===expected[1][1]&&
      run.dom_stroke_events?.every(e=>e.trusted),`${key}: trusted normal-UI integer stroke`);
    const tool=side==='source'?run.config.state.tool:run.config.state.source_geometry.per_tool[4];
    check(side==='source'?run.config.state.tool_index===3&&tool._tangent===(state==='on')&&
      tool._culling===false&&tool._lockPosition===false&&[0,'None'].includes(tool._idAlpha)&&
      !run.config.state.symmetry&&!run.config.state.continuous:
      run.config.state.kind==='Smooth'&&tool.mode==='SourceOrdinary'&&
      tool.options.tool.Smooth.tangent===(state==='on')&&!tool.options.culling&&
      !tool.options.prevent_folds&&!tool.options.negative&&
      !run.config.state.symmetry_x&&!run.config.state.continuous.requested&&
      !run.config.state.lock_position,`${key}: published Smooth settings`);
    history[key]={before_undo:files[`${key}_before`].equals(files[`${key}_undo`]),
      after_redo:files[`${key}_after`].equals(files[`${key}_redo`]),
      one_step:run.after_history?.undo===run.before_history?.undo+1};
    check(Object.values(history[key]).every(Boolean),`${key}: public history`);
    check(run.held_previews?.length===1,`${key}: one held snapshot`);
    preview[key]={quiet:run.quiet_gpu?.position_words_sha256===run.after_gpu?.position_words_sha256&&
      run.quiet_gpu?.normal_words_sha256===run.after_gpu?.normal_words_sha256};
    check(preview[key].quiet,`${key}: quiet post-release GPU`);
    for(const stage of ['after','undo','redo'])check(exact(mesh[`${key}_before`].faces,mesh[`${key}_${stage}`].faces),
      `${key} ${stage}: topology changed`);
    check(exact(mesh[`${key}_before`].attributes,mesh[`${key}_after`].attributes),
      `${key}: attributes/protection changed`);
    for(const stage of ['before','after','undo','redo']){
      const observed=run[`${stage}_gpu`],m=mesh[`${key}_${stage}`],packed=Buffer.alloc(m.ids.length*12);
      m.ids.forEach((id,i)=>m.positions[id].forEach((x,c)=>packed.writeFloatLE(x,i*12+c*4)));
      const identity=side==='root'?(run[`${stage}_object_identity`]??run.before_object_identity):null;
      gpu[`${key}_${stage}`]={unique:observed?.active_vbo_candidates===1&&observed?.gl_error===0,
        position_exact:observed?.position_words_sha256===sha(packed),
        identity:side==='source'||(identity?.active_subtool_id===1&&identity?.vertices===625&&
          exact(identity.selected_subtool_ids,[1])&&identity.subtools?.length===1&&
          identity.subtools[0].visible&&Number.isInteger(observed?.buffer_token)&&observed.buffer_token>0)};
      check(Object.values(gpu[`${key}_${stage}`]).every(Boolean),`${key} ${stage}: active GPU identity/Save words`);
    }
  }
  for(const stage of ['before','after','undo','redo']){
    paired[`${state}_${stage}`]=compare(mesh[`source_${state}_${stage}`].positions,
      mesh[`root_${state}_${stage}`].positions);
    check(paired[`${state}_${stage}`].max_abs_component<=1e-6,`${state} ${stage}: paired XYZ >1e-6`);
    if(stage==='before')check(paired[`${state}_${stage}`].different_components===0,
      `${state}: initial positions differ`);
    check(exact(mesh[`source_${state}_${stage}`].attributes,mesh[`root_${state}_${stage}`].attributes)&&
      exact(mesh[`source_${state}_${stage}`].faces,mesh[`root_${state}_${stage}`].faces),
      `${state} ${stage}: attributes/faces differ`);
    const normals=compare(gpuRows(s,`source_${state}`,stage,'normal'),
      gpuRows(r,`root_${state}`,stage,'normal'));
    paired[`${state}_${stage}_gpu_normals`]=normals;
    check(normals.max_abs_component<=1e-6,`${state} ${stage}: GPU display normals >1e-6`);
  }
  const heldPositions=compare(gpuRows(s,`source_${state}`,'held','position'),
    gpuRows(r,`root_${state}`,'held','position')),
    heldNormals=compare(gpuRows(s,`source_${state}`,'held','normal'),
      gpuRows(r,`root_${state}`,'held','normal'));
  paired[`${state}_held`]={positions:heldPositions,normals:heldNormals};
  check(heldPositions.max_abs_component<=1e-6&&heldNormals.max_abs_component<=1e-6,
    `${state}: held active GPU words >1e-6`);
  for(const side of ['source','root']){
    const key=`${side}_${state}`,run=runs[key],observed=run.held_previews[0].gpu;
    check(observed.active_vbo_candidates===1&&observed.gl_error===0&&
      (side==='source'||(Number.isInteger(observed.buffer_token)&&observed.buffer_token>0&&
        run.held_previews[0].live?.state?.active_subtool_id===1)),`${key}: held active GPU/object`);
    const before=mesh[`${key}_before`].positions,after=mesh[`${key}_after`].positions,
      delta=compare(before,after);
    changes[key]=delta;
    check(delta.changed_vertices>0&&delta.changed_ids.includes(287),`${key}: Smooth must change center spike`);
    check(exact(before[286],after[286]),`${key}: fully protected ID286 changed`);
    check(before.every((row,id)=>delta.changed_ids.includes(id)||exact(row,after[id])),
      `${key}: supposedly untouched vertex changed`);
    effects[key]={spike:after[287].map((v,c)=>v-before[287][c]),
      protected:after[286].map((v,c)=>v-before[286][c]),
      half_unprotected:after[288].map((v,c)=>v-before[288][c])};
  }
  check(exact(changes[`source_${state}`].changed_ids,changes[`root_${state}`].changed_ids),
    `${state}: changed RGB-ID membership differs`);
}
for(const side of ['source','root']){
  check(exact(mesh[`${side}_on_before`].positions,mesh[`${side}_off_before`].positions),
    `${side}: on/off initial geometry differs`);
  const a=effects[`${side}_on`].spike,b=effects[`${side}_off`].spike;
  const max=Math.max(...a.map((v,c)=>Math.abs(v-b[c])));
  effects[`${side}_toggle_spike_max`]=max;
  check(max>=1e-4,`${side}: tangent toggle lacks frozen center-spike distinction`);
}
const audit={status:failures.length?'FAILED frozen C309 default Smooth gate; evidence retained':
  'COMPLETE scoped C309 default Smooth pair',failures,paths,
  source_build_sha256:runs.source_on.browser_build,obrush_wasm_sha256:runs.root_on.browser_build,
  fixture_sha256:runs.source_on.fixture.sha256,harness_sha256:runs.source_on.harness_sha256,
  input,paired,changes,history,preview,gpu,effects,
  limitations:['One 625-vertex asymmetric curved grid and one normal-UI held move; full Smooth/Tangent row stays open',
    'Source WebGL1 upload shadow versus OBrush WebGL2 readback',
    'Public saved normals are not used as a GPU display-normal oracle']};
const out=path.join(path.dirname(sourceOnPath),'paired-audit-'+path.basename(path.dirname(rootOnPath))+'-with-off.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,failures,paired_after_on:paired.on_after,
  paired_after_off:paired.off_after,effects}));
if(failures.length)process.exitCode=1;
