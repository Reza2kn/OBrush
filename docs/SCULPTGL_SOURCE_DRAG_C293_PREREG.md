# C293 prospective settled-camera Source Drag pair

**Static preregistration only. No C293 browser result or product candidate
exists.** C286 setup, C288 geometry/Redo normals, C289 diagnostic no-go and
C290 unstable OBrush paired-pick failures remain preserved. C293 is a
different normal-UI workflow that explicitly settles each host's camera
between Reset and Center/fit actions; it does not reclassify any earlier
result. The pinned SculptGL commit is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independently authored curved-grid fixture SHA-256 is
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.
Record exact OBrush HEAD, WASM/JS hashes and harness/audit SHA-256s before
any run. C291/C292 hold the Cargo/browser lane until their explicit release;
all C293 work stays static until then.

The frozen read-only preflight harness is
`scripts/check-sculptgl-source-drag-c293-settled.mjs` (SHA-256
`e23767b67d8f8b73ae7559ada388d44267ec668658a7ced81a02302fca17e5c3`)
and its independent paired scorer is
`scripts/audit-sculptgl-source-drag-c293-settled.mjs` (SHA-256
`5feb9b56b1f8c5b04af03ade73aa3749bb0852db9bce9746e5670b5f3a60b51c`).
The harness uses owned ports, preserves per-side raw results and captures
public Before/after-hover Save, history and active GPU position/normal words.
The scorer maps every RGB ID, checks full attributes and triangle topology,
compares each active GPU position buffer with its own public Save, and uses
an independent front-orthographic ray/triangle query for the two controlled
hover positions. Do not treat the preflight as a Drag stroke result.

## Frozen UI sequence and camera gate

Open fresh documents through each normal browser UI at 1024×808 logical
pixels, DPR 1, with Scale and center off. Select front orthographic, Drag,
radius 96 px, mouse, pressure factors zero, Alpha None, symmetry/culling/
Continuous/Lock off. On Source, click visible `Reset (bar)` and wait until
every actual Camera timer entry is zero (or the reference's terminal value);
then observe identical center, translation, projection, near and viewport
at two checkpoints spanning at least 250 ms and 12 animation frames. Only
then deliver the same off-mesh double-tap used by C288; wait for all timers
to settle and repeat the stability observation. On OBrush, use visible
`Reset view`, wait for identical published target/position64/projection/
source-frame near and zoom across the same time/frame bound, then visible
`Center` and another complete stability observation. Do not hover, pick,
Save or begin a brush gesture while either camera moves. Capture actual
Source reset-vs-picked branch and pre-fit near/zoom as in C290; record the
corresponding Root frame at each milestone. Every wait has a bounded 5 s
timeout and preserves its failed receipt.

Before any geometry comparison or product edit, the settled Source/OBRush
X/Y projection ratios must each differ by <=`1e-8`; requested world rays at
offsets `(-64,-20)`, `(-28,-20)`, `(8,-20)`, `(44,-20)` and the 96 px world
radius must differ by <=`1e-8`; two independent front-orthographic ray/
triangle pick points must agree within `1e-7` per component. Publish both
public Before Saves, active-object identity, and matched 1,089 RGB-ID
positions/attributes/topology. The preflight is read-only: public Save and
history depth must remain exactly unchanged after controlled hover/pick.
If any camera, ray, pick or ownership gate fails, stop and keep the raw
failure. Do not rescale the root brush or copy Source camera values through
a hidden setter to force admission.

## Prospective shared-core Drag candidate gate

Only after the camera preflight passes may a separate, reviewable Source
Drag candidate be built. Keep the historical Legacy Drag route as the
default for old projects. The candidate must share native/browser geometry
in `obrush-core`, enforce budgets and rejection rollback, and use Source's
selected-union normal refresh on history rather than snapshot Redo normals.
Its prospective build requires `cargo fmt`, full workspace tests, strict
Clippy, WASM target/build and a new owned browser pair. No candidate code or
test-only Source setting is part of this preregistration.

Use the **unchanged C288 pointer trace**: press viewport-center offset
`(-64,-20)`, held moves `(-28,-20)`, `(8,-20)`, `(44,-20)`, release at
`(44,-20)`, with two 14.4 px Source substeps per segment. No Save/UI click
during hold. Capture public Before/After/Undo/Redo Save and active GPU
position/display-normal words at Before, each held move, After, quiet After,
Undo and Redo. Preserve C288's exact maximum XYZ and active display-normal
error limits of **`1e-6`** at every paired stage. Changed RGB-ID membership
must match exactly; protected ID544 stays bitwise fixed, IDs541/548 both
move, and held captures progress. Own GPU position words must equal each
own public Save exactly at public stages; active VBO/object identity and
quiet-after equality must pass. Each side's Undo restores exact own Before
bytes, Redo exact own After bytes, and the released gesture adds exactly one
undo step. Source/OBRush must match the RGB-ID set of active normal rows
whose After-versus-Redo words change, with paired Redo normals <=`1e-6`.
Attributes and topology stay exact. Zero-travel produces no geometry or
history change; invalid input after an admitted preview restores exact
Before and remains noncommittable on release.

Retain every failed preflight, build, setup and numerical receipt. A narrow
pass would cover this settled-camera fixture only; dynamic topology, alpha,
symmetry, alternate cameras, other Drag settings and long-session response
remain open. It cannot close full `SGL-BRUSH-DRAG` or ZBrush parity.
