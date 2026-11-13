import { useState, type KeyboardEvent } from "react";
import { Search } from "lucide-react";
import { api, toAppError, type CommandInfo } from "../api";
import { Modal } from "../components/Modal";
import { ranked } from "../lib/rank";
import { useCommands } from "../queries/commands";
import { useStore } from "../state/store";

const withoutPalette = (commands: CommandInfo[]) => commands.filter((c) => c.id !== "palette.open");

const close = () => useStore.getState().setPaletteOpen(false);

function run(id: string) {
  close();
  api.runCommand(id).catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
}

/** ⌘K command palette, ranked by name (ADR 0031). */
function Palette() {
  const { data = [] } = useCommands();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const results = ranked(withoutPalette(data), query, (c) => [c.title, c.id]);
  const count = results.length;
  const picked = Math.min(active, Math.max(count - 1, 0));
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      if (count > 0) setActive((picked + step + count) % count);
    } else if (e.key === "Enter" && results[picked]) {
      e.preventDefault();
      run(results[picked].id);
    }
  };
  return (
    <Modal className="palette" labelledBy="palette-input" onClose={close}>
      <label className="palette__search">
        <Search size={16} />
        <input
          id="palette-input"
          autoFocus
          placeholder="Search commands"
          aria-label="Search commands"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={onKeyDown}
        />
      </label>
      <div className="palette__list scroller" role="listbox" aria-label="Commands">
        {results.length === 0 && <div className="palette__empty">No matching command</div>}
        {results.map((command, i) => (
          <button
            key={command.id}
            className="menu-row"
            role="option"
            aria-selected={i === picked}
            onMouseMove={() => setActive(i)}
            onClick={() => run(command.id)}
          >
            {command.title}
            {command.shortcut && <span className="menu-row__key">{command.shortcut}</span>}
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
