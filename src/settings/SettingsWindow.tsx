import { ChevronLeft } from "lucide-react";
import { useMemo, useState } from "react";
import { Icon } from "../components/icons";
import { useT } from "../i18n";
import { SETTINGS_PAGES } from "./pages";
import { RevealTarget, SettingsOpen } from "./controls";
import { groupsOf, resolveTarget, searchSettings, type SettingsMatch } from "./registry";

/** What Settings is asked to show: a page or row id. A new object per request, so asking again reveals again. */
export interface SettingsRequest {
  id?: string;
}

const targetOf = (match: SettingsMatch) => match.row?.id ?? match.page.id;

function NavButton({ page, row, current, onOpen }: SettingsMatch & { current: string; onOpen: (id: string) => void }) {
  return (
    <button
      type="button"
      className="settings-nav__row"
      aria-label={row ? `${row.title}, ${page.label}` : undefined}
      aria-current={!row && current === page.id ? "page" : undefined}
      onClick={() => onOpen(row?.id ?? page.id)}
    >
      <Icon name={page.icon} size={15} />
      <span className="settings-nav__text">
        <span>{page.label}</span>
        {row && <span className="settings-nav__setting">{row.title}</span>}
      </span>
    </button>
  );
}

/** The page list under its group headings; a search replaces it with every matching page and row, best first. */
function SettingsNav({ current, onOpen }: { current: string; onOpen: (id: string) => void }) {
  const text = useT().settings.window;
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const results = searching ? searchSettings(SETTINGS_PAGES, query) : [];
  return (
    <nav className="settings-nav" aria-label={text.nav}>
      <div className="settings-nav__titlebar" data-tauri-drag-region />
      <input
        className="field settings-nav__search"
        type="search"
        aria-label={text.search}
        placeholder={text.searchPlaceholder}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && results[0]) onOpen(targetOf(results[0]));
        }}
      />
      <div className="settings-nav__list scroller">
        {searching ? (
          results.length === 0 ? (
            <p className="settings-nav__empty">{text.noResults}</p>
          ) : (
            results.map((match) => <NavButton key={targetOf(match)} {...match} current={current} onOpen={onOpen} />)
          )
        ) : (
          groupsOf(SETTINGS_PAGES).map(({ group, pages }) => (
            <div key={group} role="group" aria-label={group}>
              <div className="settings-nav__heading" aria-hidden="true">
                {group}
              </div>
              {pages.map((page) => (
                <NavButton key={page.id} page={page} current={current} onOpen={onOpen} />
              ))}
            </div>
          ))
        )}
      </div>
    </nav>
  );
}

/** `request.id` may name a row (its page opens scrolled to it, washed in the accent once) or a detail one level
 *  down (`<page id>/<detail id>`), shown in the pane with a back button. */
export function SettingsWindow({ request, onOpen }: { request: SettingsRequest; onOpen: (id: string) => void }) {
  const text = useT().settings.window;
  const { page, detail } = resolveTarget(SETTINGS_PAGES, request.id);
  const target = useMemo(() => (request.id && request.id !== page.id && !detail ? { id: request.id } : undefined), [request, page, detail]);
  const { Body } = page;
  const Detail = detail !== undefined ? page.detail?.Body : undefined;
  return (
    <div className="settings-window">
      <SettingsNav current={page.id} onOpen={onOpen} />
      <div className="settings-pane">
        <header data-tauri-drag-region>
          {Detail && (
            <button type="button" className="icon-btn settings-back" aria-label={text.back(page.label)} onClick={() => onOpen(page.id)}>
              <ChevronLeft size={18} />
            </button>
          )}
          <h2 data-tauri-drag-region>{Detail && detail ? page.detail?.title(detail) : page.label}</h2>
        </header>
        <div className="settings-body scroller" key={detail === undefined ? page.id : `${page.id}/${detail}`}>
          <SettingsOpen.Provider value={onOpen}>
            {Detail && detail ? (
              <Detail id={detail} />
            ) : (
              <>
                {page.intro && <p className="settings-intro">{page.intro}</p>}
                <RevealTarget.Provider value={target}>
                  <Body />
                </RevealTarget.Provider>
              </>
            )}
          </SettingsOpen.Provider>
        </div>
      </div>
    </div>
  );
}
