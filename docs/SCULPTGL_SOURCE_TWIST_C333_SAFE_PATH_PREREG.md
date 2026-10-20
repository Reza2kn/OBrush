# C333 Twist safe second-held path and mask-isolation gate

This is a static protocol, frozen before any C333 browser/Cargo run or product
edit. C331 belongs to Pinch. Keep the prior C318/C323/C330 receipts and
failures unchanged. The pinned Source commit is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` (Twist bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`),
and the independent C306 fixture SHA-256 is
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`.
The C330 pinned Source normal-UI result SHA-256 is
`e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b`;
the accepted owned-server OBrush result SHA-256 is
`8c85deda6fab55d48b3fe5320059259e876d6d86c783fe7a3078a90d858d6060`.
The still-failed frozen C318 paired audit SHA-256 is
`55b12db71e8f0adc75a48b0a5eb11879a3fa94cb35bdf68e677573a7ca445217`.

## What the pinned path actually shows

The Source Twist implementation calls `pickVerticesInSphere` at every held
move, but `SculptBase.preUpdate` returns before repicking the surface while a
non-continuous Twist is active; `SculptManager.canBeContinuous` explicitly
excludes Twist. In the pinned C318 capture, the picked intersection point is
`[-0.04278556723147631, 0.010696381330490112, 0.020572029735376418]`
and radius squared is `0.004469251379393152` at press and every held point.
Offline strict-sphere selection over the observed active GPU positions returns
the *same 38 RGB IDs* Before, held0, held1 and held2:

`[474,475,476,477,478,506,507,508,509,510,511,539,540,541,542,543,544,545,572,573,574,575,576,577,578,605,606,607,608,609,610,639,640,641,642,643,673,674]`.

The Source GPU normals change on exactly those 38 IDs at each effective held
move and on no outside ID. This path does not exhibit the feared later-path
normal-membership repick. The anchored selection remains a risk for another
fixture, symmetry, changing hit center/radius, dynamic topology, or a
near-boundary f32 case; do not change product selection solely from C318.

The C318 scorer's `displacement(577) < displacement(578)` check is invalid
for this fixture. ID577 is half unprotected at normalized brush distance
`0.6520446946`, Source radial falloff `0.4333896567`, giving effective
mask-times-falloff `0.2166948284`. Fully unprotected ID578 is near the brush
edge at normalized distance `0.9295340502`; its falloff and effective weight
are only `0.0270675063`. Source's first effective held displacement is
`0.0069533061` at ID577 versus `0.0012393753` at ID578; its After values
are `0.0130216089` versus `0.0023271653`. Both apps rightly fail the old
ordering. The more controlled first-held observable is the signed rotation
about the pinned center divided by the *individual* mask and radial falloff:

| App | ID577 inferred base angle | ID578 inferred base angle |
| --- | ---: | ---: |
| Source | `0.7378150477382629` | `0.7378152163116813` |
| OBrush C330 | `0.7378148034223942` | `0.737813023575975` |

Those values agree to within `2.2e-6` radians across apps. The protected
ID543 has zero displacement. Do not edit the frozen C318 scorer or erase its
recorded failure; use the corrected angular check only in C333.

## Separate normal-UI gate

Keep C306 fixture, symmetry off, lock position off, Source Twist tool index 2,
75-pixel brush radius, culling off, no alpha/continuous mode, OBrush visible
strength 1, settled 1024×808 logical viewport, and the same trusted integer
press/held delivery, GPU readback, public Save/Undo/Redo and owned-browser
closure checks as C318. Change only the last relative pointer offset:

`(-48,-12) → (-12,-12) → (-15,-42) → (-24,-43)`.

The first effective angle remains `atan2(30,33) = 0.7378150601204649` rad;
the new second effective absolute angle is `atan2(31,24) =
0.9119902906774204` rad. A read-only analytic rotation of the independent
initial fixture at this angle, with the pinned Source radial kernel and masks,
has 38 picked IDs, 101 affected faces, minimum face projection `0.2885399479`,
minimum face quality `0.1189614231`, minimum adjacent-fold cosine
`0.9924099326`, **zero face and fold failures** under the existing guard
conditions. This is a prediction, not a browser result; the future Source
capture must be checked independently before a product claim.

The future C333 scorer must require equal pinned fixture/controls/viewport
and input path, exact selected-ID membership from each held GPU position plus
the Source captured pick center/radius, and normal-changed IDs confined to
that selected set. First-held and second-held paired active GPU XYZ and raw
normal maxima must each be `<=1e-6`, with the same public Save, GPU identity,
quiet release and exact own Undo/Redo checks. For the first effective held
point, compute signed XY rotation about the captured Source center for IDs
577/578, divide by each ID's independent mask and `1−4d³+3d⁴`, and require
each app's two inferred base angles and paired corresponding angles to agree
within `5e-6` rad; protected ID543 must not move. Do **not** require the raw
displacement of ID577 to be less than ID578. Before accepting second-held
geometry, independently evaluate actual Source and OBrush surfaces against
the unchanged face-quality/projection/fold guard and require zero violations.
If either actual target violates the guard, preserve the receipt as a failed
safe-path protocol, without weakening the guard or treating it as parity.

This one safe path can probe later state/normal behavior without the known
C318 folded-target confound. It cannot close the Twist row or establish full
SculptGL parity. No C333 runtime or product edit has occurred.
