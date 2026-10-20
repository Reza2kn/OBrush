# C327 Pinch viewport-size setup-only correction

Frozen before C327 browser use. Preserve the [C308](SCULPTGL_SOURCE_PINCH_C308_SETUP_FAILURE.md)
and [C325](SCULPTGL_SOURCE_PINCH_C325_SETUP_FAILURE.md) failed setup receipts.
The only C325 execution failure was its OBrush viewport-origin assertion:
the ordinary inset viewport measured `[225,31,1024,768]`, not
`[0,0,1024,768]`. The pixel width and height were already correct.

C327 compares only `viewport.pixels.slice(2)` with `[1024,768]` after the
unchanged `alignViewport` and 12-frame/250 ms camera-settlement sequence.
The source bundle, independent fixture, visible Pinch/Negative controls,
stroke path, radius and intensity, OBrush calibration, protected/half-mask
witnesses, all paired XYZ/GPU/display-normal/history thresholds, and frozen
C308 paired/four-run auditors are unchanged. Only the setup assertion,
receipt directory, protocol label, and comment differ from C325. Run four
fresh owned browser sessions in the same source-off, source-on, OBrush-off,
OBRush-on order; do not reuse the prior Source outputs to hide a new failure.
The first failing setup or numerical gate is a failed C327 receipt.

`scripts/check-sculptgl-source-pinch-c327-ui.mjs` passed `node --check` and
has SHA-256
`aebe34fa4760d01155b289ac1bde0752bc3aa956550faa7729928ab9df93e43c`.
The original C308 auditors and their preregistered hashes still score the
underlying four-run matrix. C327 remains a scoped static-mesh mouse witness;
it cannot close the full Pinch, SculptGL, or ZBrush parity rows.
