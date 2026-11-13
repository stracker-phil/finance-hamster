import { useState, type KeyboardEvent } from "react";
import { Search } from "lucide-react";
import { api, toAppError, type CommandInfo } from "../api";
import { Modal } from "../components/Modal";
import { ranked } from "../lib/rank";
import { routeOf } from "../lib/targets";
import { useCommands } from "../queries/commands";
import { useElsewhere } from "../queries/year";
import { useStore } from "../state/store";

const withoutPalette = (commands: CommandInfo[]) => commands.filter((c) => c.id !== "palette.open");

const close = () => useStore.getState().setPaletteOpen(false);

function run(id: string) {
  close();
  api.runCommand(id).catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
}

interface Option {
  key: string;
  label: string;
  hint?: string;
  go: () => void;
}

/** ⌘K command palette, ranked by name (ADR 0031), followed by the documents and bank lines the text matches. */
function Palette() {
  const { data = [] } = useCommands();
  const [query, setQuery] = useState(() => useStore.getState().paletteQuery);
  const [active, setActive] = useState(0);
  const { data: elsewhere = [] } = useElsewhere(query);
  const results: Option[] = [
    ...ranked(withoutPalette(data), query, (c) => [c.title, c.id]).map((c) => ({ key: c.id, label: c.title, hint: c.shortcut ?? undefined, go: () => run(c.id) })),
    ...elsewhere.map((match, i) => ({
      key: `match-${i}`,
      label: match.label,
      hint: match.place,
      go: () => {
        close();
        useStore.getState().navigate(routeOf(match.target));
      },
    })),
  ];
  const count = results.length;
  const picked = Math.min(active, Math.max(count - 1, 0));
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      if (count > 0) setActive((picked + step + count) % count);
    } else if (e.key === "Enter" && results[picked]) {
      e.preventDefault();
      results[picked].go();
    }
  };
  return (
    <Modal className="palette" labelledBy="palette-input" onClose={close}>
      <label className="palette__search">
        <Search size={16} />
        <input
          id="palette-input"
          autoFocus
          placeholder="Search commands, documents and bank lines"
          aria-label="Search everywhere"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={onKeyDown}
        />
      </label>
      <div className="palette__list scroller" role="listbox" aria-label="Commands">
        {results.length === 0 && <div className="palette__empty">Nothing matches</div>}
        {results.map((option, i) => (
          <button key={option.key} className="menu-row" role="option" aria-selected={i === picked} onMouseMove={() => setActive(i)} onClick={option.go}>
            {option.label}
            {option.hint && <span className="menu-row__key">{option.hint}</span>}
          </button>
        ))}
      </div>
    </Modal>
  );
}

export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  return open ? <Palette /> : null;
}
