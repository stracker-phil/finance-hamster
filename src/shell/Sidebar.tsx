import { useState, type DragEvent } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { api, toAppError } from "../api";
import { Icon, type IconName } from "../components/icons";
import { sidebarKey, type Route } from "../state/route";
import { useStore } from "../state/store";

export interface SidebarYear {
  year: number;
  closed: boolean;
}

const NAV: { route: Route; label: string; icon: IconName }[] = [
  { route: { page: "today" }, label: "Today", icon: "home" },
  { route: { page: "subscriptions" }, label: "Subscriptions", icon: "repeat" },
  { route: { page: "statements" }, label: "Statements", icon: "stack" },
  { route: { page: "documents" }, label: "Documents", icon: "docs" },
];

function NavArrows() {
  const canBack = useStore((s) => s.back.length > 0);
  const canForward = useStore((s) => s.forward.length > 0);
  const goBack = useStore((s) => s.goBack);
  const goForward = useStore((s) => s.goForward);
  return (
    <div className="sidebar__top" data-tauri-drag-region>
      <button className="icon-btn" onClick={goBack} disabled={!canBack} title="Back (⌘[)">
        <ChevronLeft size={16} />
      </button>
      <button className="icon-btn" onClick={goForward} disabled={!canForward} title="Forward (⌘])">
        <ChevronRight size={16} />
      </button>
    </div>
  );
}

const openSettings = () => {
  api.runCommand("settings.open").catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
};

const hasFiles = (e: DragEvent) => e.dataTransfer.types.includes("Files");

/** Drop target for invoice PDFs. */
function DropZone() {
  const [over, setOver] = useState(false);
  return (
    <div
      className={over ? "dropzone dropzone--over" : "dropzone"}
      onDragOver={(e) => {
        if (!hasFiles(e)) return;
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
      }}
    >
      <span className="dropzone__icon">
        <Icon name="addDoc" size={18} />
      </span>
      <div className="dropzone__title">{over ? "Release to add" : "Drop an invoice PDF"}</div>
      <div className="dropzone__sub">It lands in the tax year of its payment</div>
      <button className="btn btn--small" disabled>
        Choose File…
      </button>
    </div>
  );
}

export function Sidebar({ years }: { years: SidebarYear[] }) {
  const route = useStore((s) => s.route);
  const navigate = useStore((s) => s.navigate);
  const current = sidebarKey(route);
  const isCurrent = (r: Route) => sidebarKey(r) === current;
  return (
    <aside className="sidebar" aria-label="Sidebar">
      <NavArrows />
      <nav className="sidebar__nav">
        {NAV.map((row) => (
          <button key={row.label} className="nav-row" aria-current={isCurrent(row.route) ? "page" : undefined} onClick={() => navigate(row.route)}>
            <Icon name={row.icon} />
            <span className="nav-row__label">{row.label}</span>
          </button>
        ))}
      </nav>
      {years.length > 0 && (
        <>
          <div className="sidebar__caption">Tax years</div>
          <div className="sidebar__years">
            {years.map(({ year, closed }) => (
              <button key={year} className="nav-row" aria-current={isCurrent({ page: "year", year }) ? "page" : undefined} onClick={() => navigate({ page: "year", year })}>
                <span className={closed ? "year-tile" : "year-tile year-tile--open"}>&apos;{String(year).slice(2)}</span>
                <span className="nav-row__label">Tax year {year}</span>
                <span className="nav-row__meta">{closed ? <Icon name="lock" size={11} strokeWidth={2.2} /> : "Open"}</span>
              </button>
            ))}
          </div>
        </>
      )}
      <DropZone />
      <div className="sidebar__footer">
        <button className="icon-btn" onClick={openSettings} title="Settings and rules (⌘,)">
          <Icon name="settings" />
        </button>
      </div>
    </aside>
  );
}
