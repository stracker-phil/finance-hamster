import { useEffect } from "react";
import { isTypingTarget } from "../commands/pageKeys";
import { parentOf } from "../state/route";
import { useStore } from "../state/store";

/** Rows a list page can step through with ↑/↓ or J/K. */
const ROWS = "button.docs-row, button.sub-row, button.today-row, button.cov-row__open, button.year-row";

/** Something over the page owns the keys: a sheet, the palette or a menu. */
const covered = () => !!document.querySelector(".overlay") || !!useStore.getState().menu || useStore.getState().paletteOpen;

/** Esc leaves a page one level down; Esc in a field there leaves the field first. */
function leave(e: KeyboardEvent) {
  if (isTypingTarget(e.target)) {
    e.target.blur();
    return true;
  }
  const { route, back, goBack, navigate } = useStore.getState();
  const parent = parentOf(route);
  if (!parent) return false;
  if (back.length > 0) goBack();
  else navigate(parent);
  return true;
}

/** The `index`th button of the page's first segmented control. */
function segmentButton(index: number) {
  const control = document.querySelector(".panel :is(.seg, .year-switch)");
  return control?.querySelectorAll<HTMLButtonElement>(".seg-btn")[index];
}

/** ⌥1 to ⌥9 pick a segment of the page's first segmented control. */
function pickSegment(e: KeyboardEvent) {
  const digit = /^Digit([1-9])$/.exec(e.code)?.[1];
  if (!digit || isTypingTarget(e.target)) return false;
  const segment = segmentButton(Number(digit) - 1);
  segment?.click();
  return !!segment;
}

/** ↓/J and ↑/K move the focus to the next or previous row of the page's list; the first press takes the first row. */
function stepRow(e: KeyboardEvent) {
  const step = { ArrowDown: 1, j: 1, ArrowUp: -1, k: -1 }[e.key];
  if (!step || isTypingTarget(e.target)) return false;
  const rows = [...document.querySelectorAll<HTMLButtonElement>(`.page :is(${ROWS})`)].filter((r) => !r.disabled);
  if (rows.length === 0) return false;
  const at = rows.indexOf(document.activeElement as HTMLButtonElement);
  const target = at < 0 ? 0 : Math.min(Math.max(at + step, 0), rows.length - 1);
  const next = rows[target];
  next.focus();
  next.scrollIntoView({ block: "nearest" });
  return true;
}

/** The main window's keys outside the menu bar (`docs/ux.md` → Keys); page keys and sheets come first. */
export function useWindowKeys() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.metaKey || e.ctrlKey || covered()) return;
      let handled = false;
      if (e.altKey) handled = !e.shiftKey && pickSegment(e);
      else if (e.key === "Escape") handled = leave(e);
      else if (!e.shiftKey) handled = stepRow(e);
      if (handled) e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
