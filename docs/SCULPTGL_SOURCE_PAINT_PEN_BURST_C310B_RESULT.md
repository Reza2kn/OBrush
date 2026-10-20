# C310b Source Paint same-frame pen burst — scoped result

The independently frozen C310b normal-browser pair passed all 13 audited
checks. This establishes an ordered held-pen pressure effect for ordinary
Source Paint on the pinned 10,114-vertex sphere. It does not establish
physical stylus delivery, native pressure, general brush parity, or exact
cross-app painted color parity.

The reference was SculptGL `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
(host bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`).
The common independent fixture was
`tests/fixtures/independent-id-sphere-c310.sgl` (687,848 bytes, SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`).
OBRush's official browser WASM had SHA-256
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.
The exact harness and auditor hashes are frozen in
`SCULPTGL_SOURCE_PAINT_PEN_BURST_C310B_PREREG.md`.

At 64 px radius and 25% intensity, with pressure intensity factor 1,
changing the first same-frame pressure from 0.25 to 0.70 while holding the
second at 0.75 changed saved RGB at 85 original SculptGL vertices and 98
OBRush vertices. Their maximum single-channel differences were 0.11205256
and 0.11376923. With factor 0, both pairs were bit-identical in their own
public color arrays. Two trusted held pen moves shared a browser app frame;
OBRush's applied-dab count rose by exactly two (1→3, 4→6, 7→9, 10→12),
with no release dab. All four traces kept original positions, faces, masks
and materials unchanged. Active GPU color words matched each corresponding
public save at Before, After, Undo and Redo. Each OBrush stroke added one
history entry and one normal Undo/Redo exactly restored Before/After.

The 257-move OBrush negative control reported one overflow and restored
identical before/after `.obrush` bytes, history depths (undo 14, redo 0),
and saved paint display state. The direct browser observation contains the
trusted event transcript, public files, GPU word captures, UI actions, and
build hashes:

- Source result: `.artifacts/sculptgl-source-paint-pen-burst-c310b/source-eb4f8ef6-7ffc-4ae6-842b-36760ef4c201/result.json`, SHA-256 `bbd9763eb47626cf82cc53dbbd5085b4421861dc7f7e4d7531c209cbd7776b27`.
- OBrush result: `.artifacts/sculptgl-source-paint-pen-burst-c310b/root-a9356d39-1971-4369-b9aa-b6c9674adf8f/result.json`, SHA-256 `feae82bdee902ae26cf8359b29488a65d0598bad3396f51ac05b0928d8bb0514`.
- Independent audit: `.artifacts/sculptgl-source-paint-pen-burst-c310b/source-eb4f8ef6-7ffc-4ae6-842b-36760ef4c201/audit-root-a9356d39-1971-4369-b9aa-b6c9674adf8f.json`, SHA-256 `ce77fbb6a3918c34aca94bc07a30c86200b64a5dda2d9b8be858e3d1fc8784b3`.

The first frozen C310 run remains failed and retained under
`.artifacts/sculptgl-source-paint-pen-burst-c310/`. Its audit SHA-256 is
`aca5c508acdca136417ca5d1020ceba747bf7a68a7399524ccf87d77dbdaf43b`.
That harness alternately re-enabled OBrush material channels across fresh
imports, causing 252 material-component changes in cases 2 and 4. It also
mistook SculptGL's internal `_undos.length` rise from 1 to 3 for two
user-visible undo steps, though one normal Undo/Redo exactly restored the
saved states. C310b reads the channel state after normal UI actions and
audits the user-visible Source Undo/Redo, while retaining OBrush's strict
one-entry history depth gate.

Validation: `cargo fmt --all -- --check`, full locked workspace tests,
strict native and WASM Clippy, and the official `scripts/build-web.sh`
completed successfully. After the read-only C310b metrics addition, strict
native/WASM Clippy and the official web build passed again. A redundant
full-test rerun was stopped after the earlier full pass; its partial log is
retained at `.artifacts/c310b-cargo-test.log`.
