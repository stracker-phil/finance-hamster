import { Check } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { nextEnabled, rowStartingWith } from "./keys";
import { place, type Placement } from "./placement";
import { isEnabled, isItem, type MenuEntry, type MenuItem } from "./types";

function swallowSpace(e: KeyboardEvent) {
  if (e.key === " ") e.preventDefault();
}

interface Props {
  entries: MenuEntry[];
  label: string;
  placement: Placement;
  /** The row highlighted on opening, -1 for none. */
  initial: number;
  onChoose: (item: MenuItem) => void;
}

/** One menu panel: its rows and the keys that move through them, as a macOS menu. */
export function MenuPanel({ entries, label, placement, initial, onChoose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const enabled = entries.flatMap((entry, index) => (isEnabled(entry) ? [index] : []));
  const [active, setActive] = useState(initial);
  const [position, setPosition] = useState<CSSProperties>({ left: 0, top: 0, visibility: "hidden" });
  // A check column only when a row is a choice, so the texts of a menu line up.
  const checks = entries.some((entry) => isItem(entry) && entry.checked !== undefined);

  useLayoutEffect(() => {
    const panel = ref.current;
    if (!panel) return;
    const size = { width: panel.offsetWidth, height: panel.offsetHeight };
    setPosition(place(placement, size, { width: window.innerWidth, height: window.innerHeight }));
  }, [placement]);

  // The focus follows the highlight; a hidden panel takes no focus, so it focuses again once placed.
  useEffect(() => {
    const row = active >= 0 ? ref.current?.querySelector<HTMLElement>(`[data-index="${active}"]`) : null;
    (row ?? ref.current)?.focus({ preventScroll: true });
  }, [active, position]);

  const choose = (index: number) => {
    const entry = entries[index];
    if (isEnabled(entry)) onChoose(entry);
  };

  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    switch (e.key) {
      case "ArrowDown":
        setActive(nextEnabled(enabled, active, 1));
        break;
      case "ArrowUp":
        setActive(nextEnabled(enabled, active, -1));
        break;
      case "Home":
        if (enabled.length) setActive(enabled[0]);
        break;
      case "End":
        if (enabled.length) setActive(enabled[enabled.length - 1]);
        break;
      case "Enter":
        if (active >= 0) choose(active);
        break;
      case " ":
        // The focus goes back to the opener before the key comes up, and a Space keyup there would press it again.
        window.addEventListener("keyup", swallowSpace, { capture: true, once: true });
        if (active >= 0) choose(active);
        break;
      case "Tab":
        break;
      default: {
        if (e.key.length !== 1 || e.metaKey || e.ctrlKey || e.altKey) return;
        const match = rowStartingWith(entries, enabled, active, e.key);
        if (match !== undefined) setActive(match);
      }
    }
    e.preventDefault();
    e.stopPropagation();
  };

  return (
    <div
      ref={ref}
      className={checks ? "menu menu--floating menu--checks" : "menu menu--floating"}
      role="menu"
      aria-label={label}
      tabIndex={-1}
      style={position}
      onKeyDown={onKeyDown}
      onMouseLeave={() => setActive(-1)}
    >
      {entries.map((entry, index) => {
        if (entry === "separator") return <div key={index} className="menu-sep" role="separator" />;
        if (!isItem(entry))
          return (
            <div key={index} className="menu-caption" aria-hidden="true">
              {entry.caption}
            </div>
          );
        return <MenuRow key={index} item={entry} index={index} active={index === active} onHover={() => isEnabled(entry) && setActive(index)} onChoose={() => choose(index)} />;
      })}
    </div>
  );
}

function MenuRow({ item, index, active, onHover, onChoose }: { item: MenuItem; index: number; active: boolean; onHover: () => void; onChoose: () => void }) {
  const choice = item.checked !== undefined;
  const disabled = item.enabled === false;
  return (
    <button
      type="button"
      role={choice ? "menuitemradio" : "menuitem"}
      tabIndex={-1}
      data-index={index}
      className={item.danger ? "menu-row menu-row--danger" : "menu-row"}
      data-active={active || undefined}
      title={item.title}
      aria-checked={choice ? item.checked : undefined}
      aria-disabled={disabled || undefined}
      onMouseEnter={onHover}
      onClick={onChoose}
    >
      <Check className="menu-row__check" size={13} strokeWidth={2.2} aria-hidden="true" visibility={item.checked ? "visible" : "hidden"} />
      {item.icon && <span className="menu-row__icon">{item.icon}</span>}
      <span className="menu-row__text">{item.text}</span>
      {item.hint && <span className="menu-row__key">{item.hint}</span>}
    </button>
  );
}
