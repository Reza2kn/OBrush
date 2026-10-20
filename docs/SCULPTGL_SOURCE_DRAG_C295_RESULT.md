# C295 settled-camera Source Drag candidate result

**Outcome: the separately preregistered, normal-browser C295 pair passed all
scored stroke gates for its one 1,089-vertex curved-grid fixture.** This is a
scoped Source Drag candidate result, not complete `SGL-BRUSH-DRAG` or ZBrush
parity. C286/C288/C289/C290 failures and C293's candidate setup failure stay
preserved; the C293 read-only camera preflight passed separately. No code is
promoted by this report.

The frozen C295 harness SHA-256 was
`74eb82886a1d607aee8db935f8b0aace4b690563761b62af257005ed0e7e8ea9`;
independent scorer SHA-256 was
`0f785a5bf693222b15e5f9372c18ff7009ca6b5e37dc7b8bc935c21642f70b75`.
Both Source and OBrush captures recorded worktree HEAD
`7a5dad4a5e029294cb8e7452a804313905b41b04`, an evidence-only descendant
of candidate source commit `86bf24790f220806a36516b42a30728143cdbeb5`.
The served OBrush portable WASM SHA-256 was
`7fca5f5dff2aba91325624b14109a9162795ac1e4a818c30c86cad64a48fb5d2`,
JS SHA-256
`5e32635762339f2ee2cfb0408787af8b041d9ecd5e10263fb11136809811ca06`;
SIMD WASM/JS SHA-256s were
`299c8221fc214dc6190957347b2bacec0ebc3d0158ade0bf247b66700edcfd3e` /
`25734abf2e8d2c3c0b30afc405b8be58aba6f4089ca20b51a0c3e2fbdd86f90c`.
The pinned SculptGL commit was `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the independent fixture SHA-256 was
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.

Both normal UIs completed public Before/After/Undo/Redo Saves and held GPU
captures. Settled Source Reset/fit windows were 397/400 ms; OBrush were
604/602 ms, each with 12 terminal, stable frames. Paired X/Y projection
ratios were exactly 1; four requested world rays and the 96 px world radius
had zero difference. The source and root received the same trusted integer
pointer path, and both changed exactly the same 95 RGB IDs. IDs541/548 moved;
protected ID544 retained its position. Each side had one Undo step, exact own
Before bytes after Undo and own After bytes after Redo; quiet After matched
released GPU words. Every public stage's active GPU position buffer matched
its own Save exactly, and active object/VBO, attributes and topology passed.

The maximum paired XYZ / active display-normal component differences were:

| Stage | XYZ | Display normal |
| --- | ---: | ---: |
| Before | 0 | 0 |
| Held 0 | `7.450580597e-9` | `8.731149137e-11` |
| Held 1 | `2.421438694e-8` | `2.328306437e-10` |
| Held 2 / After | `4.796311259e-8` | `2.910383046e-10` |
| Undo | 0 | 0 |
| Redo | `4.796311259e-8` | `4.074536264e-10` |

All are below the unchanged `1e-6` paired limits. The After-to-Redo active
display-normal changed-RGB-ID set matched exactly: 31 IDs on each side. This
is the previously missing selected-union history behavior; public saved
normals were not used as the display oracle.

Raw ignored receipts remain at:

- `.artifacts/sculptgl-source-drag-c295-ui/source-4d72b051-98bc-4f26-8d02-31685d3f8755/result.json` (SHA-256 `7c39265a404e24a285699d687f0a97894a720501ccdfe07feb1ccac59c1a5221`)
- `.artifacts/sculptgl-source-drag-c295-ui/root-84f26dc3-bb1a-4984-84c6-9454b2022c1a/result.json` (SHA-256 `0a5c6b0199bca992bcf2d891d7b66e8dcde5ed1881f86b270c80bee449808d2e`)
- `.artifacts/sculptgl-source-drag-c295-ui/source-4d72b051-98bc-4f26-8d02-31685d3f8755/paired-audit-root-84f26dc3-bb1a-4984-84c6-9454b2022c1a.json` (SHA-256 `c89f88d1792f0cc1ca94ae96a53541a0e1a8bcdcd9d04a1f79fa568ae25d2cca`)

The scorer returned `failures: []`; both owned Chrome/server processes exited
with no cleanup errors and root port 8792 was untouched. Candidate validation
passed `cargo fmt --all -- --check`, focused Source Drag 5/5, full
`cargo test --workspace`, strict `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo check -p obrush-core --target wasm32-unknown-unknown`, and
`bash scripts/build-web.sh` with lockfile-matched `wasm-bindgen` 0.2.129.
Core tests cover invalid ray rejection before geometry, zero-travel no-history,
and explicit exact cancellation after an invalid post-preview direction with
a noncommittable `end`. The C295 normal-browser pair did **not** inject a
malformed input after a held preview; host `finish` cancellation for that
case is code-reviewed but lacks a paired UI receipt. That remains a boundary
for any broader parity claim.

The candidate requires the opt-in Source Drag code from isolated commit
`f37f2b0` followed by the selected-union history change at `86bf247`.
The C287 camera transition is already in root as `fa67d33`; this isolated
worktree replayed it as `5339bcc`. The legacy Drag default for older projects
is preserved. Alpha, symmetry, other cameras/brush settings, dynamic topology,
and long-session performance are outside this paired result.
