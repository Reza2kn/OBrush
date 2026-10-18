# C219: exact Source mask GPU-buffer reuse

The ordinary WebGL2 renderer previously deleted the main Source mask buffer
on every changed geometry upload. Its next linear paint call rebuilt a
999,002-float mask array and uploaded 3,996,008 bytes, even for a Brush dab
that only changed positions and normals. The active selected Source object now
keeps a bounded snapshot of the exact raw canonical mask words and mask
convention that produced that buffer. A new mesh may reuse the buffer only
when its raw mask bytes and convention still match. A mask edit, signed-zero
change, tiny value hidden by display rounding, changed convention, cache miss
or inactive part takes the existing rebuild path. The snapshot is at most
`MAX_VERTICES * 4` bytes (4,000,000 bytes); the shared Rust mesh, file and
history behavior is unchanged. UV seam mask buffers keep their separate
rebuild policy.

The focused regression verifies that changing geometry preserves the raw
mask snapshot, while a subnormal raw mask invisible after display conversion,
signed zero and a convention change invalidate it. Under the frozen
999,002-vertex, 1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and 60-second/400-requested-moves-per-second protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`),
the normal browser applied 902 dabs. The renderer recorded 902 mask-buffer
reuses and two total mask-buffer creations, including startup. The timed
WebGL recorder saw **one** 3,996,008-byte mask upload; the immediately prior
C218 final-source trace saw **902** at that exact size. That is 3,600,403,208
fewer requested mask-upload bytes across two separate runs, a direct transfer
count rather than a measured GPU-memory or frame-speed improvement. The C219
served portable WASM SHA-256 is
`e61740096172417fc678766c0be080595b3845dd519abea8ef54ddd83899ce09`.

| Final C219 normal-browser observation | Value | Local gate |
| --- | ---: | ---: |
| Independent RAF interval P95 | 66.8 ms | ≤33.34 ms |
| Independent RAF interval maximum | 583.3 ms | ≤250 ms |
| App frame CPU P95 | 69.6 ms | ≤16.667 ms |
| Mouse-release acknowledgment | 129.1 ms | ≤100 ms |
| Renderer mesh-upload CPU mean | 11.68 ms | No isolated speed gate |
| Renderer scene-admission CPU mean | 7.80 ms | No isolated speed gate |

**All four responsiveness gates still fail.** C218 and C219 applied different
contacts and deformed different final shapes, so their similar RAF and upload
timings do not establish a speed ratio. Mask transport was a large redundant
transfer, but queued geometry, scene admission and other renderer work still
dominate the visible lag.

The complete timed browser receipt is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-60b6afae-fe9e-469e-9386-a3d1a670211b/result.json`
(SHA-256 `10c9a671d38db98816f980093f51137ffbfdc4b0dc49ada64d99ed6481873578`).
The independent public-file audit at `.artifacts/c219-audit.json` (SHA-256
`4214b9b7e4e1fba9ce6ac8aabd17c50f61b458bf97cabc142d4a8ff2373e29a3`)
passes complete Save and exact Undo/Redo, unchanged scene metadata, faces
and non-position channels, finite changed geometry, and no new degenerate or
reversed affected faces. It records 40,792 changed vertices and 82,406 changed
faces. The isolated Chrome owner exited cleanly with no app/console errors.

A separate normal-browser mask/filter/extraction suite passed all 17 cases,
including masked edits, exact histories and visible-GPU/save checks. Its
receipt is
`.artifacts/mask-filter-browser/c219-source-mask-reuse-a7703871-3a33-41aa-922f-611b4d0de5e4/result.json`
(SHA-256 `e3a49a58e0e07db267b91ca94b4fb12af63281afed45889df40f6bb16c6c5040`).
This suite does not independently count a mask-buffer invalidation on the
million-vertex selected Source path; the exact raw snapshot regression and
normal-browser unchanged-mask counters cover narrower parts of that claim.

Final-source checks pass `cargo fmt --all -- --check`, `git diff --check`,
2,524 workspace tests with zero failures and one intentional ignore across
103 result groups, strict native/WASM Clippy, `scripts/build-web.sh`,
`scripts/package-macos.sh` and macOS signature verification. The test log
SHA-256 is `4a40ff868f2fb04a09cdfab8d9e082a9d1c1abbb8a4cbd85df943c40818ad83e`;
the packaged native executable SHA-256 is
`ef11c95730f7e80efb22d2e2ef102c0259d57cc19dad0210652bf73b76a85e58`.
This renderer-local improvement does not establish SculptGL or ZBrush parity.
