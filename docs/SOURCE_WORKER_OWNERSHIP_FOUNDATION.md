# Source worker ownership foundation

This change introduces a **shared-core ownership boundary**, not an ordinary
browser Worker brush. `SourceWorkerTransaction` moves one complete `Document`
and its `SourceGeometryStroke` into a single owner. The ordinary Source
selection-history entry, retained normal state, mask admission, geometry
validation, storage caps and release/cancel behavior remain the existing core
implementations. The wrapper has no geometry implementation and no renderer or
browser dependency. It can prepare screen/ray input against the currently owned
mesh or accept an already prepared controlled contact pair.

The host supplies a nonzero epoch and exact one-based dab sequence. A stale
epoch, repeated number, gap or invalid input does not advance the sequence.
Release and cancel require the next number. A rejected release leaves the
move-owned transaction available for a correctly ordered retry; a failed core
release returns the canceled transaction so its full document can be recovered.
No public project serialization is used for this ownership step.

The independently written unit fixture is a three-vertex triangle with the
Source `UnprotectedF32V1` mask convention and all mask values `1.0`. It uses
Inflate intensity `0.625`, two contacts centered at `[0, 0, 0]` with squared
radius `4`, pressure `0.5`, and one input-preparation case from a 512×512
orthographic view at screen point `[320, 192]` with radius `220`. The tests
compare public `.obrush` words after two dabs, one-step Undo/Redo and retained
normal `f32` bits against the direct shared-core route. The cancel test checks
the original public state, empty cold normal cache and preserved prior redo.

Validation from base `584344c2340b96e5adba871074202b696fdc3fa7`:

- `cargo test --locked -p obrush-core source_worker_transaction`: four of four
  targeted tests pass. Two-dab public Save and direct-core comparisons are
  byte-identical; the retained normal `f32` word arrays have zero differences.
- `cargo fmt --all -- --check`, strict native all-target Clippy and strict
  `wasm32-unknown-unknown` core-library Clippy pass. The source file SHA-256 is
  `b7af1d1eafcd3963e2c08b5465fd1bbe51fe65a35fc1b1573d73e6b1e31d0fdc`.
- The first strict Clippy run failed on four `result_large_err` warnings for
  returning the complete move-owned document/transaction on rejection. These
  are narrowly allowed with comments: boxing the owner would add a fallible
  allocation on the rejected admission/release path. The rerun passed.

This isolated check did not run the complete workspace suite or a browser.
The combined workspace suite is required after integration; no rendering or
host code changed here.

This ownership value cannot be sent through `postMessage`: separate WASM Worker
realms do not share Rust `Document`/`Arc` memory. A bounded live snapshot or a
worker-owned document from session start, a corresponding main-thread epoch
gate for returned messages, checked sparse **position and normal** publication,
and all UI/camera/alpha/selection routing are still required. The host must
also retain a recoverable baseline for Worker failure. No 100k/1m held-pointer
SP-WORKER-01 run was made, and no responsiveness or SculptGL parity gate is
closed by this core boundary.
