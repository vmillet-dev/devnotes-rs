//! The history's SQL: written after each send, purged by age and by count as it is written.

use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::{
    HistoryEntry, HistoryItem, HistoryRecord, HistorySummary, KEPT_AT_MOST, RETENTION_DAYS,
};
use crate::db::schema::http_history;
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::http::model::HttpMethod;
use crate::vault::key::Vault;

#[derive(Queryable, Selectable)]
#[diesel(table_name = http_history, check_for_backend(diesel::sqlite::Sqlite))]
struct Row {
    id: String,
    sent_at: String,
    request_id: Option<String>,
    method: String,
    status: Option<i32>,
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
        Ok(HistoryItem {
            sent_at: iso8601::parse(&self.sent_at).map_err(|_| corrupt(&self.id, "sentAt"))?,
            method: self
                .method
                .parse::<HttpMethod>()
                .map_err(|()| corrupt(&self.id, "method"))?,
            status: self.status.and_then(|status| u16::try_from(status).ok()),
            summary: open_json(vault, &self.id, "summary", &self.summary)?,
            request_id: self.request_id,
            id: self.id,
        })
    }
}

/// Writes the entry, then lets go of what is past the retention or the count.
pub fn record(
    connection: &mut Library,
    method: HttpMethod,
    request_id: Option<&str>,
    summary: &HistorySummary,
    record: &HistoryRecord,
    now: DateTime<Utc>,
) -> Result<String, StorageError> {
    connection.transaction(|connection, vault| {
        let id = Uuid::now_v7().to_string();
        // A request deleted since it was sent is history without its link.
        let request_id = request_id.filter(|id| {
            crate::db::schema::http_requests::table
                .find(*id)
                .count()
                .get_result::<i64>(connection)
                .is_ok_and(|count| count > 0)
        });
        diesel::insert_into(http_history::table)
            .values((
                http_history::id.eq(&id),
                http_history::sent_at.eq(iso8601::format(now)),
                http_history::request_id.eq(request_id),
                http_history::method.eq(method.as_str()),
                http_history::status.eq(record
                    .response
                    .as_ref()
                    .map(|response| i32::from(response.status))),
                http_history::summary.eq(seal_json(vault, summary)?),
                http_history::record.eq(seal_json(vault, record)?),
            ))
            .execute(connection)?;
        purge(connection, now)?;
        Ok(id)
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
    use crate::http::send::{Exchange, SentBody};

    fn summary() -> HistorySummary {
        HistorySummary {
            name: "Ping".to_string(),
            url: "https://api.exemple.fr".to_string(),
            millis: Some(1),
            size: Some(0),
            failure: None,
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

    fn record_at(library: &mut Library, at: DateTime<Utc>) {
        super::record(
            library,
            HttpMethod::Get,
            Some("gone"),
            &summary(),
            &record(),
            at,
        )
        .unwrap();
    }
}
