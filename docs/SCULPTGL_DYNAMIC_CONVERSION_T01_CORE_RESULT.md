# T01 shared-core dynamic conversion result

Candidate: isolated `codex/topology-dynamic-conversion` worktree from OBrush `1ecc42def96bd8dbe90b340d4aa4b8233ac134a1`. Reference source and independently authored fixture SHA-256 hashes are pinned in [the preregistration](SCULPTGL_DYNAMIC_CONVERSION_T01_PREREG.md). No commercial source or binaries were added.

The first fixture run **failed** (exit 101, 0/2 tests) before conversion because the legacy `Mesh::from_obj` reader intentionally did not retain quad/UV metadata. The fixture loader was corrected to the public bounded `Mesh::import_obj` path; the failed result is retained here rather than represented as a candidate regression.

The corrected focused test passed 2/2. The entire locked workspace suite passed **2,647 tests, 0 failures, 1 ignored** across 113 result groups. `cargo fmt --all -- --check`, strict native workspace/all-target Clippy and strict WASM workspace/lib Clippy all passed, using the shared root Cargo target to avoid duplicate build storage.

On the independently authored UV quad with installed nonuniform masks and materials and one retained source-static level, preparation removed the retained hierarchy and original face/UV metadata in one Undo step. The visible position, triangle-index, RGB, mask and material vectors were exactly equal before/after (zero component/word differences); the resulting plain mesh reports adaptive detail available. `.obrush` save/load, Undo/Redo, repeated-conversion no-op, and explicit Morph Target, unbaked-layer and open-stroke refusal passed. The test does not claim equal visible normals, dynamic decimation, a persistent on/off mode, factor controls, normal-browser UI behavior, or SculptGL parity.

Next gate: expose the operation in normal native/browser topology UI with a clear warning about removed retained levels and UVs, then test the actual browser Undo/Redo/Save route against the pinned reference's activation/deactivation. Source dynamic-mode factors and local collapse remain unimplemented.
