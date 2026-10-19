// Explicit experimental Source Worker input. The only geometry producer is
// source-geometry-worker.js; trusted ordinary viewport PointerEvents own the
// gesture, and the Rust studio checks every preview and final publication.
export function createHeldSourceWorkerInput(handle, canvas, notice, wasmBackend) {
  let enabled = false;
  let active = null;
  let button;
  const totals = { pointerMoves: 0, coalescedMoves: 0, sentDabs: 0,
                   acknowledgedDabs: 0, previewBytes: 0, finishBytes: 0,
                   commits: 0, noOps: 0, rejections: 0, discardedAtRelease: 0,
                   lastDiscardedAtRelease: 0, lastEndpointDropped: false,
                   lastBeginMs: null, lastFirstPreviewMs: null,
                   lastReleaseToCommitMs: null, maxPreviewMainMs: 0,
                   maxWorkerCallMs: 0 };

  const coordinates = event => {
    const bounds = canvas.getBoundingClientRect();
    if (Math.abs(bounds.width - canvas.clientWidth) >= 1 ||
        Math.abs(bounds.height - canvas.clientHeight) >= 1) {
      throw Error('Source Worker requires an untransformed 100% canvas');
    }
    return [event.clientX - bounds.left, event.clientY - bounds.top];
  };
  const label = () => {
    if (!button) return;
    button.textContent = active ? 'Source Worker · working' :
      enabled ? 'Source Worker · trial on' : 'Source Worker · trial off';
    button.setAttribute('aria-pressed', String(enabled));
    button.dataset.workerTrialStatus = JSON.stringify({enabled, active:Boolean(active), ...totals});
  };
  const stop = (reason, inform = true) => {
    const session = active;
    if (!session) return;
    active = null;
    session.worker.terminate();
    try { handle.source_worker_held_cancel(session.epoch); } catch {}
    try { canvas.releasePointerCapture(session.pointerId); } catch {}
    totals.rejections++;
    label();
    if (inform) notice(`Source Worker gesture rejected: ${String(reason).slice(0, 350)}`);
  };
  const post = (session, kind, fields = {}) => {
    if (session !== active || session.outstanding) return;
    const id = ++session.nextId;
    session.outstanding = { id, kind, sequence: fields.sequence };
    session.worker.postMessage({ id, kind, strokeEpoch: session.epoch, ...fields },
      fields.warmSeed ? [fields.warmSeed.buffer] : []);
  };
  const pump = session => {
    if (session !== active || session.outstanding) return;
    try {
      // Release is a hard input boundary: never drain an accumulated pointer
      // path after the button is up. A very quick click retains its down dab.
      if (session.released && session.nextSequence > 1) {
        const dropped = session.samples.length + Number(Boolean(session.latest));
        totals.discardedAtRelease += dropped;
        session.discardedAtRelease += dropped;
        session.endpointDropped ||= Boolean(session.latest);
        session.samples = [];
        session.latest = null;
      }
      if (session.samples.length) {
        const input = session.samples.shift();
        const sequence = String(session.nextSequence++);
        totals.sentDabs++;
        post(session, 'apply', { sequence, input });
        return;
      }
      if (session.latest) {
        const raw = session.latest;
        session.latest = null;
        const samples = JSON.parse(handle.source_worker_held_samples(
          session.epoch, raw.x, raw.y, raw.pressure, raw.pen));
        if (!Array.isArray(samples) || samples.length > 32) {
          throw Error('Source Worker path sampler returned an invalid batch');
        }
        session.samples = samples.map(input => JSON.stringify(input));
        pump(session);
        return;
      }
      if (session.released) {
        post(session, 'finish', { sequence: String(session.nextSequence++) });
      }
    } catch (error) { stop(error); }
  };
  const response = (session, message) => {
    if (session !== active) return;
    const expected = session.outstanding;
    if (!expected || !message || message.id !== expected.id ||
        message.kind !== expected.kind || message.success !== true) {
      stop(message?.error || 'Source Worker message identity or success was invalid');
      return;
    }
    session.outstanding = null;
    try {
      if (expected.kind === 'begin') {
        totals.lastBeginMs = performance.now() - session.startedAt;
        pump(session);
      } else if (expected.kind === 'apply') {
        if (message.strokeEpoch !== session.epoch || message.sequence !== expected.sequence ||
            !(message.delta instanceof Uint8Array)) {
          throw Error('Source Worker preview response was stale or malformed');
        }
        const mainStarted = performance.now();
        handle.source_worker_held_preview(session.epoch, session.revision, message.delta);
        totals.maxPreviewMainMs = Math.max(totals.maxPreviewMainMs,
          performance.now() - mainStarted);
        if (totals.lastFirstPreviewMs === null || session.firstPreviewAt === null) {
          session.firstPreviewAt = performance.now();
          totals.lastFirstPreviewMs = session.firstPreviewAt - session.startedAt;
        }
        totals.maxWorkerCallMs = Math.max(totals.maxWorkerCallMs,
          Number(message.timing?.worker_call_ms) || 0);
        totals.acknowledgedDabs++;
        totals.previewBytes += message.delta.byteLength;
        pump(session);
      } else if (expected.kind === 'finish') {
        if (message.strokeEpoch !== session.epoch || message.sequence !== expected.sequence ||
            !(message.delta instanceof Uint8Array) ||
            !(message.retained instanceof Uint8Array)) {
          throw Error('Source Worker final response was stale or malformed');
        }
        totals.finishBytes += message.delta.byteLength + message.retained.byteLength;
        const changed = handle.source_worker_held_commit(session.epoch, session.revision,
          message.delta, message.retained);
        if (typeof changed !== 'boolean') throw Error('Source Worker publication outcome was invalid');
        if (changed) totals.commits++; else totals.noOps++;
        totals.lastDiscardedAtRelease = session.discardedAtRelease;
        totals.lastEndpointDropped = session.endpointDropped;
        totals.lastReleaseToCommitMs = performance.now() - session.releasedAt;
        active = null;
        session.worker.terminate();
        try { canvas.releasePointerCapture(session.pointerId); } catch {}
        label();
      }
    } catch (error) { stop(error); }
  };
  const pointer = event => {
    if (!enabled) return;
    const type = event.type;
    if (!active) {
      if (type !== 'pointerdown' || event.target !== canvas ||
          !['mouse', 'pen'].includes(event.pointerType) || event.button !== 0 ||
          !(event.buttons & 1)) return;
      let x, y;
      try { [x, y] = coordinates(event); }
      catch (error) { notice(String(error)); return; }
      if (!handle.source_worker_held_pointer_in_viewport(x, y)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (!event.isTrusted) { notice('Source Worker trial requires a real viewport pointer event.'); return; }
      let capturedEpoch;
      try {
        const startedAt = performance.now();
        const seed = JSON.parse(handle.source_worker_held_begin(x, y,
          event.pointerType === 'pen' ? event.pressure : 0.5,
          event.pointerType === 'pen',
          event.altKey || event.ctrlKey || event.shiftKey || event.metaKey));
        capturedEpoch = seed.epoch;
        const warmSeed = handle.source_worker_held_seed_packet(seed.epoch);
        const url = new URL('./source-geometry-worker.js', import.meta.url);
        if (wasmBackend === 'simd') url.searchParams.set('simd', '1');
        const worker = new Worker(url, { type: 'module' });
        const session = { worker, epoch: seed.epoch, revision: seed.revision,
          pointerId: event.pointerId, nextId: 0, nextSequence: 1,
          outstanding: null, latest: null,
          samples: [JSON.stringify(seed.initial_input)], released: false,
          discardedAtRelease: 0, endpointDropped: false,
          startedAt, firstPreviewAt: null, releasedAt: null };
        active = session;
        worker.onmessage = reply => response(session, reply.data);
        worker.onerror = error => { error.preventDefault(); stop(error.message || 'Worker crashed'); };
        worker.onmessageerror = () => stop('Worker message could not be decoded');
        canvas.setPointerCapture(event.pointerId);
        post(session, 'begin', { project: seed.project, options: seed.options, warmSeed });
        label();
      } catch (error) {
        if (active) stop(error);
        else {
          if (capturedEpoch) handle.source_worker_held_cancel(capturedEpoch);
          notice(`Source Worker unavailable: ${String(error).slice(0, 350)}`);
        }
      }
      return;
    }
    const session = active;
    if (event.pointerId !== session.pointerId) {
      if (type === 'pointerdown') stop('A second pointer interrupted the Worker gesture');
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    if (!event.isTrusted) { stop('Synthetic pointer input interrupted the Worker gesture'); return; }
    if (type === 'pointercancel') { stop('Pointer contact was cancelled'); return; }
    if (type !== 'pointermove' && type !== 'pointerup') return;
    if (event.altKey || event.ctrlKey || event.shiftKey || event.metaKey) {
      stop('Modifier changed during Source Worker gesture'); return;
    }
    if (session.released) return;
    if (type === 'pointermove' && !(event.buttons & 1)) {
      stop('Primary pointer contact was lost'); return;
    }
    try {
      const [x, y] = coordinates(event);
      if (!handle.source_worker_held_pointer_in_viewport(x, y)) {
        stop('Pointer left the prepared viewport'); return;
      }
      if (session.latest) totals.coalescedMoves++;
      session.latest = { x, y,
        pressure: event.pointerType === 'pen' ? event.pressure : 0.5,
        pen: event.pointerType === 'pen' };
      if (type === 'pointermove') totals.pointerMoves++;
      if (type === 'pointerup') {
        session.released = true;
        session.releasedAt = performance.now();
      }
      pump(session);
    } catch (error) { stop(error); }
  };
  for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel']) {
    document.addEventListener(type, pointer, { capture: true });
  }
  window.addEventListener('blur', () => stop('Browser focus was lost', false));
  const start = () => { enabled = true; label(); return true; };
  const disable = () => {
    enabled = false;
    if (active) stop('Source Worker trial was disabled', false);
    label();
  };
  const status = () => ({ enabled, active: Boolean(active),
    epoch: active?.epoch || null, pending: active?.outstanding?.kind || null,
    queuedSamples: active?.samples.length || 0, ...totals });
  const mountTrialButton = () => {
    button = document.createElement('button');
    button.style.cssText = 'position:fixed;right:16px;top:54px;z-index:12;background:#292d35;color:#ffb483;border:1px solid #a86745;padding:8px 11px;border-radius:4px;cursor:pointer';
    button.title = 'Experimental Source Draw Worker. Exact warm state is transferred; only plain static, unmodified Source Draw is eligible. Unsupported strokes are rejected.';
    button.onclick = () => enabled ? disable() : start();
    document.body.append(button);
    label();
  };
  return { start, disable, status, mountTrialButton };
}
