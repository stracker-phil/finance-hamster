//! When a subscription charges next and what it costs per month, always as of today.

use chrono::{Months, NaiveDate};

use super::matcher::plan_on;
use super::store::{PlanRow, SubscriptionRow};
use crate::date;
use crate::statements::store::TransactionRow;

/// A subscription with its plans (oldest first) and its charges (oldest first).
pub struct Running<'a> {
    pub sub: &'a SubscriptionRow,
    pub plans: Vec<&'a PlanRow>,
    pub charges: Vec<&'a TransactionRow>,
}

impl Running<'_> {
    /// The plan in effect today; before the first plan starts, that one.
    pub fn current(&self, today: NaiveDate) -> Option<&PlanRow> {
        plan_on(&self.plans, &date::iso(today))
    }

    /// The last charge plus the plan's interval; with no charge yet, the plan's start. None once ended.
    pub fn next_charge(&self, today: NaiveDate) -> Option<NaiveDate> {
        if self.sub.ended_on.is_some() {
            return None;
        }
        let plan = self.current(today)?;
        let next = match self.charges.last() {
            Some(last) => date::parse(&last.value_date)?
                .checked_add_months(Months::new(plan.interval.months()))?,
            None => date::parse(&plan.start)?,
        };
        // A plan that started after the last charge charges first on its start.
        let start = date::parse(&plan.start)?;
        Some(if start > next { start } else { next })
    }

    /// Euro cents per month, rounded to the cent: the plan's amount in euros, else its last charge's euro amount.
    pub fn monthly_cents(&self, today: NaiveDate) -> Option<i64> {
        let plan = self.current(today)?;
        let cents = if plan.currency == "EUR" {
            plan.cents
        } else {
            self.charges.last()?.cents.abs()
        };
        let months = i64::from(plan.interval.months());
        Some((cents + months / 2) / months)
    }

    /// What one charge costs in euros: the plan's amount, else its last charge's.
    pub fn charge_cents(&self, today: NaiveDate) -> Option<i64> {
        let plan = self.current(today)?;
        if plan.currency == "EUR" {
            Some(plan.cents)
        } else {
            self.charges.last().map(|c| c.cents.abs())
        }
    }
}
