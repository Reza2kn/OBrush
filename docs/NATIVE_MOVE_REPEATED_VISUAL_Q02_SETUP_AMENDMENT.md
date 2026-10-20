# Q02 browser setup amendment after failed first run

The frozen Q02 numerical and image gates in
[Q02 preregistration](NATIVE_MOVE_REPEATED_VISUAL_Q02_PREREG.md) are unchanged.
The first owned-Chrome run stopped **before any sphere contact** while trying
to set the visible sliders. The reusable `check-move.mjs` harness still used
0.1.7 brush-setting Y coordinates. The current UI has more brush tiles:
its actual `settings.radius` hit rectangle was `[15,563,183,581]` and
`settings.strength` was `[15,624,183,642]`. The stale radius Y of 428 hit
the Local Scale tile instead. The app consequently remained at radius 0.180,
strength 35%, with Local Scale selected. This was a harness layout failure,
not a Move result.

The failed receipt is
`.artifacts/move-regression/q02-ead34103-8e98-4a6a-83f2-5e4875b497e2/`:
`result.json` SHA-256
`ccac03d129830ce5196c1b76c107e7454fb7705ffc92bf2d3ea2063b4c70434a`,
`failure.png` SHA-256
`b4dee5aa04e4a116740be4cbdd50d16b403c5abd52437c6bbf3f7b1e956828a2`,
and exact `harness.mjs` SHA-256
`dcc6ad839079c326f358024ffa196d44a0805df9ec5a0ed46f6548a986fd8417`.
It served official WASM SHA-256
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`.

The bounded correction reads the app's painted Move tile and slider hit
rectangles on the ordinary visible UI, then clicks within those rectangles
and requires readback of the original exact Move/radius/strength/X values
before any contact. No geometry, timing, image, or history threshold changed.
