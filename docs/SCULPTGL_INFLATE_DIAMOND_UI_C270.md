# C270 scoped normal-UI Inflate diamond pair

The [prospective protocol](SCULPTGL_INFLATE_DIAMOND_UI_PREREG.md) was first
committed as `a7d3f5a` before browser execution. The independently authored
four-vertex, two-face OBJ is SHA-256
`9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35`.
The pinned hosted SculptGL bundle is SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the OBrush normal-browser WASM built with `scripts/build-web.sh` is SHA-256
`cb343fca19408fe9ed69cc065f31ea491aae8d18ff75f3b498ac4ebbb8d588d7`.
The application source at build was the isolated branch's `f9169a2` commit
(product code descended from `866defb`); later C270 commits edit only this
protocol, its harness, auditor and report. The official web build passed
with lockfile-matched `wasm-bindgen 0.2.129`.

Both browsers used their ordinary file input, visible Inflate selector,
orthographic framing, 30% intensity, no alpha, culling,
symmetry or continuous mode, and neutral full-force mouse input with
pressure-to-radius and pressure-to-intensity disabled. The OBJ has no mask
channel, so all four vertices are unmasked. OBrush's normal Source mode
requires explicit raw-U mask representation; its visible **Preview mask
conversion** and **Convert** controls changed the imported legacy
representation before sculpting. That conversion is a separate pre-stroke
history action, not a brush dab. Source Negative was selected through the
visible per-tool checkbox, which shares a label with hidden checkboxes for
other tools. The normal UI state, trusted DOM events, actual download events
and complete files are in the raw receipts.

The original preregistered horizontal gesture from center `(+8,0)` to
`(+24,0)` changed zero vertices in both applications. Its path lies exactly
on the diamond's shared triangle edge. A separately preregistered interior
gesture at Y `+64`, with the same 16-point travel, again changed zero
vertices. Complete screenshots showed the mesh under that path. These are
failed useful-stroke admissions, retained without changing their radius or
tolerance. Before viewing further outputs, [the C270C addendum](SCULPTGL_INFLATE_DIAMOND_UI_PREREG.md)
froze one 80-point move inside the lower face: source press `(520,508)`, move
`(600,508)`, release. Source viewport was 1024×808; OBrush's drawing region
was 1024×768. The measured orthographic projection X coefficients were
`1.2211402654647827` and `1.1706534624099731`; SculptGL's radius was 400
CSS points and OBrush calibrated its visible radius
and X offsets by `0.9586560164441112`. Projection Y coefficients were
`1.5475836992263794` and `1.5608713626861572`; its Y offset was calibrated
by `0.958656067448161`. Initial world rays differed by at most
`5.56e-17` in X and `1.39e-17` in Y, with zero measured world-radius
difference. OBrush reported one applied dab per final run; the source normal
UI does not expose a comparable public dab count. Equal later picked
contacts are not assumed.

| C270C case | Changed mapped IDs in each app | Max/RMS paired final XYZ error | OBrush active GPU After/Redo |
| --- | --- | --- | --- |
| Positive | `1,2,3` | `3.725290298461914e-9` / `1.1132631250611199e-9` | exact / exact |
| Negative | `1,2,3` | `3.725290298461914e-9` / `1.1132631250611199e-9` | exact / exact |

Each side's Before/Undo and After/Redo public downloads are byte-exact.
The two original oriented faces and all four original XY words stay exact;
all imported vertices begin unmasked and each application's own
non-position attributes stay fixed. The apps' imported default colors and
material fields are **not** equal across formats, so the paired attribute
gate is diagnostic and false. OBrush's active After and Redo WebGL2 geometry
buffer has one unique drawn 160-byte candidate each time; read-back position,
normal and full interleaved words match the independently packed saved
`.obrush` mesh. The initial mapped position words were identical. The final
positive Z words for IDs `1,2,3` were SculptGL
`3c0438b5,3c2db9a6,38fd3251` and OBrush
`3c0438b4,3c2db9aa,38fd3282`; negative runs have the corresponding sign
bits. These are the actual public Float32 outputs, not the earlier C269
direct-kernel constants. Both useful UI cases pass the prospectively frozen
`3.0e-8` maximum component tolerance.

The paired raw result/audit SHA-256 triples are:

| Case | SculptGL result | OBrush result | Independent audit |
| --- | --- | --- | --- |
| Positive | `aa4e5d9d6888cf0daadcb5eb1df8f85069599b1a4460ac5f75796aaa8c50f880` | `18c67d3018f553e6c9570f37e05a25b029b758b7f83395b730264251ee70dd5b` | `9fd25b97d3e63632bdf28a0f40b398ead996209fed5578eb5b15ec94acfa1f9a` |
| Negative | `62b278f73de95434db8425551462395e2a6a729e2c0fc4d9e988d71463a11c73` | `d3f8ea2054579931940ee1a503629adbcd2719428bc6a9c8bc82207f8e566f36` | `0bcd659753142522de974ddc28cb245a0f4ee50be444f95e3bf2120bf24faacb` |

Raw files are retained under the isolated worktree's ignored
`.artifacts/sculptgl-inflate-ui-diamond/`; a 53 MiB copy excluding disposable
Chrome profiles is also at
`/Users/Ajab/AI/OBrush/.artifacts/sculptgl-inflate-ui-diamond-c270/`.
The evidence index
[sculptgl-inflate-ui-diamond-c270.json](evidence/sculptgl-inflate-ui-diamond-c270.json)
lists the exact relative paths and hashes, including failed setup and
no-change attempts. The first source setup failed because the inherited
driver expected a missing alpha observer; another inspected a hidden
duplicate Negative checkbox. The first OBrush setup tried Source mode before
the required visible mask conversion; another looked for the conversion in
the wrong control map. Those failures are retained and no source/product
behavior was modified to bypass them. The C270C harness and independent
auditor pass `node --check` and `git diff --check`. The focused C269 core
paired-word test passed `1/1`, including exact core Undo/Redo; full workspace
tests and native packaging were not rerun for these documentation/harness
changes.

This is a narrow numerical UI witness for one flat unmasked four-vertex
fixture, two polarities and one useful gesture. It does not complete the
original 10,114-vertex UI cohort, repeated altered normals, mixed masks,
alpha, actual pen pressure, dense or native interaction, million-vertex
responsiveness, or the full `SGL-BRUSH-INFLATE` or ZBrush parity gates.
