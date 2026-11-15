import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { api } from "../api";
import { openMenu, useMenuOpenOn, type MenuEntry } from "../components/menu";
import { showError, undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";
import { CloseYearSheet } from "./CloseYearSheet";
import { t, useT } from "../i18n";

async function reopen(year: number) {
  const text = t().year.menu;
  const confirmed = await useStore.getState().confirm({ title: text.reopenTitle(year), text: text.reopenText, action: text.reopenAction });
  if (confirmed) api.yearReopen(year).then(undoToast).catch(showError);
}

function exportSummary(year: number) {
  api
    .yearExportSummary(year)
    .then((saved) => saved && useStore.getState().showToast("success", t().year.menu.exported(year)))
    .catch(showError);
}

export function YearMenu({ year, archived, closed }: { year: number; archived: number; closed: boolean }) {
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  const [closing, setClosing] = useState(false);
  const open = useMenuOpenOn(button);
  const navigate = useStore((s) => s.navigate);
  const text = useT().year.menu;
  const show = () => {
    if (!button) return;
    const entries: MenuEntry[] = [
      { text: text.log, run: () => navigate({ page: "log", year }) },
      { text: text.archive, hint: String(archived), run: () => navigate({ page: "archive", year }) },
      { text: text.export, run: () => exportSummary(year) },
      { text: text.ruleset, run: () => void api.settingsShow("tax-years").catch(showError) },
      "separator",
      ...(closed
        ? [
            { text: text.snapshot, run: () => void api.yearOpenSnapshot(year).catch(showError) },
            { text: text.reopen, run: () => void reopen(year) },
          ]
        : [{ text: text.close, run: () => setClosing(true) }]),
    ];
    openMenu(entries, button, text.title(year));
  };
  return (
    <div className="year-menu">
      <button ref={setButton} className="icon-btn" aria-label={text.more} aria-haspopup="menu" aria-expanded={open} onClick={show}>
        <MoreHorizontal size={16} />
      </button>
      {closing && <CloseYearSheet year={year} onClose={() => setClosing(false)} />}
    </div>
  );
}
