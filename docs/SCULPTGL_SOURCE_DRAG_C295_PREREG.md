# C295 Source Drag candidate setup-only correction

**Frozen before any C295 browser run.** C293's candidate Source capture passed,
but its OBrush capture stopped before the stroke because its harness retained
`else if (!matchSourcePath)` after removal of that obsolete C288 option.
Preserve both C293 raw receipts and the setup failure in
`SCULPTGL_SOURCE_DRAG_C293_CANDIDATE_SETUP_FAILURE.md`; C295 is a distinct
normal-UI pair, not a retroactive C293 pass.

C295 copies the C293 candidate harness with exactly one behavioral change:
remove the obsolete OBrush-only conditional and use the already-present
unconditional OBrush radius setter later in the same setup block. The separate
protocol ID, output directory, and descriptive comments identify C295.
The paired scorer changes only that protocol ID and result label. Its numeric
gates, RGB-ID mapping, active GPU/display-normal oracle, public Save/Undo/Redo,
history, and protected-vertex checks remain unchanged. The corrected harness
SHA-256 is
`74eb82886a1d607aee8db935f8b0aace4b690563761b62af257005ed0e7e8ea9`;
its scorer SHA-256 is
`0f785a5bf693222b15e5f9372c18ff7009ca6b5e37dc7b8bc935c21642f70b75`.
`node --check` passed for each before capture. Candidate product code remains
at isolated commit `86bf24790f220806a36516b42a30728143cdbeb5` plus this
evidence-only descendant; no source change occurs between C293 and C295.

Use the same pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and independently authored 1,089-ID fixture SHA-256
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.
The read-only C293 settled-camera preflight audit SHA-256 was
`da28020451312241c1c4859e0050312f16b3683ff94c9a0b79eaf74157d6a57b`.
Repeat the same 1024×808 normal UI Reset-wait-fit-wait sequence, Drag 96 px,
pressure factors zero, Alpha None, no symmetry/culling/Continuous/Lock,
and pointer path `(-64,-20) -> (-28,-20) -> (8,-20) -> (44,-20)`.
Both camera states must be terminal and stable for 12 frames and 250 ms;
X/Y projection ratio, all four requested rays and world radius must agree
within `1e-8`. Preserve the exact `1e-6` maximum XYZ and active GPU
display-normal gates at Before, every held move, After, Undo, and Redo,
plus exact changed-ID membership, protected ID544, one-step history,
public Save/GPU identity and After-to-Redo normal changed-ID set. No UI Save
or click occurs during hold. Do not alter product, fixture, input, or scorer
after seeing the result. If C295 fails a substantive geometry/GPU/history
gate, retain the failure and revert the product trial rather than tuning this
pair. A narrow pass cannot close the full SculptGL Drag row or ZBrush parity.
