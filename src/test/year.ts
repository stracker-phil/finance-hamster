import type { AttentionView, CheckQuestion, Elsewhere, KennzahlView, MonthView, QuestionView, YearCheckView, YearPage, YearRow } from "../api";

/** A demo document row of the tax year page, with `patch` applied. */
export function yearRow(patch: Partial<YearRow> = {}): YearRow {
  return {
    id: "doc-1",
    kind: "document",
    title: "Demo Books Ltd",
    detail: "10 Nov · 60 % of €22.48",
    amount: "€13.49",
    draft: false,
    carriedFrom: null,
    kz: "720",
    attach: false,
    ...patch,
  };
}

/** A demo Kennzahl section with one row, with `patch` applied. */
export function kennzahlView(patch: Partial<KennzahlView> = {}): KennzahlView {
  return { key: "720", kz: "720", name: "Fachliteratur", copy: "1228,40", rows: [yearRow()], note: null, add: null, rate: null, reported: false, ...patch };
}

/** A demo question of an open year, with `patch` applied. */
export function questionView(patch: Partial<QuestionView> = {}): QuestionView {
  return { id: "commute", label: "Days worked from home", help: "Count whole days", unit: "days", value: "", answered: false, ...patch };
}

/** A demo attention entry, with `patch` applied. */
export function attentionView(patch: Partial<AttentionView> = {}): AttentionView {
  return { kind: "draft", title: "Example Market", detail: "Still a draft", documentId: "doc-2", entryId: null, ...patch };
}

/** A demo month section, with `patch` applied. */
export function monthView(patch: Partial<MonthView> = {}): MonthView {
  return { label: "November", total: "€13.49", rows: [yearRow()], ...patch };
}

/** A demo tax year page, with `patch` applied. */
export function yearPage(patch: Partial<YearPage> = {}): YearPage {
  return {
    year: 2026,
    ruleset: "2026 v1",
    state: "open",
    total: "€1,228.40",
    progress: "Draft · 1 invoice missing",
    progressPercent: 60,
    pauschaleNote: null,
    questions: [],
    attention: [],
    kennzahlen: [kennzahlView()],
    months: [monthView()],
    archived: 0,
    ...patch,
  };
}

/** A demo year check question that asks for a document, with `patch` applied. */
export function checkQuestion(patch: Partial<CheckQuestion> = {}): CheckQuestion {
  return { key: "books", kz: "720", question: "Did you buy work books?", hint: null, add: "document", unit: null, value: "", ...patch };
}

/** A demo year check of an open year with no questions, with `patch` applied. */
export function yearCheckView(patch: Partial<YearCheckView> = {}): YearCheckView {
  return { year: 2026, open: true, questions: [], lastRun: null, yearOver: false, line: "Year check: not run yet", closePrompt: null, ...patch };
}

/** A demo search match, with `patch` applied. */
export function elsewhere(patch: Partial<Elsewhere> = {}): Elsewhere {
  return { label: "Pixelforge Ltd. invoice", place: "Tax year 2026", target: { page: "document", documentId: "doc-9", year: 2026 }, ...patch };
}
