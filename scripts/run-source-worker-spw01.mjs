#!/usr/bin/env node
// Frozen SP-WORKER-01 ordinary held-pointer diagnostic. No product setters.
// The original C58/C209 CDP helpers are supplied by --bench-stage and hashed.
import { readFile, writeFile, mkdir, stat } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = Object.fromEntries(process.argv.slice(2).map(arg => {
  const match = /^--([a-z-]+)=(.*)$/.exec(arg);
  if (!match) throw Error(`Expected --name=value, got ${arg}`);
  return [match[1], match[2]];
}));
const requireGate = (condition, label) => { if (!condition) throw Error(label); };
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const protocolPath = path.join(root, 'docs/SCULPTGL_WORKER_SUSTAINED_PROTOCOL.json');
const protocolBytes = await readFile(protocolPath), protocol = JSON.parse(protocolBytes);
const mode = args.mode, label = args.cohort;
requireGate(['ordinary', 'worker'].includes(mode), 'mode must be ordinary or worker');
const cohort = protocol.cohorts.find(row => row.label === label);
requireGate(cohort, 'cohort must be 100k or 1m');
const bench = path.resolve(args['bench-stage'] || '');
requireGate(args['bench-stage'] && (await stat(path.join(bench, 'workspace/large-perf-driver-c203.mjs'))).isFile(), 'explicit C58/C209 bench stage required');
const fixtureManifestBytes=await readFile(path.join(bench,'FIXTURE_MANIFEST.json'));
requireGate(hash(fixtureManifestBytes)===protocol.fixture_manifest_sha256,'frozen independent fixture manifest hash');
const originalProtocolBytes=await readFile(path.join(bench,'SUSTAINED_PREREGISTRATION.json'));
requireGate(hash(originalProtocolBytes)===protocol.parent_protocol_sha256,'original 60-second protocol hash');
const helperNames = [
  'workspace/paired-driver-60s.mjs', 'workspace/large-perf-driver-c203.mjs',
  'workspace/perf-driver.mjs', 'workspace/perf-recording-transfer.mjs',
  'workspace/perf-memory-sampler.mjs', 'observer/perf-recorder.mjs',
  'observer/perf-export.mjs', 'observer/perf-memory.mjs'
];
const helperHashes = Object.fromEntries(await Promise.all(helperNames.map(async name =>
  [name, hash(await readFile(path.join(bench, name)))])));
const fixture = path.resolve(args.fixture || '');
requireGate(args.fixture && hash(await readFile(fixture)) === cohort.sgl_sha256, 'frozen SGL SHA-256 mismatch');
const fixtureBytes = (await stat(fixture)).size;
requireGate(fixtureBytes > 0 && fixtureBytes <= 128 * 1024 * 1024, 'bounded fixture');
const manifestItem=JSON.parse(fixtureManifestBytes).files.find(row=>row.label===label);
requireGate(manifestItem&&manifestItem.sha256===cohort.sgl_sha256&&
  manifestItem.bytes===fixtureBytes&&manifestItem.vertices===cohort.vertices&&
  manifestItem.triangles===cohort.triangles,'manifest/cohort/fixture identity');
if (args.preflight === 'true') {
  console.log(JSON.stringify({status:'static preflight only; no browser or build',mode,label,
    product_commit:(await import('node:child_process')).execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
    protocol_sha256:hash(protocolBytes),source_protocol_sha256:hash(originalProtocolBytes),
    fixture_manifest_sha256:hash(fixtureManifestBytes),fixture_sha256:cohort.sgl_sha256,
    fixture_bytes:fixtureBytes,helper_hashes:helperHashes},null,2));
  process.exit(0);
}
const { OwnedCdp } = await import(pathToFileURL(path.join(bench, 'workspace/paired-driver-60s.mjs')));
const { LargePerfDriver, equalFiles } = await import(pathToFileURL(path.join(bench, 'workspace/large-perf-driver-c203.mjs')));
const { installPerfRecorder } = await import(pathToFileURL(path.join(bench, 'observer/perf-recorder.mjs')));
const { installPerfExport } = await import(pathToFileURL(path.join(bench, 'observer/perf-export.mjs')));
const { installPerfMemory } = await import(pathToFileURL(path.join(bench, 'observer/perf-memory.mjs')));
const { readScalarRecording } = await import(pathToFileURL(path.join(bench, 'workspace/perf-recording-transfer.mjs')));
const { sampleMemory, startMemorySamples } = await import(pathToFileURL(path.join(bench, 'workspace/perf-memory-sampler.mjs')));
const frozen = JSON.parse(originalProtocolBytes);
const out = path.join(root, '.artifacts/source-worker-sustained-spw01', `${label}-${mode}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const source = `(${installPerfExport.toString()})(window);\n` +
  `(${installPerfMemory.toString()})(window);\n` +
  `(${installPerfRecorder.toString()})(window,${JSON.stringify(frozen.recorder_limits)});\n` +
  await readFile(path.join(root, 'scripts/source-gpu-readback-observer.js'), 'utf8') + '\n' +
  `(()=>{const trace={messages:[],main:{},files:[],errors:[],overflow:false};window.__workerSustained=trace;
    document.addEventListener('change',e=>{if(e.target?.type==='file')trace.files.push({at:performance.now(),trusted:e.isTrusted,
      id:e.target.id,names:Array.from(e.target.files||[],f=>f.name)});},true);
    const Original=window.Worker;window.Worker=class extends Original{
      constructor(...a){super(...a);if(!String(a[0]).includes('source-geometry-worker'))return;
        const post=this.postMessage.bind(this);this.postMessage=(m,t)=>{if(trace.messages.length>=100000){trace.overflow=true;return post(m,t)}
          trace.messages.push({direction:'main-to-worker',kind:m?.kind,id:m?.id,at:performance.now(),
            bytes:(m?.project?.length||0)+(m?.options?.length||0)+(m?.input?.length||0)+(m?.warmSeed?.byteLength||0),
            transfer_or_clone:t?.length?'transfer':'clone',transfer_count:t?.length||0});return post(m,t)};
        this.addEventListener('message',e=>{const m=e.data;if(trace.messages.length>=100000){trace.overflow=true;return}
          trace.messages.push({direction:'worker-to-main',kind:m?.kind,id:m?.id,at:performance.now(),
            bytes:(m?.delta?.byteLength||0)+(m?.retained?.byteLength||0),
            transfer_or_clone:m?.delta?.buffer?.byteLength===m?.delta?.byteLength?'transfer':'unknown',
            success:m?.success,timing:m?.timing||null});});}
    };
    window.__instrumentWorkerMain=async()=>{try{const backend=window.obrushWasmBackend==='simd'?'./pkg-simd/obrush_app.js':'./pkg/obrush_app.js';
      const mod=await import(backend),p=mod.WebHandle.prototype;
      for(const name of ['source_worker_held_begin','source_worker_held_seed_packet','source_worker_held_samples','source_worker_held_preview','source_worker_held_commit']){
        const original=p[name];if(typeof original!=='function')throw Error('Missing '+name);
        p[name]=function(...a){const start=performance.now();try{return original.apply(this,a)}finally{const v=trace.main[name]||{count:0,total_ms:0,max_ms:0};
          const ms=performance.now()-start;v.count++;v.total_ms+=ms;v.max_ms=Math.max(v.max_ms,ms);trace.main[name]=v;}};}
      return true;}catch(error){trace.errors.push(String(error));return false;}};
  })();`;

const result = { status:'running', out, mode, protocol_sha256:hash(protocolBytes),
  source_protocol_sha256:hash(originalProtocolBytes),fixture_manifest_sha256:hash(fixtureManifestBytes),
  product_commit:(await import('node:child_process')).execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
  helper_hashes:helperHashes, fixture:{label,vertices:cohort.vertices,triangles:cohort.triangles,
    path:fixture,sha256:cohort.sgl_sha256,bytes:fixtureBytes},
  requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  page_observations:[], acknowledgments:[], failed_acknowledgments:[], viewport_measurements:[],
  cleanup_errors:[], scheduled:[], tail:[], memory_samples:[],
  worker_sustained_attestation:{input_path:'ordinary_browser_held_pointer',manual_worker_api_calls:0,
    wasm_dab_shortcuts:0,ordinary_ui_file_import:false,
    runtime_path:mode==='worker'?'worker_geometry_published_to_normal_studio':'ordinary_main_thread_source'} };
const owner = new OwnedCdp(out, result);
let page, pressed=false, recording=false, sampler;
const gpu={stages:{}};
const gpuStage=async stage=>{
  const v=await page.viewport(),x=(v.screen[0]+v.screen[2])/2+80,y=(v.screen[1]+v.screen[3])/2;
  const capture=page.evaluate(`window.__sourceGpuReadback.capture(${cohort.vertices})`);
  await pause(10);
  await page.pointer('mouseMoved',x+(stage.length%3),y,'none',0);
  const observation=await capture;
  gpu.stages[stage]=observation;
  if(observation.active_vbo_candidates!==1 || observation.gl_error!==0) throw Error(`GPU ${stage}: ${JSON.stringify(observation)}`);
};
const workerStatus=()=>page.evaluate('window.obrushSourceWorkerTrialStatus()');
try {
  const web=path.join(root,'web/dist'),wasm=path.join(web,'pkg/obrush_app_bg.wasm');
  result.product_binding={wasm_sha256:hash(await readFile(wasm)),js_sha256:hash(await readFile(path.join(web,'pkg/obrush_app.js'))),
    index_sha256:hash(await readFile(path.join(web,'index.html'))),worker_js_sha256:hash(await readFile(path.join(web,'source-geometry-worker.js'))),
    worker_input_js_sha256:hash(await readFile(path.join(web,'source-worker-held-input.js')))};
  const url=(await owner.serve(web,'root'))+(mode==='worker'?'?source-worker-held':'');
  await owner.start();result.browser=await owner.send('Browser.getVersion');
  page=await owner.page('root',url,source);Object.setPrototypeOf(page,LargePerfDriver.prototype);
  await page.send('Page.bringToFront');
  await page.prepareFixture(result.fixture);
  const fileEvents=await page.evaluate('window.__workerSustained.files');
  requireGate(fileEvents.some(row=>row.trusted&&row.names.length===1&&row.names[0]===path.basename(fixture)),
    'normal trusted file import event missing');
  result.worker_sustained_attestation.ordinary_ui_file_import=true;
  result.worker_sustained_attestation.file_events=fileEvents;
  result.setup=(await page.live()).state;
  const p=result.setup.source_geometry?.per_tool?.[0];
  requireGate(p?.mode==='SourceOrdinary'&&p.radius===64&&p.options.intensity===.25&&
    p.options.tool.Brush.clay===false&&p.options.tool.Brush.accumulate===false&&
    result.setup.symmetry_x===false,'actual ordinary Source Draw controls');
  requireGate(await page.evaluate('window.__instrumentWorkerMain()'),'main stage instrumentation');
  if(mode==='worker'){
    const trial=await page.evaluate(`(()=>{const b=Array.from(document.querySelectorAll('button')).find(x=>x.textContent.includes('Source Worker · trial off'));
      if(!b) return null; const r=b.getBoundingClientRect();return {x:(r.left+r.right)/2,y:(r.top+r.bottom)/2,rect:[r.left,r.top,r.right,r.bottom]};})()`);
    requireGate(trial,'normal rendered Worker trial button');result.ui_actions.push({kind:'worker trial button',...trial});
    await page.click(trial.x,trial.y);requireGate((await workerStatus()).enabled===true,'normal Worker button did not enable');
  }
  result.before=(await page.save('before')).record;await gpuStage('before');
  const viewport=await page.viewport(),center=[(viewport.screen[0]+viewport.screen[2])/2,(viewport.screen[1]+viewport.screen[3])/2];
  const first=[center[0]+80,center[1]];result.viewport=viewport;
  await pause(1000);result.memory_samples.push(await sampleMemory(owner,page,'before'));
  const clock=await page.evaluate('({now:performance.now(),origin:performance.timeOrigin,visibility:document.visibilityState})');
  requireGate(clock.visibility==='visible','owned page not foreground');result.clock=clock;
  await page.pointer('mouseMoved',...first);await page.evaluate(`window.__obrushPerf.begin({vertices:${cohort.vertices}})`);recording=true;
  sampler=startMemorySamples(owner,page,result.memory_samples);
  await page.pointer('mousePressed',...first,'left',1);pressed=true;await page.raf();
  const start=performance.now(),pending=[];
  for(let i=1;i<=cohort.requested_moves;i++){
    const due=start+i*1000/protocol.input.motion_hz,remain=due-performance.now();if(remain>0)await pause(remain);
    const t=i/protocol.input.motion_hz,point=[center[0]+80*Math.cos(2*Math.PI*t/2),center[1]+60*Math.sin(2*Math.PI*t/2)];
    const row={index:i,planned:due,sent:performance.now(),point};result.scheduled.push(row);
    pending.push(page.send('Input.dispatchMouseEvent',{type:'mouseMoved',x:point[0],y:point[1],button:'none',buttons:1,pointerType:'mouse',force:.5})
      .then(()=>{row.ack=performance.now()},error=>{row.failure=String(error)}));
  }
  result.release={requested:performance.now(),point:first};
  await page.pointer('mouseReleased',...first,'left',0);pressed=false;
  result.release.acknowledged=performance.now();await Promise.all(pending);
  requireGate(result.scheduled.every(row=>Number.isFinite(row.ack)&&!row.failure),'all CDP moves acknowledged');
  const tailStart=performance.now();for(let i=0;i<protocol.bounds.tail_samples;i++){
    result.tail.push({node:performance.now(),summary:await page.evaluate('window.__obrushPerf.summary()'),
      queue:await page.evaluate('window.obrushSculptMetrics()'),worker:await workerStatus()});
    const remain=tailStart+(i+1)*protocol.bounds.tail_sample_ms-performance.now();if(remain>0)await pause(remain);
  }
  result.memory_sampling=await sampler.stop();sampler=null;result.memory_samples.push(await sampleMemory(owner,page,'after-tail'));
  await page.evaluate('window.__obrushPerf.end()');recording=false;
  const transferred=await readScalarRecording(page);result.scalar_transport=transferred.transport;
  result.recorder={path:path.join(out,'scalar-recording.json'),bytes:transferred.bytes.length,sha256:hash(transferred.bytes)};
  await writeFile(result.recorder.path,transferred.bytes);
  result.worker_trace=await page.evaluate('window.__workerSustained');
  result.worker_trial=await workerStatus();
  if(mode==='worker')requireGate(result.worker_trial.enabled&&!result.worker_trial.active&&
    result.worker_trial.commits===1&&result.worker_trial.rejections===0,
    'worker held gesture did not make one complete normal publication');
  const delivered=transferred.record.events.filter(row=>row[2]>=1&&row[2]<=3);
  result.worker_sustained_attestation.delivered_dom_events=delivered.length;
  result.worker_sustained_attestation.trusted_dom_trace_sha256=hash(Buffer.from(JSON.stringify(delivered)));
  result.after=(await page.save('after')).record;await gpuStage('after');
  await pause(500);result.stable=(await page.save('stable')).record;await gpuStage('stable');
  requireGate(await equalFiles(result.stable.path,result.after.path),'public after/stable differs');
  await page.chord('z');result.undo=(await page.save('undo')).record;await gpuStage('undo');
  requireGate(await equalFiles(result.undo.path,result.before.path),'public undo differs');
  await page.chord('z',true);result.redo=(await page.save('redo')).record;await gpuStage('redo');
  requireGate(await equalFiles(result.redo.path,result.after.path),'public redo differs');
  result.status='Completed actual sustained timed/file/history cohort; independent GPU/shape/interaction audit pending';
} catch(error) {
  result.status='FAILED actual sustained cohort; preserve raw receipt';result.failure=String(error.stack||error);process.exitCode=1;
  try{result.failure_state=await page?.state()}catch(e){result.failure_state_error=String(e)}
  try{result.worker_trial=await workerStatus();result.worker_trace=await page.evaluate('window.__workerSustained')}catch(e){result.failure_worker_error=String(e)}
  try{if(recording){await page.evaluate('window.__obrushPerf.end()');recording=false;
    const transferred=await readScalarRecording(page);result.failure_recorder={path:path.join(out,'failed-scalar-recording.json'),bytes:transferred.bytes.length,sha256:hash(transferred.bytes)};
    await writeFile(result.failure_recorder.path,transferred.bytes)}}catch(e){result.failure_recorder_error=String(e)}
} finally {
  if(sampler)try{result.memory_sampling=await sampler.stop()}catch(e){result.cleanup_errors.push(String(e))}
  if(pressed&&page)try{const v=await page.viewport();await page.pointer('mouseReleased',(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2,'left',0)}catch(e){result.cleanup_errors.push(String(e))}
  try{await owner.close()}catch(e){result.cleanup_errors.push(String(e));process.exitCode=1}
  const resultBytes=Buffer.from(JSON.stringify(result,null,2)+'\n');await writeFile(path.join(out,'result.json'),resultBytes);
  await writeFile(path.join(out,'gpu-browser.json'),JSON.stringify(gpu,null,2)+'\n');
  if(mode==='worker'&&result.worker_trace){
    const main=result.worker_trace.main||{},rows=result.worker_trace.messages||[];
    const names=['source_worker_held_begin','source_worker_held_seed_packet','source_worker_held_samples',
      'source_worker_held_preview','source_worker_held_commit'];
    const mainMs=name=>Number(main[name]?.total_ms)||0;
    const messages=rows.filter(row=>row.bytes>0).map(row=>({
      kind:row.kind==='begin'?'init':row.kind==='finish'?'publish':'dab',
      direction:row.direction,bytes:row.bytes,transfer_or_clone:row.transfer_or_clone,
      id:row.id,at:row.at,timing:row.timing||null}));
    const sends=new Map(rows.filter(row=>row.direction==='main-to-worker').map(row=>[row.id,row]));
    const roundTrips=rows.filter(row=>row.direction==='worker-to-main'&&sends.has(row.id))
      .map(row=>({id:row.id,kind:row.kind,ms:row.at-sends.get(row.id).at}));
    const mainStages={serialize:mainMs(names[0])+mainMs(names[1])+mainMs(names[2]),
      validate:mainMs(names[3]),publish:mainMs(names[4])};
    const largestMainStage=Object.entries(mainStages).sort((a,b)=>b[1]-a[1])[0]||null;
    const transport={result_sha256:hash(resultBytes),messages,
      accepted_dabs:result.worker_trial?.acknowledgedDabs??null,
      main_serialize_cpu_ms:mainStages.serialize,
      main_validate_cpu_ms:mainStages.validate,main_publish_cpu_ms:mainStages.publish,
      worker_geometry_cpu_ms:rows.filter(row=>row.direction==='worker-to-main'&&row.kind==='apply')
        .reduce((sum,row)=>sum+(Number(row.timing?.worker_call_ms)||0),0),
      round_trips:roundTrips,largest_main_stage:largestMainStage,
      main_method_calls:main,trace_errors:result.worker_trace.errors||[],
      trace_overflow:result.worker_trace.overflow===true,
      scope:'Worker geometry CPU sums reported apply calls; begin/finish CPU omitted and is a remaining admission gap. Incoming transfer classification is unknown unless independently attested.'};
    await writeFile(path.join(out,'worker-transport.json'),JSON.stringify(transport,null,2)+'\n');
  }
  console.log(JSON.stringify({out,status:result.status,failure:result.failure||null,result_sha256:hash(resultBytes),cleanup_errors:result.cleanup_errors}));
}
