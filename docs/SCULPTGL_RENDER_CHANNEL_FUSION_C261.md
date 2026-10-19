# C261: witnessed Source upload channel scan

The selected Source renderer already receives a controlled-dab chunk witness
for position and normal changes. It previously walked the complete mesh once
to find mask/RGB edits outside those chunks, then separately compared the raw
mask snapshot and material rows. The renderer now checks the exact raw mask and
material words during that existing complete attribute walk. The fused result
is used only when a selected-chunk witness exists and cache lengths and mask
convention match. Without a witness, the independent full-channel comparisons
remain in force. An out-of-range or changed channel cannot authorize reuse.
The public mesh, geometry math, history and saved format are unchanged.

A focused test checks unchanged channels, a signed-bit raw-mask edit outside
the geometric chunk, a distant material edit, and the no-witness fallback.
`cargo fmt --all -- --check`, focused app test, strict native and WASM app
Clippy, and `scripts/build-web.sh` passed. The build used the lockfile-matched
wasm-bindgen CLI 0.2.129 from the main checkout's ignored `.tools` directory.
The first build attempt without that path failed with the script's CLI-version
guard; no artifact from it was used. An earlier strict Clippy attempt exposed
the Source Move `collapsible_if` warning from the base commit. After the
separate `c52ebc5` fix, strict Clippy passed without suppressions.

A normal Chrome 10,114-vertex imported-Source Draw stroke changed 109 saved
position rows and preserved the other public channels. Before/Undo and
After/Redo project files are byte-exact. An on-demand active WebGL2 VBO
readback found one 404,560-byte draw buffer, zero GL errors and a position-word
SHA-256 identical to public Save (`4e15c1a67b1290039a5a33ff30cac201b1da5e3628f2245c4fc0020987f8258d`).
The displayed normal-word SHA-256 was
`f1052f89299dd239fe02a19a0a70bca8730a1daa02f0e9ad38954541ecd5e947`;
this is a captured output, not a paired normal-parity comparison. The
[browser result](../.artifacts/sculptgl-ui-brush-pair/root-69e1d3d7-b199-4b1e-953b-20457ab11e70/result.json)
has SHA-256 `e1d576ecbf3dcd1aa619e0b05705ccd7de8a5db858c68aad9eea558a8bdea795`.

Before running performance trials, an ignored local
[preregistration](../.artifacts/dense-render-stage/bench/RENDER_FUSION_PREREG.json)
fixed a 99,602-vertex / 199,200-triangle independent SGL fixture (SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`),
the original 60-second 400-requested-moves/s protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`),
and baseline-then-trial order. The baseline was the same code with the renderer
edit absent; the trial was commit `9879622`. Portable WASM hashes were
`4635cca85d45be608459c6b522b3a03a0cf5f26570897ca15747f7a693d221bc`
and `85614e910b5d2c594f790474d99cf98f8c95171c0f19cb1f89c089553795a710`.

| Observed normal-browser 100k/60s measure | Baseline | Trial |
| --- | ---: | ---: |
| Applied dabs | 1,143 | 1,144 |
| Renderer upload mean | 0.5112 ms | 0.4336 ms |
| Renderer callback mean | 1.7336 ms | 1.6017 ms |
| Active RAF P95 | 16.7 ms | 16.7 ms |
| App RAF CPU P95 | 9.8 ms | 9.1 ms |
| Release acknowledgment | 14.7 ms | 17.5 ms |
| Active responsiveness gates passed | 4/4 | 4/4 |

Both runs completed exact public Save/Undo/Redo with unchanged non-position
channels and topology, finite useful edits, no new affected-face degenerates
or reversals, no application/console errors, and clean owned-Chrome closure.
The independent [baseline audit](../.artifacts/dense-render-stage/bench/baseline-audit.json)
and [trial audit](../.artifacts/dense-render-stage/bench/trial-audit.json)
passed every check; their SHA-256 hashes are
`9d7c792437bde79e765915bceff2aa4ea28d2a67c6078a7baeb99abc4f1579b7`
and `0da8bdae74bb9b27b33fb2a53a881f594ad9388b0052789ae061426ba6bf4343`.
The full [timing summary](../.artifacts/dense-render-stage/bench/pair-summary.json)
has SHA-256 `fbc37d7c2725c5fd564960fa890ae96793c4e9a1156571e3c2c594b64b1873c3`.
The [baseline result](../.artifacts/dense-render-stage/bench/actual/SP-LONG00-100k-60s-root-35a34f43-5bd2-4973-bbd2-a7b658549a3b/result.json)
and [trial result](../.artifacts/dense-render-stage/bench/actual/SP-LONG00-100k-60s-root-88c0eada-4dc6-4a4c-97f1-62ae76552a70/result.json)
have SHA-256 `ae484c7a29c837fece8b0eaf4e16b4b2e07aa09101e55bc98a95205b6b4713df`
and `a05ba39c816c0219e28182c6202f98aa1647f43f81803f27342906cb7edf6247`.

These are separate browser runs with one different applied dab and potentially
different scheduling. The lower observed upload mean is consistent with the
intended scan consolidation, but is not a causal speed estimate. The 100k
cohort was already within all four interaction gates at baseline. The
million-vertex run and a paired full active-GPU normal audit remain open;
SculptGL and ZBrush acceptance gates do not change.
