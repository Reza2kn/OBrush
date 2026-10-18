# C98: posed-source voxel bounds and overlap colors

C97 matched the geometry of one selected affine/source-origin pair but left a maximum RGB difference of `1.0` where red and blue surfaces overlap. The cause was upstream of interpolation: SculptGL grows its voxel box from each world-space transformed coordinate before writing that coordinate into a `Float32Array`. OBrush had grown the box from its already rounded `f32` mesh copy. That changed the grid origin and voxel size by a few parts in 10⁸, enough to change which equal-distance source surface owns hundreds of color samples.

The shared Rust core now carries an optional pre-rounding world bound into selected remesh when a selected SubTool has affine/source-origin metadata. It still makes bounded temporary `f32` world-space mesh copies for geometry, retains the original local meshes for undo, and validates before replacing the document. Unposed selected remesh and ordinary one-mesh remesh continue deriving bounds from their mesh positions. An independently authored SGL pair has a left X shear of `0.15` and translation `-0.3` and a right translation of `+0.3`; selection is right then left. Both outputs use Marching Cubes resolution `16`, with Relax off and on.

| Mapped comparison with pinned SculptGL | Relax off | Relax on |
| --- | ---: | ---: |
| Oriented triangles | 2,064 / 2,064 | 2,064 / 2,064 |
| Maximum world-position difference, browser save | 2.045e-7 | 3.134e-7 |
| Maximum RGB difference, browser save | **2.930e-8** | **2.930e-8** |
| Maximum roughness/metalness difference, browser save | 2.384e-8 | 2.384e-8 |
| Maximum unmasked-weight difference, browser save | 0 | 0 |

The native shared-core output has **exact mapped RGB, material and unmasked values** on this fixture for both Relax states. The browser save retains small nonzero RGB/material differences, and the browser's visible GPU buffers agree with its saved project. The initial C97 browser RGB maximum was `1.0` with RMS `0.3311`. This C98 result closes that *specific* posed two-box color failure, not arbitrary overlap or full remeshing parity.

An isolated diagnostic copy of the MIT-licensed pinned SculptGL bundle exposed its voxel field before flood fill. Its resulting positions, faces, colors, materials and matrix were exactly equal to the unmodified pinned run on both cases. The diagnostic showed the reference grid step `0.17187500186264515`; OBrush's earlier rounded-bound step was `0.171875`. The diagnostic copy is not used as the reference oracle, and all source changes and failed trials remain in ignored local artifacts. A f32-bucket tie rule was tried in OBrush and rejected: it reduced but did not remove the color error. The final change uses the unmodified distance-ownership rule with the corrected grid bounds.

The core regression checks the grid step, paired red/blue color sums, default provenance-loss rejection, explicit consent, three extraction modes and exact undo/redo. The isolated browser uses the visible selected-remesh controls and verifies consent, closedness, save/GPU agreement and history for both Relax states. The [C98 evidence](evidence/sculptgl-posed-voxel-bounds-c98-0.1.20.json) records source, fixture, reference, build, comparison and failed-trial hashes.

The final source passed `cargo fmt`, 2,391 workspace tests (one ignored fixture generator), strict native and WASM Clippy, the WASM target check, and both official build scripts. The local macOS app's ad-hoc signature verified. The isolated browser loaded the served WASM SHA-256 `a7a166a926d1484023b983f6af60558d0c1f589fea5ec5e06c35ceab132a9a84` without JavaScript or GL errors.

Arbitrary affine coefficients, asymmetric overlaps, multiple posed/unposed selections, UV retention, complex color gradients, sustained million-vertex responsiveness and native GUI sculpting are still unverified. Production browser sculpting remains Rust/WASM CPU geometry with WebGL2 rendering; WebGPU compute/draw is a separate prototype. ZBrush parity is not inferred.
