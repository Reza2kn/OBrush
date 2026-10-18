# C206: rejected packed-row byte comparison

After the C205 separate-channel cache regressed, the original packed cache was
restored. A smaller trial replaced its ten scalar bit comparisons per vertex
with a byte comparison against the exact row returned by the already existing
`packed_selected_row` function. The signed-zero/attribute and sparse-range
tests passed. The frozen 999,002-vertex / 60-second normal-browser protocol
also passed the independent full-file, channel, shape and Undo/Redo audit.

The trial browser WASM SHA-256 was
`0df721618530b0911dc569f1312328d2c62653b6bc7a1105e93ebf01aa960c6d`.
The raw result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-80ba7202-4e28-4e46-97c5-36e2f5f8ed57/result.json`
(SHA-256 `404a8c4aceb6fe6ad5b73a58fec0be2b7aea8f5449d9940108d34687ab434680`);
its audit `.artifacts/c206-audit-1m.json` has SHA-256
`f0eb2fedc1473ffc8832de03b63517087a2992ea13c654fa4d56e28bf0aa70a4`.

| Measured 1M/60s observation | C204 original row comparison | C206 packed-byte trial |
| --- | ---: | ---: |
| Applied dabs | 838 | 850 |
| Mesh upload CPU per callback | 12.95 ms | 12.94 ms |
| Full renderer callback CPU | 27.78 s | 27.93 s |
| RAF P95 | 75.6 ms | 76.2 ms |
| Release acknowledgment | 144.3 ms | 142.2 ms |

Separate delivered contacts prohibit a causal end-to-end speed ratio, and the
directly instrumented target stage did not materially improve. The trial was
removed. A rebuilt browser WASM matches the C204 production SHA-256 exactly:
`9fb8e028cfeb2d7488701082fa5bfed14097f7bfbaf8b9667f89b5900925e765`.
The full-mesh admission and upload scans still need a stronger solution; all
four million-vertex responsiveness gates and SculptGL parity remain open.
