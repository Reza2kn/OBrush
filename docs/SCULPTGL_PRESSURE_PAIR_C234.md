# C234: paired pressure-factor input and saved-shape check

The pinned SculptGL source at commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
defines both pressure multipliers as `1 + factor * (2 * pressure - 1)` in
`src/misc/Tablet.js` (SHA-256
`02798e04ecba359831de333fe8e6f61576d96a707bf48a9dd6e870a3d9037561`).
Its Brush applies the intensity multiplier and Picking applies the radius
multiplier. At pressure 0.25, factor 1 therefore predicts 0.5 times the base
value; at the neutral pressure 0.5 it predicts 1. The current OBrush Source
geometry implementation uses the same multiplier in its captured radius and
intensity paths. These source-level formulas are a narrow numerical contract,
not a brush response or parity result.

The [C234 browser harness](../scripts/check-sculptgl-pressure-pair-c234.mjs)
ran five normal-browser cases independently in each application. Both imported
the same independently authored RGB-ID SGL sphere with 10,114 vertices and
20,224 triangles (SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`).
The [independent public-save audit](../scripts/audit-sculptgl-pressure-pair-c234.mjs)
found **zero differing original vertex positions before any strokes**. The
pinned hosted SculptGL bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
OBrush WASM SHA-256 was
`ac7b284dd33eab4c1202b19ddc91565327cdf2df62d7f7a6495a64b664394c39`.
Both ran in separate owned Chrome 155.0.8059.80 sessions at device scale 1,
with actual visible factor controls set to the specified values. The harness
requested a press at `(592, 444)`, a held 12-pixel horizontal move, and release
after 160 ms at each point; the complete delivered coordinates and event times
are in the result files. The OBrush side used Source Brush geometry mode to
match the pinned SculptGL Brush family. Its ordinary Draw factor behavior has
separate [C231](SCULPTGL_PRESSURE_FACTOR_BROWSER_C231.md) and
[C232](SCULPTGL_PRESSURE_RAMP_BROWSER_C232.md) evidence.

| Case | Pointer pressure | Radius/intensity factors | Pinned SculptGL changed original IDs | OBrush changed original IDs | OBrush max absolute position change |
| --- | ---: | ---: | ---: | ---: | ---: |
| neutral-quarter | pen 0.25 | 0 / 0 | 0 | 98 | 0.005674 |
| radius-quarter | pen 0.25 | 1 / 0 | 0 | 25 | 0.002826 |
| intensity-quarter | pen 0.25 | 0 / 1 | 0 | 98 | 0.002837 |
| pen-full-neutral | pen 1 | 0 / 0 | 0 | 98 | 0.005674 |
| mouse-neutral | mouse 0.5 | 1 / 1 | 87 | 98 | 0.005674 |

Every pen case delivered a trusted `pointerdown` with the requested pressure,
a held `pointermove`, and two `mousemove` events in **both** browsers. The
pinned SculptGL pen saves nevertheless had zero changed original positions,
including full pressure with both factors zero. Its mouse case changed 87 IDs.
OBrush's pen and mouse cases changed 25–98 IDs; its radius and intensity
responses go in the predicted direction. The paired mouse final meshes differ
at 98 original IDs, maximum position-component difference `0.00096405` world
units. This is a direct measured difference on one common fixture, not an
accepted tolerance or a pressure comparison. The prerecorded imported-position
tolerance was `5e-5` and delivered-pressure tolerance was `1e-6`; the imported
positions matched exactly, and the pressure event gate passed.

The [SculptGL result](../.artifacts/sculptgl-pressure-pair-c234/source-5651f2a0-da23-4df2-9181-f0694a88e3e9/result.json),
[OBrush result](../.artifacts/sculptgl-pressure-pair-c234/root-6a53fdae-4094-45a5-adce-c418b45e2fa6/result.json),
and [paired audit](../.artifacts/sculptgl-pressure-pair-c234/source-5651f2a0-da23-4df2-9181-f0694a88e3e9/audit-root-6a53fdae-4094-45a5-adce-c418b45e2fa6.json)
retain every public save, fixture/build hash, CDP input trace, visible-control
action, and cleanup record. OBrush Undo/Redo recovered exact before/after save
bytes in all five cases. SculptGL mouse Undo/Redo was exact; after its four
no-op pen gestures, Redo matched the no-op save, while Undo did **not** match
the before save. This is an unresolved history distinction and prevents an
all-cases history pass. Both final owned browser processes and servers exited.

Earlier failed attempts are preserved in the same `.artifacts/sculptgl-pressure-pair-c234/`
directory: the initial stationary press produced no SculptGL change, the first
observer omitted `mousemove` and could not diagnose delivery, and one later
five-case run failed its immediate owned-process closure admission even though
the PID was absent at the subsequent shell check. The successful runs above
use the complete event observer and pass owned-process closure. The cause of
the pinned browser's synthetic pen no-op remains open. A physical tablet or a
reference-admitted pen gesture, with a recorded changed SculptGL mesh, is
needed before comparing pressure-driven geometry. This run does not promote
either pressure inventory ID or establish SculptGL or ZBrush parity.

Reproduce sequentially after a local `scripts/build-web.sh` artifact is in
`web/dist`:

```sh
node scripts/check-sculptgl-pressure-pair-c234.mjs source
node scripts/check-sculptgl-pressure-pair-c234.mjs root
node scripts/audit-sculptgl-pressure-pair-c234.mjs SOURCE_RESULT_JSON ROOT_RESULT_JSON
```

The harness verifies the pinned hosted bundle and fixture hashes before
starting, and records the current OBrush WASM hash rather than silently
requiring a stale build. `node --check` passed for both C234 scripts. No Rust
application source was changed by C234; the root task owns workspace Rust and
feature-ledger verification.
