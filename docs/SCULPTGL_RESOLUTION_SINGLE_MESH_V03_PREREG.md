# V03 preregistration: exclude scene UI and floor-grid strip

**Frozen before any V03 browser run.** [V02](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_RESULT.md)
failed its exact V01 pixel segmentation at both DPR values; its image bounds
were never evaluated. Untouched V02 PNGs show two independent contaminants:
the pinned Source **Scene** dropdown remains open across the upper-left
sphere, and the Source floor grid forms a separate component at CSS scene
`y=879..959`. OBrush's floor grid intersects its sphere. V03 makes only
the following visual capture/mask amendment; V01 and V02 remain failed.

After the same visible Source clear-confirm-import and recorded non-visible
Symmetry and resolution-controller setup, close the Scene dropdown with a
trusted pointer click on empty topbar space. Verify the dropdown is hidden
from read-only DOM state and record that check before the first capture.
The click must not change selected mesh, vertex count, history, brush, camera
or render ratio. No equivalent click is needed in OBrush. Keep both apps'
existing grid display settings unchanged; record them across all ratios.

The pixel analyzer continues to save the untouched full screenshots and
full CSS scene crops. For **Source only**, the segmentation input excludes
the bottom 10% of the scene crop (`y >= floor(0.90 * height)`), which contains
the observed floor-grid strip. First assert that every non-background sphere
pixel in the central x-band (`0.30w..0.70w`) belongs to a main connected
component whose lower bounding edge is strictly above `0.85h`, and that the
contact center is within the retained area; otherwise V03 fails segmentation.
For OBrush, use the complete scene crop, including its grid. The Source mask
outside its retained area is fixed false for all before/after/stable images
at every ratio and DPR; static/color regions and silhouette distances use
the same fixed analysis area. No per-image crop adjustment, threshold search,
object recentering, warping or manual retouching is permitted. A remaining
second component above 0.5% of the selected sphere component fails V03.

All other [V01 gates](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_PREREG.md) and
[V02 controller setup limitation](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_PREREG.md)
remain unchanged: pinned source/build/fixture, one total and visible mesh,
fresh 1600×1000 CSS pages at DPR 1 and 2 for 1×/0.5×/2×, same camera and
12-CSS-pixel held stroke, ≥24 changed original vertices, +1 history step,
identical within-app changed IDs/model positions, allocation and error
checks, normal-browser before/after/stable PNGs, and the exact V01 color,
static and silhouette bounds. The OBrush grid crossing the sphere remains
a known interpretation limit; if it causes segmentation failure, retain the
failure and preregister another gate rather than changing V03. V03 can show
only within-app single-mesh resolution stability. It cannot establish
cross-app visual equivalence, ordinary-UI parity (Source controllers remain
non-visible), long-stroke quality or complete `SGL-RENDER-RESOLUTION` closure.
