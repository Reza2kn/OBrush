# C293 candidate setup failure

The prospective C293 settled-camera Source Drag candidate **did not reach a
paired geometry result**. The pinned Source normal-UI stroke captured public
Before/After/Undo/Redo and held GPU states, but the OBrush run stopped before
its stroke at a stale test-harness branch: `ReferenceError: matchSourcePath is
not defined` at `scripts/check-sculptgl-source-drag-c293-ui.mjs:197`. This is
a harness setup failure, not evidence that Drag geometry passed or failed.
The frozen harness SHA-256 was
`759e84f8929748a666ce323fc3d9419d001e8e33671c53992e4e3818cfe4ccc4`;
scorer SHA-256 was
`e08ee1e0d056bc8be2b5b7cf4cabcf2f5a2cd3f55c4db8986003b849a300e0c7`.
The Source capture receipt is
`.artifacts/sculptgl-source-drag-c293-ui/source-3e1b7a10-5328-488c-8a04-9f4d923e1268/result.json`
(SHA-256 `b3b365cf36d81cfea630dd6b3c7d5cbc581c54b094743affad658eb776bb8564`);
the failed OBrush receipt is
`.artifacts/sculptgl-source-drag-c293-ui/root-a8f4b3fd-6623-4385-acbb-e8c780021f64/result.json`
(SHA-256 `e0dc59167e8f66736513fd62ff43f249eedd93c1a89678da479dbe7a1ef3ab15`).
Both owned browser processes closed; root port 8792 was untouched. The
independently authored fixture and pinned SculptGL bundle retained their
preregistered hashes. No C293 geometry tolerance or prior failed result is
changed by this failure.

Candidate code was isolated at commit `86bf24790f220806a36516b42a30728143cdbeb5`.
`cargo fmt --all -- --check`, focused Source Drag 5/5 tests,
`cargo test --workspace`, strict `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo check -p obrush-core --target wasm32-unknown-unknown` passed.
The first `scripts/build-web.sh` shell invocation failed because the file is
not executable; invoking it with `bash` then found the older global
`wasm-bindgen` 0.2.104. The subsequent `bash scripts/build-web.sh` with
`WASM_BINDGEN_BIN=/Users/Ajab/AI/OBrush/.tools/wasm-bindgen/bin/wasm-bindgen`
used lockfile-matched 0.2.129 and passed. Both earlier local build failures
remain in task output and `.artifacts/c293-build-web.log` preserves the final
successful build output. No product code is promoted from this incomplete
candidate check.
