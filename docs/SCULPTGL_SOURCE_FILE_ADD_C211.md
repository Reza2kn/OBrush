# C211: Source File Add accepts mesh formats

The SculptGL-profile **Ctrl+O/I** command and normal File-panel Add picker now
accept `.sgl`, `.obj`, `.ply` and `.stl`. SGL still follows the captured native
word-order/header path. OBJ/PLY/STL use the existing bounded shared-Rust parser,
then a new shared-core source-option preparation and atomic metadata-preserving
SubTool append. Browser and native hosts call the same core geometry path.
Malformed files and a changed request revision reject before installation;
the original document and its Undo/Redo remain available. Source `modelurl`
startup for OBJ/PLY/STL also uses this path, including `scalecenter=1`.

For a mesh import, original local point and explicit color words remain in the
mesh by default. With **Scale and center**, OBrush keeps the local points and
stores the source-style `100 / diagonal` uniform normalization in an editable
local-to-world affine. With **sRGB vertex color** unchecked, only explicitly
imported RGB rows undergo the shared-core linear-to-sRGB transfer; STL/default
colors are not re-encoded. The imported normal policy is preserved. The
independent tetra OBJ has local bounds `[1,2,3,3,6,9]`, diagonal
`sqrt(56) = 7.483314773547882`, stored scale `13.363062` and local center
`[2,4,6]`. The checked browser save retains all four original local points and
its explicit RGB `[0.5,0.25,0.75]`. The same 120-byte OBJ was loaded into the
pinned hosted SculptGL kernel (SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`)
with `scalecenter=1`. All 16 stored affine `f32` words and all 12 RGB `f32`
words match. Independently transformed world points have maximum nearest-point
component difference `1.100e-6` when OBrush's JSON decimal values are parsed as
JavaScript numbers. This is one paired file; face order, render pixels and
multi-object parsing remain unchecked.

The rebuilt normal browser used actual Ctrl+O, the intercepted file chooser,
the hidden input, visible top-bar Save, keyboard Undo/Redo and a literal
same-origin `modelurl`. Independently authored OBJ/PLY/STL files appended from
one to four scene parts, and the saved four-part project survived an empty SGL
Add and malformed OBJ attempt unchanged. The OBJ Add saved-project SHA-256 was
`c6d62064dd2dcffc379149415edb932dbad24b7f885b7010c125af29b840cb0e`;
Undo yielded `2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`,
and Redo reproduced the first hash exactly. The final four-part, post-empty-SGL
and post-malformed hashes were all
`515d251c1b60d2b54551bc5c386d2ab8e6246eeed9fdddbe9f803b94f3b78ac2`.
The startup URL installed its OBJ as one part with the same `sqrt(56)`
normalization. No browser exception was recorded. The final result is
`.artifacts/source-file-add-browser/94ae0fca-9be9-4e80-9c6d-ca80d58f5ac8/result.json`
(SHA-256 `17055a8e5b23b3e95603661395d47864798f63d1d432c605a97114047e1fdb41`);
the served portable WASM SHA-256 is
`2074ab5822b0c534281bcc9388c3aebb951be7c12aaca8baca09d7ee9b0405dc`.
The test runner is `scripts/check-sculptgl-source-file-add.mjs`.

Failed checks remain under `.artifacts/source-file-add-browser/`: the initial
runner queried stroke metrics instead of app metrics, lacked file-chooser
interception, tried the Source-profile Ctrl+S paint shortcut instead of the
visible Save button, and percent-encoded a `modelurl` that the pinned query
grammar retains literally. Each was corrected in the runner; none was counted
as a passing import. A paired-reference attempt clicked its hidden input
without a user activation and opened no chooser; direct CDP file-input
selection on the actual pinned page passed. The first strict native Clippy run caught an eight-argument
private validator after the metadata-budget change; it was refactored and both
native/WASM strict Clippy passed.

Final verification passed: `cargo fmt --all -- --check`, `git diff --check`,
2,512 workspace tests with zero failures and one ignored across 103 groups,
strict native and WASM Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh`. The packaged macOS app passed `codesign --verify`
and its executable SHA-256 is
`a50c4e60573bb6d5ecd18dd64bc58c232c5fb7b32f64835b4b5e307f0f40202d`.
The final test log SHA-256 is
`ece1b2c8e3dbba1bf13eda805a7ad42e6f9d4b72bc883969ef695057d52d296e`.

The pinned source may return multiple objects from one OBJ/PLY/STL file;
OBRush's current general mesh parsers produce one mesh per file. Multi-object
and material/UV interchange, broader reference geometry/pixel matching, native GUI
interaction and sustained large-mesh responsiveness remain open. This is a
scoped File Add workflow improvement, **not** SculptGL or ZBrush parity.
