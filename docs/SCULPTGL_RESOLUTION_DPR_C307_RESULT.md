# C307 DPR resolution audit: candidate rejected

**No product change accepted. `SGL-RENDER-RESOLUTION` remains open.** The
[preregistered C307 hypothesis](SCULPTGL_RESOLUTION_DPR_C307_PREREG.md) was
that OBrush multiplied an already DPR-scaled browser scene target and therefore
rendered at twice SculptGL's CSS-pixel density at DPR 2. The normal-browser
baseline falsified that premise. Candidate `ad5457c` was deliberately not
built or promoted: it would halve the measured scene target at DPR 2.

## Frozen inputs and build

The pinned SculptGL bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
(mirror wrapper
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`).
The independent two-tetra OBJ was 367 bytes, SHA-256
`fab0a005d7c4cb666d4c17f1b6a5d115a4051abfcd0df254182c591307af2300`.
Both apps imported it through their normal browser paths; the receipts record
three scene objects including the startup object. OBrush product source was
unchanged from `11f01b9b0cf936180fa41aabb14f74522ab69e08`; the baseline
checkout `b7c48334a04df83ac07f5531a810aeca13c51c6c` added only this
preregistration and fixture. The official `bash scripts/build-web.sh` passed
using root-local wasm-bindgen CLI 0.2.129. Portable WASM SHA-256 was
`67051db667ad51adba1fc65f9b28bdb05bd23a1610058285836082c191a7ada6`;
SIMD WASM was
`649e50fc7f3725d45b56e7d435320382293b104ca3c49fadfaad6d37e54ba750`.
The first build invocation failed because this worktree's fallback global
wasm-bindgen CLI was 0.2.104. It was not a code/build failure; the matched
CLI rerun passed.

## Normal-browser observations

Owned Chrome used a 1600×1000 CSS viewport, app UI zoom 1, and CDP device
scale factors 1 and 2. The reference CSS viewport was 1290×960 at either
DPR; its canvas backing at ratios 1/.5/2 was respectively 1290×960,
645×480, and 2580×1920 at both DPR values. Its history stayed at 2 and all
recorded GL errors were zero.

| Browser DPR | OBrush canvas backing | OBrush source viewport in egui points | Actual Source scene pixels at 1× | Derived scene CSS size |
| --- | --- | --- | --- | --- |
| 1 | 1600×1000 | 1140×912 | 1140×912 | 1140×912 |
| 2 | 1600×1000 | 575×412 | 1150×824 | 1150×824 |

At DPR 2, egui's viewport points are half the CSS viewport size, while
`viewport_in_pixels()` produces the CSS-sized scene target. This is why the
candidate's use of unscaled `info.viewport.width()/height()` would have
requested 575×412 pixels instead of the measured 1150×824 at 1×. The OBrush
canvas backing itself remained 1600×1000 despite DPR 2; the CDP screenshot
was 3200×2000. Whole-canvas high-DPI behavior and chrome layout therefore
still differ from the reference. The DPR 2 scene was 10 pixels wider and 88
pixels shorter than DPR 1 after OBrush UI layout; reference CSS scene size
stayed fixed. No CSS/pixel appearance equivalence is claimed.

The DPR 1 baseline receipt
`.artifacts/sculptgl-resolution-dpr-c307/79471c94-92e3-4f62-af8f-c5426395205c/result.json`
(SHA-256 `3e889a94ec4ad40933789e0ebe6e6543ee5f340a64f9635ad92d362f2aed9fb1`)
measured OBrush scene pixels 1140×912, 570×456, and 2280×1824 at ratios
1/.5/2. Screenshots were distinct, no scene/GL errors were reported, history
stayed Undo=1/Redo=0, and all four saved `.obrush` files were byte-identical:
5,318,809 bytes, SHA-256
`c759e9cfed1e06ec2f0aa523d2693cd7bbd2c9e3e04127d6b0c80672572d98aa`.
However, the final visible slider click landed at `0.9800000190734863`, not
exactly 1×. The initial harness reported a scoped pass because it tolerated
0.04; **the preregistered exact return subgate failed**. This receipt is
retained without reinterpreting the threshold.

The first DPR 2 attempt reached reference 1/.5/2 and OBrush 1×, then timed
out saving the OBrush project using the C247 fixed topbar click. Two input
corrections (a lower Save click and Ctrl+S) also timed out at the same point.
Their retained result SHA-256 values are respectively
`7a2bd7e593da97af221855d3e04ecf93071e90ed70df4361ee5910427b173932`,
`f67d8c4c0c3a0570f7ebcb576da6f2702b4a5dd97d7ceb2ff243b11862b28a83`,
and `50079c98e306ff6b5b8077ee43afb8f62c7bd7280ff078919cfb2c5dcf736b81`.
All three independently measured the same DPR 2 1× OBrush scene 1150×824,
reference dimensions above, unchanged Undo=1/Redo=0, and zero captured GL
errors. DPR 2 OBrush .5×/2× allocations, exact saves, return, GPU/save match,
and hover-circle CSS radius were **not verified**. The hover/away captures
had no differing pixels, so the brush-circle gate was not met. No further
product candidate was run, because the premise for `ad5457c` had already
failed.

The browser harness and attempted cursor analyzer are retained as
`scripts/check-sculptgl-resolution-dpr-c307.mjs` and
`scripts/measure-resolution-cursor-c307.py` in the isolated candidate
worktree; runtime corrections are retained in ignored
`.artifacts/c307-harness/` (latest harness SHA-256
`d8ffdeb3116c3683a3a33185f1db64f6aa37fcd123048f49ad53a4e15399ae7d`).
Neither the baseline receipt nor these unrun candidate scripts constitute
full reference parity.
