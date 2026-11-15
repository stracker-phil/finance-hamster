#!/bin/zsh
# Captures the running debug app's window in each look (docs/ux.md, Palette checks): light.png and dark.png,
# then sets the appearance back to Automatic. screenshot-looks.sh <out-dir>
DIR=${1:?usage: screenshot-looks.sh <out-dir>}
HERE=${0:A:h}
mkdir -p "$DIR"
appearance() {
  "$HERE/js.sh" "window.__TAURI_INTERNALS__.invoke('update_settings', { patch: { appearance: '$1' } })" >/dev/null
  sleep 1
}
for mode in light dark; do
  appearance $mode
  "$HERE/screenshot.sh" "$DIR/$mode.png" && echo "$DIR/$mode.png"
done
appearance system
