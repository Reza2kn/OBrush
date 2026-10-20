# C321 failed setup gate

The unchanged integrated DPR product `3644908` and official WASM `a3a7503ced76fecf7dee2e8270a1c655b3eafad543053fb80e1887a806743967` passed the C320 resolution/Save portion at DPR1 and DPR2, but C321 stopped before any mouse or pen contact. After selecting the sphere, the normal `X` key event left `symmetry_x=true`; the frozen assertion required `false`. Draw, radius and strength read `Draw`, `0.18000000715255737`, and `0.3499999940395355`. This is a test setup/focus failure, not evidence of brush placement.

- DPR1 result: `.artifacts/sculptgl-dpr-input-c321/ddec8e62-62b4-4a75-9f9b-8efe6ed73a5f/result.json`, SHA-256 `6add4f5d25b9fb0815b45033630adf4eca68e9fc8ddc3d7fb9b5bc6d7851ded5`.
- DPR2 result: `.artifacts/sculptgl-dpr-input-c321/948d7774-fc5b-4721-bc1d-a992dd578f5f/result.json`, SHA-256 `467248cd1c30967424e228207a891c87890d5c02bb47dd18b8a2e03af11f6a8c`.

Both results remain failed. C322 will use the visible checkbox, with published state verification, and will not alter these receipts.
