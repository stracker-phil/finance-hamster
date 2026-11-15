//! Bank accounts: name, bank, IBAN and whether it is one of the owner's own (transfers between own accounts
//! never count twice). The IBAN is stored whole, to match statements, and shown masked.

pub mod commands;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::tr;

pub const TABLE: &str = "accounts";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AccountRow {
    pub id: String,
    pub name: String,
    pub bank: String,
    /// Upper case, without spaces.
    pub iban: String,
    pub own: bool,
    pub created_at: i64,
    /// What other accounts' statements print for this one (`Haushaltskonto`); None when not set.
    #[serde(default)]
    pub statement_name: Option<String>,
}

/// An account as the page shows it.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub bank: String,
    /// `AT•• •••• 0001`.
    pub iban_masked: String,
    pub own: bool,
    pub statement_name: Option<String>,
}

#[derive(Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct NewAccount {
    pub name: String,
    pub bank: String,
    pub iban: String,
    pub own: bool,
    #[serde(default)]
    pub statement_name: Option<String>,
}

/// The fields to change; the IBAN stays, as statements are matched by it.
#[derive(Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AccountEdit {
    pub name: String,
    pub bank: String,
    pub own: bool,
    #[serde(default)]
    pub statement_name: Option<String>,
}

/// Upper case without spaces, when it has a valid IBAN check sum.
pub fn normalize_iban(text: &str) -> Option<String> {
    let iban: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    let shape = (15..=34).contains(&iban.len())
        && iban.chars().all(|c| c.is_ascii_alphanumeric())
        && iban[..2].chars().all(|c| c.is_ascii_alphabetic())
        && iban[2..4].chars().all(|c| c.is_ascii_digit());
    if !shape {
        return None;
    }
    let rearranged = iban[4..].chars().chain(iban[..4].chars());
    let mut remainder = 0u32;
    for c in rearranged {
        let value = c.to_digit(36)?;
        remainder = if value < 10 {
            (remainder * 10 + value) % 97
        } else {
            (remainder * 100 + value) % 97
        };
    }
    (remainder == 1).then_some(iban)
}

/// The country code and the last four digits: `AT•• •••• 0001`.
pub fn mask_iban(iban: &str) -> String {
    let country = iban.get(..2).unwrap_or_default();
    let last = iban.get(iban.len().saturating_sub(4)..).unwrap_or_default();
    format!("{country}•• •••• {last}")
}

fn view(row: AccountRow) -> Account {
    Account {
        iban_masked: mask_iban(&row.iban),
        id: row.id,
        name: row.name,
        bank: row.bank,
        own: row.own,
        statement_name: row.statement_name,
    }
}

/// Trimmed, None when empty.
fn optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

const COLUMNS: &str = "id, name, bank, iban, own, created_at, statement_name";

fn from_sql(row: &rusqlite::Row) -> rusqlite::Result<AccountRow> {
    Ok(AccountRow {
        id: row.get(0)?,
        name: row.get(1)?,
        bank: row.get(2)?,
        iban: row.get(3)?,
        own: row.get(4)?,
        created_at: row.get(5)?,
        statement_name: row.get(6)?,
    })
}

pub fn rows(conn: &Connection) -> AppResult<Vec<AccountRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM {TABLE} ORDER BY created_at, name"
    ))?;
    let rows = query.query_map([], from_sql)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn row(conn: &Connection, id: &str) -> AppResult<Option<AccountRow>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM {TABLE} WHERE id = ?"),
            [id],
            from_sql,
        )
        .optional()?)
}

/// Every account in the order it was added.
pub fn list(db: &Db) -> AppResult<Vec<Account>> {
    Ok(db.read(rows)?.into_iter().map(view).collect())
}

fn required_name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidField {
            field: "account.name".into(),
            message: tr!("The name can't be empty", "Der Name darf nicht leer sein"),
        });
    }
    Ok(value.to_string())
}

pub fn add(db: &Db, input: NewAccount, now: i64) -> AppResult<(Account, Written)> {
    let name = required_name(&input.name)?;
    let iban = normalize_iban(&input.iban).ok_or_else(|| AppError::InvalidField {
        field: "account.iban".into(),
        message: tr!("This isn't a valid IBAN", "Das ist keine gültige IBAN"),
    })?;
    if db.read(rows)?.iter().any(|a| a.iban == iban) {
        return Err(AppError::InvalidField {
            field: "account.iban".into(),
            message: tr!(
                "An account with this IBAN exists",
                "Ein Konto mit dieser IBAN gibt es schon"
            ),
        });
    }
    let row = AccountRow {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        bank: input.bank.trim().to_string(),
        iban,
        own: input.own,
        created_at: now,
        statement_name: optional(input.statement_name),
    };
    let op = Op::new(
        tr!("Add Account {name}", "Konto {name} hinzufügen"),
        vec![Change::insert(TABLE, &row)?],
    );
    let written = db.write(op, Source::Ui)?;
    Ok((view(row), written))
}

pub fn edit(db: &Db, id: &str, input: AccountEdit) -> AppResult<Written> {
    let before = db
        .read(|c| row(c, id))?
        .ok_or_else(|| AppError::NotFound(tr!("No account {id}", "Kein Konto {id}")))?;
    let after = AccountRow {
        name: required_name(&input.name)?,
        bank: input.bank.trim().to_string(),
        own: input.own,
        statement_name: optional(input.statement_name),
        ..before.clone()
    };
    let mut changes = vec![Change::update(TABLE, &before, &after)?];
    // Which lines are transfers between own accounts follows the accounts; the rules decide them again.
    if (before.own, &before.statement_name) != (after.own, &after.statement_name) {
        let accounts: Vec<AccountRow> = db
            .read(rows)?
            .into_iter()
            .map(|a| if a.id == after.id { after.clone() } else { a })
            .collect();
        let own = crate::rules::apply::own_of(accounts);
        let now = chrono::Utc::now().timestamp_millis();
        changes.extend(db.read(|c| crate::rules::apply::changes_with_own(c, &own, now))?);
    }
    let op = Op::new(
        tr!("Edit Account {}", "Konto {} bearbeiten", after.name),
        changes,
    );
    db.write(op, Source::Ui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    const PRIVATE: &str = "AT241999900000000001";
    const HOUSEHOLD: &str = "AT941999900000000002";

    fn open() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    fn new_account(name: &str, iban: &str) -> NewAccount {
        NewAccount {
            name: name.into(),
            bank: "Example Sparkasse".into(),
            iban: iban.into(),
            own: true,
            statement_name: None,
        }
    }

    fn edit_to(name: &str, bank: &str, own: bool) -> AccountEdit {
        AccountEdit {
            name: name.into(),
            bank: bank.into(),
            own,
            statement_name: None,
        }
    }

    fn named(account: &Account, statement_name: Option<&str>) -> AccountEdit {
        AccountEdit {
            statement_name: statement_name.map(Into::into),
            ..edit_to(&account.name, &account.bank, account.own)
        }
    }

    fn field_error(field: &str, message: &str) -> AppError {
        AppError::InvalidField {
            field: field.into(),
            message: message.into(),
        }
    }

    #[test]
    fn spaces_and_lower_case_are_accepted_in_an_iban() {
        for (input, expected) in [
            (PRIVATE, PRIVATE),
            ("AT24 1999 9000 0000 0001", PRIVATE),
            ("at24 1999 9000 0000 0001", PRIVATE),
            ("  at941999900000000002\n", HOUSEHOLD),
        ] {
            assert_eq!(
                normalize_iban(input),
                Some(expected.to_string()),
                "{input:?}"
            );
        }
    }

    #[test]
    fn an_iban_with_wrong_check_digits_or_a_bad_shape_is_rejected() {
        for input in [
            "AT251999900000000001",
            "AT241999900000000002",
            "",
            "AT24",
            "AT24 1999 9000",
            "1224 1999 9000 0000 0001",
            "ATXX1999900000000001",
            "AT24199990000000000!",
            "AT24199990000000000é",
            "AT2419999000000000011999900000000001999",
        ] {
            assert_eq!(normalize_iban(input), None, "{input:?}");
        }
    }

    #[test]
    fn a_masked_iban_shows_the_country_and_the_last_four_digits() {
        assert_eq!(mask_iban(PRIVATE), "AT•• •••• 0001");
        assert_eq!(mask_iban(HOUSEHOLD), "AT•• •••• 0002");
    }

    #[test]
    fn adding_an_account_trims_it_stores_the_iban_whole_and_lists_it_masked() {
        let (_dir, db) = open();
        let input = NewAccount {
            name: "  Private  ".into(),
            bank: " Example Sparkasse ".into(),
            iban: "at24 1999 9000 0000 0001".into(),
            own: true,
            statement_name: None,
        };

        let (account, written) = add(&db, input, 10).unwrap();

        assert_eq!(
            written,
            Written {
                label: "Add Account Private".into(),
                tables: vec!["accounts".into()]
            }
        );
        let expected = Account {
            id: account.id.clone(),
            name: "Private".into(),
            bank: "Example Sparkasse".into(),
            iban_masked: "AT•• •••• 0001".into(),
            own: true,
            statement_name: None,
        };
        assert_eq!(account, expected);
        assert_eq!(list(&db).unwrap(), vec![expected]);
        let stored = db.read(|c| row(c, &account.id)).unwrap().unwrap();
        assert_eq!(
            stored,
            AccountRow {
                id: account.id,
                name: "Private".into(),
                bank: "Example Sparkasse".into(),
                iban: PRIVATE.into(),
                own: true,
                created_at: 10,
                statement_name: None,
            }
        );
    }

    #[test]
    fn an_account_can_be_marked_as_not_own() {
        let (_dir, db) = open();
        let input = NewAccount {
            own: false,
            ..new_account("Partner", HOUSEHOLD)
        };

        let (account, _) = add(&db, input, 1).unwrap();

        assert!(!account.own);
    }

    #[test]
    fn an_empty_name_is_refused_on_the_name_field() {
        let (_dir, db) = open();
        for name in ["", "   "] {
            let err = add(&db, new_account(name, PRIVATE), 1).unwrap_err();
            assert_eq!(
                err,
                field_error("account.name", "The name can't be empty"),
                "{name:?}"
            );
        }
        assert_eq!(list(&db).unwrap(), vec![]);
    }

    #[test]
    fn an_invalid_iban_is_refused_on_the_iban_field() {
        let (_dir, db) = open();
        let err = add(&db, new_account("Private", "AT251999900000000001"), 1).unwrap_err();
        assert_eq!(err, field_error("account.iban", "This isn't a valid IBAN"));
        assert_eq!(list(&db).unwrap(), vec![]);
    }

    #[test]
    fn an_iban_that_exists_is_refused_however_it_is_written() {
        let (_dir, db) = open();
        add(&db, new_account("Private", PRIVATE), 1).unwrap();

        for iban in [PRIVATE, "at24 1999 9000 0000 0001"] {
            let err = add(&db, new_account("Copy", iban), 2).unwrap_err();
            assert_eq!(
                err,
                field_error("account.iban", "An account with this IBAN exists"),
                "{iban}"
            );
        }
        assert_eq!(list(&db).unwrap().len(), 1);
    }

    #[test]
    fn the_name_is_checked_before_the_iban() {
        let (_dir, db) = open();
        let err = add(&db, new_account("", "nonsense"), 1).unwrap_err();
        assert_eq!(err, field_error("account.name", "The name can't be empty"));
    }

    #[test]
    fn undoing_an_add_removes_the_account() {
        let (_dir, db) = open();
        add(&db, new_account("Private", PRIVATE), 1).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(list(&db).unwrap(), vec![]);
    }

    #[test]
    fn accounts_list_in_the_order_they_were_added() {
        let (_dir, db) = open();
        add(&db, new_account("Zebra", PRIVATE), 1).unwrap();
        add(&db, new_account("Alpha", HOUSEHOLD), 2).unwrap();

        let names: Vec<String> = list(&db).unwrap().into_iter().map(|a| a.name).collect();

        assert_eq!(names, vec!["Zebra".to_string(), "Alpha".to_string()]);
    }

    #[test]
    fn editing_changes_name_bank_and_own_but_keeps_the_iban() {
        let (_dir, db) = open();
        let (account, _) = add(&db, new_account("Private", PRIVATE), 1).unwrap();

        let written = edit(
            &db,
            &account.id,
            edit_to("  Daily  ", " Example Bank ", false),
        )
        .unwrap();

        assert_eq!(
            written,
            Written {
                label: "Edit Account Daily".into(),
                tables: vec!["accounts".into()]
            }
        );
        assert_eq!(
            list(&db).unwrap(),
            vec![Account {
                id: account.id.clone(),
                name: "Daily".into(),
                bank: "Example Bank".into(),
                iban_masked: "AT•• •••• 0001".into(),
                own: false,
                statement_name: None,
            }]
        );
        let stored = db.read(|c| row(c, &account.id)).unwrap().unwrap();
        assert_eq!(stored.iban, PRIVATE);
        assert_eq!(stored.created_at, 1);
    }

    #[test]
    fn undoing_an_edit_restores_the_account() {
        let (_dir, db) = open();
        let (account, _) = add(&db, new_account("Private", PRIVATE), 1).unwrap();
        edit(&db, &account.id, edit_to("Daily", "Other", false)).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(list(&db).unwrap(), vec![account]);
    }

    #[test]
    fn editing_an_unknown_account_is_not_found() {
        let (_dir, db) = open();
        let err = edit(&db, "nope", edit_to("Daily", "Bank", true)).unwrap_err();
        assert_eq!(err, AppError::NotFound("No account nope".into()));
    }

    #[test]
    fn editing_to_an_empty_name_is_refused_and_changes_nothing() {
        let (_dir, db) = open();
        let (account, _) = add(&db, new_account("Private", PRIVATE), 1).unwrap();

        let err = edit(&db, &account.id, edit_to("  ", "Other", false)).unwrap_err();

        assert_eq!(err, field_error("account.name", "The name can't be empty"));
        assert_eq!(list(&db).unwrap(), vec![account]);
    }

    #[test]
    fn an_account_is_sent_to_the_page_in_camel_case() {
        let account = Account {
            id: "a".into(),
            name: "Private".into(),
            bank: "Example Sparkasse".into(),
            iban_masked: "AT•• •••• 0001".into(),
            own: true,
            statement_name: Some("Alex Beispiel".into()),
        };
        assert_eq!(
            serde_json::to_value(&account).unwrap(),
            serde_json::json!({
                "id": "a",
                "name": "Private",
                "bank": "Example Sparkasse",
                "ibanMasked": "AT•• •••• 0001",
                "own": true,
                "statementName": "Alex Beispiel"
            })
        );
    }

    #[test]
    fn a_statement_name_is_trimmed_and_an_empty_one_is_none_on_add_and_edit() {
        let (_dir, db) = open();
        for (input, expected) in [
            (Some("  Haushalt  "), Some("Haushalt")),
            (Some("   "), None),
            (Some(""), None),
            (None, None),
        ] {
            let (account, _) = add(
                &db,
                NewAccount {
                    statement_name: input.map(Into::into),
                    ..new_account("Account", PRIVATE)
                },
                1,
            )
            .unwrap();
            assert_eq!(account.statement_name.as_deref(), expected, "add {input:?}");
            assert_eq!(
                list(&db)
                    .unwrap()
                    .iter()
                    .find(|a| a.id == account.id)
                    .and_then(|a| a.statement_name.clone())
                    .as_deref(),
                expected,
                "listed {input:?}"
            );
            db.undo().unwrap().unwrap();
        }

        let (account, _) = add(&db, new_account("Private", PRIVATE), 1).unwrap();
        for (input, expected) in [
            (Some("  Alex Beispiel "), Some("Alex Beispiel")),
            (Some("  "), None),
        ] {
            edit(&db, &account.id, named(&account, input)).unwrap();
            let stored = db.read(|c| row(c, &account.id)).unwrap().unwrap();
            assert_eq!(stored.statement_name.as_deref(), expected, "edit {input:?}");
        }
    }

    #[test]
    fn editing_only_the_name_of_an_account_writes_only_the_accounts_table() {
        let (_dir, db) = open();
        let (account, _) = add(&db, new_account("Private", PRIVATE), 1).unwrap();

        let written = edit(&db, &account.id, edit_to("Daily", &account.bank, true)).unwrap();

        assert_eq!(written.tables, vec!["accounts".to_string()]);
    }
}
