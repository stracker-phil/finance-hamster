//! The demo amounts and trips of the tax year page: union dues as typed and two business trips by car.

use std::path::Path;

use crate::db::{Db, Source};
use crate::error::AppResult;
use crate::taxrules::model::Ruleset;
use crate::year::entries::{self, NewEntry};

fn amount(item: &str, date: &str, amount: &str, text: &str) -> NewEntry {
    NewEntry::Amount {
        item_key: item.into(),
        date: date.into(),
        amount: amount.into(),
        text: text.into(),
        evidence: None,
    }
}

fn trip(date: &str, km: &str, text: &str) -> NewEntry {
    NewEntry::Trip {
        item_key: "travel".into(),
        date: date.into(),
        km: km.into(),
        text: text.into(),
    }
}

/// Adds the demo entries to `year`; returns how many ops it wrote.
pub fn seed(db: &Db, config_dir: &Path, year: i32, now: i64) -> AppResult<usize> {
    let Some(row) = db.read(|c| crate::taxyears::row(c, year))? else {
        return Ok(0);
    };
    let ruleset: Ruleset = serde_json::from_str(&row.rules)?;
    let demo = [
        amount(
            "union-dues",
            &format!("{year}-03-31"),
            "186,00",
            "Union membership",
        ),
        trip(&format!("{year}-05-14"), "64", "Linz · meetup talk"),
        trip(&format!("{year}-10-02"), "52", "Wels · team day"),
    ];
    for entry in &demo {
        entries::add(db, config_dir, &ruleset, year, entry, now, Source::Seed)?;
    }
    Ok(demo.len())
}
