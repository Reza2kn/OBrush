# C296 integrated local checkpoint

The primary `codex/sculptgl-parity-foundation` checkout combines the C287
camera transition, C291 session MatCap import, C294 bounded UV JPEG import,
and C295 opt-in Source Drag selected-union route. The corresponding reports
contain the independent fixtures, paired browser results, and failed checks.
This checkpoint verifies their combined build; it does not close a full
SculptGL or ZBrush feature row.

- Source HEAD `b5532d5e631ab106cdf73a274b4d4cd62c5b22f4`; the last commit
  adds a test-only invalid-preview Drag host regression after the artifact
  build, without changing product code.
- `cargo fmt --all -- --check`, `cargo test --workspace --locked --quiet`,
  native strict workspace Clippy and strict WASM core/app Clippy passed on
  the integrated checkout. The workspace test run had one intentionally
  ignored frozen-snapshot generator and no failures.
- `scripts/build-web.sh` passed with lockfile-matched `wasm-bindgen` 0.2.129.
  Portable WASM SHA-256: `944cd8fec4350d0fd95829e47846c63d613d6ab1a5e905055c850bc01fbbf096`;
  SIMD WASM: `d96cbd1c51365a338c1259c384b8ee33d930ffce36bda96e9209578bc52ce66f`;
  portable JS: `80fca3b2e9ad20bc3cb1fcefb2780f057108fb6a5c07738d39ebfe17a40d3754`.
- `scripts/package-macos.sh` passed and `codesign --verify --deep --strict`
  accepted `.artifacts/OBrush.app`. Executable SHA-256:
  `8b54d13f0606c8ac21ede56051dde12b55657e28a63ff00a03ec2af2894c77a6`.
- The existing root browser server at `127.0.0.1:8792` returned both WASM
  files byte-for-byte equal to the rebuilt local files. A separate background
  browser tab visibly loaded the default 40,962-vertex/81,920-face sphere
  and ordinary sculpting controls after startup.

The 317-ID current inventory still has 309 active rows with full acceptance
open. C292's fresh million-vertex run still failed all four responsiveness
gates. C291 and C294 are session-only appearance slices, and C295 covers one
settled-camera Drag fixture with broader settings/topology/performance open.
