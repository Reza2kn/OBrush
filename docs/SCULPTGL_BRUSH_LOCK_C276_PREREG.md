# C276 prospective integer-coordinate Source Brush Lock pair

This is separate from the failed C274 frozen pair and the C275 no-go input
path. The C274 raw result and 1e-6 held-radius failure remain immutable. C276
tests a distinct, exactly representable normal-browser gesture to determine
whether the shared-core Source Brush Lock route can be admitted in a narrow
scope without pretending to solve fractional-coordinate quantization.

Use the same pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
hosted JS hash `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent 10,114-vertex RGB-ID sphere hash
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`,
and independent hook alpha hash
`0486e281eeb2766ecf096ef55fe42023545675d9d9bc123441cd729109ac290a`.
Use Brush/Draw, Clay/Accumulate/Negative/Culling/X symmetry/Continuous/Prevent
folds off, no mask, orthographic front framing, radius 64 screen points,
intensity 25%, pressure factors zero, hook alpha and Lock on. Use public normal
UI for every control, import, gesture, Save, Undo and Redo.

Set both applications to 1024×808 *logical sculpt viewport* at DPR 1 and UI
zoom 1 before fitting the fixture. Source's existing 1024×768 backing target
has 1024×808 logical canvas bounds; OBrush needs a normal browser-window
height adjustment to reach 1024×808 logical canvas bounds. Record exact
window/viewport/backing measurements. After normal UI fit, require both
orthographic projection X/Y, world brush radius and center rays to match
within `1e-6` before accepting the paired stroke. The screen path in each
application is integer press at its measured sculpt-viewport center, held
move +80 CSS X, held move -80 CSS Y, held move back +80 CSS X, release.
Assert trusted DOM pointer coordinates are exactly those requested integers;
do not use injected app state or rounded surrogate inputs. The Source fixture
is newly captured in this run and is not substituted for the frozen C273/C274
Source oracle.

Freeze acceptance: paired Before/attributes/faces and exact changed RGB-ID
membership; Source and OBrush each change exactly 26 IDs; paired final XYZ
component max `<=1e-6` with actual max/RMS; public Before=Undo and
After=Redo exact in each app; active GPU position words equal each public
Save with unique active-object identity; compare active GPU display-normal
words with max/RMS reported, never saved normals as the display oracle. Each
side must have no press dab, distinct asymmetric-alpha directional held
previews, equal horizontal/vertical held radii within `1e-6`, fixed center,
exact repeated endpoint, release=last preview and quiet stable frames. Also
preserve C274 fractional-coordinate evidence as an explicit excluded case.
Record exact harness, JS/WASM, reference and fixture hashes and all failed
checks. If viewport/projection alignment or any paired gate fails, leave
the product candidate unpromoted and revert it after retaining the receipts.
