// Inject with Page.addScriptToEvaluateOnNewDocument before loading the studio.
// Read-only on-demand WebGL2 active-draw VBO audit; it never substitutes a
// project file or changes brush/render state. Capture after a normal RAF.
(() => {
  const contexts = [];
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (kind, ...args) {
    const gl = originalGetContext.call(this, kind, ...args);
    if (kind !== 'webgl2' || !gl || contexts.some(row => row.gl === gl)) return gl;
    const sizes = new Map();
    const state = { gl, sizes, wantedBytes: 0, drawBuffers: new Set(), drawCount: 0 };
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
      if (!state.wantedBytes) return;
      state.drawCount++;
      const count = gl.getParameter(gl.MAX_VERTEX_ATTRIBS);
      for (let index = 0; index < count; index++) {
        if (!gl.getVertexAttrib(index, gl.VERTEX_ATTRIB_ARRAY_ENABLED)) continue;
        const buffer = gl.getVertexAttrib(index, gl.VERTEX_ATTRIB_ARRAY_BUFFER_BINDING);
        if (buffer && sizes.get(buffer) === state.wantedBytes) state.drawBuffers.add(buffer);
      }
    };
    const drawElements = gl.drawElements;
    gl.drawElements = function (...params) { captureDraw(); return drawElements.apply(this, params); };
    const drawElementsInstanced = gl.drawElementsInstanced;
    gl.drawElementsInstanced = function (...params) { captureDraw(); return drawElementsInstanced.apply(this, params); };
    return gl;
  };
  async function hex(bytes) {
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return Array.from(new Uint8Array(digest), x => x.toString(16).padStart(2, '0')).join('');
  }
  window.__sourceGpuReadback = {
    async capture(vertexCount) {
      if (!Number.isSafeInteger(vertexCount) || vertexCount < 3 || vertexCount > 1_000_000)
        throw Error('GPU audit vertex count exceeds frozen bound');
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
      const hits = contexts.flatMap(state => [...state.drawBuffers].map(buffer => ({ state, buffer })));
      for (const state of contexts) state.wantedBytes = 0;
      if (hits.length !== 1) return { active_vbo_candidates: hits.length,
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
      for (let i = 0; i < vertexCount; i++) {
        positions.set(raw.subarray(i * 40, i * 40 + 12), i * 12);
        normals.set(raw.subarray(i * 40 + 12, i * 40 + 24), i * 12);
      }
      return { active_vbo_candidates: 1, draw_count: state.drawCount,
        readback_bytes: raw.byteLength, actual_readback_sha256: await hex(raw),
        position_words_sha256: await hex(positions), normal_words_sha256: await hex(normals),
        gl_error: gl.getError() };
    }
  };
})();
