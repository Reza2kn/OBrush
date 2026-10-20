# Q02 repeated Move: numerical pass, visual gate inconclusive

The [frozen Q02 gate](NATIVE_MOVE_REPEATED_VISUAL_Q02_PREREG.md) reached
twelve ordinary browser Move contacts on a fresh default sphere at radius
`0.0370000005`, strength `0.7900000215`, X symmetry and backface culling on.
The current official root WASM and loopback-served bytes were both SHA-256
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`
(24,194,074 bytes); the owned browser was Chrome 155.0.8059.85, CSS viewport
1600 × 1000 at DPR 1. The isolated harness source was SHA-256
`897e3336d9526f36876f5ee02193fd4dc8f4d8cbbc8280fd485c17706a761eb9`.

The frozen numerical gate passed. Eight of twelve overlapping contacts
changed the public mesh; the last four were exact no-ops, which is material
to interpreting the visual exposure. The final project had 46,642 vertices
and 93,280 faces versus 40,962/81,920 initially. Seventy-eight original
vertices moved, up to 0.0138459 world units. Every saved contact mesh was
finite, closed, manifold and consistently wound, with no degenerate face or
fold over 90°. Across all twelve saves, minimum `cross / longest-edge²`
quality was 0.09999956, minimum outward radial face cosine was 0.26755,
and largest adjacent fold was 71.49°. At the final state these were
0.09999974, 0.64483 and 50.26°, respectively. The starting sphere's
minimum quality was 0.68823 and minimum radial cosine 0.99999906. These
metrics are local safety proxies; they do not detect all self-intersections.

Trusted DOM pointer events were recorded for all contacts. GPU packed
position/normal/mask/color bytes matched the final public mesh. The release
probes and later idle public Save were bit-identical; there was no queued
post-release sculpting. The eight effective contacts corresponded to eight
Undo states in exact reverse order, and Redo restored the final mesh exactly.
The final public project is
`.artifacts/move-regression/q02-amended-ad1c3330-e445-4602-9e65-903c4a0c2977/sphere-twelve-repeated-q02-final.obrush`
(SHA-256 `5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632`).

I reviewed the original 1600 × 1000 screenshots before contact and after
contacts 4, 8 and 12. The sphere occupies roughly 650 screen pixels across,
while the edited marks are only small patches near its center. There is no
large detached spike in this full-sphere view, but tiny pinched highlights
are visible. That scale cannot resolve the user's close-up, sharp triangular
tearing complaint. **Q02's visual-quality gate is inconclusive and the
complaint remains open.** An ordinary zoomed camera check of the same saved
states is needed before interpreting those highlights. A matched pinned
SculptGL Move run was not performed: the two apps' camera, brush footprint
and save geometry would first need to be calibrated, so an unmatched image
would not be a defensible parity comparison.

The complete retained pass receipt is
`.artifacts/move-regression/q02-amended-ad1c3330-e445-4602-9e65-903c4a0c2977/`
(`result.json` SHA-256
`136b6614c222e67716226ee1c5ec7a8e85b844c513e55f76f296f78a0866d95b`).
Initial PNG SHA-256 is
`88c8e1a0d698452aa4b177e90d782100914c541028fc86496a6c9fa077a0fb5b`;
after-4 is `0aabcfc9a9eb68ef060452a4449531201b2a9245a3aec2eac65b15302e7353bd`,
after-8 is `ed0926a1423dd7af9694a003c29a83f3e2b6316cdd6def6bf28ba65f438b44c4`,
and after-12 is `03523c5b08d54ba412d69f9f59d2f93473558b164a45d5c7ee84e0a451667928`.
The owned Chrome process and loopback server exited normally.

The first Q02 run failed during visible-slider setup because the old
hardcoded Y coordinate hit Local Scale after the UI grew. Its exact failed
receipt and unchanged-gate correction are retained in
[the setup amendment](NATIVE_MOVE_REPEATED_VISUAL_Q02_SETUP_AMENDMENT.md).
Q02 does not replace the [M03 native setup failure](NATIVE_MOVE_M03_NATIVE_REPORT.md),
the 0.1.7 quality findings, or any ZBrush/SculptGL Move reference gate.
