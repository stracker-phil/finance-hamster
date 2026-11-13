//! Every event the backend emits to the page, with its payload type.

/// No payload; the page then calls `take_launch_events`.
pub const LAUNCH_EVENT: &str = "launch";
/// No payload; the hidden main window was shown again.
pub const WINDOW_OPENED_EVENT: &str = "window_opened";
/// Payload: the command id; the main window carries out a page command (`commands::run`).
pub const RUN_COMMAND_EVENT: &str = "run_command";
/// Payload: the Settings page or row id to show.
pub const OPEN_SETTINGS_EVENT: &str = "open_settings";
/// Payload: `db::commands::DataChanged`, the tables a write touched.
pub const DATA_CHANGED_EVENT: &str = "data_changed";
/// No payload; `settings.json` changed and every window reads it again.
pub const SETTINGS_CHANGED_EVENT: &str = "settings_changed";

/// A dropped statement was read (or refused); payload `ImportReady`.
pub const IMPORT_READY_EVENT: &str = "import_ready";
