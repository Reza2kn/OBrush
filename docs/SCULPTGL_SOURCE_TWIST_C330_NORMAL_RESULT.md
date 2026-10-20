# C330 Twist selected-normal result: first held GPU gate passes; full row remains open

The isolated C330 candidate follows preregistration `0a72fde24ce25f3097c08d73cec29356ed72e712`
and product candidate `000c31abdabaf75daf6e9dfde459a64903848bf9`.
It retains the C323 Twist-only radial kernel and all face/fold guard conditions.
For an imported Source Twist, the host now seeds a document-owned selected-normal
state before the ordinary history snapshot, refreshes picked rows after an
accepted guarded move, and renders that state while held and idle. Fully
protected picked vertices are included in normal refresh; stationary neighbors
outside the picked set keep their previous GPU normal words. Public Mesh
normals, Local Scale, and other ordinary brush render routes remain unchanged.

The pinned Source C318 normal-UI result is unchanged (SHA-256
`e5fa09b0ef16bba7017553d3d270f0907cf00d552243fe333ab23456d562843b`).
The independent C306 fixture, pinned Source bundle and frozen C318 harness/
scorer match the hashes in [the C330 plan](SCULPTGL_SOURCE_TWIST_C330_NORMAL_PLAN.md).
The new owned-server OBrush result is
`.artifacts/sculptgl-source-twist-c318-ui/off-root-e3dde0e0-fca9-43a7-a9d7-24d9454f6cb3/result.json`,
SHA-256 `8c85deda6fab55d48b3fe5320059259e876d6d86c783fe7a3078a90d858d6060`.
Portable browser WASM SHA-256 is
`80ee77c5b15d4243a1071b71b17e4478bc8e3b9724ecc090eb501b3518cdbd1a`;
JS SHA-256 is `c6408888f69db706f24d72aa29ba2d0d0a220f46d3ab582588d5f70e8dfdf543`.
The frozen paired audit is
`.artifacts/sculptgl-source-twist-c318-ui/off-source-0e8561b6-6366-4827-966b-3cace02407e1/paired-audit-off-root-e3dde0e0-fca9-43a7-a9d7-24d9454f6cb3.json`,
SHA-256 `55b12db71e8f0adc75a48b0a5eb11879a3fa94cb35bdf68e677573a7ca445217`.

At the first effective held move, the same 37 vertices moved; maximum paired
XYZ component difference is `1.4901161193847656e-8`. The active GPU normal
maximum is `8.731149137020111e-11`, down from C323's
`3.113818820565939e-5`. Both apps change 38 GPU normal rows. The root GPU
normal at stationary neighbor ID611 stays at its Before words, while the
protected picked ID543 updates. Held0 and Undo pair exactly. Both apps pass
their own public one-step Undo/Redo, saved-position GPU identity, and quiet
release checks.

The complete frozen C318 scorer still reports **failure**: held2, After and
Redo maximum XYZ difference is `0.00922725722193718`, and the active GPU
normal maximum is `5.1000737585127354e-5`. The retained guard clips the
reference's folded later target, as documented in C323. The mask-versus-
spatial-falloff ID577/578 criterion remains failed on both sides. Source
repicks each held event, whereas this candidate's normal selection is anchored
to the original local-transform footprint; later-path behavior needs a
separate causal audit. No full Twist or SculptGL parity is claimed.

Validation: `cargo fmt --all --check`, full `cargo test --locked --workspace`,
strict native workspace Clippy, core/app WASM check, strict app WASM Clippy,
and `scripts/build-web.sh` with pinned wasm-bindgen 0.2.129 passed. The final
`obrush-app` lib rerun also passed all 781 tests after the refresh-rejection
cleanup path was tightened. An initial full test run failed only the two new
tests because their OBJ fixture used the
legacy mask convention; the fixture was corrected to Source's unprotected
convention and the focused and full reruns passed. The first strict Clippy run
found one new collapsible-if style error, corrected before its passing rerun.
An initial otherwise complete browser capture failed the frozen harness's
owned-server closure check because an external local server URL was supplied;
its failed result is retained at
`.artifacts/sculptgl-source-twist-c318-ui/off-root-146499fd-4a8d-457f-ad22-df02afc4631a/result.json`,
SHA-256 `fe1a8a2ca9af0876f4218079390ad8ffaf36aad255f50a33e0d86e49abec3d45`.
The unchanged harness then passed with its own server. The first audit attempt
also used the root checkout for the pinned Source receipt, which resides in
this isolated worktree, and the second omitted the shared helper root; those
setup failures were corrected without changing captures, scorer or product.
