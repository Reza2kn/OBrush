# C277 prospective normal-UI Brush Clay pair

This is a frozen prospective test for one adjacent active SculptGL inventory row,
`SGL-BRUSH-BRUSH-CLAY`. It does not revise the C274 fractional-coordinate failure,
the C276 integer-coordinate Lock witness, or the full-row status. The pinned
reference is SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
hosted `sculptgl.js` SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independent RGB-ID 10,114-vertex / 20,224-triangle SGL sphere is SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
No copied reference geometry or commercial asset is used.

Both applications will import the same sphere through their visible file UI,
use Brush/Draw in Source mode, orthographic front fit, a 1024×808 logical sculpt
viewport at DPR 1 and UI zoom 1, radius 64 screen pixels, intensity 25%,
Accumulate **on**, Negative/Culling/X symmetry/Continuous/Prevent folds/Lock
**off**, Alpha **None**, pressure factors zero, no mask, static topology and
one visible object. Two fresh independent document runs per application will
use the visible Clay checkbox **on** and **off**, respectively. Published
checkbox/tool states, measured projection and world radius must agree across
each pair to `1e-6`; the browser must deliver trusted integer coordinates.
The on state is the pinned default Brush plane form; the off state is ordinary
Brush displacement.

For each run, press at logical viewport center plus `(80,0)` screen pixels,
then move with the button held to offsets `(92,0)`, `(80,0)`, `(92,0)`,
`(80,0)`, `(92,0)`, `(80,0)` in that order, with an acknowledged animation
frame at each step, then release at `(80,0)`. The exact browser pixel points
are derived from the measured viewport center and must all be integers; no
post-hoc trace rescaling is allowed. Capture public Save before, after the
first held move, after the sixth held move/release, Undo and Redo. Read active
GPU position and display-normal words at the corresponding states, including
held frames, with unique visible-object identity and zero GL error. Save
normals are never substituted for display-normal evidence.

Acceptance is fixed before running: both apps must change at least one and the
same RGB vertex IDs in each Clay state at the first and final held stages;
the maximum paired XYZ component difference and active GPU display-normal
component difference may not exceed `1e-6` at any matched stage. Public
mapped attributes, ordered faces and untouched vertices must remain exact;
each active GPU position stream must match its same-stage public Save exactly
as Float32 words. Each app must recover its exact Before with Undo and exact
After with Redo, with one sculpt-history step and no late post-release change.
The first and sixth held states must differ within each application, proving
that the repeated trace reached the plane/buildup path. Clay on and off final
states must differ within each application, and their effect must involve the
same set of RGB IDs across applications with maximum paired effect-component
difference `<=1e-6`. Source's `_clay`/`_accumulate` and OBrush's visible
per-tool state must report the requested values throughout.

Preserve every failed setup, browser and numerical receipt. First audit the
unchanged current build; only a concrete paired failure may justify a scoped
shared-core/host change. After an edit, run formatting, workspace tests, strict
native and WASM Clippy, fresh web build and the same paired browser gate.
One static sphere, one camera, one pressure state and one trace cannot close
the full Clay row, other brush options or SculptGL/ZBrush parity.
