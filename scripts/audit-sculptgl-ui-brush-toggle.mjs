// Compare a normal-UI Brush option's effect within each app, then across apps.
// Inputs are public normal-browser result receipts: source-off, source-on,
// root-off, root-on. No application runtime state is used as a geometry oracle.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { decodeIndependent } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs';
import { decodeRootF32Model } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-model.mjs';

const paths=process.argv.slice(2);
if(paths.length!==4)throw Error('Pass source-off source-on root-off root-on result.json');
const sha=b=>createHash('sha256').update(b).digest('hex');
const f32=w=>{const b=Buffer.allocUnsafe(4);b.writeUInt32LE(w);return b.readFloatLE();};
const runs=await Promise.all(paths.map(async p=>JSON.parse(await readFile(p))));
if(runs.some((r,i)=>r.side!==(i<2?'source':'root') || r.stroke_mode!==runs[0].stroke_mode ||
  r.fixture.sha256!==runs[0].fixture.sha256 || r.browser_build!==runs[i<2?0:2].browser_build))
  throw Error('Run side, stroke, fixture or per-app build mismatch');
if(runs.some((r,i)=>r.config.state[i<2?'tool':'source_geometry'][i<2?'_accumulate':'per_tool']===undefined))
  throw Error('Accumulate state missing');
const modes=runs.map((r,i)=>i<2?r.config.state.tool._accumulate:r.config.state.source_geometry.per_tool[0].options.tool.Brush.accumulate);
if(JSON.stringify(modes)!==JSON.stringify([false,true,false,true]))throw Error('Expected off/on/off/on published modes');
async function mapped(i,record){
  const bytes=await readFile(record.path);
  if(sha(bytes)!==record.sha256)throw Error('Public save hash changed: '+paths[i]);
  const data=i<2?(()=>{const m=decodeIndependent(bytes).meshes[0];return {positions:m.positions.map(row=>row.map(f32)),colors:m.colors.map(row=>row.map(f32))};})():decodeRootF32Model(bytes);
  const rows=Array(data.positions.length),seen=new Set();
  for(let j=0;j<data.positions.length;j++){
    const c=data.colors[j],id=Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16);
    if(id<0||id>=rows.length||seen.has(id))throw Error('Invalid mapped RGB vertex ID: '+id);
    seen.add(id);rows[id]=data.positions[j];
  }
  if(seen.size!==rows.length)throw Error('Missing mapped RGB vertex ID');
  return rows;
}
const positions=await Promise.all(runs.map((r,i)=>mapped(i,r.after)));
const before=await Promise.all(runs.map((r,i)=>mapped(i,r.before)));
const n=positions[0].length;
if(positions.some(p=>p.length!==n))throw Error('Mapped vertex counts differ');
const sourceIds=[],rootIds=[],deltaIds=[];let max=0,sumsq=0,components=0;
let sourceMax=0,rootMax=0,initialDifferent=0;
for(let i=0;i<n;i++){
  for(let c=0;c<3;c++)if(before.some(p=>p[i][c]!==before[0][i][c]))initialDifferent++;
  const source=positions[1][i].map((v,c)=>v-positions[0][i][c]);
  const root=positions[3][i].map((v,c)=>v-positions[2][i][c]);
  for(const v of source)sourceMax=Math.max(sourceMax,Math.abs(v));
  for(const v of root)rootMax=Math.max(rootMax,Math.abs(v));
  if(source.some(v=>v!==0))sourceIds.push(i);
  if(root.some(v=>v!==0))rootIds.push(i);
  let different=false;
  for(let c=0;c<3;c++){
    const d=source[c]-root[c];if(d!==0){different=true;components++;}
    max=Math.max(max,Math.abs(d));sumsq+=d*d;
  }
  if(different)deltaIds.push(i);
}
const delivered=runs.map(r=>r.dom_stroke_events?.map(e=>({type:e.type,x:e.x,y:e.y,buttons:e.buttons,trusted:e.trusted})));
const sameEventsWithinApp=JSON.stringify(delivered[0])===JSON.stringify(delivered[1])&&JSON.stringify(delivered[2])===JSON.stringify(delivered[3]);
const audit={runs:paths,fixture_sha256:runs[0].fixture.sha256,mode_sequence:modes,vertices:n,
  initial_different_components:initialDifferent,
  source_toggle_changed_ids:sourceIds,root_toggle_changed_ids:rootIds,
  source_max_abs_toggle_effect:sourceMax,root_max_abs_toggle_effect:rootMax,
  toggle_changed_membership_equal:JSON.stringify(sourceIds)===JSON.stringify(rootIds),
  cross_app_toggle_delta:{different_vertex_ids:deltaIds,components,max_abs_component:max,rms_component:Math.sqrt(sumsq/(3*n))},
  same_delivered_dom_event_pattern_within_app:sameEventsWithinApp,
  status:initialDifferent===0&&sourceIds.length>0&&rootIds.length>0&&sameEventsWithinApp?'COMPLETE useful paired toggle-effect comparison':'FAILED toggle-effect admission; evidence retained'};
const out=path.join(path.dirname(paths[1]),'accumulate-toggle-effect-audit.json');
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,source_changed:sourceIds.length,root_changed:rootIds.length,membership_equal:audit.toggle_changed_membership_equal,difference_vertices:deltaIds.length,max_abs_component:max}));
if(audit.status.startsWith('FAILED'))process.exitCode=1;
