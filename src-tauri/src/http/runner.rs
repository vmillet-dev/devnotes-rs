//! A collection or a folder run: its requests one after the other, over iterations or the rows
//! of a data file, each sent as a send is — scripts, tests, extractions — but written in no
//! history. What a run is told and what it says back; the loop is the command's.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::{
    HttpItem, HttpItemKind, HttpMethod, HttpNode, HttpRequest, HttpTree, RequestKind, RunEntry,
};
use super::scripts::model::{ScriptStop, TestResult};
use super::send::SentResponse;
use crate::error::{AppError, ErrorCode, FileContext};

pub const HTTP_RUN_EVENT: &str = "http-run";

/// The rows of a data file, each an iteration; its columns are `pm.iterationData`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunData {
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DataFault {
    #[error("The file reads as neither a CSV with a header nor a JSON array of objects")]
    Unreadable,
    #[error("The file holds no row")]
    Empty,
}

/// The file at `path`, read where it lies.
pub fn read_file(path: &str) -> Result<RunData, AppError> {
    let text = std::fs::read_to_string(path).context(path)?;
    read_data(&text).map_err(AppError::from)
}

/// A CSV with its header, or a JSON array of objects: what the file's text reads as.
pub fn read_data(text: &str) -> Result<RunData, DataFault> {
    let text = text.trim_start_matches('\u{feff}');
    let data = if text.trim_start().starts_with('[') {
        from_json(text)?
    } else {
        from_csv(text)?
    };
    if data.rows.is_empty() {
        return Err(DataFault::Empty);
    }
    Ok(data)
}

fn from_json(text: &str) -> Result<RunData, DataFault> {
    let rows: Vec<serde_json::Map<String, serde_json::Value>> =
        serde_json::from_str(text).map_err(|_| DataFault::Unreadable)?;
    let mut columns: Vec<String> = Vec::new();
    let rows = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|(key, value)| {
                    if !columns.contains(&key) {
                        columns.push(key.clone());
                    }
                    let value = match value {
                        serde_json::Value::String(text) => text,
                        other => other.to_string(),
                    };
                    (key, value)
                })
                .collect()
        })
        .collect();
    Ok(RunData { columns, rows })
}

/// RFC 4180: a field between quotes holds commas, line breaks and doubled quotes.
fn records(text: &str) -> Result<Vec<Vec<String>>, DataFault> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => record.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            (_, c) => field.push(c),
        }
    }
    if quoted {
        return Err(DataFault::Unreadable);
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records.retain(|record| record.iter().any(|field| !field.trim().is_empty()));
    Ok(records)
}

fn from_csv(text: &str) -> Result<RunData, DataFault> {
    let mut records = records(text)?.into_iter();
    let columns: Vec<String> = records
        .next()
        .ok_or(DataFault::Empty)?
        .into_iter()
        .map(|column| column.trim().to_string())
        .collect();
    if columns.iter().all(String::is_empty) {
        return Err(DataFault::Unreadable);
    }
    let rows = records
        .map(|record| {
            columns
                .iter()
                .zip(record.into_iter().chain(std::iter::repeat(String::new())))
                .filter(|(column, _)| !column.is_empty())
                .map(|(column, value)| (column.clone(), value))
                .collect()
        })
        .collect();
    Ok(RunData { columns, rows })
}

/// What a run is asked to do: its switches are the runner's checkboxes, each its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// Chosen by the front: what its events and « Annuler » name it by.
    pub id: String,
    /// The collection or the folder run.
    pub item: HttpItem,
    /// Its requests in the order they run, those left out absent.
    pub order: Vec<String>,
    pub iterations: u32,
    pub delay_ms: u32,
    /// An iteration a row, over `iterations`.
    pub data: Option<RunData>,
    pub stop_on_failure: bool,
    /// What the scripts and the extractions write is kept; otherwise for the run alone.
    pub keep_variables: bool,
    /// Each answer kept in the report, for the session.
    pub keep_responses: bool,
    pub environment_id: Option<String>,
    /// The one yes a protected environment asked for before the run.
    pub confirmed: bool,
}

/// An error as its code and the values its sentence states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunError {
    pub code: ErrorCode,
    pub params: BTreeMap<String, String>,
}

/// One request of one iteration, once it ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub iteration: u32,
    pub request_id: String,
    pub name: String,
    /// The folders it sits in, under the item run.
    pub path: Vec<String>,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub status: Option<u16>,
    pub reason: String,
    pub millis: u32,
    pub tests: Vec<TestResult>,
    /// What kept it from an answer.
    pub error: Option<RunError>,
    pub stop: Option<ScriptStop>,
    /// `pm.execution.skipRequest()`: not sent, not failed.
    pub skipped: bool,
    pub response: Option<SentResponse>,
}

impl RunResult {
    /// No answer, a script that stopped, or a test that failed.
    #[must_use]
    pub fn failed(&self) -> bool {
        !self.skipped
            && (self.error.is_some()
                || self.stop.is_some()
                || self.tests.iter().any(|test| !test.passed && !test.skipped))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RunStep {
    /// A request is about to leave: the progress line says which.
    Started {
        iteration: u32,
        index: u32,
        name: String,
    },
    Ended {
        result: Box<RunResult>,
    },
    /// The run is over: ran to its end, stopped at a failure or by a script, or cancelled.
    Finished {
        cancelled: bool,
        millis: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    pub id: String,
    pub step: RunStep,
}

/// A request under the item run, in the tree's order: its id, its name, and the folders between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub id: String,
    pub name: String,
    pub path: Vec<String>,
}

/// Each request under `item`, in the tree's order.
#[must_use]
pub fn placed_under(tree: &HttpTree, item: &HttpItem) -> Vec<Placed> {
    fn walk(nodes: &[HttpNode], path: &[String], out: &mut Vec<Placed>) {
        for node in nodes {
            match node {
                HttpNode::Request { request } => out.push(Placed {
                    id: request.id.clone(),
                    name: request.name.clone(),
                    path: path.to_vec(),
                }),
                HttpNode::Folder { folder, children } => {
                    let mut deeper = path.to_vec();
                    deeper.push(folder.name.clone());
                    walk(children, &deeper, out);
                }
            }
        }
    }
    fn find<'a>(nodes: &'a [HttpNode], id: &str) -> Option<&'a [HttpNode]> {
        nodes.iter().find_map(|node| match node {
            HttpNode::Folder { folder, children } if folder.id == id => Some(children.as_slice()),
            HttpNode::Folder { children, .. } => find(children, id),
            HttpNode::Request { .. } => None,
        })
    }
    let mut out = Vec::new();
    for collection in &tree.collections {
        let children = match item.kind {
            HttpItemKind::Collection if collection.collection.id == item.id => {
                Some(collection.children.as_slice())
            }
            HttpItemKind::Folder => find(&collection.children, &item.id),
            _ => None,
        };
        if let Some(children) = children {
            walk(children, &[], &mut out);
        }
    }
    out
}

/// The order a run takes: the one last arranged, less the requests gone since, then those added
/// since in the tree's order, each run.
#[must_use]
pub fn run_order(saved: &[RunEntry], placed: &[Placed]) -> Vec<RunEntry> {
    let here = |id: &str| placed.iter().any(|request| request.id == id);
    let mut order: Vec<RunEntry> = Vec::with_capacity(placed.len());
    for entry in saved {
        if here(&entry.id) && !order.iter().any(|kept| kept.id == entry.id) {
            order.push(entry.clone());
        }
    }
    for request in placed {
        if !order.iter().any(|kept| kept.id == request.id) {
            order.push(RunEntry {
                id: request.id.clone(),
                enabled: true,
            });
        }
    }
    order
}

/// What a request of a run ended as.
#[must_use]
pub fn result_of(
    iteration: u32,
    request: &HttpRequest,
    path: Vec<String>,
    answer: Result<SentResponse, AppError>,
    keep_response: bool,
) -> RunResult {
    let mut result = RunResult {
        iteration,
        request_id: request.id.clone(),
        name: request.name.clone(),
        path,
        kind: request.kind,
        method: request.method,
        status: None,
        reason: String::new(),
        millis: 0,
        tests: Vec::new(),
        error: None,
        stop: None,
        skipped: false,
        response: None,
    };
    match answer {
        Ok(response) => {
            result.status = Some(response.status);
            result.reason.clone_from(&response.reason);
            result.millis = response.millis;
            result.tests.clone_from(&response.scripts.tests);
            result.stop.clone_from(&response.scripts.failure);
            result.response = keep_response.then_some(response);
        }
        Err(error) if error.code == ErrorCode::HttpScriptSkipped => result.skipped = true,
        Err(error) => {
            result.error = Some(RunError {
                code: error.code,
                params: error.params,
            });
        }
    }
    result
}

/// A request of the order that is no longer in the library when its turn comes.
#[must_use]
pub fn gone(iteration: u32, placed: &Placed, error: AppError) -> RunResult {
    RunResult {
        iteration,
        request_id: placed.id.clone(),
        name: placed.name.clone(),
        path: placed.path.clone(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        status: None,
        reason: String::new(),
        millis: 0,
        tests: Vec::new(),
        error: Some(RunError {
            code: error.code,
            params: error.params,
        }),
        stop: None,
        skipped: false,
        response: None,
    }
}

/// The runs under way, by id: whether « Annuler » was asked, and the send each is at.
#[derive(Default)]
pub struct Runs {
    running: Mutex<HashMap<String, (bool, Option<String>)>>,
}

impl Runs {
    pub fn start(&self, id: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.insert(id.to_string(), (false, None));
        }
    }

    /// The send it is at, which « Annuler » cancels too.
    pub fn at(&self, id: &str, send: &str) {
        if let Ok(mut running) = self.running.lock()
            && let Some((_, at)) = running.get_mut(id)
        {
            *at = Some(send.to_string());
        }
    }

    #[must_use]
    pub fn cancelled(&self, id: &str) -> bool {
        self.running
            .lock()
            .ok()
            .and_then(|running| running.get(id).map(|(cancelled, _)| *cancelled))
            .unwrap_or(true)
    }

    /// The send to cancel with it; `None` when no run of that id is under way.
    pub fn cancel(&self, id: &str) -> Option<Option<String>> {
        let mut running = self.running.lock().ok()?;
        let (cancelled, at) = running.get_mut(id)?;
        *cancelled = true;
        Some(at.clone())
    }

    pub fn finish(&self, id: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.remove(id);
        }
    }
}

/// `pm.execution.setNextRequest`: the request named, or `null` to end the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    To(String),
    End,
}

/// Where a run goes after a request: on, to the request a script named, or to its end.
#[must_use]
pub fn next_index(order: &[Placed], at: usize, next: Option<&Next>) -> Option<usize> {
    match next {
        None => (at + 1 < order.len()).then_some(at + 1),
        Some(Next::End) => None,
        Some(Next::To(name)) => order.iter().position(|placed| &placed.name == name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn a_csv_reads_with_its_header_quotes_and_blank_lines() {
        let data = read_data(
            "\u{feff}customer,amount,note\r\ncus_QX12a9,12,\"a, b\"\r\n\r\ncus_R7a01x,8,\"say \"\"hi\"\"\nthen\"\n",
        )
        .unwrap();
        assert_eq!(data.columns, ["customer", "amount", "note"]);
        assert_eq!(
            data.rows,
            [
                row(&[
                    ("customer", "cus_QX12a9"),
                    ("amount", "12"),
                    ("note", "a, b")
                ]),
                row(&[
                    ("customer", "cus_R7a01x"),
                    ("amount", "8"),
                    ("note", "say \"hi\"\nthen")
                ]),
            ]
        );
        let short = read_data("a,b\n1").unwrap();
        assert_eq!(short.rows, [row(&[("a", "1"), ("b", "")])]);
    }

    #[test]
    fn a_json_array_of_objects_reads_a_row_an_object() {
        let data =
            read_data(r#"[{"customer":"cus_1","amount":12},{"customer":"cus_2","vip":true}]"#)
                .unwrap();
        assert_eq!(data.columns, ["customer", "amount", "vip"]);
        assert_eq!(
            data.rows[0],
            row(&[("customer", "cus_1"), ("amount", "12")])
        );
        assert_eq!(data.rows[1], row(&[("customer", "cus_2"), ("vip", "true")]));
    }

    #[test]
    fn a_file_that_is_neither_or_holds_nothing_is_said() {
        assert_eq!(read_data("[1, 2]"), Err(DataFault::Unreadable));
        assert_eq!(read_data("a,b\n\"open"), Err(DataFault::Unreadable));
        assert_eq!(read_data("a,b\n"), Err(DataFault::Empty));
        assert_eq!(read_data("[]"), Err(DataFault::Empty));
        assert_eq!(read_data(""), Err(DataFault::Empty));
    }

    #[test]
    fn the_order_kept_drops_what_is_gone_and_ends_on_what_was_added() {
        let placed: Vec<Placed> = ["a", "b", "c", "d"]
            .iter()
            .map(|id| Placed {
                id: (*id).to_string(),
                name: (*id).to_uppercase(),
                path: Vec::new(),
            })
            .collect();
        let entry = |id: &str, enabled: bool| RunEntry {
            id: id.to_string(),
            enabled,
        };

        assert_eq!(
            run_order(&[], &placed),
            [
                entry("a", true),
                entry("b", true),
                entry("c", true),
                entry("d", true)
            ],
            "nothing arranged: the tree's order"
        );
        assert_eq!(
            run_order(
                &[
                    entry("c", true),
                    entry("gone", true),
                    entry("a", false),
                    entry("c", false)
                ],
                &placed
            ),
            [
                entry("c", true),
                entry("a", false),
                entry("b", true),
                entry("d", true)
            ]
        );
    }

    #[test]
    fn a_run_goes_on_follows_the_request_a_script_named_or_stops() {
        let order: Vec<Placed> = [("a", "Login"), ("b", "Payer"), ("c", "Reçu")]
            .iter()
            .map(|(id, name)| Placed {
                id: (*id).to_string(),
                name: (*name).to_string(),
                path: Vec::new(),
            })
            .collect();
        assert_eq!(next_index(&order, 0, None), Some(1));
        assert_eq!(next_index(&order, 2, None), None);
        let to = |name: &str| Next::To(name.to_string());
        assert_eq!(next_index(&order, 0, Some(&to("Reçu"))), Some(2));
        assert_eq!(next_index(&order, 2, Some(&to("Login"))), Some(0));
        assert_eq!(next_index(&order, 0, Some(&to("Ailleurs"))), None);
        assert_eq!(next_index(&order, 0, Some(&Next::End)), None);
    }

    #[test]
    fn a_result_fails_on_an_error_a_stop_or_a_failed_test_never_when_skipped() {
        let base = RunResult {
            iteration: 0,
            request_id: "a".to_string(),
            name: "Login".to_string(),
            path: Vec::new(),
            kind: RequestKind::Http,
            method: HttpMethod::Get,
            status: Some(200),
            reason: "OK".to_string(),
            millis: 1,
            tests: vec![TestResult {
                passed: true,
                ..TestResult::default()
            }],
            error: None,
            stop: None,
            skipped: false,
            response: None,
        };
        assert!(!base.failed());
        let failing_test = RunResult {
            tests: vec![TestResult::default()],
            ..base.clone()
        };
        assert!(failing_test.failed());
        assert!(
            RunResult {
                error: Some(RunError {
                    code: ErrorCode::HttpRefused,
                    params: BTreeMap::new(),
                }),
                ..base.clone()
            }
            .failed()
        );
        assert!(
            !RunResult {
                skipped: true,
                ..failing_test
            }
            .failed()
        );
    }
}
