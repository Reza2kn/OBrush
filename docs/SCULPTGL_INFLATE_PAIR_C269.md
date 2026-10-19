# C269 scoped pinned Inflate kernel pair

[The protocol](SCULPTGL_INFLATE_PAIR_PREREG.md) was committed as `edbe50b` before
the first reference run. The independently authored four-vertex diamond OBJ is
SHA-256 `9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35`.
The pinned hosted SculptGL bundle is SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The test called that bundle's actual `Inflate.inflate` method in an owned
headless Chrome session on the specified positions, normals, proxy and
material-mask factors. The input radius squared was `1`, center `[0,0,0]`,
alpha None, selected IDs `[0,1,2,3]`, and intensity `0.3` for both positive and
negative runs; a separate zero-intensity run was also made.

The reference's Z Float32 words were:

| Case | Vertex 0 | Vertex 1 (half mask) | Vertex 2 (full mask) | Vertex 3 |
| --- | --- | --- | --- | --- |
| Positive | `3ca8f5c3` | `3c28f5c3` | `00000000` | `3ca8f5c3` |
| Negative | `bca8f5c3` | `bc28f5c3` | `00000000` | `bca8f5c3` |
| Zero | `00000000` | `00000000` | `00000000` | `00000000` |

All X/Y words stayed at their input values. The largest difference between a
reference Z value and the independently preregistered real-number arithmetic
was `8.195638646901759e-10`, below the frozen `3.0e-8` tolerance. The full
reference receipt is [sculptgl-inflate-kernel-c269.json](evidence/sculptgl-inflate-kernel-c269.json),
SHA-256 `58756288a41913e28e8e91af3be679beb56618e15a12232e1407daece20945a5`.

Before adding the measured reference words to its assertion, the OBrush
shared-core test passed `1/1` in
`cargo test --locked -p obrush-core --test source_inflate_pair`. It checks the
same analytic displacements at the frozen tolerance, exact unchanged XY, mask
and topology, and exact public `.obrush` JSON Before→Undo and After→Redo
recovery. The then-current test executable SHA-256 was
`a1d183fd1c2c7b9e7cd89b4ef3299f57d754c6877424cdc878337166f9d8aa4a`.
The new explicit paired-reference-word assertion was added after the reference
run and has **not yet been executed**. Direct observed OBrush-versus-SculptGL
maximum/RMS differences therefore remain unreported. This report retains that
verification gap rather than treating a shared analytic oracle as an observed
paired numerical result.

The reference probe did not exercise normal SculptGL UI input or public Save,
Undo, Redo; the OBrush test used shared core and public JSON/history methods,
not the browser UI or GPU. The preregistered normal-browser cohort, broader
mask/alpha/pressure, altered normals over successive dabs, native GUI,
million-vertex responsiveness and the full `SGL-BRUSH-INFLATE` acceptance
gate remain open. No SculptGL or ZBrush parity claim follows from this small
direct-method witness.
