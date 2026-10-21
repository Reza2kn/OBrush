# OBrush feature parity contract

**Native consumer smoke:** [the 2026-10-20 packaged macOS run](NATIVE_UI_SMOKE_2026_10_20.md)
verified one visible Draw gesture with 834 changed vertices, byte-exact
public Undo/Redo saves, and byte-exact project reopen/save. This does not
establish the native brush family, pressure, performance, SculptGL, or ZBrush
gates.
[M01's native input regression](NATIVE_MOVE_MULTIFRAME_M01_REPORT.md)
shows a multi-frame Move at radius 0.037/79% with X symmetry changes both
sides of a fresh sphere with valid faces and exact Undo/Redo. A compressed
one-frame press/move/release remains a no-op; the earlier automated native
gesture did not record enough input timing to diagnose its cause or prove
visual tearing fixed.
[M02's bounded ordered-event recovery](NATIVE_MOVE_SAME_FRAME_M02_REPORT.md)
now handles a mouse Move press/move/release delivered in one egui frame;
the focused host gate passed exact deformation and Undo/Redo at radius
0.037/79%, and a twelve-pull numerical quality check stayed valid. A normal
browser Move gesture also passed public Save/Undo/Redo, but its fast-event
burst crossed 11 frames. An actual one-frame consumer witness and visual
tearing comparison remain open.
[M03's packaged native attempt](NATIVE_MOVE_M03_NATIVE_REPORT.md) reached
the visible Move settings and saved an untouched sphere, but computer-control
coordinate input failed before a stroke. It is inconclusive for native Move
deformation and rendered tearing.
[Q02's twelve-contact normal-browser Move run](NATIVE_MOVE_REPEATED_VISUAL_Q02_RESULT.md)
passed bounded mesh, GPU, release-idle and exact eight-step Undo/Redo gates
at radius 0.037/79% with X symmetry. Four contacts were no-ops, and the
full-sphere screenshots show marks too small to judge close-up triangular
tearing, so the user's visual complaint remains open.
[Q05 and Q06 close-up Move attempts](NATIVE_MOVE_CLOSEUP_Q06_RESULT.md)
retained original-resolution images but failed their paired-camera gates.
Offline analysis of the Q02 saved mesh found a sharp-looking region with
minimum local face quality about 0.1 and folds up to 50.26°, both allowed by
the current Move guard. This is a numerical diagnosis, not a matched visual
pass.
[B02's tangential Move guard](NATIVE_MOVE_GUARD_B02_RESULT.md) keeps a
30° local fold limit for mostly sideways small-radius pulls while retaining
broader outward-pull behavior. Twelve repeated browser contacts on one fresh
sphere stayed below 30.01° with useful displacement and exact saved/GPU
geometry and Undo/Redo. The original user-scale tearing complaint remains
open: this narrow fixture and camera view cannot prove broad visual quality
or repair an already folded mesh.
[CR01's repeated Crease browser comparison](SCULPTGL_CREASE_OVERLAP_CR01_REPORT.md)
preserves a failed paired protocol because OBrush's actual default pressure
setting differed from the frozen pressure-off precondition. On one independent
18,242-vertex sphere, eight Crease strokes changed both mirror sides without
new tiny or folded faces and had exact eight-step Undo/Redo. This scoped safety
observation does not resolve close-up tearing or Crease parity; CR02 freezes a
closer default-pressure check.
[CR02's closer Crease browser run](SCULPTGL_CREASE_OVERLAP_CR02_REPORT.md)
observed continuous grooves, finite mesh, no new tiny or flipped faces, and
exact eight-step Undo/Redo on the same sphere. Its frozen gate remains failed:
the closer camera was set before the strokes, per-dab side traces were not
retained on both hosts, and legacy mouse pressure differed from the matched
DOM force premise. Close-up tearing on other sculpted meshes remains open.
[The supplied saved-sculpt audit](USER_SCULPT_FOLD_AUDIT.md) found 78 already
present adjacent-face folds above 90° in the original version-1 project,
despite a valid closed mesh. The current fixed-topology Crease guard does not
repair that damage, and generic local smoothing worsened minimum face quality.
An opt-in constrained repair remains a proposed, unimplemented remedy.
[The stationary Draw build gate](DRAW_STATIONARY_BUILD_GATE.md) reproduces
the fresh-sphere plateau at the default 0.180/35% settings: the first
unchanged same-point contact is 96, and picked height at contacts 128 and
256 is identical. Two bounded salvage attempts and a tangent-only fairing
trial still miss the frozen growth/quality gate; their source changes were
discarded. An ignored failing regression retains this public-alpha blocker.
[The later Sparkdancer recovery checks](SPARK_DRAW_2026_10_21.md) pass 256
default Draw contacts and two leg checks through the ordinary browser, with
local refinement and exact public Undo/Redo. The separate sphere512 endurance
test and the browser 30 fps pacing gate remain open; these scoped passes do not
establish unlimited buildup or general tearing closure.
[MRP10's persistent Worker trial](SCULPTGL_MILLION_WORKER_RESIDENCY_MRP10_RESULT.md)
is isolated and query-gated. One Chrome Worker served two checked commits on a
40,642-vertex sphere with exact public Save/Undo/Redo. Million-vertex pacing,
memory, complete GPU words and the ordinary browser path remain unverified.
[MRP11's fresh million-vertex ordinary control](SCULPTGL_MILLION_RELEASE_ORIGIN_MRP11_REPORT.md)
passed 17 independent file/history/shape checks but failed all four browser
responsiveness gates, including a 394.38 ms release acknowledgment. An
immutable-origin validation cache was therefore not promoted; held-frame
latency and the Source display-normal oracle remain open.

**Resolution-stroke R02:** [The paired browser report](SCULPTGL_RESOLUTION_STROKE_R02_RESULT.md)
preserves a failed pinned-reference press/release gate, then measures one
held 12-CSS-pixel Draw move at resolution 0.5/1/2× and DPR 1/2. Within
each app, all six tiny strokes changed the same single position word with
exact post-stroke geometry and one history step. This sparse result did not
justify a product change; the mirrored reference ratio used its GUI
controller and the harness needed manual cleanup after writing receipts.
Broad stroke and full resolution-family parity remain open.

**Symmetry-offset C255:** [The held-browser pair](SCULPTGL_SYMMETRY_HELD_C255_DIAGNOSIS.md)
reached a mirrored edit in both apps after a recorded camera calibration,
but the OBrush brush footprint was about four times wider on screen and its
measured image region changed 2,340 pixels versus 34 in pinned SculptGL.
Five earlier failed setup receipts remain retained. This does not establish
offset, brush, camera or mirror-guide parity.
[The matched-camera C256 trace](SCULPTGL_SYMMETRY_RADIUS_C256_REPORT.md)
located this fixture's fourfold footprint difference in synthetic mouse
pressure handling: both browser events reported zero, but pinned SculptGL
used a 0.25 radius factor while OBrush normalized contact to 0.5 and used
factor 1. Geometry and pixels still differed, so no brush correction or
parity claim follows.
[C257's matched-pressure pen attempt](SCULPTGL_SYMMETRY_PRESSURE_C257_REPORT.md)
reached full radius in pinned SculptGL but made no reference vertex edit,
so its frozen paired-deformation gate failed before an OBrush comparison.
[C258's original-call trace](SCULPTGL_SYMMETRY_INPUT_C258_REPORT.md)
found why: the pinned pen route restarts at its first held move, leaving
less than the 37.5 CSS-pixel dab spacing; a matched-pressure mouse route
admitted a mirrored reference dab. OBrush has not yet been paired to that
mouse route, and symmetry parity remains open.
[C259's matched mouse-pressure pair](SCULPTGL_SYMMETRY_MOUSE_PRESSURE_C259_REPORT.md)
then admitted one held Draw dab in both normal browsers on an independent
three-vertex fixture with SymOffset +0.5. Its maximum active-position
component difference was 5.48e-5 local units, inside the frozen 0.005
candidate bound, and both had a mirrored edit. This narrow geometry result
does not score cross-app appearance or close the full symmetry/Draw rows.
[C260's signed-offset curved-mesh gate](SCULPTGL_SYMMETRY_CURVED_C260_REPORT.md)
failed its frozen direct-vertex-ID comparison in both signs. The baseline
coordinates themselves matched as a set, but SculptGL's import optimization
renumbered nearly all vertices; a posthoc coordinate remap showed close
geometry and is recorded only as a diagnosis. C261 freezes that mapping
before a new stroke; C260 remains failed.
[C261's separately frozen baseline-coordinate pair](SCULPTGL_SYMMETRY_CURVED_C261_REPORT.md)
passed scoped Draw geometry gates on the curved fixture for SymOffset
+0.25 and -0.25: changed-vertex IoU was 0.9875/0.9886 and maximum
position-delta errors were 0.000665/0.000883 local units. Normal browser
history passed in both apps; appearance, other meshes/settings and the full
rows remain open.

**Dense resolution-stroke R03:** [The paired browser report](SCULPTGL_RESOLUTION_DENSE_R03_RESULT.md)
passed the frozen 24-changed-vertex minimum on an independent 18,242-vertex
sphere at DPR 1/2 and render resolution 0.5/1/2×. Each app retained exact
within-app post-stroke geometry and one history step across resolutions; OBrush
added 850 adaptive vertices per stroke. The mirrored reference ratio used a
GUI controller, and overlapping OBrush scene objects make screenshot quality
uninterpretable. The full row remains open.

**Single-mesh resolution images V01/V02:** [V01](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_RESULT.md)
failed Source normal-UI Symmetry setup. [V02](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_RESULT.md)
used a recorded non-visible controller step and confirmed one-mesh,
≥24-vertex, history and within-app geometry invariance at DPR 1/2 and
0.5/1/2×. Its frozen pixel segmentation failed because the Source Scene
dropdown and floor grid contaminated the images. No image-quality verdict
or full resolution-family parity follows; [V03](SCULPTGL_RESOLUTION_SINGLE_MESH_V03_PREREG.md)
is a separate frozen UI/mask follow-up. [V03's normal-browser result](SCULPTGL_RESOLUTION_SINGLE_MESH_V03_RESULT.md)
also failed: a fading Source dropdown obscured DPR-2 before images and
OBRush's floor grid corrupted sphere silhouettes. Its geometry/history
remained invariant; [V04](SCULPTGL_RESOLUTION_SINGLE_MESH_V04_PREREG.md)
freezes visible guide-off and all-menu-hidden capture setup before another
pixel check.
[V04's paired browser gate](SCULPTGL_RESOLUTION_SINGLE_MESH_V04_RESULT.md)
passed at DPR 1/2 with the grid off, one imported sphere, unchanged
within-app geometry/history across 0.5/1/2× and full-scene color and
silhouette bounds. Source Symmetry and render ratio still used non-visible
controller setup; longer strokes, other scenes, cross-app image similarity
and the complete resolution-family row remain open.

**Current scoped additions, full rows open:** [C334](SCULPTGL_CAMERA_PICKED_PIVOT_C334_STATIC_RESULT.md)
adds picked orbit pivots in Source framing. Authored-mesh normal-browser checks
in OBrush and pinned SculptGL matched independent integer-pixel ray hits within
`1.27e-6` world units and kept the view fixed on press; the paired final camera
pose, rapid input, DPR-2, public OBJ and browser-backed native package gates
remain open.
[T01/T02](SCULPTGL_DYNAMIC_CONVERSION_T02_UI_RESULT.md) add an explicit,
undoable conversion from retained/UV geometry to adaptive triangles with a
visible loss acknowledgement. The shared-core fixture, full workspace tests,
and the frozen integrated normal-browser gate passed, including rendered loss
counts, repeat-conversion guard, File Open/Save/Undo/Redo and reload. Pinned
SculptGL conversion and deactivation remain unpaired.
[C338](SCULPTGL_SKETCHFAB_PLY_C338_RESULT.md) adds an offline, all-part PLY
export with SculptGL's Sketchfab axis orientation. An ordinary OBrush browser
gate passed exact XYZ/RGB/face bytes and unchanged project history; pinned
payload comparison, account upload and the full File workflow remain open.

**Pinch and Twist remain open:** [C329's Pinch pair](SCULPTGL_SOURCE_PINCH_C329_RESULT.md)
matched both Negative settings within `7.45e-9` XYZ and `8.73e-11` displayed
normal difference on 45 changed vertices, with equal admitted dab counts. Its
frozen pick-center calibration gate still failed by `2.98e-8` against a `1e-8`
bound. The earlier [C328](SCULPTGL_SOURCE_PINCH_C328_RESULT.md) had mismatched
dab counts and a less precise calibration. [C330's Twist pair](SCULPTGL_SOURCE_TWIST_C330_NORMAL_RESULT.md)
matches first-held positions and active display normals within `1.5e-8` and
`8.8e-11` on the independent fixture. A separate [C333 guard-safe two-move
path](SCULPTGL_SOURCE_TWIST_C333_SAFE_PATH_RESULT.md) passed its full scoped
browser gate, including positions, display normals, mask-normalized angles,
GPU, Save and history. The earlier folded C318 path still differs because
the reference folds faces that OBrush's guard protects. Neither full brush row
is closed.

**Scoped Source Paint pen input:** [C310b](SCULPTGL_SOURCE_PAINT_PEN_BURST_C310B_RESULT.md)
passes 13 frozen normal-browser checks for two ordered same-frame pressure
moves on an independent sphere, including useful color differences, factor-zero
controls, GPU/save correspondence, exact Undo/Redo and atomic overflow rollback.
The first [C310](SCULPTGL_SOURCE_PAINT_PEN_BURST_C310_PREREG.md) browser audit
failed because its harness toggled persistent material-channel settings and
used an internal SculptGL undo count as a user-visible step count; that receipt
is retained. Physical stylus delivery, native pressure, other Paint settings
and full reference color parity remain open.

**Default Smooth remains open:** [C309](SCULPTGL_DEFAULT_SMOOTH_C309_RESULT.md)
matched the pinned SculptGL positions and active GPU normals exactly for the
ten vertices touched in its normal-browser off/on comparison, but its frozen
gate failed because the authored raised control lay outside the brush
footprint. That fixture failure is retained. The new independent in-footprint
[C311 pair](SCULPTGL_DEFAULT_SMOOTH_C311_RESULT.md) passed all frozen scoped
checks at default 50 px/75% with Relax-only off/on: affected positions and
active GPU display normals matched exactly through held, Undo and Redo, and
the raised and mask controls were exercised. Other settings, meshes and long
sessions remain open.

**Scoped high-DPI browser input:** [C326](SCULPTGL_DPR_INPUT_C326_RESULT.md)
passed a frozen DPR 1/2 normal-browser gate on an independent fixture. Canvas
backing, scene target, Save and resolution round-trip, visible brush ring, and
one mouse and pressure-0.5 pen Draw contact stayed aligned across both DPRs;
the official main-branch web build has the same WASM hash. Earlier
[C307](SCULPTGL_RESOLUTION_DPR_C307_RESULT.md) falsified a scene-only change,
and its failed receipt remains. Other browsers, native input, multiple strokes
and full resolution-family parity remain open.

**Scoped Source UV appearance correction:** [C299](SCULPTGL_UV_TINT_C299_RESULT.md)
removes unintended default-clay multiplication from imported UV images while
retaining the explicit vertex-color toggle. In the same normal OBrush browser
fixture, the four quadrant colors moved substantially closer to pinned
SculptGL; the largest channel difference at representative interior samples
fell from 162 to 7. The cameras were not calibrated, so exact reference
rendering parity remains open.

**Scoped project image durability:** [C298](SCULPTGL_PROJECT_ASSETS_C298_RESULT.md)
stores bounded custom MatCap and UV images in versioned `.obrush` JSON with
undoable import, reset and project replacement. A normal browser Save/Open
gate reopened authored image fixtures with pixel-identical viewport crops;
native GUI Save/Open and full SculptGL/ZBrush project parity remain open.

**Scoped UV JPEG import:** [C294](SCULPTGL_UV_JPEG_C294_RESULT.md)
extended the visible UV image chooser from PNG to bounded JPEG
with a shared native/browser Rust decoder. An authored two-face UV OBJ and
asymmetric JPEG loaded in OBrush's normal browser with correct quadrant order,
GPU texture allocation and malformed-image retention. [C297's normal-UI
reference run](SCULPTGL_UV_REFERENCE_C297_RESULT.md) confirmed the same
quadrant order after reselecting UV shader following OBJ import. C299 improved
the Source color response; calibrated matched pixels and the complete UV
import row remain open. C298 separately verifies OBrush project persistence
for authored UV images.

**Scoped custom MatCap import:** [C291](SCULPTGL_MATCAP_IMPORT_C291_RESULT.md)
added bounded PNG/JPEG MatCap import to the ordinary viewport.
The pinned SculptGL and OBrush normal UIs both appended and selected the same
authored asymmetric image with matching quadrant orientation; OBrush also
passed built-in/custom reselect and malformed-file retention. Reference
reselect, exact reference appearance/GPU texels, resize, Source
objects and the rest of the file matrix remain open.

**Scoped Brush Lock:** [The C273 pinned normal-UI trial](SCULPTGL_BRUSH_LOCK_UI_C273.md)
found that Source mode rejected Lock Position. An isolated route through the
older `LockedStamp` changed 87 vertices where the reference changed 26 and
was rejected. [The shared-core design note](SCULPTGL_BRUSH_LOCK_KERNEL_DESIGN.md)
records the reference state transitions used by the new Source locked stroke.
The new shared-core Source Draw Lock route passes [one C276 integer-coordinate
normal-UI pair](SCULPTGL_BRUSH_LOCK_C276.md): both apps change the same 26
vertices, with maximum final position difference `5.19e-13` and bit-identical
final active GPU display normals. [C274's fractional-coordinate pair](SCULPTGL_BRUSH_LOCK_C274_FAILED.md)
failed its frozen equal-radius gate by `3.05e-5` CSS pixels; [C275](SCULPTGL_BRUSH_LOCK_C275_NO_GO.md)
located the precision loss in trusted browser pointer events before OBrush
receives them. The full Brush Lock parity gate remains open.

**Scoped Brush Negative witness:** [The C272 pinned normal-UI pair](SCULPTGL_BRUSH_NEGATIVE_UI_C272.md)
matches the reference's visible Negative-off and Negative-on Brush gestures
on the independent 10,114-vertex sphere. Both polarities change the same 87
IDs within `2.98e-8` per position component, restore exact public Undo/Redo,
and pass active GPU position and paired display-normal checks. The single
unmasked gesture does not close the complete Negative row.

**Scoped Brush Clay witness:** [The C278 four-run normal-UI pair](SCULPTGL_BRUSH_CLAY_C278.md)
matches Clay on/off and a repeated held gesture on the independent sphere:
each side changes the same 98 IDs, with maximum paired final position error
`5.96e-8` and matching held GPU position/display-normal checks. The earlier
C277 test interrupted its own held stroke by clicking Save and remains a
preserved failed receipt. The complete Clay row stays open.

**Scoped Smooth Tangent witness:** [The C285 normal-UI on/off pair](SCULPTGL_SMOOTH_TANGENT_C285_RESULT.md)
matches pinned SculptGL geometry and active GPU display-normal words exactly
through Before, held preview, After, Undo and Redo on one independent curved
441-vertex fixture. The corrected run preserves [C283's setup failure](SCULPTGL_SMOOTH_TANGENT_C283_RESULT.md)
separately. The full Smooth Tangent row stays open.

**Camera transition slice:** [C287](SCULPTGL_CAMERA_TRANSITION_C287_REPORT.md)
adds a 200 ms quartic animation for explicit Fit/axis/Snap/Reset actions.
An authored pose-sampling fixture and normal-browser RAF trace verify
intermediate motion and wheel interruption; full picked-focus, shortcut,
camera-matrix and timer-cadence parity remain open.

**Source Drag trial:** [C286/C288](SCULPTGL_SOURCE_DRAG_C288_RESULT.md)
tested an opt-in shared-core moving-center Drag on an independently authored
curved grid. The second normal-UI pair changed the same 95 vertices and
passed own-file/history/GPU position checks, but exceeded its frozen `1e-6`
paired position gate by `2.06e-6` and its Redo display-normal gate by
`1.62e-4`. The candidate code was reverted and the full Drag row remains open.
[A read-only replay](SCULPTGL_SOURCE_DRAG_C288_CAUSAL.md) attributes the
position difference to captured camera/pick/radius inputs within f32 rounding;
[C289](SCULPTGL_SOURCE_DRAG_C289_PREREG.md) freezes a separate causal repair
gate for those inputs and Redo display normals.
[Its read-only bounds check](SCULPTGL_SOURCE_DRAG_C289_BOUNDS_DIAGNOSTIC.md)
found identical mesh bounds but a Source center drift during overlapping reset
timers; without the pre-fit near/zoom state, a camera correction was not
qualified and Drag remains open.
[C290's read-only normal-browser capture](SCULPTGL_SOURCE_DRAG_C290_RESULT.md)
then confirmed the exact reference fit branch and nine delayed center updates,
but the paired final-pick gate failed because OBrush's new camera transition
had not settled. No Drag product correction was admitted.
[C293's settled-camera preflight](SCULPTGL_SOURCE_DRAG_C293_PREFLIGHT_RESULT.md)
passes paired projection, all four rays, radius, independent picks and
read-only Save/GPU/history checks. Its first candidate browser run stopped at
an [unrelated harness setup error](SCULPTGL_SOURCE_DRAG_C293_CANDIDATE_SETUP_FAILURE.md).
[The separately frozen C295 normal-UI pair](SCULPTGL_SOURCE_DRAG_C295_RESULT.md)
then passed held geometry, GPU display normals, public Save/Undo/Redo and
selected-union Redo-normal checks on that fixture (maximum paired XYZ
`4.80e-8`). The opt-in Source Drag route is integrated; other settings,
topology and sustained performance leave the full row open.
[The C305 mirror and mask pair](SCULPTGL_SOURCE_DRAG_C305_RESULT.md) also
passes normal-browser X-symmetry off/on geometry, active GPU normals,
public history and a masked-vertex matrix on the curved-grid fixture. Both
apps changed the same 95 IDs with symmetry off and 136 on; maximum paired
position-component error was `4.84e-8`. An independent replay of C295 passed.
The earlier C300/C301/C303 and C304 setup failures remain recorded; Alpha,
pressure, alternative cameras, dynamic topology and long sessions keep the
full Drag gate open.

**Scoped Inflate witness:** [The C269 direct pinned-method report](SCULPTGL_INFLATE_PAIR_C269.md)
records the reference's positive, negative and zero Float32 response on one
independent masked diamond, alongside a passing shared-core arithmetic,
paired-word assertion and public-history test. [The C270 normal-UI diamond
pair](SCULPTGL_INFLATE_DIAMOND_UI_C270.md) matches positive and negative
strokes on the mapped vertices within `3.73e-9`, with public Undo/Redo and
OBRush GPU row checks. The wider Inflate and full parity gates stay open.

**Current million-vertex browser baseline:** [The C264 frozen Source Brush run](SCULPTGL_MILLION_CURRENT_C264.md)
passes independent complete Save/Undo/Redo and affected-face shape checks, but
fails all four responsiveness gates (RAF P95 66.7 ms, release 159.0 ms).
[The fresh C292 run](SCULPTGL_MILLION_FRESH_PROFILE_C292_RESULT.md) on the
integrated camera build repeats all 17 file/history/shape checks successfully
while still failing all four responsiveness gates (RAF P95 66.7 ms, release
134.7 ms). Its overlapping geometry and viewport timings do not yet admit a
narrow performance edit.
[The C271 exact-row normal-witness trial](SCULPTGL_MILLION_NORMAL_WITNESS_C271.md)
reduced one publication substage but still failed all four gates on a fresh
million-vertex run; its product code was rejected and the failed receipt kept.
[The MRP01 contiguous attribute-scan trial](SCULPTGL_MILLION_ATTRIBUTE_SCAN_TRIAL.md)
also passed exact public-file and shape checks but worsened the ordinary
browser upload callback from 10.328 to 13.242 ms and failed all four
responsiveness gates. Its renderer code was rejected and its raw receipt kept.
[The MRP02 certified-hover trial](SCULPTGL_MILLION_CERTIFIED_HOVER_MRP02_RESULT.md)
found an exact duplicate cursor pick costing 22.9 ms per active viewport
callback. Its candidate reduced RAF P95 from 66.7 to 33.4 ms and app CPU P95
from 62.4 to 35.1 ms on the frozen million-vertex browser test, but still
failed all four global responsiveness gates. The candidate code was rejected,
the 17/17 file/history/shape audit passed, and an independent active-GPU
word oracle remains open.
[The read-only MRP03 million-vertex trace](SCULPTGL_MILLION_MRP03_RESULT.md)
finds 22.41 ms mean cursor/overlay and 21.68 ms mean Source geometry in
the same steady-stroke viewport frames, plus 520.6 ms initial-press input
and a 367.8 ms early Source dab. Its matched control and diagnostic both
pass complete file/history/shape checks and fail all four responsiveness
gates; no product optimization or reference-parity claim follows.
[The read-only MRP04 cold-path split](SCULPTGL_MILLION_COLD_HOT_MRP04_RESULT.md)
confirms one ~295 ms press-alpha normal-state seed and one ~222 ms first-dab
seed on the pinned million-vertex fixture, dominated by ordered-tree builds.
Its matched normal-browser cohorts pass independent 17/17 file/history/shape
audits but still fail all four responsiveness gates. A seed-reuse
microbenchmark was admitted, but [MRP05](SCULPTGL_MILLION_SEED_REUSE_MRP05_RESULT.md)
failed its frozen browser peak-memory rule despite exact state words and
187/254 ms median native/WASM first-dab timing gains. The separate
[MRP06 memory repeat](SCULPTGL_MILLION_MEMORY_MRP06_RESULT.md) also
failed its frozen individual peak-RSS cap despite identical state and WASM
heap words; no Arc-reuse product candidate or parity claim follows.
[MRP07's isolated sparse-normal probe](SCULPTGL_MILLION_SPARSE_PRESS_MRP07_RESULT.md)
matches scoped alpha-track words and saves 173/297 ms median native/WASM
press time without additional WASM linear-memory pages. Exact allocation and
malformed-state rejection parity remains unproved, so its code has not been
merged into the ordinary press path and the four responsiveness gates remain
failed.
[MRP08's isolated face-bounds candidate](SCULPTGL_MILLION_FACE_BOUNDS_MRP08_RESULT.md)
matched all 1,998,000 public face-bound words but saved only 0.392 ms median
in the bounded native screen, far below its 20 ms admission target; it was
not integrated or promoted to the browser gate.
[MRP09's cell-reserve probe](SCULPTGL_MILLION_CELL_CAPACITY_MRP09_RESULT.md)
found only 3.531 ms median inside 94,280 ordered-tree reserve calls, so its
frozen early stop rejected a capacity-growth product change.
[The sparse held-pointer Worker trial](SOURCE_WORKER_SPARSE_HELD_TRIAL.md) checks two
ordinary viewport strokes, warm state, sparse GPU words, public Save/Undo/Redo
and cancellation on a 40k sphere. It remains query-gated and unpromoted:
[the release-endpoint follow-up](SOURCE_WORKER_RELEASE_ENDPOINT_TRIAL.md)
now delivers the final pointer sample on two scoped Chrome strokes, but its
release took `292.5/305.6 ms` and the 100k/million-vertex sustained gate is open.
[The SPW01 sustained diagnostic](SOURCE_WORKER_SPW01_100K_DIAGNOSTIC.md)
passes the 100k ordinary shape/history check but records a Worker admission
failure on the frozen imported Source model: its metadata is unsupported by
the original sparse preview renderer. [The metadata-aware follow-up](SOURCE_WORKER_METADATA_PREVIEW_RESULT.md)
admits a scoped identity-affine shader-0 Source object behind the trial switch:
its 100k run applied 1,287 dabs with exact public history and GPU positions,
but failed worst-case pacing, queue-tail and all-state GPU-normal gates. The
1m Worker run remains unmeasured.
[The one-pass Worker publication trial](SOURCE_WORKER_TAIL_RESULT.md) reduced
the measured 100k main-thread finish callback from 931.5 to 565.9 ms with
passing public Save/Undo/Redo, shape and GPU-position checks. The frozen
queue-tail, RAF-max, pacing-max and edited display-normal gates still fail;
the held Worker remains query-gated and off by default.
[The read-only publication profile](SOURCE_WORKER_PUBLICATION_STAGES_RESULT.md)
attributes 373.6 ms to complete-scene witness/delta decode and 171.0 ms to
retained warm decode in a separate 100k run. It leaves the product algorithm
unchanged and retains the frozen queue-tail, RAF-max and GPU-normal failures.
[The streamed scene-witness trial](SOURCE_WORKER_SCENE_WITNESS_RESULT.md)
then reduced that measured scene/delta stage to 283.7 ms on a fresh 100k run
while preserving exact public history and GPU positions. The frozen RAF-max,
100 ms queue-tail and GPU-normal gates still fail, so the Worker stays off by
default.
[The retained warm-decode trial](SOURCE_WORKER_WARM_DECODE_TRIAL.md) reached
155.6 ms against a preregistered `<120 ms` browser target and failed its GPU
normal check. Its product and diagnostic code were rejected; the raw failure
receipt remains available.
[The verified-baseline split](SOURCE_WORKER_VERIFIED_BASELINE_RESULT.md)
measured 267.9 ms in complete-scene witness work and only 13.4 ms in delta
decoding on a fresh frozen 100k run. Delta decoding missed the preregistered
15 ms threshold, so no cached-baseline decoder was built. The held Worker
continues to fail the frozen RAF-max, 100 ms queue-tail and public-normal
checks and remains off by default.
[The validation-once retained-decoder trial](SOURCE_WORKER_VALIDATED_DECODE_PREREG.md)
passed full build checks but measured 149.2 ms retained decode against its
fixed `<120 ms` target; active RAF maximum was 450.1 ms against 250 ms. Its
product code was rejected and the failed browser receipt retained. A
[read-only finish-phase profile](SOURCE_WORKER_FINISH_PACKET_PROFILE_RESULT.md)
measured 164.2 ms in retained-state encoding within a 194.3 ms Worker
finish interval on a fresh frozen 100k run. Buffer takes took 1.5 ms;
packet-size/transfer causality remains unproved, and the 466.6 ms active
RAF maximum and 100 ms queue-tail gates still fail. A
[read-only release timeline](SOURCE_WORKER_RELEASE_TIMELINE_RESULT.md)
observed the Worker still active with finish pending at release+102.4 ms;
its checked main commit occupied 475.3 ms and overlapped a 481.5 ms app-frame
gap. A narrow encode/decoder change cannot satisfy both current gates.
[The read-only warm-encoder profile](SOURCE_WORKER_WARM_ENCODE_SUBPHASE_RESULT.md)
measured 99.0 ms in semantic fingerprinting, including 55.9 ms in its tree
walk, during a fresh 100k finish. It clears the preregistered investigation
threshold but has no same-state prediction of a 40 ms net fusion gain; no
codec code was promoted and the frozen RAF, queue and GPU-normal checks fail. A
[same-state native/WASM encoder trial](SOURCE_WORKER_WARM_ENCODE_FUSION_RESULT.md)
then preserved exact v1 packet bytes and two future dabs, but saved only
1.85 ms median in nine paired browser Worker encodes against the frozen 40 ms
gate (7/9 positive against 8/9). Its trial code was reverted. A
[typed complete-scene witness design](SOURCE_WORKER_TYPED_SCENE_WITNESS_DESIGN.md)
sets a proof gate for the larger witness stage. [The read-only phase
profile](SOURCE_WORKER_TYPED_SCENE_WITNESS_GATE_RESULT.md) measured 251.1 ms
in JSON serialization and hashing within a 265.9 ms witness, clearing its
preregistered 150 ms investigation threshold. [The typed-witness
trial](SOURCE_WORKER_TYPED_SCENE_WITNESS_RESULT.md) then took 305.3 ms in
the witness against a 200 ms target and 1000.3 ms at begin against 458 ms.
Its code was rejected; the frozen RAF, queue-tail and normal checks still fail.
[The independent display-normal audit](SOURCE_GPU_DISPLAY_NORMAL_ORACLE_PREREG.md)
matches fresh Source face normals to active GPU rows before sculpting and after
Undo. Edited After and Redo states have byte-identical public meshes but
1,147 differing GPU normal rows in an owned 100k browser run; the frozen
saved-normal GPU gate remains failed and no edited-state oracle is canonical.
[The warm retained-state codec](SOURCE_WORKER_RETAINED_WARM_SEAM.md) preserves
later-dab geometry across a simulated realm transfer, and [the Chrome Worker
diagnostic](SOURCE_WORKER_CROSS_REALM_DIAGNOSTIC.md) checks a small cold stroke
and atomic publication. Neither activates the ordinary browser brush Worker.
[The held-pointer Worker diagnostic](SOURCE_WORKER_HELD_TRIAL.md) reaches a
second warm stroke in Chrome, but is unpromoted: its 40k-vertex first previews
took 521/1,045 ms, release dropped path endpoints, and full-mesh per-dab work
remains. Its Save/Redo check does not close Undo-to-first-stroke or GPU gates.
[The normal-scratch trial](SCULPTGL_NORMAL_SCRATCH_TRIAL_C264.md) remains
unpromoted after all responsiveness gates failed and its GPU-normal check used
an invalid public-normal oracle. [The C265](SCULPTGL_RENDER_ROW_RANGES_C265.md)
and [C266](SCULPTGL_RENDER_ROW_RANGES_C266.md) renderer trials also failed to
establish a responsiveness improvement; their code was rejected.

**C263 integrated checkpoint:** [The combined Source/browser/macOS receipt](SCULPTGL_COMBINED_C263.md)
records passing workspace tests, strict native/WASM Clippy, fresh local builds,
Source Move and viewport Shift-click fixes, scoped Continuous/culling evidence,
and a measured 100k renderer diagnostic. Ordinary Worker sculpting,
million-vertex responsiveness, and every full SculptGL/ZBrush gate remain open.

**Scoped Source Brush culling:** [The C262 fixture and normal-browser report](SCULPTGL_BRUSH_CULLING_C262.md)
confirms the existing Source Brush/Clay checkbox and inclusive front-vertex
predicate on an opposed-sheet/zero-normal fixture, with exact core Undo/Redo.
[The C280 pinned opposed-sheet pair](SCULPTGL_BRUSH_CULLING_C280_RESULT.md)
confirmed back-sheet protection with Culling on, but failed its frozen paired
front-geometry gate by `3.12e-6` at a second held move on a shared triangle
edge. [C281](SCULPTGL_BRUSH_CULLING_C281_RESULT.md) directly observed a
different adjacent-face pick on a fresh run, and [C282's interior-face
pair](SCULPTGL_BRUSH_CULLING_C282_RESULT.md) matched on/off geometry and
history within `1.12e-8` but failed its frozen press/early-held pick-report
availability gate. The full Culling row remains open.

**Seed-only Source Worker state seam:** [The retained seed witness](SOURCE_WORKER_RETAINED_SEED_SEAM.md)
checks a fixed 112-byte cross-realm packet and exact later-dab reconstruction
for a generation-zero Source normal/tree state. It refuses warm retained state
and does not publish a Worker brush or transfer a Document and its history;
ordinary browser responsiveness and all parity gates remain open.

**C260 integrated build:** [The combined Source/browser/macOS receipt](SCULPTGL_COMBINED_C260.md)
records 2,582 passing workspace tests, strict native/WASM Clippy, fresh
signed macOS and browser builds, and scoped browser checks for default
primitive levels and placement, Source Paint All, full Sphere shape,
Cylinder/Torus posed bounds and the mirror guide. The sparse Worker codec is
non-publishing; sustained brush responsiveness and every full SculptGL and
ZBrush acceptance gate remain open.

**Scoped Source Paint All:** [The method-level and normal-browser report](SCULPTGL_SOURCE_PAINT_ALL.md)
checks soft-mask RGB/material channel blending, a reachable Source-mode Fill
control, unchanged geometry/inactive part, public Save and exact core
Undo/Redo on an independent fixture. The pinned SculptGL method matches the
analytic fixture, but a paired reference-browser UI run and broader mesh,
default material and native cases remain open; the active gate is not closed.

**C259 Source primitive placement:** [The paired normal-browser check](SCULPTGL_PRIMITIVE_PLACEMENT_C259.md)
matches first-Add world bounds for Sphere/Cube/Cylinder/Torus within
`9e-7`, retains four centered parts, undoes one Add and round-trips the posed
scene through public Save/Open byte-exactly. Camera pixels, picked brush
response, edited levels, native GUI and sustained performance remain open.

**C259 Source Brush radius audit:** [The pinned normal-browser pair](SCULPTGL_BRUSH_RADIUS_SCREEN_C259.md)
confirms that current Source ordinary Brush already converts a 5–500
viewport-point radius at the picked depth; the frozen 0.1.14 world-radius
gap is historical. One aligned 64-pixel pair changes the same 87 of 10,114
vertices in both apps, with exact public Undo/Redo and a maximum paired
After component difference of `2.98e-8`. Zoom, DPR, render resolution,
pressure, displayed footprint and other Source tools keep the full gate open.

**C258 Source primitive levels:** [The paired browser and core receipt](SCULPTGL_PRIMITIVE_RETAINED_LEVELS_C258.md)
records default Sphere/Cube/Cylinder/Torus retained-level counts matching the
pinned SculptGL runtime at `4/4/3/3`, with a dense Cube
Add→Undo→Redo→Save→Clear→Open→Lower→Higher→Save round trip. The
[cylinder normal diagnostic](SCULPTGL_CYLINDER_NORMAL_GPU_PAIR.md) narrows the
previous `6.043°` difference to serialized mesh-normal cache: one sampled
active viewport normal differs from the reference by `0.0000153°`. Edited
coarse-to-fine response, world placement, full displayed-normal and pixel
comparisons, native GUI and sustained brush responsiveness remain open.

**C257 combined Source primitives:** [The integrated browser and macOS receipt](SCULPTGL_COMBINED_C257.md)
records passing workspace tests, strict native/WASM Clippy, fresh web/macOS
builds, all four pinned default primitive counts in the normal browser,
the sphere's full normalized vertex/normal comparison, dense-cube project
round trip and the imported Source mirror-guide check. Retained Source
levels, exact scene placement, cylinder/torus shape, mirrored sculpt response
and million-vertex sustained brush responsiveness remain open.

**C252 combined local build:** [The integrated browser and macOS receipt](SCULPTGL_COMBINED_C252.md)
records passing workspace tests, strict native/WASM Clippy, fresh web/macOS
builds, paired primitive/clear browser controls, and an opt-in WebGPU sparse
release check after merging C249–C251 and the per-object Source mirror work.
The default SculptGL-profile cube now matches pinned counts for vertices,
original quads and render triangles. The guide follows the active Source
object's normalized offset in an OBrush browser pair. Shape/normal appearance,
retained levels, native GUI, and the million-vertex responsiveness gates remain
open; the 309 active SculptGL inventory gates are not complete.

**C247 viewport resolution:** [The pinned-source and paired browser report](SCULPTGL_RESOLUTION_C247.md)
adds a 0.5–2× Source resolution slider with bounded scene rendering. Two
independent fixtures show measured pixel-target changes, exact unchanged
OBRush project saves/history and no WebGL errors at 0.5/1/2×. SculptGL scales
its whole canvas backing while OBrush scales the scene target; the later C326
DPR/input fixture passes, while native GUI and full resolution-family parity
remain open.

**C245 same-frame browser pen samples:** [The normal-browser diagnostic](SCULPTGL_PEN_BURST_C245.md)
shows that two trusted held pen moves arriving before one app frame previously
lost the first move's pressure and position. A bounded ordered browser queue
now retains both for an active ordinary brush; distinct pressure traces
produce distinct saved Draw meshes with exact GPU/save and Undo/Redo checks.
Physical pens, native stylus input, transformed layouts, overflow behavior in
the browser, matched SculptGL brush shape and full parity remain open.

**C302 Source Brush pen burst:** [The pinned normal-browser pair](SCULPTGL_SOURCE_PEN_BURST_C302_RESULT.md)
retains two same-frame Source Brush pen moves with separate pressure values.
All 12 scoped audit checks pass: both dabs are admitted without a release
duplicate, pressure changes the saved geometry in each app, factor-zero
controls remain exact, OBrush GPU rows and Undo/Redo match its public files,
and a 257-move overflow cancels with exact prior project bytes and history.
The geometry is not an exact cross-app match. Synthetic browser input does not
establish physical-tablet, native, other-brush, or full pressure parity.

**C246 global darken-unselected audit:** [The pinned-source and paired browser
report](SCULPTGL_DARKEN_C246.md) confirms the existing OBrush Source shader
selects the same unselected-part `0.3` linear-color factor on one independent
two-part SGL fixture. Both controls leave mesh opacity/shader and history
unchanged; OBrush's saved project bytes are exact before/after. The reference
controller was invoked directly because its mirrored checkbox had no usable
hit target, and the fixture was too small in OBrush for a matched visual test.
The full active acceptance gate remains open.

**C243 selected-part transparency:** [The pinned-source and paired browser report](SCULPTGL_TRANSPARENCY_C243.md)
maps the Rendering percentage to selected Source-part opacity. An independent
overlapping SGL pair saves exact 0/50/100% values in both applications and
uses the expected opaque/transparent WebGL states, front/back faces and
wireframe draws. OBrush's Source appearance remains undoable; viewport pixel
matching, broader scene/depth cases and the full acceptance gate remain open.

**C244 selected-part flat shading:** [The pinned-source and paired browser
report](SCULPTGL_FLAT_C244.md) confirms the Source checkbox now applies one
flat-shading value to selected parts in one checked shared-core edit. A
two-part SGL fixture shows the selected triangle's flat shader flag toggling
in both normal browsers. Analytic appearance, reference multi-selection UI,
history semantics and the full active acceptance gate remain open.

**C242 selected-part wireframe:** [The pinned-source and browser report](SCULPTGL_WIREFRAME_C242.md)
tracks the Rendering checkbox and W on imported quad/triangle parts. A
shared-core selected-part Source edit now keeps one OBrush undo step and
public save/export flags; the exact static/dynamic/multires, transparent and
native appearance matrix remains open.

**C241 selected mesh counters:** [The paired browser and shared-core report](SCULPTGL_MESH_INFO_C241.md)
adds top-bar vertex/face counts and a quad-aware face count in the inspector.
An independent mixed OBJ shows 6 vertices, 2 original faces and 3 render
triangles in both browsers. OBrush's higher/lower level and empty-scene counts
also track the editable mesh; broader reference sequences remain open.

**C238 corrected pen admission:** [The multi-dab comparison](SCULPTGL_PRESSURE_MULTIDAB_ADMISSION_C238.md)
matches three geometry-bearing moves per browser on the same 10,114-vertex
fixture. Both pinned SculptGL and OBrush show roughly `1e-4` deviation from
an exact half displacement after multiple dabs; the prior one-dab versus
multi-dab comparison was invalid. This does not establish raw brush-shape or
physical-pen parity.

**C239 rendering inventory reconciliation:** [The grid/contour evidence audit](SCULPTGL_RENDER_INVENTORY_RECONCILIATION_C239.md)
links three existing rendering reports into the 317-ID crosswalk. Thirty-nine
active rows now have scoped evidence; all active acceptance gates remain open.
This is documentation reconciliation, not a behavior change.

**C237 scene inventory reconciliation:** [The selected-scene evidence audit](SCULPTGL_SCENE_INVENTORY_RECONCILIATION_C237.md)
links seven existing paired scene-action reports into the 317-ID crosswalk.
Thirty-six active rows now have scoped evidence, while every active acceptance
gate remains open. This is documentation reconciliation, not a behavior change.

**C236 admitted pen gesture:** [The pinned-source diagnosis and paired browser report](SCULPTGL_PEN_GESTURE_C236.md)
shows that SculptGL consumes the first held synthetic pen move as Hammer
`panstart`; later `panmove` events produce sculpt changes. On the same imported
10,114-vertex mesh, both apps show the expected radius and intensity factor
direction with exact public Undo/Redo. The later C238 audit corrects unequal
dab admission in this comparison. Final brush shape and physical pen behavior
remain unverified; pressure and full SculptGL/ZBrush parity remain open.

**C234 paired pressure input:** [The common-fixture browser report](SCULPTGL_PRESSURE_PAIR_C234.md)
finds exact imported positions across 10,114 vertices and OBrush Source-mode
response to radius and intensity factors. Pinned SculptGL's synthetic pen
gestures leave saved geometry unchanged even at full pressure, despite trusted
move delivery; its mouse gesture changes geometry. The reference pen input
path was subsequently resolved for synthetic three-move gestures in C236; the
C234 one-move run alone is not pressure geometry parity.

**C233 app-frame profile:** [The complete million-vertex browser report](SCULPTGL_APP_FRAME_PROFILE_C233.md)
records 50.13 ms per update inside the viewport during a frozen long stroke,
versus 1.09 ms before it. Source geometry averages 22.41 ms per dab in that
run; all public-file/history/shape checks pass, but all four responsiveness
gates fail. This diagnoses the main-thread work and changes no parity status.

**C232 held-stroke pressure ramp:** [The four-case normal-browser report](SCULPTGL_PRESSURE_RAMP_BROWSER_C232.md)
shows a trusted synthetic pen move from pressure 0.25 to 0.75 changes Draw
geometry with intensity factor 1, while factor 0 stays exact. The first GPU
dab matches between compared runs, all final GPU buffers match public saves,
and one-stroke Undo/Redo is exact. Physical tablets and paired reference
strokes remain open.

**C231 browser factor response:** [The four-case normal-browser report](SCULPTGL_PRESSURE_FACTOR_BROWSER_C231.md)
verifies visible factor controls, trusted synthetic pen/mouse events, saved
geometry and GPU readback. Radius factor 1 narrows the changed original-vertex
set from 427 to 129; intensity factor 1 halves displacement within `7.0e-8`
maximum position-component error on one fresh sphere. Pressure ramps,
physical tablets and paired SculptGL UI response remain open.

**C230 pressure factors:** [The scoped control and input report](SCULPTGL_PRESSURE_CONTROLS_C230.md)
exposes SculptGL-style 0..1 intensity/radius factors for ordinary OBrush
brushes and resamples them for every pen dab while keeping mouse input neutral.
The full workspace, strict native/WASM Clippy, browser build and macOS package
pass; an isolated UI check shows the controls. Physical pens, separate Source
response and full paired reference behavior remain open.

The [current SculptGL inventory crosswalk](SCULPTGL_CURRENT_LEDGER.md) counts
all 317 pinned source IDs without treating the historical 0.1.14 gap labels
as present implementation status. Its linked row-level evidence is scoped;
the full active acceptance gates remain open.

**C229 retained topology scan:** [The two-run million-vertex report](SCULPTGL_RETAINED_TOPOLOGY_WALK_C229.md)
records a full public-mesh validation with a faster observed face/triangle
walk and exact file/history/shape audits. Retained match mean is 6.47/6.57 ms
versus 7.19/7.35 ms in two separate C228 runs. RAF P95 remains 66.7 ms and
all four responsiveness gates still fail; no parity status changes.

**C228 certified Source render handoff:** [The shared-core and million-vertex browser report](SCULPTGL_CERTIFIED_RENDER_HANDOFF_C228.md)
records 947 exact-pair handoffs, a passing public-file/history/shape audit
and roughly 4.1 ms observed renderer admission versus 8.0 ms in the separate
C225 baseline. Active RAF P95 is 66.7 ms, but all four responsiveness gates
still fail; release latency is variable and parity is not promoted.

**C227 rejected private-state witness trial:** [The million-vertex browser report](SCULPTGL_PRIVATE_WITNESS_TRIAL_C227.md)
records exact file/history/shape checks but no responsiveness gain. The trial
was removed; all four active-recorder gates remain failed.

**C226 paired million-vertex browser cohort:** [The pinned SculptGL comparison](SCULPTGL_MILLION_PAIRED_C226.md)
uses the same 1M/60s fixture and protocol in separate owned browsers. OBrush
observes 83.3 ms RAF P95 and 161.2 ms release; pinned SculptGL observes
16.8 ms and 8.0 ms. Independent public-file/history/shape audits pass both;
different delivered dabs prevent a matched-response or causal speed claim.

**C225 rejected million-vertex attribute-walk trial:** [The two-run report](SCULPTGL_ATTRIBUTE_WALK_TRIAL_C225.md)
records a new C224 baseline with a passing full file/history/shape audit and a
trial with slightly lower upload mean but unchanged 83.3 ms RAF P95 and worse
release acknowledgment. The trial was removed; all four preregistered
active-recorder responsiveness gates fail in the new baseline.

**C224 SculptGL-profile binary STL:** [The paired browser report](SCULPTGL_BINARY_STL_C224.md)
records a shared-Rust writer and zero differing bytes across two imported
object facets in pinned SculptGL versus OBrush normal-browser downloads.
Broader STL interchange, native UI and large-scene responsiveness remain open.

**C223 exact reference PLY return:** [The shared-core and browser report](SCULPTGL_PLY_QUAD_RETURN_C223.md)
retains the 98,304 original quads in a pinned SculptGL PLY and returns all
9,535,926 bytes exactly through OBrush's normal browser File Add/Export path.
The saved project also recovers exactly through Undo/Redo. Broader interchange,
native physical GUI and responsiveness comparisons remain open.

**C222 SculptGL-profile binary PLY:** [The paired browser report](SCULPTGL_BINARY_PLY_C222.md)
records shared-Rust tri/quad, byte-RGB and padding output, matching selected
imported-object geometry and color in the pinned runtime, plus actual reference
PLY return through the browser with exact Save/Undo/Redo. C223 subsequently
retained valid imported quads; broader File-family and performance gates remain
open.

**C221 rejected topology-match trial:** [The million-vertex browser report](SCULPTGL_TOPOLOGY_MATCH_TRIAL_C221.md)
records a slower retained match despite passing geometry/history checks. The
trial was removed and production artifacts returned byte-for-byte to C219.

**C220 rejected attribute-snapshot trial:** [The million-vertex browser report](SCULPTGL_ATTRIBUTE_SNAPSHOT_TRIAL_C220.md) records 788 active shortcut uses but worse measured mesh-upload CPU (17.73 ms versus C219's 11.68 ms). The trial was removed and both rebuilt artifacts match C219 byte-for-byte. Responsiveness and parity remain open.

**C219 Source mask GPU reuse:** [The million-vertex browser report](SCULPTGL_MASK_BUFFER_REUSE_C219.md) verifies 902 exact raw-mask buffer reuses and one timed 3,996,008-byte mask upload versus 902 in the preceding build; the full sculpt-file audit and 17 normal-browser mask workflows pass. RAF P95 remains 66.8 ms and release acknowledgment 129.1 ms, with all four responsiveness gates failed.

**C218 million-vertex viewport bounds:** [The final-source browser report](SCULPTGL_SPARSE_BOUNDS_C218.md) confirms 243 incremental bounds updates under the frozen 1M/60s stroke and exact public-file/history audit. RAF P95 is still 66.8 ms and release acknowledgment 117.6 ms; all four responsiveness gates fail, so no speedup or parity is claimed.

**C217 SculptGL OBJ color options:** [The paired browser and core report](SCULPTGL_OBJ_COLOR_C217.md) adds independent `#MRGB`/`#MAT` and appended-RGB switches to the shared Rust scene exporter. Four normal-browser downloads match the pinned SculptGL runtime's imported-object color/material payloads exactly on one nondefault two-object fixture; broader OBJ and material workflows remain open.

**C216 scoped OBJ/PLY/STL scene export:** [The shared-core and normal-browser report](SCULPTGL_SCENE_GEOMETRY_EXPORT_C216.md) exports selected or all parts through the common Rust document, including hidden selected parts and baked source poses. Five browser downloads retain the exact saved project. Pinned SculptGL file-byte comparison, `#MAT` encoding and large-scene responsiveness remain open.

**C215 Source File Drop:** [The paired browser report](SCULPTGL_FILE_DROP_C215.md) connects OBJ/SGL/PLY/STL window drops to the bounded shared Rust File Add path. A dropped OBJ plus ignored text file is exercised in both normal OBrush and pinned SculptGL browsers; OBrush's exact project Undo passes. Native egui receives supported dropped paths through the same import host, while physical drag, native GUI and broad file semantics remain unchecked.

**C214 multi-file Source Add:** [The paired browser report](SCULPTGL_MULTI_FILE_ADD_C214.md) checks one three-file OBJ/PLY/STL picker selection in both OBrush and pinned SculptGL. OBrush retains each file as a separate, exact Undo/Redo step and continues past one malformed middle file; native uses the same file-request and shared-core import path. Per-file visual/numerical comparison, native GUI execution and full File-family parity remain open.

OBrush aims for full feature parity with the **current desktop edition of ZBrush**. This is the development objective, not the status of the first prototype. A working sculpt viewport, or a brush with a familiar name, does not establish ZBrush parity.

## Reference and evidence

The reference version is provisionally pinned to **ZBrush desktop 2026.2.1**, advertised on Maxon's [current product page](https://www.maxon.net/en/zbrush) and [What's New page](https://www.maxon.net/en/zbrush/new), checked on **2026-10-12**. The user selected current desktop ZBrush; iPad and Core are not substitutes for this baseline. Maxon's detailed [What's New guide](https://help.maxon.net/zbr/en-us/Content/html/features/whats-new-in-zbrush/zbrush-whats-new.html) still heads its version inventory with 2026.1. Treat older help topics as feature descriptions and check their applicability against release notes and the actual 2026.2.1 application when available.

The user subsequently installed ZBrush and reported an active trial. Its local application metadata confirms **2026.2.1**. REA performed static Mach-O/artifact inspection of the installed executable and a local OBrush **0.1.0 release-mode build**; historical, hash-bound records are [ZBrush inspection](evidence/zbrush-native-inspection.json) and [OBrush release inspection](evidence/obrush-native-inspection.json). A bounded search of the ARM64 string section identifies brush/settings labels, without establishing brush mathematics or live behavior. A historical 0.1.0 OBJ import/export exchange through ZBrush is recorded for **one fixture** in [OBJ interchange evidence](evidence/obj-zbrush-roundtrip.json). These records do not establish updated 0.1.1 behavior. Reference sculpt-stroke execution and comparative sculpting quality/performance remain **unverified**. Toolchain and investigation details are in [REA_WORKFLOW.md](REA_WORKFLOW.md).

This is a **first inventory**, not an exhaustive enumeration of every palette command, parameter, brush, bundled plugin, or supported version of an integration. Expand each family into individually testable commands using Maxon's [desktop documentation and reference guide](https://help.maxon.net/zbr/en-us/Content/html/zbrush-online-help.html). Keep legacy desktop workflows in the inventory until their presence in the pinned version is established. Do not publish a parity percentage while the inventory is incomplete.

**C211 Source File Add:** [The browser and shared-core report](SCULPTGL_SOURCE_FILE_ADD_C211.md) routes SculptGL-profile Ctrl+O/I, the File-panel Add picker and non-SGL startup URLs through validated OBJ/PLY/STL/SGL append. A tetra OBJ's source-style affine normalization, RGB and exact Undo/Redo are checked in the normal browser; the same input matches pinned SculptGL's 16 affine and 12 color `f32` words with `1.100e-6` maximum nearest world-position component difference after JSON parsing. PLY/STL, empty SGL and malformed OBJ are also exercised. Multi-object mesh files, broader paired checks and large-mesh responsiveness remain open; no SculptGL or ZBrush parity is claimed.

**C212 multi-object OBJ Add:** [The scoped browser report](SCULPTGL_MULTI_OBJECT_OBJ_C212.md) retains two `o` records as separate SubTools, shares their group normalization, and commits them as one Undo/Redo step. The same six-vertex file in pinned SculptGL and OBrush matches both objects' affine and RGB `f32` words; nearest world-position component differences are at most `4.072e-6` after saved JSON parsing. Multi-object PLY/STL, materials, wider OBJ syntax, normal-browser million-vertex responsiveness and complete SculptGL/ZBrush parity remain open.

**C213 selected-chunk upload:** [The complete million-vertex browser report](SCULPTGL_SELECTED_CHUNK_UPLOAD_C213.md) verifies 887 lineage-certified changed-chunk uploads with whole-mesh mask/RGB change detection, exact public-file/shape/history audit and an 11.86 ms observed mesh-upload CPU mean versus 12.74 ms in the separate C210 run. RAF P95 66.8 ms, CPU P95 70.1 ms and release 165.5 ms still fail frozen responsiveness gates; distinct contacts prevent a causal end-to-end speed claim. No parity status changes.

**C210 controlled-normal delta:** [The scoped browser report](SCULPTGL_CONTROLLED_NORMAL_DELTA_C210.md) narrows shared-core normal-change detection for a checked Source dab and passes the independent million-vertex file/shape/history audit. The small measured state-advance reduction does not close browser responsiveness or reference parity.

**C207–C209 release-stage profile:** [The measured report](SCULPTGL_RELEASE_STAGE_PROFILE_C207_C209.md) records two reverted release shortcuts and an instrumented complete million-vertex browser run. Shared-core release took 81.0 ms and mouse-release acknowledgment 167.2 ms in that run; the independent shape/file/history audit passes. This is diagnosis, not a responsiveness or parity promotion.

**SculptGL is an intermediate foundation checkpoint.** Its toolset does not replace the ZBrush target or remove advanced desktop, legacy-document, production or integration workflows from scope. The comparison and separate capability gates below prevent that scope reduction.

**C205–C206 rejected upload trials:** [Separate-channel caching](SCULPTGL_UPLOAD_LAYOUT_TRIAL_C205.md) raised directly measured mesh-upload CPU in the million-vertex browser trace; [packed-row byte comparison](SCULPTGL_ROW_COMPARE_TRIAL_C206.md) did not materially lower it. Both were removed and the rebuilt production WASM matches C204 exactly. All responsiveness and parity gates remain open.

**C204 renderer-frame profile:** [The measured browser report](SCULPTGL_RENDER_FRAME_PROFILE_C204.md) attributes the current complete million-vertex trace to 2.50 seconds of snapshot copying, 10.08 seconds of scene admission and 16.22 seconds of mesh upload within the renderer callback. Exact file/history/shape audits pass, but all four responsiveness gates remain failed. No parity status changes.

**C203 complete million-vertex companion audit:** [The browser report](SCULPTGL_FILE_PUBLICATION_C203.md) extends only post-timing download polling to complete full-project Save/Undo/Redo checks on the C202 build. The independent audit passes, but RAF P95 96.2 ms and release 202.0 ms still fail the frozen responsiveness gates. Different delivered contacts prevent a causal speed comparison; no parity status changes.

**C202 release retained match:** [The scoped browser report](SCULPTGL_RELEASE_MATCH_C202.md) reuses the exact retained-state admission within one Source release call. The 100K/60-second normal-browser run passes full-project and responsiveness checks. Two 1M attempts failed the frozen download polling window; one has timing-only evidence at 144.8 ms release and 72.6 ms RAF P95. C203 subsequently completes the full 1M file audit for the same build; responsiveness still fails. No parity status changes.

**C201 rejected topology matcher:** [The failed-performance browser report](SCULPTGL_TOPOLOGY_MATCH_TRIAL_C201.md) records a slower contiguous face/triangle scan: retained match 16.18 ms per dab versus 7.48 ms in C200. Exact geometry/history passed, but the matcher was removed. No parity status changes.

**C200 release validation:** [The scoped browser report](SCULPTGL_RELEASE_VALIDATION_C200.md) reuses active mesh/pose checks within one end-of-stroke call. The full-project million-vertex browser audit passes; release acknowledgment measured 146.9 ms versus 263.8 ms in the separate C199 run, while RAF P95 is 76.4 ms and mean queued dab is 20.59 ms. All responsiveness gates still fail; no SculptGL or ZBrush parity is claimed.

**C199 prepared Source match:** [The scoped browser report](SCULPTGL_PREPARED_MATCH_C199.md) removes a repeated full retained-mesh comparison within one exclusive prepared dab. The full-project browser audit passes, with mean queued work 19.55 ms versus 24.53 ms in the separate C197 run. RAF P95 72.4 ms and release 263.8 ms still fail the SculptGL responsiveness gates; no SculptGL or ZBrush parity is claimed.

**C197 Source witness trial:** [The scoped browser report](SCULPTGL_SOURCE_WITNESS_C197.md) makes controlled Source dabs copy retained position words and update only selected rows, preserving full-field history and general refresh behavior. A million-vertex browser run passes complete geometry/history audit, but RAF P95 remains 77.5 ms and release takes 288 ms. Two exact retained-match shortcuts worsened timing and were removed. SculptGL performance and ZBrush parity remain open.

**C181 browser WASM SIMD fingerprint:** [The scoped report](WEB_WASM_SIMD_C181.md) records a dual portable/SIMD browser build, automatic fallback, identical million-vertex three-contact output and visible history, and a shorter source-match stage (56.4 ms versus 121.9 ms). The 10-second SIMD WebGPU Brush preview still fails RAF and release responsiveness gates, with 470.4 ms release. The normal studio remains WASM geometry plus WebGL2; neither SculptGL performance nor ZBrush parity is established.

**C182 resident commit duplicate-check removal:** [The two-stroke and long-stroke browser comparison](WEBGPU_RESIDENT_COMMIT_C182.md) keeps the first mesh/pose validation and all scene/budget/history checks while removing a repeated million-vertex scan. The second stroke's replacement stage measured 91.7 ms before and 53.8 ms after, with identical visible output and history, but release still took 146.1 ms. The final 10-second run still failed RAF and release gates at 50.1 ms P95 and 433.3 ms release. No SculptGL performance or ZBrush parity claim is made.

**C183 resident upload startup:** [The scoped browser comparison](WEBGPU_UPLOAD_STARTUP_C183.md) gives the transient GPU upload a BLAKE3-checked version 2 with version 1 decode support and separates the bridge's checked commit identity from its unused CPU reference snapshot. On the same two-stroke million-vertex trace, first preview start measured 3,595 ms before and 590 ms with SIMD afterward; visible edit and Undo/Redo hashes matched. The final 10-second preview still fails RAF/release gates at 50.1 ms P95 and 437.3 ms release; normal-studio WebGPU integration and SculptGL/ZBrush parity remain open.

**C184 checked sparse candidate validation:** [The scoped report](WEBGPU_RESIDENT_SPARSE_VALIDATION_C184.md) uses the exact validated capture and checked sparse rows to avoid rescanning unchanged million-vertex candidate channels while retaining world-position, metadata, scene, fold and history checks. The matched second stroke's release measured 146.8 ms before and 128.0 ms after with identical visible output/history. The final 10-second run still fails RAF and release gates at 50.1 ms P95 and 414.7 ms release. This does not promote normal-studio WebGPU or reference parity.

**C185–C186 normal-pass trials:** [The preserved report](WEBGPU_NORMAL_PASS_TRIALS_C185_C186.md) records two rejected GPU-pass reductions. Fusing position and face commits and replacing the full-face commit with a bounded indirect worklist retained checked native/browser output but did not improve the 10-second million-vertex RAF or GPU-wait results. Both code changes were removed; the restored GPU build matches C184, and no normal-studio WebGPU or parity status is promoted.

**C187 bounded GPU queue trial:** [The preserved browser report](WEBGPU_TWO_FLIGHT_TRIAL_C187.md) records a failed WASM borrow-boundary run followed by a corrected owned-fence two-dab run. The latter nearly doubled applied dabs without improving the 50.1 ms RAF P95 or about 414 ms release. The opt-in queue and owned-fence API were removed; no WebGPU promotion or parity status changes.

**C188 URL display startup:** [The scoped browser report](SCULPTGL_URL_DISPLAY_C188.md) connects three SculptGL-profile URL requests to the existing initial grid/contour renderer and preserves exact idle red/blue contour colors. Normal browser pixel, state and invalid-color checks pass. Pinned-reference visual response, later imports, other URL options and SculptGL/ZBrush parity remain open.

**C189 world-axis scale gizmo:** [The scoped report](SCULPTGL_SCALE_GIZMO_C189.md) adds visible X/Y/Z square scale handles to the normal transform viewport. An independently authored browser drag checks the displayed matrix, unchanged held mesh buffers/document, one shared-core commit, a measured 1.2549765 X factor around `[2,0,0]`, and exact Undo/Redo. Pinned SculptGL gesture/visual comparison, uniform ring and rotation handles, broader selected/masked browser cases and sustained performance remain open.

**C190 uniform ring and source scale policy:** [The scoped report](SCULPTGL_UNIFORM_RING_C190.md) adds an interactive uniform scale ring and updates axis cubes to the pinned SculptGL screen-distance formula. A normal browser pointer run checks source-derived axis/ring factors, one checked shared-core commit, visible matrix preview, exact history and selected-group mask-weighted output. Physical pinned-reference gesture/pixel comparison, rotation/plane handles, broader cameras and sustained large-mesh latency remain open. C189's values remain historical rather than describing the current axis gesture.

**C191 plane translation gizmo:** [The scoped report](SCULPTGL_PLANE_GIZMO_C191.md) adds X/Y/Z plane pads using the pinned SculptGL ray/plane and pointer-offset gesture policy. All three planes pass a native constrained-drag test; an isolated normal-browser Plane Z drag passes held-preview, one shared-core commit, independent affine arithmetic and exact Undo/Redo within 17 transform workflows. Paired reference gesture/pixels, other browser plane/camera poses, rotation and sustained large-mesh performance remain open.

**C192 all-plane browser follow-up:** [The scoped report](SCULPTGL_PLANE_GIZMO_C192.md) exercises X/Y/Z plane pads and a mask-weighted two-part Plane Z gesture in the normal browser. Twenty workflows pass with exact prohibited axes, held-preview isolation, checked vertex arithmetic and exact history. A larger masked pull is preserved as a fold-guard rejection; pinned-source physical gestures, other poses and sustained performance remain open.

**C193 world-axis rotation arcs:** [The scoped report](SCULPTGL_ROTATION_GIZMO_C193.md) adds X/Y/Z tangent-drag rotation arcs and a shared-core selected-group commit. A normal browser run passes 24 transform workflows, including source-rule angle checks, rigid held preview, independent affine arithmetic, a mask-weighted two-part drag and exact history. Physical pinned-source arc/pick behavior, other poses and sustained performance remain open; the declared source `ROT_W` circle has no update dispatch in the pinned code and needs runtime clarification.

**C194 pinned rotation runtime:** [The scoped probe](SCULPTGL_GIZMO_RUNTIME_C194.md) confirms source X/Y/Z direct gizmo updates and mask-protected commits on an independently authored 266-vertex sphere. The declared and drawn `ROT_W` view circle is absent from the runtime pickable list and leaves the edit matrix unchanged under a forced direct update. Physical reference mouse picking and matched OBrush geometry remain open; no parity status is promoted.

**C195 physical reference rotation:** [The browser-event probe](SCULPTGL_GIZMO_PHYSICAL_C195.md) passes X/Y/Z hover, press, drag, masked position/matrix oracles, commit and exact GUI Undo/Redo on the pinned source with integer pointer coordinates. The failed fractional-coordinate run is preserved. Same-fixture OBrush geometry, alternate poses and sustained responsiveness remain open; no parity status is promoted.

**C196 paired physical rotation:** [The same-sphere browser comparison](SCULPTGL_GIZMO_PAIR_C196.md) passes X/Y/Z OBrush arc gestures, pinned source angle-rule checks, mask-weighted geometry and exact history. At each pinned physical-drag angle, every OBrush vertex differs from the source output by at most `8.380e-8`. Alternate poses, real devices, long-session performance and the remaining Transform family are open; no parity status is promoted.

**C134 selected-group numeric transform:** [The checked report](SCULPTGL_SELECTED_GROUP_TRANSFORM_C134.md) records an atomic shared-core world transform for selected parts, a normal browser numeric-panel run on a posed two-mesh SGL, exact serialized affine translation and Undo/Redo hashes. Interactive group gizmo and pinned-source comparison remain open; this does not promote SculptGL or ZBrush transform parity.

**C135 group-gizmo follow-up:** [The checked report](SCULPTGL_SELECTED_GROUP_GIZMO_C135.md) records shared selected-group translation preview and one atomic release through the viewport gizmo, with nine normal browser transform workflows and exact saved history. Other SculptGL transform handles and mask-weighted/paired source behavior remain open; no parity status is promoted.

**C136 selected mask-weighted translation:** [The scoped report](SCULPTGL_MASKED_TRANSFORM_C136.md) records an opt-in shared-core mask-weighted commit for selected numeric and gizmo translation, with rigid drag preview, protected vertices, atomic history and eleven normal browser transform workflows. The pinned SculptGL source formula was inspected, but an instrumented reference runtime comparison, retained-detail transport, masked rotation/scale and sustained performance remain open; no transform parity status is promoted.

**C137 mask-weighted affine follow-up:** [The scoped report](SCULPTGL_MASKED_AFFINE_C137.md) extends the opt-in selected-group commit to numeric XYZ rotation and uniform scale on posed meshes; an independent world-space oracle and twelve normal browser transform workflows pass. Pinned SculptGL runtime behavior, retained hierarchy, nonuniform scale, the source rotation/scale handles and sustained responsiveness remain open. This supersedes C136's rotation/scale gap only; no family parity status is promoted.

**C138 pinned-runtime mask-transform pair:** [The paired report](SCULPTGL_MASKED_TRANSFORM_PAIR_C138.md) compares controlled `Transform.end()` outputs from the pinned SculptGL runtime with OBrush's normal browser numeric controls on the same independently authored masked OBJ. After explicit vertex-order mapping, maximum output-component differences are `4.715e-8` for translation and `1.658e-7` for rotation plus uniform scale; both histories recover exactly. Source gizmo gestures, multi-selection, posed/reference-rich meshes and sustained performance remain open. This is a narrow measured comparison, not family parity.

**C139 world-axis scale foundation:** [The checked report](SCULPTGL_AXIS_SCALE_C139.md) adds independent positive XYZ scale controls and a selected-group shared-core transaction. On the pinned `Transform.end()` cube, mapped OBrush output differs by at most `5.288e-8` per position component after a controlled diagonal source edit matrix; thirteen normal browser transform workflows pass. Physical scale handles, reference poses/groups, retained masked detail and performance remain open. Numeric scale is not SculptGL gizmo parity.

**C140 brush timing boundary:** [The scoped report](SCULPTGL_DAB_TIMING_C140.md) adds read-only wall-time counters for queued brush application and stroke release. Four normal-browser stationary pen/mouse contacts populated the counters with exact GPU/save correspondence; the later alpha-import step of that legacy harness failed and is preserved. This is diagnostic evidence only. Production WebGPU brush integration, the unchanged million-vertex responsiveness gates and SculptGL/ZBrush brush comparison remain open.

**C141 sustained Source timing matrix:** [The six-cohort report](SCULPTGL_SOURCE_TIMING_C141.md) runs the frozen 10K/100K/1M Source Brush browser traces for 60 and 120 seconds with the current build. Both smaller sizes pass the named local data and responsiveness gates; both million-vertex runs fail the same four responsiveness gates. Queued Source geometry averages about 40 ms per applied dab at 1M, directly measuring a main-thread cost that a renderer-only WebGPU swap would leave in place. This remains a local scaling diagnostic; production WebGPU integration and matched SculptGL performance/response are open.

**C142 Source stage timing:** [The browser report](SCULPTGL_SOURCE_STAGE_C142.md) splits the million-vertex queued Source dab into 17.56 ms preparation and 23.30 ms deformation on average. The frozen 60-second run passes whole-project/history/shape checks and fails the same four responsiveness gates. A C143 triangle-dispatch shortcut showed no material improvement and was removed; its failed browser check is preserved. These timings guide GPU integration and do not establish a speedup or parity.

**C144 Source preparation split:** [The frozen browser run](SCULPTGL_SOURCE_PREPARATION_C144.md) measures 5.32 ms retained-state matching, 12.26 ms contact preparation including validation/picking, and 23.28 ms deformation per applied dab at 999,002 vertices. The same four responsiveness gates fail. This confirms that a deformation-only GPU pass would leave significant CPU work; integrated resident picking, state and drawing remain open.

**C145 retained-geometry validation:** [The shared-core/browser report](SCULPTGL_RETAINED_PICK_C145.md) removes redundant full position/triangle validation after an exact retained-state match while still checking mutable attributes, UV/surface metadata and limits. The 1M/60-second browser run indicates lower contact time, but delivered contacts differ and the same four responsiveness gates fail. Production WebGPU pointer sculpting and matched reference performance remain open.

**C146 opt-in WebGPU pointer handoff:** [The scoped report](WEBGPU_POINTER_C146.md) records a real browser mouse stroke on the 40,962-vertex startup sphere through a separate WebGPU canvas and a checked sparse commit into the shared Rust document. The changed project and viewport, plus visible Undo/Redo recovery, pass. It covers one experimental Inflate stroke only; normal brushes still use WASM CPU geometry and WebGL2, and million-vertex responsiveness and source brush response remain open.

**C147 million-vertex WebGPU pointer check:** [The scoped report](WEBGPU_MILLION_POINTER_C147.md) records a corrected imported-camera ray span and stale source-normal cache replacement, followed by an accepted 999,002-vertex browser GPU stroke with exact visible Undo/Redo. A 10-second longer pointer stream still has about 50 ms RAF P95 and 1.7 s release, failing the responsiveness target. A one-pass pick shortcut did not improve observed pick time and was removed. This is neither production WebGPU integration nor SculptGL parity.

**C148 checked GPU commit fingerprint:** [The scoped report](WEBGPU_COMMIT_FINGERPRINT_C148.md) replaces JSON-formatted SHA-256 stale-mesh matching with a versioned binary cryptographic fingerprint covering every serialized mesh channel. On the same three-contact million-vertex browser fixture, measured commit time fell from about 1.59 s to 0.45 s while viewport and Undo/Redo hashes remained exact. A longer portable run still has 50.1 ms RAF P95 and 561.8 ms release, failing the responsiveness gates. This remains one experimental Inflate stroke, not a source brush or parity result.

**C149 resident topology reuse:** [The scoped report](WEBGPU_TOPOLOGY_CACHE_C149.md) retains an immutable, signature-checked full topology cache across successive experimental WebGPU position commits. In a two-stroke million-vertex browser replay the first/second fold-guard stages took 137.1/9.6 ms, with exact visible two-step Undo/Redo; the second release still took 360.5 ms. Different stroke geometry precludes a controlled end-to-end speed ratio. Normal-studio WebGPU integration, sustained responsiveness, and SculptGL parity remain open.

**C150 repeated resident release follow-up:** [The scoped report](WEBGPU_RESIDENT_RELEASE_C150.md) profiles validation, normals, scene and history separately; preserves the exact canonical mesh fingerprint while hashing little-endian arrays directly; and uses a bounded, bit-exact source witness before local normal refresh on a successive GPU stroke. On the independently authored million-vertex two-stroke browser fixture, second-stroke full/local normal stages were 95.5/11.5 ms across distinct one-run builds and the final-build release was 255.1 ms with exact visible two-step Undo/Redo. A final-build 10-second first stroke still had 50.1 ms RAF P95 and 549.0 ms release, failing responsiveness. Production WebGPU brushes and full SculptGL parity remain open.

**C151 mask thresholds and GPU-pick audit:** [The scoped report](SCULPTGL_MASK_THRESHOLDS_GPU_PICK_C151.md) adds a shared-core threshold/repetition/no-op history check for the already opt-in SculptGL-style Mask Sharpen response. Two conservative triangle-block GPU-pick trials kept exact short-stroke viewport and history hashes but did not improve the million-vertex long-stroke wait; both were removed. The ordinary browser studio and its sustained responsiveness gap are unchanged; this does not establish mask-family or SculptGL parity.

**C152 selected SGL export:** [The scoped report](SCULPTGL_SELECTED_SGL_EXPORT_C152.md) adds selected-part export in selection order through the shared core and normal browser File panel. An independent three-box browser fixture exported two selected parts while retaining its scene and selection; a hidden selected part, strict loss policy and empty selection are covered by core tests. The pinned source branch was inspected, but a live paired SculptGL export, other file formats, the full inventory and million-vertex responsiveness remain open.

**C153 GPU-owned pick experiment:** [The scoped report](WEBGPU_OWNED_PICK_C153.md) removes per-dab pick mappings in the opt-in WebGPU Inflate preview while retaining native/browser geometry and checked document/Undo recovery. A three-dab million-vertex browser trace matches the earlier readback viewport exactly; the final 10-second run still fails frame and release responsiveness, and fenced profiling identifies pick and draw as the expensive stages. A rejected unpaced stroke, failed profiler and unhelpful block broadphase are preserved. Normal brushes remain WASM CPU with WebGL2, and SculptGL/ZBrush parity remains open.

**C155 depth-pick experiment:** [The scoped browser report](WEBGPU_DEPTH_PICK_C155.md) adds a separate opt-in WebGPU depth-texture pick for the resident Inflate preview and preserves a failed validation trial. Corrected million-vertex short and ten-second pointer runs pass checked document/history, but the long run retains 50.1 ms RAF P95 and about 541 ms release, essentially unchanged from C153. The candidate's three-dab pick-plus-sculpt stage is about 104 ms versus 107 ms for the previous GPU ray method. Normal browser brushes still run shared Rust/WASM geometry and WebGL2; sustained responsiveness and reference parity remain open.

**C156 paired million-vertex sustained Brush:** [The scoped comparison](SCULPTGL_MILLION_PAIRED_C156.md) runs the same frozen 1M/60-second Source Ordinary Brush fixture and protocol in pinned SculptGL and current OBrush browsers. The reference passes four local responsiveness gates at 21.5 ms RAF P95; OBrush fails them at 93.2 ms before and 91.9 ms after removing redundant full-position changed-row scans in shared Rust. All three complete shape and exact history audits pass. Different delivered event sequences prevent a causal speed ratio or matched brush-response claim; normal browser performance and SculptGL parity remain open.

**C157 Source phase timing and single-pass count:** [The scoped report](SCULPTGL_SOURCE_PHASE_C157.md) measures the normal million-vertex browser brush in six core sub-stages and removes one redundant whole-mesh changed-row count for single-pass controlled strokes. The final run passes whole-project and history audit but still has 91.0 ms RAF P95, 36.71 ms mean queued dab, and 300.1 ms release acknowledgement. An initial WASM clock panic and a missing aggregate-counter run are preserved as failed diagnostics. Distinct delivered contacts prevent a causal end-to-end speed claim; production WebGPU integration and SculptGL/ZBrush parity remain open.

**C158 shared immutable stroke witness:** [The scoped report](SCULPTGL_SHARED_DOMAIN_C158.md) shares already-validated mask/color/material/UV witnesses across a staged controlled dab while keeping normal words private for atomic publication. A pinned million-vertex browser run passes independent whole-project geometry and history audit, with observed 35.32 ms mean dab and 89.7 ms RAF P95. It still fails all four responsiveness gates. Delivered contacts differ from C157, so the observation does not establish a causal end-to-end speed ratio or source brush parity; integrated resident GPU work and the remaining SculptGL inventory are open.

**C159 verified warm contact:** [The scoped report](SCULPTGL_VERIFIED_CONTACT_C159.md) pairs a retained geometry match with the captured stroke-domain witness inside one exclusive Document borrow, avoiding a repeated whole-mesh attribute scan during warm contact picking while preserving full validation on cold or mismatched inputs. The pinned million-vertex browser run passes whole-project geometry/history audit; observed preparation is 7.56 ms and queued dab 27.52 ms on average, with 86.7 ms RAF P95. All four responsiveness gates still fail, distinct delivered contacts prevent a causal speed ratio, and SculptGL/ZBrush parity remain open.

**C160 prepared-dab exact-domain seal:** [The scoped report](SCULPTGL_PREPARED_SEAL_C160.md) carries the C159 exact-domain proof across one exclusive prepared-document borrow, removing the second full-domain comparison only for that private path. The final-build pinned million-vertex browser run passes whole-project geometry/history audit, with observed 25.04 ms mean queued dab and 79.5 ms RAF P95. All four responsiveness gates still fail, the separate runs have different delivered contacts, and the full SculptGL inventory and ZBrush target remain open.

**C161 Source topology upload token:** [The scoped report](SCULPTGL_TOPOLOGY_TOKEN_C161.md) uses shared immutable Source topology identity to omit repeated full WebGL triangle-index uploads during ordinary brush strokes. On the pinned million-vertex browser fixture, full 23,976,000-byte index uploads fall from 778 to one, while the complete mesh/history audit passes. The separate runs have different delivered events; mean queued dab remains about 25 ms, all four responsiveness gates still fail, and a legacy mask-only upload gate fails outside this Source-only scope. Production WebGPU brushes and SculptGL/ZBrush parity remain open.

**C162 Ctrl background mask workflow:** [The paired browser report](SCULPTGL_BACKGROUND_MASK_C162.md) adds Ctrl-click empty viewport to invert the active mask and Ctrl-drag empty viewport to clear it through shared-core history. Trusted browser input in the pinned SculptGL and OBrush builds produced the same all-masked/all-unmasked semantic transitions on their different startup spheres; OBrush saved project hashes recover exactly through Undo/Redo, and Save during a held background press cancels the later release. Other input devices, nondefault masks, native GUI, performance, and full reference parity remain open.

**C163 WebGPU stage boundary:** [The three-contact report](WEBGPU_STAGE_BOUNDARY_C163.md) separates synchronous WebGPU presentation from the fenced point/normal and draw stages on the million-vertex browser fixture. A tested face-row sentinel retained exact visible edit/history but had no defensible speedup and was removed. This is diagnostic progress; the normal studio remains WASM/WebGL2, and sustained responsiveness and reference parity remain open.

**C164 JPEG alpha input:** [The bounded browser report](SCULPTGL_ALPHA_JPEG_C164.md) adds shared native/WASM PNG-or-JPEG decoding to the per-tool brush alpha picker. An independently encoded 16×16 JPEG has exact Mean RGB samples in unit checks; the normal browser imports it through the visible picker, sculpts, and recovers exact clipped viewport hashes through Undo/Redo. Square/Skin/None per-tool choice also passes in that session. Other browser image formats, exact reference presets and brush-response parity remain open.

**C165 Source Brush JPEG workflow:** [The scoped browser report](SCULPTGL_SOURCE_JPEG_C165.md) carries the same independent JPEG through explicit raw-mask conversion, Source Brush selection and one visible drag. Three applied dabs, a changed viewport and exact keyboard Undo/Redo hashes pass. The Legacy Draw path also passes again. This is one input/workflow check, not a pinned SculptGL vertex comparison, sustained performance result or family-parity promotion.

**C166 pinned JPEG alpha pair:** [The runtime report](SCULPTGL_JPEG_ALPHA_PAIR_C166.md) imports that independent JPEG through pinned SculptGL's real browser image-input route. All 256 reference texture values and five primary/mirrored nearest-sampling points agree with the independent 64/192 oracle; the reference texture checksum equals the captured OBrush Source Brush texture. Per-vertex sculpt output, other images/tools and sustained reference performance remain open.

**C167 transparent PNG alpha correction:** [The paired report](SCULPTGL_TRANSPARENT_ALPHA_C167.md) finds that pinned SculptGL's Canvas2D import zeros hidden RGB under alpha 0, whereas OBrush's earlier Mean RGB conversion used that hidden color. Shared native/WASM conversion now matches the pinned `[0,85,85,255]` four-pixel output while Legacy Weighted Opacity stays distinct. The final normal browser Source Brush captures the matching alpha checksum and produces exact visible Undo/Redo; arbitrary color profiles and same-mesh displacement parity remain open.

**C168 low-opacity PNG alpha pair:** [The pinned-runtime report](SCULPTGL_ALPHA_QUANTIZATION_C168.md) exposes 136/256 alpha-byte differences on an independent RGBA matrix after C167. The shared Rust converter now reproduces the pinned Chrome Canvas2D premultiply/readback rounding for all 256 measured pixels; the normal OBrush Source Brush captures the same texture checksum and retains exact visible Undo/Redo. Browser/color-profile breadth and matched vertex displacement are still open.

**C169 controlled alpha Brush dab:** [The paired kernel report](SCULPTGL_BRUSH_ALPHA_PAIR_C169.md) compares one ordinary Brush dab in pinned SculptGL's actual browser method with the shared Rust Source geometry path on the same independent three-vertex triangle and exact 256-byte imported alpha. All nine output f32 position words match. The source call supplies its picked normal directly, so normal UI strokes and repeated-dab response remain unpaired; a non-equivalent second-dab trial is preserved. Dense-mesh brush quality, production WebGPU integration, sustained responsiveness, and full SculptGL/ZBrush parity remain open.

**C170 refreshed-normal Brush pair:** [The two-dab report](SCULPTGL_BRUSH_REPEATED_NORMAL_C170.md) replaces C169's non-equivalent fixed-normal second-dab trial with a controlled reference face-normal refresh and actual pinned `Picking.computePickedNormal()` call. On the same one-face triangle, all nine f32 position words match after each of two accumulated dabs both without alpha and with the exact imported 16×16 alpha. Normal UI pointer strokes, moving-contact alpha frames, dense meshes, long-stroke quality and performance remain unpaired; no family parity is promoted.

**C171 moving alpha Brush pair:** [The moving-contact report](SCULPTGL_BRUSH_MOVING_ALPHA_C171.md) advances the second picked center on the same triangle and runs the pinned runtime's `Picking.updateAlpha(false)` after its normal refresh. The shared Rust alpha track and geometry match all three second-dab alpha samples and all nine output position words; 15/16 frame words match bitwise, with one harmless `+0.0`/`-0.0` distinction preserved as a failed strict check. Actual pointer strokes, broader meshes, sustained responsiveness and full SculptGL/ZBrush parity remain open.

**C172 normal-browser Brush pair:** [The paired UI report](SCULPTGL_UI_BRUSH_PAIR_C172.md) imports the same independent 10,114-vertex SGL sphere through both normal browser file inputs and uses trusted pointer strokes. After calibrating OBrush's rendered orthographic pixel offsets and radius to SculptGL's different canvas projection, both apps change the same 96 vertex IDs for a two-move drag; only 19 mapped vertices differ, by at most `5.960464477539063e-8` per component, with exact per-app public Undo/Redo. A fresh pair repeats the same output hashes and counts. Alpha/pressure/symmetry, arbitrary meshes/views, native GUI and the failed million-vertex responsiveness gates remain open; no family parity is promoted.

**C173 Brush setting pairs:** [The normal-browser setting report](SCULPTGL_UI_BRUSH_SETTINGS_C173.md) adds two visible-slider settings on the same independent sphere. At the source UI's published 31 px/50% and 95 px/10% settings, projection-calibrated OBrush changes the same 23 and 194 vertex IDs respectively; maximum mapped position-component differences are `2.9802322387695312e-8` and `5.960464477539063e-8`. Complete attributes/faces and per-app public Undo/Redo pass. This expands measured ordinary Brush response without closing alpha, pressure, symmetry, dense-mesh performance or full family parity.

**C174 Brush Accumulate effect:** [The normal-browser toggle report](SCULPTGL_UI_BRUSH_ACCUMULATE_C174.md) pairs the actual Accumulate checkbox and a six-move retrace on the same independent sphere. Both apps change the same 98 IDs in each mode, and turning Accumulate on changes the same 96 IDs within each app, with equal `0.00035369396209716797` maximum effect and at most `1.1920928955078125e-7` cross-app effect-component difference. Whole-file attributes/faces and public Undo/Redo pass. Other stroke shapes, settings, topology, sustained performance and full Brush parity remain open.

**C175 session history length:** [The paired report](SCULPTGL_HISTORY_LIMIT_C175.md) adds a 3–50 history-length control with default 15 in the shared Rust document and normal browser inspector. Pinned SculptGL and OBrush visible workflows each retain only three of five scene edits at limit 3, restore exact own-format saves through Redo, and keep saves unchanged on 50/10 changes. A second branch confirms the pinned 10→3 nearest-past-first behavior: six Undo and three Redo actions become three Undo and zero Redo in both. Mixed sculpt edits are checked separately in C176; topology edits, memory-pressure behavior and full history parity remain open.

**C176 mixed scene and sculpt history:** [The paired normal-browser report](SCULPTGL_MIXED_HISTORY_C176.md) interleaves sphere addition, a real sculpt contact and cube addition at history limit three. SculptGL changes 998 position words; OBrush changes its saved project. Both Undo through cube, sculpt and sphere states in the same order and recover byte-identical own-format exports after three Redos. The fixtures do not equate brush shapes, and memory-pressure/native-GUI/long-session history remain open.

**C177 topology and scene history:** [The paired normal-browser report](SCULPTGL_TOPOLOGY_HISTORY_C177.md) interleaves cube addition, visible static subdivision and sphere addition at history limit three. Pinned SculptGL grows its cube from 98,306 to 393,218 vertices; OBrush grows its own from 24,578 to 98,306. Both Undo through sphere, fine cube and coarse cube to an empty scene, then recover byte-identical own-format exports through three Redos. Different primitive starting meshes and remaining memory-pressure/native-GUI/long-session gates prevent a history-family parity claim.

**C178 bounded Source vertex uploads:** [The measured normal-browser report](SCULPTGL_SPARSE_VERTEX_UPLOAD_C178.md) caches one active object's exact packed rows under 48 MiB and uploads bounded changed WebGL chunks after a fixed-topology proof. On the frozen million-vertex/60-second workload, full vertex and material uploads fall from 804 each to one each, and recorded mesh-sized transfer falls from 41.79 to 6.37 GB. The independent whole-project/changed-face audit passes, but RAF interval P95 remains 76.9 ms in both separate runs; all four responsiveness gates still fail. This is a transfer reduction, not a measured speedup or SculptGL/ZBrush parity.

**C179 explicit-direction WebGPU Brush primitive:** [The scoped report](WEBGPU_BRUSH_C179.md) adds an ordinary Brush displacement candidate to the separate resident-GPU path. On one independently generated 162-vertex sphere, native Metal and browser WebGPU agree on every position word; the seven-vertex sparse edit retains exact Undo/Redo/cancel. Live pointer picking, full Source policy and normal-studio integration remain open, as do all four sustained responsiveness gates. This does not establish a browser speedup or reference parity.

**C180 GPU-owned picked Brush preview:** [The scoped report](WEBGPU_PICKED_BRUSH_C180.md) connects a real browser Brush pointer stroke to GPU-resident pick, original-face direction, displacement and visible checked history. Native Metal and browser WebGPU agree byte-exactly on one sphere probe, and a nonplanar quad probe matches a scalar direction reference within `3.73e-9` output units. A 10-second million-vertex preview stroke passes visible Undo/Redo but still has 50.1 ms RAF P95 and 539.2 ms release, failing responsiveness gates. This opt-in separate canvas is not normal-studio integration, full Source behavior, or SculptGL/ZBrush parity.

## Status rules

| Status | Meaning |
| --- | --- |
| Planned | No delivered implementation is established. |
| Foundation / unverified | A first implementation exists; its full row-specific gate is not yet established. Limited completed local checks are recorded separately. |
| Locally verified foundation | Named OBrush checks pass on a recorded commit/platform; comparison to ZBrush remains unverified. |
| Compared / gaps remain | Reference and OBrush ran the same fixture; differences and limitations are recorded. |
| Parity verified | All named behavior, interchange, usability and performance gates pass against the pinned reference on the declared platforms. |

Record status separately for native desktop and browser. A native build cannot establish browser execution, and a browser build cannot establish pointer pressure, file persistence, or GPU rendering. Attach evidence to a commit, feature ID and exact runtime; downgrade stale results after a material change. Tests below are **OBrush acceptance requirements**, not claims that ZBrush already passed them or descriptions of its internal algorithms.

## First sculpting milestone

The prototype has a Rust sculpting core shared by native and WebAssembly applications, with a GPU viewport. The historical **0.1.0** checks and their limits remain in [VALIDATION.md](VALIDATION.md): **36 tests** (32 core, 4 app), strict native/WebAssembly Clippy checks, browser/native editing and saving, project exchange, and a corrected browser import of ZBrush's `#MRGB` vertex colors. These results do not establish the changed 0.1.1 rendering/subdivision behavior or every row-specific acceptance gate; only F14 has the limited reference comparison described below. The brush labels are OBrush tools; matching ZBrush's response curves and detailed settings remains later work. Source vocabulary: Maxon's [brush guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/sculpting-brushes.html), [masking](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/masking/masking.html), [PolyPaint](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/painting-your-model/polypaint/polypaint.html), and [subdivision levels](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/subdivision-levels/subdivision-levels.html).

**0.1.1 visual-quality update:** the default sphere now has 40,962 vertices / 81,920 triangles; the app offers Loop smooth subdivision and retained linear refinement, stable arc-length dab sampling with path breaks, and offscreen viewport multisampling that requests 4× with 2×/direct-rendering fallbacks and reports actual allocation. The [visual-quality report](VISUAL_QUALITY.md) retains the original density/stroke failures and exact numerical evidence. The source passes **47 tests** (39 core, 8 app) and formatting after the final input-discontinuity changes. Strict native/WASM Clippy and web plus ad-hoc-signed macOS builds passed during this update; the [build record](evidence/visual-quality-build.json) binds final source/build hashes to the completed checks. An earlier 0.1.1 Chrome startup displayed 4× antialiasing active without reported errors. Runtime checks were not repeated after the final input/state fixes; current native quality visuals, resize/toggle behavior and complete editing workflows remain **unverified**. These changes do not establish ZBrush appearance, subdivision or brush parity.

**0.1.2 deformation update:** repeated Crease at the app defaults (radius 0.18, strength 0.35) is now exercised against a frozen 0.1.1 core. Geometry dabs use a shared reduced step when needed to preserve local face orientation/quality and limit newly extreme adjacent folds; Crease uses a gentler pinch, blended surface normal and gradual front-facing influence. Move anchors follow the actual picked triangle point after deformation, including protection and symmetry. [CREASE_STABILITY.md](CREASE_STABILITY.md) and its [comparison](evidence/crease-stability-comparison.json) retain baseline and intermediate failures, exact fixture parameters and measured groove/stability results. [Build evidence](evidence/crease-stability-build.json) identifies the final checked implementation. Local tests do not establish global self-intersection prevention, repair of previously damaged geometry, native/browser live behavior or ZBrush brush parity.

**0.1.3 Draw-quality update:** Draw now uses an area-weighted patch direction, blends mirrored directions while retaining maximum scalar influence, fairs small surface bumps, and applies tighter local quality bounds. Accepted Draw dabs add bounded conforming edge refinement with mask/color interpolation and stroke undo. Angle-weighted normals preserve existing corner contributions through linear face splits. [DRAW_QUALITY.md](DRAW_QUALITY.md), its [CPU comparison](evidence/draw-overlap-comparison.json), [rendered fixtures](evidence/draw-visual-comparison.json) and [build/runtime records](evidence/draw-quality-build.json) separate local evidence from reference claims. Repeated overlapping Draw checks need smoothness/density metrics in addition to no inversions. Split-only Draw refinement does not establish Sculptris Pro, remeshing, decimation or persistent subdivision parity.

**0.1.4 performance update:** shared core optimizations preserve all 12 same-ray final OBJ fixtures exactly against frozen 0.1.3; four direct mesh JSON comparisons additionally cover mask bits. Local corner normals, patch-only fairing, compact adjacency and conservative refinement candidates reduce measured native sculpting cost. Ordered input batches retain captured world rays/settings and whole-stroke history, with save/undo/import barriers and synchronous Move anchors. [Performance evidence](BROWSER_PERFORMANCE.md) distinguishes browser redraw/input measurements from native CPU timing and physical input latency. This does not establish reference performance or brush parity.

**0.1.5 sustained-brushing update:** the input queue retains at most 32 pending dabs, discards older unprocessed motion under load, and commits at most the latest pending endpoint on release. The 50 ms age setting is a discard threshold checked between atomic operations, not a hard input-latency guarantee. Face incidence accelerates local normals and deformation guards. Crease blends mirrored directions, follows a coherent depression direction, relaxes local irregularities and limits new adjacent folds to 90°. Draw, Clay, Inflate, Flatten and Crease now share bounded conforming local detail; child triangles are checked before splitting, including protected boundaries. These changes intentionally alter some geometry and overload input behavior. [LONG_STROKES.md](LONG_STROKES.md) distinguishes long browser input tests, native CPU timing, independently authored geometry checks and defects already present in the supplied model. Existing folds are not automatically repaired; global self-intersections, native live interaction and ZBrush response/performance remain unverified.

| ID | Prototype capability | Status | Acceptance gate |
| --- | --- | --- | --- |
| F01 | Draw | Locally verified foundation — overlap/refinement fixtures | A recorded surface stroke changes positions only inside its influence; inverse mode reverses the displacement sign; zero strength makes no change. In 0.1.3, local refinement can split incident faces across the patch boundary without changing their existing surface; tests cover masks, whole-stroke topology undo, shading and same-input smoothness. |
| F02 | Clay | Foundation / unverified | Repeated dabs accumulate a controlled raised form on the fixture; radius/strength alter the result; masked vertices remain fixed. |
| F03 | Inflate | Locally verified foundation | A dab moves affected vertices along their normals, positive and negative; opposite-side vertices remain fixed under the selected backface policy. |
| F04 | Smooth | Foundation / unverified | Roughness decreases on a noisy patch; unaffected vertices remain identical; no invalid indices or nonfinite positions appear. |
| F05 | Flatten | Locally verified foundation | The selected patch's distance to the brush plane decreases; plane and sign behavior are defined; masked vertices remain fixed. |
| F06 | Crease | Locally verified foundation — deformation fixtures; reference response unverified | A stroke forms a measurable groove and pinched cross-section; inverse response and radius are defined. The 0.1.2 fixtures additionally check per-dab orientation/quality, adjacent folds, masks and symmetry; global intersections and live reference behavior remain separate gates. |
| F07 | Move | Foundation / unverified — rest-space gesture/refinement fixtures | Dragging changes a stable material grab with falloff and useful displacement; tiny long pulls report safety limits rather than producing folded slivers. Verify event subdivision, symmetry overlap, pressure/alpha/masks, exact return, history and release on both hosts; orbit does not alter geometry. Broad/elastic deformation and reference response remain separate gaps. |
| F08 | Mask painting | Locally verified foundation | Paint, erase, invert and clear produce bounded mask weights; fully masked vertices reject sculpt displacement. |
| F09 | Vertex paint | Locally verified foundation | A stroke changes vertex colors with falloff and strength; undo restores exact previous colors; saved project round-trip preserves them. |
| F10 | X symmetry | Locally verified foundation | An off-center stroke on a symmetric fixture yields mirrored edits within declared floating-point tolerance; centerline vertices receive one intended application. |
| F11 | Basic subdivision | Locally verified foundation — linear and Loop fixture tests | Splitting the fixture produces valid indices and expected counts; mask/color interpolation is checked; undo restores the original mesh. Smooth stencils, unsupported topology and unchanged failure state are checked locally in 0.1.1. This historical linear/Loop gate does not establish ZBrush subdivision output; the separate 0.1.6 persistent-level foundation is covered under T04. |
| F12 | Picking and viewport | Locally verified foundation; 0.1.4 headless browser checks partial | Camera orbit/pan/zoom and ray picking align with the visible surface across resizing and HiDPI; rendering survives mesh edits, occlusion and wireframe switching. The 0.1.4 headless WebGL2 checks exercised Draw, orbit and actual 4× samples with timed input/redraw measurements; resize, HiDPI and wireframe coverage remain incomplete. |
| F13 | Stroke history | Locally verified foundation | One completed stroke is one undo step; undo/redo restores geometry, color and mask exactly; a new edit invalidates redo; cancelled/no-op strokes create no entry. |
| F14 | OBJ import/export | Compared / gaps remain — one fixture | Real OBrush browser OBJ imported into ZBrush 2026.2.1 and exported with 10,242 vertices, 20,480 faces and identical face indices; maximum position error `4.690415757354602e-8`, RMS `2.043570307341458e-8`. ZBrush emits PolyPaint in `#MRGB` rather than inline vertex RGB; corrected OBrush browser import/re-export now retains 10,242 RGB entries with measured first-record floating-point decode error `1.6470588226624727e-8`. MRGB masks, UVs, materials, negative-index/polygon reference cases and failure recovery remain separate parity gates; this exploratory comparison does not complete OBJ parity. |
| F15 | OBrush project files | Locally verified foundation; C298 image assets | Round-trip every field promised by the schema, validate version and mesh data before replacing the scene, and reject truncation or invalid input without data loss. C298 adds bounded UV/MatCap images and selection with browser Save/Open pixel-exact crop evidence; native GUI Save/Open, full file matrix and reference formats remain open. This is an OBrush format, not ZPR/ZTL compatibility. |
| F16 | Native and browser delivery | Locally verified foundation; 0.1.10 builds and scoped headless scene checks | Build and launch each target; perform a visible sculpt, undo, save/load and OBJ exchange in each; report tested OS, browser, GPU backend and available file APIs. Historical 0.1.4–0.1.8 receipts retain earlier host/Move/topology/layer/input checks on their runtime hashes. The 0.1.9 receipt covers eight Morph workflows, canonical project/history checks, 66 exact GPU vertex snapshots, 58 target binding checks, 15 layer compositions and eight quiet short releases. The 0.1.10 scene receipt additionally binds eight workflows, 71 complete visible GPU vertex/index comparisons and an inspector clipping gate. The native application is packaged and ad-hoc-signature verified; native GUI, physical devices, download filesystem persistence and the full host matrix remain unverified. |

The [OBJ comparison record](evidence/obj-zbrush-roundtrip.json) pins the source, reference-export and corrected browser re-export hashes. The actual ZBrush export is [primary format evidence](evidence/obj-mrgb-format-observations.json): its header specifies eight hexadecimal digits per vertex in **`MMRRGGBB`** order, with at most 64 entries per `#MRGB` line. This fixture contains 10,242 records and every `MM` byte is `ff`. Reference RGB quantization differs from the source by at most `0.0037254901960783737` per channel, below one 8-bit step; numerical tolerances were not preregistered for this exploratory comparison. RGB byte order/count are established by that output; the mask byte's exact range/direction requires a controlled masked-state export. OBrush's color decoder validates the mask byte but does not currently import its semantics; masks default to unmasked. Do not confuse this OBJ mask byte with the Draw palette's **MRGB** material/color mode, or infer it from another interchange format. This reference fixture has uniform default color; nonuniform paint, full UV, material and masking interchange remain unverified against ZBrush.

**0.1.6 sculpting-foundations update:** the shared core adds manual constrained QEM decimation, voxel remeshing of closed solids, persistent Loop levels, pressure weights and PNG raster alphas. Native threads and browser workers isolate remeshing/decimation; accepted topology results replace levels as one undo step. Version 2 projects retain the hierarchy and read version 1 files. [SCULPTING_FOUNDATIONS.md](SCULPTING_FOUNDATIONS.md) records the supported workflows, source/build/runtime evidence and limits. The remesher passes named healthy-solid invariants but fails the supplied damaged sculpt's fold-repair gate; failed checks remain in [remeshing evidence](evidence/topology-remesh.json). These additions do not promote any family to ZBrush or SculptGL parity. Physical tablet input, HD geometry, quad retopology and multiple objects remain gaps.

**0.1.7 Move correction:** the host uses shared rest-space Move gestures with cumulative camera-plane displacement, continuous mirror-vector blending and bounded preparation detail. Generated Move faces use the same quality measure as the deformation guard, with preparation headroom. [MOVE_QUALITY.md](MOVE_QUALITY.md) retains the baseline sliver/return failures, the rejected near-freeze candidate, measured corrections and range limits. Its [build record](evidence/move-quality-build.json) binds the 156 passing tests, required checks, native package and final browser runtime. New-detail no-ops preserve redo; persistent levels retain topology. The native sweep still fails four proposed tiny-brush movement-range gates, and existing damage/global intersections remain unresolved. No reference or family-parity status is promoted.

**0.1.8 geometry-layer foundation:** fixed-topology layers record shape offsets, blend intensity 0–1, hide/show, rename, delete and bake with undo. Version 3 projects retain the stack; nonlayered projects remain version 2 and older files remain readable. Existing subdivision levels stay stored, with navigation/topology edits blocked until bake. [SCULPT_LAYERS.md](SCULPT_LAYERS.md), its [core contracts](evidence/sculpt-layers-core.json), [seven browser workflows](evidence/sculpt-layers-browser.json) and [build record](evidence/sculpt-layers-build.json) bind 179 passing tests and the exact release artifacts. Layered paint/masks, inversion/exaggeration, further stack operations and per-level recording remain gaps. Short synthetic mouse traces do not establish physical input latency, long-session performance or ZBrush layer equivalence; S07 remains partial.

**0.1.9 Morph Target foundation:** one geometry-only target stores the current shape on its creation subdivision level, with full restore/swap and protected local Morph brushing. Targets survive retained-level navigation/addition; explicit object/topology replacement removes them with undo, while failed operations and zero-collapse reductions preserve them. Version 4 retains targets and optional layers; earlier formats remain readable. [MORPH_TARGETS.md](MORPH_TARGETS.md), its [core contracts](evidence/morph-target-core.json), [eight browser workflows](evidence/morph-target-browser.json) and [build receipt](evidence/morph-target-build.json) bind 208 passing tests and the exact release artifacts. Target response, additional Morph controls, native GUI/physical devices and broad performance remain unverified; S07 stays partial.

**0.1.10 SubTools foundation:** separate parts retain independent meshes, masks, paint, levels, layers and Morph Targets. Version 5 persists the scene; earlier files remain readable. [SUBTOOLS.md](SUBTOOLS.md) records limits and the unfinished folders, merge/split, transforms, booleans, interchange and reference gates. The final [build](evidence/subtools-build.json) and [browser](evidence/subtools-browser.json) receipts bind 247 tests, eight workflows/42 behavior gates, 71 GPU scene snapshots and the added inspector clipping gate. Native GUI, physical devices, larger/longer scenes and matched reference behavior remain unverified; no family parity is promoted.

**0.1.11 whole-part transform foundation:** shared baked translation, fixed-world X/Y/Z rotation and positive uniform scale move retained levels, hidden/faded layers and Morph Targets together, preserving attributes and inactive objects. A shader-only translation gizmo preview commits once on release. The final source passes **282 tests** (247 core, 35 app) and all seven required source/build checks. Eight frozen-browser workflows and 43 behavior gates pass, with exact visible buffers/save/history and bounded short release observations. [TRANSFORMS.md](TRANSFORMS.md) and its [receipts](evidence/transforms-build.json) retain numerical errors, the corrected coarse-level no-op redo regression, failed immediate-sampling checks, and unfinished masked/local/multi-object, pivot, deformation and reference gates. Native GUI, physical devices, large-scene latency and ZBrush equivalence remain unverified; no family parity is promoted.

**0.1.12 mask extraction foundation:** linear mask clipping, welded edge cuts and positive outward thickness create a new plain, unmasked SubTool while retaining the complete source and whole-scene undo. The final source passes **323 tests** (285 core, 38 app) and all seven required source/build checks. Eight frozen-browser workflows and 53 behavior gates pass, covering analytic geometry/volume, manifold holes and disconnected regions, exact visible buffers/save/history, rich source preservation and atomic rejection. [MASK_EXTRACTION.md](MASK_EXTRACTION.md) and its [receipts](evidence/mask-extract-build.json) retain exact parameters, numerical errors and failed fixture/helper checks. Preview/Accept, smoothing, visibility/inward/symmetric extraction, Polygroups, broader base-mesh tools, global intersections, native GUI and reference behavior remain absent or unverified; no T12 family parity is promoted.

**0.1.13 mask filter foundation:** Grow, Shrink, Blur and Sharpen operate on the current active mask with one useful-edit undo step. Grow/Shrink use unique indexed-neighbor max/min, Blur uses their uniform mean, and Sharpen increases pointwise contrast around 0.5. These are independent scalar policies: the reference guide describes softer Grow/Shrink edges, and no matched ZBrush filtering fixture has been executed. The final source passes **361 tests** (320 core, 41 app) and all seven required checks; eight frozen-browser workflows and 45 behavior gates pass, with exact visible buffers/save/history. [MASK_FILTERS.md](MASK_FILTERS.md) and its [build receipt](evidence/mask-filter-build.json) record the controls, exact math, numerical results, retained failures and remaining masking scope. Screen and feature selection, visibility workflows, adjustment curves and reference kernels remain gaps; no S06 family parity is promoted.

**0.1.14 region mask foundation:** explicit Rectangle/Lasso modes select projected active-part vertices through depth, with a captured view and cancellable outline before a single useful-edit mask transaction. Rectangle bounds are inclusive; Lasso uses even-odd inclusion and a declared boundary tolerance with point/work limits. The final source passes **422 tests** (365 core, 57 app) and all seven required checks; eight frozen-browser workflows and 59 behavior gates pass with exact mask bits, visible buffers, saves and history. [MASK_REGIONS.md](MASK_REGIONS.md) and its [build receipt](evidence/mask-region-build.json) define OBrush's independent rules and retain failed checks. Alphas, soft edges, occlusion/front-face modes, reference gestures, feature/color/group/UV masks and visibility workflows remain gaps; no S06 family parity is promoted.

## Topology and scene roadmap

**0.1.15–0.1.16 SculptGL checkpoint work:** additional brush/input/camera/scene and STL/PLY workflows are recorded in [the host candidates](SCULPTGL_HOST_CANDIDATE1.md) and [navigation follow-up](SCULPTGL_HOST_CANDIDATE2.md). The newer [0.1.16 candidate](SCULPTGL_CORE_CANDIDATE2.md) passes 785 workspace tests and required native/WASM checks, adds bounded material/corner-UV data, independent material painting and source-static triangle/quad stencils, and fixes a retained actual OBJ UV-return failure. Scoped [Paint/static browser checks](SCULPTGL_PAINT_STATIC_CANDIDATE2.md) pass named channel/history/UV workflows while retaining an Escape frame-transition timing failure. Source brush response, tangent-frame hierarchy behavior, editable SGL, native GUI and matched performance remain distinct open gates. The complete [317-ID source inventory](SCULPTGL_PARITY.md) stays frozen and is not a parity percentage or completion claim. No ZBrush family below is promoted by these results.

**0.1.17 foundation:** [The checked report](SCULPTGL_FOUNDATIONS_0_1_17.md) records canonical source-style mask persistence, Fill/picker transactions, retained level deletion, per-tool settings and UV/curvature/opacity/PNG controls. Candidate2 passes949 workspace tests and required builds; named browser cases retain complete saved/history/GPU data, with130 independent buffer reconstructions. Initial helper, clipped-control and notice-overlay failures remain preserved, with separate corrected/completion checks. These results close only their local cases. Continuous/source Paint, editable SGL/empty scenes, full rendering/baking and source/native/performance gates remain open; no family receives full parity status.

**0.1.20 Source pick work:** [The C66 report](SOURCE_PICK_ACCELERATION_C66.md) records a retained current-face candidate path for ordinary Source primary picking, exact local indexed/full-scan comparisons, complete browser saves and history, and the original 10K/100K/999,002-vertex sustained cohorts. The million-vertex 60/120-second runs still fail four responsiveness gates; the 100K/60-second first trial failed one pacing maximum and its repeat passed. Mirrored picking, the whole Source brush-response inventory, native GUI/physical input and the full SculptGL 317+5 workflow comparison remain open. These results do not promote a family to parity.

**C67 face-row follow-up:** [The C67 report](RETAINED_FACE_ROWS_C67.md) records a bounded first-triangle-row cache that removes the remaining original-face scan from verified retained primary ray picking. Exact mixed triangle/quad and dense-sphere scalar comparisons, 2,350 workspace tests and scoped browser brush checks pass. The frozen 999,002-vertex/60-second browser cohort completes public data/history checks but fails the same four responsiveness gates as C66 (CPU frame P95 106.9 ms; independent RAF P95 107.7 ms). The full validation/state scan and mirrored path remain open; no SculptGL or ZBrush parity status changes.

**C68 Source allocation follow-up:** [The C68 report](SPARSE_SOURCE_C68.md) records sparse candidate rows in place of one whole-position proposal clone per useful dab. Native old/new complete project outputs match on the million-vertex five-dab fixture and 11 tool/guard cases; workspace and scoped browser checks pass. The frozen 1M/60s browser cohort still fails four responsiveness gates (CPU frame P95 108.8 ms). No controlled real-time speedup or reference parity follows.

**C69 Source guard follow-up:** [The C69 report](SPARSE_SOURCE_GUARD_C69.md) records sparse fold-guard proposals and rescaling without another full-position candidate clone. The frozen complete million-vertex project and all 11 tool/control projects remain byte-identical; workspace, official builds and scoped browser brush checks pass. The 1M/60s browser cohort still fails four responsiveness gates (CPU frame P95 103.2 ms). Different delivered contacts preclude a controlled speed claim. WebGPU production integration and reference parity remain open.

**C70 signed extraction follow-up:** [The C70 report](SIGNED_MASK_EXTRACTION_C70.md) records zero and inward mask extraction and a visible −5..5 world-unit thickness control. Shared Rust core and owned-browser save/GPU/history checks pass on independent sheet and sphere fixtures. The pinned SculptGL extractor still uses whole incident faces and three border-smoothing passes for nonzero thickness, whereas OBrush clips the selected patch; no extraction-family parity is promoted.

**C71 extraction selection follow-up:** [The C71 report](WHOLE_FACE_EXTRACTION_C71.md) records an optional whole-incident-face selection path in the shared core and browser inspector. The independently authored sheet now exposes both clipped 5/3 and whole-face 4/2 patches at threshold 0.5; signed shell, source-preservation, save/GPU and history checks pass. SculptGL's offset, bridge and smoothing response and matched reference runtime remain open. Browser sculpting still uses WASM CPU geometry with a WebGL2 viewport; WebGPU is not integrated into the production path, and extraction selection does not change the failed million-vertex responsiveness gates.

**C72 static-upload investigation:** [The retained C72 report](STATIC_UPLOAD_TRIAL_C72.md) records two rejected exact-copy WebGL buffer-reuse candidates. Both removed repeated 24 MB index and 8 MB material uploads on the complete million-vertex 60-second workload, but active CPU-frame P95 was 108.0/111.3 ms versus 97.3 ms in a restored C71 replay on the same Chrome version. Delivered contacts differed, so no controlled speed ratio is claimed; both candidates were removed. The final served WASM is byte-identical to C71 and the SculptGL responsiveness/parity gates remain open.

**C73 extraction selection follow-up:** [The C73 report](KEEP_SOURCE_EXTRACTION_C73.md) adds an option to append the extracted part while leaving the source active. Shared-core history and an 11-case normal browser workflow pass, with a signed native package. This matches only SculptGL's source-active selection step; its offset, bridge, smoothing and extracted-channel policy remain unmatched. The browser remains WASM CPU geometry plus WebGL2 rendering, and the SculptGL responsiveness/parity gates remain open.

**C74 paired extraction follow-up:** [The C74 report](SCULPTGL_EXTRACTION_REFERENCE_C74.md) records six actual SculptGL outputs from independent open-sheet and closed-cube OBJ fixtures and a shared-core, opt-in OBrush offset/fresh-channel path. The zero-thickness open sheet has exact unordered point positions; the closed cube already differs by 0.003382 units at zero thickness and nonzero shells differ substantially. The reference open-sheet nonzero output has collapsed/open geometry, which OBrush's checked manifold shell deliberately does not reproduce. Full SculptGL extraction, million-vertex responsiveness and ZBrush parity remain open.

**C75 source-normal follow-up:** [The C75 report](SCULPTGL_EXTRACTION_NORMALS_C75.md) records face-averaged source normals in the opt-in extraction mode. Both zero-thickness sheet and cube fixtures now match pinned SculptGL on unordered point positions exactly. A four-case instrumented reference diagnostic independently replays its three border-smoothing passes with zero coordinate error, but smoothing is not integrated into OBrush; the nonzero shells and SculptGL parity remain open. The current browser still uses WASM CPU geometry and WebGL2 drawing; the separate WebGPU prototypes do not change production responsiveness.

**C76 closed-source extraction follow-up:** [The C76 report](SCULPTGL_EXTRACTION_SMOOTHING_C76.md) integrates three logical-quad-neighbor border-smoothing passes for the optional closed-source extraction path. Both signed closed-cube outputs now lie within `9.22e-8` world units of pinned SculptGL on unordered points; an owned browser run passes 13 extraction/filter cases with visible controls, history and actual GPU-buffer checks. The open-sheet reference collapses triangles and remains intentionally different, while arbitrary surfaces, complex masks, million-vertex responsiveness and full SculptGL/ZBrush parity remain open.

**C77 original-quad extraction follow-up:** [The C77 report](SCULPTGL_QUAD_EXTRACTION_C77.md) allows extraction from sources retaining original quads and UVs without changing the source, and whole-face mode admits both triangles of a selected quad. The independently authored masked quad cube matches pinned SculptGL on unordered zero-thickness points exactly. Its nonzero reference outputs have ten open edges; OBrush keeps a closed shell and does not claim those shapes match. The owned browser passed 14 cases including all three quad thicknesses, source UV retention, history and GPU-buffer checks. Mixed topology, arbitrary masks, sustained performance and full SculptGL/ZBrush parity remain open.

**C78 mixed-face extraction follow-up:** [The C78 report](SCULPTGL_MIXED_EXTRACTION_C78.md) pairs a five-quad/two-triangle closed source through the pinned SculptGL controls and OBrush. The single masked vertex selects its two incident quads and only one split triangle. Zero-thickness unordered points and counts match exactly; nonzero SculptGL outputs retain eight open edges, while OBrush retains a closed shell. This fixture does not establish general mixed-surface, responsive sculpting or complete parity.

**C79 posed-source extraction follow-up:** [The C79 report](SCULPTGL_AFFINE_EXTRACTION_C79.md) allows a version 7 posed source to create a separate world-space part while retaining the source's local mesh, editable affine, SGL origin and history. The paired translated triangle matches pinned SculptGL on zero-thickness unordered positions exactly. At nonzero thickness, the reference output has collapsed faces and six open edges, while OBrush keeps a checked closed shell. Reflected-pose and f32-collapse cases have independent core checks; broader transformed surfaces and SculptGL/ZBrush parity remain open.

**C80 mask-filter follow-up:** [The C80 report](SCULPTGL_MASK_FILTERS_C80.md) adds an opt-in SculptGL response for Mask Blur and Sharpen in the shared core and browser UI. On one independently authored paired cube, eight Sharpen weights match the pinned hosted reference exactly and Blur differs by at most one f32 rounding step after protection conversion. Boundary-quads have a separate analytic test; general topology, long-session behavior and SculptGL/ZBrush parity remain open.

**C81 open-quad mask comparison:** [The C81 report](SCULPTGL_MASK_FILTERS_C81.md) pairs that response with pinned SculptGL on an independently authored open 3×3 quad grid, extending the boundary check from an analytic test to a browser reference. Nine Blur weights differ by at most `2.98e-8` in unprotected weight and nine Sharpen weights match exactly after mapping by source position. Both source controls and OBrush browser controls were exercised; arbitrary topology, native GUI, sustained performance and SculptGL/ZBrush parity remain open.

**C82 Block remesh follow-up:** [The C82 report](SCULPTGL_BLOCK_REMESH_C82.md) adds a separate Block remesh extraction path and visible control. On one paired cube at resolution 64, all 24,578 world-space points and 49,152 oriented triangles match the pinned hosted reference exactly. An independent sphere remains different by 22 vertices and 44 triangles, with measured nearest-vertex distances and volume differences; arbitrary remesh parity and the sustained browser performance gate remain open.

**C83 Block-grid correction:** [The C83 report](SCULPTGL_BLOCK_GRID_C83.md) retains the paired cube match and brings the independent sphere at resolution 32 into exact point and oriented-triangle agreement with pinned SculptGL after its recorded mesh transform. The shared Rust core and visible browser save match for both fixtures. This does not cover arbitrary sources, selected fusion, attributes, Marching Cubes, sustained responsiveness or complete SculptGL/ZBrush parity.

**C84 smooth Surface Nets follow-up:** [The C84 report](SCULPTGL_SMOOTH_SURFACE_NETS_C84.md) adds an optional shared-core/browser Smooth Surface Nets path alongside the original triangle remesher and Block mode. Cube and sphere cell topology and oriented triangles match the pinned hosted reference after quad splitting; maximum world-position differences in the browser saves are `1.446e-7` and `1.236e-7` respectively. Quad persistence, UV/material transfer, multi-mesh fusion, open/arbitrary shapes, the separate Marching Cubes workflow and sustained responsiveness remain open.

**C85 quad persistence follow-up:** [The C85 report](SCULPTGL_SURFACE_NETS_QUADS_C85.md) retains Smooth Surface Nets and Block quads in the shared core, `.obrush` projects and OBJ. On the paired cube and sphere, all 24,576 and 4,744 oriented reference quads match after cell mapping, with the same sub-`1.5e-7` browser position errors. Visible browser save, OBJ export/import, GPU rendering and history passed on both, plus a Block sphere regression. OBJ requires explicit allowance for omitted material channels. UV/material transfer, arbitrary shapes, fusion, Marching Cubes, post-remesh adaptive-detail sculpting and sustained performance remain open; this does not establish the full SculptGL remeshing family.

**C86 sculptable quad-level follow-up:** [The C86 report](SCULPTGL_REMESH_SUBDIVIDE_C86.md) pairs a resolution-16 sphere's Smooth Surface Nets then one static smooth subdivision against the pinned hosted reference. Both produce 4,690 vertices and 4,688 oriented quads; all mapped face sets agree and maximum world-position difference is `2.413e-7`. The OBrush browser subsequently changed 72 fine-level vertices with Draw and preserved exact coarse/fine navigation, quad faces and history. The resolution-32 OBrush continuation also passed locally, but two reference-browser attempts timed out after Subdivide, so larger-size reference output/performance remain unknown. Static retained subdivision does not close adaptive-detail, remesh or full parity gates.

**C87 Marching Cubes follow-up:** [The C87 report](SCULPTGL_MARCHING_CUBES_C87.md) adds a separate shared-core/browser triangular voxel-remesh action and Relax topology control. On an independent resolution-16 plain sphere, all 1,172 mapped vertices and 2,340 oriented triangles match the pinned SculptGL reference within `2.893e-7` world units with relaxation on; the off run matches within `2.315e-7`. The isolated browser passed visible controls, closedness, `.obrush` history/GPU consistency and OBJ triangle export/import. A rich `#MAT` source still differs after relaxation (maximum `0.029677` world units) because OBrush does not import that per-vertex unmasked-weight channel. Selected-mesh fusion, other shapes/parameters and sustained performance remain open.

**C88 rich Marching Cubes follow-up:** [The C88 report](SCULPTGL_MARCHING_CUBES_ATTRIBUTES_C88.md) adds bounded `#MAT` import and voxel unmasked-weight transport to the shared Rust path. On the same rich sphere with relaxation on, the maximum/RMS mapped world-position differences fall to `0.0006440559` / `0.0000243014`; 1,161 of 1,172 vertices are within `1e-6`. Eleven vertices still differ, so this is a narrower improvement, not complete rich-attribute or remeshing parity. The normal browser remains WASM CPU geometry plus WebGL2 drawing; WebGPU compute-and-draw integration and the failed sustained million-vertex responsiveness gates remain open.

**C89 matched-source Marching Cubes follow-up:** [The C89 report](SCULPTGL_MARCHING_CUBES_ORDER_C89.md) corrects the earlier comparison's UV mismatch by running the exact same no-UV, rich `#MAT` OBJ in both apps. OBrush computes SculptGL's no-UV import face order for voxel mask sampling without changing the document. On this resolution-16 sphere, all 1,172 native-core unmasked weights and 2,340 oriented triangles match, while Relax-on positions differ by at most `3.341e-7` world units; the browser save differs by at most `3.276e-7` in position and `1.35e-8` in unmasked weight. Retained-UV remesh, material/color voxel transfer, multi-mesh fusion, broader topology and sustained responsiveness remain open. WebGPU is still a separate prototype, not the delivered compute/draw backend.

**C90 UV-source Marching Cubes follow-up:** [The C90 report](SCULPTGL_MARCHING_CUBES_UV_C90.md) pairs an independently authored rich OBJ with the same per-corner UV bytes in both apps. Marching Cubes rejects UV loss by default; an explicit control permits an undoable discard. On this one resolution-16 sphere, all 2,340 oriented triangles and all 1,172 native unmasked weights match the pinned SculptGL run, with Relax-on positions within `3.341e-7` world units. An isolated browser run verified the default rejection, explicit control, undo/redo, OBJ exchange and visible GPU buffers. UV-preserving remesh, voxel material/color transfer, multi-mesh fusion, broader topology and sustained performance remain open; no family parity is promoted.

**C91 voxel-channel Marching Cubes follow-up:** [The C91 report](SCULPTGL_MARCHING_CUBES_CHANNELS_C91.md) moves RGB, roughness and metallic transfer for Marching Cubes onto the same sampled voxel field as unmasked weight. Against the same-byte rich UV sphere at resolution 16, all 3,516 RGB, 2,344 roughness/metallic and 1,172 unmasked native values match the pinned SculptGL output exactly after vertex mapping, with all 2,340 oriented triangles still matching. This narrow comparison does not verify arbitrary topology, multi-mesh fusion, UV-preserving remesh or sustained responsiveness; no remeshing family parity is promoted.

**C92 selected-mesh Marching Cubes follow-up:** [The C92 report](SCULPTGL_MARCHING_CUBES_FUSION_C92.md) records a one-transaction fused remesh of selected SubTools. On two independently authored overlapping colored boxes at resolution 16, both native core and isolated browser saves match all 2,108 pinned SculptGL oriented triangles after vertex mapping. Native RGB, material and unmasked values match exactly; the browser-save maximum world-position difference is `3.246e-7` with Relax on. Visible selection, save/GPU agreement and exact undo/redo passed. This remains a one-pair fixture: arbitrary selection, mixed UVs, affine/source-origin parts, sustained responsiveness and full remeshing parity are open. The delivered browser remains WASM CPU geometry plus WebGL2 drawing; WebGPU compute/draw prototypes are separate.

**C93 selected-mesh Surface Nets follow-up:** [The C93 report](SCULPTGL_SELECTED_SURFACE_NETS_C93.md) extends selected fusion to Smooth Surface Nets and Block. On the same independently authored overlapping-box pair at resolution 16, each mode matches all 1,056 pinned SculptGL oriented quads after vertex mapping. Moving color/material/mask transfer to the shared voxel field resolves a maximum `1.0` RGB overlap error: all mapped native values now match exactly. Visible browser fusion, GPU/save correspondence, undo/redo and existing one-mesh workflows pass. This is a paired fixture, not complete remeshing or sustained-performance parity; production WebGPU integration remains open.

**C94 mixed-UV selected-remesh follow-up:** [The C94 report](SCULPTGL_MIXED_UV_FUSION_C94.md) allows explicit UV discard when selected textured and untextured meshes are fused. The original UV-bearing sources remain available for voxel sampling, while a temporary UV-free concatenation avoids inventing coordinates. On an independently authored overlapping-box pair at resolution 16, the browser's Marching Cubes output maps all 2,108 oriented triangles to pinned SculptGL with a maximum Relax-on position difference of `3.246e-7` world units. The browser's default rejection, consent, history and GPU/save checks pass; shared-core tests also cover Surface Nets and Block. UV-preserving fusion, arbitrary meshes and sustained responsiveness remain open. WASM CPU geometry plus WebGL2 remains the delivered browser path; WebGPU prototypes are separate.

**C95 mixed-UV Surface Nets/Block comparison:** [The C95 report](SCULPTGL_MIXED_UV_NETS_BLOCK_C95.md) pairs the same selected textured/untextured boxes against pinned SculptGL in both quad remesh modes. Each browser output maps all 1,056 oriented reference quads; maximum world-position differences are `1.462e-7` for Smooth Surface Nets and `1.891e-7` for Block. Browser rejection before UV-loss consent, save/GPU correspondence and undo/redo pass. One concurrent Block browser launch failed before the workflow (`fetch failed`); its isolated serial rerun passed, and both receipts are retained. Arbitrary UV seams, selected transforms and sustained responsiveness remain open.

**C96 seamed/masked selected Marching Cubes comparison:** [The C96 report](SCULPTGL_SEAMED_MASK_FUSION_C96.md) adds a real split UV corner and nonuniform unmasked weights to one selected box. With explicit UV loss at resolution 16, the browser output again maps all 2,108 oriented pinned-reference triangles; maximum Relax-on position and unmasked-weight differences are `3.246e-7` world units and `9.164e-8`. Visible rejection/consent, GPU/save and exact undo/redo pass. This one seam and mask pattern does not establish arbitrary UV/attribute behavior, selected transforms, full remeshing parity or sustained responsiveness.

**C97 selected affine/source-origin Marching Cubes comparison:** [The C97 report](SCULPTGL_POSED_FUSION_C97.md) adds explicit pose/provenance-loss consent and undoable world-space pose baking for selected remesh. An independent SGL pair with one sheared and one translated box maps all 2,064 oriented pinned-reference triangles after browser Marching Cubes at resolution 16; the Relax-on maximum position difference is `2.937e-7` world units. Unmasked weights match and material channels differ by at most `2.384e-8`, but overlapping red/blue color transfer remains **unmatched** (maximum RGB difference `1.0`, RMS `0.3311`). Browser rejection/consent, GPU/save and history gates pass. This is scoped posed-geometry progress, not attribute, arbitrary-transform, full remeshing or sustained-performance parity.

**C98 posed-source voxel-bound correction:** [The C98 report](SCULPTGL_POSED_VOXEL_BOUNDS_C98.md) carries unrounded world-space bounds into selected remesh before storing posed vertices as `f32`. On the same independent pair, all 2,064 oriented triangles still map in both Relax states; native sampled RGB, material and unmasked values match pinned SculptGL exactly. The browser-save maximum mapped RGB difference is now `2.930e-8` instead of C97's `1.0`, with maximum Relax-on position difference `3.134e-7` world units. Visible rejection/consent, GPU/save and history checks pass. This fixes one overlap fixture; arbitrary meshes, all remeshing modes, native GUI and sustained responsiveness remain open.

**C99 posed selected quad-remesh comparison:** [The C99 report](SCULPTGL_POSED_QUAD_FUSION_C99.md) pairs that sheared/translated SGL selection against pinned SculptGL in Smooth Surface Nets and Block at resolution 16. Both native core and browser save map all 1,034 oriented reference quads in each mode. Native RGB/material/unmasked channels match exactly; browser-save RGB differs by at most `2.930e-8`, with maximum position differences `1.613e-7` for Nets and `1.355e-7` for Block. Visible pose-loss consent, source metadata removal, GPU/save and history checks pass. Arbitrary posed meshes, UV combinations, many-part fusion and sustained responsiveness remain open.

**C100 paired UV subdivision comparison:** [The C100 report](SCULPTGL_UV_SUBDIVISION_C100.md) compares the same independently authored all-seamed tetra, quad cube and mixed pyramid OBJ files in pinned SculptGL and the delivered OBrush browser. After one static smooth level, all 16/24/20 child faces and 48/96/64 UV-mapped corners agree after physical-corner mapping; maximum child UV component error is `2.6703e-8`. Native Rust and browser WASM arrays agree exactly as `f32` words/indices; browser save/GPU, history and level controls pass. The reference rewrites coarse positions when lowering, while OBrush retains the original coarse mesh, so complete static multiresolution navigation and texture-pixel parity remain open.

**C101 versioned static navigation and OBJ channel correction:** [The C101 report](SCULPTGL_UV_NAVIGATION_C101.md) changes newly created studio static levels to copy fine even rows into the coarse mesh on Lower, matching the pinned reference's first analysis step while keeping older serialized `v1` hierarchies on their previous path. On C100's same three OBJ fixtures, the maximum mapped lowered position difference is `1.7954e-8` and maximum UV difference is `2.6703e-8`; Higher returns within `1.7954e-8` in position. Missing OBJ vertex colors now import as the reference's white: colorless-fixture RGB is exact across input/fine/Lower/Higher; explicit-color cube RGB differs by at most `8.5511e-8`. Normal browser save/GPU/history, a larger two-mode UV-grid workflow, 2,395 workspace tests, strict native/WASM checks and official builds pass. Normal-frame detail synthesis after coarse edits, multiple paired levels, texture pixels and sustained performance remain open.

**C102 controlled coarse-edit diagnostic:** [The C102 report](SCULPTGL_UV_COARSE_EDIT_C102.md) isolates static detail synthesis after one direct coarse vertex edit on the same three OBJ files. Partial subdivision predictions match within `2.9803e-8`, but the mixed-pyramid fine response differs by `0.037433` in displacement under OBrush's current world-offset policy. A diagnostic using source-style face-averaged normals and measured reference neighbor IDs reduces the absolute final-position difference to `2.9341e-8` on that fixture; this oracle is not integrated. A naive normal-frame trial regressed the cube and was removed. Normal browser editing, source ring-order reconstruction and history/performance checks remain open.

**C103 source ring-order diagnostic:** [The C103 report](SCULPTGL_RING_RECONSTRUCTION_C103.md) predicts all 50 pinned fine-vertex first neighbors from three independently authored OBJ inputs and their UV topology, with zero reference mismatches and exact ordered parent/child physical face rows. A separate normal-frame reconstruction using those predicted neighbors puts the largest final-position component error at `5.9605e-8` across the three direct coarse-edit fixtures. This is a Python diagnostic, not a shared-core/browser implementation; normal brush editing, multiple levels, history and sustained performance remain open.

**C104–C105 static normal-frame delivery:** [C104](SCULPTGL_RING_CORE_C104.md) ports the UV-aware ring and raw face-averaged normal frame into bounded shared Rust, matching all 50 pinned fine-neighbor choices and the three controlled coarse-edit outputs within `5.9605e-8`. [C105](SCULPTGL_STATIC_FRAME_C105.md) activates that transport as a new versioned `v3` scheme for newly created static levels, retains `v1`/`v2` project behavior, and records the same maximum product-path geometry difference on the paired fixtures. Scoped rebuilt-browser static and coarse Draw workflows pass save/GPU/history/reload gates. Multiple edited levels, broad topology/UVs, source brush response, native GUI and sustained million-vertex responsiveness remain open; this is not SculptGL or ZBrush subdivision parity.

**C106 chained static-frame follow-up:** [The C106 report](SCULPTGL_TWO_LEVEL_FRAME_C106.md) records a second-level coarse-edit mismatch of up to `0.010241102` on the UV-seamed pyramid, then preserves source child-face order and cyclic orientation through the next ring. The corrected three-fixture second-level outputs differ by at most `5.9605e-8` in position. The retained-source order is saved and validated when lower levels are deleted. Normal brush response, broader subdivision classes and sustained performance remain open.

**C107 three-level static-frame check:** [The C107 report](SCULPTGL_THREE_LEVEL_FRAME_C107.md) compares one direct coarse edit through three levels on the same pinned-reference fixtures, with a maximum third-level position-component difference of `1.7881393e-7` and zero mapped second/third first-neighbor mismatches. The comparator now checks that the same physical coarse vertex was edited. A normal browser Draw stroke on a 1,536-quad UV grid propagates to 4,138 third-level vertices and passes scoped save/GPU/history/reload checks. Paired brush response, arbitrary topology and sustained performance remain open.

**C108 first Reverse level:** [The C108 report](SCULPTGL_REVERSE_C108.md) records a normal browser command and one paired independent four-quad fixture. The recovered coarse points/oriented face and returned fine points/oriented faces match pinned SculptGL exactly on that integer-valued example. Browser save/GPU/history/reload and nonproduct rejection pass. Repeated Reverse, varied imported surfaces/UVs, paired channels, native GUI and sustained performance remain open. The browser still uses WASM CPU geometry and WebGL2 drawing.

**C109 repeated Reverse:** [The C109 report](SCULPTGL_REVERSE_C109.md) records a second normal-browser Reverse on a same-byte nonplanar quad grid in OBrush and pinned SculptGL. Both recover exact 9/4 then 4/1 physical vertices/faces; on return to 25 vertices the oriented faces match and the largest mapped position difference is `7.450580596923828e-9`. OBrush preserves edited finer-level channels, history, save/GPU agreement and reload on its scoped checks. UV-bearing hierarchy rebasing, broad topologies, native GUI and sustained browser performance remain open.

**C110 UV Reverse:** [The C110 report](SCULPTGL_REVERSE_UV_C110.md) records a versioned per-level UV-ID retention path and one matched two-step UV grid. Both browser UIs retain UV coordinates at counts 25 → 9 → 4 and match all intermediate oriented face-corner UV values; returned finest positions differ by at most `7.450580596923828e-9` while OBrush retains its original finest UV IDs/corners. Core malformed-UV import/rejection, browser history/save/GPU/reload and official builds are scoped checks. Arbitrary seams, mixed topology, native GUI and sustained responsiveness remain open.

**C111 seamed UV Reverse:** [The C111 report](SCULPTGL_REVERSE_SEAM_C111.md) fixes a shared-core rejection of a legitimate fine UV alias split and completes two Reverse steps on a same-byte two-chart grid in the normal OBrush browser. Fine UV IDs/corners, history, reload and GPU/save agreement pass. Geometry matches pinned SculptGL at both coarse levels; the returned fine maximum position difference is `7.450580596923828e-9`. The pinned reference's first intermediate UV table and actual OBJ export contain an out-of-range face UV index; one readable corner differs from OBrush by `0.5`, so intermediate seam UV parity remains explicitly unproven. Broad seams, native GUI and sustained browser responsiveness remain open.

**C112 WebGPU workspace bridge:** [The C112 report](WEBGPU_WORKSPACE_C112.md) promotes the independently authored resident compute/pick/draw prototype into a checked Rust workspace crate. On the unchanged million-vertex fixture, current-source native Metal and browser WebGPU agree on six named ray outcomes and complete perspective/orthographic pixels without whole-position readback before draw. The normal OBrush browser still uses WASM CPU geometry and WebGL2; this probe does not close studio brush, interchange, undo or sustained-performance gates.

**C113 document capture:** [The C113 report](WEBGPU_DOCUMENT_CAPTURE_C113.md) adds a bounded, immutable active-subtool snapshot from the normal `Document` to the shared GPU input, retaining exact local-to-world pose and reporting any transient protected-mask f32 roundtrip changes. It does not yet connect the normal studio canvas or move editing/history ownership to the GPU; all browser responsiveness and reference parity gates remain open.

**C114 saved-document WebGPU probe:** [The C114 report](WEBGPU_DOCUMENT_C114.md) checks one transformed versioned `.obrush` project through native Metal and browser WebGPU. Seven world-space rays match the core CPU picker on face ID, browser/native GPU local hit words match, and complete 512×512 browser/native RGBA frames match. This is an isolated single-subtool canvas; normal studio rendering/brush/history, transformed lighting, multi-subtool composition and sustained performance remain open.

**C115 affine-normal GPU probe:** [The C115 report](WEBGPU_AFFINE_NORMALS_C115.md) applies the saved object's inverse-transpose normal matrix in the isolated GPU shader. A nonplanar reflected/sheared project changes 23,120 displayed pixels compared with the old local-normal lighting, while native Metal and browser WebGPU still match every pixel and seven picks. This is scoped correctness for the separate GPU canvas, not normal-studio WebGPU integration, full appearance parity or a responsiveness result.

**C116 resident-result document boundary:** [The C116 report](WEBGPU_DOCUMENT_COMMIT_C116.md) adds a shared-core, checked acceptance path for an explicitly read-back GPU position buffer. A native Metal point pass changed seven vertices on a 162-vertex sphere; the accepted mesh can be undone and redone with exact saved JSON recovery. Stale mesh/pose/selection, protected changes, invalid bytes, folds and a redo branch reject before mutation. This is still a diagnostic point program; the normal browser studio does not use GPU sculpting and its SculptGL responsiveness gate remains open.

**C117 browser WebGPU/WASM document handoff:** [The C117 report](WEBGPU_BROWSER_COMMIT_C117.md) runs that same checked result through browser WebGPU and a separate WASM document authority. On the same independently generated sphere project, browser/native GPU position bytes and accepted `.obrush` JSON match exactly, browser Undo/Redo restores exact project strings, and the visible canvas changes. It remains an isolated diagnostic canvas with a full position readback, not the normal studio or a SculptGL brush/performance parity result.

**C118 sparse GPU-result boundary:** [The C118 report](WEBGPU_SPARSE_COMMIT_C118.md) adds a shared-core/WASM sparse vertex-result transaction. Seven changed vertices produce the same complete project and history through 140 record bytes as the full 2,592-byte result. The isolated browser still reads back all GPU positions before constructing those records; normal-studio WebGPU integration and measured transfer/latency improvement remain open.

**C119 GPU-owned compact readback:** [The C119 report](WEBGPU_COMPACT_READBACK_C119.md) moves changed-row selection onto GPU compute. One isolated sphere point pass maps 140 changed-row bytes plus bounded count/failure words, with no full-position readback; native Metal and browser WebGPU match on sparse output and complete accepted project. This is not yet a normal-studio brush, sustained responsiveness result or SculptGL parity claim.

**C120 million-vertex compact capacity:** [The C120 report](WEBGPU_MILLION_SPARSE_C120.md) checks the unchanged 999,002-vertex SGL fixture. A single point pass changes 41,065 vertices; native full/sparse data agree exactly, browser/native sparse hashes agree, and the browser maps 821,300 result bytes without a full-position map. The normal studio's failed sustained million-vertex latency gates and complete SculptGL feature inventory remain open.

**C121 normal studio GPU-result handoff:** [The C121 report](WEBGPU_STUDIO_HANDOFF_C121.md) connects a separate offscreen WebGPU point pass to the actual browser studio document under a one-use capture token. A 40,962-vertex startup sphere changes through the validated sparse core commit; the visible WebGL viewport redraws, and ordinary keyboard Undo/Redo exactly recovers its before/after sphere pixels. Normal pointer brushes, picking and drawing still use CPU/WASM plus WebGL2, and no sustained speed or SculptGL brush-response parity is established.

**C122 normal-studio grid control:** [The C122 report](SCULPTGL_GRID_C122.md) implements the source-style fixed XZ scene grid in the ordinary native/WebGL2 renderer, wires its localized checkbox and SGL header import/export flag, and confirms exact browser off/on/off viewport recovery with unchanged saved project. Hosted SculptGL pixel comparison, other cameras/DPR/native GUI, contour and the remaining inventory are still open.

**C123 selected contour:** [The C123 report](SCULPTGL_CONTOUR_C123.md) implements a bounded selected-mesh mask and source-style Sobel edge pass in the ordinary GL/WebGL2 studio, with localized display and RGBA controls plus SGL header flag import/export. Frozen browser off/on/off and full/half/zero-alpha checks show visible contour, exact zero-alpha/off restoration and unchanged saved project. A matched hosted-reference render, multi-part/occlusion/transparent/DPR/native GUI matrix and complete SculptGL inventory remain open.

**C124 paired contour response:** [The C124 report](SCULPTGL_CONTOUR_PAIR_C124.md) compares the same independently authored 10,114-vertex/20,224-face SGL sphere in pinned SculptGL and frozen OBrush browser builds. It corrects the source-scene mask color and places its contour before linear merge/AA. The observed normalized changed-pixel footprint is 4.25258 versus 4.44400 pixels per estimated perimeter pixel (OBRush +4.50%), with exact OBrush project preservation and both browsers error-free. Camera/material/color-space and broader visual/performance gates remain open, so this is not parity promotion.

**C125 overlapping transparent objects:** [The C125 report](SCULPTGL_CONTOUR_MULTIPART_C125.md) compares the pinned source and OBrush on the same independently authored two-part SGL fixture. Both outline only the selected front object and OBrush preserves the exact saved project through off/on/off. At DPR=1, measured contour footprint is 2.0146 vs 2.6023 changed pixels per rectangle-perimeter pixel, leaving a visible thickness mismatch. At process-level DPR=2, both browser contour toggles pass and OBrush allocates its expected 3200×1826 canvas; source and OBrush use different backing resolutions, so physical high-DPI and matched pixel-response coverage remain open.

**C126 SGL Open camera alignment:** [The C126 report](SCULPTGL_SGL_OPEN_CAMERA_C126.md) fixes the ordinary whole-scene SGL Open path's legacy orbit angle and source fit ordering. On the same two-part fixture and matched 1140×825 viewports, the 16-value view matrices match exactly; orthographic zoom differs 0.1417%, and selected-contour changed pixels are 3,821 vs 3,813 with each mask entirely within one pixel of the other. This supersedes C125's unmatched-camera outline-width diagnostic, but pixel colors, previous-scene clip effects, other camera modes and complete parity remain open.

**C127 opaque occlusion contour:** [The C127 report](SCULPTGL_CONTOUR_OCCLUSION_C127.md) pairs an independently authored SGL where the selected rear quad is wholly hidden by an opaque front quad. Both browsers show the selected outline through the front surface; at matched 1140×825 viewport size, source/OBRush off/on responses change 3,208/3,200 pixels and each mask lies within one pixel of the other. Exact pixels, other selection/depth cases and full parity remain open.

**C128 combined multi-selection contour:** [The C128 report](SCULPTGL_CONTOUR_MULTISELECT_C128.md) adds the opaque foreground quad to the selected rear quad in both pinned SculptGL and the normal OBrush browser studio. Each browser changes only the combined outer silhouette: 4,808/4,800 off/on RGB pixels in matched 1140×825 viewport crops, with every changed pixel within one pixel of the other mask. OBrush's normal scene-panel selection and contour checkbox preserve the exact project and restore the off viewport. The source selection used its application method without a physical Shift-click; broader selection-operation, color, high-DPI and full parity gates remain open.

**C129 physical multi-selection contour:** [The C129 report](SCULPTGL_CONTOUR_PHYSICAL_SELECT_C129.md) uses independently authored closed boxes to exercise an actual pinned-SculptGL Shift-click and the normal OBrush scene-panel Ctrl-click. The source click selects the front part without changing any vertices; both browsers' selected-union contour toggles change 5,149 RGB pixels at matched viewport size. All source changed pixels fall within one pixel of OBrush's and all OBrush changed pixels within two pixels of the source's. The source's transient brush cursor is excluded from the contour-only pixel comparison. Selection scope for downstream operations, exact samples, other meshes and full parity remain open.

**C130 selected-group Delete:** [The C130 report](SCULPTGL_SELECTED_DELETE_C130.md) adds an atomic shared-core operation to delete every selected SubTool through the normal browser Delete button. On an independently authored three-box SGL, pinned SculptGL physical Shift-click and OBrush scene-panel Ctrl-click produce the same selected membership before deleting two boxes; both leave the unselected box and support Undo/Redo. OBrush recovers exact pre/post-delete saved project hashes. The source leaves the survivor unselected, while OBrush's nonempty-scene invariant selects it; duplicate, isolate, transform, export and full parity gates remain open.

**C131 selected-group Duplicate:** [The C131 report](SCULPTGL_SELECTED_DUPLICATE_C131.md) copies two selected boxes through the normal browser control in the same selection order observed in pinned SculptGL. Both append two copies, reactivate the last selected original, and remove the copies with two Undos; OBrush's pre/post project bytes recover exactly after Redo. Larger scenes, other selected-group operations, native GUI and full parity remain open.

**C132 selected-group Isolate/Show all:** [The C132 report](SCULPTGL_SELECTED_ISOLATE_C132.md) adds shared-core visibility transactions and normal browser controls for the entire selected group. A pinned SculptGL GUI-handler run and OBrush's visible buttons produce `[false,true,true]` visibility for a two-box selection, restore all three through Show all, and recover their prior states through Undo/Redo. OBrush's saved-project and viewport hashes recover exactly. A second normal browser run physically selects the SculptGL basic shortcut profile and verifies selected-group `I`, `Ctrl+D`, Delete and Undo behavior. Broader visibility/picking gates remain open.

**C133 selected Merge:** [The C133 report](SCULPTGL_SELECTED_MERGE_C133.md) adds a normal Merge selected action and explicit checked pose, mask-rounding and UV-loss policies in the shared core. Pinned SculptGL and normal OBrush browser runs on separated boxes and a posed two-mesh SGL agree on world positions and oriented faces; indexed posed positions and RGB match exactly, with material differences at most `7.152557379708213e-9`. Exact project Undo/Redo and current-build Delete/Duplicate/Isolate/keyboard regressions pass. Other channels, diverse scenes, native GUI, performance and full parity remain open.

| ID | Reference feature family | Status | Acceptance gate before parity can be claimed |
| --- | --- | --- | --- |
| T01 | [DynaMesh](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/dynamesh/dynamesh.html) | Foundation / unverified — independent voxel remesher; healthy-solid checks, damaged-sculpt repair fails | Remesh stretched and intersecting solids at multiple resolutions; verify closed surfaces, controlled detail/volume loss, group handling and undo; compare reference fixtures. |
| T02 | [Sculptris Pro](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/sculptris-pro/sculptris-pro.html) | Foundation / unverified — bounded edge splitting for five dab tools and Move preparation | Refine only the brush region and simplify smooth regions while preserving manifoldness, colors/masks and thin features; measure per-dab latency. Split-only detail does not satisfy the simplification or reference-response gates. |
| T03 | [ZRemesher](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/topology/zremesher/zremesher.html) | Planned | Meet agreed face-count tolerance and quad-quality thresholds on organic/hard-surface fixtures; test guides, groups, symmetry, borders and projection of detail. A voxel remesher or triangle decimator alone is insufficient. |
| T04 | [Multi-resolution and HD geometry](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/subdivision-levels/subdivision-levels.html) | Foundation / unverified — persistent Loop world-offset levels and scoped source-static normal-frame transport; HD absent | Navigate persistent subdivision levels; low-level edits preserve fine detail; project/reconstruct levels; verify border/crease rules and memory-bounded HD editing. |
| T05 | Dynamic subdivision, thickness and MicroPoly | Planned | Preview without modifying the base mesh; vary crease/thickness settings; apply the preview and verify matching geometry and reversible changes. |
| T06 | [SubTools and folders](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/subtools/subtools.html) | Foundation / unverified — lifecycle, whole-part transforms and mask extraction; native GUI, folders/merge/split and broader group actions absent or unverified | Create/select/duplicate/reorder/rename/hide/solo/merge/split/delete independently; folders and scene state survive save/load; undo affects the intended objects. |
| T07 | [ZModeler](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/zmodeler/zmodeler.html) | Planned | Enumerate point/edge/face/curve action-target-modifier combinations; test extrude, inset, bevel, bridge, weld, loops and creases on closed/open fixtures; verify valid topology and replay. |
| T08 | Manual Retopo, legacy Topology Brush and ZSphere topology | Planned | Compare the current Retopo workflow separately from legacy tools: reference surface selection, new/existing SubTool rules, snapped vertex/edge/patch creation, loops, welding and quad topology with declared snapping tolerances. |
| T09 | [Polygroups](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/polygroups/polygroups.html) | Planned | Assign by connectivity, visibility, mask/color and painted region; group IDs survive topology operations and serialization; isolate/restore visibility without losing vertices. |
| T10 | Live Boolean and mesh fusion | Planned | Preview and commit union/difference/intersection on nested, touching and intersecting objects; check volume and manifoldness, retain source objects and report unresolved inputs. |
| T11 | Gizmo, Transpose, pivots and deformers | Foundation / unverified — active whole-part numeric transforms and translation preview; broader workflows absent | Numeric/world/local transforms, multi-object transforms, surface pivots, bend/twist/FFD and masked deformation produce measurable results; units and undo are consistent. Stager must retain per-object Home/Target transforms; Align/Distribute must match view-relative and visible-object/component scopes. |
| T12 | Base mesh creation | Foundation / unverified — signed mask extraction subset; other named workflows remain planned | Test parameterized primitives, QuickMesh quad primitives/groups, mask extraction, ShadowBox, Snapshot3D, ZSketch/skins and Text3D/SVG generation. Check supported text/font behavior, contours/holes, extrusion/bevels, source-editability restrictions and units against the reference; do not infer a permanently editable text object. |
| T13 | Hard-surface workflows | Planned | Independently test clip/trim/knife/slice, planar polish, panel loops, bevel and crease preservation; distinguish displacement-only clipping from topology-cutting operations. BevelPro has a separate PolyGroup-border/mask/preview/apply/Boolean-output and plugin-boundary gate from ZModeler bevels. |

The secondary topology families above come from Maxon's [base mesh guide](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/base-mesh-creation/base-mesh-creation.html) and [desktop user-guide inventory](https://help.maxon.net/zbr/en-us/Content/html/zbrush-online-help.html). Each row must be split into its component commands before it can reach Parity verified.

## Sculpting, painting and procedural roadmap

| ID | Reference feature family | Status | Acceptance gate before parity can be claimed |
| --- | --- | --- | --- |
| S01 | Complete brushes, custom presets and settings | Planned | Inventory all shipped desktop brush types and modifiers; compare golden stroke fixtures with fixed radius, intensity, depth, falloff and surface orientation; round-trip custom brush presets. |
| S02 | [Stroke types and Lazy Mouse](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/strokes/strokes.html) | Foundation / unverified — freehand dab spacing only | Test continuous/dotted/drag/line/spray/freehand/curve and stabilized strokes; spacing is independent of event rate; replay pressure, speed and direction; adjust/interpolate strokes. The 0.1.1 straight-path chunking regression checks OBrush dab spacing/geometry; other stroke types, stabilization and reference response remain unimplemented or unverified. |
| S03 | Alphas, texture brushes and surface noise | Foundation / unverified — bounded PNG grayscale brush weights with rotation and X reflection | Import/create/transform custom alpha images; repeat/rotate/project and bake noise on UV and non-UV surfaces; define bit depth and sampling; match repeat seams and undo. |
| S04 | Curve brushes, IMM and VDM | Planned | Insert single/multiple meshes, manage a custom library, deform along edited curves and stamp vector displacement including overhangs; preserve orientation, scale, welding and reversibility. |
| S05 | Symmetry beyond X | Planned | Test Y/Z/radial/local/dynamic and posable symmetry on declared fixtures; distinguish geometric symmetry from vertex correspondence; preserve reference coordinate behavior. |
| S06 | Full masking and visibility | Foundation / unverified — brush weights, projected rectangle/lasso and independent mask filters | Brush/rectangle/lasso/region, topology/backface/feature masks, grow/shrink/blur/sharpen and mask-by-color/group/UV/draft each receive independent fixtures; combine and hide/isolate reversibly. |
| S07 | Sculpt layers, morph targets, HistoryRecall and Thick Skin | Foundation / unverified — geometry layers and one level-bound geometry Morph Target; other gates remain planned | Track separate gates for 3D Layer geometry/color/mask recording and blending; subdivision-linked Morph Target storage/swap/local restoration with topology invalidation; HistoryRecall/Project UndoHistory; and Thick Skin. OBrush geometry layers record/blend/hide/delete/bake; paint/masks, negative/exaggerated intensity, broader stack operations and per-level recording remain gaps. Existing levels stay stored but layers lock navigation until bake. The Morph Target stores/restores/swaps shape with local masked restoration, origin-level availability, topology invalidation and version4 persistence. Reference response, Project Morph, Create Difference Mesh and other Morph controls remain unverified or absent. Ordinary undo completes none of the remaining gates. |
| S08 | Detail projection, bas relief and bridge effects | Planned | Project source detail through multiple levels without back-surface leakage; create measured bas-relief and curve/slime bridges with controllable parameters; compare reference outputs. |
| P01 | Complete PolyPaint | Planned | Paint and sample RGB/material channels, fill, mask and mirror; verify subdivision color behavior and conversion between vertex colors and texture maps with declared color-space policy. |
| P02 | [UV mapping and UV Master](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/exporting-your-model/uv-mapping/uv-mapping.html) | Planned | Generate/manage UVs, use existing seams, control painting and group constraints, packing/copy/paste/flatten operations; check overlap, distortion and padding and preserve seams through interchange/subdivision. UV Master does not provide manual seam placement. MorphUV sculpt/paint needs separate flatten/restore, correspondence, seam and topology-restriction checks. |
| P03 | Texture, normal, displacement and vector displacement maps | Planned | Bake known surfaces at multiple sizes/bit depths, verify tangent conventions, scale/midpoint, seams and multi-object export; reproduce the surface in an independent renderer. |
| P04 | Spotlight, image planes and projection painting | Planned | Load and transform reference images, control opacity and projection, paint across view changes and texture seams; verify saved placement. Track Projection Master depth/color/material drop/pickup and Drop3D adaptive-geometry/subdivision rules separately from ordinary image projection. |
| G01 | FiberMesh and grooming | Planned | Generate fibers from masked surfaces, groom by multiple brushes, control density/thickness/length and export; track performance and reproducibility. |
| G02 | ArrayMesh, NanoMesh, MicroMesh and instances | Planned | Edit instance sources and transforms, repeat in stages and replace surface polygons; preview/apply; check instance counts, memory and exported geometry. |
| G03 | [Cloth and dynamics](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/cloth-simulation/cloth-simulation.html) | Planned | Drape a pinned patch over collision fixtures, inflate/deflate and manipulate cloth with brushes; measure penetration, stretch, solver stability and reproducibility. |
| G04 | ZSpheres, mannequins, rigging and pose | Planned | Build a branching armature, create a skin, bind a mesh, pose joints and coordinate multiple objects through a proxy; verify topology, weights and deformation through save/load. |

Official feature descriptions: [sculpting](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/sculpting/sculpting.html), [texturing](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/texturing/texturing.html), and [desktop user guide](https://help.maxon.net/zbr/en-us/Content/html/zbrush-online-help.html). The 2026 inventory must include the updated mesh/brush viewer, manual retopology operations, and Python support described by the [What's New guide](https://help.maxon.net/zbr/en-us/Content/html/features/whats-new-in-zbrush/zbrush-whats-new.html).

## Production and platform roadmap

| ID | Reference feature family | Status | Acceptance gate before parity can be claimed |
| --- | --- | --- | --- |
| R01 | Materials, MatCap, lights and cameras | Planned | Create/save materials; assign channels, manipulate lights/environment and reproduce fixed camera shots; verify orthographic/perspective/FOV, color management and material persistence. |
| R02 | BPR and NPR rendering | Planned | Render antialiasing, shadows/AO, transparency, fibers and subsurface effects; export passes and apply filter stacks; compare controlled images and elapsed time. A MatCap/PBR viewport does not complete BPR/NPR/pass gates. |
| R03 | [Redshift integration and AOVs](https://help.maxon.net/zbr/en-us/Content/html/user-guide/materials-lights-rendering/rendering/redshift/redshift.html) | Planned | Inventory material parameters, interactive updates, denoising and AOV outputs; test an explicitly installed renderer integration. Redshift is a separate application; an OBrush renderer is not automatically binary/API compatibility. |
| R04 | Movies, timeline and turntables | Planned | Record strokes/history, animate camera/material/object tracks and export supported movie formats with tested dimensions, frames and timing. |
| R05 | 2.5D illustration, pixol/document workflows and QuickSketch | Planned | Persist depth/color/material in a document, place tools and compose layers; test painting and document/image export independently of the 3D viewport. |
| I01 | Mesh/scene interchange | Planned beyond F14 | Inventory OBJ/STL/PLY/FBX/USD/USDZ and any other formats advertised for the pinned desktop/plugins; use independently authored fixtures for hierarchy, units, normals, groups, UVs, colors and materials; disclose supported/lost fields per format. A format name does not imply full-scene support: the documented USD plugin describes mesh interchange. |
| I02 | Native ZBrush formats | Planned / compatibility unverified | ZTL/ZPR/ZBR/brush/material/alpha presets need separately specified formats and legitimate fixtures. Reading one file is insufficient; test versions, unsupported chunks, round trips and failure recovery. No native-format compatibility is currently claimed. |
| I03 | GoZ, Substance Bridge and other application bridges | Planned | Pin both endpoint versions; transfer geometry, topology updates, shading/textures and return edits without silent data loss; validate each application independently. The 2026.2.1 baseline includes Substance Bridge and updated GoZ endpoints. |
| I04 | 3D-print and scale tools | Planned | Validate units, bounding dimensions, watertightness, thickness/draft analysis and decimation; verify exported dimensions in an independent tool and preservation of thin features. |
| E01 | [ZScript and Python scripting](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/zscript/zscript-python.html) | Planned | Specify ZScript and Python compatibility separately; run supported command sequences, UI/IO/macros and automation with errors, cancellation and replay. The documented initial Python implementation bridges ZScript commands and does not expose unrestricted internal mesh data. |
| E02 | Plugin ecosystem and bundled utilities | Foundation / unverified — manual triangle decimation only; ecosystem compatibility absent | Inventory each utility separately (including Decimation Master, Multi Map Exporter, UV/SubTool/Transpose/Scale masters and PolyGroupIt); test concrete workflows and API contracts. Rust plugins alone do not run existing ZBrush plugins. |
| U01 | Asset browsers and user content | Planned | Search/preview/organize/import custom models, brushes, materials and alphas; distinguish bundled/user libraries; survive missing files and large libraries; ship only appropriately licensed assets. |
| U02 | Custom interface, hotkeys, navigation and accessibility | Planned | Save/load layouts and shortcuts, resolve conflicts, remap navigation, restore defaults; compare desktop workflows including imported hotkeys; test keyboard access, scaling and input-device behavior. |
| U03 | QuickSave, recovery, history and project durability | Planned beyond F13/F15 | Crash/restart restores a valid recent project; versions/migrations are explicit; quota/full-disk/cancelled saves preserve the previous file; timed/idle QuickSave, retained versions and undo-history inclusion are explicit; memory limits and recovery are measured. |
| H01 | Desktop hardware and performance | Planned / benchmarks absent | Run the agreed feature suite on macOS and Windows, including relevant ARM/x86 devices; record CPU/GPU/RAM/backend, mesh size, p50/p95 input-to-display latency, frame time, peak memory and load/save/remesh time. |
| H02 | Browser execution (additional OBrush objective) | Foundation / unverified | Test the same Rust operation fixtures in WebAssembly; exercise supported GPU backends, pen pressure, file upload/download, persistence, resize/context loss and offline packaging on declared browsers. Publish mesh/memory limits and differences. |

Official production references: [rendering](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/rendering/rendering.html), [movies](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/movies/movies.html), [illustration](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/illustration/illustration.html), [import/export](https://help.maxon.net/zbr/en-us/Content/html/features/main-features/import-export/import-export.html), [customization](https://help.maxon.net/zbr/en-us/Content/html/user-guide/customizing-zbrush/customizing-zbrush.html), [system requirements](https://help.maxon.net/zbr/en-us/Content/html/getting-started/system-requirements/system-requirements.html) and [2026.2.1 changes](https://www.maxon.net/en/zbrush/new). Browser support is an OBrush addition, not a claim about ZBrush desktop.

## ZBrush scope beyond the SculptGL checkpoint

Reviewed on 2026-10-12 against the [SculptGL source revision `8e45dafc`](https://github.com/stephomi/sculptgl/tree/8e45dafc0f7906e5238dfffc1a2df742bd25f29d) and primary Maxon documentation. This is a feature-family inventory, not an exhaustive command audit, live comparison or parity result. “Absent” means no corresponding exposed workflow was found in the audited registered tools, GUI, geometry, shader and file modules. Related basics can exist without completing the named ZBrush workflow. The [source audit record](evidence/sculptgl-scope-audit.json) pins sources and the separately retrieved hosted script; its relation to the upstream revision is not established.

| ZBrush capability to cover | SculptGL distinction | OBrush families |
| --- | --- | --- |
| [3D Layers](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/3d-layers/3d-layers.html), [Morph Targets](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/morph-targets/morph-targets.html), HistoryRecall and Thick Skin | Adjustable sculpt/paint/mask layers, stored morph surfaces and these history/thickness tools are absent; ordinary undo and subdivision already exist. | S07 |
| [ZRemesher](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/topology/zremesher/introduction/introduction.html) and [Retopo](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/topology/Retopo/01-retopo.html) | Guided edge-flow rebuilding and snapped manual topology construction are absent. SculptGL supports quads and voxel output; those are not guided retopology. | T03, T08 |
| [ZModeler](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/zmodeler/zmodeler.html), panel loops, knife/trim/slice and BevelPro | Point/edge/face modeling and the complete hard-surface workflow are absent or require distinct tools beyond basic sculpt deformation. | T07, T13 |
| SubTool folders, [Stager](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/stager/stager.html) and [Live Boolean](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/live-boolean/live-boolean.html) | Folder-wide operations, stored Home/Target transforms and a live add/subtract/intersect stack are absent. Multiple meshes, transforms, duplication, isolation and voxel fusion already exist. ZBrush folders are documented as non-nested. | T06, T10, T11 |
| [PolyGroups](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/polygroups/polygroups.html) and advanced masks/selection | Persistent face-group operations and group/UV/draft/thickness-specific workflows go beyond existing masks, blur/sharpen, extraction and isolation. | T09, S06, I04 |
| [IMM](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/insert-mesh/insert-multi-mesh/insert-multi-mesh.html), [VDM](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/vector-displacement-meshes/vector-displacement-meshes.html), curve brushes and richer strokes | Reusable inserted mesh libraries, edited-curve placement and vector stamps with undercuts are absent. Basic raster alphas and continuous strokes already exist. | S01, S02, S04 |
| [HD Geometry](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/subdivision-levels/hd-geometry/hd-geometry.html), dynamic preview, thickness and MicroPoly | Regional HD editing and these preview/thickness/pattern workflows are distinct missing goals; ordinary persistent levels already exist. No performance/capacity comparison is inferred. | T04, T05, H01, H02 |
| [Radial/posable symmetry](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/symmetry/symmetry.html) | Radial repetition and posed vertex-correspondence symmetry are absent. SculptGL already has a local mirror plane and offset. | S05 |
| [UV Master](https://help.maxon.net/zbr/en-us/Content/html/user-guide/zbrush-plugins/uv-master/uv-master.html), [MorphUV](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpt-paint-in-morph-uv/sculpt-paint-in-morph-uv.html) and production map generation | General unwrap/control painting/packing, flattened-UV sculpting and high-to-low normal/displacement/vector-displacement baking are absent. Existing UV display/interchange and color/roughness/metalness texture export are supported; do not label all texture baking absent. | P01, P02, P03 |
| [Spotlight](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/reference-images/spotlight/spotlight.html), NoiseMaker, bas relief, Projection Master and Drop3D | Image-assisted projection, procedural surface/noise controls, relief and pixol drop/pickup workflows go beyond basic imported alphas or background images. | P04, S03, S08, R05 |
| [FiberMesh](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/fibermesh/fibermesh.html) and [cloth](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/cloth-simulation/cloth-simulation.html) | Generated/groomed fibers, cloth collision simulation and cloth brushes are absent. | G01, G03 |
| ZSpheres, ZSketch, mannequins, rigging and posing | Branching skin generation, mesh binding, joint poses and coordinated posing are absent; generic move/transform brushes are related basics. | G04, T12, T11 |
| ArrayMesh, NanoMesh, MicroMesh and QuickMesh/Text3D/SVG | Procedural arrays/surface instances, richer quad primitives and text/vector geometry require separate goals; ordinary duplication and primitive creation already exist. | G02, T05, T12, E02 |
| [BPR](https://help.maxon.net/zbr/en-us/Content/html/user-guide/materials-lights-rendering/rendering/bpr/bpr.html), NPR, material/light systems, [Redshift](https://help.maxon.net/zbr/en-us/Content/html/user-guide/materials-lights-rendering/rendering/redshift/redshift.html) and keyed timelines | Production render passes, renderer integration and keyed scene/presentation animation are absent. SculptGL already has MatCap/PBR viewport rendering and texture/image export. | R01–R04 |
| 2.5D pixol illustration and QuickSketch | A separate depth/color/material-bearing document canvas is absent. A 3D sculpt viewport is a different workflow. | R05 |
| ZTL/ZPR/ZBR, [GoZ](https://help.maxon.net/zbr/en-us/Content/html/user-guide/zbrush-other-programs/goz/about-goz/about-goz.html), Substance Bridge, scripting and plugins | Native formats, versioned application bridges and a compatible ZScript/Python/plugin interface are absent. SculptGL has its own SGL and ordinary mesh formats. Python's current documented scope is a ZScript command bridge. | I01–I03, E01, E02 |
| QuickSave/recovery, custom layouts/hotkeys/assets and print/scale utilities | Reference recovery/version policies, customized studio workflows and named print/scale utilities remain separate goals beyond ordinary saving, shortcuts, libraries and mesh export. | U01–U03, I04, E02 |

SculptGL and ZBrush share foundation categories including brushes, pressure, masks/extraction, mirror symmetry, multiple meshes, subdivision, adaptive subdivision/decimation, voxel remeshing, MatCap/PBR and mesh/UV texture export. Shared labels do not establish matching behavior, output, usability or performance. In particular, DynaMesh and Sculptris Pro remain reference-comparison gates even when a related remesher/adaptive brush exists.

### Separately tracked capability gates

These child IDs refine existing families without changing their implementation status. Unimplemented children remain Planned; 0.1.6 foundations do not promote their siblings or parent to parity. Expand further to actual palette commands and supported parameters before any completion claim.

| Child IDs | Separately required checks |
| --- | --- |
| T04.1 / T04.2 | Ordinary subdivision editing/reconstruction versus regional HD editing, painting, persistence and rendering/baking. Only the named ordinary-level foundation exists. |
| T05.1 / T05.2 / T05.3 | Non-destructive subdivision preview/apply, dynamic thickness, and MicroPoly patterns, each with saved state and undo. |
| T06.1 / T06.2 / T10.1 / T10.2 | Object lifecycle, folder scope, Boolean preview and Boolean commit; retain operands and validate attributes/history separately. |
| T08.1 / T08.2 | Current Retopo vertex/edge/patch workflow versus legacy Topology Brush/ZSphere topology; each gets independent reference fixtures. |
| T11.1 / T11.2 / T11.3 | Gizmo/Transpose/deformer actions, Stager Home/Target transforms, and view-relative Align/Distribute with object/component scopes. |
| T12.1 / T12.2 / T12.3 | Parametric/QuickMesh primitives and groups; extraction/ShadowBox/Snapshot3D/skins; Text3D/SVG with supported fonts/contours and reference editability limits. |
| T13.1 / T13.2 | Hard-surface brush/topology operations versus BevelPro preview/apply/Boolean-output and plugin boundary. |
| S04.1 / S04.2 / S04.3 | Curve placement/editing, InsertMesh/IMM libraries, and VDM creation/application with overhangs/undercuts. |
| S07.1 / S07.2 / S07.3 / S07.4 | Geometry/color/mask layer recording/blending; subdivision-linked morph storage/swap/restore; HistoryRecall/Project UndoHistory; Thick Skin constraints. |
| P02.1 / P02.2 / P03.1–P03.4 | UV generation/control/packing versus MorphUV; color, normal, scalar displacement and vector displacement export, each with explicit conventions. |
| P04.1 / P04.2 / P04.3 | Reference images/Spotlight; Projection Master depth/color/material drop/pickup; Drop3D geometry/subdivision behavior. |
| G02.1 / G02.2 / G02.3 | ArrayMesh staged repetition, NanoMesh surface distributions, MicroMesh replacement; editing/persistence/export each tested separately. |
| G04.1 / G04.2 / G04.3 | ZSphere/skin/ZSketch creation, mannequin/rigging and coordinated multi-object posing. |
| E01.1 / E01.2 | ZScript command/macro compatibility versus Python command-bridge compatibility, with explicit unsupported operations. |
| I04.1 / I04.2 / U03.1 / U03.2 | Scale/print calibration and selected/visible/all export; timed/idle QuickSave/version policy and crash/full-disk recovery. |

The crosswalk covers major families, not every command. Bundled-versus-third-party boundaries, deprecated commands, external endpoint versions and behavior in the installed 2026.2.1 reference remain individually verifiable obligations. No feature family is removed because SculptGL lacks it, and no implementation or parity status is changed by this documentation audit.

## Milestone gates

1. **M0 — runnable foundation:** F01–F16 implemented and locally checked; native and browser demonstrations include edit/undo/save/load. Record unavailable platform checks. Do not relabel this as ZBrush parity.
2. **M1 — scalable sculpting:** spatial acceleration, stroke sampling/pressure/backface rules, persistent subdivision, layers, complete masks, crash recovery and bounded history. Benchmark 10k, 100k and 1M-vertex fixtures before adding higher tiers; these are test sizes, not capacity claims.
3. **M2 — topology and scene:** T01–T13 with manifold/attribute propagation tests and multi-object editing. Compare DynaMesh, adaptive sculpting and quad retopology as separate workflows.
4. **M3 — production assets:** S/P/G families with custom brush/alpha libraries, UV/map baking, procedural geometry, dynamics and posing. Complete interchange fixtures and field-loss reports.
5. **M4 — complete desktop workflows:** R/I/E/U families, legacy document tools, integrations and scripts. Enumerate every palette/command and each supported parameter; resolve bundled-versus-third-party boundaries explicitly rather than silently excluding hard items.
6. **M5 — parity audit:** independent artists run a fixed scenario suite on the pinned ZBrush and OBrush versions; compare behavior, output, persistence, usability and performance on every promised platform. Publish remaining failures; accept 100% only with a complete inventory and no unresolved required gate.

Scheduling and performance budgets remain to be measured; no completion date or resource estimate is asserted by this roadmap. Browser limits must be honest, even if they are lower than native limits.

## Evidence record required for a feature promotion

For every gate, retain the feature ID, source URL/reference version, OBrush commit, fixture/license/hash, parameters, recorded input (coordinates/time/pressure/modifiers), platform/backend, command or reproduction steps, expected result, measured result, tolerance, artifacts, and unresolved differences. Choose numerical tolerances **before** comparing outputs. Prefer independently created sphere, thin sheet, sharp box, noisy patch, colored/UV mesh, overlapping solids and multi-object fixtures.

Local tests establish OBrush invariants. Public documentation establishes documented reference behavior. Only a recorded reference run establishes measured ZBrush behavior. Binary observations provide evidence about a specific executable and do not themselves prove feature parity; see the [REA comparison workflow](REA_WORKFLOW.md).
