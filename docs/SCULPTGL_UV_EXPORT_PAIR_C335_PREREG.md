# C335: Paired color/roughness/metalness UV export

The current OBrush File panel exposes all five pinned texture sizes and all
three UV PNG channels. Its shared, display-free Rust baker passed substantial
independent OBrush PNG checks in
[the prior consumer report](SCULPTGL_UV_BAKE_CONSUMER_0_1_18.md).
The user-visible gap is **actual paired SculptGL PNG output**: coverage,
orientation, channel transfer, raster edges, and the 17×17 blur/gutter have
not been compared pixel by pixel on the same imported file. The full
`SGL-IO-TEXTURE-*` rows therefore remain open. No core algorithm change is
justified until the comparison locates a numerical difference.

## Frozen input and checks

Use `crates/obrush-core/tests/fixtures/independent-uv-export-gradient.obj`
(SHA-256 `67ab2881acab8d09b70ae394541d1ee6147803a95665dcbb263fee5233c54fc3`).
It is an independently authored two-triangle square, with a UV island from
0.125 to 0.875 in both axes and four distinct RGB and `#MAT` rows. It has
no external asset. Import the same bytes through both normal browser file
inputs, select the imported mesh, and export Color, Roughness and Metalness at
256 and 1024 pixels. Use normal File controls in OBrush and the closest
available normal reference controls. Capture actual downloads, not an
internal readback substituted for a PNG.

Decode each PNG RGBA independently and record reference/build hashes, exact
size and channel settings, full pixel-difference histogram, maximum channel
difference, changed pixel coordinates, covered-interior difference,
edge/gutter difference, and exact saved-scene hashes before/after each
export. A scoped pass requires identical decoded RGBA for all six pairs and
unchanged scene/history in each app. On failure, retain the original output
files and diagnose raster, color-space/channel transfer, gutter and PNG
orientation separately before editing `obrush-core`. Add one missing-UV
negative case: neither app should produce an accepted PNG or alter the scene.

The fixture is a preparation artifact, not reference-pixel evidence. Static
inspection shows pinned `ShaderPaintUV` renders an active mesh to a
256–4096 target, followed by one 17×17 blur pass for unfilled pixels.
OBRush implements a bounded CPU analogue. These observations do not prove
pixel equivalence. The pinned source is upstream commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; inspected `GuiFiles.js`,
`ShaderPaintUV.js`, and `ShaderBlur.js` SHA-256 values are respectively
`d7e4055e0cb20ff47fd29da9bd8380bf8d17020197c0b19f4cc2a9364d34263a`,
`6d52b6228aa5f8c5595c6a9e7ec5c2f22bc6387fe74ebb38fd40609f4fd2ba72`,
and `14fc3db71b62e8700655ca33a315b50ee343e52768b8261e02353dcd692f8855`.
Unmodified OBrush `uv_bake.rs` SHA-256 is
`a7d48d0726ce492dfedf29ebe898313a26e016533f89f86c51a5403ab04a665f`.

Arbitrary UV seams/overlap, all five sizes, source GPU/browser variation,
native GUI interaction, large textures and sustained responsiveness remain
open after this six-output fixture. No texture family parity is claimed.
