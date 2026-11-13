import { useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown, ChevronLeft, Plus, Search, X } from "lucide-react";
import { FILTER_PLACEHOLDER } from "../pages/pages";
import type { Route } from "../state/route";
import { useStore } from "../state/store";

/** The title and, for a nested page, the crumb back to its parent. */
export interface HeaderTitle {
  title: string;
  sub?: string;
  crumb?: { label: string; to: Route };
}

function FilterField({ page }: { page: Route["page"] }) {
  const filter = useStore((s) => s.filter);
  const setFilter = useStore((s) => s.setFilter);
  const scoped = FILTER_PLACEHOLDER[page];
  const placeholder = scoped ?? "Search everywhere";
  const key = scoped ? "⌘F" : "⌘K";
  return (
    <label className="filter-field">
      <Search size={14} />
      <input
        value={filter}
        placeholder={placeholder}
        aria-label={placeholder}
        onChange={(e) => setFilter(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape" && filter) {
            e.stopPropagation();
            setFilter("");
          }
        }}
      />
      {filter ? (
        <button className="icon-btn icon-btn--tiny" onClick={() => setFilter("")} aria-label="Clear filter (Esc)">
          <X size={11} />
        </button>
      ) : (
        <span className="kbd">{key}</span>
      )}
    </label>
  );
}

const NEW_ITEMS = [
  { label: "Import Statement…", key: "⌘I" },
  { label: "Add Invoice PDF…", key: "⌘O" },
  { label: "New Subscription", key: "⌘N" },
];

function NewMenu() {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!anchor.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open]);
  return (
    <span className="header__anchor" ref={anchor}>
      <button className="btn new-btn" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        <Plus size={15} />
        New
        <ChevronDown size={12} />
      </button>
      {open && (
        <div className="menu new-menu" role="menu">
          {NEW_ITEMS.map((item) => (
            <button key={item.label} className="menu-row" role="menuitem" disabled>
              {item.label}
              <span className="menu-row__key">{item.key}</span>
            </button>
          ))}
        </div>
      )}
    </span>
  );
}

function Crumb({ label, to }: { label: string; to: Route }) {
  const navigate = useStore((s) => s.navigate);
  return (
    <>
      <button className="icon-btn" onClick={() => navigate(to)} aria-label={`Back to ${label}`}>
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
