# SculptGL reference runtime observations

This is a **partial reference characterization**, not an OBrush parity result. The frozen [317-workflow baseline](SCULPTGL_PARITY.md) remains unchanged. The [runtime evidence](evidence/sculptgl-reference-runtime.json) appends real deployed-bundle observations and five previously omitted dependency controls. A final mixed-scene preparation timed out, and three exact redo-normal gates remain failed.

On 2026-10-13, `scripts/check-sculptgl-reference.mjs` ran isolated CLI headless Chrome/CDP against a localhost mirror of [the deployed SculptGL application](https://stephaneginier.com/sculptgl/). It owns its server, browser process, profile and tab. It neither attaches to the user's browser nor uses native GUI, accessibility or VoiceOver. All application changes use normal DOM controls, trusted browser input, the actual file input and actual downloads. JavaScript reads diagnostics and complete arrays; it does not assign tool, camera or mesh state.

## Reference provenance and differences

The kernel bundle remains byte-identical to the freshly retrieved hosted `sculptgl.js`: 471,154 bytes, SHA256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`. The original HTML is 1,777 bytes, SHA256 `ef620c4f9903760a1235717e93b17f96dfce4c093426c8bf2dfeebb16ffc0cb9`. Pinned upstream source is `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; **source equivalence of the deployed bundle is not established**.

The mirror wrapper is 1,836 bytes, SHA256 `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`. Its transparent constructor Proxy forwards `Reflect.construct` and retains the returned normal application instance for read-only snapshots. The wrapper removes Google telemetry, serves relative assets locally and blocks nonlocal requests. Google Fonts was blocked, so font fallback/layout differs. Event/WebGL diagnostic wrappers forward the original calls. These differences preclude an unqualified claim about the live HTTPS deployment or its performance.

The asset ledger preserves 22 successful runtime asset retrievals and a **404 for the hosted `sculptgl.js.LICENSE.txt` sidecar**. Pinned upstream MIT/source/dependency licenses remain preserved in ignored tooling. Asset identity does not establish additional image licensing. The failed sidecar fetch is not hidden or classified as a failed application feature.

The principal run used Chrome **155.0.8059.40**, protocol 1.3, a 1600×1000 viewport at DPR 1. Its result is `.artifacts/sculptgl-reference-runtime/candidate-5-7cfa5239-a082-4c21-98e2-0edef402a8e7/result.json`. Complete file/hash receipts, runtime DOM inventories, input traces, screenshots, kernel/wrapper hashes and retained candidate failures are in the evidence JSON. The owned Chrome process exited with code 0 and its localhost server closed; an immediate PID-specific check confirmed the process was gone.

## Completed observations and failed gates

Five independent analytic-sphere trials exercised **Brush, Pinch, Twist, Local scale and Drag**. Each fixture has 266 vertices, 528 triangles, 1,584 seam-bearing UV pairs, nonuniform RGB/roughness/metalness and seven fully protected points represented by material.z = 0. Each trial used a 110-pixel radius, 35% intensity where available, symmetry disabled and a recorded eight-point gesture; Brush's Clay toggle was disabled. Tool-specific remaining defaults, camera matrices, exact screen coordinates and input spacing are retained in the machine record.

Each trial made a useful finite geometry change, preserved protected positions and preserved RGB/material/face/corner-UV data exactly. Actual Undo restored all captured mesh channels exactly. Actual Redo restored positions and attribute channels exactly, but normals differ in three trials:

| Trial | Different normal components / 798 | Maximum absolute normal difference | RMS normal difference | Exact full-channel redo gate |
| --- | ---: | ---: | ---: | --- |
| Brush | 6 | 0.0013011908158659935 | 0.00007085720918035414 | Failed |
| Pinch | 6 | 0.00016185268759727478 | 0.000007935930159336005 | Failed |
| Drag | 6 | 0.02148471027612686 | 0.0007854184330643602 | Failed |
| Twist | 0 | 0 | 0 | Passed for this sample |
| Local scale | 0 | 0 | 0 | Passed for this sample |

These are derived-channel failures in the reference, not evidence of an OBrush advantage. The zero-error gate was retained, and subsequent reference characterization continued without modifying the kernel or tolerance. Negative strokes, other strengths/radii, culling, alphas, pressure, thin surfaces, repeated long pulls and full response-quality comparisons remain untested.

Normal controls selected **Perspective and Orthographic**, **Orbit, Spherical and Plane**, then set **FOV 60°**. The exact camera modes/matrices changed while every captured mesh channel stayed exact. This proves those selections for this sample; camera motion, picked pivot, axis views, clipping, resize and HiDPI remain separate gates.

Actual Add commands created and selected four primitives:

| Primitive | Vertices | Original faces | Render triangles | UVs | Selected multires level / count |
| --- | ---: | ---: | ---: | --- | --- |
| Sphere | 98,306 | 98,304 | 196,608 | Present | 3 / 4 |
| Cube | 98,306 | 98,304 | 196,608 | Present | 3 / 4 |
| Cylinder | 66,562 | 67,584 | 133,120 | Absent | 2 / 3 |
| Torus | 65,536 | 65,536 | 131,072 | Absent | 2 / 3 |

These are actual creation counts/state, not a shape comparison or capacity claim. Every primitive has a nonidentity uniform scale matrix; exact values are retained. Independent ASCII/binary PLY quad cubes imported with 8 vertices, 6 quads and 12 render triangles. Independent ASCII/colored binary STL cubes imported with 8 welded vertices and 12 triangles. Those imports had finite position/normal arrays; the complete decoded RGB/material/face data is preserved for later comparison.

The final attempted sphere + UV quad + Add Cube scene did not yield a mixed transformed export. After the ordinary Add Cube click, a read-only `Runtime.evaluate` timed out. Its cause is unresolved, the whole run remains failed, and end-of-run trusted-input/error assertions were not reached. Earlier successful candidate event diagnostics and the incomplete final run are kept separately.

## Actual format downloads

These files came from the real Save buttons and Chrome download completion events. They are independently authored fixtures, not user or proprietary models.

| Download | Bytes | SHA256 |
| --- | ---: | --- |
| Empty SGL v3 | 36 | `b0dac0273abd5070928a01b167461d9eb35a41b6784f3574ff813ab1744c830c` |
| UV quad cube SGL v3 | 832 | `960ead9ece5da207eeb05104c073254089150bd5a419c031460f01b492c241ec` |
| Rich sphere SGL v3 | 39,304 | `c1710b94cdfc8fd5cf8a4e87213b776d092a24e87ce2efac69a37d63e95d800c` |
| Rich sphere OBJ | 81,235 | `e494fef5a4e81f42219821d4c94bc1420f131fc919a36e87e030d99b904a63fe` |
| Rich sphere binary PLY | 17,980 | `7eae083b726432287baf8dd6f23caacbf5bc2391bcef8c2e2a2c8b33222577ad` |
| Rich sphere binary STL | 26,484 | `e845a8fa88e879c989414c2797019521139cd586dc564b721ba6cf304936d70e` |

Independent byte inspection of the three SGL downloads consumes each file exactly. It matches the source snapshots' positions/RGB/materials/original triangle or quad words/UVs/corner maps/matrices/centers/render flags and camera flags/FOV **exactly**. The empty file is v3, grid 1, mirror 0, contour 0, Perspective 0, Orbit 0, FOV 45, pivot 1, mesh count 0. The rich file preserves the changed FOV 60.

Actual SGL reimport retains the rich sphere's counts and UVs, but its raw ordered arrays differ. A separate post hoc analysis finds an exact bijection using all 266 unique positions: positions, normals, RGB and materials match exactly under that map, and the complete multiset of oriented faces with corner UVs matches exactly. The pinned mesh initialization has [cache optimization and reindexing](https://github.com/stephomi/sculptgl/blob/8e45dafc0f7906e5238dfffc1a2df742bd25f29d/src/mesh/Mesh.js#L2035). The raw-order differences remain recorded; this semantic comparison is not a widened gate or OBrush result.

The actual OBJ contains 266 vertices, 528 faces, 1,584 UV pairs, 266 `#MRGB` and 266 `#MAT` records. Its `#MRGB` mask byte is `ff` throughout; the material.z protection channel remains in `#MAT`. Actual binary PLY contains exact ordered positions/faces and quantized RGB, with **6,864 additional trailing zero bytes** after the declared records. Binary STL contains 528 exact triangle position records and VisCAM color-active attributes. PLY/STL do not carry the fixture's roughness, metalness, unmasked weights or UVs. Full measured RGB/material quantization errors are retained; no interchange result is inferred for transformed scenes, selection export, malformed inputs or OBrush consumers.

## Additive deployed dependency controls

The hosted bundle embeds Yagui's Extra UI menu, omitted from the frozen app-source inventory. Its embedded source starts at UTF-8 byte offset **68,392** of the hashed bundle; exact source snippet/hash and the constructed normal DOM are preserved. Add these five stable IDs without changing the original 317-row baseline:

| ID | Reference control | Status and acceptance gate |
| --- | --- | --- |
| SGL-THEME-OVERALL | Extra UI → Overall color | Hosted dependency registration and runtime DOM established; change palette through normal UI and verify widget/background/text behavior plus document/history preservation. |
| SGL-THEME-WIDGET | Extra UI → Widget color | Registration/DOM established; verify affected widgets and no model/history mutation. |
| SGL-THEME-BACKGROUND | Extra UI → Back color | Registration/DOM established; verify the UI background independently of the scene background. |
| SGL-THEME-TEXT | Extra UI → Text color | Registration/DOM established; verify readable control surfaces and no scene/history mutation. |
| SGL-THEME-BORDER | Extra UI → Border | Registration/DOM established; verify border toggle and preserved scene/history. |

No color/border mutation or matched OBrush custom theme workflow was executed. Other complete deployed dropdown options are retained: 13 sculpt-tool entries including unified Transform, None/Square/Skin alphas, four shaders, nine MatCaps, five environments, three background modes and ten language choices. Their presence in the DOM is not a successful workflow test.

## Retained harness failures and follow-up

Candidate 1's numeric helper appended text to the existing value, causing the ordinary radius control to clamp to 500; its exact 110 assertion failed before sculpting. Later candidates used native browser `selectAll` editing with exact values. Candidates 3–4 received trusted native Arrow/Home events without changing the macOS headless SELECT. A separate isolated diagnostic showed trusted character typeahead selects Pinch through the normal native SELECT route. Candidate 5 used that route; no DOM value, synthetic change event or application setter was injected. The initial receipt validator also retained a `KeyError: bytes` from assuming DOM metadata had a byte-count field; correction validates all declared hashes/counts and computes missing receipt byte sizes without changing any runtime gate.

The current helper additionally logs CDP request hashes/outcomes and permits a declared `--skip-mixed` initial-scope run. No later successful run is implied by that source correction. A further frozen run must complete the diagnostic assertions and isolate the mixed-scene timeout. Broader topology, rendering/material/UV painting, masks, all option combinations, camera gestures, translation/rotation/nonuniform transforms, languages/backgrounds/themes, endpoints, native/physical-device interaction, matched OBrush outputs and long-session performance still require their own evidence.
