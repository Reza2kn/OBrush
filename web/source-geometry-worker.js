// Display-free Source geometry owner. Only the ordinary studio may seed this
// module worker; all geometry and retained state remain in shared Rust/WASM.
let BrowserSourceWorkerStroke;
const ready = (async () => {
  const backend = new URL(self.location.href).searchParams.has('simd')
    ? './pkg-simd/obrush_app.js' : './pkg/obrush_app.js';
  const module = await import(backend);
  await module.default();
  BrowserSourceWorkerStroke = module.BrowserSourceWorkerStroke;
})();
let owner;
let epoch;
let nextSequence = 1;
let failed = false;
let chain = Promise.resolve();
let queued = 0;

function reject(id, error) {
  failed = true;
  owner?.free();
  owner = undefined;
  self.postMessage({ id, success: false, error: String(error).slice(0, 1024) });
}

async function dispatch(event, arrived, backlog) {
  const { id, kind, project, options, input, strokeEpoch, sequence, warmSeed } = event.data || {};
  if (!Number.isSafeInteger(id) || id < 1 || failed) return;
  try {
    await ready;
    if (kind === 'begin') {
      if (owner || typeof project !== 'string' || typeof options !== 'string' ||
          typeof strokeEpoch !== 'string' || project.length > 512 * 1024 * 1024 ||
          options.length > 2048 || !(warmSeed instanceof Uint8Array) ||
          warmSeed.byteLength > 256 * 1024 * 1024) throw Error('Invalid Source worker seed');
      owner = new BrowserSourceWorkerStroke(project, options, strokeEpoch, warmSeed);
      epoch = strokeEpoch;
      self.postMessage({ id, kind, success: true });
      return;
    }
    if (!owner || strokeEpoch !== epoch || typeof sequence !== 'string' ||
        sequence !== String(nextSequence)) throw Error('Stale or out-of-order Source worker message');
    if (kind === 'apply') {
      if (typeof input !== 'string' || input.length > 8 * 1024) {
        throw Error('Invalid Source worker input');
      }
      const started = performance.now();
      const delta = owner.apply(epoch, sequence, input);
      const timing = JSON.parse(owner.timing_json());
      if (!(delta instanceof Uint8Array) || delta.byteLength < 88 ||
          delta.byteLength > 32 * 1024 * 1024) throw Error('Invalid Source worker delta');
      nextSequence++;
      self.postMessage({ id, kind, success: true, strokeEpoch, sequence, delta,
                         timing: { ...timing, queued_before: backlog,
                                   queue_wait_ms: started - arrived,
                                   worker_call_ms: performance.now() - started } }, [delta.buffer]);
      return;
    }
    if (kind === 'finish') {
      const result = owner.finish(epoch, sequence);
      const delta = result.take_delta();
      const retained = result.take_retained();
      result.free();
      owner.free();
      owner = undefined;
      if (!(delta instanceof Uint8Array) || delta.byteLength < 88 ||
          delta.byteLength > 32 * 1024 * 1024 ||
          !(retained instanceof Uint8Array) || retained.byteLength > 256 * 1024 * 1024) {
        throw Error('Invalid completed Source worker result');
      }
      nextSequence++;
      self.postMessage({ id, kind, success: true, strokeEpoch, sequence, delta, retained },
                       [delta.buffer, retained.buffer]);
      return;
    }
    throw Error('Unknown Source worker message');
  } catch (error) { reject(id, error); }
};

self.onmessage = event => {
  if (queued >= 512) {
    reject(event.data?.id, Error('Source worker input queue exceeded 512 messages'));
    return;
  }
  const arrived = performance.now();
  const backlog = queued++;
  chain = chain.then(() => dispatch(event, arrived, backlog)).finally(() => { queued--; });
};
