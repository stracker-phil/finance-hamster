// Re-exports the IPC types generated from Rust (`bun run types`); everything defined below is frontend-only.
export type * from "./types.gen";
import type { AddReady, DataChanged, ImportReady } from "./types.gen";

/** The app as macOS knows it (the bundle's name and version). */
export interface AppInfo {
  name: string;
  version: string;
}

/** Every event Rust emits (`src-tauri/src/events.rs`) with its payload. */
export interface EventPayloads {
  /** Launch requests are waiting; take them with `takeLaunchEvents`. */
  launch: null;
  /** The hidden main window was shown again. */
  window_opened: null;
  /** The main window carries out this page command (menu, shortcut, palette, CLI). */
  run_command: string;
  /** The Settings window shows this page or row. */
  open_settings: string;
  /** A write changed these tables. */
  data_changed: DataChanged;
  /** `settings.json` changed. */
  settings_changed: null;
  /** A profile was renamed. */
  profiles_changed: null;
  /** A dropped statement was read, or refused. */
  import_ready: ImportReady;
  add_ready: AddReady;
  /** A check of the stored PDFs finished. */
  files_checked: null;
  /** A shortcut changed; the commands list reads it again. */
  commands_changed: null;
}

export type EventName = keyof EventPayloads;
