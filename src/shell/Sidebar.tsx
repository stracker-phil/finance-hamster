import { useState, type DragEvent } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { api, toAppError } from "../api";
import { Icon, type IconName } from "../components/icons";
import { useT } from "../i18n";
import { sidebarKey, type Route } from "../state/route";
import { startAddDocument } from "../documents/addFlow";
import { useCoverage } from "../queries/statements";
import { useStore } from "../state/store";
import { ProfileSwitcher } from "./ProfileSwitcher";

export interface SidebarYear {
  year: number;
  closed: boolean;
}

const NAV: { route: Route; label: "today" | "subscriptions" | "statements" | "documents"; icon: IconName }[] = [
  { route: { page: "today" }, label: "today", icon: "home" },
  { route: { page: "subscriptions" }, label: "subscriptions", icon: "repeat" },
  { route: { page: "statements" }, label: "statements", icon: "stack" },
  { route: { page: "documents" }, label: "documents", icon: "docs" },
];

function NavArrows() {
  const canBack = useStore((s) => s.back.length > 0);
  const canForward = useStore((s) => s.forward.length > 0);
  const goBack = useStore((s) => s.goBack);
  const goForward = useStore((s) => s.goForward);
  const text = useT().shell.sidebar;
  return (
    <div className="sidebar__top" data-tauri-drag-region>
      <button className="icon-btn" onClick={goBack} disabled={!canBack} title={text.back}>
        <ChevronLeft size={16} />
      </button>
      <button className="icon-btn" onClick={goForward} disabled={!canForward} title={text.forward}>
        <ChevronRight size={16} />
      </button>
    </div>
  );
}

const openSettings = () => {
  api.runCommand("app:settings").catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
};

const hasFiles = (e: DragEvent) => e.dataTransfer.types.includes("Files");

/** Drop target for invoice PDFs. */
function DropZone() {
  const [over, setOver] = useState(false);
  const text = useT().shell.sidebar;
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
      <div className="dropzone__title">{over ? text.dropRelease : text.dropTitle}</div>
      <div className="dropzone__sub">{text.dropSub}</div>
      <button className="btn btn--small" onClick={startAddDocument}>
        {text.chooseFile}
      </button>
    </div>
  );
}

export function Sidebar({ years }: { years: SidebarYear[] }) {
  const route = useStore((s) => s.route);
  const navigate = useStore((s) => s.navigate);
  const current = sidebarKey(route);
  const { data: coverage } = useCoverage();
  const toReview = coverage?.accounts.reduce((sum, a) => sum + a.toReview, 0) ?? 0;
  const isCurrent = (r: Route) => sidebarKey(r) === current;
  const { sidebar: text, titles } = useT().shell;
  return (
    <aside className="sidebar" aria-label={text.label}>
      <NavArrows />
      <nav className="sidebar__nav">
        {NAV.map((row) => (
          <button key={row.label} className="nav-row" aria-current={isCurrent(row.route) ? "page" : undefined} onClick={() => navigate(row.route)}>
            <Icon name={row.icon} />
            <span className="nav-row__label">{titles[row.label]}</span>
            {row.route.page === "statements" && toReview > 0 && (
              <span className="nav-badge" aria-label={text.toReview(toReview)}>
                {toReview}
              </span>
            )}
          </button>
        ))}
      </nav>
      {years.length > 0 && (
        <>
          <div className="sidebar__caption">{text.taxYears}</div>
          <div className="sidebar__years">
            {years.map(({ year, closed }) => (
              <button key={year} className="nav-row" aria-current={isCurrent({ page: "year", year }) ? "page" : undefined} onClick={() => navigate({ page: "year", year })}>
                <span className={closed ? "year-tile" : "year-tile year-tile--open"}>&apos;{String(year).slice(2)}</span>
                <span className="nav-row__label">{titles.taxYear(year)}</span>
                <span className="nav-row__meta">{closed ? <Icon name="lock" size={11} strokeWidth={2.2} /> : text.open}</span>
              </button>
            ))}
          </div>
        </>
      )}
      <DropZone />
      <div className="sidebar__footer">
        <button className="icon-btn" onClick={openSettings} title={text.settings}>
          <Icon name="settings" />
        </button>
        <ProfileSwitcher />
      </div>
    </aside>
  );
}
