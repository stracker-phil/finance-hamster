//! Subscriptions for tests: forms, a database with imported statements, and plans written as rows.

use chrono::NaiveDate;
use tempfile::TempDir;

use crate::db::{Change, Db, Op, Source};
use crate::statements::import;
use crate::statements::import_tests::open_with_accounts_and_year;
use crate::subscriptions::edit::{self, SubscriptionForm};
use crate::subscriptions::store::{self, Interval, PlanRow, Reminder, SubscriptionRow};
use crate::test_support::statements::Statement;

pub const NOW: i64 = 1_700_000_000;

pub fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// The day the tests run on.
pub fn today() -> NaiveDate {
    day(2026, 11, 13)
}

/// The demo accounts, an open 2026 year with item `books`, and the statements imported without rules.
pub fn imported(statements: &[Statement]) -> (TempDir, Db) {
    let (dir, db) = open_with_accounts_and_year();
    for statement in statements {
        let pending = import::prepare(&db, dir.path(), statement.pdf()).unwrap();
        import::commit(&db, dir.path(), pending, NOW, Source::Import).unwrap();
    }
    (dir, db)
}

/// An active monthly EUR subscription starting 2026-01-01 that matches `match_text` at exactly `amount`.
pub fn form(name: &str, match_text: &str, amount: &str) -> SubscriptionForm {
    SubscriptionForm {
        name: name.into(),
        tone: 1,
        category: "Media".into(),
        active: true,
        ended_on: None,
        amount: amount.into(),
        currency: "EUR".into(),
        interval: Interval::Monthly,
        start: "2026-01-01".into(),
        reminder: Reminder::Off,
        item_key: None,
        match_text: match_text.into(),
        exact_only: false,
    }
}

/// Creates the subscription through `edit::create` and returns its id.
pub fn add(db: &Db, form: &SubscriptionForm) -> String {
    edit::create(db, form, today(), NOW, Source::Ui).unwrap();
    db.read(store::subscriptions)
        .unwrap()
        .into_iter()
        .find(|s| s.name == form.name.trim())
        .unwrap()
        .id
}

pub fn subscription_row(id: &str, name: &str, match_text: &str) -> SubscriptionRow {
    SubscriptionRow {
        id: id.into(),
        name: name.into(),
        tone: 1,
        category: "Media".into(),
        ended_on: None,
        reminder: Reminder::Off,
        item_key: None,
        match_text: match_text.into(),
        exact_only: false,
        created_at: 1,
    }
}

pub fn plan_row(
    id: &str,
    subscription_id: &str,
    cents: i64,
    start: &str,
    end: Option<&str>,
) -> PlanRow {
    PlanRow {
        id: id.into(),
        subscription_id: subscription_id.into(),
        name: id.into(),
        cents,
        currency: "EUR".into(),
        interval: Interval::Monthly,
        start: start.into(),
        end: end.map(Into::into),
        created_at: 1,
    }
}

/// Writes a subscription and its plans as they are, without matching lines.
pub fn insert_rows(db: &Db, sub: &SubscriptionRow, plans: &[PlanRow]) {
    let mut changes = vec![Change::insert(store::SUBSCRIPTIONS, sub).unwrap()];
    for plan in plans {
        changes.push(Change::insert(store::PLANS, plan).unwrap());
    }
    db.write(Op::new("Add subscription rows", changes), Source::Seed)
        .unwrap();
}
