# Small-radius Move guard: B01 failure and B02 candidate witness

The user's small-radius Move tearing report remains **open**. This is a
bounded native/browser geometry correction, not a SculptGL or ZBrush parity
claim. The independent fixture is a fresh level-six OBrush sphere; no
commercial asset or user sculpt was copied into the repository.

## Cause and rejected B01

The retained Q02 normal-browser run had a 71.49° adjacent fold after its
first repeated Move contact and 50.26° at the end. The initial B01 candidate
blended a 30° lateral fold limit toward the previous 90° limit once the pull
had only 25% normal alignment. B01 **failed** its frozen `<=30.01°` gate:
its sixth contact grew an existing mirrored edge near
`(±0.113, 0.27, 0.957)` from 3.218° to 33.662°; the maximum reached
36.333° after eight contacts. The edge existed before the contact, so this
was not an unchecked refinement boundary. The failed result is retained at
`.artifacts/move-regression/guard-b01-aeb1e84e-8169-4516-a7d2-f0bcfb01e800/result.json`
(SHA-256 `0b14a8b5e3a386517fa5ad45b42937ef635d5ee4376f12253c2d9e7d09b54fb2`).
The offline locator `scripts/diagnose-move-folds.py` produced
`.artifacts/move-guard-fold-localization.jsonl` (SHA-256
`0f00decb09ca3dac227deb30cce3e843a6951beecb2fe7ce86110555558aa435`).

## B02 behavior and validation

The [B02 preregistration](NATIVE_MOVE_GUARD_B02_PREREG.md) freezes the same
browser gate before rerun. `MoveStroke` now keeps the 30° adjacent-face
limit while tangential motion is at least as large as normal motion. Once
normal motion dominates, the limit blends continuously to the established
90° extrusion latitude. The existing triangle quality floor, rest-space
guard, mirror behavior and history path remain in place.

`cargo fmt --all --check`, full `cargo test --workspace --all-targets`
(113 passing result groups), strict native and WASM Clippy, official
`scripts/build-web.sh`, and `scripts/package-macos.sh` all passed.
The ad-hoc macOS app passed `codesign --verify`. The twelve existing
`move_surface` regressions still pass, including three outward-pull
usefulness tests that a blanket 30° limit had failed. A shared-core
independent analogue at radius `0.037`, strength `0.79`, X symmetry changed
1,016 vertices, added 988 refined vertices, moved an original point by
0.010749 world units, and reached 29.993° maximum adjacent fold.

The frozen B02 ordinary-browser replay used visible Move settings, a fresh
40,962-vertex sphere, twelve mouse contacts and twelve held samples per
contact at 30 ms. The owned Chrome process exited 0. Its 28 recorded
interaction gates passed: nine effective contacts each mapped to one exact
Undo step; Redo restored the saved final mesh; GPU mesh and public Save were
exact; no queued deformation appeared after release. All twelve saved
contact meshes were finite, closed, manifold and consistently wound, with
minimum triangle quality `>=0.09`. The worst adjacent fold across contacts
was 29.999921°; 78 original vertices moved, with maximum displacement
0.016142801 world units. The final public project has 46,226 vertices and
92,448 faces. The same-tab close-up had exact camera distance, target and
view-projection after Undo (all measured error `0`), and 2,940 original
ROI pixels exceeded the frozen RGB-channel difference of 8. The visible
mark is small in this view; this witness does not resolve the user's
large-scale triangular tearing complaint.

The B02 receipt is
`.artifacts/move-regression/guard-b02-05d30536-cc23-4866-a96a-fd2f4a7afff8/result.json`
(SHA-256 `fe96c73b498c7d617645602eadd0912d6ad91d249dc64035496acaafd036fcfc`).
Its exact portable/served WASM SHA-256 was
`5719040ac5ed8e6331942dfd62a7452ee712959807ed659da9b4e0f931bd2141`,
SIMD WASM was `f8998435aed52f410b60f64da38e1c92993366488f0e86665e84d19eaa34e964`,
and browser harness was
`135255a6e91d8bd4a4c95c2458be1fcd00ab6abf1c53f53c1a496e0d7fcfabde`.
Initial/final public project hashes were
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
and `abad829145eacf69ba1034550fa8b80be629eedb504a2c136bf68f64696a7894`.
The packaged macOS binary SHA-256 was
`a11ca58f392dbce5963eb1e5eb0076c51fc2c171808d2838671f10077e92d894`.

The browser fixture is a synthetic headless mouse trace, not physical pen
input or a native GUI gesture. Its modest deformation and one camera angle
cannot prove global self-intersection freedom, visual quality under every
pull, or reference parity. The earlier Q03–Q06 close-up camera failures
remain retained and failed independently of this same-tab B02 witness.
