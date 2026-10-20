#!/usr/bin/env bash
# Copy licenses/provenance for bundled, pinned public rendering assets.
set -euo pipefail
cd "$(dirname "$0")/.."
destination="${1:?Pass the distribution licenses directory}"
mkdir -p "$destination"
cp LICENSE-MIT LICENSE-APACHE "$destination/"
asset_root='crates/obrush-app/assets/sculptgl'
if [[ -d "$asset_root" ]]; then
  mkdir -p "$destination/sculptgl"
  if [[ ! -f "$asset_root/LICENSE" && ! -f "$asset_root/licenses/SculptGL-MIT.txt" ]]; then
    echo 'Bundled SculptGL asset license is missing.' >&2
    exit 1
  fi
  for notice in "$asset_root/LICENSE" "$asset_root"/*.json "$asset_root/licenses"/*.txt; do
    if [[ -f "$notice" ]]; then cp "$notice" "$destination/sculptgl/"; fi
  done
fi

mc_notice='crates/obrush-core/SCULPTGL_MIT_NOTICE.txt'
if [[ -f "$mc_notice" ]]; then
  mkdir -p "$destination/marching-cubes"
  cp "$mc_notice" "$destination/marching-cubes/"
fi

font_root='crates/obrush-app/assets/ui-fonts'
if [[ -d "$font_root" ]]; then
  mkdir -p "$destination/ui-fonts"
  if [[ ! -f "$font_root/OFL-1.1.txt" || ! -f "$font_root/FONT_MANIFEST.json" ]]; then
    echo 'Bundled UI font license/provenance is missing.' >&2
    exit 1
  fi
  for notice in "$font_root"/*.json "$font_root"/*.txt; do
    if [[ -f "$notice" ]]; then cp "$notice" "$destination/ui-fonts/"; fi
  done
fi
