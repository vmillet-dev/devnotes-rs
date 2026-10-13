//! A request written as code for another tool: one shape every target reads (`CodeRequest`),
//! built from what a send would write, and one module a target. The variables are resolved
//! here, a secret left as its `{{name}}` unless it is asked for in clear.

pub mod angular;
pub mod axios;
pub mod curl;
pub mod fetch;
pub mod httpie;
pub mod javascript;
pub mod powershell;
pub mod undici;
pub mod wget;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::environments::VariableKind;
use super::graphql;
use super::model::{HttpMethod, KeyPlace, RequestAuth, RequestBody, RequestDocument, RequestKind};
use super::send;
use super::settings::Inherited;
use super::variables::{self, Resolver, VariableLayer};
use crate::closed_enum::closed_enum;
use crate::notes::language::Language;

closed_enum! {
    /// What the code is written for; the front lists them and names them.
    pub enum CodeTarget {
        #[default]
        Curl = "curl",
        Httpie = "httpie",
        Wget = "wget",
        Powershell = "powershell",
        Fetch = "fetch",
        Axios = "axios",
        Undici = "undici",
        Angular = "angular",
    }
}

impl CodeTarget {
    /// What a snippet of it is saved as.
    #[must_use]
    pub fn language(self) -> Language {
        match self {
            Self::Curl | Self::Httpie | Self::Wget => Language::Sh,
            Self::Powershell => Language::Ps1,
            Self::Fetch | Self::Axios | Self::Undici => Language::Js,
            Self::Angular => Language::Ts,
        }
    }
}

/// The dialog's four switches, each its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodeOptions {
    /// Off, every `{{name}}` is written as typed.
    pub resolve_variables: bool,
    /// A secret written in clear: the front asks first, every time.
    pub include_secrets: bool,
    /// A command line's arguments one a line: a program is laid out whatever it says.
    pub multiline: bool,
    pub follow_redirects: bool,
}

/// What a target could not write, said by the front: wget sends no multipart form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CodeNotice {
    MultipartUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCode {
    pub code: String,
    pub language: Language,
    /// The secrets left as their `{{name}}`, in the order the request names them.
    pub secrets: Vec<String>,
    /// The names nothing sets, written as typed.
    pub unresolved: Vec<String>,
    pub notice: Option<CodeNotice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePart {
    pub name: String,
    /// The text, or the path of the file.
    pub value: String,
    pub file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeBody {
    None,
    Text(String),
    Form(Vec<(String, String)>),
    Multipart(Vec<CodePart>),
    /// A file sent as the body, by its path.
    File(String),
}

/// The request as every target reads it: what a send would write, the application's own
/// headers left out — the program names itself — and a Basic auth apart, each target having
/// its own way to write one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub basic: Option<(String, String)>,
    pub body: CodeBody,
    pub follow_redirects: bool,
}

impl CodeRequest {
    /// The `Content-Type` the headers set, if any: a target that takes it apart reads it here.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str())
    }
}

/// `api.example.com/users` is written as `http://api.example.com/users`, as it is sent.
fn with_scheme(url: &str) -> String {
    let url = url.trim();
    if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{url}")
    }
}

fn append_query(url: &mut String, pairs: &[(&str, &str)]) {
    for (name, value) in pairs {
        url.push(if url.contains('?') { '&' } else { '?' });
        let encoded: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair(name, value)
            .finish();
        url.push_str(&encoded);
    }
}

/// What a send would write, as a target reads it.
#[must_use]
pub fn request_of(
    kind: RequestKind,
    method: HttpMethod,
    document: &RequestDocument,
    inherited: &Inherited,
    follow_redirects: bool,
) -> CodeRequest {
    let mut url = with_scheme(&document.url);
    let auth = send::auth_of(document, inherited);
    if let RequestAuth::ApiKey {
        name,
        value,
        place: KeyPlace::Query,
    } = auth
        && !name.trim().is_empty()
    {
        append_query(&mut url, &[(name.trim(), value)]);
    }
    let basic = match auth {
        RequestAuth::Basic { username, password } => Some((username.clone(), password.clone())),
        _ => None,
    };
    let (method, body) = match kind {
        RequestKind::Graphql if document.graphql.as_get => {
            let variables = graphql::variables(&document.graphql).ok().flatten();
            let mut pairs = vec![("query", document.graphql.query.as_str())];
            if let Some(variables) = variables {
                pairs.push(("variables", variables));
            }
            if let Some(name) = document
                .graphql
                .operation_name
                .as_deref()
                .filter(|name| !name.is_empty())
            {
                pairs.push(("operationName", name));
            }
            append_query(&mut url, &pairs);
            (HttpMethod::Get, CodeBody::None)
        }
        RequestKind::Graphql => {
            let variables = graphql::variables(&document.graphql).ok().flatten();
            (
                HttpMethod::Post,
                CodeBody::Text(graphql::body(&document.graphql, variables)),
            )
        }
        RequestKind::Http | RequestKind::Websocket => (method, body_of(&document.body)),
    };
    CodeRequest {
        method,
        url,
        headers: send::written_for_code(kind, document, inherited),
        basic,
        body,
        follow_redirects,
    }
}

fn body_of(body: &RequestBody) -> CodeBody {
    match body {
        RequestBody::None => CodeBody::None,
        RequestBody::Json { text } | RequestBody::Text { text } if text.is_empty() => {
            CodeBody::None
        }
        RequestBody::Json { text } | RequestBody::Text { text } => CodeBody::Text(text.clone()),
        RequestBody::Form { fields } => CodeBody::Form(
            fields
                .iter()
                .filter(|row| row.enabled && !row.key.trim().is_empty())
                .map(|row| (row.key.trim().to_string(), row.value.clone()))
                .collect(),
        ),
        RequestBody::Multipart { parts } => CodeBody::Multipart(
            parts
                .iter()
                .filter(|part| part.enabled && !part.key.trim().is_empty())
                .map(|part| CodePart {
                    name: part.key.trim().to_string(),
                    value: part.value.clone(),
                    file: part.file,
                })
                .collect(),
        ),
        RequestBody::Binary { path } if path.trim().is_empty() => CodeBody::None,
        RequestBody::Binary { path } => CodeBody::File(path.trim().to_string()),
    }
}

/// The layers a send resolves from, the secrets taken out unless they are asked for: a secret
/// left out is written as its `{{name}}`.
fn without_secrets(layers: &[VariableLayer]) -> Vec<VariableLayer> {
    layers
        .iter()
        .map(|layer| VariableLayer {
            variables: layer
                .variables
                .iter()
                .filter(|variable| variable.kind != VariableKind::Secret)
                .cloned()
                .collect(),
            ..layer.clone()
        })
        .collect()
}

/// The code of a request for `target`, from the layers its variables resolve from.
#[must_use]
pub fn generate(
    target: CodeTarget,
    options: CodeOptions,
    kind: RequestKind,
    method: HttpMethod,
    document: &RequestDocument,
    inherited: &Inherited,
    layers: &[VariableLayer],
) -> GeneratedCode {
    let everything = Resolver::new(layers);
    let mut secrets = Vec::new();
    let mut unresolved = Vec::new();
    for used in everything.used(&variables::texts(kind, document, inherited)) {
        match used.resolved {
            None => unresolved.push(used.name),
            Some(found) if found.kind == VariableKind::Secret && !options.include_secrets => {
                secrets.push(used.name);
            }
            Some(_) => {}
        }
    }
    let (document, inherited) = if options.resolve_variables {
        let resolver = if options.include_secrets {
            everything
        } else {
            Resolver::new(&without_secrets(layers))
        };
        variables::resolve(document, inherited, &resolver)
    } else {
        (document.clone(), inherited.clone())
    };
    let request = request_of(
        kind,
        method,
        &document,
        &inherited,
        options.follow_redirects,
    );
    let notice = (target == CodeTarget::Wget && matches!(request.body, CodeBody::Multipart(_)))
        .then_some(CodeNotice::MultipartUnsupported);
    let code = match target {
        CodeTarget::Curl => curl::write(&request, options.multiline),
        CodeTarget::Httpie => httpie::write(&request, options.multiline),
        CodeTarget::Wget => wget::write(&request, options.multiline),
        CodeTarget::Powershell => powershell::write(&request, options.multiline),
        CodeTarget::Fetch => fetch::write(&request),
        CodeTarget::Axios => axios::write(&request),
        CodeTarget::Undici => undici::write(&request),
        CodeTarget::Angular => angular::write(&request),
    };
    GeneratedCode {
        code,
        language: target.language(),
        secrets,
        unresolved,
        notice,
    }
}

/// `'…'` for a POSIX shell: a quote inside closes, escapes and reopens.
#[must_use]
pub fn shell_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Arguments on one line, or one a line under the first, each but the last ending on `\`.
#[must_use]
pub fn shell_lines(first: &str, arguments: &[String], multiline: bool) -> String {
    if !multiline {
        let mut line = first.to_string();
        for argument in arguments {
            line.push(' ');
            line.push_str(argument);
        }
        return line;
    }
    let mut lines = vec![first.to_string()];
    lines.extend(arguments.iter().map(|argument| format!("  {argument}")));
    let last = lines.len() - 1;
    lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            if at == last {
                line.clone()
            } else {
                format!("{line} \\")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `a=1&b=x%20y`, as a form is sent.
#[must_use]
pub fn form_encoded(fields: &[(String, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in fields {
        serializer.append_pair(name, value);
    }
    serializer.finish()
}

#[cfg(test)]
pub(crate) mod golden {
    //! The cases every target is held to, and the files that hold what each writes. Run with
    //! `GOLDEN_WRITE=1` to write them again, then read the diff before keeping it.

    use super::{CodeBody, CodePart, CodeRequest};
    use crate::http::model::HttpMethod;

    pub(crate) fn cases() -> Vec<(&'static str, CodeRequest)> {
        let get = CodeRequest {
            method: HttpMethod::Get,
            url: "https://api.exemple.fr/customers/cus_42/invoices?page=1&limit=50".to_string(),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                (
                    "Authorization".to_string(),
                    "Bearer {{accessToken}}".to_string(),
                ),
            ],
            basic: None,
            body: CodeBody::None,
            follow_redirects: false,
        };
        let json = CodeRequest {
            method: HttpMethod::Post,
            url: "https://api.exemple.fr/customers".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            basic: None,
            body: CodeBody::Text("{\n  \"name\": \"O'Brien\",\n  \"plan\": \"pro\"\n}".to_string()),
            follow_redirects: true,
        };
        let form = CodeRequest {
            method: HttpMethod::Post,
            url: "https://auth.exemple.fr/token".to_string(),
            headers: vec![(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            )],
            basic: Some(("billing-web".to_string(), "{{clientSecret}}".to_string())),
            body: CodeBody::Form(vec![
                ("grant_type".to_string(), "client_credentials".to_string()),
                ("scope".to_string(), "invoices read".to_string()),
            ]),
            follow_redirects: false,
        };
        let multipart = CodeRequest {
            method: HttpMethod::Put,
            url: "https://api.exemple.fr/documents/7".to_string(),
            headers: Vec::new(),
            basic: None,
            body: CodeBody::Multipart(vec![
                CodePart {
                    name: "title".to_string(),
                    value: "Facture".to_string(),
                    file: false,
                },
                CodePart {
                    name: "scan".to_string(),
                    value: "C:/Users/ada/facture.pdf".to_string(),
                    file: true,
                },
            ]),
            follow_redirects: false,
        };
        let file = CodeRequest {
            method: HttpMethod::Post,
            url: "https://api.exemple.fr/upload".to_string(),
            headers: vec![(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )],
            basic: None,
            body: CodeBody::File("/tmp/archive.zip".to_string()),
            follow_redirects: false,
        };
        vec![
            ("get", get),
            ("json", json),
            ("form", form),
            ("multipart", multipart),
            ("file", file),
        ]
    }

    /// The file under `golden/`, or written there when `GOLDEN_WRITE` is set.
    pub(crate) fn check(target: &str, case: &str, written: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/http/codegen/golden")
            .join(format!("{target}-{case}.txt"));
        if std::env::var_os("GOLDEN_WRITE").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, written).unwrap();
            return;
        }
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("no golden file {}", path.display()))
            .replace("\r\n", "\n");
        assert_eq!(written, expected, "{target}-{case}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::environments::Variable;
    use crate::http::model::KeyValue;
    use crate::http::settings::inherit;
    use crate::http::variables::VariableScope;

    fn layers() -> Vec<VariableLayer> {
        let variable = |name: &str, value: &str, kind: VariableKind| Variable {
            name: name.to_string(),
            value: value.to_string(),
            kind,
            ..Variable::default()
        };
        vec![VariableLayer {
            scope: VariableScope::Environment,
            from: "Staging".to_string(),
            variables: vec![
                variable("baseUrl", "https://api.exemple.fr", VariableKind::Text),
                variable("token", "t0k3n", VariableKind::Secret),
            ],
        }]
    }

    fn document() -> RequestDocument {
        RequestDocument {
            url: "{{baseUrl}}/me?tenant={{tenant}}".to_string(),
            headers: vec![KeyValue {
                key: "Authorization".to_string(),
                value: "Bearer {{token}}".to_string(),
                ..KeyValue::default()
            }],
            ..RequestDocument::default()
        }
    }

    fn options(resolve: bool, secrets: bool) -> CodeOptions {
        CodeOptions {
            resolve_variables: resolve,
            include_secrets: secrets,
            multiline: false,
            follow_redirects: false,
        }
    }

    fn generated(options: CodeOptions) -> GeneratedCode {
        generate(
            CodeTarget::Curl,
            options,
            RequestKind::Http,
            HttpMethod::Get,
            &document(),
            &inherit(&[]),
            &layers(),
        )
    }

    #[test]
    fn every_target_survives_its_stored_spelling() {
        for target in CodeTarget::ALL {
            assert_eq!(target.as_str().parse::<CodeTarget>(), Ok(target));
        }
    }

    #[test]
    fn a_secret_stays_its_name_unless_asked_for_and_what_nothing_sets_is_said() {
        let kept = generated(options(true, false));
        assert!(
            kept.code
                .contains("https://api.exemple.fr/me?tenant={{tenant}}")
        );
        assert!(kept.code.contains("Bearer {{token}}"));
        assert_eq!(kept.secrets, ["token"]);
        assert_eq!(kept.unresolved, ["tenant"]);
        assert_eq!(kept.language, Language::Sh);

        let clear = generated(options(true, true));
        assert!(clear.code.contains("Bearer t0k3n"));
        assert!(clear.secrets.is_empty());

        let typed = generated(options(false, false));
        assert!(typed.code.contains("{{baseUrl}}/me"));
    }

    #[test]
    fn the_application_names_itself_and_a_basic_auth_is_written_apart() {
        let document = RequestDocument {
            url: "api.exemple.fr/x".to_string(),
            auth: RequestAuth::Basic {
                username: "ada".to_string(),
                password: "pw".to_string(),
            },
            ..RequestDocument::default()
        };
        let request = request_of(
            RequestKind::Http,
            HttpMethod::Get,
            &document,
            &inherit(&[]),
            false,
        );
        assert_eq!(request.url, "http://api.exemple.fr/x");
        assert_eq!(request.basic, Some(("ada".to_string(), "pw".to_string())));
        assert!(request.headers.is_empty(), "{:?}", request.headers);
    }

    #[test]
    fn a_key_in_the_query_and_a_graphql_get_go_in_the_url() {
        let keyed = RequestDocument {
            url: "https://x/a?b=1".to_string(),
            auth: RequestAuth::ApiKey {
                name: "api key".to_string(),
                value: "k&1".to_string(),
                place: KeyPlace::Query,
            },
            ..RequestDocument::default()
        };
        let request = request_of(
            RequestKind::Http,
            HttpMethod::Get,
            &keyed,
            &inherit(&[]),
            false,
        );
        assert_eq!(request.url, "https://x/a?b=1&api+key=k%261");

        let mut graphql = RequestDocument {
            url: "https://x/graphql".to_string(),
            ..RequestDocument::default()
        };
        graphql.graphql.query = "{ me { id } }".to_string();
        graphql.graphql.as_get = true;
        let request = request_of(
            RequestKind::Graphql,
            HttpMethod::Post,
            &graphql,
            &inherit(&[]),
            false,
        );
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.url, "https://x/graphql?query=%7B+me+%7B+id+%7D+%7D");
    }

    #[test]
    fn wget_says_it_sends_no_multipart_form() {
        let document = RequestDocument {
            url: "https://x".to_string(),
            body: RequestBody::Multipart {
                parts: vec![crate::http::model::FormPart {
                    key: "a".to_string(),
                    value: "1".to_string(),
                    ..Default::default()
                }],
            },
            ..RequestDocument::default()
        };
        let code = generate(
            CodeTarget::Wget,
            options(true, false),
            RequestKind::Http,
            HttpMethod::Post,
            &document,
            &inherit(&[]),
            &[],
        );
        assert_eq!(code.notice, Some(CodeNotice::MultipartUnsupported));
    }

    #[test]
    fn a_shell_quote_survives_a_quote_and_lines_end_on_a_backslash() {
        assert_eq!(shell_quoted("O'Brien"), r"'O'\''Brien'");
        assert_eq!(
            shell_lines("curl", &["-a".to_string(), "-b".to_string()], true),
            "curl \\\n  -a \\\n  -b"
        );
        assert_eq!(
            shell_lines("curl", &["-a".to_string(), "-b".to_string()], false),
            "curl -a -b"
        );
    }
}
