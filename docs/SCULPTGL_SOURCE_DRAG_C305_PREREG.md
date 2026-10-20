# C305 Source Drag viewport-alignment setup correction

Frozen before any C305 browser execution. Preserve C304's failed on-OBRush
receipt and completed other captures unchanged. C304 stopped before import or
stroke because its ordinary browser setup reached 1024×768, then adding 40
outer-window pixels yielded a stable 1025×808 logical viewport. C305 is a
new, setup-only protocol. Product code, fixture, pointer path, settings,
reference, and all numerical/shape/GPU/history/matrix and C295 regression
gates remain identical to C304/C300. C304's candidate code and portable
WASM/JS SHA-256 `7bda17d3c4f92ce0f3d5a29261f60d57de459daf0dcc2447b1917aa47c618bce`
and `bb07c4da377b12963fb12c77444d95a37df00560aa33291e5ab1d10ee0aae830`
are the intended C305 baseline; verify actual served bytes again.

The only capture behavior change from C304 is in OBrush viewport setup:
after the unchanged exact 1024×768 baseline and +40 window-height step,
observe up to eight normal frames. If exact 1024×808 is absent and height is
808 with width at most two pixels away, send at most two ordinary
`Browser.setWindowBounds` outer-width corrections of `1024 - observed_width`
pixels, each followed by eight normal frames and an exact-size check. Record
each outer/logical width. Any other height/width or exhausted correction
fails before fixture import. There is no DOM/control/model mutation, pointer
change, tolerance relaxation or posthoc geometry adjustment.

Use the same pinned SculptGL commit/bundle SHA-256
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` /
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent fixture SHA-256
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`,
and frozen 1e-8 projection, 1e-6 paired XYZ and active-GPU-normal thresholds
in `SCULPTGL_SOURCE_DRAG_C300_PREREG.md`. Run one candidate normal-UI quartet
on owned browser/server processes only after C302 and MRP02 explicitly
release the exclusive lane. Preserve the first failing check and do not
promote if any substantive gate fails.

Frozen SHA-256 (all three passed `node --check`):

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-drag-c305-ui.mjs` | `5e97661f6abc97785a6c3332d7b2f562b29a45178ae8b880a068428bf52b647f` |
| `scripts/audit-sculptgl-source-drag-c305-ui.mjs` | `274ba287caf459043116316331d41a9ecbac3d74c25c6021d01014b7b50a4789` |
| `scripts/audit-sculptgl-source-drag-c305-matrix.mjs` | `cb030417fbb17593931bd86dab6cb1029dc8190467c4ae1b25ad2cbe93b0614f` |
