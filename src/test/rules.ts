import type { RulesField, RulesGroup, RulesOverview, RulesYear, RulesYearView } from "../api";

/** An open year 2026 on the built-in rules, with `patch` applied. */
export function rulesYear(patch: Partial<RulesYear> = {}): RulesYear {
  return {
    year: 2026,
    state: "open",
    source: "builtin",
    ruleset: "2026 v1",
    basedOn: null,
    builtinNewer: null,
    problems: [],
    hasBuiltin: true,
    unverified: 0,
    ...patch,
  };
}

/** The overview of `years` (newest first), with `patch` applied. */
export function rulesOverview(years: RulesYear[] = [rulesYear()], patch: Partial<RulesOverview> = {}): RulesOverview {
  return { years, nextYear: null, nextIsBuiltin: false, ...patch };
}

/** A home office limit field with `patch` applied. */
export function rulesField(patch: Partial<RulesField> = {}): RulesField {
  return { path: "homeOffice/limit", label: "Home office limit", value: "300", unit: "EUR", kind: "amount", ...patch };
}

/** The editable built-in rules of 2026 with one group and one field, with `patch` applied. */
export function rulesYearView(patch: Partial<RulesYearView> = {}, fields: RulesField[] = [rulesField()]): RulesYearView {
  const group: RulesGroup = { title: "Home office", fields, facts: [] };
  return { year: 2026, editable: true, source: "builtin", ruleset: "2026 v1", basedOn: null, groups: [group], notes: null, sources: [], ...patch };
}
