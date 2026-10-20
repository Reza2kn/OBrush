// C310b ordinary Source Paint pen-burst pair. Run each side separately and retain
// trusted normal-UI event, public-file, and one-stroke history evidence.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const side = process.argv[2];
if (!['source', 'root'].includes(side)) throw Error('Use source or root');
const home = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const references = path.resolve(process.env.OBRUSH_REFERENCE_ROOT || home);
const { OwnedCdp } = await import(pathToFileURL(path.join(references, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs')));
const { PerfDriver } = await import(pathToFileURL(path.join(references, '.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs')));
const sha = b => createHash('sha256').update(b).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve,ms));
const fixture = path.join(home, 'tests/fixtures/independent-id-sphere-c310.sgl');
const expected = {bytes:687848,sha256:'39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b',vertices:10114,triangles:20224};
const bytes = await readFile(fixture);
if (!expected || bytes.length !== expected.bytes || sha(bytes) !== expected.sha256) throw Error('Common authored fixture changed');
const mirror = path.join(references, '.tools/sculptgl-reference/host-mirror');
const bundle = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (bundle !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') throw Error('Pinned hosted bundle changed');
const web = path.join(home, 'web/dist');
const out = path.join(home, '.artifacts/sculptgl-source-paint-pen-burst-c310b', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {
  side, status:'running', pinned_source_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  fixture:{path:fixture,sha256:sha(bytes),bytes:bytes.length,vertices:expected.vertices,triangles:expected.triangles},
  harness_sha256:sha(await readFile(new URL(import.meta.url))),
  browser_build:side==='source'?bundle:sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm'))),
  cases:[], requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  viewport_measurements:[], notice_acknowledgments:[], acknowledgments:[], failed_acknowledgments:[],
  page_observations:[], cleanup_errors:[],
  tolerances_chosen_before_run:{delivered_pressure_abs:1e-6, imported_position_component_abs:5e-5},
  scope:'Source ordinary Paint and OBrush Source Paint first useful dab plus two same-frame held moves; synthetic pen, paint only'
};
const owner = new OwnedCdp(out,result);
let page,pressed=false;
const cases = [
  {name:'low-high',pointerType:'pen',pressPressure:.25,burstPressures:[.25,.75],radius:0,intensity:1},
  {name:'near-high',pointerType:'pen',pressPressure:.25,burstPressures:[.70,.75],radius:0,intensity:1},
  {name:'factor-zero-low-high',pointerType:'pen',pressPressure:.25,burstPressures:[.25,.75],radius:0,intensity:0},
  {name:'factor-zero-near-high',pointerType:'pen',pressPressure:.25,burstPressures:[.70,.75],radius:0,intensity:0},
];
let observer = `(()=>{const rows=[];const gate=window.__c310={rows,frames:0,hold:false,callbacks:[],release(){this.hold=false;for(const cb of this.callbacks.splice(0)){this.frames++;cb(performance.now());}}};const raf=window.requestAnimationFrame;window.requestAnimationFrame=function(cb){return raf.call(this,t=>{if(gate.hold){gate.callbacks.push(cb);return;}gate.frames++;cb(t);});};for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])window.addEventListener(type,e=>{if(rows.length<4096)rows.push({type,pointerType:e.pointerType,pressure:e.pressure,x:e.clientX,y:e.clientY,buttons:e.buttons,which:e.which,button:e.button,trusted:e.isTrusted,frame:gate.frames,at:performance.now()});},true)})();\n`;
const gpuObserver=await readFile(path.join(home,'scripts',side==='source'?'source-paint-c310-webgl1-colors.js':'source-paint-c310-webgl2-colors.js'),'utf8');
result.gpu_observer_sha256=sha(Buffer.from(gpuObserver));
observer+=gpuObserver+'\n';
if(side==='source')for(const file of ['focused-alpha-input-observer.js','focused-alpha-hit-observer.js'])observer+=await readFile(path.join(references,'.artifacts/sculptgl-large-sustained-c58-stage/observer',file),'utf8')+'\n';
const pen = async (type,x,y,pointerType,pressure,down) => {
  const params={type,x,y,pointerType,force:pressure,button:type==='mouseMoved'?'none':'left',buttons:down?1:0,...(type==='mouseMoved'?{}:{clickCount:1})};
  result.input_trace.push({side,...params}); await page.send('Input.dispatchMouseEvent',params);
};
const gpu = () => page.evaluate(side==='source' ?
  `window.__sourceActiveWords.capture(${expected.vertices})` :
  `window.__sourceGpuReadback.capture(${expected.vertices},true)`);
try {
  const url=await owner.serve(side==='source'?mirror:web,side);
  await owner.start();
  const endpoint=owner.stderr.match(/DevTools listening on (ws:\/\/\S+)/)?.[1];
  const protocol=endpoint?await (await fetch(`http://127.0.0.1:${new URL(endpoint).port}/json/protocol`)).json().catch(()=>null):null;
  result.pen_protocol=protocol?.domains?.find(x=>x.domain==='Input')?.commands?.find(x=>x.name==='dispatchMouseEvent')?.parameters?.filter(x=>['force','pointerType'].includes(x.name))??null;
  if (!result.pen_protocol?.some(x=>x.name==='force') || !result.pen_protocol?.some(x=>x.name==='pointerType'&&x.enum?.includes('pen'))) throw Error('Owned Chrome lacks CDP pen/force');
  page=await owner.page(side,url,observer);
  Object.setPrototypeOf(page,PerfDriver.prototype);
  await page.send('Page.bringToFront');
  if(side==='source')await page.evaluate(`(()=>{const m=window.__sculptglReferenceInstance,rows=window.__c310Hammer=[];m._hammer.on('hammer.input panstart panmove panend',e=>{if(rows.length<4096)rows.push({type:e.type,pointerType:e.pointerType,pointers:e.pointers?.length,center:[e.center?.x,e.center?.y],deltaX:e.deltaX,deltaY:e.deltaY,at:performance.now(),action:m._action,lastNbPointers:m._lastNbPointers});});return true})()`);
  for (const item of cases) {
    const row={...item,status:'running'}; result.cases.push(row);
    row.setup=await page.prepareFixture({path:fixture,vertices:expected.vertices});
    if (side==='source') {
      await page.sourceSelect('_ctrlSculpting',page.controller('_ctrlSculpting')+'._ctrlSculpt.domSelect',8);
      const menu=page.controller('_ctrlSculpting')+'._menu.domUl';
      const nums=`Array.from(${menu}.querySelectorAll('input[type=number]')).filter(e=>e.getBoundingClientRect().width>0)`;
      await page.sourceNumber('_ctrlSculpting',nums+'[0]',64);
      await page.sourceNumber('_ctrlSculpting',nums+'[1]',25);
      for(const [label,value] of [['Roughness',false],['Metallic',false],['Continuous',false],['Symmetry',false]]) await page.sourceCheckbox('_ctrlSculpting',label,value);
      const tab=`window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')`;
      await page.sourceNumber('_ctrlTablet',`${tab}[0]`,item.radius);
      await page.sourceNumber('_ctrlTablet',`${tab}[1]`,item.intensity);
      row.settings=await page.evaluate(`(()=>({radius:window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')[0].value,intensity:window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')[1].value}))()`);
    } else {
      await page.rootControl('brush.paint');
      await page.rootControl('source_paint.mode.source');
      for(const [key,value] of [['source_paint.radius.value',64],['source_paint.intensity.value',.25],['source_paint.color.r.value',1],['source_paint.color.g.value',.766],['source_paint.color.b.value',.336]]) await page.rootNumber(key,value);
      await page.rootCheckbox('source_paint.roughness.enabled',s=>s.source_paint.write_channels.roughness,false);
      await page.rootCheckbox('source_paint.metallic.enabled',s=>s.source_paint.write_channels.metallic,false);
      await page.rootNumber('source_paint.pressure.radius.value',item.radius);
      await page.rootNumber('source_paint.pressure.intensity.value',item.intensity);
      row.settings=(await page.state()).source_paint;
    }
    await page.closeMenus();
    row.before=(await page.save(item.name+'-before')).record;
    row.before_history=(await page.state()).history;
    row.before_gpu=await gpu();
    const v=await page.viewport();
    const x=(v.screen[0]+v.screen[2])/2+80,y=(v.screen[1]+v.screen[3])/2;
    const startX=side==='root'?x+12:x;
    row.requested_path=[[startX,y],...(side==='source'?[12]:[]).map(dx=>[x+dx,y]),[x+24,y],[x+36,y],[x+48,y]];
    const offset=await page.evaluate('window.__c310.rows.length');
    const hammerOffset=side==='source'?await page.evaluate('window.__c310Hammer.length'):null;
    await pen('mouseMoved',startX,y,item.pointerType,item.pressPressure,false);
    await pen('mousePressed',startX,y,item.pointerType,item.pressPressure,true); pressed=true;
    await pause(160);
    row.state_after_press=side==='source'?await page.evaluate('({action:window.__sculptglReferenceInstance._action,mesh:window.__sculptglReferenceInstance.getMesh()?.getNbVertices(),focus:window.__sculptglReferenceInstance._focusGui,lastNbPointers:window.__sculptglReferenceInstance._lastNbPointers})'):await page.evaluate('window.obrushSculptMetrics()');
    if(side==='source'){
      await pen('mouseMoved',x+12,y,item.pointerType,item.pressPressure,true);
      await pause(20);
    }
    await pen('mouseMoved',x+24,y,item.pointerType,item.pressPressure,true);
    await pause(20);
    row.state_after_first_useful_move=side==='source'?await page.evaluate('({action:window.__sculptglReferenceInstance._action,lastNbPointers:window.__sculptglReferenceInstance._lastNbPointers})'):await page.evaluate('window.obrushSculptMetrics()');
    await page.evaluate('window.__c310.hold=true');
    const burstOffset=await page.evaluate('window.__c310.rows.length');
    try{
      await pen('mouseMoved',x+36,y,item.pointerType,item.burstPressures[0],true);
      await pen('mouseMoved',x+48,y,item.pointerType,item.burstPressures[1],true);
    }finally{await page.evaluate('window.__c310.release()');}
    await pause(160);
    row.burst_dom_events=await page.evaluate(`window.__c310.rows.slice(${burstOffset})`);
    const delivered=row.burst_dom_events.filter(e=>e.type==='pointermove'&&e.pointerType==='pen'&&e.trusted&&e.buttons===1);
    if(delivered.length!==2||delivered[0].frame!==delivered[1].frame||delivered.some((e,i)=>Math.abs(e.pressure-item.burstPressures[i])>1e-6))
      throw Error(`Two trusted same-frame held pressures not delivered: ${item.name} ${JSON.stringify(delivered)}`);
    row.state_after_burst=side==='source'?await page.evaluate('({action:window.__sculptglReferenceInstance._action,lastNbPointers:window.__sculptglReferenceInstance._lastNbPointers})'):await page.evaluate('window.obrushSculptMetrics()');
    await pen('mouseReleased',x+48,y,item.pointerType,0,false); pressed=false;
    for(let i=0;i<32;i++)await page.raf();
    row.state_after_release=side==='root'?await page.evaluate('window.obrushSculptMetrics()'):null;
    row.dom_events=await page.evaluate(`window.__c310.rows.slice(${offset})`);
    if(side==='source')row.hammer_events=await page.evaluate(`window.__c310Hammer.slice(${hammerOffset})`);
    if (!row.dom_events.some(e=>e.type==='pointerdown'&&e.pointerType===item.pointerType&&e.trusted&&Math.abs(e.pressure-item.pressPressure)<1e-6)) throw Error('Trusted declared pointer pressure missing '+item.name);
    row.after=(await page.save(item.name+'-after')).record;
    row.after_history=(await page.state()).history;
    row.after_gpu=await gpu();
    await page.chord('z'); row.undo=(await page.save(item.name+'-undo')).record; row.undo_gpu=await gpu();
    await page.chord('z',true); row.redo=(await page.save(item.name+'-redo')).record; row.redo_gpu=await gpu();
    row.status='complete public save and one-stroke history capture';
  }
  if(side==='root'){
    const row={name:'overflow-257',status:'running'};result.overflow=row;
    row.setup=await page.prepareFixture({path:fixture,vertices:expected.vertices});
    await page.rootControl('brush.paint');
    await page.rootControl('source_paint.mode.source');
    for(const [key,value] of [['source_paint.radius.value',64],['source_paint.intensity.value',.25],['source_paint.color.r.value',1],['source_paint.color.g.value',.766],['source_paint.color.b.value',.336]]) await page.rootNumber(key,value);
    await page.rootCheckbox('source_paint.roughness.enabled',s=>s.source_paint.write_channels.roughness,false);
    await page.rootCheckbox('source_paint.metallic.enabled',s=>s.source_paint.write_channels.metallic,false);
    await page.rootNumber('source_paint.pressure.radius.value',0);
    await page.rootNumber('source_paint.pressure.intensity.value',1);
    await page.closeMenus();
    row.before=(await page.save('overflow-before')).record;
    row.before_history=(await page.state()).history;
    const v=await page.viewport(),x=(v.screen[0]+v.screen[2])/2+80,y=(v.screen[1]+v.screen[3])/2;
    const offset=await page.evaluate('window.__c310.rows.length');
    await pen('mouseMoved',x+12,y,'pen',.25,false);
    await pen('mousePressed',x+12,y,'pen',.25,true);pressed=true;
    await pause(160);
    await pen('mouseMoved',x+24,y,'pen',.25,true);
    await pause(20);
    await page.evaluate('window.__c310.hold=true');
    try{
      for(let index=0;index<257;index++)
        await pen('mouseMoved',x+36+(index%100),y,'pen',index%2?.75:.25,true);
    }finally{await page.evaluate('window.__c310.release()');}
    await pause(160);
    row.before_release_metrics=await page.evaluate('window.obrushSculptMetrics()');
    await pen('mouseReleased',x+92,y,'pen',0,false);pressed=false;
    for(let i=0;i<32;i++)await page.raf();
    row.dom_events=await page.evaluate(`window.__c310.rows.slice(${offset})`);
    row.after=(await page.save('overflow-after')).record;
    row.after_metrics=await page.evaluate('window.obrushSculptMetrics()');
    row.after_history=(await page.state()).history;
    row.status='complete bounded overflow and public after-save capture';
  }
  result.status='complete four-case same-frame Source Paint pen capture; independent public-file audit pending';
} catch(error) {
  result.status='FAILED; preserve partial evidence'; result.failure=String(error.stack??error);process.exitCode=1;
} finally {
  if(pressed&&page)try{const v=await page.viewport();await pen('mouseReleased',(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2,'pen',0,false);}catch(error){result.cleanup_errors.push(String(error));}
  try{await owner.close();}catch(error){result.cleanup_errors.push(String(error));process.exitCode=1;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,cleanup_errors:result.cleanup_errors}));
  process.exit(process.exitCode??0);
}
