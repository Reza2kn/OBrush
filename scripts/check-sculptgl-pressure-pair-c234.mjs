// Five independent, normal-browser pressure cases per side on one authored SGL.
// Run one side at a time: node scripts/check-sculptgl-pressure-pair-c234.mjs source|root
// Public save bytes and actual trusted pointer delivery are retained. No app state
// is changed through Runtime.evaluate; product controls use real CDP input.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import path from 'node:path';
import { OwnedCdp } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/paired-driver.mjs';
import { PerfDriver } from '../.artifacts/sculptgl-large-sustained-c58-stage/workspace/perf-driver.mjs';

const side = process.argv[2];
if (!['source', 'root'].includes(side)) throw Error('Use source or root');
const home = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const sha = b => createHash('sha256').update(b).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve,ms));
const fixture = path.join(home, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/fixtures/independent-id-sphere-10k.sgl');
const manifest = JSON.parse(await readFile(path.join(home, '.artifacts/sculptgl-sparse-guard-c69-stage/bench/FIXTURE_MANIFEST.json')));
const expected = manifest.files.find(x => x.label === '10k');
const bytes = await readFile(fixture);
if (!expected || bytes.length !== expected.bytes || sha(bytes) !== expected.sha256) throw Error('Common authored fixture changed');
const mirror = path.join(home, '.tools/sculptgl-reference/host-mirror');
const bundle = sha(await readFile(path.join(mirror, 'sculptgl.js')));
if (bundle !== '79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600') throw Error('Pinned hosted bundle changed');
const web = path.join(home, 'web/dist');
const out = path.join(home, '.artifacts/sculptgl-pressure-pair-c234', `${side}-${randomUUID()}`);
await mkdir(out, { recursive: true });
const result = {
  side, status:'running', pinned_source_commit:'8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
  fixture:{path:fixture,sha256:sha(bytes),bytes:bytes.length,vertices:expected.vertices,triangles:expected.triangles},
  browser_build:side==='source'?bundle:sha(await readFile(path.join(web,'pkg/obrush_app_bg.wasm'))),
  cases:[], requests:[], cdp_trace:[], ui_actions:[], input_trace:[], download_events:[], downloads:[],
  screenshots:[], dialogs:[], exceptions:[], console_errors:[], blocked_external_requests:[],
  viewport_measurements:[], notice_acknowledgments:[], acknowledgments:[], failed_acknowledgments:[],
  page_observations:[], cleanup_errors:[],
  tolerances_chosen_before_run:{delivered_pressure_abs:1e-6, imported_position_component_abs:5e-5},
  scope:'Source Brush geometry mode on OBrush; separate from C231 ordinary Draw factors'
};
const owner = new OwnedCdp(out,result);
let page,pressed=false;
const cases = [
  {name:'neutral-quarter',pointerType:'pen',pressure:.25,radius:0,intensity:0},
  {name:'radius-quarter',pointerType:'pen',pressure:.25,radius:1,intensity:0},
  {name:'intensity-quarter',pointerType:'pen',pressure:.25,radius:0,intensity:1},
  {name:'pen-full-neutral',pointerType:'pen',pressure:1,radius:0,intensity:0},
  {name:'mouse-neutral',pointerType:'mouse',pressure:.5,radius:1,intensity:1},
];
let observer = `(()=>{const rows=[];window.__c234Pointers=rows;for(const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])window.addEventListener(type,e=>{if(rows.length<4096)rows.push({type,pointerType:e.pointerType,pressure:e.pressure,x:e.clientX,y:e.clientY,buttons:e.buttons,which:e.which,button:e.button,trusted:e.isTrusted,at:performance.now()});},true)})();\n`;
if(side==='source')for(const file of ['focused-alpha-input-observer.js','focused-alpha-hit-observer.js'])observer+=await readFile(path.join(home,'.artifacts/sculptgl-large-sustained-c58-stage/observer',file),'utf8')+'\n';
const pen = async (type,x,y,pointerType,pressure,down) => {
  const params={type,x,y,pointerType,force:pressure,button:type==='mouseMoved'?'none':'left',buttons:down?1:0,...(type==='mouseMoved'?{}:{clickCount:1})};
  result.input_trace.push({side,...params}); await page.send('Input.dispatchMouseEvent',params);
};
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
  for (const item of cases) {
    const row={...item,status:'running'}; result.cases.push(row);
    row.setup=await page.prepareFixture({path:fixture,vertices:expected.vertices});
    if (side==='source') {
      const tab=`window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')`;
      await page.sourceNumber('_ctrlTablet',`${tab}[0]`,item.radius);
      await page.sourceNumber('_ctrlTablet',`${tab}[1]`,item.intensity);
      row.settings=await page.evaluate(`(()=>({radius:window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')[0].value,intensity:window.__sculptglReferenceInstance.getGui()._ctrlTablet._menu.domUl.querySelectorAll('input[type=number]')[1].value}))()`);
    } else {
      await page.rootNumber('source_geometry.pressure.radius.value',item.radius);
      await page.rootNumber('source_geometry.pressure.intensity.value',item.intensity);
      row.settings=(await page.state()).source_geometry.per_tool[0];
    }
    await page.closeMenus();
    row.before=(await page.save(item.name+'-before')).record;
    const v=await page.viewport();
    const x=(v.screen[0]+v.screen[2])/2+80,y=(v.screen[1]+v.screen[3])/2;
    row.requested_path=[[x,y],[x+12,y]];
    const offset=await page.evaluate('window.__c234Pointers.length');
    await pen('mouseMoved',x,y,item.pointerType,item.pressure,false);
    await pen('mousePressed',x,y,item.pointerType,item.pressure,true); pressed=true;
    await pause(160);
    row.state_after_press=side==='source'?await page.evaluate('({action:window.__sculptglReferenceInstance._action,mesh:window.__sculptglReferenceInstance.getMesh()?.getNbVertices(),focus:window.__sculptglReferenceInstance._focusGui})'):await page.evaluate('window.obrushSculptMetrics()');
    await pen('mouseMoved',x+12,y,item.pointerType,item.pressure,true);
    await pause(160);
    row.state_after_move=side==='source'?await page.evaluate('({action:window.__sculptglReferenceInstance._action,mesh:window.__sculptglReferenceInstance.getMesh()?.getNbVertices(),focus:window.__sculptglReferenceInstance._focusGui})'):await page.evaluate('window.obrushSculptMetrics()');
    await pen('mouseReleased',x+12,y,item.pointerType,0,false); pressed=false;
    for(let i=0;i<8;i++)await page.raf();
    row.dom_events=await page.evaluate(`window.__c234Pointers.slice(${offset})`);
    if (!row.dom_events.some(e=>e.type==='pointerdown'&&e.pointerType===item.pointerType&&e.trusted&&Math.abs(e.pressure-item.pressure)<1e-6)) throw Error('Trusted declared pointer pressure missing '+item.name);
    row.after=(await page.save(item.name+'-after')).record;
    await page.chord('z'); row.undo=(await page.save(item.name+'-undo')).record;
    await page.chord('z',true); row.redo=(await page.save(item.name+'-redo')).record;
    row.status='complete public save and one-stroke history capture';
  }
  result.status='complete five-case normal-browser capture; independent geometry audit pending';
} catch(error) {
  result.status='FAILED; preserve partial evidence'; result.failure=String(error.stack??error);process.exitCode=1;
} finally {
  if(pressed&&page)try{const v=await page.viewport();await pen('mouseReleased',(v.screen[0]+v.screen[2])/2,(v.screen[1]+v.screen[3])/2,'pen',0,false);}catch(error){result.cleanup_errors.push(String(error));}
  try{await owner.close();}catch(error){result.cleanup_errors.push(String(error));process.exitCode=1;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify({out,status:result.status,failure:result.failure??null,cleanup_errors:result.cleanup_errors}));
  process.exit(process.exitCode??0);
}
