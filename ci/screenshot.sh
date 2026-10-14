#!/bin/sh
# Render Bigify headlessly (Xvfb + Mesa software GL) in design-preview mode
# and save a screenshot. Run inside the build container:
#   ci/run.sh ci/screenshot.sh nowPlaying 1920x1080
# Output: screenshots/<screen>-<size>.png
set -e
SCREEN="${1:-home}"
SIZE="${2:-1920x1080}"
OUT="/src/screenshots/${SCREEN}-${SIZE}.png"
mkdir -p /src/screenshots
BIN="${BIGIFY_BIN:-/src/target/debug/bigify}"
export XDG_CONFIG_HOME=/tmp/bigify-config XDG_STATE_HOME=/tmp/bigify-state XDG_CACHE_HOME=/tmp/bigify-cache
mkdir -p "$XDG_CONFIG_HOME/bigify"
# Windowed so the forced size is honoured.
printf 'startFullscreen = false\ncontrollerInput = false\n' > "$XDG_CONFIG_HOME/bigify/settings.toml"
BIGIFY_DESIGN_PREVIEW=1 BIGIFY_SCREENSHOT="$OUT" BIGIFY_START_SCREEN="$SCREEN" \
BIGIFY_WINDOW_SIZE="$SIZE" LIBGL_ALWAYS_SOFTWARE=1 QT_QPA_PLATFORM=xcb \
  timeout 60 xvfb-run -a -s "-screen 0 ${SIZE}x24" "$BIN" 2>&1 | grep -v "^libEGL warning" || true
ls -la "$OUT"
