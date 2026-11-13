import type { SettingsPage } from "../registry";
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

/** Every Settings page in nav order (mockup Settings). */
export const SETTINGS_PAGES: SettingsPage[] = [
  {
    id: "rules",
    group: "Finance",
    label: "Rules",
    icon: "rules",
    intro: "Rules run on every import, top to bottom. Most are created right in Review with “Always do this”. Your own choice on a line always wins.",
    Body: RulesPage,
    rows: [
      { id: "add-rule", title: "New rule", keywords: ["ignore", "assign", "item", "bank text"] },
      { id: "rerun", title: "Apply rules again", keywords: ["re-run", "rerun"] },
    ],
  },
  {
    id: "vendors",
    group: "Finance",
    label: "Vendors",
    icon: "vendors",
    intro: "Vendors are suggested while you type on a document. Typing a new name there adds it here.",
    Body: VendorsPage,
    rows: [
      { id: "vendor-list", title: "Vendors", keywords: ["rename", "merge", "default item"] },
      { id: "vendor-rules", title: "Rules for vendors", keywords: ["delete"] },
    ],
  },
  {
    id: "accounts",
    group: "Finance",
    label: "Accounts",
    icon: "accounts",
    Body: AccountsPage,
    rows: [
      { id: "bank-accounts", title: "Bank accounts", keywords: ["iban", "bank"] },
      { id: "own-transfers", title: "Hide transfers between my accounts", keywords: ["transfer"] },
      { id: "add-account", title: "Add Account", keywords: ["new", "bank"] },
    ],
  },
  {
    id: "taxrules",
    group: "Finance",
    label: "Tax Rules",
    icon: "taxRules",
    intro: "The rules for each tax year live in one JSON file. Edit it in any text editor, then reload. Hamster checks the file before using it and never changes a closed year.",
    Body: TaxRulesPage,
    rows: [
      { id: "rulesets", title: "Rulesets", keywords: ["json", "kennzahl", "draft", "open", "closed"] },
      { id: "reload", title: "Reload after editing", keywords: ["rules"] },
      { id: "import", title: "Import a JSON file", keywords: ["rules"] },
      { id: "duplicate", title: "Start next year", keywords: ["duplicate", "rules"] },
      { id: "rules-folder", title: "Rules folder", keywords: ["finder"] },
    ],
  },
  {
    id: "reminders",
    group: "App",
    label: "Reminders",
    icon: "reminders",
    intro: "Reminders are macOS notifications. Nothing is sent anywhere.",
    Body: RemindersPage,
    rows: [
      { id: "default-reminder", title: "Default reminder for new subscriptions", keywords: ["renewal", "notification"] },
      { id: "price-changes", title: "Tell me when a price changes", keywords: ["plan", "notification"] },
      { id: "import-after", title: "Remind me to import after", keywords: ["statement", "overdue"] },
      { id: "gaps", title: "Tell me about gaps", keywords: ["statement", "missing"] },
      { id: "filing", title: "Filing reminder", keywords: ["tax year", "march", "close"] },
      { id: "year-questions", title: "Year questions", keywords: ["november", "questions"] },
    ],
  },
  {
    id: "privacy",
    group: "App",
    label: "Privacy & Data",
    icon: "privacy",
    intro: "Everything stays on this Mac in one folder. Hamster contains no network code, so nothing can leave it.",
    Body: PrivacyPage,
    rows: [
      { id: "data-folder", title: "Data folder", keywords: ["location", "finder"] },
      { id: "verify-files", title: "Verify files", keywords: ["checksum", "integrity", "missing"] },
      { id: "archive", title: "Days in Archive before delete", keywords: ["delete", "exclude", "archive"] },
      { id: "deletion-log", title: "Deletion log", keywords: ["deleted", "delete"] },
      { id: "network", title: "Internet access", keywords: ["network", "offline"] },
    ],
  },
  {
    id: "appearance",
    group: "App",
    label: "Appearance",
    icon: "appearance",
    Body: AppearancePage,
    rows: [
      { id: "mode", title: "Mode", keywords: ["dark", "light", "automatic", "theme"] },
      { id: "accent", title: "Accent", keywords: ["color"] },
      { id: "number-format", title: "Number format", keywords: ["decimal", "comma"] },
      { id: "original-currency", title: "Show foreign currency next to amounts", keywords: ["usd", "currency"] },
    ],
  },
  { id: "shortcuts", group: "App", label: "Shortcuts", icon: "shortcuts", Body: ShortcutsPage, rows: [{ id: "commands", title: "Commands", keywords: ["keys"] }] },
  { id: "status", group: "App", label: "Status", icon: "status", intro: "A quick health check. Everything is computed on this Mac.", Body: StatusPage, rows: [
      { id: "status-statements", title: "Statement chains", keywords: ["balance", "gap"] },
      { id: "status-files", title: "Files and journal", keywords: ["checksum", "journal", "inbox"] },
      { id: "status-rules", title: "Rules files", keywords: ["json", "unverified"] },
    ] },
  { id: "about", group: "App", label: "About", icon: "about", Body: AboutPage, rows: [{ id: "version", title: "Version" }] },
];
