//! The HTTP collections as the interface sees them: a tree of collections, folders and
//! requests, a request's sealed document, and where an item can be moved to.

use super::environments::Variable;
use super::graphql::GraphqlDocument;
use super::transport::TransportSettings;
use super::websocket::WebsocketDocument;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use crate::error::ValidationError;

closed_enum! {
    pub enum HttpMethod {
        #[default]
        Get = "GET",
        Post = "POST",
        Put = "PUT",
        Patch = "PATCH",
        Delete = "DELETE",
        Head = "HEAD",
        Options = "OPTIONS",
    }
}

impl HttpMethod {
    /// `GET` and `HEAD` seldom carry a body: many servers ignore one, and some refuse it. A
    /// `DELETE` with one is in real use, and an `OPTIONS` may have one by its own definition.
    #[must_use]
    pub fn seldom_carries_a_body(self) -> bool {
        matches!(self, Self::Get | Self::Head)
    }
}

closed_enum! {
    /// Chosen when the request is created, and its own for good: a request is not turned into
    /// a socket. The rail writes the method before its name, or `GQL` and `WS`.
    pub enum RequestKind {
        #[default]
        Http = "http",
        Graphql = "graphql",
        Websocket = "websocket",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HttpItemKind {
    Collection,
    Folder,
    Request,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpItem {
    pub kind: HttpItemKind,
    pub id: String,
}

/// What a collection or a folder hands its requests (#478, #484). Each field is
/// `#[serde(default)]`: one added later needs no migration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ContainerSettings {
    /// `Inherit` on a collection is no auth at all: nothing is above it.
    pub auth: RequestAuth,
    pub headers: Vec<KeyValue>,
    pub transport: TransportSettings,
    /// Over the environment's and the globals', under a folder's and a request's own.
    pub variables: Vec<Variable>,
}

/// What a request sends as its body. A file is a path, read by Rust when the request is sent:
/// its bytes never enter the library.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RequestBody {
    #[default]
    None,
    Json {
        text: String,
    },
    Text {
        text: String,
    },
    /// `application/x-www-form-urlencoded`.
    Form {
        fields: Vec<KeyValue>,
    },
    /// `multipart/form-data`: a text field, or a file by its path.
    Multipart {
        parts: Vec<FormPart>,
    },
    Binary {
        path: String,
    },
}

impl RequestBody {
    /// Whether anything leaves as the body: a kind chosen and left empty sends none.
    #[must_use]
    pub fn carries_something(&self) -> bool {
        match self {
            Self::None => false,
            Self::Json { text } | Self::Text { text } => !text.is_empty(),
            Self::Form { fields } => fields
                .iter()
                .any(|row| row.enabled && !row.key.trim().is_empty()),
            Self::Multipart { parts } => parts
                .iter()
                .any(|part| part.enabled && !part.key.trim().is_empty()),
            Self::Binary { path } => !path.trim().is_empty(),
        }
    }

    /// A body is sent as typed whatever the method — some APIs read one on a `GET` — and this
    /// is what the editor says it by.
    #[must_use]
    pub fn is_unusual_on(&self, method: HttpMethod) -> bool {
        method.seldom_carries_a_body() && self.carries_something()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct FormPart {
    pub enabled: bool,
    pub key: String,
    /// The text sent, or the path of the file sent.
    pub value: String,
    pub file: bool,
    pub description: String,
}

impl Default for FormPart {
    fn default() -> Self {
        Self {
            enabled: true,
            key: String::new(),
            value: String::new(),
            file: false,
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum KeyPlace {
    #[default]
    Header,
    Query,
}

/// `Inherit` takes the auth of the nearest folder, then the collection, that sets one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RequestAuth {
    #[default]
    Inherit,
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        name: String,
        value: String,
        place: KeyPlace,
    },
}

/// A request's parts, sealed as one document. Each field is `#[serde(default)]`: one added
/// later needs no migration, and a document written by an older version still opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestDocument {
    pub url: String,
    /// In step with the URL's query: an enabled row is in it, a disabled one is kept aside.
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body: RequestBody,
    pub auth: RequestAuth,
    pub description: String,
    /// Read only for a GraphQL request, which sends it in place of the body.
    pub graphql: GraphqlDocument,
    /// Read only for a WebSocket.
    pub websocket: WebsocketDocument,
    /// Over what the folders and the collection set.
    pub transport: TransportSettings,
    /// The nearest level a `{{name}}` resolves from.
    pub variables: Vec<Variable>,
}

/// A row of a request's parameters or headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyValue {
    pub enabled: bool,
    pub key: String,
    pub value: String,
    pub description: String,
}

/// Enabled: a row is typed to be sent.
impl Default for KeyValue {
    fn default() -> Self {
        Self {
            enabled: true,
            key: String::new(),
            value: String::new(),
            description: String::new(),
        }
    }
}

/// What stops a header from being sent: the editor marks its row by it, a send names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HeaderFault {
    Name,
    Value,
}

/// What an HTTP token holds beside letters and digits (RFC 9110, 5.6.2).
const TOKEN_MARKS: &[u8] = b"!#$%&'*+-.^_`|~";

/// What stops a header from being sent, if anything: its name is an HTTP token — no space, no
/// colon, no accent —, its value holds no line break nor any other control character. Nothing
/// checked a name, and one typed with its value in it failed every request under its folder.
#[must_use]
pub fn header_fault(name: &str, value: &str) -> Option<HeaderFault> {
    // A `{{variable}}` stands for a letter: what it resolves to is the send's to judge.
    let name = super::variables::as_letters(name.trim());
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || TOKEN_MARKS.contains(&byte))
    {
        return Some(HeaderFault::Name);
    }
    value
        .chars()
        .any(|character| character.is_control() && character != '\t')
        .then_some(HeaderFault::Value)
}

/// Each row's fault, by its rank. A row switched off or left without a name is not sent, and
/// not judged.
#[must_use]
pub fn header_faults(rows: &[KeyValue]) -> Vec<Option<HeaderFault>> {
    rows.iter()
        .map(|row| {
            (row.enabled && !row.key.trim().is_empty())
                .then(|| header_fault(&row.key, &row.value))
                .flatten()
        })
        .collect()
}

/// `Nom: valeur` lines as rows, what follows the first colon the value: a line copied from a
/// browser's network tab, or a header typed whole in the name's field. A line without a colon
/// is no header, and an HTTP/2 pseudo-header (`:authority`) is the client's to write.
#[must_use]
pub fn headers_from_text(text: &str) -> Vec<KeyValue> {
    text.lines()
        .filter_map(|line| {
            let (name, value) = line.trim().split_once(':')?;
            (!name.trim().is_empty()).then(|| KeyValue {
                key: name.trim().to_string(),
                value: value.trim().to_string(),
                ..KeyValue::default()
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpCollection {
    pub id: String,
    pub name: String,
    pub position: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpFolder {
    pub id: String,
    pub collection_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub position: u32,
}

/// A request as the tree lists it: no document, which is read by id when it opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSummary {
    pub id: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub id: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestDraft {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
}

/// What a request's first save creates on its way: the collection to put it in, a folder
/// under the place it names, or both — the folder then goes in the new collection.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NewPlace {
    #[specta(optional)]
    pub collection: Option<String>,
    #[specta(optional)]
    pub folder: Option<String>,
}

/// A field left out is untouched. ⚠️ No `kind`: it is chosen when a request is created, so
/// there is nothing here to refuse.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestPatch {
    #[specta(optional)]
    pub name: Option<String>,
    #[specta(optional)]
    pub method: Option<HttpMethod>,
    #[specta(optional)]
    pub document: Option<RequestDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HttpNode {
    Folder {
        folder: HttpFolder,
        children: Vec<HttpNode>,
    },
    Request {
        request: HttpRequestSummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpCollectionNode {
    pub collection: HttpCollection,
    pub children: Vec<HttpNode>,
}

/// Built by Rust: the front draws it and never joins a request to its folder itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpTree {
    pub collections: Vec<HttpCollectionNode>,
}

/// What a deletion takes with it, said before it happens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpContents {
    pub folders: u32,
    pub requests: u32,
}

/// Under which parent, and at which rank among its folders and requests.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpPlace {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub index: u32,
}

pub fn validated_name(raw: &str) -> Result<String, ValidationError> {
    crate::name::readable(raw, "collection, folder or request")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_method_and_kind_survives_its_stored_spelling() {
        for method in HttpMethod::ALL {
            assert_eq!(method.as_str().parse::<HttpMethod>(), Ok(method));
        }
        for kind in RequestKind::ALL {
            assert_eq!(kind.as_str().parse::<RequestKind>(), Ok(kind));
        }
    }

    /// Key `X-Recette: 1`, value `rsgfdregr`: accepted, then every request failed on its URL.
    #[test]
    fn a_header_name_is_a_token_and_a_value_holds_no_control_character() {
        for name in ["X-Recette", "accept", "X_a.b~c!#$%&'*+^`|", " Trimmed "] {
            assert_eq!(header_fault(name, "v"), None, "{name}");
        }
        for name in [
            "X-Recette: 1",
            "X Recette",
            "Clé",
            "",
            "a\tb",
            "a(b)",
            "a\"b",
            "a/b",
            "a@b",
        ] {
            assert_eq!(header_fault(name, "v"), Some(HeaderFault::Name), "{name}");
        }
        for value in [
            "",
            "application/json; charset=utf-8",
            "Équipe/1",
            "a\tb",
            " padded ",
        ] {
            assert_eq!(header_fault("X-A", value), None, "{value:?}");
        }
        for value in ["a\nb", "a\r\nX-Injected: 1", "bell\u{7}", "del\u{7f}"] {
            assert_eq!(
                header_fault("X-A", value),
                Some(HeaderFault::Value),
                "{value:?}"
            );
        }
    }

    /// The rule is what the client can build: it must not refuse more, nor let through more.
    #[test]
    fn the_rule_agrees_with_what_the_client_builds() {
        use reqwest::header::{HeaderName, HeaderValue};

        for name in [
            "X-Recette",
            "X Recette",
            "X:1",
            "Clé",
            "a(b)",
            "x_y",
            "a`b",
            "",
        ] {
            assert_eq!(
                header_fault(name, "").is_none(),
                HeaderName::from_bytes(name.as_bytes()).is_ok(),
                "{name}"
            );
        }
        for value in [
            "plain",
            "tab\there",
            "Équipe",
            "line\nbreak",
            "nul\u{0}",
            "del\u{7f}",
        ] {
            assert_eq!(
                header_fault("X-A", value).is_none(),
                HeaderValue::from_str(value).is_ok(),
                "{value:?}"
            );
        }
    }

    #[test]
    fn a_variable_in_a_header_stands_for_a_letter_until_it_is_filled() {
        assert_eq!(header_fault("X-{{tenant}}-Id", "{{token}}"), None);
        assert_eq!(header_fault("{{name}}", ""), None);
        // Not a variable: a dotted path is text, as everywhere else.
        assert_eq!(header_fault("{{user.name}}", ""), Some(HeaderFault::Name));
        assert_eq!(header_fault("X {{tenant}}", ""), Some(HeaderFault::Name));
    }

    #[test]
    fn a_row_switched_off_or_without_a_name_is_not_judged() {
        let row = |enabled: bool, key: &str, value: &str| KeyValue {
            enabled,
            key: key.to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        };

        assert_eq!(
            header_faults(&[
                row(true, "Accept", "*/*"),
                row(true, "X Recette", "1"),
                row(false, "X Recette", "1"),
                row(true, "  ", "orphan"),
                row(true, "X-A", "a\nb"),
            ]),
            vec![
                None,
                Some(HeaderFault::Name),
                None,
                None,
                Some(HeaderFault::Value)
            ]
        );
    }

    #[test]
    fn header_lines_are_read_as_rows_at_their_first_colon() {
        let read = |text: &str| -> Vec<(String, String)> {
            headers_from_text(text)
                .into_iter()
                .map(|row| {
                    assert!(row.enabled);
                    (row.key, row.value)
                })
                .collect()
        };
        let pair = |key: &str, value: &str| (key.to_string(), value.to_string());

        assert_eq!(read("X-Recette: 1"), vec![pair("X-Recette", "1")]);
        assert_eq!(read("X-Recette:"), vec![pair("X-Recette", "")]);
        assert_eq!(
            read("Referer: https://exemple.fr:8443/a"),
            vec![pair("Referer", "https://exemple.fr:8443/a")]
        );
        // As a browser's network tab copies them: a request line, pseudo-headers, blank lines.
        assert_eq!(
            read(
                "GET /events HTTP/2\r\n:authority: api.exemple.fr\r\naccept: */*\r\n\r\n  x-client : web  \n"
            ),
            vec![pair("accept", "*/*"), pair("x-client", "web")]
        );
        assert_eq!(read("no colon here"), Vec::new());
    }

    /// A `POST` switched to `GET` kept its body and sent it, and nothing said so.
    #[test]
    fn a_body_is_unusual_on_a_get_and_a_head_and_on_no_other_method() {
        let json = RequestBody::Json {
            text: "{\"a\":1}".to_string(),
        };
        let warned: Vec<HttpMethod> = HttpMethod::ALL
            .into_iter()
            .filter(|method| json.is_unusual_on(*method))
            .collect();
        assert_eq!(warned, vec![HttpMethod::Get, HttpMethod::Head]);

        for method in HttpMethod::ALL {
            assert!(!RequestBody::None.is_unusual_on(method));
        }
    }

    #[test]
    fn a_kind_chosen_and_left_empty_is_no_body() {
        let row = |enabled: bool, key: &str| KeyValue {
            enabled,
            key: key.to_string(),
            ..KeyValue::default()
        };
        let part = |enabled: bool, key: &str| FormPart {
            enabled,
            key: key.to_string(),
            ..FormPart::default()
        };
        let empty = [
            RequestBody::None,
            RequestBody::Json {
                text: String::new(),
            },
            RequestBody::Text {
                text: String::new(),
            },
            RequestBody::Form {
                fields: vec![row(false, "q"), row(true, " ")],
            },
            RequestBody::Multipart {
                parts: vec![part(false, "file"), part(true, "")],
            },
            RequestBody::Binary {
                path: "  ".to_string(),
            },
        ];
        for body in &empty {
            assert!(!body.carries_something(), "{body:?}");
        }

        let filled = [
            RequestBody::Text {
                text: " ".to_string(),
            },
            RequestBody::Form {
                fields: vec![row(true, "q")],
            },
            RequestBody::Multipart {
                parts: vec![part(true, "file")],
            },
            RequestBody::Binary {
                path: "C:/tmp/a.bin".to_string(),
            },
        ];
        for body in &filled {
            assert!(body.is_unusual_on(HttpMethod::Get), "{body:?}");
        }
    }

    #[test]
    fn a_document_written_before_a_field_existed_still_opens() {
        let document: RequestDocument =
            serde_json::from_str(r#"{"url":"https://exemple.fr"}"#).unwrap();

        assert_eq!(document.url, "https://exemple.fr");
        assert_eq!(document.description, "");
        assert_eq!(
            serde_json::from_str::<RequestDocument>(r#"{"later":1}"#).unwrap(),
            RequestDocument::default()
        );
    }
}
