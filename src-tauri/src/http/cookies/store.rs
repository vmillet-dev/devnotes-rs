//! The jar's SQL. Every cookie is sealed whole, so matching and replacing happen in Rust over
//! the jar read entire — a jar holds tens of cookies, not thousands.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use specta::Type;
use uuid::Uuid;

use super::{Change, JarCookie};
use crate::db::schema::http_cookies;
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::vault::key::Vault;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StoredCookie {
    pub id: String,
    pub cookie: JarCookie,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CookieDomain {
    pub domain: String,
    pub cookies: Vec<StoredCookie>,
}

fn open(vault: &Vault, id: String, sealed: &str) -> Result<StoredCookie, StorageError> {
    let cookie =
        serde_json::from_str(&vault.open(sealed)?).map_err(|_| StorageError::CorruptRow {
            id: id.clone(),
            field: "cookie",
        })?;
    Ok(StoredCookie { id, cookie })
}

fn read(
    connection: &mut SqliteConnection,
    vault: &Vault,
) -> Result<Vec<StoredCookie>, StorageError> {
    http_cookies::table
        .select((http_cookies::id, http_cookies::cookie))
        .load::<(String, String)>(connection)?
        .into_iter()
        .map(|(id, sealed)| open(vault, id, &sealed))
        .collect()
}

/// The jar, what has expired left out.
pub fn jar(connection: &mut Library, now: DateTime<Utc>) -> Result<Vec<JarCookie>, StorageError> {
    let (connection, vault) = connection.split();
    Ok(read(connection, vault)?
        .into_iter()
        .map(|stored| stored.cookie)
        .filter(|cookie| cookie.expires.is_none_or(|expires| expires > now))
        .collect())
}

/// An answer's changes, in one transaction: each replaces the same cookie, an expired one
/// removes it, and what has expired since goes too.
pub fn apply(
    connection: &mut Library,
    changes: &[Change],
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    if changes.is_empty() {
        return Ok(());
    }
    connection.transaction(|connection, vault| {
        let mut known = read(connection, vault)?;
        for change in changes {
            let (Change::Set(cookie) | Change::Remove(cookie)) = change;
            for stale in known.iter().filter(|stored| stored.cookie.same_as(cookie)) {
                diesel::delete(http_cookies::table.find(&stale.id)).execute(connection)?;
            }
            known.retain(|stored| !stored.cookie.same_as(cookie));
            if let Change::Set(cookie) = change {
                let id = Uuid::new_v4().to_string();
                let json = serde_json::to_string(cookie)
                    .map_err(|error| StorageError::Vault(format!("unwritable cookie: {error}")))?;
                diesel::insert_into(http_cookies::table)
                    .values((
                        http_cookies::id.eq(&id),
                        http_cookies::expires_at.eq(cookie.expires.map(iso8601::format)),
                        http_cookies::cookie.eq(vault.seal(&json)?),
                    ))
                    .execute(connection)?;
                known.push(StoredCookie {
                    id,
                    cookie: cookie.clone(),
                });
            }
        }
        diesel::delete(
            http_cookies::table.filter(http_cookies::expires_at.le(iso8601::format(now))),
        )
        .execute(connection)?;
        Ok(())
    })
}

/// By domain, then name: what the manager lists.
pub fn by_domain(connection: &mut Library) -> Result<Vec<CookieDomain>, StorageError> {
    let (connection, vault) = connection.split();
    let mut cookies = read(connection, vault)?;
    cookies.sort_by(|a, b| {
        (&a.cookie.domain, &a.cookie.name, &a.cookie.path).cmp(&(
            &b.cookie.domain,
            &b.cookie.name,
            &b.cookie.path,
        ))
    });
    let mut domains: Vec<CookieDomain> = Vec::new();
    for stored in cookies {
        match domains.last_mut() {
            Some(last) if last.domain == stored.cookie.domain => last.cookies.push(stored),
            _ => domains.push(CookieDomain {
                domain: stored.cookie.domain.clone(),
                cookies: vec![stored],
            }),
        }
    }
    Ok(domains)
}

fn removed(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

pub fn delete(connection: &mut Library, id: &str) -> Result<u32, StorageError> {
    let (connection, _) = connection.split();
    Ok(removed(
        diesel::delete(http_cookies::table.find(id)).execute(connection)?,
    ))
}

/// A domain's cookies, answering how many went.
pub fn delete_domain(connection: &mut Library, domain: &str) -> Result<u32, StorageError> {
    let (connection, vault) = connection.split();
    let ids: Vec<String> = read(connection, vault)?
        .into_iter()
        .filter(|stored| stored.cookie.domain == domain)
        .map(|stored| stored.id)
        .collect();
    Ok(removed(
        diesel::delete(http_cookies::table.filter(http_cookies::id.eq_any(ids)))
            .execute(connection)?,
    ))
}

pub fn count(connection: &mut Library) -> Result<u32, StorageError> {
    let (connection, _) = connection.split();
    let count: i64 = http_cookies::table.count().get_result(connection)?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

pub fn clear(connection: &mut Library) -> Result<u32, StorageError> {
    let (connection, _) = connection.split();
    Ok(removed(
        diesel::delete(http_cookies::table).execute(connection)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn cookie(name: &str, domain: &str, value: &str) -> JarCookie {
        JarCookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            host_only: false,
            path: "/".to_string(),
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
        }
    }

    #[test]
    fn a_cookie_set_again_replaces_itself_and_one_expired_goes() {
        let mut library = crate::db::open_in_memory().unwrap();
        let now: DateTime<Utc> = "2026-10-07T12:00:00Z".parse().unwrap();
        apply(
            &mut library,
            &[
                Change::Set(cookie("session", "a.fr", "1")),
                Change::Set(cookie("theme", "a.fr", "dark")),
                Change::Set(JarCookie {
                    expires: Some(now + TimeDelta::seconds(10)),
                    ..cookie("brief", "b.fr", "x")
                }),
            ],
            now,
        )
        .unwrap();
        apply(
            &mut library,
            &[
                Change::Set(cookie("session", "a.fr", "2")),
                Change::Remove(cookie("theme", "a.fr", "")),
            ],
            now + TimeDelta::seconds(20),
        )
        .unwrap();

        let domains = by_domain(&mut library).unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].cookies.len(), 1);
        assert_eq!(domains[0].cookies[0].cookie.value, "2");
        assert_eq!(jar(&mut library, now).unwrap().len(), 1);

        apply(&mut library, &[Change::Set(cookie("x", "b.fr", "1"))], now).unwrap();
        assert_eq!(delete_domain(&mut library, "a.fr").unwrap(), 1);
        let id = by_domain(&mut library).unwrap()[0].cookies[0].id.clone();
        assert_eq!(delete(&mut library, &id).unwrap(), 1);
        assert_eq!(count(&mut library).unwrap(), 0);
    }
}
