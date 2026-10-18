# C152: selected-part SGL export

This follow-up implements the selected-mesh branch of pinned SculptGL's SGL export workflow in the shared Rust core and normal browser studio. It closes one specific missing **capability** under inventory ID `SGL-IO-EXPORT-SCOPE`; the full row and SculptGL parity remain open.

## Reference and fixture

- Pinned SculptGL source: `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, locally audited `src/gui/GuiFiles.js` SHA-256 `d7e4055e0cb20ff47fd29da9bd8380bf8d17020197c0b19f4cc2a9364d34263a` and `src/Scene.js` SHA-256 `092d17a08278700d6d85d3f8b9880e9152ce8c71be607fbe1584d0c5b5602f52`. `_getExportMeshes()` returns all scene meshes when `_exportAll` is set, otherwise the selected array, or no export for an empty selection. Selection array order is significant.
- Independent browser fixture: `.artifacts/scene-multiselect-c130/independent-three-closed-boxes.sgl`, SHA-256 `939110d73dec934f0af70fb5c364eeb18f3ce18608449fe76804896c50c9c0ae`. It has three 8-point, 12-triangle closed boxes and opens with part 3 selected.

## Implementation and checks

`SglExportScope::SelectedParts` filters and orders exported object views according to the selected ID list. It fails before mutation for an empty/invalid selection. The browser File panel offers a selected-parts checkbox, runs the existing loss review, then downloads only those parts. Review is tied to document revision, settings, selection IDs and active part so a stale selection cannot be silently exported.

The shared-core test creates three parts, hides one, selects `[last, first]`, and checks the two exported objects in that order, including the hidden selected part and its transformed source position. It also checks source document byte equality, empty-selection rejection and strict loss-policy rejection.

The ordinary browser run imported the three-box fixture, Ctrl-selected part 1 to form `[3,1]`, enabled selected-only SGL, accepted the conversion review and downloaded a version-3 SGL of 1,052 bytes with two objects (16 points, 24 triangles), SHA-256 `0812906465cec9de998f0545ce4d939112ad55ac7e4b0b801ac9d888e08cbf86`. The source still had three objects and selected IDs `[3,1]`; browser exceptions, console errors and blocked external requests were zero. Full trace, screenshots and downloaded bytes for the final source are in `.artifacts/sgl-selected-c152/browser-final/`.

The final browser build used `web/dist/pkg/obrush_app_bg.wasm` SHA-256 `e90e387ea20279bf8d203c3decf79fb79cfb30ad682d03033bb6e8d081136e48` and its separate experimental WebGPU package SHA-256 `b844314d03b48a4591a7539bf497b7e96c8cb9679198497ec8d61cd81bb1bf59`. This export path uses the normal WASM studio and does not exercise the WebGPU package.

The first browser replay, preserved at `.artifacts/sgl-selected-c152/browser-r1/result.json`, missed the checkbox just after scrolling and stopped before export. A second isolated replay passed. This shows a harness timing sensitivity; no data was exported in the failed run.

`cargo fmt --all -- --check`, workspace tests (2,478 passed, zero failed, one ignored across 100 suites), strict native workspace Clippy, strict `wasm32-unknown-unknown` Clippy for core/app/GPU, `scripts/build-web.sh` and `scripts/package-macos.sh` pass on the final source. The packaged macOS executable SHA-256 is `b43d274f07fd4f415aaeb0bcfbf084b882f394b3922fd8e1f15d13dbf9c58958`. The first strict Clippy attempt failed on an over-complex pending-review tuple; this was replaced with a named record before the passing rerun. The logs remain in `.artifacts/sgl-selected-c152/`.

## Limits

This is source-code-aligned scope behavior plus an OBrush browser check, not a paired live SculptGL export comparison. The browser test exercised two visible boxes; the hidden selected object and selection ordering were checked in shared-core tests. The same selection scope has not been established for OBJ, PLY or STL export, or for native GUI download. Material appearance, whole-scene conversions, all inventory acceptance gates and sustained performance remain separate work.
