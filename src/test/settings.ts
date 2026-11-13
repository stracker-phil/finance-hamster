import type { Settings } from "../api";

/** The settings a fresh install has, with `patch` applied. */
export function settings(patch: Partial<Settings> = {}): Settings {
  return { appearance: "system", accent: "system", numberFormat: "dot_decimal", showOriginalCurrency: true, ...patch };
}
