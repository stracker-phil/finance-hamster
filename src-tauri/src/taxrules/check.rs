//! Reading a rules file: the schema's constraints that serde types can't hold, then the Reload checks of ADR 0040.
//! Every problem is reported at once, each naming where it is.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use super::model::{Ask, EstimateRule, Evidence, ItemKind, QuestionType, Ruleset, Vehicle};

const SCHEMA_VERSION: u32 = 1;

/// Parses and checks a file's text; `used_keys` are the item keys existing entries refer to, which must stay.
pub fn read(text: &str, used_keys: &BTreeSet<String>) -> Result<Ruleset, Vec<String>> {
    let ruleset: Ruleset = serde_json::from_str(text).map_err(|e| vec![e.to_string()])?;
    let problems = problems(&ruleset, used_keys);
    if problems.is_empty() {
        Ok(ruleset)
    } else {
        Err(problems)
    }
}

fn problems(r: &Ruleset, used_keys: &BTreeSet<String>) -> Vec<String> {
    let mut p = Vec::new();
    schema_constraints(r, &mut p);
    reload_checks(r, used_keys, &mut p);
    p
}

fn is_key(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_kz(s: &str) -> bool {
    s.split('/')
        .all(|part| part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()))
}

fn is_ruleset_name(s: &str) -> bool {
    let Some((year, version)) = s.split_once(" v") else {
        return false;
    };
    year.len() == 4
        && year.chars().all(|c| c.is_ascii_digit())
        && !version.is_empty()
        && version.chars().all(|c| c.is_ascii_digit())
}

fn date(s: &str) -> Option<NaiveDate> {
    (s.len() == 10)
        .then(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .flatten()
}

/// 2024 is a leap year, so `02-29` is a valid day.
fn is_month_day(s: &str) -> bool {
    s.len() == 5 && date(&format!("2024-{s}")).is_some()
}

fn schema_constraints(r: &Ruleset, p: &mut Vec<String>) {
    if r.schema_version != SCHEMA_VERSION {
        p.push(format!(
            "schemaVersion: this Hamster reads version {SCHEMA_VERSION}, not {}",
            r.schema_version
        ));
    }
    if !is_ruleset_name(&r.ruleset) {
        p.push(format!(
            "ruleset: \"{}\" should read like \"2026 v1\"",
            r.ruleset
        ));
    }
    if !(2000..=2100).contains(&r.year) {
        p.push(format!("year: {} is outside 2000 to 2100", r.year));
    }
    if !is_month_day(&r.settings.half_year_from) {
        p.push(format!(
            "settings.halfYearFrom: \"{}\" isn't a MM-DD day",
            r.settings.half_year_from
        ));
    }
    for (class, years) in &r.useful_life {
        if !(1..=50).contains(years) {
            p.push(format!(
                "usefulLife.{class}: {years} is outside 1 to 50 years"
            ));
        }
    }
    for (i, rate) in r.rates.kilometergeld.iter().enumerate() {
        for (field, value) in [("from", &rate.from), ("to", &rate.to)] {
            if date(value).is_none() {
                p.push(format!(
                    "rates.kilometergeld[{i}].{field}: \"{value}\" isn't a YYYY-MM-DD date"
                ));
            }
        }
    }
    for (i, q) in r.questions.iter().enumerate() {
        if !is_key(&q.id) {
            p.push(format!("questions[{i}].id: \"{}\" isn't a key", q.id));
        }
        ask_text(q.ask.as_ref(), &format!("questions[{i}]"), p);
    }
    if r.sections.is_empty() {
        p.push("sections: at least one section is needed".into());
    }
    for (i, s) in r.sections.iter().enumerate() {
        if !is_key(&s.id) {
            p.push(format!("sections[{i}].id: \"{}\" isn't a key", s.id));
        }
    }
    if r.items.is_empty() {
        p.push("items: at least one item is needed".into());
    }
    for (i, item) in r.items.iter().enumerate() {
        let at = format!("items[{i}]");
        if !is_key(&item.key) {
            p.push(format!("{at}.key: \"{}\" isn't a key", item.key));
        }
        if !is_kz(&item.kz) {
            p.push(format!(
                "{at}.kz: \"{}\" should read like \"717\" or \"718/916\"",
                item.kz
            ));
        }
        if item.private_share.is_some_and(|share| share > 100) {
            p.push(format!("{at}.privateShare: more than 100 percent"));
        }
        if item.kind == ItemKind::Trips && item.vehicle.is_none() {
            p.push(format!("{at}.vehicle: a trips item needs a vehicle"));
        }
        if item.kind == ItemKind::ReportedByOthers
            && item.evidence.is_some_and(|e| e != Evidence::None)
        {
            p.push(format!(
                "{at}.evidence: an item reported by others takes no evidence"
            ));
        }
        ask_text(item.ask.as_ref(), &at, p);
        if item.ask.is_some() {
            if !item.visible {
                p.push(format!(
                    "{at}.ask: a hidden item can't be asked about; show it or remove ask"
                ));
            }
            if item.kind == ItemKind::ReportedByOthers {
                p.push(format!(
                    "{at}.ask: an item reported by others has nothing to add"
                ));
            }
        }
    }
}

fn ask_text(ask: Option<&Ask>, at: &str, p: &mut Vec<String>) {
    if ask.is_some_and(|a| a.question.trim().is_empty()) {
        p.push(format!("{at}.ask.question: the question is empty"));
    }
}

fn reload_checks(r: &Ruleset, used_keys: &BTreeSet<String>, p: &mut Vec<String>) {
    let mut keys = BTreeSet::new();
    for item in &r.items {
        if !keys.insert(item.key.as_str()) {
            p.push(format!("items: the key \"{}\" is used twice", item.key));
        }
    }
    for key in used_keys {
        if !keys.contains(key.as_str()) {
            p.push(format!(
                "items: \"{key}\" is missing, but entries use it; keys can't be removed or renamed"
            ));
        }
    }
    let sections: BTreeSet<&str> = r.sections.iter().map(|s| s.id.as_str()).collect();
    let questions: BTreeMap<&str, QuestionType> = r
        .questions
        .iter()
        .map(|q| (q.id.as_str(), q.kind))
        .collect();
    for (i, item) in r.items.iter().enumerate() {
        let at = format!("items[{i}] ({})", item.key);
        if !sections.contains(item.section.as_str()) {
            p.push(format!(
                "{at}: the section \"{}\" doesn't exist",
                item.section
            ));
        }
        if let Some(d) = &item.depreciation
            && !r.useful_life.contains_key(&d.asset_class)
        {
            p.push(format!(
                "{at}: the asset class \"{}\" has no usefulLife",
                d.asset_class
            ));
        }
        for rule in &item.estimate {
            match rule {
                EstimateRule::Requires { question, .. }
                    if !questions.contains_key(question.as_str()) =>
                {
                    p.push(format!("{at}: the question \"{question}\" doesn't exist"));
                }
                EstimateRule::ReduceBy { question } => match questions.get(question.as_str()) {
                    None => p.push(format!("{at}: the question \"{question}\" doesn't exist")),
                    Some(QuestionType::Integer) => p.push(format!(
                        "{at}: reduceBy needs an amount question, \"{question}\" is an integer"
                    )),
                    Some(QuestionType::Amount) => {}
                },
                EstimateRule::Requires { .. } | EstimateRule::Cap { .. } => {}
            }
        }
        if item.kind == ItemKind::Trips
            && let Some(vehicle) = item.vehicle
            && !r
                .rates
                .kilometergeld
                .iter()
                .any(|rate| rate.vehicle == vehicle)
        {
            p.push(format!(
                "{at}: there is no kilometergeld rate for the vehicle \"{}\"",
                vehicle.as_str()
            ));
        }
    }
    km_ranges(r, p);
}

/// Each vehicle's ranges must lie inside the year and not overlap.
fn km_ranges(r: &Ruleset, p: &mut Vec<String>) {
    let first = NaiveDate::from_ymd_opt(r.year, 1, 1);
    let last = NaiveDate::from_ymd_opt(r.year, 12, 31);
    let mut by_vehicle: BTreeMap<Vehicle, Vec<(NaiveDate, NaiveDate, usize)>> = BTreeMap::new();
    for (i, rate) in r.rates.kilometergeld.iter().enumerate() {
        let (Some(from), Some(to)) = (date(&rate.from), date(&rate.to)) else {
            continue;
        };
        let at = format!("rates.kilometergeld[{i}]");
        if from > to {
            p.push(format!("{at}: from is after to"));
        }
        if Some(from) < first || Some(to) > last {
            p.push(format!("{at}: the range falls outside {}", r.year));
        }
        by_vehicle
            .entry(rate.vehicle)
            .or_default()
            .push((from, to, i));
    }
    for ranges in by_vehicle.values_mut() {
        ranges.sort();
        for pair in ranges.windows(2) {
            let ((_, end, a), (start, _, b)) = (pair[0], pair[1]);
            if start <= end {
                p.push(format!(
                    "rates.kilometergeld[{a}] and [{b}]: the ranges overlap"
                ));
            }
        }
    }
}
