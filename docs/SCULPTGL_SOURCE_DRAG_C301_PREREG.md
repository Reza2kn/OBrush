# C301 Source Drag symmetry setup-only correction

Frozen before any C301 browser run. C300's unchanged-product symmetry-off
normal-UI pair passed its full scorer; Source symmetry-on completed. OBrush
symmetry-on failed **before the stroke** because the frozen C300 harness used
`settings.symmetry`, an offscreen duplicate while the Source geometry panel was
open. Preserve all four C300 raw captures including that failure and its
symmetry-off numerical audit. C301 is a separately numbered setup-only rerun,
not a retroactive C300 pass.

C301 copies C300's capture/scorer/matrix scripts with one behavioral change:
OBRush sets the same public `symmetry_x` value through the visible
`source_geometry.mirror` checkbox instead of the offscreen `settings.symmetry`
control. Protocol ID, result directory and messages change from C300 to C301.
A static normalized-diff check confirmed this is the only behavior change.
C301 retains the exact C300 pinned SculptGL commit/bundle SHA
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` /
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent fixture SHA
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`,
1024×808 settled camera, 96 px Drag, Alpha None, no Continuous/Lock,
pressure factors zero, trusted integer pointer offsets
`(-100,-20) → (-64,-20) → (-28,-20) → (8,-20)`, and symmetry off/on cases.
All projection (`1e-8`), paired XYZ/active GPU display-normal (`1e-6`),
RGB membership, protected/half-mask witnesses, right-side symmetry gain,
public Save/Undo/Redo, active GPU identity/words, quiet GPU and C295 regression
gates are unchanged from `SCULPTGL_SOURCE_DRAG_C300_PREREG.md`.

Frozen script SHA-256:

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-drag-c301-ui.mjs` | `2f36416f88ffc2eeff592fe73e6acf7a4ef02cb424d4663e74308257a3ff0f7a` |
| `scripts/audit-sculptgl-source-drag-c301-ui.mjs` | `45542ea7833655081d6fda0f54c0863782e6bfad49d26c32c56d86651a6d2dc1` |
| `scripts/audit-sculptgl-source-drag-c301-matrix.mjs` | `f4b5b9990c91c4f04bfcdc20a3940bea2c6a236ed3487fea7c2382e9d9be9599` |

`node --check` passed for all three. Run one unchanged-product quartet,
record actual root HEAD and served WASM/JS hashes, then score both cases and
the matrix. If setup is still blocked or a substantive geometry/history/GPU
gate fails, preserve it and stop before product promotion. Any candidate build
and rerun must use these same C301 scripts and gates; C295 remains an
independent regression receipt. Full Drag/SculptGL/ZBrush parity stays open.
