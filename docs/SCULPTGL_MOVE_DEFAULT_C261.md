# C261: Source Move Topological Check default

The pinned SculptGL `Move.js` constructor sets `_topoCheck = true` and
`_negative = false`. Its normal Move menu exposes a Topological Check checkbox;
the first setting selects `pickVerticesInSphereTopological`, while clearing it
selects the unconnected geometric sphere. OBrush already had the corresponding
shared-core connected-patch behavior and an ordinary Move checkbox, but its
fresh Source-profile Move preference inherited the OBrush default `false`.
This change applies the pinned default once on Source-profile startup or first
entry through the profile menu. It leaves a fresh OBrush-profile session's
default `false` and subsequent user toggles intact. Source Move Along Normal
remains initially off in both applications.

Pinned reference: SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Isolated OBrush browser WASM SHA-256
`274bc0ed3a3633479863791d97aa395d90703083d67dfd8f7762ce220b742c6c`.
The independently authored isolated Chrome/CDP check script is
`.artifacts/sculptgl-move-default-c261/check.mjs` (SHA-256
`4a3504c61d2ba3714e6e2ee360ac0a1688da180ff6ae198de482a8bede687157`).
It uses each application's normal visible Move selector and checkbox. The
reference browser published tool 9, checked/owner `true`, normal `false`; its
normal click published checked/owner `false`. OBrush with
`?obrushprofile=sculptgl` published Move, topology `true`, normal `false`;
its visible checkbox published topology `false`. The passing receipts are
`.artifacts/sculptgl-move-default-c261/f97ffddd-1304-4f01-b729-936693346cca/result.json`
(SHA-256 `aedec301cadd86e3b578df5d6bb36920d2e0d2efaa2ec55af62b195812ecac38`)
and `.artifacts/sculptgl-move-default-c261/107cb981-15aa-4148-af69-0cf050c6fcb2/result.json`
(SHA-256 `5778ed4300acc30c8ccb17f95c515d7a8402da652ccd4a61d172b6ef9caa2f1e`).
Earlier failed harness checks are retained beside them; they exposed missing
diagnostic arrays, a two-server closure assumption, an exact-case label
assumption and a click during sidebar scrolling, rather than a product defect.

An independent shared-core two-sheet fixture has `21 × 21` vertices per
disconnected sheet, spacing `0.0625`, rear offset `z = 0.03125`, Move radius
`0.5`, strength `0.35`, X displacement `0.02`, no symmetry, no adaptive detail,
no backface culling, and Topological Check on. Its front seed moves while the
rear sheet's point `(3,2)` stays bit-exact. Public `.obrush` JSON reopens and
re-saves byte-exactly; one Undo restores the prior JSON and one Redo restores
the changed JSON. The existing companion test checks that disabling the option
can move both sheets. `cargo fmt --all`, the focused core test and
`scripts/build-web.sh` pass; the combined workspace/Clippy/native checks are
performed by the integration lane.

This proves a default-control correction and an independent OBrush scope and
history fixture. It does not prove matched SculptGL Move displacement,
surface refinement, pressure, mirrored selection, edited multires levels or
long-stroke response. OBrush brush preferences are currently initialized afresh
per process; `.obrush` saves the document, not the current tool choice.
