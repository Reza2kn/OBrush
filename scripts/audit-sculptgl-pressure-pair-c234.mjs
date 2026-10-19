// Audit two C234 runs by original RGB vertex identity and all public save bytes.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { decodeIndependent } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs';
import { decodeRootF32Model } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs';

const [sourcePath,rootPath]=process.argv.slice(2);
if(!sourcePath||!rootPath)throw Error('Pass source and root result.json');
const source=JSON.parse(await readFile(sourcePath)),root=JSON.parse(await readFile(rootPath));
if(source.side!=='source'||root.side!=='root'||source.fixture.sha256!==root.fixture.sha256)throw Error('Different fixture or sides');
const sha=b=>createHash('sha256').update(b).digest('hex');
const f32=word=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(word);return b.readFloatLE()};
const decode=(side,b)=>side==='root'?decodeRootF32Model(b):(()=>{const m=decodeIndependent(b).meshes[0];return {positions:m.positions.map(row=>row.map(f32)),colors:m.colors.map(row=>row.map(f32)),faces:m.faces}})();
const indexed=m=>{
  const result=new Map();
  m.colors.forEach((c,i)=>{const id=Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16);if(result.has(id))throw Error('Duplicate fixture RGB vertex ID '+id);result.set(id,m.positions[i]);});
  return result;
};
const diff=(a,b)=>{
  if(a.size!==b.size)throw Error('Vertex count mismatch');
  let changed=0,components=0,max=0,sum=0;
  for(const [id,x] of a){const y=b.get(id);if(!y)throw Error('Missing RGB vertex ID '+id);let any=false;for(let i=0;i<3;i++){const d=x[i]-y[i];any ||= Math.fround(x[i])!==Math.fround(y[i]);components+=Number(Math.fround(x[i])!==Math.fround(y[i]));max=Math.max(max,Math.abs(d));sum+=d*d;}changed+=Number(any);}
  return {vertices:a.size,changed_vertices:changed,changed_components:components,max_abs_component:max,rms_component:Math.sqrt(sum/(a.size*3))};
};
const records={};
for(const [side,run] of [['source',source],['root',root]]){
  records[side]={};
  for(const c of run.cases){
    const files={};
    for(const stage of ['before','after','undo','redo']){
      const b=await readFile(c[stage].path);if(sha(b)!==c[stage].sha256)throw Error(`${side} ${c.name} ${stage} hash mismatch`);
      files[stage]=b;
    }
    const before=indexed(decode(side,files.before)),after=indexed(decode(side,files.after));
    records[side][c.name]={requested:{pointerType:c.pointerType,pressure:c.pressure,radius:c.radius,intensity:c.intensity,path:c.requested_path},
      delivered:{events:c.dom_events.map(e=>({type:e.type,pointerType:e.pointerType??null,pressure:e.pressure??null,buttons:e.buttons,trusted:e.trusted})),
        trusted_declared_pointerdown:c.dom_events.some(e=>e.type==='pointerdown'&&e.trusted&&e.pointerType===c.pointerType&&Math.abs(e.pressure-c.pressure)<=1e-6),
        mousemove_count:c.dom_events.filter(e=>e.type==='mousemove').length,
        pointermove_held_count:c.dom_events.filter(e=>e.type==='pointermove'&&e.buttons===1).length},
      save_sha256:Object.fromEntries(['before','after','undo','redo'].map(s=>[s,c[s].sha256])),
      before_equals_undo:files.before.equals(files.undo),after_equals_redo:files.after.equals(files.redo),
      shape_change:diff(before,after)};
    records[side][c.name]._before=before;records[side][c.name]._after=after;
  }
}
const paired={};
for(const name of Object.keys(records.source)){
  paired[name]={before:diff(records.source[name]._before,records.root[name]._before),after:diff(records.source[name]._after,records.root[name]._after)};
  delete records.source[name]._before;delete records.source[name]._after;
  delete records.root[name]._before;delete records.root[name]._after;
}
const audit={source_result:sourcePath,root_result:rootPath,fixture_sha256:source.fixture.sha256,
  source_build:source.browser_build,root_build:root.browser_build,records,paired,
  admission:{same_fixture:true,all_trusted_down:Object.values(records).every(side=>Object.values(side).every(c=>c.delivered.trusted_declared_pointerdown)),
    all_root_history_exact:Object.values(records.root).every(c=>c.before_equals_undo&&c.after_equals_redo),
    all_source_redo_exact:Object.values(records.source).every(c=>c.after_equals_redo)},
  interpretation:'The pinned browser received trusted pointermove and mousemove for every synthetic pen case, but the public save remained geometrically unchanged even at full pressure with neutral factors. The cause is unresolved; these no-op outcomes cannot establish pressure geometry parity.',
  limitations:['Synthetic CDP pen input is not a physical tablet.','The pinned SculptGL pen no-op requires further event/gesture diagnosis before a paired pressure-shape claim.','OBrush case uses Source Brush geometry mode; C231 ordinary Draw factor response is separate.','Different brush algorithms, camera rays and dabs prevent equal raw mesh expectation even when pressure multipliers agree.']};
audit.status=audit.admission.all_trusted_down&&audit.admission.all_root_history_exact&&audit.admission.all_source_redo_exact?'complete paired input-path comparison; pressure geometry parity unresolved':'FAILED audit admission; preserve evidence';
const out=path.join(path.dirname(sourcePath),'audit-'+path.basename(path.dirname(rootPath))+'.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,source:Object.fromEntries(Object.entries(records.source).map(([k,v])=>[k,v.shape_change])),root:Object.fromEntries(Object.entries(records.root).map(([k,v])=>[k,v.shape_change])),paired_before:paired['neutral-quarter'].before}));
if(audit.status.startsWith('FAILED'))process.exitCode=1;
