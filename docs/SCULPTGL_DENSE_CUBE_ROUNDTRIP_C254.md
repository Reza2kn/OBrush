# C254: direct dense-cube browser history and project round trip

The C249 Source-profile cube matched the pinned SculptGL default's raw
98,306 welded vertices, 98,304 original quads and 196,608 render triangles,
but its new dense mesh had not been exercised through the normal browser
Undo/Redo and project Save/Open controls. This check extends the paired C248
browser harness without changing the Rust implementation. The pinned
SculptGL source remains commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
and bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

The browser used OBrush commit `a4b64005f8d24eaa9fe08dbe8558dfbcfb458383`
and app WASM SHA-256
`a69b490da76b6075f8a73684f8b5596e0d28ece6415a6d960686fa1a7eae2a2d`.
The harness SHA-256 is
`3db3b8525bdc47df76872bfcadd50dc76933baa9c2d7b44773a36a2691a8314f`.
Its independently authored six-vertex mixed-face OBJ is imported first;
both applications then add Sphere, Cube, Cylinder and Torus, undo and redo
Torus, and clear the scene. OBrush then adds the dense Cube through the
visible scene control, undoes and redoes it, saves it through the visible
top-bar control, clears the scene, loads the saved `.obrush` through the
normal file input, and saves it again.

The passing receipt is
`.artifacts/sculptgl-primitives-browser/2fcab0db-eab3-4c28-b30c-0a90d62c2030/result.json`
(SHA-256 `6adc8bcba3788aedd9e3c5e09ee7831c4d4097caf23bdc532f1a4896f2653e51`).
The direct Cube had 98,306 vertices / 98,304 original quads / 196,608
triangles. Undo left an empty scene; Redo restored all three counts. The
first saved JSON contained those same counts. After Clear and actual Open,
the app reported `Loaded ...`; the second saved JSON was byte-for-byte
identical to the first at 12,204,939 bytes, both SHA-256
`bc408d7dd494c612a29a735e75a94828d4fd234e1ce83d44351d7648db9c6b20`.
No runtime exceptions were recorded.

Three failed harness attempts remain preserved at
`.artifacts/sculptgl-primitives-browser/7ce8a41e-9c32-4ada-abfd-b84d85166b6e/`,
`1e06d741-16a0-46e2-9b60-218e166fdccc/`, and
`b45e5fce-a23b-4e3b-b782-f147c7e2a8e4/`. The first Ctrl+S invoked
Chrome's page save; the next Meta+S produced no OBrush download in this
Source-profile browser run. The harness now clicks the visible Save project
button and observes the browser's completed download event. An intermediate
passing attempt did not prove reload, because its wait could have returned
the preexisting cube; the final check clears before loading and requires
the app's `Loaded` message.

This proves the narrow dense-cube browser history and `.obrush` round trip.
It does not prove Source cube coordinate/normal/material parity, retained
subdivision levels, native GUI behavior, or responsive sculpting on that
mesh. The C248 primitive acceptance gate remains open.
