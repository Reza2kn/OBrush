# C311 default Smooth: scoped normal-UI pair passed

The [C311 preregistered normal-UI comparison](SCULPTGL_DEFAULT_SMOOTH_C311_PREREG.md)
passed every frozen gate on the independent footprint-corrected 625-vertex
fixture. This is a scoped result for one 50 px/75% Smooth gesture, with
Relax-only off and on, not a closure of `SGL-BRUSH-SMOOTH` or ZBrush parity.
The earlier [C309 failed footprint gate](SCULPTGL_DEFAULT_SMOOTH_C309_RESULT.md)
is retained unchanged. No product code was edited for C311.

The pinned SculptGL source commit was
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; reference bundle
SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The OBrush source commit was integrated root
`dab2bfe10c8e4125edafe53d3f107357c59feeb7`, including C310b's
Paint-only pen queue. The browser artifacts built with the lockfile-matched
wasm-bindgen CLI were: portable WASM SHA-256
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`,
JS SHA-256
`0c97e03a7ac8e183d852368fe62651eae0b550f6e65c823e1841bd6e21eedefb`,
SIMD WASM SHA-256
`bad8421313c20b74955ae7d785fcbce24666497a55051d60a4eb8c4844c49754`.
`bash scripts/build-web.sh` completed successfully. No Rust product change
was made in this worktree, so no additional workspace test, Clippy or
native package gate was triggered by C311.

Fixture SHA-256:
`46d9e6de3587335009127dc7c15f70dd0a82db7a2c26711dbbff588514a66303`.
Frozen harness SHA-256:
`3f484f3d9445c5a36a649d01ebcbcc488c56c8734c3392b5589dce36f217f4de`.
Frozen audit SHA-256:
`5ec81c7406f2ae7f7bc17bf5c54c3146d13b8947b6eb2a1f4623535932f9016a`.
The four retained raw records (paths relative to this worktree) are:

- Source Relax-only off: `.artifacts/sculptgl-default-smooth-c311-ui/source-off-9888498a-75f1-4813-a7f1-148df607cc27/result.json`
- OBrush Relax-only off: `.artifacts/sculptgl-default-smooth-c311-ui/root-off-49469ab1-f9f4-45c5-bdb1-fd6dd35c5dd8/result.json`
- Source Relax-only on: `.artifacts/sculptgl-default-smooth-c311-ui/source-on-1f763fad-4afc-4dda-a42c-573027273257/result.json`
- OBrush Relax-only on: `.artifacts/sculptgl-default-smooth-c311-ui/root-on-51e04d10-04a2-4c6c-810f-932ea585410b/result.json`

The frozen paired audit is
`.artifacts/sculptgl-default-smooth-c311-ui/source-on-1f763fad-4afc-4dda-a42c-573027273257/paired-audit-root-on-51e04d10-04a2-4c6c-810f-932ea585410b-with-off.json`.
Its `failures` array is empty. Both programs changed exactly the same ten
RGB IDs, `288, 311–314, 337–339, 362–363`, in each mode. Raised ID 337
and half-unprotected ID 338 changed; protected ID 336 stayed fixed. The
maximum Source/OBRush mapped XYZ difference was **zero** at Before, held,
After, Undo and Redo in both modes. Paired active GPU display-normal and
held GPU-position differences were likewise **zero**. Each app's active
GPU positions exactly matched its own public Save, and all GPU identities,
quiet-after-release, topology, attributes and one-step public Undo/Redo
gates passed.

The actual world-radius difference was `9.72e-8` or less and the largest
requested world-ray difference was `3.87e-8`, below the frozen `1e-6` limits.
The X/Y projection ratios were between `0.999997664` and `0.999997836`,
within the frozen `5e-6` setup bound. Both on/off runs delivered the same
trusted integer pointer gesture through the normal UI. The raised ID 337's
on/off displacement differed by `0.0084662642` in **each** app, above the
frozen `1e-4` discrimination gate. Its Smooth-off displacement was
`[0, 0, -0.008671307936310768]`, and Relax-only on was
`[0.00021783770353067666, 0.0012994222342967987, -0.0002050437033176422]`.

This result covers one curved imported mesh, a single mouse gesture,
ordinary default radius/intensity, and a Relax-only on control. Other
radii, strengths, cameras, pressure, Culling, Lock, alpha, symmetry,
topologies and sustained performance remain open. Source WebGL1 upload
shadow and OBrush WebGL2 readback are distinct display observations; the
comparison used active GPU words, not saved mesh normals, for display.
