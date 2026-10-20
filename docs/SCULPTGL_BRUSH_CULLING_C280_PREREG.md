# C280 prospective opposed-sheet Brush Culling pair after C279 fit failure

C279 remains a failed setup check, with its raw Source-off and OBrush-off
captures preserved. Both normal UIs reached the same 1024×808 logical sculpt
viewport on the independent opposed-sheet fixture, but their published
orthographic X projection coefficients were `2.62935733795166` and
`2.6293652057647705`. Their ratio differs from 1 by about `2.99e-6`, above
C279's frozen `1e-6` projection-ratio gate. C279 stopped before OBrush
sculpting. This is not counted as a paired geometry failure or a pass.
The published Y ratio is `1.0000029334976093`; Source fit center Z is
`0.008044370313944307` while OBrush's is `0.009999999776482582`. The
unchanged 64px world brush radii differ by `1.4225368472758904e-7` units.
At the planned 40/52/64px horizontal offsets, the largest actual world-ray
X difference is `1.4225368472758904e-7`. The remaining fit discrepancy is
observable and bounded here; a C280 geometry pass cannot be attributed to a
large mismatched initial ray or brush radius under its `1e-6` world-space
gate. The distinct fit centers and any depth-sensitive pick behavior are
still limitations of this one paired gesture.

C280 separately preregisters the same pinned SculptGL commit/bundle, exact
fixture SHA-256 `80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`,
visible controls, four fresh documents, integer center+40→52→64 stroke,
active GPU/public Save/Undo/Redo observations and all C279 culling geometry,
history, membership, attribute, topology and `1e-6` paired-component limits.
The *only* setup-bound change is accepting a published X/Y projection ratio
within `5e-6` of 1 while requiring the actual initial/held world-ray X and
Y differences at every requested pointer point to be `<=1e-6` world units,
and the actual world brush radius difference `<=1e-6`. No pointer coordinate
rescaling or non-UI camera manipulation is allowed. Both actual projection
coefficients and the resulting ray/radius differences must be reported.

The C280 run must still prove that culling off moves front and back IDs, on
moves front IDs and leaves all back IDs bit-exact, both apps change the same
IDs per state, active GPU display normals and positions pair within `1e-6`,
public history is exact and one step, active GPU position words equal public
Save at Before/After/Undo/Redo, and the post-release frame is quiet. Preserve
any new setup or numerical failure. This one fixture remains a scoped witness;
the full culling row stays open.
