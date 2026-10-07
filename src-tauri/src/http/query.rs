//! A request's query string and its parameters table, kept in step both ways. Nothing is
//! decoded or encoded: `{{page}}` and `%20` read back exactly as they were typed.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::KeyValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QuerySide {
    Url,
    Params,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncedQuery {
    pub url: String,
    pub params: Vec<KeyValue>,
}

/// The side that was edited wins; the other is rewritten from it.
pub fn sync(url: &str, params: &[KeyValue], edited: QuerySide) -> SyncedQuery {
    match edited {
        QuerySide::Url => SyncedQuery {
            url: url.to_string(),
            params: params_from_url(url, params),
        },
        QuerySide::Params => SyncedQuery {
            url: url_with_params(url, params),
            params: params.to_vec(),
        },
    }
}

/// `(before the query, the query, from the fragment on)`: the `#` ends the query.
fn parts(url: &str) -> (&str, Option<&str>, &str) {
    let (head, fragment) = url.find('#').map_or((url, ""), |at| url.split_at(at));
    match head.split_once('?') {
        Some((base, query)) => (base, Some(query), fragment),
        None => (head, None, fragment),
    }
}

/// The URL's pairs replace the table's enabled rows in order; a disabled row is not in the URL
/// and stays where it was, and a description follows its key.
fn params_from_url(url: &str, previous: &[KeyValue]) -> Vec<KeyValue> {
    let (_, query, _) = parts(url);
    let mut typed = query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_string(), value.to_string())
        });

    let mut rows = Vec::with_capacity(previous.len());
    for row in previous {
        if !row.enabled {
            rows.push(row.clone());
        } else if let Some((key, value)) = typed.next() {
            let description = if key == row.key {
                row.description.clone()
            } else {
                String::new()
            };
            rows.push(KeyValue {
                enabled: true,
                key,
                value,
                description,
            });
        }
    }
    rows.extend(typed.map(|(key, value)| KeyValue {
        enabled: true,
        key,
        value,
        description: String::new(),
    }));
    rows
}

/// The enabled rows with a key, as `key=value` joined by `&`; no row, no `?`.
fn url_with_params(url: &str, params: &[KeyValue]) -> String {
    let (base, _, fragment) = parts(url);
    let query: Vec<String> = params
        .iter()
        .filter(|row| row.enabled && !row.key.is_empty())
        .map(|row| format!("{}={}", row.key, row.value))
        .collect();
    if query.is_empty() {
        format!("{base}{fragment}")
    } else {
        format!("{base}?{}{fragment}", query.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(enabled: bool, key: &str, value: &str, description: &str) -> KeyValue {
        KeyValue {
            enabled,
            key: key.to_string(),
            value: value.to_string(),
            description: description.to_string(),
        }
    }

    #[test]
    fn a_typed_query_fills_the_table_as_written() {
        let synced = sync(
            "{{baseUrl}}/customers/{{customerId}}/invoices?page=1&limit=50&q=a%20b&flag#top",
            &[],
            QuerySide::Url,
        );

        assert_eq!(
            synced.params,
            vec![
                row(true, "page", "1", ""),
                row(true, "limit", "50", ""),
                row(true, "q", "a%20b", ""),
                row(true, "flag", "", ""),
            ]
        );
    }

    #[test]
    fn a_disabled_row_stays_put_and_a_description_follows_its_key() {
        let previous = [
            row(true, "page", "1", "Page, à partir de 1"),
            row(false, "status", "paid", "Filtrer par statut"),
            row(true, "limit", "50", "100 au plus"),
        ];

        let synced = sync("/invoices?page=2&size=50", &previous, QuerySide::Url);

        assert_eq!(
            synced.params,
            vec![
                row(true, "page", "2", "Page, à partir de 1"),
                row(false, "status", "paid", "Filtrer par statut"),
                row(true, "size", "50", ""),
            ]
        );
        assert_eq!(
            sync("/invoices", &previous, QuerySide::Url).params,
            vec![previous[1].clone()]
        );
    }

    #[test]
    fn the_table_rewrites_the_query_keeping_the_path_and_the_fragment() {
        let params = [
            row(true, "page", "{{page}}", ""),
            row(false, "status", "paid", ""),
            row(true, "", "orphan", ""),
            row(true, "limit", "50", ""),
        ];

        assert_eq!(
            sync("{{baseUrl}}/invoices?old=1#top", &params, QuerySide::Params).url,
            "{{baseUrl}}/invoices?page={{page}}&limit=50#top"
        );
        assert_eq!(
            sync(
                "/invoices?old=1",
                &[row(false, "a", "1", "")],
                QuerySide::Params
            )
            .url,
            "/invoices"
        );
    }
}
