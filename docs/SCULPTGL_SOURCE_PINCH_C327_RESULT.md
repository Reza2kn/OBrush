# C327 Pinch four-run capture and failed paired auditor

Four fresh ordinary-browser C327 captures completed on integrated product
commit `ce0f0de`, with the [C327 setup-only protocol](SCULPTGL_SOURCE_PINCH_C327_PREREG.md),
the pinned SculptGL hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and the independent 2,401-vertex fixture SHA-256
`f864eb78bf6f280c92fcaa3debcaa406c9839a82e861d213e396233ac6269144`.
The official web build passed with wasm-bindgen 0.2.129. Its actual portable
WASM/JS SHA-256 values were
`a3a7503ced76fecf7dee2e8270a1c655b3eafad543053fb80e1887a806743967` /
`99df924c02cc595ec609249677f773bf6e7ac664938f3dc9991b8cbf64f10f90`.

| Case | Ignored raw receipt directory | Result JSON SHA-256 |
| --- | --- | --- |
| Reference Negative off | `.artifacts/sculptgl-source-pinch-c327-ui/source-off-6783979b-c517-4644-9bcd-8a135736c56c/` | `5eae268adcc74361330c1450e81fea5714c87b138497e5749563c5e1d34faa62` |
| Reference Negative on | `.artifacts/sculptgl-source-pinch-c327-ui/source-on-3336d206-dd32-4386-a324-3481a3f4fb3e/` | `41cea2e74f4066209399b6c3430d188a404f70fe41e437f7c14ca9c59e1df97e` |
| OBrush Negative off | `.artifacts/sculptgl-source-pinch-c327-ui/root-off-c51a56c4-b397-425a-8ff8-e6bd23d15f5c/` | `2b06a50e2ae8fab4f64c35808716016402151ab78a7340228f5c8d853222dfd6` |
| OBrush Negative on | `.artifacts/sculptgl-source-pinch-c327-ui/root-on-a0dec72b-a56b-44d6-aae8-a1ffe843a881/` | `bae68a1f4119c22715769ad1fa90f9966816fa4150c516d60f6fab93368674a0` |

Each side recorded public Before/After/Undo/Redo and one new Undo step. Both
reference press probes reported zero picked vertices; the OBrush press had no
`last_pass`. The held move was the first measured effective dab: reference
reported 41 picked vertices and OBrush reported a valid `last_pass` in each
polarity. The frozen C308 paired auditor threw `root: missing measured picked
centers` for both polarities before writing paired JSON. That failure is
retained and **no paired geometry, GPU-normal, mask or parity result is
claimed** from C327. A separately frozen audit-only correction may assess
the effective move using these raw captures; all original numerical bounds
remain in force.
