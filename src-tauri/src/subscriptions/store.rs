//! The `subscriptions` and `plans` rows.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

pub const SUBSCRIPTIONS: &str = "subscriptions";
pub const PLANS: &str = "plans";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Interval {
    Monthly,
    Quarterly,
    Yearly,
}

impl Interval {
    fn parse(text: &str) -> Interval {
        match text {
            "quarterly" => Interval::Quarterly,
            "yearly" => Interval::Yearly,
            _ => Interval::Monthly,
        }
    }

    pub fn months(self) -> u32 {
        match self {
            Interval::Monthly => 1,
            Interval::Quarterly => 3,
            Interval::Yearly => 12,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Interval::Monthly => "Monthly",
            Interval::Quarterly => "Quarterly",
            Interval::Yearly => "Yearly",
        }
    }
}

/// How long before a charge the reminder comes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Reminder {
    Off,
    OneDay,
    ThreeDays,
    OneWeek,
}

impl Reminder {
    fn parse(text: &str) -> Reminder {
        match text {
            "one_day" => Reminder::OneDay,
            "three_days" => Reminder::ThreeDays,
            "one_week" => Reminder::OneWeek,
            _ => Reminder::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Reminder::Off => "Off",
            Reminder::OneDay => "1 day before",
            Reminder::ThreeDays => "3 days before",
            Reminder::OneWeek => "1 week before",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SubscriptionRow {
    pub id: String,
    pub name: String,
    /// The tile's colour, one of the palette's account tones.
    pub tone: i64,
    pub category: String,
    /// `YYYY-MM-DD`, the last day it runs; None while active.
    pub ended_on: Option<String>,
    pub reminder: Reminder,
    /// Each charge counts as an ANV payment on this item.
    pub item_key: Option<String>,
    /// Bank text a charge contains, case-insensitive.
    pub match_text: String,
    /// How far a charge may differ from its plan's amount; None matches any amount.
    pub tolerance_cents: Option<i64>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlanRow {
    pub id: String,
    pub subscription_id: String,
    pub name: String,
    /// The amount in the plan's own currency, without sign.
    pub cents: i64,
    pub currency: String,
    pub interval: Interval,
    /// `YYYY-MM-DD`; the first day of the plan.
    pub start: String,
    /// The last day of the plan; None for the current one.
    pub end: Option<String>,
    pub created_at: i64,
}

impl PlanRow {
    pub fn covers(&self, day: &str) -> bool {
        self.start.as_str() <= day && self.end.as_deref().is_none_or(|end| day <= end)
    }
}

pub fn subscriptions(conn: &Connection) -> AppResult<Vec<SubscriptionRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT id, name, tone, category, ended_on, reminder, item_key, match_text, tolerance_cents, created_at \
         FROM {SUBSCRIPTIONS} ORDER BY name COLLATE NOCASE, id"
    ))?;
    let rows = query.query_map([], |row| {
        let reminder: String = row.get(5)?;
        Ok(SubscriptionRow {
            id: row.get(0)?,
            name: row.get(1)?,
            tone: row.get(2)?,
            category: row.get(3)?,
            ended_on: row.get(4)?,
            reminder: Reminder::parse(&reminder),
            item_key: row.get(6)?,
            match_text: row.get(7)?,
            tolerance_cents: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every plan, oldest start first.
pub fn plans(conn: &Connection) -> AppResult<Vec<PlanRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT id, subscription_id, name, cents, currency, interval, start, end, created_at FROM {PLANS} \
         ORDER BY start, created_at"
    ))?;
    let rows = query.query_map([], |row| {
        let interval: String = row.get(5)?;
        Ok(PlanRow {
            id: row.get(0)?,
            subscription_id: row.get(1)?,
            name: row.get(2)?,
            cents: row.get(3)?,
            currency: row.get(4)?,
            interval: Interval::parse(&interval),
            start: row.get(6)?,
            end: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
