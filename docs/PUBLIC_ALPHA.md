# OBrush public alpha

OBRush is an open-source sculpting studio written in Rust. The same sculpting core runs in a macOS app and in the browser through WebAssembly. **This is a public alpha:** anyone may try it, inspect the code and report what breaks. Draw, Crease, Move, masks, paint, subdivision, multiple parts, OBJ/PLY/STL interchange and versioned `.obrush` projects are available in varying degrees of maturity. [SculptGL and ZBrush compatibility is unfinished](FEATURE_PARITY.md), and no percentage-to-parity claim is made.

To build the current source locally, run `cargo run --locked -p obrush-app --release` for the desktop app. On macOS, `scripts/package-macos.sh` creates `.artifacts/OBrush.app`. For the browser, follow the [WASM prerequisites and build commands](../README.md#browser), then serve the generated `web/dist` locally. **No hosted URL, public binary download, or public Git remote is configured in this checkout.** Add the actual versioned links, commit and artifact SHA-256 values to the public release page before announcing a downloadable build. The [release checklist](PUBLIC_ALPHA_GATE.md) distinguishes this source-build route from a tested installable package.

Save early and often. Browser reload/close loses unsaved work, and OBrush does not yet have autosave or crash recovery. Keep `.obrush` project copies while experimenting. `.obrush` is OBrush's editable project format; OBJ/PLY/STL exports are interchange formats and may not retain all masks, layers, materials or scene state. Untrusted or malformed imports are meant to be rejected without replacing the current document, but please report any failure of that safeguard immediately.

## Known issues we want feedback on

| Area | Current alpha status |
| --- | --- |
| Repeated Draw and worked surfaces | Repeated same-point Draw dabs can stop adding height; after extensive work, marks may look jagged or low-poly. The [stationary failure](DRAW_STATIONARY_BUILD_GATE.md) and [quality gate](PUBLIC_ALPHA_GATE.md#high-priority-quality-acceptance-after-or-alongside-publication) are open. |
| Move and Crease | Small, strong Move grabs can create sharp triangular folds; a bounded [Move improvement](NATIVE_MOVE_GUARD_B02_RESULT.md) passed one fixture, but close-up user-visible tearing is unresolved. Repeated Crease passed one scoped safety check, while its [matched close-up gate](SCULPTGL_CREASE_OVERLAP_CR02_REPORT.md) did not pass. |
| Dense sculpting | A one-million-vertex browser stroke [failed all four responsiveness gates](SCULPTGL_MILLION_RELEASE_ORIGIN_MRP11_REPORT.md). Start with modest meshes; save before costly operations. |
| Pen input | Pressure controls exist, but real physical tablets and native pressure behavior have not been broadly validated. |
| Recovery and formats | No autosave/crash recovery, no ZBrush ZPR/ZTL support, and partial UV/material interchange. Save editable work as `.obrush`. |

## Tested platforms and support scope

| Route | Evidence and limit |
| --- | --- |
| macOS arm64 | Local ad-hoc-signed app; one [packaged Draw/Save/Undo/Redo/Open smoke](NATIVE_UI_SMOKE_2026_10_20.md) passed. No notarized installer or clean-machine public download is yet verified. |
| Browser | Static WASM/WebGL2 path exercised in owned Chrome 155 normal-browser tests. Current local portable WASM SHA-256 is `5719040ac5ed8e6331942dfd62a7452ee712959807ed659da9b4e0f931bd2141`; this is **not** a published build identity. Other browsers and devices need testing. |
| Windows and Linux | Source targets exist but current native GUI execution is unverified. |

When the repository is public, please file reproducible bugs in **GitHub Issues** and put feature ideas in **GitHub Discussions / Ideas**, where others can vote with reactions. Include the build commit, OS/browser/GPU, tool settings, fresh or imported mesh, steps, screenshot and an optional `.obrush` project. The public repository's direct Issues/Discussions links must be inserted before the release announcement; none can be verified without a configured remote. Feature requests are votes and evidence, not a promise of parity or a delivery date.
