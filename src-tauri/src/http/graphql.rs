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
    /// A brace, a parenthesis or a string the query leaves open: said where it is typed. It
    /// warns and stops nothing — the server stays the judge of the rest.
    pub query_problem: Option<QueryProblem>,
    /// What the operation sent declares without a default and the variables give no value.
    pub missing: Vec<String>,
    /// The keys of the variables the operation sent does not declare.
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QueryFault {
    UnclosedBrace,
    UnclosedParenthesis,
    UnclosedString,
    /// Closes nothing, or what another sign opened.
    StrayBrace,
    StrayParenthesis,
}

/// Where a query stops reading: the sign at fault, by its line and column from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueryProblem {
    pub fault: QueryFault,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Operation {
    /// `None` for `query { … }` and for the shorthand `{ … }`.
    name: Option<String>,
    variables: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Variable {
    name: String,
    /// `$first: Int = 10`: it has a value without being given one.
    has_default: bool,
}

#[derive(Debug, Default)]
struct Scanned {
    operations: Vec<Operation>,
    problem: Option<QueryProblem>,
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

/// The characters of a query, each with where it stands.
struct Reader<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: u32,
    column: u32,
}

impl Reader<'_> {
    /// The next character and its place, from 1.
    fn next(&mut self) -> Option<(char, u32, u32)> {
        let read = self.chars.next()?;
        let at = (read, self.line, self.column);
        if read == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(at)
    }

    fn next_if(&mut self, wanted: impl FnOnce(char) -> bool) -> Option<char> {
        let found = self.chars.peek().copied().filter(|next| wanted(*next))?;
        self.next();
        Some(found)
    }

    fn word(&mut self) -> String {
        std::iter::from_fn(|| self.next_if(|next| next.is_ascii_alphanumeric() || next == '_'))
            .collect()
    }

    fn skip_spaces(&mut self) {
        while self.next_if(char::is_whitespace).is_some() {}
    }

    /// Past a string whose opening quote was just read; `false` when it does not close. A
    /// `"""block"""` runs over lines, a plain one ends with its own.
    fn skip_string(&mut self) -> bool {
        if self.next_if(|next| next == '"').is_some() {
            if self.next_if(|next| next == '"').is_none() {
                return true;
            }
            let mut quotes = 0;
            while let Some((read, ..)) = self.next() {
                match read {
                    '\\' => {
                        self.next();
                        quotes = 0;
                    }
                    '"' => quotes += 1,
                    _ => quotes = 0,
                }
                if quotes == 3 {
                    return true;
                }
            }
            return false;
        }
        while let Some(read) = self.next_if(|next| next != '\n') {
            match read {
                '\\' => {
                    self.next_if(|next| next != '\n');
                }
                '"' => return true,
                _ => {}
            }
        }
        false
    }
}

/// One pass over a query, past its strings and comments: its operations with what each
/// declares, and the first brace, parenthesis or string that does not pair.
fn scan(query: &str) -> Scanned {
    let mut reader = Reader {
        chars: query.chars().peekable(),
        line: 1,
        column: 1,
    };
    let mut scanned = Scanned::default();
    // What is open, and where it was opened.
    let mut open: Vec<(char, u32, u32)> = Vec::new();
    // A keyword was read and its selection is not: the next `{` is not the shorthand's.
    let mut headed = false;
    // The depth at which the operation being read declares its variables.
    let mut declaring: Option<usize> = None;
    let fault = |scanned: &mut Scanned, fault, line, column| {
        scanned.problem.get_or_insert(QueryProblem {
            fault,
            line,
            column,
        });
    };

    while let Some((read, line, column)) = reader.next() {
        match read {
            '#' => while reader.next_if(|next| next != '\n').is_some() {},
            '"' => {
                if !reader.skip_string() {
                    fault(&mut scanned, QueryFault::UnclosedString, line, column);
                }
            }
            '{' | '(' => {
                if read == '{' && open.is_empty() {
                    if !headed {
                        scanned.operations.push(Operation::default());
                    }
                    headed = false;
                }
                if read == '(' && open.is_empty() && headed {
                    declaring = Some(1);
                }
                open.push((read, line, column));
            }
            '}' | ')' => {
                let pair = if read == '}' { '{' } else { '(' };
                if open.last().is_some_and(|(opened, ..)| *opened == pair) {
                    open.pop();
                } else {
                    let stray = if read == '}' {
                        QueryFault::StrayBrace
                    } else {
                        QueryFault::StrayParenthesis
                    };
                    fault(&mut scanned, stray, line, column);
                }
                if declaring.is_some_and(|depth| open.len() < depth) {
                    declaring = None;
                }
            }
            '$' if declaring == Some(open.len()) => {
                let name = reader.word();
                if let (Some(operation), false) = (scanned.operations.last_mut(), name.is_empty()) {
                    operation.variables.push(Variable {
                        name,
                        has_default: false,
                    });
                }
            }
            '=' if declaring == Some(open.len()) => {
                if let Some(variable) = scanned
                    .operations
                    .last_mut()
                    .and_then(|operation| operation.variables.last_mut())
                {
                    variable.has_default = true;
                }
            }
            read if open.is_empty() && read.is_ascii_alphabetic() => {
                let keyword = format!("{read}{}", reader.word());
                match keyword.as_str() {
                    "query" | "mutation" | "subscription" => {
                        reader.skip_spaces();
                        let name = reader.word();
                        scanned.operations.push(Operation {
                            name: (!name.is_empty()).then_some(name),
                            variables: Vec::new(),
                        });
                        headed = true;
                    }
                    "fragment" => headed = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // Left open at the end: the innermost, the one nearest to where the typing stopped.
    if let Some((opened, line, column)) = open.last().copied() {
        let unclosed = if opened == '{' {
            QueryFault::UnclosedBrace
        } else {
            QueryFault::UnclosedParenthesis
        };
        fault(&mut scanned, unclosed, line, column);
    }
    scanned
}

/// The names of the document's operations, read past strings, comments and selections.
pub fn operations(query: &str) -> Vec<String> {
    scan(query)
        .operations
        .into_iter()
        .filter_map(|operation| operation.name)
        .collect()
}

/// The operation a send runs: the one named, or the first as the server takes it.
fn sent<'a>(operations: &'a [Operation], document: &GraphqlDocument) -> Option<&'a Operation> {
    document
        .operation_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .and_then(|name| {
            operations
                .iter()
                .find(|operation| operation.name.as_deref() == Some(name))
        })
        .or_else(|| operations.first())
}

/// The keys the variables give a value: `None` when they are not a JSON object as typed, which
/// is said for what it is and not as a list of missing values.
fn given(variables: &str) -> Option<Vec<String>> {
    if variables.trim().is_empty() {
        return Some(Vec::new());
    }
    serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(variables)
        .ok()
        .map(|object| object.keys().cloned().collect())
}

pub fn describe(document: &GraphqlDocument) -> GraphqlAnswer {
    let scanned = scan(&document.query);
    let (mut missing, mut unknown) = (Vec::new(), Vec::new());
    if let (Some(operation), Some(given)) = (
        sent(&scanned.operations, document),
        given(&document.variables),
    ) {
        missing = operation
            .variables
            .iter()
            .filter(|variable| !variable.has_default && !given.contains(&variable.name))
            .map(|variable| variable.name.clone())
            .collect();
        unknown = given
            .into_iter()
            .filter(|key| {
                !operation
                    .variables
                    .iter()
                    .any(|variable| &variable.name == key)
            })
            .collect();
    }
    GraphqlAnswer {
        operations: scanned
            .operations
            .into_iter()
            .filter_map(|operation| operation.name)
            .collect(),
        variables_problem: if document.variables.trim().is_empty() {
            None
        } else {
            crate::json::parse::parse(&document.variables).err()
        },
        query_problem: scanned.problem,
        missing,
        unknown,
    }
}

/// The variables with a `null` for each one the operation declares and they lack, written at
/// the end of the object as typed: nothing else of it is touched, its numbers least of all.
pub fn with_missing(document: &GraphqlDocument) -> String {
    let missing = describe(document).missing;
    if missing.is_empty() {
        return document.variables.clone();
    }
    let added = missing
        .iter()
        .map(|name| format!("  {}: null", serde_json::Value::from(name.as_str())))
        .collect::<Vec<_>>()
        .join(",\n");
    let written = document
        .variables
        .trim()
        .strip_suffix('}')
        .map(str::trim_end)
        .filter(|kept| kept.trim() != "{");
    match written {
        Some(kept) => format!("{kept},\n{added}\n}}"),
        None => format!("{{\n{added}\n}}"),
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
        assert_eq!(scan(query).problem, None);
        // A block string runs over lines, quotes and braces included.
        let described = r#"
            """
            Says "hello" { and more
            """
            query Greeting { hello(note: "a \" b") }
        "#;
        assert_eq!(operations(described), ["Greeting"]);
        assert_eq!(scan(described).problem, None);
    }

    fn document(query: &str, variables: &str) -> GraphqlDocument {
        GraphqlDocument {
            query: query.to_string(),
            variables: variables.to_string(),
            ..GraphqlDocument::default()
        }
    }

    /// A query missing a brace was sent without a word, and answered by a 400.
    #[test]
    fn a_brace_a_parenthesis_or_a_string_left_open_is_said_with_its_place() {
        let problem = |query: &str| {
            scan(query)
                .problem
                .map(|found| (found.fault, found.line, found.column))
        };

        assert_eq!(
            problem("query {\n  viewer {\n    login\n  }\n"),
            Some((QueryFault::UnclosedBrace, 1, 7))
        );
        // The innermost one, nearest to where the typing stopped.
        assert_eq!(
            problem("{ viewer {\n  repos(first: 3 {"),
            Some((QueryFault::UnclosedBrace, 2, 18))
        );
        assert_eq!(
            problem("{ user(id: 1 }"),
            Some((QueryFault::StrayBrace, 1, 14))
        );
        assert_eq!(
            problem("{ user(id: 1) ) }"),
            Some((QueryFault::StrayParenthesis, 1, 15))
        );
        assert_eq!(problem("{ user }\n}"), Some((QueryFault::StrayBrace, 2, 1)));
        assert_eq!(
            problem("{ search(text: \"open) { id }\n}"),
            Some((QueryFault::UnclosedString, 1, 16))
        );
        assert_eq!(
            problem("{ a }\n\"\"\"never closed"),
            Some((QueryFault::UnclosedString, 2, 1))
        );
        assert_eq!(
            problem("{ user(id: 1"),
            Some((QueryFault::UnclosedParenthesis, 1, 7))
        );
        // A sign in a comment or in a string pairs with nothing.
        assert_eq!(problem("{ a(note: \"}\") } # }"), None);
        assert_eq!(problem(""), None);
    }

    /// The variables were typed after the query, in its box: nothing said what goes where.
    #[test]
    fn the_variables_an_operation_declares_are_compared_with_those_given() {
        let query = r"
            query Country($code: ID!, $first: Int = 10, $after: String) {
              country(code: $code) { cities(first: $first, after: $after) { name } }
            }
            mutation Rename($id: ID!, $name: String!) { rename(id: $id, name: $name) { id } }
        ";
        let said = |variables: &str, operation: Option<&str>| {
            let answer = describe(&GraphqlDocument {
                operation_name: operation.map(str::to_string),
                ..document(query, variables)
            });
            (answer.missing, answer.unknown)
        };
        let names = |names: &[&str]| names.iter().map(ToString::to_string).collect::<Vec<_>>();

        // Without a default and without a value; the first operation when none is named.
        assert_eq!(said("", None), (names(&["code", "after"]), names(&[])));
        assert_eq!(
            said(r#"{ "code": "FR", "page": 2 }"#, None),
            (names(&["after"]), names(&["page"]))
        );
        assert_eq!(
            said(r#"{"id": 1}"#, Some("Rename")),
            (names(&["name"]), names(&[]))
        );
        assert_eq!(
            said(r#"{"id": 1}"#, Some("Gone")),
            (names(&["code", "after"]), names(&["id"]))
        );
        // Not an object as typed: said as that, not as a list of what is missing.
        assert_eq!(said("{\"code\":", None), (names(&[]), names(&[])));
        assert_eq!(said("[1]", None), (names(&[]), names(&[])));

        // The shorthand and a fragment declare nothing; no operation, nothing is compared.
        let shorthand = describe(&document(
            "fragment F on User { id } { me { ...F } }",
            r#"{"a": 1}"#,
        ));
        assert_eq!(
            (shorthand.missing, shorthand.unknown),
            (names(&[]), names(&["a"]))
        );
        let empty = describe(&document("", r#"{"a": 1}"#));
        assert_eq!((empty.missing, empty.unknown), (names(&[]), names(&[])));
        // A `$variable` used in a selection is not one declared.
        let used = describe(&document("query ($a: Int) { x(a: $a, b: $b) }", ""));
        assert_eq!(used.missing, names(&["a"]));
    }

    #[test]
    fn what_is_missing_is_added_as_null_at_the_end_of_the_object_as_typed() {
        let query = "query Country($code: ID!, $after: String) { country(code: $code) { name } }";
        let added = |variables: &str| with_missing(&document(query, variables));

        assert_eq!(added(""), "{\n  \"code\": null,\n  \"after\": null\n}");
        assert_eq!(added("{ }"), "{\n  \"code\": null,\n  \"after\": null\n}");
        assert_eq!(
            added("{\n  \"n\": 12345678901234567890,\n  \"code\": \"FR\"\n}\n"),
            "{\n  \"n\": 12345678901234567890,\n  \"code\": \"FR\",\n  \"after\": null\n}"
        );
        // Nothing missing, or nothing that reads: left as it is.
        let whole = r#"{"code": "FR", "after": null}"#;
        assert_eq!(added(whole), whole);
        assert_eq!(added("{\"code\":"), "{\"code\":");
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
