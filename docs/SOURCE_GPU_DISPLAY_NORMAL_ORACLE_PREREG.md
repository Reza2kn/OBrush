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

## New owned 100k ordinary row capture

An exclusive owned Chrome run used commit `402b514`, the frozen independent
99,602-vertex SGL SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`,
and the same 60-second/400-requested-moves-per-second protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
It used Source Ordinary Draw, radius 64 screen pixels, intensity 0.25,
`clay=false`, `accumulate=false`, X symmetry off, Studio clay display. The
served portable WASM and JS hashes were
`6bd8e1b585e4cb92c465dbb62e6d9243b7349b708f370d846aee203bcf498403`
and `d88da99a09a9af1198258a23e2dc16c577f85012275f4ad235699458e13ac79a`.
The result and GPU receipt SHA-256 values are respectively
`78fbf6a00f0d23ff4abb6cbc82cbb870db727265534ed47d2c8fef7f3c5200cf`
and `5aa223258e5e47223608127cb7122b9fe834f231a5ca7031a7d05dede1ba865a`.
The raw receipts live in ignored
`.artifacts/source-worker-sustained-spw01/100k-ordinary-0c2ba0ae-6805-4b64-8923-b99c172b71e5/`.
The owned browser closed with no cleanup errors. This ordinary diagnostic is
not an opt-in Worker qualification or a SculptGL parity result.

The normal renderer state reported **no retained state Before**; After and
Stable had `StaticSelectedRefresh` generation 1147, Undo generation 1148,
and Redo generation 1149. All used `StoredOriginalFaces` in Studio clay.
Each active GPU VBO was unique and had GL error zero. The independent static
face oracle again matched Before and Undo exactly. It failed After, Stable,
and Redo, as retained-state policy predicts. Public Saves and GPU position
words were exact for Before versus Undo and After versus Stable versus Redo.
The raw GPU normal streams were exact for Before/Undo and After/Stable.
**After/Redo differed in 1,147 of 99,602 normal rows** despite byte-identical
public Saves and exact GPU positions. Their first differing row is 34,899;
the full first-16 row f32 bit pairs are preserved in
`display-normal-rows-after-redo.json`, SHA-256
`7c7138c77782429d0af6357c2d41d39d1f2d6ae21319a00b1517cd30564ea81a`.
Of the 1,147 differing-normal rows, 1,140 also changed position between
Before and After; 7 did not. This location and the reported generation change
support a retained selected-row history explanation, but the browser receipt
does not include every selected-vertex ID, so it cannot independently prove
the exact normal-producing history sequence. Neither After nor Redo is
declared canonical on that evidence.

The first attempt to feed the raw-row GPU receipt to the frozen auditor failed
its 1 MiB receipt bound; the raw receipt is retained unchanged. A separately
hashed metadata-only projection (`gpu-browser-hashes-only.json`, SHA-256
`712b1330c8e1df04db61538a1ae41153c76ebce6c0717daa6d8a2ebe3c696910`)
let the unchanged auditor run. It confirmed all five active GPU position hashes
match their public Saves and, by its original saved-normal oracle, all five
normal checks fail. Its nonzero output is retained in `frozen-gpu-audit.json`,
SHA-256 `c046026bd6b611358d4b7d0a73d033493e5e2f48f3e32186bef9ea0f77023c21`.
This does not alter or reclassify the frozen SP-WORKER-01 gate.
