# OBrush resident GPU bridge

This workspace crate contains the independently written `wgpu` 27 compute,
retained-normal, ray-picking and indexed-drawing prototype for Metal and browser
WebGPU. It consumes `obrush-core::resident_point_program::ResidentPointInput`,
which validates and owns a bounded copy of an existing mesh. The shared core
still handles geometry, project IO and history without a GPU.

The browser studio does not call this crate. Its `ResidentProbe` export is a
separate diagnostic path for full-mesh SGL capture, versioned `.obrush`
active-subtool capture, a pointwise Inflate-like operation and direct canvas
presentation. The project path composes a saved object pose with a world camera
and converts world pick rays in shared Rust. It does not replace or edit the
document. The point operation is **not** the studio Inflate brush; the probe
does not implement complete Source brush, document transaction, undo, or
multi-subtool behavior. Its shader applies the saved pose's inverse-transpose
normal transform, but saved UV/material/render settings remain unsupported.
Do not interpret its
kernel timings as application speedups or SculptGL parity.

The shared core now offers a checked, undoable acceptance boundary for an
explicitly read-back GPU position buffer. It verifies the source mesh and pose,
rejects stale/invalid/folded results and preserves redo on no-op. This bridge is
exercised by `resident_commit_probe`; the normal studio does not call it yet.

The bounded memory contract is checked before GPU allocation. A device error,
invalid input, camera mismatch or cancellation must not silently install
candidate geometry in a document. Before studio integration, connect current
document/stroke ownership, all brush controls, history/save barriers, CPU
fallback and device-loss recovery; then rerun the pinned 60/120-second browser
matrix and reference comparisons. See [the GPU investigation](../../docs/WEBGPU_COMPUTE.md).

The crate is checked in CI for native workspace tests and browser WASM
compilation. A local probe can be built with:

```sh
cargo build --locked -p obrush-gpu --lib --target wasm32-unknown-unknown --profile web-release
.tools/wasm-bindgen/bin/wasm-bindgen --target web --out-dir .artifacts/gpu-probe/pkg --out-name resident target/wasm32-unknown-unknown/web-release/obrush_gpu.wasm
```
