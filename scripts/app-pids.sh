# Sourced by scripts that start or stop Hamster. `app_pids path/to/hamster` lists the app process ids of
# the release or the debug build, whichever the path names, wherever it runs from: its binary without a
# subcommand (the CLI has one).
app_pids() {
  local bundle='Hamster'
  [[ "$1" == *"Hamster (Debug).app/"* ]] && bundle='Hamster \(Debug\)'
  pgrep -f "${bundle}\.app/Contents/MacOS/hamster( --hidden)?$"
}
