#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then echo 'This script packages a macOS app on macOS.' >&2; exit 1; fi
cargo build --locked -p obrush-app --release
obrush_version=$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "obrush-app"))')
app='.artifacts/OBrush.app'
mkdir -p "$app/Contents/MacOS"
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
# Seal this locally built app with an ad-hoc signature. No developer account
# or network notarization is required for the local development bundle.
/usr/bin/codesign --force --sign - "$app"
/usr/bin/codesign --verify --verbose=2 "$app"
printf 'Local macOS app ready: %s\n' "$app"
