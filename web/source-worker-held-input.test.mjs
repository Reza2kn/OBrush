import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createHeldSourceWorkerInput } from './source-worker-held-input.js';

function harness({ pen = false } = {}) {
  const listeners = new Map();
  const canvas = {
    clientWidth: 1000, clientHeight: 800,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 1000, height: 800 }),
    setPointerCapture() {}, releasePointerCapture() {},
  };
  globalThis.document = { addEventListener(type, callback) { listeners.set(type, callback); } };
  globalThis.window = { addEventListener() {} };
  const workers = [];
  globalThis.Worker = class {
    constructor() { this.sent = []; this.terminated = false; workers.push(this); }
    postMessage(message) { this.sent.push(message); }
    terminate() { this.terminated = true; }
    reply() {
      const message = this.sent.shift();
      assert.ok(message);
      this.onmessage({ data: {
        id: message.id, kind: message.kind, success: true,
        strokeEpoch: message.strokeEpoch, sequence: message.sequence,
        delta: new Uint8Array(88), retained: new Uint8Array(0),
      } });
      return message;
    }
  };
  let nextEpoch = 1;
  const calls = { cancel: [], commit: [], samples: [], endpoint: [] };
  const wire = (x, y, pressure) => JSON.stringify({ screen: [x, y], pressure });
  const handle = {
    source_worker_held_pointer_in_viewport: () => true,
    source_worker_held_begin(x, y, pressure) {
      return JSON.stringify({ epoch: String(nextEpoch++), revision: '1', project: '{}',
        options: '{}', initial_input: JSON.parse(wire(x, y, pressure)) });
    },
    source_worker_held_seed_packet: () => new Uint8Array(0),
    source_worker_held_samples(epoch, x, y, pressure) {
      calls.samples.push([epoch, x, y, pressure]);
      return JSON.stringify([JSON.parse(wire(x - 1, y, pressure)),
        JSON.parse(wire(x, y, pressure))]);
    },
    source_worker_held_endpoint(epoch, x, y, pressure, isPen) {
      calls.endpoint.push([epoch, x, y, pressure, isPen]);
      return wire(x, y, pressure);
    },
    source_worker_held_preview() {},
    source_worker_held_commit(...args) { calls.commit.push(args); return true; },
    source_worker_held_cancel(epoch) { calls.cancel.push(epoch); },
  };
  const trial = createHeldSourceWorkerInput(handle, canvas, () => {}, 'scalar');
  trial.start();
  const emit = (type, x, y, pressure = pen ? 0.7 : 0.5, buttons = type === 'pointerup' ? 0 : 1) => {
    const event = { type, target: canvas, pointerType: pen ? 'pen' : 'mouse',
      button: 0, buttons, pointerId: 7, clientX: x, clientY: y, pressure,
      isTrusted: true, altKey: false, ctrlKey: false, shiftKey: false,
      metaKey: false, preventDefault() {}, stopImmediatePropagation() {} };
    listeners.get(type)(event);
  };
  return { trial, emit, workers, calls, handle };
}

test('release coalesces backlog but applies the final trusted mouse-up exactly once', () => {
  const { trial, emit, workers, calls } = harness();
  emit('pointerdown', 100, 100);
  emit('pointermove', 110, 100);
  emit('pointermove', 120, 100);
  emit('pointerup', 130, 100);
  const worker = workers[0];
  assert.equal(worker.reply().kind, 'begin');
  const endpoint = worker.reply();
  assert.equal(endpoint.kind, 'apply');
  assert.equal(endpoint.endpoint, true);
  assert.deepEqual(JSON.parse(endpoint.input), { screen: [130, 100], pressure: 0.5 });
  assert.equal(worker.reply().kind, 'finish');
  assert.equal(worker.sent.length, 0);
  assert.equal(calls.commit.length, 1);
  assert.equal(calls.cancel.length, 0);
  assert.equal(trial.status().lastEndpointApplied, true);
  assert.equal(trial.status().lastEndpointDropped, false);
  assert.deepEqual(trial.status().lastEndpointScreen, [130, 100]);
  assert.equal(trial.status().lastEndpointPressure, 0.5);
  assert.equal(trial.status().lastSkippedQueuedDabs, 1);
  assert.equal(trial.status().lastSkippedPendingMove, true);
});

test('release waits for one outstanding dab and preserves last contact pen pressure', () => {
  const { trial, emit, workers, calls } = harness({ pen: true });
  emit('pointerdown', 100, 100, 0.4);
  const worker = workers[0];
  worker.reply(); // begin sends the initial dab
  emit('pointermove', 110, 100, 0.73);
  emit('pointerup', 120, 100, 0);
  assert.equal(worker.sent.length, 1); // only the outstanding initial dab
  assert.equal(worker.reply().kind, 'apply');
  const endpoint = worker.reply();
  assert.deepEqual(JSON.parse(endpoint.input), { screen: [120, 100], pressure: 0.73 });
  assert.equal(worker.reply().kind, 'finish');
  assert.equal(calls.commit.length, 1);
  assert.equal(trial.status().lastEndpointPressureSource, 'last-contact');
  assert.equal(trial.status().lastEndpointPressure, 0.73);
  assert.equal(trial.status().sentDabs, 2);
});

test('duplicate release endpoint is not applied twice; cancel publishes nothing', () => {
  const { trial, emit, workers, calls } = harness();
  emit('pointerdown', 100, 100);
  const worker = workers[0];
  worker.reply(); // begin sends the initial dab
  worker.reply(); // initial dab acknowledged
  emit('pointerup', 100, 100);
  assert.equal(worker.reply().kind, 'finish');
  assert.equal(trial.status().lastEndpointAlreadyCovered, true);
  assert.equal(calls.commit.length, 1);
  emit('pointerdown', 200, 200);
  emit('pointercancel', 200, 200);
  assert.equal(calls.commit.length, 1);
  assert.equal(calls.cancel.length, 1);
  assert.equal(workers[1].terminated, true);
});
