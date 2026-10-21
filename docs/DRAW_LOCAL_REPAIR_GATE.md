# Local Draw repair: frozen consumer gate

This is an OBrush regression gate for sustained local Draw work. It does not
establish SculptGL or ZBrush parity. The independent authored core fixture is
`crates/obrush-core/tests/draw_local_repair_gate.rs`; its three tests are
deliberately ignored until an integrated candidate passes them unchanged.

## Authored core fixture

The fixture builds a 21×21 open, curved, consistently wound grid from OBJ text,
with one test moving a central grid point to leave an interior sliver. It sets
per-vertex RGB/material values and protects the outer band with masks. Each
contact performs a fresh vertical raycast onto the *current* surface, then
applies Draw at strength 0.35, zero drag, no symmetry. The three fixed cases
are: curved patch at radius 1.0 for 12 contacts; curved sliver at radius 0.18
for 12 contacts; and the same sliver for 128 contacts. A useful contact must
lift the ray-hit surface by at least `0.002 × radius` and report changed
vertices. No run of three ineffective contacts is allowed; at least 8 of the
first 12 and 80% of the 128 must be useful, with cumulative lift at least
`0.02 × radius`. Refinement is capped at 1,024 added vertices per contact.

Every contact checks finite positive-area faces, at most two oppositely wound
faces per interior edge, a single path/cycle vertex link, and finite face
quality/fold measurements. Original positions, masks, colors and materials
outside the nominal brush support plus 0.18 world units must remain exact.
The full 12/128-contact stroke must Undo and Redo to exactly equal serialized
project JSON.

Baseline on source commit `14be24e`: both 12-contact cases pass. The
128-contact case fails at contact 75: scale `0.00390625`, 557 reported changed
vertices, `detail_limited=true`, but measured surface lift only `0.00001692772`
against the frozen `0.00036` useful threshold. Preserve this failure; a tiny
reported edit is not a satisfactory repair.

Run: `cargo test --locked -p obrush-core --test draw_local_repair_gate -- --ignored --nocapture`.

## Private real-model browser replay

The user-supplied STL stays outside the repository. The local input SHA-256 is
`5502a1c2ce42adbf70729cb1b00a8110eb8c5ace5d1f6f332c2ad2e1e7b9ef9c`.
Before any edits, normal browser Open must yield 369,407 vertices, 738,900
retained triangles and a report of four skipped exact-repeated facets. Save
the imported `.obrush` and its full-resolution PNG. Use an owned Chrome profile
at 1280×900 CSS pixels, DPR 1, the official `scripts/build-web.sh` output,
and record its source/WASM hashes and selected backend. Use visible controls
and trusted pointer events only. The existing local front-view control is
`yaw ≈ 0.35`, `pitch ≈ -1.42`, distance ≈50.38; one ordinary zoom step yields
distance ≈39.24. An earlier opposite-side attempt at pitch +1.46 failed to
edit because its camera ray treated the viewport right/bottom coordinates as
width/height. Preserve that failure; commit `c9c4d8c` calibrated a separate
positive-pitch route, which does not replace the three front-view points.

The calibrated positive-pitch route uses captured camera eye
`(-2.7587698,34.199276,20.482391)`, target
`(-0.27341652,-4.7973504,16.926394)`, 40° vertical field of view, and
viewport bounds `[left=225,top=57,right=1045,bottom=869]` in CSS pixels.
Its frozen 12-contact sequence is `(635,463), (639,463), (643,463),
(647,463), (651,463), (647,463), (643,463), (639,463), (635,463),
(643,459), (643,463), (643,467)`. The default-strength r=1.0 and r=0.18
lanes start from separate fresh imports with X symmetry off. Capture picks,
per-contact displacement and timing, saved mesh, original-resolution pixels,
Undo/Redo and Reload under the same numerical and latency gates below. The
exact core ray/control is `crates/obrush-core/tests/spark_import_draw.rs` at
`c9c4d8c`; ordinary browser evidence is still required.

On the front view, independently confirm actual surface picks at three fixed
CSS points in the 1280×900 viewport: torso (575,270), upper leg (645,550),
and lower leg (750,750). A miss fails setup and is retained; do not silently
move a point after seeing the result. At each point, test Draw strength 0.35 at
radii 1.0 and 0.18 in separate fresh imported documents. With X symmetry off,
apply 12 repeated held-contact dabs at 120 Hz, save after contacts 1, 6 and 12,
and capture original-resolution PNGs with cursor moved away. Repeat one
default-settings lane with X symmetry on. Browser geometry must show a
measurable local surface displacement of `≥0.002 × radius` in at least 8 of
12 contacts, cumulative displacement `≥0.02 × radius`, and no three-contact
stall. Count changed pixels (absolute RGB channel delta ≥10) in a 160×160
crop centered at each hit; require at least 25 changed pixels by contact 12.
Retain the PNGs for direct inspection of new spikes, gaps and faceting.

Numerically inspect before/after project meshes: no nonfinite or zero-area
faces; no increase in count of guard-quality faces below `q=0.1` or folds
over 60° in the edited patch; exact original attributes and positions beyond
the union of nominal brush supports. Keep scene validity and GPU errors null.
Save, Undo, Redo and Reload must reproduce exact before/after project hashes.
Record every input dispatch and visible revision timestamp: 60 fps is the
smooth target (`p95 ≤33.34 ms`), `p95 ≤100 ms` is a permissive dense-model
ceiling and must not be called smooth. Pointer release must acknowledge within
100 ms, with no edits or brush work queued after 150 ms. Do not close the
complaint on a core-only pass or on a full-figure screenshot.
