# C334 picked camera pivot — frozen candidate and browser gate

This is a bounded candidate for `SGL-CAMERA-PICK-PIVOT`, not a camera-family
parity claim. It is frozen before running Cargo or either browser. The product
patch lives on `codex/camera-picked-pivot` and is **unverified** until the
exclusive validation lane runs.

## Sources and exact setup

- Pinned SculptGL source commit:
  `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.
- `src/math3d/Camera.js` SHA-256:
  `da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe`.
- `src/gui/GuiCamera.js` SHA-256:
  `c23c8008273ee949e0eba3e91cc690a9eba878956d03edc243d12ad45f9a0b97`.
- `src/Sculptgl.js` SHA-256:
  `0941577e0b8b0c5cc93e6ca63fa69ea630a730661e183e38e4ba964b1bff1462`.
- OBrush base commit: `1ecc42def96bd8dbe90b340d4aa4b8233ac134a1`.
- Authored mesh: [`fixtures/camera-picked-pivot-c334.obj`](fixtures/camera-picked-pivot-c334.obj).
  SHA-256 `a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b`.
  The file has four positions
  and two triangles, one raised corner, no external assets or source geometry.
- Ordinary browser sculpt viewport: **measured** 1024×768 CSS pixels at DPR 1;
  size each browser's outer window separately to reach that scene rectangle,
  and fail setup if either measured rectangle differs. Repeat the OBrush
  no-motion and one-motion checks at DPR 2. Perspective, 45° FOV, Orbit mode,
  speed 100%, source/static framing in OBrush. Set Pivot **on** through each
  visible camera panel after loading the OBJ and letting Fit settle 250 ms.
  Also run the same fixture with Pivot **off** as the control.
- Pick interior point `(0.40, -0.20, approximately 0.052941176)` on triangle `(1,2,3)`;
  derive its actual browser pixel through each settled projection and assert
  the independent ray hit is within 0.005 world units. A mouse right-button
  press at that pixel, without pointer travel, is the first event. The second
  event is a 40 CSS-pixel horizontal right-button drag and release. Record the
  actual delivered screen points, camera pose, projection/view matrices,
  point pick, public OBJ, and project history for both programs.

Normal-UI event order is fixed: launch fresh owned browser profiles, clear the
startup mesh, import the fixture through each visible file picker, fit and
settle; configure the visible camera controls; save a public Before OBJ and
camera snapshot; move the trusted mouse pointer over the projected interior
point; right-button down; wait one animation frame without motion and take the
held snapshot; right-button up at the same coordinate; wait 250 ms and take
the stationary After snapshot. Reset to the identical settled starting state
for Pivot off. Then repeat from a fresh settled state with one 40 CSS-pixel
rightward move between press and release. Source camera internals may be read
by passive instrumentation, but neither side's camera or geometry methods may
be invoked to perform the gesture. File export and GUI controls use the normal
visible routes. Keep raw browser traces and failed setup checks.

## Acceptance before any promotion

1. The Source framing Pivot control is enabled and, on initial switch to this
   framing, checked as in the pinned reference's `pivot=true` URL default.
   Toggling it off changes only the control; it does not deform or replace the
   document. Imported SGL camera settings can override the default.
2. With Pivot on, the first right-button down retargets the camera center to
   the independently ray-picked world point. It keeps the displayed view and
   eye fixed before pointer motion: maximum view-matrix component delta
   `<= 2e-5` in the reference and `<= 2e-5` in OBrush at DPR 1, and OBrush
   at DPR 2. With Pivot off, center/offset/zoom remain bit-for-bit unchanged.
   Mouse-up without travel must not create a delayed motion.
3. The 40-pixel drag changes the view while keeping the selected orbit center;
   Pivot-on and Pivot-off final poses differ by at least `1e-4` in one view
   matrix component within each app. The mesh's public OBJ positions and
   Undo/Redo counts remain unchanged. Record paired Source/OBRush matrix and
   projected-pivot deltas; this gate does not assume the existing orbit gains
   match until measured.
4. Repeat the OBrush pivot operation for a second distinct interior point and
   for orthographic projection. Perspective pivot presses preserve the full
   view matrix within `2e-5`; orthographic presses preserve each authored
   vertex's projected X/Y within `0.25` CSS pixels, since SculptGL explicitly
   clears its depth offset and may change the view's Z translation. A Fit or
   Reset clears the retained offset without changing the document. A rapid
   press-move-release in one animation frame must retain the final camera
   endpoint exactly once.
5. Run `cargo fmt --all -- --check`, full workspace tests, strict native and
   WASM Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, and a normal
   browser after source host/render integration. Record current build hashes,
   exact numerical differences, failed checks, and limitations. Do not
   silently widen thresholds or mark the full camera row complete.

The candidate changes only Source-framed picked orbit semantics; its source
translation/zoom response, Spherical/Plane modes, touch gestures, nondefault
FOV and different topology remain outside this gate. In particular, an
existing camera orbit-gain difference could still fail full pose parity.

Read-only static preflight before the browser lane:
`OBRUSH_SHARED_ROOT=/Users/Ajab/AI/OBrush node scripts/audit-sculptgl-camera-pivot-c334-static.mjs`.
This checks pinned source/fixture hashes and the independent affine identity
for two successive off-center pivots in perspective and orthographic views;
it is not a product or browser result.
