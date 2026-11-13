//! The Tauri commands the page may call, in one list.

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::launch::take_launch_events,
        log_frontend_error,
        crate::commands::handlers::list_commands,
        crate::commands::handlers::run_command,
        crate::settings::commands::get_settings,
        crate::settings::commands::update_settings,
        show_data_folder,
        data_folder,
        crate::db::commands::undo,
        crate::db::commands::redo,
        crate::accounts::commands::list_accounts,
        crate::accounts::commands::add_account,
        crate::accounts::commands::edit_account,
        crate::taxyears::commands::list_tax_years,
        crate::taxrules::commands::rules_overview,
        crate::taxrules::commands::rules_reload_preview,
        crate::taxrules::commands::rules_reload_apply,
        crate::taxrules::commands::rules_import,
        crate::taxrules::commands::rules_duplicate,
        crate::taxrules::commands::rules_open,
        crate::taxrules::commands::rules_show_folder,
        crate::preview::commands::pdf_view_show,
        crate::preview::commands::pdf_view_move,
        crate::preview::commands::pdf_view_hide,
        crate::preview::commands::pdf_view_suspend,
        crate::preview::commands::pdf_view_resume,
        crate::preview::commands::pdf_view_visible,
        crate::statements::commands::statement_import_choose,
        crate::statements::commands::statement_import_commit,
        crate::statements::commands::statement_import_cancel,
        crate::statements::commands::statements_coverage,
        crate::statements::commands::account_lines,
        crate::rules::commands::rules_view,
        crate::rules::commands::rule_save,
        crate::rules::commands::rule_delete,
        crate::rules::commands::rules_rerun,
        crate::review::commands::review_queue,
        crate::review::commands::review_decide,
        crate::documents::commands::document_add_choose,
        crate::documents::commands::document_add_commit,
        crate::documents::commands::document_add_cancel,
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
fn show_data_folder() -> crate::error::AppResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(crate::config_dir()).map_err(|e| {
        crate::error::AppError::Internal(format!("Couldn't show the data folder: {e}"))
    })
}

/// Writes a page error (a crash, an e2e result) to the app's log, where scripts and bug reports find it.
#[tauri::command]
fn log_frontend_error(message: String) {
    log::error!("Page: {message}");
}
