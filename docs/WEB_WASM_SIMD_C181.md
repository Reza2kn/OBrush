# C181: portable and SIMD browser document fingerprints

The browser build now ships two versions of the same Rust application: a
portable WASM package and one with BLAKE3's WebAssembly SIMD implementation
enabled. Startup selects SIMD only after a `v128` feature probe; a failed SIMD
load falls back to the portable package. `?wasm-scalar=1` forces the portable
package for diagnostics. Shared document geometry, sparse-result validation,
file format and Undo/Redo are unchanged. This reduces one CPU stage of the
experimental GPU Brush stroke; it does not replace WASM with WebGPU or make
the regular studio GPU-resident.

The independently generated `999,002`-vertex SGL sphere used by C180 has
SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`.
With the same three-contact, radius/strength and camera browser trace, the
SIMD and forced-portable runs both changed 5,171 vertices. Their before,
after, visible Undo and visible Redo viewport PNG hashes were identical.
The checked document's source-match stage took 56.4 ms with SIMD versus
121.9 ms with portable WASM. Complete release took 373.6 ms versus 441.3 ms
in these individual runs. These are local wall times, not a controlled
reference comparison or a claim of sustained speedup.
The UI settings captured for both traces were Draw, radius `0.1800000072`,
strength `0.3499999940`, X symmetry on, accumulation on, hardness zero,
world-space radius, and a `[225,57,1365,882]` viewport. Pressure affected
strength, but the CDP mouse contacts had no pen-pressure variation.

An owned browser with SIMD feature detection deliberately returning false
selected the portable package, sculpted the startup sphere, and recovered
the same before/after pixels through visible Undo/Redo. The saved result is
`.artifacts/gpu-pointer-c146/c181-no-simd-fallback/result.json` (SHA-256
`de27a8a5c6f8b0803e26f1099cdccb5dfb1f855e7affec683ea2b83880fca42e`).
The paired million-vertex results are
`.artifacts/gpu-pointer-c146/c181-dual-simd-r0/result.json` (SHA-256
`36e124eeed38724eb9e0434c1d79e03525dfed943d030f3679b06f04cad54888`)
and `.artifacts/gpu-pointer-c146/c181-dual-scalar-r0/result.json` (SHA-256
`dc346c8f33401a8da302ae5f5f25500843d0651c3a79cc96246d9e33b7710bcb`).

The 10-second million-vertex SIMD run sent 223 pointer moves, received 225
contacts including press and release, coalesced 112, and applied 113 GPU
dabs. It changed 25,484 vertices and restored exact before/after viewport
hashes through Undo/Redo. RAF interval P95 was 50.1 ms, maximum 449.9 ms,
and release 470.4 ms, with 378.6 ms in the checked commit. The sparse
position map was 509,680 bytes. It **fails** the existing RAF P95 ≤33.34 ms,
RAF maximum ≤250 ms and release-idle ≤100 ms gates. Run evidence is
`.artifacts/gpu-pointer-c146/c181-simd-million-long-r0/result.json`
(SHA-256 `8950754a6a7250f5a4a6a34ade2d5a9c52b505e6cd3a8330ea67ae4eee2988e3`).
Its 25,484 changed vertices differ from C180's 25,482 because the timed
pointer trace delivered a different number and spacing of events; these
two long runs cannot be treated as a paired numerical result.

The paired packages' WASM SHA-256 hashes are portable
`758085510b6ad94a3433ca2c21a026c8b7b20f9c04bdc0cf8ef16da8d267d26e`
and SIMD `d6469fbefa854002261606a92b6a6aa0bca62c9bcefd8e9d0a97b1d14a1e2df7`.
The final HTML SHA-256 is
`4fd9012ac3983ad7bbe6346a851a2a4c5fef7ed202688a710960c49c0620c5e8`.
The browser run used these locally built packages. The normal studio's
WASM geometry/WebGL2 path, matched SculptGL performance and ZBrush parity
remain open.

Final checks passed: `cargo fmt --all -- --check`, 2,494 workspace tests
across 103 groups (zero failed, one ignored), strict native and WASM-target
workspace Clippy for both portable and SIMD feature builds,
`node --check web/resident-pointer.js`, `scripts/build-web.sh`,
`scripts/package-macos.sh`, and `git diff --check`. Fresh owned browser
startup/visible-history checks passed for both the SIMD-capable build and
forced portable fallback after the final build. These checks do not change
the failed responsiveness result above.
