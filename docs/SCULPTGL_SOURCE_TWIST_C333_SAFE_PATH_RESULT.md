# C333 Source Twist guard-safe normal-browser result

C333 passed its **scoped, preregistered** normal-UI paired gate on the independent C306 fixture. It does not close the Twist row. The older C318 folded-target audit remains failed and unchanged; this path intentionally avoids that confound. No product code changed for C333.

The protocol was frozen before either capture in isolated commit `5df1b47` (integrated as `309f828a28db2f92d85e8d236e57d974ae27274d`). The harness SHA-256 was `76c71521c76f23904f9f15d36fd65dd38e6250ebb06bb9eae55381afbc630add`; the scorer SHA-256 was `125ae92268f74e24be94eb25f9a105f2585379f1ad196958a3d41a8339859776`. The fixture was `a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`, the pinned SculptGL bundle `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600` at reference commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, and the owned served OBrush WASM was `aa374ba02c4ccd6b26d0bd06e6c73e298efc579462ee6f0e2980614a5cc33437` with JS `7f5bed563b0664135079fcf7d3f75e1f2cee45f119ad7932d90b2a53a8f56753`. The served checkout was `309f828`; changes after the C330 product validation were documentation and capture scripts.

The normal UI path used Source Twist tool index 2, 75-pixel radius, no alpha, symmetry, lock position, continuous mode or culling; OBrush visible strength 1; settled 1024×808 viewport; and trusted integer offsets `(-48,-12) → (-12,-12) → (-15,-42) → (-24,-43)`. Both owned browser sessions completed public Before/After/Undo/Redo saves and closed cleanly. Source blocked only its external Google Fonts request. No console error, exception, failed acknowledgement or cleanup error occurred.

| Observed paired maximum | XYZ component | Raw display-normal component |
| --- | ---: | ---: |
| First effective held move | `1.4901161193847656e-8` | `8.731149137020111e-11` |
| Second effective held move | `1.4901161193847656e-8` | `1.1641532182693481e-10` |
| After / Redo | `1.4901161193847656e-8` | `1.1641532182693481e-10` |

All are below the frozen `1e-6` thresholds. The same 38 RGB IDs were selected from the actual held GPU positions in both apps; all changed display-normal IDs stayed inside that set. Protected ID543 did not move. ID577's half-mask and ID578's near-edge radial falloff yielded first-held mask-normalized angles of `0.7378150477` and `0.7378152163` radians in Source, versus `0.7378148034` and `0.7378130236` in OBrush. The within-app and paired discrepancies passed the frozen `5e-6` radian threshold. The prior C318 raw displacement-order check is still invalid for these unequally located IDs and remains recorded as failed.

The independent unchanged guard evaluator found 101 affected faces on each app's actual second-held surface. Source minimum face projection was `0.2885399103`, quality `0.1189614353`, adjacent-fold cosine `0.9924099330`; OBrush values were `0.2885401073`, `0.1189615575`, `0.9924099516`. Neither surface had a face or fold violation. The first-held and After surfaces also had zero violations. Public Save matched the active GPU positions, topology and attributes stayed stable, release became quiet, and each app's single Undo/Redo restored exact own saved bytes.

Raw Source result SHA-256: `d2e88654e7b36758d9bc7cb0910388be1cabef18b2aadabc38ea62c1482833ad`. Raw OBrush result SHA-256: `f847aa54f6dfefb8540397badf49688a573728b5dd9ac7809ce8d87c212a50c2`. Paired audit SHA-256: `7dc74394500eefccca162b7e5f39167c9859dc2d61e68c049882e0071e0db40e`. All are retained under `.artifacts/sculptgl-source-twist-c333-ui/` in the isolated Twist worktree.

This is one 1089-vertex, settled-camera, symmetry-off, guard-safe path. The Source WebGL1 upload shadow and OBrush WebGL2 readback are the display-normal oracles. Other geometry, folded targets, changing selection, symmetry and dynamic-topology cases remain unmeasured here.
