# MRP04: first-press and first-dab causal split — preregistration

This is a proposed **read-only** follow-up to [MRP03](SCULPTGL_MILLION_MRP03_RESULT.md),
not a product optimization. MRP03 used a fresh owned Chrome profile, exact
999,002-vertex SGL SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`
and 60-second/400-requested-moves/s protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
Its diagnostic passed 17/17 independent file/history/shape checks but failed
0/4 responsiveness gates. Initial press had a 520.6 ms input interval inside
a 555.8 ms app callback; the early first useful dab had 367.8 ms geometry
inside a 391.7 ms app callback. The steady path spent 22.41 ms in cursor/
overlay and 21.68 ms in geometry on each of 994 same-frame dabs. Removing
only cold setup cannot close the steady P95; removing only hover cannot close
the first-press and first-dab RAF maximum.

## Concrete competing causes

The saved MRP03 SGL fixture has `scene_v7.objects[0].affine.interaction_center`,
so `Document::source_geometry_interaction_center()` returns metadata without
building a normal state for this cohort. With no retained Source state,
`prepare_source_geometry_press_alpha()` constructs a temporary
`SourceGeometryNormalState::seed(mesh)` at press, and the first controlled dab
constructs another seed in `Document::apply_source_geometry_controlled_dab()`.
The inspected code proves two construction calls but **does not** measure
their durations or prove that deduplicating them preserves the exact binding,
allocation and rollback contract. The press also performs `scene_pick()`
before `begin_source_geometry_stroke()`, and the latter performs blocker,
Source hit, stroke capture, contact and alpha preparation. Duplicate mesh
validation or indexed picking may dominate instead. The first dab's nested
core deformation maximum was 333.9 ms; the current timing does not separate
seed, validation, tree construction, selection and actual deformation.

## Probe and attribution gates

Use a temporary, feature-gated WASM diagnostic with fixed-size scalar
storage, never per-frame JSON. Keep the ordinary control package, runner
events, queue policy, geometry, renderer, public files and thresholds
unchanged. Against one pinned integrated commit, record entry/exit timestamps
and call counts in the **same** browser clock for:

1. Press `scene_pick()`; `begin_source_geometry_stroke()` total; its
   `blocker_controls`, `begin_admitted` (with Source hit and begin-stroke
   submarks), press-contact preparation and press-alpha preparation.
2. Each absent-state `SourceGeometryNormalState::seed(mesh)` call, tagged by
   press-alpha or first controlled dab; the complete validation before it;
   retained-state match checks; and the post-seed deformation/publish path.
3. Hover/cursor pick in the press and first-dab frames, so those intervals
   can be reconciled with MRP03 `App::update` and RAF timestamps.

The browser fixture must be confirmed to have no retained Source state and
the expected affine interaction center before timing. Require exactly one
press-alpha seed and one first-dab seed in this cohort; any other count
rejects the duplicate-seed attribution and triggers code-path review.
Bound timestamp count and fail closed on overflow, missing marks, clock
misalignment or seed/validation error. Run an uninstrumented same-revision
control and one diagnostic, preserve actual delivered-dab counts and complete
17-check Before/After/Stable/Undo/Redo audits. Preserve actual GPU VBO
position/display-normal words if a later product candidate uses them; this
diagnostic does not invent an oracle from saved public mesh normals.

Report initial-press and first-useful-dab nested durations individually,
with exact call counts and same-frame residuals. A seed-reuse candidate is
eligible for a separate frozen experiment only if the two measured seed
calls together explain ≥25% of the respective app callback or ≥10 ms each,
and a same-state Rust/WASM microbenchmark predicts ≥5 ms net reduction
without larger peak memory. Otherwise reject that mechanism. Even if admitted,
the candidate must retain the original four full-cohort gates (RAF P95
≤33.34 ms, maximum ≤250 ms, app CPU P95 ≤16.667 ms, Chrome release-command
acknowledgment ≤100 ms) and may claim only its measured substage until all
four pass.

Any reuse design must keep the shared Rust core authoritative and distinguish
an accepted first dab from no-hit, protected, canceled, failed or stale
contacts. The temporary press seed cannot become a persistent state merely
because it was built: storage caps, exact normal f32 words, mask/alpha and
X-mirror behavior, complete one-step Undo/Redo, original position/face
validation and rollback on second-pass failure must remain checked. Compare
exact old/new public files and retained normal-state words for independent
authored success, no-op, rejection and mirror fixtures before attempting a
browser performance run. Preserve failed checks and revert if any exact
contract fails. A later combined hot-path design still needs a validated
cursor pick and steady geometry budget; MRP04 alone cannot establish
SculptGL/ZBrush parity.
