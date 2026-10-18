# C197: controlled Source position witness trial

The ordinary browser's million-vertex Source Brush still misses its responsiveness
gate. This trial narrows one shared-core operation: during a validated controlled
dab, only selected vertices can receive position writes, so the copied retained
normal state now carries the prior position words and replaces those selected
words. General refresh, history restoration, and private in-place state refresh
still rebuild all position words. A new test checks signed-zero bits, duplicate
selections, exact normals, and an out-of-selection history counterexample.

The final workspace run passed 2,506 tests with zero failures and one ignored;
strict native and WASM Clippy passed. The normal browser used the frozen
999,002-vertex, 1,998,000-triangle fixture and 60-second/400-requested-moves-per-
second protocol, SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The selected-witness WASM SHA-256 is
`d6c1e7ee0d9cdcd32986bebd25afee359d661d35438bdd26cb6f497792e3b12d`;
the control, changing only this shortcut, is
`fa91d1811751a7d8d2f38bed766310468c802245b3a8ed0f030c5b4f8e038db8`.

| Observed normal-browser value | Selected witness | Control |
| --- | ---: | ---: |
| Delivered moves / applied dabs | 1,770 / 805 | 1,790 / 808 |
| Mean retained match / dab | 7.22 ms | 7.19 ms |
| Mean state advance / dab | 6.91 ms | 7.08 ms |
| Mean complete queued dab | 24.53 ms | 24.58 ms |
| RAF P95 | 77.5 ms | 77.0 ms |
| Release acknowledgement | 288 ms | 262 ms |

The roughly 0.17 ms state-stage reduction is too small to move frame or release
latency. Both runs fail the existing RAF P95 ≤33.34 ms and release ≤100 ms
gates. Their delivered contacts differ, so these observations are not a causal
end-to-end speed ratio. The selected-witness run has exact whole-file Undo/Redo,
and an independent whole-project audit checks every public vertex, triangle and
nongeometry channel. It finds 40,768 changed vertices, 82,356 affected faces,
no new degenerate or reversed faces, and maximum changed-edge stretch 2.1833.
The audit and raw browser run are in
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-05561b96-eb85-446d-84b1-c4949b82987c/`.

Two alternative exact position comparisons in the retained match were tested
and **rejected**. A byte-slice comparison raised mean match to 10.70 ms and
RAF P95 to 87.5 ms; a three-word slice comparison raised them to 10.88 ms and
89.9 ms. Both used the same frozen browser protocol and passed exact Save,
Undo and Redo, but delivered fewer events. Their raw runs are retained under
`SP-LONG00-1m-60s-root-ce6ab4d0-f236-48f5-a08a-27c54cba00c8/` and
`SP-LONG00-1m-60s-root-d957fb8a-21af-4dd7-bfc5-3cd3ebc2c468/`.
The original vertex-by-vertex bit comparison is restored. Two earlier broad
selected-scope attempts failed
`private_refresh_preserves_every_copied_field_order_history_and_owned_buffer`:
the first left an out-of-selection position stale; the second changed the
general shared refresh's complete-field behavior. The first failed run remains
`.artifacts/c197-workspace-test.log`; the second failure was observed before
`.artifacts/c197-workspace-test-final.log` was replaced by the passing final
suite. The scoped controlled path is the only surviving change.

This is a small core efficiency improvement, not a browser lag fix or a parity
promotion. The larger measured costs remain retained matching, deformation,
WebGL paint and release; exact cross-dab validation ownership or GPU-resident
integration needs separate work.
