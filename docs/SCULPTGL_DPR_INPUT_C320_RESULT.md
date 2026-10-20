# C320 scoped integrated-build result

On integrated product `3644908` (which retains C310b Paint pressure) and official WASM SHA-256 `a3a7503ced76fecf7dee2e8270a1c655b3eafad543053fb80e1887a806743967`, the frozen C320 normal-browser fixture passed its DPR1/2 dimensions, visible Save, exact slider, public project, history, screenshot and cursor-ring checks. The source fixture was the independent C307 367-byte OBJ, SHA-256 `fab0a005d7c4cb666d4c17f1b6a5d115a4051abfcd0df254182c591307af2300`; the pinned SculptGL bundle was SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

| DPR | Result SHA-256 | Cursor analyzer SHA-256 | Canvas backing | Source CSS scene | Source host scene |
|---|---|---|---|---|---|
| 1 | `f487880e2c9535cdc56a46a3a2cb6ce5d8f5ac4b6931a8fffb7982b50cc63402` | `68b900e0c04e0b39eb30be8f1c4b2e2acb85ebe6b6b25ec0a78ccf6a2b9d6fd8` | 1600×1000 | 1140×912 | 1140×912 |
| 2 | `b80894bb695d0eb51b07386888333169e7cf1d276a93db39e1620ec98639a4db` | `42c821a0a951ae523b67249e5aeb6272a94bc447f2381106bd407c7b16e69c1b` | 3200×2000 | 1140×912 | 2280×1824 |

Both browser receipts are under `.artifacts/sculptgl-dpr-input-c320/`. All four visible Save downloads for 1→0.5→2→1 were byte-identical within and across DPRs, SHA-256 `be93583af23807d3457cf6b7f41dcb539311fa1103b31896338bc1f0cfbc69e3`; scene-target dimensional errors were [0,0] at each ratio. The independent ring analyzer passed its scoped CSS radius gate at both DPRs. Workspace tests and the official web build passed. Mouse/pen contact, native/WASM strict Clippy and macOS packaging remain required before product promotion. C316 and C319 failed receipts remain failed.
