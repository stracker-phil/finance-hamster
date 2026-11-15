//! One year's rules in plain words for Settings → Tax Rules: the values that change from year to year, each
//! with the path `edit::set_value` takes.

use serde::Serialize;
use ts_rs::TS;

use super::model::{EstimateRule, Item, Ruleset};
use crate::date::{self, DateOrder};
use crate::money::{Money, format_money, format_plain};
use crate::settings::NumberFormat;
use crate::taxyears::{RulesSource, TaxYearRow, YearState};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum FieldKind {
    Amount,
    Count,
    Percent,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesField {
    /// `/`-separated path into the rules; items are named by key.
    pub path: String,
    pub label: String,
    /// The value as typed in the field.
    pub value: String,
    pub unit: Option<String>,
    pub kind: FieldKind,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesGroup {
    pub title: String,
    pub fields: Vec<RulesField>,
    /// Facts shown beside the fields but not edited here.
    pub facts: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesYearView {
    pub year: i32,
    pub editable: bool,
    pub source: RulesSource,
    pub ruleset: String,
    pub based_on: Option<String>,
    pub groups: Vec<RulesGroup>,
    pub notes: Option<String>,
    pub sources: Vec<String>,
}

/// The unit a field's value is typed in.
pub fn kind_of(path: &str) -> FieldKind {
    match path.rsplit('/').next().unwrap_or_default() {
        "privateShare" => FieldKind::Percent,
        "maxUnits" | "min" => FieldKind::Count,
        _ if path.starts_with("usefulLife/") => FieldKind::Count,
        _ => FieldKind::Amount,
    }
}

fn asset_class(class: &str) -> String {
    match class {
        "computer" => crate::tr!("Computers", "Computer"),
        "furniture" => crate::tr!("Furniture", "Möbel"),
        other => other.to_string(),
    }
}

fn years() -> Option<String> {
    Some(crate::tr!("years", "Jahre"))
}

fn amount(
    path: String,
    label: String,
    cents: i64,
    unit: Option<String>,
    format: NumberFormat,
) -> RulesField {
    RulesField {
        path,
        label,
        value: format_plain(cents, format),
        unit: unit.or(Some("€".into())),
        kind: FieldKind::Amount,
    }
}

fn count(path: String, label: String, value: String, unit: Option<String>) -> RulesField {
    RulesField {
        kind: kind_of(&path),
        path,
        label,
        value,
        unit,
    }
}

fn general(r: &Ruleset, format: NumberFormat, order: DateOrder) -> RulesGroup {
    let mut fields = Vec::new();
    if let Some(p) = r.settings.werbungskostenpauschale {
        fields.push(amount(
            "settings/werbungskostenpauschale".into(),
            crate::tr!(
                "Flat allowance the tax office grants anyway",
                "Pauschale, die das Finanzamt ohnehin anrechnet"
            ),
            p.0,
            None,
            format,
        ));
    }
    fields.push(amount(
        "settings/gwgLimit".into(),
        crate::tr!(
            "Purchases above this are spread over their useful life",
            "Käufe darüber werden über die Nutzungsdauer verteilt"
        ),
        r.settings.gwg_limit.0,
        None,
        format,
    ));
    let half = date::parse(&format!("{}-{}", r.year, r.settings.half_year_from))
        .map(|d| date::day_month(d, order))
        .unwrap_or_else(|| r.settings.half_year_from.clone());
    RulesGroup {
        title: crate::tr!("General", "Allgemein"),
        fields,
        facts: vec![crate::tr!(
            "Bought from {half} on: the first year counts half",
            "Ab {half} gekauft: Das erste Jahr zählt halb"
        )],
    }
}

fn useful_life(r: &Ruleset) -> RulesGroup {
    RulesGroup {
        title: crate::tr!("Useful life", "Nutzungsdauer"),
        fields: r
            .useful_life
            .iter()
            .map(|(class, n)| {
                count(
                    format!("usefulLife/{class}"),
                    asset_class(class),
                    n.to_string(),
                    years(),
                )
            })
            .collect(),
        facts: Vec::new(),
    }
}

fn kilometergeld(r: &Ruleset, format: NumberFormat, order: DateOrder) -> RulesGroup {
    let vehicle = |v: super::model::Vehicle| match v {
        super::model::Vehicle::Car => crate::tr!("Car", "Auto"),
        super::model::Vehicle::Motorbike => crate::tr!("Motorbike", "Motorrad"),
        super::model::Vehicle::Bicycle => crate::tr!("Bicycle", "Fahrrad"),
    };
    let day =
        |text: &str| date::parse(text).map_or(text.to_string(), |d| date::day_month(d, order));
    RulesGroup {
        title: crate::tr!("Kilometergeld", "Kilometergeld"),
        fields: r
            .rates
            .kilometergeld
            .iter()
            .enumerate()
            .map(|(i, rate)| {
                let label = format!(
                    "{}, {} – {}",
                    vehicle(rate.vehicle),
                    day(&rate.from),
                    day(&rate.to)
                );
                amount(
                    format!("rates/kilometergeld/{i}/perKm"),
                    label,
                    rate.per_km.0,
                    Some("€ / km".into()),
                    format,
                )
            })
            .collect(),
        facts: Vec::new(),
    }
}

fn item_group(r: &Ruleset, item: &Item, format: NumberFormat) -> Option<RulesGroup> {
    let at = format!("items/{}", item.key);
    let mut fields = Vec::new();
    if let Some(share) = item.private_share {
        fields.push(count(
            format!("{at}/privateShare"),
            crate::tr!(
                "Private share, preset for new documents",
                "Privatanteil, vorbelegt für neue Belege"
            ),
            share.to_string(),
            Some("%".into()),
        ));
    }
    let unit = |id: &str| {
        r.questions
            .iter()
            .find(|q| q.id == id)
            .and_then(|q| q.display_unit())
            .map(str::to_string)
    };
    for (i, rule) in item.estimate.iter().enumerate() {
        let path = |field: &str| format!("{at}/estimate/{i}/{field}");
        match rule {
            EstimateRule::Cap {
                amount: cap,
                carry_over,
            } => fields.push(amount(
                path("amount"),
                if *carry_over {
                    crate::tr!(
                        "Counted per year at most; the rest in later years",
                        "Pro Jahr höchstens gezählt; der Rest in späteren Jahren"
                    )
                } else {
                    crate::tr!("Counted at most", "Höchstens gezählt")
                },
                cap.0,
                None,
                format,
            )),
            EstimateRule::Requires { question, min } => fields.push(count(
                path("min"),
                crate::tr!("Needed at least", "Mindestens nötig"),
                min.to_string(),
                unit(question),
            )),
            EstimateRule::ReduceByPerUnit {
                question,
                per_unit,
                max_units,
            } => {
                let per = unit(question).unwrap_or_default();
                fields.push(amount(
                    path("perUnit"),
                    crate::tr!(
                        "The tax office subtracts per {per}",
                        "Das Finanzamt zieht pro {per} ab"
                    ),
                    per_unit.0,
                    None,
                    format,
                ));
                fields.push(count(
                    path("maxUnits"),
                    crate::tr!("Counted at most", "Höchstens gezählt"),
                    max_units.to_string(),
                    unit(question),
                ));
            }
            EstimateRule::ReduceBy { .. } => {}
        }
    }
    (!fields.is_empty()).then(|| RulesGroup {
        title: format!("{} · KZ {}", item.display_label(), item.kz),
        fields,
        facts: Vec::new(),
    })
}

pub fn year_view(
    row: &TaxYearRow,
    r: &Ruleset,
    format: NumberFormat,
    order: DateOrder,
) -> RulesYearView {
    let mut groups = vec![
        general(r, format, order),
        useful_life(r),
        kilometergeld(r, format, order),
    ];
    groups.extend(
        r.items
            .iter()
            .filter(|i| i.visible)
            .filter_map(|i| item_group(r, i, format)),
    );
    groups.retain(|g| !g.fields.is_empty() || !g.facts.is_empty());
    RulesYearView {
        year: row.year,
        editable: row.state == YearState::Open,
        source: row.rules_source,
        ruleset: row.ruleset.clone(),
        based_on: r.based_on.clone(),
        groups,
        notes: r.notes.clone(),
        sources: r.sources.iter().map(|s| s.label.clone()).collect(),
    }
}

/// `€0.50` for an amount field, `3 years` for a count: how an edit's new value reads in its op label.
pub fn shown_value(
    kind: FieldKind,
    cents_or_count: i64,
    unit: Option<&str>,
    format: NumberFormat,
) -> String {
    match kind {
        FieldKind::Amount => format_money(&Money::eur(cents_or_count), format),
        FieldKind::Percent => format!("{cents_or_count} %"),
        FieldKind::Count => match unit {
            Some(unit) => format!("{cents_or_count} {unit}"),
            None => cents_or_count.to_string(),
        },
    }
}
