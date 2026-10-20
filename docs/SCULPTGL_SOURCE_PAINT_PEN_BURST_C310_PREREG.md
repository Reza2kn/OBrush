# C310 Source Paint ordered pen burst — frozen candidate gate

This is a preregistration, not a result or a parity claim. The pinned reference
is SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted
bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Reuse the independently authored RGB-ID 10,114-vertex, 20,224-triangle SGL
sphere from C302, SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
It is a common imported fixture, not a copied commercial asset. Record its
actual file path, bytes, hash, OBrush source/build hashes, harness hash and
all failed attempts in the result.

Scope: ordinary, non-Lock, non-Continuous **Source Paint** in the normal
browser UI. Preserve the existing mouse, idle-hover, default-pressure,
Legacy Paint, Lock, Continuous, geometry Brush and navigation routes. Keep
the source's captured f64 browser pressure and screen coordinates with each
ordered held pen move; no pressure value from the final egui pointer state may
be substituted for an earlier move. Use the existing 256-event bound. On a
257-move same-frame overflow cancel the whole Source Paint gesture atomically,
including previously applied paint, pending work, color display state and its
open history snapshot. Do not silently coalesce or commit a partial stroke.

Use the same 1024×768 CSS viewport, DPR 1 and 100% UI scale for each side.
Set normal Paint's visible RGB swatch to an authored non-default color and
record its exact UI value. Use a 64-pixel radius and 25% intensity, with
radius pressure factor 0 and intensity pressure factor 1. Turn mirror,
alpha, material channels, Lock and Continuous off. Use the same view, first
effective contact and geometry-bearing screen path in four independent
fresh-import runs: two trusted held pen moves before one OBrush app frame
with pressures 0.25→0.75 and 0.70→0.75; repeat both with intensity factor 0.
Record actual reference Hammer useful moves instead of assuming each CDP
request produces a dab. Capture DOM isTrusted, pointerType, pressure,
coordinates, buttons, event time and app-frame index.

Before running, freeze a no-pressure-shape equality gate and the positive
pressure-difference gate; do not tune settings to a favorable output. The
reference and OBrush must each admit two distinct useful burst moves. On
OBRush, the useful burst count must rise by two, and no release/final egui
sample may add a dab after 32 quiet frames. Within each app, factor 1 must
produce different saved painted RGB values at at least one original vertex
between low→high and near-high. Factor 0 must produce identical own-format
final public files for those two traces. Positions, topology, mask and
unselected paint channels must remain unchanged. Record changed vertex IDs,
per-channel maximum absolute differences and all complete file hashes;
cross-app equality is a separate unclaimed gate. Read active GPU color rows
at Before/After/Undo/Redo and compare with each app's own saved public data.
OBRush Before→After→Undo→Redo must be one exact history step, with stable
GPU/file correspondence and no post-release edits.

Negative controls: a 257-move burst must report an overflow and restore exact
pre-gesture `.obrush` bytes, Undo/Redo depths and paint display state; a
normal mouse stroke at default pressure must match the unmodified baseline
under the same fixture and path. If the reference fails to admit both useful
moves, preserve the failed paired receipt and report only an OBrush input
diagnostic. Synthetic trusted CDP pen events cannot establish physical stylus
delivery, native pressure, arbitrary brushes, transformed canvas behavior,
long-session responsiveness or complete SculptGL/ZBrush parity.
