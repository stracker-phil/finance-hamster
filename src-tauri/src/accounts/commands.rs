use tauri::{AppHandle, State};

use super::{Account, AccountEdit, NewAccount};
use crate::db::commands::announce;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    super::list(&state.db)
}

#[tauri::command]
pub fn add_account(
    app: AppHandle,
    state: State<'_, AppState>,
    account: NewAccount,
) -> AppResult<Account> {
    let (account, written) = super::add(&state.db, account, chrono::Utc::now().timestamp_millis())?;
    announce(&app, &written);
    Ok(account)
}

#[tauri::command]
pub fn edit_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    account: AccountEdit,
) -> AppResult<()> {
    let written = super::edit(&state.db, &id, account)?;
    announce(&app, &written);
    Ok(())
}
