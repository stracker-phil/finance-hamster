//! The years the Accounts page steps through: every tax year and every year with any data.

use std::collections::BTreeSet;

use rusqlite::Connection;

use crate::error::AppResult;

const YEARS_WITH_DATA: &str = "
    SELECT year FROM (
    SELECT year FROM tax_years
    UNION SELECT CAST(substr(from_date, 1, 4) AS INTEGER) FROM statements
    UNION SELECT CAST(substr(to_date, 1, 4) AS INTEGER) FROM statements
    UNION SELECT CAST(substr(value_date, 1, 4) AS INTEGER) FROM transactions
    UNION SELECT CAST(substr(pay_date, 1, 4) AS INTEGER) FROM documents
    UNION SELECT year FROM entries
    UNION SELECT year FROM payslips
    UNION SELECT CAST(substr(date, 1, 4) AS INTEGER) FROM others
    ) WHERE year IS NOT NULL";

/// The years to show, oldest first; `this_year` is always one of them.
pub fn shown(conn: &Connection, this_year: i32) -> AppResult<Vec<i32>> {
    let mut query = conn.prepare(YEARS_WITH_DATA)?;
    let mut years: BTreeSet<i32> = query
        .query_map([], |row| row.get::<_, i32>(0))?
        .collect::<Result<_, _>>()?;
    years.insert(this_year);
    Ok(years.into_iter().collect())
}
