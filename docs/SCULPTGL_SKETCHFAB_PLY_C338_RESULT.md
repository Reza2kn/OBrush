# C338 offline Sketchfab-ready PLY, normal browser

The merged OBrush browser build at source commit
`0bbfa4acc9f440b05efdd0afea41a495e20972c9`, portable WASM SHA-256
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`,
passed the scoped [C338 UI gate](SCULPTGL_SKETCHFAB_PLY_C338_PREREG.md).
The first run failed because the automation did not move the inspector scrollbar;
that failed receipt remains at
`.artifacts/sculptgl-sketchfab-ply-c338/858dacf6-2f2f-4bc0-96a5-ce3f9c168722/report.json`.
The corrected script-only rerun passed at
`.artifacts/sculptgl-sketchfab-ply-c338/abf462f7-7efb-4a18-8cb2-aa4ec5bbe196/report.json`.

Through ordinary File controls, the browser added the independent triangle
fixture twice and downloaded a 400-byte binary PLY (SHA-256
`a9f6266e8db00ff10198cd372cb635d1f2b92cfd700a951ec11790a790cecaba`).
The six XYZ records exactly repeated `(1,-3,2)`, `(-4,6,5)`, `(7,-9,-8)`;
RGB records were exact red, green and blue, with faces `[0,1,2]` and
`[3,4,5]` and 26 zero padding bytes. The `.obrush` Save before and after
export had identical SHA-256
`1a060b3ceae1035ab001e1ccad71fef00361c35eff5f1785a9a8ff74d219837f`;
revision, history and selection were unchanged. No browser errors were reported.

This is an offline payload export, not account upload. A normal-browser pinned
SculptGL ZIP/payload comparison, OAuth, API processing, native save-dialog run,
large allocation boundary and current external service behavior remain open.
`SGL-EXTERNAL-SKETCHFAB` is not closed.
