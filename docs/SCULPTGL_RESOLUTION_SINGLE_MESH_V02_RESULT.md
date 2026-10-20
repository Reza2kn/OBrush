# V02 single-mesh image gate: pixel segmentation failed

**V02 did not pass; `SGL-RENDER-RESOLUTION` remains Open.** This is a
separate attempt under the [V02 amendment](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_PREREG.md),
not a relabeling of [V01's normal-UI setup failure](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_RESULT.md).
V02 used the pinned Source Symmetry GUI controller as a recorded non-visible
setup step. Source render ratio also used its controller. OBrush used its
visible Open, Symmetry and resolution controls. The Source scene was cleared
with the visible Scene menu and confirmation, then the independent OBJ was
imported through the normal file chooser; OBrush's visible Open replaced the
startup object. Each before/after scene had exactly one visible/total mesh.

The final paired DPR 2 and DPR 1 runs each saved 24 normal-browser PNGs
(before, after and stable for three ratios in each app). At each DPR and
ratio, the same 12-CSS-pixel held move changed 37 original Source vertices
and 104 original OBrush vertices; OBrush also added 788 adaptive vertices.
Each stroke gained one history step. Within each app and DPR, 0.5× and 2×
had the same changed vertex IDs and exactly zero maximum before/after
coordinate delta against 1×; saved OBrush pre-contact project bytes were
identical across ratios. The Source camera comparison was at most
`2.13e-14` model units, within the frozen `1e-10` floating-pose tolerance.
These are geometry/history observations only.

The frozen V01 pixel analyzer exited 1 on the first Source 1× before image
at both DPR values. Its 1290×960 CSS scene crop found a second foreground
component larger than the preregistered 0.5% limit: 5,088 pixels at DPR 2,
4,246 at DPR 1. The component occupies the Source floor-grid strip at
`x=0..1289, y=879..959`. Visual inspection of the untouched Source PNGs
also shows the Scene dropdown remained open over the sphere and connects to
its segmented foreground. OBrush's floor grid crosses its sphere. Thus the
frozen segmentation is contaminated by visible UI/guide pixels, and the
static-color and silhouette bounds were **not evaluated**. This is a
failed visual gate, not evidence of matching render quality. It does not
indicate geometry drift, either.

Pinned Source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
mirror wrapper SHA-256 `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`;
official OBrush product revision `ff81bbf7e837ebf5c50c7462cf7b7e421b732450`,
portable WASM SHA-256 `d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`.
The independent 18,242-vertex/36,480-face OBJ SHA-256 is
`492dd0cfa56544bab0e816efa68a4b48b522ddaf5c70cbb2f95b6c92d167ec2e`.
Final V02 harness SHA-256 `3cfcbb4f5fa61f52e127c283293ff37aa83c7047e6888b272ea9340767985f28`;
frozen V01 pixel analyzer SHA-256
`1f9f91ec9cd9a12a1f280c3710b95a926fa073d3e9155a4cc637df7bd2410b92`.

All command-exit-1 receipts are preserved locally in ignored
`.artifacts/sculptgl-resolution-single-mesh-v02/`. The first three are
failed harness/setup attempts; the last two are complete geometry/history
runs with failed pixel analysis. No run was silently promoted.

| Receipt directory | DPR | `result.json` SHA-256 | Stopping reason |
| --- | ---: | --- | --- |
| `a1e8f412-bf0d-41f1-a913-3da7c2480013` | 2 | `2c7cebeecdcf2cc624fdfe7a69e54103769333c2986f493d51ff3c1a1d58d692` | Open chooser before visible host was ready |
| `f93cf5f8-5a15-4805-9fed-0742cb5b69ca` | 2 | `5375d1baf14abfcb33831d58ce970896a226b465bd2601930cd3ec311fca2734` | Single-mesh `.obrush` JSON shape assumption in harness |
| `390490fc-ab39-4272-9e22-8282c7d630cb` | 2 | `570c4a925b17fe65e9cf87a94fb67dc78d3e2d7feb9b83106f3adcde79e3735c` | Exact floating-camera equality in harness, then offline segmentation failure |
| `4d2f7753-52db-40ff-8bb4-f44a31ca6eca` | 2 | `a4a33076189014bad1e6ff98ff16236174e9574e9439d0db5af047978ef793bd` | Floor-grid component 5,088 px; pixel analysis SHA-256 `128895bd749adc6c67a9e9c0e22de122402de7d71d1108f13d83c24fa8209746` |
| `25a47eb5-1d59-43de-8660-90f4b46a31c1` | 1 | `40a5bdef49c41234b91a4405032368796ca5d24c425316f0a21e39d536d7d788` | Floor-grid component 4,246 px; pixel analysis SHA-256 `c73b46c08076b4a2d6b63bc5b3031da52161ceeebedf9cf909ed4db30a4ab233` |

The first four receipt passes involved harness corrections after failures;
their per-run script bytes are not all recoverable. Only the final two
complete runs use the final frozen harness hash. The interrupted Chrome
cleanup produced exit 130 on some attempts; the runner's result file and
captured failure remain authoritative, and its owned Chrome was closed.
The next visual gate must separately preregister a UI/guide cleanup or
mask before evaluating image bounds; it cannot revise V02's failed verdict.
