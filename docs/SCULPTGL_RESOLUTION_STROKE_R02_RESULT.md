# Resolution-stroke R02: scoped paired browser audit

**No product change; `SGL-RENDER-RESOLUTION` remains Open.** The frozen
[C327 press/release gate](SCULPTGL_RESOLUTION_CONTACT_C327_PREREG.md) failed
in pinned SculptGL before reaching OBrush. At DPR 2 and 1× resolution, a
centered press/release on the imported sphere left history 2→2 and changed
zero vertex words, although the cursor visibly landed on the sphere. Its
original receipt and screenshots are at
`.artifacts/sculptgl-resolution-contact-c327/f908691e-b90a-4018-b667-34a2e7f4cfb6/`.
An earlier harness poll was interrupted after ~2 minutes with no first
screenshot because it serialized the whole reference mesh on each status
read; the preserved note is at
`.artifacts/sculptgl-resolution-contact-c327/761e44be-06f2-4c8b-8bdf-2fd9e7258e49/interrupted-harness.json`.
Neither failure is an OBrush product verdict.

The [frozen held-move follow-up](SCULPTGL_RESOLUTION_STROKE_C328_PREREG.md)
changed only the input protocol: left press, one held 12-CSS-pixel horizontal
move after 60 ms, then release after 120 ms. The harness uses C328 in its
filename because that was its preregistered name; this report uses **R02**
to avoid collision with the existing Source Pinch C328 report. Its paired
measurements passed at DPR 1 and 2 for fresh-page resolution 1×, 0.5× and
2×. They establish only a **single, very small stroke's** model-space
invariance across display resolutions within each app.

The pinned SculptGL commit was
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and mirror wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
The official root `web/dist` portable OBrush WASM was SHA-256
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`,
from product revision `30a7e7ad6b48f2802f685ded7d11b2c5e0806443`.
The receipt's `build.commit` field records isolated **harness** commit
`5cefe5866f8d3b7351e6d7a72f330ac0f2de39cc`; no product was rebuilt.
The independent imported sphere OBJ is 24,272 bytes, SHA-256
`165e2cfacb2f0d99312fd3f4f3f79c2696be3eee9b05047ac7f57c891783ba9b`.

| DPR | Reference backing at 1 / 0.5 / 2× | OBrush host backing | OBrush scene at 1 / 0.5 / 2× |
| --- | --- | --- | --- |
| 1 | 1290×960 / 645×480 / 2580×1920 | 1600×1000 | 1140×912 / 570×456 / 2280×1824 |
| 2 | 1290×960 / 645×480 / 2580×1920 | 3200×2000 | 1140×912 / 570×456 / 2280×1824 |

The OBrush scene sizes have matching observed WebGL texture allocations at
every ratio and DPR. No recorded GL or scene error occurred. Reference CSS
input was `(645,520)`→`(657,520)`, radius 50 px, intensity 0.5; OBrush CSS
input was `(795,513)`→`(807,513)`, Draw radius 0.18, strength 0.35. OBrush
cursor-start error was `[0,0]` physical pixels in all six cases. The
different brush units, camera and shader prohibit a cross-app geometry claim.

Each of the six reference strokes changed **one position word**, retained
the imported sphere and incremented history 2→3. Each OBrush stroke also
changed **one position word**, retained selection and incremented Undo 1→2.
Within each app and DPR, both 0.5× and 2× changed the same word ID as 1×;
maximum post-stroke coordinate difference was exactly `0` model units.
OBRush's before-contact public project was byte-identical across all six
cases, SHA-256
`a0e3553d8e1223f112f771b90e3a3ed94302b92205bd3caf84f9cc9b7c216f08`.

| DPR | Local receipt directory, 15 screenshots each | `result.json` SHA-256 |
| --- | --- | --- |
| 2 | `.artifacts/sculptgl-resolution-stroke-c328/5423e612-6dbf-4303-90ae-056d0959922f/` | `df3ece658ae3f9753639f40a9740aefeb611938b25cd4c2575b5eddb204efcef` |
| 1 | `.artifacts/sculptgl-resolution-stroke-c328/8faf25e8-6ecc-450a-b065-16e251f7d990/` | `0f04f15484143569c5bdbe626383d9f9c9f04508b9b95270886c35ed9e1e7d99` |

The frozen held-move harness SHA-256 was
`b09dfcb329870e1d3e5e78b788c4f827a3fa04e547c379d98555ff7671df6fbf`;
`node --check` and `git diff --check` passed. Both runs wrote a passing
`result.json` and closed owned Chrome, but Node lingered during cleanup.
The DPR 2 process eventually exited 0 after an interrupt; the DPR 1 process
was interrupted with exit 130. This is a **harness cleanup failure**, so
the run is a complete scoped measurement, not a clean command-level pass.
No Cargo checks were run because no product code changed.

The reference ratio changed through its public GUI controller, not a visible
click, because the mirror topbar lacks a reliable hit target. OBrush used its
visible slider and normal mouse input. A one-word deformation is too sparse
to establish broad brush quality or stroke invariance; a denser independent
fixture with a preregistered minimum changed-vertex count, clean harness
exit, long strokes, pressure, symmetry, browser zoom and native GUI remain
open. Full SculptGL and ZBrush parity remains open.
