# MRP07: sparse press picked-normal feasibility gate

MRP04 attributed 294.8 ms to an absent-state press-alpha seed and 221.8 ms
to a separate first controlled-dab seed on the 999,002-vertex public fixture.
MRP05/MRP06 did not admit retaining the press seed across frames under their
browser memory gates. This is a **new read-only feasibility test** for a
lower-memory approach: derive only the picked original-face corner normals
needed to prepare the press alpha tracks, discard those small values, then
allow the existing first-dab full seed to run unchanged. It is not permission
to publish a product optimization, and the failed MRP05/MRP06 results remain
failed.

The sparse calculation must reproduce the existing
`SourceNormalField::compute(StaticOriginalFaces)` arithmetic: use stored
original quads or triangle faces in their original order, compute each face
normal in Float64 with the same cross-product order, cast each component to
Float32, scatter in **descending face order** into Float64 sums and counts for
at most the eight corner IDs of two contacts, then cast each averaged normal
to Float32. It must validate every face normal, preserve duplicate corner
incidence, and use the existing picked-normal interpolation and track-update
routine rather than a new distance/weight formula. The calculation must use
only bounded per-contact scratch and must never mutate the mesh, document,
history, normal cache or stroke. The normal-field storage-admission checks
and geometry validation must remain observable; if exact rejection parity
cannot be established, retain the old full-seed press path.

Before any host/product edit, compare the sparse result to a freshly built
full `SourceGeometryNormalState` on independent triangle and quad fixtures:
flat, curved, extraordinary valence, duplicate corner incidence where valid,
zero/undefined normal, malformed contact, exact corner hit, mirror pair,
masked/protected contact, and same-size public topology/position edits. Check
each returned disposition, report field and `SourceGeometryAlphaTracks`
Float64 bit pattern, including rejection class, against the full path. On the
same saved 351-MB public project (SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`),
compare at least 32 deterministic actual original-face contacts spanning
front/back, triangle boundaries and seam-adjacent rows; do not select only
smooth central contacts. Retain the complete contact list and failures.

Time five alternating fresh-process native pairs and five owned-Chrome WASM
pairs of **press alpha preparation only** on that file, then separately time
first-dab full seed unchanged. Record actual tool/build hashes, no greater
peak WASM linear-memory pages, and exact output words. The diagnostic is
admitted for a later product candidate only if all exact-word/rejection
checks pass, median press CPU time falls by at least 50 ms in both runtimes,
and its deterministic WASM heap maximum is no larger. Sampled Chrome RSS is
reported, never silently interpreted as proof of no increase. A later
product candidate must independently verify press-only alpha preview,
no-contact/no-op/rejection, mirror, one-step Undo/Redo, public file and GPU
words, then pass the frozen 60-second four-threshold normal browser gate.
