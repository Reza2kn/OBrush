# C318 Twist visible-strength factor isolation

Frozen before any C318 browser run or Twist product edit, on integrated source
commit `4bd3843105f676466d1ba3c3d08930c34d7b411a`. This is a controlled
follow-up to [the failed C317 Twist baseline](SCULPTGL_SOURCE_TWIST_C317_RESULT.md),
which remains failed: 37 equal changed RGB IDs, but After paired XYZ maximum
`0.021797308698296547` and active display-normal maximum
`0.00011306075612083077` against `1e-6`. C317's preregistered
half-mask-versus-adjacent condition failed on both apps; keep that failure
visible here rather than retroactively changing its scorer.

Use the same independent C306 SGL fixture SHA-256
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`,
same pinned SculptGL bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
verified Source Twist selector index **2**, 75-pixel radius, symmetry/culling/
continuous/lock off, Alpha None, 1024×808 logical viewport, settled Reset/fit
camera and trusted integer arc `(-48,-12) → (-12,-12) → (-15,-42) →
(-42,-44)`. Source settings and input are unchanged. The **only intentional
normal-UI setting difference from C317** is clicking the visible OBrush
`settings.strength` slider at its right endpoint, requiring a published
`1.0` instead of C317's published `0.35`. OBrush product code is unchanged.
Record and verify the served WASM/JS and harness hashes at capture time.

The C318 scorer retains all C317 paired Before, three held, After, Undo and
Redo geometry/display-normal limits (`<=1e-6`), exact changed-ID membership,
own public one-step history, unchanged topology/attributes, protected ID543,
active GPU Save positions, quiet-release and the previously failed
half-mask-adjacent condition. It also binds to the exact C317 failed audit
SHA-256 `ef609ecd5b418c6701fe91045073d1fb83f04faefe011185fd18a3aac6d0e984`
and raw result SHA-256 values
`63f64bfb382ef7be89c0e17b4cd9c70a45e85c63e3555e5c99f2941338efafdb`
(Source) and
`b82401e10b242255597adc6908af68c9bb8b0ff0fb657b841661706dc0a102bc`
(OBRush). Its factor-isolation gate requires identical C317/C318 Source
After geometry and held GPU words, identical OBrush Before mesh, and a
reduction of at least 50% in paired After maximum XYZ error from C317. It
records held/normal error changes without treating that factor gate as
complete Twist parity. Preserve failures and do not tune the fixture, path,
thresholds or product code in response to C318 observations.

This test can attribute a portion of the scoped discrepancy to the visible
strength setting if its controls pass. It cannot isolate the remaining
falloff, current-versus-original surface, safety guard, pressure, alpha,
symmetry or sustained-performance differences; those and full SculptGL/
ZBrush parity remain open.
