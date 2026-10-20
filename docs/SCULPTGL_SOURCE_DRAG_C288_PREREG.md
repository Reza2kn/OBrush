# C288 Source Drag bounded-projection paired capture

C286 is a retained failed setup, not an accepted geometry result. Its Source
normal-UI capture was complete, but the OBrush run stopped before Before Save:
under the independently authored C286 33×33 curved grid, the published
1024×808 front-orthographic fit projection ratio was `1+7.618793e-6` in X
and `1+7.643452e-6` in Y, above C286's frozen `5e-6` gate. Raw result:
`.artifacts/sculptgl-source-drag-c286-ui/root-63e856f4-b4b5-4e81-a2a5-d66c63be3d36/result.json`.
The published world-radius difference was `6.521270e-7`, and the largest
requested integer-ray difference was `4.347513e-7`, each below the separately
frozen `1e-6` gate.

Before any C288 geometry comparison, one new bounded pair is frozen. Reuse
C286 fixture SHA-256
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`,
the exact four integer pointer points `(-64,-20)`, `(-28,-20)`, `(8,-20)`,
`(44,-20)` relative to viewport center, Source Drag radius 96 px, pressure
factors zero, Alpha None, symmetry/culling/Continuous/Lock off, normal UI,
public Before/After/Undo/Redo Save, three held active GPU captures and quiet
post-release capture. The sole setup-bound change from C286 is X/Y published
projection ratio <=`1e-5`, which encloses the observed `7.643452e-6` with
`2.356548e-6` margin. The world-radius and requested-ray bounds remain
`1e-6`; no pointer, radius, fixture or geometry tolerance changes.

All C286 frozen XYZ, active display-normal, RGB changed-ID membership,
protected ID544, distinct IDs541/548, active object/VBO, history,
attributes and topology gates remain unchanged. Pair XYZ and active GPU
normal tolerances are each `1e-6`; public GPU positions must equal own Save
words bitwise. A geometry/GPU/history failure ends the attempt and leaves
full `SGL-BRUSH-DRAG` open. A scoped pass still leaves dynamic topology,
alpha, symmetry, alternate camera and long-session response open. Candidate
Source Drag remains opt-in and older projects retain Legacy by default.

Frozen harness SHA-256:
`343518413fbc6eda75a25c6f32052a74a45bcb6cb4cec043310373f79fa16bea`.
Frozen numerical audit SHA-256:
`73bc20a0124e166a8a1a017c74be49bfe419bea85a773a18f0279d7ae4902880`.
Pinned reference bundle SHA-256:
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Candidate web WASM SHA-256:
`fef05973a75a7ad5667a7ba5f261368bea509fcff9e40c8adcec9691e52e70c4`.
Candidate web JS SHA-256:
`f5be35803fed50aa7f8a8e46153c4fe26b2cdb833d692db5f94770c112d889e5`.
