# V04 preregistration: clean guide-free browser image capture

**Frozen before any V04 browser run. `SGL-RENDER-RESOLUTION` stays Open.**
[V03](SCULPTGL_RESOLUTION_SINGLE_MESH_V03_RESULT.md) left a transitional
Source Extra UI dropdown over DPR-2 *before* screenshots and OBrush's floor
grid crossed its sphere. V04 changes only normal visible UI guide setup and
capture readiness; it returns to V01's complete, unmasked scene crop. V01,
V02 and V03 remain failed at their original gates.

After normal Source Scene Clear and OBJ import, while the Scene menu is
visible, click its **Show grid** checkbox with a trusted pointer action and
assert the pinned app's `_showGrid` is false. Move the pointer to empty
topbar space. Read all topbar child dropdowns and require each computed
`opacity <= 0.001` and `pointer-events: none`. Wait at least 350 ms, then
read them all again and require the same state before screenshot. Record
both readbacks and selected mesh, history, camera, brush and render ratio
before and after; menu/grid UI handling must not alter the model or camera.
At each fresh OBrush page use the visible **Show grid** control located by
its tracked `display.grid` rectangle in the rendering panel; assert
`display_flags.show_grid === false` before capture. No direct property or
document mutation is allowed for grid setup in either app. Source Symmetry
and resolution ratio remain the separately recorded non-visible V02
controller exceptions.

Use the original V01 foreground segmentation on the **entire** measured
CSS scene rectangle, with no bottom exclusion or added mask. The exact
V01 RGB threshold, 0.5% extra-component bound, color/static/silhouette
bounds, 25×25 erosion, ≥20,000 interior pixels, and all R03/V01 scene,
geometry/history/allocation/error gates remain unchanged. Retain the same
independent OBJ, pinned Source/build, 1600×1000 CSS viewport, separate DPR
1/2 processes, fresh 1×/0.5×/2× pages per app, exact 12-CSS-pixel held
stroke, all 24 before/after/stable normal-browser PNGs and their hashes.
Any visible UI control failure, residual dropdown, grid, segmentation or
pixel-bound failure is a V04 failure and stays preserved. A V04 pass would
only establish scoped, guide-free, within-app single-mesh resolution image
stability; full visible Source UI and SculptGL/ZBrush parity remain open.
