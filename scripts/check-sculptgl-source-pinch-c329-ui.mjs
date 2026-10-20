// C329 ordinary-browser Pinch press/move completion and public press control.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const shared = path.resolve(process.env.OBRUSH_SHARED_ROOT ?? root);
const { OwnedCdp } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')).href);
const { PerfDriver } = await import(pathToFileURL(path.join(shared,'.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')).href);
const side = process.argv.find(a => a.startsWith('--side='))?.slice(7);
if (!['source', 'root'].includes(side)) throw Error('Use --side=source or --side=root');
const polarity = process.argv.find(a => a.startsWith('--negative='))?.slice(11);
if (!['off','on'].includes(polarity)) throw Error('Use --negative=off or --negative=on');
const negative = polarity === 'on';
const mode = process.argv.find(a=>a.startsWith('--mode='))?.slice(7);
if (!['press-only','press-move'].includes(mode)) throw Error('Use --mode=press-only or --mode=press-move');
const pressOnly = mode === 'press-only';
const radius = 50;
const intensity = 0.75;
const accumulate = process.argv.includes('--accumulate');
if (!Number.isFinite(radius) || radius < 5 || radius > 500 ||
    !Number.isFinite(intensity) || intensity < 0 || intensity > 1) {
  throw Error('Use a finite --radius=5..500 and --intensity=0..1');
}
if (accumulate) throw Error('C329 requires Pinch accumulation off');
const matchSourcePath = process.argv.find(a=>a.startsWith('--match-source='))?.slice('--match-source='.length);
if (matchSourcePath && side !== 'root') throw Error('Source-ray matching applies to root side');
if (side==='root' && !matchSourcePath) throw Error('C329 root run requires --match-source=<same-polarity/mode-source-result.json>');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const fixture = path.join(root, 'tests/fixtures/independent-source-pinch-c308.sgl');
const expected = JSON.parse(await readFile(path.join(root, 'tests/fixtures/independent-source-pinch-c308.json')));
const fixtureBytes = await readFile(fixture);
if (!expected || fixtureBytes.length !== expected.bytes || sha(fixtureBytes) !== expected.sha256) {
  throw Error('Independent C308 fixture identity changed');
}
if (sha(fixtureBytes) !== 'f864eb78bf6f280c92fcaa3debcaa406c9839a82e861d213e396233ac6269144')
  throw Error('Preregistered fixture SHA-256 changed');
const web = path.resolve(process.env.OBRUSH_WEB_DIST ?? path.join(root, 'web/dist'));
const webUrl = process.env.OBRUSH_WEB_URL ?? null;
const webRepositoryRoot = path.resolve(web, '../..');
const mirror = path.join(shared, '.tools/sculptgl-reference/host-mirror');
const pinned = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (pinned !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') {
  throw Error('Pinned SculptGL bundle changed');
}
const out = path.join(root, '.artifacts/sculptgl-source-pinch-c329-ui', `${side}-${polarity}-${mode}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {side, negative, protocol_commit:'c329-pinch-contact-calibrated',
  obrush_source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
  served_web_repository_head:side==='root'?execFileSync('git',['rev-parse','HEAD'],{cwd:webRepositoryRoot,encoding:'utf8'}).trim():null,
  served_web_dist:web,served_web_url:side==='root'?webUrl:null,
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  reference_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  stroke_mode:mode, status:'running', fixture:{path:fixture, bytes:fixtureBytes.length, sha256:sha(fixtureBytes), vertices:expected.vertices, triangles:expected.triangles},
  requested_brush:{radius,intensity,accumulate,negative},
  browser_build:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm'))) : pinned,
  browser_js:side === 'root' ? sha(await readFile(path.join(web,'pkg/obrush_app.js'))) : null,
  browser_simd_build:side === 'root' ? sha(await readFile(path.join(web,'pkg-simd/obrush_app_bg.wasm'))) : null,
  browser_simd_js:side === 'root' ? sha(await readFile(path.join(web,'pkg-simd/obrush_app.js'))) : null,
  requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  viewport_measurements:[], notice_acknowledgments:[], acknowledgments:[], failed_acknowledgments:[],
  page_observations:[], cleanup_errors:[], protocol_failures:[]};
const owner = new OwnedCdp(out, result);
let page, pressed = false;
async function installSourcePinchTrace(){
  if(side!=='source')return;
  const installed=await page.evaluate(`(()=>{
    const manager=window.__sculptglReferenceInstance.getSculptManager();
    if(manager.getToolIndex()!==5)throw Error('C329 selected tool is not Pinch');
    const tool=manager.getCurrentTool();
    if(window.__c329PinchTrace || typeof tool.stroke!=='function')throw Error('C329 Pinch trace already installed or unavailable');
    const original=tool.stroke,trace={phase:'setup',rows:[],errors:[]};
    tool.stroke=function(...args){
      const picking=args[0],before={phase:trace.phase,at:performance.now(),
        center:Array.from(picking.getIntersectionPoint()),
        radius_squared:picking.getLocalRadius2(),
        picked_vertices:picking.getPickedVertices()?.length??null};
      try{const result=Reflect.apply(original,this,args);trace.rows.push({...before,completed:true});return result;}
      catch(error){trace.errors.push({...before,error:String(error)});throw error;}
    };
    window.__c329PinchTrace=trace;
    return {tool_index:manager.getToolIndex(),pass_through:true};
  })()`);
  result.source_trace_install=installed;
}
async function contactSnapshot(label){
  const data=side==='source' ? await page.evaluate(`(()=>{const t=window.__c329PinchTrace;return {count:t?.rows.length??null,rows:t?.rows??null,errors:t?.errors??null,phase:t?.phase??null};})()`) :
    await page.evaluate('window.obrushSculptMetrics()');
  result.contact_snapshots??=[];
  const row={label,at:Date.now(),data};result.contact_snapshots.push(row);return row;
}
async function completedCount(){
  const s=await contactSnapshot('count probe');
  return side==='source'?s.data.count:s.data.applied_dabs-result.initial_applied_dabs;
}
async function awaitSettledContacts(label,target){
  const started=Date.now();let stable=0,firstStable=0,previous=null;
  for(let i=0;i<120;i++){
    const count=await completedCount();
    const latest=result.contact_snapshots.at(-1).data;
    const drained=side==='source'||(latest.pending_dabs===0&&latest.browser_pen_moves_queued===0);
    if(count===previous&&drained){if(stable===0)firstStable=Date.now();stable++;}
    else stable=0;
    previous=count;
    if(stable>=12&&Date.now()-firstStable>=250){
      const admission={label,target,count,pass:count===target && !(latest.errors?.length),
        probes:i+1,elapsed_ms:Date.now()-started,drained};
      result.contact_admissions??=[];result.contact_admissions.push(admission);
      if(!admission.pass)result.protocol_failures.push(`C329 ${label}: ${side} completed ${count} dabs, expected ${target}`);
      return latest;
    }
    await page.raf();await pause(25);
  }
  const failure=`C329 ${label}: ${side} completed contact count/queue did not stabilize`;
  result.protocol_failures.push(failure);
  return result.contact_snapshots.at(-1)?.data??null;
}
async function alignActualSourceCanvas() {
  const observations=[];
  for(let i=0;i<8;i++){
    await page.raf();
    const v=await page.viewport();
    observations.push({canvas:v.canvas,rect:v.rect,pixels:v.pixels,window:v.window});
    const css=[v.rect[2]-v.rect[0],v.rect[3]-v.rect[1]];
    if(JSON.stringify(v.canvas)==='[1024,768]' && JSON.stringify(css)==='[1024,768]'){
      result.actual_canvas_alignment={observations,pass:true};return v;
    }
    const current=await owner.send('Browser.getWindowBounds',{windowId:page.windowId});
    const width=current.bounds.width+1024-v.canvas[0],height=current.bounds.height+768-v.canvas[1];
    if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<900||width>2300||height<600||height>1800)
      throw Error('C329 bounded actual Source canvas resize failed');
    await owner.send('Browser.setWindowBounds',{windowId:page.windowId,bounds:{width,height,windowState:'normal'}});
  }
  result.actual_canvas_alignment={observations,pass:false};
  throw Error('C329 actual 1024x768 Source canvas not reached');
}
async function stableCamera(label) {
  const deadline = Date.now() + 5000;
  let signature = null, since = 0, frames = 0, first = null, latest = null, samples = 0;
  while (Date.now() < deadline) {
    const camera = side === 'source' ? await page.evaluate(`(()=>{const c=window.__sculptglReferenceInstance.getCamera();return {
      center:Array.from(c._center),translation:Array.from(c._trans),projection:Array.from(c._proj),
      near:c._near,width:c._width,height:c._height,
      timer_entries:Object.entries(c._timers).map(([name,value])=>({name,value}))};})()`) :
      (await page.live()).state.camera;
    const terminal = side === 'source' ? camera?.timer_entries.every(({value})=>value===0||value===-1||value===null) :
      camera?.source_framing?.error == null && camera?.source_scale_pending == null;
    const next = terminal ? JSON.stringify(side === 'source' ?
      [camera.center,camera.translation,camera.projection,camera.near,camera.width,camera.height] :
      [camera.precise_target,camera.position64,camera.projection,
       camera.source_framing?.near,camera.source_framing?.projection_zoom,camera.viewport]) : null;
    samples++;
    if (next !== null && next === signature) {
      frames++; latest = camera;
      if (frames >= 12 && Date.now() - since >= 250) {
        result.stability ??= {};
        result.stability[label] = {first,latest,samples,frames,elapsed_ms:Date.now()-since,signature,terminal:true};
        return camera;
      }
    } else { signature=next; since=Date.now(); frames=0; first=camera; latest=camera; }
    await page.raf(); await pause(25);
  }
  result.stability ??= {};
  result.stability[label] = {first,latest,samples,frames,elapsed_ms:Date.now()-since,signature,terminal:false};
  throw Error(`C329 ${label} camera failed 12-frame/250ms stability in 5s`);
}
async function prepareSourcePinchFixture(page) {
  await alignActualSourceCanvas();
  for (const [label,value] of [['Scale and center',false],['Export all',true],['sRGB vertex color',true]])
    await page.sourceCheckbox('_ctrlFiles',label,value);
  await page.clear();
  await page.file('#fileopen',fixture);
  await page.wait(()=>page.state(),s=>s.meshes.length===1&&s.meshes[0].vertices===expected.vertices,
    'Complete independent Source fixture import');
  await page.sourceSelect('_ctrlSculpting',page.controller('_ctrlSculpting')+'._ctrlSculpt.domSelect',5);
  const menu=page.controller('_ctrlSculpting')+'._menu.domUl';
  const nums=`Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
  await page.sourceNumber('_ctrlSculpting',nums+'[0]',50);
  await page.sourceNumber('_ctrlSculpting',nums+'[1]',75);
  for(const [label,value] of [['Negative',false],
    ['Thin surface',false],['Symmetry',false],['Continuous',false]])
    await page.sourceCheckbox('_ctrlSculpting',label,value);
  const tablet=page.controller('_ctrlTablet')+'._menu.domUl.querySelectorAll("input[type=number]")';
  await page.sourceNumber('_ctrlTablet',tablet+'[0]',0);
  await page.sourceNumber('_ctrlTablet',tablet+'[1]',0);
  const alpha=await page.evaluate(`(()=>{const menu=${menu},e=Array.from(menu.querySelectorAll('select')).find(e=>Array.from(e.options).some(o=>o.value==='None'));return {value:e?.value,owner:window.__sculptglReferenceInstance.getSculptManager().getCurrentTool()._idAlpha};})()`);
  result.source_alpha_initial=alpha;
  if(alpha.value!=='None'||![0,'None'].includes(alpha.owner))throw Error('Source Alpha None was not published');
  await page.sourceSelect('_ctrlCamera',page.controller('_ctrlCamera')+'._ctrlProjection.domSelect',1);
  await page.sourceButton('_ctrlCamera','Reset (bar)');
  await page.closeMenus();
  const viewport=await page.viewport();
  const point=[viewport.screen[0]+16,viewport.screen[1]+viewport.pixels[3]-16];
  await page.pointer('mouseMoved',...point);
  if(await page.evaluate('window.__sculptglReferenceInstance._focusGui!==false'))
    throw Error('Actual Source canvas does not own fit gesture');
  for(let tap=0;tap<2;tap++){
    await page.pointer('mousePressed',...point,'left',1,{clickCount:tap+1});
    await page.pointer('mouseReleased',...point,'left',0,{clickCount:tap+1});
  }
  await page.wait(()=>page.evaluate('Object.values(window.__sculptglReferenceInstance.getCamera()._timers).every(v=>v===0||v===-1||v===null)'),
    v=>v,'Source fit timers settle');
  await page.closeMenus();
  return {state:await page.state(),viewport:await page.viewport()};
}
try {
  if (side==='root' && webUrl) {
    result.served_http_hashes={};
    for(const [name,localFile] of [['pkg/obrush_app_bg.wasm','pkg/obrush_app_bg.wasm'],
      ['pkg/obrush_app.js','pkg/obrush_app.js']]){
      const response=await fetch(new URL(name,webUrl),{cache:'no-store'});
      if(!response.ok)throw Error(`Existing OBrush web server returned ${response.status} for ${name}`);
      const actual=sha(Buffer.from(await response.arrayBuffer()));
      const expected=sha(await readFile(path.join(web,localFile)));
      result.served_http_hashes[name]={actual,expected,exact:actual===expected};
      if(actual!==expected)throw Error(`Existing OBrush web server serves stale ${name}`);
    }
  }
  const url = side==='root' && webUrl ? webUrl : await owner.serve(side === 'source' ? mirror : web, side);
  await owner.start();
  const stage = path.join(shared, '.artifacts/sculptgl-large-sustained-c58-stage');
  const pointerObserver = `(()=>{const rows=[];window.__uiBrushDelivered=rows;for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])window.addEventListener(type,e=>{if(rows.length<2048)rows.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons,trusted:e.isTrusted,at:performance.now()});},true);})();\n`;
  const gpuObserver = side === 'source' ? await readFile(path.join(root,'scripts/source-webgl1-active-words-observer.js'),'utf8') :
    await readFile(path.join(root,'scripts/brush-negative-gpu-readback-observer.js'),'utf8');
  result.gpu_observer_sha256=sha(Buffer.from(gpuObserver));
  const observer = pointerObserver + gpuObserver + '\n' + (side === 'source' ?
    (await readFile(path.join(stage, 'observer/focused-alpha-input-observer.js'),'utf8')) + '\n' +
    (await readFile(path.join(stage, 'observer/focused-alpha-hit-observer.js'),'utf8')) : '');
  page = await owner.page(side, url, observer);
  Object.setPrototypeOf(page, PerfDriver.prototype);
  await page.send('Page.bringToFront');
  result.setup = side==='source' ? await prepareSourcePinchFixture(page) :
    await page.prepareFixture({path:fixture, vertices:expected.vertices});
  if (side === 'root') {
    await page.rootControl('brush.pinch');
    await page.rootControl('source_geometry.mode.source');
  }
  await stableCamera('after_fit');
  result.setup = {state:await page.state(),viewport:await page.viewport()};
  const sourceCanvas=result.setup.viewport.canvas;
  const sourceCss=[result.setup.viewport.rect[2]-result.setup.viewport.rect[0],result.setup.viewport.rect[3]-result.setup.viewport.rect[1]];
  const rootSize=result.setup.viewport.logical_size;
  if (side==='source' ?
      JSON.stringify(sourceCanvas)!=='[1024,768]' || JSON.stringify(sourceCss)!=='[1024,768]' :
      JSON.stringify(rootSize)!=='[1024,768]')
    throw Error('C329 actual 1024x768 sculpt viewport was not reached');
  if (side === 'source') {
    const menu = 'window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl';
    await page.sourceMenu('_ctrlSculpting');
    const negativeControl = `Array.from(${menu}.querySelectorAll('input[type=checkbox]')).find(e=>e.parentElement.textContent.includes('Negative')&&e.parentElement.getBoundingClientRect().width>0)`;
    const negativeLabel = `(${negativeControl}).nextElementSibling`;
    const negativeBefore = await page.evaluate(`(()=>{const e=${negativeControl},r=e?.parentElement.getBoundingClientRect();return {checked:e?.checked,disabled:e?.disabled,rect:r?[r.left,r.top,r.right,r.bottom]:null};})()`);
    if (!negativeBefore.rect || negativeBefore.disabled) throw Error('Visible Source Negative checkbox unavailable');
    let hit;
    for(let i=0;i<120;i++){
      hit=await page.evaluate(`(()=>{const e=${negativeLabel},r=e?.getBoundingClientRect(),x=r&&(r.left+r.right)/2,y=r&&(r.top+r.bottom)/2,h=r&&document.elementFromPoint(x,y);return {tag:e?.tagName,rect:r?[r.left,r.top,r.right,r.bottom]:null,hit:h?.outerHTML?.slice(0,256),ready:!!r&&(h===e||e.contains(h))};})()`);
      if(hit.ready)break;
      await page.raf();
    }
    result.ui_actions.push({side:'source',checkbox:'Negative',negativeBefore,hit});
    if(!hit?.ready || hit.tag!=='LABEL')throw Error('Visible Source Negative label not actual hit target');
    if (negativeBefore.checked !== negative) await page.sourceClick(negativeLabel);
    const negativeAfter = await page.wait(()=>page.evaluate(`(${negativeControl}).checked`),v=>v===negative,'Visible Source Negative published');
    result.ui_actions.push({side:'source',checkbox:'Negative',negativeAfter});
    if (negativeAfter !== negative) throw Error('Source Negative did not publish');
  } else if (!matchSourcePath) {
    await page.rootNumber('source_geometry.radius.value',radius);
    await page.rootNumber('source_geometry.intensity.value',intensity);
  }
  if (side === 'root') {
    await page.rootNumber('source_geometry.pressure.radius.value',0);
    await page.rootNumber('source_geometry.pressure.intensity.value',0);
    await page.rootCheckbox('source_geometry.prevent_folds',s=>s.source_geometry.per_tool[3].options.prevent_folds,false);
    await page.rootCheckbox('source_geometry.lock_position',s=>s.lock_position,false);
  }
  if (side === 'root') await page.rootCheckbox('source_geometry.negative',s=>s.source_geometry.per_tool[3].options.negative,negative);
  if (matchSourcePath) {
    const sourceRun = JSON.parse(await readFile(matchSourcePath));
    if (sourceRun.side !== 'source' || sourceRun.fixture.sha256 !== result.fixture.sha256 ||
        sourceRun.stroke_mode !== result.stroke_mode ||
        (sourceRun.requested_brush?.radius??50) !== radius ||
        (sourceRun.requested_brush?.intensity??0.75) !== intensity ||
        (sourceRun.requested_brush?.accumulate??false) !== accumulate || sourceRun.negative !== negative) throw Error('Source ray-binding run mismatch');
    const sourceBrush=sourceRun.config?.state?.tool;
    if (!sourceBrush || !Number.isFinite(sourceBrush._radius) || !Number.isFinite(sourceBrush._intensity)) throw Error('Source published Pinch settings missing');
    const sourceProjectionX = sourceRun.setup.state.camera.projection[0];
    const sourceProjectionY = sourceRun.setup.state.camera.projection[5];
    const rootProjectionX = result.setup.state.camera.projection[0];
    const rootProjectionY = result.setup.state.camera.projection[5];
    const sourceWidth = sourceRun.setup.viewport.screen[2]-sourceRun.setup.viewport.screen[0];
    const rootWidth = result.setup.viewport.screen[2]-result.setup.viewport.screen[0];
    const sourceHeight = sourceRun.setup.viewport.screen[3]-sourceRun.setup.viewport.screen[1];
    const rootHeight = result.setup.viewport.screen[3]-result.setup.viewport.screen[1];
    const offsetScale = rootProjectionX/sourceProjectionX*rootWidth/sourceWidth;
    const verticalScale = rootProjectionY/sourceProjectionY*rootHeight/sourceHeight;
    const actualSourceCanvas=sourceRun.setup.viewport.canvas;
    if (JSON.stringify(actualSourceCanvas)!=='[1024,768]' ||
        ![sourceProjectionX,sourceProjectionY,rootProjectionX,rootProjectionY,sourceWidth,rootWidth,sourceHeight,rootHeight,offsetScale,verticalScale].every(Number.isFinite) ||
        offsetScale<0.5 || offsetScale>2 || verticalScale<0.5 || verticalScale>2)
      throw Error('C329 actual two-axis source ray calibration failed');
    const sourceWorldRadius=2*sourceBrush._radius/(sourceWidth*sourceProjectionX);
    const rootRadiusRequested=sourceBrush._radius*offsetScale;
    const rootWorldRadius=2*rootRadiusRequested/(rootWidth*rootProjectionX);
    const sourceWorldRadiusY=2*sourceBrush._radius/(sourceHeight*sourceProjectionY);
    const rootWorldRadiusY=2*rootRadiusRequested/(rootHeight*rootProjectionY);
    const sampleOffsets=[10,17.5,25];
    const rayComparisons=sampleOffsets.map(v=>({source_x:2*v/(sourceWidth*sourceProjectionX),
      root_x:2*v*offsetScale/(rootWidth*rootProjectionX),
      source_y:2*v/(sourceHeight*sourceProjectionY),
      root_y:2*v*offsetScale/(rootHeight*rootProjectionY)}));
    const maxRayDifference=Math.max(...rayComparisons.flatMap(v=>[
      Math.abs(v.source_x-v.root_x),Math.abs(v.source_y-v.root_y)]));
    if (Math.abs(sourceWorldRadius-rootWorldRadius)>1e-8 ||
        Math.abs(sourceWorldRadiusY-rootWorldRadiusY)>1e-8 || maxRayDifference>1e-8)
      throw Error('C329 actual world ray/radius bound failed');
    result.ray_alignment = {source_result:matchSourcePath,source_projection_x:sourceProjectionX,
      source_projection_y:sourceProjectionY,root_projection_x:rootProjectionX,root_projection_y:rootProjectionY,
      source_width:sourceWidth,root_width:rootWidth,source_height:sourceHeight,root_height:rootHeight,
      offset_scale:offsetScale,vertical_scale:verticalScale,ray_comparisons:rayComparisons,
      max_ray_difference:maxRayDifference,source_world_radius:sourceWorldRadius,root_world_radius:rootWorldRadius,
      world_radius_abs_difference:Math.abs(sourceWorldRadius-rootWorldRadius),
      source_world_radius_y:sourceWorldRadiusY,root_world_radius_y:rootWorldRadiusY,
      world_radius_y_abs_difference:Math.abs(sourceWorldRadiusY-rootWorldRadiusY),
      source_published_radius:sourceBrush._radius,source_published_intensity:sourceBrush._intensity,
      radius_requested:rootRadiusRequested,scope:'Actual two-axis projection calibrated physical rays/radius; raw projection matrices retained; not proof of delivered contacts'};
    await page.rootNumber('source_geometry.radius.value',rootRadiusRequested);
    await page.rootNumber('source_geometry.intensity.value',sourceBrush._intensity);
  }
  await page.closeMenus();
  result.config = await page.live();
  const p = side === 'source' ? result.config.state.tool : result.config.state.source_geometry.per_tool[3];
  if (side === 'source') {
    const tablet = page.controller('_ctrlTablet') + '._menu.domUl.querySelectorAll("input[type=number]")';
    result.source_tablet_controls = await page.evaluate(`Array.from(${tablet}).map(e=>Number(e.value))`);
    if (result.source_tablet_controls.slice(0,2).some(v=>v!==0)) throw Error('Source pressure factors are not zero');
  } else if (p.radius_factor!==0 || p.options.intensity_factor!==0) {
    throw Error('OBRush pressure factors are not zero');
  }
  if (side === 'source' ? !(result.config.state.tool_index===5 && p._negative===negative &&
    !p._culling && !p._lockPosition && [0,'None'].includes(p._idAlpha) &&
    !result.config.state.symmetry && !result.config.state.continuous) :
    !(result.config.state.kind==='Pinch' && p.mode==='SourceOrdinary' &&
      p.options.tool==='Pinch' && p.options.negative===negative &&
      !p.options.culling && !p.options.prevent_folds &&
      !result.config.state.lock_position && !result.config.state.symmetry_x &&
      !result.config.state.continuous.requested &&
      (result.config.state.alpha===null || result.config.state.alpha==='None'))) {
    throw Error('Normal UI Pinch settings differ from admitted comparison');
  }
  if (side === 'source' ? p._radius!==radius || p._intensity!==intensity :
    Math.abs(p.radius-(result.ray_alignment?.radius_requested??radius))>1e-6 ||
    Math.abs(p.options.intensity-(result.ray_alignment?.source_published_intensity??intensity))>1e-6) throw Error('Requested Pinch radius/intensity did not publish');
  await installSourcePinchTrace();
  result.before = (await page.save('before')).record;
  result.before_history = side==='source' ?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()') :
    (await page.live()).state.history;
  if(side==='root')result.before_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.before_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  result.initial_applied_dabs=side==='root'?(await page.evaluate('window.obrushSculptMetrics().applied_dabs')):null;
  await contactSnapshot('before input');
  const viewport = await page.viewport();
  const center = [(viewport.screen[0]+viewport.screen[2])/2,(viewport.screen[1]+viewport.screen[3])/2];
  const scale=result.ray_alignment?.offset_scale??1;
  const pathPixels = [[center[0]+10*scale,center[1]],[center[0]+25*scale,center[1]]];
  result.requested_path = pressOnly?pathPixels.slice(0,1):pathPixels;
  const deliveredAt = await page.evaluate('window.__uiBrushDelivered?.length??null');
  const pickExpression=`(()=>{const p=window.__sculptglReferenceInstance.getPicking();return {
    face:p.getPickedFace(),point:Array.from(p.getIntersectionPoint()),
    radius_squared:p.getLocalRadius2(),picked_vertices:p.getPickedVertices()?.length??null};})()`;
  await page.pointer('mouseMoved',...pathPixels[0]);
  if(side==='source')await page.evaluate(`window.__c329PinchTrace.phase='press'`);
  await page.pointer('mousePressed',...pathPixels[0],'left',1,{force:1}); pressed = true;
  await awaitSettledContacts('press held',0);
  result.press_pick = side==='source' ? await page.evaluate(pickExpression) :
    (await page.live()).state.source_geometry.active?.last_pass;
  if(!pressOnly){
    if(side==='source')await page.evaluate(`window.__c329PinchTrace.phase='move'`);
    for (const point of pathPixels.slice(1)) await page.pointer('mouseMoved',...point,'left',1,{force:1});
    await awaitSettledContacts('held move',2);
    result.move_pick = side==='source' ? await page.evaluate(pickExpression) :
      (await page.live()).state.source_geometry.active?.last_pass;
  }
  if(side==='source')await page.evaluate(`window.__c329PinchTrace.phase='release'`);
  await page.pointer('mouseReleased',...(pressOnly?pathPixels[0]:pathPixels.at(-1)),'left',0); pressed = false;
  await awaitSettledContacts('released',pressOnly?0:2);
  result.dom_stroke_events = await page.evaluate(`window.__uiBrushDelivered?.slice(${deliveredAt})??null`);
  if (!result.dom_stroke_events?.length || result.dom_stroke_events.some(e=>!e.trusted)) throw Error('Trusted normal DOM stroke delivery missing');
  const expectedInput=pressOnly?['pointerdown','pointerup']:['pointerdown','pointermove','pointerup'];
  for(const type of expectedInput)if(!result.dom_stroke_events.some(e=>e.type===type&&e.trusted))
    throw Error(`C329 trusted normal ${type} was not delivered`);
  result.release_state = side === 'root' ? await page.evaluate('window.obrushSculptMetrics()') :
    await page.evaluate('({timer:window.__sculptglReferenceInstance.getSculptManager()._sculptTimer,action:window.__sculptglReferenceInstance._action})');
  result.after_history = side==='source' ?
    await page.evaluate('(()=>{const s=window.__sculptglReferenceInstance.getStateManager();return {undo:s._curUndoIndex+1,redo:s._redos.length};})()') :
    (await page.live()).state.history;
  result.after = (await page.save(pressOnly?'after-press':'after')).record;
  if(side==='root')result.after_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.after_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  if(!pressOnly){
  await page.chord('z'); result.undo = (await page.save('undo')).record;
  if(side==='root')result.undo_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.undo_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  await page.chord('z',true); result.redo = (await page.save('redo')).record;
  if(side==='root')result.redo_object_identity=await page.evaluate('(()=>{const s=window.obrushBrushMetrics();return {active_subtool_id:s.active_subtool_id,selected_subtool_ids:s.selected_subtool_ids,subtools:s.subtools.map(x=>({id:x.id,vertices:x.vertices,triangles:x.triangles,visible:x.visible})),vertices:s.vertices};})()');
  result.redo_gpu = await page.evaluate(side === 'source' ?
    `window.__sourceActiveWords.capture(${expected.vertices})` :
    `window.__sourceGpuReadback.capture(${expected.vertices}, true)`);
  }
  if(side==='source'){
    result.source_contact_trace=await page.evaluate('({rows:window.__c329PinchTrace.rows,errors:window.__c329PinchTrace.errors})');
    if(result.source_contact_trace.errors.length)result.protocol_failures.push('C329 Source Pinch stroke wrapper recorded errors');
    if(result.source_contact_trace.rows.some(row=>row.phase!=='move') ||
       result.source_contact_trace.rows.length!==(pressOnly?0:2))
      result.protocol_failures.push('C329 Source completed stroke phase/count mismatch');
  }else{
    const metrics=await page.evaluate('window.obrushSculptMetrics()');
    result.final_contact_metrics=metrics;
    if(metrics.browser_pen_moves_dropped!==0 || metrics.browser_pen_moves_overflowed!==false ||
       metrics.pending_dabs!==0 || metrics.browser_pen_moves_queued!==0 ||
       metrics.applied_dabs-result.initial_applied_dabs!==(pressOnly?0:2))
      result.protocol_failures.push('C329 OBrush completed contact/drop/queue gate failed');
  }
  result.status = result.protocol_failures.length ? 'FAILED C329 contact protocol; public evidence retained' : pressOnly ?
    'normal browser UI press-only control and public Before/After captured; paired audit pending' :
    'normal browser UI two-dab stroke and public Before/After/Undo/Redo captured; paired audit pending';
  if(result.protocol_failures.length)process.exitCode=1;
} catch (error) {
  result.status = 'FAILED normal browser UI stroke; evidence retained';
  result.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  if (pressed && page) try { const v=await page.viewport(); await page.pointer('mouseReleased',(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2,'left',0); }
  catch(error) { result.cleanup_errors.push(String(error)); }
  try { await owner.close(); } catch(error) { result.cleanup_errors.push(String(error)); process.exitCode=1; }
  await writeFile(path.join(out,'result.json'), JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,cleanup_errors:result.cleanup_errors}));
}
