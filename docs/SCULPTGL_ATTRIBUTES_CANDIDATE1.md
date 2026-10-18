# OBrush 0.1.16 candidate 1: material and surface attribute checks

The normal browser host preserves version 6 roughness/metallic rows, original
triangle/quad faces and corner UV IDs through the checked import, short Draw,
save, undo/redo and explicit detach workflows. Studio/PBR/Normal, flat shading
and PBR exposure also produce measured viewport pixel changes while preserving
the project and camera. These are bounded local foundation checks. SculptGL and
ZBrush parity remain unverified.

**A real OBJ consumer gap remains open.** The checked OBJ writer retains normals,
original faces and corner UVs, but normal OBJ import in this frozen build drops
the UV/original-face metadata. The returned project is version 2 with 2,522
vertices and 5,040 triangles, preserving positions/RGB while omitting its surface
field. The failed return gate is retained without relaxing its expectation or
rerunning it on the same binary.

The [machine-readable evidence](evidence/sculptgl-attributes-runtime-candidate1.json)
preserves all four additive run results: three original failed runs and one
passing malformed-only run. Ten distinct bounded case names have passing
observations; one OBJ UV-return case fails. These counts are not a feature-parity
denominator. Independent Python repacking validates all **96 actual downloads**
and **85 complete visible GPU/project pairs**, including the original 40-byte
geometry/normal/mask/RGB stream and the separate 8-byte material stream.

## Build and tooling boundaries

- Immutable candidate: `.artifacts/sculptgl-release-0.1.16-candidate1`.
- Source manifest SHA256:
  `69e971aa62613f66677740d9a7a4a28e82d81da71274c1f6b44dd3511deec5b2`.
- Build record SHA256:
  `007558df30d11a3786eb451bb441bc8a0786053722d567778b94bb421bdc05b8`.
- Frozen first-run helper SHA256:
  `9ac93b412dce1adfc65e257d74845ded4db2f341b3943b32d6eafc6759894bfa`.
- Corrected standard OBJ parser helper SHA256:
  `b78999c0e4ce1aa3bcec18e1e054c0609a5d9d8f5a8edfb098ebc16e0c9cebe7`.
- Narrow quad-export/detach helper SHA256:
  `7027f3ce1364d4e5d9f1ed5a384b4a01837f28a5e393b139d1646ada4ea98690`.
- Malformed preparation correction helper SHA256:
  `30e48104d5652918c322e76e50d89f890fb2ab05eeafee16d3cfff78636f0654`.

Every executed helper is copied into its retained run directory. The binaries
and frozen source were unchanged between runs. Every actual localhost
HTML/JS/WASM response matches its immutable file hash; the topology worker was
frozen but not requested. Material paint is outside this candidate's scope.

The initial OBJ parser incorrectly required exactly `v/vt` and rejected the
writer's valid `v/vt/vn` records. Its correction adds exact normal-value and
normal-corner checks while retaining all geometry/RGB/original-face/UV checks.
No numerical tolerance was changed. The corrected parser verifies the retained
610,997-byte actual OBJ against its independently read source project.

The third run's malformed preparation incorrectly requested Append for an
`.obrush` project. The host supports OBJ/PLY/STL mesh Append and rejected the
unsupported route before any malformed-file gate ran. The fourth run instead
uses normal Add Cube, keeping the inactive UV/material part and an active cube
with implicit material defaults. The original preparation failure and unchanged
recovery/redo gates are retained.

## Independently authored fixtures and GPU channels

| Fixture | Vertices | Triangles | Original faces | UV coordinates |
| --- | ---: | ---: | ---: | ---: |
| Noisy closed sphere with material rows | 2,522 | 5,040 | — | — |
| Same sphere with triangle corner UVs | 2,522 | 5,040 | 5,040 triangles | 15,120 |
| Welded grid cube with quad corner UVs | 1,538 | 3,072 | 1,536 quads | 1,734 |

Independent preparation verifies closed edge incidence of two, Euler
characteristic two, material ranges and each quad's explicit 0–2 render
diagonal. The triangle fixture contains 7,418 geometric-vertex/equal-UV-value
groups using distinct UV IDs; the quad fixture contains six such groups. Exact
corner-ID comparisons preserve those identities rather than welding equal
values.

Normal file input loads all three version 6 fixtures. Default positive Draw
uses radius `0.180`, strength `35%` and X symmetry. Each sample makes a useful
position edit, preserves fully masked original points and every original
material row, saves exact GPU channels and has exact whole-project undo/redo and
a quiet release. The materials-only mesh may refine with bounded new rows.
Original-face surfaces keep their triangle/index/UV metadata and vertex count
static, with adaptive detail paused.

Explicit material rows match a separate, enabled attribute-4 buffer at eight
bytes per vertex. Implicit legacy material draws have an empty, disabled stream
and `u_has_material_data = false`, using the bit-exact f32 default uniform
`[0.18, 0.08]`. The check covers actual draw buffers and uniform state, not just
an inspector label. It does not establish large-scene upload performance.

## Measured viewport response

The actual Studio clay/PBR/Normal combo, Flat shading checkbox and exposure
slider endpoints `0.1` and `8.0` are operated through normal controls. Cropped
1124×896 viewport screenshots exclude inspector and toolbar changes. All
selected modes/flat/exposure values also match the actual GL uniforms.

The declared pixel gates require at least 1,000 RGB pixels changed by at least
two levels, maximum channel difference of at least three and RMS RGB difference
of at least 0.5 on the 0–255 scale. All eight shader, flat and exposure pairs pass
for both legacy defaults and explicit nonuniform materials. Independent Pillow
12.3.0/NumPy 2.3.5 decoding reproduces the JavaScript PNG reader's counts and RMS
values within `1e-12`. The smallest observed response is the Normal smooth/flat
pair: 436,148 changed pixels, maximum difference 63 and RMS
`2.915379051455049`. Projects and camera remain exact throughout.

This establishes meaningful local pixel response, not matched SculptGL material
curves, radiometric correctness, environment-map/texture lighting or reference
visual parity. The renderer is an independent direct-light implementation.

## Actual exports, detach and failed-file recovery

For both UV fixtures, all five normal format selections—OBJ, ASCII/binary PLY
and ASCII/binary STL—reject attribute loss by default without starting a mesh
download. The source project and existing redo remain exact. Normal explicit
allow produces actual files and exact omitted-channel reports. OBJ omits only
the material rows; PLY/STL also report the original faces, UV coordinate and UV
face counts. Export does not remove these channels from the source document or
consume redo.

Independent byte parsing checks OBJ positions/normals/RGB and all face/corner UV
words, PLY positions/normals/RGB/indices, and STL triangle coordinates, normals
and declared colored attributes. The complete OBJ consumer return still fails
as described above. Masks, layers and history are documented mesh-format losses;
this is not a complete material-file exchange gate.

Normal Remove UVs and original faces preserves geometry/RGB/masks and material
rows, enables adaptive detail, and restores exact original seam metadata through
whole-project undo/redo. Four independently malformed version 6 files—short
material rows, out-of-range material value, invalid UV index and wrong quad
diagonal—are rejected through actual file input. Each preserves the complete
two-part scene and prior redo, including both explicit and implicit material GPU
draws. Arbitrary layer/Morph/multires failure state and the full allocation corpus
are outside this browser sample.

Only owned isolated headless Chrome 155.0.8059.40 profiles and localhost servers
were used, at 1600×1000 and DPR1. Nonlocal requests were blocked; all input events
were trusted. Every owned Chrome exited with code 0, all four PIDs were confirmed
gone and all servers closed. No app/mesh/tool/camera state assignment, user tabs,
user profiles, native GUI or accessibility control was used. Release probes
include asynchronous GPU reads and make no latency or performance claim. Native
GL, physical devices, material paint and matched reference response remain
separate gates.
