import type { FileCheck, FileProblem, StatusGroup, StatusRow } from "../api/types.gen";

export const fileProblem = (patch: Partial<FileProblem> = {}): FileProblem => ({
  trouble: "missing",
  file: "docs/2026/2026-11-09 Demo Cloud Ltd.pdf",
  what: "Document · Demo Cloud Ltd invoice",
  ...patch,
});

export const fileCheck = (patch: Partial<FileCheck> = {}): FileCheck => ({
  checked: 12,
  problems: [],
  at: new Date(2026, 10, 13, 9, 30).getTime(),
  ...patch,
});

export const statusRow = (patch: Partial<StatusRow> = {}): StatusRow => ({
  label: "Private",
  detail: "Balances chain from statement to statement",
  value: "OK",
  tone: "ok",
  copyText: null,
  ...patch,
});

export const statusGroup = (patch: Partial<StatusGroup> = {}): StatusGroup => ({
  id: "statements",
  title: "Statements",
  rows: [statusRow()],
  ...patch,
});
