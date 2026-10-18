# Paired SculptGL mask-extraction fixtures (C74)

The pinned SculptGL bundle (`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`)
ran in an owned isolated Chrome session through its normal OBJ file input,
Masking tool selector, Thickness field and **Extract !** button. No tool or
mesh setter was called by the harness. The mirror wrapper was hash-checked
before launch; its relation to the separately hosted deployment remains the
earlier stated limitation. Two independently authored OBJs were used: a
four-vertex, two-triangle open sheet with OBrush protection weights
`[0,1,1,0]` (SculptGL unmasked weights `[1,0,0,1]`), and a
closed eight-vertex, twelve-triangle cube with one masked corner. Both carry
nonuniform source RGB and nondefault roughness/metalness. The runtime snapshots, input
events, actual source/output arrays and the independent geometry audit are
retained under
`.artifacts/sculptgl-reference-runtime/extract-c74-paired-52a94733-2e98-4317-b919-47c1da3075f3/`.

| Fixture / thickness | SculptGL points / triangles | Boundary edges / collapsed triangles | OBrush points / triangles | Boundary edges / collapsed triangles | Largest reference-to-OBRush nearest-point distance |
| --- | ---: | ---: | ---: | ---: | ---: |
| Open sheet / 0 | 4 / 2 | 4 / 0 | 4 / 2 | 4 / 0 | 0 |
| Open sheet / −0.125 | 8 / 8 | 6 / 4 | 8 / 12 | 0 / 0 | 0.422110 |
| Open sheet / +0.125 | 8 / 8 | 6 / 4 | 8 / 12 | 0 / 0 | 0.422110 |
| Closed cube / 0 | 7 / 6 | 6 / 0 | 7 / 6 | 6 / 0 | 0.003382 |
| Closed cube / −0.125 | 14 / 24 | 0 / 0 | 14 / 24 | 0 / 0 | 1.289791 |
| Closed cube / +0.125 | 14 / 24 | 0 / 0 | 14 / 24 | 0 / 0 | 1.399791 |

For all six reference cases the source mesh was byte-exact in the captured
position/normal/color/material/face/UV/matrix arrays after extraction, remained
active, and the new part started with white RGB and f32 defaults 0.18 roughness,
0.08 metalness and unmasked weight 1. The zero-thickness sheet is offset +0.01
along its normals, exactly matched by OBrush's new opt-in whole-face offset
mode on unordered output points. For negative thickness, SculptGL offsets its
first layer −0.01 and its second a further −0.125. OBrush's opt-in mode uses
the same signed distances but retains its checked manifold bridge construction.

The open-sheet reference's nonzero output has four collapsed triangles and six
open boundary edges after its three border-smoothing passes. OBrush keeps a
closed, noncollapsed 12-triangle shell for this case; reproducing the damaged
reference geometry would regress the user-reported tearing behavior. The cube
has a clean closed reference output, yet its nonzero surface moves substantially
under border smoothing. OBrush currently uses angle-weighted source normals;
SculptGL averages raw incident face normals. That policy difference is a
plausible source of the measured zero-thickness cube gap and needs direct
isolation. The nonzero nearest-point figures also reflect the unimplemented
smoothing/bridge policy. Nearest-point distance ignores face correspondence and
does not establish shape or workflow parity.

The first reference run incorrectly compared f32 stored default material
values to unrounded JavaScript literals. Its three `default_materials` failures
are retained. The check was corrected to compare the exact f32 defaults; the
second six-case run passed without changing the reference bundle. This was a
harness failure, not a product-material difference.

The final OBrush source passes formatting, 2,356 workspace tests (one ignored),
strict native workspace and WASM core Clippy, the official browser build and
ad-hoc-signed macOS packaging. The ordinary browser suite passes all 12
mask-filter/extraction cases on the built and served WASM
`5a3127c85f9912a76e13b647786eda2f7db81ba35371eda1d1ff81345b9175ad`.
It saved 123 project snapshots, checked 9,640 visible-control bounds without a
failure, reported actual 4× MSAA, and had no GL or JavaScript errors. Its new
case exercised the visible option, version 6 saved material channels, exact
source preservation and Undo/Redo, and saved-to-GPU correspondence. The signed
native executable hash is
`cd7c6be50faf4b20241a104fa0c9e1d710f62564bbbe745e2221797a33054fe4`.
The first browser replay failed at the harness's version 5-only scene check
after the correctly saved version 6 material project; its result is retained.
The check now accepts the documented scene versions 5–7 while preserving all
identity, payload and GPU assertions.

The OBrush option is separate from the previous source-aligned extraction mode.
It runs in the shared Rust core and remains subject to finite-coordinate,
face-orientation, allocation and whole-scene validation before the document is
changed. The comparison is restricted to the named analytic inputs and exact
settings. Transformed parts, retained subdivision, arbitrary quad topology,
reference visual appearance, native GUI interaction, long-session performance
and ZBrush 2026.2.1 extraction remain unverified. The [evidence index](evidence/sculptgl-extract-reference-c74-0.1.20.json)
binds the source, runtime results and final build checks.
