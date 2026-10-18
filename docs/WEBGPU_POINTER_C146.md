# C146 opt-in WebGPU pointer handoff

The normal browser studio still uses WASM CPU geometry and WebGL2. This change
adds an **opt-in** `?webgpu-preview=1` button for one WebGPU Inflate stroke on
the captured active mesh. It is an integration probe, not a SculptGL/ZBrush
brush match or evidence of sustained speed. The shared Rust document retains
validation, file state and Undo/Redo authority.

The capture sends a bounded, versioned, SHA-256 checked binary mesh/pose upload
instead of a full `.obrush` JSON string. Core tests compare the decoded sphere
and original-face fixtures with the existing capture row for row, including
mask, color, triangles and original faces; malformed length, checksum, counts,
attributes and triangles reject. The browser overlays a separately owned GPU
canvas on the studio viewport, uses the studio camera ray, coalesces pointer
motion, and commits compact sparse changed positions through a one-use core
token on release. Escape, blur, document/view change and GPU errors cancel.

The final owned-Chrome run is
`.artifacts/gpu-pointer-c146/browser-final/result.json` with the generated
WebGPU preview and before/after/undo/redo viewport PNGs alongside it. On the
40,962-vertex startup sphere at 1140×825 CSS pixels and device scale 1, a real
CDP mouse press/move/release delivered three contacts and three GPU dabs.
The `.obrush` project SHA-256 changed from
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
to `05700eec778de9fc8f0bf77310266f0c9e3cde1d3d41ab8d5297de845d0a2a68`.
The viewport PNG SHA-256 changed from
`8259e213d90c586b07ba9555d45f60972ff4b9493857c1261a9bd29c25883b8d`
to `0511e9128e91171514470f5a1ad64206bba43753020c63788e7c6fc421921816`.
Undo and Redo reproduced those respective viewport hashes exactly. GPU mapped
sizes were 8, 8, 8, 4, 4, 6680 and 4 bytes, with no whole-position readback;
there were no browser exceptions, console errors or external requests.

The failed browser attempts are preserved in `browser-r2` through `browser-r8`.
The material integration failure in `browser-r5` rejected a non-unit studio ray;
normalizing the emitted ray in f64 fixed the GPU pick. The first complete
stroke in `browser-r6` changed the project, but that harness tried to start a
fresh capture after Undo while a redo branch existed. `browser-r7` compared
whole-window screenshots and failed because status text outside the viewport
changed; viewport-only comparison passed in the final run. The failures are
not erased or counted as passes.

Validation: `cargo fmt --check`; `cargo test --workspace --locked` (2,474
passed, zero failed); strict native all-target and core/app/GPU WASM Clippy;
`scripts/build-web.sh`; `scripts/package-macos.sh`; and the final owned browser
run passed. The first strict native Clippy run failed on a manual-clamp style
warning, which was fixed and passed on rerun. Final build SHA-256:

| Artifact | SHA-256 |
| --- | --- |
| `web/dist/pkg/obrush_app_bg.wasm` | `e7c2dcda3ca5b2b53a4e59944f11e5b7d60c6a6ae277843385dd45098b79e4c6` |
| `web/dist/gpu-pkg/resident_bg.wasm` | `7303a030829f2d7c5f1d6be43d815fb936850956a8aef630e4088a4efa82bfc6` |
| `web/dist/resident-pointer.js` | `fb45e2ac967346e47632577988e303db2edb3f2d93cdcbf0f970acce7a73599a` |

Open: this is Inflate only, with one captured active mesh, one stroke and an
extra full GPU upload at entry. It does not establish long-stroke latency,
million-vertex responsiveness, brush response matching, multi-subtool
composition, pressure behavior or production replacement of the WebGL2 path.
