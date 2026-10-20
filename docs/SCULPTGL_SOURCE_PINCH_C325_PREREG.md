# C325 Pinch viewport setup-only correction

Frozen before C325 browser use. Preserve the [C308 failed setup
receipt](SCULPTGL_SOURCE_PINCH_C308_SETUP_FAILURE.md) unchanged. C308's first
reference run stopped before contact because the harness compared the full
reference canvas size `[1024,808]` to `[1024,768]` despite its independent
sculpt-pixel packet being exactly `[0,0,1024,768]`.

The C325 capture changes only that assertion to require the actual
`viewport.pixels` packet equal `[0,0,1024,768]`. The C308 pinned reference,
independently authored fixture and SHA-256, four Negative off/on normal-UI
captures, camera settlement, visible controls, pointer path, radius/intensity,
mask witnesses, numerical tolerances, active GPU and public history gates
remain unchanged. The output directory and protocol ID differ so C308 and
C325 receipts cannot be confused. The unchanged C308 paired and four-run
auditors score C325 using their original frozen gates; their filenames and
status strings retain C308 because they describe the underlying gate.

The corrected `scripts/check-sculptgl-source-pinch-c325-ui.mjs` passed
`node --check`, SHA-256
`210ccb82fca8fd6d17c5b90ab838ba77248c3628e5a383cdaf3506e1caa85226`.
The exact diff from C308 changes only its comment, output path, protocol ID,
and the faulty viewport assertion. Reuse the already built isolated OBrush
browser bytes only after verifying the actually served hashes. Run reference
off/on before same-polarity OBrush runs. Any further setup or numerical
failure is a C325 failed receipt, not permission to alter the frozen gate.
