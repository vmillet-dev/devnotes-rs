//! The library's cookie jar: what answers set, and what requests send back. RFC 6265 as far as
//! a client for developers needs it — domain and path matching, expiry, `Secure`.

pub mod store;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use url::Url;

use super::model::KeyValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JarCookie {
    pub name: String,
    pub value: String,
    /// Lowercased, without a leading dot.
    pub domain: String,
    /// Set without `Domain`: sent to that host alone, not its subdomains.
    pub host_only: bool,
    pub path: String,
    /// `None` lasts the session — here, until removed.
    pub expires: Option<DateTime<Utc>>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
}

impl JarCookie {
    /// What makes two cookies the same one: a newer replaces it.
    pub fn same_as(&self, other: &Self) -> bool {
        self.name == other.name && self.domain == other.domain && self.path == other.path
    }

    fn expired(&self, now: DateTime<Utc>) -> bool {
        self.expires.is_some_and(|expires| expires <= now)
    }
}

/// What an answer does to the jar: an expiry in the past removes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Set(JarCookie),
    Remove(JarCookie),
}

fn domain_matches(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.ends_with('.'))
}

fn path_matches(path: &str, cookie: &str) -> bool {
    path == cookie
        || (path.starts_with(cookie)
            && (cookie.ends_with('/') || path[cookie.len()..].starts_with('/')))
}

/// The directory of the request's path: `/api/users` → `/api`.
fn default_path(url: &Url) -> String {
    let path = url.path();
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(at) => path[..at].to_string(),
    }
}

fn parse(line: &str, url: &Url, now: DateTime<Utc>) -> Option<Change> {
    let host = url.host_str()?.to_ascii_lowercase();
    let mut parts = line.split(';');
    let (name, value) = parts.next()?.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let mut cookie = JarCookie {
        name: name.to_string(),
        value: value.trim().trim_matches('"').to_string(),
        domain: host.clone(),
        host_only: true,
        path: default_path(url),
        expires: None,
        secure: false,
        http_only: false,
        same_site: None,
    };
    let mut max_age: Option<i64> = None;
    for attribute in parts {
        let (key, value) = attribute
            .split_once('=')
            .map_or((attribute.trim(), ""), |(key, value)| {
                (key.trim(), value.trim())
            });
        match key.to_ascii_lowercase().as_str() {
            "domain" if !value.is_empty() => {
                let domain = value.trim_start_matches('.').to_ascii_lowercase();
                // A server sets a cookie for itself or a domain above it, never for another.
                if !domain_matches(&host, &domain) {
                    return None;
                }
                cookie.domain = domain;
                cookie.host_only = false;
            }
            "path" if value.starts_with('/') => cookie.path = value.to_string(),
            "max-age" => max_age = value.parse().ok(),
            "expires" => {
                cookie.expires = DateTime::parse_from_rfc2822(&value.replace('-', " "))
                    .ok()
                    .map(|date| date.with_timezone(&Utc))
                    .or(cookie.expires);
            }
            "secure" => cookie.secure = true,
            "httponly" => cookie.http_only = true,
            "samesite" if !value.is_empty() => cookie.same_site = Some(value.to_string()),
            _ => {}
        }
    }
    // As browsers do: a plain connection cannot set what only a secure one may read.
    if cookie.secure && !is_secure(url) {
        return None;
    }
    if let Some(seconds) = max_age {
        cookie.expires = Some(now + TimeDelta::seconds(seconds.max(0)));
    }
    Some(if cookie.expired(now) {
        Change::Remove(cookie)
    } else {
        Change::Set(cookie)
    })
}

fn is_secure(url: &Url) -> bool {
    url.scheme() == "https" || url.scheme() == "wss"
}

/// What the answer's `Set-Cookie` headers do to the jar, in their order.
pub fn received(headers: &[KeyValue], url: &Url, now: DateTime<Utc>) -> Vec<Change> {
    headers
        .iter()
        .filter(|header| header.key.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|header| parse(&header.value, url, now))
        .collect()
}

/// The `Cookie` header a request to `url` carries, the longest paths first; `None` for none.
pub fn header_for(jar: &[JarCookie], url: &Url, now: DateTime<Utc>) -> Option<String> {
    let host = url.host_str()?.to_ascii_lowercase();
    let secure = is_secure(url);
    let mut sent: Vec<&JarCookie> = jar
        .iter()
        .filter(|cookie| !cookie.expired(now))
        .filter(|cookie| {
            if cookie.host_only {
                host == cookie.domain
            } else {
                domain_matches(&host, &cookie.domain)
            }
        })
        .filter(|cookie| path_matches(url.path(), &cookie.path))
        .filter(|cookie| secure || !cookie.secure)
        .collect();
    sent.sort_by_key(|cookie| std::cmp::Reverse(cookie.path.len()));
    (!sent.is_empty()).then(|| {
        sent.iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-10-07T12:00:00Z".parse().unwrap()
    }

    fn set_cookie(value: &str) -> KeyValue {
        KeyValue {
            key: "Set-Cookie".to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        }
    }

    fn set(change: Change) -> JarCookie {
        match change {
            Change::Set(cookie) => cookie,
            Change::Remove(cookie) => panic!("removed {cookie:?}"),
        }
    }

    #[test]
    fn a_cookie_takes_its_host_and_path_unless_it_names_a_domain_above() {
        let url = Url::parse("https://api.exemple.fr/v1/users").unwrap();
        let changes = received(
            &[
                set_cookie("session=abc; HttpOnly; Secure"),
                set_cookie("theme=dark; Domain=.EXEMPLE.fr; Path=/; Max-Age=3600; SameSite=Lax"),
                set_cookie("evil=1; Domain=other.fr"),
                set_cookie("old=1; Expires=Wed, 01 Oct 2025 07:28:00 GMT"),
            ],
            &url,
            now(),
        );

        assert_eq!(changes.len(), 3);
        let session = set(changes[0].clone());
        assert_eq!(
            (
                session.domain.as_str(),
                session.host_only,
                session.path.as_str()
            ),
            ("api.exemple.fr", true, "/v1")
        );
        assert!(session.secure && session.http_only && session.expires.is_none());
        let theme = set(changes[1].clone());
        assert_eq!(
            (theme.domain.as_str(), theme.host_only),
            ("exemple.fr", false)
        );
        assert_eq!(theme.expires, Some(now() + TimeDelta::hours(1)));
        assert!(matches!(&changes[2], Change::Remove(cookie) if cookie.name == "old"));

        let plain = Url::parse("http://api.exemple.fr/").unwrap();
        assert!(received(&[set_cookie("csrf=1; Secure")], &plain, now()).is_empty());
    }

    #[test]
    fn a_request_carries_what_matches_its_host_path_and_scheme_the_longest_paths_first() {
        let cookie =
            |name: &str, domain: &str, host_only: bool, path: &str, secure: bool| JarCookie {
                name: name.to_string(),
                value: "v".to_string(),
                domain: domain.to_string(),
                host_only,
                path: path.to_string(),
                expires: None,
                secure,
                http_only: false,
                same_site: None,
            };
        let jar = vec![
            cookie("root", "exemple.fr", false, "/", false),
            cookie("api", "api.exemple.fr", true, "/v1", false),
            cookie("secure", "exemple.fr", false, "/", true),
            cookie("elsewhere", "other.fr", false, "/", false),
            cookie("sibling", "exemple.fr", true, "/", false),
            JarCookie {
                expires: Some(now() - TimeDelta::seconds(1)),
                ..cookie("gone", "exemple.fr", false, "/", false)
            },
        ];

        let plain = Url::parse("http://api.exemple.fr/v1/users").unwrap();
        assert_eq!(
            header_for(&jar, &plain, now()).as_deref(),
            Some("api=v; root=v")
        );
        let secure = Url::parse("https://api.exemple.fr/v10").unwrap();
        assert_eq!(
            header_for(&jar, &secure, now()).as_deref(),
            Some("root=v; secure=v")
        );
        let other = Url::parse("https://nowhere.fr/").unwrap();
        assert_eq!(header_for(&jar, &other, now()), None);
    }
}
