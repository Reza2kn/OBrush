# C282 prospective interior-face opposed-sheet Brush Culling pair

C279 failed its original projection setup gate; C280 failed a second-held
`1e-6` paired geometry gate on a ray lying on a triangle edge. C281's
read-only replay directly observed a lower-face pick and failed its frozen
upper-face diagnostic hypothesis. All three receipts remain unchanged.
C282 is a **distinct** unchanged-product normal-UI pair designed to test
Culling on/off away from the edge, not a revision of those results.

Use the same pinned Source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, Source JS SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent opposed-sheet fixture SHA-256
`80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`,
four fresh documents (Source/OBRush × Culling on/off), 1024×808 logical
viewport, front orthographic fit, Brush with Clay off, Accumulate on,
64px radius and 25% intensity, Alpha None, pressure zero, Negative off,
symmetry off, Continuous off, Lock off and no alternate topology. Keep
the same public Before/After/Undo/Redo, two held active GPU snapshots,
trusted normal DOM input, active object identity, post-release quiet frame,
attribute/topology/history and culling membership checks. Keep C280's
published X/Y projection-ratio bound `5e-6` plus actual world-ray and
brush-radius difference `<=1e-6`, and the **unchanged `1e-6` paired XYZ
and GPU position/normal component tolerance**. No product edit, pointer
rescaling, direct kernel call, reference patch or fixture change.

The only gesture change is the integer path from viewport center
`(+44,-8) → (+50,-8) → (+56,-8)` CSS pixels, mouse down, two held moves,
mouse up. With the measured C281 Source orthographic coefficients, the
corresponding local XY centers are approximately
`(0.03268384,0.00594252)`, `(0.03714073,0.00594252)` and
`(0.04159761,0.00594252)`. All are strictly inside the front-sheet
triangle with RGB IDs `[221,222,243]`. Relative to that triangle's XY
vertices `(0.025,0)`, `(0.05,0)`, `(0.05,0.025)`, the minimum barycentric
coordinate over the three rays is about `0.06965`, and the smallest
perpendicular XY distance to any triangle edge is about `0.0012313` world
units. The latter exceeds the observed C280 maximum cross-app ray/radius
difference `1.46565e-7` by more than 8000×. Before any geometry result
is accepted, compute actual Source/OBRush projection-derived rays for all
three points and require each to have minimum barycentric coordinate
`>=0.05` and minimum XY face-edge distance `>=0.001` to that same face.
Read-only Source and OBrush picked face/normal/contact observations at
press and both held stages must identify this same RGB face; retain a
failure if they do not. This input geometry gate is fixed prospectively.

Success is only a scoped 882-vertex interior-face witness. C279/C280/C281
failures and the full Brush Culling parity row remain open regardless.
Record exact harness/audit/served build hashes and any failed setup or
numerical checks. Do not run C282 until the exclusive browser lane is
released.

Frozen C282 harness SHA-256:
`38c9f3a391d45b31e27092c5469794a07e4c0bf18f87346295cd83f0e893fa71`.
Frozen audit SHA-256:
`d88dd0928a3a768f9e09f05ea6fc78daabce950ab783063fd23b3d4ee15ef8ab`.
