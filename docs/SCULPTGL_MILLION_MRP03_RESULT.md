# MRP03: million-vertex normal-browser frame attribution

**Read-only diagnostic; no product optimization or parity promotion.** The
feature-gated MRP03 probe from older commit `ab008d4` was ported onto
integrated product base `30a7e7a` (probe commits `048c13b` and `170d939`).
The ordinary build excludes its fields and trace logic; the diagnostic build
also requires an explicit browser flag, set immediately before the frozen
pointer press. A single 519,756-byte trace was extracted after the unchanged
40 release-tail observations. Neither runner changed its 24,000-motion
schedule. The exact original runner SHA-256 was
`fca0fbbd0a69c3e642e0b1271cfd09e0a0f5834b2032e8562cff9adf93f3c87e`;
owned control and diagnostic runner SHA-256 values were
`420b8ddcd7b7a54f020315f2b2ab116d1c4e74b75ae4406c4117f2f86713764f`
and `ac6999e8690c8a622f6b45c40dcd4996b0651448c366363767daaf682d1825d1`.
The copied hook SHA-256 was
`61a891fcf9c7af806bb9b3e93ac85e5a91453bffded8c0a152fa3bdce8218bce`.

The independently authored 999,002-vertex/1,998,000-triangle SGL fixture
SHA-256 was `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`;
the original 60-second/400-requested-moves/s protocol SHA-256 was
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
Both cohorts used owned Chrome `155.0.8059.85`, visible page and separate
browser profiles. CDP request receipts confirm that Chrome loaded the SIMD
WASM packages: uninstrumented SHA-256
`099814507ce5a70ccdbf415f54813c4c2f1de50e6e6a5804907e4339365f25ec`,
diagnostic SHA-256
`8982a455415c7c3b70e6822936dec4fd14265eebff9f4b96b06e257a761f87d5`.
The portable packages were also built, with SHA-256
`b2b6a116c22ff8100b49e2b95354cc1d6b2bbb1e292079901d88dcfe52706748`
and `c93b725f842d1f4c51bb89f82d64c6a8d545ee638f839c2a87311c5a0764fd31`.

| Frozen active metric | Uninstrumented control | MRP03 diagnostic | Gate |
| --- | ---: | ---: | ---: |
| Applied Source dabs | 980 | 1,019 | report only |
| RAF interval P95 | 66.8 ms | 66.7 ms | ≤33.34 ms |
| RAF maximum | 583.3 ms | 583.4 ms | ≤250 ms |
| App callback CPU P95 | 67.1 ms | 62.5 ms | ≤16.667 ms |
| Chrome release-command acknowledgment | 198.77 ms | 159.97 ms | ≤100 ms |
| Frozen responsiveness gates passed | **0/4** | **0/4** | 4/4 |

Different delivered dab counts, browser scheduling and probe overhead make
the between-cohort timing differences observational, **not** a measured
speedup. Both independent whole-project auditors passed all 17 checks:
Before/Undo and After/Stable/Redo public files were exact, non-position
channels and original directed faces were unchanged, positions/normals were
finite, useful edits were made, and no affected face became degenerate or
reversed. The control changed 40,755 vertices/82,330 faces; the diagnostic
changed 40,734 vertices/82,290 faces. Both runs had 64 memory samples with complete 62-sample
1 Hz schedules, zero recorder/app/console/GL errors, zero pending dabs after
the tail and clean owned Chrome/server exits.

## Same-frame diagnostic

The diagnostic retained 1,028 complete frames and did not overflow its
4,096-frame/128-event caps. Its Node-to-page clock bracket uncertainty was
±0.30 ms. Among 994 steady-stroke frames, each with one applied dab and a
paint callback, `App::update` averaged **48.21 ms** (P95 51.1 ms). Within
those *same viewport frames*, the disjoint cursor/overlay interval averaged
**22.41 ms** (P95 23.0 ms), Source queued geometry **21.68 ms** (P95 24.3
ms), and source-frame preparation 3.05 ms (P95 3.3 ms). The renderer callback,
which executes outside `App::update`, averaged 9.30 ms on these frames; its
upload substage averaged 8.11 ms. Do not add that renderer interval to the
app CPU P95, and do not infer a whole-frame win from a single isolated stage.
The directly measured 22.41 ms cursor and 21.68 ms geometry phases explain
most of the 47.2 ms mean `viewport()` cost during steady stroke.

The longest RAF gap occurred at initial press (583.4 ms). One initial-press
`App::update` took 555.8 ms, including 520.6 ms in the input phase and 22.5
ms cursor/overlay. No Source dab was applied in that frame. The next major
early-stroke gap was 416.7 ms; one early `App::update` took 391.7 ms, including
367.8 ms Source geometry. Existing nested core counters put that first dab's
deformation maximum at 333.9 ms, but MRP03 does not split the 520.6 ms press
input or the cold deformation further. The release-tail app update maximum
was 103.6 ms, including 77.2 ms input and 22.3 ms cursor/overlay. Release
command acknowledgment remained 159.97 ms. The page's release-event markers
and the CDP command request/acknowledgment are separate observations; MRP03
does not prove which component delayed the command.

The measured result rules out a hover-only or renderer-only fix for the
complete responsiveness contract. Even the prior rejected certified-hover
candidate left a 35.1 ms app CPU P95 and failed 0/4 gates. A combined path
must address ordinary per-dab geometry, initial-press work and cold first-dab
work as well as cursor picking and the release tail. A separate preregistered
first-press/first-dab subphase probe is needed before a bounded product edit;
MRP03 admits none. Actual GPU VBO position/display-normal words were not
independently checked in this diagnostic, so it cannot qualify a renderer or
Worker candidate.

## Receipts and failures

Raw control result, scalar and independent audit SHA-256 values:
`8dcceaf5ed994f848313178e66f5ca663575a785114146e16725223b066251a3`,
`0d7c4089eb82c6e6e7423835893ce9e4395f3f898a18834cccb23516fd5dbdd2`,
`8eb1885965a2ecba5e97cb5715620dd82a57cb75c82057f8d1ff465114329af2`.
The ignored directory is
`.artifacts/mrp03-stage/bench/actual/SP-LONG00-1m-60s-root-f47b8dcd-abb3-48cd-badd-287ed04184d3/`.

Raw diagnostic result, scalar, independent audit, trace and parsed report
SHA-256 values:
`db410019c46349eb908d3493a5decd704aa2d95b929aa89a5434f187ea0244c5`,
`f3f0ed0e9cdf9f2e570e2ab9de1bc287f748b110092d846b452031b8eb3355a0`,
`3d04d0efae471630535cc9abaf96a30768d2b243a1d2b93d10c012aec9326f68`,
`abfe2dc89ed4c125584d75ddf94499b23ad1ef4ab7442438ae1e75f731ad4fa4`,
`eeebc66a79f341ced27ebe327aa78a540e964a8ecf601fd7fd634634306c5475`.
The ignored directory is
`.artifacts/mrp03-stage/bench/actual/SP-LONG00-1m-60s-root-cda787fb-bb3c-43af-85d7-5f44eadf4d4d/`.

`cargo fmt --all -- --check`, full `cargo test --locked --workspace
--all-targets`, native strict Clippy, ordinary and feature-enabled WASM strict
Clippy, and the official `scripts/build-web.sh` completed. The first
feature-enabled strict Clippy run failed on a collapsible diagnostic `if`;
that code was corrected and the rerun passed. Two initial web-build invocations
failed because this worktree's script lacks an executable bit and the global
`wasm-bindgen` is 0.2.104; running it with `bash` and the repository's
lockfile-matched 0.2.129 CLI passed. These failures are retained in the task
transcript and are not called successful checks.
