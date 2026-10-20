// C329 Pinch Negative matrix; the two contact-calibrated pair audits are immutable inputs.
import {readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';

const [offPath,onPath]=process.argv.slice(2);
if(!offPath||!onPath)throw Error('Pass Negative-off and Negative-on paired audit JSON paths');
const off=JSON.parse(await readFile(offPath));
const on=JSON.parse(await readFile(onPath));
if(off.negative!==false||on.negative!==true||off.fixture_sha256!==on.fixture_sha256||
   off.fixture_sha256!=='f864eb78bf6f280c92fcaa3debcaa406c9839a82e861d213e396233ac6269144')
  throw Error('C329 matrix case order or pinned fixture mismatch');

const perHost={};
for(const side of ['source','root']){
  const a=new Map(off.center_effect[side].per_id.map(row=>[row.id,row.projection]));
  const b=new Map(on.center_effect[side].per_id.map(row=>[row.id,row.projection]));
  const witnesses=[];
  for(const [id,positive] of a){
    const negative=b.get(id);
    if(positive>1e-7&&negative< -1e-7)
      witnesses.push({id,negative_off:positive,negative_on:negative});
  }
  perHost[side]={common_opposite_witness_count:witnesses.length,examples:witnesses.slice(0,12),
    protected_id_1201_exact:off.gates.protected_1201_exact&&on.gates.protected_1201_exact,
    half_id_1202_moves:off.gates.half_1202_moves&&on.gates.half_1202_moves};
}
const gates={off_pair_passed:off.status.startsWith('PASSED'),on_pair_passed:on.status.startsWith('PASSED'),
  source_opposite_witness:perHost.source.common_opposite_witness_count>0,
  root_opposite_witness:perHost.root.common_opposite_witness_count>0,
  protected_and_half:['source','root'].every(side=>
    perHost[side].protected_id_1201_exact&&perHost[side].half_id_1202_moves)};
const failed=Object.entries(gates).filter(([,v])=>!v).map(([k])=>k);
const out=path.join(path.dirname(offPath),'matrix-audit-c329.json');
const result={off_audit:offPath,on_audit:onPath,fixture_sha256:off.fixture_sha256,
  per_host:perHost,gates,failed_gates:failed,
  status:failed.length?'FAILED C329 Pinch four-run matrix; output retained':'PASSED scoped C329 Pinch four-run matrix',
  limitation:'A single static fixture and ordinary mouse gesture do not close the complete Pinch or ZBrush parity rows.'};
await writeFile(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({out,status:result.status,failed_gates:failed,per_host:perHost}));
if(failed.length)process.exitCode=1;
