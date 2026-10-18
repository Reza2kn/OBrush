# C205: rejected separate-channel Source upload cache

The C204 renderer profile identified 16.22 seconds of mesh-upload CPU in the
normal-browser 999,002-vertex / 60-second Source Brush trace. An attempted
cache layout stored exact positions, selected normals, raw masks and colors in
separate arrays, then compared each channel by bytes over 2,048-row chunks.
Changed chunks still used the original 40-byte packed GPU rows. A focused
native test checked exact reconstructed output and a mask-convention fallback.

The trial used the unchanged frozen input/protocol hashes in
[C204](SCULPTGL_RENDER_FRAME_PROFILE_C204.md) and browser WASM SHA-256
`aac4a5524572133d960df5402f448497451fc57bc13cad30921c15324939c29a`.

| Measured 1M/60s observation | C204 original cache | C205 separate-channel trial |
| --- | ---: | ---: |
| Applied dabs | 838 | 728 |
| Mesh upload CPU | 16.22 s, 12.95 ms/callback | 21.72 s, 19.02 ms/callback |
| Scene admission CPU | 10.08 s | 9.89 s |
| Complete renderer callback CPU | 27.78 s | 33.03 s |
| RAF interval P95 / maximum | 75.6 / 633.2 ms | 86.8 / 620.7 ms |
| Release acknowledgment | 144.3 ms | 186.8 ms |

The full browser file/history/shape audit passed, including 40,753 changed
vertices, 82,326 affected faces, no new degenerates or reversed affected
faces. Its raw result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-24b9a920-8053-4322-ac02-7fd552f59b68/result.json`
(SHA-256 `6de82692968f10baedadc0e07b3687ae7176233a3fd919608685601b05c9fe1a`);
the independent audit `.artifacts/c205-audit-1m.json` has SHA-256
`02e0552474bb8ef64bcf3e2f35ec63539cd4aa84c7876b3870f20ec209d91063`.

Different contacts and load prohibit a causal end-to-end speed ratio, but the
directly timed target stage increased materially. The cache-layout change was
removed. The rebuilt production browser WASM returned exactly to C204 SHA-256
`9fb8e028cfeb2d7488701082fa5bfed14097f7bfbaf8b9667f89b5900925e765`.
All four million-vertex responsiveness gates and SculptGL parity remain open.
