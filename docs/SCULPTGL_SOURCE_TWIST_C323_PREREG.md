# C323 Twist falloff and state-path differential: frozen before runtime

This protocol is frozen after [C318's failed paired parity gate](SCULPTGL_SOURCE_TWIST_C318_RESULT.md)
and before any C323 browser run or Twist product edit. The C318 result hashes
are Source `e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b`,
OBRush `0cec63c177442128f8b1ac6caf3422ed3faaed0fc2cf19cd2c3c6647336809f4`,
and failed audit `91d134fb6adf8fbf44704271ce992cb991c9dbaf022dec10d681a6f4378d7865`.
The independent 1,089-vertex, 2,048-triangle fixture SHA-256 remains
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`;
the pinned SculptGL bundle SHA-256 remains
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and the no-edit OBrush WASM SHA-256 remains
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.

The read-only analysis `scripts/analyze-sculptgl-source-twist-c323.mjs`
(SHA-256 `7862ceaa3ff82dd7fe8386944d8c4542c70ee5e783ebfc138a59d3a9e8e03769`)
mapped both public Before exports and held GPU position words by independent
encoded RGB vertex ID. It used the exact pinned Source press center
`[-0.04278556723147631, 0.010696381330490112, 0.020572029735376418]`,
radius squared `0.004469251379393152`, +Z twist axis from eye `[0,0,-1]`,
and preserved each vertex's mask. For each of the 37 changed IDs it fitted a
single total rotation angle to either the Source radial kernel
`1 - 4d³ + 3d⁴` or the current OBrush kernel `(1 - d²)²`, with
`d = distance(Before vertex, captured center) / radius`. Exact per-vertex
actual/predicted angle and position residuals are in
[the C323 CSV](SCULPTGL_SOURCE_TWIST_C323_RESIDUALS.csv), SHA-256
`ac61cb9b44c2d1d9de899342fde9ac3ad6ba19eb91785ebc7f25f8d0aa828649`.
The full ignored analysis JSON SHA-256 is
`ebe9262b3409c5f8005d0f543b7faad63239dc0a5fa42f03fe2cb6940a961c09`.

| C318 held state | App | Fitted total angle (rad) | Own-kernel maximum position residual | Opposite-kernel maximum residual |
| --- | --- | ---: | ---: | ---: |
| First effective move | Source | 0.7378150538035294 | 3.26848989856687e-9 | 0.0017352267943309245 |
| First effective move | OBrush | 0.7378149144238997 | 9.05503831299254e-9 | 0.0014290753209543693 |
| Second effective move | Source | 1.3854483502670982 | 5.562232683480439e-9 | 0.0032578295141490283 |
| Second effective move | OBrush | 1.1243609910381753 | 1.1890741912336142e-8 | 0.002177661382371797 |

**Two-stage causal gate before a product candidate.** The first effective
move has equal total angle to within `2e-7` rad, yet different exact radial
profiles. A falloff-only candidate must first reproduce Source's 37 changed
IDs, first-held positions and active GPU display normals within `1e-6`, with
the same public Undo/Redo and GPU Save checks. The second move has a separate
`0.2610873592289229`-rad accepted-angle gap. A candidate must measure that
gap before and after the falloff change, identify whether it comes from the
deformation guard or evolving picked/mesh state, then meet the same second-held
`1e-6` position/normal gate **without** violating the project's deformation
integrity policy. If the pinned reference violates that policy, retain the
failed parity check and report the safety conflict; do not silently weaken the
guard. The analytical fits alone cannot prove the cause of the angle gap.

To distinguish pointer-path/state dependence, the separate C323 normal-UI
capture uses the same fixture, settled 1024×808 viewport, Source Twist
selector index 2, 75-pixel radius, OBrush published strength 1, mask,
alpha None, symmetry/culling/continuous/lock off, and trusted integer points.
It changes only the C318 path `(-48,-12) → (-12,-12) → (-15,-42) →
(-42,-44)` to a direct endpoint `(-48,-12) → (-12,-12) → (-42,-44)`.
The first held point initializes the angle anchor in both apps. The frozen
`scripts/check-sculptgl-source-twist-c323-ui.mjs` harness SHA-256 is
`6c11532166b0cdc4ab2bde489058d5512306c9183fbb7abf313fe269d11c0029`;
the paired `scripts/audit-sculptgl-source-twist-c323-ui.mjs` scorer SHA-256 is
`3ce1106e37a53712e357f359cb582e529e9b2da46647e264d34e7020293a58f9`.

The scorer requires identical pinned build/fixture, Before positions and
fixed-anchor GPU words, an identical Source picked center/axis/radius,
published controls, exact trusted path, one-step public Undo/Redo, active
GPU/Save correspondence, quiet release, unchanged topology/attributes and
fully protected ID543. It fits both direct-endpoint outputs to their own
fixed-center radial models at `<=1e-6`. For each app, it classifies the
direct-vs-C318 final maximum component difference as path-independent at
`<=1e-6`, measurably path-dependent at `>=1e-5`, or indeterminate between
those limits. This classification is an observation, not a presupposed pass;
either outcome is retained. The already failed C318 `1e-6` Source/OBRush
parity and adjacent ID577/578 mask comparison are preserved, not silently
rescored. Do not change the fixture, thresholds, product or harness after a
failed C323 run. Full Twist, SculptGL and ZBrush parity remain open.
