import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError, AppInfo, CommandInfo, ErrorKind, EventName, EventPayloads, LaunchEvent, Settings, SettingsPatch } from "./types";

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
