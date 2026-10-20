# C298 project image assets: preregistered contract

Pinned implementation base: `f053e5be449a5d11866bee5e18a169f41e077dd3`.
This is a project-durability check for the existing C291 custom MatCap and C294
UV image controls. It does not establish reference rendering parity.

Independent fixtures already checked in before this implementation:

| Fixture | SHA-256 |
| --- | --- |
| `crates/obrush-app/tests/fixtures/c291_asymmetric_matcap.png` | `33cd9048b3665b9f24ba0739b9e2a2752400b3249a05cd4a31e3b7be3487ad07` |
| `crates/obrush-app/tests/fixtures/c294_uv_quad.obj` | `10ff58d736037223672442a8e4bbeb5697224b9c65cb09235472beddb8845819` |
| `crates/obrush-app/tests/fixtures/c294_uv_quadrants.png` | `384666bb9aa66c8150ca5c11a72fb860eb95e171d042c9ad490bd6194e21e441` |
| `crates/obrush-app/tests/fixtures/c294_uv_quadrants.jpg` | `98956aa6d0717701f23d34c3414c371bafe1e6ed9c45914aec5217f454926b84` |

Acceptance before observing results:

1. Core versioned JSON owns bounded canonical RGBA images, a custom MatCap
   catalog with stable indices, and active UV/MatCap selection. Older v1–v7
   JSON loads with no image assets. A save/reopen of either fixture retains exact
   canonical RGBA bytes, dimensions, names, selected shading and image index.
2. Native and WASM parse the same project with identical asset bytes/selection.
   A normal-browser Save→Open of the authored UV and MatCap images displays the
   same sampled quadrants/MatCap colors as before Save, measured through GPU
   readback at fixed camera/canvas points. Record numerical differences.
3. Invalid version, dimensions, base64, byte count, catalog count, total budget,
   selection index, truncated JSON, or GPU-limit assets reject before replacing
   document or viewport. Capture before/after public project hash and GPU samples.
4. Opening a valid project is one Undo event. Undo restores the complete prior
   project asset state and visible appearance, Redo restores the imported one.
   Importing/resetting an image is independently undoable and remains bounded.
5. Record exact build hashes, browser URL, fixture hashes, native/WASM test
   results, failed checks, and the remaining difference from SculptGL's SGL
   image/file workflow. Browser proof is required for the full C298 claim.

Maximum planned asset storage is one 2048² UV RGBA image (16 MiB) plus four
MatCaps capped at 32 MiB total GPU storage, with checked aggregate core bytes.
Projects continue to respect the existing 512 MiB JSON input/output limit.
