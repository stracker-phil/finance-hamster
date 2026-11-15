use tauri::{AppHandle, State};

use super::RuleForm;
use super::preview::RulePreview;
use super::view::{self, RulesView};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::AppResult;
use crate::state::AppState;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[tauri::command]
pub fn rules_view(state: State<'_, AppState>) -> AppResult<RulesView> {
    view::rules(
        &state.db,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Adds a rule (no `id`) or changes one; returns the label of the change.
#[tauri::command]
pub fn rule_save(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
    form: RuleForm,
) -> AppResult<String> {
    let written = super::save(&state.db, id.as_deref(), &form, now(), Source::Ui)?;
    announce(&app, &written);
    Ok(written.label)
}

#[tauri::command]
pub fn rule_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<String> {
    let written = super::delete(&state.db, &id, now(), Source::Ui)?;
    announce(&app, &written);
    Ok(written.label)
}

/// Applies the rules to every line again; returns how many lines changed, in words.
#[tauri::command]
pub fn rules_rerun(app: AppHandle, state: State<'_, AppState>) -> AppResult<String> {
    Ok(match super::rerun(&state.db, now(), Source::Ui)? {
        None => crate::tr!(
            "Rules applied · no line changed",
            "Regeln angewendet · keine Buchung geändert"
        ),
        Some((written, lines)) => {
            announce(&app, &written);
            crate::tr_plural!(
                lines,
                "Rules applied · {lines} line changed",
                "Rules applied · {lines} lines changed",
                "Regeln angewendet · {lines} Buchung geändert",
                "Regeln angewendet · {lines} Buchungen geändert"
            )
        }
    })
}

/// What the sheet's rule would match, while it is being edited.
#[tauri::command]
pub fn rule_preview(state: State<'_, AppState>, form: RuleForm) -> AppResult<RulePreview> {
    super::preview::preview(
        &state.db,
        &form,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}
