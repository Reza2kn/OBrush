# C281 read-only Brush Culling picked-face replay

C281 used the separately frozen read-only harness SHA-256
`e2bcb2f4fc5cd7073b058836946809ef33c4679a78ff53ff7646703f1bc21561`
and audit SHA-256
`4bad417fb1e2690dee6e8d1ba5ef9d98de429c4e068ecc9632e0d6d7a1f7a3ac`.
The fixture, pinned Source bundle, visible Brush settings, exact trusted
integer gesture, setup bounds, public Save/Undo/Redo and active GPU gates
were unchanged from C280. The OBrush WASM and JS were byte-identical to
C280 (`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`
and `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`).
The served web repository HEAD was
`adfcd05b38ac8e6148132c22b17679e77eaf5940` but the observed build
hashes, not the later HEAD, identify the product bytes. No product code
changed for this replay.

Raw completed runs:

- Source on: `.artifacts/sculptgl-brush-culling-c281-ui/source-on-e3189044-6249-4020-ae31-d0de6f5b09e2/result.json`
- OBrush on: `.artifacts/sculptgl-brush-culling-c281-ui/root-on-d46d9f57-f99d-49eb-9c6d-432e2a2b43ba/result.json`
- Source off: `.artifacts/sculptgl-brush-culling-c281-ui/source-off-d8f0f6ff-5d62-4cfc-ad58-169413108852/result.json`
- OBrush off: `.artifacts/sculptgl-brush-culling-c281-ui/root-off-8a160a1a-05d4-4d3e-bf9c-8222d88c5073/result.json`
- Frozen audit: `.artifacts/sculptgl-brush-culling-c281-ui/source-on-e3189044-6249-4020-ae31-d0de6f5b09e2/paired-audit-root-on-d46d9f57-f99d-49eb-9c6d-432e2a2b43ba-with-off.json`

Two setup failures remain separately preserved: the first launch omitted
`OBRUSH_SHARED_ROOT` and could not resolve the local reference driver
(`.artifacts/sculptgl-brush-lock-ui/c274-checks/c281-source-on.log`); the
first OBrush-on run captured geometry/history but failed the harness-owned
single-side server closure admission when it used the preexisting external
port (`.artifacts/sculptgl-brush-culling-c281-ui/root-on-ab49898b-b80a-4065-952b-397e9d8cd8e3/result.json`). The successful OBrush runs used an owned
server serving the same measured WASM/JS bytes. No fixture, trace, settings,
acceptance tolerance or harness hash changed in these retries.

The C281 replay **fails its preregistered diagnostic hypothesis**. Source-on
directly picked face index 206, RGB IDs `[200,222,221]`, at
`[0.047540128231048584,-1.862645149230957e-8,0.021131693027200527]`
with normal `[0.010458265947246385,-0.002207766062001845,0.9999428735894826]`.
Source-off also picked face 206 with the same RGB IDs, at Y
`-2.7939677238464355e-9`. OBrush on/off picked the corresponding lower
face RGB IDs `[200,222,221]` at Y zero. The direct normals agree with
the second-held GPU displacement directions within `7.1e-7` for Source and
`8.0e-7` for OBrush. C280's earlier Source-on upper-face selection was
inferred from its displacement and first-held GPU normals, but C280 did not
record the direct picked-face field; C281 cannot retroactively prove it.

Because all four C281 runs chose the lower face, this separate replay's
paired geometry was close: on-state maximum public XYZ difference
`2.421438694000244e-8`; off-state `1.6763806343078613e-8`. The original
C280 second-held/on failure of `3.120319831850793e-6` remains valid and
unreplaced. The exact-edge fixture exposes run-to-run Source face selection
near Y zero under tiny normal-UI fit/pick variations. It does not establish
a deterministic culling kernel gap or justify a geometry tie-break change.
The C281 hypothesis failure, C280 numerical failure and full Brush Culling
parity row remain open.
