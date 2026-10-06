//! Two JSON documents compared as values, not as the characters that wrote them: the changes by
//! their `JSONPath`, the two documents laid side by side on aligned rows, and the RFC 6902 patch
//! that turns the first into the second.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use similar::{Algorithm, DiffOp, capture_diff_slices};
use specta::Type;

use crate::count::saturating_u32;

/// Past this, the rows are cut: the changes and the patch stay whole.
pub(crate) const MAX_ROWS: usize = 20_000;
/// A value in the list of changes is cut to this many characters.
const SUMMARY_CHARS: usize = 40;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonDiffRequest {
    pub a: String,
    pub b: String,
    /// On, `{"a":1,"b":2}` and `{"b":2,"a":1}` are the same document.
    pub ignore_key_order: bool,
    /// On, two strings that differ only by their spaces are the same.
    pub ignore_whitespace: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonDiffSide {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonChangeKind {
    Added,
    Removed,
    Modified,
    /// The same keys, in another order: counted with the modifications.
    Reordered,
}

/// What a value was, as the list of changes shows it: itself when short, its size otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ValueSummary {
    Scalar { text: String },
    Object { keys: u32 },
    Array { items: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonChange {
    pub kind: JsonChangeKind,
    pub path: String,
    pub before: Option<ValueSummary>,
    pub after: Option<ValueSummary>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonDiffCounts {
    pub added: u32,
    pub removed: u32,
    pub modified: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonDiffLine {
    /// One-based, in that side's own text.
    pub number: u32,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonDiffRowKind {
    Same,
    Added,
    Removed,
    Modified,
}

/// One row of the side-by-side view: a side without a line is a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonDiffRow {
    pub kind: JsonDiffRowKind,
    pub left: Option<JsonDiffLine>,
    pub right: Option<JsonDiffLine>,
    /// The `JSONPath` of the value the row writes, for a change to scroll to.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonDiffAnswer {
    Compared {
        changes: Vec<JsonChange>,
        counts: JsonDiffCounts,
        rows: Vec<JsonDiffRow>,
        rows_truncated: bool,
        /// RFC 6902, from A to B, as JSON.
        patch: String,
    },
    Unreadable {
        side: JsonDiffSide,
        line: u32,
        column: u32,
    },
}

// --- Paths.

fn child(path: &str, key: &str) -> String {
    let mut chars = key.chars();
    let plain = chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_');
    if plain {
        format!("{path}.{key}")
    } else {
        format!("{path}[{}]", serde_json::to_string(key).unwrap_or_default())
    }
}

/// RFC 6901: `~` and `/` escaped, in that order.
fn pointer(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

// --- Comparing.

struct Differ {
    ignore_key_order: bool,
    ignore_whitespace: bool,
    changes: Vec<JsonChange>,
    patch: Vec<Value>,
}

fn summary(value: &Value) -> ValueSummary {
    match value {
        Value::Object(map) => ValueSummary::Object {
            keys: saturating_u32(map.len()),
        },
        Value::Array(items) => ValueSummary::Array {
            items: saturating_u32(items.len()),
        },
        _ => {
            let text = value.to_string();
            let cut: String = text.chars().take(SUMMARY_CHARS).collect();
            ValueSummary::Scalar {
                text: if cut.len() < text.len() {
                    format!("{cut}…")
                } else {
                    cut
                },
            }
        }
    }
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Differ {
    /// What two values are compared by: key order and spaces taken out when asked.
    fn canonical(&self, value: &Value) -> String {
        match value {
            Value::String(text) if self.ignore_whitespace => {
                Value::String(collapsed(text)).to_string()
            }
            Value::Object(map) => {
                let mut entries: Vec<(&String, String)> = map
                    .iter()
                    .map(|(key, value)| (key, self.canonical(value)))
                    .collect();
                if self.ignore_key_order {
                    entries.sort_by(|a, b| a.0.cmp(b.0));
                }
                let inner: Vec<String> = entries
                    .into_iter()
                    .map(|(key, value)| format!("{}:{value}", Value::String(key.clone())))
                    .collect();
                format!("{{{}}}", inner.join(","))
            }
            Value::Array(items) => {
                format!(
                    "[{}]",
                    items
                        .iter()
                        .map(|item| self.canonical(item))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            other => other.to_string(),
        }
    }

    fn change(
        &mut self,
        kind: JsonChangeKind,
        path: &str,
        before: Option<&Value>,
        after: Option<&Value>,
    ) {
        self.changes.push(JsonChange {
            kind,
            path: path.to_owned(),
            before: before.map(summary),
            after: after.map(summary),
        });
    }

    fn operation(&mut self, op: &str, at: &str, value: Option<&Value>) {
        let mut entry = Map::new();
        entry.insert("op".to_owned(), Value::String(op.to_owned()));
        entry.insert("path".to_owned(), Value::String(at.to_owned()));
        if let Some(value) = value {
            entry.insert("value".to_owned(), value.clone());
        }
        self.patch.push(Value::Object(entry));
    }

    fn compare(&mut self, a: &Value, b: &Value, path: &str, at: &str) {
        match (a, b) {
            (Value::Object(left), Value::Object(right)) => self.objects(left, right, path, at),
            (Value::Array(left), Value::Array(right)) => self.arrays(left, right, path, at),
            _ if self.canonical(a) == self.canonical(b) => {}
            _ => {
                self.change(JsonChangeKind::Modified, path, Some(a), Some(b));
                self.operation("replace", at, Some(b));
            }
        }
    }

    fn objects(
        &mut self,
        left: &Map<String, Value>,
        right: &Map<String, Value>,
        path: &str,
        at: &str,
    ) {
        for (key, before) in left {
            let (key_path, key_at) = (child(path, key), pointer(at, key));
            if let Some(after) = right.get(key) {
                self.compare(before, after, &key_path, &key_at);
            } else {
                self.change(JsonChangeKind::Removed, &key_path, Some(before), None);
                self.operation("remove", &key_at, None);
            }
        }
        for (key, after) in right.iter().filter(|(key, _)| !left.contains_key(*key)) {
            self.change(JsonChangeKind::Added, &child(path, key), None, Some(after));
            self.operation("add", &pointer(at, key), Some(after));
        }
        if !self.ignore_key_order {
            let common = |map: &Map<String, Value>, other: &Map<String, Value>| -> Vec<String> {
                map.keys()
                    .filter(|key| other.contains_key(*key))
                    .cloned()
                    .collect()
            };
            if common(left, right) != common(right, left) {
                self.change(JsonChangeKind::Reordered, path, None, None);
            }
        }
    }

    /// Elements matched as a longest common sequence, so one inserted at the top is one change
    /// and not a change of every element below it. `cursor` is where each operation of the patch
    /// lands in the array as the operations before it have left it.
    fn arrays(&mut self, left: &[Value], right: &[Value], path: &str, at: &str) {
        let keys_left: Vec<String> = left.iter().map(|item| self.canonical(item)).collect();
        let keys_right: Vec<String> = right.iter().map(|item| self.canonical(item)).collect();
        let mut cursor = 0usize;

        for operation in capture_diff_slices(Algorithm::Myers, &keys_left, &keys_right) {
            let (_, old, new) = operation.as_tag_tuple();
            let paired = old.len().min(new.len());
            if let DiffOp::Equal { len, .. } = operation {
                cursor += len;
                continue;
            }
            for ((index, before), after) in left
                .iter()
                .enumerate()
                .skip(old.start)
                .take(paired)
                .zip(&right[new.start..])
            {
                let at_index = format!("{at}/{cursor}");
                self.compare(
                    before,
                    after,
                    &format!("{path}[{}]", new.start + index - old.start),
                    &at_index,
                );
                cursor += 1;
            }
            for (index, before) in left
                .iter()
                .enumerate()
                .take(old.end)
                .skip(old.start + paired)
            {
                self.change(
                    JsonChangeKind::Removed,
                    &format!("{path}[{index}]"),
                    Some(before),
                    None,
                );
                self.operation("remove", &format!("{at}/{cursor}"), None);
            }
            for (index, after) in right
                .iter()
                .enumerate()
                .take(new.end)
                .skip(new.start + paired)
            {
                self.change(
                    JsonChangeKind::Added,
                    &format!("{path}[{index}]"),
                    None,
                    Some(after),
                );
                self.operation("add", &format!("{at}/{cursor}"), Some(after));
                cursor += 1;
            }
        }
    }
}

// --- Laying the two documents out.

struct Line {
    text: String,
    /// What the line is compared by: its text, with spaces in a string collapsed when asked.
    key: String,
    path: String,
}

struct Printer<'a> {
    differ: &'a Differ,
    lines: Vec<Line>,
}

impl Printer<'_> {
    fn push(&mut self, depth: usize, label: Option<&str>, body: &str, key_body: &str, path: &str) {
        let prefix = format!(
            "{}{}",
            "  ".repeat(depth),
            label
                .map(|key| format!("{}: ", Value::String(key.to_owned())))
                .unwrap_or_default()
        );
        // The comma only says whether a sibling follows: a line that gains or loses one is the
        // same line, or every last key before an added one would read as changed.
        let key_body = key_body.strip_suffix(',').unwrap_or(key_body);
        self.lines.push(Line {
            text: format!("{prefix}{body}"),
            key: format!("{prefix}{key_body}"),
            path: path.to_owned(),
        });
    }

    /// Pretty JSON, two spaces, one line per value. `order` puts an object's keys in the order of
    /// the other side's, so the lines of the same keys meet on the same rows.
    fn value(
        &mut self,
        value: &Value,
        order: Option<&Value>,
        depth: usize,
        label: Option<&str>,
        path: &str,
        comma: bool,
    ) {
        let tail = if comma { "," } else { "" };
        match value {
            Value::Object(map) if !map.is_empty() => {
                self.push(depth, label, "{", "{", path);
                let keys = self.ordered(map, order);
                for (index, key) in keys.iter().enumerate() {
                    let other = order.and_then(|order| order.get(key.as_str()));
                    self.value(
                        &map[key.as_str()],
                        other,
                        depth + 1,
                        Some(key),
                        &child(path, key),
                        index + 1 < keys.len(),
                    );
                }
                self.push(
                    depth,
                    None,
                    &format!("}}{tail}"),
                    &format!("}}{tail}"),
                    path,
                );
            }
            Value::Array(items) if !items.is_empty() => {
                self.push(depth, label, "[", "[", path);
                for (index, item) in items.iter().enumerate() {
                    let other = order.and_then(|order| order.get(index));
                    self.value(
                        item,
                        other,
                        depth + 1,
                        None,
                        &format!("{path}[{index}]"),
                        index + 1 < items.len(),
                    );
                }
                self.push(depth, None, &format!("]{tail}"), &format!("]{tail}"), path);
            }
            _ => {
                let key = match value {
                    Value::String(text) if self.differ.ignore_whitespace => {
                        Value::String(collapsed(text)).to_string()
                    }
                    other => other.to_string(),
                };
                self.push(
                    depth,
                    label,
                    &format!("{value}{tail}"),
                    &format!("{key}{tail}"),
                    path,
                );
            }
        }
    }

    fn ordered(&self, map: &Map<String, Value>, order: Option<&Value>) -> Vec<String> {
        match order.and_then(Value::as_object) {
            Some(other) if self.differ.ignore_key_order => other
                .keys()
                .filter(|key| map.contains_key(*key))
                .chain(map.keys().filter(|key| !other.contains_key(*key)))
                .cloned()
                .collect(),
            _ => map.keys().cloned().collect(),
        }
    }
}

fn print(differ: &Differ, value: &Value, order: Option<&Value>) -> Vec<Line> {
    let mut printer = Printer {
        differ,
        lines: Vec::new(),
    };
    printer.value(value, order, 0, None, "$", false);
    printer.lines
}

fn row(
    kind: JsonDiffRowKind,
    left: Option<(usize, &Line)>,
    right: Option<(usize, &Line)>,
) -> JsonDiffRow {
    let line = |(index, line): (usize, &Line)| JsonDiffLine {
        number: saturating_u32(index + 1),
        text: line.text.clone(),
    };
    JsonDiffRow {
        kind,
        path: left
            .or(right)
            .map(|(_, line)| line.path.clone())
            .unwrap_or_default(),
        left: left.map(line),
        right: right.map(line),
    }
}

fn rows(left: &[Line], right: &[Line]) -> Vec<JsonDiffRow> {
    let keys_left: Vec<&str> = left.iter().map(|line| line.key.as_str()).collect();
    let keys_right: Vec<&str> = right.iter().map(|line| line.key.as_str()).collect();
    let mut rows = Vec::new();

    for operation in capture_diff_slices(Algorithm::Myers, &keys_left, &keys_right) {
        let (_, old, new) = operation.as_tag_tuple();
        let paired = old.len().min(new.len());
        let kind = match operation {
            DiffOp::Equal { .. } => JsonDiffRowKind::Same,
            DiffOp::Replace { .. } => JsonDiffRowKind::Modified,
            DiffOp::Delete { .. } => JsonDiffRowKind::Removed,
            DiffOp::Insert { .. } => JsonDiffRowKind::Added,
        };
        for offset in 0..paired {
            let (a, b) = (old.start + offset, new.start + offset);
            rows.push(row(kind, Some((a, &left[a])), Some((b, &right[b]))));
        }
        for line in left
            .iter()
            .enumerate()
            .take(old.end)
            .skip(old.start + paired)
        {
            rows.push(row(JsonDiffRowKind::Removed, Some(line), None));
        }
        for line in right
            .iter()
            .enumerate()
            .take(new.end)
            .skip(new.start + paired)
        {
            rows.push(row(JsonDiffRowKind::Added, None, Some(line)));
        }
    }
    rows
}

pub fn diff(request: &JsonDiffRequest) -> JsonDiffAnswer {
    let parse = |text: &str, side| {
        serde_json::from_str::<Value>(text).map_err(|error| JsonDiffAnswer::Unreadable {
            side,
            line: saturating_u32(error.line()),
            column: saturating_u32(error.column()),
        })
    };
    let (a, b) = match (
        parse(&request.a, JsonDiffSide::A),
        parse(&request.b, JsonDiffSide::B),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(unreadable), _) | (_, Err(unreadable)) => return unreadable,
    };

    let mut differ = Differ {
        ignore_key_order: request.ignore_key_order,
        ignore_whitespace: request.ignore_whitespace,
        changes: Vec::new(),
        patch: Vec::new(),
    };
    differ.compare(&a, &b, "$", "");

    let counts = differ
        .changes
        .iter()
        .fold(JsonDiffCounts::default(), |mut counts, change| {
            match change.kind {
                JsonChangeKind::Added => counts.added += 1,
                JsonChangeKind::Removed => counts.removed += 1,
                JsonChangeKind::Modified | JsonChangeKind::Reordered => counts.modified += 1,
            }
            counts
        });
    let mut rows = rows(&print(&differ, &a, None), &print(&differ, &b, Some(&a)));
    let rows_truncated = rows.len() > MAX_ROWS;
    rows.truncate(MAX_ROWS);

    JsonDiffAnswer::Compared {
        patch: serde_json::to_string_pretty(&differ.patch).unwrap_or_default(),
        changes: differ.changes,
        counts,
        rows,
        rows_truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAGING: &str = r#"{
  "service": "billing",
  "replicas": 2,
  "database": { "host": "db.staging.internal", "pool": 10 },
  "features": { "newInvoices": true, "betaExports": true },
  "logLevel": "debug"
}"#;

    const PRODUCTION: &str = r#"{
  "replicas": 6,
  "service": "billing",
  "database": { "host": "db.prod.internal", "pool": 40, "readReplica": "db-ro.prod.internal" },
  "features": { "newInvoices": true },
  "logLevel": "info",
  "sentry": { "sampleRate": 0.2 }
}"#;

    fn compared(
        a: &str,
        b: &str,
        ignore_key_order: bool,
        ignore_whitespace: bool,
    ) -> (Vec<JsonChange>, JsonDiffCounts, Vec<JsonDiffRow>, Value) {
        match diff(&JsonDiffRequest {
            a: a.to_owned(),
            b: b.to_owned(),
            ignore_key_order,
            ignore_whitespace,
        }) {
            JsonDiffAnswer::Compared {
                changes,
                counts,
                rows,
                patch,
                ..
            } => (changes, counts, rows, serde_json::from_str(&patch).unwrap()),
            JsonDiffAnswer::Unreadable { side, line, column } => panic!("{side:?} {line}:{column}"),
        }
    }

    fn summaries(changes: &[JsonChange]) -> Vec<(JsonChangeKind, &str)> {
        changes
            .iter()
            .map(|change| (change.kind, change.path.as_str()))
            .collect()
    }

    #[test]
    fn the_mockup_s_two_configurations_differ_by_seven_changes() {
        let (changes, counts, _, _) = compared(STAGING, PRODUCTION, true, true);

        assert_eq!(
            summaries(&changes),
            [
                (JsonChangeKind::Modified, "$.replicas"),
                (JsonChangeKind::Modified, "$.database.host"),
                (JsonChangeKind::Modified, "$.database.pool"),
                (JsonChangeKind::Added, "$.database.readReplica"),
                (JsonChangeKind::Removed, "$.features.betaExports"),
                (JsonChangeKind::Modified, "$.logLevel"),
                (JsonChangeKind::Added, "$.sentry"),
            ]
        );
        assert_eq!(
            counts,
            JsonDiffCounts {
                added: 2,
                removed: 1,
                modified: 4
            }
        );
        assert_eq!(
            changes[2].before,
            Some(ValueSummary::Scalar {
                text: "10".to_owned()
            })
        );
        assert_eq!(changes[6].after, Some(ValueSummary::Object { keys: 1 }));
    }

    #[test]
    fn key_order_counts_only_when_asked() {
        let (changes, ..) = compared(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#, true, true);
        assert!(changes.is_empty());

        let (changes, counts, ..) = compared(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#, false, true);
        assert_eq!(summaries(&changes), [(JsonChangeKind::Reordered, "$")]);
        assert_eq!(counts.modified, 1);
    }

    #[test]
    fn spaces_inside_a_string_count_only_when_asked() {
        let (changes, ..) = compared(r#"{"a":"x  y "}"#, r#"{"a":" x y"}"#, true, true);
        assert!(changes.is_empty());

        let (changes, ..) = compared(r#"{"a":"x  y "}"#, r#"{"a":" x y"}"#, true, false);
        assert_eq!(summaries(&changes), [(JsonChangeKind::Modified, "$.a")]);
    }

    #[test]
    fn an_element_inserted_at_the_top_is_one_change() {
        let (changes, _, _, patch) = compared(
            r#"[{"id":1},{"id":2}]"#,
            r#"[{"id":0},{"id":1},{"id":2}]"#,
            true,
            true,
        );

        assert_eq!(summaries(&changes), [(JsonChangeKind::Added, "$[0]")]);
        assert_eq!(
            patch,
            serde_json::json!([{ "op": "add", "path": "/0", "value": { "id": 0 } }])
        );
    }

    /// The patch is checked by applying it, which is what anyone will do with it.
    fn apply(document: &mut Value, patch: &Value) {
        for operation in patch.as_array().unwrap() {
            let at = operation["path"].as_str().unwrap();
            let (parent, last) = at.rsplit_once('/').unwrap();
            let last = last.replace("~1", "/").replace("~0", "~");
            let target = document.pointer_mut(parent).unwrap();
            match (operation["op"].as_str().unwrap(), target) {
                ("add", Value::Array(items)) => {
                    items.insert(last.parse().unwrap(), operation["value"].clone());
                }
                ("add" | "replace", Value::Object(map)) => {
                    map.insert(last, operation["value"].clone());
                }
                ("replace", Value::Array(items)) => {
                    items[last.parse::<usize>().unwrap()] = operation["value"].clone();
                }
                ("remove", Value::Array(items)) => {
                    items.remove(last.parse().unwrap());
                }
                ("remove", Value::Object(map)) => {
                    map.remove(&last);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn the_patch_turns_a_into_b() {
        for (a, b) in [
            (STAGING, PRODUCTION),
            (r#"{"l":[1,2,3,4,5]}"#, r#"{"l":[0,2,9,4,6,7]}"#),
            (
                r#"{"a/b":{"c~d":1},"x":[{"k":1},{"k":2}]}"#,
                r#"{"a/b":{"c~d":2},"x":[{"k":2}]}"#,
            ),
        ] {
            let (_, _, _, patch) = compared(a, b, true, false);
            let mut document: Value = serde_json::from_str(a).unwrap();
            apply(&mut document, &patch);
            assert_eq!(
                document,
                serde_json::from_str::<Value>(b).unwrap(),
                "{patch}"
            );
        }
    }

    #[test]
    fn the_rows_align_the_same_keys_and_mark_what_moved() {
        let (_, _, rows, _) = compared(STAGING, PRODUCTION, true, true);

        let replicas = rows.iter().find(|row| row.path == "$.replicas").unwrap();
        assert_eq!(replicas.kind, JsonDiffRowKind::Modified);
        assert_eq!(replicas.left.as_ref().unwrap().text, "  \"replicas\": 2,");
        assert_eq!(replicas.right.as_ref().unwrap().text, "  \"replicas\": 6,");
        let service = rows.iter().find(|row| row.path == "$.service").unwrap();
        assert_eq!(
            service.kind,
            JsonDiffRowKind::Same,
            "B's keys follow A's order when order is ignored"
        );
        let added = rows
            .iter()
            .find(|row| row.path == "$.database.readReplica")
            .unwrap();
        assert_eq!(
            (added.kind, added.left.is_none()),
            (JsonDiffRowKind::Added, true)
        );
        let kept = rows
            .iter()
            .find(|row| row.path == "$.features.newInvoices")
            .unwrap();
        assert_eq!(
            kept.kind,
            JsonDiffRowKind::Same,
            "a comma lost is no change"
        );
        let removed = rows
            .iter()
            .find(|row| row.path == "$.features.betaExports")
            .unwrap();
        assert_eq!(
            (removed.kind, removed.right.is_none()),
            (JsonDiffRowKind::Removed, true)
        );
    }

    #[test]
    fn a_side_that_does_not_parse_says_which_and_where() {
        assert_eq!(
            diff(&JsonDiffRequest {
                a: "{}".to_owned(),
                b: "{\n  \"a\": ,\n}".to_owned(),
                ignore_key_order: true,
                ignore_whitespace: true
            }),
            JsonDiffAnswer::Unreadable {
                side: JsonDiffSide::B,
                line: 2,
                column: 8
            }
        );
    }
}
