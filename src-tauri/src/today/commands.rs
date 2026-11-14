use tauri::State;

use super::TodayView;
use crate::date::DateOrder;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn today_view(state: State<'_, AppState>) -> AppResult<TodayView> {
    let settings = state.settings.get();
    super::today_view(
        &state.db,
        chrono::Local::now().date_naive(),
        settings.number_format,
        DateOrder::system(),
        settings.import_after_days,
    )
}
