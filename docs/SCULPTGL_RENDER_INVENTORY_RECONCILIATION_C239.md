# C239: grid and contour inventory evidence links

Three active rendering rows in the [pinned 317-ID crosswalk](SCULPTGL_CURRENT_LEDGER.md)
now point to existing normal-browser and paired-reference reports. This only
reconciles evidence; no rendering code changed and no acceptance gate closed.

| Pinned ID | Existing scoped evidence | Open boundary |
| --- | --- | --- |
| `SGL-RENDER-GRID` | [C122](SCULPTGL_GRID_C122.md) implements the source-style 44-segment grid in the ordinary OBrush viewport and checks its off/on/off pixels and unchanged public project. | No pinned hosted SculptGL pixel pair, camera/DPR matrix or native GUI appearance comparison; the default visibility differs. |
| `SGL-RENDER-CONTOUR` | [C123](SCULPTGL_CONTOUR_C123.md) checks selected-silhouette off/on/off pixels; [C124](SCULPTGL_CONTOUR_PAIR_C124.md) compares the same imported 10,114-vertex mesh in both browsers. | The observed normalized contour footprint is 4.50% wider in OBrush; camera, projection, color and occlusion matching remain open. |
| `SGL-RENDER-CONTOUR-COLOR` | [C123](SCULPTGL_CONTOUR_C123.md) checks visible RGBA picker and alpha-dependent pixels; [C124](SCULPTGL_CONTOUR_PAIR_C124.md) records source mask and compositing order. | Matched color-space/pixel acceptance across colors, backgrounds and transparent scenes remains open. |

The links live in
[`sculptgl-current-overrides.json`](evidence/sculptgl-current-overrides.json)
and feed `python3 scripts/audit-sculptgl-inventory.py`. The prior reports
remain bounded diagnostics, not full rendering parity. The million-vertex
[C233 profile](SCULPTGL_APP_FRAME_PROFILE_C233.md) still fails all four
responsiveness gates.
