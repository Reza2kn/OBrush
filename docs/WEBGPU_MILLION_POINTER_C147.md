# C147 million-vertex WebGPU pointer check

The C146 opt-in Inflate path was run against the existing independently
generated `independent-id-sphere-1m.sgl` fixture: 999,002 vertices, 1,998,000
triangles, SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`.
This is a local browser integration and scaling check, not a matched SculptGL
brush-response or performance pass. The normal studio still uses WASM CPU
geometry and WebGL2; WebGPU requires `?webgpu-preview=1` and currently runs
one Inflate stroke on one active mesh.

The first real browser run rendered the sphere on WebGPU but did not hit it.
The imported Source orthographic camera emits a signed near-plane ray from
world Z about 1,875, while the previous GPU bridge limited travel to about
327 units. The bridge now calculates ray travel to the actual far clip plane
from the captured camera frame and bounds it to the resident pick domain.
Focused orthographic/perspective camera tests and a real million-vertex browser
pick cover this correction.

The first accepted GPU edit then exposed a WebGL renderer error: the obsolete
source-normal field was retained while rebuilding the edited million-vertex
field, exceeding the per-field scratch/cache budget. The renderer now drops a
stale field before rebuilding; a failed rebuild already removes the object
from its draw cache. The failed viewport image and result remain in
`.artifacts/gpu-pointer-c146/million-r2`; the corrected run is `million-r4`.

`million-r4` and the final `million-dedup` run each delivered three browser
pointer contacts, three GPU dabs, a changed document revision (2 to 3), a
changed normal WebGL viewport and exact visible Undo/Redo recovery. In the
final run the before and Undo viewport SHA-256 are both
`5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`;
the after and Redo SHA-256 are both
`ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7`.
No full GPU position readback occurred; the mapped sparse result was 115,380
bytes, plus bounded count/failure/pick words. The browser reported no
exception, console error or external request.

The 10-second `long-r0` browser run scheduled 231 CDP pointer moves and
received 233 contacts including press/release. It applied 117 dabs, coalesced
116 contacts and passed the same document/viewport/history checks. GPU pick
await time totaled 10,265.9 ms, with a maximum individual pick of 217.5 ms.
Independent requestAnimationFrame intervals had a 49.9 ms median, 50.1 ms P95
and 1,683.2 ms maximum; release completed in 1,711.8 ms, of which 1,614.6 ms
was the sparse readback plus synchronous shared-core commit. The mapped sparse
result was 509,720 bytes. This is not responsive enough for the frozen
million-vertex gates (RAF P95 at most 33.34 ms, release queue empty within
100 ms), and the longer path was not compared against SculptGL on the same
input trace. Different delivered contacts prevent a controlled speed ratio to
the prior WASM runs.

A one-pass nearest-point GPU pick experiment was built and run as
`long-nearest-r0`: measured pick await time remained about 10.3 seconds, and
its repeated-lap result was rejected by the shared-core fold guard. That
shortcut was removed; its failure is retained. Removing one redundant
candidate validation before `Document::replace_mesh` preserved the final
three-dab viewport/history result, but commit measured 1,604.8 ms before and
1,597.7 ms after, so no speedup is claimed. The remaining CPU commit and
GPU full-triangle pick costs are open engineering work.

Browser records and before/after/undo/redo PNGs are in
`.artifacts/gpu-pointer-c146/{million-r0,million-r1,million-r2,million-r3,million-r4,long-r0,long-nearest-r0,million-restored,million-dedup}`.
Failed attempts remain available. These checks cover one camera/mesh and
Inflate stroke; they do not establish pressure behavior, other brushes,
multi-subtool WebGPU display, long-stroke quality or SculptGL parity.

Final-source validation: `cargo fmt --check`, 2,475 workspace tests across
80 reported suites (zero failures), strict all-target native Clippy, strict
core/app/GPU WASM Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`
and the final `million-dedup` owned-browser check passed. The reports are in
`.artifacts/gpu-pointer-c146/`. The final web artifact SHA-256 values are
`38516fa75d23d69e616d1ca1192c44ee22675c053a0bf5ed8c5bd17cde7c4233`
for `obrush_app_bg.wasm`,
`c52a13d1665da2eb4d6fdc2e40b8ed44ebb811eb79ee78499526eb681e12974d`
for `resident_bg.wasm`, and
`783aa3ca64fd459a8c8b1279b69af6a6651a06a9788dcd67afa94ce1067aa93d`
for `resident-pointer.js`. The macOS package passed local signing validation.
