# C225: million-vertex baseline and rejected attribute-walk trial

The frozen 999,002-vertex / 1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and 60-second, 400-requested-moves/s protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`)
were rerun on the committed C224 browser build. The normal owned Chrome run
completed Save and exact Undo/Redo, applied 886 dabs, and passed the
independent whole-project/shape audit: 40,804 changed vertices, 82,431
changed faces, no new degenerate faces or orientation reversals. The
[baseline result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-b16cbce9-74a8-4078-aed6-e482806a2d52/result.json)
has SHA-256 `45bdb6bc641b884040c9356ef603ebc3edae4d4f38a4c6ea76deb61baab19406`;
the [independent audit](../.artifacts/c225-baseline-audit.json) has SHA-256
`3781b5889062a67edc0cf0865e488ab9a2984e5dcaa410c0fae3f6633b1392ce`.
The portable WASM was
`5b591faca1e520b37d1b542b1d824b4deb024f1cb1b0d7eb15163fef1dafd91f`.

A trial changed the selected sparse renderer upload's complete-channel check
to walk mask, RGB and old packed rows together, eliminating per-vertex
optional lookups when both channels were complete. Incomplete channels kept
the old path. Focused signed-zero, changed-attribute and exact packed-row
tests passed. The full frozen browser run applied 892 dabs, passed the same
whole-project/shape audit (40,780 changed vertices, 82,382 changed faces,
no new degenerate or reversed faces), and had no app/console/GPU errors. The
[trial result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-35ca83f6-64cc-4f04-9320-14ad909b8fad/result.json)
and [trial audit](../.artifacts/c225-trial-audit.json) are preserved. Its
portable WASM was
`5d6e78d266a85dd95c385160c139ed7b347a98f647b7d36ee5dce53545206906`.
The trial audit SHA-256 is
`647fc696032bfe000ccb00ea98560118844fa1637e0943b813393629a73f9fad`.

| Observed 1M/60s measure | C224 baseline | Trial |
| --- | ---: | ---: |
| Applied / coalesced dabs | 886 / 12 | 892 / 9 |
| Source geometry / retained-match mean | 19.797 / 7.372 ms | 19.794 / 7.390 ms |
| Renderer admission / upload mean | 7.984 / 11.882 ms | 7.991 / 11.618 ms |
| Renderer callback mean | 21.155 ms | 20.863 ms |
| Active-recording RAF interval P95 | 83.3 ms | 83.3 ms |
| Active-recording RAF interval maximum | 600.0 ms | 600.0 ms |
| Active-recording app-frame CPU P95 | 71.2 ms | 71.6 ms |
| Stroke-only RAF interval maximum | 133.4 ms | 116.7 ms |
| Release acknowledgment | 161.2 ms | 164.0 ms |

The preregistered active-recorder gates include setup, as in C219; all four
fail in these two runs. The separately reported stroke-only window runs from
the first scheduled move through the release request and excludes setup. Its
RAF maximum is below 250 ms, but its P95, frame CPU P95 and release
acknowledgment still fail. These are separate browser runs with different
delivered contacts, so the 0.264 ms upload-mean difference is not a causal
speed estimate. Neither release nor frame P95 improved. **The trial was
removed.** The unchanged source-mesh
retained match and renderer admission each cost about 7–8 ms per active dab;
the next performance work must address their validated ownership boundary
or the larger deformation path, while preserving stale-state rejection and
the complete public file/history/shape audit. This report does not promote
SculptGL or ZBrush parity.

The reusable `scripts/summarize-source-sustained.py` checks the recorder hash
and calculates the original active-recorder gates and the explicit stroke-only
diagnostic interval. Its C225
two-run output is `.artifacts/c225-timed-summary.json` (SHA-256
`ee5c1322fbd43010cb4b9ed1d435bdad3c5cbaea10c94e1faed21efbe61096dd`).
As a historical cross-check, the summarizer reproduces C219's documented
66.8 ms active RAF P95, 583.3 ms active RAF maximum and 69.6 ms app CPU P95.
After removing the trial, `scripts/build-web.sh` restored the portable WASM
byte-for-byte to C224 at SHA-256
`5b591faca1e520b37d1b542b1d824b4deb024f1cb1b0d7eb15163fef1dafd91f`.
