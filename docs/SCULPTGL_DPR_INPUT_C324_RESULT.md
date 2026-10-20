# C324 failed keyboard setup

The frozen C324 integrated-browser run used product commit `3644908`, official WASM SHA-256 `a3a7503ced76fecf7dee2e8270a1c655b3eafad543053fb80e1887a806743967`, independent C307 fixture and pinned SculptGL mirror at CSS 1600×1000, DPR1/2. The Save, exact resolution slider, canvas backing and scene-target portion passed again. After selecting the sphere and explicitly focusing the DOM canvas, the normal CDP `W` keydown/up did not turn the published `symmetry_x` false within 15 seconds at either DPR. No mouse or pen contact was attempted; keyboard setup and placement remain unverified. The DOM focus assertion passed, which does not prove egui gave keyboard focus to the viewport.

- DPR1 result: `.artifacts/sculptgl-dpr-input-c324/73a061e9-5f75-42e3-92ce-034959d8cdf8/result.json`, SHA-256 `17f1059f0b077485a5402f6a101edfb910ec924aa7e56996954a0b34d70d9779`.
- DPR2 result: `.artifacts/sculptgl-dpr-input-c324/5d636c2f-7036-4b7e-981d-d8420245543b/result.json`, SHA-256 `ea93c18206f9386b0037061afc964723958819f14a498853e303b0c97c949120`.

Keep C324 failed. Any follow-up must establish a working, visible symmetry control route with published-state verification before measuring contact; it cannot infer brush placement from these runs.
