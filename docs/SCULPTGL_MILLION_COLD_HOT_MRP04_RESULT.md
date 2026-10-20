# MRP04: million-vertex cold-path split

**Read-only diagnosis; no product optimization or parity promotion.** The
[preregistered probe](SCULPTGL_MILLION_COLD_HOT_MRP04_PREREG.md) was built from
isolated commit `a04d5b8` over integrated root `30a7e7a`. It uses a pure-Rust,
feature-gated 128-event recorder in `obrush-core`, enabled immediately before
the ordinary Source press. Events use one monotonic WASM clock and a
`Performance.now()` anchor bracket; the existing MRP03 frame trace supplies
same-frame hover, update and paint marks. The diagnostic stops recording after
the first controlled post-seed dab. The ordinary package omits the recorder.

The matched normal-Chrome cohorts used the exact 999,002-vertex SGL fixture
SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`
and 60-second/24,000-request protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
Owned Chrome `155.0.8059.85` loaded actual SIMD WASM SHA-256
`cd8cc176a2ffae765d66a69f1fc6e8d865b86670dfa6c5294cac6f3093267e0e`
in control and `78aa37df83257fc18dd8120a0ae844126c07b481eb3ca93c58ebb0aed14d25be`
in diagnostic. Both ended with zero pending dabs, app/console/recorder/memory
errors, clean owned Chrome/server closure and a separately reconstructed
**17/17** independent public-file/history/shape audit. That auditor's source
SHA-256 is `cba2af215e350e900c1f2592bd9b9383e326ad60750146051cc92d1a081d5b42`.

| Frozen metric | Control | Diagnostic | Gate |
| --- | ---: | ---: | ---: |
| Applied Source dabs | 950 | 845 | report only |
| RAF interval P95 | 66.7 ms | 83.4 ms | ≤33.34 ms |
| RAF maximum | 633.4 ms | 616.7 ms | ≤250 ms |
| App callback CPU P95 | 66.7 ms | 76.0 ms | ≤16.667 ms |
| Chrome release-command acknowledgment | 185.92 ms | 185.61 ms | ≤100 ms |
| Responsiveness gates passed | **0/4** | **0/4** | **4/4** |

Different delivered contacts and probe overhead mean the between-cohort
numbers are observational, not a causal speed ratio. The control changed
40,720 vertices/82,260 faces; the diagnostic changed 40,777/82,376. Exact
Before/Undo and After/Stable/Redo files, metadata, non-position channels and
triangles passed independently; no changed face became degenerate or reversed.

## Cold-path attribution

The fixture retained its affine `interaction_center`, and the bounded trace
finished with **exactly one** press-alpha seed and **exactly one** first-dab
seed: 28/128 events, no overflow or clock error. The browser-clock anchor
bracket was `[42845.6, 42845.6]` ms at the available 0.1-ms resolution.
The initial-press frame took 590.9 ms in `App::update`, including 552.6 ms
input and 23.4 ms cursor/overlay, with zero dabs. The first useful dab frame
took 409.6 ms in `App::update`, including 382.8 ms geometry and 25.6 ms
cursor/overlay, with one applied dab.

| Nested step | Initial press | First useful dab |
| --- | ---: | ---: |
| Scene pick | 23.6 ms | — |
| Begin Source session total | 528.8 ms | — |
| Blocker / Source hit / document begin snapshot | 13.4 / 37.9 / 28.8 ms | — |
| Press contacts / alpha preparation | 34.4 / 399.9 ms | — |
| Whole normal-state seed | **294.8 ms** | **221.8 ms** |
| Inside seed: mesh validation / normal compute / ordered-tree build | 13.4 / 39.3 / **222.7** ms | 14.1 / 37.6 / **154.1** ms |
| Controlled contact preparation / contact-capture validation | — | 34.6 / 50.5 ms |
| Post-seed deformation and publish | — | 63.5 ms |

Rows are nested and must not be summed. The press pick plus Source begin
interval reconciles with the 552.6-ms input span to within the trace's clock
resolution. The first dab's seed and post-seed work account for most of its
382.8-ms geometry interval; the residual includes host and core steps outside
these marks. The ordered-tree build dominates both seeds. Both seed calls
exceed the preregistered 10-ms duration condition and together exceed 25% of
their respective update callbacks. **This admits a same-state seed-reuse
microbenchmark, not the optimization itself.** No peak-memory or exact
reuse-state result exists yet. The steady 22-ms cursor and ~22-ms geometry
costs from MRP03 still require a separate combined path.

## Receipts and validation

The exact ignored receipt root is `.artifacts/mrp04-stage/bench/` in the
isolated worktree; its parsed report SHA-256 is
`045241f7d490e9ee161914eca046fdfc9244e8c0ffbfa0bd528a886e27488ad3`.
Control result/scalar/audit SHA-256 values:
`a63d5d1c78421eea584ed5fcdc480a344492e67812f9a08c22cf17d2b3fc6ef4`,
`e49fd555fc474c6e59cd00a96f8484098f81779aa7735f3edc21fe8d864c95e1`,
`ad70b4bc73b737a49f37077e043aeb7a6b197bf22b39008d14a0491708aba830`.
Diagnostic result/scalar/audit/cold/frame SHA-256 values:
`a1775766483363e0e7ccf9a4403c2dc49aadec46706fc9c2cd21b16cdc309e20`,
`dbdd2af367663e13d1f6b13d4afb776e30e85b53777c96cae3f8a5e2b43a3440`,
`d55bbe8f60a75db8f9d78d89df59c69d03ac1cbb5a3d83bb6d94d5facb0c0a56`,
`de2a532487d6d679e5bd22e8aefc51469d12271be25daccdb23fd52f9bbf66d6`,
`ac95f9104debe816eba448ac4a427975f5111f047cc3f82e265ce317a9bba57e`.
The control/diagnostic runner SHA-256 values are
`1789fac44e34e9e2e48b65767b52036014c0d46acc649c7b235bdc14efc15ec6` and
`b9f2efb42d4e77acf577987c5ec9f1e35ad0e948e24bd183d7a099eb176249a4`;
the once-after-tail hook is
`711c125ffd6c2406ec3336c946e21ac733688ad19f6713a0c8839232a8bb05eb`.

`cargo fmt --all`, full locked workspace tests, strict native Clippy in
ordinary and diagnostic configurations, strict WASM Clippy in ordinary and
diagnostic configurations, the official `bash scripts/build-web.sh`, and
separate portable/SIMD diagnostic builds passed. The first full-test attempt
failed on a mutably borrowed document span moved into an instrumentation
closure; the second failed on a host closure error conversion. Both probe-only
compilation errors were corrected and the full rerun passed. These failed
checks remain in the task transcript. Actual GPU VBO position/display-normal
words were not independently audited in MRP04, so no render or Worker
candidate is admitted by this diagnosis.
