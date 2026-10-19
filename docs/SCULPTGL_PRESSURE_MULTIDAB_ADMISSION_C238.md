# C238: synthetic pen gesture admission and multi-dab pressure response

The C236 three-move pressure comparison did **not** deliver the same number of
geometry-bearing moves to the two browsers. In the pinned SculptGL host, a
160 ms pause between held pen moves lets `_lastNbPointers` reset to zero after
each `panmove`. The next move restarts the stroke at its new position. The
public SGL save stops changing after the first effective dab. OBrush applied
three dabs in C236. Its `1.0704994201660156e-4` maximum deviation from an
exact half displacement therefore cannot be diagnosed as a pressure-factor
bug by comparison to SculptGL's one-dab `2.9802322387695312e-8` result.

This follow-up used the independently authored 10,114-vertex, 20,224-triangle
RGB-ID sphere (`SHA-256 39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`).
Pinned source commit: `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
hosted SculptGL bundle SHA-256:
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
OBrush's C236/C238 browser WASM SHA-256:
`ac7b284dd33eab4c1202b19ddc91565327cdf2df62d7f7a6495a64b664394c39`.
The test worktree began at OBrush commit
`359d5bb091db0dbf7937ad38cb624bb4a60cd6a7`. Both browsers imported the
same sphere with bit-identical decoded original positions by RGB ID.

## Why the slow held gesture stops

Pinned `upstream/src/SculptGL.js:168-211` converts Hammer `panstart` into
`onDeviceDown` and `panmove` into `onDeviceMove`. If the current pointer count
differs from `_lastNbPointers`, `onPanMove` ends and restarts the stroke. Its
`_isIOS()` check at lines 200-203 shares a name with the prototype method:
`this._isIOS !== undefined` is true while that method is present, and the
method returns a truthy function. Consequently the 60 ms reset at lines
192-196 is scheduled for the tested desktop host too. This source-level
explanation is consistent with the captured states; it is not a claim about
an external tablet device. SculptBase's `sculptStroke` also requires movement
strictly greater than `0.15 * radius * pixelRatio` (9.6 px here) before it
makes a stroke (`upstream/src/editing/tools/SculptBase.js:122-151`). A
`panstart` at +12 px therefore only sets the stroke origin.

The slow source run requested a pen down at `(592,444)` and held samples at
horizontal offsets +12/+24/+36/+48 px, with pressure 0.25, brush radius 64,
intensity 0.25, and pressure factors zero. Each held sample paused 160 ms.
CDP delivered trusted pointer moves (`buttons=1`, pressure 0.25), and Hammer
reported `pointers.length=1` for every `panmove`. The **later captured
`_lastNbPointers` was zero**, after the 60 ms timer. It would be incorrect to
say the Hammer `panmove` itself had zero pointers.

| Held moves | Original IDs changed | Public after-save SHA-256 |
| ---: | ---: | --- |
| 1 | 0 | `a78ea33cb2daaadd28d1036a187cdfbd06f486279ca4e7568ccb3fba88220514` |
| 2 | 87 | `f261284d265fd5dff2d945e73666da839920b893031fddb2dd19464dce5358b8` |
| 3 | 87 | `f261284d265fd5dff2d945e73666da839920b893031fddb2dd19464dce5358b8` |
| 4 | 87 | `f261284d265fd5dff2d945e73666da839920b893031fddb2dd19464dce5358b8` |

The 2-4 move SGL files are byte-identical, not merely equal within tolerance.
Their redo saves also match their after saves. The normal UI imports, settings,
CDP input, Hammer events, stroke states, and public saves are in
`.artifacts/sculptgl-pen-dab-count-c237/source-d136b541-70e0-4c71-bba9-cdcd62de0b1b/`.

## Fast admission and paired pressure factors

With 20 ms between held samples, the same four Source offsets yielded 0, 87,
96, and 107 changed original IDs. Their after-save hashes were, in order:
`a78ea33cb2daaadd28d1036a187cdfbd06f486279ca4e7568ccb3fba88220514`,
`f261284d265fd5dff2d945e73666da839920b893031fddb2dd19464dce5358b8`,
`8676fcc6243848bee2791d8cf36ce438158a771c83a801e0ecdfc4411c81bdca`,
and `a5341bd6819ecce39df89bba3d1a2829bf5a915286e2f69167abb509cbe7f5ed`.
The observed pointer count stayed one through the held moves. Each added
`panmove` produced a distinct saved mesh after the initial `panstart`, so this
sequence admits a multi-dab Source pressure comparison. The full fast-ladder
trace is in `.artifacts/sculptgl-pen-fast-dab-count-c238/source-01fca240-e254-4af1-ad3d-e48195512deb/`.

The factor matrix used Source four 20 ms held samples (+12, +24, +36, +48),
including three `panmove` events at +24/+36/+48. OBrush started at +12 and
used three 160 ms held moves at +24/+36/+48. Its counter advanced by exactly
one dab at each move. All six cases used pressure 0.25 and the same public
fixture. Neutral factors were `(radius=0, intensity=0)`, narrowed radius was
`(1,0)`, and halved intensity was `(0,1)`. Brush radius was 64 and nominal
intensity 0.25. The half-error is
`max_ID,axis |(half_position - before) - 0.5*(neutral_position - before)|`.

| Browser | Neutral changed IDs | Radius-factor changed IDs | Intensity-factor changed IDs | Max half-error |
| --- | ---: | ---: | ---: | ---: |
| SculptGL, three admitted `panmove`s | 107 | 31 | 107 | `1.02996826171875e-4` |
| OBrush, three applied dabs | 122 | 39 | 124 | `1.1524558067321777e-4` |

Both browsers narrow the radius-affected set and have a nonlinear final
displacement after multiple dabs. OBrush's one-dab C234 saved mesh, checked
offline on the same fixture, changed 98 IDs in both neutral and intensity
cases and had max half-error `2.9802322387695312e-8`. Thus the small
one-dab error becomes a roughly `1e-4` multi-dab error in **both** products.
That pattern does not justify a shared-core/native-WASM pressure math change.
The C238 paired audit verifies same fixture, trusted declared pointerdown,
three Source `panmove`s, three OBrush applied dabs, exact OBrush undo/redo,
and exact Source redo. Files:
`.artifacts/sculptgl-pressure-matched-dabs-c238/source-c04ce05e-d6fa-4bd9-afd0-399dca154bc6/`
and `.artifacts/sculptgl-pressure-matched-dabs-c238/root-17f3e136-f3ad-43a1-b9d2-59bb188fe9fc/`.

The first attempted OBrush pairing started at +0 and moved +24/+36/+48. Its
first 24 px segment generated **two** spacing dabs, yielding four total;
the admission check failed, with max half-error `2.0939111709594727e-4`,
and its run and audit remain under
`.artifacts/sculptgl-pressure-matched-dabs-c238/root-69089426-7cf6-43c3-8e87-bccfd263a249/`
and the corresponding `audit-root-69089426-7cf6-43c3-8e87-bccfd263a249.json`
in the Source result directory. An exploratory fixed-contact three-dab Rust
test also failed an exact-half assertion (max error `5.739927292e-5`);
`.artifacts/c237-core-fixed-pass.log` preserves that failed check. The
temporary test was removed because live-mesh multi-dab nonlinearity is also
present in the admitted reference result.

The reference/browser comparison remains **directional**, not raw-shape or
physical tablet parity. Source and OBrush brush algorithms, picking rays,
normal refresh, and screen-to-surface contact may differ. Synthetic CDP pen
events establish only this hosted browser path. No product geometry or input
code was changed.

Reproduce from a checkout with the pinned local fixture/host mirror and
browser driver dependencies:

```sh
node scripts/check-sculptgl-pen-dab-count-c237.mjs
node scripts/check-sculptgl-pen-fast-dab-count-c238.mjs
node scripts/check-sculptgl-pressure-matched-dabs-c238.mjs source
node scripts/check-sculptgl-pressure-matched-dabs-c238.mjs root
node scripts/audit-sculptgl-pressure-matched-dabs-c238.mjs SOURCE_RESULT_JSON ROOT_RESULT_JSON
```

The audit scripts read and hash public saves independently; they do not rely
on browser screenshots or product-internal mesh arrays for the result.
