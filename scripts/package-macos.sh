#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then echo 'This script packages a macOS app on macOS.' >&2; exit 1; fi
source_commit="$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)"
if git diff --quiet HEAD -- 2>/dev/null && [[ -z "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
  source_tree_state='clean'
else
  source_tree_state='dirty'
fi
lock_sha="$(shasum -a 256 Cargo.lock | awk '{print $1}')"
cargo build --locked -p obrush-app --release
if [[ "$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)" != "$source_commit" ||
      "$(shasum -a 256 Cargo.lock | awk '{print $1}')" != "$lock_sha" ]]; then
  echo 'Source commit or Cargo.lock changed during the macOS build.' >&2
  exit 1
fi
if git diff --quiet HEAD -- 2>/dev/null && [[ -z "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
  end_tree_state='clean'
else
  end_tree_state='dirty'
fi
if [[ "$end_tree_state" != "$source_tree_state" ]]; then
  echo 'Source tree state changed during the macOS build.' >&2
  exit 1
fi
obrush_version=$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "obrush-app"))')
app='.artifacts/OBrush.app'
external_info='.artifacts/OBrush-build-info.json'
# An interrupted build must never leave a manifest that approves an older app.
printf '{"status":"incomplete"}\n' > "$external_info"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/obrush "$app/Contents/MacOS/OBrush.next"
mv -f "$app/Contents/MacOS/OBrush.next" "$app/Contents/MacOS/OBrush"
python3 - "$obrush_version" "$app/Contents/Info.plist" <<'PY'
import plistlib, sys
version, output = sys.argv[1:]
with open(output, "wb") as stream:
    plistlib.dump({
        "CFBundleName": "OBrush", "CFBundleDisplayName": "OBrush",
        "CFBundleIdentifier": "org.obrush.studio", "CFBundleVersion": version,
        "CFBundleShortVersionString": version, "CFBundleExecutable": "OBrush",
        "CFBundlePackageType": "APPL", "NSHighResolutionCapable": True,
    }, stream)
PY
bash scripts/copy-asset-notices.sh "$app/Contents/Resources/licenses"
archs="$(/usr/bin/lipo -archs "$app/Contents/MacOS/OBrush")"
python3 - "$app/Contents/Resources/build-info.json" "$app/Contents/MacOS/OBrush" "$obrush_version" "$source_commit" "$source_tree_state" "$lock_sha" "$archs" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

output, binary, version, commit, tree_state, lock_sha, arch = sys.argv[1:]
info = {
    'schema_version': 2,
    'version': version,
    'source_commit': commit,
    'source_tree_state': tree_state,
    'cargo_lock_sha256': lock_sha,
    'pre_sign_binary_sha256': hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
    'architecture': arch,
}
Path(output).write_text(json.dumps(info, indent=2, sort_keys=True) + '\n')
PY
# Seal this locally built app with an ad-hoc signature. No developer account
# or network notarization is required for the local development bundle.
/usr/bin/codesign --force --sign - "$app"
/usr/bin/codesign --verify --verbose=2 "$app"
python3 - "$app/Contents/Resources/build-info.json" "$app/Contents/MacOS/OBrush" "$external_info" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

internal, binary, output = map(Path, sys.argv[1:])
info = json.loads(internal.read_text())
info['app_binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
output.write_text(json.dumps(info, indent=2, sort_keys=True) + '\n')
PY
printf 'Local macOS app ready: %s\n' "$app"
