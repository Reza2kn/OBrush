# C323 Twist-only falloff candidate: first-held positions pass, normals fail

The Twist-only Source radial kernel candidate is commit
`3b7ea54595c71e326645493a634b873d4fb62b41` in the isolated Twist
worktree. It changes only `obrush-core`'s Twist falloff from `(1-d²)²` to
`1-4d³+3d⁴` and updates focused analytic/safety tests. The original
`DeformationGuard` face, quality and fold conditions remain unchanged. The
portable browser WASM SHA-256 is
`466be693b023b632f98d453aa0eb55fd7327c4daa11ff9836e3c8fb7d36b109e`;
the browser JS SHA-256 is
`90298706ca9d817629fd844b17ac9d5b00e3d8dd1d9ea9ed3cf5a807d2d9b33d`.

The unchanged pinned C318 Source normal-UI receipt was paired with a fresh
normal-UI OBrush capture using the same independent C306 curved-grid fixture,
75-pixel Twist radius, visible strength 1, trusted integer path and held
GPU observations. The frozen C318 scorer **failed** its complete paired gate.
At the first effective held move, all 37 changed RGB vertex IDs match and
maximum position-component difference is only
`1.4901161193847656e-8`, below the `1e-6` position limit. The active GPU
display-normal maximum remains `3.113818820565939e-5`, above its `1e-6`
limit. Held move 0 and Undo match exactly. Both apps passed their own public
one-step Undo/Redo, GPU Save-position and quiet-release checks. The old
ID577-versus-adjacent-ID578 condition still fails on both apps because it
mixes mask and spatial falloff; that failure is preserved.

At held move 2 and After, paired maximum XYZ difference is
`0.00922725722193718`, while maximum active display-normal difference is
`0.000051000737585127354`. This is the expected safety conflict documented
in [the C323 candidate plan](SCULPTGL_SOURCE_TWIST_C323_CANDIDATE_PLAN.md):
the reference final surface violates the retained face/fold guard. The
candidate narrows the first-held position gap but does not meet the frozen
first-held GPU-normal gate, and neither the Twist row nor full SculptGL
parity is closed.

The first-held normal residual is sharply localized by update policy. Among
the 37 moved IDs, Source and OBrush GPU normals differ by at most
`8.731149137020111e-11`. Source changes normals at those 37 IDs plus
stationary protected ID543; OBrush changes normals at those 37 plus 28 stationary
neighbor IDs. The largest mismatch, `3.113818820565939e-5`, is stationary
ID611: Source retained its Before normal while OBrush refreshed it. The
pinned Source Twist calls `mesh.updateGeometry(facesFromPickedVertices,
pickedVertices)`, and `updateVerticesNormal` writes only the supplied picked
vertices. OBrush's local-transform path calls
`recompute_normals_near_vertices_cached(dirty)`, which deliberately updates
all neighboring vertex normals. This is a plausible exact cause, supported
by the per-ID GPU words; a future scoped selected-normal owner must be
independently verified before changing display behavior. Do not rewrite
public saved normals or weaken the guard to hide this mismatch.

Raw ignored receipts:

- Unchanged pinned Source result: `.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/result.json`, SHA-256 `e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b`.
- Candidate OBrush result: `.artifacts/sculptgl-source-twist-c318-ui/off-root-404d1cb8-1e68-461d-a6e3-047354704873/result.json`, SHA-256 `0f4cd13920c9c1aaafb80e9ddc9c61625f1dd829c4514c41666610604442afa8`.
- Frozen failed paired audit: `.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/paired-audit-off-root-404d1cb8-1e68-461d-a6e3-047354704873.json`, SHA-256 `a56274e56b1c5f95d4580efd290a79a18a265c63c5b46b8feb8d3e87b56737c8`.

Validation on this candidate: `cargo fmt --all`, focused
`sculptgl_brushes` (32/32), full `cargo test --locked --workspace`, native
strict Clippy, `obrush-core` WASM target check, app WASM strict Clippy, and
`scripts/build-web.sh` passed. The first focused run failed its scalar oracle
because the new test helper had not yet assigned zero outside the radius;
that test-only helper was corrected before the successful rerun. The
failed run is retained in this report. The normal-UI browser result failed
as above, so no further browser package or candidate change was made.
