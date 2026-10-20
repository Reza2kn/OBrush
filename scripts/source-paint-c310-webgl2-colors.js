// C310 read-only color extension of the Brush Lock active-draw VBO audit. A stage can be quiet after
// Save/Undo/Redo, so retain the last actually drawn VBO identity and read its
// current bytes when no later frame is drawn. No app state is changed.
(() => {
  const contexts = [];
  const bufferTokens = new WeakMap();
  let nextBufferToken = 0;
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (kind, ...args) {
    const gl = originalGetContext.call(this, kind, ...args);
    if (kind !== 'webgl2' || !gl || contexts.some(row => row.gl === gl)) return gl;
    const sizes = new Map();
    const state = { gl, sizes, wantedBytes: 0, drawBuffers: new Set(),
      retainedDrawBuffers: new Set(), lastDrawSequence: new Map(),
      drawCount: 0, retainedDrawCount: 0 };
    contexts.push(state);
    const bind = gl.bindBuffer;
    gl.bindBuffer = function (target, buffer) {
      if (target === gl.ARRAY_BUFFER) state.bound = buffer;
      return bind.call(this, target, buffer);
    };
    const bufferData = gl.bufferData;
    gl.bufferData = function (target, data, ...rest) {
      if (target === gl.ARRAY_BUFFER && state.bound) {
        const bytes = typeof data === 'number' ? data : data?.byteLength;
        if (Number.isSafeInteger(bytes)) sizes.set(state.bound, bytes);
      }
      return bufferData.call(this, target, data, ...rest);
    };
    const captureDraw = () => {
      state.retainedDrawCount++;
      if (state.wantedBytes) state.drawCount++;
      const count = gl.getParameter(gl.MAX_VERTEX_ATTRIBS);
      for (let index = 0; index < count; index++) {
        if (!gl.getVertexAttrib(index, gl.VERTEX_ATTRIB_ARRAY_ENABLED)) continue;
        const buffer = gl.getVertexAttrib(index, gl.VERTEX_ATTRIB_ARRAY_BUFFER_BINDING);
        if (buffer) {
          if (!bufferTokens.has(buffer)) bufferTokens.set(buffer, ++nextBufferToken);
          state.retainedDrawBuffers.add(buffer);
          state.lastDrawSequence.set(buffer, state.retainedDrawCount);
          if (sizes.get(buffer) === state.wantedBytes) state.drawBuffers.add(buffer);
        }
      }
    };
    const drawElements = gl.drawElements;
    gl.drawElements = function (...params) { captureDraw(); return drawElements.apply(this, params); };
    const drawElementsInstanced = gl.drawElementsInstanced;
    gl.drawElementsInstanced = function (...params) { captureDraw(); return drawElementsInstanced.apply(this, params); };
    const deleteBuffer = gl.deleteBuffer;
    gl.deleteBuffer = function (buffer) {
      state.retainedDrawBuffers.delete(buffer);
      state.drawBuffers.delete(buffer);
      state.lastDrawSequence.delete(buffer);
      sizes.delete(buffer);
      return deleteBuffer.call(this, buffer);
    };
    return gl;
  };
  async function hex(bytes) {
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return Array.from(new Uint8Array(digest), x => x.toString(16).padStart(2, '0')).join('');
  }
  function base64(bytes) {
    // Bounded diagnostic transfer. Avoid spreading 1.2 MB of byte arguments.
    let encoded = '';
    for (let i = 0; i < bytes.length; i += 0x8000)
      encoded += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return btoa(encoded);
  }
  window.__sourceGpuReadback = {
    async capture(vertexCount, includeNormalWords = false) {
      if (!Number.isSafeInteger(vertexCount) || vertexCount < 3 || vertexCount > 1_000_000)
        throw Error('GPU audit vertex count exceeds frozen bound');
      if (includeNormalWords && vertexCount > 100_000)
        throw Error('Raw normal-word diagnostic exceeds 100k-vertex bound');
      const wantedBytes = vertexCount * 40;
      if (wantedBytes > 64 * 1024 * 1024) throw Error('GPU audit byte cap');
      for (const state of contexts) {
        state.wantedBytes = wantedBytes;
        state.drawBuffers.clear();
        state.drawCount = 0;
      }
      for (let frame = 0; frame < 120; frame++) {
        await new Promise(resolve => requestAnimationFrame(() => resolve()));
        if (contexts.some(state => state.drawBuffers.size)) break;
      }
      let hits = contexts.flatMap(state => [...state.drawBuffers].map(buffer => ({ state, buffer })));
      const observation = hits.length ? 'fresh_draw' : 'retained_active_draw';
      if (!hits.length) hits = contexts.flatMap(state => [...state.retainedDrawBuffers]
        .filter(buffer => state.sizes.get(buffer) === wantedBytes).map(buffer => ({ state, buffer })));
      for (const state of contexts) state.wantedBytes = 0;
      if (hits.length !== 1) return { active_vbo_candidates: hits.length, observation,
        draw_count: contexts.reduce((sum, state) => sum + state.drawCount, 0),
        readback_bytes: 0, failure: 'Missing or ambiguous active draw VBO' };
      const { state, buffer } = hits[0], gl = state.gl;
      const prior = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
      const raw = new Uint8Array(wantedBytes);
      try {
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.getBufferSubData(gl.ARRAY_BUFFER, 0, raw);
      } finally { gl.bindBuffer(gl.ARRAY_BUFFER, prior); }
      // OBrush's normal mesh VBO has ten little-endian f32 per vertex:
      // xyz position, xyz normal, and four further attributes. Hash only the
      // relevant six words, while also hashing the complete actual readback.
      const positions = new Uint8Array(vertexCount * 12);
      const normals = new Uint8Array(vertexCount * 12);
      const colors = new Uint8Array(vertexCount * 12);
      for (let i = 0; i < vertexCount; i++) {
        positions.set(raw.subarray(i * 40, i * 40 + 12), i * 12);
        normals.set(raw.subarray(i * 40 + 12, i * 40 + 24), i * 12);
        colors.set(raw.subarray(i * 40 + 28, i * 40 + 40), i * 12);
      }
      return { active_vbo_candidates: 1, observation, draw_count: state.drawCount,
        retained_draw_count: state.retainedDrawCount,
        buffer_token: bufferTokens.get(buffer),
        last_draw_sequence: state.lastDrawSequence.get(buffer),
        readback_bytes: raw.byteLength, actual_readback_sha256: await hex(raw),
        position_words_sha256: await hex(positions), normal_words_sha256: await hex(normals),
        color_words_sha256: await hex(colors), color_words_base64: base64(colors),
        ...(includeNormalWords ? { position_words_base64: base64(positions),
          normal_words_base64: base64(normals) } : {}),
        gl_error: gl.getError() };
    }
  };
})();
