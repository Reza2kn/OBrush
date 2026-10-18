# Closed-source extraction border smoothing (C76)

The opt-in **SculptGL surface offset** now applies three synchronous border
smoothing passes to nonzero shells extracted from closed triangular sources.
It uses the patch triangle edges and logical side-wall quad edges as neighbors;
the side-wall rendering diagonal is not a smoothing edge. The outer skin and
source-side patch boundary are smoothed, while the source-side interior stays
fixed. This is the selection and neighbor graph observed on the independent
closed-cube reference fixture. A candidate with collapsed or nonfinite faces
is discarded, retaining the original checked shell. Open source surfaces
retain that shell because the pinned SculptGL open-sheet extraction has four
collapsed triangles and six open boundary edges at nonzero thickness.

The C75 pinned SculptGL outputs were compared with the rebuilt C76 shared
Rust core on the same independently authored OBJ sheet and cube at threshold
0.5 and thickness 0, −0.125 and +0.125 world units:

| Fixture and thickness | Previous largest nearest-point gap | C76 gap | Output topology |
| --- | ---: | ---: | --- |
| Closed cube / 0 | 0 | 0 | 7 points, 6 triangles |
| Closed cube / −0.125 | 1.289791363 | 0.000000047707 | 14 points, 24 triangles, closed |
| Closed cube / +0.125 | 1.399791382 | 0.000000092159 | 14 points, 24 triangles, closed |
| Open sheet / ±0.125 | 0.422109516 | 0.422109516 | OBrush 12-triangle closed shell; reference 8 triangles, 4 collapsed |

For the closed cube, the signed volumes differ from the reference by less
than `7e-9` world units cubed. The signs differ by thickness because the
reference reverses its chosen winding for positive thickness; both output
meshes have consistently paired manifold edges. Nearest-point matching does
not establish face/UV correspondence, visual appearance, self-intersection
freedom or arbitrary-surface parity. The C75 instrumented diagnostic replay
identified the three-pass rule; C76 integrates that independently in the
shared core. Mixed quads, transformed objects, complex masks and larger
curved surfaces still need paired reference cases and guard evaluation.

The browser suite added two signed-cube runs through the visible extraction
controls. The final isolated Chrome run passed 13 cases, including exact
source preservation, undo/redo, save and actual GPU-buffer correspondence.
The first C76 browser run failed a newly added **harness** expectation that
both signed shells have positive volume. The pinned reference and OBrush both
have negative signed volume for the positive-thickness cube; the failure
receipt is retained. The corrected run checked nonzero volume with the
reference's thickness-dependent winding and passed without changing the
built WASM. Source formatting, 2,356 workspace tests (one ignored), strict
native/WASM Clippy, the official browser build and signed macOS package
passed. No million-vertex responsiveness or full SculptGL/ZBrush parity claim
follows. The [evidence index](evidence/sculptgl-extract-smoothing-c76-0.1.20.json)
binds the exact source and runtime artifacts.
