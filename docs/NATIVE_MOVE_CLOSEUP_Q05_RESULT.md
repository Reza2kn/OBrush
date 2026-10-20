# Q05 Move close-up: failed paired-camera gate

Q05 ran its [frozen protocol](NATIVE_MOVE_CLOSEUP_Q05_PREREG.md) in owned
Chrome 155.0.8059.85 at 1600 × 1000 CSS pixels/DPR 1. The official local
and served WASM SHA-256 both matched
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`.
The exact failed result is retained at
`.artifacts/native-move-q05/aebfa7a5-928c-40ef-b656-b0545304d0d6/result.json`
(SHA-256 `cf69207fe6d1ab5d6a668481a7d6fef24e9cf99c2ddb3855fb370e7acf9de49a`).
The run exited 1 and owned Chrome closed.

Both Q02 public projects loaded and saved with their exact input SHA-256
(`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
initial, `5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632`
final). Navigation left each mesh unchanged. The ordinary pointer move
enabled all three wheel events. Initial-mesh camera distances were
`3.27466154 → 2.55030894 → 1.98618257 → 1.54684055`; final-mesh
distances were
`3.27992177 → 2.55440569 → 1.98937321 → 1.54932547`.
`frame_scene()` on public Open fits each mesh separately. The initial
camera distance differed by `0.00526023` and view-projection by
`0.00526047`, exceeding the frozen `1e-5` paired-camera tolerance. The
final cameras also differ. Q05 therefore fails as an image comparison.

Both 1600 × 1000 original PNGs were saved. Initial image SHA-256 is
`d79de89dd941cf6ec6e3178caadedd2b439ade0d27c52651fcc7a790d7637ced`;
final image SHA-256 is
`d9dbb390dd337a6cecf3487ade57c89251d597411526837d240289da3effe141`.
The frozen ROI has 1,423 pixels with an RGB-channel difference above 8,
but this count is descriptive because camera matching failed. The final
image appears to have a sharp triangular-looking bump near original pixel
`[330,340]` that the initial image lacks. This is a visual clue, not a
passed paired quality verdict. Q06 will measure it at matched camera pose.
