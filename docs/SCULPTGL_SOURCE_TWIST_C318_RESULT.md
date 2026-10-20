# C318 Twist visible-strength factor: causal improvement, parity still failed

The frozen C318 normal-UI factor control passed. It replayed C317's independent
1,089-vertex curved-grid fixture, verified Source Twist selector index 2 and
the same four-point trusted arc. Source After positions and all three held GPU
position words were identical to C317; OBrush's Before mesh was identical.
The only intended setting change was OBrush's published visible strength from
`0.3499999940395355` to `1`. The paired After maximum XYZ difference fell
from `0.021797308698296547` to `0.010854240506887436`, a
`0.5020375837620865` reduction, just over the preregistered 50% factor
threshold. This attributes a substantial portion of the scoped C317
discrepancy to the strength setting. It does **not** establish Twist parity.

The frozen `1e-6` paired geometry/display-normal gate still failed at After,
Redo and held moves 1 and 2. The active After display-normal maximum fell
from `0.00011306075612083077` to `0.000051311333663761616`. Held-move-1
maximum XYZ fell from `0.011974568711593747` to
`0.0032686684280633926`; held-move-2 fell from
`0.021797308698296547` to `0.010854240506887436`. Held move 0 and Undo
matched exactly. Both apps changed precisely the same 37 RGB vertex IDs,
preserved topology and attributes, and passed their own public one-step
Undo/Redo, active GPU Save-position and quiet-release checks. Fully protected
ID543 remained stationary in both. Source displaced unprotected ID541 by
`0.020939597747869732` and OBrush by `0.016191191416395256`; Source
displaced half-unprotected ID577 by `0.013021608945545986` and OBrush by
`0.008076061948106561`.

The preregistered ID577-versus-adjacent-ID578 displacement condition failed
again in **both** apps. Because those vertices occupy different locations in
the brush falloff, this is a confounded mask-strength check. Its failure is
retained; it is not evidence that either app ignores the mask. No product
code, fixture, threshold or input path was changed in response to C318.

The frozen C318 harness SHA-256 was
`5ea518ad24d4fe0f4906fd18f498dd032bc1bfa5ba76629d6deb26b`, scorer
SHA-256 `e2a991c0186ef1b8f4ad82e243af4dda03c64e6a383e0ce2cf2925798f861a4f`,
fixture SHA-256
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`,
pinned SculptGL bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and unchanged OBrush WASM SHA-256
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.
Projection X/Y ratios were 1, all four sampled rays and the 75-pixel world
radius matched exactly. The scorer bound to the exact failed C317 audit and
raw-result hashes documented in [the C318 preregistration](SCULPTGL_SOURCE_TWIST_C318_PREREG.md).

Raw ignored receipts:

- Source result: `.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/result.json`, SHA-256 `e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b`.
- OBrush result: `.artifacts/sculptgl-source-twist-c318-ui/off-root-cedcecb2-8563-4039-8b9c-55aadba8970b/result.json`, SHA-256 `0cec63c177442128f8b1ac6caf3422ed3faaed0fc2cf19cd2c3c6647336809f4`.
- Paired failed audit: `.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/paired-audit-off-root-cedcecb2-8563-4039-8b9c-55aadba8970b.json`, SHA-256 `91d134fb6adf8fbf44704271ce992cb991c9dbaf022dec10d681a6f4378d7865`.

This single settled-camera arc with symmetry off isolates the visible
strength factor only. The remaining error could involve the falloff,
current-versus-original surface, rotation kernel or other differences;
pressure, alpha, symmetry and sustained performance were not exercised.
The SculptGL Twist row remains open.
