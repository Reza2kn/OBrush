# C302: Source Brush same-frame pen samples — frozen gate

Freeze this gate before changing Source-mode input replay. The pinned reference
is SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and the
unmodified local hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independently authored RGB-ID sphere is the 10,114-vertex, 20,224-face
`independent-id-sphere-10k.sgl` fixture SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
Record the exact OBrush source commit, browser WASM hash, harness hash, raw
event trace, public saves, GPU rows, UI settings and limitations in the result.

The candidate corrects only active, non-Continuous, non-Lock, ordinary Source
geometry Brush. It must retain ordered held pen samples and their individual
f64 browser pressure and screen coordinates when two trusted CDP pen moves
arrive before one OBrush app frame. Source Paint, Drag, Continuous and Lock are
outside this change. A valid source stroke begins through the normal viewport
on the common imported fixture. Visible radius and intensity controls are
set to 64 pixels and 25%; both pressure factors are set to intensity 1 and
radius 0, with mirror, culling, alpha, Clay, Accumulate and Prevent folds off.
The browser viewport is 1024×768 CSS pixels at DPR 1 and UI scale 100%.

Two independent runs start with pressure 0.25 and the same first effective
point. The pinned Hammer path presses at `x` then uses `x+12` as panstart;
OBRush presses at `x+12`. Both sides' first useful held move is `x+24`.
After that first useful dab, send held moves at `x+36` and `x+48` before one
OBRush app frame. The case pressures are 0.25 then 0.75; the control pressures
are 0.70 then 0.75. Both end at `x+48` and release pressure 0. Values,
coordinates, event times, isTrusted, buttons, app-frame indices and actual
accepted dab counts must be recorded. The reference uses its actual Hammer
path, including any panstart; do not equate requested moves with useful dabs.
The harness must first demonstrate on each side that both trusted moves were
delivered, and that the reference admitted distinct geometry-bearing moves.

Acceptance for this scoped candidate: on the OBrush side the two cases share
exact before public files, first useful point, pressure and accepted dab count
(do not Save while held, which would end or disturb the stroke); the final public geometry differs
between low→high and near-high, with at least one changed original vertex and
no unexpected color/mask/topology changes. Each case has exact own-format
Before→After→Undo→Redo, active GPU positions matching the saved mesh and a
quiet release tail (32 app frames). The OBrush accepted-dab count must rise
by exactly two during the burst and stay unchanged through release and the
final egui endpoint sample. The reference side must show two admitted
useful moves and a corresponding non-identical public geometry pair. Report
changed ID sets, maximum per-component differences and all full-file hashes;
do not assert matched Source/OBRush final geometry from this batching gate.

Separate negative controls: factor intensity 0 makes each app's two pressure
traces match its own final file exactly; a 257-sample burst must report a
bounded overflow, split or cancel the path, and preserve exact before public
bytes and Undo/Redo depths on cancellation rather than
silently merge all samples into one wrong-pressure dab. The overflow run may
be scoped to OBrush. If the reference does not admit both useful moves,
record the failed paired gate and keep the OBrush same-frame diagnostic as a
separate result. Do not retune event timing or thresholds under this C302 ID.

All input is synthetic trusted browser/CDP pen input, not an actual tablet.
This gate does not qualify native stylus delivery, transformed canvases,
Source Paint, other brushes, arbitrary meshes, exact reference rendering or
complete SculptGL pressure parity. Preserve every failed run and its artifacts.
