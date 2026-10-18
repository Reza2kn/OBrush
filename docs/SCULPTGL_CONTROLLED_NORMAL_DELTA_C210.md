# C210: scoped Source normal-change detection

The controlled shared-core Source geometry path refreshes only the selected
vertex-normal rows and their incident original-face rows. It previously copied
the retained normal fields and then scanned the full next/previous fields to
decide whether the derived cache changed. C210 compares only the rows this
checked operation may refresh. General/history refreshes retain the full-field
comparison, and exact bit comparison (including signed zero) is unchanged.
A focused sphere/cube/quad test compares the selected-write path against the
original full-field result, including duplicate and empty selections.

The normal browser used the same independently authored 999,002-vertex,
1,998,000-triangle SGL sphere and frozen 60-second/400-requested-move protocol
as [C209](SCULPTGL_RELEASE_STAGE_PROFILE_C207_C209.md). The C210 served WASM
SHA-256 is `cd99f3f9191a0818dbfadf81ddf6ca73172d5289f3a3469f19d152941fc5a60e`.
The C209 post-release-observation runner is
unchanged. These are separate runs with different applied dabs; no causal
end-to-end speed ratio is claimed.

| Measured stage | C209 full-field | C210 selected rows |
| --- | ---: | ---: |
| Applied dabs | 864 | 869 |
| Normal-state advance mean | 6.88 ms | 6.48 ms |
| Complete queued Source dab mean | 19.49 ms | 19.11 ms |
| Renderer upload mean | 12.67 ms | 12.74 ms |
| Mouse-release acknowledgment | 167.2 ms | 172.8 ms |

The directly timed target stage is about 0.40 ms lower per dab, but the normal
browser is still far from the million-vertex responsiveness gates. The C210
complete public-file/shape/history audit passes: 40,807 changed vertices,
82,436 affected faces, no new degenerate or reversed affected faces, exact
stable Save and Undo/Redo, no application/console errors and clean owned
browser exit. Raw result:
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-fc213655-a553-4d6f-a862-230cc592c5fa/result.json`
(SHA-256 `671c7ffb04ee221165821505f05b44f563879b747f4677a4a39c0760ac0412a5`).
Independent audit: `.artifacts/c210-audit-1m.json` (SHA-256
`322bb85ef4ea4c2e7182ed48e4c31791878f4296cffc3986ddaa0ee23195b80c`).
No SculptGL or ZBrush parity status is promoted.

Final checks pass: `cargo fmt --all -- --check`, 2,512 workspace tests with
zero failures and one ignored across 83 groups, strict native and WASM Clippy,
`scripts/build-web.sh`, `scripts/package-macos.sh`, and the macOS app signature
check. The workspace test log SHA-256 is
`8f0eff377406fc89441a115a69b939e15695a976c37ebc33f5d8ec61252939f7`.
