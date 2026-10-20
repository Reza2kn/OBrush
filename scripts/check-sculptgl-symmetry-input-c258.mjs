// C258: frozen reference-only pen/mouse pressure and stroke-admission crossover.
// Usage: node scripts/check-sculptgl-symmetry-input-c258.mjs PINNED_HOST_MIRROR
import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const insist = (condition, why) => { if (!condition) throw Error(why); };
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const frozen = Object.freeze({
  offset: 0.5, radius: 250, intensity: 0.5, pressure: 0.5,
  local_path: [[3.70, -1.99, 0], [3.845, -1.99, 0], [3.99, -1.99, 0]],
  spacing_css: 37.5, span_relative_tolerance: 0.01, trace_limit: 500,
});
function event(type, point, pointerType, buttons, force) {
  return { type, x: point[0], y: point[1], button: 'left', buttons,
    pointerType, force, clickCount: type === 'mouseMoved' ? 0 : 1 };
}
function assertHeldEvents(events, pointerType) {
  const first = events.findIndex(e => e.type === 'pointerdown' && e.buttons === 1);
  insist(first >= 0, `${pointerType} missing held pointerdown`);
  const held = events.slice(first).filter(e =>
    (e.type === 'pointerdown' || e.type === 'pointermove') && e.buttons === 1);
  insist(held.some(e => e.type === 'pointerdown') && held.some(e => e.type === 'pointermove'),
    `${pointerType} missing held pointer lifecycle`);
  insist(held.every(e => e.trusted && e.pointerType === pointerType &&
    Math.abs(e.pressure - frozen.pressure) <= 1e-6),
  `${pointerType} did not deliver trusted held pressure 0.5: ${JSON.stringify(held)}`);
  return held;
}
function summarizeTrace(trace) {
  const count = kind => trace.filter(row => row.kind === kind).length;
  return { pan_update: count('panUpdate'), device_down: count('deviceDown'), device_move: count('deviceMove'),
    device_up: count('deviceUp'), tool_start: count('toolStart'),
    sculpt_stroke: count('sculptStroke'), make_stroke: count('makeStroke'),
    brush_stroke: count('brushStroke'), primary_pick: count('pickPrimary'),
    mirror_pick: count('pickMirror'), primary_radius: count('radiusPrimary'),
    mirror_radius: count('radiusMirror') };
}
if (process.argv.includes('--self-check')) {
  const p = frozen.local_path[0];
  for (const pointerType of ['pen', 'mouse']) {
    const press = event('mousePressed', p, pointerType, 1, 0.5);
    const move = event('mouseMoved', p, pointerType, 1, 0.5);
    const release = event('mouseReleased', p, pointerType, 0, 0);
    insist(press.force === 0.5 && move.force === 0.5 && release.force === 0 &&
      press.pointerType === pointerType && release.buttons === 0,
    `${pointerType} event contract failed`);
    const held = assertHeldEvents([
      { type: 'pointermove', buttons: 1, trusted: true, pointerType, pressure: 0 },
      { type: 'pointerdown', buttons: 1, trusted: true, pointerType, pressure: 0.5 },
      { type: 'pointermove', buttons: 1, trusted: true, pointerType, pressure: 0.5 },
    ], pointerType);
    insist(held.length === 2, 'Precontact hover leaked into held events');
  }
  const counts = summarizeTrace([{ kind: 'deviceDown' }, { kind: 'brushStroke' }]);
  insist(counts.device_down === 1 && counts.brush_stroke === 1 && counts.make_stroke === 0,
    'Trace count self-check failed');
  console.log('C258 event and bounded-trace self-check passed');
  process.exit(0);
}

const reference = path.resolve(process.argv[2] || '.tools/sculptgl-reference/host-mirror');
const out = path.resolve('.artifacts/sculptgl-symmetry-input-c258/' + randomUUID());
await mkdir(out, { recursive: true });
const fixture = await readFile('crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl');
const fixturePath = path.join(out, 'independent-affine-two-mesh.sgl');
await writeFile(fixturePath, fixture);
const bundle = await readFile(path.join(reference, 'sculptgl.js'));
const manifest = JSON.parse(await readFile(path.join(reference, 'mirror-manifest.json')));
insist(sha(bundle) === manifest.original_bundle.sha256, 'Pinned reference bundle differs');
insist(sha(fixture) === '99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd',
  'Independent fixture differs');
const result = {
  scope: 'C258 pinned SculptGL reference-only normal-browser pressure/admission crossover; no OBrush parity claim',
  source: { commit: '8e45dafc0f7906e5238dfffc1a2df742bd25f29d',
    bundle_sha256: sha(bundle), wrapper_sha256: sha(await readFile(path.join(reference, 'index.html'))) },
  build: { checkout_commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim() },
  harness_sha256: sha(await readFile(new URL(import.meta.url))),
  fixture: { sha256: sha(fixture), bytes: fixture.length }, frozen,
  arms: [], errors: [], status: 'running',
};
const instrument = `(() => {
  const q = window.__c258 = { events: [], trace: [], errors: [] };
  const push = (a, item) => { if (a.length >= 500) throw Error('C258 bounded trace overflow'); a.push(item); };
  for (const type of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup'])
    document.addEventListener(type, e => push(q.events, {
      type, trusted: e.isTrusted, pointerType: e.pointerType ?? null,
      pressure: e.pressure ?? null, buttons: e.buttons, which: e.which,
      x: e.clientX, y: e.clientY, at: performance.now(),
      target: e.target?.id ?? null,
    }), true);
  window.addEventListener('error', e => push(q.errors, String(e.message)), true);
})();`;
const traceInstall = `(() => {
  const a = window.__sculptglReferenceInstance, q = window.__c258;
  if (!a || !q || q.installed) return false;
  q.installed = true;
  const push = row => { if (q.trace.length >= 500) throw Error('C258 bounded trace overflow');
    q.trace.push({ at: performance.now(), ...row }); };
  const tool = a.getSculptManager().getCurrentTool();
  const pose = () => ({ action: a._action, mouse: [a._mouseX, a._mouseY],
    last: [tool._lastMouseX, tool._lastMouseY], active: a.getMeshes().indexOf(a.getMesh()),
    vertices: Array.from(a.getMesh().getVertices()) });
  const detail = ev => ev ? { type: ev.type ?? 'proxy', which: ev.which ?? null,
    pageX: ev.pageX ?? null, pageY: ev.pageY ?? null,
    shiftKey: !!ev.shiftKey, ctrlKey: !!ev.ctrlKey, altKey: !!ev.altKey } : null;
  const wrap = (obj, key, kind, extra) => {
    const original = obj[key];
    obj[key] = function (...args) {
      const before = pose();
      const added = extra?.(args, before) ?? {};
      const value = original.apply(this, args);
      push({ kind, event: detail(args[0]), before, after: pose(),
        returned: typeof value === 'boolean' ? value : null,
        returned_truthy: !!value, returned_type: typeof value, ...added });
      return value;
    };
  };
  wrap(a, 'onDeviceDown', 'deviceDown');
  wrap(a, 'onDeviceMove', 'deviceMove');
  wrap(a, 'onDeviceUp', 'deviceUp');
  wrap(a, 'onPanUpdateNbPointers', 'panUpdate', args => ({ nb_pointers: args[0] }));
  wrap(tool, 'start', 'toolStart');
  wrap(tool, 'sculptStroke', 'sculptStroke', (_, before) => {
    const dx = before.mouse[0] - before.last[0], dy = before.mouse[1] - before.last[1];
    return { spacing_distance: Math.hypot(dx, dy),
      spacing_threshold: 0.15 * tool._radius * a.getPixelRatio() };
  });
  wrap(tool, 'makeStroke', 'makeStroke', args => ({ screen: args.slice(0, 2) }));
  wrap(tool, 'stroke', 'brushStroke', args => ({
    side: args[0] === a.getPickingSymmetry() ? 'mirror' : 'primary',
    selected_before: Array.from(args[0]?.getPickedVertices() ?? []),
    local_radius_squared: args[0]?.getLocalRadius2() ?? null,
  }));
  for (const [picker, kind] of [[a.getPicking(), 'pickPrimary'],
      [a.getPickingSymmetry(), 'pickMirror']]) {
    const originalRadius = picker.computeWorldRadius2;
    picker.computeWorldRadius2 = function (ignorePressure) {
      const pressure = q.events.filter(e => e.type === 'pointerdown' || e.type === 'pointermove').at(-1)?.pressure ?? null;
      const screenRadius = tool.getScreenRadius();
      const wasProject = this.project, wasUnproject = this.unproject;
      const trace = { kind: kind === 'pickPrimary' ? 'radiusPrimary' : 'radiusMirror',
        ignore_pressure: !!ignorePressure, pressure, screen_radius: screenRadius,
        local_hit: Array.from(this.getIntersectionPoint()), mesh_scale_squared: this.getMesh()?.getScale2() ?? null };
      this.project = function (v) { trace.world = Array.from(v);
        const p = wasProject.call(this, v); trace.projected = Array.from(p); return p; };
      this.unproject = function (x,y,z) { trace.unproject_screen = [x,y,z];
        const p = wasUnproject.call(this,x,y,z); trace.edge = Array.from(p); return p; };
      try {
        const squared = originalRadius.call(this, ignorePressure);
        trace.world_radius_squared = squared;
        trace.local_radius_squared = squared / trace.mesh_scale_squared;
        trace.offset_x = trace.unproject_screen?.[0] - trace.projected?.[0];
        trace.pressure_factor = trace.offset_x / screenRadius;
        push(trace);
        return squared;
      } finally { this.project = wasProject; this.unproject = wasUnproject; }
    };
    const original = picker.pickVerticesInSphere;
    picker.pickVerticesInSphere = function (r2) {
      const selected = original.call(this, r2);
      push({ kind, r2, ids: Array.from(selected),
        face: this.getPickedFace(), point: Array.from(this.getIntersectionPoint()),
        mesh: a.getMeshes().indexOf(this.getMesh()), action: a._action,
        pressure: q.events.filter(e => e.type === 'pointerdown' || e.type === 'pointermove').at(-1)?.pressure ?? null });
      return selected;
    };
  }
  return true;
})()`;
let server;
async function runArm(pointerType, base) {
  const arm = { pointer_type: pointerType, status: 'running', errors: [], browser_logs: [], screenshots: [] };
  result.arms.push(arm);
  let chrome, socket, session, next = 0, stderr = '';
  const waiting = new Map();
  async function send(method, params = {}, selected = session) {
    const id = ++next;
    const reply = new Promise((resolve, reject) => {
      const timer = setTimeout(() => { waiting.delete(id); reject(Error(`CDP timeout ${method}`)); }, 30000);
      waiting.set(id, { resolve, reject, timer });
    });
    socket.send(JSON.stringify({ id, method, params, ...(selected ? { sessionId: selected } : {}) }));
    return reply;
  }
  async function evaluate(expression) {
    const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (reply.exceptionDetails) throw Error(JSON.stringify(reply.exceptionDetails));
    return reply.result.value;
  }
  async function until(fn, why) {
    let last;
    for (let i = 0; i < 150; i++) {
      try { const value = await fn(); if (value) return value; } catch (error) { last = error; }
      await pause(100);
    }
    throw Error(`Timed out ${why}${last ? `: ${last}` : ''}`);
  }
  async function screenshot(name) {
    const bytes = Buffer.from((await send('Page.captureScreenshot',
      { format: 'png', fromSurface: true })).data, 'base64');
    const filename = `${pointerType}-${name}.png`;
    await writeFile(path.join(out, filename), bytes);
    arm.screenshots.push({ name: filename, sha256: sha(bytes),
      width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) });
  }
  async function frame() { await evaluate('new Promise(requestAnimationFrame)'); }
  async function park() {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1550, y: 970, buttons: 0 });
    await frame(); await pause(120);
  }
  async function enterNumber(rect, value) {
    const x = (rect[0] + rect[2]) / 2, y = (rect[1] + rect[3]) / 2;
    insist(x > 0 && x < 1600 && y > 72 && y < 990, `Reference control off-screen: ${rect}`);
    for (const type of ['mousePressed', 'mouseReleased'])
      await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
    await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'a', code: 'KeyA',
      windowsVirtualKeyCode: 65, modifiers: 4, commands: ['selectAll'] });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA',
      windowsVirtualKeyCode: 65, modifiers: 4 });
    await send('Input.insertText', { text: String(value) });
    await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Enter',
      code: 'Enter', windowsVirtualKeyCode: 13 });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter',
      code: 'Enter', windowsVirtualKeyCode: 13 });
    await pause(200);
  }
  try {
    chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
      ['--headless=new', `--user-data-dir=${path.join(out, `chrome-${pointerType}`)}`,
        '--remote-debugging-port=0', '--no-first-run', '--disable-background-networking',
        '--window-size=1600,1000', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
    chrome.stderr.on('data', bytes => { stderr += bytes; });
    const endpoint = await until(async () => stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/)?.[1],
      'owned Chrome endpoint');
    socket = new WebSocket(endpoint);
    await new Promise((resolve, reject) => {
      socket.addEventListener('open', resolve, { once: true });
      socket.addEventListener('error', reject, { once: true });
    });
    socket.addEventListener('message', message => {
      const payload = JSON.parse(String(message.data));
      if (payload.method === 'Runtime.exceptionThrown' ||
          (payload.method === 'Log.entryAdded' && payload.params?.entry?.level === 'error'))
        arm.browser_logs.push(payload.params);
      if (!payload.id) return;
      const pending = waiting.get(payload.id); if (!pending) return;
      waiting.delete(payload.id); clearTimeout(pending.timer);
      payload.error ? pending.reject(Error(JSON.stringify(payload.error))) : pending.resolve(payload.result);
    });
    const target = await send('Target.createTarget', { url: 'about:blank' }, null);
    session = (await send('Target.attachToTarget',
      { targetId: target.targetId, flatten: true }, null)).sessionId;
    for (const domain of ['Page', 'Runtime', 'DOM', 'Log']) await send(`${domain}.enable`);
    await send('Emulation.setDeviceMetricsOverride',
      { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
    await send('Page.addScriptToEvaluateOnNewDocument', { source: instrument });
    await send('Page.navigate', { url: `${base}/` });
    await until(async () => await evaluate('window.__sculptglReferenceInstance?.getMeshes().length===1'),
      'pinned SculptGL startup');
    const root = (await send('DOM.getDocument')).root;
    const nodeId = (await send('DOM.querySelector',
      { nodeId: root.nodeId, selector: '#fileopen' })).nodeId;
    insist(nodeId, 'Missing reference file input');
    await send('DOM.setFileInputFiles', { nodeId, files: [fixturePath] });
    await until(async () => await evaluate('window.__sculptglReferenceInstance.getMeshes().length===3'),
      'reference import');
    await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.showHide(true)');
    await evaluate('window.__sculptglReferenceInstance.getGui()._ctrlScene.onOffsetSymmetry(0.5)');
    const controls = `(() => { const a=window.__sculptglReferenceInstance,
      t=a.getSculptManager().getCurrentTool(), m=a.getMesh(), c=a.getCanvas(),
      r=c.getBoundingClientRect(); return { meshes:a.getMeshes().length,
      active:a.getMeshes().indexOf(m), visible:a.getMeshes().map(x=>x.isVisible()),
      tool_index:a.getSculptManager().getToolIndex(),
      symmetry:a.getSculptManager().getSymmetry(), offset:m.getSymmetryOffset(),
      radius:t._radius, intensity:t._intensity, alpha:t._idAlpha,
      clay:t._clay, accumulate:t._accumulate, negative:t._negative,
      culling:t._culling, lock:t._lockPosition,
      continuous:a.getSculptManager()._continuous,
      canvas:[r.left,r.top,r.right,r.bottom], positions:Array.from(m.getVertices()) }; })()`;
    const radiusInput = `(() => { const menu=window.__sculptglReferenceInstance.getGui()._ctrlSculpting._menu.domUl;
      const e=Array.from(menu.querySelectorAll('input[type=number]'))
        .find(e=>e.getBoundingClientRect().width>0); const r=e?.getBoundingClientRect();
      return r?{value:Number(e.value),rect:[r.left,r.top,r.right,r.bottom]}:null; })()`;
    const input = await evaluate(radiusInput);
    insist(input?.rect, 'Visible reference radius numeric control missing');
    await enterNumber(input.rect, frozen.radius);
    insist((await evaluate(radiusInput)).value === frozen.radius, 'Reference radius differs');
    arm.controls = await evaluate(controls);
    insist(arm.controls.meshes === 3 && arm.controls.active === 2 &&
      arm.controls.visible.filter(Boolean).length === 1 && arm.controls.visible[2] &&
      arm.controls.tool_index === 0 && arm.controls.symmetry &&
      arm.controls.offset === frozen.offset &&
      arm.controls.radius === frozen.radius && arm.controls.intensity === frozen.intensity &&
      arm.controls.alpha === 0 && !arm.controls.continuous &&
      arm.controls.clay && arm.controls.accumulate && !arm.controls.negative &&
      !arm.controls.culling && !arm.controls.lock,
    `Reference controls differ: ${JSON.stringify(arm.controls)}`);
    const projected = `(() => { const a=window.__sculptglReferenceInstance,
      m=a.getMesh(), cam=a.getCamera(), r=a.getCanvas().getBoundingClientRect();
      return ${JSON.stringify(frozen.local_path)}.map(v=>{ const q=m.getMatrix();
        const w=[q[0]*v[0]+q[4]*v[1]+q[8]*v[2]+q[12],
          q[1]*v[0]+q[5]*v[1]+q[9]*v[2]+q[13],
          q[2]*v[0]+q[6]*v[1]+q[10]*v[2]+q[14]], s=cam.project(w);
        return [r.left+s[0],r.top+s[1]]; }); })()`;
    arm.path = await evaluate(projected);
    arm.path_span_css = Math.hypot(arm.path.at(-1)[0] - arm.path[0][0],
      arm.path.at(-1)[1] - arm.path[0][1]);
    insist(arm.path_span_css > frozen.spacing_css &&
      arm.path.every(([x,y]) => x > arm.controls.canvas[0]+2 &&
        x < arm.controls.canvas[2]-2 && y > arm.controls.canvas[1]+2 &&
        y < arm.controls.canvas[3]-2),
    `Reference path failed canvas/spacing gate: ${JSON.stringify(arm.path)}`);
    insist(await evaluate(traceInstall), 'C258 original-call trace not installed');
    await park();
    await screenshot('before');
    await evaluate('window.__c258.events.length=0;window.__c258.trace.length=0');
    arm.vertices_before = Array.from((await evaluate(controls)).positions);
    const [first, ...rest] = arm.path;
    await send('Input.dispatchMouseEvent', event('mouseMoved', first, pointerType, 0, 0));
    await send('Input.dispatchMouseEvent', event('mousePressed', first, pointerType, 1, 0.5));
    await frame(); await pause(50);
    for (const point of rest) {
      await send('Input.dispatchMouseEvent', event('mouseMoved', point, pointerType, 1, 0.5));
      await frame(); await pause(50);
    }
    await send('Input.dispatchMouseEvent', event('mouseReleased', arm.path.at(-1),
      pointerType, 0, 0));
    for (let i=0; i<4; i++) await frame();
    await pause(120);
    arm.events = await evaluate('window.__c258.events');
    arm.trace = await evaluate('window.__c258.trace');
    arm.page_errors = await evaluate('window.__c258.errors');
    arm.held_events = assertHeldEvents(arm.events, pointerType);
    arm.trace_counts = summarizeTrace(arm.trace);
    await park(); await screenshot('after');
    arm.vertices_after = Array.from((await evaluate(controls)).positions);
    arm.vertex_deltas = arm.vertices_after.map((value, index) =>
      value - arm.vertices_before[index]);
    arm.gpu_error = await evaluate('window.__sculptglReferenceInstance._gl.getError()');
    insist(arm.gpu_error === 0, `Reference WebGL error ${arm.gpu_error}`);
    insist(arm.page_errors.length === 0 &&
      !arm.browser_logs.some(log => log.exceptionDetails),
    `Reference browser exceptions: ${JSON.stringify({ page: arm.page_errors,
      cdp: arm.browser_logs })}`);
    arm.status = 'measured';
  } catch (error) {
    arm.status = 'failed'; arm.errors.push(String(error?.stack ?? error));
  } finally {
    for (const item of waiting.values()) clearTimeout(item.timer);
    if (socket) { try { socket.close(); } catch {} }
    if (chrome) { chrome.kill('SIGTERM'); await pause(250); if (chrome.exitCode === null) chrome.kill('SIGKILL'); }
  }
}
try {
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
      const file = path.resolve(reference, '.' + (pathname === '/' ? '/index.html' : pathname));
      if (!file.startsWith(reference + path.sep)) throw Error('Outside pinned mirror');
      const bytes = await readFile(file);
      response.setHeader('Content-Type', ({ '.js': 'text/javascript', '.html': 'text/html',
        '.css': 'text/css' })[path.extname(file)] || 'application/octet-stream');
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  for (const pointerType of ['pen', 'mouse']) await runArm(pointerType, base);
  const [pen, mouse] = result.arms;
  if (pen.path_span_css && mouse.path_span_css) {
    result.path_span_relative_difference = Math.abs(pen.path_span_css/mouse.path_span_css-1);
    insist(result.path_span_relative_difference <= frozen.span_relative_tolerance,
      `Two-arm camera/path mismatch ${result.path_span_relative_difference}`);
  }
  insist(result.arms.every(arm => arm.status === 'measured'), 'One or both C258 arms failed');
  result.status = 'measured';
} catch (error) {
  result.status = 'failed'; result.errors.push(String(error?.stack ?? error));
} finally {
  if (server) await new Promise(resolve => server.close(resolve));
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ status: result.status, out,
    arms: result.arms.map(arm => ({ pointer_type: arm.pointer_type,
      status: arm.status, trace_counts: arm.trace_counts,
      vertex_deltas: arm.vertex_deltas, errors: arm.errors.slice(-1) })),
    errors: result.errors }, null, 2));
  if (result.status !== 'measured') process.exitCode = 1;
}
