import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { api } from "../api";
import { useCoversPdf } from "../lib/pdfCover";
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
  const [open, setOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const navigate = useStore((s) => s.navigate);
  useCoversPdf(open);
  const go = (fn: () => void) => () => {
    setOpen(false);
    fn();
  };
  return (
    <div className="year-menu">
      <button className="icon-btn" aria-label="More" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        <MoreHorizontal size={16} />
      </button>
      {open && (
        <div className="menu year-menu__list" role="menu">
          <button className="menu-row" role="menuitem" onClick={go(() => navigate({ page: "log", year }))}>
            Log
          </button>
          <button className="menu-row" role="menuitem" onClick={go(() => navigate({ page: "archive", year }))}>
            Archive
            <span className="menu-row__key">{archived}</span>
          </button>
          <button className="menu-row" role="menuitem" onClick={go(() => exportSummary(year))}>
            Export Summary PDF…
          </button>
          <button className="menu-row" role="menuitem" onClick={go(() => void api.rulesOpen(year).catch(showError))}>
            Open Ruleset JSON
          </button>
          <div className="menu-sep" />
          {closed ? (
            <>
              <button className="menu-row" role="menuitem" onClick={go(() => void api.yearOpenSnapshot(year).catch(showError))}>
                Open Snapshot PDF
              </button>
              <button className="menu-row" role="menuitem" onClick={go(() => void reopen(year))}>
                Reopen Year…
              </button>
            </>
          ) : (
            <button className="menu-row" role="menuitem" onClick={go(() => setClosing(true))}>
              Close Year…
            </button>
          )}
        </div>
      )}
      {closing && <CloseYearSheet year={year} onClose={() => setClosing(false)} />}
    </div>
  );
}
