# C275 prospective precise held-pointer input path

Freeze this follow-up before changing the C274 product candidate. C274 failed
only its equal held-radius condition: OBrush reported
`76.6925048828125`, `76.69247436523438`, `76.6925048828125` screen pixels
where the pinned SculptGL Source run used equal 80-pixel held distances. The
C274 result and failed audit remain immutable in
`SCULPTGL_BRUSH_LOCK_C274_FAILED.md`.

The C275 hypothesis is that the OBrush normal browser pointer route narrows
DOM double-precision screen coordinates to `egui::Pos2` `f32` before the Source
Lock press/current samples are captured. First audit every producer and
consumer along the normal browser pointer-to-SourceGeometryInput path,
including frame buffering, viewport and camera transforms, queued held input,
alpha orientation, local/legacy tools, and native input. Add a bounded precise
`f64` coordinate path only where an actual raw browser value can be preserved;
do not synthesize missing precision or round a computed radius to satisfy a
fixture. Keep existing `egui` and legacy coordinates unchanged for their
separate consumers. Source geometry and history remain in `obrush-core` and
native/browser geometry behavior shared.

Use the same pinned reference commit, independent fixture and alpha hashes,
settings, press/+80 X/-80 Y/back +80 X/release trace, C273 Source oracle,
held-radius tolerance `1e-6`, changed-ID membership and count (exactly 26),
paired position tolerance `1e-6`, active GPU position/display-normal checks,
history checks, no press dab, rotated/repeated/released preview and quiet-frame
conditions from C274. Run the normal-UI pair once with fresh exact JS/WASM and
harness hashes. Preserve all failed checks. If the precise input path cannot
be proved or the frozen paired gate still fails, retain the failed receipt
and revert the product candidate rather than promoting a partial Lock route.
