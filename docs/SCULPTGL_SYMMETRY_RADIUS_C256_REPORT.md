# C256 matched-camera Source Draw radius-chain result

The preregistered normal-browser C256 diagnostic passed its **scoped**
radius-chain checks. It explains the exactly fourfold footprint in C255: the
owned Chrome `Input.dispatchMouseEvent` generated trusted mouse
`PointerEvent.pressure = 0` on both pages. SculptGL copies that raw value into
`Tablet.pressure`, producing `1 + 0.75 × (2 × 0 − 1) = 0.25`. OBrush's Source
host intentionally normalizes a mouse contact to `0.5`, producing factor
`1 + 0.75 × (2 × 0.5 − 1) = 1`. Both nominal Draw radii were `250`, SculptGL's
pixel ratio was `1`, and the accepted physical radii were **62.5 versus 250
CSS pixels**. This is an input-policy difference exposed by the synthetic
browser event; it does not justify quartering OBrush's brush radius or assert
how an actual mouse or pen behaves in either application.

The independent affine two-mesh SGL, normalized SymOffset `+0.5`, active
triangle, Draw intensity `0.5`, alpha None and held three-point path remained
fixed. The default SculptGL camera projected the local path to
`51.86778783694649` CSS px. OBrush's visible Source camera zoom was typed as
`6.49223465795`, producing `51.86778551089003` CSS px: relative span error
`4.4845877567e-8`. Each projected path exceeded SculptGL's `37.5` CSS px
spacing threshold. No hidden camera or document mutation was used.

| Original-call measurement | SculptGL | OBrush |
| --- | ---: | ---: |
| Trusted browser mouse pressure | `0` | `0` |
| Captured brush pressure | `0` | `0.5` |
| Nominal screen radius | `250` | `250` |
| Pressure factor | `0.25` | `1` |
| Project/unproject X offset (CSS px) | `62.5` | `250` |
| Local radius² | `0.12211257183866298` | `1.9538020398131277` |
| Local radius | `0.3494460928936865` | `1.3977846900768114` |

SculptGL's original `Picking.computeWorldRadius2()` call was wrapped only to
record its inputs and return value; the wrapper delegated exactly once. Its
projected hit was `(910.99999163265, 835.00000917720,
0.4002131280191151)`, and its actual unproject X argument was
`973.49999163265`. Its world squared distance and active mesh scale squared
`1` reproduced the picker/mirror local radius². OBrush's frozen Source input
view/viewport and saved pose independently reproduced its completed pass
radius² to the reported precision; primary and mirror radii agreed.

The three-vertex ordinary `.obrush` saves establish a geometry difference as
well: the affected vertex moved in Z by `+0.019469663500785828` in SculptGL
and `+0.13731301` in OBrush, about `7.05×`. Same-threshold changed RGB pixels
were `34` and `3296` (`96.94×`) in the preregistered crops. Those counts are
still confounded by brush pressure policy, renderer, color, silhouette and
crop location. They are not a visual-quality score, and the complete
symmetry/brush rows remain open. The next paired gate should control a
**matched raw and captured pressure** through normal host input, then compare
exact vertex deltas and fixed-state images before changing geometry code.

The final browser receipt is
`.artifacts/sculptgl-symmetry-radius-c256/4bdb59f3-345c-4de7-b032-97b2e1fe936f/result.json`,
SHA-256 `0f1c79734c9be91ace859d545c4aa39f8c1ac96e9b9ef170437943e4a1904f36`.
Its harness SHA-256 is
`af33a436b9b17f7cbcb2ca11a9f9218b3c2372348876f120f5ea45862c3fd960`,
official OBrush WASM SHA-256 is
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`,
and pinned SculptGL bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The fixture SHA-256 is
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`.
The receipt records the root checkout commit
`a7b21f623bcc647c06d5fb55004faf594f03aea7`; the product WASM is the
previously verified C255 build, with no intervening product-code edit. The
ordinary before/after `.obrush` files and screenshots are retained beside
the receipt.

Failed C256 attempts remain in separate ignored artifact directories. They
failed during harness setup, before a paired radius result: `08dd4dbe…`
(camera control in wrong panel, receipt SHA `70e8467c…`), `5a323f85…`
(same, `21cca801…`), `e960dbb2…` (camera rectangle remained off-screen,
`8f3245df…`), `e78c2ca3…` (Save shortcut did not download, `315fef2f…`),
`11eab9d7…` (aborted after an unbounded 5.3 MB DevTools polling loop; a
manual incomplete receipt preserves its screenshots, `a4e97c66…`) and
`a24aab87…` (one full 5.3 MB DevTools transfer timed out, `14478c26…`). The
final harness scrolls the visible right inspector after moving the pointer,
clicks the visible Save button and reads normal downloads from disk. Its
syntax and independent matrix self-check passed, including replay of the
frozen C255 Source capture. No Cargo or product code was changed.
