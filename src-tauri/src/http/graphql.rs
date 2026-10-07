//! A GraphQL request is an HTTP one whose body Rust writes: the query, its variables and the
//! operation chosen. Its answer is read apart: `errors` beside `data`.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use specta::Type;

use super::response::pretty_json;
use crate::json::parse::JsonError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct GraphqlDocument {
    pub query: String,
    /// A JSON object as typed; empty sends none.
    pub variables: String,
    /// Which operation, when the query holds several; `None` lets the server take the only one.
    pub operation_name: Option<String>,
    /// Sent as a GET, its parts in the query string, rather than a POST with a JSON body.
    pub as_get: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlAnswer {
    /// The named operations, in the order written.
    pub operations: Vec<String>,
    pub variables_problem: Option<JsonError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlError {
    pub message: String,
    /// `invoices[0].amount`, joined from the error's `path`.
    pub path: Option<String>,
    /// `3:5`, the first of its `locations`.
    pub location: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlResult {
    /// `data` laid out, its numbers as they came; `None` when the answer had none.
    pub data: Option<String>,
    pub errors: Vec<GraphqlError>,
}

/// The names of the document's operations, read past strings, comments and selections.
pub fn operations(query: &str) -> Vec<String> {
    let word = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| -> String {
        std::iter::from_fn(|| chars.next_if(|next| next.is_ascii_alphanumeric() || *next == '_'))
            .collect()
    };
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut chars = query.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut escaped = false;
                for next in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        break;
                    }
                }
            }
            '{' | '(' => depth += 1,
            '}' | ')' => depth = depth.saturating_sub(1),
            c if depth == 0 && c.is_ascii_alphabetic() => {
                let keyword = format!("{c}{}", word(&mut chars));
                if matches!(keyword.as_str(), "query" | "mutation" | "subscription") {
                    while chars.next_if(|next| next.is_whitespace()).is_some() {}
                    let name = word(&mut chars);
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
            }
            _ => {}
        }
    }
    names
}

pub fn describe(document: &GraphqlDocument) -> GraphqlAnswer {
    GraphqlAnswer {
        operations: operations(&document.query),
        variables_problem: if document.variables.trim().is_empty() {
            None
        } else {
            crate::json::parse::parse(&document.variables).err()
        },
    }
}

/// The variables as typed, when they are a JSON object: copied, not re-serialised.
pub(crate) fn variables(document: &GraphqlDocument) -> Result<Option<&str>, String> {
    let text = document.variables.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let raw: &RawValue = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if !raw.get().starts_with('{') {
        return Err("the variables are not an object".to_string());
    }
    Ok(Some(text))
}

/// `{"query": …, "variables": …, "operationName": …}` for a POST.
pub(crate) fn body(document: &GraphqlDocument, variables: Option<&str>) -> String {
    let quoted = |text: &str| serde_json::to_string(text).unwrap_or_default();
    let mut body = format!("{{\"query\":{}", quoted(&document.query));
    if let Some(variables) = variables {
        body.push_str(",\"variables\":");
        body.push_str(variables);
    }
    if let Some(name) = document
        .operation_name
        .as_deref()
        .filter(|name| !name.is_empty())
    {
        body.push_str(",\"operationName\":");
        body.push_str(&quoted(name));
    }
    body.push('}');
    body
}

#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(default)]
    errors: Vec<WireError>,
}

#[derive(Deserialize)]
struct WireError {
    #[serde(default)]
    message: String,
    #[serde(default)]
    path: Vec<serde_json::Value>,
    #[serde(default)]
    locations: Vec<WireLocation>,
}

#[derive(Deserialize)]
struct WireLocation {
    line: u32,
    column: u32,
}

fn joined(path: &[serde_json::Value]) -> Option<String> {
    let mut out = String::new();
    for step in path {
        match step {
            serde_json::Value::Number(index) => {
                out.push('[');
                out.push_str(&index.to_string());
                out.push(']');
            }
            serde_json::Value::String(field) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(field);
            }
            _ => {}
        }
    }
    (!out.is_empty()).then_some(out)
}

/// `None` when the body is not a GraphQL answer: no `data` and no `errors` to read.
pub fn result(body: &str) -> Option<GraphqlResult> {
    let envelope: Envelope<'_> = serde_json::from_str(body).ok()?;
    if envelope.data.is_none() && envelope.errors.is_empty() {
        return None;
    }
    Some(GraphqlResult {
        data: envelope
            .data
            .filter(|raw| raw.get() != "null")
            .and_then(|raw| pretty_json(raw.get())),
        errors: envelope
            .errors
            .into_iter()
            .map(|error| GraphqlError {
                message: error.message,
                path: joined(&error.path),
                location: error
                    .locations
                    .first()
                    .map(|location| format!("{}:{}", location.line, location.column)),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_operations_are_read_past_strings_comments_and_selections() {
        let query = r#"
            # query Commented { x }
            query Invoices($first: Int) { invoices(first: $first, note: "mutation Fake") { id } }
            mutation Pay { pay { id } }
            { anonymous }
            subscription
              Ticks { tick }
        "#;

        assert_eq!(operations(query), ["Invoices", "Pay", "Ticks"]);
        assert!(operations("{ me { id } }").is_empty());
    }

    #[test]
    fn the_body_carries_the_variables_as_typed_and_the_operation_chosen() {
        let document = GraphqlDocument {
            query: "query A { a }".to_string(),
            variables: " {\"id\": 12345678901234567890} ".to_string(),
            operation_name: Some("A".to_string()),
            as_get: false,
        };

        let variables = variables(&document).unwrap();
        assert_eq!(
            body(&document, variables),
            r#"{"query":"query A { a }","variables":{"id": 12345678901234567890},"operationName":"A"}"#
        );
        assert!(
            super::variables(&GraphqlDocument {
                variables: "[1]".to_string(),
                ..GraphqlDocument::default()
            })
            .is_err()
        );
        assert!(
            describe(&GraphqlDocument {
                variables: "{\"a\":".to_string(),
                ..GraphqlDocument::default()
            })
            .variables_problem
            .is_some()
        );
    }

    #[test]
    fn an_answer_is_read_as_data_and_errors_apart() {
        let read = result(
            r#"{"data":{"invoice":null,"n":1.10},"errors":[{"message":"Not found","path":["invoices",0,"amount"],"locations":[{"line":3,"column":5}]}]}"#,
        )
        .unwrap();

        assert_eq!(
            read.data.as_deref(),
            Some("{\n  \"invoice\": null,\n  \"n\": 1.10\n}")
        );
        assert_eq!(
            read.errors,
            [GraphqlError {
                message: "Not found".to_string(),
                path: Some("invoices[0].amount".to_string()),
                location: Some("3:5".to_string()),
            }]
        );
        assert_eq!(result(r#"{"data":null,"errors":[]}"#), None);
        assert_eq!(result(r#"{"id":7}"#), None);
        assert_eq!(result("not json"), None);
    }
}
