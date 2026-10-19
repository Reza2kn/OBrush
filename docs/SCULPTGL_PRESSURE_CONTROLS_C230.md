# C230: visible pressure factors and per-dab sampling

The normal OBrush brush inspector now offers a SculptGL-style pressure curve
with separate intensity and radius factors from 0 to 1 in 0.01 steps. Enabling
it selects the pinned SculptGL defaults, intensity 0 and radius 0.75. For a
pen contact at pressure `p`, each factor `f` multiplies its base setting by
`1 + f * (2p - 1)`; mouse input uses the neutral point `p = 0.5`. The prior
binary pressure checkboxes remain available when the curve is off. Changing a
brush setting no longer resets the factor profile or stores a pressure-scaled
radius/strength as the new base setting.

An ordinary stroke captures the factor values and base tool settings at
press. Each later dab recalculates intensity, world radius and, when selected,
screen-space footprint from its current pen pressure. The terminal sample
retains the pen release pressure. An unrelated UI change first finishes the
old stroke, preserving its captured profile. The separate Source geometry
response has its own pressure policy and is **not** covered by this change.

The focused host tests cover an unrelated color edit with factors retained;
pressure samples at 0, 0.25, 0.5 and 1; two successive per-dab pressures;
neutral mouse behavior; and the screen-space footprint ratio. The final
source passed `cargo fmt --all -- --check`, `cargo test --workspace --locked`
(2,533 passed, 0 failed, 1 intentional ignore across 83 groups), strict
native and WASM workspace Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh`. The packaged macOS app has a valid ad-hoc
signature. Portable browser WASM SHA-256:
`9742032c6abf9c56068093a08f7dd158e24292f2acbec33d8abe8ad9ea7e486e`.
Packaged macOS executable SHA-256:
`215b0318a45abdc99d17f58e95bf74338e748a27adc6b0fde781f7f1f0d79d41`.

The normal browser loaded this build from the existing loopback server at
`http://127.0.0.1:8792/` in a separate tab. Its inspector visibly switched
from binary pressure checkboxes to factor controls, displayed the 0/0.75
defaults, and displayed 0.45 after changing the intensity slider. This is a
UI observation, not a physical pen or paired SculptGL stroke comparison. A
[separate four-case isolated browser run](../.artifacts/brush-inputs-0.1.6/c230-pressure-only-cfa7405f-afc0-46c2-8637-3359dd6d075e/result.json)
on this exact WASM passed default-policy zero-pressure no-op, mouse/pen-full
equivalence, quarter-pressure displacement (maximum absolute component error
`7.5e-8` against 0.25 of full pressure), public Save and actual GPU-row
checks. It does not exercise the new factor controls under pen events.
The existing full nine-case browser harness still fails when it directly
sets an alpha file without a current import request; its final-build
[failed result](../.artifacts/brush-inputs-0.1.6/c230-final-pressure-0fc293e0-26f3-4b6b-9c60-730a1f3fa08e/result.json)
is preserved. Do not treat that harness as passing.

The pinned SculptGL factor formula and defaults are catalogued under
`SGL-INPUT-PRESSURE-RADIUS` and `SGL-INPUT-PRESSURE-INTENSITY` in the
[inventory](SCULPTGL_PARITY.md). Arbitrary pressure ramps in a real browser,
physical tablet input, Source-response pressure, native GUI operation,
reference-matched shape/footprint output and long-stroke responsiveness
remain open. No input-family parity is promoted.
