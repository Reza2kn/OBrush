# OBrush 0.1 validation

Recorded **2026-10-12** on Apple M2 / ARM64, macOS **26.5.1**, Rust **1.96.0**. Browser runtime checks used Chrome **155.0.8059.39** and the Codex in-app browser. This report establishes a local sculpting foundation and one interchange comparison. It does not establish ZBrush feature parity.

## Build identity

| Artifact | SHA-256 |
| --- | --- |
| Local ad-hoc signed macOS bundle executable | `d1016cb942106fbb52c3b85786fc410494544d3783cd4bfdef020a7d75ed0f0f` |
| Raw `target/release/obrush` | `7414f7dcd720a33b0cf77e984261afcfdfd78a06490daba909ce3e482b96a057` |
| Generated `web/dist/pkg/obrush_app_bg.wasm` | `cfc4bf37330f6c5c252fa34bd2b7a203c21ff3b321161b13b148bf55ca60520f` |
| `Cargo.lock` | `b020b262b57f4a1b27d3894273596eccde20f8bd37db4e952dda1b73ccdb4490` |

The bundle executable differs from the Cargo binary because packaging adds an ad-hoc app signature. The bundle launched successfully after this step; `codesign --verify --deep --strict` passes. [REA release evidence](evidence/obrush-native-inspection.json) independently binds the bundle/raw build identities. Source is local and uncommitted; no published release, CI execution, notarization or Windows/Linux runtime is claimed.

## Completed automated checks

```sh
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p obrush-app --target wasm32-unknown-unknown -- -D warnings
scripts/build-web.sh
scripts/package-macos.sh
```

All completed successfully after the final core change: **36 tests** (25 core unit, 5 MRGB import, 2 workflow integration, 3 camera and 1 shortcut regression). The WASM build and strict WASM lint both compile the real shared app/core, not a separate JavaScript sculpt implementation.

Tests cover outward/shared primitive topology, nearest raycast and coordinate scales, backface/mask protection, inverted brushes, stable smoothing, flattening, mirror seam behavior, vertex colors, attribute subdivision, grouped strokes and bounded history, malformed mesh/OBJ/project data, concave polygons/negative indices, project versions, bounded serialization/allocation, derived-normal reconstruction, MRGB decoding and viewport-ray correspondence. They establish the tested OBrush invariants; they do not exhaust every reference acceptance gate.

The CPU timings in [the core README](../crates/obrush-core/README.md) exclude rendering, UI, IO/history and browser overhead. No FPS, interactive million-vertex capacity or ZBrush performance equivalence is asserted.

## Live runtime observations

| Surface | Completed observation | Evidence/limit |
| --- | --- | --- |
| Native macOS app | Launch GPU viewport with the studio theme; import a browser-saved project; final signed build import/display the ZBrush-exported OBJ/colors; sculpt, Cmd-Z undo/Cmd-Shift-Z redo, and native project save | Local `.artifacts/native-final.png`; complete camera gesture matrix remains unaudited |
| In-app browser, compact 515×598 view | Draw surface stroke, Paint stroke, project download, project reimport, OBJ download, wireframe, whole-stroke undo and repaired Cmd-Shift-Z redo | Local `.artifacts/browser-compact.jpg`; modal object/surface settings keep the viewport usable |
| Chrome, desktop 1633×830 view | Import/display painted project; import ZBrush OBJ and the native-saved sculpt; display decoded MRGB colors; export decoded vertex RGB; paint visible mask; subdivide and undo topology | Local `.artifacts/browser-desktop.jpg`; final console warning/error log was empty |
| Topology change | Subdivide 10,242 vertices / 20,480 triangles to 40,962 / 81,920; viewport renders; Cmd-Z restores the original counts and mask appearance | Live Chrome observation; triangle midpoint refinement, not persistent subdivision levels |
| Browser file contents | Downloaded project contained 10,242 vertices, 20,480 faces, 419 vertices with radius deviation >1e-4 and 435 painted vertices | Independently parsed the actual downloaded `.obrush` JSON; file SHA `d665f765667e947c14a08b70677c5cbcf83f662f08bcd5f22aa91d8c2a6110b8` |

Draw/Paint compact-view input used (335,285)→(390,340) and (315,290)→(400,320), radius 0.18, strength 0.35 and X symmetry enabled. These are UI smoke inputs, not pressure/time-normalized golden brush traces. Chrome mask input used (675,350)→(930,390). Camera ray/projection consistency across portrait/landscape ratios is numerically tested; the full orbit/pan/pen/touch gesture matrix is not live-certified.

The final native-saved fixture has 152 vertices changed by more than 1e-7 relative to the imported reference, maximum displacement 0.004697592927244736, and SHA `0e6d64867c58684f05505d42ccf5ae3c63721e7a5134d79bfe8341cb067eced6`. Native input was (1350,860)→(1550,1000), Draw radius 0.18 / strength 0.35 / X symmetry. The actual native-saved `.obrush` file was then imported and rendered in Chrome.

The temporary in-app viewport override was reset. Browser build files are served on **127.0.0.1:8787** by `scripts/serve-web.py`; the app needs no external API/backend. The local native bundle and browser tab are available for review. Raw visual fixtures remain ignored local artifacts, not commercial source/assets.

## Measured ZBrush interchange

Reference: the user's installed desktop **ZBrush 2026.2.1**. Procedure: OBrush browser **Export OBJ** → ZBrush **Tool Import** → select imported tool → **Tool Export OBJ** → OBrush import. The existing ZBrush document was not cleared. No reference sculpt stroke was executed in this comparison.

[Round-trip evidence](evidence/obj-zbrush-roundtrip.json) records:

- All **10,242 vertices** and **20,480 faces** preserved; face-index arrays identical.
- Corresponding-index maximum position distance **4.690415757354602e-8**; RMS **2.043570307341458e-8**.
- ZBrush rewrote inline floating-point RGB to 8-bit `#MRGB` comments. Observed source/reference maximum color-component difference **0.0037254901960783737**.
- The importer was fixed after observing that gap. A live browser re-export has all 10,242 decoded RGB records; first value `[0.64705884, 0.6862745, 0.7372549]` matches `[165,175,188]/255` within f32 rounding.
- Synthetic nonuniform MRGB fixtures verify ordering/multiple lines, precedence, malformed data and count/size limits. The leading mask byte is validated but **ignored**; OBJ mask compatibility is unimplemented.

This exploratory fixture did not preregister numerical acceptance tolerances and contains default source colors. Treat its measured errors as observations. Nonuniform painting/project transfer was checked separately; comprehensive masked/UV/material OBJ interoperability and brush-response parity remain open.

Reproduce geometry/RGB measurements from retained local fixtures:

```sh
python3 scripts/compare-obj.py .artifacts/obrush-export.obj .artifacts/zbrush-roundtrip.obj
```

The script assumes corresponding vertex order and reports hashes/counts/errors; it does not invent a nearest-point registration or assert parity.

## Failures found and resolved

- Browser startup initially panicked in glow's WebGL object-binding lookup for null/external resources. The renderer now avoids that unsupported lookup and restores scalar state while egui reinstates its own bindings; subsequent live startup/edit/wireframe checks passed.
- The native system light theme replaced parts of the custom studio theme. The app now explicitly chooses its dark theme; final native visual check passed.
- Undo/redo/file shortcuts relied on modifiers still being held at frame time and could be blocked by toolbar focus. They now consume key-event modifiers; a regression test and live redo/topology-undo checks passed.
- Replacing the running app executable in place led to a macOS code-signature failure on relaunch. Packaging now uses a fresh inode and signs/verifies the complete local app; final launch/import passed. The bundle is local ad-hoc signed, not notarized for distribution.
- The temporary eframe screenshot helper captured a black buffer. It was removed; actual native/browser screenshots provide visual evidence.

REA was built locally from its pinned source; Ghidra/JDK and a compiled ARM64 decompiler pass readiness. The focused `ClayBuildup` query hit its 120-second startup deadline after 123.81 seconds and returned **no algorithm findings**. No larger retry or proprietary-source reconstruction is claimed. See [REA_WORKFLOW.md](REA_WORKFLOW.md).

## Remaining limits

One mesh; nine initial brush operations; no tablet pressure, adaptive topology, persistent multi-resolution, SubTools, UV/material IO, native ZPR/ZTL, advanced brush/alpha libraries, booleans, dynamics, production renderer, scripting/plugin compatibility or autosave/recovery. Project files preserve geometry/masks/colors, not session history/camera/settings. Large edits run on the UI thread; validation limits are not performance guarantees. Browser reload/close can lose unsaved edits.

[FEATURE_PARITY.md](FEATURE_PARITY.md) retains all open families and separates local checks from reference comparisons. No feature has been promoted to complete ZBrush parity.
