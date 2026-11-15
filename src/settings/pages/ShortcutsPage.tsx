import { useState, type KeyboardEvent, type ReactNode } from "react";
import { api, toAppError, type CommandInfo } from "../../api";
import { useCommands } from "../../queries/commands";
import { acceleratorOf } from "../shortcutKeys";
import { ConfirmSheet } from "../../components/ConfirmSheet";
import { useT, type Messages } from "../../i18n";
import { Group, Row } from "../controls";

/** Single keys that work on one page only; they are printed on that page's buttons, so they stay fixed
 *  (ADR 0046). */
const pageKeys = (text: Messages["settings"]["shortcuts"]) => [
  {
    page: text.reviewPage,
    anchor: "review-keys",
    keys: [
      { label: text.reviewChoices, keys: "A  S  I" },
      { label: text.skip, keys: "J" },
    ],
  },
  {
    page: text.yearCheckPage,
    anchor: "year-check-keys",
    keys: [
      { label: text.yesNoLater, keys: "Y  N  L" },
      { label: text.previousNext, keys: "↑  ↓" },
    ],
  },
];

function ShortcutRow({ command }: { command: CommandInfo }) {
  const text = useT().settings.shortcuts;
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [taking, setTaking] = useState<{ accelerator: string; label: string; from: string } | null>(null);
  const run = (call: Promise<void>) => {
    setRecording(false);
    call.then(() => setError(null)).catch((e: unknown) => setError(toAppError(e).message));
  };
  const take = (accelerator: string) => {
    setRecording(false);
    api
      .keybindingConflict(command.id, accelerator)
      .then((conflict) => {
        if (conflict) setTaking({ accelerator, label: conflict.shortcut, from: conflict.holder });
        else run(api.keybindingSet(command.id, accelerator));
      })
      .catch((e: unknown) => setError(toAppError(e).message));
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
    if (accelerator) take(accelerator);
  };
  let description: ReactNode = undefined;
  if (recording) description = text.recording;
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
        {text.reset}
      </button>
      <button
        className={recording ? "shortcut-btn shortcut-btn--recording" : "shortcut-btn"}
        aria-label={text.shortcutFor(command.title)}
        aria-pressed={recording}
        onClick={() => setRecording((r) => !r)}
        onKeyDown={recording ? onKey : undefined}
        onBlur={() => setRecording(false)}
      >
        {recording ? text.typeShortcut : (command.shortcut ?? text.none)}
      </button>
      {taking && (
        <ConfirmSheet
          title={text.reassignTitle(taking.label)}
          text={text.reassignText(taking.label, taking.from)}
          action={text.reassign}
          onCancel={() => setTaking(null)}
          onConfirm={() => {
            setTaking(null);
            run(api.keybindingReassign(command.id, taking.accelerator));
          }}
        />
      )}
    </Row>
  );
}

export function ShortcutsPage() {
  const { data = [] } = useCommands();
  const text = useT().settings.shortcuts;
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const shown = data.filter((c) => !needle || [c.title, c.id, c.shortcut ?? ""].some((t) => t.toLowerCase().includes(needle)));
  return (
    <>
      <input className="field shortcuts-search" type="search" placeholder={text.search} aria-label={text.search} value={query} onChange={(e) => setQuery(e.target.value)} />
      <Group title={text.everyWindow} anchor="commands">
        {shown.length === 0 && <Row label={text.noMatch(query)} />}
        {shown.map((command) => (
          <ShortcutRow key={command.id} command={command} />
        ))}
      </Group>
      {pageKeys(text).map((group) => (
        <Group key={group.anchor} title={group.page} anchor={group.anchor}>
          {group.keys.map((k) => (
            <Row key={k.label} label={k.label}>
              <span className="kbd">{k.keys}</span>
            </Row>
          ))}
        </Group>
      ))}
    </>
  );
}
