import type { ItemOption, RuleForm, RuleView, RulesView } from "../api";

/** A demo rule form (ignore, no conditions but one bank text), with `patch` applied. */
export function ruleForm(patch: Partial<RuleForm> = {}): RuleForm {
  return { contains: ["ALPENMARKT"], amount: null, tolerance: null, accountId: null, ownCounterparty: false, action: "ignore", itemKey: null, ...patch };
}

/** A demo rule as Settings → Rules lists it, with `patch` applied. */
export function ruleView(patch: Partial<RuleView> = {}): RuleView {
  return { id: "rule-1", label: "Bank text contains ALPENMARKT", detail: "→ ignore · 3 matches · last 7 Nov", builtin: false, form: ruleForm(), ...patch };
}

export const ITEMS: ItemOption[] = [
  { key: "tools", label: "KZ 169 · Digitale Arbeitsmittel", kz: "169" },
  { key: "books", label: "KZ 720 · Fachliteratur", kz: "720" },
];

/** A demo rules view, with `patch` applied. */
export function rulesView(patch: Partial<RulesView> = {}): RulesView {
  return { groups: [{ title: "Ignore", rules: [ruleView()] }], items: ITEMS, ...patch };
}
