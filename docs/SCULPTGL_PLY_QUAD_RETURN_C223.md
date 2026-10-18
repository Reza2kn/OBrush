# C223: exact SculptGL binary PLY quad return

PLY import in shared Rust now retains an original quad face when its 0–2
diagonal produces two consistently oriented, nondegenerate triangles. The
same decision helper is used by OBJ and PLY. Triangles and polygons in a mixed
PLY still cover the canonical triangle array; an unsuitable quad diagonal is
triangulated and reported rather than stored as a misleading quad. Faces are
allocated within the existing two-million-triangle bound, and the document is
not replaced until the complete file validates.

The independently authored tests check a planar ASCII quad, a curved binary
SculptGL-style quad, a mixed quad/triangle export round trip, and a concave
quad whose 0–2 diagonal is unsuitable. The source-marked binary importer still
requires its exact zero-padding length; truncation and nonzero padding reject.

The actual pinned SculptGL browser file from C222 is 9,535,926 bytes,
SHA-256
`b0c09df364e296d3b9dc9998b7b8a608428edf70ca139696ec56713d707a1b69`.
It contains 98,312 vertices, 98,304 quads and two triangles. Every quad passes
the safe-diagonal check. A shared-core import followed by a fresh OBrush
SculptGL-profile PLY export reproduced **every byte** of that file.

The normal Chrome continuation uses the visible **Scale and center** option
off, the File Add chooser, part selection and selected-only binary PLY export.
The downloaded OBrush PLY has the **same full-file SHA-256** as SculptGL's
reference file. The imported two-part `.obrush` project has SHA-256
`390033b53b486b580f659595408927bfea3aec5b3094586deea0594e6c848cb7`;
Redo after Undo restores that exact hash. The browser reported zero runtime
errors. The passing [browser receipt](../.artifacts/sculptgl-obj-color-browser/80342685-6ab2-4b35-ae05-5dc6294cb68d/result.json)
has SHA-256 `bc511b8ff87ab19f98c2a4de86f3b65d576703eaebe0c4baff02c3afe5cb777e`.
The runner SHA-256 is
`5af6b17cbaf34e25acddecc218ba3ace3ab3d5ea742ff85e4d50cfaea2c8cef9`;
the served portable WASM SHA-256 is
`7220c22ba05fa92ca84a17f4c5a0062d7a941206b08b457bfe3e1cebe5c5ba52`.
The final source passes `cargo fmt --all -- --check`, 2,529 workspace tests
with zero failures and one intentional ignore, strict native/WASM Clippy,
`scripts/build-web.sh`, `scripts/package-macos.sh`, and macOS ad-hoc signature
verification. The packaged executable SHA-256 is
`6b5cc3fbec176699e1b0d5dc896c810083b3e50d3fea7a3bf320582f0c869b63`.

Two earlier browser attempts are retained as failed *harness navigation*
checks: the driver could not scroll to the off-screen Scale and center control.
Dragging the visible inspector scrollbar resolved that without changing the
product. This exact return establishes one pinned PLY file/workflow, including
its large quad sphere; it does not prove broad PLY import/export parity,
native physical GUI behavior, sustained performance or SculptGL parity.
