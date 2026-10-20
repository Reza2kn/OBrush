# V01 single-mesh visual gate: normal-UI setup failed

**No pixel or product verdict. `SGL-RENDER-RESOLUTION` remains Open.** The
[frozen V01 gate](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_PREREG.md) required
normal UI setup of a single visible source mesh with symmetry off before
any stroke. Owned Chrome at DPR 2 reached pinned SculptGL's visible Scene
menu, clicked **Clear scene**, accepted its confirmation, and imported the
independent 18,242-vertex OBJ into the now-empty scene. The next normal UI
step could not switch off the Source Symmetry checkbox. Read-only DOM
inspection showed its measured label at CSS `[1298,532,1404.9,554]`, but
`document.elementFromPoint` at its center returned a separate full-height
Rendering `gui-sidebar` overlay. Trusted mouse-down/release on the apparent
label and checkbox left the controller and sculpt manager symmetry true.
V01 therefore stopped before its first screenshot or stroke, as preregistered.

The first attempt also exposed a harness locator error: the Source Scene
topbar is an `LI` whose text includes its entire submenu, so a literal
`textContent === 'Scene'` query missed it. A corrected, read-only locator
successfully opened the menu and reset the scene in subsequent attempts.
The normal-UI Symmetry obstruction remained. The initial frozen harness was
commit `8fab10cd150877bb42ff5e426e5aa5afbe600344`, SHA-256
`3a82a2f0b37c90169c163977849b92e7bbd3b5d3d26aed265e220006c20e85df`.
The intervening diagnostic edits were not individually committed, so their
per-run script bytes are **not fully recoverable**; this limits these
receipts to setup diagnosis, not numerical parity evidence. The final
diagnostic harness SHA-256 is
`d156e6e55c9bce53b7590ecfed268788af7b58ed64bb213d928b7552bcf089b2`.

All eight command-exit-1 receipts are preserved locally under
`.artifacts/sculptgl-resolution-single-mesh-v01/`, one `result.json` per
directory. None captured a V01 stroke or image:

| Receipt directory | `result.json` SHA-256 | First failure |
| --- | --- | --- |
| `27cbab2e-a5ad-4cf7-8379-1269f4f2977b` | `9eb0e214c92389ffb0ffe5babd5887ef7e4bac98eec821f8d9f7f8d713448823` | Scene text locator |
| `2530247d-09a7-4117-9d1d-5b3559bcd2d8` | `ade17adb1fa3565b7208af5dce39da79dfe66048a30c7b0b578ff63e845c9c27` | Scene text locator with DOM diagnostic |
| `8dd6fdf3-b743-46c1-8181-7e60bb838001` | `25ac63b94d60a742db565b772614f4be8cafdebbf8174c05d974a4d250058344` | Symmetry timeout |
| `b9e93ee8-fb56-484b-bfdb-4538a3c7ab9c` | `8194da5f284b88e157e9187254a30d2aab9a5ae31434f3c43ec820845fc79b44` | Symmetry click |
| `a3a73c6b-0e00-4fd4-975a-ea841c0df777` | `2a681f89216292678a0596bc64d9c85c8e155fe5060a8c63cc71b48666e3dad2` | Checkbox-label click diagnostic |
| `c0c65d03-8ebe-45e2-85ff-fa7ee2b4f089` | `cbde080c486b9c23db3cadd890325dbbd856c88b8de049337fa9c6e9020ad35a` | Symmetry label click |
| `14372baa-4d6f-43f6-a3b5-d798c50c7277` | `b53d16e5559f78b5e0946fbac456660ff0bab7f6743b711475a2b54625a2e2ea` | Mouse press/release diagnostic |
| `2b4379b6-3757-47e1-abaf-2c01ae499475` | `26f670a8f1af28a07d01fcb8ce9f7853269ca6171ffccb05f4c5ae123889259d` | Overlay hit-target diagnostic |

The pinned source and official OBrush build remained those in
[R03](SCULPTGL_RESOLUTION_DENSE_R03_RESULT.md); the failed setup did not
touch OBrush. A distinct [V02 amendment](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_PREREG.md)
permits only the Source GUI controller's Symmetry setting as a non-visible
setup step while keeping V01's scene, stroke, pixel and geometry thresholds.
