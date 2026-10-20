# C256: Source Draw radius-chain diagnostic

This is a preregistered **diagnostic**, not a symmetry, brush, or pixel-parity
claim. C255's final normal-browser held-stroke receipt changed 34 RGB pixels in
SculptGL and 2,340 in OBrush. The measured mirrored-picker radius was
`sqrt(0.12211026693735978) = 0.34944279494` local units in SculptGL and
`sqrt(1.5621323064788237) = 1.24985291394` in OBrush. Their respective
projected path scales were `178.856215705` and `200.023535840` CSS pixels per
local unit. Thus the **observed** physical radii were approximately `62.5000`
and `250.0000` CSS pixels, almost exactly fourfold. Different rendering and
the 11.83% camera-scale mismatch still confound the changed-pixel ratio.

## Frozen scene and camera

Use the pinned SculptGL source at `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
and the independently authored 540-byte
`independent-affine-two-mesh.sgl` fixture (SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`).
Retain the wrapper, bundle, WASM, script, fixture and result hashes. Use a fresh
owned Chrome profile, 1600×1000 CSS viewport and device scale 1, and the normal
import/UI paths in C255. Isolate the active triangle in each application; keep
local vertices `(1,-2,0), (4,-2,0), (2.5,2,0)`, translation Z `+0.5`, normalized
X SymOffset `+0.5`, ordinary Source/Draw, nominal radius `250`, intensity
`0.5`, alpha None, X symmetry on, continuous off and the three-point held path
`(3.70,-1.99,0) → (3.845,-1.99,0) → (3.99,-1.99,0)`. Follow OBrush's normal
mask conversion before Source mode as in C255. Keep both applications' normal
pressure handling; do not set hidden pressure or camera state.

The reference camera is its default imported scene camera. The target path
span is `51.86830255448524` CSS pixels (`178.85621570512154` CSS px/local).
Use OBrush's visible Solo and Frame actions, then ordinary mouse-wheel zoom
until its span exceeds 50 CSS pixels. Adjust the visible Source camera
`camera.projection_zoom` numeric field only until the path span differs from the
reference by at most 1%; record every wheel event, typed value and resulting
camera. C255's 15-wheel state was zoom `5.805141852541628`, span
`58.00682539371928`; linear scaling predicts zoom about
`6.4921702319528745`, but the **measured** span is the acceptance criterion.
Freeze both cameras before the brush event. Require each projected path point
inside the visible canvas, primary and mirrored face 0 contacts, and a path
span greater than `0.15 × 250 = 37.5` CSS pixels so SculptGL can emit a held
dab. An unmet setup check is a preserved failed receipt, not a candidate pass.

## Exact radius chain to record

Record a capture for **each pointerdown and held pointermove**, with an event
sequence/time key so values cannot be mixed across dabs:

| Stage | Pinned SculptGL source | OBrush Source code / capture |
| --- | --- | --- |
| Raw input | Trusted `PointerEvent.pressure`, `pointerType`, buttons, CSS X/Y; `SculptGL.js:49-53,79-81` assigns `Tablet.pressure = event.pressure` on down/move. | Browser raw sample `pressure64`, `pen`, `in_contact`, CSS X/Y in `source_geometry.last_completed.capture.input`; record event pressure too. |
| Nominal radius | Visible Draw input and `SculptBase._radius = 250`; `getScreenRadius() = _radius × Scene._pixelRatio` (`SculptBase.js:439-440`). Capture pixel ratio, canvas backing/client sizes and camera width/height. | Visible Source radius and per-tool `radius = 250`; capture `screen_radius` and `radius_factor` in completed Source input/options, viewport and DPR. |
| Pressure factor | `Tablet.getPressureRadius() = 1 + 0.75 × (Tablet.pressure × 2 − 1)` (`Tablet.js:1-12`); capture the **actual** factor at each picker call, not just raw pressure after release. | `offset = screen_radius × (1 + radius_factor × (pressure × 2 − 1))` (`source_geometry_controls.rs:325-340`); capture actual locked press pressure and radius factor. |
| Projection | `Picking.computeWorldRadius2()` world hit, `Camera.project(world)`, X-offset passed to `Camera.unproject(x+offset,y,z)`, unprojected edge, squared world distance, `mesh.getScale2()`, primary and mirrored local radius² (`Picking.js:378-392`; `Camera.js:399-415`). Wrap the existing picker methods for read-only tracing and delegate to their originals exactly once; do not invoke a second picking or stroke. | Capture Source view64/viewport64, world hit, projected hit, X-offset, unprojected edge and completed primary/mirror local radius². Recompute projection and inverse from the captured matrix independently, and compare the result with the completed pass. |
| Held outcome | Active three vertex coordinates and picked face/radius per dab before/after; undo count. | Same active three coordinates through ordinary save/export or GPU readback, completed dab observations and undo count. |

The reference mirror picker copies the primary radius at `SculptBase.js:160-175`,
so a final mirror radius alone cannot identify its pressure input. Instrument
`Picking.computeWorldRadius2` on the primary picker **during the original
call**, while recording both pickers' final radii. Also record a literal
`PointerEvent.pressure` in the trusted browser-event log; C255 logged
coordinates/buttons but omitted pressure. Wrapping is observational only and
must preserve the original return value and call count.

## Numerical hypotheses and interpretation

At matched projection, SculptGL's C255 radius would still be about
`0.34944279` local units (`62.5` CSS px). If OBrush keeps an effective 250 CSS
px, its local radius should become `250 / 178.85621570512154 =
1.3977708239794837`, and the local-radius ratio should be close to four.
These values are **predictions for the existing behavior**, not desired parity.
The matched-camera scale tolerance is 1%, and self-consistency between each
captured X-offset, projected/unprojected radius and picker/pass local radius
must be within 1% (or `1e-5` local units, whichever is larger). A 2% tolerance
around the fourfold physical-radius ratio is enough to classify the C255
anomaly. Preserve exact values and any failures rather than widening a gate.

The most direct falsifiable explanation is raw pointer pressure: at pressure
`0`, the pinned SculptGL curve multiplies `250` by `0.25`, yielding **exactly
62.5 CSS px**. At pressure `0.5`, the factor is `1`, yielding 250 CSS px. The
C255 OBrush captured Source input reported pressure `0.5`; its event log did
not record reference pressure. If C256 captures reference pressure `0` at the
primary picker and OBrush `0.5`, with equal screen-radius/DPR and a fourfold
X-offset, classify this as a fixture/host pressure-input mismatch. It does
**not** justify quartering OBrush's brush radius. If reference pressure is
`0.5`, find the earliest unequal scalar among screen radius, pressure factor,
X-offset, camera inverse and mesh scale; record it without guessing a fix.
If those values match yet geometry differs, this hypothesis is falsified and
the next gate must isolate projection or picker timing.

C256 closes only this radius-chain diagnosis when both sides' original-call
traces are complete and numerically self-consistent. It cannot close a full
SculptGL row or justify a visual/parity percentage. A later correction needs
a fresh independently frozen held-stroke pair, exact vertex deltas and fixed
cursor/guide-state screenshots; preserve the C255 68.82× mismatch as evidence.

Baseline receipt: `.artifacts/sculptgl-symmetry-held-c255/d1139ea4-b7a5-495b-9d43-366864370cdf/result.json`, SHA-256
`7f8708374eca1d1a2cf4822b937c51c903cf67068b738efcc0b4b86653ce9c0c`.

The runner is `scripts/check-sculptgl-symmetry-radius-c256.mjs`. Invoke it as
`node scripts/check-sculptgl-symmetry-radius-c256.mjs web/dist .tools/sculptgl-reference/host-mirror`
only while holding the shared browser lane. Before that, `node --check` and
`node scripts/check-sculptgl-symmetry-radius-c256.mjs --self-check` exercise
syntax and independent projection/inversion without Chrome. Setting
`C256_C255_RECEIPT` to the baseline receipt path additionally replays its
captured OBrush matrix and radius. Each browser attempt writes its full JSON,
ordinary pre/post `.obrush` saves, and screenshots to a unique ignored
`.artifacts/sculptgl-symmetry-radius-c256/` directory, including failures.
