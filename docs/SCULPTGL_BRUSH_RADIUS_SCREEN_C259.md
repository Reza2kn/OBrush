# C259: Source Brush screen-radius audit

**Scoped result: Source ordinary Brush already uses a screen radius. The full
`SGL-BRUSH-BRUSH-RADIUS` acceptance gate remains open.** The frozen
[0.1.14 inventory](SCULPTGL_PARITY.md) says the prototype radius was in world
units. Current Source geometry is different: `SourceGeometryPreferences.radius`
is a bounded 5–500 viewport-point value, and the shared Rust preparation
projects the picked point, offsets its screen X coordinate by the radius and
tablet factor, unprojects at the same depth, and divides the squared world
distance by the source object scale squared. The ordinary OBrush tools retain
their separate world-radius and optional screen-radius policy. No geometry or
host code was changed in this audit.

The pinned SculptGL source is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Its `SculptBase.getScreenRadius()` multiplies the tool's radius by the
render-resolution ratio; `Picking.computeWorldRadius2()` projects the actual
hit, offsets screen X, and unprojects at the hit depth. Browser coordinates,
canvas size and the offset all receive the same ratio. This is the relevant
reference rule, not a fixed world radius.

The current normal-browser pair reused the independently authored 10,114
vertex, 20,224 triangle ID-colored SGL sphere, 687,848 bytes, SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
Both apps imported it through their public file inputs, selected ordinary
Brush with Clay, Accumulate, Alpha, symmetry, pressure factors and fold
prevention off, and received one press, one move and one release through
trusted browser input at radius 64 and intensity 0.25. The separate normal
viewports were 1024×808 in SculptGL and 1024×768 in OBrush; their projection
X coefficients were `0.536931574344635` and `0.5147326588630676`.
Accordingly the OBrush normal UI was set to
`64 × 0.5147326588630676 / 0.536931574344635 = 61.35398203662279`
viewport points, which matches initial horizontal world rays and world
radius to `1.11e-16` and `2.78e-17` respectively. Both pages received eight
trusted DOM stroke events.

The independent whole-public-save audit found exact Before positions and
oriented faces across all 10,114 mapped IDs. Each app changed the same 87
IDs; their After saves differ at 10 IDs and 11 position components, with
maximum component difference `2.9802322387695312e-8`. Each app's Undo and
Redo saves reproduce its own Before and After bytes exactly. This supports
the existing Source Brush screen-radius route at one aligned camera and one
mouse setting; it does not establish equal internal dabs or the complete
radius gate.

The pinned normal-browser run used OBrush WASM SHA-256
`1011b3424506ed58d39310c0c09e2692d78d404dec46760854888b87df868d79`.
The ignored local receipts are:

| Record | Directory below `.artifacts/sculptgl-ui-brush-pair/` | SHA-256 |
| --- | --- | --- |
| SculptGL run | `source-a44f8764-6d5c-4a1e-9759-9a90943fd8d8/result.json` | `f551229ee18b595b084a7b758d1dd82f7230f2b56e175bf47948d2a54fe55e98` |
| OBrush run | `root-f583232e-2bb9-41b7-ad1b-80e46207a45c/result.json` | `6e9acb8fe841c9eba2665a0e5b0745822dde67b8d5d8521f425fd22d84d12c4b` |
| Independent audit | `source-a44f8764-6d5c-4a1e-9759-9a90943fd8d8/paired-audit-root-f583232e-2bb9-41b7-ad1b-80e46207a45c.json` | `9dfec105c7ed8300f1bd53343441a1e5f0ef0cf7a96b8e7c39faaf961dd265ac` |

The runs used `scripts/check-sculptgl-ui-brush-pair.mjs` with
`--side=source|root --one-move --radius=64 --intensity=0.25` (the OBrush
run also used `--match-source=<source result.json>`), followed by
`scripts/audit-sculptgl-ui-brush-pair.mjs`. The two earlier C173 settings at
31 and 95 published source pixels remain separate evidence. No new Rust
source was compiled for this documentation audit. The normal browser checks
do **not** cover two zooms, portrait layout, browser DPR 2, render-resolution
ratios, a pressure ramp, every Source tool, native GUI or sustained load.
Those tests, along with displayed-circle matching and complete radius
response, remain open.
