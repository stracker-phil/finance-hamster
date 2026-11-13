import { useEffect, useRef } from "react";

/** Typing in a field never triggers a page key. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

/** Single keys a page answers to while it shows (Review's A, S, I, J): `keys` maps a key to its action. Keys with
 *  ⌘, ⌃ or ⌥ belong to the menu bar; a sheet over the page (`.overlay`) takes the keys first. */
export function usePageKeys(keys: Record<string, () => void>) {
  const current = useRef(keys);
  useEffect(() => {
    current.current = keys;
  });
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey || e.defaultPrevented || isTypingTarget(e.target)) return;
      if (document.querySelector(".overlay")) return;
      const action = current.current[e.key.toLowerCase()] ?? current.current[e.key];
      if (!action) return;
      e.preventDefault();
      action();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
