#!/usr/bin/env bash
# Rasterize every console_*.svg → console_*.png at 1024×1024 via Edge headless.
set -e
EDGE="/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
DIR="$(cd "$(dirname "$0")" && pwd -W 2>/dev/null || cd "$(dirname "$0")" && pwd)"

for svg in "$(dirname "$0")"/console_*.svg; do
  base="$(basename "$svg" .svg)"
  out="$(dirname "$svg")/${base}.png"
  url="file:///$(cygpath -m "$svg" 2>/dev/null || echo "$svg")"
  echo "→ $base"
  "$EDGE" --headless --disable-gpu --no-sandbox \
    --window-size=1024,1024 \
    --default-background-color=00000000 \
    --screenshot="$(cygpath -m "$out" 2>/dev/null || echo "$out")" \
    "$url" >/dev/null 2>&1
done
echo "Done."
