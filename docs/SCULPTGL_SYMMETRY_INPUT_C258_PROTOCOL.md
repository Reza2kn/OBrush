# C258: reference input and stroke-admission crossover

C257 failed because a trusted `pen` press and held moves at pressure `0.5`
produced SculptGL's factor-`1` picker radius but no changed vertex. C256 used
the same independent triangle and virtually the same projected path with
`mouse` pressure `0` and did change a vertex. The pinned SculptGL source
attaches both compatibility mouse handlers and Hammer pan handlers; Hammer's
non-mouse `onPanStart`/`onPanMove` can call `onDeviceDown`/`onDeviceMove` in
addition to the mouse route. This is a plausible action-state explanation,
not a demonstrated cause. C258 is a separate read-only diagnostic. It does
not modify C257's failed receipt or claim brush parity.

Use the same pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent `.sgl` fixture SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`,
active triangle, Solo, SymOffset `+0.5`, ordinary Draw with nominal radius
`250`, intensity `0.5`, alpha None, X symmetry and no continuous mode. Use
fresh owned Chrome profiles at 1600×1000/DPR1 for two separate arms and the
same three local points `(3.70,-1.99,0) → (3.845,-1.99,0) →
(3.99,-1.99,0)`. The projected span must exceed `37.5` CSS px; its two-arm
relative difference must be ≤1%. No hidden camera, mesh, tool or Tablet
state mutation is allowed. Use visible import, Solo and brush controls.

Arm P repeats C257's documented `Input.dispatchMouseEvent` `pointerType:
"pen"`, `force: 0.5` held route. Arm M changes only `pointerType` to `"mouse"`
with the same `force: 0.5`; release uses force `0`. Both arms record the
**actual** trusted PointerEvent lifecycle, pressure, buttons and coordinates.
If Chrome does not deliver pressure `0.5` from pointerdown through held
pointermoves for either arm, classify that arm as input setup failure, retain
the receipt and do not infer SculptGL behavior. In particular, do not replace
host events with synthetic DOM events or assign `Tablet.pressure` directly.

Before each stroke, install observation-only wrappers around the **original**
SculptGL instance's `onDeviceDown`, `onDeviceMove`, `onDeviceUp` and
`onPanUpdateNbPointers`, the active
tool's `start`, `sculptStroke`, `makeStroke` and `stroke`, and the primary and
symmetry pickers' `pickVerticesInSphere`. Each wrapper must invoke its
original exactly once with the same receiver, arguments and return value.
Record timestamp and event type/`which`/coordinates, action before and after,
active mesh, `canEdit` return, tool last/current mouse coordinates, spacing
distance and threshold, makeStroke primary/mirror hit, selected vertex IDs,
pressure/radius factor and active vertex positions before/after. Bounded log:
≤500 entries or fail explicitly, never silently truncate. Preserve browser
console errors, screenshots with parked cursor and zero/nonzero WebGL status.

The diagnostic first asks whether the pen arm loses SCULPT_EDIT admission,
fails the `37.5` px spacing gate, misses the face, picks no vertices, or enters
`Brush.stroke` but yields zero vertex displacement. Classify from the trace,
not from the unchanged screenshot. If the mouse arm receives trusted pressure
`0.5`, factor `1`, an admitted dab and at least one nonzero vertex delta while
P fails, report the host-route difference and use **a later separately
preregistered paired gate** for mouse-pressure OBrush comparison. If both arms
fail, the cause is not isolated; retain both receipts and investigate the
shared selection/brush route before another pair. If both deform, compare
their exact vertex deltas and action traces, but do not call them OBrush parity.

Report per-arm full source/build/harness hashes, exact camera/path/radius,
every vertex delta, action/admission counts and failure sites. Preserve all
failed attempts. No OBrush product edit, ledger row closure or SculptGL
parity assertion follows from C258 alone. Do not run this browser gate until
the shared Chrome lane is explicitly released.
