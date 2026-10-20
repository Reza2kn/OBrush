# C283 Smooth Tangent baseline: setup stopped before OBrush sculpt

The pinned Source normal UI completed Tangent-on with the independent
441-vertex fixture, exact 64px/50% controls, held GPU and public
Before/After/Undo/Redo. Raw Source-on record:
`.artifacts/sculptgl-smooth-tangent-c283-ui/source-on-b7410e67-7097-4a6c-a1dd-d010167ec793/result.json`.
The unchanged C283 harness SHA-256 was
`32808829523761b1dab53cee2fc99d46d9ba1e7d10a29b3dc3abb1c26cb91dea`.

The first OBrush-on run failed before it touched the sculpt mesh:
`Normal control offscreen source_geometry.pressure.radius.value`. Raw
failed record:
`.artifacts/sculptgl-smooth-tangent-c283-ui/root-on-0e373345-0b05-44c5-9891-8dad129a2cd3/result.json`.
The frozen harness had switched to Smooth but requested Source numeric
controls while the per-tool mode was still `Legacy`; the actual UI only
renders those numeric controls once mode is `SourceOrdinary`
(`source_geometry_ui::render`). The absent control is a test-setup
sequencing error. C283 has no paired numerical result and establishes no
Smooth Tangent product defect or parity pass. The Source-on and failed
OBRush-on receipts are retained without rewriting their harness or
acceptance limits.
