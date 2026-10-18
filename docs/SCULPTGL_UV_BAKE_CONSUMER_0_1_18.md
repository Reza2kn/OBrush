# UV PNG consumer checks in 0.1.18 candidate2

Normal browser controls exported 26 independently checked PNG files on the frozen 0.1.18 candidate2. The five output sizes (256, 512, 1024, 2048 and 4096) each passed Color, Roughness and Metallic output checks. These are local OBrush workflow checks. No SculptGL rendering or baking family is promoted to parity.

The [machine-readable receipt](evidence/sculptgl-uv-bake-consumer-0.1.18.json) records the exact candidate, WASM, helpers, preregistrations, fixtures, download hashes, numerical gates and cleanup. Read [FEATURE_PARITY.md](FEATURE_PARITY.md) for the distinction between foundations and reference parity.

## Actual consumer scope

Each case loaded an independently authored project through the normal file input and used the rendered UV size/channel controls. The helper retained the actual browser download, parsed PNG signatures, chunk CRCs, bounded compressed/decoded lengths and filter modes independently of the Rust exporter, and checked the output against authored analytic fields. Passive observation forwarded native Worker calls without changing jobs, clocks or messages. Saved projects, prior redo states, document revisions, dirty flags, brush preferences and the complete visible GPU channels were checked around accepted exports.

| Workflow | Actual coverage | Result |
| --- | --- | --- |
| Five sizes × three channels | 15 full-UV affine exports; every RGBA pixel, orientation and alpha | Exact |
| Welded geometric seam with different face-corner UV indices | Three 256px channel exports | Every covered pixel exact; 21 specified edge/gutter/hole probes per channel; only uncovered points admit one RGB byte, alpha exact |
| Protected and Unprotected mask conventions | Six 1024px exports plus channel comparison | Full image exact; mask protection does not darken exported channels |
| Implicit material storage | Two 1024px exports | Default roughness byte 46 and metallic byte 20 exact, without material allocation |
| Missing UV data | Normal export controls unavailable | Document, redo and GPU preserved |
| Work cap | Nine overlapping quads at 4096 exceed 268435456 candidate raster steps | Normal rejected job; no accepted PNG, source and redo retained |
| Active-job Cancel | Seven overlapping quads at 4096 | Owned native worker terminated, no partial/successful PNG, exact source/redo/GPU retention |
| Camera/radius/style/next-size response during work | Separate prompt-response helper, same seven-quad fixture | All four actions observed while the same 4096 job ID was still busy; Cancel and retention gates passed |

The accepted exports contain 75,628,544 decoded pixels; 75,542,528 covered pixels had exact independent comparisons. The 63 selected edge/gutter/hole probes contain 12 covered points and 51 uncovered points. Only those 51 uncovered points admit one RGB byte of difference; covered points also pass the full exact image gate. All alpha checks require 255 exactly. The runtime receipts include 171 registered downloads and 145 saved-project/GPU pairs in the original run, plus eight pairs in the separate prompt-response run.

## Preserved failures and separate follow-up

The original full helper remains **failed**. It completed 31 passing case observations and 26 accepted PNG exports, then failed the final requirement to observe all four response actions while the same worker was still busy. Its normal UI routing took about 2.819 seconds after starting the job; the worker had completed by the final observation. This records missing simultaneous coverage, not a proved numerical defect or an input-latency claim. Its unexpectedly completed 4096 PNG is retained separately and is excluded from the 26 accepted exports.

A separately frozen helper performed the same required four normal actions promptly, beginning with the next-size setting while those controls were already visible. It waited for actual popup/value/visible-control acknowledgments instead of fixed helper pauses. The fixture, captured 4096 request, same-job criteria, source/history/GPU checks and Cancel requirements stayed unchanged. This follow-up passed. It does not retroactively change the original failed result or establish physical performance.

The earlier observer-template source preparation error also remains preserved. Both actual runs closed their owned Chrome process with exit 0 and closed the owned localhost server; user tabs and port 8787 were not operated.

## Remaining gates

The subsequent [independent consolidation](evidence/sculptgl-independent-runtime-consolidation-0.1.19.json) completed a second full decode and reconstruction. Pillow verified actual PNG signatures, dimensions, CRCs and RGBA data; independently written NumPy analytic fields matched all 75,542,528 covered pixels exactly across all 26 files. The same declared gutter probes retained their one-byte bound only for uncovered RGB points, with alpha exact. Python separately reconstructed all 304 saved/GPU pairs across eight named raw runs, including the 153 UV pairs, without changing any original failed status. The original machine-readable UV receipt remains a historical record of the earlier pending second pass.

Exact paired SculptGL TexBake output, GPU raster edge/overlap/quantization/gutter behavior, UV generation, native GUI, persistent project textures, full memory bounds and physical input/performance remain separate requirements. The passive worker observation windows are not benchmark results.
