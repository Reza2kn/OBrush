// Frozen C293 read-only settled-camera preflight scorer; no product mutation.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const stage = path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace');
const { decodeIndependent } = await import(pathToFileURL(path.join(stage, 'independent-fixtures.mjs')).href);
const { decodeRootF32Model } = await import(pathToFileURL(path.join(stage, 'perf-model.mjs')).href);
const [sourcePath, rootPath] = process.argv.slice(2);
if (!sourcePath || !rootPath) throw Error('Pass frozen C293 Source and OBrush result.json paths');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const source = JSON.parse(await readFile(sourcePath));
const obrush = JSON.parse(await readFile(rootPath));
const checks = [];
const check = (condition, name, detail) => checks.push({ name, pass: !!condition, detail });
const finite = values => values.every(Number.isFinite);
const f32 = bits => { const b=Buffer.alloc(4); b.writeUInt32LE(bits); return b.readFloatLE(); };
const idOf = c => Math.round(c[0]*255) | (Math.round(c[1]*255)<<8) | (Math.round(c[2]*255)<<16);
check(source.protocol === 'c293-read-only-settled-camera-preflight' &&
  obrush.protocol === source.protocol && source.side === 'source' && obrush.side === 'root' &&
  source.harness_sha256 === obrush.harness_sha256,
  'frozen paired protocol and harness', [source.harness_sha256, obrush.harness_sha256]);
check(source.fixture_sha256 === 'fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e' &&
  obrush.fixture_sha256 === source.fixture_sha256 &&
  source.reference_sha256 === '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600',
  'independent fixture and pinned reference hashes');
check(source.status.startsWith('captured C293') && obrush.status.startsWith('captured C293') &&
  !source.failure && !obrush.failure && !source.cleanup_errors.length && !obrush.cleanup_errors.length,
  'both owned browser captures and closure', [source.status, obrush.status]);
for (const run of [source, obrush]) {
  for (const label of ['after_reset','after_fit']) {
    const v = run.stability?.[label];
    check(v?.terminal === true && v.frames >= 12 && v.elapsed_ms >= 250 &&
      v.elapsed_ms <= 5000 && v.signature &&
      JSON.stringify(run.side === 'source' ?
        [v.latest.center,v.latest.translation,v.latest.projection,v.latest.near,v.latest.width,v.latest.height] :
        [v.latest.precise_target,v.latest.position64,v.latest.projection,
          v.latest.source_framing?.near,v.latest.source_framing?.projection_zoom,v.latest.viewport]) === v.signature,
      `${run.side} ${label} camera stable`, v && {frames:v.frames,elapsed_ms:v.elapsed_ms});
  }
  check(JSON.stringify(run.snapshots.final.viewport.logical_size) === '[1024,808]',
    `${run.side} normal 1024x808 viewport`);
  check(run.public_before.sha256 === run.public_after_hover.sha256 &&
    JSON.stringify(run.before_history) === JSON.stringify(run.after_hover_history),
    `${run.side} read-only public Save/history`);
  check(run.before_gpu.active_vbo_candidates === 1 && run.after_hover_gpu.active_vbo_candidates === 1 &&
    run.before_gpu.gl_error === 0 && run.after_hover_gpu.gl_error === 0 &&
    run.before_gpu.position_words_sha256 === run.after_hover_gpu.position_words_sha256 &&
    run.before_gpu.normal_words_sha256 === run.after_hover_gpu.normal_words_sha256,
    `${run.side} active GPU unchanged by hover`);
}
const ss = source.snapshots.final.witness, rs = obrush.snapshots.final.state.camera;
const sc = ss?.camera, rc = rs?.source_framing;
check(ss?.mesh_count === 1 && ss.vertex_count === 1089 && sc?.projection_type === 1 &&
  ss.local?.length === 6 && finite(ss.local) && sc.timers_settled &&
  rc?.policy === 'sculptgl_static_v1' && rs?.orthographic === true,
  'active objects, finite bounds and orthographic modes');
const sx = sc?.projection?.[0], sy = sc?.projection?.[5],
  rx = rs?.projection?.[0], ry = rs?.projection?.[5];
const ratioX = rx/sx, ratioY = ry/sy;
check([sx,sy,rx,ry].every(Number.isFinite) && sx>0 && sy>0 && rx>0 && ry>0 &&
  Math.abs(ratioX-1)<=1e-8 && Math.abs(ratioY-1)<=1e-8,
  'frozen settled X/Y projection ratio <=1e-8', {ratioX,ratioY});
const ray = (offset,px,py) => [2*offset[0]/(1024*px),-2*offset[1]/(808*py)];
const offsets=[[-64,-20],[-28,-20],[8,-20],[44,-20]];
const rayDiffs=offsets.map(v=>Math.max(...ray(v,sx,sy).map((x,i)=>Math.abs(x-ray(v,rx,ry)[i]))));
const radiusSource=2*96/(1024*sx),radiusRoot=2*96/(1024*rx);
check(rayDiffs.every(d=>d<=1e-8) && Math.abs(radiusSource-radiusRoot)<=1e-8,
  'frozen world rays and 96px radius <=1e-8', {rayDiffs,radiusSource,radiusRoot});
const trace=source.source_trace||[],fit=trace.find(e=>e.name==='computeFrustumFit'),
  reset=trace.find(e=>e.name==='resetCameraMeshes'),pivot=trace.find(e=>e.name==='setAndFocusOnPivot');
const fitNear=fit?.before?.near,fitX=404/fitNear,fitExpected=Math.sqrt(1+fitX*fitX)/fitX;
check(!!fit && !!reset && !!pivot && Number.isFinite(fitNear) &&
  Math.abs(fitExpected-fit.returned)<=1e-12,
  'Source actual reset branch and pre-fit frustum formula',
  {fitNear,fitExpected,returned:fit?.returned,pivot:pivot?.args});
const sourceModel=decodeIndependent(await readFile(source.public_before.path)).meshes[0];
const rootModel=decodeRootF32Model(await readFile(obrush.public_before.path));
const sPositions=sourceModel.positions.map(v=>v.map(f32)),sColors=sourceModel.colors.map(v=>v.map(f32));
const sById=new Map();
for(let i=0;i<sPositions.length;i++)sById.set(idOf(sColors[i]),sPositions[i]);
let maxBefore=0;
for(let id=0;id<1089;id++)for(let k=0;k<3;k++)maxBefore=Math.max(maxBefore,Math.abs((sById.get(id)||[])[k]-rootModel.positions[id][k]));
function canonical(model,side){
  const positions=side==='source'?sPositions:model.positions;
  const colors=side==='source'?sColors:model.colors;
  const mats=side==='source'?model.materials.map(v=>v.map(f32)):model.materials;
  const ids=colors.map(idOf),attributes=Array(ids.length),faces=[];
  ids.forEach((id,i)=>{attributes[id]=[...colors[i],...mats[i].slice(0,2),
    side==='source'?mats[i][2]:model.unprotected[i]];});
  for(const face of model.faces){const row=face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]);
    const k=row.indexOf(Math.min(...row));faces.push([...row.slice(k),...row.slice(0,k)].join(','));}
  faces.sort();
  const packed=Buffer.alloc(ids.length*12);
  ids.forEach((id,i)=>positions[i].forEach((x,k)=>packed.writeFloatLE(x,i*12+k*4)));
  return {ids,attributes,faces,position_sha256:sha(packed)};
}
const sourceCanon=canonical(sourceModel,'source'),rootCanon=canonical(rootModel,'root');
check(sourceModel.faces.length === 2048 && rootModel.faces.length === 2048 &&
  sById.size === 1089 && rootModel.positions.length === 1089 && maxBefore === 0 &&
  JSON.stringify(sourceCanon.attributes)===JSON.stringify(rootCanon.attributes) &&
  JSON.stringify(sourceCanon.faces)===JSON.stringify(rootCanon.faces),
  'public Before RGB-ID positions/attributes/topology exact', {maxBefore});
check(source.before_gpu.position_words_sha256===sourceCanon.position_sha256 &&
  obrush.before_gpu.position_words_sha256===rootCanon.position_sha256,
  'each active GPU position buffer equals own public Before Save');
function frontPick(positions,faces,x,y){
  let best=null;
  for(const face of faces){const [i,j,k]=face, a=positions[i],b=positions[j],c=positions[k];
    const den=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1]);
    if(!Number.isFinite(den)||Math.abs(den)<1e-20)continue;
    const u=((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/den;
    const v=((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/den;
    const w=1-u-v;
    if(u < -1e-8||v < -1e-8||w < -1e-8)continue;
    const z=u*a[2]+v*b[2]+w*c[2];
    if(!best||z>best[2])best=[x,y,z];
  }
  return best;
}
const pickRows=[];
for(const [i,offset] of [[0,[-64,-20]],[1,[44,-20]]]){
  const sp=source.hover_picks[i]?.pick, sr=ray(offset,sx,sy), rr=ray(offset,rx,ry);
  const predictedSource=frontPick(sPositions,sourceModel.faces,...sr);
  const predictedRoot=frontPick(rootModel.positions,rootModel.faces,...rr);
  const sourceError=predictedSource&&sp?.point?Math.max(...sp.point.map((v,k)=>Math.abs(v-predictedSource[k]))):Infinity;
  const pairedError=predictedRoot&&sp?.point?Math.max(...sp.point.map((v,k)=>Math.abs(v-predictedRoot[k]))):Infinity;
  pickRows.push({offset,sourceActual:sp?.point,predictedSource,predictedRoot,sourceError,pairedError});
}
check(pickRows.every(p=>p.sourceError<=1e-7 && p.pairedError<=1e-7),
  'actual Source and independent paired settled picks <=1e-7', pickRows);
const outcome={protocol:'c293-read-only-settled-camera-preflight-audit',source_result:sourcePath,root_result:rootPath,
  source_result_sha256:sha(await readFile(sourcePath)),root_result_sha256:sha(await readFile(rootPath)),
  audit_sha256:sha(await readFile(new URL(import.meta.url))),checks,pass:checks.every(c=>c.pass),
  scope:'Read-only camera admission only; no Source Drag product/parity pass'};
const outPath=path.join(path.dirname(sourcePath),'paired-audit-'+path.basename(path.dirname(rootPath))+'.json');
await writeFile(outPath,JSON.stringify(outcome,null,2)+'\n');
console.log(JSON.stringify({out:outPath,pass:outcome.pass,failed:checks.filter(c=>!c.pass).map(c=>c.name)}));
if(!outcome.pass)process.exitCode=1;
