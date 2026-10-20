# Q02 repeated small-radius Move browser visual gate

**Frozen before browser contact.** Use an owned headless Chrome profile and
loopback server serving the current official root `web/dist` bytes, with the
local and served WASM SHA-256 checked equal. Start from the app's fresh
40,962-vertex sphere at a 1600 × 1000 CSS viewport and DPR 1. Use the
ordinary visible Move selector and radius/strength sliders, accepting only
radius `0.037 ± 0.00011`, strength `0.79 ± 0.0011`, X symmetry on and
backface culling on as read back from the app. Preserve a screenshot and
public `.obrush` Save before contact.

Perform the fixed twelve overlapping contacts below, relative to the
rendered viewport center. Each quadruple is press X/Y, then total delta
X/Y in CSS pixels. Send twelve linearly interpolated held moves per
contact, separated by 30 ms. After release, record GPU/pending-work probes
at immediate, +200 ms and +800 ms intervals before the next contact.
Preserve trusted DOM pointer traces and screenshots after contacts 4, 8,
and 12, plus public `.obrush` Saves after each contact and final Undo/Redo.

```
[-85,-30, 30,-10] [-90,-25, 25, 15] [-80,-35,-30, 10]
[-75,-25, 25,-15] [-95,-20, 30,  5] [-85,-40,-25,-10]
[-75,-40, 30, 15] [-95,-40, 25, -5] [-85,-30, 35,-10]
[-90,-28, 30, 12] [-78,-35,-25, 10] [-80,-22, 30,-12]
```

Numerical gate: at least four contacts change the public mesh; at least
eight original vertices change overall; maximum original displacement is
at least 0.001 world units. Every saved mesh must have finite positions,
zero degenerate faces, boundary/nonmanifold/inconsistent-winding edges,
no adjacent dihedral greater than 90°, and minimum `cross / longest-edge²`
triangle quality at least 0.09. The minimum outward radial face cosine
must remain positive. Each effective contact must be one Undo state; undo
all effective contacts restores the exact initial public mesh, and redo
restores the final public mesh. After release the saved mesh and subsequent
idle save must be bit-identical. The displayed WebGL position/normal/mask/
color vertex buffer should match the saved public mesh's packed f32 bytes.

Visual gate: examine before, intermediate, and final normal-viewport PNGs
at original pixels, explicitly recording whether any sharp triangular
spikes, torn-looking slivers or obvious separated facets arise in the edited
zone. A numerical pass without direct image review is **not** a visual
pass. These face/fold/radial checks are self-intersection proxies, not a
complete surface collision test. The synthetic mouse and one mesh do not
establish physical pen, native GUI, long-session speed, ZBrush or SculptGL
parity. If a pinned SculptGL Move run can be staged through its normal UI
with matching contact geometry, report that comparison separately; do not
promote an unmatched reference screenshot to a paired claim.

Preserve every setup/runtime failure and its output rather than changing
thresholds or contact coordinates afterward. The user's prior tearing report
stays open unless both visible and mesh evidence support this narrow case.
