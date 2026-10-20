# Q01 repeated Move pull quality diagnostic

**Separate from the frozen M02 input gate.** This diagnostic tests geometry
under the reported Move settings, without claiming a native rendered visual
match. It uses a fresh default `Mesh::sphere(6)`, radius `0.037`, strength
`0.79`, X symmetry and backface culling on. For twelve completed strokes,
it selects the current vertex nearest the stated positive-X/front direction
`(0.25, 0, 0.9682458)`, prepares a fresh shared-core Move stroke, and applies
eight cumulative camera-plane steps of `(5,-2)` logical pixels at `0.0004`
model units/pixel. Each completed pull is one history step. No random input
or smoothing is allowed.

Record changed-vertex counts and the minimum triangle quality
`2√3 × |(b−a)×(c−a)| / (|b−a|²+|c−b|²+|a−c|²)` after each pull, plus the
minimum radial face cosine `unit(cross)·unit(centroid)`. Gate requires valid
mesh after every step; all faces must have cross length greater than `1e-10`
and positive radial cosine, and at least one pull must change geometry. Undo
all twelve pulls must restore exact public project JSON. Low but positive
triangle quality is diagnostic evidence, not a pass for visual smoothness.

Preserve any failure and the measured series. No browser/native screenshot is
part of Q01, and it cannot close the user's triangular tearing report.
