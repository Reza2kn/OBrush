// Independent whole-public-save and active-GPU audit for C270. This receives
// two separately owned normal-browser result.json paths, one polarity pair.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [sourcePath, rootPath] = process.argv.slice(2);
assert(sourcePath && rootPath, 'Pass source and root result.json paths');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? path.join(path.dirname(new URL(import.meta.url).pathname), '..'));
const { decodeIndependent } = await import(pathToFileURL(path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/independent-fixtures.mjs')).href);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const word = value => { const b = Buffer.allocUnsafe(4); b.writeFloatLE(value); return b.readUInt32LE(); };
const float = bits => { const b = Buffer.allocUnsafe(4); b.writeUInt32LE(bits); return b.readFloatLE(); };
const source = JSON.parse(await readFile(sourcePath));
const root = JSON.parse(await readFile(rootPath));
assert.equal(source.side, 'source'); assert.equal(root.side, 'root');
assert.equal(source.negative, root.negative);
assert.equal(source.cohort, root.cohort);
assert.equal(source.fixture.sha256, root.fixture.sha256);
const files = {}, meshes = {};
for (const [side, result] of [['source', source], ['root', root]]) {
  for (const stage of ['before', 'after', 'undo', 'redo']) {
    const record = result[stage]; assert(record?.path && record.sha256, `${side} ${stage} public save missing`);
    const bytes = await readFile(record.path);
    assert.equal(sha(bytes), record.sha256, `${side} ${stage} public save hash drift`);
    files[`${side}_${stage}`] = bytes;
    if (stage === 'undo' || stage === 'redo') continue;
    const mesh = side === 'source' ? decodeIndependent(bytes).meshes[0] : (() => {
      const m=JSON.parse(bytes).mesh;
      assert(m?.positions && m.normals && m.triangles && m.masks && m.colors, 'Complete public OBJ mesh missing');
      const rows=a=>a.map(row=>row.map(Math.fround)),mask=m.masks.map(Math.fround);
      return {positions:rows(m.positions),colors:rows(m.colors),materials:m.materials?rows(m.materials):null,
        unprotected:m.mask_convention==='unprotected_f32_v1'?mask:mask.map(v=>Math.fround(1-v)),
        faces:m.surface?.faces??m.triangles};
    })();
    assert.equal(mesh.positions.length, 4, `${side} vertex count`);
    const decode = value => side === 'source' ? float(value) : Math.fround(value);
    const original = [[-.5,0],[0,-.5],[.5,0],[0,.5]];
    const ids = mesh.positions.map(p => {
      const xy = [decode(p[0]), decode(p[1])];
      const id = original.findIndex(q => q[0] === xy[0] && q[1] === xy[1]);
      assert(id >= 0, `${side} unknown original XY: ${xy}`); return id;
    });
    assert.equal(new Set(ids).size, 4, `${side} duplicated diamond ID`);
    const positions = [], positionWords = [], normals = [], normalWords = [], attrs = [];
    for (let i = 0; i < 4; i++) {
      const id = ids[i];
      const p = mesh.positions[i].map(decode);
      positions[id] = p; positionWords[id] = p.map(word);
      // Public SGL does not carry a separately ordered normal array in this
      // decoder; only Root's GPU normal words are checked against its save.
      if (side === 'root') {
        const document = JSON.parse(bytes), row = document.mesh.normals[i].map(Math.fround);
        normals[id] = row; normalWords[id] = row.map(word);
      }
      const colors = mesh.colors[i].map(decode), material = mesh.materials?.[i]?.map(decode)??null;
      attrs[id] = { colors: colors.map(word), material: material?.map(word)??null, unprotected: word(side === 'source' ? material[2] : mesh.unprotected[i]) };
    }
    const faces = mesh.faces.map(face => face.slice(0,face[3]===0xffffffff?3:4).map(i=>ids[i]));
    const oriented = faces.map(face => { const min = face.indexOf(Math.min(...face)); return [...face.slice(min),...face.slice(0,min)].join(','); }).sort();
    meshes[`${side}_${stage}`] = { positions, positionWords, normals, normalWords, attrs, oriented };
  }
}
const compare = (a,b) => {
  let changed = [], components = 0, maximum = 0, sum = 0;
  for (let id=0; id<4; id++) {
    if (a.positionWords[id].some((w,c)=>w!==b.positionWords[id][c])) changed.push(id);
    for (let c=0;c<3;c++) {
      const delta = a.positions[id][c] - b.positions[id][c];
      components += Number(a.positionWords[id][c] !== b.positionWords[id][c]);
      maximum = Math.max(maximum,Math.abs(delta)); sum += delta*delta;
    }
  }
  return { changed_ids:changed, different_components:components, max_abs_component:maximum, rms_component:Math.sqrt(sum/12) };
};
const shape = side => compare(meshes[`${side}_before`],meshes[`${side}_after`]);
const beforePair = compare(meshes.source_before,meshes.root_before);
const afterPair = compare(meshes.source_after,meshes.root_after);
const sourceChange = shape('source'), rootChange = shape('root');
const exact = (a,b) => JSON.stringify(a)===JSON.stringify(b);
const expectedGpu = stage => {
  const d=JSON.parse(files[`root_${stage}`]),m=d.mesh;
  const bytes=Buffer.alloc(m.positions.length*40);
  for(let i=0;i<m.positions.length;i++) {
    const displayMask=m.mask_convention==='unprotected_f32_v1'?Math.fround(1-Math.fround(m.masks[i])):m.masks[i];
    const values=[...m.positions[i],...m.normals[i],displayMask,...m.colors[i]];
    assert.equal(values.length,10);
    values.forEach((v,c)=>bytes.writeFloatLE(v,i*40+c*4));
  }
  return {bytes:bytes.length,sha256:sha(bytes),position_words:Array.from({length:4},(_,i)=>m.positions[i].map(word)).flat(),normal_words:Array.from({length:4},(_,i)=>m.normals[i].map(word)).flat()};
};
const gpu = {};
for (const stage of ['after','redo']) {
  const expected=expectedGpu(stage),observed=root[`${stage}_gpu`];
  const matches=observed?.rows?.filter(row=>row.bytes===expected.bytes&&row.sha256===expected.sha256&&exact(row.position_words,expected.position_words)&&exact(row.normal_words,expected.normal_words))??[];
  gpu[stage]={expected,observed,exact_candidate_count:matches.length,pass:matches.length===1&&observed.gl_error===0};
}
const history={source_before_undo:files.source_before.equals(files.source_undo),source_after_redo:files.source_after.equals(files.source_redo),root_before_undo:files.root_before.equals(files.root_undo),root_after_redo:files.root_after.equals(files.root_redo)};
const expectedFaces=['0,1,2','0,2,3'];
const faces={source_input:exact(meshes.source_before.oriented,expectedFaces),root_input:exact(meshes.root_before.oriented,expectedFaces),source_stable:exact(meshes.source_before.oriented,meshes.source_after.oriented),root_stable:exact(meshes.root_before.oriented,meshes.root_after.oriented),paired_before:exact(meshes.source_before.oriented,meshes.root_before.oriented),paired_after:exact(meshes.source_after.oriented,meshes.root_after.oriented)};
const attributes={source_stable:exact(meshes.source_before.attrs,meshes.source_after.attrs),root_stable:exact(meshes.root_before.attrs,meshes.root_after.attrs),paired_before:exact(meshes.source_before.attrs,meshes.root_before.attrs),paired_after:exact(meshes.source_after.attrs,meshes.root_after.attrs)};
attributes.all_unmasked_before=['source','root'].every(side=>meshes[`${side}_before`].attrs.every(row=>row.unprotected===word(1)));
const alignment=root.ray_alignment;
const sourceProjectionX=source.setup.state.camera.projection[0],rootProjectionX=root.setup.state.camera.projection[0];
const sourceWidth=source.setup.viewport.screen[2]-source.setup.viewport.screen[0],rootWidth=root.setup.viewport.screen[2]-root.setup.viewport.screen[0];
const rayDiff=source.requested_path.map(([x],i)=>Math.abs(2*(x-(source.setup.viewport.screen[0]+source.setup.viewport.screen[2])/2)/(sourceWidth*sourceProjectionX)-2*(root.requested_path[i][0]-(root.setup.viewport.screen[0]+root.setup.viewport.screen[2])/2)/(rootWidth*rootProjectionX)));
const sourceProjectionY=source.setup.state.camera.projection[5],rootProjectionY=root.setup.state.camera.projection[5];
const sourceHeight=source.setup.viewport.screen[3]-source.setup.viewport.screen[1],rootHeight=root.setup.viewport.screen[3]-root.setup.viewport.screen[1];
const rayYDiff=source.requested_path.map(([,y],i)=>Math.abs(2*(y-(source.setup.viewport.screen[1]+source.setup.viewport.screen[3])/2)/(sourceHeight*sourceProjectionY)-2*(root.requested_path[i][1]-(root.setup.viewport.screen[1]+root.setup.viewport.screen[3])/2)/(rootHeight*rootProjectionY)));
const radiusDiff=Math.abs(2*source.config.state.tool._radius/(sourceWidth*sourceProjectionX)-2*root.config.state.source_geometry.per_tool[2].radius/(rootWidth*rootProjectionX));
const membershipEqual=exact(sourceChange.changed_ids,rootChange.changed_ids);
const numericalGate=sourceChange.changed_ids.length>0&&rootChange.changed_ids.length>0&&membershipEqual&&Math.max(...rayDiff)<=1e-6&&Math.max(...rayYDiff)<=1e-6&&radiusDiff<=1e-6&&afterPair.max_abs_component<=3e-8;
const audit={source_result:sourcePath,root_result:rootPath,source_result_sha256:sha(await readFile(sourcePath)),root_result_sha256:sha(await readFile(rootPath)),
  fixture_sha256:source.fixture.sha256,negative:source.negative,cohort:source.cohort,source_bundle_sha256:source.pinned_bundle_sha256,root_wasm_sha256:root.obrush_wasm_sha256,
  alignment:{source_projection_x:sourceProjectionX,root_projection_x:rootProjectionX,source_width:sourceWidth,root_width:rootWidth,max_ray_x_difference:Math.max(...rayDiff),max_ray_y_difference:Math.max(...rayYDiff),world_radius_abs_difference:radiusDiff,record:alignment},
  history,faces,attributes,before_pair:beforePair,after_pair:afterPair,source_change:sourceChange,root_change:rootChange,changed_membership_equal:membershipEqual,numerical_gate:numericalGate,gpu,
  public_words:{source_before:meshes.source_before.positionWords,root_before:meshes.root_before.positionWords,source_after:meshes.source_after.positionWords,root_after:meshes.root_after.positionWords},
  limitations:['Four-vertex flat unmasked OBJ only','Equal requested rays do not establish equal delivered dabs or later picked contacts','No physical pen/native GUI or sustained performance claim']};
audit.admitted=sourceChange.changed_ids.length>0&&rootChange.changed_ids.length>0&&beforePair.different_components===0&&
  Object.values(history).every(Boolean)&&Object.values(faces).every(Boolean)&&
  attributes.all_unmasked_before&&attributes.source_stable&&attributes.root_stable&&gpu.after.pass&&gpu.redo.pass;
audit.status=audit.admitted?'complete scoped UI pair; inspect numerical_gate separately':'FAILED scoped UI admission; retained';
const out=path.join(path.dirname(sourcePath),`diamond-audit-${path.basename(path.dirname(rootPath))}.json`);
await writeFile(out,JSON.stringify(audit,null,2)+'\n');
console.log(JSON.stringify({out,status:audit.status,admitted:audit.admitted,numerical_gate:numericalGate,source_change:sourceChange,root_change:rootChange,after_pair:afterPair,gpu:{after:gpu.after.pass,redo:gpu.redo.pass}}));
if(!audit.admitted||!numericalGate)process.exitCode=1;
