# C334 picked pivot receipt — scoped candidate integrated, parity open

Run on 2026-10-20 in the isolated `codex/camera-picked-pivot` worktree, before
Cargo or browser validation:

`OBRUSH_SHARED_ROOT=/Users/Ajab/AI/OBrush node scripts/audit-sculptgl-camera-pivot-c334-static.mjs`

Exit 0. The script confirmed pinned `Camera.js` SHA-256
`da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe`
and authored OBJ SHA-256
`a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b`.
The static checker itself has SHA-256
`39aee7d4bc178bff519875b9013a4e1cb3a34a5aac3fa4d2a19f65de8603b4a3`.
It evaluated two successive picked points with identity and 36° Y rotation,
for perspective and orthographic views. Maximum independent camera-space
component difference was `8.881784197001252e-16` in perspective and
`3.3306690738754696e-16` for orthographic X/Y. The static test deliberately
does not assert orthographic depth because the pinned reference clears its
depth offset on a picked pivot.

`git diff --check` also passed. Neither check compiles or runs the candidate
product code, proves source Float32 rounding, or exercises the ordinary
browser. The acceptance gates in the preregistration remain pending.

The first focused Cargo compile failed with `E0425` at
`source_camera_host.rs:314-315`: the offset had been placed in the
`fit_bounds` temporary frame, where `candidate` is not in scope. The candidate
was corrected so the fit calculation retains its original zero offset and
the actual prepared view consumes the picked offset. The focused camera test
then passed, as did all seven navigation tests and the complete workspace
test command. This failed compile remains part of the receipt; browser and
strict-check results must be reported separately.

Strict native workspace/all-target Clippy initially failed on a candidate-local
`unwrap` after `is_some` in `renderer.rs:408`. The branch now captures the
immutable camera values before taking a mutable Source camera borrow, and
the native command passed on rerun. `cargo fmt --all -- --check`, strict
`wasm32-unknown-unknown` workspace/lib Clippy, and the official
`scripts/build-web.sh` also passed. The built portable browser WASM SHA-256 is
`769f8aa9544e7e5c1ea39c23e9e92184743734d5a4d02cb3d8bad3cb0b90bb6a`.
At that stage the normal browser protocol and macOS package gate were still
pending; the later sections record the browser run. This is not a completed
camera parity result.

The first owned OBrush normal-browser run imported the authored four-vertex
OBJ through the visible file route at 1024×768 and passed the stationary
perspective Pivot-on/off cases. The second run also passed the 40 CSS-pixel
orbit and orthographic Pivot-on checks. Its retained result is
`.artifacts/camera-picked-pivot-c334-root-ui/a855031e-eca4-4ce9-bdd5-12d1dc194c38/result.json`.
Perspective held view deltas were `1.67e-16` (on) and `0` (off), and the
orthographic projected point shifted `0` CSS pixels on press. The 40-pixel
drag changed view components by `0.319`, `0.311`, and `0.307` respectively.
These are OBrush-only observations, not paired equivalence.

The first pinned Source browser attempt failed because the visible checkbox
label is `Picking pivot`, not the presumed `Pivot`. The second failed its
frozen viewport gate: after using the normal camera menu the canvas measured
1024×808, not 1024×768. Neither failed run is counted as a parity result.

The paired normal-browser run then used a bounded visible-window resize to
reach an **actual measured** 1024×768 scene viewport and performed Source Fit
before picking, as the frozen protocol requested. The fractional projected
pixel was rounded to the integer delivered browser pixel; a separate
Gauss-Jordan inverse view-projection and authored-triangle ray/face
intersection supplies the expected picked point. This corrects the failed
setup/oracle, retaining the original `0.005` world-unit tolerance. An earlier
Source setup at default Reset zoom had 1024×768 but produced a picked center
roughly `0.055` world units from the chosen sample because the fractional
pixel represented a large world-space interval. That result is retained as a
failed protocol setup, not a product failure.

Final bounded receipts:

- Pinned Source: `.artifacts/camera-picked-pivot-c334-source-ui/5447f332-14a1-4d4e-b8c3-d3617ae43a40/result.json`.
  The integer pixel `(662,499)` picked within `1.27e-6` world units of the
  independent ray hit; Pivot-on stationary view delta was `0`; Pivot-off
  center stayed unchanged. Its 40-pixel orbit changed a view component by
  `0.155615`.
- OBrush: `.artifacts/camera-picked-pivot-c334-root-ui/d55119cb-d0c9-40a0-a864-ac8845ed817a/result.json`.
  Pivot-on integer-ray error was at most `1.56e-8` at `(879,510)` and the
  stationary perspective view delta was `4.44e-16`. Pivot-off center and view
  stayed unchanged. Orthographic projected XY delta was `0` pixels with
  independent ray error `1.96e-8`; a second perspective point had ray error
  `3.44e-8`. All four 40-pixel gestures changed the view; Undo/Redo counts
  stayed `(2,0)`. Normal Fit and Reset both cleared the picked offset to
  `[0,0,0]`.

The paired view matrices are not equal: the two browsers reached different
camera orientation/zoom states through their separate ordinary Fit controls,
and their 40-pixel orbit deltas differ. This is a picked-pivot candidate
check, not a full camera-parity result. The frozen rapid single-frame gesture,
public OBJ byte comparison, DPR-2 case, and browser-backed native package
remain open. No additional build or package was run after these browser-only
gate changes.

After the candidate was merged with the topology and File export changes, the
root workspace test suite, strict native/WASM Clippy, official web build and
local macOS package passed. That combined build has portable WASM SHA-256
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`
and ad-hoc-signed macOS executable SHA-256
`61421c5c1102d7102e1fc25f065fd4189de45d454cee90753b0d21d1995bbcc2`.
The native package was built and signature-verified, but its picked-pivot
interaction has not been exercised in a native window.
