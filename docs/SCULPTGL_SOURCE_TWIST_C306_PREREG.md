# C306 Source Twist normal-UI pair: preregistration

Frozen before a C306 browser capture or Twist product edit. The pinned
reference is SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
host mirror `sculptgl.js` SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and independent Python-authored SGL v3 fixture
`tests/fixtures/independent-source-twist-c306.sgl` SHA-256
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`
(72,132 bytes, 1,089 vertices, 2,048 triangles). The fixture generator and
manifest are committed with this preregistration. It is a curved grid with RGB
vertex IDs, a fully protected ID543 and half-unprotected ID577. Neither
application supplied its encoder.

The primary hypothesis is that the current OBrush Twist route differs from the
reference because OBrush captures one original patch and applies an absolute
rotation, whereas pinned `Twist.js` picks vertices at every held move, rotates
the current surface around that move's intersection, and multiplies each
incremental angle by the current falloff, mask and alpha. The reference uses
the negative picked eye direction as its view axis, skips moves inside a
30-CSS-pixel radius around mouse-down, and does not expose an intensity
slider. The initial baseline must run on unmodified OBrush code. Any later
candidate must be narrow, use shared Rust geometry, and receive its own build
and normal-UI receipt; a passing legacy unit test is insufficient.

In both normal browser UIs: import this exact fixture through File Add with
scale/center disabled; select Twist (tool index 3); set radius to 75 CSS
pixels, X symmetry/culling/continuous/lock off, Alpha None, tablet intensity
and radius factors zero, orthographic camera, Reset and fit with terminal
camera settlement of at least 12 frames and 250 ms. Use a logical 1024×808
viewport and trusted integer pointer events. Freeze the initial four-point
path relative to viewport center as `(-48,-12)` mouse-down, `(-12,-12)`,
`(-15,-42)`, `(-42,-44)`, with held snapshots after each move. This moves
outward past the 30-pixel dead zone, then describes two angular turns. No UI
Save or control click occurs during the hold. Release at the final point.
Record the actual source and OBrush camera matrices, rays, initial pick,
settings, delivered events, fixture/build/harness SHA-256 values and all
failed setup checks. Camera projection X/Y ratios, the four requested rays
and world radius must agree within `1e-8`; if not, stop before interpreting
geometry and retain the setup failure. Build hashes are captured from the
served files after the exclusive build lane is granted.

Before, each held move, After, Undo and Redo: use independent SGL and `.obrush`
decoders to map positions and attributes by RGB ID; compare the active GPU
position and display-normal words, not public saved normals as a display
oracle. Required gates are identical changed RGB-ID membership, max paired
XYZ and display-normal component difference `<=1e-6` at every stage, exact
own-app public Before/Undo and After/Redo files, one history step, unchanged
topology/attributes, protected ID543 immobile, half-protected ID577 moving
less than an adjacent unprotected vertex, active GPU positions exactly
matching each own Save, one active object/VBO and no GL errors. The held
snapshots must show progressive deformation and final quiet GPU state must
equal released After. Preserve numerical failures rather than adjusting the
fixture, thresholds or pointer path after observing them.

This is one scoped Twist gesture. Radius at other zoom/DPR, X symmetry,
culling, Alpha, pressure, dynamic topology, native GUI and long-session
responsiveness remain open, as do full SculptGL and ZBrush parity.
