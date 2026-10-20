# C325 Pinch viewport-origin setup failure

The [C325 setup-only correction](SCULPTGL_SOURCE_PINCH_C325_PREREG.md)
completed both pinned SculptGL Negative-off and Negative-on ordinary-browser
strokes and their public Before/After/Undo/Redo exports. Their owned raw
receipts are locally ignored under
`.artifacts/sculptgl-source-pinch-c325-ui/source-off-c6de0e90-5d0a-4e4d-af19-94827901e256/`
and
`.artifacts/sculptgl-source-pinch-c325-ui/source-on-6ad4160c-5167-4b87-8063-42497b9a937d/`.
Receipt SHA-256 values are respectively
`c8f23ff0b5fc4706935622b88c266ab83066891da81b885b3870e6c7a0748fe9`
and `5a60def84f33205082ef0ff1919d1948b0c8b966ebb87d3afddf247da1977dd4`.
On each reference side, Undo exported the exact Before bytes and Redo the
exact After bytes. This is reference capture only; paired geometry/GPU gates
were not scored.

The first OBrush Negative-off session then stopped **before pointer contact**.
Its raw receipt is
`.artifacts/sculptgl-source-pinch-c325-ui/root-off-3a98cd6c-3fb1-4292-8733-d2da26f5c71b/result.json`,
SHA-256 `8d16443b0ca33172ca5b86990d3a8d4444c7c6b59e1ab0e57d15251d6873084e`.
The actual `viewport.pixels` packet was `[225,31,1024,768]`: first two
numbers locate OBrush's inset sculpt rectangle within the browser window;
last two are its already-correct width and height. The C325 harness
incorrectly required the full packet `[0,0,1024,768]`. Camera settlement
completed over 12 unchanged terminal frames and 596 ms. This is a second
harness setup error, not a Pinch deformation outcome. No OBrush After, Undo,
Redo or GPU pair was produced; Negative-on OBrush was not attempted.

The C325 reference Save hashes for Negative off were Before/Undo
`085bc2a4979611febb28cba58fe4a0a3681fc4b292e5d7f97c4164e370638f32`
and After/Redo
`3b0976a220e4b12c1c9f831efd64203ac0adda92dba57ef8c42370cb83cb4ed0`.
For Negative on they were the same Before/Undo hash and After/Redo
`74dead0a223e657db288a996b3051cad346c13f155764ef6d5ff278a54011a0c`.
These hashes bind the exported files but do not establish the frozen four-run
matrix. C308's earlier failed setup remains separately preserved.
