# C308 Pinch first reference setup failure

The frozen [C308 protocol](SCULPTGL_SOURCE_PINCH_C308_PREREG.md) ran once on
the pinned reference, Negative off, and stopped **before pointer contact**.
The raw owned-browser receipt is ignored locally at
`.artifacts/sculptgl-source-pinch-c308-ui/source-off-d972be62-e6ce-473a-8be6-3fc589365ed1/result.json`,
SHA-256 `1504aa09aabc5123668e31d759493c17bd8c81e208a812b1482cb5b6fc44ae13`.
No geometry, GPU, history, Negative-on, or OBrush pair was scored.

The `PerfDriver.alignViewport` setup had correctly aligned the sculpt region
to 1024 × 768 CSS pixels at DPR 1. The reference `viewport` packet recorded
`pixels=[0,0,1024,768]`, `canvas=[1024,808]`,
`screen=[0,40,1024,848]`, and window `[1334,848]`; the additional 40 canvas
pixels belong to the reference top strip. The C308 harness then erroneously
asserted `logical_size=[1024,768]`, while the actual logical canvas was
`[1024,808]`. The camera had already stabilized for 12 frames over 399 ms,
and import and visible settings had completed. This is an audit setup error,
not a Pinch deformation outcome.

The isolated OBrush browser build completed through `bash scripts/build-web.sh`
with pinned wasm-bindgen CLI 0.2.129 and a shared target directory; no product
code was changed. Its portable WASM/JS SHA-256 values were
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c` /
`0c97e03a7ac8e183d852368fe62651eae0b550f6e65c823e1841bd6e21eedefb`;
SIMD WASM/JS values were
`bad8421313c20b74955ae7d785fcbce24666497a55051d60a4eb8c4844c49754` /
`dcf7bbedf285a086f15cb89eb9dba5c9943d95fe73aa3b04af6bb81124e09e8a`.
These hashes identify the built bytes only; no OBrush browser case ran.
