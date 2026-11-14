#!/bin/zsh
# Captures the README screenshots, dark and light, from the debug app on a fresh demo seed: screens.sh [dir]
# (default docs/screens). Moves the debug data folder aside, starts the debug build at a fixed size in the
# background (it keeps painting while covered), drives it through its store and restores the folder after.
# Build first: bun run build:app.
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${1:-$ROOT/docs/screens}
HERE="$ROOT/scripts/dev"
APP="$ROOT/src-tauri/target/debug/bundle/macos/Hamster (Debug).app"
BIN="$APP/Contents/MacOS/hamster"
DATA="$HOME/Library/Application Support/com.philippstracker.hamster.debug"
ASIDE="$DATA.screens-backup"
[[ -x "$BIN" ]] || { echo "Build first: bun run build:app"; exit 1; }
[[ ! -e "$ASIDE" ]] || { echo "$ASIDE exists from an earlier run; restore or remove it first"; exit 1; }
mkdir -p "$OUT"

"$ROOT/scripts/stop-app.sh" "$BIN"
[[ -e "$DATA" ]] && mv "$DATA" "$ASIDE"
restore() {
  "$ROOT/scripts/stop-app.sh" "$BIN"
  rm -rf "$DATA"
  [[ -e "$ASIDE" ]] && mv "$ASIDE" "$DATA"
}
trap restore EXIT

"$ROOT/scripts/sign-app.sh" "$APP" com.philippstracker.hamster.debug
HAMSTER_WINDOW_FRAME="80,60,1440,900" HAMSTER_BACKGROUND=1 nohup "$BIN" >/dev/null 2>&1 &
disown
# The first launch seeds the demo data.
for _ in {1..60}; do
  sleep 1
  "$BIN" status >/dev/null 2>&1 && break
done
sleep 4

shot() { sleep 1.5; "$HERE/screenshot.sh" "$OUT/$1.png"; echo "$OUT/$1.png"; }
appearance() {
  "$HERE/js.sh" "window.__TAURI_INTERNALS__.invoke('update_settings', { patch: { appearance: '$1' } })"
  sleep 1
}
go() { "$HERE/js.sh" "window.hamster.getState().navigate($1)"; }

appearance dark
go '{ page: "today" }'
shot today-dark
go '{ page: "subscriptions" }'
sleep 1
"$HERE/js.sh" '(() => { [...document.querySelectorAll(".sub-row")].find((r) => r.textContent.includes("Lumen"))?.click(); })()'
shot subscriptions-dark
go '{ page: "statements" }'
shot statements-dark

appearance light
go '{ page: "year", year: Number(new Date().getFullYear()) }'
shot year-light
go '{ page: "review" }'
sleep 1
# A picked choice shows what confirming would do.
"$HERE/js.sh" '(() => { document.querySelector("[aria-label=\"Ignore (I)\"]")?.click(); setTimeout(() => document.querySelector(".review-card__always input")?.click(), 200); })()'
shot review-light
go '{ page: "documents" }'
shot documents-light
