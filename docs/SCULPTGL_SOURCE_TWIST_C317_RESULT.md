# C317 corrected Twist selector baseline: failed scoped parity gate

The corrected normal-UI pair selected pinned SculptGL Twist (tool index 2)
and OBrush Twist on the same independent 1,089-vertex curved grid. **The
frozen paired gate failed.** Both apps changed the same 37 RGB vertex IDs and
passed their own public one-step Undo/Redo, topology/attribute, active GPU
position and quiet-release checks. Their After positions differed by up to
`0.021797308698296547`, and active display normals by up to
`0.00011306075612083077`, versus the frozen `1e-6` limits. The same paired
failure appeared at held moves 1 and 2 and Redo; held move 0 and Undo matched
exactly. This is a measured Twist gap, not a parity result or a product fix.

C306's selector had used index 3 (Smooth). Its complete raw captures and
failed audit remain in `.artifacts/sculptgl-source-twist-c306-ui/`, with exact
hashes in [the C317 preregistration](SCULPTGL_SOURCE_TWIST_C317_PREREG.md).
Those C306 geometry numbers cannot be interpreted as Twist. C317 changed
only the selector and source index assertion, protocol label and output
directory; the fixture, 75-pixel radius, four-point arc, input route and
numeric gates remained unchanged. Its harness SHA-256 was
`2f4253b9d9c59b5046f1514c4f192981d38d3f1d53d58becd778c97c0e08b28f`,
scorer SHA-256
`2a43d3c1438cefa0b33fcdb1aef67408b2913a6c362bcf5dd60d74c9f4955c06`.
The pinned SculptGL bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the unchanged aligned OBrush portable WASM was
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.
The fixture SHA-256 was
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`.

Both cameras settled for at least 12 terminal frames and 250 ms. X/Y
projection ratios were 1, all four tested rays and the 75-pixel world radius
matched exactly in the scorer. Both browsers delivered the same trusted
integer points relative to their viewport centers. The reference captured
the picked face and fixed center at mouse-down; both normal UIs completed
Before/After/Undo/Redo Saves and three held GPU snapshots. Both applications
left fully protected RGB ID543 stationary. The source moved ID541 by
`0.020939597747869732` and OBrush by `0.007216470161449752`; source moved
half-unprotected ID577 by `0.013021608945545986`, OBrush by
`0.0034870693428928116`. OBrush's visible Twist strength was `0.35` while
the pinned Twist has no intensity control; this is a plausible contributor,
not an isolated causal measurement. The source and OBrush falloff and
incremental/anchored kernels also differ statically. A later controlled
factor test is needed before selecting a correction.

The scorer also failed its preregistered ID577-versus-adjacent-ID578
displacement condition **on both apps**. The adjacent vertex has a different
position and falloff, so this condition does not isolate mask strength. It is
preserved as a failed check, not reinterpreted as a source/OBRush mask defect.
No product code changed and no Twist row is closed.

Raw ignored receipts:

- Source result: `.artifacts/sculptgl-source-twist-c317-ui/off-source-c05faf00-caab-47f0-b8fa-872f8cb3f34b/result.json`, SHA-256 `63f64bfb382ef7be89c0e17b4cd9c70a45e85c63e3555e5c99f2941338efafdb`.
- OBrush result: `.artifacts/sculptgl-source-twist-c317-ui/off-root-f071d509-3b57-476d-9575-0b64d1ddbebc/result.json`, SHA-256 `b82401e10b242255597adc6908af68c9bb8b0ff0fb657b841661706dc0a102bc`.
- Paired failed audit: `.artifacts/sculptgl-source-twist-c317-ui/off-source-c05faf00-caab-47f0-b8fa-872f8cb3f34b/paired-audit-off-root-f071d509-3b57-476d-9575-0b64d1ddbebc.json`, SHA-256 `ef609ecd5b418c6701fe91045073d1fb83f04faefe011185fd18a3aac6d0e984`.

The first `build-web.sh` invocation failed because the default
`wasm-bindgen` CLI did not match Cargo.lock; the successful build used the
existing local 0.2.129 CLI. A proposed deletion of the partial isolated
`target` directory was rejected by automatic approval review, so it was
preserved. A shared Cargo target was used for the build; the isolated
worktree's target link was removed afterward. Data free space was 17 GiB
after the run. No further builds were made.
