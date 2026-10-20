# C290 read-only Drag camera causality result

**Outcome: Source reset/fit cause established; frozen paired final-pick gate
failed. No product edit or Source Drag parity claim.** The preregistered
normal-browser harness `scripts/check-sculptgl-source-drag-c290-causality.mjs`
had SHA-256
`b6295e6c1c3942ae38789db957080d7016b3c22b54fd04096ba384c51cee40f9`
on both sides. Pinned SculptGL bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the independent fixture was
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.
The OBrush served browser WASM/JS SHA-256s were
`1380a9c6bbfaee43d637846803bdaddf941dba8f43893af747ccf3323778737d` /
`35d354ed50431b29537bd2591b737aeaccecbf3b92ef37265316c205017b710b`.
Both browsers used separately owned ports and exited; root port 8792 was
untouched. Public Before and after-hover Save hashes and undo/redo depth
matched within each side, so the diagnostic made no document edit.

Raw results remain at
`.artifacts/sculptgl-source-drag-c290-causality/source-a39ca1b8-350c-4ee4-a13b-2972e962988d/result.json`
(SHA-256 `93719a6a68ed53862481ff2077238213c27ad81531db3f764cf00966af117833`)
and
`.artifacts/sculptgl-source-drag-c290-causality/root-eebeafa4-8eed-4192-9307-80db4819b5c3/result.json`
(SHA-256 `ddca3074b12fdf96a4e09bc8cf94f9046ca4c9e9886445254d943b1d0f6956ae`).

The normal UI double-tap actually took the **off-mesh**
`resetCameraMeshes` branch. Its fit received near distance
`875.4443916752775` and the same visible-mesh box measured in C289.
`computeFrustumFit` returned `2.3865531085337444`, exactly the pinned
orthographic formula with `x = 404 / near` (absolute residual zero).
`setAndFocusOnPivot` received box-center Z `0.02551280055195093` and
derived zoom `0.810560431738396`. After this fit, nine observed callbacks
from the still-running 200 ms reset `centerDelay` summed to
`-0.005141115309712296`, reconstructing the final Source center Z
`0.020371685242238636` with zero numerical residual. This confirms the
specific overlapping-timer mechanism inferred from C289; differing source
mesh bounds were already excluded there.

The settled Source X projection was `2.1905438899993896`. Both read-only
hover intersections hit the mesh. Their published X/Y points agree with
independent front-orthographic ray coordinates at offsets `(-64,-20)` and
`(44,-20)` within `4.20e-9` per component. This validates the Source
camera-to-pick propagation on these two inputs. Source normal fields were
not recomputed by the read-only intersection query and are not used as
geometry evidence.

The OBrush normal UI `camera.center` was followed by the first hover **before
its camera animation settled**. Its X projection changed from
`0.033181313425302505` at the nominal final capture to
`0.1601649820804596` and `0.43961018323898315` at the two successive
hover samples; its precise target Z changed from `0.00853404655092788` to
`0.02220236027579416` and `0.02447269322349234`. Those are different
camera frames, so a controlled Source/OBRush final pick comparison is
inadmissible and the frozen C290 paired gate **fails**. The successful
Source-only causal subgate does not change C288's failed geometry or Redo
normal results. A settled paired diagnostic would require a distinct,
preregistered attempt with an explicit animation-stability witness; C290
must not be silently rerun or retoleranced.
