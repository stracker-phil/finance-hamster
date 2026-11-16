//! The Tauri commands the page may call, in one list.

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::launch::take_launch_events,
        log_frontend_error,
        crate::commands::handlers::list_commands,
        crate::commands::handlers::keybinding_set,
        crate::commands::handlers::keybinding_reset,
        crate::commands::handlers::keybinding_conflict,
        crate::commands::handlers::keybinding_reassign,
        crate::commands::handlers::run_command,
        crate::settings::commands::get_settings,
        crate::settings::commands::update_settings,
        show_data_folder,
        data_folder,
        backup_included,
        crate::db::commands::undo,
        crate::db::commands::redo,
        crate::accounts::commands::list_accounts,
        crate::accounts::commands::add_account,
        crate::accounts::commands::edit_account,
        crate::taxyears::commands::list_tax_years,
        crate::taxrules::commands::rules_overview,
        crate::taxrules::commands::rules_year,
        crate::taxrules::commands::rules_set_value,
        crate::taxrules::commands::rules_json,
        crate::taxrules::commands::rules_preview_json,
        crate::taxrules::commands::rules_set_json,
        crate::taxrules::commands::rules_use_builtin,
        crate::taxrules::commands::rules_start_next,
        crate::taxrules::commands::rules_import_choose,
        crate::taxrules::commands::rules_import,
        crate::payslips::commands::payslips_year,
        crate::payslips::commands::payslips_list,
        crate::others::commands::others_list,
        crate::others::commands::other_view,
        crate::others::commands::other_add_pdf,
        crate::others::commands::other_add_waiting,
        crate::others::commands::other_edit,
        crate::others::commands::other_delete,
        crate::statements::commands::statements_list,
        crate::statements::commands::statement_view,
        crate::payslips::commands::payslip_view,
        crate::payslips::commands::payslip_add,
        crate::payslips::commands::payslip_add_pdf,
        crate::payslips::commands::payslip_attach,
        crate::payslips::commands::payslip_guess,
        crate::payslips::commands::payslip_add_waiting,
        crate::payslips::commands::payslip_edit,
        crate::payslips::commands::payslip_delete,
        crate::taxrules::commands::rules_take_notices,
        crate::preview::commands::pdf_view_show,
        crate::preview::commands::pdf_view_move,
        crate::preview::commands::pdf_view_hide,
        crate::preview::commands::pdf_view_cover,
        crate::preview::commands::pdf_view_visible,
        crate::statements::commands::statement_import_choose,
        crate::statements::commands::statement_import_commit,
        crate::statements::commands::statement_import_cancel,
        crate::statements::commands::statements_coverage,
        crate::statements::commands::account_lines,
        crate::rules::commands::rules_view,
        crate::rules::commands::rule_save,
        crate::rules::commands::rule_preview,
        crate::rules::commands::rule_delete,
        crate::rules::commands::rules_rerun,
        crate::review::commands::review_queue,
        crate::review::commands::review_decide,
        crate::documents::commands::document_add_choose,
        crate::documents::commands::document_add_commit,
        crate::documents::commands::document_add_cancel,
        crate::documents::commands::document_line_choice,
        crate::documents::commands::document_estimate,
        crate::documents::commands::document_view,
        crate::documents::commands::document_edit,
        crate::documents::commands::document_line_options,
        crate::documents::commands::documents_list,
        crate::documents::commands::vendors_view,
        crate::documents::commands::vendor_change,
        crate::documents::commands::document_step,
        crate::documents::commands::document_delete,
        crate::documents::commands::document_history,
        crate::documents::commands::year_log,
        crate::documents::commands::archive_list,
        crate::documents::commands::deletion_log,
        crate::year::commands::year_page,
        crate::year::commands::year_answer,
        crate::year::commands::year_categories,
        crate::year::commands::year_set_category_shown,
        crate::year::commands::year_entry_add,
        crate::year::commands::year_close_check,
        crate::year::commands::year_close,
        crate::year::commands::year_reopen,
        crate::year::commands::year_open_snapshot,
        crate::year::commands::year_export_summary,
        crate::subscriptions::commands::subscriptions_view,
        crate::today::commands::today_view,
        crate::integrity::commands::files_verify,
        crate::integrity::commands::files_last_check,
        crate::integrity::commands::status_view,
        crate::integrity::commands::settings_show,
        crate::subscriptions::commands::subscription_detail,
        crate::subscriptions::commands::subscription_save,
        crate::subscriptions::commands::subscription_change_plan,
        crate::subscriptions::commands::subscription_set_ended,
        crate::subscriptions::commands::subscription_delete,
        crate::subscriptions::commands::subscription_match_count,
        crate::subscriptions::commands::subscription_prefill,
        crate::year::commands::year_entry_delete,
        crate::year::commands::year_entry_attach,
        crate::year::commands::year_evidence_choose,
        crate::year::commands::year_evidence_open,
        crate::search::search_elsewhere,
        crate::profiles::commands::profiles_list,
        crate::profiles::commands::profile_create,
        crate::profiles::commands::profile_switch,
        crate::profiles::commands::profile_rename,
    ]
}

/// The data folder with the home folder shortened to `~`, as Settings shows it.
#[tauri::command]
fn data_folder() -> String {
    let dir = crate::config_dir();
    match std::env::home_dir()
        .and_then(|home| dir.strip_prefix(home).ok().map(|rest| rest.to_path_buf()))
    {
        Some(rest) => format!("~/{}/", rest.display()),
        None => format!("{}/", dir.display()),
    }
}

/// Reveals the data folder in Finder (Settings → Privacy & Data).
#[tauri::command]
fn show_data_folder(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    crate::commands::handlers::show_data_folder(&app)
}

/// Whether Time Machine backs up the data folder; None when `tmutil` can't say (no backup set up).
#[tauri::command]
fn backup_included() -> Option<bool> {
    let output = std::process::Command::new("/usr/bin/tmutil")
        .arg("isexcluded")
        .arg(crate::config_dir())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    if text.contains("[Included]") {
        Some(true)
    } else if text.contains("[Excluded]") {
        Some(false)
    } else {
        None
    }
}

/// Writes a page error (a crash, an e2e result) to the app's log, where scripts and bug reports find it.
#[tauri::command]
fn log_frontend_error(message: String) {
    log::error!("Page: {message}");
}
