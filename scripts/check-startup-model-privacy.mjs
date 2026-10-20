// Host-only privacy regression. Browser execution remains a separate gate.
import assert from 'node:assert/strict';
import { createStartupModelInput } from '../web/startup-model-input.js';

const token = 'unique_model_token_5749';
const url = `https://user:${token}@fixture.invalid/tetra.obj?token=${token}`;
const events = [];
const handle = {
  load_startup_model_bytes(id, bytes) { events.push({ kind: 'load', id, bytes: Array.from(bytes) }); },
  fail_startup_model(id, message) { events.push({ kind: 'fail', id, message }); },
};
const canvas = { focus() { events.push({ kind: 'focus' }); } };
const notices = [];
const oldFetch = globalThis.fetch;
const wait = () => new Promise(resolve => setTimeout(resolve, 5));
try {
  const input = createStartupModelInput(() => handle, canvas, text => notices.push(text), 'https://fixture.invalid/');
  globalThis.fetch = async () => { throw new Error(`Browser fetch echoed ${url}`); };
  input.start(1, url);
  await wait();
  assert.equal(events[0]?.kind, 'fail');
  assert.equal(events[0]?.message, 'Model URL read failed. Check URL, CORS and server response.');
  assert(!JSON.stringify({ events, notices }).includes(token));

  events.length = 0;
  notices.length = 0;
  globalThis.fetch = async () => ({ status: 404, headers: new Headers(), body: null });
  input.start(2, url);
  await wait();
  assert.equal(events[0]?.message, 'Model GET requires HTTP200; returned 404.');
  assert(!JSON.stringify({ events, notices }).includes(token));

  events.length = 0;
  notices.length = 0;
  globalThis.fetch = async (requestUrl, options) => {
    assert.equal(requestUrl.href, url);
    assert.equal(options.credentials, 'omit');
    return {
      status: 200,
      headers: new Headers({ 'content-length': '4' }),
      body: new ReadableStream({ start(controller) { controller.enqueue(new Uint8Array([0, 255, 4, 7])); controller.close(); } }),
    };
  };
  input.start(3, url);
  await wait();
  assert.deepEqual(events[0], { kind: 'load', id: 3, bytes: [0, 255, 4, 7] });
  assert.deepEqual(events[1], { kind: 'focus' });
  assert(!JSON.stringify({ events, notices }).includes(token));
  console.log('startup model URL privacy and byte preservation: passed');
} finally {
  globalThis.fetch = oldFetch;
}
