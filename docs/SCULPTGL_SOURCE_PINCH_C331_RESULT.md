# C331 independent flat-plane picker calibration: failed

The [preregistered C331 protocol](SCULPTGL_SOURCE_PINCH_C331_PREREG.md) was run in two owned ordinary-browser sessions using the independent 1,089-vertex exact-binary flat fixture SHA-256 `12fc61da6ad54c03c456107a3edec82511fe6352b7da366eec9dc5a5f0c670bc`. The pinned hosted SculptGL bundle SHA-256 was `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`. The integrated OBrush source was `ab428f2`, with served official portable WASM SHA-256 `aa374ba02c4ccd6b26d0bd06e6c73e298efc579462ee6f0e2980614a5cc33437`. The frozen capture harness SHA-256 was `9336ffa4824a3d878f99e8e0aecace095c98b460b1c58d723ae4ac4b1dd7a64e`. The source result JSON SHA-256 is `fcdf790f0172c3a5f8ced076efbe2c29a6777b408267ed30b50b8558b46a0248`; OBrush JSON SHA-256 is `2e44ad8a4dfc33b0d12e68bb3e199fa973393530f686779cbe148a0c4ac5b506`. Both remain under ignored `.artifacts/sculptgl-source-pinch-c331-ui/` in the isolated worktree.

Both cameras reached an unrotated 1024×768 orthographic sculpt viewport at DPR 1. The published projection X/Y values were equal across hosts (`2.3413069248199463` / `3.1217427253723145`), so physical ray/radius scale comparison was exact. Six press/release probes per host completed with **zero** Pinch dabs, no change in public Before/After mesh SHA-256, stable history, and unchanged active GPU position words. The source recorded a picked point at every press. OBrush's normal UI diagnostic `source_geometry.active.last_pass` was `null` on all six zero-dab presses; its separately published read-only camera ray was captured after each release. A camera ray is not an observed brush pick and was not substituted for one.

The offline auditor SHA-256 `2c55d0454b8a47ecc6d3613e73699d230259ebca0f8bc2f5e1dfc8940541a800` checked pinned gl-matrix 3.3.0 primary-source hashes, reproduced every captured source world-to-screen Float32 matrix word exactly, then inverted that matrix and intersected its ray with `z=0`. Every source observed picked XYZ matched this independent predicted point exactly at all six probes. The central predicted/observed Y was `+7.450580596923828e-9`; the analytic rounded-affine formula and full gl-matrix inverse agreed exactly. OBrush's published camera rays matched the independent `f32` screen-NDC plus `f64` projection replay to below `1e-18` in X/Y. This confirms matrix-representation effects on the independent plane, but it does not prove the full curved-mesh C329 discrepancy was solely from that effect.

| Canvas-local probe | Predicted Source minus OBrush plane point Y | Frozen `1e-8` physical-ray gate |
| --- | ---: | --- |
| `(512,384)` | `+7.450580596923828e-9` | pass |
| `(520,384)` | `+7.450580596923828e-9` | pass |
| `(512,376)` | `+1.3455856857566562e-8` | **fail** |
| `(512,392)` | `+2.0538692235665124e-8` | **fail** |
| `(520,376)` | `+1.3455856857566562e-8` | **fail** |
| `(520,392)` | `+2.0538692235665124e-8` | **fail** |

The six missing OBrush observed pick points and four off-center ray differences fail C331. The four-run C329 Pinch geometry matrix and its direct picked-center gate remain failed, and no product geometry edit is justified here. A new preregistered run would need a read-only normal-UI hit observation that exists even when no dab is applied, or a separately justified changed contact protocol. It would also need to account for the *different* Float32 screen-NDC and inverse-matrix representation paths before any pair bound is selected. This result cannot close complete Pinch, SculptGL or ZBrush parity.

The offline audit JSON SHA-256 is `9690cf67fd182d220b318d0049af3f2e192e759f46ffcec3b45856d7730cf8ff` and is retained alongside raw captures. The audit script was written after capture against the pre-existing frozen numerical gates; its source and receipt hashes are included so the score is reproducible. No browser rerun, product edit or package operation followed the failed audit.
