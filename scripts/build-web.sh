#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cli="${WASM_BINDGEN_BIN:-.tools/wasm-bindgen/bin/wasm-bindgen}"
if [[ ! -x "$cli" ]]; then
  if command -v wasm-bindgen >/dev/null 2>&1; then cli="$(command -v wasm-bindgen)"; else
    echo 'Install the lockfile-matched tool locally: cargo install wasm-bindgen-cli --version 0.2.129 --locked --root .tools/wasm-bindgen' >&2
    exit 1
  fi
fi
if [[ "$("$cli" --version)" != 'wasm-bindgen 0.2.129' ]]; then
  echo 'The browser build requires wasm-bindgen CLI 0.2.129 to match Cargo.lock' >&2
  exit 1
fi
cargo build --locked -p obrush-app --lib --target wasm32-unknown-unknown --profile web-release
cargo build --locked -p obrush-gpu --lib --target wasm32-unknown-unknown --profile web-release
mkdir -p web/dist/pkg
"$cli" --target web --out-dir web/dist/pkg --out-name obrush_app target/wasm32-unknown-unknown/web-release/obrush_app.wasm
# Keep the portable package for browsers without WebAssembly SIMD. The
# optional BLAKE3 SIMD build uses the same Rust document and public format.
cargo build --locked -p obrush-app --lib --target wasm32-unknown-unknown --profile web-release --features obrush-core/wasm-simd-fingerprint
mkdir -p web/dist/pkg-simd
"$cli" --target web --out-dir web/dist/pkg-simd --out-name obrush_app target/wasm32-unknown-unknown/web-release/obrush_app.wasm
mkdir -p web/dist/gpu-pkg
"$cli" --target web --out-dir web/dist/gpu-pkg --out-name resident target/wasm32-unknown-unknown/web-release/obrush_gpu.wasm
cp web/index.html web/dist/index.html
cp web/topology-worker.js web/dist/topology-worker.js
cp web/source-geometry-worker.js web/dist/source-geometry-worker.js
cp web/source-worker-held-input.js web/dist/source-worker-held-input.js
cp web/uv-bake-worker.js web/dist/uv-bake-worker.js
cp web/uv-bake-input.js web/dist/uv-bake-input.js
cp web/viewport-texture-input.js web/dist/viewport-texture-input.js
cp web/startup-model-input.js web/dist/startup-model-input.js
cp web/resident-pointer.js web/dist/resident-pointer.js
bash scripts/copy-asset-notices.sh web/dist/licenses
printf 'Browser build ready. Serve with: python3 scripts/serve-web.py\n'
