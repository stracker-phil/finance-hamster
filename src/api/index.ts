import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { YearCheckView, Account, AccountEdit, AccountLines, Action, Elsewhere, YearPage, NewEntry, TodayView, RulePreview, FileCheck, StatusGroup, CloseCheck, SubscriptionsView, SubscriptionDetail, SubscriptionForm, PlanForm, MatchCount, AddInput, AddPreview, ArchivedDocument, DeletionView, DocumentEdit, DocumentGroup, DocumentStep, DocumentView, HistoryEntry, VendorChange, VendorsView, Estimate, LineChoice, Coverage, ReviewQueue, RuleForm, RulesView, Frame, ImportPreview, PdfSource, AppError, AppInfo, CommandInfo, KeyConflict, ErrorKind, EventName, EventPayloads, JsonPreview, LaunchEvent, NewAccount, RulesOverview, RulesYearView, Settings, SettingsPatch, TaxYear } from "./types";

export * from "./types";

/** The app's IPC entry point (ADR 0002). Tests mock this module. */
export const api = {
  takeLaunchEvents: () => invoke<LaunchEvent[]>("take_launch_events"),
  /** Writes a page error (a crash, an e2e result) to the app's log. */
  logFrontendError: (message: string) => invoke<void>("log_frontend_error", { message }),
  appInfo: async (): Promise<AppInfo> => {
    const [name, version] = await Promise.all([getName(), getVersion()]);
    return { name, version };
  },
  listCommands: () => invoke<CommandInfo[]>("list_commands"),
  runCommand: (id: string) => invoke<void>("run_command", { id }),
  /** Gives a command its own shortcut (`CmdOrCtrl+Shift+K`), or none with null. */
  keybindingSet: (id: string, accelerator: string | null) => invoke<void>("keybinding_set", { id, accelerator }),
  keybindingReset: (id: string) => invoke<void>("keybinding_reset", { id }),
  keybindingConflict: (id: string, accelerator: string) => invoke<KeyConflict | null>("keybinding_conflict", { id, accelerator }),
  keybindingReassign: (id: string, accelerator: string) => invoke<void>("keybinding_reassign", { id, accelerator }),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (patch: SettingsPatch) => invoke<Settings>("update_settings", { patch }),
  showDataFolder: () => invoke<void>("show_data_folder"),
  dataFolder: () => invoke<string>("data_folder"),
  /** The label of what was undone, or null when there was nothing. */
  undo: () => invoke<string | null>("undo"),
  redo: () => invoke<string | null>("redo"),
  listAccounts: () => invoke<Account[]>("list_accounts"),
  addAccount: (account: NewAccount) => invoke<Account>("add_account", { account }),
  editAccount: (id: string, account: AccountEdit) => invoke<void>("edit_account", { id, account }),
  listTaxYears: () => invoke<TaxYear[]>("list_tax_years"),
  rulesOverview: () => invoke<RulesOverview>("rules_overview"),
  rulesYear: (year: number) => invoke<RulesYearView>("rules_year", { year }),
  /** Each returns the label of its change. */
  rulesSetValue: (year: number, path: string, value: string) => invoke<string>("rules_set_value", { year, path, value }),
  rulesJson: (year: number) => invoke<string>("rules_json", { year }),
  rulesPreviewJson: (year: number, text: string) => invoke<JsonPreview>("rules_preview_json", { year, text }),
  rulesSetJson: (year: number, text: string) => invoke<string>("rules_set_json", { year, text }),
  rulesUseBuiltin: (year: number) => invoke<string>("rules_use_builtin", { year }),
  rulesStartNext: () => invoke<string>("rules_start_next"),
  /** What happened to the rules while the app started; empty after the first call. */
  rulesTakeNotices: () => invoke<string[]>("rules_take_notices"),
  /** Asks for a statement PDF and reads it; null when the panel was cancelled. */
  statementImportChoose: () => invoke<ImportPreview | null>("statement_import_choose"),
  /** The label of the import. */
  statementImportCommit: (token: string) => invoke<string>("statement_import_commit", { token }),
  statementImportCancel: (token: string) => invoke<void>("statement_import_cancel", { token }),
  /** `query` keeps the accounts whose name contains it. */
  statementsCoverage: (query: string) => invoke<Coverage>("statements_coverage", { query }),
  /** `query` keeps the lines that contain it. */
  accountLines: (accountId: string, query: string) => invoke<AccountLines>("account_lines", { accountId, query }),
  /** `query` filters the rows. */
  yearPage: (year: number, query: string) => invoke<YearPage>("year_page", { year, query }),
  /** The label of the change, or null when nothing changed. */
  yearAnswer: (year: number, question: string, value: string) => invoke<string | null>("year_answer", { year, question, value }),
  /** Adds an amount or a trip; the label of the change. */
  yearEntryAdd: (year: number, entry: NewEntry) => invoke<string>("year_entry_add", { year, entry }),
  yearEntryDelete: (year: number, id: string) => invoke<string>("year_entry_delete", { year, id }),
  /** Attaches the PDF at `path` to an amount; the label of the change. */
  yearEntryAttach: (year: number, id: string, path: string) => invoke<string>("year_entry_attach", { year, id, path }),
  /** Asks for an evidence PDF; its full path, or null when cancelled. */
  yearEvidenceChoose: () => invoke<string | null>("year_evidence_choose"),
  yearEvidenceOpen: (year: number, id: string) => invoke<void>("year_evidence_open", { year, id }),
  yearCheckView: (year: number) => invoke<YearCheckView>("year_check_view", { year }),
  yearCheckComplete: (year: number) => invoke<YearCheckView>("year_check_complete", { year }),
  /** What blocks closing the year, and what only warns. */
  yearCloseCheck: (year: number) => invoke<CloseCheck>("year_close_check", { year }),
  /** Closes the year with its snapshot and summary PDF; the label of the change. */
  yearClose: (year: number) => invoke<string>("year_close", { year }),
  yearReopen: (year: number) => invoke<string>("year_reopen", { year }),
  yearOpenSnapshot: (year: number) => invoke<void>("year_open_snapshot", { year }),
  /** Asks where to save the summary PDF; false when cancelled. */
  yearExportSummary: (year: number) => invoke<boolean>("year_export_summary", { year }),
  /** Where else the filter text turns up. */
  searchElsewhere: (query: string) => invoke<Elsewhere[]>("search_elsewhere", { query }),
  rulesView: () => invoke<RulesView>("rules_view"),
  /** Adds a rule (`id` null) or changes one; the label of the change. */
  ruleSave: (id: string | null, form: RuleForm) => invoke<string>("rule_save", { id, form }),
  ruleDelete: (id: string) => invoke<string>("rule_delete", { id }),
  /** What the sheet's rule would match before it is saved. */
  rulePreview: (form: RuleForm) => invoke<RulePreview>("rule_preview", { form }),
  /** How many lines changed, in words. */
  rulesRerun: () => invoke<string>("rules_rerun"),
  reviewQueue: () => invoke<ReviewQueue>("review_queue"),
  /** Asks for an invoice PDF and reads it; null when the panel was cancelled. */
  documentAddChoose: () => invoke<AddPreview | null>("document_add_choose"),
  /** The label of the change. */
  documentAddCommit: (input: AddInput) => invoke<string>("document_add_commit", { input }),
  documentAddCancel: (token: string) => invoke<void>("document_add_cancel", { token }),
  documentLineChoice: (id: string) => invoke<LineChoice | null>("document_line_choice", { id }),
  /** What a bank line counts as with an item; null without a line. */
  documentView: (id: string) => invoke<DocumentView>("document_view", { id }),
  /** Changes one field of a draft; the label of the change, or null when nothing changed. */
  documentEdit: (id: string, change: DocumentEdit) => invoke<string | null>("document_edit", { id, change }),
  documentLineOptions: (id: string, query: string) => invoke<LineChoice[]>("document_line_options", { id, query }),
  /** Finish, Unlock, Exclude or Restore; the label of the change. */
  documentStep: (id: string, step: DocumentStep) => invoke<string>("document_step", { id, step }),
  /** Deletes an excluded document for good; the label of the change. */
  documentDelete: (id: string, reason: string, understood: boolean) => invoke<string>("document_delete", { id, reason, understood }),
  documentHistory: (id: string) => invoke<HistoryEntry[]>("document_history", { id }),
  yearLog: (year: number, query: string) => invoke<HistoryEntry[]>("year_log", { year, query }),
  archiveList: (year: number) => invoke<ArchivedDocument[]>("archive_list", { year }),
  deletionLog: () => invoke<DeletionView[]>("deletion_log"),
  documentsList: (query: string) => invoke<DocumentGroup[]>("documents_list", { query }),
  vendorsView: () => invoke<VendorsView>("vendors_view"),
  /** The label of the change. */
  vendorChange: (id: string, change: VendorChange) => invoke<string>("vendor_change", { id, change }),
  documentEstimate: (lineId: string | null, itemKey: string | null) => invoke<Estimate | null>("document_estimate", { lineId, itemKey }),
  /** Decides a line by hand; `always` also adds a rule matching `term` (text of the line; null: its key). The
   *  label of the change. `subscriptionId` links a Subscription decision. */
  reviewDecide: (lineId: string, action: Action, itemKey: string | null, subscriptionId: string | null, always: boolean, term: string | null = null) =>
    invoke<string>("review_decide", { lineId, action, itemKey, subscriptionId, always, term }),
  todayView: () => invoke<TodayView>("today_view"),
  /** Checks every stored PDF now. */
  filesVerify: () => invoke<FileCheck>("files_verify"),
  /** The last file check since launch; null before the first. */
  filesLastCheck: () => invoke<FileCheck | null>("files_last_check"),
  statusView: () => invoke<StatusGroup[]>("status_view"),
  /** Opens Settings at a page or row id. */
  /** Whether Time Machine backs up the data folder; null when it can't say. */
  backupIncluded: () => invoke<boolean | null>("backup_included"),
  settingsShow: (page: string) => invoke<void>("settings_show", { page }),
  /** `query` keeps the subscriptions whose name or category contains it. */
  subscriptionsView: (query: string) => invoke<SubscriptionsView>("subscriptions_view", { query }),
  subscriptionDetail: (id: string) => invoke<SubscriptionDetail>("subscription_detail", { id }),
  /** Adds a subscription (`id` null) or fixes one; the label of the change. */
  subscriptionSave: (id: string | null, form: SubscriptionForm) => invoke<string>("subscription_save", { id, form }),
  subscriptionChangePlan: (id: string, form: PlanForm) => invoke<string>("subscription_change_plan", { id, form }),
  /** Ends a subscription on `endedOn`; null resumes it. */
  subscriptionSetEnded: (id: string, endedOn: string | null) => invoke<string>("subscription_set_ended", { id, endedOn }),
  subscriptionDelete: (id: string) => invoke<string>("subscription_delete", { id }),
  subscriptionMatchCount: (form: SubscriptionForm) => invoke<MatchCount>("subscription_match_count", { form }),
  /** A New sheet's form prefilled from a bank line. */
  /** A New Subscription form from a line; `term` (text of the line) becomes its match text. */
  subscriptionPrefill: (lineId: string, term: string | null = null) => invoke<SubscriptionForm>("subscription_prefill", { lineId, term }),
  pdfViewShow: (source: PdfSource, frame: Frame) => invoke<void>("pdf_view_show", { source, frame }),
  pdfViewMove: (frame: Frame) => invoke<void>("pdf_view_move", { frame }),
  pdfViewHide: () => invoke<void>("pdf_view_hide"),
  pdfViewSuspend: () => invoke<void>("pdf_view_suspend"),
  pdfViewResume: () => invoke<void>("pdf_view_resume"),
  /** A page (1-based) of a known PDF as an image URL, served by Rust's `pdf-page` scheme. */
  pdfPageUrl: (source: PdfSource, page: number, width: number) => `pdf-page://localhost/${source.kind}/${encodeURIComponent(source.id)}/${page}.png?w=${Math.round(width)}`,
  listen: <E extends EventName>(event: E, handler: (payload: EventPayloads[E]) => void): Promise<UnlistenFn> =>
    listen<EventPayloads[E]>(event, (e) => handler(e.payload)),
};

/** Every `ErrorKind` Rust sends; the `Record` makes the compiler check the list is complete. */
const ERROR_KINDS: Record<ErrorKind, true> = {
  io: true,
  invalid_input: true,
  not_found: true,
  internal: true,
};

export function toAppError(e: unknown): AppError {
  if (typeof e !== "object" || e === null || !("message" in e)) return { kind: "io", message: String(e) };
  const message = String(e.message);
  if ("kind" in e && Object.hasOwn(ERROR_KINDS, String(e.kind))) return { ...e, message } as AppError;
  return { kind: "io", message };
}
