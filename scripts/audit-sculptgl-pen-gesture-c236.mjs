// Independent public-SGL audit of the pinned C236 pen-gesture diagnostic.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { decodeIndependent } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs';

const resultPath=process.argv[2];
if(!resultPath)throw Error('Pass C236 source result.json');
const run=JSON.parse(await readFile(resultPath));
if(run.side!=='source'||run.pinned_source_commit!=='8e45dafc0f7906e5238dfffc1a2df742bd25f29d')throw Error('Not pinned Source result');
const sha=b=>createHash('sha256').update(b).digest('hex');
const f32=w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE()};
const positions=b=>{
  const m=decodeIndependent(b).meshes[0],ids=new Map();
  m.colors.forEach((c,i)=>{const rgb=c.map(f32),id=Math.round(rgb[0]*255)|(Math.round(rgb[1]*255)<<8)|(Math.round(rgb[2]*255)<<16);if(ids.has(id))throw Error('Duplicate RGB ID '+id);ids.set(id,m.positions[i].map(f32));});
  return ids;
};
const rows=[];
for(const c of run.cases){
  const saves={};
  for(const stage of ['before','after','undo','redo']){const b=await readFile(c[stage].path);if(sha(b)!==c[stage].sha256)throw Error('Save hash changed '+c.name+' '+stage);saves[stage]=b;}
  const before=positions(saves.before),after=positions(saves.after);
  if(before.size!==after.size)throw Error('Vertex count changed '+c.name);
  let changed=0,max=0;
  for(const[id,x]of before){const y=after.get(id);if(!y)throw Error('Missing RGB ID '+id);let any=false;for(let axis=0;axis<3;axis++){any ||= Math.fround(x[axis])!==Math.fround(y[axis]);max=Math.max(max,Math.abs(x[axis]-y[axis]));}changed+=Number(any);}
  rows.push({name:c.name,pointerType:c.pointerType,pressure:c.pressure,requested_path:c.requested_path,
    changed_original_ids:changed,max_abs_position_component:max,before_equals_undo:saves.before.equals(saves.undo),after_equals_redo:saves.after.equals(saves.redo),
    trusted_declared_pointerdown:c.dom_events.some(e=>e.type==='pointerdown'&&e.trusted&&e.pointerType===c.pointerType&&Math.abs(e.pressure-c.pressure)<1e-6),
    pointermove_held:c.dom_events.filter(e=>e.type==='pointermove'&&e.buttons===1).length,
    mousemove_held:c.dom_events.filter(e=>e.type==='mousemove'&&e.buttons===1).length,
    hammer_panstart:c.hammer_events.filter(e=>e.type==='panstart').length,
    hammer_panmove:c.hammer_events.filter(e=>e.type==='panmove').length,
    action_after_press:c.state_after_press.action,actions_after_moves:c.states_after_moves.map(s=>s.action)});
}
const audit={source_result:resultPath,fixture_sha256:run.fixture.sha256,source_build:run.browser_build,rows,
  status:rows.every(r=>r.trusted_declared_pointerdown&&r.after_equals_redo)?'complete source-only public-SGL gesture audit':'FAILED gesture admission'};
const out=path.join(path.dirname(resultPath),'gesture-audit.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,rows}));
if(audit.status.startsWith('FAILED'))process.exitCode=1;
