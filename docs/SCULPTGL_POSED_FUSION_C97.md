# C97: selected affine/source-origin remesh

OBRush now accepts selected remesh of source-origin SubTools with affine poses after explicit consent to bake their world transforms and discard their source provenance. The shared Rust core validates each source before making bounded temporary world-space copies. It keeps the originals for undo, clears pose/provenance on the fused result, and reports which source IDs were baked and discarded. The browser exposes the consent next to selected remesh. The default remains rejection.

The independently authored `independent-posed-fusion.sgl` fixture contains two closed eight-vertex colored boxes: a red left box with an X shear of `0.15` and translation `-0.3`, and a blue right box with translation `+0.3`. Both use roughness `0.4`, metalness `0.6`, and unmasked weight `1`. The selected order is right then left. Pinned hosted SculptGL (upstream revision `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`) and OBrush each run Marching Cubes at resolution `16`, with Relax off and on. The local reference mirror preserves the hosted bundle; the browser checks use an owned isolated Chrome process. The evidence record binds all captured files and hashes.

| Comparison | Relax off | Relax on |
| --- | ---: | ---: |
| Oriented triangles matching after position mapping | 2,064 / 2,064 | 2,064 / 2,064 |
| Maximum mapped world-position difference | 3.0245e-7 | 2.9372e-7 |
| Maximum unmasked-weight difference | 0 | 0 |
| Maximum roughness/metalness difference | 2.3842e-8 | 2.3842e-8 |
| Maximum RGB difference | **1.0** | **1.0** |
| RGB RMS difference | **0.3311** | **0.3311** |

The full RGB failure is material: overlapping red and blue surfaces do not choose the same voxel color ownership as SculptGL in all locations. A diagnostic reversing OBrush's source order reduced RGB RMS to `0.05912` but still left maximum difference `0.49694`; changing selected order is therefore not a verified fix. The production browser retains the requested right-then-left selection. The reference's selected order and imported face/vertex order were inspected, and the discrepancy remains open. Geometry agreement does not establish attribute parity, arbitrary affine support, or sustained performance. Exact fixture, source, runtime, comparison and failed-check hashes are indexed in [the C97 evidence](evidence/sculptgl-posed-fusion-c97-0.1.20.json).

Core tests cover default rejection, explicit consent, three remesh modes, world-space extent, provenance removal and exact undo/redo. Both visible browser Marching Cubes cases pass selection, rejection/consent, closed-solid, GPU/save correspondence and exact history checks without JavaScript or GL errors. An initial browser comparison failed because the diagnostic harness captured GPU words only for small buffers; the complete-buffer hash check fixed that harness error. That failed run remains in local artifacts. No earlier sculpted geometry or external ZBrush file is needed for this fixture.

On the final source, `cargo fmt`, 2,391 workspace tests (one ignored fixture generator), strict native and WASM Clippy, the WASM target check, and both official package scripts passed. The macOS app's ad-hoc signature verified. The isolated final browser run loaded the served WASM SHA-256 `d80420d4be32cbab5d298fae179f3b443e73b8aac4c0939491d2b219349ceb90`; its two visible workflows passed. An earlier strict native Clippy run found a large topology-worker enum variant, which was boxed before the passing rerun. Both logs are preserved in the evidence.

The normal browser continues to use Rust/WASM CPU geometry and WebGL2 rendering. WebGPU compute/draw work is a separate prototype; this check makes no responsiveness claim. Selected pose handling is a narrower SculptGL workflow gain, not SculptGL or ZBrush feature parity.
