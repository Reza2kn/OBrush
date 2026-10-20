# C307 preregistration: Source resolution under browser DPR 2

**Status: proposed, unrun. The `SGL-RENDER-RESOLUTION` gate remains open.**
This protocol was fixed before changing product code. C247 measured the
Source 0.5–2× slider only at browser DPR 1. The pinned SculptGL kernel's
`onCanvasResize` sets `canvas.width = viewport.clientWidth * _pixelRatio` and
likewise for height; `_pixelRatio` starts at 1 and the slider changes it.
`setMousePosition` applies `_pixelRatio` to page-coordinate offsets. These
paths do not multiply by `window.devicePixelRatio`. In OBrush at this baseline,
`paint_objects_at_resolution` multiplies `PaintCallbackInfo::viewport_in_pixels()`
by the slider value, and the host viewport pixels already include DPR. At
DPR 2, the 1× scene is therefore predicted to rasterize at about twice the
reference's CSS-pixel density per axis. This prediction needs normal-browser
measurement before a product change is accepted.

## Frozen inputs

- Reference: SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  bundled `sculptgl.js` SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
  mirrored wrapper SHA-256
  `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
- New independently authored eight-face, two-tetra OBJ:
  `crates/obrush-core/tests/fixtures/independent-resolution-dpr-c307.obj`.
  It is 367 bytes with SHA-256
  `fab0a005d7c4cb666d4c17f1b6a5d115a4051abfcd0df254182c591307af2300`.
  Record the exact OBrush commit/WASM hashes in every receipt before
  interpreting measurements. The fixture is loaded in
  both browsers through their normal import paths after startup.
- Owned Chrome 1600×1000 CSS viewport, emulated device scale factor 1 and 2,
  browser zoom 100%, app UI zoom 1, same fixture bytes and camera preset per
  app at each ratio. Capture all six 0.5×/1×/2× cases per DPR without resizing
  the CSS viewport. The cameras and shaders are not calibrated between apps.
- Source reference ratio changes may call its normal `onPixelRatio` controller
  as C247 did where its mirrored topbar is not reliably clickable. OBrush
  must use the normal visible Source `Resolution` slider. No debug API may
  set OBrush's slider or modify its document.

## Gates fixed in advance

1. Log the reference CSS viewport and canvas backing dimensions for each
   ratio. They must satisfy `canvas = round(CSS viewport × ratio)` per axis
   within 1 pixel at both DPR values. Log `window.devicePixelRatio` and the
   reference's `_pixelRatio`; both ratios must be explicit.
2. Log OBrush canvas CSS and backing sizes, `egui` viewport rectangle in CSS
   points, reported scene pixels, and *actual* WebGL color/depth allocations.
   At DPR 2, an accepted Source fix must allocate a scene target within 1
   pixel per axis of `round(OBRush scene CSS size × slider ratio)` at 0.5×,
   1× and 2×. DPR 1 must retain C247's dimensions within 1 pixel. The host
   UI canvas may retain its DPR-sized backing; report it separately rather
   than claiming whole-canvas parity.
3. The scene must remain visible in screenshots at all 12 state pairs;
   0.5×/1×/2× OBrush screenshot hashes must be distinct within each DPR,
   with no GL or scene errors. A 1→0.5→2→1 round trip must restore the exact
   saved `.obrush` bytes, history counts and selection. Compare the normal
   GPU object's vertices/indices with the saved object on a spot check.
4. With the pointer at a documented CSS coordinate on one tetra, capture
   Source brush circle at 1× and 2× for each DPR. Its screen-space CSS radius
   should differ by no more than 2 pixels within one DPR, because this slider
   controls render resolution, not brush size. Record the app's measured
   radius and visible screenshot; if measurement is obstructed, fail this
   subgate and retain the attempt.
5. Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, strict
   native and WASM Clippy, `bash scripts/build-web.sh`, and
   `bash scripts/package-macos.sh` after candidate product changes. Record
   build hashes and a normal browser run on that build. Preserve any failed
   baseline or candidate receipts.

The corrective candidate may compute Source scene pixels from the user-point
viewport width/height times slider ratio, instead of scaling the physical
host viewport. It must keep the host dimensions for camera/picking and compositing,
and keep bounded allocation/fallback. This is a candidate, not an accepted
design until the paired browser gates run. Native Retina behavior, browser
zoom other than 100%, camera/shader pixel equivalence, broad brush-input
parity and complete resolution-family parity remain open regardless of C307.
