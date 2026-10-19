# C264: current ordinary million-vertex Source Brush baseline

This is a before-change measurement of commit `28fabe23fb220fe586c5f131049ee83350e39711` in an isolated browser build. It uses the unchanged C58/C226 999,002-vertex, 1,998,000-triangle SGL sphere (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`) and frozen 60-second, 400-requested-moves/s protocol (SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`). The pinned reference remains SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; this run tested OBrush only. The harness copy changes only product paths to this worktree and writes a separate ignored receipt; its SHA-256 is `c798912ae60af508aaba185913ad04ed710083030cf33c12a72722302a051493`.

The normal browser was Chrome `155.0.8059.80` in an owned, fresh profile. The build used `scripts/build-web.sh` and wasm-bindgen `0.2.129`. Delivered `web/dist/index.html`, `pkg/obrush_app_bg.wasm`, and `pkg/obrush_app.js` SHA-256 values are `110b727af7aa0554003e683585c95d429d28e4318adbddc77177f276ac6ca6e6`, `926a0962235d74343aae9c7019e2335d1c8611ef3a10e4aa67849cbb8efad086`, and `efe138bbc6c262e1cc03cd1a641a7f831ee73e80fd7fd84fa147c81e12e22047`. The harness's inherited `product_binding.scope` text still says C68; the content hashes above bind the actual current product.

| Measured active interval | Current C263/C264 baseline | Frozen C226 OBrush | Pinned C226 SculptGL |
| --- | ---: | ---: | ---: |
| RAF interval P95 | 66.7 ms | 83.3 ms | 16.8 ms |
| RAF interval maximum | 666.6 ms | 600.0 ms | 33.4 ms |
| App RAF callback CPU P95 | 68.0 ms | 71.2 ms | 0.2 ms |
| Mouse-release acknowledgment | 159.0 ms | 161.2 ms | 8.0 ms |
| Applied OBrush dabs / recorded reference callbacks | 957 | 886 | 1,139 |
| Preregistered active responsiveness gates passed | **0/4** | **0/4** | **4/4** |

The independently scheduled 24,000 motion requests ran for 59,997.4 ms. Differences in delivered contacts/dabs, run order and browser conditions preclude a causal speedup or matched sculpt-response comparison to either older run. The active RAF maximum includes pre/post stroke recording boundaries; the stroke-only maximum here was 133.2 ms. Both the active and stroke P95 fail the 33.34 ms target.

At the final queue observation, the 957 applied Source dabs averaged 22.943 ms of shared-core geometry, comprising 6.843 ms retained match and 15.973 ms deformation (including 7.116 ms state advance and 1.987 ms Legacy normal refresh). The renderer callback averaged 15.879 ms over 1,367 samples, including 4.260 ms scene admission and 10.328 ms mesh upload. These nested timer scopes and sample counts are not additive per frame; input delivery, browser scheduling and GPU completion lie outside some scopes. The render path recorded 956 selected-chunk witness uploads, one full-scan fallback, 957 mask-buffer reuses and two mask uploads. The release queue was empty at the end of the two-second tail; the 159.0 ms acknowledgment still fails the 100 ms gate.

The actual `.obrush` before/after/stable/Undo/Redo saves have SHA-256 `367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`, `95f8a8945bf622d0869b657a0829e779402da50ca658ed85792aea67934f055f`, the same after hash, the same before hash, and the same after hash, respectively. The independent `scripts/audit-source-sustained.py` result passes all 16 checks: exact metadata and non-position channels/faces, finite positions/normals, useful edits, and no new affected-face degeneracy or orientation reversal. It counted 40,787 changed vertices and 82,394 changed faces; maximum and RMS displacement on changed vertices were 0.225585 and 0.113138. The complete recorder had no overflow or app/console errors, complete 1 Hz memory sampling, and the owned Chrome exited cleanly.

The ignored raw receipt is `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-6e08e6a5-d6b4-4885-b7cc-28defb7d39c4/result.json` (SHA-256 `9cc22bcc0f2989a2d1e53576a13bda5db2528dd51ebbb217beaf3ff1ef9ff3e5`); its scalar recording is SHA-256 `988f6a144d066c9c2f018de292ba1d1e4f8d27b9fbf5c65a5476767892f76f2e`. This is one OBrush browser run, not a full device/matrix or GPU-output parity gate. Million-vertex responsiveness and SculptGL/ZBrush parity remain open.
