# C287 camera snap/focus/reset transition preregistration

Pinned source: SculptGL `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
`src/math3d/Camera.js` SHA-256
`da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe`.
Its snap and move-to timers run for 200 ms using `easeOutQuart(r) =
1 - (1 - min(1,r))^4`, ordinarily sampled by a 16.6 ms interval. Orbit/
translation inertia use duration -1 and are disabled. Quaternion deltas use
identity-to-target slerp increments; center/offset/translation use eased linear
deltas. This fixture freezes the mathematical envelope, not browser timer jitter.

Authored pose: target `(1.25,-0.5,2)`, distance `8`, camera orientation a
positive 60-degree Y rotation; destination target `(-0.75,1.5,0)`, distance
`4`, identity orientation. At 0/50/100/150/200 ms, frozen eased fractions are
`0`, `0.68359375`, `0.9375`, `0.99609375`, `1`. Target X values are
`1.25`, `-0.1171875`, `-0.625`, `-0.7421875`, `-0.75`; distances are
`8`, `5.265625`, `4.25`, `4.015625`, `4`; remaining Y-rotation angles are
`60`, `18.984375`, `3.75`, `0.234375`, `0` degrees. This fixture uses no model
mesh or document mutation.

Acceptance gates, frozen before running code:

1. A pure transition samples the values above within `1e-6` for scalar
   channels, `1e-5` degrees for orientation; sampled quaternions stay finite
   and unit length. Values before start clamp to start; at/after 200 ms equal
   the final pose exactly.
2. Starting a new snap at a 50 ms sampled pose begins at precisely that pose
   without a jump and reaches the second target 200 ms later. Direct orbit,
   pan, zoom and sculpt contact cancel any pending transition immediately.
3. Actual Source-profile Front/Left/Top/Snap/Reset/Fit actions animate through
   the normal native/browser UI, repaint only while active, and produce no
   geometry/history change. A picked pivot and current camera projection/zoom
   remain coherent during animation. UI/browser gates wait for the exclusive
   browser lane and must be reported independently of the pure fixture.
4. Run fmt, full workspace tests, strict native/WASM Clippy, Web build and an
   owned browser after host integration. Preserve failures and exact build
   hashes.

This is one camera-transition slice. Source camera matrix differences,
all navigation variants, timer jitter, gesture interruption semantics and
the full `SGL-CAMERA-FOCUS-ANIMATION`/SculptGL/ZBrush gates remain open until
their separate normal-UI fixtures pass.
