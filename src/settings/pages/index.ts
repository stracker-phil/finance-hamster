import type { SettingsPage } from "../registry";
import { AboutPage } from "./AboutPage";
import { AccountsPage } from "./AccountsPage";
import { AppearancePage } from "./AppearancePage";
import { PrivacyPage } from "./PrivacyPage";
import { ShortcutsPage } from "./ShortcutsPage";
import { StatusPage } from "./StatusPage";
import { TaxRulesPage } from "./TaxRulesPage";

/** Every Settings page in nav order (mockup Settings). */
export const SETTINGS_PAGES: SettingsPage[] = [
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
    id: "privacy",
    group: "App",
    label: "Privacy & Data",
    icon: "privacy",
    intro: "Everything stays on this Mac in one folder. Hamster contains no network code, so nothing can leave it.",
    Body: PrivacyPage,
    rows: [
      { id: "data-folder", title: "Data folder", keywords: ["location", "finder"] },
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
  { id: "status", group: "App", label: "Status", icon: "status", Body: StatusPage, rows: [{ id: "checks", title: "Checks", keywords: ["integrity"] }] },
  { id: "about", group: "App", label: "About", icon: "about", Body: AboutPage, rows: [{ id: "version", title: "Version" }] },
];
