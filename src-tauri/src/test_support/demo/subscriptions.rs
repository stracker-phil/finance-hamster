//! The demo subscriptions: a streaming service, an AI tool billed in dollars that counts for ANV and the
//! household insurance, found by its contract number rather than the insurer's name; one cancelled
//! audiobook service.

use chrono::{Datelike, NaiveDate};

use crate::db::{Db, Source};
use crate::error::AppResult;
use crate::subscriptions::edit::{self, SubscriptionForm};
use crate::subscriptions::store::{Interval, Reminder};

struct Demo {
    name: &'static str,
    tone: i64,
    category: &'static str,
    amount: &'static str,
    currency: &'static str,
    match_text: &'static str,
    item_key: Option<&'static str>,
    reminder: Reminder,
    /// The last day of a cancelled one, in the seed year.
    ended: Option<(u32, u32)>,
}

const DEMOS: [Demo; 4] = [
    Demo {
        name: "Streamflix",
        tone: 5,
        category: "Media",
        amount: "12,99",
        currency: "EUR",
        match_text: "STREAMFLIX",
        item_key: None,
        reminder: Reminder::Off,
        ended: None,
    },
    Demo {
        name: "Lumen AI",
        tone: 2,
        category: "AI & Software",
        amount: "24,00",
        currency: "USD",
        match_text: "LUMEN AI",
        item_key: Some("digital-equipment"),
        reminder: Reminder::ThreeDays,
        ended: None,
    },
    Demo {
        name: "Household insurance",
        tone: 4,
        category: "Insurance",
        amount: "38,90",
        currency: "EUR",
        match_text: "4400-1122",
        item_key: None,
        reminder: Reminder::Off,
        ended: None,
    },
    Demo {
        name: "Tonfeld Audiobooks",
        tone: 3,
        category: "Media",
        amount: "9,95",
        currency: "EUR",
        match_text: "TONFELD",
        item_key: None,
        reminder: Reminder::Off,
        ended: Some((6, 30)),
    },
];

/// Adds the demo subscriptions; returns how many ops it wrote.
pub fn seed(db: &Db, today: NaiveDate, now: i64) -> AppResult<usize> {
    let year = today.year();
    for demo in &DEMOS {
        let form = SubscriptionForm {
            name: demo.name.into(),
            tone: demo.tone,
            category: demo.category.into(),
            active: demo.ended.is_none(),
            ended_on: demo.ended.map(|(m, d)| format!("{year}-{m:02}-{d:02}")),
            amount: demo.amount.into(),
            currency: demo.currency.into(),
            interval: Interval::Monthly,
            start: format!("{year}-01-01"),
            reminder: demo.reminder,
            item_key: demo.item_key.map(Into::into),
            match_text: demo.match_text.into(),
            tolerance: Some("0".into()),
        };
        edit::create(db, &form, today, now, Source::Seed)?;
    }
    Ok(DEMOS.len())
}
