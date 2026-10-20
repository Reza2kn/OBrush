# C322 failed visible-control setup

The unchanged integrated product `3644908` with official WASM SHA-256 `a3a7503ced76fecf7dee2e8270a1c655b3eafad543053fb80e1887a806743967` passed the earlier C320 Save, slider, backing and scene checks again at DPR1/2, but C322 stopped before any mouse or pen contact. Its `settings.symmetry` checkbox remained outside the visible left panel after the frozen wheel attempts. The final published rect was `[15,1343,95.6875,1361]` at DPR1 and `[15,1351.5,95.6875,1369.5]` at DPR2. No stroke placement was measured.

- DPR1 result: `.artifacts/sculptgl-dpr-input-c322/0d548136-9391-4997-8522-0cf441741689/result.json`, SHA-256 `920b5a01ed94fcd646d1786b03eac7c20761da4c83be8936f8bbaaa329618228`.
- DPR2 result: `.artifacts/sculptgl-dpr-input-c322/d997e30b-63c3-4620-8cb4-1ad02adfe86c/result.json`, SHA-256 `830314d5575a847e16e710ddc77fe4546a1e6df59d4d10d07b20a8f07990e803`.

Static source inspection then identified a deterministic setup correction: `X` invokes frame scene; `W` toggles X symmetry. C321 had used `X`. C324 freezes canvas focus plus the normal `W` shortcut and requires the published symmetry value to become false. C322 remains failed.
