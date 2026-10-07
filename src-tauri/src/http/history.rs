//! What was sent, kept sealed: the request as it left with its secrets masked, and what came
//! back, its body up to `BODY_KEPT`. Listed by local day, kept `RETENTION_DAYS` and
//! `KEPT_AT_MOST` entries.

pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::{HttpMethod, KeyValue, RequestBody, RequestDocument};
use super::query::{self, QuerySide};
use super::send::{Exchange, SentBody, SentResponse};
use crate::error::ErrorCode;
use crate::notes::view::offset_from_minutes;

pub const RETENTION_DAYS: i64 = 30;
pub const KEPT_AT_MOST: i64 = 500;
pub const BODY_KEPT: usize = 256 * 1024;
const MASK: &str = "••••••••";

/// What a list shows, sealed apart from the record so a list opens no body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub name: String,
    pub url: String,
    pub millis: Option<u32>,
    pub size: Option<u32>,
    pub failure: Option<ErrorCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub exchange: Exchange,
    pub response: Option<SentResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub sent_at: DateTime<Utc>,
    pub method: HttpMethod,
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub summary: HistorySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    /// `2026-10-07`, the local day.
    pub day: String,
    pub items: Vec<HistoryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub item: HistoryItem,
    pub record: HistoryRecord,
}

/// What a send leaves to be recorded.
#[derive(Debug, Clone)]
pub struct Sent {
    pub name: String,
    pub request_id: Option<String>,
    pub exchange: Exchange,
    /// The API key's name, as `Outgoing::secrets` holds it.
    pub secrets: Vec<String>,
    pub outcome: Result<SentResponse, ErrorCode>,
}

fn is_secret(name: &str, secrets: &[String]) -> bool {
    let name = name.trim().to_ascii_lowercase();
    matches!(
        name.as_str(),
        "authorization" | "proxy-authorization" | "cookie"
    ) || secrets.contains(&name)
}

/// `Bearer abc` keeps its scheme: what kind of auth left is worth knowing, the token is not.
fn masked_value(name: &str, value: &str) -> String {
    match value.split_once(' ') {
        Some((scheme, _)) if name.eq_ignore_ascii_case("authorization") => {
            format!("{scheme} {MASK}")
        }
        _ => MASK.to_string(),
    }
}

/// The query's secret values replaced as they are written, the rest of the URL untouched.
fn masked_url(url: &str, secrets: &[String]) -> String {
    let Some((head, rest)) = url.split_once('?') else {
        return url.to_string();
    };
    let (query, fragment) = rest
        .split_once('#')
        .map_or((rest, None), |(query, fragment)| (query, Some(fragment)));
    let pairs: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _)) if secrets.contains(&key.to_ascii_lowercase()) => {
                format!("{key}={MASK}")
            }
            _ => pair.to_string(),
        })
        .collect();
    let mut masked = format!("{head}?{}", pairs.join("&"));
    if let Some(fragment) = fragment {
        masked.push('#');
        masked.push_str(fragment);
    }
    masked
}

pub fn mask(exchange: &Exchange, secrets: &[String]) -> Exchange {
    Exchange {
        method: exchange.method,
        url: masked_url(&exchange.url, secrets),
        headers: exchange
            .headers
            .iter()
            .map(|header| KeyValue {
                value: if is_secret(&header.key, secrets) {
                    masked_value(&header.key, &header.value)
                } else {
                    header.value.clone()
                },
                ..header.clone()
            })
            .collect(),
        body: exchange.body.clone(),
    }
}

fn cut(text: &str) -> (String, bool) {
    if text.len() <= BODY_KEPT {
        return (text.to_string(), false);
    }
    let mut end = BODY_KEPT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

/// What is sealed: the request masked, its body and the answer's cut to `BODY_KEPT`.
pub fn record_of(sent: &Sent) -> (HistorySummary, HistoryRecord) {
    let mut exchange = mask(&sent.exchange, &sent.secrets);
    if let SentBody::Text { text } = &exchange.body {
        exchange.body = SentBody::Text { text: cut(text).0 };
    }
    let response = sent.outcome.as_ref().ok().map(|response| {
        let (body, shortened) = cut(&response.body);
        SentResponse {
            pretty: if shortened {
                None
            } else {
                response.pretty.clone()
            },
            cut: response.cut || shortened,
            body,
            exchange: exchange.clone(),
            ..response.clone()
        }
    });
    let summary = HistorySummary {
        name: sent.name.clone(),
        url: exchange.url.clone(),
        millis: response.as_ref().map(|response| response.millis),
        size: response.as_ref().map(|response| response.size),
        failure: sent.outcome.as_ref().err().copied(),
    };
    (summary, HistoryRecord { exchange, response })
}

/// An entry sent again: what it was called, and the request it rebuilds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDraft {
    pub name: String,
    pub method: HttpMethod,
    pub document: RequestDocument,
}

/// The request as it left, minus what was masked — a secret is typed again, never replayed as
/// dots — and minus the `Content-Type` the body implies again.
pub fn draft_of(entry: &HistoryEntry) -> HistoryDraft {
    let exchange = &entry.record.exchange;
    let url = match exchange.url.split_once('?') {
        Some((head, query)) => {
            let kept: Vec<&str> = query
                .split('&')
                .filter(|pair| {
                    !pair
                        .split_once('=')
                        .is_some_and(|(_, value)| value.starts_with(MASK))
                })
                .collect();
            if kept.is_empty() {
                head.to_string()
            } else {
                format!("{head}?{}", kept.join("&"))
            }
        }
        None => exchange.url.clone(),
    };
    let typed = exchange
        .headers
        .iter()
        .find(|header| header.key.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.to_ascii_lowercase());
    let headers = exchange
        .headers
        .iter()
        .filter(|header| {
            !header.value.contains(MASK) && !header.key.eq_ignore_ascii_case("content-type")
        })
        .map(|header| KeyValue {
            enabled: true,
            ..header.clone()
        })
        .collect();
    let body = match &exchange.body {
        SentBody::Text { text } if typed.as_deref().is_some_and(|kind| kind.contains("json")) => {
            RequestBody::Json { text: text.clone() }
        }
        SentBody::Text { text } => RequestBody::Text { text: text.clone() },
        SentBody::None | SentBody::Bytes { .. } | SentBody::Multipart { .. } => RequestBody::None,
    };
    let params = query::sync(&url, &[], QuerySide::Url).params;
    HistoryDraft {
        name: entry.item.summary.name.clone(),
        method: exchange.method,
        document: RequestDocument {
            url,
            params,
            headers,
            body,
            ..RequestDocument::default()
        },
    }
}

/// Retention holds even when the history is never opened. Never fatal.
pub fn sweep_at_startup(db: &crate::db::Db) {
    let purged = crate::db::lock(db)
        .and_then(|mut connection| store::purge_expired(&mut connection, Utc::now()));
    if let Err(error) = purged {
        log::warn!("Expired HTTP history not purged: {error}");
    }
}

/// Newest first, a group a local day.
pub fn by_day(items: Vec<HistoryItem>, tz_offset_minutes: i32) -> Vec<HistoryDay> {
    let offset = offset_from_minutes(tz_offset_minutes);
    let mut days: Vec<HistoryDay> = Vec::new();
    for item in items {
        let day = item
            .sent_at
            .with_timezone(&offset)
            .format("%Y-%m-%d")
            .to_string();
        match days.last_mut() {
            Some(last) if last.day == day => last.items.push(item),
            _ => days.push(HistoryDay {
                day,
                items: vec![item],
            }),
        }
    }
    days
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        }
    }

    fn exchange(url: &str, headers: Vec<KeyValue>, body: SentBody) -> Exchange {
        Exchange {
            method: HttpMethod::Post,
            url: url.to_string(),
            headers,
            body,
        }
    }

    #[test]
    fn the_secrets_are_masked_wherever_they_left_and_the_rest_is_kept() {
        let masked = mask(
            &exchange(
                "https://api.exemple.fr/x?page=1&Key=s3cret#top",
                vec![
                    header("Authorization", "Bearer abc.def"),
                    header("Cookie", "session=1"),
                    header("X-Api-Key", "k"),
                    header("Accept", "application/json"),
                ],
                SentBody::None,
            ),
            &["key".to_string(), "x-api-key".to_string()],
        );

        assert_eq!(
            masked.url,
            "https://api.exemple.fr/x?page=1&Key=••••••••#top"
        );
        assert_eq!(
            masked
                .headers
                .iter()
                .map(|header| header.value.as_str())
                .collect::<Vec<_>>(),
            [
                "Bearer ••••••••",
                "••••••••",
                "••••••••",
                "application/json"
            ]
        );
    }

    #[test]
    fn a_record_cuts_long_bodies_and_says_how_it_ended() {
        let long = "é".repeat(BODY_KEPT);
        let sent = Sent {
            name: "Login".to_string(),
            request_id: None,
            exchange: exchange(
                "https://api.exemple.fr",
                Vec::new(),
                SentBody::Text { text: long.clone() },
            ),
            secrets: Vec::new(),
            outcome: Err(ErrorCode::HttpRefused),
        };

        let (summary, record) = record_of(&sent);
        assert_eq!(summary.failure, Some(ErrorCode::HttpRefused));
        assert_eq!((summary.millis, record.response), (None, None));
        let SentBody::Text { text } = record.exchange.body else {
            panic!("a text body");
        };
        assert!(text.len() <= BODY_KEPT && long.starts_with(&text));
    }

    #[test]
    fn an_entry_sent_again_leaves_its_secrets_and_implied_type_behind() {
        let entry = HistoryEntry {
            item: HistoryItem {
                id: "h".to_string(),
                sent_at: "2026-10-07T10:00:00Z".parse().unwrap(),
                method: HttpMethod::Post,
                status: Some(201),
                request_id: None,
                summary: HistorySummary {
                    name: "Payer".to_string(),
                    url: String::new(),
                    millis: None,
                    size: None,
                    failure: None,
                },
            },
            record: HistoryRecord {
                exchange: mask(
                    &exchange(
                        "https://api.exemple.fr/pay?page=2&key=s3cret",
                        vec![
                            header("Authorization", "Bearer t"),
                            header("Accept", "application/json"),
                            header("Content-Type", "application/json"),
                        ],
                        SentBody::Text {
                            text: "{\"a\":1}".to_string(),
                        },
                    ),
                    &["key".to_string()],
                ),
                response: None,
            },
        };

        let draft = draft_of(&entry);
        assert_eq!(
            (draft.name.as_str(), draft.method),
            ("Payer", HttpMethod::Post)
        );
        assert_eq!(draft.document.url, "https://api.exemple.fr/pay?page=2");
        assert_eq!(draft.document.params.len(), 1);
        assert_eq!(
            draft
                .document
                .headers
                .iter()
                .map(|header| header.key.as_str())
                .collect::<Vec<_>>(),
            ["Accept"]
        );
        assert_eq!(
            draft.document.body,
            RequestBody::Json {
                text: "{\"a\":1}".to_string()
            }
        );
    }

    #[test]
    fn entries_are_grouped_by_the_local_day() {
        let item = |id: &str, at: &str| HistoryItem {
            id: id.to_string(),
            sent_at: at.parse().unwrap(),
            method: HttpMethod::Get,
            status: Some(200),
            request_id: None,
            summary: HistorySummary {
                name: id.to_string(),
                url: String::new(),
                millis: None,
                size: None,
                failure: None,
            },
        };
        let days = by_day(
            vec![
                item("c", "2026-10-07T00:30:00Z"),
                item("b", "2026-10-06T23:30:00Z"),
                item("a", "2026-10-06T10:00:00Z"),
            ],
            -120,
        );

        assert_eq!(
            days.iter()
                .map(|day| (day.day.as_str(), day.items.len()))
                .collect::<Vec<_>>(),
            [("2026-10-07", 2), ("2026-10-06", 1)]
        );
    }
}
