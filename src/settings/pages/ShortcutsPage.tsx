import { useState, type KeyboardEvent, type ReactNode } from "react";
import { api, toAppError, type CommandInfo } from "../../api";
import { useCommands } from "../../queries/commands";
import { acceleratorOf } from "../shortcutKeys";
import { Group, Row } from "../controls";

/** Review's single keys; they work only on the Review page. */
const REVIEW_KEYS = [
  { label: "ANV / Subscription / Ignore", keys: "A  S  I" },
  { label: "Skip", keys: "J" },
];

function ShortcutRow({ command }: { command: CommandInfo }) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = (call: Promise<void>) => {
    setRecording(false);
    call.then(() => setError(null)).catch((e: unknown) => setError(toAppError(e).message));
  };
  const onKey = (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setRecording(false);
      return;
    }
    if (e.key === "Backspace" || e.key === "Delete") {
      run(api.keybindingSet(command.id, null));
      return;
    }
    const accelerator = acceleratorOf(e);
    if (accelerator) run(api.keybindingSet(command.id, accelerator));
  };
  let description: ReactNode = command.id;
  if (recording) description = "Press the new shortcut; ⌫ removes it, esc cancels.";
  if (error)
    description = (
      <span className="settings-row__error" role="alert">
        {error}
      </span>
    );
  return (
    <Row label={command.title} description={description}>
      {/* Keeps its slot, so the shortcut beside it never moves. */}
      <button
        className="link-btn shortcut-reset"
        style={command.custom ? undefined : { visibility: "hidden" }}
        aria-hidden={!command.custom}
        tabIndex={command.custom ? undefined : -1}
        disabled={!command.custom}
        onClick={() => run(api.keybindingReset(command.id))}
      >
        Reset
      </button>
      <button
        className={recording ? "shortcut-btn shortcut-btn--recording" : "shortcut-btn"}
        aria-label={`Shortcut for ${command.title}`}
        aria-pressed={recording}
        onClick={() => setRecording((r) => !r)}
        onKeyDown={recording ? onKey : undefined}
        onBlur={() => setRecording(false)}
      >
        {recording ? "Type shortcut…" : (command.shortcut ?? "None")}
      </button>
    </Row>
  );
}

export function ShortcutsPage() {
  const { data = [] } = useCommands();
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const shown = data.filter((c) => !needle || [c.title, c.id, c.shortcut ?? ""].some((t) => t.toLowerCase().includes(needle)));
  return (
    <>
      <input className="field shortcuts-search" type="search" placeholder="Search shortcuts" aria-label="Search shortcuts" value={query} onChange={(e) => setQuery(e.target.value)} />
      <Group title="Commands" anchor="commands">
        {shown.length === 0 && <Row label={`No command matches “${query}”`} />}
        {shown.map((command) => (
          <ShortcutRow key={command.id} command={command} />
        ))}
      </Group>
      <Group title="Review" anchor="review-keys">
        {REVIEW_KEYS.map((k) => (
          <Row key={k.label} label={k.label} description="Single keys on the Review page">
            <span className="kbd">{k.keys}</span>
          </Row>
        ))}
      </Group>
    </>
  );
}
