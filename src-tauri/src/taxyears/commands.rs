use tauri::State;

use super::TaxYear;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_tax_years(state: State<'_, AppState>) -> AppResult<Vec<TaxYear>> {
    super::list(&state.db)
}
