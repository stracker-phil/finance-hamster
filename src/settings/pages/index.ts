import { useLanguage, t } from "../../i18n";
import { de } from "../../i18n/de";
import { en, type Messages } from "../../i18n/en";
import type { SettingsPage, SettingsRow } from "../registry";
import { AboutPage } from "./AboutPage";
import { AccountsPage } from "./AccountsPage";
import { AppearancePage } from "./AppearancePage";
import { PrivacyPage } from "./PrivacyPage";
import { RemindersPage } from "./RemindersPage";
import { RulesPage } from "./RulesPage";
import { ShortcutsPage } from "./ShortcutsPage";
import { StatusPage } from "./StatusPage";
import { TaxRulesPage } from "./TaxRulesPage";
import { VendorsPage } from "./VendorsPage";

type Settings = Messages["settings"];
type Pick = (s: Settings) => string;

/** The language the window isn't in, so its titles still find a row. */
const other = (): Settings => (useLanguage.getState().language === "de" ? en : de).settings;

/** A row titled in the current language; the search also finds it by its title in the other one. */
function row(id: string, title: Pick, keywords: string[] = []): SettingsRow {
  return {
    id,
    get title() {
      return title(t().settings);
    },
    get keywords() {
      return [...keywords, title(other())];
    },
  };
}

function page(fields: Omit<SettingsPage, "label" | "group" | "intro" | "aliases">, texts: { group: Pick; label: Pick; intro?: Pick }): SettingsPage {
  return {
    ...fields,
    get group() {
      return texts.group(t().settings);
    },
    get label() {
      return texts.label(t().settings);
    },
    get intro() {
      return texts.intro?.(t().settings);
    },
    get aliases() {
      return [texts.label(other()), texts.group(other())];
    },
  };
}

const finance: Pick = (s) => s.window.groups.finance;
const app: Pick = (s) => s.window.groups.app;

/** Every Settings page in nav order (mockup Settings); texts follow the language (ADR 0050). */
export const SETTINGS_PAGES: SettingsPage[] = [
  page(
    {
      id: "rules",
      icon: "rules",
      Body: RulesPage,
      rows: [row("add-rule", (s) => s.rules.newRule, ["ignore", "assign", "item", "bank text", "banktext", "ignorieren"]), row("rerun", (s) => s.rules.rerun, ["re-run", "rerun"])],
    },
    { group: finance, label: (s) => s.rules.label, intro: (s) => s.rules.intro },
  ),
  page(
    {
      id: "vendors",
      icon: "vendors",
      Body: VendorsPage,
      rows: [row("vendor-list", (s) => s.vendors.list, ["rename", "merge", "default item", "umbenennen", "zusammenführen"]), row("vendor-rules", (s) => s.vendors.rules, ["delete", "löschen"])],
    },
    { group: finance, label: (s) => s.vendors.label, intro: (s) => s.vendors.intro },
  ),
  page(
    {
      id: "accounts",
      icon: "accounts",
      Body: AccountsPage,
      rows: [
        row("bank-accounts", (s) => s.accounts.bankAccounts, ["iban", "bank"]),
        row("own-transfers", (s) => s.accounts.ownTransfers, ["transfer", "umbuchung"]),
        row("add-account", (s) => s.accounts.add, ["new", "bank", "neu"]),
      ],
    },
    { group: finance, label: (s) => s.accounts.label },
  ),
  page(
    {
      id: "taxrules",
      icon: "taxRules",
      Body: TaxRulesPage,
      rows: [
        row("rulesets", (s) => s.taxRules.rulesets, ["json", "kennzahl", "draft", "open", "closed", "entwurf"]),
        row("reload", (s) => s.taxRules.reload, ["rules", "regeln"]),
        row("import", (s) => s.taxRules.import, ["rules", "regeln"]),
        row("duplicate", (s) => s.taxRules.duplicate, ["duplicate", "rules", "duplizieren"]),
        row("rules-folder", (s) => s.taxRules.folder, ["finder"]),
      ],
    },
    { group: finance, label: (s) => s.taxRules.label, intro: (s) => s.taxRules.intro },
  ),
  page(
    {
      id: "reminders",
      icon: "reminders",
      Body: RemindersPage,
      rows: [
        row("default-reminder", (s) => s.reminders.defaultReminder, ["renewal", "notification", "mitteilung"]),
        row("price-changes", (s) => s.reminders.priceChanges, ["plan", "notification", "preis"]),
        row("import-after", (s) => s.reminders.importAfter, ["statement", "overdue", "kontoauszug"]),
        row("gaps", (s) => s.reminders.gaps, ["statement", "missing", "lücke"]),
        row("filing", (s) => s.reminders.filing, ["tax year", "march", "close", "steuerjahr", "märz"]),
        row("year-check", (s) => s.reminders.yearCheck, ["november", "questions", "year check", "fragen"]),
      ],
    },
    { group: app, label: (s) => s.reminders.label, intro: (s) => s.reminders.intro },
  ),
  page(
    {
      id: "privacy",
      icon: "privacy",
      Body: PrivacyPage,
      rows: [
        row("data-folder", (s) => s.privacy.dataFolder, ["location", "finder", "ort"]),
        row("verify-files", (s) => s.privacy.verify, ["checksum", "integrity", "missing", "prüfsumme"]),
        row("archive", (s) => s.privacy.archiveDays, ["delete", "exclude", "archive", "löschen"]),
        row("deletion-log", (s) => s.privacy.deletionLog, ["deleted", "delete", "gelöscht"]),
        row("backup", (s) => s.privacy.timeMachine, ["backup"]),
        row("network", (s) => s.privacy.internet, ["network", "offline", "netzwerk"]),
      ],
    },
    { group: app, label: (s) => s.privacy.label, intro: (s) => s.privacy.intro },
  ),
  page(
    {
      id: "appearance",
      icon: "appearance",
      Body: AppearancePage,
      rows: [
        row("language", (s) => s.appearance.language, ["sprache", "deutsch", "german", "english", "englisch"]),
        row("mode", (s) => s.appearance.mode, ["dark", "light", "automatic", "theme", "dunkel", "hell"]),
        row("accent", (s) => s.appearance.accent, ["color", "farbe"]),
        row("number-format", (s) => s.appearance.numberFormat, ["decimal", "comma", "komma"]),
        row("original-currency", (s) => s.appearance.originalCurrency, ["usd", "currency", "währung"]),
      ],
    },
    { group: app, label: (s) => s.appearance.label },
  ),
  page(
    {
      id: "shortcuts",
      icon: "shortcuts",
      Body: ShortcutsPage,
      rows: [
        row("commands", (s) => s.shortcuts.everyWindow, ["commands", "keys", "rebind", "keyboard", "reassign", "befehle", "tasten"]),
        row("review-keys", (s) => s.shortcuts.reviewPage, ["review keys", "anv", "skip"]),
        row("year-check-keys", (s) => s.shortcuts.yearCheckPage, ["yes", "no", "later", "ja", "nein", "später"]),
      ],
    },
    { group: app, label: (s) => s.shortcuts.label, intro: (s) => s.shortcuts.intro },
  ),
  page(
    {
      id: "status",
      icon: "status",
      Body: StatusPage,
      rows: [
        row("status-statements", (s) => s.status.statements, ["balance", "gap", "saldo", "lücke"]),
        row("status-files", (s) => s.status.files, ["checksum", "journal", "inbox"]),
        row("status-rules", (s) => s.status.rulesFiles, ["json", "unverified", "ungeprüft"]),
      ],
    },
    { group: app, label: (s) => s.status.label, intro: (s) => s.status.intro },
  ),
  page(
    {
      id: "about",
      icon: "about",
      Body: AboutPage,
      rows: [
        row("version", (s) => s.about.version),
        row("about-data-folder", (s) => s.about.dataFolder, ["finder", "show"]),
        row("about-log-folder", (s) => s.about.logFolder, ["logs", "diagnostics"]),
      ],
    },
    { group: app, label: (s) => s.about.label },
  ),
];
