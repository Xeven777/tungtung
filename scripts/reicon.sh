#!/usr/bin/env bash
# Regenerate every app icon from a single source PNG.
#
# Usage:
#   ./scripts/reicon.sh path/to/new-icon.png
#   ./scripts/reicon.sh path/to/new-icon.png --no-tauri
#
# What it does:
#   1. Optimizes + resizes the source to 1024px -> ./icon.png (repo root)
#   2. Copies it to ./public/icon.png + generates favicons
#      (favicon-16/32/48.png, favicon.ico)
#   3. Runs `tauri icon` to regenerate src-tauri/icons/
#      (pngs, icon.icns, icon.ico) and prunes mobile extras
#      (android/, ios/, 64x64.png) which this desktop-only app doesn't need.
#
# Requirements: ImageMagick (`magick`), python3 + Pillow, bun.
set -euo pipefail

SRC="${1:-}"
NO_TAURI=false
[[ "${2:-}" == "--no-tauri" ]] && NO_TAURI=true

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "Usage: $0 path/to/new-icon.png [--no-tauri]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

command -v magick >/dev/null || { echo "error: 'magick' (ImageMagick) not found" >&2; exit 1; }
command -v python3 >/dev/null || { echo "error: python3 not found" >&2; exit 1; }

echo "→ optimizing $SRC -> icon.png (1024px max, stripped, max compression)"
magick "$SRC" -resize '1024x1024>' -strip \
  -define png:compression-level=9 -define png:compression-filter=5 \
  icon.png

echo "→ public/icon.png + favicons"
cp icon.png public/icon.png
python3 - <<'PY'
from PIL import Image
im = Image.open("icon.png")
for s in (16, 32, 48):
    c = im.copy()
    c.thumbnail((s, s), Image.LANCZOS)
    c.save(f"public/favicon-{s}.png", optimize=True)
im.save("public/favicon.ico", sizes=[(16, 16), (32, 32), (48, 48)])
print("  favicons written")
PY

if [[ "$NO_TAURI" == true ]]; then
  echo "→ skipping 'tauri icon' (--no-tauri)"
else
  echo "→ tauri icon icon.png"
  bunx --package @tauri-apps/cli tauri icon icon.png
  echo "→ pruning mobile extras (android/, ios/, 64x64.png)"
  rm -rf src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/64x64.png
fi

echo
echo "done. changed files:"
ls -la icon.png public/icon.png public/favicon* src-tauri/icons/ | sed 's/^/  /'
echo
echo "next: rebuild to verify — bun run build"
