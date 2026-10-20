# Typed complete-scene witness trial: no-go

This is the result of the frozen trial preregistered in
`SOURCE_WORKER_TYPED_SCENE_WITNESS_DESIGN.md`. The product candidate is preserved
as commit `c095eb56b5a1672e64015d3e2d74b6f853791b24` in the isolated
worktree history, then reverted. It must not be integrated or used to promote
the query-gated Source Worker. The candidate serializes the same validated
`ProjectRef` as public Save through a tagged, counted Serde JSON formatter.
It avoids JSON number formatting at final publication, but performs an extra
complete traversal at capture after generating the public Worker seed.

## Frozen 100k result

- Protocol SHA-256: `800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`.
- Independently created 99,602-vertex / 199,200-triangle SGL fixture SHA-256:
  `8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
- Build product commit: `c095eb56b5a1672e64015d3e2d74b6f853791b24`.
  Browser WASM SHA-256: portable
  `d7b71a4ed5d0c9e8be76e55bc534b51e7bcbeefea9b66c62720850af72fcadc9`,
  SIMD `622c9e3c0ff4af3214fefd90513149e822bda29d0177c765f4efe2a7e1589891`.
- Raw result SHA-256:
  `c134a0911ac5be97dae5d979b7433e792f1da30c8f884b884ecf9811fcebbfea`.
  All raw files and audit outputs are retained at
  `.artifacts/source-worker-sustained-spw01/100k-worker-4f7c564a-0901-4933-97da-ce800a483876/`.
- Exact held-pointer protocol: 60 seconds, 24,000 requested moves, Source
  Draw radius 64 pixels, intensity 0.25, no symmetry. The browser delivered
  1,286 accepted dabs and published one final commit with its release endpoint.

The complete-scene witness took **305.3 ms**, against the preregistered
**at most 200 ms** gate and the earlier JSON witness's 265.9 ms. Typed
serialization and hashing alone took 290.8 ms, including 82.6 ms in the hash
writer. The normal begin call took **1000.3 ms**, against the **at most
458 ms** gate and earlier 407.8 ms. The second full capture traversal is a
plausible contributor, but capture stages were not separately timed; this
trial does not prove the amount it caused. The main final publication took
510.2 ms, with 13.4 ms delta decode and 169.3 ms retained-state decode.
This one browser run is not a matched-stroke speed comparison.

## Correctness and failures

Focused Source Worker tests and the full locked workspace suite passed after
two local test corrections: an initial compile failed because this toolchain
does not derive `Default` for 64-element arrays; a first fixture assertion
selected the already active SubTool 2. The final focused suite, strict native
workspace/all-targets Clippy, strict WASM core/app/gpu Clippy, `cargo fmt`,
and `scripts/build-web.sh` passed. The adversarial corpus covered targeted
mesh positions, normals, triangles, masks, colors, materials, UV coordinates
and indices, names, layers, morph, signed zero, imported v7 provenance,
selection, visibility, invalid fields, and in-place stale edits with stable
host identity. It is not an exhaustive proof for future custom serializers
or serde_json upgrades. The fixed 64-level nesting and bounded typed-byte
stream could conservatively reject a public-JSON-valid extreme scene.

The browser shape audit passed every check: no new changed-face degeneracy or
orientation reversal; 4,012 vertices and 8,272 faces changed. Normal public
Save hashes were byte-identical for Before/Undo
(`1100e81f00726ea03ad699f16356b6bddea8861b795262f51b0b56b8af3c7e46`)
and After/Stable/Redo
(`e1d8bfae132b9834185efaef996d3fb7f95c7ac4369789d2a72631c356c7eab8`).
GPU position words matched in all five states. Static display-normal words
matched the actual GPU readback for Before and Undo. The frozen verifier
still failed active RAF maximum (**716.6 ms**), proof of an empty queue by
100 ms after release (the first tail probe was late), and its raw public
mesh-normal GPU comparison (which differs from the static display-normal
contract). The raw `gpu-audit.json`, `frozen-verifier.json`, shape audit,
static-normal audits, browser result, transport, and five public projects are
retained without rewriting these failures.

The preregistered performance gates failed decisively. This implementation
is a rejected experiment; the held Source Worker remains off by default and
its SculptGL/ZBrush parity remains unproven.
