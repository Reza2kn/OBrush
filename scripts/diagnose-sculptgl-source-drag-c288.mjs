// Read-only independent replay of C288's published six-sample Source Drag path.
import {readFileSync} from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const local=path.resolve(path.dirname(new URL(import.meta.url).pathname),'..');
const shared=path.resolve(process.env.OBRUSH_SHARED_ROOT??local);
const stage=path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace');
const {decodeIndependent}=await import(pathToFileURL(path.join(stage,'independent-fixtures.mjs')).href);
const {decodeRootF32Model}=await import(pathToFileURL(path.join(stage,'perf-model.mjs')).href);
const [sourcePath,rootPath]=process.argv.slice(2);
if(!sourcePath||!rootPath)throw Error('Pass C288 Source and OBrush result.json paths');
const s=JSON.parse(readFileSync(sourcePath));
const r=JSON.parse(readFileSync(rootPath));
if(s.protocol_commit!=='c288-source-drag-curved-grid-integer-viewport808'||r.protocol_commit!==s.protocol_commit||s.fixture.sha256!==r.fixture.sha256||s.harness_sha256!==r.harness_sha256||s.held_previews.length!==3||r.held_previews.length!==3)throw Error('Frozen C288 paired identity or held captures missing');
const f=w=>{const b=Buffer.alloc(4);b.writeUInt32LE(w);return b.readFloatLE()};
function canonical(side,bytes){const m=side==='source'?decodeIndependent(bytes).meshes[0]:decodeRootF32Model(bytes);const raw=side==='source'?{positions:m.positions.map(v=>v.map(f)),colors:m.colors.map(v=>v.map(f)),materials:m.materials.map(v=>v.map(f))}:m;const ids=raw.colors.map(c=>Math.round(c[0]*255)|(Math.round(c[1]*255)<<8)|(Math.round(c[2]*255)<<16));const pos=[],u=[];ids.forEach((id,i)=>{pos[id]=raw.positions[i];u[id]=side==='source'?raw.materials[i][2]:raw.unprotected[i]});return {pos,u};}
const sb=canonical('source',readFileSync(s.before.path)),sa=canonical('source',readFileSync(s.after.path)),rb=canonical('root',readFileSync(r.before.path)),ra=canonical('root',readFileSync(r.after.path));
const srcC=[s.press_pick.point,...s.held_previews.map(x=>x.pick.point)];const rootC=r.held_previews.map(x=>x.live.state.source_geometry.active.last_pass.center);const sx=(srcC[2][0]-srcC[1][0])/2, rx=(rootC[1][0]-rootC[0][0])/2;
const sourceCenters=[srcC[0],srcC[1],[srcC[1][0]+sx,srcC[1][1],srcC[1][2]],srcC[2],[srcC[2][0]+sx,srcC[2][1],srcC[2][2]],srcC[3]];
const rootCenters=[[rootC[0][0]-rx,rootC[0][1],rootC[0][2]],rootC[0],[rootC[0][0]+rx,rootC[0][1],rootC[0][2]],rootC[1],[rootC[1][0]+rx,rootC[1][1],rootC[1][2]],rootC[2]];
const sr2=s.press_pick.radius_squared,rr2=r.held_previews[0].live.state.source_geometry.active.last_pass.local_radius_squared;
function simulate(before,centers,radius2){let pos=before.pos.map(v=>[...v]);for(let step=1;step<centers.length;step++){const ctr=centers[step],prev=centers[step-1],dir=ctr.map((v,i)=>v-prev[i]);for(let id=0;id<pos.length;id++){const v=pos[id];const dx=v[0]-ctr[0],dy=v[1]-ctr[1],dz=v[2]-ctr[2];const d=Math.sqrt(dx*dx+dy*dy+dz*dz)/Math.sqrt(radius2);if(d>=1)continue;const q=d*d,w=(3*q*q-4*q*d+1)*before.u[id];pos[id]=v.map((x,i)=>dir[i]===0?x:Math.fround(x+dir[i]*w));}}return pos;}
const ps=simulate(sb,sourceCenters,sr2),pr=simulate(rb,rootCenters,rr2),pSame=simulate(sb,rootCenters,rr2);
function residual(a,b,c,d){let max=0,rms=0,n=0;for(let id=0;id<a.length;id++)for(let k=0;k<3;k++){const e=(a[id][k]-b[id][k])-(c[id][k]-d[id][k]);max=Math.max(max,Math.abs(e));rms+=e*e;n++;}return {max,rms:Math.sqrt(rms/n)};}
function stats(a,b){let max=0,maxId=-1,maxC=-1,sum2=0,nz=0;const rows=[];for(let id=0;id<a.length;id++){let row=0;for(let c=0;c<3;c++){const d=Math.abs(a[id][c]-b[id][c]);row=Math.max(row,d);if(d>max){max=d;maxId=id;maxC=c;}sum2+=d*d;if(d)nz++;}if(row)rows.push([row,id]);}rows.sort((a,b)=>b[0]-a[0]);return {max,maxId,maxC,rms:Math.sqrt(sum2/(a.length*3)),nz,top:rows.slice(0,10)};}
console.log(JSON.stringify({source_result:sourcePath,root_result:rootPath,fixture_sha256:s.fixture.sha256,harness_sha256:s.harness_sha256,sourceCenters,rootCenters,sourceRadius:Math.sqrt(sr2),rootRadius:Math.sqrt(rr2),sourceToObserved:stats(ps,sa.pos),rootToObserved:stats(pr,ra.pos),cameraOnlyPredicted:stats(ps,pSame),pairedObserved:stats(sa.pos,ra.pos),predictedPaired:stats(ps,pr),sameCameraSourceToRoot:stats(pSame,ra.pos),cameraAttributionResidual:residual(ps,pr,sa.pos,ra.pos)},null,2));
