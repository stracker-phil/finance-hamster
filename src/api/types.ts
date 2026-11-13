// Re-exports the IPC types generated from Rust (`bun run types`); everything defined below is frontend-only.
export type * from "./types.gen";

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
}

export type EventName = keyof EventPayloads;
