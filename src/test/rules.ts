import type { ReloadPreview, RulesFile } from "../api";

/** An open, valid rules file for 2026, with `patch` applied. */
export function rulesFile(patch: Partial<RulesFile> = {}): RulesFile {
  return {
    fileName: "rules-2026.json",
    year: 2026,
    state: "open",
    ruleset: "2026 v1",
    basedOn: null,
    items: 12,
    unverified: 0,
    problems: [],
    edited: false,
    missing: false,
    closedOn: null,
    loadedOn: null,
    ...patch,
  };
}

/** A pending reload of 2026 from `2026 v1` to `2026 v2`, with `patch` applied. */
export function reloadPreview(patch: Partial<ReloadPreview> = {}): ReloadPreview {
  return { year: 2026, from: "2026 v1", to: "2026 v2", changes: ["Home office limit changed"], problems: [], ...patch };
}
