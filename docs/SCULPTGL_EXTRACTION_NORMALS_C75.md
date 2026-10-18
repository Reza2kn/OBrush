# SculptGL extraction normals and smoothing diagnostic (C75)

The opt-in **SculptGL surface offset** now computes the extraction direction
from the arithmetic mean of raw incident source-face normals. It rounds each
face normal and the mean to f32 before normalization, including a single
combined normal for a quad. The ordinary OBrush sculpting and rendering normal
policy is unchanged. Whole-incident-face selection remains required.

The C74 paired fixtures were rerun through the C75 Rust core, retaining the
same six pinned SculptGL outputs. On the independently authored open sheet and
closed cube, the unordered output vertex positions at thickness zero now match
the reference exactly (maximum and RMS nearest-point distance both zero).
The prior cube maximum was 0.003382. This is a fixture-level coordinate match,
not extraction parity: corresponding face order, curved/quad surfaces and
nonzero thickness are not covered by that result.

At thickness ±0.125, the open-sheet reference still has eight triangles, six
boundary edges and four collapsed triangles; OBrush has a validated closed
12-triangle shell with no collapsed faces. The closed-cube reference and
OBRush both have 14 vertices and 24 triangles, but the largest reference-to-
OBRush nearest-point distances remain 1.289791363 inward and 1.399791382
outward. Border smoothing is the remaining dominant observed difference.

A separate reference-only diagnostic wrapped the pinned Masking tool's
`smoothBorder` entry point in an isolated CLI-owned Chrome profile. It captured
the mesh immediately before and after the original call, plus its selected
vertices, neighbor rings and edge flags. The wrapper delegated to the original
method; because it instrumented the method, this is diagnostic evidence rather
than an unmodified normal-product run. Four cases covered sheet and cube at
both signed thicknesses. An independent three-pass synchronous Laplacian
replay, with the reference's boundary-neighbor rule and f32 writes, reproduced
**every captured output position word exactly** in all four cases. For the
closed cube, smoothing selected 13 of 14 vertices, leaving the source-side
center fixed. This identifies the algorithm and affected ring without
accepting the collapsed open-sheet result as product behavior. Integrating a
safe border-smoothing variant, validating its topology and checking larger
surfaces remain open.

The final source passed `cargo fmt --all -- --check`, the full workspace
all-target test suite (2,356 passed, one ignored), strict native workspace
Clippy, strict WASM core Clippy, `scripts/build-web.sh` and
`scripts/package-macos.sh` with a valid ad-hoc signature. The isolated browser
mask/extraction suite passed all 12 cases on the rebuilt WASM and checked
source preservation, history and actual saved-to-GPU buffers. The browser
suite's sheet fixture does not establish the cube result in the UI; the exact
cube comparison is a shared-core fixture. No new long-session performance
claim follows. The [evidence index](evidence/sculptgl-extract-normals-c75-0.1.20.json)
binds the reference, comparisons, diagnostic, built artifact and browser run.
