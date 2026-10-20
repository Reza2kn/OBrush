# M02 same-frame Move recovery and Q01 quality result

The native egui host now reconstructs a complete fast Move contact from its
ordered raw input events when press, motion, and release land in one frame.
It starts the ordinary Move hit/pick at the actual press position, passes up
to 64 validated in-viewport points through the existing Move sampler, then
uses the ordinary release/history path. Missing motion, overflow, pointer
loss, competing buttons, and malformed order retain the prior no-edit path.
Normal held-frame input does not enter the new branch. The parser and route
are in `crates/obrush-app/src/move_drag.rs` and `app.rs`.

The frozen M02 egui gate passed: the same-frame eight-step contact at radius
`0.037`, strength `0.79`, X symmetry and backface culling on changed at least
eight original sphere vertices on both X sides; each sampled mesh was valid,
all final faces remained nondegenerate/outward, and one Undo/Redo restored
exact before/after project JSON. The prior compressed no-edit behavior remains
as a legacy control with recovery disabled. Stationary and rejected raw
streams made no geometry/history change. The M01 multi-frame control also
passed unchanged. These are host/egui tests; the retained native CUA attempt
still lacks frame and pick evidence.

The separately preregistered Q01 repeated-pull diagnostic applied twelve
completed pulls at the same reported settings on a fresh sphere. It changed
`26, 350, 658, 272, 286, 286, 286, 286, 286, 286, 286, 286` original
vertices by pull. Minimum normalized triangle quality across the sequence
was `0.21055092` (pull 2), and minimum radial face cosine was `0.58072788`
(pull 12); all were positive, with no degenerate or inverted radial faces.
Every intermediate mesh validated, twelve history steps were made, and
twelve Undos restored exact initial public project JSON. Those metrics do
not establish visual smoothness, absence of every self-intersection, or a
ZBrush/SculptGL match.

Verification on worktree parent `304f11ac9221bfb4970aae84f6f253f23b102869`:

- `cargo fmt --all -- --check` — passed.
- `cargo test -p obrush-app move_drag::tests -- --nocapture` — 8 passed.
- `cargo test --workspace` — passed; log retained at
  `.artifacts/native-move-m02/workspace-test.log`.
- Strict native workspace/all-targets and WASM app/GPU Clippy — passed; logs
  retained under `.artifacts/native-move-m02/`.
- Official `scripts/build-web.sh` — passed. Portable WASM SHA-256
  `f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`;
  SIMD WASM SHA-256
  `b9ee97f4b8fd3ef503dc655a5d924e514807f0192729b55e9700f10c608a9230`.
- Official `scripts/package-macos.sh` — passed, ad-hoc signature verified;
  packaged executable SHA-256
  `8dc4e14d400afbad61da81a796869f4feae4deaa692c0614b0c210a2ee6c2d18`.

The frozen owned-browser smoke used Chrome `155.0.8059.85` and exact served
portable WASM hash above. Visible `7` selected Move. The browser's actual
settings were radius `0.1800000072`, strength `0.3499999940`, X symmetry and
backface culling on. A held gesture changed 640 original vertices; public
project SHA-256 changed from
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
to `6d51d1b91285d913483eaa03226519d5889626a42c66647f9dd3030a2ee40321`.
One ordinary Undo restored the exact before project and Redo restored the
exact after project. All 11 DOM contact events were trusted. A fast CDP
burst produced the same 640-vertex edit and history result, **but its events
spanned 11 RAF indices** (`101`–`111`), so it is not a browser same-frame
witness. The held gesture spanned RAF indices `163`–`200`. No JS error was
recorded. The exact result, project files, screenshots, and traces are local
at `.artifacts/native-move-m02/browser-f46e1d57-21ad-4995-8e66-c2388776dd44/`;
`result.json` SHA-256 is
`0db3969abb3f4bc62fc810066e8e3f4aabcecc118d07d6ab2bf1cbb6598f5393`.
The exact browser harness is `scripts/check-native-move-m02-browser.mjs`
(SHA-256 `8e6fc1bec721c77ef7751dd3c2aebfefb5bd0f17c4543ab67a81e22de366fdf8`).
Its immediate cleanup snapshot records a null Chrome exit code; the harness
returned exit code zero and a subsequent process check found no owned Chrome.

**Open:** a normal native multi-frame Move gesture with saved changed project
and frame/pick evidence; an actual one-frame browser/native consumer contact;
and visual comparison of repeated small-radius pulls against the user's
reported triangular tearing. The browser smoke used default UI radius and
strength, while M02/Q01 numerical tests used the reported values.
