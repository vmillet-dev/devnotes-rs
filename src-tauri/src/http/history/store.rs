//! The history's SQL: written after each send, purged by age and by count as it is written.

use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::{
    HistoryEntry, HistoryItem, HistoryRecord, HistorySummary, KEPT_AT_MOST, RETENTION_DAYS,
    url_parts,
};
use crate::db::schema::http_history;
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::vault::key::Vault;

#[derive(Queryable, Selectable)]
#[diesel(table_name = http_history, check_for_backend(diesel::sqlite::Sqlite))]
struct Row {
    id: String,
    sent_at: String,
    request_id: Option<String>,
    summary: String,
}

fn corrupt(id: &str, field: &'static str) -> StorageError {
    StorageError::CorruptRow {
        id: id.to_string(),
        field,
    }
}

fn open_json<T: serde::de::DeserializeOwned>(
    vault: &Vault,
    id: &str,
    field: &'static str,
    sealed: &str,
) -> Result<T, StorageError> {
    serde_json::from_str(&vault.open(sealed)?).map_err(|_| corrupt(id, field))
}

fn seal_json<T: serde::Serialize>(vault: &Vault, value: &T) -> Result<String, StorageError> {
    let json = serde_json::to_string(value)
        .map_err(|error| StorageError::Vault(format!("unwritable history: {error}")))?;
    vault.seal(&json)
}

impl Row {
    fn open(self, vault: &Vault) -> Result<HistoryItem, StorageError> {
        let summary: HistorySummary = open_json(vault, &self.id, "summary", &self.summary)?;
        let (url_head, url_tail) = url_parts(&summary.url);
        Ok(HistoryItem {
            url_head,
            url_tail,
            sent_at: iso8601::parse(&self.sent_at).map_err(|_| corrupt(&self.id, "sentAt"))?,
            method: summary.method,
            status: summary.status,
            summary,
            request_id: self.request_id,
            id: self.id,
        })
    }
}

/// A request deleted since it was sent is history without its link.
fn still_there<'a>(
    connection: &mut SqliteConnection,
    request_id: Option<&'a str>,
) -> Option<&'a str> {
    request_id.filter(|id| {
        crate::db::schema::http_requests::table
            .find(*id)
            .count()
            .get_result::<i64>(connection)
            .is_ok_and(|count| count > 0)
    })
}

/// Writes the entry, then lets go of what is past the retention or the count.
pub fn record(
    connection: &mut Library,
    request_id: Option<&str>,
    summary: &HistorySummary,
    record: &HistoryRecord,
    now: DateTime<Utc>,
) -> Result<String, StorageError> {
    connection.transaction(|connection, vault| {
        let id = Uuid::now_v7().to_string();
        let request_id = still_there(connection, request_id);
        diesel::insert_into(http_history::table)
            .values((
                http_history::id.eq(&id),
                http_history::sent_at.eq(iso8601::format(now)),
                http_history::request_id.eq(request_id),
                http_history::summary.eq(seal_json(vault, summary)?),
                http_history::record.eq(seal_json(vault, record)?),
            ))
            .execute(connection)?;
        purge(connection, now)?;
        Ok(id)
    })
}

/// One entry gone, answered whole: what its undo hands back to `restore`.
pub fn delete(connection: &mut Library, id: &str) -> Result<HistoryEntry, StorageError> {
    let removed = entry(connection, id)?;
    let (connection, _) = connection.split();
    diesel::delete(http_history::table.find(id)).execute(connection)?;
    Ok(removed)
}

/// Puts back an entry `delete` answered, under its id and at its instant: it takes its place
/// in the list again. One still there is left as it is.
pub fn restore(connection: &mut Library, entry: &HistoryEntry) -> Result<(), StorageError> {
    connection.transaction(|connection, vault| {
        let item = &entry.item;
        let request_id = still_there(connection, item.request_id.as_deref());
        diesel::insert_or_ignore_into(http_history::table)
            .values((
                http_history::id.eq(&item.id),
                http_history::sent_at.eq(iso8601::format(item.sent_at)),
                http_history::request_id.eq(request_id),
                http_history::summary.eq(seal_json(vault, &item.summary)?),
                http_history::record.eq(seal_json(vault, &entry.record)?),
            ))
            .execute(connection)?;
        Ok(())
    })
}

/// Older than the retention, or past the count from the newest.
fn purge(connection: &mut SqliteConnection, now: DateTime<Utc>) -> Result<usize, StorageError> {
    let horizon = iso8601::format(now - TimeDelta::days(RETENTION_DAYS));
    let aged = diesel::delete(http_history::table.filter(http_history::sent_at.lt(horizon)))
        .execute(connection)?;
    let kept: Vec<String> = http_history::table
        .select(http_history::id)
        .order((http_history::sent_at.desc(), http_history::id.desc()))
        .limit(KEPT_AT_MOST)
        .load(connection)?;
    let counted = diesel::delete(http_history::table.filter(http_history::id.ne_all(kept)))
        .execute(connection)?;
    Ok(aged + counted)
}

pub fn purge_expired(connection: &mut Library, now: DateTime<Utc>) -> Result<usize, StorageError> {
    let (connection, _) = connection.split();
    purge(connection, now)
}

/// Newest first.
pub fn list(connection: &mut Library) -> Result<Vec<HistoryItem>, StorageError> {
    let (connection, vault) = connection.split();
    http_history::table
        .select(Row::as_select())
        .order((http_history::sent_at.desc(), http_history::id.desc()))
        .load(connection)?
        .into_iter()
        .map(|row| row.open(vault))
        .collect()
}

pub fn entry(connection: &mut Library, id: &str) -> Result<HistoryEntry, StorageError> {
    let (connection, vault) = connection.split();
    let (row, sealed) = http_history::table
        .find(id)
        .select((Row::as_select(), http_history::record))
        .first::<(Row, String)>(connection)
        .optional()?
        .ok_or_else(|| StorageError::HttpItemNotFound(id.to_string()))?;
    let record = open_json(vault, id, "record", &sealed)?;
    Ok(HistoryEntry {
        item: row.open(vault)?,
        record,
    })
}

pub fn count(connection: &mut Library) -> Result<u32, StorageError> {
    let (connection, _) = connection.split();
    let count: i64 = http_history::table.count().get_result(connection)?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Everything, answering how many went.
pub fn clear(connection: &mut Library) -> Result<u32, StorageError> {
    let (connection, _) = connection.split();
    let removed = diesel::delete(http_history::table).execute(connection)?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::history::{HistoryRecord, HistorySummary};
    use crate::http::model::HttpMethod;
    use crate::http::send::{Exchange, SentBody};

    fn summary() -> HistorySummary {
        HistorySummary {
            kind: crate::http::model::RequestKind::Http,
            name: "Ping".to_string(),
            url: "https://api.exemple.fr".to_string(),
            millis: Some(1),
            size: Some(0),
            failure: None,
            failure_params: std::collections::BTreeMap::new(),
            method: HttpMethod::Delete,
            status: Some(401),
        }
    }

    fn record() -> HistoryRecord {
        HistoryRecord {
            exchange: Exchange {
                method: HttpMethod::Get,
                url: "https://api.exemple.fr".to_string(),
                headers: Vec::new(),
                body: SentBody::None,
            },
            response: None,
        }
    }

    #[test]
    fn what_is_past_the_retention_or_the_count_goes_as_the_next_is_written() {
        let mut library = crate::db::open_in_memory().unwrap();
        let now: DateTime<Utc> = "2026-10-07T12:00:00Z".parse().unwrap();
        let long_ago = now - TimeDelta::days(RETENTION_DAYS + 1);
        record_at(&mut library, long_ago);
        assert_eq!(count(&mut library).unwrap(), 1);

        for second in 0..=KEPT_AT_MOST {
            record_at(&mut library, now + TimeDelta::seconds(second));
        }

        assert_eq!(i64::from(count(&mut library).unwrap()), KEPT_AT_MOST);
        let newest = list(&mut library).unwrap();
        assert_eq!(newest[0].sent_at, now + TimeDelta::seconds(KEPT_AT_MOST));
        assert_eq!(
            purge_expired(&mut library, now + TimeDelta::days(RETENTION_DAYS + 1)).unwrap(),
            usize::try_from(KEPT_AT_MOST).unwrap()
        );
    }

    /// « Vider l'historique » was the only way out for the one send made with the wrong token.
    #[test]
    fn one_entry_is_deleted_and_put_back_where_it_was() {
        let mut library = crate::db::open_in_memory().unwrap();
        let now: DateTime<Utc> = "2026-10-07T12:00:00Z".parse().unwrap();
        for second in 0..3 {
            record_at(&mut library, now + TimeDelta::seconds(second));
        }
        let before = list(&mut library).unwrap();
        let middle = before[1].id.clone();

        let removed = delete(&mut library, &middle).unwrap();
        assert_eq!(removed.item, before[1]);
        assert_eq!(
            list(&mut library)
                .unwrap()
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            [before[0].id.as_str(), before[2].id.as_str()]
        );
        assert!(matches!(
            delete(&mut library, &middle),
            Err(StorageError::HttpItemNotFound(_))
        ));

        restore(&mut library, &removed).unwrap();
        // Put back twice is put back once.
        restore(&mut library, &removed).unwrap();
        assert_eq!(list(&mut library).unwrap(), before);
        assert_eq!(entry(&mut library, &middle).unwrap(), removed);
    }

    #[test]
    fn a_list_reads_each_url_in_its_two_parts() {
        let mut library = crate::db::open_in_memory().unwrap();
        record_at(&mut library, "2026-10-07T12:00:00Z".parse().unwrap());

        let item = &list(&mut library).unwrap()[0];
        let (head, tail) = url_parts(&item.summary.url);
        assert_eq!((&item.url_head, &item.url_tail), (&head, &tail));
        assert!(!item.url_head.is_empty());
    }

    /// Both were columns of their own, readable beside the instant of the send.
    #[test]
    fn the_method_and_the_status_of_a_send_are_read_from_its_sealed_summary() {
        let mut library = crate::db::open_in_memory().unwrap();
        record_at(&mut library, "2026-10-07T12:00:00Z".parse().unwrap());

        let item = list(&mut library).unwrap().remove(0);
        assert_eq!((item.method, item.status), (HttpMethod::Delete, Some(401)));

        let removed = delete(&mut library, &item.id).unwrap();
        restore(&mut library, &removed).unwrap();
        let back = entry(&mut library, &item.id).unwrap().item;
        assert_eq!((back.method, back.status), (HttpMethod::Delete, Some(401)));
    }

    fn record_at(library: &mut Library, at: DateTime<Utc>) {
        super::record(library, Some("gone"), &summary(), &record(), at).unwrap();
    }
}
