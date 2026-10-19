# C250 active Source mirror guide (implementation evidence)

The Source viewport's default symmetry guide now uses the **active object's**
normalized `symmetry_offset`. It asks the shared Rust `SourceMirrorPlane` for
the same local plane used by Source geometry, then applies the rendered object's
model-view and inverse-transpose normal matrices. The existing global guide
checkbox and brush symmetry toggle must both be on; only the active object's
surface receives the guide. The older world-unit brush plane and zero-offset
center preview remain explicit choices.

An independent local tetrahedral OBJ fixture uses vertices `(0,0,0)`,
`(2,0,0)`, `(0,4,0)`, `(0,0,6)`; active-object center `(1,2,3)`; offsets `0`
and `0.5`; and a nonuniform/sheared model-view with columns `(2,0,0,0)`,
`(1,3,0,0)`, `(0,0,-4,0)`, `(10,20,30,1)`. The core's loose-root diagonal is
`sqrt(56)`, so the half-radius is `sqrt(56)/2`; the two local X planes are
`1` and `1 + sqrt(56)/4` (the core returns f32 origins). The test compares
each resulting eye-space uniform bit-for-bit with the shared core's mirror
plane transformed by that matrix. The independent normal-matrix fixture also
checks normalized eye-space X; guide calculation leaves document JSON and
history unchanged. A separate check rejects a missing Source root rather than
drawing a plausible but wrong line. The renderer's fallback cache is keyed by
both immutable mesh and retained normal-state identity, with an edit and a
normal-state change tested separately.

The exact 48-byte OBJ text is in the unit test and has SHA-256
`6fa7564b941ad2444fcdc7ba741955d26a4170c30e9fb90c383f7886b648d94a`.
The first targeted test run failed 2/3 tests with `MaskConvention` because the
OBJ test mesh kept the legacy protected-mask convention. The independent Source
fixture now explicitly selects `UnprotectedF32V1`; the corrected run passes
3/3. This fixture correction did not change product behavior.

The isolated browser build was served on `http://127.0.0.1:8811/` and loaded
two independently generated versioned `.obrush` fixtures of the same 642-vertex,
1,280-face level-3 sphere, with identity Source affine pose, interaction center
`(0,0,0)`, and X symmetry enabled. Only the active object's `symmetry_offset`
differs: `0` or `+0.5`. Their SHA-256 values are
`53232e7ea2bfaf9a80934a7dfa29cebf1e8d8536852d9602dec08e3c21d8d70d`
and `547e0452534a34afa47f5bc626dba544d9efb00fb7b9f0c25b6727361e811596`,
respectively. With **Symmetry guide** enabled and **Active object plane**
selected, the guide band moved visibly right on loading the `+0.5` fixture in
the same camera view. The captured 1280×720 browser PNGs have SHA-256
`4ef58a882fc5b44fea04f9155ac4d6f1cca2a0d0db018d7a9a00586e1f9e72ea`
for zero and `505f91dcbb5ad6197fe441ff8c4ac107c88efefd94f3828cb8e69ae1c088bcc7`
for half offset. The global guide checkbox also hid and restored the band on
the fresh sphere. The browser loaded the SIMD package whose WASM SHA-256 is
`f098c744c2e92257058185d683cbb7a028115a0518475ca05eeb70b4f9ec9cc7`.

Pinned SculptGL source revision
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` computes the per-object symmetry origin from
`getSymmetryOrigin()` using the interaction center plus local normal times
`symOffset * computeLocalRadius()`. Its radius uses half the diagonal of the
octree's loose root. OBrush uses its bounded shared-core Source octree state
when available; an imported immutable mesh without retained state is prepared
once per snapshot while this guide is active. This is an implementation and
internal numerical/OBRush browser check, **not a paired SculptGL browser comparison**. The
camera producer, guide pixel appearance, native window, non-X symmetry axes,
and full rendering acceptance gate remain open.

Seeding an imported mesh's Source octree happens synchronously on the first
guide frame. Its 100k/1M-vertex latency has not been measured here and may
cause a visible stall; the cache only avoids repeating it for the same mesh
snapshot. This change does not establish real-time or million-vertex
responsiveness.
