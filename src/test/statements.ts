import type { AccountCoverage, AccountLines, Coverage, ImportPreview, LineView, StatementGroup } from "../api";

/** A demo statement read and waiting in the Import sheet, with `patch` applied. */
export function importPreview(patch: Partial<ImportPreview> = {}): ImportPreview {
  return {
    token: "tok-1",
    fileName: "statement-010-2026.pdf",
    summary: "Kontoauszug Nr. 010/2026 · 4 pages · 1 Nov – 10 Nov 2026",
    accountName: "Private",
    lines: 42,
    balance: "1,412.30 + 2,250.00 − 1,986.44 = 1,675.86",
    chain: "continues",
    chainTitle: "Follows statement Nr. 009/2026",
    chainDetail: "Opening balance matches the previous closing balance.",
    file: "statements/2026/2026-11-10 Private.pdf",
    ...patch,
  };
}

/** A demo account row of the coverage table, with `patch` applied. */
export function accountCoverage(patch: Partial<AccountCoverage> = {}): AccountCoverage {
  return {
    id: "acc-1",
    name: "Private",
    last: "Nr. 010 · until 10 Nov",
    state: "ok",
    stateLabel: "Up to date",
    months: [
      { cover: "full", tip: "Sep: complete" },
      { cover: "part", tip: "Oct: until 20 Oct" },
      { cover: "missing", tip: "Nov: no statement" },
    ],
    note: null,
    ...patch,
  };
}

/** A demo coverage over three months, with `patch` applied. */
export function coverage(patch: Partial<Coverage> = {}): Coverage {
  return {
    months: [
      { label: "Sep", january: false },
      { label: "Oct", january: false },
      { label: "Nov", january: false },
    ],
    accounts: [accountCoverage()],
    ...patch,
  };
}

/** A demo line of a statement, with `patch` applied. */
export function lineView(patch: Partial<LineView> = {}): LineView {
  return { id: "line-1", date: "10 Nov", title: "Example Market", original: null, amount: "−€22.48", credit: false, transfer: false, ...patch };
}

/** A demo statement group, with `patch` applied. */
export function statementGroup(patch: Partial<StatementGroup> = {}): StatementGroup {
  return { id: "st-1", label: "Nr. 010/2026", range: "1 Nov – 10 Nov", count: "1 line", lines: [lineView()], ...patch };
}

/** A demo account's lines, with `patch` applied. */
export function accountLines(patch: Partial<AccountLines> = {}): AccountLines {
  return { id: "acc-1", name: "Private", ibanMasked: "AT•• •••• 0001", groups: [statementGroup()], ...patch };
}
