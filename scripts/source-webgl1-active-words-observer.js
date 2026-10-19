// Inject before the pinned SculptGL page. WebGL1 cannot read buffer contents:
// shadow exact forwarded uploads, then identify vertex/normal buffers used by
// the active draw's shader attributes. This does not touch application state.
(() => {
  const contexts = [];
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (kind, ...args) {
    const gl = originalGetContext.call(this, kind, ...args);
    if (!['webgl', 'experimental-webgl'].includes(kind) || !gl || contexts.some(x => x.gl === gl)) return gl;
    const state = { gl, bound: null, uploads: new Map(), draws: [] };
    contexts.push(state);
    const bind = gl.bindBuffer;
    gl.bindBuffer = function (target, buffer) {
      if (target === gl.ARRAY_BUFFER) state.bound = buffer;
      return bind.call(this, target, buffer);
    };
    const bytes = data => data instanceof ArrayBuffer ? new Uint8Array(data) :
      ArrayBuffer.isView(data) ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength) : null;
    const bufferData = gl.bufferData;
    gl.bufferData = function (target, data, ...rest) {
      if (target === gl.ARRAY_BUFFER && state.bound) {
        const source = bytes(data);
        if (source) state.uploads.set(state.bound, new Uint8Array(source));
        else if (Number.isSafeInteger(data) && data >= 0 && data <= 8 * 1024 * 1024)
          state.uploads.set(state.bound, new Uint8Array(data));
      }
      return bufferData.call(this, target, data, ...rest);
    };
    const bufferSubData = gl.bufferSubData;
    gl.bufferSubData = function (target, offset, data, ...rest) {
      if (target === gl.ARRAY_BUFFER && state.bound) {
        const source = bytes(data), dest = state.uploads.get(state.bound);
        if (source && dest && Number.isSafeInteger(offset) && offset >= 0 && offset + source.length <= dest.length)
          dest.set(source, offset);
      }
      return bufferSubData.call(this, target, offset, data, ...rest);
    };
    const capture = () => {
      const program = gl.getParameter(gl.CURRENT_PROGRAM);
      if (!program) return;
      const vertexIndex = gl.getAttribLocation(program, 'aVertex');
      const normalIndex = gl.getAttribLocation(program, 'aNormal');
      if (vertexIndex < 0 || normalIndex < 0) return;
      const vertex = gl.getVertexAttrib(vertexIndex, gl.VERTEX_ATTRIB_ARRAY_BUFFER_BINDING);
      const normal = gl.getVertexAttrib(normalIndex, gl.VERTEX_ATTRIB_ARRAY_BUFFER_BINDING);
      if (!vertex || !normal) return;
      state.draws.push({ vertex, normal, at: performance.now() });
      if (state.draws.length > 64) state.draws.shift();
    };
    for (const name of ['drawElements', 'drawArrays']) {
      const draw = gl[name];
      gl[name] = function (...params) { capture(); return draw.apply(this, params); };
    }
    return gl;
  };
  const hex = async data => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', data)),
    x => x.toString(16).padStart(2, '0')).join('');
  const base64 = data => {
    let raw = '';
    for (let i = 0; i < data.length; i += 0x8000)
      raw += String.fromCharCode(...data.subarray(i, i + 0x8000));
    return btoa(raw);
  };
  window.__sourceActiveWords = {
    async capture(vertexCount) {
      if (!Number.isSafeInteger(vertexCount) || vertexCount < 3 || vertexCount > 100000)
        throw Error('Active-word vertex bound');
      const wanted = vertexCount * 12;
      for (let i = 0; i < 4; i++) await new Promise(resolve => requestAnimationFrame(resolve));
      const hits = contexts.flatMap(state => state.draws.slice(-8).map(draw => ({ state, draw })))
        .filter(({state, draw}) => state.uploads.get(draw.vertex)?.byteLength === wanted &&
          state.uploads.get(draw.normal)?.byteLength === wanted);
      const pairs = [];
      for (const hit of hits)
        if (!pairs.some(row => row.draw.vertex === hit.draw.vertex && row.draw.normal === hit.draw.normal))
          pairs.push(hit);
      if (pairs.length !== 1) return { active_vbo_candidates: pairs.length,
        failure: 'Missing or ambiguous forwarded WebGL1 active draw buffers' };
      const {state, draw} = pairs[0], position = state.uploads.get(draw.vertex),
        normal = state.uploads.get(draw.normal);
      return {active_vbo_candidates: 1, forwarded_upload_bytes: [position.length, normal.length],
        position_words_sha256: await hex(position), normal_words_sha256: await hex(normal),
        position_words_base64: base64(position), normal_words_base64: base64(normal),
        gl_error: state.gl.getError()};
    }
  };
})();
