# C260 integrated Source primitive, Paint All and worker-codec validation

The combined checkout is `dea8e72` on `codex/sculptgl-parity-foundation`.
Pinned SculptGL source commit:
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; hosted bundle SHA-256:
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
All paired checks used the unchanged pinned local source and ordinary browser
controls, unless a linked report says explicitly that it used a method-level
reference fixture.

This integration adds retained Source-default primitive levels (`4/4/3/3`),
the pinned seed-normalization pose on each level, and a reachable Source
Paint All operation with per-channel soft-mask blending. It also adds a
bounded, **non-publishing** sparse position/normal packet codec in shared Rust
core. The codec is not an ordinary browser Worker or a performance result.
The [level report](SCULPTGL_PRIMITIVE_RETAINED_LEVELS_C258.md), [placement
report](SCULPTGL_PRIMITIVE_PLACEMENT_C259.md), [Paint All report](SCULPTGL_SOURCE_PAINT_ALL.md),
and [worker seam report](SOURCE_WORKER_SPARSE_CODEC_SEAM.md) give fixture
parameters and exact limits. A [paired radius audit](SCULPTGL_BRUSH_RADIUS_SCREEN_C259.md)
corrects the stale claim that current Source Brush uses world-space radius;
its one aligned browser pair changed the same 87 vertex IDs.

## Source and artifact checks

`cargo fmt --all -- --check`, the generated 317-ID inventory check, the
complete `cargo test --workspace --locked --quiet`, strict native workspace
all-target Clippy and strict `wasm32-unknown-unknown` workspace-library
Clippy passed. The final workspace run reports **2,582 passed tests in 90
groups, zero failures, one ignored fixture**. Its log is
`.artifacts/combined-c260/workspace-tests-final.log` (SHA-256
`c727fab9e2b21797912f74928346723375ceffdcacbeb276aff5e4b109ca5121`).
The previous complete test log also showed no test failures, but its shell
wrapper returned error after Cargo finished because it attempted to assign
zsh's read-only `status` variable; the corrected final command exited zero.

`scripts/build-web.sh` passed with lockfile-matched wasm-bindgen CLI `0.2.129`.
The portable browser WASM SHA-256 is
`66f80111dd6c871f557050037cdbba9068828220f5da430c4c501d323d15fb90`,
optional SIMD WASM SHA-256 is
`2d954b8e0a0356301642e241e3069cd3b988e537a31ab28d2d81b41abc51b95b`,
and GPU WASM SHA-256 is
`ab5beac5c7a53349243bbab4f10c065764b9010727802433bfaf5ce2904a5a77`.
`scripts/package-macos.sh` passed and `codesign --verify --deep --strict`
accepted `.artifacts/OBrush.app`; its executable SHA-256 is
`67ff57a0156f7e42edf74764154e94dbbe6797c7b80bbb63808a343dfaf7cd9a`.
The ordinary browser server at `http://127.0.0.1:8792/` returned HTTP 200
after the build. The native GUI was packaged and signature-checked, not
manually exercised in this run.

## Combined normal-browser checks

| Check | Result | Complete receipt SHA-256 |
| --- | --- | --- |
| Source Paint All visible control, soft-mask RGB/material, exact geometry | [pass](../.artifacts/sculptgl-paint-all-browser/baabab71-6db4-4fd8-ba19-5b940455263a/result.json) | `9cf55a41a46c4dc57e5b4676516a2e47a9be1b042b809acae3541a0d6c03e691` |
| Four default primitive poses/world bounds, multi-part Undo and byte-exact Save/Open | [pass](../.artifacts/sculptgl-primitive-placement/ddf43435-71ec-4535-bc03-af2ec4fcf7ec/result.json) | `34df2cd2c6695156650caff766475504bf3f7cac993b611fd5ee55301e141cdb` |
| Add/Undo/Redo/Clear, `4/4/3/3` retained levels, dense Cube Save/Open/Lower/Higher | [pass](../.artifacts/sculptgl-primitives-browser/aaea8526-47f4-4e49-8946-d79b9bed8f80/result.json) | `a01086b8cc3bf0a134c10746426a826f6cb38a6a720e64a39c7707e613f0ecc2` |
| Full 98,306-vertex normalized Sphere shape and normals | [pass](../.artifacts/sculptgl-primitives-browser/0f66e9d6-b1cd-4148-aff8-86683aa1c700/result.json) | `a9994134099969d03fd3bd3a3de17c2d307b476f315b53ab71bcf3984f7b17b1` |
| Source mirror offset/guide and reference mirror pick | [pass](../.artifacts/sculptgl-symmetry-offset-c255/e4001ebc-c739-4442-b3d1-b6d8a3cccdbb/result.json) | `dd785b9d395a0e2d42819a4f5fc52d8868e6b26c69c90d770723c3f5936986dd` |
| Cylinder/Torus local shape and saved-affine world bounds | [pass](../.artifacts/sculptgl-cylinder-torus-shape/2558e997-a345-408f-8013-29574b649168/result.json) | `196f2339c8b1f8604b7c0dcb562e8b1dd15c449cb9a7b15ce1abefe032a83751` |

The sphere's maximum normalized position difference was `2.183e-7` and
maximum normal angle `0.02762°` on this run. First-Add primitive world-bound
errors remained below `9e-7`; Cylinder/Torus full shape world-bound errors
were `3.690e-7` and `2.180e-7`. The first Cylinder/Torus combined receipt
compared SculptGL world coordinates to OBrush local coordinates; it is
preserved in the [placement report](SCULPTGL_PRIMITIVE_PLACEMENT_C259.md),
and the corrected harness checks OBrush's public saved affine.

These are scoped fixtures, not a SculptGL-family completion claim. All 309
active inventory acceptance gates remain open. Edited coarse-to-fine sculpt
behavior, paired pixel/pick response, Source mirror stroke behavior, full
Paint All reference GUI response, 100k/1m sustained held-pointer latency,
native GUI behavior and ZBrush 2026.2.1 parity remain unverified.
