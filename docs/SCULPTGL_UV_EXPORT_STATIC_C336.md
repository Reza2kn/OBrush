# C336: UV export static-path comparison before the paired run

This is a source inspection, not a browser result. It does not change the
product. The pinned SculptGL source is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and the OBrush base is
`1ecc42def96bd8dbe90b340d4aa4b8233ac134a1`. The independently
authored C335 fixture and six-output frozen browser gate remain unchanged.

| Stage | Pinned source behavior | Current shared Rust behavior | Static conclusion |
| --- | --- | --- | --- |
| Input channels | `ShaderPaintUV` interpolates vertex RGB or material x/y, writes opaque RGBA. | `UvBakeJob::plan` chooses RGB or material x/y with implicit material defaults. | Same declared channels; fragment precision and conversion remain unmeasured. |
| UV transform and output orientation | Vertex shader maps `(u,v)` to `(2u−1,1−2v)`; `readPixels` bottom row is copied to Canvas2D first row without another flip. | `plan` maps to `(u*n,(1−v)*n)` and PNG row zero is high V. | Same orientation on paper. |
| Color target | `RttPaint` is unsigned-byte RGBA and nearest-filtered with repeat wrap. | Raster quantizes covered RGB to bytes before gutter sampling, and `wrap` repeats at edges. | Same target/sampler intent; GPU coverage and rounding remain unmeasured. |
| Blur | `ShaderBlur` samples a 17×17 neighborhood, only when center alpha is below 0.1; it decodes sampled sRGB, averages by total sampled alpha, re-encodes sRGB, and writes alpha one. | `pixel_sum` decodes covered byte RGB, rolling 17×17 `columns` are averaged only for uncovered pixels, final alpha is 255. | Same declared footprint and transfer; floating-point order/rounding remain unmeasured. |
| Overlapping UV faces | `GuiFiles` constructs the PaintUV RTT with `depth=null`, so it has no depth attachment. Despite Scene's current depth-test state, later fragments can replace earlier ones on the color target. | Raster processes faces in order and later covered pixels replace earlier ones. | The intended ordering appears aligned; an overlapping fixture is still needed. |
| Resource behavior | Source allocates two GPU RTTs and a Canvas2D PNG path for chosen 256–4096 dimension. | Core bounds raster work/scratch, moves captured mesh to worker, emits bounded PNG. | Different architectures; peak memory and responsiveness require runtime measurements. |

The relevant pinned source SHA-256 values are `GuiFiles.js`
`d7e4055e0cb20ff47fd29da9bd8380bf8d17020197c0b19f4cc2a9364d34263a`,
`ShaderPaintUV.js`
`6d52b6228aa5f8c5595c6a9e7ec5c2f22bc6387fe74ebb38fd40609f4fd2ba72`,
`ShaderBlur.js`
`14fc3db71b62e8700655ca33a315b50ee343e52768b8261e02353dcd692f8855`,
`Scene.js`
`092d17a08278700d6d85d3f8b9880e9152ce8c71be607fbe1584d0c5b5602f52`,
`Rtt.js`
`63978b3765cd3b75773873496a03bd8ab57a882267cdc74820145ace377a549b`,
and `colorSpace.glsl`
`825018b5f5cc7dedcc4703c8241b865fe46d11d4cb8e652b72d6c75ca798e70b`.
Unmodified OBrush `uv_bake.rs` SHA-256 is
`a7d48d0726ce492dfedf29ebe898313a26e016533f89f86c51a5403ab04a665f`.

No source-proven product defect justifies changing the shared baker before
C335's normal-browser same-byte PNG comparison. In particular, changing
top-left raster rules or sRGB arithmetic from inspection alone would risk
regressing exact independently checked covered pixels. The next action is
the frozen paired run; preserve any numerical failure before a bounded core
candidate is proposed. Full `SGL-IO-TEXTURE-*` gates remain open.
