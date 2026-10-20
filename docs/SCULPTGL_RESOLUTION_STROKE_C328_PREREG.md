# C328 preregistration: short held Draw move under resolution changes

**Frozen before C328 execution; `SGL-RENDER-RESOLUTION` remains Open.** The
[C327 press/release gate](SCULPTGL_RESOLUTION_CONTACT_C327_PREREG.md) failed
at pinned SculptGL's 1× case: the imported sphere and centered cursor were
visible, but history remained 2→2 and zero vertex words changed. The original
receipt is
`.artifacts/sculptgl-resolution-contact-c327/f908691e-b90a-4018-b667-34a2e7f4cfb6/result.json`.
The preceding startup poll was interrupted because the harness serialized
the full reference mesh on every poll; its receipt is at
`.artifacts/sculptgl-resolution-contact-c327/761e44be-06f2-4c8b-8bdf-2fd9e7258e49/interrupted-harness.json`.
Neither attempt is a product parity result. C328 changes only the input
protocol: one trusted left press, a held 12 CSS-pixel horizontal move after
60 ms, 120 ms hold, and release. This matches SculptGL's update-on-move path.

Use the same pinned source, independent sphere fixture, official OBrush
portable WASM, 1600×1000 CSS viewport, DPR 1/2, fresh-page imports and
0.5/1/2× ratio changes as C327. The exact source/build/fixture hashes and
cursor positions must be recorded in each run. Source resolution still uses
the mirrored GUI controller because the mirror's visible topbar is not a
reliable hit target; OBrush uses its visible slider. This is scoped paired
input evidence, not full normal-UI parity.

Acceptance is frozen: each ratio/app must change the selected imported
sphere with a single completed history step and no GL/scene error; both ends
of the CSS segment must be within the viewport. The fixed CSS start and
12-pixel end must be identical within each app across ratios. Before-contact
OBRush public project bytes, selection, Draw controls and history must match
across ratios; OBrush cursor start must be within 2 CSS px of the requested
point. Reference backing and OBrush actual scene allocations must match
their respective CSS scene × ratio dimensions within one pixel. Within each
app, 0.5× and 2× must retain the same vertex count, changed-vertex IDs and
post-stroke positions within 1e-5 model units of 1×. If the source varies,
record that independently and do not promote OBrush by analogy. Any failed
case remains a failed receipt; a product candidate is justified only by a
measured OBrush-specific discrepancy.

Other tools, long strokes, pressure, symmetry-off, browser zoom, native GUI,
cross-app brush-shape equality and full SculptGL/ZBrush parity remain open.
