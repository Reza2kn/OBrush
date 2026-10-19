# C224: SculptGL-profile binary STL facet return

The SculptGL-profile File > STL binary selection now uses a separate shared
Rust writer. It produces the pinned exporter's blank 80-byte header, little
endian triangle count, float32 facet positions/normals and VisCAM-style
averaged 5-bit RGB attribute words. The ordinary OBrush STL writer remains
separate. Scene selection, world-pose baking, loss reporting and history
isolation still go through the common scene geometry export path.

An independently authored colored triangle checks the blank header, count,
facet normal, positions, color word and unchanged input mesh. A selected
two-object normal-browser export was compared with the tail of a pinned
SculptGL **File > Save STL** download after importing the same OBJ in both
applications. The reference file has 196,610 facets and 9,830,584 bytes,
SHA-256 `26a386f14591766e26c873e56b55da9b873d36e75486d392a0304dab57f67509`.
The two imported-object facets (100 bytes) match OBrush byte-for-byte, with
zero differing bytes; both 80-byte headers are all zero. The OBrush selected
file has two facets, 184 bytes and SHA-256
`e567a63041566c60b2ae41baf874edca3453058c41691bff15b5bdd1f88c2d02`.
The passing normal-browser receipt is
`.artifacts/sculptgl-obj-color-browser/1b92a148-ff17-4af7-ac08-7e5a86decbdb/result.json`,
SHA-256 `167014b66324a20a2ecad18cfa19469a79dbcb756105fba4271d397d3858182a`.
The served portable WASM SHA-256 is
`5b591faca1e520b37d1b542b1d824b4deb024f1cb1b0d7eb15163fef1dafd91f`.
Strict native and WASM Clippy and the packaged macOS ad-hoc signature pass;
the packaged executable SHA-256 is
`0e03bd4b359323655f7ea69788207743fe086978f83226c1324274aaedc1b471`.

The first full workspace suite found a **test-oracle error**: the independently
assembled expected facet used positive zero for a normal component, whereas
the source-compatible float32 cross product and observed browser bytes use
negative zero. The failed log is retained at
`.artifacts/c224-workspace-tests.log`; the corrected focused test and final
workspace rerun pass. The final `cargo test --workspace --locked` result is
2,530 passed, zero failed and one intentional ignore across 83 result groups.
`cargo fmt --all -- --check`, `git diff --check`, strict native/WASM Clippy,
`scripts/build-web.sh` and `scripts/package-macos.sh` pass.

This check covers the plain Save STL option on one two-object planar fixture.
It does not cover source color half steps, skewed/degenerate normals,
alternate affine transforms, Sculpteo/Materialise coordinate or color
options, native physical GUI, arbitrary STL import/export return, large-scene
latency, or full SculptGL parity. The million-vertex responsiveness gates
remain failed.
