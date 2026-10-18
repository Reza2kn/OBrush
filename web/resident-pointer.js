// Experimental pointer-driven WebGPU Inflate or ordinary Brush. The Rust document owns
// validation/history; a transient GPU canvas owns only one captured stroke.
export function createResidentPointerStudio(handle, studioCanvas, notice, options = {}) {
  let active = null;
  let gpuModule;
  let last = null;

  function status() {
    return active ? {
      phase: active.phase,
      token: active.id,
      dabs: active.dabs,
      received: active.received,
      coalesced: active.coalesced,
      gpuOwnedPicks: active.gpuOwnedPicks,
      gpuWaitMs: active.gpuWaitMs,
      profilePickMs: active.profilePickMs,
      profileSculptMs: active.profileSculptMs,
      profileDrawMs: active.profileDrawMs,
      profilePresentCpuMs: active.profilePresentCpuMs,
      startup: active.startup,
      gpuPick: active.gpuPick,
      brush: active.brush,
      depthPick: active.depthPick,
      pending: !!active.pending,
      hit: active.hit,
      ray: active.ray,
      width: active.canvas.width,
      height: active.canvas.height,
    } : { phase: 'idle', last };
  }

  async function start() {
    if (active) throw new Error('WebGPU preview is already active.');
    const startupAt = performance.now();
    const metrics = JSON.parse(handle.brush_metrics());
    const viewport = metrics.camera?.viewport;
    const matrix = metrics.camera?.view_projection;
    if (!Array.isArray(viewport) || viewport.length !== 4 ||
        !Array.isArray(matrix) || matrix.length !== 16 ||
        viewport.some(value => !Number.isFinite(value)) ||
        viewport[2] <= viewport[0] || viewport[3] <= viewport[1]) {
      throw new Error('The studio viewport has no valid captured camera.');
    }
    const base = studioCanvas.getBoundingClientRect();
    const width = viewport[2] - viewport[0];
    const height = viewport[3] - viewport[1];
    const scale = window.devicePixelRatio || 1;
    const canvas = document.createElement('canvas');
    canvas.id = 'obrush-resident-pointer-canvas';
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    Object.assign(canvas.style, {
      position: 'fixed', left: `${base.left + viewport[0]}px`,
      top: `${base.top + viewport[1]}px`, width: `${width}px`,
      height: `${height}px`, zIndex: '10', touchAction: 'none',
      cursor: 'crosshair',
    });
    const label = document.createElement('div');
    label.id = 'obrush-resident-pointer-label';
    label.textContent = `WebGPU ${options.brush ? 'Brush' : 'Inflate'} preview${options.brush ? ' · GPU-owned ray pick and direction' : options.depthPick ? ' · depth pick' : options.gpuPick ? ' · GPU-owned ray pick' : ''} · one stroke · Esc to cancel`;
    Object.assign(label.style, {
      position: 'fixed', left: `${base.left + viewport[0] + 12}px`,
      top: `${base.top + viewport[1] + 12}px`, zIndex: '11',
      pointerEvents: 'none', color: '#ffb483',
      background: '#17191de6', padding: '7px 10px',
      font: '12px system-ui,sans-serif', borderRadius: '4px',
    });
    const packet = handle.resident_studio_begin_upload();
    const captureMs = performance.now() - startupAt;
    if (!(packet instanceof Uint8Array) || packet.length < 8) {
      throw new Error('The studio returned an invalid GPU upload.');
    }
    const id = new DataView(packet.buffer, packet.byteOffset, 8)
      .getBigUint64(0, true).toString();
    let probe;
    let moduleMs;
    let uploadMs;
    let presentMs;
    try {
      gpuModule ||= await import('./gpu-pkg/resident.js');
      await gpuModule.default();
      moduleMs = performance.now() - startupAt - captureMs;
      probe = await gpuModule.ResidentProbe.load_upload_canvas(packet.subarray(8), canvas);
      uploadMs = performance.now() - startupAt - captureMs - moduleMs;
      probe.set_world_camera_gl_matrix(matrix, canvas.width, canvas.height);
      probe.checkpoint();
      if (!probe.present()) throw new Error('WebGPU could not present the captured mesh.');
      if (options.depthPick) await probe.wait_submitted();
      presentMs = performance.now() - startupAt - captureMs - moduleMs - uploadMs;
    } catch (error) {
      probe?.destroy();
      try { handle.resident_studio_cancel(id); } catch { /* stale document */ }
      throw error;
    }
    const session = {
      id, probe, canvas, label, viewport, matrix,
      revision: metrics.document_revision, radius: metrics.radius,
      strength: metrics.strength, phase: 'ready', pointer: null,
      pending: null, busy: false, releasing: false, aborting: false,
      dabs: 0, received: 0, closed: false, watcher: null,
      error: null, hit: null,
      ray: null,
      coalesced: 0, pick_ms: 0, maximum_pick_ms: 0,
      gpuOwnedPicks: 0, gpuPick: !!options.gpuPick || !!options.brush,
      brush: !!options.brush,
      depthPick: !!options.depthPick,
      gpuWaitMs: 0, maximumGpuWaitMs: 0,
      profilePickMs: 0, profileSculptMs: 0, profileDrawMs: 0,
      profilePresentCpuMs: 0,
      release_at: null, stroke_at: null, commit_ms: null,
      core_timing: null,
      startup: { captureMs, moduleMs, uploadMs, presentMs, totalMs: performance.now() - startupAt },
    };
    active = session;
    document.body.append(canvas, label);

    function close() {
      if (session.closed) return;
      session.closed = true;
      last = {dabs: session.dabs, received: session.received,
        coalesced: session.coalesced, gpuOwnedPicks: session.gpuOwnedPicks,
        gpuWaitMs: session.gpuWaitMs, maximumGpuWaitMs: session.maximumGpuWaitMs,
        profilePickMs: session.profilePickMs,
        profileSculptMs: session.profileSculptMs,
        profileDrawMs: session.profileDrawMs,
        profilePresentCpuMs: session.profilePresentCpuMs,
        pick_ms: session.pick_ms,
        maximum_pick_ms: session.maximum_pick_ms,
        release_ms: session.release_at === null ? null : performance.now() - session.release_at,
        stroke_ms: session.stroke_at === null ? null : performance.now() - session.stroke_at,
        commit_ms: session.commit_ms,
        startup: session.startup,
        core_timing: session.core_timing, gpuPick: session.gpuPick,
        brush: session.brush,
        depthPick: session.depthPick,
        error: session.error, hit: session.hit, ray: session.ray,
        aborted: session.aborting};
      clearInterval(session.watcher);
      window.removeEventListener('keydown', escape, true);
      window.removeEventListener('blur', cancel);
      canvas.remove(); label.remove();
      probe.destroy();
      if (active === session) active = null;
    }

    async function finish() {
      if (session.closed) return;
      session.phase = session.aborting ? 'cancelling' : 'committing';
      clearInterval(session.watcher);
      try {
        if (session.aborting) {
          probe.cancel();
          handle.resident_studio_cancel(id);
        } else {
          const started = performance.now();
          const failure = await probe.failure_vertex();
          if (failure !== null) throw new Error(`GPU candidate rejected at vertex ${failure}`);
          const sparse = await probe.sparse_positions();
          handle.resident_studio_commit(id, sparse);
          session.core_timing = JSON.parse(handle.resident_studio_commit_timing());
          session.commit_ms = performance.now() - started;
        }
      } catch (error) {
        session.error = String(error);
        try { handle.resident_studio_cancel(id); } catch { /* stale document */ }
        notice(`WebGPU preview ended without an edit: ${error}`);
      } finally {
        close();
      }
    }


    async function drain() {
      if (session.busy || session.closed) return;
      session.busy = true;
      try {
        while (session.pending && !session.aborting) {
          const point = session.pending;
          session.pending = null;
          const ray = session.depthPick ? null : Array.from(handle.resident_studio_ray(point.x, point.y));
          session.ray = ray;
          const strength = Math.max(0, Math.min(1, session.strength * point.pressure));
          if (strength === 0) continue;
          if (session.brush) {
            if (options.profileStages) {
              let stageStarted = performance.now();
              probe.queue_world_ray_pick(ray.slice(0, 6), ray[6]);
              await probe.wait_submitted();
              session.profilePickMs += performance.now() - stageStarted;
              stageStarted = performance.now();
              probe.advance_brush_last_pick(session.radius, strength);
              await probe.wait_submitted();
              session.profileSculptMs += performance.now() - stageStarted;
            } else {
              probe.advance_brush_world_ray(ray.slice(0, 6), ray[6], session.radius, strength);
            }
            session.gpuOwnedPicks++;
          } else if (session.depthPick) {
            const stageStarted = options.profileStages ? performance.now() : 0;
            probe.advance_depth_pixel(point.pixelX, point.pixelY, session.radius, strength);
            if (options.profileStages) {
              await probe.wait_submitted();
              session.profileSculptMs += performance.now() - stageStarted;
            }
            session.gpuOwnedPicks++;
          } else if (session.gpuPick) {
            if (options.profileStages) {
              let stageStarted = performance.now();
              probe.queue_world_ray_pick(ray.slice(0, 6), ray[6]);
              await probe.wait_submitted();
              session.profilePickMs += performance.now() - stageStarted;
              stageStarted = performance.now();
              probe.advance_last_pick(session.radius, strength);
              await probe.wait_submitted();
              session.profileSculptMs += performance.now() - stageStarted;
            } else {
              probe.advance_world_ray(ray.slice(0, 6), ray[6], session.radius, strength);
            }
            session.gpuOwnedPicks++;
          } else {
            const pickStarted = performance.now();
            const hit = JSON.parse(await probe.pick_world_ray(ray.slice(0, 6), ray[6]));
            const pickMs = performance.now() - pickStarted;
            session.pick_ms += pickMs;
            session.maximum_pick_ms = Math.max(session.maximum_pick_ms, pickMs);
            session.hit = hit;
            if (!hit) continue;
            probe.advance_at(...hit.local_position, session.radius, strength, 1);
          }
          const presentStarted = options.profileStages ? performance.now() : 0;
          if (!probe.present()) throw new Error('WebGPU presentation stopped.');
          if (options.profileStages) {
            session.profilePresentCpuMs += performance.now() - presentStarted;
          }
          session.dabs++;
          if (session.gpuPick || session.depthPick || session.brush) {
            // Pace to completed GPU work while pointer events coalesce.
            // This crosses no per-dab mesh or pick bytes to the CPU.
            const waitStarted = performance.now();
            await probe.wait_submitted();
            const waitMs = performance.now() - waitStarted;
            session.gpuWaitMs += waitMs;
            session.maximumGpuWaitMs = Math.max(session.maximumGpuWaitMs, waitMs);
            if (options.profileStages) session.profileDrawMs += waitMs;
          }
        }
      } catch (error) {
        session.error = String(error);
        session.aborting = true;
        notice(`WebGPU preview cancelled: ${error}`);
      } finally {
        session.busy = false;
        if (session.aborting || session.releasing) await finish();
      }
    }

    function point(event) {
      const bounds = canvas.getBoundingClientRect();
      const x = Math.min(bounds.width - 0.01, Math.max(0, event.clientX - bounds.left));
      const y = Math.min(bounds.height - 0.01, Math.max(0, event.clientY - bounds.top));
      return {
        x: viewport[0] + x, y: viewport[1] + y,
        pixelX: Math.min(canvas.width - 1, Math.max(0, Math.floor(x * canvas.width / bounds.width))),
        pixelY: Math.min(canvas.height - 1, Math.max(0, Math.floor(y * canvas.height / bounds.height))),
        pressure: event.pointerType === 'pen' ? Math.max(0, Math.min(1, event.pressure)) : 1,
      };
    }
    function enqueue(event) {
      session.received++;
      if (session.pending) session.coalesced++;
      session.pending = point(event);
      void drain();
    }
    function cancel() {
      if (session.closed) return;
      session.pending = null;
      session.aborting = true;
      void drain();
    }
    function escape(event) {
      if (event.key === 'Escape') { event.preventDefault(); cancel(); }
    }
    canvas.addEventListener('pointerdown', event => {
      if (session.pointer !== null || event.button !== 0) return;
      event.preventDefault(); canvas.setPointerCapture(event.pointerId);
      session.pointer = event.pointerId;
      session.stroke_at = performance.now();
      session.phase = 'sculpting';
      enqueue(event);
    });
    canvas.addEventListener('pointermove', event => {
      if (event.pointerId === session.pointer && !session.releasing) enqueue(event);
    });
    canvas.addEventListener('pointerup', event => {
      if (event.pointerId !== session.pointer) return;
      event.preventDefault(); enqueue(event);
      session.releasing = true;
      session.release_at = performance.now();
      session.phase = 'releasing';
      void drain();
    });
    canvas.addEventListener('pointercancel', cancel);
    window.addEventListener('keydown', escape, true);
    window.addEventListener('blur', cancel);
    session.watcher = setInterval(() => {
      if (session.closed) return;
      try {
        const live = JSON.parse(handle.brush_metrics());
        if (live.document_revision !== session.revision ||
            JSON.stringify(live.camera?.viewport) !== JSON.stringify(session.viewport) ||
            JSON.stringify(live.camera?.view_projection) !== JSON.stringify(session.matrix)) cancel();
      } catch { cancel(); }
    }, 250);
    return status();
  }

  function stop() {
    if (!active) return false;
    active.aborting = true;
    active.pending = null;
    // The active session's own cancellation listener owns GPU cleanup.
    active.canvas.dispatchEvent(new Event('pointercancel'));
    return true;
  }
  return { start, stop, status };
}
