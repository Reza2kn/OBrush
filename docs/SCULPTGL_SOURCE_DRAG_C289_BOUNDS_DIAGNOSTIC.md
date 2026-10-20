# C289 read-only Source Drag camera-fit diagnostic

The frozen normal-browser diagnostic `scripts/check-sculptgl-source-drag-c289-bounds.mjs`
(SHA-256 `77dbde73227c46443f4b4f7d289a79d339b2f6e7e1e5422fe83da1be7988303a`)
ran without a sculpt stroke or product edit. The independent fixture SHA-256 was
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`,
the pinned SculptGL bundle was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and the OBrush normal web WASM/JS were respectively
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99` /
`03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.
The OBrush served repository HEAD was `b651b3a33ea167180a61d69746204271762eb01e`.

The successful raw receipts are retained at
`.artifacts/sculptgl-source-drag-c289-bounds/source-ddf5813d-36fc-42e9-9f88-2afea79e652c/result.json`
(SHA-256 `aac1ccaf37c7638d8aba46e07179ad832d6aadbbc4fd93e75f04a42a8ff2e75f`)
and
`.artifacts/sculptgl-source-drag-c289-bounds/root-984b514b-dfa5-4539-93a9-10fb3fbb335c/result.json`
(SHA-256 `d1993ae195d2023c3ecb3661bfec29be580ff5affd8bffa23555a3be2a7e2b78`).
An earlier OBrush attempt used the existing port 8792 server and failed the
driver's actual owned-server closure check **after capturing setup**. Its raw
receipt remains at
`.artifacts/sculptgl-source-drag-c289-bounds/root-2ec17e7d-bd82-491f-8bd4-89131e210994/result.json`
(SHA-256 `2ea4666b3cc76c8d7874e438e4df299ddeac79c7a21a1d000f331287aeaa5a58`).
The corrected OBrush run used an owned server, unchanged fixture, script and
WASM/JS bytes. This is a setup/closure correction, not a relaxed numerical gate.

The two public Before saves have all 1,089 unique RGB IDs and **bitwise equal
positions**: maximum per-component difference zero. Source's direct active
`getLocalBound`, `computeWorldBound`, and visible-scene box are identical both
after import and after the settled fit:
`[-0.30000001192092896,-0.30000001192092896,0.0100556630641222,
0.30000001192092896,0.30000001192092896,0.04096993803977966]`.
Their center Z is `0.02551280055195093`, equal to OBrush's precise target.
Thus differing mesh bounds or geometry do **not** explain C288's camera scale.

After import, Source's camera center Z was exactly `0.02551280055195093` and
its center/offset/translate/quaternion timers were active. After the normal
UI projection/reset and off-mesh double-tap sequence, all those timers were
zero, but its center Z was `0.020371685242238632`, a `0.005141115309712299`
shift toward zero while the boxes remained unchanged. Source's final near
plane was `875.4567153998172`, against OBrush's `875.461840222185`;
the `0.0051248223677475835` near difference is 99.683% of the center shift.
Source/OBRush fitted zooms were `0.8105604317383962` /
`0.8105542845967979`, and X/Y projection words retained the C288 difference.

Pinned `GuiCamera.resetCamera` calls `Camera.resetView`, which starts a
200 ms `centerDelay` toward zero. The subsequent double-tap was aimed off
the mesh; its `resetCameraMeshes` branch calls `setAndFocusOnPivot`, which replaces the translation
timer but does not cancel the still-running **center** timer. Its callback
applies an increment from the original center. A remaining fraction
`0.20151121` of the original `0.02551280055195093` center exactly yields
the observed `0.005141115309712299` drift. This is a strong source-grounded
explanation for the moved center and near plane. The diagnostic did not
capture the double-tap branch or `_near` at the instant `computeFrustumFit`
ran, so it cannot quantify the remaining zoom difference as a single
deterministic fit correction. The C288 2.06e-6 Drag geometry miss remains
failed; the C289 camera/product edit is **no-go** on this evidence. Do not
hard-code a scale adjustment or relax the geometry gate.
