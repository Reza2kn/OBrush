// Offline C331 scorer. This was authored after capture; the gates were preregistered.
import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [sourcePath, rootPath, outPath] = process.argv.slice(2);
if (!sourcePath || !rootPath || !outPath) throw Error('Usage: node audit-sculptgl-source-pinch-c331.mjs source.json root.json out.json');
const source=JSON.parse(await readFile(sourcePath)), root=JSON.parse(await readFile(rootPath));
const sha=b=>createHash('sha256').update(b).digest('hex');
const project=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??project);
const primary=path.join(shared,'.tools/sculptgl-reference/gl-matrix-3.3.0-primary/src');
const mirror=path.join(project,'.artifacts/sculptgl-source-pinch-c331-ui/gl-matrix-3.3.0-primary-esm');
const pinned={
  'common.js':'15a5ffd69ef73c1fb428442897a5f2fef3030b65bd6af40d8eccd9a6acc89e2c',
  'mat4.js':'43e5a8ec18298628f1b15e20e3d6990aa774a35912186544a60608c048f05e4b',
  'vec3.js':'c617f49fb3282053d24b861372b3fa239cf88d2909505b0e5bc68a8f834ea158',
};
await mkdir(mirror,{recursive:true});
for(const [name,expected] of Object.entries(pinned)){
  const input=await readFile(path.join(primary,name));
  if(sha(input)!==expected)throw Error(`Pinned primary ${name} changed`);
  await copyFile(path.join(primary,name),path.join(mirror,name));
}
await writeFile(path.join(mirror,'package.json'),'{"type":"module"}\n');
const mat4=await import(pathToFileURL(path.join(mirror,'mat4.js')).href);
const vec3=await import(pathToFileURL(path.join(mirror,'vec3.js')).href);
const f32=x=>Math.fround(x);
const norm=v=>Math.hypot(...v);
const sub=(a,b)=>a.map((v,i)=>v-b[i]);
const srcCamera=source.camera_matrix, rootCamera=root.camera_matrix;
const expectedFixture='12fc61da6ad54c03c456107a3edec82511fe6352b7da366eec9dc5a5f0c670bc';
if(source.side!=='source'||root.side!=='root'||source.fixture.sha256!==expectedFixture||root.fixture.sha256!==expectedFixture||
  source.probes?.length!==6||root.probes?.length!==6)throw Error('C331 fixture/host/probe mismatch');
const world=mat4.create();
mat4.mul(world,srcCamera.viewport,srcCamera.projection);
mat4.mul(world,world,srcCamera.view);
const worldWords=Array.from(world).map((x,i)=>({index:i,actual:x,recorded:srcCamera.world_to_screen[i],exact:Object.is(x,srcCamera.world_to_screen[i])}));
const inverse=mat4.create();
mat4.invert(inverse,world);
if(!inverse.every(Number.isFinite))throw Error('Reference camera inverse is singular');
const issues=[];
if(worldWords.some(row=>!row.exact))issues.push('Pinned Float32 world-to-screen replay differs from captured camera matrix');
const cameraAxisAligned=[srcCamera.view[0],srcCamera.view[5],srcCamera.view[10],srcCamera.view[15]].every(x=>x===1)&&
  [1,2,3,4,6,7,8,9,11,12,13].every(i=>srcCamera.view[i]===0)&&
  srcCamera.center[0]===0&&srcCamera.center[1]===0&&
  rootCamera.target64[0]===0&&rootCamera.target64[1]===0&&
  source.setup.viewport.canvas.join(',')==='1024,768'&&root.setup.viewport.logical_size.join(',')==='1024,768';
if(!cameraAxisAligned)issues.push('Frozen axis-aligned 1024x768 DPR1 camera setup failed');
const sourceRect=source.setup.viewport.rect,rootRect=root.setup.viewport.screen;
const sourceScale=srcCamera.world_to_screen[5];
const formulaY=384*f32(1/sourceScale)+f32(-384/sourceScale);
const replayCenter=vec3.transformMat4([0,0,0],[512,384,0],inverse)[1];
const centerAgreement=Math.abs(formulaY-replayCenter);
if(centerAgreement>1e-12)issues.push('Central analytic Float32 formula differs from full gl-matrix inverse replay');
const rows=source.probes.map((p,i)=>{
  const q=root.probes[i];
  if(p.index!==i||q.index!==i||JSON.stringify(p.offset)!==JSON.stringify(q.offset))issues.push(`Probe ${i} offset mismatch`);
  const srcX=p.point[0]-sourceRect[0],srcY=768-(p.point[1]-sourceRect[1]);
  const near=vec3.transformMat4([0,0,0],[srcX,srcY,0],inverse);
  const far=vec3.transformMat4([0,0,0],[srcX,srcY,0.1],inverse);
  const direction=sub(far,near);const t=-near[2]/direction[2];
  const sourcePredicted=near.map((v,j)=>v+t*direction[j]);
  const sourceObserved=p.observed_pick?.point??null;
  const sourceResidual=sourceObserved?sub(sourceObserved,sourcePredicted):null;
  const rootLocalY=f32(q.point[1]-rootRect[1]);
  const rootLocalX=f32(q.point[0]-rootRect[0]);
  const ndcX=f32(f32(f32(2*rootLocalX)/f32(1024))-1);
  const ndcY=f32(1-f32(f32(2*rootLocalY)/f32(768)));
  const rootPredicted=[ndcX/rootCamera.projection64[0],ndcY/rootCamera.projection64[5],0];
  const ray=q.published_camera_ray;
  // Orthographic camera points straight toward -Z, so its plane hit is the ray's X/Y.
  const rootRayPlane=ray ? [ray['0'],ray['1'],0] : null;
  const rootRayResidual=rootRayPlane?sub(rootRayPlane,rootPredicted):null;
  const rootObserved=q.observed_pick?.center??q.observed_pick?.point??null;
  const rootObservedResidual=rootObserved?sub(rootObserved,rootPredicted):null;
  const predictedPair=sub(sourcePredicted,rootPredicted);
  const sourcePass=sourceResidual!==null&&norm(sourceResidual)<=1e-8;
  const rootObservedPass=rootObservedResidual!==null&&norm(rootObservedResidual)<=1e-8;
  const rayPass=norm(predictedPair)<=1e-8;
  if(!sourcePass)issues.push(`Probe ${i} Source observed/model residual failed`);
  if(!rootObservedPass)issues.push(`Probe ${i} OBrush observed/model residual missing or failed`);
  if(!rayPass)issues.push(`Probe ${i} predicted physical ray difference exceeded 1e-8`);
  if(!ray||Math.abs(ray['3'])>1e-12||Math.abs(ray['4'])>1e-12||Math.abs(ray['5']+1)>1e-12||
      !rootRayResidual||norm(rootRayResidual)>1e-8)issues.push(`Probe ${i} OBrush published camera ray/model failed`);
  if(source.before.sha256!==p.after.sha256||root.before.sha256!==q.after.sha256)issues.push(`Probe ${i} public mesh changed on press-only`);
  if(source.before_gpu.position_words_sha256!==p.gpu.position_words_sha256||
      root.before_gpu.position_words_sha256!==q.gpu.position_words_sha256)issues.push(`Probe ${i} GPU position words changed on press-only`);
  return {index:i,offset:p.offset,source_observed:sourceObserved,source_predicted:sourcePredicted,source_residual:sourceResidual,
    root_observed:rootObserved,root_predicted:rootPredicted,root_published_ray_plane:rootRayPlane,
    root_published_ray_residual:rootRayResidual,root_observed_residual:rootObservedResidual,
    predicted_pair_difference:predictedPair,predicted_pair_distance:norm(predictedPair),
    source_pass:sourcePass,root_observed_pass:rootObservedPass,predicted_ray_pass:rayPass};
});
if(source.before.sha256!==source.after.sha256||root.before.sha256!==root.after.sha256)issues.push('Final public mesh save differs from before');
if(JSON.stringify(source.before_history)!==JSON.stringify(source.after_history)||
  JSON.stringify(root.before_history)!==JSON.stringify(root.after_history))issues.push('History changed on press-only');
if(source.contact_admissions.some(x=>!x.pass)||root.contact_admissions.some(x=>!x.pass))issues.push('Completed zero-contact admission failed');
if(source.protocol_failures.length||root.protocol_failures.length)issues.push('Capture protocol reported failures');
const audit={protocol:'C331 independent picker calibration',source_result:sourcePath,root_result:rootPath,
  source_sha256:sha(await readFile(sourcePath)),root_sha256:sha(await readFile(rootPath)),
  gl_matrix_primary_sha256:pinned,world_to_screen_replay:worldWords,axis_aligned_camera:cameraAxisAligned,
  central_formula_y:formulaY,central_full_replay_y:replayCenter,central_agreement:centerAgreement,
  rows,issues,pass:issues.length===0,scope:'Calibration only; C329 failed gate unchanged; OBrush press-only picked point unavailable'};
await writeFile(outPath,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out:outPath,pass:audit.pass,issues:audit.issues,
  max_predicted_pair_distance:Math.max(...rows.map(r=>r.predicted_pair_distance))}));
if(!audit.pass)process.exitCode=1;
