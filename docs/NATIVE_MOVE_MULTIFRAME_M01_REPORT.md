# M01 native Move frame-semantics result

The retained native CUA attempt is **inconclusive**, not a demonstrated Move
failure. Its first apparent drag saved identical before/after public projects
(SHA-256 `2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`,
zero changed positions), but did not record egui frame boundaries, a surface
pick, or delivered button events. The attempt manifest is local at
`/Users/Ajab/AI/OBrush/.artifacts/native-move-cua-attempt.json` (SHA-256
`51a83623f5b5de7f2d94074bea6749ceae3474a8b44383c78ddc296f401a2627`).

The pre-registered M01 regression in `crates/obrush-app/src/move_drag.rs`
uses real egui `RawInput` frames, the production Move branch order,
`MoveDrag`, shared-core `MoveStroke`, and public `Document` history. On a
fresh default sphere at radius `0.037`, strength `0.79`, X symmetry on,
and backface culling on, eight held motion frames produce at least one
accepted deformation and at least eight changed original vertices across
both X sides. Mesh validation passes after each frame; every final triangle
is nondegenerate and outward-facing. The completed drag creates exactly one
Undo step, and Undo/Redo restore exact before/after public project JSON.

The same press, move, and release delivered in **one egui frame** leave the
pointer released at the final position, with no held movement, no changed
geometry, and no history. This is consistent with the application's
`pressed`-first/`else if down || released` Move handling, and gives a concrete
reason a UI automation drag can appear to do nothing. It does not establish
that the earlier CUA drag was actually coalesced; a missed surface pick would
also explain that result.

Verification on worktree parent `be7a572f6ccdd3137301710f3d332b08fcaea032`:

- `cargo fmt --all -- --check` — passed.
- `cargo test -p obrush-app move_drag::tests -- --nocapture` — 4 passed.
- `cargo test --workspace` — passed, including the two M01 tests.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.

No production behavior changed; the new code is native test-only. The
browser, WASM build, and macOS package were not exercised for this test-only
change. **Visual tearing remains open.** A normal native multi-frame gesture
with recorded frame/pick evidence and saved changed project is still needed
to evaluate the rendered result at the reported settings.
