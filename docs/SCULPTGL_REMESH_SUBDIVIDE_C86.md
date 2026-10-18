# Sculptable Surface Nets quad levels (C86)

The retained-quad Surface Nets result from C85 can now be followed by the
existing **SculptGL static smooth** subdivision route, an ordinary Draw
stroke, level navigation and project history without losing the quad faces.
The shared-core test uses an independently authored closed cube; the browser
workflow uses the independently authored sphere and the visible remesh,
subdivision and brush controls. This is a checked end-to-end workflow, not a
claim that adaptive topology refinement works on retained quads.

The pinned hosted SculptGL bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
At voxel resolution 16, both browsers started from the same independent
266-vertex sphere geometry. The visible Smooth Surface Nets result was then
subdivided once:

| Measurement | SculptGL | OBrush |
| --- | ---: | ---: |
| Fine vertices | 4,690 | 4,690 |
| Fine oriented quads | 4,688 | 4,688 |
| Fine render triangles | 9,376 | 9,376 |
| Unmatched oriented quads after nearest-vertex mapping | 0 | 0 |
| Maximum / RMS world-position difference | — | `2.413e-7` / `7.933e-8` |

The comparison used a `2e-6` world-unit tolerance chosen before the paired
run. All 4,690 nearest vertex IDs were unique, and no oriented quad was
unmatched after cyclic face rotation. This is a complete-face comparison for
this fixture, not a general subdivision result. The OBrush browser then made
a default Draw stroke that changed 72 fine-level vertices. Its quad faces
stayed exact; coarse level, fine return, save and both subdivision/sculpt
Undo/Redo were exact in the captured project and visible GPU buffers.
The final source passed formatting, 2,374 workspace tests, strict native
and WASM Clippy, and WASM compilation. The owned browser loaded the same
`e64dadb0f902f0bf37f1f7251e753810e6efd74e1c546f6e0844660cd3e072ed`
WASM bytes as the C85 signed macOS/browser build checkpoint.

The same workflow passed locally on a resolution-32 OBrush sphere (18,978
fine vertices and 18,976 quads, with 298 vertices changed by Draw). Two
reference-browser attempts at that resolution did **not** yield a subdivision
snapshot: the first exceeded the harness's 30-second CDP observation timeout,
and a bounded retry exceeded 120 seconds after the Subdivide action. Those
failures are retained, and no reference speed or output claim is made for
resolution 32. The smaller paired run isolates a working reference case;
it does not explain the larger reference stall.

[C86 evidence](evidence/sculptgl-remesh-subdivide-c86-0.1.20.json) binds the
source, reference bundle, snapshots, numerical comparison, failed runs and
validation. The remaining remesh family includes arbitrary/open sources,
selected multi-mesh fusion, UV/material transfer, Marching Cubes, adaptive
detail on retained quads, native GUI/physical input and sustained browser
responsiveness. SculptGL and ZBrush parity remain unverified.
