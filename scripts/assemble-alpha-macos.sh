#!/usr/bin/env bash
# Assemble a versioned public-alpha archive from the official local app bundle.
set -euo pipefail
cd "$(dirname "$0")/.."

release_tag="${1:?Pass a version tag such as v0.1.20-alpha.1}"
if [[ ! "$release_tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-alpha\.[0-9]+)?$ ]]; then
  echo 'Expected vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-alpha.N.' >&2
  exit 1
fi
package_version="${BASH_REMATCH[1]}"
manifest_version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "obrush-app"))')"
if [[ "$package_version" != "$manifest_version" ]]; then
  echo "Tag $release_tag does not match obrush-app version $manifest_version." >&2
  exit 1
fi
if [[ "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  echo 'The public-alpha macOS archive requires a native Apple Silicon host.' >&2
  exit 1
fi

app="${OBRUSH_ALPHA_APP_PATH:-.artifacts/OBrush.app}"
binary="$app/Contents/MacOS/OBrush"
if [[ ! -f "$binary" ]]; then
  echo 'Run scripts/package-macos.sh first.' >&2
  exit 1
fi
archs="$(/usr/bin/lipo -archs "$binary")"
if [[ "$archs" != arm64 ]]; then
  echo "Expected one arm64 app binary, got: $archs" >&2
  exit 1
fi
source_commit="$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)"
if ! git diff --quiet HEAD -- 2>/dev/null || [[ -n "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
  echo 'Public release assembly requires a clean source checkout.' >&2
  exit 1
fi
lock_sha="$(shasum -a 256 Cargo.lock | awk '{print $1}')"
python3 scripts/verify-alpha-macos-build.py "$app" "$package_version" "$source_commit" "$lock_sha"
/usr/bin/codesign --verify --verbose=2 "$app"
for notice in \
  "$app/Contents/Resources/licenses/sculptgl" \
  "$app/Contents/Resources/licenses/marching-cubes" \
  "$app/Contents/Resources/licenses/ui-fonts"; do
  if [[ ! -d "$notice" ]]; then
    echo "Missing bundled asset notices: $notice" >&2
    exit 1
  fi
done

release_dir='.artifacts/release'
stage="$release_dir/$release_tag"
archive="$release_dir/OBrush-$release_tag-macos-arm64.zip"
mkdir -p "$release_dir"
if [[ -e "$stage" || -e "$archive" ]]; then
  echo "Release staging or archive already exists for $release_tag; choose a fresh output directory." >&2
  exit 1
fi
mkdir "$stage"
/usr/bin/ditto "$app" "$stage/OBrush.app"
cp LICENSE-MIT LICENSE-APACHE "$stage/"
cat > "$stage/START-HERE.txt" <<EOF
OBRush $release_tag public alpha (macOS Apple Silicon)

Open OBrush.app to start sculpting. This app is ad-hoc signed for local use,
not Apple notarized. macOS may ask you to allow it in System Settings.
Save projects explicitly; browser and native autosave/recovery are unfinished.

This alpha is not at SculptGL or ZBrush feature parity. Please report
reproducible bugs and vote on feature requests in the public source repository.
EOF
cp "$app/Contents/Resources/build-info.json" "$stage/BUILD-INFO.json"
printf 'tag=%s\nsignature=ad-hoc; not notarized\n' "$release_tag" > "$stage/RELEASE-INFO.txt"
/usr/bin/codesign --verify --verbose=2 "$stage/OBrush.app"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$stage" "$archive"
printf '%s  %s\n' "$(shasum -a 256 "$archive" | awk '{print $1}')" "$(basename "$archive")" > "$release_dir/SHA256SUMS.txt"
echo "Ready: $archive and $release_dir/SHA256SUMS.txt"
