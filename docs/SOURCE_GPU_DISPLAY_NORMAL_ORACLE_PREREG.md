# Source GPU display-normal oracle: preregistration

This check is independent of the renderer and does not alter product geometry or
the frozen SP-WORKER-01 gate. The frozen gate compares active GPU normal f32
words to public Save `mesh.normals`; it remains failed, including on the
ordinary baseline. The new checks explain and test the display-normal policy,
but cannot retroactively pass that gate.

## Static import check

Before reading the existing 100k ordinary GPU hashes, predict that an imported
SGL with Source metadata and no retained paint state uploads a static
original-face field. Recompute it independently from the public Save's f32
position words and original `surface.faces` in Python: each face cross product
uses f64 arithmetic followed by an f32 cache boundary; visit faces in
descending row order and average cached face normals at each incident vertex
in f64, then round each vertex component to f32. If no original surface is
present, use triangle rows. Preserve duplicate corners and face order. Hash
packed little-endian f32 vertex normals. Compare it to the active VBO's normal
hash at **before**, and separately at **undo** only if the browser state is
known to have a fresh static producer. Do not treat `mesh.normals` as the
display oracle. Fail on invalid topology, nonfinite values, or hash mismatch.

The first existing fixture is the 99,602-vertex independent SGL documented in
`SOURCE_WORKER_SPW01_100K_DIAGNOSTIC.md`; use its public Save and separately
captured `gpu-browser.json` receipt. This is a read-only post-hoc diagnostic,
not a new browser parity run. Record all hash results, including failures.

## Retained and history checks

`SourceGeometryNormalState` refreshes incident faces but only selected vertex
normal rows. It keeps unrelated historical rows. Undo/Redo refresh captured
selected rows from the **current** retained field; two identical public Save
files may therefore have different display normals. A Save-only reconstruction
cannot be an exact oracle after painting. The five-stage active-GPU oracle for
a Worker candidate must be a separately captured ordinary-route VBO under an
exact matched input schedule and public mesh identity at each stage. Compare
both routes' normal hashes only after unique active VBOs, identical public
Save bytes and identical delivered dab inputs have been independently proved.
Otherwise report `unqualified`, never `pass`. A paired ordinary route is a
reference for OBrush route equivalence, not SculptGL or ZBrush parity.

For a new owned-browser run, record source commit and served WASM/JS hashes,
fixture and protocol hashes, normal/public Save hashes at all five stages,
delivered input and dab traces, active VBO uniqueness, GL errors, and raw
failure receipts. Only after the root agent grants the exclusive browser lane.

## Read-only 100k result from the existing ordinary receipt

The independently implemented static face oracle was run after the above
prediction against the already retained 100k ordinary browser receipt. It
recomputed 199,200 original triangle-face rows and 99,602 vertex rows from
the public Save, with no call into OBrush code. The browser receipt SHA-256 is
`ad6b4cc7c62ad8f5e60c41664d4feb50a4d58a30dc9517a1e9ed56246e439df8`.
Each stage had one active VBO and GL error zero.

| Stage | Fresh static oracle vs active GPU normal words | Public Save normal words vs active GPU normal words |
| --- | --- | --- |
| Before | exact `02a0c38496e4147fc63cce3da0967a2c08975598ed1f25f38f83b6a6fc9f7461` | unequal `69e1025160b4e534c5955f9eafe7636b6ec4e767620ab1e40f66591ee338f724` |
| After | unequal; static `fa2df7bbff7527c217e218b3eda695c1bcf2a48a98a31710b02779de2b7beb96`, GPU `100311cd3e382c48c89e2631628068f0879503ae99b2b1e85a39aa64671994ef` | unequal |
| Undo | exact, same static/GPU hash as Before | unequal, same public hash as Before |
| Redo | unequal; static same as After, GPU `b5af30b5880c1c1e498207ddcd772214000152fe246a4fc389f5ada4b5557cd9` | unequal |

The raw result files are retained under ignored
`.artifacts/gpu-display-normal-oracle/100k-ordinary-{before,after,undo,redo}.json`.
Their SHA-256 values respectively are `dd1c395bb733be57b7612a9a3e91a11696775fd03b5f53b52ecab0980a6063ff`,
`eba0da6f18703bd0570a542329c08c9f244c326839232b6d43e936d8d3663e94`,
`75b723cc08494877263b82816e46ce0461fef601af3dd45745b3ad6fd0c629ea`,
and `9b402bfb6ac51f24f8c0c562f5e6132ad6fa3cb8d8db4e812d941994901d8823`.
The After/Redo static checks exited 1 and remain failed receipts.

After and Redo have byte-identical public Saves, but different active GPU
normal hashes. This is consistent with the selected-row history policy in
`SourceGeometryNormalState::refreshed_for_history`, but hashes alone cannot
locate affected rows or prove the exact state transition in this run. The
observer now optionally captures raw normal words at up to 100,000 vertices,
and `scripts/audit-source-display-normal-rows.py` independently compares
per-row f32 bits, preserving the ordinary route as a separate reference.
The existing five-stage receipt captured hashes only; row-level claims await
the next owned browser run.
