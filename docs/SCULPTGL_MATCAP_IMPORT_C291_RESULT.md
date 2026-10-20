# C291 custom MatCap import — bounded result

The [frozen C291 contract](SCULPTGL_MATCAP_IMPORT_C291_PREREG.md) targets
`SGL-RENDER-IMPORT-MATCAP`. OBrush now accepts bounded PNG/JPEG images from a
visible **Import MatCap image…** action in the ordinary viewport. It appends a
named option, selects and renders it, and lets the user reselect built-ins or
the imported image. Browser and native hosts use the same bounded Rust
decoder/catalog. The UI explicitly says custom MatCaps are session-only;
Source per-object metadata remains restricted to stable built-in IDs.

## Identity and checks

- SculptGL source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  hosted JS SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
  The source-file hashes and authored fixture are pinned in the prereg.
- OBrush product base `6baac8c1a36dac03c111ff008536d5e62081e482`;
  fixture SHA-256
  `33cd9048b3665b9f24ba0739b9e2a2752400b3249a05cd4a31e3b7be3487ad07`.
- `cargo fmt --all`, complete `cargo test --workspace --locked --quiet`, strict
  native/WASM Clippy, `scripts/build-web.sh`, and `scripts/package-macos.sh`
  passed. A later UI-control observation addition passed app library tests,
  both strict Clippy targets, and both artifact builds again. The macOS app
  passed its ad-hoc signature verification.
- Final portable WASM SHA-256
  `901312b6c113fdbb0ad9f5fe793a77f2ac674a2556d3452f63410439899f57bd`;
  SIMD WASM
  `4ab6583bf3974696653be391333e61931fd557b0ca6797afd0165803d2040d69`;
  packaged macOS executable
  `c4be0421f3b1e4171441a3eafe78947cd57c7562de1cf812467f34fcdef123bf`.

An isolated Chrome process at DPR 1 and 1600×1000 visited the pinned local
SculptGL mirror on port 8795 and the own OBrush build on port 8794. Both had a
fresh sphere and the MatCap shader. Trusted UI input opened the visible
import action; CDP supplied the same local fixture to each native file input.
The reference appended `c291_asymmetric_matcap.png` as option 9 and selected
it. OBrush appended and selected id 9; diagnostics show a 64×64 GPU MatCap
resource with no scene error. The screenshot center samples at sphere-relative
offsets `(-0.4r,-0.4r)`, `(+0.4r,-0.4r)`, `(-0.4r,+0.4r)`, `(+0.4r,+0.4r)`
were, in RGBA order:

| App | Upper left | Upper right | Lower left | Lower right |
| --- | --- | --- | --- | --- |
| SculptGL | `78,79,40,255` | `177,79,216,255` | `79,176,216,255` | `177,176,40,255` |
| OBrush | `42,23,5,255` | `108,23,59,255` | `42,66,59,255` | `108,66,4,255` |

The channel ordering and both axes agree. The absolute displayed colors
differ substantially; the different spheres/material pipelines were not
calibrated, so exact appearance parity is **not** established. OBrush's
trusted combo selections went custom 9 → built-in 0 → custom 9. Its four
sample pixels after reselect exactly repeated the first custom screenshot.
The import and reselect kept document revision 1, 40,962 vertices, 81,920
triangles, and Undo/Redo depths 0/0. A malformed-image chooser response
displayed `Texture import failed; previous asset kept` and retained custom
id 9, catalog length 1, the GPU resource, revision 1, and history 0/0.

The local renderer preflights each decoded image against the shared 2048-pixel
side/16 MiB input bounds, reported GPU texture-size limit, and a cumulative
32 MiB custom-texture budget before selection changes. Four custom images at
most may be added in a session. Unit tests check independently authored PNG
orientation, invalid PNG and oversize header/input rejection, stable IDs,
catalog-cap and cumulative-budget atomicity, and bounded JPEG decoding.

## Preserved failures and open gates

The first two browser harness attempts tried to scroll a long egui inspector
with CDP wheel input. The first never moved the control into view; another
scrolled the viewport/camera and hit an unrelated control. Both were discarded
and the normal-browser run restarted on a fresh sphere. A later reselect
attempt clicked an offscreen combo row while its popup list was clipped;
the final run dragged that popup's own scrollbar and selected id 9 correctly.
These harness mistakes were not treated as product passes. Their screenshots
and the final state/screenshot receipt remain in ignored `.artifacts/c291/`.

The pinned SculptGL reference's native HTML select did not change from custom
id 9 under the CDP Home/End or ArrowUp/ArrowDown attempts, including an
explicit DOM focus. Its subsequent selection/reselection gate is therefore
**unqualified**, despite the appended option and correct imported image.
The frozen resize, second valid import, oversized/malformed chooser matrix,
exact GPU texel upload, public save/Undo/Redo file comparison, and JPEG
normal-UI/native GUI runs were not completed. A GPU driver allocation error
after numerical preflight could still occur at first paint; its recovery has
not been exercised. Session-only storage and the preset-only Source object
path differ from SculptGL's full behavior. The inventory row remains open.

No owned browser or server was left running; ports 8794 and 8795 were closed.
