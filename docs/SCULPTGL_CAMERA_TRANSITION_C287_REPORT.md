# C287 camera-transition slice — bounded result

This implements a display-only 200 ms quartic transition for explicit camera
actions in OBrush: Center/Fit, Front, Left, Top, Source-profile Snap view, and
Source-profile Reset view. Direct pointer contact, orbit/pan/zoom, source
keyboard movement, camera scaling/framing changes, or scene revision cancels
the animation. Scene-load framing remains immediate. The document and history
are not touched by the transition host.

## Pinned inputs and scope

- SculptGL upstream commit: `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.
- `src/math3d/Camera.js` SHA-256:
  `da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe`.
- `src/gui/GuiCamera.js` SHA-256:
  `c23c8008273ee949e0eba3e91cc690a9eba878956d03edc243d12ad45f9a0b97`.
- OBrush base commit before this slice:
  `7d50940b98b1d608dc6f0a769982a91f70bf37b1`.
- Frozen authored off-axis 0/50/100/150/200 ms scalar/quaternion sample
  and acceptance gates: `docs/SCULPTGL_CAMERA_TRANSITION_C287_PREREG.md`.

The pinned source uses 200 ms `easeOutQuart` for view snapping and move-to,
sampled by a 16.6 ms `setInterval`. Its ordinary rotate/translate inertia is
disabled. OBrush samples the same continuous curve on GUI repaint; the
browser's actual frame cadence is independent of the reference timer.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo test --workspace --locked --quiet`: pass, including all three new
  authored camera-transition tests and the existing workspace suite.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo clippy --target wasm32-unknown-unknown -p obrush-core -p obrush-app
  --lib --locked -- -D warnings`: pass.
- `WASM_BINDGEN_BIN=/Users/Ajab/AI/OBrush/.tools/wasm-bindgen/bin/wasm-bindgen
  bash scripts/build-web.sh`: pass. The lockfile-matched CLI is 0.2.129.
- `bash scripts/package-macos.sh`: pass, including ad-hoc signature
  verification.

The isolated normal browser build was served from this worktree at localhost
port 8793; the root 8792 server was left untouched. Fresh default-sphere UI
actions changed the camera gizmo endpoints for Left, Top, and Front. Source
Reset displayed projection zoom `80.00` and the small centered sphere;
Center/Fit displayed zoom `4.05134` and refilled the viewport. The visible
vertex/face counts remained 40,962/81,920.

For an intermediate-time witness, a temporary observer was added only to the
ignored `web/dist` build. It sampled the existing read-only
`window.obrushBrushMetrics().camera` each `requestAnimationFrame` after a
trusted canvas UI click. The Left action returned these observed rows
(`t` measured from pointer-up, not the Rust transition start):

| t (ms) | active | camera view-axis sample |
| ---: | :---: | :--- |
| 14 | true | `[0.9394,-0.0614,0.3374]` |
| 41 | true | `[0.7500,0.0168,-0.6612]` |
| 75 | true | `[0.3345,0.0159,-0.9423]` |
| 108 | true | `[0.1036,0.0061,-0.9946]` |
| 158 | true | `[0.0056,0.0004,-1.0000]` |
| 192 | true | `[0,0,-1]` |
| 208 | false | `[0,0,-1]` |

A separate Front UI click followed by a real browser wheel-scroll action
completed the scroll 53 ms after the click. The observer saw `active=true`
at 7/28/43 ms, then `active=false` by 60 ms; camera distance changed
`4 → 5.13610172`, while the interrupted view axis remained
`[0.7693,0,-0.6389]` through 338 ms. No delayed snap occurred. The click
itself took 34 ms. This is a bounded zoom-interruption witness, not a proof
for all pointer and keyboard gestures.

SHA-256 of produced artifacts:

- portable browser WASM: `1380a9c6bbfaee43d637846803bdaddf941dba8f43893af747ccf3323778737d`
- SIMD browser WASM: `aaab52885dd8a86820a693fb5068f7879eff4e2df7fe2c3780a4427fa568e488`
- packaged macOS executable: `0b984f39509df2178184f272cfc61dfa84f357fc2f38bd28ebf17bbc47f3a79f`

## Failed checks and limits

The first compilation failed because adding a diagnostic field within an
already-large `serde_json::json!` literal exceeded Rust's macro recursion
limit. The field was moved to a separate assignment; the full workspace
suite then passed. The first worktree Web build failed because it had no local
wasm-bindgen CLI and the global CLI was 0.2.104; the lockfile-matched 0.2.129
CLI in the primary checkout was explicitly selected and the build passed.
The first browser sampler read sculpt metrics, which do not contain camera
state; it was corrected to read brush metrics and then produced the traces
above. These failures were not silently treated as passes.

The camera endpoint math and picked-pivot/scene bounds behavior are separate
scopes. The Source profile still reports static camera policy, and this work
does not establish its full camera matrix, scene octree, timer jitter,
shortcut, or ZBrush parity. In particular, OBrush has a single camera target
where SculptGL animates center and offset separately; the explicit Snap view
button supplements OBrush's existing per-motion Shift snap and is not a
claim of exact SculptGL keydown routing.
