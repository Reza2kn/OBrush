# C99: posed selected Smooth Surface Nets and Block

C98 corrected the pre-rounding world bounds for selected affine/source-origin remesh. C99 checks that same independently authored sheared red box and translated blue box against pinned SculptGL in its two quad extraction modes, Smooth Surface Nets and Block. The selected order is right then left, resolution is `16`, and the browser requires explicit consent to bake both poses and discard their source provenance. The test uses the real visible controls, saved project and GPU buffers. The baseline is the pinned hosted SculptGL bundle at upstream revision `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; no diagnostic bundle is used for this comparison.

| Mapped comparison | Smooth Surface Nets | Block |
| --- | ---: | ---: |
| Oriented reference quads matched, native / browser | 1,034 / 1,034 | 1,034 / 1,034 |
| Maximum native world-position difference | 1.3793e-7 | 1.2991e-7 |
| Maximum browser-save world-position difference | 1.6125e-7 | 1.3550e-7 |
| Maximum native RGB / material / unmasked difference | 0 / 0 / 0 | 0 / 0 / 0 |
| Maximum browser-save RGB difference | 2.9303e-8 | 2.9303e-8 |
| Maximum browser-save roughness/metalness difference | 2.3842e-8 | 2.3842e-8 |
| Maximum browser-save unmasked difference | 0 | 0 |

Each output has `1,036` vertices and `1,034` persistent quads. The isolated browser verifies visible additive selection, default refusal before pose/provenance-loss consent, source metadata removal, closed solid, visible GPU/save correspondence, and exact undo/redo. The shared Rust core test now also fixes the reference grid step `0.17187500186264515`, quad counts, color sums and pose/history behavior across both modes. The comparison script was corrected to interpret the version-7 unprotected mask convention directly; earlier protected-mask projects still invert that channel.

This is a paired result on one low-resolution two-box fixture. It does not establish arbitrary transforms, complex source meshes, mixed UVs under pose, many-part fusion, high-resolution responsiveness or complete remeshing parity. The browser still uses shared Rust/WASM CPU geometry and WebGL2 drawing; WebGPU compute/draw remains separate. Native GUI and physical stylus behavior were not exercised. [C99 evidence](evidence/sculptgl-posed-quad-fusion-c99-0.1.20.json) binds the fixture, pinned reference, native and browser comparisons, runtime artifacts and checks.

The final source passed `cargo fmt`, 2,391 workspace tests (one ignored fixture generator), strict native and WASM Clippy, the WASM target check and both official build scripts. The local macOS app's ad-hoc signature verified. Both isolated browser runs served the rebuilt WASM SHA-256 `a7a166a926d1484023b983f6af60558d0c1f589fea5ec5e06c35ceab132a9a84` without JavaScript or GL errors.
