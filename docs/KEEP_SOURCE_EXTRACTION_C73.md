# Keep-source mask extraction option (C73)

The mask-extraction inspector now offers **Keep source selected**. When enabled,
extraction appends a new inactive SubTool while the original part remains active
with its mask and metadata intact. When disabled, the existing behavior selects
the new part. The option is session state, not a project-schema field; either
choice is a single whole-scene Undo/Redo step. The shared Rust core validates
the new part and complete candidate scene before committing it.

An independently authored four-vertex, two-triangle unit sheet with masks
`[0,1,1,0]`, threshold `0.5`, whole-incident-face selection and signed thickness
`-0.125` produces an eight-vertex, twelve-triangle shell. Its signed volume is
`0.12499999999999999` against the analytic value `0.125`. Core and normal UI
browser checks confirm exact source preservation, source part 1 staying active,
independent new part 2, saved project and visible GPU buffers, and exact whole
scene Undo/Redo.

Validation: `cargo fmt --all --check`, full `cargo test --workspace --all-targets`
(2,354 passed, one ignored), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo clippy -p obrush-core --target wasm32-unknown-unknown -- -D warnings`,
`scripts/build-web.sh`, and `scripts/package-macos.sh` passed. The signed native
app verified with `codesign -vv`. The owned normal browser run passed all 11
mask-filter/extraction cases with 119 saved snapshots, actual 4× MSAA, 9,346
visible-control bounds checks without failures, and no GL or JavaScript errors.
The [browser result](../.artifacts/mask-filter-browser/keep-source-c73-160fc1dc-79cf-4904-9b4e-d3157e335359/result.json)
retains the full snapshots and diagnostics. The built and served WASM SHA-256 is
`826337f306ee37787c7795dbf6501f4a74980c66a73bcae131e9366ac40d281b`;
the signed native executable is
`060d17fde480cf69f6fc7ecb940d65c3425b6295c5447f992e90c116288acf50`.
The [evidence index](evidence/sculptgl-keep-source-extract-c73-0.1.20.json)
records exact fixture parameters and source hashes.

This matches only the pinned SculptGL source's **source-active selection** after
extraction. SculptGL's signed pre-offset, bridge policy, three smoothing passes
at nonzero thickness and extracted default channel behavior remain different.
No paired SculptGL runtime or ZBrush comparison was run. Native GUI, physical
input, global collision behavior and large-mesh latency remain unverified. The
browser still uses WebGL2 drawing and CPU geometry in WASM; no SculptGL or
ZBrush parity or million-vertex responsiveness claim follows.
