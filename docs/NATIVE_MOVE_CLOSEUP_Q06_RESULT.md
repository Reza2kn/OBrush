# Q06 Move close-up: failed camera calibration

Q06 ran its [frozen protocol](NATIVE_MOVE_CLOSEUP_Q06_PREREG.md) in owned
Chrome 155.0.8059.85 at 1600 × 1000 CSS pixels/DPR 1, with matching local
and served official WASM SHA-256
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`.
The exact failed result is retained at
`.artifacts/native-move-q06/37f0ee75-008a-4b60-80e5-185e28b51275/result.json`
(SHA-256 `e087420549de81c2617302e5732ac1a8de471f5b7ee01d72e8a54a9815076fab`).
The run exited 1; owned Chrome exited 0.

Both exact Q02 public projects loaded and saved without mesh changes. Three
wheel events reached `1.54684055` before and `1.54932547` after. The
frozen fractional after-only event `deltaY=-0.770475948625655` moved the
after camera to `1.53743410`, instead of matching before. Paired distance
error was `0.00940645` and view-projection maximum error `0.00940657`,
against the `1e-5` gate. The fractional browser wheel event evidently did
not follow the full-step scaling used to derive the correction. No further
calibration was attempted. Q06 therefore provides no valid paired pixel
or tearing verdict, though both original-resolution PNGs remain available
for descriptive inspection.

Offline analysis of the exact Q02 projects locates a sharp-looking edited
area near world `[-0.135,0.26,0.956]`. Within 0.06 world units of that
point, the undeformed sphere has 77 local triangles, minimum
`cross/longest-edge²` quality about `0.7273`, and maximum adjacent face
fold `0.71°`. The final mesh has 1,096 local triangles, minimum quality
about `0.1000`, and maximum adjacent fold `50.26°`, with five local edges
over `30°`. The first Q02 Move contact alone produced a local maximum fold
of `71.49°`. These are strong numerical and visual clues for the user's
triangular bump, but they are not a matched-camera image verdict. The
existing Move deformation guard allows quality `0.1` and up to `90°`
adjacent folds, so those shapes satisfy current safety constraints.
