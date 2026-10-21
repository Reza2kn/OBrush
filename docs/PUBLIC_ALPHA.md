# OBrush public alpha

OBRush is an open-source sculpting studio written in Rust. The same sculpting core runs in a macOS app and in the browser through WebAssembly. **This is a public alpha:** anyone may try it, inspect the code and report what breaks. Draw, Crease, Move, masks, paint, subdivision, multiple parts, OBJ/PLY/STL interchange and versioned `.obrush` projects are available in varying degrees of maturity. [SculptGL and ZBrush compatibility is unfinished](FEATURE_PARITY.md), and no percentage-to-parity claim is made.

Try the [browser app on OBrush.App](https://obrush.app/), [v0.1.20-alpha.1 release downloads](https://github.com/Reza2kn/OBrush/releases/tag/v0.1.20-alpha.1), and [source repository](https://github.com/Reza2kn/OBrush). The [launch verification](PUBLIC_ALPHA_LAUNCH.md) records public download integrity and the hosted-app check. To build the source locally, run `cargo run --locked -p obrush-app --release` for desktop. On macOS, `scripts/package-macos.sh` creates `.artifacts/OBrush.app`. For a local browser build, follow the [WASM prerequisites and build commands](../README.md#browser), then serve `web/dist`. Use the published release's commit and artifact hashes when reporting a bug.

Save early and often. The browser warns before leaving edited work; this warning remains after requesting a download because the page cannot confirm that the file was saved. Check your Downloads before leaving. Browser reload/close discards the in-memory document, and OBrush does not yet have autosave or crash recovery. Keep `.obrush` project copies while experimenting. `.obrush` is OBrush's editable project format; OBJ/PLY/STL exports are interchange formats and may not retain all masks, layers, materials or scene state. Untrusted or malformed imports are meant to be rejected without replacing the current document, but please report any failure of that safeguard immediately.

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
| Browser | Static WASM/WebGL2 path exercised in owned Chrome 155 browser tests. Official bundles include `build-info.json` with the source commit and exact WASM hashes; use the published bundle's metadata when reporting bugs. Other browsers and devices need testing. |
| Windows and Linux | Source targets exist but current native GUI execution is unverified. |

Please use the public [bug-report form](https://github.com/Reza2kn/OBrush/issues/new?template=bug_report.yml) for reproducible problems and the [feature-request form](https://github.com/Reza2kn/OBrush/issues/new?template=feature_request.yml) for ideas. Search existing issues first and vote with a 👍 reaction instead of filing a duplicate. Include the build commit, OS/browser/GPU, tool settings, fresh or imported mesh, steps, screenshot and an optional `.obrush` project. Issue attachments are public; remove private information before uploading. Votes and reports guide the roadmap but are not a promise of parity or a delivery date.
