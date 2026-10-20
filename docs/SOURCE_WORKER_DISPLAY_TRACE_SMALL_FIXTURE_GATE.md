# Source Worker trace small-fixture gate (frozen before runtime)

Static review on 2026-10-19 compared isolated trace commit `50d4dbc`
against root `509b69e` and its C330 Twist plan. Root changed only docs and the
C329 Pinch harness after the trace base `2156e290`; there is no current code
overlap. C330 proposes selected-normal retention for imported Twist, while
this trace is admitted only to a separate, plain Source Draw Worker and does
not alter Twist. A future C330 core patch touching `source_geometry.rs` must
be rebased and reviewed at the exact normal-refresh call sites before either
branch is integrated. No product-code merge is authorized by this review.

Create one independent six-vertex noncoplanar two-face fixture, with an
original quad and an adjacent triangle sharing two vertices. Give the shared
face a nonzero crease angle so static original-face display normals differ
from public mesh normals. Save the fixture and its construction script/hash
before observing the browser output. Place the Draw contact so it selects a
masked, unchanged vertex and moves at least one neighboring vertex while a
different vertex adjacent to that moved face remains unselected. If the
ordinary Source brush admission cannot produce those conditions in one pass,
freeze a second independent fixture before looking at a result. Do not
silently weaken the conditions.

The trace must be explicitly enabled by `source-worker-held` plus
`source-worker-display-trace-v1`; use real normal-browser pointer events and
the normal Save, Undo and Redo routes. Capture Before, after each of two
accepted dabs, After, Stable, Undo and Redo public files and active VBO words.
Require one primary pass, no mirror/culling/alpha/pressure factors, actual
Worker reply IDs and monotonic sequences. Decode every binary chunk with
length/count checks and independently recompute its SHA-256. Assert every
post-dab position row belongs to its selected IDs, exact f32 words are
retained, an unchanged selected ID is present, and the adjacent unselected
ID is absent. Compare the independent replay's display-normal words to the
VBO at every stage; public normal words remain a separate invariant.

Before a 100k run, mutate one trace at a time: remove a selected unchanged
ID, flip one post-position bit, reorder or duplicate sequence, mismatch a
reply ID, omit a selected no-op dab, truncate a chunk, exceed 1 MiB/dab or
128 MiB total, and alter face order. Every mutation must fail the replay
admission. Cancel once and verify no five-stage success is emitted; the public
Before/redo and baseline GPU words must remain unchanged. Compare the trace
disabled build's ordinary preview/final delta bytes and public Save/history
to the trace-enabled run for the same controlled input. Record hashes,
parameters, differences, and failures; an instrumented run is not a timing
or SculptGL/ZBrush parity result.

The current sidecar alone lacks the result-bound JSON translator, full
Before/After/Stable/Undo/Redo harness binding, and runtime checks. The old
100k `SP-WORKER-01` failure remains unchanged. Until all checks above pass,
classification is `missing_replay_evidence`.

## C330 static rebase audit, 2026-10-19

Root `0d8af64` adds `SourceGeometryNormalState::refreshed_for_twist` in
`source_geometry.rs` and `Document::begin_source_twist_stroke` /
`refresh_source_twist_normals` in `source_geometry_document.rs`; its app
changes are in `app.rs` and `brush.rs`. A three-way `git merge-tree` over base
`2156e290`, root `0d8af64`, and sidecar `3c1e9f8` reports both branches
touching `source_geometry.rs` but **no conflict markers**. C330 inserts its
method beside `refreshed_after_selected_write`; the sidecar edits
`SourceGeometryReport`, `SourceGeometryStroke`, and the two existing ordinary
refresh paths (`apply_access_math` and `refresh_only`). It does not trace,
alter, or intercept Twist, and C330 does not change Worker transaction/host
or the ordinary Source Draw normal-refresh call sites.

There is one semantic boundary to guard: C330 can leave a valid retained
selected-normal producer after a prior Twist. A Worker Draw beginning from
that edited document must replay from the actual retained baseline, whereas
the current independent offline oracle seeds static original-face normals
from the public Before mesh. That oracle is therefore admissible only for a
fresh fixture with no prior Twist or other edited retained normal state, or
after a separately validated retained-baseline witness is added. The frozen
small fixture must start fresh and assert the baseline VBO exactly matches the
independent static-face seed before the first dab. Do not treat a clean textual
merge as proof of that semantic condition. No Cargo/browser validation or
root checkout edit was performed in this audit.
