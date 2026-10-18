# Browser responsiveness in 0.1.4

The 0.1.3 browser lag was reproduced in an isolated headless Chrome test. During
crossing Draw strokes, browser scheduling paused for 1.70–1.75 seconds and the
main thread recorded tasks lasting 1.17–1.19 seconds. Final 0.1.4 removes those
long tasks in the same two bounded input trials while retaining adaptive detail,
geometry safeguards and actual 4× viewport antialiasing.

The fix reduces core sculpt work, builds the WASM release for execution speed,
and distributes captured dabs across successive redraws. The queue preserves
accepted dabs, their brush settings and stroke history order. Normal processing
uses at most two dabs per frame and checks an 8 ms budget **between** atomic
dabs. An individual dab can exceed that budget. Save, undo and other commands
complete queued work before acting; a large pending backlog can still delay a
command.

The input adapter still samples aggregate pointer state rather than replaying
every raw event. Multiple complete strokes or bent movements arriving within
one frame can be combined. Starting another stroke also tests the currently
committed visible surface before pending geometry is complete. Queue retention
therefore covers accepted dabs, not every possible raw input path. The release
capture protects an active stroke's endpoint when its release and a command share
a frame; commands issued mid-drag without a release flush accepted work but do
not sample newly arrived raw movement first. These remain input coverage limits.

## Matched browser measurements

Two old-build trials and two final-build trials use Chrome 155.0.8059.39 on an
Apple M2 with ANGLE Metal, a 1600×1000 canvas and device pixel ratio 1. Each starts
with a fresh 40,962-vertex sphere, Draw radius 0.180, strength 35%, X symmetry,
and the same camera. Node dispatches identical pointer schedules independently
of browser acknowledgements: a continuous line has 39 events over 800 ms;
crossing lines have 54 events over 1320 ms, including a 200 ms pause.

These are actual mesh `drawElements` submission intervals, measured on the CPU.
They establish how regularly the app submits updated mesh frames; they do not
measure GPU presentation or physical input-to-photon latency.

| Input / trial | 0.1.3 mesh submission p95 | 0.1.4 mesh submission p95 | 0.1.3 longest scheduling gap | 0.1.4 longest scheduling gap | Long tasks >50 ms, old → new |
| --- | ---: | ---: | ---: | ---: | ---: |
| Continuous, 1 | 145.8 ms | 28.7 ms | 333.4 ms | 16.8 ms | 5 → 0 |
| Continuous, 2 | 130.7 ms | 28.8 ms | 316.7 ms | 16.8 ms | 4 → 0 |
| Crossing, 1 | 496.7 ms | 29.1 ms | 1699.8 ms | 16.8 ms | 5 → 0 |
| Crossing, 2 | 504.5 ms | 24.6 ms | 1749.9 ms | 16.8 ms | 5 → 0 |

The independent `requestAnimationFrame` scheduling pump has final p95 gaps of
16.7–16.8 ms. Actual mesh submissions occur later within callbacks, so their
intervals above are different. Crossing mesh-submission maxima of 167–170 ms
include the intentional pause between strokes. Idle and camera orbit show no
long tasks in either build; idle does not continuously redraw the mesh.

Both final trials delivered all 39 continuous and 54 crossing events to the DOM.
The old build coalesced continuous input to 22–23 events and crossing input to
13. Final DOM queue-delay maxima during these Draw paths are 15.9–17.5 ms,
compared with up to 619.6 ms before. These values include synthetic dispatch
scheduling and do not establish mouse or pen hardware latency.

Crossing input plus observed drain falls from 2.82–2.86 seconds to 1.60 seconds.
That total includes the 1.32-second scheduled gesture and a required quiet
interval of at least 250 ms. The drain check observes input, mesh uploads and
callbacks; it does not directly inspect the private Rust queue.

Instrumented large `bufferData` calls consume only 0.5–0.6 ms of CPU submission
time across each old crossing phase; the updated build submits more frames and
uses 4.2–4.4 ms. These are not total GPU transfer or rendering times. The large
CPU tasks and collapsed event/frame delivery identify sculpt scheduling as the
main observed lag source in this fixture.

## Saved output and pending commands

Both final timed trials actually modify 2,976 original vertices and save a
42,526-vertex / 85,048-triangle crossing sculpt. Undo changes the correct last
stroke; redo restores the exact saved mesh. Final snapshots match between the
two trials. The saved crossing mesh has finite positions, valid indices, no
degenerate faces and no adjacent face angles over 60° or 90°; its maximum is
36.52°. This is a local sanity check, not a general self-intersection guarantee.

A separate burst check sends eight zigzags, a final 250-pixel segment, mouseup
and a real Save-button click without awaiting acknowledgements or yielding an
animation frame between the final segment and command. The recorded app rAF
callbacks confirm that mouseup and Save release share one input window. Before
the command, only 81,992 triangles have been submitted; Save contains the
completed 94,816-triangle result. Its bytes match a later quiet Save and the
independent quiet-control run that includes the same final segment. Whole-stroke
undo restores the original sphere and redo restores the exact saved result.

This stress command records a 3.001-second main-thread task while synchronously
finishing pending edits and saving. Ordinary Draw phases exclude serialization,
save and history commands; their zero-long-task result does not mean every
operation is instantaneous.

Browser baseline and final fields are not asserted bit-identical: for trial 2,
face indices, colors and masks agree while the maximum corresponding position
difference is about 2.02e-7. Event coalescing differs between builds. No browser
output tolerance was preregistered. The fixed-world-ray native comparison below
is the separate exact-output preservation test.

[The browser record](evidence/browser-performance-comparison.json) pins the
final WASM SHA-256 `a74998f7a85de0b5e95846535552485e38a86a26bc5f8b14f1b8d79f1275fd8c`,
frozen artifacts, identical harness, full scheduled traces, raw timing/export
hashes, snapshots, intermediate passes and the corrected harness failure. All
Chrome processes/profiles and loopback servers are isolated test resources;
owned Chrome processes exit and owned servers close. No desktop, accessibility,
user-tab or user-process control is involved.

These measurements cover one machine, browser/backend and DPR, with two short
trials. Native GUI, Safari/Firefox, alternative GPUs, scaling/resize/context
loss, large imported meshes, long sessions and physical tablet pressure remain
outside this check. It establishes improved browser responsiveness on the tested
paths, not a fixed FPS guarantee or ZBrush parity.

## Native core and geometry preservation

The smoother Draw surface in 0.1.3 was expensive to update. On this Apple M2,
a current-surface pick plus one Draw dab typically occupied about 34–37 ms of
native CPU time. Several dabs processed before a repaint could therefore delay
visible feedback. This CPU diagnosis agrees with the browser measurements above;
these native timings themselves do not measure browser frames.

The core now prepares smoothing from compact adjacency, updates normals near
changed geometry while preserving the existing accumulation order, and rejects
refinement candidates outside the footprint or below the edge target sooner.
The adaptive detail and geometry safeguards remain active. App input scheduling
and actual WebGL performance are checked separately.

The independent comparison freezes the old 0.1.3 source and applies identical
world rays to fresh level-6 spheres: radius 0.180, strength 35%, X symmetry on,
Draw, no inversion. Each dab picks the current surface. Release-mode timings
exclude external diagnostics, exports, history, app input, and rendering.

| Fixture | Dabs | Old sculpt median / p95 | Updated sculpt median / p95 |
| --- | ---: | ---: | ---: |
| Four overlapping lines | 164 | 35.10 / 41.50 ms | 9.25 / 9.94 ms |
| Four crossing rounds | 328 | 33.25 / 41.57 ms | 8.85 / 10.00 ms |
| Eight loops | 520 | 36.13 / 45.90 ms | 9.49 / 11.33 ms |
| Repeated raised point | 128 | 34.06 / 40.55 ms | 8.93 / 9.82 ms |

Median sculpt cost improves by about 3.8 times. Updated pick-plus-sculpt p95 is
10.22–11.78 ms across these four fixtures. One dab remains an atomic core edit;
these measurements establish neither a hard time bound nor application FPS.
OS scheduling, background work and thermal behavior can affect wall time.

An instrumented copy of the frozen baseline identifies adjacency rebuilding as
the largest cost: 795 rebuilds consumed 16.47 seconds of 33.61 seconds of total
sculpt time. Normal rebuilds consumed 4.33 seconds. Inclusive timers overlap,
so their totals must not be added. The instrumented and unmodified baseline
exported exactly identical OBJ files for all four fixtures.

The improvement preserves the previous smoother surface. All twelve original
quality fixtures, including tilted silhouette strokes and a finer level-7 sphere,
have byte-identical final OBJ files and exactly identical numerical quality
measurements versus 0.1.3. Every dab retains zero original-face inversion events.
All local Draw gates still pass: zero adjacent angles over 60°/90°, roughness
p95 at most 0.12, triangle quality at least 0.15, and bounded edge length with
useful height retained. These are local fixture checks, not a general guarantee
against self-intersection.

Four fixtures also serialize and compare every original and appended position,
normal, triangle index, mask and color: zero bit differences. The masks and
colors here are fresh-sphere defaults; imported, painted or masked edits need
their separate regression tests. The fixed geometry does not reconstruct the
user's unsaved sculpt and does not establish ZBrush parity.

[The machine-readable record](evidence/browser-core-performance.json) contains
source/probe/output hashes, both baseline runs, intermediate candidates, all
threshold counts, exact geometry comparisons, timing definitions and limits.
Frozen sources, complete ray arrays, OBJ/mesh exports, failed historical checks,
and reproducible instrumentation are retained under ignored `.artifacts/`.
[The earlier Draw investigation](DRAW_QUALITY.md) remains historical evidence.

The [0.1.4 build record](evidence/browser-performance-build.json) pins the final
source, web/native artifacts and 84 passing tests, with formatting, strict
native/WASM Clippy, pure-core WASM compilation and macOS signature checks.
