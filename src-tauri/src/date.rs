//! Calendar dates (`YYYY-MM-DD`, never timestamps, ADR 0010) and how the window labels them: English month and
//! weekday names, day and month in the order of the macOS region.

use chrono::{Datelike, NaiveDate};

/// Parses `YYYY-MM-DD`.
pub fn parse(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

pub fn iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Whether a date reads `Nov 13, 2026` or `13 Nov 2026`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateOrder {
    MonthFirst,
    DayFirst,
}

impl DateOrder {
    /// The region's order, read once per run; day first where it cannot be read (and in tests, so labels never
    /// depend on the machine).
    pub fn system() -> DateOrder {
        static ORDER: std::sync::OnceLock<DateOrder> = std::sync::OnceLock::new();
        *ORDER.get_or_init(|| read_pattern().map_or(DateOrder::DayFirst, |p| order_of(&p)))
    }
}

/// The current locale's pattern for day, abbreviated month and year (`MMM d, y`, `d. MMM y`).
fn read_pattern() -> Option<String> {
    use objc2_foundation::{NSDateFormatter, NSLocale, NSString};
    // Tests run on a fixed order, whatever the Mac's region.
    if cfg!(any(test, feature = "test-util")) {
        return None;
    }
    let locale = NSLocale::currentLocale();
    NSDateFormatter::dateFormatFromTemplate_options_locale(
        &NSString::from_str("yMMMd"),
        0,
        Some(&locale),
    )
    .map(|pattern| pattern.to_string())
}

/// Day first when the pattern's day comes before its month; quoted literals (`'de'`) are skipped.
fn order_of(pattern: &str) -> DateOrder {
    let unquoted: String = pattern.split('\'').step_by(2).collect();
    match (unquoted.find('d'), unquoted.find(['M', 'L'])) {
        (Some(d), Some(m)) if d < m => DateOrder::DayFirst,
        _ => DateOrder::MonthFirst,
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// `November`.
pub fn month(date: NaiveDate) -> &'static str {
    MONTHS[date.month0() as usize]
}

/// `Nov`.
pub fn short_month(date: NaiveDate) -> &'static str {
    &month(date)[..3]
}

/// `13 Nov` / `Nov 13`.
pub fn day_month(date: NaiveDate, order: DateOrder) -> String {
    match order {
        DateOrder::DayFirst => format!("{} {}", date.day(), short_month(date)),
        DateOrder::MonthFirst => format!("{} {}", short_month(date), date.day()),
    }
}

/// `13 Nov 2026` / `Nov 13, 2026`.
pub fn day_month_year(date: NaiveDate, order: DateOrder) -> String {
    match order {
        DateOrder::DayFirst => format!("{} {} {}", date.day(), short_month(date), date.year()),
        DateOrder::MonthFirst => format!("{} {}, {}", short_month(date), date.day(), date.year()),
    }
}

/// `Friday, 13 November` / `Friday, November 13`, the Today page's line.
pub fn weekday_long(date: NaiveDate, order: DateOrder) -> String {
    let weekday = WEEKDAYS[date.weekday().num_days_from_monday() as usize];
    match order {
        DateOrder::DayFirst => format!("{weekday}, {} {}", date.day(), month(date)),
        DateOrder::MonthFirst => format!("{weekday}, {} {}", month(date), date.day()),
    }
}

/// `Tue, 10 Nov` / `Tue, Nov 10`, a bank line's day.
pub fn weekday_short(date: NaiveDate, order: DateOrder) -> String {
    let weekday = &WEEKDAYS[date.weekday().num_days_from_monday() as usize][..3];
    format!("{weekday}, {}", day_month(date, order))
}

/// `November` for 11.
pub fn month_name(month: u32) -> &'static str {
    NaiveDate::from_ymd_opt(2000, month.clamp(1, 12), 1).map_or("", self::month)
}

/// `November 2026`, a month heading.
pub fn month_year(date: NaiveDate) -> String {
    format!("{} {}", month(date), date.year())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn dates_round_trip_through_iso_text() {
        for date in [day(2026, 11, 13), day(2024, 2, 29), day(2026, 1, 1)] {
            assert_eq!(parse(&iso(date)), Some(date), "{date}");
        }
        assert_eq!(iso(day(2026, 1, 5)), "2026-01-05");
    }

    #[test]
    fn parse_trims_surrounding_whitespace() {
        assert_eq!(parse(" 2026-11-13\n"), Some(day(2026, 11, 13)));
    }

    #[test]
    fn invalid_date_text_parses_to_none() {
        for text in [
            "",
            "tomorrow",
            "2026-13-01",
            "2026-02-30",
            "2025-02-29",
            "13.11.2026",
            "2026-11",
        ] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_pattern_decides_the_order() {
        for (pattern, order) in [
            ("MMM d, y", DateOrder::MonthFirst),
            ("d. MMM y", DateOrder::DayFirst),
            ("d MMM y", DateOrder::DayFirst),
            ("y MMM d", DateOrder::MonthFirst),
            ("d 'de' MMM 'de' y", DateOrder::DayFirst),
            ("MMM 'd' d", DateOrder::MonthFirst),
            ("d LLL y", DateOrder::DayFirst),
            ("y", DateOrder::MonthFirst),
            ("", DateOrder::MonthFirst),
        ] {
            assert_eq!(order_of(pattern), order, "{pattern:?}");
        }
    }

    #[test]
    fn the_system_order_is_day_first_in_tests() {
        assert_eq!(DateOrder::system(), DateOrder::DayFirst);
    }

    #[test]
    fn day_and_month_read_in_the_chosen_order() {
        let date = day(2026, 11, 3);
        assert_eq!(day_month(date, DateOrder::DayFirst), "3 Nov");
        assert_eq!(day_month(date, DateOrder::MonthFirst), "Nov 3");
    }

    #[test]
    fn day_month_year_reads_in_the_chosen_order() {
        let date = day(2026, 11, 13);
        assert_eq!(day_month_year(date, DateOrder::DayFirst), "13 Nov 2026");
        assert_eq!(day_month_year(date, DateOrder::MonthFirst), "Nov 13, 2026");
    }

    #[test]
    fn the_long_weekday_line_names_weekday_and_month_in_full() {
        let date = day(2026, 11, 13);
        assert_eq!(
            weekday_long(date, DateOrder::DayFirst),
            "Friday, 13 November"
        );
        assert_eq!(
            weekday_long(date, DateOrder::MonthFirst),
            "Friday, November 13"
        );
    }

    #[test]
    fn the_short_weekday_line_abbreviates_the_weekday_and_reads_in_the_chosen_order() {
        let date = day(2026, 11, 10);
        assert_eq!(weekday_short(date, DateOrder::DayFirst), "Tue, 10 Nov");
        assert_eq!(weekday_short(date, DateOrder::MonthFirst), "Tue, Nov 10");
        assert_eq!(
            weekday_short(day(2026, 11, 15), DateOrder::DayFirst),
            "Sun, 15 Nov"
        );
    }

    #[test]
    fn every_weekday_and_month_has_its_english_name() {
        let weekdays = [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ];
        for (offset, name) in weekdays.iter().enumerate() {
            let date = day(2026, 11, 9 + offset as u32);
            assert_eq!(
                weekday_long(date, DateOrder::DayFirst).split(',').next(),
                Some(*name)
            );
        }
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        for (index, name) in months.iter().enumerate() {
            let date = day(2026, index as u32 + 1, 1);
            assert_eq!(month_year(date), format!("{name} 2026"));
            assert_eq!(
                day_month(date, DateOrder::DayFirst),
                format!("1 {}", &name[..3])
            );
        }
    }

    #[test]
    fn a_month_number_gives_its_english_name_and_stays_within_the_year() {
        for (month, name) in [
            (1, "January"),
            (7, "July"),
            (11, "November"),
            (12, "December"),
            (0, "January"),
            (13, "December"),
        ] {
            assert_eq!(month_name(month), name, "{month}");
        }
    }

    #[test]
    fn a_month_heading_names_month_and_year() {
        assert_eq!(month_year(day(2026, 11, 13)), "November 2026");
    }
}
