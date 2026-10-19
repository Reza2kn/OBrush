# Source Worker scene-witness phase gate

The read-only phase probe in commit `e46de05` ran the first step of
[the preregistered design](SOURCE_WORKER_TYPED_SCENE_WITNESS_DESIGN.md).
It used the frozen SP-WORKER-01 100k protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`
and independent SGL fixture SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The actual browser WASM SHA-256 was
`35dce7cc1334d4cebac53d08948ef69b20fc0031ca88469e7eba15c636183779`,
binding JS `cf80c1750a12007beadfd2c032a25ebb54865e8bf9f6e177a5a56f5b1277af30`,
and held-input JS `f804ad97824def3a35443f158b1d9a37e0dccc6f0067e1b62e35b8e4adac1bb3`.
The raw result SHA-256 is
`2f02b002ab6838c9058e3f66f2e3b4f600ceecf9734a2910b4ff6125afad34e8`.
Raw browser, shape, GPU, static-normal and failed frozen-verifier receipts
remain under
`.artifacts/source-worker-sustained-spw01/100k-worker-dd329384-9ee2-448a-8fed-4526d40a2374/`.

| Complete-scene witness phase | Measured time |
| --- | ---: |
| Owner/epoch/revision check | <0.1 ms |
| Complete-state validation | 10.6 ms |
| Level/layer reconciliation | <0.1 ms |
| Metadata/version/ProjectRef construction | 3.9 ms |
| JSON serialization plus BLAKE3 hashing | **251.1 ms** |
| BLAKE3 updates within the previous row | 23.2 ms |
| Total scene witness | **265.9 ms** |
| Delta decode (separate) | 13.2 ms |

Hash time is included in serialization time, not additive. The remaining
JSON formatting/write portion is about 227.9 ms. Thus serialization exceeds
the preregistered 150 ms admission threshold. This establishes that a
security-equivalent typed event encoding is worth testing; it is not evidence
that such an implementation will be faster or correct. No typed serializer
was implemented in this diagnostic. The complete Rust final publication was
470.9 ms, JS callback 473.5 ms, normal begin call 405.9 ms, and release to
public commit 680.8 ms on 1,290 acknowledged dabs.

The frozen verifier remains **incomplete_or_failed**: active RAF maximum
466.6 ms, no proven empty Worker queue by 100 ms, and the raw public-mesh
normal GPU oracle failed in all five states. Pacing P99/max passed at
1.07/6.29 ms, RAF P95 was 16.7 ms and app-frame CPU P95 10.6 ms. The
independent public Save audit passed exact Before/Undo and
After/Stable/Redo bytes. Shape audit passed with 4,011 changed vertices,
zero new degeneracy and zero orientation reversals. GPU position words
matched all five public Save stages; static Source display normals matched
GPU Before and Undo. The final endpoint was applied, one public commit
occurred and browser errors were empty. Edited display normals still lack
an independent canonical oracle.

Focused and full workspace tests, strict native/WASM Clippy, `cargo fmt`
and a fresh web build passed. The held Worker remains query-gated, off by
default and unpromoted. This phase result does not establish SculptGL or
ZBrush parity.
