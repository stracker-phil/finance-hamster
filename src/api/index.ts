import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Account, AccountEdit, AccountLines, Action, Coverage, ReviewQueue, RuleForm, RulesView, Frame, ImportPreview, PdfSource, AppError, AppInfo, CommandInfo, ErrorKind, EventName, EventPayloads, Imported, LaunchEvent, NewAccount, ReloadPreview, RulesFile, Settings, SettingsPatch, TaxYear } from "./types";

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
  rulesOverview: () => invoke<RulesFile[]>("rules_overview"),
  rulesReloadPreview: () => invoke<ReloadPreview[]>("rules_reload_preview"),
  /** The label of the applied change, or null when no file had changed. */
  rulesReloadApply: () => invoke<string | null>("rules_reload_apply"),
  /** Null when the file panel was cancelled. */
  rulesImport: () => invoke<Imported | null>("rules_import"),
  /** The new draft's file name. */
  rulesDuplicate: () => invoke<string>("rules_duplicate"),
  rulesOpen: (year: number) => invoke<void>("rules_open", { year }),
  rulesShowFolder: () => invoke<void>("rules_show_folder"),
  /** Asks for a statement PDF and reads it; null when the panel was cancelled. */
  statementImportChoose: () => invoke<ImportPreview | null>("statement_import_choose"),
  /** The label of the import. */
  statementImportCommit: (token: string) => invoke<string>("statement_import_commit", { token }),
  statementImportCancel: (token: string) => invoke<void>("statement_import_cancel", { token }),
  statementsCoverage: () => invoke<Coverage>("statements_coverage"),
  accountLines: (accountId: string) => invoke<AccountLines>("account_lines", { accountId }),
  rulesView: () => invoke<RulesView>("rules_view"),
  /** Adds a rule (`id` null) or changes one; the label of the change. */
  ruleSave: (id: string | null, form: RuleForm) => invoke<string>("rule_save", { id, form }),
  ruleDelete: (id: string) => invoke<string>("rule_delete", { id }),
  /** How many lines changed, in words. */
  rulesRerun: () => invoke<string>("rules_rerun"),
  reviewQueue: () => invoke<ReviewQueue>("review_queue"),
  /** Decides a line by hand; `always` also adds a rule for its key. The label of the change. */
  reviewDecide: (lineId: string, action: Action, itemKey: string | null, always: boolean) => invoke<string>("review_decide", { lineId, action, itemKey, always }),
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
