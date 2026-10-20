// C300 frozen two-case Source Drag symmetry/mask matrix gate.
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
const [offPath,onPath]=process.argv.slice(2);
if(!offPath||!onPath)throw Error('Pass off/on paired-audit JSON paths');
const [off,on]=await Promise.all([offPath,onPath].map(async p=>JSON.parse(await readFile(p))));
const failures=[];
const check=(value,message)=>{if(!value)failures.push(message);};
check(off.status==='COMPLETE scoped C300 Source Drag case'&&!off.failures?.length,'Symmetry-off paired case failed');
check(on.status==='COMPLETE scoped C300 Source Drag case'&&!on.failures?.length,'Symmetry-on paired case failed');
check(off.harness_sha256===on.harness_sha256&&off.fixture_sha256===on.fixture_sha256&&off.source_build_sha256===on.source_build_sha256&&off.obrush_wasm_sha256===on.obrush_wasm_sha256,'Matrix bytes/build differ');
const witness={};
for(const side of ['source','root']){
  const offRight=off.witnesses?.[side]?.[548]?.displacement;
  const onRight=on.witnesses?.[side]?.[548]?.displacement;
  const onLeft=on.witnesses?.[side]?.[541]?.displacement;
  const mask=on.witnesses?.[side]?.[545]?.displacement;
  const protectedDistance=on.witnesses?.[side]?.[544]?.displacement;
  witness[side]={off_right:offRight,on_right:onRight,on_left:onLeft,on_half_masked:mask,on_protected:protectedDistance,
    right_symmetry_gain:onRight-offRight};
  check(Number.isFinite(onRight)&&Number.isFinite(offRight)&&onRight>offRight+1e-6,`${side}: symmetry did not increase right ID548 movement by >1e-6`);
  check(Number.isFinite(onLeft)&&onLeft>0&&Number.isFinite(mask)&&mask>0&&protectedDistance===0,`${side}: mask/left witness failed`);
}
const result={status:failures.length?'FAILED frozen C300 matrix; evidence retained':'COMPLETE scoped C300 symmetry/mask matrix',failures,offPath,onPath,witness};
const out=path.join(path.dirname(offPath),'matrix-audit-'+path.basename(path.dirname(onPath))+'.json');
await writeFile(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({out,status:result.status,failures,witness}));
if(failures.length)process.exitCode=1;
