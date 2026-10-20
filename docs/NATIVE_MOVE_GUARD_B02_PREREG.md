# B02 tangential Move guard correction

**Frozen before B02 test/build/browser contact.** B01 remains failed: the sixth
repeated ordinary-browser contact grew an existing mirrored edge from 3.218°
to 33.662°, and the maximum reached 36.333° by contact eight. The B01 guard
started relaxing its 30° limit at only 25% normal alignment. This classifies
a mostly tangential Move on a curved surface as an extrusion. B02 keeps the
30° limit whenever the tangential component of the requested displacement is
at least as long as its normal component. Above that 45° geometric boundary,
it blends continuously to the existing 90° extrusion latitude at fully
normal motion. The triangle quality floor and all topology/history safeguards
remain unchanged. B01's failed receipt and baseline are retained.

Before promotion, require `cargo fmt --all --check`, complete workspace tests,
strict native and WASM Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh`. Specifically retain all existing
`move_surface` usefulness assertions, including a small radius 0.037
outward-Z extension above 0.018, long refined-surface extension above radius,
and monotonic clamping. Record the independent core analogue's changed and
refined vertices, maximum original displacement, and maximum fold; a zero or
inert Move fails.

Then replay the unchanged B01 `sphere-twelve-repeated-q02` case in owned
normal Chrome at 1600 × 1000/DPR 1 with 0.037 radius, 79% strength, X
symmetry, twelve ordinary mouse contacts and twelve 30 ms held samples per
contact. Preserve all B01 gates verbatim, including every saved-contact fold
`<=30.01°`, maximum original displacement `>=0.005`, exact GPU/save/Undo/Redo,
and the same-tab matched-camera close-up with 100 informative changed pixels
in the fixed ROI. Record exact source/WASM/harness hashes and all failures.
This is one bounded candidate gate; it does not close the user's broader
Move tearing report or establish SculptGL/ZBrush parity.
