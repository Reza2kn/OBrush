# Build OBrush

OBrush uses one Rust sculpting core for desktop and browser builds.

## Desktop

Install Rust through rustup. `rust-toolchain.toml` selects the tested Rust 1.96.0 compiler, formatter, Clippy and WASM target for reproducible build checks.

```sh
cargo run --locked -p obrush-app --release
```

On macOS, create a double-clickable local app:

```sh
scripts/package-macos.sh
open .artifacts/OBrush.app
```

The macOS bundle is built for the local host architecture and ad-hoc signed for local use. Windows/Linux targets use the same app source but have not been executed in this milestone. Linux needs its normal graphics/window-system development libraries.

## Browser

The browser uses the same Rust engine compiled to WebAssembly and the same viewport rendered with WebGL2. No backend or external API is required for sculpting.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked --root .tools/wasm-bindgen
scripts/build-web.sh
python3 scripts/serve-web.py
```

Open **http://127.0.0.1:8787**. The server binds only to localhost and serves `web/dist`, not the repository. A modern browser with WebGL2 enabled is required. The glue generator must match the version of `wasm-bindgen` in `Cargo.lock`; update the script/tool together when upgrading dependencies. The build is static and can be hosted as the contents of `web/dist`.

Use **Open** to import a local `.obj`, `.ply`, `.stl` or `.obrush` file; **Save project** and the active-mesh export controls download files. Browser downloads are user-managed saves. The browser warns before leaving an edited document, including after requesting a download: check that the file reached your Downloads before choosing to leave. Reloading/closing still discards the in-memory document; autosave and recovery are planned. The [public-alpha save-safety check](PUBLIC_ALPHA_SAFETY_2026_10_20.md) records the tested scope and native UI limitation.

## Check a change

```sh
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p obrush-app --target wasm32-unknown-unknown
cargo clippy --locked -p obrush-app --target wasm32-unknown-unknown -- -D warnings
cargo run --locked -p obrush-core --example core_timing --release
```

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines and [the feature contract](FEATURE_PARITY.md) for validation requirements.
