# OBrush development

OBrush is an independently written Rust sculpting studio. Target desktop ZBrush
2026.2.1; browser support is an additional target. Read docs/FEATURE_PARITY.md
before making feature/completion claims. A familiar brush name, unit test, or
successful build does not establish reference parity.

- Keep geometry, file IO and history in obrush-core; it must compile for WASM
  without a display or GPU.
- Keep native/browser geometry behavior shared. Validate imported files before
  replacing the document, enforce allocation limits, and preserve undo.
- Run cargo fmt, workspace tests and strict Clippy after relevant changes.
  Check the WASM target and run the browser after rendering/host changes.
- Use scripts/build-web.sh and scripts/package-macos.sh for local artifacts.
  Keep Cargo.lock and the wasm-bindgen CLI version aligned.
- Use independently created fixtures and record reference/build hashes, exact
  parameters, numerical differences and limitations. Preserve failed checks.
- .tools and .artifacts are local, ignored tooling/output directories. Do not
  add commercial binaries, license/account data, or copied proprietary source.
- Public project formats are OBJ and versioned .obrush JSON. ZPR/ZTL, UVs,
  material IO, pressure and advanced topology remain explicit gaps.
