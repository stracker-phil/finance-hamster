import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { api } from "../api";
import { openMenu, useMenuOpenOn, type MenuEntry } from "../components/menu";
import { showError, undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";
import { CloseYearSheet } from "./CloseYearSheet";

async function reopen(year: number) {
  const confirmed = await useStore.getState().confirm({
    title: `Reopen Tax Year ${year}?`,
    text: "Its documents and answers can change again. The snapshot taken when it was closed stays.",
    action: "Reopen",
  });
  if (confirmed) api.yearReopen(year).then(undoToast).catch(showError);
}

function exportSummary(year: number) {
  api
    .yearExportSummary(year)
    .then((saved) => saved && useStore.getState().showToast("success", `Exported the Tax Year ${year} summary`))
    .catch(showError);
}

export function YearMenu({ year, archived, closed }: { year: number; archived: number; closed: boolean }) {
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  const [closing, setClosing] = useState(false);
  const open = useMenuOpenOn(button);
  const navigate = useStore((s) => s.navigate);
  const show = () => {
    if (!button) return;
    const entries: MenuEntry[] = [
      { text: "Log", run: () => navigate({ page: "log", year }) },
      { text: "Archive", hint: String(archived), run: () => navigate({ page: "archive", year }) },
      { text: "Export Summary PDF…", run: () => exportSummary(year) },
      { text: "Open Ruleset JSON", run: () => void api.rulesOpen(year).catch(showError) },
      "separator",
      ...(closed
        ? [
            { text: "Open Snapshot PDF", run: () => void api.yearOpenSnapshot(year).catch(showError) },
            { text: "Reopen Year…", run: () => void reopen(year) },
          ]
        : [{ text: "Close Year…", run: () => setClosing(true) }]),
    ];
    openMenu(entries, button, `Tax Year ${year}`);
  };
  return (
    <div className="year-menu">
      <button ref={setButton} className="icon-btn" aria-label="More" aria-haspopup="menu" aria-expanded={open} onClick={show}>
        <MoreHorizontal size={16} />
      </button>
      {closing && <CloseYearSheet year={year} onClose={() => setClosing(false)} />}
    </div>
  );
}
