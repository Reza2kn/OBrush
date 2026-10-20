# V01 preregistration: single-mesh render-resolution image quality

**Frozen before any V01 browser run; `SGL-RENDER-RESOLUTION` remains Open.**
[R03](SCULPTGL_RESOLUTION_DENSE_R03_RESULT.md) proved one dense stroke's
within-app geometry/history invariance, but its OBrush screenshots had an
imported sphere overlapping the startup sphere. V01 removes that confound and
tests visible pixels. It is a new gate, not a reinterpretation of R03.

## Fixed source, fixture and scene route

- Reference: pinned SculptGL `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  original bundle SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
  local mirror wrapper SHA-256
  `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
  OBrush must use a single recorded official `web/dist` product revision and
  portable WASM SHA-256 for the entire pair; do not substitute a local build.
- Fixture: R03's independently generated closed sphere, 18,242 vertices,
  36,480 triangles, 1,383,168 bytes, SHA-256
  `492dd0cfa56544bab0e816efa68a4b48b522ddaf5c70cbb2f95b6c92d167ec2e`.
  Preserve the OBJ bytes in every local receipt. Set identical material,
  background, wireframe-off and shading controls *within each app* at all
  ratios; record their values. Cross-app shaders need not match.
- Reference normal UI: fresh page, click the visible Scene **Reset** control,
  accept its confirmation dialog, assert zero meshes, then import the OBJ
  through its ordinary File chooser. Assert exactly one total and one visible
  mesh, with the fixture's vertex/triangle count, selected. No direct
  `clearScene()` or geometry API call is allowed. The mirrored GUI controller
  may set the resolution ratio, as in R03; record this non-visible setup
  limitation explicitly. If Reset or its dialog cannot be actuated as normal
  UI, V01 fails setup and stops rather than hiding a mesh in internal state.
- OBrush normal UI: fresh `?obrushprofile=sculptgl` page, focus the canvas,
  press Ctrl+O (File Open), select the OBJ in `#import-file`, and assert the
  startup object was *replaced*: one scene object, one visible and selected
  subtool, and its public project matches the fixture vertex/triangle count.
  No Add, hidden startup object, Solo workaround, direct document API or
  pre-mutated `.obrush` project is allowed.

## Frozen browser and contact matrix

Use one owned Chrome, 1600×1000 CSS pixels, 100% browser zoom, separate DPR
1 and DPR 2 processes. At each DPR, create fresh pages for 1×, 0.5× and 2×
resolution in *each* app. Use the same within-app camera pose/framing for all
three ratios. Capture normal-browser, full-color PNG screenshots before the
stroke and 350 ms after its release, plus a second static screenshot 500 ms
later. Preserve original PNGs, screen/scene rectangles and SHA-256s. No
synthetic framebuffer rendering or manual image retouching.

At the measured canvas/scene center, use R03's one trusted held horizontal
move of +12 CSS pixels: press, wait 60 ms, move, wait 120 ms, release.
Reference Brush radius 50 CSS pixels and intensity 0.5; OBrush Draw radius
0.18, strength 0.35, mouse pressure off, symmetry off in both apps. Record
the exact pointer coordinates and brush settings. If the fresh single-mesh
camera changes the projected brush footprint relative to R03, keep the
12-pixel input and report it; do not tune after seeing images.

## Pixel measurement fixed before execution

1. Decode each PNG without color management or lossy conversion. DPR 2 images
   must be exactly 3200×2000 and DPR 1 exactly 1600×1000; 2×2 box-average
   DPR 2 pixels into CSS-pixel RGB before comparison. Crop to the measured
   scene rectangle in CSS coordinates. Store the normalized crop as a PNG.
   Record actual source backing, OBrush scene/color/depth allocations and
   GL/scene errors; apply R03's ±1-pixel dimension tolerance.
2. Find the single sphere's silhouette from the before image, using the
   median RGB color of the four 20×20 CSS-pixel scene corners as background;
   foreground is Euclidean RGB distance >20. Keep the largest connected
   foreground component. Its area must cover 15–75% of the scene crop, it
   must contain the contact point and have no second connected component
   larger than 0.5% of its area. If lighting/background makes this segmentation
   fail, mark V01 **inconclusive**, preserve pixels and stop; do not adjust
   threshold post hoc. Erode this mask by 12 CSS pixels for color comparisons,
   and record the erosion area. Require at least 20,000 interior pixels.
3. Within each app and DPR, align the three normalized images at their fixed
   scene coordinates, without registration or warping. Compare 0.5× and 2×
   against 1× on the intersection of their *before* eroded masks. For both
   before and after pairs, report mean and 99th-percentile per-pixel maximum
   absolute RGB-channel difference and fraction above 64/255. Pass bounds:
   2× versus 1× mean ≤8, p99 ≤40, fraction >64 ≤0.5%; 0.5× versus 1×
   mean ≤16, p99 ≤64, fraction >64 ≤2%. The 500-ms static pair at each
   setting must have mean ≤2, p99 ≤8, fraction >32 ≤0.1% in the same mask.
   A bound failure is a visible-render discrepancy even if geometry matches.
4. For silhouette fidelity, compute symmetric boundary distance in CSS pixels
   between each resolution's foreground mask and that app's 2× mask, both
   before and after the stroke. Require 95th-percentile distance ≤2 px at 1×
   and ≤4 px at 0.5×, and maximum ≤6/10 px respectively. Report the full
   distributions and actual foreground areas. Never infer ZBrush-level edge
   quality from these bounds alone.
5. Recheck the R03 model gates: exactly one visible/total mesh throughout;
   the reference and OBrush each change at least 24 original vertices and
   gain one history step per stroke; within-app changed IDs, complete
   post-stroke positions and pre-contact public project bytes (OBRush) match
   across ratios within `1e-5` model units. Any failed scene, geometry,
   history, allocation, screenshot or image bound fails V01. Preserve
   `result.json`, screenshots, mask/normalized crops, exact hashes and error.

This gate compares each renderer with itself across scale and catches broad
single-mesh flicker, speckling and silhouette drift. It does **not** establish
cross-app pixel similarity, equivalent brush shapes, fine detail, pressure,
long strokes, browser zoom, native rendering, other materials/meshes or full
SculptGL/ZBrush parity. If it passes, the next visual gate should use
independent, longer overlapping strokes and a human-reviewed edge panel.
