# C305 Source Drag X-symmetry and mask matrix result

**Outcome: the frozen C305 normal-browser off/on matrix passed its scoped
SculptGL comparison.** The visible Source Drag mirror checkbox and shared-core
mirrored stroke candidate remain isolated pending root integration. This
evidence does not close the full `SGL-BRUSH-DRAG`, SculptGL or ZBrush parity
rows. C300/C301/C303 failed symmetry-on setup receipts and C304's one-pixel
viewport setup failure remain unchanged and are reported separately.

The independently authored 1,089-vertex curved-grid fixture SHA-256 was
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.
Pinned SculptGL commit and bundle SHA-256 were
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` /
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The isolated OBrush capture HEAD was
`f8628e6067c26c0971d4ae7b7b16149cbabaedcd`, descendant of integrated
root `0643619afdd2d303b0a37f276648ff54b6eb2305`; served portable
WASM/JS SHA-256 were
`7bda17d3c4f92ce0f3d5a29261f60d57de459daf0dcc2447b1917aa47c618bce` /
`bb07c4da377b12963fb12c77444d95a37df00560aa33291e5ab1d10ee0aae830`.
SIMD WASM/JS SHA-256 were
`7a27d560ac8c4f3f5e3f2365579f459f051a2c12bbe9fec30a5601b8a7cd6875` /
`20c00be0756cf1d54d41abbeaa6cc57496d4beee784833900a99d718516fb390`.
The frozen C305 capture/scorer/matrix script SHA-256s are in the C305 prereg.

Both normal UIs received the same trusted integer four-point path at 96 px
radius with Alpha None, zero pressure factors, one public history step and
stable 1024×808 logical viewport. X/Y projection ratios, four world-ray X
coordinates and world radius matched exactly. The C305 bounded width-nudge
branch was **not needed** in this successful quartet; no nudge was recorded.
No UI click or Save occurred during a held stroke. Public Before, After, Undo
and Redo Saves, three held active-GPU captures, quiet After, active object/VBO
identity, exact own GPU-position-to-Save words, topology and attributes all
passed. Undo restored each host's exact Before bytes, Redo its exact After
bytes; After-to-Redo display-normal changed-ID membership matched. Saved
normals were not used as the display-normal oracle.

| Symmetry | After max paired XYZ | After max paired active GPU normal | Exact changed-ID membership |
| --- | ---: | ---: | --- |
| Off | `4.842877388e-8` | `3.492459655e-10` | 95 IDs both hosts |
| On | `4.470348358e-8` | `3.492459655e-10` | 136 IDs both hosts |

All stages, including held previews and Redo, remained below frozen `1e-6`
XYZ/display-normal limits; the greatest paired Redo normal gap was
`4.656612873e-10`. Source and OBrush both moved ID548 by
`0.05558568798005581` with symmetry on versus zero off, exceeding the
preregistered `1e-6` matrix gain. Both moved ID541 and half-masked ID545;
fully protected ID544 stayed exact. Each paired and matrix audit returned
`failures: []`.

The independent **unchanged C295** single-gesture normal-browser regression
was rerun against these same candidate WASM/JS bytes. It passed public
Save/Undo/Redo, active GPU and geometry gates with After XYZ max
`4.796311259e-8` and active GPU display-normal max `2.910383046e-10`,
`failures: []`. The first local C305 scorer invocation omitted the required
`OBRUSH_SHARED_ROOT` path and failed to import a read-only decoder before
scoring. Reinvocation with that environment path made no fixture, product,
input or tolerance change; its successful JSON receipts are retained below.

Raw ignored receipts in this isolated worktree (copy with verified hashes
before archiving):

| Receipt under `.artifacts/` | SHA-256 |
| --- | --- |
| `sculptgl-source-drag-c305-ui/off-source-db38e60a-121b-4ff9-bcae-957c83c3aab1/result.json` | `ec92e771c588e0291e0a89d816977b92978ffcd8153240c96a6d183a96d45360` |
| `sculptgl-source-drag-c305-ui/off-root-54121ccb-32c7-4e4a-ba83-398a7d40fa5d/result.json` | `da6e44d2bf9afb463c320c8ff7075f8136351ef264b2f49f0a109b64bb53274d` |
| `sculptgl-source-drag-c305-ui/on-source-e9bab9b7-38f8-4715-8c4a-de308cd10f4d/result.json` | `ca0d7018fe329daa5219bd794464956dfb0a69a17a7123505837ebefc0de5336` |
| `sculptgl-source-drag-c305-ui/on-root-d7ca5f3a-f38d-4098-a601-f4a848ae1f37/result.json` | `917bdb250ceb8f87773e22d5414f568e737cd7b376a99e9d40799c4c27640278` |
| `sculptgl-source-drag-c305-ui/off-source-db38e60a-121b-4ff9-bcae-957c83c3aab1/paired-audit-off-root-54121ccb-32c7-4e4a-ba83-398a7d40fa5d.json` | `1141dba99786fbffa283285d9629ba273637649702df065f5b0cba894ffc983c` |
| `sculptgl-source-drag-c305-ui/on-source-e9bab9b7-38f8-4715-8c4a-de308cd10f4d/paired-audit-on-root-d7ca5f3a-f38d-4098-a601-f4a848ae1f37.json` | `ca8dc7b6e25e716635e7b7a1c41610fb7fa788514fa721ab6e4ecd962dfd633f` |
| `sculptgl-source-drag-c305-ui/off-source-db38e60a-121b-4ff9-bcae-957c83c3aab1/matrix-audit-on-source-e9bab9b7-38f8-4715-8c4a-de308cd10f4d.json` | `9f9d3a3b80755abe53945322b623e391f4eb2bf4f5bf08d02bf87f9b388fff39` |
| `sculptgl-source-drag-c295-ui/source-51e64e1e-8330-4f92-8872-3928bcff2e29/result.json` | `9d545e0f3d7c6d2d87078a01c787111306508aa5833ad4a711360fb792ab2ae4` |
| `sculptgl-source-drag-c295-ui/root-2a47b1b1-d739-4505-b98b-f85a8122dfdd/result.json` | `3ea08b4a8783ca27a491a17713477b66050fb16b39e03fcb8eb17347ad1f179c` |
| `sculptgl-source-drag-c295-ui/source-51e64e1e-8330-4f92-8872-3928bcff2e29/paired-audit-root-2a47b1b1-d739-4505-b98b-f85a8122dfdd.json` | `2e19663fd051d30082b9dc03b8944a521122359167b6e4a5d4dce7eceabf6d76` |

The candidate passed `cargo fmt --all -- --check`, the full workspace test
suite, strict native and WASM Clippy, WASM core check, `scripts/build-web.sh`
with lockfile-matched wasm-bindgen 0.2.129, and `scripts/package-macos.sh`
with code-sign verification. Native/browser geometry stays in shared
`obrush-core`. A masked invalid second pass and noncommittable release are
covered by focused tests; the browser pair did not inject a malformed input
after a held preview. Alpha stamps, pressure, alternative cameras, dynamic
topology, symmetry offset and long-session performance remain unpaired.
