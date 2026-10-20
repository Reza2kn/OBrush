# T01 dynamic topology conversion preregistration

Reference: pinned SculptGL `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, `src/gui/GuiTopology.js` `dynamicToggleActivate` (SHA-256 `db0253fea0bad1fbfa14b081a530ed226dcc72b295ddf98078082091ca1b96ad`) and `src/mesh/dynamic/MeshDynamic.js` `initFromMesh`/`setFacesFromMesh` (SHA-256 `974bb3c07dac18090ef821981f96f40a4155f104524e4eed108a6c56b407a123`). Candidate baseline: OBrush `1ecc42def96bd8dbe90b340d4aa4b8233ac134a1`. Independently authored fixture `crates/obrush-core/tests/fixtures/independent-dynamic-quad.obj`: SHA-256 `c18fb0d1445a0c98ebae6f86b0dd256e351b9abbe8779ca9d00a748798dafffd`. Candidate source hash must be recorded by the executor before tests.

## Scope and gates

This candidate is a **shared-core conversion foundation**, not SculptGL dynamic-mode parity. On a textured, colored asymmetric quad, install nonuniform masks and materials, add one retained source-static level, then convert the visible fine mesh to OBrush's plain adaptive triangle surface.

1. Before mutation, validate the whole candidate against existing mesh, scene, history and encoded-project limits. A failed conversion must preserve current serialized state and redo. An open brush stroke is rejected without ending or altering it.
2. Conversion is one undoable operation. It removes the retained hierarchy and original face/UV metadata, retaining **exact** position, triangle-index, RGB, mask and material words at each visible vertex. It must report removed level/face/UV counts. The resulting mesh accepts the existing adaptive brush path; a second conversion is a history-preserving no-op.
3. Undo restores the original hierarchy and UV-bearing quad surface byte-for-byte in `.obrush` JSON. Redo recovers the converted state exactly. Both states reimport through the public project parser.
4. A bound Morph Target or unbaked geometry layer blocks conversion with unchanged project/history rather than silently deleting user data.

Comparison still required before promoting `SGL-TOPO-DYNAMIC-ACTIVATE`: run the same fixture in a normal pinned SculptGL browser and OBrush browser, record exact pre/post mesh/channel arrays, face and UV counts, Undo/Redo, save/reload, and hashes. SculptGL's dynamic on/off mode, subdivision/decimation factors, linear toggle, mesh simplification, pressure and long-stroke behavior remain separate open gates. The core operation here does not claim any of those controls.
