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
source_commit="$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)"
if git diff --quiet HEAD -- 2>/dev/null && [[ -z "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
  source_tree_state='clean'
else
  source_tree_state='dirty'
fi
lock_sha="$(shasum -a 256 Cargo.lock | awk '{print $1}')"
mkdir -p web/dist
printf '{"status":"incomplete"}\n' > web/dist/build-info.json
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
if [[ "$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)" != "$source_commit" ||
      "$(shasum -a 256 Cargo.lock | awk '{print $1}')" != "$lock_sha" ]]; then
  echo 'Source commit or Cargo.lock changed during the browser build.' >&2
  exit 1
fi
if git diff --quiet HEAD -- 2>/dev/null && [[ -z "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
  end_tree_state='clean'
else
  end_tree_state='dirty'
fi
if [[ "$end_tree_state" != "$source_tree_state" ]]; then
  echo 'Source tree state changed during the browser build.' >&2
  exit 1
fi
obrush_version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "obrush-app"))')"
python3 - "$source_commit" "$source_tree_state" "$lock_sha" "$obrush_version" > web/dist/build-info.json <<'PY'
import hashlib
import json
import sys
from pathlib import Path

commit, tree_state, lock_sha, version = sys.argv[1:]
root = Path('web/dist')
def sha256(path):
    return hashlib.sha256((root / path).read_bytes()).hexdigest()
info = {
    'schema_version': 1,
    'version': version,
    'source_commit': commit,
    'source_tree_state': tree_state,
    'cargo_lock_sha256': lock_sha,
    'wasm_bindgen_cli_version': '0.2.129',
    'portable_wasm_sha256': sha256('pkg/obrush_app_bg.wasm'),
    'simd_wasm_sha256': sha256('pkg-simd/obrush_app_bg.wasm'),
    'gpu_wasm_sha256': sha256('gpu-pkg/resident_bg.wasm'),
}
print(json.dumps(info, indent=2, sort_keys=True))
PY
printf 'Browser build ready. Serve with: python3 scripts/serve-web.py\n'
