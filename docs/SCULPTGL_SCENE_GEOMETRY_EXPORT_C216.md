# C216: scoped scene geometry export

The normal File panel now applies its All/Selected scope to OBJ, ASCII/binary
PLY and ASCII/binary STL as well as SGL. The shared Rust document exporter
includes hidden selected parts, bakes each editable affine pose into world
positions, preserves per-part OBJ object records, original OBJ tri/quad faces
and corner UV indices, and combines PLY/STL geometry with bounded vertex,
triangle and output allocations. Source document/history remain unchanged.
Formats that omit materials, masks, UVs, original faces or source
pose/provenance require the visible explicit loss checkbox; the report counts
those omissions. The same core method runs on native and WASM hosts.

The independent core test selects a hidden posed triangle and an ordinary
triangle while excluding a third part. Each of the five exports roundtrips to
two faces, six source vertices and the expected world-space point at X=11;
default strict policy rejects the pose loss, and whole-project JSON plus Undo
and Redo availability stay exact. All-scene OBJ includes the third part.

The normal Chrome check imports an independently authored two-object OBJ
through the Source File Add picker, selects its two new parts using visible
scene controls, enables the explicit loss and selected-scope controls, and
downloads all five formats using the visible File controls. The OBJ has two
object records, six vertices and two faces. Its six baked positions match the
saved imported affine applied independently to the six original local points
within `1e-5` per coordinate. Binary STL is 184 bytes for two facets; both
PLY headers report six vertices/two faces. A visible Save before and after all
downloads has identical SHA-256, and the browser recorded no runtime error.
The passing receipt is
`.artifacts/sculptgl-scene-export-browser/1fbfd95e-0f93-4731-bd7b-dd52bd3c34cb/result.json`
(SHA-256 `c7295f519d30efb08f6b0f5275110cda39f993c9bece8f13553d99ba3fd75e08`).
The runner is `scripts/check-sculptgl-scene-export.mjs` (SHA-256
`df51e5d787f378e3e8730fd805454536af393db4cab6c7a45d6922a34b617adc`);
the built portable/SIMD WASM SHA-256 values are
`a2e9cea622bfaf014e1c02a8726f14f3c5aeac118b9b800e4e1c76f8d5a15239`
and `fc3634d0605258e952c3793519a923bcd3be590a0a276243f616fe3e5e8b8832`.

The pinned SculptGL `ExportOBJ.js` source exports a mesh list with one `o`
record per mesh and transforms vertices by each mesh matrix. The checked
world-pose fixture follows that policy, but this run does not download a
matching file from the pinned SculptGL browser. In particular, SculptGL's
`#MRGB`/`#MAT` options and third material channel, exact PLY/STL attribute
encoding and a larger mixed-scene comparison remain open. OBrush reports
material loss rather than silently claiming those channels. Output currently
rejects a scene whose combined geometry exceeds the single-mesh 1M-vertex or
2M-triangle limits; large-scene throughput, native physical GUI and SculptGL
or ZBrush parity are unverified.

The first full-suite run exposed a preexisting UI test expectation that the
loss checkbox appears only when the active mesh has attributes; the checkbox
now remains available for a different selected part's source pose or material.
The first web build then caught a partially moved Rust report, and a later
build caught the diagnostic JSON macro's recursion limit after a new selected
IDs field; both were corrected. These failed logs are preserved under
`.artifacts/c216-tests.log`, `.artifacts/c216-build-web.log` and
`.artifacts/c216-build-web-metrics.log`.

Final verification passes `cargo fmt --all -- --check`, `git diff --check`,
strict native/WASM Clippy, `scripts/build-web.sh`,
`scripts/package-macos.sh` and its signature verification, and browser runtime.
The complete workspace run passed 2,518 tests with zero failures and one
intentional ignore across 103 result groups; the later diagnostic-only browser
metric edit was followed by a final 744-test app-library pass. Their log
SHA-256 values are
`c3c20cbb0a5004070888552ca3b3d5003a29b55bf19fb028deadfb76a06cfa8c`
and `fe26942a1f61f8209cd04f6a5636fe271f12de0f23045bbcbb775319f0e41636`.
The packaged native executable SHA-256 is
`cad82e92e3b5eaf71ab3052a6a424fdaa39580e5cf65b20786879cb97a059959`.
