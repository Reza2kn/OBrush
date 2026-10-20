# C323 Twist bounded candidate plan: match safe first move, retain fold guard

This static follow-up to the [frozen C323 preregistration](SCULPTGL_SOURCE_TWIST_C323_PREREG.md)
does not change product code or rescore C318's failed parity result. C318's
independent fixture SHA-256 is
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`,
Source bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and OBrush WASM SHA-256 is
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.
The exact C318 result hashes and 37-ID residual table are bound in that
preregistration. Source and OBrush fitted total angles at the first effective
move differed by only `1.393796297e-7` rad, while their radial-kernel models
produced clearly separated per-vertex displacement profiles.

The read-only `scripts/analyze-sculptgl-source-twist-c323-guard.mjs` replay
(SHA-256 `49c2b05eedc181b1f4ddb88f1f25ed655576237ffc1004ec7afe7c000521caae`)
applied the current shared-core `DeformationGuard` face projection, minimum
quality and fold-cosine inequalities to C318's public Before/After Saves and
held active GPU positions. Its ignored JSON receipt is
`.artifacts/sculptgl-source-twist-c323-guard-analysis.json`, SHA-256
`85d352206b526d12085afdeb0d6dc50efa3ec66303a88ed92197c4863498de46`.
It is an independently implemented static replay, not a call into Rust
product code; its pair enumeration and floating-point details need a core
test before being treated as an exact guard oracle.

| C318 state | Affected faces / fold pairs | Guard-violating faces | Guard-violating folds | Smallest face projection ratio | Smallest face quality | Smallest fold cosine |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Source first effective move | 101 / 165 | 0 | 0 | 0.3928852885 | 0.1717012501 | 0.9968152976 |
| OBrush first effective move | 101 / 165 | 0 | 0 | 0.4678397906 | 0.2004182093 | 0.9982520022 |
| Source second move / After | 101 / 165 | 10 | 6 | −0.1880436382 | 0.0093178871 | −0.9525625888 |
| OBrush second move / After | 101 / 165 | 0 | 0 | 0.2636428913 | 0.1000000497 | 0.9910171460 |

Source's final face 282, RGB IDs `[474,508,507]`, has orientation projection
ratio `0.0340518401` against the guard's `0.05` floor and quality
`0.0093178871` against `0.1`. Adjacent Source faces 317/318 across edge
`[509,542]` reach cosine `−0.9525625888` from nearly coplanar originals.
The current OBrush second move sits at the face-quality floor with no such
violations. This supports guard clipping as the cause of the separate
`0.2610873592`-rad accepted-angle gap. Reproducing the Source second-held
surface on this fixture would recreate folded or inverted triangles that
conflict with the project's existing anti-tear behavior.

The smallest viable product candidate is a **Twist-only radial kernel** in
`crates/obrush-core/src/brush.rs` at the `prepare_local_transform_stroke`
weight assignment (currently line 855). For Twist with default hardness,
compute `d = sqrt(distance_squared / radius_squared)` and
`1 - 4*d³ + 3*d⁴`, preserving the existing mask, alpha, culling, symmetry,
fixed captured center, axis, absolute parameter and guard flow. Keep Local
Scale's existing `(1-d²)²` profile and keep the shared
`DeformationGuard::safe`, `safe_target` and accepted-parameter binary search
unchanged. If Twist hardness is nonzero, apply the existing hardness policy
to the new Twist base kernel; this setting is outside the C318 reference
claim. The current kernel selection is in `radial_influence` near line 2276;
new code should avoid changing Move/Drag or Local Scale. The host's pointer
angle accumulation in `crates/obrush-app/src/app.rs` near lines 10833–10839
already agrees with Source at the first effective move and needs no
speculative edit.

Before accepting a candidate, update the independent core analytic tests in
`crates/obrush-core/tests/sculptgl_brushes.rs`: the shared `weight` helper near
line 112 currently encodes `(1-d²)²`, so Twist assertions near lines 185 and
402 need a Twist-specific source-kernel oracle while Local Scale retains its
existing oracle. Add one focused safety test on the authored C306 geometry or
an equivalent independently authored curved grid that verifies the first
requested rotation is accepted and the high second request is clipped without
face inversion, collapsed area or prohibited folds. Preserve existing
extreme-target, event-subdivision, mask, Undo/Redo and Local Scale tests.

Then run `cargo fmt`, workspace tests, strict Clippy, WASM target check,
`scripts/build-web.sh`, and the frozen C323 normal-UI path pair after the
exclusive runtime lane is released. A candidate only earns the scoped
first-held result if the pinned Source/OBRush first-held 37-ID positions and
active GPU display normals match to `<=1e-6`, while public Save, Undo/Redo,
GPU identity and quiet-release gates pass. Measure the second-held accepted
angle and guard result, retain its expected paired failure and the C318
receipts. C323's direct-vs-multistep classification must be recorded before
claiming that evolving state is absent. Full Twist or SculptGL parity remains
open; an exact folded Source outcome is not a valid anti-tear regression.
