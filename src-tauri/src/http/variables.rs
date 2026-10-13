//! `{{name}}` in a request, resolved from the nearest level that sets it: the request, its
//! folders, its collection, the active environment, the globals. Here, when the request is
//! sent, and nowhere else. ⚠️ A snippet's `{{field}}` looks alike and is `notes::placeholder`'s:
//! the two resolvers never meet.

use std::collections::HashMap;

use serde::Serialize;
use specta::Type;

use super::environments::{Variable, VariableKind, is_variable_name, variable_faults};
use super::graphql::GraphqlDocument;
use super::model::{FormPart, KeyValue, RequestAuth, RequestBody, RequestDocument, RequestKind};
use super::settings::Inherited;
use crate::closed_enum::closed_enum;

closed_enum! {
    /// Where a variable resolves from, the nearest first.
    pub enum VariableScope {
        #[default]
        Request = "request",
        Folder = "folder",
        Collection = "collection",
        Environment = "environment",
        Globals = "globals",
    }
}

/// One level's variables, and the name of what holds them: a folder, a collection, an
/// environment; empty for the request and the globals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableLayer {
    pub scope: VariableScope,
    pub from: String,
    pub variables: Vec<Variable>,
}

/// A variable as a request resolves it: the value the nearest level gives it, and the
/// levels further away it hides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScopedVariable {
    pub name: String,
    /// With every `{{name}}` it holds filled, as it is sent.
    pub value: String,
    pub kind: VariableKind,
    pub scope: VariableScope,
    /// The folder, collection or environment that sets it.
    pub from: Option<String>,
    pub shadows: Vec<VariableScope>,
}

/// A `{{name}}` a request holds, and what it resolves to — nothing, when no level sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsedVariable {
    pub name: String,
    pub resolved: Option<ScopedVariable>,
}

/// A value naming another is filled again, this many times at most: past it, a variable that
/// names itself stays as typed and the send names it.
const NESTING: usize = 8;

/// Each `{{name}}` of a text, by its byte range and its name, in order.
fn occurrences(text: &str) -> Vec<(std::ops::Range<usize>, &str)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("{{").map(|at| from + at) {
        let start = open + 2;
        let Some(close) = text[start..].find("}}").map(|at| start + at) else {
            break;
        };
        let name = &text[start..close];
        if is_variable_name(name) {
            found.push((open..close + 2, name));
            from = close + 2;
        } else {
            from = open + 1;
        }
    }
    found
}

/// The names a text holds, in order, each once.
pub fn names_in(text: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (_, name) in occurrences(text) {
        if !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// Each `{{name}}` as one letter: what a header's name is judged by until it is filled.
#[must_use]
pub fn as_letters(text: &str) -> String {
    let mut filled = String::with_capacity(text.len());
    let mut last = 0;
    for (range, _) in occurrences(text) {
        filled.push_str(&text[last..range.start]);
        filled.push('x');
        last = range.end;
    }
    filled.push_str(&text[last..]);
    filled
}

fn fill_once(text: &str, values: &HashMap<String, String>) -> String {
    let mut filled = String::with_capacity(text.len());
    let mut last = 0;
    for (range, name) in occurrences(text) {
        if let Some(value) = values.get(name) {
            filled.push_str(&text[last..range.start]);
            filled.push_str(value);
            last = range.end;
        }
    }
    filled.push_str(&text[last..]);
    filled
}

/// What every level in force sets, the nearest winning a name.
#[derive(Debug, Clone, Default)]
pub struct Resolver {
    found: Vec<ScopedVariable>,
    values: HashMap<String, String>,
}

impl Resolver {
    /// `layers` the nearest first. A row switched off, or one its table marks, is left out.
    #[must_use]
    pub fn new(layers: &[VariableLayer]) -> Self {
        let mut found: Vec<ScopedVariable> = Vec::new();
        for layer in layers {
            let faults = variable_faults(&layer.variables);
            for (variable, fault) in layer.variables.iter().zip(faults) {
                let name = variable.name.trim();
                if !variable.enabled || name.is_empty() || fault.is_some() {
                    continue;
                }
                if let Some(known) = found.iter_mut().find(|known| known.name == name) {
                    known.shadows.push(layer.scope);
                    continue;
                }
                found.push(ScopedVariable {
                    name: name.to_string(),
                    value: variable.value.clone(),
                    kind: variable.kind,
                    scope: layer.scope,
                    from: (!layer.from.is_empty()).then(|| layer.from.clone()),
                    shadows: Vec::new(),
                });
            }
        }
        let raw: HashMap<String, String> = found
            .iter()
            .map(|variable| (variable.name.clone(), variable.value.clone()))
            .collect();
        for variable in &mut found {
            for _ in 0..NESTING {
                let next = fill_once(&variable.value, &raw);
                if next == variable.value {
                    break;
                }
                variable.value = next;
            }
        }
        let values = found
            .iter()
            .map(|variable| (variable.name.clone(), variable.value.clone()))
            .collect();
        Self { found, values }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ScopedVariable> {
        self.found.iter().find(|variable| variable.name == name)
    }

    /// Every `{{name}}` a level sets, filled; the rest left as typed, for the send to name.
    #[must_use]
    pub fn fill(&self, text: &str) -> String {
        fill_once(text, &self.values)
    }

    /// The values of the secrets in force: what the history masks.
    #[must_use]
    pub fn secrets(&self) -> Vec<(String, String)> {
        self.found
            .iter()
            .filter(|variable| variable.kind == VariableKind::Secret && !variable.value.is_empty())
            .map(|variable| (variable.name.clone(), variable.value.clone()))
            .collect()
    }

    /// The names `texts` hold, in order, each with what it resolves to.
    #[must_use]
    pub fn used(&self, texts: &[&str]) -> Vec<UsedVariable> {
        let mut used: Vec<UsedVariable> = Vec::new();
        for text in texts {
            for name in names_in(text) {
                if !used.iter().any(|known| known.name == name) {
                    used.push(UsedVariable {
                        resolved: self.get(&name).cloned(),
                        name,
                    });
                }
            }
        }
        used
    }
}

/// Shorter than this, a secret's value is not looked for: it would be found inside every word.
const HIDDEN_FROM: usize = 3;

/// Each secret's value written back as its `{{name}}`, as typed and as a query encodes it: what
/// shows a request once it left — the timeline, the history — never shows the value. The longest
/// first, so a value inside another is not cut out of it.
#[must_use]
pub fn hide(text: &str, secrets: &[(String, String)]) -> String {
    let mut found: Vec<&(String, String)> = secrets
        .iter()
        .filter(|(_, value)| value.chars().count() >= HIDDEN_FROM)
        .collect();
    found.sort_by_key(|(_, value)| std::cmp::Reverse(value.len()));
    let mut text = text.to_string();
    for (name, value) in found {
        let written = format!("{{{{{name}}}}}");
        text = text.replace(value.as_str(), &written);
        let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
        if encoded != *value {
            text = text.replace(&encoded, &written);
        }
    }
    text
}

fn fill_rows(rows: &mut [KeyValue], resolver: &Resolver) {
    for row in rows {
        row.key = resolver.fill(&row.key);
        row.value = resolver.fill(&row.value);
    }
}

fn fill_auth(auth: &mut RequestAuth, resolver: &Resolver) {
    match auth {
        RequestAuth::Basic { username, password } => {
            *username = resolver.fill(username);
            *password = resolver.fill(password);
        }
        RequestAuth::Bearer { token } => *token = resolver.fill(token),
        RequestAuth::ApiKey { name, value, .. } => {
            *name = resolver.fill(name);
            *value = resolver.fill(value);
        }
        RequestAuth::Inherit | RequestAuth::None => {}
    }
}

fn fill_body(body: &mut RequestBody, resolver: &Resolver) {
    match body {
        RequestBody::Json { text } | RequestBody::Text { text } => *text = resolver.fill(text),
        RequestBody::Form { fields } => fill_rows(fields, resolver),
        RequestBody::Multipart { parts } => {
            for FormPart { key, value, .. } in parts {
                *key = resolver.fill(key);
                *value = resolver.fill(value);
            }
        }
        RequestBody::Binary { path } => *path = resolver.fill(path),
        RequestBody::None => {}
    }
}

fn fill_graphql(graphql: &mut GraphqlDocument, resolver: &Resolver) {
    graphql.query = resolver.fill(&graphql.query);
    graphql.variables = resolver.fill(&graphql.variables);
}

/// The request and what it inherits with every `{{name}}` a level sets filled — in the URL,
/// the parameters, the headers, the body, the auth and the GraphQL query and variables.
#[must_use]
pub fn resolve(
    document: &RequestDocument,
    inherited: &Inherited,
    resolver: &Resolver,
) -> (RequestDocument, Inherited) {
    let mut document = document.clone();
    document.url = resolver.fill(&document.url);
    fill_rows(&mut document.params, resolver);
    fill_rows(&mut document.headers, resolver);
    fill_body(&mut document.body, resolver);
    fill_auth(&mut document.auth, resolver);
    fill_graphql(&mut document.graphql, resolver);

    let mut inherited = inherited.clone();
    for known in &mut inherited.headers {
        known.header.key = resolver.fill(&known.header.key);
        known.header.value = resolver.fill(&known.header.value);
    }
    fill_auth(&mut inherited.auth, resolver);
    (document, inherited)
}

/// Every text of a request a `{{name}}` can stand in, in the order the editor shows them: what
/// is sent from it, and nothing that is not.
#[must_use]
pub fn texts<'a>(
    kind: RequestKind,
    document: &'a RequestDocument,
    inherited: &'a Inherited,
) -> Vec<&'a str> {
    let sent = |rows: &'a [KeyValue]| {
        rows.iter()
            .filter(|row| row.enabled && !row.key.trim().is_empty())
    };
    let mut texts: Vec<&str> = vec![&document.url];
    for row in inherited
        .headers
        .iter()
        .map(|known| &known.header)
        .chain(sent(&document.headers))
    {
        texts.extend([row.key.as_str(), row.value.as_str()]);
    }
    let auth = match &document.auth {
        RequestAuth::Inherit => &inherited.auth,
        own => own,
    };
    match auth {
        RequestAuth::Basic { username, password } => {
            texts.extend([username.as_str(), password.as_str()]);
        }
        RequestAuth::Bearer { token } => texts.push(token),
        RequestAuth::ApiKey { name, value, .. } => texts.extend([name.as_str(), value.as_str()]),
        RequestAuth::Inherit | RequestAuth::None => {}
    }
    match kind {
        RequestKind::Graphql => {
            texts.extend([
                document.graphql.query.as_str(),
                document.graphql.variables.as_str(),
            ]);
        }
        RequestKind::Websocket => {}
        RequestKind::Http => match &document.body {
            RequestBody::Json { text } | RequestBody::Text { text } => texts.push(text),
            RequestBody::Form { fields } => {
                for row in sent(fields) {
                    texts.extend([row.key.as_str(), row.value.as_str()]);
                }
            }
            RequestBody::Multipart { parts } => {
                for part in parts.iter().filter(|part| part.enabled) {
                    texts.extend([part.key.as_str(), part.value.as_str()]);
                }
            }
            RequestBody::Binary { path } => texts.push(path),
            RequestBody::None => {}
        },
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str, value: &str) -> Variable {
        Variable {
            name: name.to_string(),
            value: value.to_string(),
            ..Variable::default()
        }
    }

    fn layer(scope: VariableScope, from: &str, variables: Vec<Variable>) -> VariableLayer {
        VariableLayer {
            scope,
            from: from.to_string(),
            variables,
        }
    }

    #[test]
    fn every_scope_survives_its_stored_spelling() {
        for scope in VariableScope::ALL {
            assert_eq!(scope.as_str().parse::<VariableScope>(), Ok(scope));
        }
    }

    #[test]
    fn a_name_is_what_a_field_holds_and_nothing_else() {
        assert_eq!(
            names_in("{{baseUrl}}/a/{{id}}?q={{id}}&x={{ spaced }}&y={{a.b}}&{{"),
            ["baseUrl", "id"]
        );
        assert_eq!(names_in("{{{inner}}}"), ["inner"]);
        assert!(names_in("no braces, {single}, {{}}").is_empty());
    }

    #[test]
    fn the_nearest_level_wins_and_says_what_it_hides() {
        let resolver = Resolver::new(&[
            layer(VariableScope::Request, "", vec![variable("page", "2")]),
            layer(
                VariableScope::Folder,
                "Factures",
                vec![variable("page", "1")],
            ),
            layer(
                VariableScope::Environment,
                "Staging",
                vec![
                    variable("baseUrl", "https://staging"),
                    variable("page", "9"),
                ],
            ),
            layer(
                VariableScope::Globals,
                "",
                vec![variable("baseUrl", "https://global")],
            ),
        ]);

        let page = resolver.get("page").unwrap();
        assert_eq!(
            (page.value.as_str(), page.scope),
            ("2", VariableScope::Request)
        );
        assert_eq!(page.from, None);
        assert_eq!(
            page.shadows,
            [VariableScope::Folder, VariableScope::Environment]
        );
        let base = resolver.get("baseUrl").unwrap();
        assert_eq!(base.from.as_deref(), Some("Staging"));
        assert_eq!(
            resolver.fill("{{baseUrl}}/invoices?page={{page}}&tenant={{tenant}}"),
            "https://staging/invoices?page=2&tenant={{tenant}}"
        );
    }

    #[test]
    fn a_row_switched_off_or_marked_is_left_out() {
        let mut off = variable("token", "old");
        off.enabled = false;
        let resolver = Resolver::new(&[
            layer(
                VariableScope::Request,
                "",
                vec![
                    off,
                    variable("bad name", "x"),
                    variable("a", "1"),
                    variable("a", "2"),
                ],
            ),
            layer(VariableScope::Globals, "", vec![variable("token", "new")]),
        ]);

        assert_eq!(resolver.get("token").unwrap().value, "new");
        assert_eq!(resolver.get("a").unwrap().value, "1");
        assert!(resolver.get("bad name").is_none());
    }

    #[test]
    fn a_value_naming_another_is_filled_and_one_naming_itself_stops() {
        let resolver = Resolver::new(&[layer(
            VariableScope::Environment,
            "Local",
            vec![
                variable("host", "localhost:{{port}}"),
                variable("port", "8080"),
                variable("baseUrl", "http://{{host}}/api"),
                variable("loop", "a{{loop}}"),
            ],
        )]);

        assert_eq!(resolver.fill("{{baseUrl}}"), "http://localhost:8080/api");
        assert!(resolver.fill("{{loop}}").contains("{{loop}}"));
    }

    #[test]
    fn what_a_request_uses_is_listed_once_in_order_resolved_or_not() {
        let resolver = Resolver::new(&[layer(
            VariableScope::Globals,
            "",
            vec![variable("baseUrl", "https://x")],
        )]);

        let used = resolver.used(&["{{baseUrl}}/{{id}}", "{{id}} {{baseUrl}}"]);
        assert_eq!(
            used.iter()
                .map(|known| known.name.as_str())
                .collect::<Vec<_>>(),
            ["baseUrl", "id"]
        );
        assert_eq!(
            used[0].resolved.as_ref().map(|found| found.scope),
            Some(VariableScope::Globals)
        );
        assert!(used[1].resolved.is_none());
    }

    #[test]
    fn a_document_and_what_it_inherits_are_filled_everywhere_a_name_can_stand() {
        use super::super::model::KeyPlace;
        use super::super::settings::{HttpOrigin, InheritedHeader, inherit};

        let resolver = Resolver::new(&[layer(
            VariableScope::Environment,
            "Staging",
            vec![
                variable("host", "api.exemple.fr"),
                variable("token", "t0k3n"),
                variable("id", "42"),
            ],
        )]);
        let row = |key: &str, value: &str| KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        };
        let document = RequestDocument {
            url: "https://{{host}}/items/{{id}}".to_string(),
            params: vec![row("id", "{{id}}")],
            headers: vec![row("X-Item-{{id}}", "{{id}}")],
            body: RequestBody::Json {
                text: r#"{"id":{{id}}}"#.to_string(),
            },
            auth: RequestAuth::ApiKey {
                name: "key".to_string(),
                value: "{{token}}".to_string(),
                place: KeyPlace::Header,
            },
            ..RequestDocument::default()
        };
        let mut inherited = inherit(&[]);
        inherited.auth = RequestAuth::Bearer {
            token: "{{token}}".to_string(),
        };
        inherited.headers.push(InheritedHeader {
            header: row("X-Host", "{{host}}"),
            from: HttpOrigin {
                kind: super::super::model::HttpItemKind::Collection,
                id: "c".to_string(),
                name: "API".to_string(),
            },
        });

        let (document, inherited) = resolve(&document, &inherited, &resolver);

        assert_eq!(document.url, "https://api.exemple.fr/items/42");
        assert_eq!(document.params[0].value, "42");
        assert_eq!(
            (
                document.headers[0].key.as_str(),
                document.headers[0].value.as_str()
            ),
            ("X-Item-42", "42")
        );
        assert_eq!(
            document.body,
            RequestBody::Json {
                text: r#"{"id":42}"#.to_string()
            }
        );
        assert!(matches!(&document.auth, RequestAuth::ApiKey { value, .. } if value == "t0k3n"));
        assert_eq!(
            inherited.auth,
            RequestAuth::Bearer {
                token: "t0k3n".to_string()
            }
        );
        assert_eq!(inherited.headers[0].header.value, "api.exemple.fr");
    }

    #[test]
    fn a_secret_value_is_written_back_as_its_name_wherever_it_shows() {
        let secrets = vec![
            ("token".to_string(), "t0k3n".to_string()),
            ("long".to_string(), "t0k3n-and-more".to_string()),
            ("spaced".to_string(), "a b&c".to_string()),
            ("short".to_string(), "ab".to_string()),
        ];

        assert_eq!(
            hide("Bearer t0k3n, again t0k3n-and-more", &secrets),
            "Bearer {{token}}, again {{long}}"
        );
        assert_eq!(
            hide("https://x/?q=a+b%26c", &secrets),
            "https://x/?q={{spaced}}"
        );
        assert_eq!(hide("ab cd", &secrets), "ab cd");
        assert_eq!(hide("nothing here", &[]), "nothing here");
    }

    #[test]
    fn the_secrets_in_force_are_named_with_their_values() {
        let mut secret = variable("token", "t0k3n");
        secret.kind = VariableKind::Secret;
        let mut empty = variable("unset", "");
        empty.kind = VariableKind::Secret;
        let resolver = Resolver::new(&[layer(
            VariableScope::Globals,
            "",
            vec![secret, empty, variable("plain", "x")],
        )]);

        assert_eq!(
            resolver.secrets(),
            [("token".to_string(), "t0k3n".to_string())]
        );
    }
}
