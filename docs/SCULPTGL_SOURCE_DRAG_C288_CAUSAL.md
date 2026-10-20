# C288 read-only Drag residual attribution

This is a new analysis of the retained C288 raw Source/OBRush browser receipts,
not a C288 rerun or a revised acceptance gate. The independent replay script
`scripts/diagnose-sculptgl-source-drag-c288.mjs` has SHA-256
`a6220da7e65c8348a4c3cdcf841076360859317bf759d67a5e8dfd0226aa842d`.
Its raw derived output is preserved at
`.artifacts/sculptgl-source-drag-c288-ui/read-only-residual-diagnosis.json`
(SHA-256 `cbb17e99ffbbe9a26f82f399beab8d5472c6d9a526355640184c504dea6fe6e9`).
Inputs are the unchanged public Before/After SGL and `.obrush` Saves, each
side's captured held GPU/host observations, and the pinned six-sample Drag
kernel. No geometry is replaced or written by the script.

The recorded Source press and three held picked centers, plus the fixed
36-pixel/two-substep path, reconstruct the six Source centers. The three
OBRush held `last_pass` centers reconstruct its same six centers. The script
applies the pinned Drag quartic falloff to the public Before f32 positions,
raw unprotected material, and each side's recorded radius, rounding every
vertex assignment to f32 after each of five nonzero steps. It does not infer
unrecorded pressure or alpha behavior: this fixture used mouse, pressure
factors zero, and Alpha None.

The OBrush camera/center/radius replay equals its public After positions
**bitwise across all 1,089 RGB IDs**. The Source replay differs from its
public After by at most `7.450580597e-9` in 15 component words. The replayed
Source-versus-OBRush maximum is exactly the observed `2.060085535e-6` at
RGB ID606, and the full per-component difference-of-differences has maximum
`7.450580597e-9` (RMS `2.162400165e-10`). Applying OBrush's recorded centers
and radius to the same Source Before mesh yields the OBrush public After
positions bitwise. The observed geometry failure is therefore explained to
f32 rounding by the **published camera/pick/radius inputs**; this fixture
provides no measurable evidence of a different Drag displacement kernel or
sample count. C288's `1e-6` gate remains failed.

The setup publishes Source X/Y projections `2.1905438899993896` and
`2.776134729385376`, versus OBrush `2.1905605792999268` and
`2.776155948638916` (relative differences `7.618793e-6` and
`7.643452e-6`). Source/OBRush recorded local radii are
`0.08559520895911903`/`0.08559452852927832`. Their first press centers
differ by about `4.34e-7` in X and `1.40e-7` in Y. These small input
differences accumulate through five live, nonlinear Drag increments; a
single radius/ray difference alone is not a bound on final displacement.

The pinned Source `Drag.js` updates local normals after every substep. Its
`StateGeometry` history stores a union of touched vertices and refreshes
that union on Undo/Redo, explaining why Source After and Redo positions match
but 31 active display-normal rows differ (maximum `1.615153160e-4`). OBrush's
candidate kept exact snapshot normals on Redo. OBrush core already has a
bounded `SourceHistoryEvent::Geometry` selected-union transition and
`SourceGeometryNormalState::refreshed_for_history`; only the rejected Drag
route lacked admission to that path. This is a source-grounded design
boundary, not a completed or tested Drag history fix.
