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

/// `2026 v1`, or `2026 v1 (own)` for the owner's version edited in Settings.
fn is_ruleset_name(s: &str) -> bool {
    let s = s.strip_suffix(" (own)").unwrap_or(s);
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
        p.push(crate::tr!(
            "schemaVersion: this Hamster reads version {SCHEMA_VERSION}, not {}",
            "schemaVersion: Dieser Hamster liest Version {SCHEMA_VERSION}, nicht {}",
            r.schema_version
        ));
    }
    if !is_ruleset_name(&r.ruleset) {
        p.push(crate::tr!(
            "ruleset: \"{}\" should read like \"2026 v1\"",
            "ruleset: „{}“ sollte wie „2026 v1“ aussehen",
            r.ruleset
        ));
    }
    if !(2000..=2100).contains(&r.year) {
        p.push(crate::tr!(
            "year: {} is outside 2000 to 2100",
            "year: {} liegt nicht zwischen 2000 und 2100",
            r.year
        ));
    }
    if !is_month_day(&r.settings.half_year_from) {
        p.push(crate::tr!(
            "settings.halfYearFrom: \"{}\" isn't a MM-DD day",
            "settings.halfYearFrom: „{}“ ist kein Tag im Format MM-DD",
            r.settings.half_year_from
        ));
    }
    for (class, years) in &r.useful_life {
        if !(1..=50).contains(years) {
            p.push(crate::tr!(
                "usefulLife.{class}: {years} is outside 1 to 50 years",
                "usefulLife.{class}: {years} liegt nicht zwischen 1 und 50 Jahren"
            ));
        }
    }
    for (i, rate) in r.rates.kilometergeld.iter().enumerate() {
        for (field, value) in [("from", &rate.from), ("to", &rate.to)] {
            if date(value).is_none() {
                p.push(crate::tr!(
                    "rates.kilometergeld[{i}].{field}: \"{value}\" isn't a YYYY-MM-DD date",
                    "rates.kilometergeld[{i}].{field}: „{value}“ ist kein Datum im Format YYYY-MM-DD"
                ));
            }
        }
    }
    for (i, q) in r.questions.iter().enumerate() {
        if !is_key(&q.id) {
            p.push(crate::tr!(
                "questions[{i}].id: \"{}\" isn't a key",
                "questions[{i}].id: „{}“ ist kein Schlüssel",
                q.id
            ));
        }
        let at = format!("questions[{i}]");
        ask_text(q.ask.as_ref(), &at, p);
        if let Some(de) = &q.de {
            german(
                &[("label", &de.label), ("help", &de.help), ("unit", &de.unit)],
                &at,
                p,
            );
        }
    }
    if r.sections.is_empty() {
        p.push(crate::tr!(
            "sections: at least one section is needed",
            "sections: Mindestens ein Abschnitt ist nötig"
        ));
    }
    for (i, s) in r.sections.iter().enumerate() {
        if !is_key(&s.id) {
            p.push(crate::tr!(
                "sections[{i}].id: \"{}\" isn't a key",
                "sections[{i}].id: „{}“ ist kein Schlüssel",
                s.id
            ));
        }
        if let Some(de) = &s.de {
            german(&[("name", &de.name)], &format!("sections[{i}]"), p);
        }
    }
    if r.items.is_empty() {
        p.push(crate::tr!(
            "items: at least one item is needed",
            "items: Mindestens ein Posten ist nötig"
        ));
    }
    for (i, item) in r.items.iter().enumerate() {
        let at = format!("items[{i}]");
        if !is_key(&item.key) {
            p.push(crate::tr!(
                "{at}.key: \"{}\" isn't a key",
                "{at}.key: „{}“ ist kein Schlüssel",
                item.key
            ));
        }
        if !is_kz(&item.kz) {
            p.push(crate::tr!(
                "{at}.kz: \"{}\" should read like \"717\" or \"718/916\"",
                "{at}.kz: „{}“ sollte wie „717“ oder „718/916“ aussehen",
                item.kz
            ));
        }
        if item.private_share.is_some_and(|share| share > 100) {
            p.push(crate::tr!(
                "{at}.privateShare: more than 100 percent",
                "{at}.privateShare: mehr als 100 Prozent"
            ));
        }
        if item.kind == ItemKind::Trips && item.vehicle.is_none() {
            p.push(crate::tr!(
                "{at}.vehicle: a trips item needs a vehicle",
                "{at}.vehicle: Ein Fahrten-Posten braucht ein Fahrzeug"
            ));
        }
        if item.kind == ItemKind::ReportedByOthers
            && item.evidence.is_some_and(|e| e != Evidence::None)
        {
            p.push(crate::tr!(
                "{at}.evidence: an item reported by others takes no evidence",
                "{at}.evidence: Ein von anderen gemeldeter Posten nimmt keinen Nachweis"
            ));
        }
        ask_text(item.ask.as_ref(), &at, p);
        if let Some(de) = &item.de {
            german(
                &[
                    ("name", &de.name),
                    ("label", &de.label),
                    ("examples", &de.examples),
                    ("note", &de.note),
                ],
                &at,
                p,
            );
        }
        if item.ask.is_some() {
            if !item.visible {
                p.push(crate::tr!(
                    "{at}.ask: a hidden item can't be asked about; show it or remove ask",
                    "{at}.ask: Nach einem verborgenen Posten kann nicht gefragt werden; zeig ihn oder entferne ask"
                ));
            }
            if item.kind == ItemKind::ReportedByOthers {
                p.push(crate::tr!(
                    "{at}.ask: an item reported by others has nothing to add",
                    "{at}.ask: Zu einem von anderen gemeldeten Posten gibt es nichts hinzuzufügen"
                ));
            }
        }
    }
}

fn ask_text(ask: Option<&Ask>, at: &str, p: &mut Vec<String>) {
    let Some(ask) = ask else {
        return;
    };
    if ask.question.trim().is_empty() {
        p.push(crate::tr!(
            "{at}.ask.question: the question is empty",
            "{at}.ask.question: Die Frage ist leer"
        ));
    }
    if let Some(de) = &ask.de {
        german(
            &[("question", &de.question), ("hint", &de.hint)],
            &format!("{at}.ask"),
            p,
        );
    }
}

/// A German text that is present must say something; leave it out to show the English one.
fn german(fields: &[(&str, &Option<String>)], at: &str, p: &mut Vec<String>) {
    for (field, text) in fields {
        if text.as_deref().is_some_and(|t| t.trim().is_empty()) {
            p.push(crate::tr!(
                "{at}.de.{field}: the German text is empty; remove it to show the English one",
                "{at}.de.{field}: Der deutsche Text ist leer; entferne ihn, um den englischen zu zeigen"
            ));
        }
    }
}

fn missing_question(at: &str, question: &str) -> String {
    crate::tr!(
        "{at}: the question \"{question}\" doesn't exist",
        "{at}: Die Frage „{question}“ gibt es nicht"
    )
}

fn reload_checks(r: &Ruleset, used_keys: &BTreeSet<String>, p: &mut Vec<String>) {
    let mut keys = BTreeSet::new();
    for item in &r.items {
        if !keys.insert(item.key.as_str()) {
            p.push(crate::tr!(
                "items: the key \"{}\" is used twice",
                "items: Der Schlüssel „{}“ kommt zweimal vor",
                item.key
            ));
        }
    }
    for key in used_keys {
        if !keys.contains(key.as_str()) {
            p.push(crate::tr!(
                "items: \"{key}\" is missing, but entries use it; keys can't be removed or renamed",
                "items: „{key}“ fehlt, wird aber von Einträgen verwendet; Schlüssel können nicht entfernt oder umbenannt werden"
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
            p.push(crate::tr!(
                "{at}: the section \"{}\" doesn't exist",
                "{at}: Den Abschnitt „{}“ gibt es nicht",
                item.section
            ));
        }
        if let Some(d) = &item.depreciation
            && !r.useful_life.contains_key(&d.asset_class)
        {
            p.push(crate::tr!(
                "{at}: the asset class \"{}\" has no usefulLife",
                "{at}: Die Anlagenklasse „{}“ hat kein usefulLife",
                d.asset_class
            ));
        }
        for rule in &item.estimate {
            match rule {
                EstimateRule::Requires { question, .. }
                    if !questions.contains_key(question.as_str()) =>
                {
                    p.push(missing_question(&at, question));
                }
                EstimateRule::ReduceBy { question } => match questions.get(question.as_str()) {
                    None => p.push(missing_question(&at, question)),
                    Some(QuestionType::Integer) => p.push(crate::tr!(
                        "{at}: reduceBy needs an amount question, \"{question}\" is an integer",
                        "{at}: reduceBy braucht eine Betragsfrage, „{question}“ ist eine ganze Zahl"
                    )),
                    Some(QuestionType::Amount) => {}
                },
                EstimateRule::ReduceByPerUnit { question, .. } => {
                    match questions.get(question.as_str()) {
                        None => p.push(missing_question(&at, question)),
                        Some(QuestionType::Amount) => p.push(crate::tr!(
                            "{at}: reduceByPerUnit needs an integer question, \"{question}\" is an amount",
                            "{at}: reduceByPerUnit braucht eine Ganzzahlfrage, „{question}“ ist ein Betrag"
                        )),
                        Some(QuestionType::Integer) => {}
                    }
                }
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
            p.push(crate::tr!(
                "{at}: there is no kilometergeld rate for the vehicle \"{}\"",
                "{at}: Für das Fahrzeug „{}“ gibt es keinen kilometergeld-Satz",
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
            p.push(crate::tr!(
                "{at}: from is after to",
                "{at}: from liegt nach to"
            ));
        }
        if Some(from) < first || Some(to) > last {
            p.push(crate::tr!(
                "{at}: the range falls outside {}",
                "{at}: Der Zeitraum liegt außerhalb von {}",
                r.year
            ));
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
                p.push(crate::tr!(
                    "rates.kilometergeld[{a}] and [{b}]: the ranges overlap",
                    "rates.kilometergeld[{a}] und [{b}]: Die Zeiträume überschneiden sich"
                ));
            }
        }
    }
}
