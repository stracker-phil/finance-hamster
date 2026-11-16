use tauri::{AppHandle, State};

use super::Book;
use super::detail::{self, SubscriptionDetail};
use super::edit::{self, PlanForm, SubscriptionForm};
use super::view::{self, Format, MatchCount, SubscriptionsView};
use crate::date::DateOrder;
use crate::db::commands::announce;
use crate::db::{Source, Written};
use crate::error::AppResult;
use crate::state::AppState;

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn format(state: &AppState) -> Format {
    Format {
        number: state.settings.get().number_format,
        order: DateOrder::system(),
    }
}

fn done(app: &AppHandle, written: Written) -> String {
    announce(app, &written);
    written.label
}

/// `query` keeps the subscriptions whose name or category contains it.
#[tauri::command]
pub async fn subscriptions_view(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<SubscriptionsView> {
    let book = state.db.read(Book::load)?;
    let items = crate::rules::items(&state.db)?;
    Ok(view::subscriptions(
        &book,
        items,
        &query,
        today(),
        &format(&state),
    ))
}

#[tauri::command]
pub async fn subscription_detail(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<SubscriptionDetail> {
    let book = state.db.read(Book::load)?;
    let accounts = state.db.read(crate::accounts::rows)?;
    let items = crate::rules::items(&state.db)?;
    detail::detail(&book, &id, &accounts, &items, today(), &format(&state))
}

/// Adds a subscription (`id` null) or fixes one; returns the label of the change.
#[tauri::command]
pub async fn subscription_save(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
    form: SubscriptionForm,
) -> AppResult<String> {
    let written = match id {
        Some(id) => edit::update(&state.db, &id, &form, today(), now(), Source::Ui)?,
        None => edit::create(&state.db, &form, today(), now(), Source::Ui)?,
    };
    Ok(done(&app, written))
}

#[tauri::command]
pub async fn subscription_change_plan(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    form: PlanForm,
) -> AppResult<String> {
    let written = edit::change_plan(&state.db, &id, &form, now(), Source::Ui)?;
    Ok(done(&app, written))
}

/// Ends a subscription on `ended_on`; null resumes it.
#[tauri::command]
pub async fn subscription_set_ended(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    ended_on: Option<String>,
) -> AppResult<String> {
    let written = edit::set_ended(&state.db, &id, ended_on.as_deref(), now(), Source::Ui)?;
    Ok(done(&app, written))
}

#[tauri::command]
pub async fn subscription_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<String> {
    let written = edit::delete(&state.db, &id, now(), Source::Ui)?;
    Ok(done(&app, written))
}

/// How many statement lines the sheet's bank text and amount match.
#[tauri::command]
pub async fn subscription_match_count(
    state: State<'_, AppState>,
    form: SubscriptionForm,
) -> AppResult<MatchCount> {
    let lines = state.db.read(crate::statements::store::all_transactions)?;
    Ok(view::match_count(&lines, &form, &format(&state)))
}

/// A New sheet prefilled from a bank line.
#[tauri::command]
pub async fn subscription_prefill(
    state: State<'_, AppState>,
    line_id: String,
    term: Option<String>,
) -> AppResult<SubscriptionForm> {
    let format = format(&state);
    state
        .db
        .read(|c| view::prefill(c, &line_id, term.as_deref(), &format))
}
