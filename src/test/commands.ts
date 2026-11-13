import type { CommandInfo } from "../api";

/** A few bus commands as `listCommands` returns them. */
export const COMMAND_LIST: CommandInfo[] = [
  { id: "go.today", title: "Go to Today", shortcut: "⌘1", custom: false },
  { id: "go.subscriptions", title: "Go to Subscriptions", shortcut: "⌘2", custom: false },
  { id: "settings.open", title: "Open Settings", shortcut: "⌘,", custom: false },
  { id: "palette.open", title: "Command Palette", shortcut: "⌘K", custom: false },
  { id: "import.statement", title: "Import Statement…", shortcut: null, custom: false },
];
