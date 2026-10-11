//! What was sent, kept sealed: the request as it left with its secrets masked, and what came
//! back, its body up to `BODY_KEPT`. Listed by local day, kept `RETENTION_DAYS` and
//! `KEPT_AT_MOST` entries.

pub mod store;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use serde_json::value::RawValue;
use url::Url;

use super::graphql::GraphqlDocument;
use super::model::{HttpMethod, HttpRequest, KeyValue, RequestBody, RequestDocument, RequestKind};
use super::query::{self, QuerySide};
use super::response::pretty_json;
use super::send::{self, Exchange, SentBody, SentResponse};
use super::settings::InheritedHeader;
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
    /// What the failure's sentence states: the delay that ran out, and who set it.
    #[serde(default)]
    pub failure_params: BTreeMap<String, String>,
    /// What was sent, for a send reopened as a draft to be of the same kind.
    #[serde(default)]
    pub kind: RequestKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub exchange: Exchange,
    pub response: Option<SentResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub sent_at: DateTime<Utc>,
    pub method: HttpMethod,
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub summary: HistorySummary,
    /// The URL in two parts for a list (`url_parts`): the host, which a narrow row cuts, and
    /// what follows it, which it keeps.
    #[serde(default)]
    pub url_head: String,
    #[serde(default)]
    pub url_tail: String,
}

/// A URL as a list reads it. Fourteen sends to one host looked alike, cut at their end: what
/// repeats is the host, what tells them apart is the path and the query. No scheme: the whole
/// URL is one hover away. A text that is no URL is all tail.
pub fn url_parts(url: &str) -> (String, String) {
    let Some((_, rest)) = url.split_once("://") else {
        return (String::new(), url.to_string());
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    // Its credentials are nothing a list has to show.
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (host.to_string(), tail.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    /// `2026-10-07`, the local day.
    pub day: String,
    pub items: Vec<HistoryItem>,
}

/// Read to be shown; and handed back as it was read to undo its deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub item: HistoryItem,
    pub record: HistoryRecord,
}

/// What a send leaves to be recorded.
#[derive(Debug, Clone)]
pub struct Sent {
    pub kind: RequestKind,
    pub name: String,
    pub request_id: Option<String>,
    pub exchange: Exchange,
    /// The API key's name, as `Outgoing::secrets` holds it.
    pub secrets: Vec<String>,
    /// A failure as its code and the values its sentence states.
    pub outcome: Result<SentResponse, (ErrorCode, BTreeMap<String, String>)>,
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
        kind: sent.kind,
        name: sent.name.clone(),
        url: exchange.url.clone(),
        millis: response.as_ref().map(|response| response.millis),
        size: response.as_ref().map(|response| response.size),
        failure: sent.outcome.as_ref().err().map(|(code, _)| *code),
        failure_params: sent
            .outcome
            .as_ref()
            .err()
            .map(|(_, params)| params.clone())
            .unwrap_or_default(),
    };
    (summary, HistoryRecord { exchange, response })
}

/// An entry sent again: what it was called, and the request it rebuilds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDraft {
    pub name: String,
    /// The kind of what was sent: a draft is created of it, as any request is of its own.
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
}

/// A recorded URL without the query pairs it holds masked.
fn draft_url(url: &str) -> String {
    match url.split_once('?') {
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
        None => url.to_string(),
    }
}

/// The request as it left, minus what was masked — a secret is typed again, never replayed as
/// dots — and minus what is written again by itself: the `Content-Type` the body implies, and
/// the application's own headers, which would otherwise name this version for ever.
pub fn draft_of(entry: &HistoryEntry) -> HistoryDraft {
    let exchange = &entry.record.exchange;
    let url = draft_url(&exchange.url);
    let typed = exchange
        .headers
        .iter()
        .find(|header| header.key.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.to_ascii_lowercase());
    let headers = exchange
        .headers
        .iter()
        .filter(|header| {
            !header.value.contains(MASK)
                && !header.key.eq_ignore_ascii_case("content-type")
                && !send::is_default(&header.key, &header.value)
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
    let mut draft = HistoryDraft {
        name: entry.item.summary.name.clone(),
        kind: RequestKind::Http,
        method: exchange.method,
        document: RequestDocument {
            url,
            params,
            headers,
            body,
            ..RequestDocument::default()
        },
    };
    // A GraphQL send back in its two boxes, when it still reads as one.
    if entry.item.summary.kind == RequestKind::Graphql
        && let Some((graphql, url)) = graphql_of(exchange)
    {
        draft.kind = RequestKind::Graphql;
        draft.document.url = draft_url(&url);
        draft.document.params = query::sync(&draft.document.url, &[], QuerySide::Url).params;
        draft.document.body = RequestBody::None;
        draft.document.graphql = graphql;
    }
    draft
}

#[derive(Deserialize)]
struct Posted<'a> {
    query: String,
    #[serde(borrow, default)]
    variables: Option<&'a RawValue>,
    #[serde(rename = "operationName", default)]
    operation_name: Option<String>,
}

/// A GraphQL send read back into its parts, and the URL without what it carried of them: the
/// JSON body of a `POST`, or the query pairs of a `GET`. `None` when it does not read as one.
fn graphql_of(exchange: &Exchange) -> Option<(GraphqlDocument, String)> {
    let laid_out = |raw: &str| pretty_json(raw).unwrap_or_else(|| raw.to_string());
    if let SentBody::Text { text } = &exchange.body {
        let posted: Posted<'_> = serde_json::from_str(text).ok()?;
        return Some((
            GraphqlDocument {
                query: posted.query,
                variables: posted
                    .variables
                    .filter(|raw| raw.get() != "null")
                    .map(|raw| laid_out(raw.get()))
                    .unwrap_or_default(),
                operation_name: posted.operation_name,
                as_get: false,
            },
            exchange.url.clone(),
        ));
    }
    let mut url = Url::parse(&exchange.url).ok()?;
    let mut document = GraphqlDocument {
        as_get: true,
        ..GraphqlDocument::default()
    };
    let mut kept: Vec<(String, String)> = Vec::new();
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "query" => document.query = value.into_owned(),
            "variables" => document.variables = laid_out(&value),
            "operationName" => document.operation_name = Some(value.into_owned()),
            _ => kept.push((key.into_owned(), value.into_owned())),
        }
    }
    if document.query.is_empty() {
        return None;
    }
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
    Some((document, url.to_string()))
}

/// A send put back onto the request it came from, as changes to save or not: what was sent —
/// method, URL, parameters, headers, body — over what the request holds, which keeps its auth,
/// its settings and every secret.
///
/// ⚠️ A header the record holds masked takes the request's **own** row of that name back, when
/// it has one: never the dots as a value. Set by the auth or handed down, it has no row here,
/// and what set it is still there. A header its folder or collection hands down today, with
/// the value that was sent, is no change of the request's either.
pub fn onto(
    entry: &HistoryEntry,
    current: &HttpRequest,
    above: &[InheritedHeader],
) -> HistoryDraft {
    let sent = draft_of(entry);
    let exchange = &entry.record.exchange;
    let mut document = current.document.clone();
    let own = |name: &str| {
        current
            .document
            .headers
            .iter()
            .find(|row| row.key.trim().eq_ignore_ascii_case(name))
    };
    let handed_down = |header: &KeyValue| {
        above.iter().any(|known| {
            known.header.key.trim().eq_ignore_ascii_case(&header.key)
                && known.header.value == header.value
        })
    };
    document.headers = exchange
        .headers
        .iter()
        .filter_map(|header| {
            if header.value.contains(MASK) {
                return own(&header.key).cloned();
            }
            sent.document
                .headers
                .iter()
                .find(|kept| kept.key == header.key && !handed_down(kept))
                // Its description is the request's, which the record never held.
                .map(|kept| KeyValue {
                    description: own(&kept.key)
                        .map(|row| row.description.clone())
                        .unwrap_or_default(),
                    ..kept.clone()
                })
        })
        .collect();

    let mut method = current.method;
    match current.kind {
        RequestKind::Graphql => {
            if let Some((graphql, url)) = graphql_of(exchange) {
                document.graphql = graphql;
                document.url = draft_url(&url);
                document.params = query::sync(&document.url, &[], QuerySide::Url).params;
            }
        }
        RequestKind::Http | RequestKind::Websocket => {
            method = exchange.method;
            document.url = sent.document.url;
            document.params = sent.document.params;
            // A file or a multipart was not kept: the request's own stays.
            if matches!(exchange.body, SentBody::Text { .. } | SentBody::None) {
                document.body = sent.document.body;
            }
        }
    }
    HistoryDraft {
        name: current.name.clone(),
        kind: current.kind,
        method,
        document,
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
            kind: RequestKind::Http,
            name: "Login".to_string(),
            request_id: None,
            exchange: exchange(
                "https://api.exemple.fr",
                Vec::new(),
                SentBody::Text { text: long.clone() },
            ),
            secrets: Vec::new(),
            outcome: Err((
                ErrorCode::HttpTimeout,
                BTreeMap::from([("amount".to_string(), "2".to_string())]),
            )),
        };

        let (summary, record) = record_of(&sent);
        assert_eq!(summary.failure, Some(ErrorCode::HttpTimeout));
        assert_eq!(
            summary.failure_params.get("amount").map(String::as_str),
            Some("2")
        );
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
                    kind: RequestKind::Http,
                    name: "Payer".to_string(),
                    url: String::new(),
                    millis: None,
                    size: None,
                    failure: None,
                    failure_params: BTreeMap::new(),
                },
                url_head: String::new(),
                url_tail: String::new(),
            },
            record: HistoryRecord {
                exchange: mask(
                    &exchange(
                        "https://api.exemple.fr/pay?page=2&key=s3cret",
                        vec![
                            header("Authorization", "Bearer t"),
                            header("Accept", "application/json"),
                            header("Content-Type", "application/json"),
                            header("User-Agent", "DevNotes/0.10.1"),
                            header("X-Agent", "DevNotes/0.10.1"),
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
            ["Accept", "X-Agent"]
        );
        assert_eq!(
            draft.document.body,
            RequestBody::Json {
                text: "{\"a\":1}".to_string()
            }
        );
    }

    /// Cut at its end, every URL of a host read `https://httpbin.org/hea…`.
    #[test]
    fn a_url_is_read_in_two_parts_the_host_and_what_tells_two_sends_apart() {
        let parts = |url: &str| {
            let (head, tail) = url_parts(url);
            format!("{head}|{tail}")
        };

        assert_eq!(
            parts("https://httpbin.org/redirect/3"),
            "httpbin.org|/redirect/3"
        );
        assert_eq!(
            parts("http://127.0.0.1:8080/users?page=2&key=••••••••#top"),
            "127.0.0.1:8080|/users?page=2&key=••••••••#top"
        );
        assert_eq!(parts("https://ada:pw@api.exemple.fr"), "api.exemple.fr|");
        assert_eq!(parts("https://api.exemple.fr?x=1"), "api.exemple.fr|?x=1");
        assert_eq!(parts("pas une URL"), "|pas une URL");
    }

    fn sent(url: &str, headers: Vec<KeyValue>, body: SentBody, secrets: &[&str]) -> HistoryEntry {
        let secrets: Vec<String> = secrets.iter().map(ToString::to_string).collect();
        HistoryEntry {
            item: HistoryItem {
                id: "h".to_string(),
                sent_at: "2026-10-07T10:00:00Z".parse().unwrap(),
                method: HttpMethod::Post,
                status: Some(200),
                request_id: Some("r".to_string()),
                summary: HistorySummary {
                    kind: RequestKind::Http,
                    name: "Payer".to_string(),
                    url: String::new(),
                    millis: None,
                    size: None,
                    failure: None,
                    failure_params: BTreeMap::new(),
                },
                url_head: String::new(),
                url_tail: String::new(),
            },
            record: HistoryRecord {
                exchange: mask(&exchange(url, headers, body), &secrets),
                response: None,
            },
        }
    }

    fn held(kind: RequestKind, document: RequestDocument) -> HttpRequest {
        HttpRequest {
            id: "r".to_string(),
            collection_id: "c".to_string(),
            folder_id: None,
            name: "ma req".to_string(),
            kind,
            method: HttpMethod::Get,
            document,
            created_at: "2026-10-01T00:00:00Z".parse().unwrap(),
            updated_at: "2026-10-01T00:00:00Z".parse().unwrap(),
        }
    }

    /// « Rouvrir » made a stranger of the send; and a mask must never come back as a value.
    #[test]
    fn a_send_goes_back_onto_its_request_with_the_request_s_own_secrets() {
        use crate::http::model::{HttpItemKind, KeyPlace, RequestAuth};
        use crate::http::settings::HttpOrigin;

        let entry = sent(
            "https://api.exemple.fr/pay?page=2&key=s3cret",
            vec![
                header("Authorization", "Bearer old"),
                header("X-Token", "s3cret"),
                header("X-Client", "web"),
                header("X-Trace", "7"),
                header("Content-Type", "application/json"),
                header("User-Agent", "DevNotes/0.10.1"),
            ],
            SentBody::Text {
                text: "{\"a\":1}".to_string(),
            },
            &["key", "x-token"],
        );
        let current = held(
            RequestKind::Http,
            RequestDocument {
                url: "https://api.exemple.fr/pay".to_string(),
                headers: vec![
                    KeyValue {
                        description: "le jeton du moment".to_string(),
                        ..header("x-token", "n3w")
                    },
                    KeyValue {
                        description: "à garder".to_string(),
                        ..header("X-Trace", "1")
                    },
                    header("X-Gone", "1"),
                ],
                auth: RequestAuth::ApiKey {
                    name: "key".to_string(),
                    value: "n3w".to_string(),
                    place: KeyPlace::Query,
                },
                description: "Paie une facture".to_string(),
                ..RequestDocument::default()
            },
        );
        let above = vec![InheritedHeader {
            header: header("x-client", "web"),
            from: HttpOrigin {
                kind: HttpItemKind::Collection,
                id: "c".to_string(),
                name: "API".to_string(),
            },
        }];

        let draft = onto(&entry, &current, &above);

        assert_eq!(
            (draft.name.as_str(), draft.method),
            ("ma req", HttpMethod::Post)
        );
        // The key the auth writes is the auth's still, and no dot is in the URL.
        assert_eq!(draft.document.url, "https://api.exemple.fr/pay?page=2");
        assert_eq!(draft.document.params.len(), 1);
        assert_eq!(draft.document.auth, current.document.auth);
        assert_eq!(draft.document.description, "Paie une facture");
        assert_eq!(
            draft.document.body,
            RequestBody::Json {
                text: "{\"a\":1}".to_string()
            }
        );
        // Masked: the request's own row, as it is now. Sent: its value, the row's description.
        // Handed down, implied or written by the auth: no row of the request's.
        assert_eq!(
            draft
                .document
                .headers
                .iter()
                .map(|row| (
                    row.key.as_str(),
                    row.value.as_str(),
                    row.description.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("x-token", "n3w", "le jeton du moment"),
                ("X-Trace", "7", "à garder"),
            ]
        );
        assert!(!format!("{draft:?}").contains(MASK));
    }

    /// Reopened as a draft, a GraphQL send was an HTTP `POST` with a JSON body.
    #[test]
    fn a_send_reopened_as_a_draft_is_of_the_kind_that_was_sent() {
        let mut posted = sent(
            "https://api.exemple.fr/graphql",
            vec![header("X-Trace", "1")],
            SentBody::Text {
                text: r#"{"query":"{ me }"}"#.to_string(),
            },
            &[],
        );
        assert_eq!(draft_of(&posted).kind, RequestKind::Http);

        posted.item.summary.kind = RequestKind::Graphql;
        let draft = draft_of(&posted);
        assert_eq!(draft.kind, RequestKind::Graphql);
        assert_eq!(draft.document.graphql.query, "{ me }");
        assert_eq!(draft.document.body, RequestBody::None);
        assert_eq!(draft.document.headers.len(), 1);

        // Said to be GraphQL and not reading as one: sent again as what it was on the wire.
        let odd = HistoryEntry {
            item: posted.item.clone(),
            record: HistoryRecord {
                exchange: exchange("https://api.exemple.fr/graphql", Vec::new(), SentBody::None),
                response: None,
            },
        };
        assert_eq!(draft_of(&odd).kind, RequestKind::Http);
    }

    #[test]
    fn a_graphql_send_goes_back_into_its_query_and_variables_and_a_file_is_left_alone() {
        let posted = sent(
            "https://api.exemple.fr/graphql",
            Vec::new(),
            SentBody::Text {
                text: r#"{"query":"query Me { me }","variables":{"n":12345678901234567890},"operationName":"Me"}"#.to_string(),
            },
            &[],
        );
        let current = held(RequestKind::Graphql, RequestDocument::default());
        let draft = onto(&posted, &current, &[]);
        assert_eq!(draft.method, HttpMethod::Get);
        assert_eq!(draft.document.url, "https://api.exemple.fr/graphql");
        assert_eq!(
            draft.document.graphql,
            GraphqlDocument {
                query: "query Me { me }".to_string(),
                variables: "{\n  \"n\": 12345678901234567890\n}".to_string(),
                operation_name: Some("Me".to_string()),
                as_get: false,
            }
        );

        let got = sent(
            "https://api.exemple.fr/graphql?query=%7B%20me%20%7D&variables=%7B%22a%22%3A1%7D&trace=1",
            Vec::new(),
            SentBody::None,
            &[],
        );
        let draft = onto(&got, &current, &[]);
        assert_eq!(draft.document.url, "https://api.exemple.fr/graphql?trace=1");
        assert_eq!(draft.document.graphql.query, "{ me }");
        assert_eq!(draft.document.graphql.variables, "{\n  \"a\": 1\n}");
        assert!(draft.document.graphql.as_get);
        // What does not read as GraphQL changes nothing of the request's.
        let odd = sent(
            "https://api.exemple.fr/graphql",
            Vec::new(),
            SentBody::None,
            &[],
        );
        assert_eq!(
            onto(&odd, &current, &[]).document.graphql,
            current.document.graphql
        );

        // A file was not kept: the request's own body stays.
        let mut uploading = held(RequestKind::Http, RequestDocument::default());
        uploading.document.body = RequestBody::Binary {
            path: "C:/tmp/a.bin".to_string(),
        };
        let upload = sent(
            "https://api.exemple.fr/upload",
            Vec::new(),
            SentBody::Bytes { size: 12 },
            &[],
        );
        assert_eq!(
            onto(&upload, &uploading, &[]).document.body,
            uploading.document.body
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
                kind: RequestKind::Http,
                name: id.to_string(),
                url: String::new(),
                millis: None,
                size: None,
                failure: None,
                failure_params: BTreeMap::new(),
            },
            url_head: String::new(),
            url_tail: String::new(),
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
