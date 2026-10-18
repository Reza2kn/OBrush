# C200: reuse active-scene validation at stroke release

`Document::end_stroke` used to validate the same active mesh and scene pose more
than once during one exclusive mutable call. An ordinary scene with no rich
attributes validated its active mesh before the final scene check, and that
check scanned it again. Rich Source scenes also revisited the mesh/pose during
end-of-stroke preflight, final scene admission and metadata admission. The
release path now carries a local proof of which active checks have already
passed inside that call. General public scene-candidate entry points still
validate their own candidate. The final scene structure, budget, inactive
objects, origin accounting and history checks remain. An independently
authored two-part scene regression observes one active mesh validation on a
valid stroke and a rejected NaN stroke, with exact project Undo/Redo and
rollback. The regression covers both a plain active scene and a scene with
unprotected mask attributes, which takes the richer preflight path.

The frozen normal-browser cohort uses the independently authored
999,002-vertex, 1,998,000-triangle SGL fixture SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`
and 60-second/400-requested-moves-per-second protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
No alpha, mirror, clay, accumulate or fold guard is used. The C200 served WASM
SHA-256 is `b460ecf4e4ee401ccf0f0b1bf5c54f9bab18c5387b83f52ade982ed8e97fe740`.

| Normal-browser observation | C199 run | C200 run |
| --- | ---: | ---: |
| Delivered moves / applied dabs | 24,000 / 863 | 24,000 / 830 |
| Mean complete queued dab | 19.55 ms | 20.59 ms |
| Mean deformation envelope | 12.20 ms | 12.98 ms |
| Independent RAF P95 / maximum | 72.4 / 609.3 ms | 76.4 / 645.8 ms |
| App paint CPU P95 | 71.9 ms | 75.7 ms |
| Release acknowledgement | 263.8 ms | 146.9 ms |

The 116.9 ms lower release time is consistent with removing redundant
million-vertex scans. These are separate browser runs with different applied
contacts and edited shapes, so the difference is not a controlled causal
estimate. Sustained brushing did not improve. All four responsiveness gates
still fail: independent RAF P95 ≤33.34 ms, maximum ≤250 ms, app CPU P95
≤16.667 ms and release ≤100 ms. The normal studio still uses WASM CPU
sculpting and WebGL2 rendering.

The raw C200 result at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-038e3527-0750-4ea9-b14f-390af1dbb629/result.json`
records zero application/console errors and owned Chrome exit. Its SHA-256 is
`021cc5ca3b5281847c54e83fd98dadb84aa24f72e22b947413722c5d2583f692`.
The independent full-project audit at `.artifacts/c200-audit.json` (SHA-256
`5cddf009e5f2e659e599d47cc4666902e26151ac15966df8c0dc486e1422efef`)
checks every public vertex, triangle and non-position channel, exact Save,
stable Save and Undo/Redo files, and all affected face orientations and areas.
It finds 40,793 changed vertices, 82,409 affected faces, no new degenerates or
orientation reversals, and maximum changed-edge stretch 2.1839.

Final validation: `cargo fmt --all`, 2,509 passing workspace tests with no failures
and one ignored across 83 groups,
strict native and `wasm32-unknown-unknown` Clippy, `scripts/build-web.sh` and
`scripts/package-macos.sh` passed. The macOS package signature verified.
This narrows one release bottleneck; it does not establish SculptGL or ZBrush
parity, a real-time million-vertex viewport, or a release under 100 ms.
