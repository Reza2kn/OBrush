# C107: three-level source-static detail check

This checkpoint extends the C106 controlled coarse-edit comparison from two smooth levels to three. The pinned SculptGL browser imported the same independently authored, all-seamed tetra, quad cube and mixed pyramid OBJ fixtures, used the real Subdivide and resolution controls, then received one direct coarse-vertex delta `(0.125, -0.09375, 0.0625)` before raising through all three levels. The edit is diagnostic and does not claim brush or history parity. OBrush ran the corresponding `Document` sequence through the shared Rust core.

The first comparison incorrectly edited OBrush pyramid vertex 0, while SculptGL vertex 0 maps to OBrush physical vertex 3 on that OBJ. It reported a false maximum first-level difference of `0.0424584537`. The corrected probe edits vertex 3; the comparator now verifies the physical coarse edit mapping and exact delta before comparing outputs. A deliberately wrong-vertex replay is retained and rejected with `physical coarse edit differs`.

| Fixture | Third-level vertices | Maximum third-level position component difference | Second/third first-neighbor mismatches |
| --- | ---: | ---: | ---: |
| Tetra | 130 | `2.9802322e-8` | 0 / 0 |
| Quad cube | 386 | `1.7881393e-7` | 0 / 0 |
| Mixed pyramid | 194 | `5.9604645e-8` | 0 / 0 |

The comparison pairs physical vertices by unchanged pre-edit positions and checks every final position. The source first-neighbor IDs agree after the same mapping at both measured higher levels. The largest third-level error is within three `f32` steps at unit scale; no general tolerance for other meshes or brush operations is inferred.

The reference run used Chrome 155.0.8059.80 with no exceptions or console errors, and its owned browser and server closed normally. A separate normal OBrush browser run on the official rebuilt WASM added three static levels to an independently authored 1,536-quad UV grid, lowered to coarse, made one Draw stroke, and raised through all three levels. It moved 18 of 1,538 coarse vertices and propagated visible changes to 4,138 of 98,306 third-level vertices. All ten completed project downloads had matching saved-project/GPU checks; brush undo/redo, quiet release, four-level retention and project reload gates passed. This is a delivery check, not a paired reference brush-response comparison or a frame-time qualification.

The exact source and result hashes, preliminary false mismatch and corrected full comparisons are in [the C107 evidence](evidence/sculptgl-three-level-frame-c107-0.1.20.json). The two-level C106 comparator replay is retained as a regression check. Workspace tests, strict native/WASM Clippy, the WASM target check and both official build scripts pass.

The paired geometry evidence covers only three small fixtures and one direct coarse edit; the browser delivery evidence covers one Draw stroke on a larger UV grid. It does not establish pinned-reference brush-response parity, edited fine-level behavior, linear subdivision, arbitrary topology, broad file/history behavior or sustained performance. The browser still uses WASM CPU geometry with WebGL2 drawing; the separate WebGPU prototype is not integrated into the studio.
