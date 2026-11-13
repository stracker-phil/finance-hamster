#!/bin/zsh
# Checks that nothing jumps in the running debug app: e2e.sh
# Clicks every sidebar row twice and opens and closes + New, measuring the panel, the header, the first sidebar
# row and the clicked control after every step, then checks that every overlay hides the native PDF preview
# (the debug build's sample document). Exits 1 on any drift of 1px or more or a preview left over an overlay.
# Start the app first: bun run build:app && scripts/dev/launch.sh
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/src-tauri/target/debug/bundle/macos/Hamster (Debug).app/Contents/MacOS/hamster"
LOG="${TMPDIR:-/tmp}/hamster-dev.log"
MARK="E2E-$(date +%s)-$$"
"$BIN" status >/dev/null 2>&1 || { echo "The debug app isn't running: scripts/dev/launch.sh"; exit 1; }

"$ROOT/scripts/dev/js.sh" "(async () => {
  const report = (text) => window.__TAURI_INTERNALS__.invoke('log_frontend_error', { message: '$MARK ' + text });
  const wait = (ms) => new Promise((done) => setTimeout(done, ms));
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height].map(Math.round).join(',');
  };
  const parts = (control) => ({
    panel: rect(document.querySelector('.panel')),
    header: rect(document.querySelector('.header')),
    firstRow: rect(document.querySelector('.sidebar__nav .nav-row')),
    filter: rect(document.querySelector('.filter-field')),
    control: rect(control),
  });
  const drift = [];
  const compare = (step, before, after) => {
    for (const name of Object.keys(before)) {
      if (before[name] !== after[name]) drift.push(step + ': ' + name + ' ' + before[name] + ' -> ' + after[name]);
    }
  };
  const clickTwice = async (name, control) => {
    if (!control) return drift.push(name + ': not found');
    const before = parts(control);
    control.click();
    await wait(500);
    compare(name + ' once', before, parts(control));
    control.click();
    await wait(500);
    compare(name + ' twice', before, parts(control));
  };
  const pdfVisible = () => window.__TAURI_INTERNALS__.invoke('pdf_view_visible');
  const expectPdf = async (step, visible) => {
    await wait(400);
    if ((await pdfVisible()) !== visible) drift.push(step + ': the PDF preview is ' + (visible ? 'hidden' : 'still showing'));
  };
  const overlays = async () => {
    const store = window.hamster.getState();
    store.navigate({ page: 'document', documentId: 'sample', year: null });
    await expectPdf('sample document', true);
    document.querySelector('.new-btn').click();
    await expectPdf('+ New open', false);
    document.querySelector('.new-btn').click();
    await expectPdf('+ New closed', true);
    store.setPaletteOpen(true);
    await expectPdf('palette open', false);
    window.hamster.getState().setPaletteOpen(false);
    await expectPdf('palette closed', true);
    store.showToast('info', 'E2E toast');
    await expectPdf('toast shown', false);
    for (const t of window.hamster.getState().toasts) window.hamster.getState().dismissToast(t.id);
    await expectPdf('toast dismissed', true);
    store.navigate({ page: 'today' });
    await expectPdf('left the document', false);
  };
  try {
    for (const row of document.querySelectorAll('.sidebar .nav-row')) await clickTwice(row.textContent.trim(), row);
    await clickTwice('New', document.querySelector('.new-btn'));
    await overlays();
    report(drift.length === 0 ? 'OK' : 'DRIFT ' + drift.join(' | '));
  } catch (error) {
    report('ERROR ' + error);
  }
})()"

for _ in {1..40}; do
  sleep 0.5
  line=$(grep -a -o "$MARK [^\"]*" "$LOG" | head -1) && [[ -n "$line" ]] && break
done
result=${line#"$MARK "}
case "$result" in
  OK) echo "No drift"; exit 0 ;;
  "") echo "No result in $LOG"; exit 1 ;;
  *) echo "${result//' | '/$'\n'}"; exit 1 ;;
esac
