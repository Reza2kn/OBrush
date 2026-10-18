// Owned pinned-SculptGL browser import. Reads the installed runtime alpha only.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const mirror = path.join(root, '.tools/sculptgl-reference/host-mirror');
const png = process.argv.includes('--hidden-rgb-png');
const quantized = process.argv.includes('--quantized-png');
if (png && quantized) throw Error('Select only one PNG fixture');
const fixtureName = quantized ? 'independent-alpha-quantization16x16.png' :
  png ? 'independent-hidden-rgb2x2.png' : 'independent-gray-bands16x16.jpg';
const fixture = path.join(root, 'crates/obrush-app/tests/fixtures', fixtureName);
const out = path.join(root, '.artifacts/sculptgl-jpeg-alpha', randomUUID());
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
await mkdir(out, { recursive: true });
const result = { status: 'running', fixture_sha256: sha(await readFile(fixture)),
  requests: [], cdp_trace: [], download_events: [], downloads: [], dialogs: [],
  exceptions: [], console_errors: [], blocked_external_requests: [], cleanup_errors: [] };
const owner = new OwnedCdp(out, result);
let sessionId;
try {
  const manifest = JSON.parse(await readFile(path.join(mirror, 'mirror-manifest.json'), 'utf8'));
  result.reference = {
    bundle_sha256: sha(await readFile(path.join(mirror, 'sculptgl.js'))),
    html_sha256: sha(await readFile(path.join(mirror, 'index.html'))),
  };
  if (result.reference.bundle_sha256 !== manifest.original_bundle.sha256 ||
      result.reference.html_sha256 !== manifest.mirrored_html.sha256) {
    throw Error('Pinned reference mirror identity changed');
  }
  const url = await owner.serve(mirror, 'pinned SculptGL');
  await owner.start();
  const target = await owner.send('Target.createTarget', { url: 'about:blank' });
  sessionId = (await owner.send('Target.attachToTarget', { targetId: target.targetId, flatten: true })).sessionId;
  owner.sessions.set(sessionId, 'sculptgl-jpeg-alpha');
  await owner.send('Runtime.enable', {}, sessionId);
  await owner.send('Page.enable', {}, sessionId);
  await owner.send('DOM.enable', {}, sessionId);
  await owner.send('Page.navigate', { url }, sessionId);
  const evalJs = async expression => {
    const response = await owner.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
    if (response.exceptionDetails) throw Error(JSON.stringify(response.exceptionDetails));
    return response.result.value;
  };
  for (let i = 0; i < 150; i++) {
    if (await evalJs('!!window.__sculptglReferenceInstance?.getGui?.()')) break;
    if (i === 149) throw Error('Reference startup deadline');
    await pause(60);
  }
  const doc = (await owner.send('DOM.getDocument', {}, sessionId)).root;
  const input = (await owner.send('DOM.querySelector', { nodeId: doc.nodeId, selector: '#alphaopen' }, sessionId)).nodeId;
  if (!input) throw Error('Pinned reference alpha file input missing');
  // The same browser file-input change route used by GuiSculpting.loadAlpha.
  await owner.send('DOM.setFileInputFiles', { nodeId: input, files: [fixture] }, sessionId);
  let alpha;
  for (let i = 0; i < 150; i++) {
    alpha = await evalJs(`(() => {
      const app = window.__sculptglReferenceInstance;
      const id = app.getSculptManager().getCurrentTool()._idAlpha;
      if (typeof id !== 'string' || !id.includes(${JSON.stringify(fixtureName)})) return null;
      const pick = app.getPicking();
      pick.setIdAlpha(id);
      const a = pick._alpha;
      if (!a?._texture) return null;
      pick._alphaLookAt.set([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]);
      pick._alphaSide = 1;
      const sym = app.getPickingSymmetry();
      sym.setIdAlpha(id);
      sym._alphaLookAt.set(pick._alphaLookAt);
      sym._alphaSide = 1;
      return { id, width:a._width, height:a._height, ratioX:a._ratioX, ratioY:a._ratioY,
        values:Array.from(a._texture),
        samples:[-.75,-.25,.25,.75,2].map(x => ({x,primary:pick.getAlpha(x,0,0),mirror:sym.getAlpha(x,0,0)})) };
    })()`);
    if (alpha) break;
    if (i === 149) throw Error('Reference JPEG alpha import deadline');
    await pause(60);
  }
  result.alpha = alpha;
  if (quantized) {
    if (alpha.width !== 16 || alpha.height !== 16 || alpha.values.length !== 256 ||
        sha(Buffer.from(alpha.values)) !== 'd13e3ecbfb0f82d8b8c2be5a4d6c794626fee15a11508d1e9d7ca299ad715534') {
      throw Error('Pinned reference quantized PNG alpha bytes differ');
    }
    if (process.argv.includes('--brush-kernel-pair')) {
      result.brush = await evalJs(`(() => {
        const app = window.__sculptglReferenceInstance;
        const tool = app.getSculptManager().getCurrentTool();
        const pick = app.getPicking();
        const before = [-0.5,-0.25,0, 0.5,-0.25,0, 0,0.5,0];
        const vertices = Float32Array.from(before);
        const materials = Float32Array.from([0,0,1, 0,0,1, 0,0,1]);
        const fake = { getVertices:() => vertices, getVerticesProxy:() => vertices,
          getMaterials:() => materials };
        const prior = tool._forceToolMesh;
        const accumulate = tool._accumulate;
        let afterFirst;
        try {
          tool.setToolMesh(fake);
          tool._accumulate = true;
          tool.brush([0,1,2], [0,0,1], [0,0,0], 2, 0.25, pick);
          afterFirst = Array.from(vertices);
          tool.brush([0,1,2], [0,0,1], [0,0,0], 2, 0.25, pick);
        } finally {
          tool._accumulate = accumulate;
          tool.setToolMesh(prior);
        }
        return {before,after_first:afterFirst,after_second:Array.from(vertices),radius_squared:2,intensity:0.25,
          normal:[0,0,1],center:[0,0,0],unmasked:[1,1,1],selected:[0,1,2],
          alpha_samples:[0,1,2].map(i => pick.getAlpha(before[i*3],before[i*3+1],0))};
      })()`);
      if (!result.brush.after_first.some((v, i) => v !== result.brush.before[i]) ||
          !result.brush.after_second.some((v, i) => v !== result.brush.after_first[i])) {
        throw Error('Pinned direct Brush response did not change the independent triangle');
      }
    }
    const movingWithAlpha = process.argv.includes('--brush-moving-alpha-pair');
    const repeatedWithAlpha = process.argv.includes('--brush-repeated-alpha-pair') || movingWithAlpha;
    if (process.argv.includes('--brush-repeated-pair') || repeatedWithAlpha) {
      const resultKey = movingWithAlpha ? 'moving_alpha_brush' :
        repeatedWithAlpha ? 'repeated_alpha_brush' : 'repeated_brush';
      result[resultKey] = await evalJs(`(() => {
        const withAlpha = ${JSON.stringify(repeatedWithAlpha)};
        const moving = ${JSON.stringify(movingWithAlpha)};
        const app = window.__sculptglReferenceInstance;
        const tool = app.getSculptManager().getCurrentTool();
        const pick = app.getPicking();
        const before = [-0.5,-0.25,0, 0.5,-0.25,0, 0,0.5,0];
        const vertices = Float32Array.from(before);
        const normals = new Float32Array(9);
        const faces = Uint32Array.from([0,1,2,4294967295]);
        const materials = Float32Array.from([0,0,1, 0,0,1, 0,0,1]);
        const fake = {getVertices:() => vertices, getVerticesProxy:() => vertices,
          getMaterials:() => materials, getNormals:() => normals, getFaces:() => faces};
        const priorMesh = tool._forceToolMesh;
        const priorAccumulate = tool._accumulate;
        const priorPickMesh = pick._mesh;
        const priorPickedFace = pick._pickedFace;
        const priorInterPoint = pick._interPoint;
        const priorPickedNormal = pick._pickedNormal;
        const priorAlpha = pick._alpha;
        const priorAlphaSide = pick._alphaSide;
        const priorAlphaOrigin = pick._alphaOrigin;
        const priorRadiusSquared = pick._rLocal2;
        const priorAlphaFrame = Float32Array.from(pick._alphaLookAt);
        let afterFirst, refreshedFaceNormal, pickedSecond, secondFrame, secondSamples;
        try {
          tool.setToolMesh(fake);
          tool._accumulate = true;
          if (!withAlpha) pick._alpha = null;
          else pick._alphaSide = 1;
          pick._mesh = fake;
          pick._pickedFace = 0;
          pick._interPoint = [0,0,0];
          pick._pickedNormal = [0,0,1];
          if (moving) {
            pick._alphaOrigin = [0,0,0];
            pick._rLocal2 = 2;
            pick.updateAlpha(false);
          }
          tool.brush([0,1,2], pick.getPickedNormal(), [0,0,0], 2, 0.25, pick);
          afterFirst = Array.from(vertices);
          const ax=vertices[3]-vertices[0], ay=vertices[4]-vertices[1], az=vertices[5]-vertices[2];
          const bx=vertices[6]-vertices[0], by=vertices[7]-vertices[1], bz=vertices[8]-vertices[2];
          refreshedFaceNormal = [Math.fround(ay*bz-az*by),Math.fround(az*bx-ax*bz),Math.fround(ax*by-ay*bx)];
          for (let i=0;i<3;i++) normals.set(refreshedFaceNormal,i*3);
          if (moving) pick._interPoint = [0.125,0.125,0];
          pick.computePickedNormal();
          pickedSecond = Array.from(pick.getPickedNormal());
          if (moving) pick.updateAlpha(false);
          secondFrame = Array.from(pick._alphaLookAt);
          secondSamples = [0,1,2].map(i => pick.getAlpha(vertices[i*3],vertices[i*3+1],vertices[i*3+2]));
          tool.brush([0,1,2], pick.getPickedNormal(), pick.getIntersectionPoint(), 2, 0.25, pick);
        } finally {
          tool._accumulate = priorAccumulate;
          tool.setToolMesh(priorMesh);
          pick._mesh = priorPickMesh;
          pick._pickedFace = priorPickedFace;
          pick._interPoint = priorInterPoint;
          pick._pickedNormal = priorPickedNormal;
          pick._alpha = priorAlpha;
          pick._alphaSide = priorAlphaSide;
          pick._alphaOrigin = priorAlphaOrigin;
          pick._rLocal2 = priorRadiusSquared;
          pick._alphaLookAt.set(priorAlphaFrame);
        }
        return {before,after_first:afterFirst,after_second:Array.from(vertices),
          refreshed_face_normal:refreshedFaceNormal,picked_second:pickedSecond,
          radius_squared:2,intensity:0.25,centers:[[0,0,0],moving?[0.125,0.125,0]:[0,0,0]],
          second_alpha_frame:secondFrame,second_alpha_samples:secondSamples,
          unmasked:[1,1,1],selected:[0,1,2],alpha:withAlpha?'quantized16x16':null};
      })()`);
      const repeated = result[resultKey];
      if (!repeated.after_first.some((v,i) => v !== repeated.before[i]) ||
          !repeated.after_second.some((v,i) => v !== repeated.after_first[i]) ||
          !repeated.picked_second.every(Number.isFinite)) {
        throw Error('Pinned repeated Brush response or refreshed picked normal invalid');
      }
    }
    result.status = 'PASS pinned SculptGL quantized PNG import and 256 alpha bytes' +
      (result.brush ? ' plus direct Brush response' : '') +
      (result.repeated_brush ? ' plus repeated no-alpha Brush response' : '') +
      (result.repeated_alpha_brush ? ' plus repeated alpha Brush response' : '') +
      (result.moving_alpha_brush ? ' plus moving alpha Brush response' : '');
  } else if (png) {
    if (alpha.width !== 2 || alpha.height !== 2 ||
        JSON.stringify(alpha.values) !== JSON.stringify([0, 85, 85, 255])) {
      throw Error('Pinned reference hidden-RGB PNG alpha values differ');
    }
    const primary = [255, 255, 85, 85, 0];
    const mirror = [85, 85, 255, 255, 0];
    if (alpha.samples.some((s, i) => s.primary !== primary[i] / 255 ||
        s.mirror !== mirror[i] / 255)) {
      throw Error('Pinned reference hidden-RGB PNG nearest samples differ');
    }
    result.status = 'PASS pinned SculptGL hidden-RGB PNG import and alpha samples';
  } else {
    if (alpha.width !== 16 || alpha.height !== 16 || alpha.values.length !== 256 ||
        !alpha.values.every((v, i) => v === (i % 16 < 8 ? 64 : 192))) {
      throw Error('Pinned reference decoded alpha differs from independent grayscale bands');
    }
    const expectedPrimary = [192, 192, 64, 64, 0];
    const expectedMirror = [64, 64, 192, 192, 0];
    if (alpha.samples.some((s, i) => s.primary !== expectedPrimary[i] / 255 ||
        s.mirror !== expectedMirror[i] / 255)) {
      throw Error('Pinned reference nearest sampler differs from expected primary/mirror values');
    }
    result.status = 'PASS pinned SculptGL JPEG import and primary/mirror alpha samples';
  }
} catch (error) {
  result.status = 'FAIL preserved';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  await owner.close();
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, result: path.join(out, 'result.json'), failure: result.failure ?? null }));
}
