import { useState, type ReactNode } from "react";
import { openMenu, useMenuOpenOn } from "../components/menu";
import { useCommands } from "../queries/commands";
import { ChevronDown, ChevronLeft, Plus, Search, X } from "lucide-react";
import { startAddDocument } from "../documents/addFlow";
import { FILTER_PLACEHOLDER } from "../pages/pages";
import { useT } from "../i18n";
import type { Route } from "../state/route";
import { useStore } from "../state/store";
import { useCoversPdf } from "../lib/pdfCover";
import { routeOf } from "../lib/targets";
import { useElsewhere } from "../queries/year";
import { newSubscription } from "../subscriptions/flow";
import { startImport } from "../statements/importFlow";

/** The title and, for a nested page, the crumb back to its parent. */
export interface HeaderTitle {
  title: string;
  sub?: string;
  crumb?: { label: string; to: Route };
}

/** The filter field's id, so ⌘F (`filter:focus`) can reach it. */
export const FILTER_FIELD_ID = "page-filter";

/** Where else the filter text turns up, under the field; ⌘↩ searches everywhere in the palette. */
function ElsewhereMenu({ query }: { query: string }) {
  const { data: matches = [] } = useElsewhere(query);
  const navigate = useStore((s) => s.navigate);
  useCoversPdf(true);
  const text = useT().shell.header;
  return (
    <div className="menu elsewhere" role="menu" aria-label={text.matchesElsewhere}>
      {matches.length > 0 && <div className="elsewhere__caption">{text.matchesElsewhere}</div>}
      {matches.map((match, i) => (
        <button key={i} className="menu-row" role="menuitem" onMouseDown={(e) => e.preventDefault()} onClick={() => navigate(routeOf(match.target))}>
          {match.label}
          <span className="menu-row__key">{match.place}</span>
        </button>
      ))}
      <button className="menu-row" role="menuitem" onMouseDown={(e) => e.preventDefault()} onClick={() => useStore.getState().searchEverywhere(query)}>
        {text.searchEverywhere}
        <span className="menu-row__key">⌘↩</span>
      </button>
    </div>
  );
}

function FilterField({ page }: { page: Route["page"] }) {
  const filter = useStore((s) => s.filter);
  const setFilter = useStore((s) => s.setFilter);
  const [focused, setFocused] = useState(false);
  const text = useT().shell.header;
  const scoped = FILTER_PLACEHOLDER[page];
  const placeholder = scoped ?? text.searchEverywhere;
  const key = scoped ? "⌘F" : "⌘K";
  return (
    <div className="filter-wrap">
      <label className="filter-field">
        <Search size={14} />
        <input
          id={FILTER_FIELD_ID}
          value={filter}
          placeholder={placeholder}
          aria-label={placeholder}
          onChange={(e) => setFilter(e.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape" && filter) {
              e.stopPropagation();
              setFilter("");
            } else if (e.key === "Enter" && (e.metaKey || !scoped) && filter.trim()) {
              e.preventDefault();
              useStore.getState().searchEverywhere(filter);
            }
          }}
        />
        {filter ? (
          <button className="icon-btn icon-btn--tiny" onClick={() => setFilter("")} aria-label={text.clearFilter}>
            <X size={11} />
          </button>
        ) : (
          <span className="kbd">{key}</span>
        )}
      </label>
      {focused && filter.trim() && <ElsewhereMenu query={filter} />}
    </div>
  );
}

/** The + New menu's commands, with the shortcut each has now. */
const NEW_ITEMS: { label: "importStatement" | "addInvoice" | "newSubscription"; command: string; run: () => void }[] = [
  { label: "importStatement", command: "statement:import", run: startImport },
  { label: "addInvoice", command: "document:add", run: startAddDocument },
  { label: "newSubscription", command: "subscription:new", run: newSubscription },
];

function NewMenu() {
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  const open = useMenuOpenOn(button);
  const { data: commands = [] } = useCommands();
  const text = useT().shell.header;
  const show = () => {
    if (!button) return;
    const entries = NEW_ITEMS.map((item) => ({
      text: text[item.label],
      hint: commands.find((c) => c.id === item.command)?.shortcut ?? undefined,
      run: item.run,
    }));
    openMenu(entries, button, text.newMenu);
  };
  return (
    <button ref={setButton} className="btn new-btn" aria-haspopup="menu" aria-expanded={open} onClick={show}>
      <Plus size={15} />
      {text.newMenu}
      <ChevronDown size={12} />
    </button>
  );
}

function Crumb({ label, to }: { label: string; to: Route }) {
  const navigate = useStore((s) => s.navigate);
  const text = useT().shell.header;
  return (
    <>
      <button className="icon-btn" onClick={() => navigate(to)} aria-label={text.backTo(label)}>
        <ChevronLeft size={16} />
      </button>
      <span className="header__crumb">{label} /</span>
    </>
  );
}

/** The page header: crumb, title, page controls, filter field and New menu. */
export function Header({ route, title, controls }: { route: Route; title: HeaderTitle; controls?: ReactNode }) {
  return (
    <header className="header" data-tauri-drag-region>
      {title.crumb && <Crumb {...title.crumb} />}
      <h1 className="header__title">{title.title}</h1>
      {title.sub && <span className="header__sub">{title.sub}</span>}
      {controls && <div className="header__slot">{controls}</div>}
      <div className="header__spacer" data-tauri-drag-region />
      <FilterField page={route.page} />
      <NewMenu />
    </header>
  );
}
