# C180: GPU-owned picked Brush preview

The opt-in `?webgpu-brush-preview=1` path now accepts a real one-stroke
browser pointer gesture for ordinary Brush. The GPU ray query, original-face
picked direction, displacement, selected normal refresh, and presentation use
resident buffers. No pick or mesh readback occurs between dabs. At release,
the shared Rust document validates a sparse position result and owns the
single Undo/Redo step. The regular studio still uses WASM brush geometry and
WebGL2 rendering; this remains a separate canvas and an opt-in preview.

The picked-direction pass maps the winning triangle to its original triangle
or quad, interpolates current raw corner normals by inverse distance, and
normalizes. It runs once before each picked Brush candidate. The WGSL source
is in `obrush-core` beside a scalar direction oracle, preserving the shared
native/browser geometry contract. The already checked candidate and normal
transaction apply the result. The current pass covers ordinary accumulated
Brush with no alpha. Source first-touch proxy, nonaccumulation, symmetry,
clay, alternate masks/policies and a complete reference comparison remain
open; this implementation does not assert Source Brush parity.

An independently generated 162-vertex sphere project (SHA-256
`ab16459719d55e25d49fa8c2cc0fd16809749a991a6d08e960f6a822d34afe0a`)
was picked from ray origin `[0.17,0.11,3]` along `[0,0,-1]`, max distance
`5`, then brushed at radius `0.5`, intensity `0.25`. Native Metal picked
triangle 249 at `[0.1700000018,0.1099999994,0.9676600695]`; the scalar
reference direction was `[0.1455348432,0.1381114125,0.9796656966]`.
The maximum output component difference against scalar was
`3.4924596548e-10`; the 200-byte sparse result committed, and Undo, Redo,
and cancel recovered exact states. Browser WebGPU produced all 648 f32
position words byte-exactly equal to native Metal, the same sparse byte count,
and an empty canceled result. A separately authored nonplanar original quad
with explicit 0–2 diagonal tested the second triangulated face mapping;
its maximum GPU/scalar output difference was `3.7252902985e-9`.

The normal browser pointer check on the 40,962-vertex startup sphere received
three contacts, ran three GPU-owned dabs, committed 334 changed vertices, and
recovered exact before/after viewport PNG hashes through visible Undo/Redo.
Release took 25.4 ms in the final-source run. The 999,002-vertex fixture (SGL
SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
also passed a three-contact pointer and visible-history check: 5,171 vertices
changed, but release took 448.5 ms. These runs do not establish sustained
responsiveness.

The separate 10-second million-vertex run requested 221 pointer moves at
roughly 10 ms spacing, received 223 contacts including press and release,
coalesced 111, and applied 112 GPU dabs. It changed 25,482 vertices; the
509,640-byte position payload is sparse. RAF interval P95 was **50.1 ms**,
maximum **533.3 ms**, and release **539.2 ms**. The visible scene changed and
Undo/Redo restored exact viewport PNG hashes, with no application, console,
GPU, or cleanup errors. This is a failed responsiveness result against the
existing RAF P95 ≤33.34 ms, RAF maximum ≤250 ms and release-idle ≤100 ms
gates; the unchanged normal-studio app CPU gate is not measured by this
separate preview. It is not a SculptGL performance comparison. Full-mesh GPU
passes and checked document replacement remain significant work.

Final evidence files are `.artifacts/history-limit/c180-picked-native.json`
(SHA-256 `dc11e88cffff281ce76538bae147ae7bdbad715c5d90b68aa7297523ed910ba3`)
and the `result.json` files under
`.artifacts/gpu-pointer-c146/c180-picked-final/`,
`c180-brush-pointer-final/`, `c180-brush-million-final/`, and
`c180-brush-million-long-final/`. Their SHA-256 hashes are respectively
`8733cebe9497cd87daab6f4adc19a104802e83ced7c4ed53020ca7129adf15d2`,
`ca01346bcccfc1667ae95c90e08fdc42e0cd875da272d3aa0d76156ddfd6d19e`,
`cdf32b0501df32c209e0c7e8ef2a5b3ce4f4a2542d75f25185700285a8c6cfac`,
and `92af86ec5de1b81d9c0e78c5afa54e35040d88d6817e6e6b41533dc2e6017125`.
The GPU WASM, pointer host and HTML SHA-256 hashes are
`8ab7c37755a738e3548f46e7648274fee8db0e13db45201f2fdb7bbf4bfb69d4`,
`25a5a29c9b9951319ef45c7746348301dd0b634434ca30b52f768e52035df20a`,
and `a075bc96cf8c1a19792b713dfaa91a4ecab3341501f04c240252675bd7e038e0`.
The authored-quad construction is preserved in
`crates/obrush-gpu/src/bin/resident_picked_brush_probe.rs` (SHA-256
`134314b0a7a8df740a0956d78e9c4057767ef8ebe0d993294973b4ef8f974550`).

The initial authored-quad probe failed because generic OBJ import emitted
two independent triangles; attaching one original quad without matching its
canonical 0–2 diagonal then failed validation. The final independent test
explicitly set both canonical triangles and original quad metadata before
capture. Those two failed runs are retained in the command record and do not
count as passing comparisons. The first attempted repeat browser run reused
an owned Chrome profile directory and failed with `EEXIST`; its result is
preserved as `c180-picked-browser/result-failed-rerun.json`. Fresh final run
directories then passed. The failed rerun overwrote the first direct browser
result JSON; its earlier pass survives only in the command output and the
fresh final direct run is the retained pass artifact. The other exploratory
browser results remain in their `-browser` directories and are not substituted
for the final-source measurements above.

Final checks passed: `cargo fmt --all -- --check`, 2,494 workspace tests
across 103 groups (zero failed, one ignored), strict native and WASM-target
workspace Clippy, `node --check web/resident-pointer.js`,
`scripts/build-web.sh`, and `scripts/package-macos.sh` with valid local app
signature. The final browser checks used the rebuilt WASM and host files;
all owned browsers exited cleanly. These are implementation checks, not
SculptGL or ZBrush parity evidence.
