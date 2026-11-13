import type { Settings } from "../api";

/** The settings a fresh install has, with `patch` applied. */
export function settings(patch: Partial<Settings> = {}): Settings {
  return { appearance: "system", accent: "system", numberFormat: "dot_decimal", showOriginalCurrency: true, hideOwnTransfers: true, archiveDays: 7,
    defaultReminder: "three_days",
    remindPriceChanges: true,
    importAfterDays: 30,
    remindGaps: true,
    remindFiling: true,
    remindQuestions: true,
    ...patch,
  };
}
