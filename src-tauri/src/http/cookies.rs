//! The library's cookie jar: what answers set, and what requests send back. RFC 6265 as far as
//! a client for developers needs it — domain and path matching, expiry, `Secure`.

pub mod store;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use url::{Host, Url};

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
    /// When it was first set, kept by a cookie that replaces it: of two paths of one length,
    /// the older is sent first (RFC 6265, 5.4). Sealed with the rest: nothing in SQL orders on it.
    #[serde(default)]
    pub created: Option<DateTime<Utc>>,
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

/// Whether a `Domain` is one under which unrelated sites register — `co.uk`, `github.io`, a
/// top-level domain —, by the public suffix list compiled in. A name no rule knows is its own
/// last label's, as browsers read it.
fn is_public_suffix(domain: &str) -> bool {
    psl::suffix(domain.as_bytes()).is_some_and(|suffix| suffix.as_bytes() == domain.as_bytes())
}

/// RFC 6265, 5.1.1: a date as servers write it — RFC 1123, RFC 850, asctime, and what lies
/// between —, read token by token: the first time, day, month and year found, a two-digit year
/// taken to 1970–2069. `None` for a date that does not read: the cookie lasts the session.
fn cookie_date(text: &str) -> Option<DateTime<Utc>> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let delimiter =
        |character: char| matches!(character, '\t' | ' '..='/' | ';'..='@' | '['..='`' | '{'..='~');
    let leading_digits = |token: &str, at_most: usize| -> Option<u32> {
        let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
        (1..=at_most)
            .contains(&digits.len())
            .then(|| digits.parse().ok())
            .flatten()
    };
    let (mut time, mut day, mut month, mut year) = (None, None, None, None);
    for token in text.split(delimiter).filter(|token| !token.is_empty()) {
        if time.is_none() {
            let parts: Vec<Option<u32>> = token
                .splitn(3, ':')
                .map(|part| leading_digits(part, 2))
                .collect();
            if let [Some(hour), Some(minute), Some(second)] = parts[..] {
                time = Some((hour, minute, second));
                continue;
            }
        }
        if day.is_none()
            && let Some(found) = leading_digits(token, 2)
        {
            day = Some(found);
            continue;
        }
        if month.is_none() && token.len() >= 3 {
            let start = token[..3].to_ascii_lowercase();
            if let Some(at) = MONTHS.iter().position(|name| *name == start) {
                month = Some(u32::try_from(at).ok()? + 1);
                continue;
            }
        }
        if year.is_none()
            && let Some(found) = leading_digits(token, 4).filter(|_| {
                let digits = token.chars().take_while(char::is_ascii_digit).count();
                digits == 2 || digits == 4
            })
        {
            year = Some(found);
        }
    }
    let (hour, minute, second) = time?;
    let year = match year? {
        found @ 70..=99 => found + 1900,
        found @ 0..=69 => found + 2000,
        found => found,
    };
    if year < 1601 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month?, day?)?
        .and_hms_opt(hour, minute, second)
        .map(|date| date.and_utc())
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
    let by_address = matches!(url.host(), Some(Host::Ipv4(_) | Host::Ipv6(_)));
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
        created: Some(now),
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
                // Itself, named: host-only, as without the attribute. An address has no domain
                // above it, and a public suffix is every site's under it (RFC 6265, 5.3).
                if domain == host {
                    continue;
                }
                if by_address || is_public_suffix(&domain) {
                    return None;
                }
                cookie.domain = domain;
                cookie.host_only = false;
            }
            "path" if value.starts_with('/') => cookie.path = value.to_string(),
            "max-age" => max_age = value.parse().ok(),
            "expires" => cookie.expires = cookie_date(value).or(cookie.expires),
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
        // One answer's lines are created in their order, a microsecond apart: two of one path
        // length go back in the order they were set, not the order the jar is read in.
        .filter_map(|header| parse(&header.value, url, now))
        .zip(0..)
        .map(|(mut change, rank)| {
            let (Change::Set(cookie) | Change::Remove(cookie)) = &mut change;
            cookie.created = Some(now + TimeDelta::microseconds(rank));
            change
        })
        .collect()
}

/// What `changes` do to a jar held in memory. The hops of one send read and write a copy: the
/// library's own is sealed, and the lock is nobody's while a request is on its way.
pub fn apply(jar: &mut Vec<JarCookie>, changes: &[Change]) {
    for change in changes {
        let (Change::Set(cookie) | Change::Remove(cookie)) = change;
        let replaced = jar
            .iter()
            .find(|known| known.same_as(cookie))
            .and_then(|known| known.created);
        jar.retain(|known| !known.same_as(cookie));
        if let Change::Set(cookie) = change {
            jar.push(JarCookie {
                created: replaced.or(cookie.created),
                ..cookie.clone()
            });
        }
    }
}

/// The `Cookie` header a request to `url` carries, the longest paths first and, of two paths
/// of one length, the older cookie first; `None` for none.
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
    sent.sort_by_key(|cookie| (std::cmp::Reverse(cookie.path.len()), cookie.created));
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

    /// #705: `x.co.uk` set a cookie for `co.uk`, sent to every `.co.uk` site; `127.0.0.1` one
    /// for `0.0.1`.
    #[test]
    fn a_public_suffix_and_an_address_take_no_domain_above_them() {
        let names = |url: &str, lines: &[&str]| -> Vec<(String, String, bool)> {
            let url = Url::parse(url).unwrap();
            let headers: Vec<KeyValue> = lines.iter().map(|line| set_cookie(line)).collect();
            received(&headers, &url, now())
                .into_iter()
                .map(|change| {
                    let cookie = set(change);
                    (cookie.name, cookie.domain, cookie.host_only)
                })
                .collect()
        };
        let cookie = |name: &str, domain: &str, host_only: bool| {
            (name.to_string(), domain.to_string(), host_only)
        };

        assert_eq!(
            names(
                "https://x.co.uk/",
                &["suffix=1; Domain=co.uk", "itself=1; Domain=x.co.uk"]
            ),
            [cookie("itself", "x.co.uk", true)]
        );
        assert_eq!(
            names(
                "https://api.exemple.co.uk/",
                &[
                    "registered=1; Domain=exemple.co.uk",
                    "pages=1; Domain=github.io"
                ]
            ),
            [cookie("registered", "exemple.co.uk", false)]
        );
        assert!(names("https://ada.github.io/", &["all=1; Domain=github.io"]).is_empty());
        assert_eq!(
            names(
                "http://127.0.0.1:8080/",
                &["above=1; Domain=0.0.1", "named=1; Domain=127.0.0.1"]
            ),
            [cookie("named", "127.0.0.1", true)]
        );
    }

    /// #705: RFC 850 and asctime were read as no date at all, and the cookie lived the session.
    #[test]
    fn an_expiry_is_read_in_every_format_servers_write() {
        let at = |text: &str| cookie_date(text).map(|date| date.to_rfc3339());
        let expected = Some("2026-10-21T07:28:00+00:00".to_string());

        assert_eq!(at("Wed, 21 Oct 2026 07:28:00 GMT"), expected);
        assert_eq!(at("Wednesday, 21-Oct-26 07:28:00 GMT"), expected);
        assert_eq!(at("Wed Oct 21 07:28:00 2026"), expected);
        assert_eq!(at("21-Oct-2026 07:28:00 GMT"), expected);
        assert_eq!(
            at("Thu, 01-Jan-70 00:00:01 GMT"),
            Some("1970-01-01T00:00:01+00:00".to_string())
        );
        assert_eq!(
            at("Sun, 01-Jan-69 00:00:01 GMT"),
            Some("2069-01-01T00:00:01+00:00".to_string())
        );
        for unreadable in [
            "",
            "tomorrow",
            "Wed, 32 Oct 2026 07:28:00 GMT",
            "21 Oct 2026",
        ] {
            assert_eq!(at(unreadable), None, "{unreadable}");
        }

        let url = Url::parse("https://exemple.fr/").unwrap();
        let change = received(
            &[set_cookie("a=1; Expires=Wednesday, 21-Oct-26 07:28:00 GMT")],
            &url,
            now(),
        );
        assert_eq!(
            set(change[0].clone()).expires.map(|date| date.to_rfc3339()),
            expected
        );
    }

    /// #705: two paths of one length came out in the order the jar was read, a random id's.
    #[test]
    fn of_two_paths_of_one_length_the_older_cookie_goes_first_and_a_replacement_keeps_its_age() {
        let url = Url::parse("https://exemple.fr/").unwrap();
        let mut jar = Vec::new();
        apply(
            &mut jar,
            &received(&[set_cookie("first=1; Path=/")], &url, now()),
        );
        apply(
            &mut jar,
            &received(
                &[
                    set_cookie("second=1; Path=/"),
                    set_cookie("third=1; Path=/"),
                ],
                &url,
                now() + TimeDelta::seconds(1),
            ),
        );
        // Read back in another order, as the store may.
        jar.reverse();
        assert_eq!(
            header_for(&jar, &url, now()).as_deref(),
            Some("first=1; second=1; third=1")
        );

        apply(
            &mut jar,
            &received(
                &[set_cookie("first=2; Path=/")],
                &url,
                now() + TimeDelta::seconds(2),
            ),
        );
        assert_eq!(
            header_for(&jar, &url, now()).as_deref(),
            Some("first=2; second=1; third=1")
        );
    }

    #[test]
    fn a_jar_in_memory_takes_a_cookie_replaces_it_and_lets_it_go() {
        let url = Url::parse("http://api.exemple.fr/login").unwrap();
        let mut jar = Vec::new();
        let line = |jar: &[JarCookie]| header_for(jar, &url, now());

        apply(
            &mut jar,
            &received(&[set_cookie("session=abc; Path=/")], &url, now()),
        );
        assert_eq!(line(&jar).as_deref(), Some("session=abc"));

        apply(
            &mut jar,
            &received(
                &[
                    set_cookie("session=def; Path=/"),
                    set_cookie("theme=dark; Path=/"),
                ],
                &url,
                now(),
            ),
        );
        assert_eq!(line(&jar).as_deref(), Some("session=def; theme=dark"));

        apply(
            &mut jar,
            &received(&[set_cookie("session=; Path=/; Max-Age=0")], &url, now()),
        );
        assert_eq!(line(&jar).as_deref(), Some("theme=dark"));
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
                created: None,
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
