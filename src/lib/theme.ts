import { useEffect, useState } from "react";
import type { Accent, Appearance } from "../api";

/** The palette's scheme (`styles/palettes/<scheme>.css`). */
export type Scheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

const systemDark = () => window.matchMedia?.(DARK_QUERY).matches ?? false;

/** Automatic follows macOS; Light and Dark are fixed. */
export function schemeFor(appearance: Appearance, darkSystem: boolean): Scheme {
  if (appearance === "system") return darkSystem ? "dark" : "light";
  return appearance;
}

/** Changes the look at once. WebKit leaves a color with a transition stale when only an inherited custom property
 *  changes, so transitions pause while the window restyles. */
function setLook(key: "scheme" | "accent", value: string) {
  const root = document.documentElement;
  if (root.dataset[key] === value) return;
  root.classList.add("restyling");
  root.dataset[key] = value;
  void root.offsetHeight;
  requestAnimationFrame(() => root.classList.remove("restyling"));
}

/** Sets `data-scheme` on <html> before the first render, so the first paint has colors. */
export function applyInitialLook() {
  setLook("scheme", systemDark() ? "dark" : "light");
  setLook("accent", "system");
}

/** Keeps the window's scheme in step with the Appearance setting (on Automatic, with macOS) and its accent. */
export function useAppearance(appearance: Appearance = "system", accent: Accent = "system") {
  const [dark, setDark] = useState(systemDark);
  useEffect(() => {
    const media = window.matchMedia?.(DARK_QUERY);
    if (!media) return;
    const follow = () => setDark(media.matches);
    media.addEventListener("change", follow);
    return () => media.removeEventListener("change", follow);
  }, []);
  useEffect(() => setLook("scheme", schemeFor(appearance, dark)), [appearance, dark]);
  useEffect(() => setLook("accent", accent), [accent]);
}
