//! A closed year shows what it was filed with. When the data now gives another Kennzahl value (a subscription's
//! item changed, a rules file edited), the snapshot's value stays the copy value and the page says what moved.

use rusqlite::OptionalExtension;

use super::close::{SNAPSHOTS, SnapshotKennzahl};
use super::page::{AttentionKind, AttentionView, KennzahlView};
use crate::db::Db;
use crate::error::AppResult;
use crate::money::parse_cents;

/// Puts the newest snapshot's values back into `kennzahlen` and names each one the data now disagrees with.
pub fn restore_filed(
    db: &Db,
    year: i32,
    kennzahlen: &mut [KennzahlView],
) -> AppResult<Vec<AttentionView>> {
    let Some(filed) = filed(db, year)? else {
        return Ok(Vec::new());
    };
    let mut changed = Vec::new();
    for k in kennzahlen.iter_mut().filter(|k| !k.reported) {
        let value = filed
            .iter()
            .find(|f| f.kz == k.kz)
            .map_or_else(|| "0,00".to_string(), |f| f.copy.clone());
        if parse_cents(&value) == parse_cents(&k.copy) {
            continue;
        }
        changed.push(AttentionView {
            kind: AttentionKind::ChangedSinceClosing,
            title: crate::tr!(
                "KZ {} changed since closing",
                "KZ {} hat sich seit dem Abschluss geändert",
                k.kz
            ),
            detail: crate::tr!(
                "Filed {value}; the data now gives {}. Reopen the year to look into it.",
                "Eingereicht {value}; die Daten ergeben jetzt {}. Öffne das Jahr wieder, um es zu prüfen.",
                k.copy
            ),
            document_id: None,
            entry_id: None,
            line_id: None,
        });
        k.copy = value;
    }
    Ok(changed)
}

fn filed(db: &Db, year: i32) -> AppResult<Option<Vec<SnapshotKennzahl>>> {
    let json: Option<String> = db.read(|c| {
        Ok(c.query_row(
            &format!(
                "SELECT kennzahlen FROM {SNAPSHOTS} WHERE year = ? ORDER BY closed_at DESC LIMIT 1"
            ),
            [year],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    json.map(|j| serde_json::from_str(&j).map_err(Into::into))
        .transpose()
}
