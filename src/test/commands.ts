import type { CommandInfo } from "../api";

/** A few bus commands as `listCommands` returns them. */
export const COMMAND_LIST: CommandInfo[] = [
  { id: "go.today", title: "Go to Today", shortcut: "⌘1" },
  { id: "go.subscriptions", title: "Go to Subscriptions", shortcut: "⌘2" },
  { id: "settings.open", title: "Open Settings", shortcut: "⌘," },
  { id: "palette.open", title: "Command Palette", shortcut: "⌘K" },
  { id: "import.statement", title: "Import Statement…", shortcut: null },
];
