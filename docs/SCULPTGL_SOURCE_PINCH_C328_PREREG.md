# C328 Pinch effective-dab audit-only correction

Frozen before any C328 paired numerical audit. C327's four captures and
[failed auditor](SCULPTGL_SOURCE_PINCH_C327_RESULT.md) are immutable inputs.
No browser session, product edit, fixture edit, or new geometry sample is
authorized by this audit correction. The C308/C327 Pinch stroke requested
one press and one held move. The reference press picked zero vertices and
OBRush had no press geometry pass, so averaging press and move centers is an
invalid deformation oracle. Both sides produced a measured move pass.

Copy the C308 paired auditor to a C328 audit file. Before scoring, require
the reference `press_pick.picked_vertices===0`, OBrush `press_pick===null`,
and both move centers finite three-vectors. Compute center-directed signed
displacement against each side's **measured move center only**. Compute the
paired pick-center difference from those move centers only. Keep the original
independent RGB-ID mapping, complete public file/history/attribute/topology
checks, protected ID 1201 and half ID 1202 gates, same changed-ID set,
`1e-6` maximum paired XYZ and active GPU display-normal bounds, `1e-8`
camera projection/world-radius/ray/pick-center bounds, and four-run opposite
sign witness above `1e-7`. Preserve every failed metric and do not relax any
threshold. The C308 four-run matrix auditor may read the C328 per-polarity
JSON after this change; its own gate stays frozen.

This is an audit-only interpretation of the already captured effective dab.
If a gate fails, C328 remains a failed comparison. Further changes require
a separately frozen protocol and cannot rewrite C327 or C328 evidence.
