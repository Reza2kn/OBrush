# C236: admitting a pinned SculptGL pen gesture

The C234 one-move paired run left the pinned SculptGL mesh unchanged under
trusted synthetic pen input. The pinned source handles a pen in two paths:
`SculptGL.js` listens to `pointerdown`/`pointermove` to store pressure, and its
Hammer recognizer treats pen input as a non-mouse pan. On `panstart`,
`onPanUpdateNbPointers` calls `onDeviceDown` at the moved point, resetting the
stroke origin. In `SculptBase.sculptStroke`, movement must exceed
`0.15 * tool radius * device pixel ratio` before a dab is made. With a radius
of 64 and pixel ratio 1, the threshold is 9.6 pixels. A single 12- or
48-pixel pen move in this browser reaches `panstart` but no `panmove`, and the
public SGL remains unchanged. A three-move gesture reaches two `panmove`
events and changes geometry. This establishes a usable synthetic reference
gesture without altering SculptGL.

The [owned browser diagnostic](../scripts/check-sculptgl-pen-gesture-c236.mjs)
used the same independently authored 10,114-vertex SGL fixture as C234
(SHA-256 `39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`)
and the pinned hosted bundle
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The [source result](../.artifacts/sculptgl-pen-gesture-c236/source-838a1d44-36fd-4efc-940e-68e558283975/result.json)
and [independent SGL audit](../.artifacts/sculptgl-pen-gesture-c236/source-838a1d44-36fd-4efc-940e-68e558283975/gesture-audit.json)
retain trusted DOM and Hammer events, public saves, exact coordinates and
event times, and clean owned-browser closure. Every case imported a fresh
fixture, set visible radius/intensity factors to zero, pressed at `(592,444)`,
held 160 ms, waited 160 ms after each move, then released and saved.

| Gesture | Pressure | Held moves | Hammer panstart/panmove | Changed original IDs | Max absolute position component |
| --- | ---: | --- | ---: | ---: | ---: |
| Pen, short | 0.25 | 12 px | 1 / 0 | 0 | 0 |
| Pen, long | 0.25 | 48 px | 1 / 0 | 0 | 0 |
| Pen, three moves | 0.25 | 12, 24, 36 px | 1 / 2 | 87 | 0.00538623 |
| Pen, full, long | 1 | 48 px | 1 / 0 | 0 | 0 |
| Mouse control | 0.5 | 12 px | 1 / 0 | 87 | 0.00548422 |

All five gestures had a trusted `pointerdown`, held `pointermove`, and held
`mousemove`. The three-move pen and mouse cases each have exact before/Undo
and after/Redo save bytes. The no-op pen cases have exact after/Redo but
before/Undo differs, consistent with the C234 history limitation. The
first-move reset and later changed mesh explain the C234 no-op as a gesture
admission problem for that protocol; they do not prove that physical tablets
behave identically or that the source and OBrush pressure responses match.

The [three-move paired matrix](../scripts/check-sculptgl-pressure-matrix-c236.mjs)
and [paired audit](../scripts/audit-sculptgl-pressure-matrix-c236.mjs) are
the follow-up comparison on the same fixture, with the reference pen gesture
now admitted. The [pinned SculptGL run](../.artifacts/sculptgl-pressure-matrix-c236/source-b149f003-a5ec-452a-a3cc-1fd49784a5d0/result.json),
[OBrush Source-mode run](../.artifacts/sculptgl-pressure-matrix-c236/root-9a489939-d04f-458e-a60f-00a4b5aeeb78/result.json),
and [whole-save audit](../.artifacts/sculptgl-pressure-matrix-c236/source-b149f003-a5ec-452a-a3cc-1fd49784a5d0/audit-root-9a489939-d04f-458e-a60f-00a4b5aeeb78.json)
retain the five fresh imports per side, trusted input traces, actual factor
controls, public files, and clean owned-browser closures. The OBrush WASM
SHA-256 was
`ac7b284dd33eab4c1202b19ddc91565327cdf2df62d7f7a6495a64b664394c39`.
The prerecorded imported-position tolerance was `5e-5` world units and the
delivered-pressure tolerance was `1e-6`; all 10,114 original vertex positions
matched exactly before each paired gesture, and all pressure delivery checks
passed. SculptGL pressed at `(592,444)` and moved to x=604, 616, 628. OBrush
pressed at `(817,441)` and moved to x=829, 841, 853. Each path starts 80
screen pixels right of its measured viewport center, with 12-pixel steps and
160 ms between held samples. Camera-ray equivalence is not established.

| Case | Pressure | Radius/intensity factors | Pinned changed original IDs | OBrush changed original IDs | Pinned max position change | OBrush max position change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Neutral quarter | pen 0.25 | 0 / 0 | 87 | 122 | 0.005386 | 0.016252 |
| Radius quarter | pen 0.25 | 1 / 0 | 21 | 39 | 0.002693 | 0.007157 |
| Intensity quarter | pen 0.25 | 0 / 1 | 87 | 124 | 0.002694 | 0.008145 |
| Neutral full | pen 1 | 0 / 0 | 87 | 122 | 0.005386 | 0.016252 |
| Neutral mouse | mouse 0.5 | 1 / 1 | 107 | 122 | 0.015753 | 0.016252 |

The radius case's changed IDs are a subset of the neutral case in each app.
Against half the neutral displacement at pressure 0.25, the pinned intensity
case's maximum position-component error is `2.98e-8` world units. The OBrush
case's error is `1.0705e-4` and it changes 124 IDs where its neutral case
changes 122. Both show the intended factor direction; the multi-dab intensity
responses differ. The neutral-quarter final saves differ at 122 original IDs,
with maximum paired component difference `0.01086611` world units. Requested
screen coordinates and factor values do not establish equal delivered brush
dabs, camera rays, or algorithms, so that paired difference is descriptive,
not a parity tolerance failure. All five cases on both sides recover exact
before/Undo and after/Redo save bytes. Every pinned pen case records two
Hammer `panmove` events and changes geometry.

This resolves the C234 synthetic one-move admission blocker and provides a
reference-paired factor-direction result on one fixture. Physical tablet
behavior, varied meshes and tools, exact brush-output matching, and the
ordinary OBrush Draw path remain separate checks. Neither pressure inventory
ID nor SculptGL/ZBrush parity is promoted.

No SculptGL, OBrush app/core, or shared ledger source was edited. `node --check`
passed for all four C236 scripts; both paired owned browsers and loopback
servers exited cleanly. The C234 failed attempts remain preserved.
